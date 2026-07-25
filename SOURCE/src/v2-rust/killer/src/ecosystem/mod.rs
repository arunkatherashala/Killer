// Phase 5: Ecosystem Module
// Standard library components: File I/O, JSON, HTTP, Crypto, Regex

pub mod filesystem;
pub mod json;
pub mod http;

pub use filesystem::{FileSystem, FileHandle, FileMode, FileSystemStats};
pub use json::{JsonParser, JsonValue, JsonStats};
pub use http::{HttpServer, HttpResponse, HttpRequest, HttpClient, HttpServerConfig, HttpClientConfig};
