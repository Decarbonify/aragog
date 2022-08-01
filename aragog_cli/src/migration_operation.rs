use arangors::{
    graph::{EdgeDefinition, Graph, GraphOptions},
    index::IndexSettings,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use aragog::schema::{CollectionSchema, GraphSchema, IndexSchema, SchemaDatabaseOperation};

use crate::{error::AragogCliError, log, log_level::LogLevel, VersionedDatabase};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationOperation {
    CreateCollection {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        wait_for_sync: Option<bool>,
    },
    DeleteCollection {
        name: String,
    },
    CreateEdgeCollection {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        wait_for_sync: Option<bool>,
    },
    DeleteEdgeCollection {
        name: String,
    },
    CreateIndex {
        name: String,
        collection: String,
        fields: Vec<String>,
        settings: IndexSettings,
    },
    DeleteIndex {
        name: String,
        collection: String,
    },
    CreateGraph {
        name: String,
        edge_definitions: Vec<EdgeDefinition>,
        #[serde(skip_serializing_if = "Option::is_none")]
        orphan_collections: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_smart: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_disjoint: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        options: Option<GraphOptions>,
    },
    DeleteGraph {
        name: String,
    },
    Aql(String),
}

impl MigrationOperation {
    pub fn apply(self, db: &mut VersionedDatabase, silent: bool) -> Result<(), AragogCliError> {
        match self {
            MigrationOperation::CreateCollection {
                name,
                wait_for_sync,
            } => {
                log(
                    format!("Executing create_collection `{}` operation", name),
                    LogLevel::Verbose,
                );
                let item = match db.schema.collection(&name) {
                    Some(_) => return Err(AragogCliError::DuplicateCollection { name }),
                    None => CollectionSchema::new(&name, false, wait_for_sync),
                };
                item.apply_to_database(db, silent)?;
                db.schema.collections.push(item);
            }
            MigrationOperation::CreateEdgeCollection {
                name,
                wait_for_sync,
            } => {
                log(
                    format!("Executing create_edge_collection `{}` operation", name),
                    LogLevel::Verbose,
                );
                let item = match db.schema.collection(&name) {
                    Some(_) => return Err(AragogCliError::DuplicateEdgeCollection { name }),
                    None => CollectionSchema::new(&name, true, wait_for_sync),
                };
                item.apply_to_database(db, silent)?;
                db.schema.collections.push(item);
            }
            MigrationOperation::DeleteCollection { name } => {
                log(
                    format!("Executing delete_collection `{}` operation", name),
                    LogLevel::Verbose,
                );
                match db.schema.collection_index(&name) {
                    None => return Err(AragogCliError::MissingCollection { name }),
                    Some(index) => {
                        let item = db.schema.collections.remove(index);
                        item.drop(db)?;
                    }
                }
            }
            MigrationOperation::DeleteEdgeCollection { name } => {
                log(
                    format!("Executing delete_edge_collection `{}` operation", name),
                    LogLevel::Verbose,
                );
                match db.schema.collection_index(&name) {
                    None => return Err(AragogCliError::MissingEdgeCollection { name }),
                    Some(index) => {
                        let item = db.schema.collections.remove(index);
                        item.drop(db)?;
                    }
                }
            }
            MigrationOperation::CreateIndex {
                collection,
                name,
                settings,
                fields,
            } => {
                log(
                    format!("Executing create_index `{}` operation", name),
                    LogLevel::Verbose,
                );
                let item = match db.schema.index(&collection, &name) {
                    Some(_) => return Err(AragogCliError::DuplicateIndex { name, collection }),
                    None => IndexSchema {
                        name,
                        collection,
                        fields,
                        settings,
                    },
                };
                item.apply_to_database(db, false)?;
                db.schema.indexes.push(item);
            }
            MigrationOperation::DeleteIndex { name, collection } => {
                log(
                    format!("Executing delete_index `{}` operation", name),
                    LogLevel::Verbose,
                );
                match db.schema.index_index(&collection, &name) {
                    None => return Err(AragogCliError::MissingIndex { collection, name }),
                    Some(index) => {
                        let item = db.schema.indexes.remove(index);
                        item.drop(db)?;
                    }
                }
            }
            MigrationOperation::CreateGraph {
                name,
                edge_definitions,
                orphan_collections,
                is_smart,
                is_disjoint,
                options,
            } => {
                log(
                    format!("Executing create_graph `{}` operation", name),
                    LogLevel::Verbose,
                );
                let item = match db.schema.graph(&name) {
                    Some(_) => return Err(AragogCliError::DuplicateGraph { name }),
                    None => GraphSchema(Graph {
                        name,
                        edge_definitions,
                        orphan_collections: orphan_collections.unwrap_or_default(),
                        is_smart,
                        is_disjoint,
                        options,
                    }),
                };
                item.apply_to_database(db, silent)?;
                db.schema.graphs.push(item);
            }
            MigrationOperation::DeleteGraph { name } => {
                log(
                    format!("Executing delete_graph `{}` operation", name),
                    LogLevel::Verbose,
                );
                match db.schema.graph_index(&name) {
                    None => return Err(AragogCliError::MissingGraph { name }),
                    Some(graph) => {
                        let item = db.schema.graphs.remove(graph);
                        item.drop(db)?;
                    }
                }
            }
            MigrationOperation::Aql(aql) => {
                log("Executing aql operation", LogLevel::Verbose);
                let res: Vec<Value> = db.aql_str(aql.as_str())?;
                log(format!("{:?}", res), LogLevel::Verbose);
            }
        };
        Ok(())
    }
}
