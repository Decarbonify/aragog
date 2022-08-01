#![allow(clippy::redundant_pub_crate)]
use std::convert::{TryFrom, TryInto};

use arangors::uclient::ClientExt;

use crate::schema::{DatabaseSchema, SCHEMA_DEFAULT_FILE_NAME, SCHEMA_DEFAULT_PATH};
use crate::{AuthMode, DatabaseConnection, Error, OperationOptions};

#[derive(Debug, Clone)]
pub(crate) struct DbCredentials {
    db_host: String,
    db_name: String,
    db_user: String,
    db_password: String,
}

#[derive(Debug, Clone)]
pub(crate) enum DbCredentialsOption {
    Auto,
    Custom(DbCredentials),
}

#[derive(Debug)]
pub(crate) enum DatabaseSchemaOption {
    Auto,
    Path(String),
    Custom(DatabaseSchema),
}

impl From<DbCredentialsOption> for DbCredentials {
    fn from(option: DbCredentialsOption) -> Self {
        match option {
            DbCredentialsOption::Custom(cred) => cred,
            DbCredentialsOption::Auto => Self {
                db_host: std::env::var("DB_HOST").expect("Please define DB_HOST env var."),
                db_name: std::env::var("DB_NAME").expect("Please define DB_NAME env var."),
                db_user: std::env::var("DB_USER").expect("Please define DB_USER env var."),
                db_password: std::env::var("DB_PASSWORD")
                    .expect("Please define DB_PASSWORD env var."),
            },
        }
    }
}

impl TryFrom<DatabaseSchemaOption> for DatabaseSchema {
    type Error = Error;

    fn try_from(option: DatabaseSchemaOption) -> Result<Self, Self::Error> {
        match option {
            DatabaseSchemaOption::Custom(schema) => Ok(schema),
            DatabaseSchemaOption::Path(path) => Self::load(&path),
            DatabaseSchemaOption::Auto => {
                let schema_path = match std::env::var("SCHEMA_PATH") {
                    Ok(path) => path,
                    Err(_err) => {
                        log::debug!(
                            "Missing SCHEMA_PATH env var, using default value: {}",
                            SCHEMA_DEFAULT_PATH
                        );
                        SCHEMA_DEFAULT_PATH.to_string()
                    }
                };
                Self::load(&format!("{}/{}", schema_path, SCHEMA_DEFAULT_FILE_NAME))
            }
        }
    }
}

/// Builder for `DatabaseConnection`
pub struct DatabaseConnectionBuilder {
    pub(crate) apply_schema: bool,
    pub(crate) auth_mode: AuthMode,
    pub(crate) credentials: DbCredentialsOption,
    pub(crate) schema: DatabaseSchemaOption,
    pub(crate) operation_options: OperationOptions,
}

impl DatabaseConnectionBuilder {
    /// Initializes the Database connection according to specified building methods.
    ///
    /// If nothing was set like in this example:
    /// ```rust
    /// # use aragog::DatabaseConnection;
    /// # #[tokio::main]
    /// # async fn main() {
    /// let db_connection = DatabaseConnection::builder()
    /// # .with_schema_path("tests/schema.yaml")
    /// # .apply_schema()
    ///     .build()
    ///     .await
    ///     .unwrap();
    /// # }
    /// ```
    /// The builder will use the following:
    /// - The schema will be loaded though `SCHEMA_PATH` env var or the default value: `./src/config/db/schema.yaml`
    /// - The credentials will be loaded through the following env vars:
    ///     * `DB_HOST` - The `ArangoDB` host to connect to (`http://localhost:8529` by default or `http://arangodb:8529` on docker containers)
    ///     * `DB_NAME` - The name of the `ArangoDB` database to connect to
    ///     * `DB_USER` - The username of a ArangoDb user with access to the database
    ///     * `DB_PASSWORD` - The password associated with `DB_USER`
    /// - The auth mode will be `AuthMode::Basic`
    ///
    /// # Panics
    ///
    /// If the provided credentials are wrong or if the database is not running the function will panic.
    /// If any of the previous env var is not specified the function will panic with an explanation message.
    #[maybe_async::maybe_async]
    pub async fn build<C: ClientExt + Send>(self) -> Result<DatabaseConnection<C>, Error> {
        let credentials = self.credentials();
        let auth_mode = self.auth_mode();
        let apply_schema = self.apply_schema;
        let operation_options = self.operation_options.clone();
        let schema = self.schema()?;
        let database = DatabaseConnection::connect(
            &credentials.db_host,
            &credentials.db_name,
            &credentials.db_user,
            &credentials.db_password,
            auth_mode,
        )
        .await?;
        DatabaseConnection::new(database, schema, apply_schema, operation_options).await
    }

    /// Specifies a custom authentication mode for `ArangoDB` connection.
    ///
    /// If not specified `AuthMode::Basic` will be used.
    #[must_use]
    #[inline]
    pub fn with_auth_mode(mut self, mode: AuthMode) -> Self {
        log::debug!(
            "[Database Connection Builder] Auth mode {:?} will be used",
            mode
        );
        self.auth_mode = mode;
        self
    }

    /// Specifies custom credentials for `ArangoDB` connection
    ///
    /// # Arguments
    ///
    /// * `db_host` - The `ArangoDB` host to connect to (`http://localhost:8529` by default or `http://arangodb:8529` on docker containers)
    /// * `db_name` - The name of the `ArangoDB` database to connect to
    /// * `db_user` - The username of a `ArangoDB` user with access to the database
    /// * `db_password` - The password associated with `db_user`
    #[must_use]
    #[inline]
    pub fn with_credentials(
        mut self,
        db_host: &str,
        db_name: &str,
        db_user: &str,
        db_password: &str,
    ) -> Self {
        log::debug!(
            "[Database Connection Builder] Custom credentials for `ArangoDB` host {} will be used",
            db_host
        );
        self.credentials = DbCredentialsOption::Custom(DbCredentials {
            db_host: String::from(db_host),
            db_name: String::from(db_name),
            db_user: String::from(db_user),
            db_password: String::from(db_password),
        });
        self
    }

    /// Specifies a custom schema for `ArangoDB` initialization.
    ///
    /// If not specified,`SCHEMA_PATH` env var will be used or the default value: `./src/config/db/schema.yaml`
    #[must_use]
    #[inline]
    pub fn with_schema(mut self, schema: DatabaseSchema) -> Self {
        log::debug!("[Database Connection Builder] Custom schema will be used");
        self.schema = DatabaseSchemaOption::Custom(schema);
        self
    }

    /// Call this method if you want the schema to be applied.
    /// This will ignore any errors, so check the `debug` to find a hidden issue.
    ///
    /// Use it when you use your own custom schema and no `aragog_cli` migrations.
    #[must_use]
    #[inline]
    pub fn apply_schema(mut self) -> Self {
        log::debug!("[Database Connection Builder] Schema will be silently applied");
        self.apply_schema = true;
        self
    }

    /// Specifies a custom schema path for `ArangoDB` initialization.
    ///
    /// If not specified,`SCHEMA_PATH` env var will be used or the default value: `./src/config/db/schema.yaml`
    #[must_use]
    #[inline]
    pub fn with_schema_path(mut self, path: &str) -> Self {
        log::debug!(
            "[Database Connection Builder] Schema from {} will be used",
            path
        );
        self.schema = DatabaseSchemaOption::Path(String::from(path));
        self
    }

    /// Specifies custom options for `write` operations (`create`, `save`, `delete`)
    ///
    /// # Note
    ///
    /// These options will be used globally as a default value, meaning you don't need to specify
    /// duplicate options when using the [`DatabaseRecord`] API.
    ///
    /// If you set `ignore_hooks` here, every [`DatabaseRecord`] operation will skip hooks.
    ///
    /// [`DatabaseRecord`]: crate::DatabaseRecord
    #[must_use]
    #[inline]
    pub fn with_operation_options(mut self, options: OperationOptions) -> Self {
        log::debug!(
            "[Database Connection Builder] custom operation options will be used: {:?}",
            options
        );
        self.operation_options = options;
        self
    }

    #[must_use]
    #[inline]
    fn credentials(&self) -> DbCredentials {
        self.credentials.clone().into()
    }

    fn schema(self) -> Result<DatabaseSchema, Error> {
        self.schema.try_into()
    }

    #[must_use]
    #[inline]
    const fn auth_mode(&self) -> AuthMode {
        self.auth_mode
    }
}
