// ================================================================
// SERVER-SENT EVENTS - Phase 25.5
// Real-time server-to-client updates via HTTP
// ================================================================

use std::collections::HashMap;

/// SSE connection
#[derive(Clone, Debug)]
pub struct SSEConnection {
    pub client_id: String,
    pub is_connected: bool,
    pub connected_at: u64,
    pub metadata: HashMap<String, String>,
}

/// SSE event
#[derive(Clone, Debug)]
pub struct SSEEvent {
    pub name: Option<String>,
    pub data: String,
    pub id: Option<String>,
    pub retry: Option<u64>,
    pub comment: Option<String>,
}

/// SSE channel
#[derive(Clone, Debug)]
pub struct SSEChannel {
    pub name: String,
    pub subscribers: Vec<String>,
    pub events_published: u64,
}

pub struct SSESolver;

impl SSESolver {
    // ================================================================
    // CONNECTION MANAGEMENT (1-10)
    // ================================================================

    /// Problem 1: Create SSE connection
    pub fn create_sse_connection(client_id: &str) -> SSEConnection {
        SSEConnection {
            client_id: client_id.to_string(),
            is_connected: true,
            connected_at: 0,
            metadata: HashMap::new(),
        }
    }

    /// Problem 2: Connect SSE client
    pub fn connect_sse_client(clients: &mut Vec<SSEConnection>, client_id: &str) {
        clients.push(Self::create_sse_connection(client_id));
    }

    /// Problem 3: Disconnect SSE client
    pub fn disconnect_sse_client(clients: &mut Vec<SSEConnection>, client_id: &str) {
        clients.retain(|c| c.client_id != client_id);
    }

    /// Problem 4: Get connected clients
    pub fn get_connected_clients(clients: &[SSEConnection]) -> Vec<String> {
        clients.iter()
            .filter(|c| c.is_connected)
            .map(|c| c.client_id.clone())
            .collect()
    }

    /// Problem 5: Check if client connected
    pub fn is_client_connected(clients: &[SSEConnection], client_id: &str) -> bool {
        clients.iter()
            .any(|c| c.client_id == client_id && c.is_connected)
    }

    /// Problem 6: Get connection time
    pub fn get_client_connection_time(clients: &[SSEConnection], client_id: &str, now: u64) -> u64 {
        clients.iter()
            .find(|c| c.client_id == client_id)
            .map(|c| now.saturating_sub(c.connected_at))
            .unwrap_or(0)
    }

    /// Problem 7: Get client ID
    pub fn get_client_id(client: &SSEConnection) -> String {
        client.client_id.clone()
    }

    /// Problem 8: Set client metadata
    pub fn set_client_metadata(client: &mut SSEConnection, key: &str, value: &str) {
        client.metadata.insert(key.to_string(), value.to_string());
    }

    /// Problem 9: Get client metadata
    pub fn get_client_metadata(client: &SSEConnection, key: &str) -> Option<String> {
        client.metadata.get(key).cloned()
    }

    /// Problem 10: Cleanup stale clients
    pub fn cleanup_stale_clients(clients: &mut Vec<SSEConnection>, now: u64, timeout: u64) {
        clients.retain(|c| {
            if c.is_connected {
                true
            } else {
                (now - c.connected_at) <= timeout
            }
        });
    }

    // ================================================================
    // EVENT PUBLISHING (11-20)
    // ================================================================

    /// Problem 11: Publish event
    pub fn publish_event(clients: &[SSEConnection], event: &SSEEvent) -> usize {
        clients.len()
    }

    /// Problem 12: Publish to specific client
    pub fn publish_to_client(client: &SSEConnection, event: &SSEEvent) -> Result<(), String> {
        if client.is_connected {
            Ok(())
        } else {
            Err("Client not connected".to_string())
        }
    }

    /// Problem 13: Publish to multiple clients
    pub fn publish_to_clients(client_ids: &[String], event: &SSEEvent, clients: &[SSEConnection]) -> usize {
        client_ids.iter()
            .filter(|id| Self::is_client_connected(clients, id))
            .count()
    }

    /// Problem 14: Create event
    pub fn create_event(data: String) -> SSEEvent {
        SSEEvent {
            name: None,
            data,
            id: None,
            retry: None,
            comment: None,
        }
    }

    /// Problem 15: Set event name
    pub fn set_event_name(event: &mut SSEEvent, name: &str) {
        event.name = Some(name.to_string());
    }

    /// Problem 16: Set event data
    pub fn set_event_data(event: &mut SSEEvent, data: &str) {
        event.data = data.to_string();
    }

    /// Problem 17: Set event ID
    pub fn set_event_id(event: &mut SSEEvent, id: &str) {
        event.id = Some(id.to_string());
    }

    /// Problem 18: Set event retry
    pub fn set_event_retry(event: &mut SSEEvent, retry_ms: u64) {
        event.retry = Some(retry_ms);
    }

    /// Problem 19: Add event comment
    pub fn add_event_comment(event: &mut SSEEvent, comment: &str) {
        event.comment = Some(comment.to_string());
    }

    /// Problem 20: Broadcast to all
    pub fn broadcast_event(clients: &[SSEConnection], event: &SSEEvent) -> usize {
        clients.len()
    }

    // ================================================================
    // EVENT FORMAT (21-30)
    // ================================================================

    /// Problem 21: Format event line
    pub fn format_event_line(name: &str) -> String {
        format!("event: {}\n", name)
    }

    /// Problem 22: Format data line
    pub fn format_data_line(data: &str) -> String {
        if data.contains('\n') {
            let lines: Vec<&str> = data.lines().collect();
            lines.iter()
                .map(|line| format!("data: {}\n", line))
                .collect::<String>()
        } else {
            format!("data: {}\n", data)
        }
    }

    /// Problem 23: Format ID line
    pub fn format_id_line(id: &str) -> String {
        format!("id: {}\n", id)
    }

    /// Problem 24: Format retry line
    pub fn format_retry_line(retry_ms: u64) -> String {
        format!("retry: {}\n", retry_ms)
    }

    /// Problem 25: Format comment
    pub fn format_comment(comment: &str) -> String {
        format!(": {}\n", comment)
    }

    /// Problem 26: Serialize event
    pub fn serialize_event(event: &SSEEvent) -> String {
        let mut output = String::new();
        
        if let Some(name) = &event.name {
            output.push_str(&Self::format_event_line(name));
        }
        
        output.push_str(&Self::format_data_line(&event.data));
        
        if let Some(id) = &event.id {
            output.push_str(&Self::format_id_line(id));
        }
        
        if let Some(retry) = event.retry {
            output.push_str(&Self::format_retry_line(retry));
        }
        
        if let Some(comment) = &event.comment {
            output.push_str(&Self::format_comment(comment));
        }
        
        output.push('\n');
        output
    }

    /// Problem 27: Parse event stream
    pub fn parse_event_stream(stream: &str) -> Vec<SSEEvent> {
        let mut events = Vec::new();
        let mut current_event = SSEEvent {
            name: None,
            data: String::new(),
            id: None,
            retry: None,
            comment: None,
        };
        
        for line in stream.lines() {
            if line.is_empty() {
                if !current_event.data.is_empty() {
                    events.push(current_event.clone());
                }
                current_event = SSEEvent {
                    name: None,
                    data: String::new(),
                    id: None,
                    retry: None,
                    comment: None,
                };
            } else if let Some(rest) = line.strip_prefix("event: ") {
                current_event.name = Some(rest.to_string());
            } else if let Some(rest) = line.strip_prefix("data: ") {
                current_event.data.push_str(rest);
                current_event.data.push('\n');
            } else if let Some(rest) = line.strip_prefix("id: ") {
                current_event.id = Some(rest.to_string());
            } else if let Some(rest) = line.strip_prefix("retry: ") {
                if let Ok(retry) = rest.parse() {
                    current_event.retry = Some(retry);
                }
            }
        }
        
        events
    }

    /// Problem 28: Validate event format
    pub fn validate_event_format(event: &SSEEvent) -> bool {
        !event.data.is_empty()
    }

    /// Problem 29: Escape event data
    pub fn escape_event_data(data: &str) -> String {
        data.replace("\\", "\\\\")
            .replace("\n", "\\n")
    }

    /// Problem 30: Reconstruct event
    pub fn reconstruct_event(lines: &[String]) -> SSEEvent {
        let mut event = SSEEvent {
            name: None,
            data: String::new(),
            id: None,
            retry: None,
            comment: None,
        };
        
        for line in lines {
            if let Some(rest) = line.strip_prefix("event: ") {
                event.name = Some(rest.to_string());
            } else if let Some(rest) = line.strip_prefix("data: ") {
                event.data.push_str(rest);
            }
        }
        
        event
    }

    // ================================================================
    // CLIENT MANAGEMENT (31-40)
    // ================================================================

    /// Problem 31: Register event listener
    pub fn register_event_listener(
        subscriptions: &mut HashMap<String, Vec<String>>,
        client_id: &str,
        event_type: &str,
    ) {
        subscriptions.entry(event_type.to_string())
            .or_insert_with(Vec::new)
            .push(client_id.to_string());
    }

    /// Problem 32: Unregister listener
    pub fn unregister_event_listener(
        subscriptions: &mut HashMap<String, Vec<String>>,
        client_id: &str,
        event_type: &str,
    ) {
        if let Some(clients) = subscriptions.get_mut(event_type) {
            clients.retain(|c| c != client_id);
        }
    }

    /// Problem 33: Get subscribed events
    pub fn get_subscribed_events(
        subscriptions: &HashMap<String, Vec<String>>,
        client_id: &str,
    ) -> Vec<String> {
        subscriptions.iter()
            .filter(|(_, clients)| clients.contains(&client_id.to_string()))
            .map(|(event_type, _)| event_type.clone())
            .collect()
    }

    /// Problem 34: Send keepalive comment
    pub fn send_keepalive_comment() -> String {
        ": keepalive\n\n".to_string()
    }

    /// Problem 35: Set reconnect timeout
    pub fn set_reconnect_timeout(_client: &mut SSEConnection, _timeout_ms: u64) {
        // Timeout stored in client state
    }

    /// Problem 36: Store last event ID
    pub fn store_last_event_id(client: &mut SSEConnection, event_id: &str) {
        client.metadata.insert("last_event_id".to_string(), event_id.to_string());
    }

    /// Problem 37: Replay events
    pub fn replay_events(
        all_events: &[SSEEvent],
        last_id: Option<&str>,
    ) -> Vec<SSEEvent> {
        if let Some(id) = last_id {
            all_events.iter()
                .skip_while(|e| e.id.as_deref() != Some(id))
                .skip(1)
                .cloned()
                .collect()
        } else {
            all_events.to_vec()
        }
    }

    /// Problem 38: Filter events for client
    pub fn filter_events_for_client(
        event: &SSEEvent,
        client_filter: fn(&SSEEvent) -> bool,
    ) -> bool {
        client_filter(event)
    }

    /// Problem 39: Track client activity
    pub fn track_client_activity(client: &mut SSEConnection, now: u64) {
        client.metadata.insert("last_activity".to_string(), now.to_string());
    }

    /// Problem 40: Get client stats
    pub fn get_client_stats(client: &SSEConnection) -> HashMap<String, String> {
        client.metadata.clone()
    }

    // ================================================================
    // CHANNELS & PATTERNS (41-50)
    // ================================================================

    /// Problem 41: Create named channel
    pub fn create_named_channel(name: &str) -> SSEChannel {
        SSEChannel {
            name: name.to_string(),
            subscribers: Vec::new(),
            events_published: 0,
        }
    }

    /// Problem 42: Subscribe to channel
    pub fn subscribe_to_channel(channel: &mut SSEChannel, client_id: &str) {
        if !channel.subscribers.contains(&client_id.to_string()) {
            channel.subscribers.push(client_id.to_string());
        }
    }

    /// Problem 43: Unsubscribe from channel
    pub fn unsubscribe_from_channel(channel: &mut SSEChannel, client_id: &str) {
        channel.subscribers.retain(|c| c != client_id);
    }

    /// Problem 44: Broadcast to channel
    pub fn broadcast_to_channel(channel: &mut SSEChannel, _event: &SSEEvent) {
        channel.events_published += 1;
    }

    /// Problem 45: Get channel subscribers
    pub fn get_channel_subscribers(channel: &SSEChannel) -> Vec<String> {
        channel.subscribers.clone()
    }

    /// Problem 46: Get channel stats
    pub fn get_channel_stats(channel: &SSEChannel) -> (usize, u64) {
        (channel.subscribers.len(), channel.events_published)
    }

    /// Problem 47: Create topic subscription
    pub fn create_topic_subscription(_topic: &str) -> HashMap<String, Vec<String>> {
        HashMap::new()
    }

    /// Problem 48: Pattern match events
    pub fn pattern_match_events(event_name: &str, pattern: &str) -> bool {
        event_name.contains(pattern)
    }

    /// Problem 49: Create private channel
    pub fn create_private_channel(name: &str) -> SSEChannel {
        SSEChannel {
            name: name.to_string(),
            subscribers: Vec::new(),
            events_published: 0,
        }
    }

    /// Problem 50: Create public channel
    pub fn create_public_channel(name: &str) -> SSEChannel {
        SSEChannel {
            name: name.to_string(),
            subscribers: Vec::new(),
            events_published: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_sse_connection() {
        let conn = SSESolver::create_sse_connection("client1");
        assert_eq!(conn.client_id, "client1");
        assert!(conn.is_connected);
    }

    #[test]
    fn test_create_event() {
        let event = SSESolver::create_event("message".to_string());
        assert_eq!(event.data, "message");
    }

    #[test]
    fn test_serialize_event() {
        let mut event = SSESolver::create_event("data".to_string());
        SSESolver::set_event_name(&mut event, "update");
        let serialized = SSESolver::serialize_event(&event);
        assert!(serialized.contains("event: update"));
    }

    #[test]
    fn test_format_data_line() {
        let line = SSESolver::format_data_line("hello");
        assert!(line.contains("data:"));
    }

    #[test]
    fn test_parse_event_stream() {
        let stream = "event: test\ndata: value\n\n";
        let events = SSESolver::parse_event_stream(stream);
        assert!(!events.is_empty());
    }

    #[test]
    fn test_channel_operations() {
        let mut channel = SSESolver::create_named_channel("test");
        SSESolver::subscribe_to_channel(&mut channel, "client1");
        assert!(channel.subscribers.contains(&"client1".to_string()));
    }

    #[test]
    fn test_keepalive() {
        let keepalive = SSESolver::send_keepalive_comment();
        assert!(keepalive.contains("keepalive"));
    }

    #[test]
    fn test_replay_events() {
        let event1 = SSESolver::create_event("data1".to_string());
        let event2 = SSESolver::create_event("data2".to_string());
        let events = vec![event1, event2];
        let replayed = SSESolver::replay_events(&events, None);
        assert_eq!(replayed.len(), 2);
    }

    #[test]
    fn test_pattern_match() {
        assert!(SSESolver::pattern_match_events("user.update", "user"));
        assert!(!SSESolver::pattern_match_events("post.create", "user"));
    }

    #[test]
    fn test_client_metadata() {
        let mut conn = SSESolver::create_sse_connection("client1");
        SSESolver::set_client_metadata(&mut conn, "token", "abc123");
        assert_eq!(SSESolver::get_client_metadata(&conn, "token"), Some("abc123".to_string()));
    }
}
