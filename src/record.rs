use arangors::uclient::ClientExt;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::db::transaction::Transaction;
use crate::query::{Query, QueryCursor, QueryResult};
use crate::transaction::TransactionBuilder;
use crate::{DatabaseAccess, DatabaseConnection, DatabaseRecord, Error};

/// The main trait of the Aragog library.
/// Trait for structures that can be stored in Database.
/// The trait must be implemented to be used as a record in [`DatabaseRecord`]
///
/// [`DatabaseRecord`]: crate::DatabaseRecord
#[maybe_async::maybe_async]
pub trait Record: DeserializeOwned + Serialize + Clone {
    /// returns the associated Collection
    /// for read and write operations.
    const COLLECTION_NAME: &'static str;

    /// Finds a document in database from its unique key.
    /// Simple wrapper for [`DatabaseRecord`]<`T`>::[`find`]
    ///
    /// [`DatabaseRecord`]: crate::DatabaseRecord
    /// [`find`]: crate::DatabaseRecord::find
    async fn find<D, C>(key: &str, db_accessor: &D) -> Result<DatabaseRecord<Self>, Error>
    where
        C: ClientExt + Send,
        D: DatabaseAccess<C> + ?Sized,
    {
        DatabaseRecord::find(key, db_accessor).await
    }

    /// Finds all documents in database matching a `Query`.
    /// Simple wrapper for [`DatabaseRecord`]<`T`>::[`get`]
    ///
    /// [`DatabaseRecord`]: crate::DatabaseRecord
    /// [`get`]: crate::DatabaseRecord::get
    async fn get<D, C>(query: &Query, db_accessor: &D) -> Result<QueryResult<Self>, Error>
    where
        C: ClientExt,
        D: DatabaseAccess<C> + ?Sized,
    {
        DatabaseRecord::get(query, db_accessor).await
    }

    /// Finds all documents in database matching a `Query` in batches.
    /// Simple wrapper for [`DatabaseRecord`]<`T`>::[`get_in_batches`]
    ///
    /// [`DatabaseRecord`]: crate::DatabaseRecord
    /// [`get_in_batches`]: crate::DatabaseRecord::get_in_batches
    async fn get_in_batches<D, C>(
        query: &Query,
        db_accessor: &D,
        batch_size: u32,
    ) -> Result<QueryCursor<Self, C>, Error>
    where
        C: ClientExt,
        D: DatabaseAccess<C> + ?Sized,
    {
        DatabaseRecord::get_in_batches(query, db_accessor, batch_size).await
    }

    /// Returns true if there are any document in database matching a `Query`.
    /// Simple wrapper for [`DatabaseRecord`]<`T`>::[`exists`]
    ///
    /// [`DatabaseRecord`]: crate::DatabaseRecord
    /// [`exists`]: crate::DatabaseRecord::exists
    #[must_use]
    async fn exists<D, C>(query: &Query, db_accessor: &D) -> bool
    where
        C: ClientExt,
        D: DatabaseAccess<C> + ?Sized,
    {
        DatabaseRecord::<Self>::exists(query, db_accessor).await
    }

    /// Creates a new document in database.
    /// Simple wrapper for [`DatabaseRecord`]<`T`>::[`create`]
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aragog::{Record, DatabaseConnection};
    /// # use serde::{Deserialize, Serialize};
    /// #
    /// #[derive(Clone, Serialize, Deserialize, Record)]
    /// pub struct User {
    ///     pub name: String,
    /// }
    /// #
    /// # #[tokio::main]
    /// # async fn main() {
    /// # let db_connection = DatabaseConnection::builder()
    ///     # .with_schema_path("tests/schema.yaml")
    ///     # .apply_schema()
    ///     # .build()
    ///     # .await
    ///     # .unwrap();
    ///
    /// let user = User { name: "Patrick".to_owned() };
    /// let created_user = User::create(user, &db_connection).await.unwrap();
    ///
    /// assert_eq!(created_user.name, "Patrick".to_owned());
    /// # }
    /// ```
    /// [`DatabaseRecord`]: crate::DatabaseRecord
    /// [`create`]: crate::DatabaseRecord::create
    async fn create<D, C>(record: Self, db_accessor: &D) -> Result<DatabaseRecord<Self>, Error>
    where
        C: ClientExt + Send,
        D: DatabaseAccess<C> + ?Sized,
    {
        DatabaseRecord::create(record, db_accessor).await
    }

    /// Creates a new `Query` instance for `Self`.
    ///
    /// # Example
    /// ```rust
    /// # use aragog::query::Query;
    /// # use aragog::Record;
    /// # use serde::{Serialize, Deserialize};
    /// #[derive(Record, Clone, Serialize, Deserialize)]
    /// pub struct User { }
    ///
    /// // All three statements are equivalent:
    /// let q = User::query();
    /// let q = Query::new(User::COLLECTION_NAME);
    /// let q = Query::new("User");
    /// ```
    #[must_use]
    fn query() -> Query {
        Query::new(Self::COLLECTION_NAME)
    }

    /// method called by [`DatabaseRecord`]::[`create`]
    /// before the database operation.
    ///
    /// Define hooks manually or with macros (see the book)
    ///
    /// [`DatabaseRecord`]: crate::DatabaseRecord
    /// [`create`]: crate::DatabaseRecord::create
    async fn before_create_hook<D, C>(&mut self, db_accessor: &D) -> Result<(), Error>
    where
        C: ClientExt,
        D: DatabaseAccess<C> + ?Sized;

    /// method called by [`DatabaseRecord`]::[`save`]
    /// before the database operation.
    ///
    /// Define hooks manually or with macros (see the book)
    ///
    /// [`DatabaseRecord`]: crate::DatabaseRecord
    /// [`save`]: crate::DatabaseRecord::save
    async fn before_save_hook<D, C>(&mut self, db_accessor: &D) -> Result<(), Error>
    where
        C: ClientExt,
        D: DatabaseAccess<C> + ?Sized;

    /// method called by [`DatabaseRecord`]::[`delete`]
    /// before the database operation.
    ///
    /// Define hooks manually or with macros (see the book)
    ///
    /// [`DatabaseRecord`]: crate::DatabaseRecord
    /// [`delete`]: crate::DatabaseRecord::delete
    async fn before_delete_hook<D, C>(&mut self, db_accessor: &D) -> Result<(), Error>
    where
        C: ClientExt,
        D: DatabaseAccess<C> + ?Sized;

    /// method called automatically by [`DatabaseRecord`]::[`create`]
    /// after the database operation.
    ///
    /// Define hooks manually or with macros (see the book)
    ///
    /// [`DatabaseRecord`]: crate::DatabaseRecord
    /// [`create`]: crate::DatabaseRecord::create
    async fn after_create_hook<D, C>(&mut self, db_accessor: &D) -> Result<(), Error>
    where
        C: ClientExt,
        D: DatabaseAccess<C> + ?Sized;

    /// method called automatically by [`DatabaseRecord`]::[`save`]
    /// after the database operation.
    ///
    /// Define hooks manually or with macros (see the book)
    ///
    /// [`DatabaseRecord`]: crate::DatabaseRecord
    /// [`save`]: crate::DatabaseRecord::save
    async fn after_save_hook<D, C>(&mut self, db_accessor: &D) -> Result<(), Error>
    where
        C: ClientExt,
        D: DatabaseAccess<C> + ?Sized;

    /// method called automatically by [`DatabaseRecord`]::[`delete`]
    /// after the database operation.
    ///
    /// Define hooks manually or with macros (see the book)
    ///
    /// [`DatabaseRecord`]: crate::DatabaseRecord
    /// [`delete`]: crate::DatabaseRecord::delete
    async fn after_delete_hook<D, C>(&mut self, db_accessor: &D) -> Result<(), Error>
    where
        C: ClientExt,
        D: DatabaseAccess<C> + ?Sized;

    /// Returns a transaction builder on this collection only.
    #[must_use]
    fn transaction_builder() -> TransactionBuilder {
        TransactionBuilder::new().collections(vec![Self::COLLECTION_NAME.to_string()])
    }

    /// Builds a transaction for this collection only.
    ///
    /// # Arguments
    ///
    /// * `db_connection` - The current database connection
    async fn transaction<C: ClientExt + Send>(
        db_connection: &DatabaseConnection<C>,
    ) -> Result<Transaction<C>, Error> {
        Self::transaction_builder().build(db_connection).await
    }
}
