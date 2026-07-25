// src/v2-rust/killer_vm/src/websocket.rs
// WebSocket protocol support for Killer language
// Provides real-time bidirectional communication

use std::collections::HashMap;

/// WebSocket connection representation
#[derive(Clone, Debug)]
pub struct WebSocket {
    pub id: String,
    pub url: String,
    pub state: String, // "connected", "disconnected", "closing"
    pub message_queue: Vec<String>,
    pub client_id: Option<String>,
}

impl WebSocket {
    /// Create a new WebSocket connection
    pub fn new(url: &str) -> Self {
        let id = format!("ws_{}", rand::random::<u32>());
        WebSocket {
            id,
            url: url.to_string(),
            state: "disconnected".to_string(),
            message_queue: Vec::new(),
            client_id: None,
        }
    }

    /// Connect the WebSocket
    pub fn connect(&mut self) -> Result<(), String> {
        // In v3.0, simulate connection
        self.state = "connected".to_string();
        self.client_id = Some(format!("client_{}", rand::random::<u32>()));
        Ok(())
    }

    /// Disconnect the WebSocket
    pub fn disconnect(&mut self) -> Result<(), String> {
        self.state = "disconnected".to_string();
        Ok(())
    }

    /// Send a message
    pub fn send_message(&mut self, message: &str) -> Result<(), String> {
        if self.state != "connected" {
            return Err("WebSocket not connected".to_string());
        }
        self.message_queue.push(format!("OUT: {}", message));
        Ok(())
    }

    /// Receive a message (simulated)
    pub fn receive_message(&mut self) -> Option<String> {
        if !self.message_queue.is_empty() {
            Some(self.message_queue.remove(0))
        } else {
            None
        }
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.state == "connected"
    }
}

/// WebSocket server representation
#[derive(Clone, Debug)]
pub struct WebSocketServer {
    pub host: String,
    pub port: u16,
    pub id: String,
    pub running: bool,
    pub clients: Vec<WebSocketClient>,
    pub message_handlers: Vec<String>, // "connect", "message", "disconnect"
}

impl WebSocketServer {
    /// Create a new WebSocket server
    pub fn new(host: &str, port: u16) -> Self {
        WebSocketServer {
            host: host.to_string(),
            port,
            id: format!("ws_server_{}", rand::random::<u32>()),
            running: false,
            clients: Vec::new(),
            message_handlers: Vec::new(),
        }
    }

    /// Start the server
    pub fn start(&mut self) -> Result<(), String> {
        self.running = true;
        Ok(())
    }

    /// Stop the server
    pub fn stop(&mut self) -> Result<(), String> {
        self.running = false;
        self.clients.clear();
        Ok(())
    }

    /// Register a message handler
    pub fn on_handler(&mut self, event_type: &str) {
        if !self.message_handlers.contains(&event_type.to_string()) {
            self.message_handlers.push(event_type.to_string());
        }
    }

    /// Add a client
    pub fn add_client(&mut self, client: WebSocketClient) {
        self.clients.push(client);
    }

    /// Broadcast message to all clients
    pub fn broadcast(&mut self, message: &str) {
        for client in &mut self.clients {
            client.send_message(message).ok();
        }
    }

    /// Get client count
    pub fn client_count(&self) -> usize {
        self.clients.len()
    }
}

/// WebSocket client connected to server
#[derive(Clone, Debug)]
pub struct WebSocketClient {
    pub client_id: String,
    pub connection_time: String,
    pub message_queue: Vec<String>,
    pub state: String, // "connected", "disconnected"
}

impl WebSocketClient {
    /// Create a new client
    pub fn new() -> Self {
        WebSocketClient {
            client_id: format!("client_{}", rand::random::<u32>()),
            connection_time: "2026-03-14T00:00:00".to_string(),
            message_queue: Vec::new(),
            state: "connected".to_string(),
        }
    }

    /// Send a message to this client
    pub fn send_message(&mut self, message: &str) -> Result<(), String> {
        if self.state == "connected" {
            self.message_queue.push(message.to_string());
            Ok(())
        } else {
            Err("Client not connected".to_string())
        }
    }

    /// Receive a message
    pub fn receive_message(&mut self) -> Option<String> {
        if !self.message_queue.is_empty() {
            Some(self.message_queue.remove(0))
        } else {
            None
        }
    }

    /// Disconnect the client
    pub fn disconnect(&mut self) -> Result<(), String> {
        self.state = "disconnected".to_string();
        Ok(())
    }
}

/// WebSocket frame for message protocol
#[derive(Clone, Debug)]
pub struct WebSocketFrame {
    pub opcode: u8, // 1=text, 2=binary, 8=close, 9=ping, 10=pong
    pub payload: String,
    pub fin: bool, // Is this the final frame?
    pub masked: bool,
}

impl WebSocketFrame {
    /// Create a text frame
    pub fn text_frame(payload: &str) -> Self {
        WebSocketFrame {
            opcode: 1,
            payload: payload.to_string(),
            fin: true,
            masked: false,
        }
    }

    /// Create a close frame
    pub fn close_frame() -> Self {
        WebSocketFrame {
            opcode: 8,
            payload: String::new(),
            fin: true,
            masked: false,
        }
    }

    /// Create a ping frame
    pub fn ping_frame() -> Self {
        WebSocketFrame {
            opcode: 9,
            payload: String::new(),
            fin: true,
            masked: false,
        }
    }

    /// Create a pong frame (response to ping)
    pub fn pong_frame() -> Self {
        WebSocketFrame {
            opcode: 10,
            payload: String::new(),
            fin: true,
            masked: false,
        }
    }

    /// Get frame type name
    pub fn frame_type(&self) -> String {
        match self.opcode {
            1 => "text".to_string(),
            2 => "binary".to_string(),
            8 => "close".to_string(),
            9 => "ping".to_string(),
            10 => "pong".to_string(),
            _ => "unknown".to_string(),
        }
    }
}

/// WebSocket message (high-level)
#[derive(Clone, Debug)]
pub struct WebSocketMessage {
    pub message_type: String, // "text", "binary", "control"
    pub data: String,
    pub timestamp: String,
    pub sender_id: Option<String>,
}

impl WebSocketMessage {
    /// Create a new message
    pub fn new(msg_type: &str, data: &str) -> Self {
        WebSocketMessage {
            message_type: msg_type.to_string(),
            data: data.to_string(),
            timestamp: "2026-03-14T00:00:00".to_string(),
            sender_id: None,
        }
    }
}

/// Parse WebSocket handshake request (simplified)
pub fn parse_websocket_handshake(request: &str) -> Result<HashMap<String, String>, String> {
    let mut headers = HashMap::new();
    
    for line in request.lines() {
        if line.contains(':') {
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() == 2 {
                headers.insert(parts[0].trim().to_lowercase(), parts[1].trim().to_string());
            }
        }
    }

    // Check for required headers
    if !headers.contains_key("upgrade") || !headers.contains_key("connection") {
        return Err("Invalid WebSocket handshake".to_string());
    }

    Ok(headers)
}

/// Generate WebSocket handshake response
pub fn generate_handshake_response(request: &str) -> Result<String, String> {
    let _headers = parse_websocket_handshake(request)?;
    
    // In a real implementation, would compute accept key
    // For v3.0, return a valid response
    let response = 
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: mock_accept_key_v3_0\r\n\
         \r\n";
    
    Ok(response.to_string())
}

/// Simulate WebSocket message encoding
pub fn encode_message(message: &str) -> String {
    // In real implementation, would encode to WebSocket frame format
    // For v3.0, return JSON representation
    format!(
        "{{\"type\":\"text\",\"data\":\"{}\",\"masked\":false}}",
        message.replace("\"", "\\\"")
    )
}

/// Simulate WebSocket message decoding
pub fn decode_message(frame_data: &str) -> Result<String, String> {
    // In real implementation, would parse WebSocket frame format
    // For v3.0, parse simple JSON representation
    if frame_data.contains("\"data\"") {
        // Extract data field
        if let Some(start) = frame_data.find("\"data\":\"") {
            if let Some(end) = frame_data[start + 8..].find('"') {
                return Ok(frame_data[start + 8..start + 8 + end].to_string());
            }
        }
    }
    Ok(frame_data.to_string())
}

/// Convert server to dict representation
pub fn server_to_dict(server: &WebSocketServer) -> HashMap<String, String> {
    let mut dict = HashMap::new();
    dict.insert("type".to_string(), "WebSocketServer".to_string());
    dict.insert("id".to_string(), server.id.clone());
    dict.insert("host".to_string(), server.host.clone());
    dict.insert("port".to_string(), server.port.to_string());
    dict.insert("running".to_string(), server.running.to_string());
    dict.insert("clients".to_string(), server.clients.len().to_string());
    dict.insert("handlers".to_string(), server.message_handlers.join(","));
    dict
}

/// Convert websocket to dict representation
pub fn websocket_to_dict(ws: &WebSocket) -> HashMap<String, String> {
    let mut dict = HashMap::new();
    dict.insert("type".to_string(), "WebSocket".to_string());
    dict.insert("id".to_string(), ws.id.clone());
    dict.insert("url".to_string(), ws.url.clone());
    dict.insert("state".to_string(), ws.state.clone());
    dict.insert("connected".to_string(), ws.is_connected().to_string());
    if let Some(client_id) = &ws.client_id {
        dict.insert("client_id".to_string(), client_id.clone());
    }
    dict
}

// Simple random number generator for v3.0 (no external crates)
mod rand {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    pub fn random<T>() -> u32 {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        ((duration.as_nanos() as u32).wrapping_mul(1103515245)).wrapping_add(12345)
    }
}
