use crate::{
    config::Config, error::AragogCliError, migration::Migration,
    migration_operation::MigrationOperation, versioned_database::VersionedDatabase,
};
use arangors::{
    collection::{response::Info, CollectionType},
    index::IndexSettings,
};

pub fn discover_migration(
    db: &mut VersionedDatabase,
    config: &Config,
) -> Result<Migration, AragogCliError> {
    let collections: Vec<Info> = db.accessible_collections()?;
    let collections_to_create: Vec<(String, bool)> = collections
        .into_iter()
        .filter(|i| {
            db.schema.collection_index(&i.name).is_none()
                && !i.is_system
                && i.name != config.schema_collection_name
        })
        .map(|i| (i.name, matches!(i.collection_type, CollectionType::Edge)))
        .collect();
    let mut migration = Migration::new("discover_migration", &config.schema_path, false)?;

    let (mut operations_up, mut operations_down) = collections_to_create.iter().try_fold(
        (vec![], vec![]),
        |(mut up, mut down), (name, is_edge)| {
            if *is_edge {
                up.push(MigrationOperation::CreateEdgeCollection {
                    name: name.clone(),
                    wait_for_sync: None,
                });
                down.push(MigrationOperation::DeleteEdgeCollection { name: name.clone() });
            } else {
                up.push(MigrationOperation::CreateCollection {
                    name: name.clone(),
                    wait_for_sync: None,
                });
                down.push(MigrationOperation::DeleteCollection { name: name.clone() });
            }
            for index in db.indexes(name)?.indexes {
                if let IndexSettings::Primary { .. } | IndexSettings::Edge { .. } = index.settings {
                    continue;
                }
                up.push(MigrationOperation::CreateIndex {
                    name: index.name.clone(),
                    collection: name.clone(),
                    fields: index.fields,
                    settings: index.settings,
                });
                down.insert(
                    0,
                    MigrationOperation::DeleteIndex {
                        name: index.name,
                        collection: name.clone(),
                    },
                )
            }
            Result::<_, AragogCliError>::Ok((up, down))
        },
    )?;
    for graph in db.graphs()?.graphs {
        if db.schema.graph_index(&graph.name).is_none() {
            operations_up.push(MigrationOperation::CreateGraph {
                name: graph.name.clone(),
                edge_definitions: graph.edge_definitions,
                orphan_collections: Some(graph.orphan_collections),
                is_smart: graph.is_smart,
                is_disjoint: graph.is_disjoint,
                options: graph.options,
            });
            operations_down.push(MigrationOperation::DeleteGraph { name: graph.name })
        }
    }
    migration.data.up = operations_up;
    migration.data.down = Some(operations_down);
    migration.save()?;
    Ok(migration)
}
