//! File Upload Handling — Receive and store file uploads

pub struct UploadedFile { pub name: String, pub size: u64 }

impl UploadedFile {
    pub fn new(name: &str, size: u64) -> Self {
        Self { name: name.to_string(), size }
    }
}
