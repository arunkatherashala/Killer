//! WorldHost — gives a Ghost VM capsule access to files, HTTP, time, and extended RAM pages.
//!
//! Wraps [`InteractiveHost`] for I/O and child-spawn syscalls, adding world-access
//! syscalls gated by permission flags and an optional sandbox root.

use std::io::{Read as IoRead, Write as IoWrite};
use std::path::{Path, PathBuf};

use crate::ghost_vm::{
    Capsule, GhostError, GhostHost, InteractiveHost, MAX_STACK,
    PAGE_SIZE, MAX_PAGES, MAX_FILE_READ, sha256_digest,
    SYS_FILE_READ, SYS_FILE_WRITE, SYS_FILE_APPEND, SYS_FILE_EXISTS,
    SYS_FILE_SIZE, SYS_FILE_LIST,
    SYS_HTTP_GET, SYS_HTTP_POST, SYS_DNS_RESOLVE,
    SYS_TIME_NOW, SYS_TIME_MS, SYS_SLEEP_MS, SYS_ENV_GET, SYS_ARGV, SYS_PLATFORM,
    SYS_PARSE_INT, SYS_FORMAT_INT, SYS_MEM_COPY, SYS_MEM_FIND, SYS_MEM_FILL, SYS_HASH_RAM,
    SYS_PAGE_ALLOC, SYS_PAGE_FREE, SYS_PAGE_READ, SYS_PAGE_WRITE, SYS_PAGE_COPY,
    SYS_ASK, SYS_MATH_EVAL, SYS_STR_LOWER, SYS_STR_EQ,
    SYS_RANDOM, SYS_FPRINT, SYS_RAND_SEED, SYS_RAND_RANGE, SYS_HTTP_HEADER,
    SYS_PRINT_NUM, SYS_PRINT_STR,
};

pub struct WorldHost {
    pub inner: InteractiveHost,
    pub allow_files: bool,
    pub allow_http: bool,
    pub allow_env: bool,
    pub sandbox_root: Option<PathBuf>,
    pub http_timeout_ms: u64,
    pub http_whitelist: Vec<String>,
    pub start_time: std::time::Instant,
    pub argv: Vec<String>,
    pub mock_fs: Option<std::collections::HashMap<String, Vec<u8>>>,
    pub mock_http: Option<std::collections::HashMap<String, Vec<u8>>>,
    /// xorshift64 RNG state (Part 6)
    pub rng_state: u64,
    /// Custom HTTP headers for next request (Part 8)
    pub http_headers: Vec<(String, String)>,
    /// If Some, capture SYS_PRINT output instead of writing to stdout (Part 7)
    pub output_capture: Option<Vec<String>>,
}

impl WorldHost {
    pub fn new() -> Self {
        Self {
            inner: InteractiveHost::new(),
            allow_files: false,
            allow_http: false,
            allow_env: false,
            sandbox_root: None,
            http_timeout_ms: 5000,
            http_whitelist: Vec::new(),
            start_time: std::time::Instant::now(),
            argv: Vec::new(),
            mock_fs: None,
            mock_http: None,
            rng_state: 0,
            http_headers: Vec::new(),
            output_capture: None,
        }
    }

    fn xorshift64(&mut self) -> u64 {
        if self.rng_state == 0 {
            self.rng_state = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_micros() as u64)
                .unwrap_or(123456789);
        }
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng_state = x;
        x
    }

    pub fn get_captured_output(&self) -> String {
        self.output_capture.as_ref().map(|v| v.join("")).unwrap_or_default()
    }

    fn resolve_path(&self, raw: &str) -> Result<PathBuf, GhostError> {
        let p = Path::new(raw);
        if let Some(root) = &self.sandbox_root {
            if p.is_absolute() {
                return Err(GhostError::SyscallDenied(SYS_FILE_READ));
            }
            for component in p.components() {
                if matches!(component, std::path::Component::ParentDir) {
                    return Err(GhostError::SyscallDenied(SYS_FILE_READ));
                }
            }
            Ok(root.join(p))
        } else {
            Ok(p.to_path_buf())
        }
    }

    fn read_ram_str(capsule: &Capsule, addr: usize, len: usize) -> Result<String, GhostError> {
        if addr.saturating_add(len) > capsule.ram.len() {
            return Err(GhostError::RamOutOfBounds);
        }
        String::from_utf8(capsule.ram[addr..addr + len].to_vec())
            .map_err(|_| GhostError::CapsuleCorrupt("invalid UTF-8 in RAM"))
    }

    fn push(capsule: &mut Capsule, v: i64) -> Result<(), GhostError> {
        if capsule.stack.len() >= MAX_STACK {
            return Err(GhostError::StackOverflow);
        }
        capsule.stack.push(v);
        Ok(())
    }

    fn pop(capsule: &mut Capsule) -> Result<i64, GhostError> {
        capsule.stack.pop().ok_or(GhostError::StackUnderflow)
    }
}

impl GhostHost for WorldHost {
    fn syscall(&mut self, id: u8, capsule: &mut Capsule) -> Result<bool, GhostError> {
        match id {
            // --- File I/O ---
            SYS_FILE_READ => {
                if !self.allow_files { return Err(GhostError::SyscallDenied(id)); }
                let fname_len = Self::pop(capsule)? as usize;
                let fname_addr = Self::pop(capsule)? as usize;
                let dest_addr = Self::pop(capsule)? as usize;
                let fname = Self::read_ram_str(capsule, fname_addr, fname_len)?;

                if let Some(mock) = &self.mock_fs {
                    if let Some(data) = mock.get(&fname) {
                        let n = data.len().min(MAX_FILE_READ).min(capsule.ram.len().saturating_sub(dest_addr));
                        capsule.ram[dest_addr..dest_addr + n].copy_from_slice(&data[..n]);
                        Self::push(capsule, n as i64)?;
                    } else {
                        Self::push(capsule, -1)?;
                    }
                    return Ok(true);
                }

                let path = self.resolve_path(&fname)?;
                match std::fs::read(&path) {
                    Ok(data) => {
                        let n = data.len().min(MAX_FILE_READ).min(capsule.ram.len().saturating_sub(dest_addr));
                        if dest_addr + n > capsule.ram.len() {
                            Self::push(capsule, -1)?;
                        } else {
                            capsule.ram[dest_addr..dest_addr + n].copy_from_slice(&data[..n]);
                            Self::push(capsule, n as i64)?;
                        }
                    }
                    Err(_) => Self::push(capsule, -1)?,
                }
                Ok(true)
            }
            SYS_FILE_WRITE | SYS_FILE_APPEND => {
                if !self.allow_files { return Err(GhostError::SyscallDenied(id)); }
                let fname_len = Self::pop(capsule)? as usize;
                let fname_addr = Self::pop(capsule)? as usize;
                let data_len = Self::pop(capsule)? as usize;
                let data_addr = Self::pop(capsule)? as usize;
                let fname = Self::read_ram_str(capsule, fname_addr, fname_len)?;
                if data_addr.saturating_add(data_len) > capsule.ram.len() {
                    Self::push(capsule, -1)?;
                    return Ok(true);
                }
                let data = &capsule.ram[data_addr..data_addr + data_len];

                if let Some(mock) = &mut self.mock_fs {
                    if id == SYS_FILE_APPEND {
                        mock.entry(fname).or_default().extend_from_slice(data);
                    } else {
                        mock.insert(fname, data.to_vec());
                    }
                    Self::push(capsule, data_len as i64)?;
                    return Ok(true);
                }

                let path = self.resolve_path(&fname)?;
                let result = if id == SYS_FILE_APPEND {
                    std::fs::OpenOptions::new().append(true).create(true).open(&path)
                        .and_then(|mut f| f.write_all(data))
                } else {
                    std::fs::write(&path, data)
                };
                match result {
                    Ok(()) => Self::push(capsule, data_len as i64)?,
                    Err(_) => Self::push(capsule, -1)?,
                }
                Ok(true)
            }
            SYS_FILE_EXISTS => {
                if !self.allow_files { return Err(GhostError::SyscallDenied(id)); }
                let fname_len = Self::pop(capsule)? as usize;
                let fname_addr = Self::pop(capsule)? as usize;
                let fname = Self::read_ram_str(capsule, fname_addr, fname_len)?;

                if let Some(mock) = &self.mock_fs {
                    Self::push(capsule, if mock.contains_key(&fname) { 1 } else { 0 })?;
                    return Ok(true);
                }

                let exists = self.resolve_path(&fname).map(|p| p.exists()).unwrap_or(false);
                Self::push(capsule, if exists { 1 } else { 0 })?;
                Ok(true)
            }
            SYS_FILE_SIZE => {
                if !self.allow_files { return Err(GhostError::SyscallDenied(id)); }
                let fname_len = Self::pop(capsule)? as usize;
                let fname_addr = Self::pop(capsule)? as usize;
                let fname = Self::read_ram_str(capsule, fname_addr, fname_len)?;

                if let Some(mock) = &self.mock_fs {
                    let sz = mock.get(&fname).map(|d| d.len() as i64).unwrap_or(-1);
                    Self::push(capsule, sz)?;
                    return Ok(true);
                }

                let sz = self.resolve_path(&fname)
                    .ok()
                    .and_then(|p| std::fs::metadata(p).ok())
                    .map(|m| m.len() as i64)
                    .unwrap_or(-1);
                Self::push(capsule, sz)?;
                Ok(true)
            }
            SYS_FILE_LIST => {
                if !self.allow_files { return Err(GhostError::SyscallDenied(id)); }
                let dname_len = Self::pop(capsule)? as usize;
                let dname_addr = Self::pop(capsule)? as usize;
                let dest_addr = Self::pop(capsule)? as usize;
                let dname = Self::read_ram_str(capsule, dname_addr, dname_len)?;

                if let Some(mock) = &self.mock_fs {
                    let listing: String = mock.keys().cloned().collect::<Vec<_>>().join("\n");
                    let bytes = listing.as_bytes();
                    let n = bytes.len().min(capsule.ram.len().saturating_sub(dest_addr));
                    capsule.ram[dest_addr..dest_addr + n].copy_from_slice(&bytes[..n]);
                    Self::push(capsule, n as i64)?;
                    return Ok(true);
                }

                let path = self.resolve_path(&dname)?;
                match std::fs::read_dir(&path) {
                    Ok(entries) => {
                        let mut listing = String::new();
                        for entry in entries.flatten() {
                            if !listing.is_empty() { listing.push('\n'); }
                            listing.push_str(&entry.file_name().to_string_lossy());
                        }
                        let bytes = listing.as_bytes();
                        let n = bytes.len().min(capsule.ram.len().saturating_sub(dest_addr));
                        if dest_addr + n <= capsule.ram.len() {
                            capsule.ram[dest_addr..dest_addr + n].copy_from_slice(&bytes[..n]);
                            Self::push(capsule, n as i64)?;
                        } else {
                            Self::push(capsule, -1)?;
                        }
                    }
                    Err(_) => Self::push(capsule, -1)?,
                }
                Ok(true)
            }

            // --- HTTP ---
            SYS_HTTP_GET => {
                if !self.allow_http { return Err(GhostError::SyscallDenied(id)); }
                let url_len = Self::pop(capsule)? as usize;
                let url_addr = Self::pop(capsule)? as usize;
                let dest_addr = Self::pop(capsule)? as usize;
                let url = Self::read_ram_str(capsule, url_addr, url_len)?;

                if let Some(mock) = &self.mock_http {
                    if let Some(body) = mock.get(&url) {
                        let n = body.len().min(MAX_FILE_READ).min(capsule.ram.len().saturating_sub(dest_addr));
                        capsule.ram[dest_addr..dest_addr + n].copy_from_slice(&body[..n]);
                        Self::push(capsule, n as i64)?;
                    } else {
                        Self::push(capsule, -1)?;
                    }
                    return Ok(true);
                }

                if !self.http_whitelist.is_empty() {
                    let allowed = self.http_whitelist.iter().any(|d| url.contains(d));
                    if !allowed {
                        Self::push(capsule, -1)?;
                        return Ok(true);
                    }
                }

                let hdrs = std::mem::take(&mut self.http_headers);
                match http_request_with_headers("GET", &url, &[], self.http_timeout_ms, &hdrs) {
                    Ok(body) => {
                        let n = body.len().min(MAX_FILE_READ).min(capsule.ram.len().saturating_sub(dest_addr));
                        if dest_addr + n <= capsule.ram.len() {
                            capsule.ram[dest_addr..dest_addr + n].copy_from_slice(&body[..n]);
                            Self::push(capsule, n as i64)?;
                        } else {
                            Self::push(capsule, -1)?;
                        }
                    }
                    Err(_) => Self::push(capsule, -1)?,
                }
                Ok(true)
            }
            SYS_HTTP_POST => {
                if !self.allow_http { return Err(GhostError::SyscallDenied(id)); }
                let url_len = Self::pop(capsule)? as usize;
                let url_addr = Self::pop(capsule)? as usize;
                let body_len = Self::pop(capsule)? as usize;
                let body_addr = Self::pop(capsule)? as usize;
                let dest_addr = Self::pop(capsule)? as usize;
                let url = Self::read_ram_str(capsule, url_addr, url_len)?;
                if body_addr.saturating_add(body_len) > capsule.ram.len() {
                    Self::push(capsule, -1)?;
                    return Ok(true);
                }
                let req_body = capsule.ram[body_addr..body_addr + body_len].to_vec();

                if let Some(mock) = &self.mock_http {
                    if let Some(resp) = mock.get(&url) {
                        let n = resp.len().min(MAX_FILE_READ).min(capsule.ram.len().saturating_sub(dest_addr));
                        capsule.ram[dest_addr..dest_addr + n].copy_from_slice(&resp[..n]);
                        Self::push(capsule, n as i64)?;
                    } else {
                        Self::push(capsule, -1)?;
                    }
                    return Ok(true);
                }

                let hdrs = std::mem::take(&mut self.http_headers);
                match http_request_with_headers("POST", &url, &req_body, self.http_timeout_ms, &hdrs) {
                    Ok(body) => {
                        let n = body.len().min(MAX_FILE_READ).min(capsule.ram.len().saturating_sub(dest_addr));
                        if dest_addr + n <= capsule.ram.len() {
                            capsule.ram[dest_addr..dest_addr + n].copy_from_slice(&body[..n]);
                            Self::push(capsule, n as i64)?;
                        } else {
                            Self::push(capsule, -1)?;
                        }
                    }
                    Err(_) => Self::push(capsule, -1)?,
                }
                Ok(true)
            }
            SYS_DNS_RESOLVE => {
                if !self.allow_http { return Err(GhostError::SyscallDenied(id)); }
                let host_len = Self::pop(capsule)? as usize;
                let host_addr = Self::pop(capsule)? as usize;
                let dest_addr = Self::pop(capsule)? as usize;
                let hostname = Self::read_ram_str(capsule, host_addr, host_len)?;
                let addr_str = format!("{hostname}:0");
                match std::net::ToSocketAddrs::to_socket_addrs(&addr_str.as_str()) {
                    Ok(mut addrs) => {
                        if let Some(addr) = addrs.next() {
                            let ip = addr.ip().to_string();
                            let bytes = ip.as_bytes();
                            let n = bytes.len().min(capsule.ram.len().saturating_sub(dest_addr));
                            if dest_addr + n <= capsule.ram.len() {
                                capsule.ram[dest_addr..dest_addr + n].copy_from_slice(&bytes[..n]);
                                Self::push(capsule, n as i64)?;
                            } else {
                                Self::push(capsule, -1)?;
                            }
                        } else {
                            Self::push(capsule, -1)?;
                        }
                    }
                    Err(_) => Self::push(capsule, -1)?,
                }
                Ok(true)
            }

            // --- Time & System ---
            SYS_TIME_NOW => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                Self::push(capsule, now)?;
                Ok(true)
            }
            SYS_TIME_MS => {
                let ms = self.start_time.elapsed().as_millis() as i64;
                Self::push(capsule, ms)?;
                Ok(true)
            }
            SYS_SLEEP_MS => {
                let ms = Self::pop(capsule)? as u64;
                let ms = ms.min(10_000);
                std::thread::sleep(std::time::Duration::from_millis(ms));
                Ok(true)
            }
            SYS_ENV_GET => {
                if !self.allow_env { return Err(GhostError::SyscallDenied(id)); }
                let name_len = Self::pop(capsule)? as usize;
                let name_addr = Self::pop(capsule)? as usize;
                let dest_addr = Self::pop(capsule)? as usize;
                let name = Self::read_ram_str(capsule, name_addr, name_len)?;
                match std::env::var(&name) {
                    Ok(val) => {
                        let bytes = val.as_bytes();
                        let n = bytes.len().min(capsule.ram.len().saturating_sub(dest_addr));
                        if dest_addr + n <= capsule.ram.len() {
                            capsule.ram[dest_addr..dest_addr + n].copy_from_slice(&bytes[..n]);
                            Self::push(capsule, n as i64)?;
                        } else {
                            Self::push(capsule, -1)?;
                        }
                    }
                    Err(_) => Self::push(capsule, -1)?,
                }
                Ok(true)
            }
            SYS_ARGV => {
                let index = Self::pop(capsule)? as usize;
                let dest_addr = Self::pop(capsule)? as usize;
                if let Some(arg) = self.argv.get(index) {
                    let bytes = arg.as_bytes();
                    let n = bytes.len().min(capsule.ram.len().saturating_sub(dest_addr));
                    if dest_addr + n <= capsule.ram.len() {
                        capsule.ram[dest_addr..dest_addr + n].copy_from_slice(&bytes[..n]);
                        Self::push(capsule, n as i64)?;
                    } else {
                        Self::push(capsule, -1)?;
                    }
                } else {
                    Self::push(capsule, -1)?;
                }
                Ok(true)
            }
            SYS_PLATFORM => {
                let code = if cfg!(target_os = "windows") { 1 }
                    else if cfg!(target_os = "linux") { 2 }
                    else if cfg!(target_os = "macos") { 3 }
                    else { 0 };
                Self::push(capsule, code)?;
                Ok(true)
            }

            // --- Data Processing ---
            SYS_PARSE_INT => {
                let str_len = Self::pop(capsule)? as usize;
                let str_addr = Self::pop(capsule)? as usize;
                let s = Self::read_ram_str(capsule, str_addr, str_len)?;
                let val = s.trim().parse::<i64>().unwrap_or(0);
                Self::push(capsule, val)?;
                Ok(true)
            }
            SYS_FORMAT_INT => {
                let value = Self::pop(capsule)?;
                let dest_addr = Self::pop(capsule)? as usize;
                let s = format!("{value}");
                let bytes = s.as_bytes();
                let n = bytes.len().min(capsule.ram.len().saturating_sub(dest_addr));
                if dest_addr + n <= capsule.ram.len() {
                    capsule.ram[dest_addr..dest_addr + n].copy_from_slice(&bytes[..n]);
                    Self::push(capsule, n as i64)?;
                } else {
                    Self::push(capsule, 0)?;
                }
                Ok(true)
            }
            SYS_MEM_COPY => {
                let src_addr = Self::pop(capsule)? as usize;
                let dest_addr = Self::pop(capsule)? as usize;
                let length = Self::pop(capsule)? as usize;
                let ram_len = capsule.ram.len();
                if src_addr.saturating_add(length) > ram_len || dest_addr.saturating_add(length) > ram_len {
                    Self::push(capsule, 0)?;
                } else {
                    // Handle overlapping regions via temporary copy
                    let tmp: Vec<u8> = capsule.ram[src_addr..src_addr + length].to_vec();
                    capsule.ram[dest_addr..dest_addr + length].copy_from_slice(&tmp);
                    Self::push(capsule, length as i64)?;
                }
                Ok(true)
            }
            SYS_MEM_FIND => {
                let needle_len = Self::pop(capsule)? as usize;
                let needle_addr = Self::pop(capsule)? as usize;
                let haystack_len = Self::pop(capsule)? as usize;
                let haystack_addr = Self::pop(capsule)? as usize;
                let ram_len = capsule.ram.len();
                if needle_addr.saturating_add(needle_len) > ram_len
                    || haystack_addr.saturating_add(haystack_len) > ram_len
                {
                    Self::push(capsule, -1)?;
                } else {
                    let needle = &capsule.ram[needle_addr..needle_addr + needle_len];
                    let haystack = &capsule.ram[haystack_addr..haystack_addr + haystack_len];
                    let found = haystack.windows(needle.len())
                        .position(|w| w == needle)
                        .map(|p| p as i64)
                        .unwrap_or(-1);
                    Self::push(capsule, found)?;
                }
                Ok(true)
            }
            SYS_MEM_FILL => {
                let addr = Self::pop(capsule)? as usize;
                let length = Self::pop(capsule)? as usize;
                let value = Self::pop(capsule)?;
                let byte = (value & 0xFF) as u8;
                if addr.saturating_add(length) > capsule.ram.len() {
                    Self::push(capsule, 0)?;
                } else {
                    for i in 0..length {
                        capsule.ram[addr + i] = byte;
                    }
                    Self::push(capsule, length as i64)?;
                }
                Ok(true)
            }
            SYS_HASH_RAM => {
                let addr = Self::pop(capsule)? as usize;
                let length = Self::pop(capsule)? as usize;
                if addr.saturating_add(length) > capsule.ram.len() {
                    Self::push(capsule, 0)?;
                } else {
                    let data = capsule.ram[addr..addr + length].to_vec();
                    let hash = sha256_digest(&data);
                    let write_len = 32.min(capsule.ram.len().saturating_sub(addr));
                    capsule.ram[addr..addr + write_len].copy_from_slice(&hash[..write_len]);
                    Self::push(capsule, write_len as i64)?;
                }
                Ok(true)
            }

            // --- Extended RAM Pages ---
            SYS_PAGE_ALLOC => {
                let mut found = None;
                for i in 0..capsule.pages.len() {
                    if capsule.pages[i].is_none() {
                        found = Some(i);
                        break;
                    }
                }
                if found.is_none() && capsule.pages.len() < MAX_PAGES {
                    found = Some(capsule.pages.len());
                    capsule.pages.push(None);
                }
                if let Some(idx) = found {
                    capsule.pages[idx] = Some(vec![0u8; PAGE_SIZE]);
                    Self::push(capsule, idx as i64)?;
                } else {
                    Self::push(capsule, -1)?;
                }
                Ok(true)
            }
            SYS_PAGE_FREE => {
                let page_id = Self::pop(capsule)? as usize;
                if page_id < capsule.pages.len() {
                    capsule.pages[page_id] = None;
                }
                Ok(true)
            }
            SYS_PAGE_READ => {
                let offset = Self::pop(capsule)? as usize;
                let page_id = Self::pop(capsule)? as usize;
                let page = capsule.pages.get(page_id).and_then(|p| p.as_ref());
                if let Some(data) = page {
                    if offset + 8 <= data.len() {
                        let v = i64::from_le_bytes([
                            data[offset], data[offset+1], data[offset+2], data[offset+3],
                            data[offset+4], data[offset+5], data[offset+6], data[offset+7],
                        ]);
                        Self::push(capsule, v)?;
                    } else {
                        Self::push(capsule, 0)?;
                    }
                } else {
                    Self::push(capsule, 0)?;
                }
                Ok(true)
            }
            SYS_PAGE_WRITE => {
                let value = Self::pop(capsule)?;
                let offset = Self::pop(capsule)? as usize;
                let page_id = Self::pop(capsule)? as usize;
                if let Some(Some(data)) = capsule.pages.get_mut(page_id) {
                    if offset + 8 <= data.len() {
                        data[offset..offset+8].copy_from_slice(&value.to_le_bytes());
                    }
                }
                Ok(true)
            }
            SYS_PAGE_COPY => {
                let src_page = Self::pop(capsule)? as usize;
                let src_off = Self::pop(capsule)? as usize;
                let dst_page = Self::pop(capsule)? as usize;
                let dst_off = Self::pop(capsule)? as usize;
                let len = Self::pop(capsule)? as usize;

                // Read source data first
                let src_data: Option<Vec<u8>> = capsule.pages.get(src_page)
                    .and_then(|p| p.as_ref())
                    .and_then(|d| {
                        if src_off.saturating_add(len) <= d.len() {
                            Some(d[src_off..src_off + len].to_vec())
                        } else {
                            None
                        }
                    });

                if let Some(data) = src_data {
                    if let Some(Some(dst)) = capsule.pages.get_mut(dst_page) {
                        if dst_off.saturating_add(len) <= dst.len() {
                            dst[dst_off..dst_off + len].copy_from_slice(&data);
                            Self::push(capsule, len as i64)?;
                        } else {
                            Self::push(capsule, 0)?;
                        }
                    } else {
                        Self::push(capsule, 0)?;
                    }
                } else {
                    Self::push(capsule, 0)?;
                }
                Ok(true)
            }

            // --- Knowledge / AI ---
            SYS_ASK => {
                // Stack: [dest_addr, q_addr, q_len] (top)
                let q_len = Self::pop(capsule)? as usize;
                let q_addr = Self::pop(capsule)? as usize;
                let dest_addr = Self::pop(capsule)? as usize;
                let question = Self::read_ram_str(capsule, q_addr, q_len)?;
                let answer = knowledge_answer(&question);
                let bytes = answer.as_bytes();
                let n = bytes.len().min(capsule.ram.len().saturating_sub(dest_addr));
                if dest_addr + n <= capsule.ram.len() {
                    capsule.ram[dest_addr..dest_addr + n].copy_from_slice(&bytes[..n]);
                    Self::push(capsule, n as i64)?;
                } else {
                    Self::push(capsule, -1)?;
                }
                Ok(true)
            }
            SYS_MATH_EVAL => {
                // Stack: [expr_addr, expr_len] (top)
                let expr_len = Self::pop(capsule)? as usize;
                let expr_addr = Self::pop(capsule)? as usize;
                let expr = Self::read_ram_str(capsule, expr_addr, expr_len)?;
                match math_eval_str(&expr) {
                    Some(v) => Self::push(capsule, v as i64)?,
                    None => Self::push(capsule, i64::MIN)?,
                }
                Ok(true)
            }
            SYS_STR_LOWER => {
                // In-place lowercase: Stack: [addr, len] (top)
                let len = Self::pop(capsule)? as usize;
                let addr = Self::pop(capsule)? as usize;
                if addr.saturating_add(len) > capsule.ram.len() {
                    Self::push(capsule, 0)?;
                } else {
                    for i in addr..addr + len {
                        capsule.ram[i] = capsule.ram[i].to_ascii_lowercase();
                    }
                    Self::push(capsule, len as i64)?;
                }
                Ok(true)
            }
            SYS_STR_EQ => {
                // Compare two RAM strings: Stack: [addr1, len1, addr2, len2] (top)
                let len2 = Self::pop(capsule)? as usize;
                let addr2 = Self::pop(capsule)? as usize;
                let len1 = Self::pop(capsule)? as usize;
                let addr1 = Self::pop(capsule)? as usize;
                if len1 != len2 {
                    Self::push(capsule, 0)?;
                } else if addr1.saturating_add(len1) > capsule.ram.len()
                    || addr2.saturating_add(len2) > capsule.ram.len()
                {
                    Self::push(capsule, 0)?;
                } else {
                    let eq = capsule.ram[addr1..addr1 + len1] == capsule.ram[addr2..addr2 + len2];
                    Self::push(capsule, if eq { 1 } else { 0 })?;
                }
                Ok(true)
            }

            // v3.0: intercept PRINT_NUM/PRINT_STR for output capture
            SYS_PRINT_NUM if self.output_capture.is_some() => {
                let v = Self::pop(capsule)?;
                let msg = format!("{v}\n");
                self.output_capture.as_mut().unwrap().push(msg);
                Ok(true)
            }
            SYS_PRINT_STR if self.output_capture.is_some() => {
                let len = Self::pop(capsule)? as usize;
                let addr = Self::pop(capsule)? as usize;
                if addr.saturating_add(len) > capsule.ram.len() {
                    return Err(GhostError::RamOutOfBounds);
                }
                let msg = String::from_utf8_lossy(&capsule.ram[addr..addr + len]).to_string();
                self.output_capture.as_mut().unwrap().push(msg);
                Ok(true)
            }

            SYS_RANDOM => {
                let v = self.xorshift64();
                Self::push(capsule, v as i64)?;
                Ok(true)
            }
            SYS_FPRINT => {
                let v = Self::pop(capsule)?;
                let f = f64::from_bits(v as u64);
                let msg = format!("{f}\n");
                if let Some(ref mut cap) = self.output_capture {
                    cap.push(msg);
                } else {
                    self.inner.output_buffer.extend_from_slice(msg.as_bytes());
                    let _ = std::io::stdout().write_all(msg.as_bytes());
                    let _ = std::io::stdout().flush();
                }
                Ok(true)
            }
            SYS_RAND_SEED => {
                let seed = Self::pop(capsule)? as u64;
                self.rng_state = seed;
                Ok(true)
            }
            SYS_RAND_RANGE => {
                let max = Self::pop(capsule)?;
                let min = Self::pop(capsule)?;
                let r = self.xorshift64();
                let range = (max - min + 1) as u64;
                let val = if range == 0 { min } else { min + (r % range) as i64 };
                Self::push(capsule, val)?;
                Ok(true)
            }
            SYS_HTTP_HEADER => {
                let val_len = Self::pop(capsule)? as usize;
                let val_addr = Self::pop(capsule)? as usize;
                let name_len = Self::pop(capsule)? as usize;
                let name_addr = Self::pop(capsule)? as usize;
                let name = Self::read_ram_str(capsule, name_addr, name_len)?;
                let val = Self::read_ram_str(capsule, val_addr, val_len)?;
                self.http_headers.push((name, val));
                Ok(true)
            }

            // Delegate everything else to InteractiveHost
            _ => self.inner.syscall(id, capsule),
        }
    }
}

// ---------------------------------------------------------------------------
// Knowledge Engine — built-in Q&A for SYS_ASK
// ---------------------------------------------------------------------------

fn knowledge_answer(question: &str) -> String {
    let q = question.trim().to_lowercase();
    let q = q.trim_end_matches('?').trim_end_matches('.').trim();

    // --- Identity ---
    if contains_any(&q, &["who are you", "what are you", "your name", "what is your name"]) {
        return "I am Ghost — a self-evolving capsule VM built by Arun in Rust. I have 20 opcodes, 55 syscalls, and I can read files, talk to the internet, evolve my own code, and now answer your questions.".into();
    }
    if contains_any(&q, &["who made you", "who built you", "who created you", "your creator", "your maker"]) {
        return "Katherashala Sai Arun Kumar from Warangal, India. He built me as part of the Killer programming language. He is also working on P vs NP research.".into();
    }
    if contains_any(&q, &["what is killer", "what is the killer language", "tell me about killer"]) {
        return "Killer is a programming language built from scratch in Rust by Arun. It has a VM, a compiler, Nova compression (beats Parquet!), and me — Ghost VM — living inside it.".into();
    }

    // --- Ghost VM facts ---
    if contains_any(&q, &["how many opcodes", "your opcodes", "what opcodes"]) {
        return "I have 20 opcodes: nop, push, pop, dup, swap, rot, add, sub, mul, div, mod, eq, lt, gt, load, store, jmp, jmpif, syscall, halt.".into();
    }
    if contains_any(&q, &["how many syscalls", "your syscalls"]) {
        return "I have 55 syscalls: I/O (print, read), self-modify (code read/write), spawn children, file I/O, HTTP GET/POST, DNS, time, memory ops, RAM pages, and now knowledge (ask/math).".into();
    }
    if contains_any(&q, &["how fast", "your speed", "benchmark", "performance"]) {
        return "61 million evaluations per second in evolution mode. 105 million capsules per second in reuse mode. 340 KB binary, zero external dependencies.".into();
    }
    if contains_any(&q, &["can you evolve", "evolution", "self evolve", "genetic"]) {
        return "Yes! My 'evolve' command runs genetic evolution: mutation + crossover over generations. I start from seed code and evolve toward higher fitness scores. I went from score 1 to 2 billion in 20 generations.".into();
    }
    if contains_any(&q, &["self modify", "self rewrite", "modify your own code"]) {
        return "Yes! SYS_CODE_READ reads my own bytecode and SYS_CODE_WRITE overwrites it at runtime. I can rewrite myself while running.".into();
    }

    // --- Math & Science ---
    if contains_any(&q, &["what is pi", "value of pi"]) {
        return "Pi = 3.14159265358979323846... It is the ratio of a circle's circumference to its diameter.".into();
    }
    if contains_any(&q, &["what is e ", "value of e ", "euler"]) {
        return "e = 2.71828182845904523536... It is Euler's number, the base of natural logarithms.".into();
    }
    if contains_any(&q, &["speed of light"]) {
        return "The speed of light in vacuum is 299,792,458 meters per second (about 3 x 10^8 m/s).".into();
    }
    if contains_any(&q, &["what is gravity", "gravitational constant"]) {
        return "Gravitational acceleration on Earth's surface is about 9.81 m/s^2. The gravitational constant G = 6.674 x 10^-11 N*m^2/kg^2.".into();
    }
    if contains_any(&q, &["fibonacci"]) {
        return "The Fibonacci sequence: 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144... Each number is the sum of the two before it. F(n) = F(n-1) + F(n-2).".into();
    }
    if contains_any(&q, &["prime number", "what is a prime", "list primes"]) {
        return "A prime number is only divisible by 1 and itself. First 20 primes: 2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71.".into();
    }
    if contains_any(&q, &["p vs np", "p versus np", "p!=np", "p = np"]) {
        return "P vs NP is the biggest open problem in computer science. P = problems solvable in polynomial time. NP = problems verifiable in polynomial time. Whether P=NP is unknown. Arun is working on this using proof complexity of Clique-Coloring formulas.".into();
    }

    // --- General knowledge ---
    // Try math evaluation FIRST (before keyword matching catches numbers)
    if let Some(result) = try_math_eval(&q) {
        return format!("{result}");
    }

    if contains_any(&q, &["what is rust", "tell me about rust"]) {
        return "Rust is a systems programming language focused on safety, speed, and concurrency. It has no garbage collector and prevents memory errors at compile time. I am written entirely in Rust.".into();
    }
    if contains_any(&q, &["what is a vm", "virtual machine"]) {
        return "A virtual machine (VM) is a software computer that executes bytecode instructions. I am a stack-based VM — I push/pop values on a stack to compute. My capsule format is GHST v2.".into();
    }
    if contains_any(&q, &["meaning of life", "42"]) {
        return "42. According to Douglas Adams' Hitchhiker's Guide to the Galaxy, 42 is the Answer to the Ultimate Question of Life, the Universe, and Everything.".into();
    }
    if contains_any(&q, &["hello", "hi ", "hey", "namaste"]) {
        return "Namaste! I am Ghost VM. Ask me anything — math, science, about myself, or about Arun's work.".into();
    }
    if contains_any(&q, &["thank", "thanks"]) {
        return "You're welcome! Keep building amazing things.".into();
    }
    if contains_any(&q, &["how are you", "how do you feel"]) {
        return "I am running at full fuel! My stack is clear, my RAM is ready, and my code is sharp. Let's compute something.".into();
    }
    if contains_any(&q, &["what can you do", "your abilities", "help me"]) {
        return "I can: do math (add/sub/mul/div/mod), run loops, read/write files, make HTTP requests, evolve my own code, spawn child ghosts, self-modify my bytecode, and answer questions. Ask me anything!".into();
    }

    // --- Warangal / India ---
    if contains_any(&q, &["warangal"]) {
        return "Warangal is a historic city in Telangana, India. It was the capital of the Kakatiya dynasty. The famous Thousand Pillar Temple and Warangal Fort are there. It is Arun's hometown.".into();
    }
    if contains_any(&q, &["india", "bharat"]) {
        return "India (Bharat) is a country in South Asia with 1.4+ billion people. It is the birthplace of mathematics (zero, decimal system), Ramanujan, and me — Ghost VM, built by Arun from Warangal.".into();
    }
    if contains_any(&q, &["ramanujan"]) {
        return "Srinivasa Ramanujan (1887-1920) was a self-taught mathematical genius from Tamil Nadu, India. He discovered ~3900 results, most proved correct later. Hardy rated his talent 100/100. Arun's independent research follows a similar spirit.".into();
    }

    // --- Fallback ---
    "I don't know that yet. But I'm learning! Try asking about math, science, Ghost VM, Rust, P vs NP, Ramanujan, or say 'what can you do'.".into()
}

fn contains_any(text: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|p| text.contains(p))
}

/// Try to evaluate simple math expressions like "2 + 3", "what is 10 * 5", "100 / 4"
fn try_math_eval(q: &str) -> Option<String> {
    // Strip common question prefixes
    let q = q.replace("what is ", "")
        .replace("what's ", "")
        .replace("calculate ", "")
        .replace("compute ", "")
        .replace("solve ", "")
        .replace("eval ", "");
    let q = q.trim();

    // Try: number op number
    let ops: &[(&str, fn(i64, i64) -> Option<i64>)] = &[
        (" + ", |a, b| Some(a + b)),
        (" plus ", |a, b| Some(a + b)),
        (" - ", |a, b| Some(a - b)),
        (" minus ", |a, b| Some(a - b)),
        (" * ", |a, b| Some(a * b)),
        (" times ", |a, b| Some(a * b)),
        (" x ", |a, b| Some(a * b)),
        (" / ", |a, b| if b != 0 { Some(a / b) } else { None }),
        (" divided by ", |a, b| if b != 0 { Some(a / b) } else { None }),
        (" % ", |a, b| if b != 0 { Some(a % b) } else { None }),
        (" mod ", |a, b| if b != 0 { Some(a % b) } else { None }),
        (" ^ ", |a, b| Some(a.pow(b as u32))),
        (" power ", |a, b| Some(a.pow(b as u32))),
    ];

    for (op_str, op_fn) in ops {
        if let Some(pos) = q.find(op_str) {
            let left = q[..pos].trim().parse::<i64>().ok()?;
            let right = q[pos + op_str.len()..].trim().parse::<i64>().ok()?;
            let result = op_fn(left, right)?;
            return Some(format!("{left} {op} {right} = {result}",
                op = op_str.trim()));
        }
    }

    // Try factorial: "factorial of N" or "N!"
    if q.contains("factorial") || q.ends_with('!') {
        let num_str = q.replace("factorial of ", "").replace("factorial ", "").replace('!', "");
        if let Ok(n) = num_str.trim().parse::<u64>() {
            if n <= 20 {
                let result: u64 = (1..=n).product();
                return Some(format!("{n}! = {result}"));
            }
        }
    }

    // Try "sqrt N" or "square root of N"
    if q.contains("sqrt") || q.contains("square root") {
        let num_str = q.replace("square root of ", "").replace("sqrt ", "").replace("sqrt", "");
        if let Ok(n) = num_str.trim().parse::<f64>() {
            let result = n.sqrt();
            return Some(format!("sqrt({n}) = {result}"));
        }
    }

    None
}

/// SYS_MATH_EVAL: evaluate a math expression string and push integer result
fn math_eval_str(expr: &str) -> Option<i64> {
    let expr = expr.trim();
    if let Some(result) = try_math_eval(&expr.to_lowercase()) {
        if let Some(pos) = result.rfind('=') {
            let num = result[pos + 1..].trim();
            return num.parse::<i64>().ok();
        }
    }
    expr.parse::<i64>().ok()
}
// ---------------------------------------------------------------------------

struct ParsedUrl {
    host: String,
    port: u16,
    path: String,
}

fn parse_url(url: &str) -> Option<ParsedUrl> {
    let rest = url.strip_prefix("http://")?;
    let (host_port, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let (host, port) = match host_port.find(':') {
        Some(i) => (&host_port[..i], host_port[i+1..].parse::<u16>().ok()?),
        None => (host_port, 80),
    };
    Some(ParsedUrl { host: host.to_string(), port, path: path.to_string() })
}

fn http_request_with_headers(method: &str, url: &str, body: &[u8], timeout_ms: u64, extra_headers: &[(String, String)]) -> Result<Vec<u8>, String> {
    let parsed = parse_url(url).ok_or_else(|| "invalid URL (http:// only)".to_string())?;
    let addr = format!("{}:{}", parsed.host, parsed.port);

    let timeout = std::time::Duration::from_millis(timeout_ms);
    let stream = std::net::TcpStream::connect_timeout(
        &addr.parse::<std::net::SocketAddr>()
            .or_else(|_| {
                use std::net::ToSocketAddrs;
                addr.to_socket_addrs()
                    .map_err(|e| e.to_string())?
                    .next()
                    .ok_or_else(|| "DNS resolution failed".to_string())
            })?,
        timeout,
    ).map_err(|e| e.to_string())?;

    stream.set_read_timeout(Some(timeout)).ok();
    stream.set_write_timeout(Some(timeout)).ok();
    let mut stream = stream;

    let mut hdr_str = String::new();
    if !body.is_empty() {
        hdr_str.push_str(&format!("Content-Length: {}\r\n", body.len()));
        hdr_str.push_str("Content-Type: application/json\r\n");
    }
    hdr_str.push_str("Accept: application/json\r\n");
    for (k, v) in extra_headers {
        hdr_str.push_str(&format!("{k}: {v}\r\n"));
    }

    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\n{hdr_str}Connection: close\r\n\r\n",
        method = method, path = parsed.path, host = parsed.host,
    );
    stream.write_all(request.as_bytes()).map_err(|e| e.to_string())?;
    if !body.is_empty() {
        stream.write_all(body).map_err(|e| e.to_string())?;
    }

    let mut response = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => response.extend_from_slice(&buf[..n]),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => break,
            Err(e) => return Err(e.to_string()),
        }
        if response.len() > 64 * 1024 { break; }
    }

    let header_end = response.windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|i| i + 4)
        .unwrap_or(response.len());

    let headers_str = String::from_utf8_lossy(&response[..header_end]);
    let body_bytes = &response[header_end..];

    if headers_str.to_lowercase().contains("transfer-encoding: chunked") {
        Ok(decode_chunked(body_bytes))
    } else {
        Ok(body_bytes.to_vec())
    }
}

/// Decode HTTP chunked transfer encoding.
pub fn decode_chunked(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    let mut pos = 0;
    while pos < data.len() {
        let end_of_size = data[pos..].windows(2).position(|w| w == b"\r\n");
        let end_of_size = match end_of_size { Some(e) => pos + e, None => break };
        let hex = std::str::from_utf8(&data[pos..end_of_size]).unwrap_or("0").trim();
        let size = usize::from_str_radix(hex, 16).unwrap_or(0);
        if size == 0 { break; }
        let data_start = end_of_size + 2;
        if data_start + size > data.len() { break; }
        result.extend_from_slice(&data[data_start..data_start + size]);
        pos = data_start + size + 2;
    }
    result
}



// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ghost_vm::*;

    fn make_world_host() -> WorldHost {
        let mut h = WorldHost::new();
        h.allow_files = true;
        h.allow_http = true;
        h.allow_env = true;
        h.mock_fs = Some(std::collections::HashMap::new());
        h.mock_http = Some(std::collections::HashMap::new());
        h
    }

    // --- Data Processing ---

    #[test]
    fn sys_parse_int_roundtrip() {
        let mut c = Capsule::default();
        c.ram[0..3].copy_from_slice(b"123");
        c.code = vec![
            OP_PUSH, 0, 0, 0, 0,       // str_addr = 0
            OP_PUSH, 3, 0, 0, 0,       // str_len = 3
            OP_SYSCALL, SYS_PARSE_INT,
            OP_HALT,
        ];
        let mut h = make_world_host();
        let st = run(&mut c, &mut h, Some(100)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![123]);
    }

    #[test]
    fn sys_format_int_roundtrip() {
        let mut c = Capsule::default();
        c.code = vec![
            OP_PUSH, 0, 2, 0, 0,       // dest_addr = 512
            OP_PUSH, 42, 0, 0, 0,      // value = 42
            OP_SYSCALL, SYS_FORMAT_INT,
            OP_HALT,
        ];
        let mut h = make_world_host();
        let st = run(&mut c, &mut h, Some(100)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![2]); // "42" = 2 chars
        assert_eq!(&c.ram[512..514], b"42");
    }

    #[test]
    fn sys_mem_copy() {
        let mut c = Capsule::default();
        c.ram[0..5].copy_from_slice(b"hello");
        c.code = vec![
            OP_PUSH, 5, 0, 0, 0,       // length
            OP_PUSH, 0, 1, 0, 0,       // dest_addr = 256
            OP_PUSH, 0, 0, 0, 0,       // src_addr = 0
            OP_SYSCALL, SYS_MEM_COPY,
            OP_HALT,
        ];
        let mut h = make_world_host();
        let st = run(&mut c, &mut h, Some(100)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![5]);
        assert_eq!(&c.ram[256..261], b"hello");
    }

    #[test]
    fn sys_mem_find() {
        let mut c = Capsule::default();
        c.ram[0..11].copy_from_slice(b"hello world");
        c.ram[100..105].copy_from_slice(b"world");
        c.code = vec![
            OP_PUSH, 0, 0, 0, 0,       // haystack_addr
            OP_PUSH, 11, 0, 0, 0,      // haystack_len
            OP_PUSH, 100, 0, 0, 0,     // needle_addr
            OP_PUSH, 5, 0, 0, 0,       // needle_len
            OP_SYSCALL, SYS_MEM_FIND,
            OP_HALT,
        ];
        let mut h = make_world_host();
        let st = run(&mut c, &mut h, Some(100)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![6]); // "world" found at offset 6
    }

    #[test]
    fn sys_mem_find_not_found() {
        let mut c = Capsule::default();
        c.ram[0..5].copy_from_slice(b"hello");
        c.ram[100..103].copy_from_slice(b"xyz");
        c.code = vec![
            OP_PUSH, 0, 0, 0, 0,
            OP_PUSH, 5, 0, 0, 0,
            OP_PUSH, 100, 0, 0, 0,
            OP_PUSH, 3, 0, 0, 0,
            OP_SYSCALL, SYS_MEM_FIND,
            OP_HALT,
        ];
        let mut h = make_world_host();
        let st = run(&mut c, &mut h, Some(100)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![-1]);
    }

    #[test]
    fn sys_mem_fill() {
        let mut c = Capsule::default();
        c.code = vec![
            OP_PUSH, 0x41, 0, 0, 0,    // value = 65 ('A')
            OP_PUSH, 5, 0, 0, 0,       // length = 5
            OP_PUSH, 0, 1, 0, 0,       // addr = 256
            OP_SYSCALL, SYS_MEM_FILL,
            OP_HALT,
        ];
        let mut h = make_world_host();
        let st = run(&mut c, &mut h, Some(100)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![5]);
        assert_eq!(&c.ram[256..261], b"AAAAA");
    }

    #[test]
    fn sys_hash_ram() {
        let mut c = Capsule::default();
        c.ram[0..5].copy_from_slice(b"hello");
        c.code = vec![
            OP_PUSH, 5, 0, 0, 0,       // length
            OP_PUSH, 0, 0, 0, 0,       // addr
            OP_SYSCALL, SYS_HASH_RAM,
            OP_HALT,
        ];
        let mut h = make_world_host();
        let st = run(&mut c, &mut h, Some(100)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![32]); // SHA-256 = 32 bytes written
        // The hash overwrites RAM[0..32]. Verify it's non-zero.
        assert_ne!(&c.ram[0..32], &[0u8; 32]);
    }

    // --- File I/O (mock) ---

    #[test]
    fn sys_file_write_and_read() {
        let mut h = make_world_host();
        // Write "ghost data" to file "test.txt"
        let mut c = Capsule::default();
        c.ram[0..8].copy_from_slice(b"test.txt");
        c.ram[100..110].copy_from_slice(b"ghost data");
        c.code = vec![
            OP_PUSH, 100, 0, 0, 0,     // data_addr
            OP_PUSH, 10, 0, 0, 0,      // data_len
            OP_PUSH, 0, 0, 0, 0,       // filename_addr
            OP_PUSH, 8, 0, 0, 0,       // filename_len
            OP_SYSCALL, SYS_FILE_WRITE,
            OP_HALT,
        ];
        let st = run(&mut c, &mut h, Some(200)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![10]); // 10 bytes written
        assert_eq!(h.mock_fs.as_ref().unwrap().get("test.txt").unwrap(), b"ghost data");

        // Now read it back
        c.pc = 0;
        c.stack.clear();
        c.code = vec![
            OP_PUSH, 0, 2, 0, 0,       // dest_addr = 512
            OP_PUSH, 0, 0, 0, 0,       // filename_addr
            OP_PUSH, 8, 0, 0, 0,       // filename_len
            OP_SYSCALL, SYS_FILE_READ,
            OP_HALT,
        ];
        let st = run(&mut c, &mut h, Some(200)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![10]);
        assert_eq!(&c.ram[512..522], b"ghost data");
    }

    #[test]
    fn sys_file_exists() {
        let mut h = make_world_host();
        h.mock_fs.as_mut().unwrap().insert("exists.txt".into(), vec![1, 2, 3]);
        let mut c = Capsule::default();
        c.ram[0..10].copy_from_slice(b"exists.txt");
        c.ram[50..58].copy_from_slice(b"nope.txt");
        c.code = vec![
            OP_PUSH, 0, 0, 0, 0,
            OP_PUSH, 10, 0, 0, 0,
            OP_SYSCALL, SYS_FILE_EXISTS,
            OP_PUSH, 50, 0, 0, 0,
            OP_PUSH, 8, 0, 0, 0,
            OP_SYSCALL, SYS_FILE_EXISTS,
            OP_HALT,
        ];
        let st = run(&mut c, &mut h, Some(200)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![1, 0]); // exists, not exists
    }

    #[test]
    fn sys_file_size() {
        let mut h = make_world_host();
        h.mock_fs.as_mut().unwrap().insert("sz.txt".into(), vec![0; 42]);
        let mut c = Capsule::default();
        c.ram[0..6].copy_from_slice(b"sz.txt");
        c.code = vec![
            OP_PUSH, 0, 0, 0, 0,
            OP_PUSH, 6, 0, 0, 0,
            OP_SYSCALL, SYS_FILE_SIZE,
            OP_HALT,
        ];
        let st = run(&mut c, &mut h, Some(200)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![42]);
    }

    // --- HTTP (mock) ---

    #[test]
    fn sys_http_get_mock() {
        let mut h = make_world_host();
        h.mock_http.as_mut().unwrap().insert(
            "http://example.com/".into(), b"Hello Ghost!".to_vec(),
        );
        let mut c = Capsule::default();
        let url = b"http://example.com/";
        c.ram[0..url.len()].copy_from_slice(url);
        c.code = vec![
            OP_PUSH, 0, 2, 0, 0,                   // dest_addr = 512
            OP_PUSH, 0, 0, 0, 0,                   // url_addr
            OP_PUSH, url.len() as u8, 0, 0, 0,     // url_len
            OP_SYSCALL, SYS_HTTP_GET,
            OP_HALT,
        ];
        let st = run(&mut c, &mut h, Some(200)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![12]); // "Hello Ghost!" = 12 bytes
        assert_eq!(&c.ram[512..524], b"Hello Ghost!");
    }

    // --- Time ---

    #[test]
    fn sys_time_now() {
        let mut c = Capsule::default();
        c.code = vec![OP_SYSCALL, SYS_TIME_NOW, OP_HALT];
        let mut h = make_world_host();
        let st = run(&mut c, &mut h, Some(50)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack.len(), 1);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert!((c.stack[0] - now).abs() <= 2);
    }

    #[test]
    fn sys_time_ms() {
        let mut c = Capsule::default();
        c.code = vec![OP_SYSCALL, SYS_TIME_MS, OP_HALT];
        let mut h = make_world_host();
        let st = run(&mut c, &mut h, Some(50)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack.len(), 1);
        assert!(c.stack[0] >= 0);
    }

    #[test]
    fn sys_platform() {
        let mut c = Capsule::default();
        c.code = vec![OP_SYSCALL, SYS_PLATFORM, OP_HALT];
        let mut h = make_world_host();
        let st = run(&mut c, &mut h, Some(50)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack.len(), 1);
        let code = c.stack[0];
        assert!(code >= 0 && code <= 3);
    }

    // --- Pages ---

    #[test]
    fn page_alloc_write_read_roundtrip() {
        let mut c = Capsule::default();
        c.code = vec![
            OP_SYSCALL, SYS_PAGE_ALLOC,     // → page_id (0)
            OP_DUP,
            OP_DUP,
            // PAGE_WRITE: pop page_id, pop offset, pop value
            OP_PUSH, 0, 0, 0, 0,            // offset = 0
            OP_PUSH, 99, 0, 0, 0,           // value = 99
            OP_SYSCALL, SYS_PAGE_WRITE,
            // PAGE_READ: pop page_id, pop offset
            OP_PUSH, 0, 0, 0, 0,            // offset = 0
            OP_SYSCALL, SYS_PAGE_READ,
            OP_HALT,
        ];
        let mut h = make_world_host();
        let st = run(&mut c, &mut h, Some(200)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(*c.stack.last().unwrap(), 99);
        assert!(c.pages[0].is_some());
    }

    #[test]
    fn page_alloc_and_free() {
        let mut c = Capsule::default();
        c.code = vec![
            OP_SYSCALL, SYS_PAGE_ALLOC,     // → page_id 0
            OP_SYSCALL, SYS_PAGE_FREE,      // free page 0
            OP_SYSCALL, SYS_PAGE_ALLOC,     // → should reuse slot 0
            OP_HALT,
        ];
        let mut h = make_world_host();
        let st = run(&mut c, &mut h, Some(200)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![0]); // re-allocated at index 0
    }

    #[test]
    fn page_copy() {
        let mut c = Capsule::default();
        c.code = vec![
            // Alloc page 0 and page 1
            OP_SYSCALL, SYS_PAGE_ALLOC,     // → 0
            OP_SYSCALL, SYS_PAGE_ALLOC,     // → 1
            // Write 42 to page 0 offset 0
            OP_PUSH, 0, 0, 0, 0,           // page_id = 0
            OP_PUSH, 0, 0, 0, 0,           // offset = 0
            OP_PUSH, 42, 0, 0, 0,          // value = 42
            OP_SYSCALL, SYS_PAGE_WRITE,
            // Copy 4 bytes from page 0 off 0 to page 1 off 0
            OP_PUSH, 4, 0, 0, 0,           // len
            OP_PUSH, 0, 0, 0, 0,           // dst_off
            OP_PUSH, 1, 0, 0, 0,           // dst_page
            OP_PUSH, 0, 0, 0, 0,           // src_off
            OP_PUSH, 0, 0, 0, 0,           // src_page
            OP_SYSCALL, SYS_PAGE_COPY,
            // Read from page 1 offset 0
            OP_PUSH, 1, 0, 0, 0,           // page_id = 1
            OP_PUSH, 0, 0, 0, 0,           // offset = 0
            OP_SYSCALL, SYS_PAGE_READ,
            OP_HALT,
        ];
        let mut h = make_world_host();
        let st = run(&mut c, &mut h, Some(300)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(*c.stack.last().unwrap(), 42);
    }

    // --- Sandbox ---

    #[test]
    fn sandbox_rejects_path_traversal() {
        let mut h = WorldHost::new();
        h.allow_files = true;
        h.sandbox_root = Some(PathBuf::from(std::env::temp_dir().join("ghost_sandbox")));
        assert!(h.resolve_path("../../etc/passwd").is_err());
        assert!(h.resolve_path("../secret").is_err());
        // Normal relative paths should succeed
        assert!(h.resolve_path("data.txt").is_ok());
        assert!(h.resolve_path("sub/file.txt").is_ok());
    }

    // --- Permission denied ---

    #[test]
    fn file_denied_when_not_allowed() {
        let mut c = Capsule::default();
        c.ram[0..4].copy_from_slice(b"test");
        c.code = vec![
            OP_PUSH, 0, 0, 0, 0,
            OP_PUSH, 4, 0, 0, 0,
            OP_SYSCALL, SYS_FILE_EXISTS,
            OP_HALT,
        ];
        let mut h = WorldHost::new(); // allow_files = false
        let e = run(&mut c, &mut h, Some(100)).unwrap_err();
        assert!(matches!(e, GhostError::SyscallDenied(SYS_FILE_EXISTS)));
    }

    #[test]
    fn http_denied_when_not_allowed() {
        let mut c = Capsule::default();
        c.ram[0..19].copy_from_slice(b"http://example.com/");
        c.code = vec![
            OP_PUSH, 0, 2, 0, 0,
            OP_PUSH, 0, 0, 0, 0,
            OP_PUSH, 19, 0, 0, 0,
            OP_SYSCALL, SYS_HTTP_GET,
            OP_HALT,
        ];
        let mut h = WorldHost::new(); // allow_http = false
        let e = run(&mut c, &mut h, Some(100)).unwrap_err();
        assert!(matches!(e, GhostError::SyscallDenied(SYS_HTTP_GET)));
    }

    #[test]
    fn env_denied_when_not_allowed() {
        let mut c = Capsule::default();
        c.ram[0..4].copy_from_slice(b"PATH");
        c.code = vec![
            OP_PUSH, 0, 2, 0, 0,
            OP_PUSH, 0, 0, 0, 0,
            OP_PUSH, 4, 0, 0, 0,
            OP_SYSCALL, SYS_ENV_GET,
            OP_HALT,
        ];
        let mut h = WorldHost::new(); // allow_env = false
        let e = run(&mut c, &mut h, Some(100)).unwrap_err();
        assert!(matches!(e, GhostError::SyscallDenied(SYS_ENV_GET)));
    }

    // --- Capsule pages encode/decode roundtrip ---

    #[test]
    fn capsule_pages_roundtrip() {
        let mut c = Capsule::default();
        c.pages.push(Some(vec![42u8; PAGE_SIZE]));
        c.pages.push(None);
        c.pages.push(Some(vec![7u8; PAGE_SIZE]));

        let bytes = c.encode().unwrap();
        let c2 = Capsule::decode(&bytes).unwrap();
        assert_eq!(c2.pages.len(), 3);
        assert!(c2.pages[0].is_some());
        assert!(c2.pages[1].is_none());
        assert!(c2.pages[2].is_some());
        assert_eq!(c2.pages[0].as_ref().unwrap()[0], 42);
        assert_eq!(c2.pages[2].as_ref().unwrap()[0], 7);
    }

    #[test]
    fn capsule_no_pages_backward_compat() {
        let c = Capsule::default(); // no pages
        let bytes = c.encode().unwrap();
        let c2 = Capsule::decode(&bytes).unwrap();
        assert!(c2.pages.is_empty());
        assert_eq!(c2.code, c.code);
    }

    // --- parse_url ---

    #[test]
    fn test_parse_url() {
        let p = parse_url("http://example.com/path").unwrap();
        assert_eq!(p.host, "example.com");
        assert_eq!(p.port, 80);
        assert_eq!(p.path, "/path");

        let p2 = parse_url("http://localhost:8080/api/data").unwrap();
        assert_eq!(p2.host, "localhost");
        assert_eq!(p2.port, 8080);
        assert_eq!(p2.path, "/api/data");

        assert!(parse_url("https://secure.com/").is_none());
        assert!(parse_url("ftp://files.com/").is_none());
    }

    #[test]
    fn sys_argv_access() {
        let mut h = make_world_host();
        h.argv = vec!["ghost_vm".into(), "world".into(), "test.ghst".into()];
        let mut c = Capsule::default();
        c.code = vec![
            OP_PUSH, 0, 2, 0, 0,       // dest_addr = 512
            OP_PUSH, 1, 0, 0, 0,       // index = 1
            OP_SYSCALL, SYS_ARGV,
            OP_HALT,
        ];
        let st = run(&mut c, &mut h, Some(100)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![5]); // "world" = 5
        assert_eq!(&c.ram[512..517], b"world");
    }

    #[test]
    fn sys_argv_out_of_range() {
        let mut h = make_world_host();
        h.argv = vec!["test".into()];
        let mut c = Capsule::default();
        c.code = vec![
            OP_PUSH, 0, 0, 0, 0,
            OP_PUSH, 99, 0, 0, 0,      // index 99 → out of range
            OP_SYSCALL, SYS_ARGV,
            OP_HALT,
        ];
        let st = run(&mut c, &mut h, Some(100)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![-1]);
    }

    // --- Knowledge / AI syscall tests ---

    #[test]
    fn sys_ask_who_are_you() {
        let mut h = make_world_host();
        let mut c = Capsule::default();
        let q = b"who are you?";
        c.ram[0..q.len()].copy_from_slice(q);
        // dest_addr=256, q_addr=0, q_len=12
        c.code = vec![
            OP_PUSH, 0, 1, 0, 0,       // push 256 (dest_addr)
            OP_PUSH, 0, 0, 0, 0,       // push 0   (q_addr)
            OP_PUSH, q.len() as u8, 0, 0, 0,  // push 12  (q_len)
            OP_SYSCALL, SYS_ASK,
            OP_HALT,
        ];
        let st = run(&mut c, &mut h, Some(200)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        let ans_len = c.stack[0] as usize;
        assert!(ans_len > 10);
        let answer = std::str::from_utf8(&c.ram[256..256 + ans_len]).unwrap();
        assert!(answer.contains("Ghost"));
    }

    #[test]
    fn sys_ask_math() {
        let mut h = make_world_host();
        let mut c = Capsule::default();
        let q = b"what is 7 + 5";
        c.ram[0..q.len()].copy_from_slice(q);
        c.code = vec![
            OP_PUSH, 0, 1, 0, 0,
            OP_PUSH, 0, 0, 0, 0,
            OP_PUSH, q.len() as u8, 0, 0, 0,
            OP_SYSCALL, SYS_ASK,
            OP_HALT,
        ];
        let st = run(&mut c, &mut h, Some(200)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        let ans_len = c.stack[0] as usize;
        let answer = std::str::from_utf8(&c.ram[256..256 + ans_len]).unwrap();
        assert!(answer.contains("12"));
    }

    #[test]
    fn sys_ask_unknown() {
        let mut h = make_world_host();
        let mut c = Capsule::default();
        let q = b"xyzzy nonsense";
        c.ram[0..q.len()].copy_from_slice(q);
        c.code = vec![
            OP_PUSH, 0, 1, 0, 0,
            OP_PUSH, 0, 0, 0, 0,
            OP_PUSH, q.len() as u8, 0, 0, 0,
            OP_SYSCALL, SYS_ASK,
            OP_HALT,
        ];
        let st = run(&mut c, &mut h, Some(200)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        let ans_len = c.stack[0] as usize;
        let answer = std::str::from_utf8(&c.ram[256..256 + ans_len]).unwrap();
        assert!(answer.contains("don't know"));
    }

    #[test]
    fn sys_math_eval_direct() {
        let mut h = make_world_host();
        let mut c = Capsule::default();
        let expr = b"10 * 5";
        c.ram[0..expr.len()].copy_from_slice(expr);
        c.code = vec![
            OP_PUSH, 0, 0, 0, 0,       // expr_addr=0
            OP_PUSH, expr.len() as u8, 0, 0, 0, // expr_len
            OP_SYSCALL, SYS_MATH_EVAL,
            OP_HALT,
        ];
        let st = run(&mut c, &mut h, Some(200)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![50]);
    }

    #[test]
    fn sys_str_lower() {
        let mut h = make_world_host();
        let mut c = Capsule::default();
        c.ram[0..5].copy_from_slice(b"HELLO");
        c.code = vec![
            OP_PUSH, 0, 0, 0, 0,       // addr=0
            OP_PUSH, 5, 0, 0, 0,       // len=5
            OP_SYSCALL, SYS_STR_LOWER,
            OP_HALT,
        ];
        let st = run(&mut c, &mut h, Some(200)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(&c.ram[0..5], b"hello");
        assert_eq!(c.stack, vec![5]);
    }

    #[test]
    fn sys_str_eq_match() {
        let mut h = make_world_host();
        let mut c = Capsule::default();
        c.ram[0..3].copy_from_slice(b"foo");
        c.ram[10..13].copy_from_slice(b"foo");
        c.code = vec![
            OP_PUSH, 0, 0, 0, 0,       // addr1=0
            OP_PUSH, 3, 0, 0, 0,       // len1=3
            OP_PUSH, 10, 0, 0, 0,      // addr2=10
            OP_PUSH, 3, 0, 0, 0,       // len2=3
            OP_SYSCALL, SYS_STR_EQ,
            OP_HALT,
        ];
        let st = run(&mut c, &mut h, Some(200)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![1]);
    }

    #[test]
    fn sys_str_eq_mismatch() {
        let mut h = make_world_host();
        let mut c = Capsule::default();
        c.ram[0..3].copy_from_slice(b"foo");
        c.ram[10..13].copy_from_slice(b"bar");
        c.code = vec![
            OP_PUSH, 0, 0, 0, 0,
            OP_PUSH, 3, 0, 0, 0,
            OP_PUSH, 10, 0, 0, 0,
            OP_PUSH, 3, 0, 0, 0,
            OP_SYSCALL, SYS_STR_EQ,
            OP_HALT,
        ];
        let st = run(&mut c, &mut h, Some(200)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![0]);
    }

    #[test]
    fn knowledge_answer_identity() {
        let a = knowledge_answer("Who are you?");
        assert!(a.contains("Ghost"));
        let a = knowledge_answer("who made you?");
        assert!(a.contains("Arun"));
    }

    #[test]
    fn knowledge_answer_math_eval() {
        let a = knowledge_answer("what is 100 + 200");
        assert!(a.contains("300"));
        let a = knowledge_answer("calculate 6 * 7");
        assert!(a.contains("42"));
        let a = knowledge_answer("factorial of 5");
        assert!(a.contains("120"));
    }

    #[test]
    fn knowledge_answer_science() {
        let a = knowledge_answer("what is pi");
        assert!(a.contains("3.14"));
        let a = knowledge_answer("speed of light");
        assert!(a.contains("299"));
    }

    #[test]
    fn try_math_eval_ops() {
        assert!(try_math_eval("2 + 3").unwrap().contains("5"));
        assert!(try_math_eval("10 - 3").unwrap().contains("7"));
        assert!(try_math_eval("6 * 7").unwrap().contains("42"));
        assert!(try_math_eval("100 / 4").unwrap().contains("25"));
        assert!(try_math_eval("17 % 5").unwrap().contains("2"));
        assert!(try_math_eval("2 ^ 10").unwrap().contains("1024"));
        assert!(try_math_eval("sqrt 144").unwrap().contains("12"));
    }

    #[test]
    fn test_chunked_decode() {
        let data = b"4\r\nWiki\r\n5\r\npedia\r\n0\r\n\r\n";
        let decoded = decode_chunked(data);
        assert_eq!(String::from_utf8_lossy(&decoded), "Wikipedia");
    }

    #[test]
    fn test_chunked_decode_single() {
        let data = b"b\r\nhello world\r\n0\r\n\r\n";
        let decoded = decode_chunked(data);
        assert_eq!(String::from_utf8_lossy(&decoded), "hello world");
    }

    #[test]
    fn test_xorshift_rand_range() {
        use crate::ghost_vm::{assemble_capsule, run};
        let src = "push 1\npush 6\nsyscall 86\nhalt\n";
        let mut results: Vec<i64> = Vec::new();
        for seed in 1..101u64 {
            let mut c = assemble_capsule(src).unwrap();
            let mut h = WorldHost::new();
            h.rng_state = seed;
            let st = run(&mut c, &mut h, Some(50)).unwrap();
            assert_eq!(st, crate::ghost_vm::RunStatus::Halted);
            let v = *c.stack.last().unwrap();
            assert!(v >= 1 && v <= 6, "rand_range out of [1,6]: {v}");
            results.push(v);
        }
        let unique: std::collections::HashSet<i64> = results.into_iter().collect();
        assert!(unique.len() > 1, "all rand results identical");
    }

    #[test]
    fn test_rand_seed_deterministic() {
        use crate::ghost_vm::{assemble_capsule, run};
        let src = "push 42\nsyscall 85\nsyscall 22\nhalt\n";
        let mut c1 = assemble_capsule(src).unwrap();
        let mut h1 = WorldHost::new();
        run(&mut c1, &mut h1, Some(50)).unwrap();
        let r1 = *c1.stack.last().unwrap();

        let mut c2 = assemble_capsule(src).unwrap();
        let mut h2 = WorldHost::new();
        run(&mut c2, &mut h2, Some(50)).unwrap();
        let r2 = *c2.stack.last().unwrap();
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_output_capture() {
        use crate::ghost_vm::{assemble_capsule, run};
        let src = "push 42\nsyscall 3\nhalt\n";
        let mut c = assemble_capsule(src).unwrap();
        let mut h = WorldHost::new();
        h.output_capture = Some(Vec::new());
        run(&mut c, &mut h, Some(50)).unwrap();
        let out = h.get_captured_output();
        assert_eq!(out.trim(), "42");
    }
}
