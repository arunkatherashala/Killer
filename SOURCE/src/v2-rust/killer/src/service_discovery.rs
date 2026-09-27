//! Service Discovery — Register and discover microservices

#[derive(Debug, Clone)]
pub struct ServiceRegistry {
    pub services: Vec<(String, String)>,
}

impl ServiceRegistry {
    pub fn new() -> Self { Self { services: vec![] } }
}
