use crate::schema::SchemaDatabaseOperation;
use arangors_lite::graph::Graph;
use arangors_lite::{ClientError, Database};
use serde::{Deserialize, Serialize};

/// Aragog schema representation of an `ArangoDB` Named Graph.
/// This struct is meant to load/generate the schema file.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GraphSchema(pub Graph);

impl From<GraphSchema> for Graph {
    fn from(schema: GraphSchema) -> Self {
        schema.0
    }
}

#[maybe_async::maybe_async]
impl SchemaDatabaseOperation for GraphSchema {
    type PoolType = Graph;

    async fn apply_to_database(
        &self,
        database: &Database,
        silent: bool,
    ) -> Result<Option<Self::PoolType>, ClientError> {
        log::debug!("Creating Graph {}", &self.0.name);
        let graph = self.clone().into();
        Self::handle_pool_result(database.create_graph(graph, true).await, silent)
    }

    async fn drop(&self, database: &Database) -> Result<(), ClientError> {
        log::debug!("Deleting Graph {}", &self.0.name);
        database.drop_graph(&self.0.name, false).await?;
        Ok(())
    }

    async fn get(&self, database: &Database) -> Result<Self::PoolType, ClientError> {
        database.graph(&self.0.name).await
    }
}
