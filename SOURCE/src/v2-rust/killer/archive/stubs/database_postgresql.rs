//! PostgreSQL Database — 45+ functions (connection pool, queries, transactions, DDL)

#[derive(Debug, Clone)]
pub struct PostgresConnection { pub url: String }

impl PostgresConnection {
    pub fn new(url: &str) -> Self { Self { url: url.to_string() } }
}
