use arangors::{AqlQuery, Document};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::{self, Display, Formatter};

use crate::db::database_service;
use crate::query::{Query, RecordQueryResult};
use crate::{DatabaseAccess, EdgeRecord, OperationOptions, Record, ServiceError};
use std::ops::{Deref, DerefMut};

/// Struct representing database stored documents.
///
/// The document of type `T` mut implement [`Record`]
///
/// # Note
///
/// `DatabaseRecord` implements `Deref` and `DerefMut` into `T`
///
/// [`Record`]: trait.Record.html
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DatabaseRecord<T> {
    /// The Document unique and indexed `_key`
    #[serde(rename(serialize = "_key", deserialize = "_key"))]
    pub(crate) key: String,
    /// The Document unique and indexed `_id`
    #[serde(rename(serialize = "_id", deserialize = "_id"))]
    pub(crate) id: String,
    /// The Document revision `_rev`
    #[serde(rename(serialize = "_rev", deserialize = "_rev"))]
    pub(crate) rev: String,
    /// The deserialized stored document
    #[serde(flatten)]
    pub record: T,
}

#[allow(dead_code)]
impl<T: Record> DatabaseRecord<T> {
    /// Creates a document in database.
    /// The function will write a new document and return a database record containing the newly created key
    ///
    /// # Note
    ///
    /// This method should be used for very specific cases, prefer using `delete` instead.
    /// If you want global operation options (always wait for sync, always ignore hooks, etc)
    /// configure your [`DatabaseConnection`] with `with_operation_options` to have a customs set
    /// of default options.
    ///
    /// # Hooks
    ///
    /// This function will launch `T` hooks `before_create` and `after_create` unless the `options`
    /// argument disables hooks.
    ///
    /// # Arguments
    ///
    /// * `record` - The document to create, it will be returned exactly as the `DatabaseRecord<T>` record
    /// * `db_accessor` - database connection reference
    /// * `options` - Operation options to apply
    ///
    /// # Returns
    ///
    /// On success a new instance of `Self` is returned, with the `key` value filled and `record` filled with the
    /// argument value
    /// A [`ServiceError`] is returned if the operation or the hooks failed.
    ///
    /// [`ServiceError`]: enum.ServiceError.html
    /// [`DatabaseConnection`]: struct.DatabaseConnection.html
    #[maybe_async::maybe_async]
    pub async fn create_with_options<D>(
        mut record: T,
        db_accessor: &D,
        options: OperationOptions,
    ) -> Result<Self, ServiceError>
    where
        D: DatabaseAccess + ?Sized,
    {
        let launch_hooks = !options.ignore_hooks;
        if launch_hooks {
            record.before_create_hook(db_accessor).await?;
        }
        let mut res =
            database_service::create_record(record, db_accessor, T::collection_name(), options)
                .await?;
        if launch_hooks {
            res.record.after_create_hook(db_accessor).await?;
        }
        Ok(res)
    }

    /// Creates a document in database.
    /// The function will write a new document and return a database record containing the newly created key
    ///
    /// # Hooks
    ///
    /// This function will launch `T` hooks `before_create` and `after_create` unless the `db_accessor`
    /// operations options specifically disable hooks.
    ///
    /// # Arguments
    ///
    /// * `record` - The document to create, it will be returned exactly as the `DatabaseRecord<T>` record
    /// * `db_accessor` - database connection reference
    ///
    /// # Returns
    ///
    /// On success a new instance of `Self` is returned, with the `key` value filled and `record` filled with the
    /// argument value
    /// A [`ServiceError`] is returned if the operation or the hooks failed.
    ///
    /// [`ServiceError`]: enum.ServiceError.html
    #[maybe_async::maybe_async]
    pub async fn create<D>(record: T, db_accessor: &D) -> Result<Self, ServiceError>
    where
        D: DatabaseAccess + ?Sized,
    {
        Self::create_with_options(record, db_accessor, db_accessor.operation_options()).await
    }

    /// Creates a document in database.
    /// The function will write a new document and return a database record containing the newly created key.
    ///
    /// # Note
    ///
    /// This function will **override** the default operations options:
    /// - Revision will be ignored
    /// - Hooks will be skipped
    /// and should be used sparingly.
    ///
    /// # Hooks
    ///
    /// This function will skip all hooks.
    ///
    /// # Arguments
    ///
    /// * `record` - The document to create, it will be returned exactly as the `DatabaseRecord<T>` record
    /// * `db_accessor` - database connection reference
    ///
    /// # Returns
    ///
    /// On success a new instance of `Self` is returned, with the `key` value filled and `record` filled with the
    /// argument value
    /// On failure a [`ServiceError`] is returned.
    ///
    /// [`ServiceError`]: enum.ServiceError.html
    #[maybe_async::maybe_async]
    pub async fn force_create<D>(record: T, db_accessor: &D) -> Result<Self, ServiceError>
    where
        D: DatabaseAccess + ?Sized,
    {
        Self::create_with_options(
            record,
            db_accessor,
            db_accessor
                .operation_options()
                .ignore_revs(true)
                .ignore_hooks(true),
        )
        .await
    }

    /// Writes in the database the new state of the record, "saving it".
    ///
    /// # Note
    ///
    /// This method should be used for very specific cases, prefer using `save` instead.
    /// If you want global operation options (always wait for sync, always ignore hooks, etc)
    /// configure your [`DatabaseConnection`] with `with_operation_options` to have a customs set
    /// of default options.
    ///
    /// # Hooks
    ///
    /// This function will launch `T` hooks `before_save` and `after_save` unless the `options`
    /// argument disables hooks.
    ///
    /// # Arguments:
    ///
    /// * `db_accessor` - database connection reference
    /// * `options` - Operation options to apply
    ///
    /// # Returns
    ///
    /// On success `()` is returned, meaning that the current instance is up to date with the database state.
    /// A [`ServiceError`] is returned if the operation or the hooks failed.
    ///
    /// [`ServiceError`]: enum.ServiceError.html
    /// [`DatabaseConnection`]: struct.DatabaseConnection.html
    #[maybe_async::maybe_async]
    pub async fn save_with_options<D>(
        &mut self,
        db_accessor: &D,
        options: OperationOptions,
    ) -> Result<(), ServiceError>
    where
        D: DatabaseAccess + ?Sized,
    {
        let launch_hooks = !options.ignore_hooks;
        if launch_hooks {
            self.record.before_save_hook(db_accessor).await?;
        }
        let mut new_record = database_service::update_record(
            self.clone(),
            self.key(),
            db_accessor,
            T::collection_name(),
            options,
        )
        .await?;
        if launch_hooks {
            new_record.record.after_save_hook(db_accessor).await?;
        }
        *self = new_record;
        Ok(())
    }

    /// Writes in the database the new state of the record, "saving it".
    ///
    /// # Hooks
    ///
    /// This function will launch `T` hooks `before_save` and `after_save` unless the `db_accessor`
    /// operations options specifically disable hooks.
    ///
    /// # Arguments:
    ///
    /// * `db_accessor` - database connection reference
    ///
    /// # Returns
    ///
    /// On success `()` is returned, meaning that the current instance is up to date with the database state.
    /// A [`ServiceError`] is returned if the operation or the hooks failed.
    ///
    /// [`ServiceError`]: enum.ServiceError.html
    #[maybe_async::maybe_async]
    pub async fn save<D>(&mut self, db_accessor: &D) -> Result<(), ServiceError>
    where
        D: DatabaseAccess + ?Sized,
    {
        self.save_with_options(db_accessor, db_accessor.operation_options())
            .await
    }

    /// Writes in the database the new state of the record.
    ///
    /// # Note
    ///
    /// This function will **override** the default operations options:
    /// - Revision will be ignored
    /// - Hooks will be skipped
    /// and should be used sparingly.
    ///
    /// # Hooks
    ///
    /// This function will skip all hooks.
    ///
    /// # Arguments:
    ///
    /// * `db_accessor` - database connection reference
    ///
    /// # Returns
    ///
    /// On success `()` is returned, meaning that the current instance is up to date with the database state.
    /// On failure a [`ServiceError`] is returned.
    ///
    /// [`ServiceError`]: enum.ServiceError.html
    #[maybe_async::maybe_async]
    pub async fn force_save<D>(&mut self, db_accessor: &D) -> Result<(), ServiceError>
    where
        D: DatabaseAccess + ?Sized,
    {
        self.save_with_options(
            db_accessor,
            db_accessor
                .operation_options()
                .ignore_hooks(true)
                .ignore_revs(true),
        )
        .await
    }

    /// Removes the record from the database.
    /// The structure won't be freed or emptied but the document won't exist in the global state
    ///
    /// # Note
    ///
    /// This method should be used for very specific cases, prefer using `delete` instead.
    /// If you want global operation options (always wait for sync, always ignore hooks, etc)
    /// configure your [`DatabaseConnection`] with `with_operation_options` to have a customs set
    /// of default options
    ///
    /// # Hooks
    ///
    /// This function will launch `T` hooks  `before_delete` and `after_delete` unless the `options`
    /// argument disables hooks.
    ///
    /// # Arguments:
    ///
    /// * `db_accessor` - database connection reference
    ///
    /// # Returns
    ///
    /// On success `()` is returned, meaning that the record is now deleted, the structure should not be used afterwards.
    /// A [`ServiceError`] is returned if the operation or the hooks failed.
    ///
    /// [`ServiceError`]: enum.ServiceError.html
    /// [`DatabaseConnection`]: struct.DatabaseConnection.html
    #[maybe_async::maybe_async]
    pub async fn delete_with_options<D>(
        &mut self,
        db_accessor: &D,
        options: OperationOptions,
    ) -> Result<(), ServiceError>
    where
        D: DatabaseAccess + ?Sized,
    {
        let launch_hooks = !options.ignore_hooks;
        if launch_hooks {
            self.record.before_delete_hook(db_accessor).await?;
        }
        database_service::remove_record::<T, D>(
            self.key(),
            db_accessor,
            T::collection_name(),
            options,
        )
        .await?;
        if launch_hooks {
            self.record.after_delete_hook(db_accessor).await?;
        }
        Ok(())
    }

    /// Removes the record from the database.
    /// The structure won't be freed or emptied but the document won't exist in the global state
    ///
    /// # Hooks
    ///
    /// This function will launch `T` hooks  `before_delete` and `after_delete` unless the `db_accessor`
    /// operations options specifically disable hooks.
    ///
    /// # Arguments:
    ///
    /// * `db_accessor` - database connection reference
    ///
    /// # Returns
    ///
    /// On success `()` is returned, meaning that the record is now deleted, the structure should not be used afterwards.
    /// A [`ServiceError`] is returned if the operation or the hooks failed.
    ///
    /// [`ServiceError`]: enum.ServiceError.html
    #[maybe_async::maybe_async]
    pub async fn delete<D>(&mut self, db_accessor: &D) -> Result<(), ServiceError>
    where
        D: DatabaseAccess + ?Sized,
    {
        self.delete_with_options(db_accessor, db_accessor.operation_options())
            .await
    }

    /// Removes the record from the database.
    /// The structure won't be freed or emptied but the document won't exist in the global state
    ///
    /// # Note
    ///
    /// This function will **override** the default operations options:
    /// - Revision will be ignored
    /// - Hooks will be skipped
    /// and should be used sparingly.
    ///
    /// # Hooks
    ///
    /// This function will skip all hooks.
    ///
    /// # Arguments:
    ///
    /// * `db_accessor` - database connection reference
    ///
    /// # Returns
    ///
    /// On success `()` is returned, meaning that the record is now deleted, the structure should not be used afterwards.
    /// On failure a [`ServiceError`] is returned.
    ///
    /// [`ServiceError`]: enum.ServiceError.html
    #[maybe_async::maybe_async]
    pub async fn force_delete<D>(&mut self, db_accessor: &D) -> Result<(), ServiceError>
    where
        D: DatabaseAccess + ?Sized,
    {
        self.delete_with_options(
            db_accessor,
            db_accessor
                .operation_options()
                .ignore_revs(true)
                .ignore_hooks(true),
        )
        .await
    }

    /// Creates and returns edge between `from_record` and `target_record`.
    ///
    /// # Hooks
    ///
    /// This function will launch `T` hooks `before_create` and `after_create`.
    ///
    /// # Example
    /// ```rust
    /// # use aragog::{DatabaseRecord, EdgeRecord, Record, DatabaseConnection};
    /// # use serde::{Serialize, Deserialize};
    /// #
    /// # #[derive(Record, Clone, Serialize, Deserialize)]
    /// # struct User {}
    /// #[derive(Clone, Record, Serialize, Deserialize)]
    /// struct Edge {
    ///     description: String,
    /// }
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// # let db_accessor = DatabaseConnection::builder()
    /// #     .with_schema_path("tests/schema.yaml")
    /// #     .apply_schema()
    /// #     .build().await.unwrap();
    /// # db_accessor.truncate();
    /// let user_a = DatabaseRecord::create(User { }, &db_accessor).await.unwrap();
    /// let user_b = DatabaseRecord::create(User { }, &db_accessor).await.unwrap();
    ///
    /// let edge = DatabaseRecord::link(&user_a, &user_b, &db_accessor,
    ///     Edge { description: "description".to_string() }
    /// ).await.unwrap();
    /// assert_eq!(edge.id_from(), user_a.id());
    /// assert_eq!(edge.id_to(), user_b.id());
    /// assert_eq!(&edge.description, "description");
    /// # }
    /// ```
    #[maybe_async::maybe_async]
    pub async fn link<A, B, D>(
        from_record: &DatabaseRecord<A>,
        to_record: &DatabaseRecord<B>,
        db_accessor: &D,
        edge_record: T,
    ) -> Result<DatabaseRecord<EdgeRecord<T>>, ServiceError>
    where
        A: Record,
        B: Record,
        D: DatabaseAccess + ?Sized,
        T: Record + Send,
    {
        let edge = EdgeRecord::new(
            from_record.id().clone(),
            to_record.id().clone(),
            edge_record,
        )?;
        DatabaseRecord::create(edge, db_accessor).await
    }

    /// Retrieves a record from the database with the associated unique `key`
    ///
    /// # Arguments:
    ///
    /// * `key` - the unique record key as a string slice
    /// * `db_accessor` - database connection reference
    ///
    /// # Returns
    ///
    /// On success `Self` is returned,
    /// On failure a [`ServiceError`] is returned:
    /// * [`NotFound`] on invalid document key
    /// * [`UnprocessableEntity`] on data corruption
    ///
    /// [`ServiceError`]: enum.ServiceError.html
    /// [`NotFound`]: enum.ServiceError.html#variant.NotFound
    /// [`UnprocessableEntity`]: enum.ServiceError.html#variant.UnprocessableEntity
    #[maybe_async::maybe_async]
    pub async fn find<D>(key: &str, db_accessor: &D) -> Result<Self, ServiceError>
    where
        D: DatabaseAccess + ?Sized,
    {
        database_service::retrieve_record(key, db_accessor, T::collection_name()).await
    }

    /// Reloads a record from the database, returning the new record.
    ///
    /// # Arguments
    ///
    /// * `db_accessor` - database connection reference
    ///
    /// # Returns
    ///
    /// On success `Self` is returned,
    /// On failure a [`ServiceError`] is returned:
    /// * [`NotFound`] on invalid document key
    /// * [`UnprocessableEntity`] on data corruption
    ///
    /// [`ServiceError`]: enum.ServiceError.html
    /// [`NotFound`]: enum.ServiceError.html#variant.NotFound
    /// [`UnprocessableEntity`]: enum.ServiceError.html#variant.UnprocessableEntity
    #[maybe_async::maybe_async]
    pub async fn reload<D>(self, db_accessor: &D) -> Result<Self, ServiceError>
    where
        D: DatabaseAccess + ?Sized,
        T: Send,
    {
        T::find(self.key(), db_accessor).await
    }

    /// Reloads a record from the database.
    ///
    /// # Returns
    ///
    /// On success `()` is returned and `self` is updated,
    /// On failure a [`ServiceError`] is returned:
    /// * [`NotFound`] on invalid document key
    /// * [`UnprocessableEntity`] on data corruption
    ///
    /// [`ServiceError`]: enum.ServiceError.html
    /// [`NotFound`]: enum.ServiceError.html#variant.NotFound
    /// [`UnprocessableEntity`]: enum.ServiceError.html#variant.UnprocessableEntity
    #[maybe_async::maybe_async]
    pub async fn reload_mut<D>(&mut self, db_accessor: &D) -> Result<(), ServiceError>
    where
        D: DatabaseAccess + ?Sized,
        T: Send,
    {
        *self = T::find(self.key(), db_accessor).await?;
        Ok(())
    }

    /// Retrieves all records from the database matching the associated conditions.
    ///
    /// # Arguments:
    ///
    /// * `query` - The `Query` to match
    /// * `db_accessor` - database connection reference
    ///
    /// # Note
    ///
    /// This is simply an AQL request wrapper.
    ///
    /// # Returns
    ///
    /// On success a `QueryResult` with a vector of `Self` is returned. It is can be empty.
    /// On failure a [`ServiceError`] is returned:
    /// * [`NotFound`] if no document matches the condition
    /// * [`UnprocessableEntity`] on data corruption
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aragog::query::{Comparison, Filter};
    /// # use serde::{Serialize, Deserialize};
    /// # use aragog::{DatabaseConnection, Record, DatabaseRecord};
    /// #
    /// # #[derive(Record, Clone, Serialize, Deserialize)]
    /// # struct User {
    /// #    username: String,
    /// #    age: u16,
    /// # }
    /// #
    /// # #[tokio::main]
    /// # async fn main() {
    /// # let db_accessor = DatabaseConnection::builder()
    /// #     .with_schema_path("tests/schema.yaml")
    /// #     .apply_schema()
    /// #     .build().await.unwrap();
    /// # db_accessor.truncate();
    /// # DatabaseRecord::create(User {username: "RobertSurcouf".to_string() ,age: 18 }, &db_accessor).await.unwrap();
    /// let query = User::query().filter(Filter::new(Comparison::field("username").equals_str("RobertSurcouf"))
    ///     .and(Comparison::field("age").greater_than(10)));
    ///
    /// // Both lines are equivalent:
    /// DatabaseRecord::<User>::get(query.clone(), &db_accessor).await.unwrap();
    /// User::get(query.clone(), &db_accessor).await.unwrap();
    /// # }
    /// ```
    ///
    /// [`ServiceError`]: enum.ServiceError.html
    /// [`NotFound`]: enum.ServiceError.html#variant.NotFound
    /// [`UnprocessableEntity`]: enum.ServiceError.html#variant.UnprocessableEntity
    #[maybe_async::maybe_async]
    pub async fn get<D>(query: Query, db_accessor: &D) -> Result<RecordQueryResult<T>, ServiceError>
    where
        D: DatabaseAccess + ?Sized,
    {
        Self::aql_get(&query.to_aql(), db_accessor).await
    }

    /// Retrieves all records from the database matching the associated conditions.
    ///
    /// # Arguments:
    ///
    /// * `query` - The AQL request string
    /// * `db_accessor` - database connection reference
    ///
    /// # Returns
    ///
    /// On success a `QueryResult` with a vector of `Self` is returned. It is can be empty.
    /// On failure a [`ServiceError`] is returned:
    /// * [`NotFound`] if no document matches the condition
    /// * [`UnprocessableEntity`] on data corruption
    ///
    /// # Warning
    ///
    /// If you call this method on a graph query only the documents that can be serialized into `T` will be returned.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use serde::{Serialize, Deserialize};
    /// # use aragog::{DatabaseConnection, Record, DatabaseRecord};
    /// #
    /// # #[derive(Record, Clone, Serialize, Deserialize)]
    /// # struct User {}
    /// #
    /// # #[tokio::main]
    /// # async fn main() {
    /// # let db_accessor = DatabaseConnection::builder()
    /// #     .with_schema_path("tests/schema.yaml")
    /// #     .apply_schema()
    /// #     .build().await.unwrap();
    /// # db_accessor.truncate();
    /// let query = r#"FOR i in User FILTER i.username == "RoertSurcouf" && i.age > 10 return i"#;
    ///
    /// DatabaseRecord::<User>::aql_get(query, &db_accessor).await.unwrap();
    /// # }
    /// ```
    ///
    /// [`ServiceError`]: enum.ServiceError.html
    /// [`NotFound`]: enum.ServiceError.html#variant.NotFound
    /// [`UnprocessableEntity`]: enum.ServiceError.html#variant.UnprocessableEntity
    #[maybe_async::maybe_async]
    pub async fn aql_get<D>(
        query: &str,
        db_accessor: &D,
    ) -> Result<RecordQueryResult<T>, ServiceError>
    where
        D: DatabaseAccess + ?Sized,
    {
        let result = db_accessor.aql_get(query).await?;
        Ok(result.into())
    }

    /// Creates a new outbound graph `Query` with `self` as a start vertex
    ///
    /// # Arguments
    ///
    /// * `edge_collection`- The name of the queried edge collection
    /// * `min` - The minimum depth of the graph request
    /// * `max` - The maximum depth of the graph request
    ///
    /// # Example
    /// ```rust no_run
    /// # use serde::{Serialize, Deserialize};
    /// # use aragog::query::Query;
    /// # use aragog::{DatabaseConnection, Record};
    /// #
    /// # #[derive(Record, Clone, Serialize, Deserialize)]
    /// # struct User {}
    /// #
    /// # #[tokio::main]
    /// # async fn main() {
    /// # let db_accessor = DatabaseConnection::builder().build().await.unwrap();
    /// let record = User::find("123", &db_accessor).await.unwrap();
    /// // Both statements are equivalent
    /// let q = record.outbound_query(1, 2, "ChildOf");
    /// let q = Query::outbound(1, 2, "ChildOf", record.id());
    /// # }
    /// ```
    pub fn outbound_query(&self, min: u16, max: u16, edge_collection: &str) -> Query {
        Query::outbound(min, max, edge_collection, &self.id)
    }

    /// Creates a new inbound graph `Query` with `self` as a start vertex
    ///
    /// # Arguments
    ///
    /// * `edge_collection`- The name of the queried edge collection
    /// * `min` - The minimum depth of the graph request
    /// * `max` - The maximum depth of the graph request
    ///
    /// # Example
    /// ```rust no_run
    /// # use serde::{Serialize, Deserialize};
    /// # use aragog::query::Query;
    /// # use aragog::{DatabaseConnection, Record};
    /// #
    /// # #[derive(Record, Clone, Serialize, Deserialize)]
    /// # struct User {}
    /// #
    /// # #[tokio::main]
    /// # async fn main() {
    /// # let db_accessor = DatabaseConnection::builder().build().await.unwrap();
    /// #
    /// let record = User::find("123", &db_accessor).await.unwrap();
    /// // Both statements are equivalent
    /// let q = record.inbound_query(1, 2, "ChildOf");
    /// let q = Query::inbound(1, 2, "ChildOf", record.id());
    /// # }
    /// ```
    pub fn inbound_query(&self, min: u16, max: u16, edge_collection: &str) -> Query {
        Query::inbound(min, max, edge_collection, &self.id)
    }

    /// Creates a new outbound graph `Query` with `self` as a start vertex
    ///
    /// # Arguments
    ///
    /// * `min` - The minimum depth of the graph request
    /// * `max` - The maximum depth of the graph request
    /// * `named_graph`- The named graph to traverse
    ///
    /// # Example
    /// ```rust no_run
    /// # use serde::{Serialize, Deserialize};
    /// # use aragog::query::Query;
    /// # use aragog::{DatabaseConnection, Record};
    /// #
    /// # #[derive(Record, Clone, Serialize, Deserialize)]
    /// # struct User {}
    /// #
    /// # #[tokio::main]
    /// # async fn main() {
    /// # let db_accessor = DatabaseConnection::builder().build().await.unwrap();
    /// let record = User::find("123", &db_accessor).await.unwrap();
    /// // Both statements are equivalent
    /// let q = record.outbound_graph(1, 2, "SomeGraph");
    /// let q = Query::outbound_graph(1, 2, "SomeGraph", record.id());
    /// # }
    /// ```
    pub fn outbound_graph(&self, min: u16, max: u16, named_graph: &str) -> Query {
        Query::outbound_graph(min, max, named_graph, &self.id)
    }

    /// Creates a new inbound graph `Query` with `self` as a start vertex
    ///
    /// # Arguments
    ///
    /// * `min` - The minimum depth of the graph request
    /// * `max` - The maximum depth of the graph request
    /// * `named_graph`- The named graph to traverse
    ///
    /// # Example
    /// ```rust no_run
    /// # use serde::{Serialize, Deserialize};
    /// # use aragog::query::Query;
    /// # use aragog::{DatabaseConnection, Record};
    /// #
    /// # #[derive(Record, Clone, Serialize, Deserialize)]
    /// # struct User {}
    /// #
    /// # #[tokio::main]
    /// # async fn main() {
    /// # let db_accessor = DatabaseConnection::builder().build().await.unwrap();
    /// let record = User::find("123", &db_accessor).await.unwrap();
    /// // Both statements are equivalent
    /// let q = record.inbound_graph(1, 2, "SomeGraph");
    /// let q = Query::inbound_graph(1, 2, "SomeGraph", record.id());
    /// # }
    /// ```
    pub fn inbound_graph(&self, min: u16, max: u16, named_graph: &str) -> Query {
        Query::inbound_graph(min, max, named_graph, &self.id)
    }

    /// Checks if any document matching the associated conditions exist
    ///
    /// # Arguments:
    ///
    /// * `query` - The `Query` to match
    /// * `db_accessor` - database connection reference
    ///
    /// # Note
    ///
    /// This is simply an AQL request wrapper.
    ///
    /// # Returns
    ///
    /// On success `true` is returned, `false` if nothing exists.
    ///
    /// # Example
    ///
    /// ```rust no_run
    /// # use serde::{Serialize, Deserialize};
    /// # use aragog::{DatabaseConnection, Record};
    /// # use aragog::query::{Query, Comparison, Filter};
    /// #
    /// # #[derive(Record, Clone, Serialize, Deserialize)]
    /// # struct User {}
    /// #
    /// # #[tokio::main]
    /// # async fn main() {
    /// # let db_accessor = DatabaseConnection::builder().build().await.unwrap();
    /// let query = User::query().filter(
    ///     Filter::new(Comparison::field("username").equals_str("MichelDu93"))
    ///         .and(Comparison::field("age").greater_than(10)));
    /// User::exists(query, &db_accessor).await;
    /// # }
    /// ```
    #[maybe_async::maybe_async]
    pub async fn exists<D>(query: Query, db_accessor: &D) -> bool
    where
        D: DatabaseAccess + ?Sized,
    {
        let aql_string = query.to_aql();
        let aql_query = AqlQuery::builder()
            .query(&aql_string)
            .batch_size(1)
            .count(true)
            .build();
        match db_accessor
            .database()
            .aql_query_batch::<Value>(aql_query)
            .await
        {
            Ok(cursor) => match cursor.count {
                Some(count) => count > 0,
                None => false,
            },
            Err(_error) => false,
        }
    }

    /// Getter for the Document `_id` built as `$collection_name/$_key
    pub fn id(&self) -> &String {
        &self.id
    }

    /// Getter for the Document `_key`
    pub fn key(&self) -> &String {
        &self.key
    }

    /// Getter for the Document `_rev`
    pub fn rev(&self) -> &String {
        &self.rev
    }
}

impl<T: Record> From<Document<T>> for DatabaseRecord<T> {
    fn from(doc: Document<T>) -> Self {
        Self {
            key: doc.header._key,
            id: doc.header._id,
            rev: doc.header._rev,
            record: doc.document,
        }
    }
}

impl<T: Record> Display for DatabaseRecord<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} Database Record", T::collection_name(), self.key())
    }
}

impl<T: Record> Deref for DatabaseRecord<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.record
    }
}

impl<T: Record> DerefMut for DatabaseRecord<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.record
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn struct_serialize_deserialize() {
        #[derive(Serialize, Deserialize, Clone)]
        struct Doc {
            a: String,
            b: u16,
            c: Vec<bool>,
        }

        let db_record = DatabaseRecord {
            key: "key".to_string(),
            id: "id".to_string(),
            rev: "rev".to_string(),
            record: Doc {
                a: "a".to_string(),
                b: 10,
                c: vec![false, true, false],
            },
        };
        let json = serde_json::to_string(&db_record).unwrap();
        let parsed_record: DatabaseRecord<Doc> = serde_json::from_str(&json).unwrap();
        assert_eq!(&parsed_record.key, &db_record.key);
        assert_eq!(&parsed_record.id, &db_record.id);
        assert_eq!(&parsed_record.rev, &db_record.rev);
        assert_eq!(parsed_record.record.a, db_record.record.a);
        assert_eq!(parsed_record.record.b, db_record.record.b);
        assert_eq!(parsed_record.record.c, db_record.record.c);
    }

    #[test]
    fn struct_with_enum_serialize_deserialize() {
        #[derive(Serialize, Deserialize, Clone)]
        struct Doc {
            doc: DocEnum,
        }

        #[derive(Serialize, Deserialize, Clone)]
        enum DocEnum {
            A { a: String, b: u16, c: Vec<bool> },
            B { a: bool, b: f64 },
        }

        let db_record = DatabaseRecord {
            key: "key".to_string(),
            id: "id".to_string(),
            rev: "rev".to_string(),
            record: Doc {
                doc: DocEnum::A {
                    a: "a".to_string(),
                    b: 10,
                    c: vec![false, true, false],
                },
            },
        };
        let json = serde_json::to_string(&db_record).unwrap();
        let parsed_record: DatabaseRecord<Doc> = serde_json::from_str(&json).unwrap();
        assert_eq!(&parsed_record.key, &db_record.key);
        assert_eq!(&parsed_record.id, &db_record.id);
        assert_eq!(&parsed_record.rev, &db_record.rev);
        match parsed_record.record.doc {
            DocEnum::A { a, b, c } => {
                assert_eq!(&a, "a");
                assert_eq!(b, 10);
                assert_eq!(c, vec![false, true, false]);
            }
            DocEnum::B { .. } => panic!("Wrong enum variant"),
        }
    }

    #[test]
    fn enum_serialize_deserialize() {
        #[derive(Serialize, Deserialize, Clone)]
        enum DocEnum {
            A { a: String, b: u16, c: Vec<bool> },
            B { a: bool, b: f64 },
        }

        let db_record = DatabaseRecord {
            key: "key".to_string(),
            id: "id".to_string(),
            rev: "rev".to_string(),
            record: DocEnum::A {
                a: "a".to_string(),
                b: 10,
                c: vec![false, true, false],
            },
        };
        let json = serde_json::to_string(&db_record).unwrap();
        let parsed_record: DatabaseRecord<DocEnum> = serde_json::from_str(&json).unwrap();
        assert_eq!(&parsed_record.key, &db_record.key);
        assert_eq!(&parsed_record.id, &db_record.id);
        assert_eq!(&parsed_record.rev, &db_record.rev);
        match parsed_record.record {
            DocEnum::A { a, b, c } => {
                assert_eq!(&a, "a");
                assert_eq!(b, 10);
                assert_eq!(c, vec![false, true, false]);
            }
            DocEnum::B { .. } => panic!("Wrong enum variant"),
        }
    }
}
