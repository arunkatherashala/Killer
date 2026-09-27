//! Role-Based Access Control (RBAC) — Role and permission management

#[derive(Debug, Clone)]
pub struct Role { pub name: String, pub permissions: Vec<String> }

impl Role {
    pub fn new(name: &str) -> Self { Self { name: name.to_string(), permissions: vec![] } }
}
