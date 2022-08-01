use crate::db::database_collection::DatabaseCollection;
use crate::transaction::{Transaction, TransactionDatabaseConnection};
use crate::{DatabaseAccess, DatabaseConnection, Error, OperationOptions};
use arangors::transaction::{TransactionCollections, TransactionSettings};
use arangors::uclient::ClientExt;
use std::collections::HashMap;

const LOCK_TIMEOUT: usize = 60000;

/// Builder for Aragog [`Transaction`]
#[derive(Debug, Default)]
pub struct TransactionBuilder {
    collections: Option<Vec<String>>,
    wait_for_sync: Option<bool>,
    lock_timeout: Option<usize>,
    operation_options: Option<OperationOptions>,
}

impl TransactionBuilder {
    /// Instantiates a new builder for a [`Transaction`]
    #[must_use]
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// The built transaction will be restricted to the specified collection names
    #[must_use]
    #[inline]
    pub fn collections(mut self, collections: Vec<String>) -> Self {
        self.collections = Some(collections);
        self
    }

    /// The built transaction will wait for Database synchronization
    #[must_use]
    #[inline]
    pub const fn wait_for_sync(mut self) -> Self {
        self.wait_for_sync = Some(true);
        self
    }

    /// Defines the transaction lock timeout (default value: 60 000)
    #[must_use]
    #[inline]
    pub const fn lock_timeout(mut self, lock_timeout: usize) -> Self {
        self.lock_timeout = Some(lock_timeout);
        self
    }

    /// Defines custom `write` operation options for this transaction.
    /// By default the options set in the [`DatabaseConnection`] are used.
    ///
    /// [`DatabaseConnection`]: crate::DatabaseConnection
    #[must_use]
    #[inline]
    pub const fn operation_options(mut self, options: OperationOptions) -> Self {
        self.operation_options = Some(options);
        self
    }

    /// Builds the transaction with the database connection
    #[maybe_async::maybe_async]
    pub async fn build<C: ClientExt>(
        self,
        db_connection: &DatabaseConnection<C>,
    ) -> Result<Transaction<C>, Error> {
        let collection_names = self
            .collections
            .unwrap_or_else(|| db_connection.collections_names());
        let accessor = db_connection
            .database()
            .begin_transaction(
                TransactionSettings::builder()
                    .lock_timeout(self.lock_timeout.unwrap_or(LOCK_TIMEOUT))
                    .wait_for_sync(self.wait_for_sync.unwrap_or(false))
                    .collections(
                        TransactionCollections::builder()
                            .write(collection_names)
                            .build(),
                    )
                    .build(),
            )
            .await?;
        log::trace!("Initialized ArangoDB transaction {}", accessor.id());
        let mut collections = HashMap::new();
        for collection in db_connection.collections() {
            let inner_collection = collection.clone_with_transaction(accessor.id().clone())?;
            collections.insert(
                collection.name().to_string(),
                DatabaseCollection::from(inner_collection),
            );
        }
        //
        log::trace!("Initialized Aragog transaction connection");
        let database = db_connection.database().clone();
        let operation_options = self
            .operation_options
            .unwrap_or_else(|| db_connection.operation_options());
        Ok(Transaction {
            accessor,
            database_connection: TransactionDatabaseConnection {
                collections,
                database,
                operation_options,
            },
        })
    }
}
