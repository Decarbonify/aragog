use crate::{DatabaseAccess, DatabaseRecord, Error, Record};
use arangors::uclient::ClientExt;

/// The `ForeignLink` trait of the Aragog library.
/// It allows to define foreign_key relations between different models.
///
/// # Example
///
/// ```rust
/// # use aragog::{Record, Validate, ForeignLink, DatabaseConnection, DatabaseRecord, AuthMode};
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
/// impl ForeignLink<User> for Order {
///     fn foreign_key(&self) -> &str {
///         self.user_id.borrow()
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
/// let order = Order {
///     content: "content".to_string(),
///     user_id: user.key().clone()
/// };
/// let linked_user = order.linked_model(&database_connection).await.unwrap();
/// assert_eq!(user.id(), linked_user.id());
/// # }
/// ```
#[maybe_async::maybe_async]
pub trait ForeignLink<T: Record> {
    /// Defines the foreign key field to the linked `T` model.
    ///
    /// # Example
    /// ```rust
    /// # use aragog::{Record, Validate, ForeignLink, DatabaseConnection, DatabaseRecord};
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
    /// impl ForeignLink<User> for Order {
    ///     fn foreign_key(&self) -> &str {
    ///         self.user_id.borrow()
    ///     }
    /// }
    /// ```
    fn foreign_key(&self) -> &str;

    /// Retrieves the record matching the defined `foreign_key`. Type inference may be required.
    #[cfg(not(feature = "blocking"))]
    async fn linked_model<D, C>(&self, db_access: &D) -> Result<DatabaseRecord<T>, Error>
    where
        Self: Sized,
        T: 'async_trait,
        C: ClientExt + Send,
        D: DatabaseAccess<C> + ?Sized,
    {
        DatabaseRecord::find(self.foreign_key(), db_access).await
    }

    /// Retrieves the record matching the defined `foreign_key`. Type inference may be required.
    #[cfg(feature = "blocking")]
    fn linked_model<D, C>(&self, db_access: &D) -> Result<DatabaseRecord<T>, Error>
    where
        C: ClientExt,
        D: DatabaseAccess<C> + ?Sized,
    {
        DatabaseRecord::find(self.foreign_key(), db_access)
    }
}
