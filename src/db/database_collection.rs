use crate::Error;
use arangors::{uclient::ClientExt, Collection};
use std::ops::Deref;

/// Struct containing the connection information on a `ArangoDB` collection
#[derive(Debug, Clone)]
pub struct DatabaseCollection<C: ClientExt> {
    /// The collection wrapper accessor of `arangors` crate driver
    collection: Collection<C>,
}

impl<C: ClientExt> DatabaseCollection<C> {
    /// Name of the collection, exactly as defined in database
    #[must_use]
    #[inline]
    pub fn name(&self) -> &str {
        self.collection.name()
    }

    /// Retrieves the total document count of this collection.
    ///
    /// # Returns
    ///
    /// On success a `i32` is returned as the document count.
    /// On failure a Error wil be returned.
    #[maybe_async::maybe_async]
    pub async fn record_count(&self) -> Result<u32, Error> {
        let properties = match self.collection.document_count().await {
            Ok(value) => value,
            Err(client_error) => return Err(Error::from(client_error)),
        };
        match properties.info.count {
            Some(value) => Ok(value),
            None => Ok(0),
        }
    }
}

impl<C: ClientExt> From<Collection<C>> for DatabaseCollection<C> {
    fn from(collection: Collection<C>) -> Self {
        Self { collection }
    }
}

impl<C: ClientExt> Deref for DatabaseCollection<C> {
    type Target = Collection<C>;

    fn deref(&self) -> &Self::Target {
        &self.collection
    }
}
