//! Middleware — Request/response interception and transformation

pub trait Middleware: Send + Sync {
    fn before(&self, _req: &str) -> String { _req.to_string() }
    fn after(&self, _res: &str) -> String { _res.to_string() }
}

pub struct DefaultMiddleware;
impl Middleware for DefaultMiddleware {}
