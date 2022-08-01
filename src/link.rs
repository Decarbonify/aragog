use crate::query::{Query, QueryResult};
use crate::{DatabaseAccess, DatabaseRecord, Error, Record};
use arangors::uclient::ClientExt;

/// The `Link` trait of the Aragog library.
/// It allows to define a query relation between different models.
///
/// # Example
///
/// ```rust
/// # use aragog::{Record, Validate, Link, DatabaseConnection, DatabaseRecord, AuthMode};
/// # use aragog::query::{Query, Comparison};
/// # use serde::{Deserialize, Serialize};
/// # use std::borrow::Borrow;
/// #
/// #[derive(Clone, Serialize, Deserialize, Record, Validate)]
/// pub struct Order {
///     pub content: String,
///     pub user_id: String,
/// }
///
/// #[derive(Clone, Serialize, Deserialize, Record, Validate)]
/// pub struct User {}
///
/// impl Link<Order> for DatabaseRecord<User> {
///     fn link_query(&self) -> Query {
///         Order::query().filter(Comparison::field("user_id").equals_str(self.key()).into())
///     }
/// }
///
/// # #[tokio::main]
/// # async fn main() {
/// # let database_connection = DatabaseConnection::builder()
/// #     .with_credentials(
/// #       &std::env::var("DB_HOST").unwrap_or("http://localhost:8529".to_string()),
/// #       &std::env::var("DB_NAME").unwrap_or("aragog_test".to_string()),
/// #       &std::env::var("DB_USER").unwrap_or("test".to_string()),
/// #       &std::env::var("DB_PWD").unwrap_or("test".to_string())
/// #     )
/// #    .with_schema_path("tests/schema.yaml")
/// #    .build()
/// #    .await
/// #    .unwrap();
/// # database_connection.truncate().await;
/// let user = DatabaseRecord::create(User {}, &database_connection).await.unwrap();
/// let order = DatabaseRecord::create(
///     Order {
///         content: "content".to_string(),
///         user_id: user.key().clone()
///     },
///     &database_connection).await.unwrap();
/// let orders = user.linked_models(&database_connection).await.unwrap();
/// assert_eq!(user.key(), &orders.first().unwrap().user_id);
/// # }
/// ```
#[maybe_async::must_be_async]
pub trait Link<T: Record + Send> {
    /// Defines the query to execute to find the `T` models linked to `Self`
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aragog::{Record, Validate, Link, DatabaseConnection, DatabaseRecord};
    /// # use aragog::query::{Query, Comparison};
    /// # use serde::{Deserialize, Serialize};
    /// # use std::borrow::Borrow;
    /// #
    /// #[derive(Clone, Serialize, Deserialize, Record, Validate)]
    /// pub struct Order {
    ///     pub content: String,
    ///     pub user_id: String,
    /// }
    ///
    /// #[derive(Clone, Serialize, Deserialize, Record, Validate)]
    /// pub struct User {}
    ///
    /// impl Link<Order> for DatabaseRecord<User> {
    ///     fn link_query(&self) -> Query {
    ///         Order::query().filter(Comparison::field("user_id").equals_str(self.key()).into())
    ///     }
    /// }
    ///```
    fn link_query(&self) -> Query;

    /// Retrieves the records matching the defined `link_query`. Type inference may be required.
    #[cfg(not(feature = "blocking"))]
    async fn linked_models<D, C>(&self, db_access: &D) -> Result<QueryResult<T>, Error>
    where
        Self: Sized,
        C: ClientExt,
        D: DatabaseAccess<C> + ?Sized,
        T: 'async_trait,
    {
        DatabaseRecord::get(&self.link_query(), db_access).await
    }

    /// Retrieves the records matching the defined `link_query`. Type inference may be required.
    #[cfg(feature = "blocking")]
    fn linked_models<D, C>(&self, db_access: &D) -> Result<QueryResult<T>, Error>
    where
        C: ClientExt,
        D: DatabaseAccess<C> + ?Sized,
    {
        DatabaseRecord::get(&self.link_query(), db_access)
    }
}
