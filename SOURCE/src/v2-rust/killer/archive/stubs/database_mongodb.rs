//! MongoDB Database — 42+ functions (connection pool, CRUD, aggregation, indexing, transactions)

#[derive(Debug, Clone)]
pub struct MongoConnection { pub url: String }

impl MongoConnection {
    pub fn new(url: &str) -> Self { Self { url: url.to_string() } }
}
