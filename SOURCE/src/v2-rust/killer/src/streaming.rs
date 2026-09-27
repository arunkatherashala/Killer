//! Streaming — Streaming response bodies with backpressure

pub trait StreamWriter {
    fn write(&mut self, data: &[u8]) -> std::io::Result<()>;
}

pub struct StreamResponse { _buffer: Vec<u8> }

impl StreamResponse {
    pub fn new() -> Self { Self { _buffer: vec![] } }
}
