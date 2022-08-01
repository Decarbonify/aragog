use crate::db::database_record_dto::DatabaseRecordDto;
use crate::error::ArangoHttpError;
use crate::query::{Query, QueryCursor, QueryResult};
use crate::{DatabaseAccess, DatabaseRecord, Error, OperationOptions, Record};
use arangors::uclient::ClientExt;
use arangors::{AqlOptions, AqlQuery};
use std::convert::TryInto;

#[maybe_async::maybe_async]
pub async fn update_record<T, D, C>(
    obj: DatabaseRecord<T>,
    key: &str,
    db_accessor: &D,
    collection_name: &str,
    options: OperationOptions,
) -> Result<DatabaseRecord<T>, Error>
where
    T: Record,
    C: ClientExt,
    D: DatabaseAccess<C> + ?Sized,
{
    log::debug!("Updating document {} {}", collection_name, key);
    let collection = db_accessor.get_collection(collection_name)?;
    let response = match collection.update_document(key, obj, options.into()).await {
        Ok(resp) => resp,
        Err(error) => return Err(Error::from(error)),
    };
    response.try_into()
}

#[maybe_async::maybe_async]
pub async fn create_record<T, D, C>(
    obj: T,
    key: Option<String>,
    db_accessor: &D,
    collection_name: &str,
    options: OperationOptions,
) -> Result<DatabaseRecord<T>, Error>
where
    T: Record,
    C: ClientExt,
    D: DatabaseAccess<C> + ?Sized,
{
    let collection = db_accessor.get_collection(collection_name)?;
    log::debug!("Creating new {} document", collection.name());
    let dto = DatabaseRecordDto::new(obj, key);
    let response = match collection.create_document(dto, options.into()).await {
        Ok(resp) => resp,
        Err(error) => return Err(Error::from(error)),
    };
    response.try_into()
}

#[maybe_async::maybe_async]
pub async fn retrieve_record<T, D, C>(
    key: &str,
    db_accessor: &D,
    collection_name: &str,
) -> Result<DatabaseRecord<T>, Error>
where
    T: Record,
    C: ClientExt,
    D: DatabaseAccess<C> + ?Sized,
{
    log::debug!("Retrieving {} {} from database", collection_name, key);
    let collection = db_accessor.get_collection(collection_name)?;
    let record = match collection.document(key).await {
        Ok(doc) => doc,
        Err(error) => {
            println!("{}", error);
            let err = Error::from(error);
            if let Error::ArangoError(ref db_error) = err {
                if ArangoHttpError::NotFound == db_error.http_error {
                    return Err(Error::NotFound {
                        item: collection_name.to_string(),
                        id: key.to_string(),
                        source: Some(db_error.clone()),
                    });
                }
            }
            return Err(err);
        }
    };
    Ok(DatabaseRecord::from(record))
}

#[maybe_async::maybe_async]
pub async fn remove_record<T, D, C>(
    key: &str,
    db_accessor: &D,
    collection_name: &str,
    options: OperationOptions,
) -> Result<(), Error>
where
    T: Record,
    C: ClientExt,
    D: DatabaseAccess<C> + ?Sized,
{
    log::debug!("Removing {} {} from database", collection_name, key);
    let collection = db_accessor.get_collection(collection_name)?;
    match collection
        .remove_document::<T>(key, options.into(), None)
        .await
    {
        Ok(_result) => Ok(()),
        Err(error) => Err(Error::from(error)),
    }
}

#[maybe_async::maybe_async]
pub async fn raw_query_records<T, D, C>(db_accessor: &D, aql: &str) -> Result<QueryResult<T>, Error>
where
    T: Record,
    C: ClientExt,
    D: DatabaseAccess<C> + ?Sized,
{
    log::debug!(
        "Querying {} records through AQL: `{}`",
        T::COLLECTION_NAME,
        aql
    );
    let query_result = match db_accessor.database().aql_str(aql).await {
        Ok(value) => value,
        Err(error) => return Err(Error::from(error)),
    };
    Ok(query_result.into())
}

#[maybe_async::maybe_async]
pub async fn query_records<T, D, C>(db_accessor: &D, query: &Query) -> Result<QueryResult<T>, Error>
where
    T: Record,
    C: ClientExt,
    D: DatabaseAccess<C> + ?Sized,
{
    let aql = query.aql_str();
    log::debug!(
        "Querying {} records through AQL: `{}`",
        T::COLLECTION_NAME,
        aql
    );
    let mut aql_query = AqlQuery::builder().query(&aql).build();
    for (var, val) in &query.bind_vars {
        aql_query = aql_query.bind_var(var, val.clone());
    }
    let query_result = match db_accessor.database().aql_query(aql_query).await {
        Ok(value) => value,
        Err(error) => return Err(Error::from(error)),
    };
    Ok(query_result.into())
}

#[maybe_async::maybe_async]
pub async fn query_records_in_batches<T, D, C>(
    db_accessor: &D,
    query: &Query,
    batch_size: u32,
) -> Result<QueryCursor<T, C>, Error>
where
    T: Record,
    C: ClientExt,
    D: DatabaseAccess<C> + ?Sized,
{
    let aql = query.aql_str();
    log::debug!(
        "Querying {} records through AQL with {} batch size: `{}`",
        T::COLLECTION_NAME,
        batch_size,
        aql
    );
    let mut aql_query = AqlQuery::new(&aql)
        .batch_size(batch_size)
        .options(AqlOptions::builder().full_count(true).build());
    for (var, val) in &query.bind_vars {
        aql_query = aql_query.bind_var(var, val.clone());
    }
    let cursor = match db_accessor.database().aql_query_batch(aql_query).await {
        Ok(value) => value,
        Err(error) => return Err(Error::from(error)),
    };
    Ok(QueryCursor::new(cursor, db_accessor.database().clone()))
}
