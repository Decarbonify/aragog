use arangors::uclient::ClientExt;
use arangors::{Cursor, Database};

use crate::query::QueryResult;
use crate::{DatabaseRecord, Record};

/// Results of AQL query as a cursor in order to batch the communication between server and client.
///
/// # Relevant methods:
/// - `next_batch` to move the cursor to the next batch
/// - `has_more` to check if the current batch is the final one
/// - `result` to get the query result of the current batch.
///
/// # Example
///
/// ```rust
/// # use aragog::query::{Comparison, Filter, QueryResult, QueryCursor};
/// # use serde::{Serialize, Deserialize};
/// # use aragog::{DatabaseConnection, Record, DatabaseRecord};
/// #
/// # #[derive(Record, Clone, Serialize, Deserialize)]
/// # struct User {
/// #    username: String,
/// #    age: u16,
/// # }
///
/// fn handle_result(result: QueryResult<User>) {
///     // Handle query results
/// }
///
/// # #[tokio::main]
/// # async fn main() {
/// # let db_accessor = DatabaseConnection::builder()
/// #     .with_schema_path("tests/schema.yaml")
/// #     .apply_schema()
/// #     .build().await.unwrap();
/// # db_accessor.truncate();
/// # DatabaseRecord::create(User {username: "RobertSurcouf".to_string() ,age: 18 }, &db_accessor).await.unwrap();
/// // Define a query
/// let query = User::query().filter(Filter::new(Comparison::field("age").greater_than(10)));
/// // Retrieve a cursor
/// let mut cursor = User::get_in_batches(&query, &db_accessor, 100).await.unwrap();
///
/// // Retrieve the first result of the query
/// handle_result(cursor.result());
/// // Iterate through the batches
/// while let Some(result) = cursor.next_batch().await {
///     handle_result(result);
/// }
/// # }
/// ```
#[derive(Debug)]
pub struct QueryCursor<T, C: ClientExt> {
    pub(crate) cursor: Cursor<DatabaseRecord<T>>,
    pub(crate) database: Database<C>,
    #[cfg(feature = "blocking")]
    pending_result: Option<QueryResult<T>>,
}

impl<T, C> QueryCursor<T, C>
where
    T: Record,
    C: ClientExt,
{
    #[must_use]
    #[inline]
    #[allow(clippy::missing_const_for_fn)]
    pub(crate) fn new(cursor: Cursor<DatabaseRecord<T>>, database: Database<C>) -> Self {
        Self {
            #[cfg(feature = "blocking")]
            pending_result: Some(cursor.result.clone().into()),
            cursor,
            database,
        }
    }

    /// Get the current cursor result
    #[must_use]
    #[inline]
    pub fn result(&self) -> QueryResult<T> {
        self.cursor.result.clone().into()
    }

    /// Does the cursor have more batches
    #[must_use]
    #[inline]
    #[allow(clippy::missing_const_for_fn)]
    pub fn has_more(&self) -> bool {
        self.cursor.more
    }

    /// Total number of documents that matched the search condition
    #[must_use]
    pub fn full_count(&self) -> Option<usize> {
        self.cursor.extra.as_ref()?.stats.as_ref()?.full_count
    }

    /// Moves the cursor to the next batch and returns the result
    #[maybe_async::maybe_async]
    pub async fn next_batch(&mut self) -> Option<QueryResult<T>> {
        if !self.has_more() {
            return None;
        }
        if let Some(ref id) = self.cursor.id {
            self.cursor = match self.database.aql_next_batch(id).await {
                Ok(cursor) => cursor,
                Err(error) => {
                    log::error!("Failed to get next batch: {}", error);
                    return None;
                }
            };
            Some(self.result())
        } else {
            log::error!("No `id` associated to Aql Cursor");
            None
        }
    }
}

#[cfg(feature = "blocking")]
impl<T: Record> Iterator for QueryCursor<T> {
    type Item = QueryResult<T>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.pending_result.clone() {
            None => self.next_batch(),
            Some(result) => {
                self.pending_result = None;
                Some(result)
            }
        }
    }
}
