//! Attribute-Based Access Control (ABAC) — Fine-grained permission policies

use std::collections::HashMap;

pub struct Policy {
    pub rules: HashMap<String, String>,
}

impl Policy {
    pub fn new() -> Self { Self { rules: HashMap::new() } }
}
