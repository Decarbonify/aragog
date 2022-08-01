use std::ops::Deref;

use arangors::{
    document::options::{InsertOptions, ReplaceOptions},
    Collection, Connection, Database,
};
use serde::{Deserialize, Serialize};

use aragog::schema::DatabaseSchema;

use crate::{
    config::Config, error::AragogCliError, log, log_level::LogLevel, migration::MigrationVersion,
};

const SCHEMA_DOC_KEY: &str = "DatabaseSchema";

#[derive(Serialize, Deserialize)]
struct SchemaWithKey {
    pub _key: String,
    pub version: Option<MigrationVersion>,
    pub collections: Vec<String>,
}

#[derive(Debug)]
pub struct VersionedDatabase {
    pub db: Database,
    pub schema_collection: Collection,
    pub schema: DatabaseSchema,
}

impl VersionedDatabase {
    pub fn init(config: &Config) -> Result<Self, AragogCliError> {
        log(
            format!("Establishing connection with {}", &config.db_host),
            LogLevel::Verbose,
        );
        let connection =
            Connection::establish_basic_auth(&config.db_host, &config.db_user, &config.db_pwd)?;
        log(
            format!("Connecting to database {}", &config.db_name),
            LogLevel::Verbose,
        );
        match connection.accessible_databases() {
            Ok(map) => {
                log(format!("Available databases: {:?}", map), LogLevel::Debug);
            }
            Err(e) => log(
                format!("Failed to retrieve accessible databases: {}", e),
                LogLevel::Info,
            ),
        };
        let db: Database = match connection.db(&config.db_name) {
            Ok(val) => val,
            Err(e) => {
                log(
                    format!(
                        "Failed to connect to database {}:\nerror: {}, \nTrying to create it...",
                        &config.db_name, e
                    ),
                    LogLevel::Info,
                );
                let res = connection.create_database(&config.db_name)?;
                log("Done", LogLevel::Info);
                res
            }
        };
        log(
            format!("Retrieving collection {}", &config.schema_collection_name),
            LogLevel::Verbose,
        );
        let schema_collection = match db.collection(&config.schema_collection_name) {
            Ok(coll) => coll,
            Err(_error) => {
                log(
                    format!(
                        "Missing collection {}, creating it...",
                        &config.schema_collection_name
                    ),
                    LogLevel::Debug,
                );
                db.create_collection(&config.schema_collection_name)?
            }
        };
        log("Retrieving Schema document", LogLevel::Verbose);
        let schema = match schema_collection.document(SCHEMA_DOC_KEY) {
            Ok(doc) => doc.document,
            Err(_err) => {
                log(
                    "Missing database schema document, creating it..",
                    LogLevel::Debug,
                );
                let schema = DatabaseSchema::default();
                let doc = SchemaWithKey {
                    _key: SCHEMA_DOC_KEY.to_string(),
                    version: schema.version,
                    collections: vec![],
                };
                schema_collection
                    .create_document(doc, InsertOptions::builder().wait_for_sync(true).build())?;
                DatabaseSchema::default()
            }
        };
        Ok(Self {
            db,
            schema_collection,
            schema,
        })
    }

    pub fn save(&self) -> Result<(), AragogCliError> {
        log(
            format!(
                "Saving schema version {} to database",
                self.schema_version()
            ),
            LogLevel::Verbose,
        );
        self.schema_collection.replace_document(
            SCHEMA_DOC_KEY,
            self.schema.clone(),
            ReplaceOptions::builder().wait_for_sync(true).build(),
            None,
        )?;
        Ok(())
    }

    pub fn schema_version(&self) -> MigrationVersion {
        self.schema.version.unwrap_or(0)
    }
}

impl Deref for VersionedDatabase {
    type Target = Database;

    fn deref(&self) -> &Self::Target {
        &self.db
    }
}
