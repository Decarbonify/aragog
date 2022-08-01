#[cfg(not(feature = "blocking"))]
use std::future::Future;

use arangors::{
    transaction::{Status, Transaction as TransactionLayer},
    uclient::ClientExt,
};

pub use {
    transaction_builder::TransactionBuilder, transaction_connection::TransactionDatabaseConnection,
    transaction_output::TransactionOutput,
};

use crate::{DatabaseConnection, Error};

mod transaction_builder;
mod transaction_connection;
mod transaction_output;

/// Struct representing a `ArangoDB` transaction.
///
/// Its `database_connection` is equivalent to a [`DatabaseConnection`] but for transactional operations.
/// Use it instead of the classic connection to use the streaming transaction.
///
/// # Example
///
/// ```rust
/// # use aragog::{DatabaseConnection, transaction::Transaction, Record, Validate, DatabaseRecord};
/// # use aragog::query::{Comparison, Filter};
/// # use serde::{Serialize, Deserialize};
///
/// #[derive(Debug, Clone, Record, Validate, Serialize, Deserialize)]
/// pub struct User {
///     pub field1: String,
///     pub field2: String
/// }
///
/// # #[tokio::main]
/// # async fn main() {
/// let db_connection = DatabaseConnection::builder()
///     # .with_schema_path("tests/schema.yaml")
///     # .apply_schema()
///     .build()
///     .await
///     .unwrap();
/// # db_connection.truncate().await;
/// // Build a transaction connection from the main database connection
/// let transaction = Transaction::new(&db_connection).await.unwrap();
/// // Safely execute document operations in the transaction, the transaction will be closed afterwards
/// let result = transaction.safe_execute(|connection| async move {
///     // All operations here will be transactional, if an error is raised, the transaction will be aborted.
///     let doc = User {
///         field1: String::from("foo"),
///         field2: String::from("bar"),
///     };
///     // The closure safely checks for errors, use the `?` operator and avoid `unwrap()`
///     let mut db_doc = DatabaseRecord::create(doc, &connection).await?;
///     db_doc.field1 = String::from("not foo");
///     db_doc.save(&connection).await?;
///     Ok(db_doc)
/// }).await.unwrap();
///
/// // We make sure everything was committed
/// assert!(result.is_committed());
/// // We retrieve our document from the classic connection to check if it worked
/// let result = User::get(
///     &User::query().filter(Comparison::field("field1").equals_str("not foo").into()),
///     &db_connection
/// ).await.unwrap();
/// assert_eq!(result.len(), 1);
/// # }
/// ```
///
/// # Note
///
/// The `WRITE` transaction operations muse be document related: `create`, `save`, `delete`, etc. The AQL operations may not work.
/// On the other hand all `READ` operations as `find`, `get`, etc should all work even with `AQL` queries.
///
/// [`DatabaseConnection`]: crate::DatabaseConnection
#[derive(Debug)]
pub struct Transaction<C: ClientExt> {
    accessor: TransactionLayer<C>,
    database_connection: TransactionDatabaseConnection<C>,
}

impl<C: ClientExt> Transaction<C> {
    /// Transaction unique identifier
    #[must_use]
    #[inline]
    pub fn id(&self) -> &str {
        self.accessor.id()
    }

    /// Instantiates a new `Transaction` from a [`DatabaseConnection`] on all collections
    ///
    /// # Arguments
    ///
    /// * `db_connection` - The current database connection
    ///
    /// The transaction will be initialized with default settings:
    /// - No disk writing wait (waitForSync)
    /// - A lock timeout of 60 000
    /// - No collection restriction
    ///
    /// For more options use [`TransactionBuilder`]
    ///
    /// [`DatabaseConnection`]: crate::DatabaseConnection
    #[maybe_async::maybe_async]
    pub async fn new(db_connection: &DatabaseConnection<C>) -> Result<Self, Error> {
        TransactionBuilder::new().build(db_connection).await
    }

    /// Tries to commit all operations from the transaction
    ///
    /// A `Transaction` instance can be committed multiple times.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aragog::{DatabaseConnection, transaction::Transaction, Record, Validate, DatabaseRecord};
    /// # use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Debug, Clone, Record, Validate, Serialize, Deserialize)]
    /// pub struct User {
    ///     pub field1: String,
    ///     pub field2: String
    /// }
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// # let db_connection = DatabaseConnection::builder()
    ///     # .with_schema_path("tests/schema.yaml")
    ///     # .apply_schema()
    ///     # .build()
    ///     # .await
    ///     # .unwrap();
    /// // Build a transaction connection from the main database connection
    /// let transaction = Transaction::new(&db_connection).await.unwrap();
    /// let doc = User {
    ///     field1: String::from("foo"),
    ///     field2: String::from("bar"),
    /// };
    /// // Use the transaction connection instead of the standard `DatabaseConnection`
    /// match DatabaseRecord::create(doc, transaction.database_connection()).await {
    ///     /// On the operation success we commit the complete pool of operations
    ///     Ok(_) => transaction.commit().await.unwrap(),
    ///     /// On the operation success we abort the complete pool of operations
    ///     Err(_) => transaction.abort().await.unwrap()
    /// }
    /// # }
    /// ```
    ///
    /// # Note
    ///
    /// For a more practical and safer use, use the `safe_execute` method which allows multiple operations
    #[maybe_async::maybe_async]
    pub async fn commit(&self) -> Result<(), Error> {
        let status = self.accessor.commit().await?;
        log::debug!("Transaction committed with status: {:?}", status);
        if !matches!(status, Status::Committed) {
            let msg = format!("Unexpected {:?} transaction status after commit", status);
            log::error!("{}", msg);
            return Err(Error::InternalError { message: Some(msg) });
        }
        Ok(())
    }

    /// Tries to abort all operations from the transaction.
    ///
    /// If the operation succeeds, the `ArangoDB` transaction will be deleted and the current
    /// `Transaction` instance can't be used anymore.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aragog::{DatabaseConnection, transaction::Transaction, Record, Validate, DatabaseRecord};
    /// # use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Debug, Clone, Record, Validate, Serialize, Deserialize)]
    /// pub struct User {
    ///     pub field1: String,
    ///     pub field2: String
    /// }
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// # let db_connection = DatabaseConnection::builder()
    ///     # .with_schema_path("tests/schema.yaml")
    ///     # .apply_schema()
    ///     # .build()
    ///     # .await
    ///     # .unwrap();
    /// // Build a transaction connection from the main database connection
    /// let transaction = Transaction::new(&db_connection).await.unwrap();
    /// let doc = User {
    ///     field1: String::from("foo"),
    ///     field2: String::from("bar"),
    /// };
    /// // Use the transaction connection instead of the standard `DatabaseConnection`
    /// match DatabaseRecord::create(doc, transaction.database_connection()).await {
    ///     /// On the operation success we commit the complete pool of operations
    ///     Ok(_) => transaction.commit().await.unwrap(),
    ///     /// On the operation failure we abort the complete pool of operations
    ///     Err(_) => transaction.abort().await.unwrap()
    /// }
    /// # }
    /// ```
    ///
    /// # Note
    ///
    /// For a more practical and safer use, use the `safe_execute` method which allows multiple operations
    #[maybe_async::maybe_async]
    pub async fn abort(&self) -> Result<(), Error> {
        let status = self.accessor.abort().await?;
        log::debug!("Transaction aborted with status: {:?}", status);
        if !matches!(status, Status::Aborted) {
            let msg = format!("Unexpected {:?} transaction status after abort", status);
            log::error!("{}", msg);
            return Err(Error::InternalError { message: Some(msg) });
        }
        Ok(())
    }

    /// Allows to run multiple operations using the transaction connection. If an operation fails or an `Err`
    /// is returned by the closure, all operations will be aborted.
    ///
    /// # Errors
    ///
    /// The closure error is returned
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aragog::{DatabaseConnection, transaction::Transaction, Record, Validate, DatabaseRecord};
    /// # use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Debug, Clone, Record, Validate, Serialize, Deserialize)]
    /// pub struct User {
    ///     pub field1: String,
    ///     pub field2: String
    /// }
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// # let db_connection = DatabaseConnection::builder()
    ///     # .with_schema_path("tests/schema.yaml")
    ///     # .apply_schema()
    ///     # .build()
    ///     # .await
    ///     # .unwrap();
    /// # db_connection.truncate().await;
    /// // Build a transaction connection from the main database connection
    /// let transaction = Transaction::new(&db_connection).await.unwrap();
    /// // Safely execute document operations in the transaction
    /// transaction.safe_execute(|connection| async move {
    ///     // All operations here will be transactional, if an error is raised, the transaction will be aborted.
    ///     let doc = User {
    ///         field1: String::from("foo"),
    ///         field2: String::from("bar"),
    ///     };
    ///     // The closure safely checks for errors, use the `?` operator and avoid `unwrap()`
    ///     let mut db_doc = DatabaseRecord::create(doc, &connection).await?;
    ///     db_doc.field1 = String::from("not foo");
    ///     db_doc.save(&connection).await?;
    ///     Ok(db_doc)
    /// }).await.unwrap();
    /// # }
    /// ```
    ///
    /// # Note
    ///
    /// Don't use `unwrap()` in the closure, as if the code panics the transaction won't be aborted nor commited.
    #[cfg(not(feature = "blocking"))]
    pub async fn safe_execute<T, O, F>(&self, operations: O) -> Result<TransactionOutput<T>, Error>
    where
        O: FnOnce(TransactionDatabaseConnection<C>) -> F,
        F: Future<Output = Result<T, Error>>,
    {
        log::trace!("Safely executing transactional operations..");
        let res = operations(self.database_connection.clone()).await;
        log::trace!(
            "Safely executing transactional operations.. Done. Success: {}",
            res.is_ok()
        );
        self.handle_safe_execute(res).await
    }

    /// Allows to run multiple operations using the transaction connection. If an operation fails or an `Err`
    /// is returned by the closure, all operations will be aborted.
    ///
    /// # Errors
    ///
    /// The closure error is returned
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aragog::{DatabaseConnection, transaction::Transaction, Record, Validate, DatabaseRecord};
    /// # use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Debug, Clone, Record, Validate, Serialize, Deserialize)]
    /// pub struct User {
    ///     pub field1: String,
    ///     pub field2: String
    /// }
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// # let db_connection = DatabaseConnection::builder()
    ///     # .with_schema_path("tests/schema.yaml")
    ///     # .apply_schema()
    ///     # .build()
    ///     # .await
    ///     # .unwrap();
    /// # db_connection.truncate().await;
    /// // Build a transaction connection from the main database connection
    /// let transaction = Transaction::new(&db_connection).await.unwrap();
    /// // Safely execute document operations in the transaction
    /// transaction.safe_execute(|connection| async move {
    ///     // All operations here will be transactional, if an error is raised, the transaction will be aborted.
    ///     let doc = User {
    ///         field1: String::from("foo"),
    ///         field2: String::from("bar"),
    ///     };
    ///     // The closure safely checks for errors, use the `?` operator and avoid `unwrap()`
    ///     let mut db_doc = DatabaseRecord::create(doc, &connection).await?;
    ///     db_doc.field1 = String::from("not foo");
    ///     db_doc.save(&connection).await?;
    ///     Ok(db_doc)
    /// }).await.unwrap();
    /// # }
    /// ```
    ///
    /// # Note
    ///
    /// Don't use `unwrap()` in the closure, as if the code panics the transaction won't be aborted nor commited.
    #[cfg(feature = "blocking")]
    pub fn safe_execute<T, O>(&self, operations: O) -> Result<TransactionOutput<T>, Error>
    where
        O: FnOnce(TransactionDatabaseConnection) -> Result<T, Error>,
    {
        log::trace!("Safely executing transactional operations..");
        let res = operations(self.database_connection.clone());
        log::trace!(
            "Safely executing transactional operations.. Done. Success: {}",
            res.is_ok()
        );
        self.handle_safe_execute(res)
    }

    #[maybe_async::maybe_async]
    async fn handle_safe_execute<T>(
        &self,
        result: Result<T, Error>,
    ) -> Result<TransactionOutput<T>, Error> {
        match result {
            Ok(value) => {
                log::debug!("Transaction succeeded. Committing..");
                self.commit().await?;
                Ok(TransactionOutput::Committed(value))
            }
            Err(err) => {
                log::debug!("Transaction failed with: {}. Aborting..", err);
                self.abort().await?;
                Ok(TransactionOutput::Aborted(err))
            }
        }
    }

    /// Retrieves the database connection of the transaction which implements [`DatabaseAccess`].
    /// This connection can be used exactly the same way was the classic database connection.
    ///
    /// [`DatabaseAccess`]: crate::DatabaseAccess
    #[must_use]
    #[inline]
    pub const fn database_connection(&self) -> &TransactionDatabaseConnection<C> {
        &self.database_connection
    }
}
