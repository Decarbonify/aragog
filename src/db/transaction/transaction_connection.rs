use std::collections::HashMap;

use arangors::{uclient::ClientExt, Database};

use crate::{db::database_collection::DatabaseCollection, DatabaseAccess, OperationOptions};

/// Struct equivalent to [`DatabaseConnection`] for transactional operations.
///
/// [`DatabaseConnection`]: crate::DatabaseConnection
#[derive(Debug, Clone)]
pub struct TransactionDatabaseConnection<C: ClientExt> {
    pub(crate) collections: HashMap<String, DatabaseCollection<C>>,
    pub(crate) database: Database<C>,
    pub(crate) operation_options: OperationOptions,
}

impl<C: ClientExt + Send> DatabaseAccess<C> for TransactionDatabaseConnection<C> {
    fn operation_options(&self) -> OperationOptions {
        self.operation_options.clone()
    }

    fn collection(&self, collection: &str) -> Option<&DatabaseCollection<C>> {
        self.collections.get(collection)
    }

    fn database(&self) -> &Database<C> {
        &self.database
    }
}
