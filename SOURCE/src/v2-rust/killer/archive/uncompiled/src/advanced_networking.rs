// Phase 13: Advanced Networking - P2P, WebSocket, messaging
// Features: Peer discovery, WebSocket connections, message routing, network protocols

use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::SystemTime;

/// Network protocol types
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum NetworkProtocol {
    Tcp,
    Udp,
    WebSocket,
    Http,
    Https,
    Custom(String),
}

impl NetworkProtocol {
    pub fn as_str(&self) -> &str {
        match self {
            NetworkProtocol::Tcp => "tcp",
            NetworkProtocol::Udp => "udp",
            NetworkProtocol::WebSocket => "ws",
            NetworkProtocol::Http => "http",
            NetworkProtocol::Https => "https",
            NetworkProtocol::Custom(name) => name,
        }
    }

    pub fn default_port(&self) -> u16 {
        match self {
            NetworkProtocol::Tcp => 0,      // Dynamic
            NetworkProtocol::Udp => 0,      // Dynamic
            NetworkProtocol::WebSocket => 80,
            NetworkProtocol::Http => 80,
            NetworkProtocol::Https => 443,
            NetworkProtocol::Custom(_) => 0,
        }
    }
}

/// Network message types
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MessageType {
    Ping,
    Pong,
    Discovery,
    DiscoveryResponse,
    Data,
    Control,
    Error,
    Custom(String),
}

impl MessageType {
    pub fn as_str(&self) -> &str {
        match self {
            MessageType::Ping => "ping",
            MessageType::Pong => "pong",
            MessageType::Discovery => "discovery",
            MessageType::DiscoveryResponse => "discovery_response",
            MessageType::Data => "data",
            MessageType::Control => "control",
            MessageType::Error => "error",
            MessageType::Custom(name) => name,
        }
    }
}

/// Network message
#[derive(Clone, Debug)]
pub struct NetworkMessage {
    pub id: String,
    pub msg_type: MessageType,
    pub source: String,
    pub destination: String,
    pub payload: Vec<u8>,
    pub timestamp: u64,
    pub ttl: u32,
}

impl NetworkMessage {
    pub fn new(
        msg_type: MessageType,
        source: String,
        destination: String,
        payload: Vec<u8>,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        NetworkMessage {
            id: format!("{}-{}", source, now),
            msg_type,
            source,
            destination,
            payload,
            timestamp: now,
            ttl: 255,
        }
    }

    /// Decrement TTL
    pub fn decrement_ttl(mut self) -> Self {
        if self.ttl > 0 {
            self.ttl -= 1;
        }
        self
    }

    /// Check if expired
    pub fn is_expired(&self, max_age_secs: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        (now - self.timestamp) > max_age_secs || self.ttl == 0
    }

    /// Get size
    pub fn size(&self) -> usize {
        self.payload.len()
    }
}

/// Peer identity
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PeerId {
    pub id: String,
}

impl PeerId {
    pub fn new(id: String) -> Self {
        PeerId { id }
    }

    /// Generate random peer ID
    pub fn random() -> Self {
        let id = format!("peer_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos());
        PeerId { id }
    }
}

/// Peer info
#[derive(Clone, Debug)]
pub struct PeerInfo {
    pub peer_id: PeerId,
    pub address: String,
    pub port: u16,
    pub protocol: NetworkProtocol,
    pub last_seen: u64,
    pub latency_ms: u32,
}

impl PeerInfo {
    pub fn new(
        peer_id: PeerId,
        address: String,
        port: u16,
        protocol: NetworkProtocol,
    ) -> Self {
        PeerInfo {
            peer_id,
            address,
            port,
            protocol,
            last_seen: current_timestamp(),
            latency_ms: 0,
        }
    }

    /// Get endpoint
    pub fn endpoint(&self) -> String {
        format!("{}:{}", self.address, self.port)
    }

    /// Update latency
    pub fn update_latency(mut self, latency_ms: u32) -> Self {
        self.latency_ms = latency_ms;
        self.last_seen = current_timestamp();
        self
    }

    /// Get socket addr
    pub fn socket_addr(&self) -> Result<SocketAddr, String> {
        format!("{}:{}", self.address, self.port)
            .parse()
            .map_err(|_| "Invalid socket address".to_string())
    }
}

/// Peer discovery
#[derive(Clone, Debug)]
pub struct PeerDiscovery {
    pub known_peers: HashMap<String, PeerInfo>,
    pub bootstrap_nodes: Vec<String>,
}

impl PeerDiscovery {
    pub fn new() -> Self {
        PeerDiscovery {
            known_peers: HashMap::new(),
            bootstrap_nodes: Vec::new(),
        }
    }

    /// Add bootstrap node
    pub fn add_bootstrap(&mut self, address: String) {
        self.bootstrap_nodes.push(address);
    }

    /// Register peer
    pub fn register_peer(&mut self, peer: PeerInfo) -> Result<(), String> {
        self.known_peers.insert(peer.peer_id.id.clone(), peer);
        Ok(())
    }

    /// Unregister peer
    pub fn unregister_peer(&mut self, peer_id: &str) -> Result<(), String> {
        if self.known_peers.remove(peer_id).is_some() {
            Ok(())
        } else {
            Err(format!("Peer {} not found", peer_id))
        }
    }

    /// Get peer
    pub fn get_peer(&self, peer_id: &str) -> Option<PeerInfo> {
        self.known_peers.get(peer_id).cloned()
    }

    /// List all peers
    pub fn list_peers(&self) -> Vec<PeerInfo> {
        self.known_peers.values().cloned().collect()
    }

    /// Peer count
    pub fn peer_count(&self) -> usize {
        self.known_peers.len()
    }

    /// Find closest peers
    pub fn find_closest_peers(&self, count: usize) -> Vec<PeerInfo> {
        let mut peers = self.list_peers();
        peers.sort_by_key(|p| p.latency_ms);
        peers.truncate(count);
        peers
    }
}

impl Default for PeerDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

/// P2P network node
#[derive(Clone, Debug)]
pub struct P2PNode {
    pub peer_id: PeerId,
    pub address: String,
    pub port: u16,
    pub protocol: NetworkProtocol,
    pub discovery: PeerDiscovery,
}

impl P2PNode {
    pub fn new(address: String, port: u16, protocol: NetworkProtocol) -> Self {
        P2PNode {
            peer_id: PeerId::random(),
            address,
            port,
            protocol,
            discovery: PeerDiscovery::new(),
        }
    }

    /// Get endpoint
    pub fn endpoint(&self) -> String {
        format!("{}:{}", self.address, self.port)
    }

    /// Connect to peer
    pub fn connect_peer(&mut self, peer: PeerInfo) -> Result<(), String> {
        self.discovery.register_peer(peer)
    }

    /// Disconnect from peer
    pub fn disconnect_peer(&mut self, peer_id: &str) -> Result<(), String> {
        self.discovery.unregister_peer(peer_id)
    }

    /// Get connected peers
    pub fn get_connected_peers(&self) -> Vec<PeerInfo> {
        self.discovery.list_peers()
    }

    /// Get peer count
    pub fn peer_count(&self) -> usize {
        self.discovery.peer_count()
    }
}

/// WebSocket connection
#[derive(Clone, Debug)]
pub struct WebSocketConnection {
    pub id: String,
    pub remote_addr: String,
    pub connected_at: u64,
    pub last_message: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
}

impl WebSocketConnection {
    pub fn new(id: String, remote_addr: String) -> Self {
        let now = current_timestamp();
        WebSocketConnection {
            id,
            remote_addr,
            connected_at: now,
            last_message: now,
            messages_sent: 0,
            messages_received: 0,
        }
    }

    /// Update last message
    pub fn update_activity(mut self) -> Self {
        self.last_message = current_timestamp();
        self
    }

    /// Increment sent count
    pub fn increment_sent(mut self) -> Self {
        self.messages_sent += 1;
        self
    }

    /// Increment received count
    pub fn increment_received(mut self) -> Self {
        self.messages_received += 1;
        self
    }

    /// Get uptime in seconds
    pub fn uptime_secs(&self) -> u64 {
        current_timestamp() - self.connected_at
    }

    /// Get idle time in seconds
    pub fn idle_time_secs(&self) -> u64 {
        current_timestamp() - self.last_message
    }

    /// Get message throughput
    pub fn message_throughput(&self) -> u64 {
        let uptime = self.uptime_secs();
        if uptime == 0 {
            0
        } else {
            (self.messages_sent + self.messages_received) / uptime
        }
    }
}

/// WebSocket server
#[derive(Clone, Debug)]
pub struct WebSocketServer {
    pub connections: HashMap<String, WebSocketConnection>,
    pub listen_addr: String,
    pub listen_port: u16,
}

impl WebSocketServer {
    pub fn new(listen_addr: String, listen_port: u16) -> Self {
        WebSocketServer {
            connections: HashMap::new(),
            listen_addr,
            listen_port,
        }
    }

    /// Add connection
    pub fn add_connection(&mut self, id: String, remote_addr: String) -> Result<(), String> {
        if self.connections.contains_key(&id) {
            return Err(format!("Connection {} already exists", id));
        }
        let conn = WebSocketConnection::new(id.clone(), remote_addr);
        self.connections.insert(id, conn);
        Ok(())
    }

    /// Remove connection
    pub fn remove_connection(&mut self, id: &str) -> Result<(), String> {
        if self.connections.remove(id).is_some() {
            Ok(())
        } else {
            Err(format!("Connection {} not found", id))
        }
    }

    /// Get connection
    pub fn get_connection(&self, id: &str) -> Option<WebSocketConnection> {
        self.connections.get(id).cloned()
    }

    /// List connections
    pub fn list_connections(&self) -> Vec<WebSocketConnection> {
        self.connections.values().cloned().collect()
    }

    /// Connection count
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    /// Broadcast message
    pub fn broadcast_message(&mut self, message: &[u8]) -> usize {
        let mut count = 0;
        for conn in self.connections.values_mut() {
            if message.len() > 0 {
                *conn = conn.clone().increment_sent();
                count += 1;
            }
        }
        count
    }

    /// Get statistics
    pub fn get_statistics(&self) -> (usize, u64, u64) {
        let total_messages = self.connections.values()
            .map(|c| c.messages_sent + c.messages_received)
            .sum();
        let total_sent = self.connections.values()
            .map(|c| c.messages_sent)
            .sum();
        (self.connections.len(), total_sent, total_messages)
    }
}

impl Default for WebSocketServer {
    fn default() -> Self {
        Self::new("0.0.0.0".to_string(), 80)
    }
}

/// Message router
pub struct MessageRouter {
    pub routes: HashMap<String, Vec<String>>, // destination -> handlers
}

impl MessageRouter {
    pub fn new() -> Self {
        MessageRouter {
            routes: HashMap::new(),
        }
    }

    /// Register route
    pub fn register_route(&mut self, destination: String, handler_id: String) {
        self.routes.entry(destination)
            .or_insert_with(Vec::new)
            .push(handler_id);
    }

    /// Route message
    pub fn route_message(&self, message: &NetworkMessage) -> Vec<String> {
        self.routes.get(&message.destination)
            .cloned()
            .unwrap_or_default()
    }

    /// Route count
    pub fn route_count(&self) -> usize {
        self.routes.len()
    }

    /// Handler count
    pub fn handler_count(&self) -> usize {
        self.routes.values().map(|h| h.len()).sum()
    }
}

impl Default for MessageRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to get current timestamp in seconds
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_protocol_as_str() {
        assert_eq!(NetworkProtocol::Tcp.as_str(), "tcp");
        assert_eq!(NetworkProtocol::WebSocket.as_str(), "ws");
    }

    #[test]
    fn test_network_protocol_default_ports() {
        assert_eq!(NetworkProtocol::Http.default_port(), 80);
        assert_eq!(NetworkProtocol::Https.default_port(), 443);
    }

    #[test]
    fn test_message_type_as_str() {
        assert_eq!(MessageType::Ping.as_str(), "ping");
        assert_eq!(MessageType::Data.as_str(), "data");
    }

    #[test]
    fn test_network_message_creation() {
        let msg = NetworkMessage::new(
            MessageType::Data,
            "peer1".to_string(),
            "peer2".to_string(),
            vec![1, 2, 3],
        );
        assert_eq!(msg.source, "peer1");
        assert_eq!(msg.destination, "peer2");
    }

    #[test]
    fn test_network_message_size() {
        let msg = NetworkMessage::new(
            MessageType::Data,
            "peer1".to_string(),
            "peer2".to_string(),
            vec![1, 2, 3, 4, 5],
        );
        assert_eq!(msg.size(), 5);
    }

    #[test]
    fn test_network_message_ttl_decrement() {
        let msg = NetworkMessage::new(
            MessageType::Data,
            "peer1".to_string(),
            "peer2".to_string(),
            vec![],
        );
        let ttl = msg.ttl;
        let decremented = msg.decrement_ttl();
        assert_eq!(decremented.ttl, ttl - 1);
    }

    #[test]
    fn test_peer_id_creation() {
        let peer_id = PeerId::new("peer1".to_string());
        assert_eq!(peer_id.id, "peer1");
    }

    #[test]
    fn test_peer_id_random() {
        let peer_id = PeerId::random();
        assert!(peer_id.id.starts_with("peer_"));
    }

    #[test]
    fn test_peer_info_creation() {
        let peer_id = PeerId::new("peer1".to_string());
        let peer = PeerInfo::new(
            peer_id,
            "localhost".to_string(),
            8080,
            NetworkProtocol::Tcp,
        );
        assert_eq!(peer.endpoint(), "localhost:8080");
    }

    #[test]
    fn test_peer_discovery_register() {
        let mut discovery = PeerDiscovery::new();
        let peer_id = PeerId::new("peer1".to_string());
        let peer = PeerInfo::new(
            peer_id,
            "localhost".to_string(),
            8080,
            NetworkProtocol::Tcp,
        );
        
        assert!(discovery.register_peer(peer).is_ok());
        assert_eq!(discovery.peer_count(), 1);
    }

    #[test]
    fn test_peer_discovery_unregister() {
        let mut discovery = PeerDiscovery::new();
        let peer_id = PeerId::new("peer1".to_string());
        let peer = PeerInfo::new(
            peer_id.clone(),
            "localhost".to_string(),
            8080,
            NetworkProtocol::Tcp,
        );
        
        discovery.register_peer(peer).unwrap();
        assert!(discovery.unregister_peer(&peer_id.id).is_ok());
        assert_eq!(discovery.peer_count(), 0);
    }

    #[test]
    fn test_p2p_node_creation() {
        let node = P2PNode::new(
            "localhost".to_string(),
            8080,
            NetworkProtocol::Tcp,
        );
        assert_eq!(node.endpoint(), "localhost:8080");
        assert!(!node.peer_id.id.is_empty());
    }

    #[test]
    fn test_websocket_connection_creation() {
        let conn = WebSocketConnection::new(
            "conn1".to_string(),
            "127.0.0.1".to_string(),
        );
        assert_eq!(conn.id, "conn1");
        assert_eq!(conn.messages_sent, 0);
    }

    #[test]
    fn test_websocket_connection_increment_sent() {
        let conn = WebSocketConnection::new(
            "conn1".to_string(),
            "127.0.0.1".to_string(),
        );
        let incremented = conn.increment_sent();
        assert_eq!(incremented.messages_sent, 1);
    }

    #[test]
    fn test_websocket_server_add_connection() {
        let mut server = WebSocketServer::new("127.0.0.1".to_string(), 80);
        assert!(server.add_connection("conn1".to_string(), "127.0.0.1".to_string()).is_ok());
        assert_eq!(server.connection_count(), 1);
    }

    #[test]
    fn test_websocket_server_remove_connection() {
        let mut server = WebSocketServer::new("127.0.0.1".to_string(), 80);
        server.add_connection("conn1".to_string(), "127.0.0.1".to_string()).unwrap();
        assert!(server.remove_connection("conn1").is_ok());
        assert_eq!(server.connection_count(), 0);
    }

    #[test]
    fn test_message_router_register_route() {
        let mut router = MessageRouter::new();
        router.register_route("dest1".to_string(), "handler1".to_string());
        assert_eq!(router.route_count(), 1);
    }

    #[test]
    fn test_message_router_route_message() {
        let mut router = MessageRouter::new();
        router.register_route("dest1".to_string(), "handler1".to_string());
        router.register_route("dest1".to_string(), "handler2".to_string());
        
        let msg = NetworkMessage::new(
            MessageType::Data,
            "peer1".to_string(),
            "dest1".to_string(),
            vec![],
        );
        
        let handlers = router.route_message(&msg);
        assert_eq!(handlers.len(), 2);
    }
}
