use arangors::ClientError;
use exitcode::ExitCode;
use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AragogCliError {
    #[error("Missing Collection: {name}")]
    MissingCollection { name: String },
    #[error("Duplicate Collection: {name}")]
    DuplicateCollection { name: String },
    #[error("Missing Edge Collection: {name}")]
    MissingEdgeCollection { name: String },
    #[error("Duplicate Edge Collection: {name}")]
    DuplicateEdgeCollection { name: String },
    #[error("Missing Index: {name} on collection {collection}")]
    MissingIndex { name: String, collection: String },
    #[error("Duplicate Index: {name} on collection {collection}")]
    DuplicateIndex { name: String, collection: String },
    #[error("Missing Graph: {name}")]
    MissingGraph { name: String },
    #[error("Duplicate Graph: {name}")]
    DuplicateGraph { name: String },
    #[error("Invalid File Name: {file_name}")]
    InvalidFileName { file_name: String },
    #[error("I/O Error: {message}")]
    IOError { message: String },
    #[error("Parsing Error: {message}")]
    ParsingError { message: String },
    #[error("Failed to initialize {item} ({message})")]
    InitError { item: String, message: String },
    #[error("Arango Error: {0}")]
    ClientError(ClientError),
}

impl From<ClientError> for AragogCliError {
    fn from(error: ClientError) -> Self {
        Self::ClientError(error)
    }
}

impl From<io::Error> for AragogCliError {
    fn from(error: io::Error) -> Self {
        Self::IOError {
            message: error.to_string(),
        }
    }
}

impl From<serde_yaml::Error> for AragogCliError {
    fn from(error: serde_yaml::Error) -> Self {
        Self::ParsingError {
            message: error.to_string(),
        }
    }
}

impl AragogCliError {
    pub const fn exit_code(&self) -> ExitCode {
        match self {
            Self::InvalidFileName { .. } | Self::ParsingError { .. } => exitcode::DATAERR,
            Self::IOError { .. } => exitcode::IOERR,
            _ => exitcode::USAGE,
        }
    }
}
