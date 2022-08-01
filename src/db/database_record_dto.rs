#![allow(clippy::used_underscore_binding)]
use crate::{DatabaseRecord, Error, Record};
use arangors::document::response::DocumentResponse;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

#[derive(Serialize, Deserialize)]
pub struct DatabaseRecordDto<T> {
    #[serde(rename = "_key")]
    #[serde(skip_serializing_if = "Option::is_none")]
    key: Option<String>,
    #[serde(flatten)]
    pub record: T,
}

impl<T: Record> DatabaseRecordDto<T> {
    #[inline]
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Can't be const in 1.56
    pub fn new(record: T, key: Option<String>) -> Self {
        Self { key, record }
    }
}

impl<T: Record> TryInto<DatabaseRecord<T>> for DocumentResponse<DatabaseRecord<T>> {
    type Error = Error;

    fn try_into(self) -> Result<DatabaseRecord<T>, Self::Error> {
        match self {
            Self::Silent => Err(Error::InternalError {
                message: Some(String::from("Received unexpected silent document response")),
            }),
            Self::Response { new, header, .. } => match new {
                Some(value) => Ok(value),
                None => Err(Error::InternalError {
                    message: Some(format!(
                        "Expected `ArangoDB` to return the new {} document",
                        header._id
                    )),
                }),
            },
        }
    }
}

impl<T: Record> TryInto<DatabaseRecord<T>> for DocumentResponse<DatabaseRecordDto<T>> {
    type Error = Error;

    fn try_into(self) -> Result<DatabaseRecord<T>, Self::Error> {
        match self {
            Self::Silent => Err(Error::InternalError {
                message: Some(String::from("Received unexpected silent document response")),
            }),
            Self::Response { header, new, .. } => {
                let record = match new {
                    Some(doc) => doc.record,
                    None => {
                        return Err(Error::InternalError {
                            message: Some(format!(
                                "Expected `ArangoDB` to return the new {} document",
                                header._id
                            )),
                        });
                    }
                };
                Ok(DatabaseRecord {
                    key: header._key.clone(),
                    id: header._id.clone(),
                    rev: header._rev,
                    record,
                })
            }
        }
    }
}
