use std::fmt::{self, Display, Formatter};

use arangors::ClientError;

pub use arango_error::ArangoError;
pub use arango_http_error::ArangoHttpError;
pub use database_error::DatabaseError;
use thiserror::private::AsDynError;

mod arango_error;
mod arango_http_error;
mod database_error;

/// Error enum used for the Arango ORM mapped as potential Http errors
#[derive(Debug)]
pub enum Error {
    /// Unhandled error.
    /// Can be interpreted as a HTTP code `500` internal error.
    InternalError {
        /// Optional message (will not be displayed)
        message: Option<String>,
    },
    /// Validations failed (see model validation as implemented in [`Validate`].
    /// Can be interpreted as a HTTP code `400` bad request.
    ///
    /// [`Validate`]: crate::Validate
    ValidationError(String),
    /// An Item (document or collection) could not be found.
    /// Can be interpreted as a HTTP code `404` not found.
    NotFound {
        /// The missing item
        item: String,
        /// The missing item identifier
        id: String,
        /// Optional database source error
        source: Option<DatabaseError>,
    },
    /// An operation failed due to format or data issue.
    ///
    /// Can be interpreted as a HTTP code `422` Unprocessable Entity.
    UnprocessableEntity {
        /// The source error
        source: Box<dyn std::error::Error>,
    },
    /// The ArangoDb Error as returned by the database host
    ///
    /// Can be interpreted as a HTTP code `500` Internal Error.
    ArangoError(DatabaseError),
    /// A database conflict occured
    ///
    /// Can be interpreted as a HTTP code `409` Conflict.
    Conflict(DatabaseError),
    /// Failed to load config or initialize the app.
    ///
    /// Can be interpreted as a HTTP code `500` Internal Error.
    InitError {
        /// Item that failed to init
        item: String,
        /// Error message
        message: String,
    },
    /// The operation is refused due to lack of authentication.
    /// Can be interpreted as a HTTP code `401` unauthorized.
    Unauthorized(Option<DatabaseError>),
    /// The operation is refused and authentication cannot resolve it.
    /// Can be interpreted as a HTTP code `403` forbidden.
    Forbidden(Option<DatabaseError>),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Error::InternalError { .. } => "Internal Error".to_string(),
                Error::ValidationError(str) => format!("Validations failed: `{}`", str),
                Error::NotFound { item, id, .. } => format!("{} {} not found", item, id),
                Error::UnprocessableEntity { .. } => "Unprocessable Entity".to_string(),
                Error::ArangoError(_) => "ArangoDB Error".to_string(),
                Error::Conflict(_) => "Conflict".to_string(),
                Error::InitError { item, message, .. } =>
                    format!("Failed to initialize `{}`: `{}`", item, message),
                Error::Unauthorized(_) => "Unauthorized".to_string(),
                Error::Forbidden(_) => "Forbidden".to_string(),
            }
        )
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::InternalError { .. } | Error::ValidationError(_) | Error::InitError { .. } => {
                None
            }
            Error::UnprocessableEntity { source } => Some(source.as_ref()),
            Error::ArangoError(e) | Error::Conflict(e) => Some(e),
            Error::Unauthorized(source)
            | Error::Forbidden(source)
            | Error::NotFound { source, .. } => source.as_ref().map(AsDynError::as_dyn_error),
        }
    }
}

impl Error {
    /// get the matching http code
    #[allow(dead_code)]
    #[must_use]
    #[inline]
    pub const fn http_code(&self) -> u16 {
        match self {
            Self::ValidationError(_str) => 400,
            Self::UnprocessableEntity { .. } => 422,
            Self::NotFound { .. } => 404,
            Self::Forbidden(_) => 403,
            Self::Unauthorized(_) => 401,
            Self::ArangoError(_) | Self::InitError { .. } | Self::InternalError { .. } => 500,
            Self::Conflict(_) => 409,
        }
    }
}

impl From<ClientError> for Error {
    fn from(error: ClientError) -> Self {
        log::debug!("Client Error: {}", error);
        match error {
            ClientError::Arango(arango_error) => {
                let arango_error = DatabaseError::from(arango_error);
                match arango_error.http_error {
                    ArangoHttpError::Unauthorized => Self::Unauthorized(Some(arango_error)),
                    ArangoHttpError::Forbidden => Self::Forbidden(Some(arango_error)),
                    ArangoHttpError::Conflict => Self::Conflict(arango_error),
                    _ => Self::ArangoError(arango_error),
                }
            }
            ClientError::Serde(serde_error) => Self::UnprocessableEntity {
                source: Box::new(serde_error),
            },
            ClientError::InvalidServer(server) => Self::InitError {
                item: server,
                message: String::from("Is not ArangoDB"),
            },
            ClientError::InsufficientPermission {
                permission,
                operation,
            } => Self::InitError {
                item: operation.clone(),
                message: format!(
                    "Insufficent permission for {} : {:?}",
                    operation, permission
                ),
            },
            ClientError::HttpClient(error) => Self::InitError {
                item: "Http Client".to_string(),
                message: error.to_string(),
            },
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::UnprocessableEntity {
            source: Box::new(err),
        }
    }
}

impl Default for Error {
    fn default() -> Self {
        Self::InternalError { message: None }
    }
}
