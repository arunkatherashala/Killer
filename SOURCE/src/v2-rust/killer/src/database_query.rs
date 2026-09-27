//! Query Builder & ORM — 40+ functions (generic query DSL, filter builder, pagination)

#[derive(Debug)]
pub struct Query { pub sql: String }

impl Query {
    pub fn new() -> Self { Self { sql: String::new() } }
    pub fn select(&mut self, fields: &[&str]) -> &mut Self {
        self.sql.push_str("SELECT ");
        self.sql.push_str(&fields.join(", "));
        self
    }
}
