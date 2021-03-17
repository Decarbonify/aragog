use crate::{DatabaseRecord, Record, ServiceError};
use serde_json::Value;

pub trait QueryResult {}

/// Typed Query result
#[derive(Debug)]
pub struct RecordQueryResult<T: Record> {
    /// Vector of the returned documents
    pub documents: Vec<DatabaseRecord<T>>,
    /// The total `documents` count
    doc_count: usize,
}

/// Result of a succeeded [`Query`]. Contains a `Vec` of `serde_json`::`Value`.
/// The structure has methods to retrieve typed models (`get_records`).
///
/// [`Query`]: struct.Query.html
#[derive(Debug)]
pub struct JsonQueryResult {
    /// Vector of the returned documents
    pub documents: Vec<Value>,
    /// The total `documents` count
    doc_count: usize,
}

impl<T: Record> RecordQueryResult<T> {
    /// Instantiates a new `QueryResult` from a document collection
    pub fn new(documents: Vec<DatabaseRecord<T>>) -> Self {
        Self {
            doc_count: documents.len(),
            documents,
        }
    }

    /// Returns the only document of the current `QueryResult`.
    /// If there is no document or more than one, a [`ServiceError`]::[`NotFound`] is returned.
    ///
    /// [`ServiceError`]: enum.ServiceError.html
    /// [`NotFound`]: enum.ServiceError.html#variant.NotFound
    pub fn uniq(self) -> Result<DatabaseRecord<T>, ServiceError> {
        if self.is_empty() || self.doc_count > 1 {
            log::error!(
                "Wrong number of {} returned: {}",
                T::collection_name(),
                self.doc_count
            );
            return Err(ServiceError::NotFound {
                item: T::collection_name().to_string(),
                id: "queried".to_string(),
                source: None,
            });
        }
        Ok(self.documents.into_iter().next().unwrap())
    }

    /// Returns the first document of the current `QueryResult`.
    /// Returns `None` if there are no documents
    pub fn first(self) -> Option<DatabaseRecord<T>> {
        if self.is_empty() {
            return None;
        }
        Some(self.documents.into_iter().next().unwrap())
    }

    /// Returns the last document of the current `QueryResult`.
    /// Returns `None` if there are no documents
    pub fn last(self) -> Option<DatabaseRecord<T>> {
        if self.is_empty() {
            return None;
        }
        Some(self.documents.into_iter().nth(self.doc_count - 1).unwrap())
    }

    /// Returns the length of `documents`
    pub fn len(&self) -> usize {
        self.doc_count
    }

    /// Returns `true` if no documents were returned
    pub fn is_empty(&self) -> bool {
        self.doc_count == 0
    }
}

impl JsonQueryResult {
    /// Instantiates a new `JsonQueryResult` from a document collection
    pub fn new(documents: Vec<Value>) -> Self {
        Self {
            doc_count: documents.len(),
            documents,
        }
    }

    /// Retrieves deserialized documents from the json results. The documents not matching `T` will not be returned.
    ///
    /// # Example
    /// If you want to do a graph query that can return different models you can use this method to retrieve the serialized record:
    /// ```rust no_run
    /// # use aragog::{query::Query, Record, DatabaseConnectionPool};
    /// # use serde::{Serialize, Deserialize};
    /// #
    /// # #[derive(Record, Clone, Serialize, Deserialize)]
    /// # struct User {}
    /// # #[derive(Record, Clone, Serialize, Deserialize)]
    /// # struct Topic {}
    /// # #[derive(Record, Clone, Serialize, Deserialize)]
    /// # struct Role {}
    /// # #[tokio::main]
    /// # async fn main() {
    /// # let db_accessor = DatabaseConnectionPool::builder().build().await.unwrap();
    /// let json_results = Query::outbound(1, 5, "ChildOf", "User/123").call(&db_accessor).await.unwrap();
    ///
    /// let user_results = json_results.get_records::<User>();
    /// let topic_results = json_results.get_records::<Topic>();
    /// let role_results = json_results.get_records::<Role>();
    /// # }
    /// ```
    pub fn get_records<T: Record>(&self) -> Vec<DatabaseRecord<T>> {
        let mut res = Vec::new();
        for value in self.documents.iter() {
            let record = serde_json::from_value(value.clone());
            if let Ok(db_record) = record {
                res.push(db_record);
            } else {
                continue;
            }
        }
        res
    }

    /// Returns the length of `documents`
    pub fn len(&self) -> usize {
        self.doc_count
    }

    /// Returns `true` if no documents were returned
    pub fn is_empty(&self) -> bool {
        self.doc_count == 0
    }
}

impl<T: Record> From<JsonQueryResult> for RecordQueryResult<T> {
    fn from(query_result: JsonQueryResult) -> Self {
        Self::new(query_result.get_records())
    }
}
