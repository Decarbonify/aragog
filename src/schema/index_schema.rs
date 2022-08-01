use crate::schema::SchemaDatabaseOperation;
use arangors::index::{Index, IndexSettings};
use arangors::uclient::ClientExt;
use arangors::{ClientError, Database};
use serde::{Deserialize, Serialize};

/// Aragog schema representation of an `ArangoDB` Index.
/// This struct is meant to load/generate the schema file.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IndexSchema {
    /// Index name (must be unique)
    pub name: String,
    /// Collection name
    pub collection: String,
    /// Index fields
    pub fields: Vec<String>,
    /// Index settings
    pub settings: IndexSettings,
}

impl From<IndexSchema> for Index {
    fn from(schema: IndexSchema) -> Self {
        Self::builder()
            .name(schema.name)
            .fields(schema.fields)
            .settings(schema.settings)
            .build()
    }
}

impl IndexSchema {
    /// Retrieve the index id
    #[must_use]
    #[inline]
    pub fn id(&self) -> String {
        format!("{}/{}", &self.collection, &self.name)
    }
}

#[maybe_async::maybe_async]
impl<C: ClientExt + Send> SchemaDatabaseOperation<C> for IndexSchema {
    type PoolType = Index;

    async fn apply_to_database(
        &self,
        database: &Database<C>,
        silent: bool,
    ) -> Result<Option<Self::PoolType>, ClientError> {
        log::debug!("Creating index {}", &self.name);
        let index = self.clone().into();
        Self::handle_pool_result(
            database.create_index(&self.collection, &index).await,
            silent,
        )
    }

    async fn drop(&self, database: &Database<C>) -> Result<(), ClientError> {
        log::debug!("Deleting index {}", &self.name);
        database.delete_index(&self.id()).await?;
        Ok(())
    }

    async fn get(&self, database: &Database<C>) -> Result<Self::PoolType, ClientError> {
        database.index(&self.name).await
    }
}
