use std::fs;

use arangors::uclient::ClientExt;
use arangors::{ClientError, Database};
use serde::{Deserialize, Serialize};

use crate::schema::{CollectionSchema, GraphSchema, IndexSchema, SchemaDatabaseOperation};
use crate::Error;

/// Aragog schema representation of an `ArangoDB` Database.
/// This struct is meant to load/generate the schema file.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DatabaseSchema {
    /// Schema version
    pub version: Option<u64>,
    /// Database collections
    pub collections: Vec<CollectionSchema>,
    /// Database Collection Indexes
    #[serde(skip_serializing_if = "Vec::is_empty", default = "Vec::new")]
    pub indexes: Vec<IndexSchema>,
    /// Database named graphs
    #[serde(skip_serializing_if = "Vec::is_empty", default = "Vec::new")]
    pub graphs: Vec<GraphSchema>,
}

impl DatabaseSchema {
    /// Find a Collection index from the schema instance
    #[must_use]
    pub fn collection_index(&self, name: &str) -> Option<usize> {
        self.collections.iter().position(|c| c.name == name)
    }

    /// Find a Collection from the schema instance
    #[must_use]
    pub fn collection(&self, name: &str) -> Option<&CollectionSchema> {
        self.collections.iter().find(|c| c.name == name)
    }

    /// Find an index index from the schema instance
    #[must_use]
    pub fn index_index(&self, collection: &str, name: &str) -> Option<usize> {
        self.indexes
            .iter()
            .position(|c| c.name == name && c.collection == collection)
    }

    /// Find an Index from the schema instance
    #[must_use]
    pub fn index(&self, collection: &str, name: &str) -> Option<&IndexSchema> {
        self.indexes
            .iter()
            .find(|c| c.name == name && c.collection == collection)
    }

    /// Find an index index from the schema instance
    #[must_use]
    pub fn graph_index(&self, name: &str) -> Option<usize> {
        self.graphs.iter().position(|c| c.0.name == name)
    }

    /// Find an Index from the schema instance
    #[must_use]
    pub fn graph(&self, name: &str) -> Option<&GraphSchema> {
        self.graphs.iter().find(|c| c.0.name == name)
    }

    /// Loads the YAML schema from the give `path`
    ///
    /// # Errors
    ///
    /// Will fail on wrong file path, file ACLs or content
    pub fn load(path: &str) -> Result<Self, Error> {
        let file = match fs::read_to_string(&path) {
            Ok(val) => val,
            Err(error) => {
                return Err(Error::InitError {
                    item: path.to_string(),
                    message: error.to_string(),
                });
            }
        };
        let value: Self = match serde_yaml::from_str(&file) {
            Ok(val) => val,
            Err(error) => {
                return Err(Error::InitError {
                    item: path.to_string(),
                    message: error.to_string(),
                });
            }
        };
        Ok(value)
    }
}

#[maybe_async::maybe_async]
impl<C: ClientExt + Send> SchemaDatabaseOperation<C> for DatabaseSchema {
    type PoolType = ();

    async fn apply_to_database(
        &self,
        database: &Database<C>,
        silent: bool,
    ) -> Result<Option<Self::PoolType>, ClientError> {
        for item in &self.collections {
            Self::handle_error(item.apply_to_database(database, silent).await, silent)?;
        }
        for item in &self.indexes {
            Self::handle_error(item.apply_to_database(database, silent).await, silent)?;
        }
        for item in &self.graphs {
            Self::handle_error(item.apply_to_database(database, silent).await, silent)?;
        }
        Ok(Some(()))
    }

    async fn drop(&self, database: &Database<C>) -> Result<(), ClientError> {
        for item in &self.collections {
            item.drop(database).await?;
        }
        for item in &self.indexes {
            item.drop(database).await?;
        }
        for item in &self.graphs {
            item.drop(database).await?;
        }
        Ok(())
    }

    async fn get(&self, _database: &Database<C>) -> Result<Self::PoolType, ClientError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use arangors::graph::{EdgeDefinition, Graph, GraphOptions};
    use arangors::index::IndexSettings;

    use crate::schema::IndexSchema;

    use super::*;

    fn schema() -> DatabaseSchema {
        DatabaseSchema {
            version: None,
            collections: vec![
                CollectionSchema {
                    name: "collectionA".to_string(),
                    is_edge_collection: false,
                    wait_for_sync: None,
                },
                CollectionSchema {
                    name: "collectionB".to_string(),
                    is_edge_collection: false,
                    wait_for_sync: Some(true),
                },
                CollectionSchema {
                    name: "edgeCollectionA".to_string(),
                    is_edge_collection: true,
                    wait_for_sync: None,
                },
            ],
            indexes: vec![
                IndexSchema {
                    name: "OnUsername".to_string(),
                    collection: "CollectionA".to_string(),
                    fields: vec!["username".to_string()],
                    settings: IndexSettings::Persistent {
                        unique: true,
                        sparse: false,
                        deduplicate: false,
                    },
                },
                IndexSchema {
                    name: "OnAgeAndemail".to_string(),
                    collection: "CollectionB".to_string(),
                    fields: vec!["age".to_string(), "email".to_string()],
                    settings: IndexSettings::Ttl { expire_after: 3600 },
                },
            ],
            graphs: vec![GraphSchema(Graph {
                name: "namedGraph".to_string(),
                edge_definitions: vec![EdgeDefinition {
                    collection: "edgeCollection1".to_string(),
                    from: vec!["collectionA".to_string()],
                    to: vec!["collectionB".to_string(), "collectionC".to_string()],
                }],
                orphan_collections: vec![],
                is_smart: None,
                is_disjoint: None,
                options: Some(GraphOptions {
                    smart_graph_attribute: None,
                    number_of_shards: None,
                    replication_factor: Some(10),
                    write_concern: None,
                }),
            })],
        }
    }

    #[test]
    fn serialization_works() {
        let schema = schema();
        serde_yaml::to_string(&schema).unwrap();
    }
}
