use arangors::{uclient::ClientExt, ClientError, Database};

pub use {
    collection_schema::CollectionSchema, database_schema::DatabaseSchema,
    graph_schema::GraphSchema, index_schema::IndexSchema,
};

mod collection_schema;
mod database_schema;
mod graph_schema;
mod index_schema;

/// Default schema path, can be overridden manually or set as `SCHEMA_PATH` env var
pub const SCHEMA_DEFAULT_PATH: &str = "./src/config/db";
/// Default schema file name, can be overridden manually
pub const SCHEMA_DEFAULT_FILE_NAME: &str = "schema.yaml";

/// Trait used for all schema elements allowing to synchronise schema changes.
/// Used by `aragog_cli` for migrations and `DatabaseConnection`
#[maybe_async::maybe_async]
pub trait SchemaDatabaseOperation<C: ClientExt + Send> {
    /// The `arangors` type to retrieve with the `get` method
    type PoolType;

    /// Utility method to allow "silent" error handling
    fn handle_error<T>(result: Result<T, ClientError>, silent: bool) -> Result<(), ClientError> {
        match result {
            Err(error) => {
                if silent {
                    log::debug!("Ignored error: {}", error);
                    return Ok(());
                }
                Err(error)
            }
            Ok(_val) => Ok(()),
        }
    }

    /// Factorisation of result and error handling for schema operations
    fn handle_pool_result(
        result: Result<Self::PoolType, ClientError>,
        silent: bool,
    ) -> Result<Option<Self::PoolType>, ClientError> {
        let res = match result {
            Err(error) => {
                Self::handle_error(Err(error) as Result<Self::PoolType, ClientError>, silent)?;
                None
            }
            Ok(val) => Some(val),
        };
        Ok(res)
    }

    /// Applies (creates) the schema element to the database
    ///
    /// # Parameters
    /// * `database` - reference the the db connection object (`arangors_lite`)
    /// * `silent` - Should the errors be ignored
    ///
    /// # Returns
    ///
    /// On success the pool type is returned (`Ok(Some(elem))`, but on silenced error nothing is returned (`Ok(None)`)
    async fn apply_to_database(
        &self,
        database: &Database<C>,
        silent: bool,
    ) -> Result<Option<Self::PoolType>, ClientError>;

    /// Deletes the schema element from the database.
    async fn drop(&self, database: &Database<C>) -> Result<(), ClientError>;

    /// Retrieves the `arangors_lite` element from the schema element
    async fn get(&self, database: &Database<C>) -> Result<Self::PoolType, ClientError>;
}
