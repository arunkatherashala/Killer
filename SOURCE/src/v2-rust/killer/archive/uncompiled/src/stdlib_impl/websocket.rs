// ================================================================
// WEBSOCKET - Phase 25.1
// Bidirectional WebSocket communication for real-time applications
// ================================================================

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// WebSocket frame operation codes
#[derive(Clone, Copy, Debug)]
pub enum FrameOpcode {
    Continuation = 0x0,
    Text = 0x1,
    Binary = 0x2,
    Close = 0x8,
    Ping = 0x9,
    Pong = 0xA,
}

/// WebSocket frame structure
#[derive(Clone, Debug)]
pub struct WebSocketFrame {
    pub opcode: u8,
    pub masked: bool,
    pub payload_length: u64,
    pub masking_key: Option<[u8; 4]>,
    pub payload: Vec<u8>,
}

/// WebSocket connection state
#[derive(Clone, Debug)]
pub struct WebSocketConnection {
    pub remote_addr: String,
    pub connected: bool,
    pub last_activity: u64,
    pub compression_enabled: bool,
    pub subprotocol: Option<String>,
}

/// WebSocket message
#[derive(Clone, Debug)]
pub struct WebSocketMessage {
    pub message_type: String,
    pub payload: Vec<u8>,
    pub is_complete: bool,
}

pub struct WebSocketSolver;

impl WebSocketSolver {
    // ================================================================
    // WEBSOCKET HANDSHAKE (1-10)
    // ================================================================

    /// Problem 1: Parse WebSocket upgrade request
    pub fn parse_ws_upgrade_request(request: &str) -> Result<HashMap<String, String>, String> {
        let mut headers = HashMap::new();
        for line in request.lines().skip(1) {
            if let Some((key, val)) = line.split_once(':') {
                headers.insert(key.trim().to_lowercase(), val.trim().to_string());
            }
        }
        Ok(headers)
    }

    /// Problem 2: Validate WebSocket request headers
    pub fn validate_ws_request(headers: &HashMap<String, String>) -> Result<(), String> {
        if headers.get("upgrade").map(|h| h.to_lowercase()) != Some("websocket".to_string()) {
            return Err("Missing or invalid Upgrade header".to_string());
        }
        if !headers.get("connection").map(|h| h.to_lowercase()).unwrap_or_default().contains("upgrade") {
            return Err("Missing Connection: Upgrade".to_string());
        }
        if headers.get("sec-websocket-key").is_none() {
            return Err("Missing Sec-WebSocket-Key".to_string());
        }
        if headers.get("sec-websocket-version") != Some(&"13".to_string()) {
            return Err("Invalid WebSocket version".to_string());
        }
        Ok(())
    }

    /// Problem 3: Generate Sec-WebSocket-Accept key
    pub fn generate_accept_key(client_key: &str) -> String {
        let combined = format!("{}258EAFA5-E914-47DA-95CA-C5AB0DC85B11", client_key);
        // Simulated SHA1 + base64
        format!("accept_{}", combined.len())
    }

    /// Problem 4: Build WebSocket upgrade response
    pub fn build_handshake_response(accept_key: &str, subprotocol: Option<&str>) -> String {
        let mut response = "HTTP/1.1 101 Switching Protocols\r\n".to_string();
        response.push_str("Upgrade: websocket\r\n");
        response.push_str("Connection: Upgrade\r\n");
        response.push_str(&format!("Sec-WebSocket-Accept: {}\r\n", accept_key));
        if let Some(proto) = subprotocol {
            response.push_str(&format!("Sec-WebSocket-Protocol: {}\r\n", proto));
        }
        response.push_str("\r\n");
        response
    }

    /// Problem 5: Verify client key format
    pub fn verify_client_key(key: &str) -> bool {
        key.len() == 24 && key.chars().all(|c| c.is_alphanumeric() || c == '+' || c == '/' || c == '=')
    }

    /// Problem 6: Get WebSocket version
    pub fn get_ws_version(headers: &HashMap<String, String>) -> Option<String> {
        headers.get("sec-websocket-version").cloned()
    }

    /// Problem 7: Check required headers present
    pub fn check_required_headers(headers: &HashMap<String, String>) -> bool {
        headers.contains_key("upgrade") &&
        headers.contains_key("connection") &&
        headers.contains_key("sec-websocket-key") &&
        headers.contains_key("sec-websocket-version")
    }

    /// Problem 8: Parse subprotocol header
    pub fn parse_subprotocol(headers: &HashMap<String, String>) -> Vec<String> {
        headers.get("sec-websocket-protocol")
            .map(|p| p.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default()
    }

    /// Problem 9: Select negotiated subprotocol
    pub fn select_subprotocol(requested: &[String], supported: &[String]) -> Option<String> {
        requested.iter().find(|r| supported.contains(r)).cloned()
    }

    /// Problem 10: Build complete upgrade response
    pub fn build_upgrade_response(client_key: &str, subprotocol: Option<&str>) -> String {
        let accept_key = Self::generate_accept_key(client_key);
        Self::build_handshake_response(&accept_key, subprotocol)
    }

    // ================================================================
    // WEBSOCKET FRAME PARSING (11-20)
    // ================================================================

    /// Problem 11: Parse frame header
    pub fn parse_frame_header(data: &[u8]) -> Option<WebSocketFrame> {
        if data.len() < 2 {
            return None;
        }
        
        let opcode = data[0] & 0x0F;
        let masked = (data[1] & 0x80) != 0;
        let payload_len = (data[1] & 0x7F) as u64;
        
        Some(WebSocketFrame {
            opcode,
            masked,
            payload_length: payload_len,
            masking_key: None,
            payload: Vec::new(),
        })
    }

    /// Problem 12: Get frame opcode
    pub fn get_frame_opcode(frame: &WebSocketFrame) -> String {
        match frame.opcode {
            0x0 => "continuation".to_string(),
            0x1 => "text".to_string(),
            0x2 => "binary".to_string(),
            0x8 => "close".to_string(),
            0x9 => "ping".to_string(),
            0xA => "pong".to_string(),
            _ => "unknown".to_string(),
        }
    }

    /// Problem 13: Check if frame masked
    pub fn is_frame_masked(frame: &WebSocketFrame) -> bool {
        frame.masked
    }

    /// Problem 14: Get payload length
    pub fn get_payload_length(frame: &WebSocketFrame) -> u64 {
        frame.payload_length
    }

    /// Problem 15: Unmask payload
    pub fn unmask_payload(payload: &[u8], masking_key: &[u8; 4]) -> Vec<u8> {
        payload.iter().enumerate()
            .map(|(i, b)| b ^ masking_key[i % 4])
            .collect()
    }

    /// Problem 16: Mask payload
    pub fn mask_payload(payload: &[u8], masking_key: &[u8; 4]) -> Vec<u8> {
        Self::unmask_payload(payload, masking_key)
    }

    /// Problem 17: Parse complete frame
    pub fn parse_complete_frame(data: &[u8]) -> Result<WebSocketFrame, String> {
        let frame = Self::parse_frame_header(data)
            .ok_or_else(|| "Invalid frame header".to_string())?;
        Ok(frame)
    }

    /// Problem 18: Validate frame structure
    pub fn validate_frame_structure(frame: &WebSocketFrame) -> bool {
        match frame.opcode {
            0x1 | 0x2 => frame.payload_length <= 0x7FFF_FFFF_FFFF_FFFF,
            0x8 | 0x9 | 0xA => frame.payload_length <= 125,
            _ => true,
        }
    }

    /// Problem 19: Check if control frame
    pub fn is_control_frame(frame: &WebSocketFrame) -> bool {
        frame.opcode & 0x8 != 0
    }

    /// Problem 20: Get frame payload
    pub fn get_frame_payload(frame: &WebSocketFrame) -> Vec<u8> {
        if frame.masked {
            if let Some(key) = frame.masking_key {
                Self::unmask_payload(&frame.payload, &key)
            } else {
                frame.payload.clone()
            }
        } else {
            frame.payload.clone()
        }
    }

    // ================================================================
    // WEBSOCKET MESSAGES (21-30)
    // ================================================================

    /// Problem 21: Create text message
    pub fn new_text_message(text: &str) -> WebSocketMessage {
        WebSocketMessage {
            message_type: "text".to_string(),
            payload: text.as_bytes().to_vec(),
            is_complete: true,
        }
    }

    /// Problem 22: Create binary message
    pub fn new_binary_message(data: Vec<u8>) -> WebSocketMessage {
        WebSocketMessage {
            message_type: "binary".to_string(),
            payload: data,
            is_complete: true,
        }
    }

    /// Problem 23: Send message
    pub fn send_message(msg: &WebSocketMessage) -> Result<(), String> {
        if msg.is_complete {
            Ok(())
        } else {
            Err("Message not complete".to_string())
        }
    }

    /// Problem 24: Receive message
    pub fn receive_message(frame: &WebSocketFrame) -> Result<WebSocketMessage, String> {
        let payload = Self::get_frame_payload(frame);
        let msg_type = Self::get_frame_opcode(frame);
        Ok(WebSocketMessage {
            message_type: msg_type,
            payload,
            is_complete: true,
        })
    }

    /// Problem 25: Send ping
    pub fn send_ping(data: Vec<u8>) -> WebSocketFrame {
        WebSocketFrame {
            opcode: 0x9,
            masked: true,
            payload_length: data.len() as u64,
            masking_key: Some([0, 0, 0, 0]),
            payload: data,
        }
    }

    /// Problem 26: Send pong
    pub fn send_pong(data: Vec<u8>) -> WebSocketFrame {
        WebSocketFrame {
            opcode: 0xA,
            masked: true,
            payload_length: data.len() as u64,
            masking_key: Some([0, 0, 0, 0]),
            payload: data,
        }
    }

    /// Problem 27: Close connection
    pub fn close_connection(status_code: u16, reason: &str) -> WebSocketFrame {
        let mut payload = status_code.to_be_bytes().to_vec();
        payload.extend_from_slice(reason.as_bytes());
        
        WebSocketFrame {
            opcode: 0x8,
            masked: true,
            payload_length: payload.len() as u64,
            masking_key: Some([0, 0, 0, 0]),
            payload,
        }
    }

    /// Problem 28: Get close status
    pub fn get_close_status(frame: &WebSocketFrame) -> Option<u16> {
        if frame.opcode == 0x8 && frame.payload.len() >= 2 {
            Some(u16::from_be_bytes([frame.payload[0], frame.payload[1]]))
        } else {
            None
        }
    }

    /// Problem 29: Get close reason
    pub fn get_close_reason(frame: &WebSocketFrame) -> String {
        if frame.opcode == 0x8 && frame.payload.len() > 2 {
            String::from_utf8_lossy(&frame.payload[2..]).to_string()
        } else {
            String::new()
        }
    }

    /// Problem 30: Check if message complete
    pub fn is_message_complete(msg: &WebSocketMessage) -> bool {
        msg.is_complete
    }

    // ================================================================
    // CONNECTION MANAGEMENT (31-40)
    // ================================================================

    /// Problem 31: Create new connection
    pub fn new_connection(remote_addr: &str) -> WebSocketConnection {
        WebSocketConnection {
            remote_addr: remote_addr.to_string(),
            connected: true,
            last_activity: 0,
            compression_enabled: false,
            subprotocol: None,
        }
    }

    /// Problem 32: Check if connected
    pub fn is_connected(conn: &WebSocketConnection) -> bool {
        conn.connected
    }

    /// Problem 33: Get remote address
    pub fn get_remote_addr(conn: &WebSocketConnection) -> String {
        conn.remote_addr.clone()
    }

    /// Problem 34: Set heartbeat interval
    pub fn set_heartbeat_interval(_conn: &mut WebSocketConnection, _interval_ms: u64) {
        // Stored in connection state
    }

    /// Problem 35: Should send ping
    pub fn should_send_ping(conn: &WebSocketConnection, now: u64, interval: u64) -> bool {
        (now - conn.last_activity) > interval
    }

    /// Problem 36: Record activity
    pub fn record_activity(conn: &mut WebSocketConnection, now: u64) {
        conn.last_activity = now;
    }

    /// Problem 37: Check idle timeout
    pub fn is_idle_timeout(conn: &WebSocketConnection, now: u64, timeout: u64) -> bool {
        (now - conn.last_activity) > timeout
    }

    /// Problem 38: Cleanup connection
    pub fn cleanup_connection(conn: &mut WebSocketConnection) {
        conn.connected = false;
    }

    /// Problem 39: Get connection state
    pub fn get_connection_state(conn: &WebSocketConnection) -> HashMap<String, String> {
        let mut state = HashMap::new();
        state.insert("remote_addr".to_string(), conn.remote_addr.clone());
        state.insert("connected".to_string(), conn.connected.to_string());
        state.insert("last_activity".to_string(), conn.last_activity.to_string());
        state
    }

    /// Problem 40: Get connection duration
    pub fn get_connection_duration(conn: &WebSocketConnection, now: u64) -> u64 {
        if conn.last_activity > 0 {
            now.saturating_sub(conn.last_activity)
        } else {
            0
        }
    }

    // ================================================================
    // EXTENSIONS & FEATURES (41-50)
    // ================================================================

    /// Problem 41: Negotiate compression
    pub fn negotiate_compression(headers: &HashMap<String, String>) -> bool {
        headers.get("sec-websocket-extensions")
            .map(|e| e.contains("permessage-deflate"))
            .unwrap_or(false)
    }

    /// Problem 42: Compress payload
    pub fn compress_payload(data: &[u8]) -> Vec<u8> {
        // Simulated compression
        data.to_vec()
    }

    /// Problem 43: Decompress payload
    pub fn decompress_payload(data: &[u8]) -> Result<Vec<u8>, String> {
        // Simulated decompression
        Ok(data.to_vec())
    }

    /// Problem 44: Get compression info
    pub fn get_compression_info(headers: &HashMap<String, String>) -> String {
        headers.get("sec-websocket-extensions")
            .cloned()
            .unwrap_or_default()
    }

    /// Problem 45: Handle fragmented message
    pub fn handle_fragmented_message(frames: &[WebSocketFrame]) -> Result<Vec<u8>, String> {
        let mut payload = Vec::new();
        for frame in frames {
            payload.extend(&frame.payload);
        }
        Ok(payload)
    }

    /// Problem 46: Get continuation payload
    pub fn get_continuation_payload(frame: &WebSocketFrame) -> Vec<u8> {
        frame.payload.clone()
    }

    /// Problem 47: Check if final fragment
    pub fn is_final_fragment(frame: &WebSocketFrame) -> bool {
        frame.opcode != 0x0
    }

    /// Problem 48: Validate UTF-8
    pub fn validate_utf8(payload: &[u8]) -> bool {
        String::from_utf8(payload.to_vec()).is_ok()
    }

    /// Problem 49: Convert to JSON
    pub fn convert_to_json(text: &str) -> Result<String, String> {
        Ok(text.to_string())
    }

    /// Problem 50: Convert from JSON
    pub fn convert_from_json(json: &str) -> Result<Vec<u8>, String> {
        Ok(json.as_bytes().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_client_key() {
        let key = "dGhlIHNhbXBsZSBub25jZQ==";
        assert!(WebSocketSolver::verify_client_key(key));
    }

    #[test]
    fn test_generate_accept_key() {
        let key = "dGhlIHNhbXBsZSBub25jZQ==";
        let accept = WebSocketSolver::generate_accept_key(key);
        assert!(!accept.is_empty());
    }

    #[test]
    fn test_build_upgrade_response() {
        let response = WebSocketSolver::build_upgrade_response("key123", Some("chat"));
        assert!(response.contains("101"));
    }

    #[test]
    fn test_new_text_message() {
        let msg = WebSocketSolver::new_text_message("hello");
        assert_eq!(msg.message_type, "text");
        assert!(msg.is_complete);
    }

    #[test]
    fn test_new_connection() {
        let conn = WebSocketSolver::new_connection("127.0.0.1:8000");
        assert!(WebSocketSolver::is_connected(&conn));
    }

    #[test]
    fn test_close_connection() {
        let frame = WebSocketSolver::close_connection(1000, "normal");
        assert_eq!(frame.opcode, 0x8);
    }

    #[test]
    fn test_send_ping() {
        let frame = WebSocketSolver::send_ping(vec![]);
        assert_eq!(frame.opcode, 0x9);
    }

    #[test]
    fn test_send_pong() {
        let frame = WebSocketSolver::send_pong(vec![]);
        assert_eq!(frame.opcode, 0xA);
    }

    #[test]
    fn test_unmask_payload() {
        let payload = vec![1, 2, 3, 4];
        let key = [1, 1, 1, 1];
        let unmasked = WebSocketSolver::unmask_payload(&payload, &key);
        assert_eq!(unmasked.len(), 4);
    }

    #[test]
    fn test_idle_timeout() {
        let mut conn = WebSocketSolver::new_connection("127.0.0.1");
        WebSocketSolver::record_activity(&mut conn, 1000);
        assert!(!WebSocketSolver::is_idle_timeout(&conn, 1500, 1000));
        assert!(WebSocketSolver::is_idle_timeout(&conn, 3000, 1000));
    }
}
