use arangors::collection::response::Properties;

use crate::{config::Config, error::AragogCliError, versioned_database::VersionedDatabase};

pub fn describe_db(config: &Config) -> Result<(), AragogCliError> {
    let db = VersionedDatabase::init(config)?;
    println!("\nDescription of {}: \n", db.name());
    match db.schema.version {
        Some(version) => println!("- Database Schema version: {}", version),
        None => println!("- Database Schema is not versioned yet (use migrate)"),
    };
    println!("- Database Graph count: {}", db.graphs()?.graphs.len());
    let mut table = table!([
        "Name",
        "Type",
        "Doc Count",
        "Index Count",
        "Wait for Sync",
        "In Schema"
    ]);
    for info in db.accessible_collections()?.iter() {
        if info.is_system {
            continue;
        }
        let collection = db.collection(&info.name)?;
        let properties: Properties = collection.document_count()?;
        let synced = db.schema.collections.iter().any(|a| a.name == info.name);
        let index_count = db.indexes(&info.name)?.indexes.len();
        table.add_row(row![
            &info.name,
            format!("{:?}", &info.collection_type),
            &properties.info.count.unwrap_or(0),
            index_count,
            &properties.detail.wait_for_sync,
            synced
        ]);
    }
    table.printstd();
    Ok(())
}

pub fn describe_collection_indexes(
    config: &Config,
    collection_name: &str,
) -> Result<(), AragogCliError> {
    let db = VersionedDatabase::init(config)?;
    db.collection(collection_name)?;
    println!(
        "\nDescription of {} collection {} indexes: \n",
        db.name(),
        collection_name
    );
    let mut table = table!(["Name", "id", "Fields", "Settings"]);
    for index in db.indexes(collection_name)?.indexes.iter() {
        table.add_row(row![
            index.name,
            index.id,
            index.fields.join(", "),
            format!("{:?}", index.settings)
        ]);
    }
    table.printstd();
    Ok(())
}
