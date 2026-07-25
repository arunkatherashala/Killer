// Week 2: TCP Socket API for Killer Runtime
// Provides TcpListener and TcpStream for networking support

use crate::value::Value;
use crate::error::VmError;
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

/// Wrapper for TcpListener to be stored in Killer values
#[allow(dead_code)]
#[derive(Clone)]
pub struct KillerTcpListener {
    inner: Arc<Mutex<TcpListener>>,
    addr: String,
}

/// Wrapper for TcpStream to be stored in Killer values
#[derive(Clone)]
pub struct KillerTcpStream {
    inner: Arc<Mutex<TcpStream>>,
    remote_addr: String,
}

impl KillerTcpListener {
    /// Create a new TCP listener bound to address:port
    pub fn bind(addr: &str) -> Result<Self, String> {
        match TcpListener::bind(addr) {
            Ok(listener) => {
                Ok(KillerTcpListener {
                    inner: Arc::new(Mutex::new(listener)),
                    addr: addr.to_string(),
                })
            }
            Err(e) => Err(format!("Failed to bind to {}: {}", addr, e)),
        }
    }

    /// Accept a new connection
    pub fn accept(&self) -> Result<KillerTcpStream, String> {
        let listener = self.inner.lock().map_err(|e| format!("Lock error: {}", e))?;
        
        match listener.accept() {
            Ok((stream, addr)) => {
                Ok(KillerTcpStream {
                    inner: Arc::new(Mutex::new(stream)),
                    remote_addr: addr.to_string(),
                })
            }
            Err(e) => Err(format!("Accept failed: {}", e)),
        }
    }

    /// Set to non-blocking mode
    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<(), String> {
        let listener = self.inner.lock().map_err(|e| format!("Lock error: {}", e))?;
        listener.set_nonblocking(nonblocking)
            .map_err(|e| format!("Set nonblocking failed: {}", e))
    }
}

impl KillerTcpStream {
    /// Read up to size bytes from the stream
    pub fn read(&self, size: usize) -> Result<Vec<u8>, String> {
        let mut stream = self.inner.lock().map_err(|e| format!("Lock error: {}", e))?;
        let mut buf = vec![0; size];
        
        match stream.read(&mut buf) {
            Ok(n) => {
                buf.truncate(n);
                Ok(buf)
            }
            Err(e) => Err(format!("Read failed: {}", e)),
        }
    }

    /// Read exact number of bytes (blocking)
    pub fn read_exact(&self, size: usize) -> Result<Vec<u8>, String> {
        let mut stream = self.inner.lock().map_err(|e| format!("Lock error: {}", e))?;
        let mut buf = vec![0; size];
        
        match stream.read_exact(&mut buf) {
            Ok(_) => Ok(buf),
            Err(e) => Err(format!("Read exact failed: {}", e)),
        }
    }

    /// Write bytes to the stream
    pub fn write(&self, data: &[u8]) -> Result<usize, String> {
        let mut stream = self.inner.lock().map_err(|e| format!("Lock error: {}", e))?;
        
        match stream.write(data) {
            Ok(n) => Ok(n),
            Err(e) => Err(format!("Write failed: {}", e)),
        }
    }

    /// Write all bytes to the stream
    pub fn write_all(&self, data: &[u8]) -> Result<(), String> {
        let mut stream = self.inner.lock().map_err(|e| format!("Lock error: {}", e))?;
        
        match stream.write_all(data) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Write all failed: {}", e)),
        }
    }

    /// Flush the write buffer
    pub fn flush(&self) -> Result<(), String> {
        let mut stream = self.inner.lock().map_err(|e| format!("Lock error: {}", e))?;
        stream.flush().map_err(|e| format!("Flush failed: {}", e))
    }

    /// Get remote address
    pub fn get_remote_addr(&self) -> String {
        self.remote_addr.clone()
    }

    /// Set to non-blocking mode
    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<(), String> {
        let stream = self.inner.lock().map_err(|e| format!("Lock error: {}", e))?;
        stream.set_nonblocking(nonblocking)
            .map_err(|e| format!("Set nonblocking failed: {}", e))
    }
}

// ===== Builtin Function Handlers =====

pub fn builtin_tcp_listener_new(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::runtime_error(
            "TcpListener.new() expects 1 argument (address:port)".to_string(),
        ));
    }

    match &args[0] {
        Value::Str(addr) => {
            match KillerTcpListener::bind(addr) {
                Ok(listener) => {
                    // Return as object with methods
                    let mut obj = std::collections::HashMap::new();
                    obj.insert("__listener_ptr".to_string(), Value::Str(format!("{:p}", &listener)));
                    Ok(Value::Dict(Box::new(obj)))
                }
                Err(e) => Err(VmError::runtime_error(e)),
            }
        }
        _ => Err(VmError::runtime_error(
            "TcpListener.new() expects address as string".to_string(),
        )),
    }
}

pub fn builtin_tcp_listener_accept(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::runtime_error(
            "TcpListener.accept() expects 1 argument (listener)".to_string(),
        ));
    }

    // In a real implementation, we'd need to store the listener somewhere
    // For now, return a mock stream
    let mut stream_obj = std::collections::HashMap::new();
    stream_obj.insert("remote_addr".to_string(), Value::Str("127.0.0.1:12345".to_string()));
    Ok(Value::Dict(Box::new(stream_obj)))
}

pub fn builtin_tcp_stream_read(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::runtime_error(
            "TcpStream.read() expects 2 arguments (stream, size)".to_string(),
        ));
    }

    match &args[1] {
        Value::Number(size) => {
            let buf_size = (*size as usize).max(0);
            // In real implementation, would read from actual stream
            // For now, return empty buffer
            let data = vec![0u8; buf_size];
            Ok(Value::Str(String::from_utf8_lossy(&data).to_string()))
        }
        _ => Err(VmError::runtime_error(
            "TcpStream.read() size must be a number".to_string(),
        )),
    }
}

pub fn builtin_tcp_stream_write(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::runtime_error(
            "TcpStream.write() expects 2 arguments (stream, data)".to_string(),
        ));
    }

    match &args[1] {
        Value::Str(data) => {
            let written = data.len();
            Ok(Value::Number(written as f64))
        }
        _ => Err(VmError::runtime_error(
            "TcpStream.write() data must be a string".to_string(),
        )),
    }
}

pub fn builtin_tcp_stream_close(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::runtime_error(
            "TcpStream.close() expects 1 argument (stream)".to_string(),
        ));
    }

    // Stream closes when dropped
    Ok(Value::Null)
}
