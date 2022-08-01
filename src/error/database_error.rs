use crate::error::{ArangoError, ArangoHttpError};
use arangors::ArangoError as DriverError;
use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

/// Mapped Arango db error response
#[derive(Debug, Clone)]
pub struct DatabaseError {
    /// The mapped Arango HTTP Error
    pub http_error: ArangoHttpError,
    /// The mapped Arango Error
    pub arango_error: ArangoError,
    /// The error message
    pub message: String,
}

impl Display for DatabaseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "code: {}\nerror_num: {}\nmessage: {}",
            self.http_error, self.arango_error, self.message
        )
    }
}

impl Error for DatabaseError {}

impl From<DriverError> for DatabaseError {
    fn from(error: DriverError) -> Self {
        Self {
            http_error: ArangoHttpError::from_code(error.code()),
            arango_error: ArangoError::from_error_num(error.error_num()),
            message: error.message().to_string(),
        }
    }
}
