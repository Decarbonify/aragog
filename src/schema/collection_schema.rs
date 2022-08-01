use arangors::collection::options::CreateParameters;
use arangors::uclient::ClientExt;
use arangors::{
    collection::{options::CreateOptions, Collection, CollectionType},
    ClientError, Database,
};
use serde::{Deserialize, Serialize};

use crate::schema::SchemaDatabaseOperation;

/// Aragog schema representation of an `ArangoDB` Collection.
/// This struct is meant to load/generate the schema file.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CollectionSchema {
    /// Collection name
    pub name: String,
    /// Defines if the collection a edge collection
    pub is_edge_collection: bool,
    /// Defines if the collection requests wait for the operations to be written on disk
    ///
    /// If set on `true` the requests might be slower. By default, `false` is used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wait_for_sync: Option<bool>,
}

impl CollectionSchema {
    /// Initializes a new collection schema with the collection name and a flag to define if
    /// it's an edge collection or not.
    #[must_use]
    #[inline]
    pub fn new(name: &str, is_edge_collection: bool, wait_for_sync: Option<bool>) -> Self {
        Self {
            name: name.to_string(),
            is_edge_collection,
            wait_for_sync,
        }
    }
}

#[maybe_async::maybe_async]
impl<C: ClientExt + Send> SchemaDatabaseOperation<C> for CollectionSchema {
    type PoolType = Collection<C>;

    async fn apply_to_database(
        &self,
        database: &Database<C>,
        silent: bool,
    ) -> Result<Option<Self::PoolType>, ClientError> {
        log::debug!("Creating Collection {}", &self.name);
        let creation_settings = CreateOptions::builder()
            .name(&self.name)
            .collection_type(if self.is_edge_collection {
                CollectionType::Edge
            } else {
                CollectionType::Document
            })
            .wait_for_sync(true)
            .build();
        let res = database
            .create_collection_with_options(creation_settings, CreateParameters::default())
            .await;
        Self::handle_pool_result(res, silent)
    }

    async fn drop(&self, database: &Database<C>) -> Result<(), ClientError> {
        log::debug!("Deleting Collection {}", &self.name);
        database.drop_collection(&self.name).await?;
        Ok(())
    }

    async fn get(&self, database: &Database<C>) -> Result<Self::PoolType, ClientError> {
        database.collection(&self.name).await
    }
}
