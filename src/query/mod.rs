#![allow(clippy::use_self)]
use crate::query::graph_query::{GraphQueryData, GraphQueryDirection};
use crate::query::operations::{AqlOperation, OperationContainer};
use crate::query::query_id_helper::get_str_identifier;
use crate::query::utils::{string_from_array, OptionalQueryString};
use crate::undefined_record::UndefinedRecord;
use crate::{DatabaseAccess, Error, Record};
use arangors::uclient::ClientExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
pub use {
    comparison::Comparison, comparison::ComparisonBuilder, filter::Filter,
    query_cursor::QueryCursor, query_result::QueryResult,
};

mod comparison;
mod filter;
mod graph_query;
mod operations;
mod query_cursor;
mod query_id_helper;
mod query_result;
mod utils;

/// Macro to simplify the [`Query`] construction:
///
/// # Examples
///
/// ```rust
/// #[macro_use]
/// extern crate aragog;
/// # use aragog::query::Query;
///
/// # fn main() {
/// // The following are equivalent:
/// let query = Query::new("Users");
/// let query = query!("Users");
/// # }
/// ```
#[macro_export]
macro_rules! query {
    ($collection:expr) => {
        $crate::query::Query::new($collection)
    };
}

/// The direction for [`Query::sort`] method
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SortDirection {
    /// Ascending
    Asc,
    /// Descending
    Desc,
}

impl Display for SortDirection {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Asc => "ASC",
                Self::Desc => "DESC",
            }
        )
    }
}

/// A query utility for `ArangoDB` to avoid writing simple AQL strings.
/// After building can be rendered as an AQL string with the [`aql_str`] method.
///
/// # Examples
///
/// ```rust
/// # use aragog::Record;
/// # use aragog::query::Query;
/// # use serde::{Serialize, Deserialize};
/// # #[macro_use] extern crate aragog;
/// #
/// #[derive(Clone, Serialize, Deserialize, Record)]
/// pub struct User {
///     pub username: String
/// }
///
/// # fn main() {
/// // You can init a query in three ways, the following lines do the exact same thing
/// let query = Query::new("Users");
/// let query2 = User::query(); // `User` needs to implement `Record`
/// let query3 = query!("Users");
/// # }
/// ```
///
/// [`aql_str`]: Self::aql_str
#[derive(Clone, Debug)]
pub struct Query {
    with_collections: OptionalQueryString,
    collection: String,
    graph_data: Option<GraphQueryData>,
    operations: OperationContainer,
    distinct: bool,
    sub_query: Option<String>,
    item_identifier: usize,
    /// bind parameters to substitute in query string
    pub bind_vars: HashMap<String, Value>,
}

impl Query {
    /// Creates a new empty `Query`.
    /// You can call `filter`, `sort`, `limit` and `distinct` to customize the query afterwards
    ///
    /// # Arguments
    ///
    /// * `collection_name`- The name of the queried collection
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aragog::query::Query;
    /// let query = Query::new("User");
    /// ```
    #[inline]
    #[must_use]
    pub fn new(collection_name: &str) -> Self {
        Self {
            with_collections: OptionalQueryString(None),
            collection: String::from(collection_name),
            graph_data: None,
            operations: OperationContainer(vec![]),
            distinct: false,
            sub_query: None,
            item_identifier: 0,
            bind_vars: HashMap::default(),
        }
    }

    /// Binds `var` attribute to be substituted by `value` in the query string
    #[must_use]
    #[inline]
    pub fn bind_var(mut self, var: &str, value: impl Into<Value>) -> Self {
        self.bind_vars.insert(var.to_owned(), value.into());
        self
    }

    /// Tries to bind `var` attribute to be substituted by `value` in the query string
    ///
    /// # Errors
    ///
    /// Returns an error if `value` can't be converted to a [`serde_json::Value`]
    pub fn try_bind_var(mut self, var: &str, value: impl serde::Serialize) -> Result<Self, Error> {
        self.bind_vars
            .insert(var.to_owned(), serde_json::to_value(value)?);
        Ok(self)
    }

    /// Creates a new outbound traversing `Query` though a `edge_collection`.
    /// You can call `filter`, `sort`, `limit` and `distinct` to customize the query afterwards
    ///
    /// # Arguments
    ///
    /// * `min` - The minimum depth of the graph request
    /// * `max` - The maximum depth of the graph request
    /// * `edge_collection`- The name of the traversed edge collection
    /// * `vertex` - The `_id` of the starting document (`User/123` for example)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aragog::query::Query;
    /// let query = Query::outbound(1, 2, "ChildOf", "User/123");
    /// ```
    #[inline]
    #[must_use]
    pub fn outbound(min: u16, max: u16, edge_collection: &str, vertex: &str) -> Self {
        Self {
            graph_data: Some(GraphQueryData {
                direction: GraphQueryDirection::Outbound,
                start_vertex: format!(r#"'{}'"#, vertex),
                min,
                max,
                named_graph: false,
            }),
            ..Self::new(edge_collection)
        }
    }

    /// Creates a new outbound traversing `Query` though a `named_grah`.
    /// You can call `filter`, `sort`, `limit` and `distinct` to customize the query afterwards
    ///
    /// # Arguments
    ///
    /// * `min` - The minimum depth of the graph request
    /// * `max` - The maximum depth of the graph request
    /// * `named_graph`- The named graph to traverse
    /// * `vertex` - The `_id` of the starting document (`User/123` for example)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aragog::query::Query;
    /// let query = Query::outbound_graph(1, 2, "SomeGraph", "User/123");
    /// ```
    #[inline]
    #[must_use]
    pub fn outbound_graph(min: u16, max: u16, named_graph: &str, vertex: &str) -> Self {
        Self {
            graph_data: Some(GraphQueryData {
                direction: GraphQueryDirection::Outbound,
                start_vertex: format!(r#"'{}'"#, vertex),
                min,
                max,
                named_graph: true,
            }),
            ..Self::new(named_graph)
        }
    }

    /// Creates a new `ANY` traversing `Query` though a `edge_collection`.
    /// You can call `filter`, `sort`, `limit` and `distinct` to customize the query afterwards
    ///
    /// # Arguments
    ///
    /// * `min` - The minimum depth of the graph request
    /// * `max` - The maximum depth of the graph request
    /// * `edge_collection`- The name of the traversed edge collection
    /// * `vertex` - The `_id` of the starting document (`User/123` for example)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aragog::query::Query;
    /// let query = Query::outbound(1, 2, "ChildOf", "User/123");
    /// ```
    #[inline]
    #[must_use]
    pub fn any(min: u16, max: u16, edge_collection: &str, vertex: &str) -> Self {
        Self {
            graph_data: Some(GraphQueryData {
                direction: GraphQueryDirection::Any,
                start_vertex: format!(r#"'{}'"#, vertex),
                min,
                max,
                named_graph: false,
            }),
            ..Self::new(edge_collection)
        }
    }

    /// Creates a new `ANY` traversing `Query` though a `named_grah`.
    /// You can call `filter`, `sort`, `limit` and `distinct` to customize the query afterwards
    ///
    /// # Arguments
    ///
    /// * `min` - The minimum depth of the graph request
    /// * `max` - The maximum depth of the graph request
    /// * `named_graph`- The named graph to traverse
    /// * `vertex` - The `_id` of the starting document (`User/123` for example)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aragog::query::Query;
    /// let query = Query::outbound_graph(1, 2, "SomeGraph", "User/123");
    /// ```
    #[inline]
    #[must_use]
    pub fn any_graph(min: u16, max: u16, named_graph: &str, vertex: &str) -> Self {
        Self {
            graph_data: Some(GraphQueryData {
                direction: GraphQueryDirection::Any,
                start_vertex: format!(r#"'{}'"#, vertex),
                min,
                max,
                named_graph: true,
            }),
            ..Self::new(named_graph)
        }
    }

    /// Creates a new inbound traversing `Query` though a `edge_collection`.
    /// You can call `filter`, `sort`, `limit` and `distinct` to customize the query afterwards
    ///
    /// # Arguments
    ///
    /// * `min` - The minimum depth of the graph request
    /// * `max` - The maximum depth of the graph request
    /// * `edge_collection`- The name of the traversed edge collection
    /// * `vertex` - The `_id` of the starting document (`User/123` for example)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aragog::query::Query;
    /// let query = Query::inbound(1, 2, "ChildOf", "User/123");
    /// ```
    #[inline]
    #[must_use]
    pub fn inbound(min: u16, max: u16, edge_collection: &str, vertex: &str) -> Self {
        Self {
            graph_data: Some(GraphQueryData {
                direction: GraphQueryDirection::Inbound,
                start_vertex: format!(r#"'{}'"#, vertex),
                min,
                max,
                named_graph: false,
            }),
            ..Self::new(edge_collection)
        }
    }

    /// Creates a new inbound traversing `Query` though a `named_grah`.
    /// You can call `filter`, `sort`, `limit` and `distinct` to customize the query afterwards
    ///
    /// # Arguments
    ///
    /// * `min` - The minimum depth of the graph request
    /// * `max` - The maximum depth of the graph request
    /// * `named_graph`- The named graph to traverse
    /// * `vertex` - The `_id` of the starting document (`User/123` for example)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aragog::query::Query;
    /// let query = Query::inbound_graph(1, 2, "SomeGraph", "User/123");
    /// ```
    #[inline]
    #[must_use]
    pub fn inbound_graph(min: u16, max: u16, named_graph: &str, vertex: &str) -> Self {
        Self {
            graph_data: Some(GraphQueryData {
                direction: GraphQueryDirection::Inbound,
                start_vertex: format!(r#"'{}'"#, vertex),
                min,
                max,
                named_graph: true,
            }),
            ..Self::new(named_graph)
        }
    }

    fn join(
        mut self,
        min: u16,
        max: u16,
        mut query: Self,
        direction: GraphQueryDirection,
        named_graph: bool,
    ) -> Self {
        self.item_identifier = query.item_identifier + 1;
        query.graph_data = Some(GraphQueryData {
            direction,
            start_vertex: get_str_identifier(self.item_identifier),
            min,
            max,
            named_graph,
        });
        self.sub_query = Some(query.aql_str());
        self
    }

    /// Adds an outbound traversing query to the current `Query`.
    ///
    /// # Arguments
    ///
    /// * `min` - The minimum depth of the graph request
    /// * `max` - The maximum depth of the graph request
    /// * `named_graph` - Is the following query on a Named graph?
    /// * `query` - The sub query to add
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aragog::query::Query;
    /// let query = Query::new("User").join_outbound(1, 2, false, Query::new("ChildOf"));
    /// assert_eq!(query.aql_str(), String::from("\
    ///     FOR b in User \
    ///         FOR a in 1..2 OUTBOUND b ChildOf \
    ///         return a\
    /// "));
    /// let query = Query::new("User").join_outbound(1, 2, true, Query::new("NamedGraph"));
    /// assert_eq!(query.aql_str(), String::from("\
    ///     FOR b in User \
    ///         FOR a in 1..2 OUTBOUND b GRAPH NamedGraph \
    ///         return a\
    /// "));
    /// ```
    #[inline]
    #[must_use]
    pub fn join_outbound(self, min: u16, max: u16, named_graph: bool, query: Self) -> Self {
        self.join(min, max, query, GraphQueryDirection::Outbound, named_graph)
    }

    /// Adds an inbound traversing query to the current `Query`.
    ///
    /// # Arguments
    ///
    /// * `min` - The minimum depth of the graph request
    /// * `max` - The maximum depth of the graph request
    /// * `named_graph` - Is the following query on a Named graph?
    /// * `query` - The sub query to add
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aragog::query::Query;
    /// let query = Query::new("User").join_inbound(1, 2, false, Query::new("ChildOf"));
    /// assert_eq!(query.aql_str(), String::from("\
    ///     FOR b in User \
    ///         FOR a in 1..2 INBOUND b ChildOf \
    ///         return a\
    /// "));
    /// let query = Query::new("User").join_inbound(1, 2, true, Query::new("NamedGraph"));
    /// assert_eq!(query.aql_str(), String::from("\
    ///     FOR b in User \
    ///         FOR a in 1..2 INBOUND b GRAPH NamedGraph \
    ///         return a\
    /// "));
    /// ```
    #[inline]
    #[must_use]
    pub fn join_inbound(self, min: u16, max: u16, named_graph: bool, query: Self) -> Self {
        self.join(min, max, query, GraphQueryDirection::Inbound, named_graph)
    }

    /// Adds an `ANY` traversing query to the current `Query`.
    ///
    /// # Arguments
    ///
    /// * `min` - The minimum depth of the graph request
    /// * `max` - The maximum depth of the graph request
    /// * `named_graph` - Is the following query on a Named graph?
    /// * `query` - The sub query to add
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aragog::query::Query;
    /// let query = Query::new("User").join_any(1, 2, false, Query::new("ChildOf"));
    /// assert_eq!(query.aql_str(), String::from("\
    ///     FOR b in User \
    ///         FOR a in 1..2 ANY b ChildOf \
    ///         return a\
    /// "));
    /// let query = Query::new("User").join_any(1, 2, true, Query::new("NamedGraph"));
    /// assert_eq!(query.aql_str(), String::from("\
    ///     FOR b in User \
    ///         FOR a in 1..2 ANY b GRAPH NamedGraph \
    ///         return a\
    /// "));
    /// ```
    #[inline]
    #[must_use]
    pub fn join_any(self, min: u16, max: u16, named_graph: bool, query: Self) -> Self {
        self.join(min, max, query, GraphQueryDirection::Any, named_graph)
    }

    /// Allow the current traversing `Query` to filter the traversed collections and avoid potentian deadlocks.
    ///
    /// # Arguments
    ///
    /// * `collections` - The names of the collections the query can traverse
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aragog::query::Query;
    /// let query = Query::new("User").with_collections(&["User", "Client"]).join_any(1, 2, false, Query::new("ChildOf"));
    /// assert_eq!(query.aql_str(), String::from("\
    ///     WITH User, Client \
    ///     FOR b in User \
    ///         FOR a in 1..2 ANY b ChildOf \
    ///         return a\
    /// "));
    /// ```
    #[inline]
    #[must_use]
    pub fn with_collections(mut self, collections: &[&str]) -> Self {
        self.with_collections =
            OptionalQueryString(Some(format!("WITH {} ", string_from_array(collections))));
        self
    }

    /// Allows to sort a current `Query` by different field names. The fields must exist or the query won't work.
    /// Every time the method is called, a new sorting condition is added.
    ///
    /// # Note
    ///
    /// If you add mutliple `sort` calls it will result in something like `SORT a.field, b.field, c.field`.
    /// If you separate the calls by a `limit` or other operation, the order will be respected and the resulting query
    /// will look like `SORT a.field LIMIT 10 SORT b.field, c.field`
    ///
    /// # Arguments
    ///
    /// * `field`: The field name, must exist in the collection
    /// * `direction`: Optional sorting direction for that field.
    /// The direction is optional because `ArangoDB` uses `ASC` sorting by default
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aragog::query::{Query, SortDirection};
    /// let query = Query::new("User")
    ///     .sort("username", Some(SortDirection::Desc))
    ///     .sort("age", Some(SortDirection::Asc)
    /// );
    /// ```
    #[inline]
    #[must_use]
    pub fn sort(mut self, field: &str, direction: Option<SortDirection>) -> Self {
        self.operations.0.push(AqlOperation::Sort {
            field: field.to_string(),
            direction: direction.unwrap_or(SortDirection::Asc),
        });
        self
    }

    /// Allows to filter a current `Query` by different comparisons.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aragog::query::{Query, Filter, Comparison};
    /// let query = Query::new("User").filter(Filter::new(Comparison::field("age").greater_than(18)));
    /// // or
    /// let query = Query::new("User").filter(Comparison::field("age").greater_than(18).into());
    /// ```
    #[inline]
    #[must_use]
    pub fn filter(mut self, filter: Filter) -> Self {
        self.operations.0.push(AqlOperation::Filter(filter));
        self
    }

    /// Allows to filter a current `Query` by different comparisons but using the `PRUNE` keyword.
    ///
    /// # Note
    ///
    /// The `prune` operation only works for graph queries (See `ArangoDB` documentation)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aragog::query::{Query, Filter, Comparison};
    /// let query = Query::outbound(1, 2, "ChildOf", "User/123").prune(Filter::new(Comparison::field("age").greater_than(18)));
    /// // or
    /// let query = Query::outbound(1, 2, "ChildOf", "User/123").prune(Comparison::field("age").greater_than(18).into());
    /// ```
    #[inline]
    #[must_use]
    pub fn prune(mut self, filter: Filter) -> Self {
        self.operations.0.push(AqlOperation::Prune(filter));
        self
    }

    /// Allows to paginate a current `Query`.
    ///
    /// # Arguments
    ///
    /// * `limit` - the maximum returned elements
    /// * `skip`- optional number of skipped elements
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aragog::query::Query;
    /// // We want maximum 10 elements but skip the first 5
    /// let query = Query::new("User").limit(10, Some(5));
    /// ```
    #[inline]
    #[must_use]
    pub fn limit(mut self, limit: u32, skip: Option<u32>) -> Self {
        self.operations.0.push(AqlOperation::Limit { skip, limit });
        self
    }

    /// Allows to avoid duplicate elements for a `Query`.
    ///
    /// # Note
    ///
    /// If you use sub-queries, only the `distinct` on the last sub query will be used.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aragog::query::{Query, Filter, Comparison};
    /// let query = Query::new("User")
    ///     .filter(Filter::new(Comparison::field("age").greater_than(18)))
    ///     .distinct();
    /// ```
    #[inline]
    #[must_use]
    pub const fn distinct(mut self) -> Self {
        self.distinct = true;
        self
    }

    /// Renders the AQL string corresponding to the current `Query`
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aragog::query::{Comparison, Query, Filter};
    /// let mut query = Query::new("User").filter(Filter::new(Comparison::field("age").greater_than(10)).
    ///     or(Comparison::field("username").in_str_array(&["Felix", "Bianca"]))).distinct();
    /// assert_eq!(query.to_aql(), String::from("\
    ///     FOR a in User \
    ///         FILTER a.age > 10 || a.username IN [\"Felix\", \"Bianca\"] \
    ///         return DISTINCT a\
    /// "));
    /// ```
    #[inline]
    #[must_use]
    #[deprecated(since = "0.17.0", note = "use `aql_str` instead")]
    pub fn to_aql(&self) -> String {
        self.aql_str()
    }

    /// Renders the AQL string corresponding to the current `Query`
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aragog::query::{Comparison, Query, Filter};
    /// let mut query = Query::new("User").filter(Filter::new(Comparison::field("age").greater_than(10)).
    ///     or(Comparison::field("username").in_str_array(&["Felix", "Bianca"]))).distinct();
    /// assert_eq!(query.aql_str(), String::from("\
    ///     FOR a in User \
    ///         FILTER a.age > 10 || a.username IN [\"Felix\", \"Bianca\"] \
    ///         return DISTINCT a\
    /// "));
    /// ```
    #[inline]
    #[must_use]
    pub fn aql_str(&self) -> String {
        let collection_id = get_str_identifier(self.item_identifier);
        let mut res = self.with_collections.to_string();
        if let Some(graph_data) = &self.graph_data {
            res = format!(
                "{}FOR {} in {}..{} {} {} {}{}",
                res,
                collection_id,
                graph_data.min,
                graph_data.max,
                graph_data.direction,
                &graph_data.start_vertex,
                if graph_data.named_graph { "GRAPH " } else { "" },
                &self.collection
            );
        } else {
            res = format!("{}FOR {} in {}", res, collection_id, &self.collection);
        }
        if !self.operations.0.is_empty() {
            res = format!("{} {}", res, self.operations.aql_str(&collection_id));
        }
        if let Some(sub_query) = &self.sub_query {
            res = format!("{} {}", res, sub_query);
        } else {
            res = format!(
                "{} return {}{}",
                res,
                if self.distinct { "DISTINCT " } else { "" },
                &collection_id
            );
        }
        res
    }

    /// Finds all documents in database matching the current `Query`.
    /// This will return a wrapper for `serde_json`::`Value` as an `UndefinedRecord`
    ///
    /// # Note
    /// Simple wrapper for [`DatabaseAccess`]::[`query`].
    /// Useful for queries returning various collection records.
    ///
    /// [`DatabaseAccess`]: crate::DatabaseAccess
    /// [`query`]: crate::DatabaseAccess::query
    #[maybe_async::maybe_async]
    pub async fn raw_call<D, C>(
        &self,
        db_accessor: &D,
    ) -> Result<QueryResult<UndefinedRecord>, Error>
    where
        C: ClientExt,
        D: DatabaseAccess<C> + ?Sized,
    {
        db_accessor.query(self).await
    }

    /// Finds all records in database matching the current `Query`.
    ///
    /// # Note
    /// Simple wrapper for [`Record`]::[`get`]
    ///
    /// [`Record`]: crate::Record
    /// [`get`]: crate::Record::get
    #[maybe_async::maybe_async]
    pub async fn call<D, T, C>(&self, db_accessor: &D) -> Result<QueryResult<T>, Error>
    where
        C: ClientExt,
        D: DatabaseAccess<C> + ?Sized,
        T: Record + Send,
    {
        T::get(self, db_accessor).await
    }

    /// Finds all documents in database matching the current `Query` using batches.
    /// This will return a wrapper for `serde_json`::`Value` as an `UndefinedRecord` inside a cursor.
    ///
    /// # Note
    /// Simple wrapper for [`DatabaseAccess`]::[`query_in_batches`].
    /// Useful for queries returning various collection records.
    ///
    /// [`DatabaseAccess`]: crate::DatabaseAccess
    /// [`query_in_batches`]: crate::DatabaseAccess::query_in_batches
    #[maybe_async::maybe_async]
    pub async fn raw_call_in_batches<D, C>(
        &self,
        db_accessor: &D,
        batch_size: u32,
    ) -> Result<QueryCursor<UndefinedRecord, C>, Error>
    where
        C: ClientExt,
        D: DatabaseAccess<C> + ?Sized,
    {
        db_accessor.query_in_batches(self, batch_size).await
    }

    /// Finds all records in database matching the current `Query` using batches.
    ///
    /// # Note
    /// Simple wrapper for [`Record`]::[`get_in_batches`]
    ///
    /// [`Record`]: crate::Record
    /// [`get_in_batches`]: crate::Record::get_in_batches
    #[maybe_async::maybe_async]
    pub async fn call_in_batches<D, T, C>(
        &self,
        db_accessor: &D,
        batch_size: u32,
    ) -> Result<QueryCursor<T, C>, Error>
    where
        C: ClientExt,
        D: DatabaseAccess<C> + ?Sized,
        T: Record + Send,
    {
        T::get_in_batches(self, db_accessor, batch_size).await
    }
}

impl Display for Query {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.aql_str())
    }
}
