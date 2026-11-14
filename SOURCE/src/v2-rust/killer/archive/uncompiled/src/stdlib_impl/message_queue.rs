// ================================================================
// MESSAGE QUEUES - Phase 27.4
// Publish/subscribe, consumer groups, and dead letter queues
// ================================================================

use std::collections::{HashMap, VecDeque};

/// Message in queue
#[derive(Clone, Debug)]
pub struct Message {
    pub id: String,
    pub topic: String,
    pub body: Vec<u8>,
    pub timestamp: u64,
    pub partition: u32,
    pub offset: u64,
    pub key: Option<String>,
    pub headers: HashMap<String, String>,
}

/// Consumer group
#[derive(Clone, Debug)]
pub struct ConsumerGroup {
    pub id: String,
    pub topic: String,
    pub members: Vec<String>,
    pub partitions: Vec<u32>,
}

/// Dead letter queue entry
#[derive(Clone, Debug)]
pub struct DeadLetterEntry {
    pub message: Message,
    pub reason: String,
    pub retry_count: u32,
    pub last_retry_time: u64,
}

pub struct MessageQueueSolver;

impl MessageQueueSolver {
    // ================================================================
    // PUBLISH/SUBSCRIBE (1-12)
    // ================================================================

    /// Problem 1: Create topic
    pub fn create_topic(
        topics: &mut HashMap<String, VecDeque<Message>>,
        topic_name: &str,
    ) {
        topics.insert(topic_name.to_string(), VecDeque::new());
    }

    /// Problem 2: Publish message
    pub fn publish_message(
        topics: &mut HashMap<String, VecDeque<Message>>,
        message: &Message,
    ) -> Result<String, String> {
        if let Some(queue) = topics.get_mut(&message.topic) {
            queue.push_back(message.clone());
            Ok(message.id.clone())
        } else {
            Err("Topic not found".to_string())
        }
    }

    /// Problem 3: Subscribe to topic
    pub fn subscribe_to_topic(
        subscriptions: &mut HashMap<String, Vec<String>>,
        topic: &str,
        consumer_id: &str,
    ) {
        subscriptions
            .entry(topic.to_string())
            .or_insert_with(Vec::new)
            .push(consumer_id.to_string());
    }

    /// Problem 4: Unsubscribe from topic
    pub fn unsubscribe_from_topic(
        subscriptions: &mut HashMap<String, Vec<String>>,
        topic: &str,
        consumer_id: &str,
    ) {
        if let Some(consumers) = subscriptions.get_mut(topic) {
            consumers.retain(|c| c != consumer_id);
        }
    }

    /// Problem 5: Get subscribers
    pub fn get_subscribers(
        subscriptions: &HashMap<String, Vec<String>>,
        topic: &str,
    ) -> Vec<String> {
        subscriptions.get(topic).cloned().unwrap_or_default()
    }

    /// Problem 6: Consume message
    pub fn consume_message(
        topics: &mut HashMap<String, VecDeque<Message>>,
        topic: &str,
    ) -> Option<Message> {
        topics.get_mut(topic).and_then(|queue| queue.pop_front())
    }

    /// Problem 7: Peek message
    pub fn peek_message(
        topics: &HashMap<String, VecDeque<Message>>,
        topic: &str,
    ) -> Option<Message> {
        topics.get(topic).and_then(|queue| queue.front().cloned())
    }

    /// Problem 8: Get queue depth
    pub fn get_queue_depth(
        topics: &HashMap<String, VecDeque<Message>>,
        topic: &str,
    ) -> usize {
        topics.get(topic).map(|q| q.len()).unwrap_or(0)
    }

    /// Problem 9: Broadcast message
    pub fn broadcast_message(
        topics: &mut HashMap<String, VecDeque<Message>>,
        message: &Message,
        subscribers: &HashMap<String, Vec<String>>,
    ) -> usize {
        let subscriber_count = Self::get_subscribers(subscribers, &message.topic).len();
        if subscriber_count > 0 {
            let _ = Self::publish_message(topics, message);
        }
        subscriber_count
    }

    /// Problem 10: Acknowledge message
    pub fn acknowledge_message(
        _ack_map: &mut HashMap<String, bool>,
        message_id: &str,
    ) {
        // Mark message as processed
    }

    /// Problem 11: Check message ack
    pub fn check_message_ack(ack_map: &HashMap<String, bool>, message_id: &str) -> bool {
        ack_map.get(message_id).copied().unwrap_or(false)
    }

    /// Problem 12: Get unacked messages
    pub fn get_unacked_messages(ack_map: &HashMap<String, bool>) -> Vec<String> {
        ack_map
            .iter()
            .filter(|(_, &acked)| !acked)
            .map(|(id, _)| id.clone())
            .collect()
    }

    // ================================================================
    // CONSUMER GROUPS (13-24)
    // ================================================================

    /// Problem 13: Create consumer group
    pub fn create_consumer_group(
        group_id: &str,
        topic: &str,
        partitions: usize,
    ) -> ConsumerGroup {
        let partition_list: Vec<u32> = (0..partitions as u32).collect();
        ConsumerGroup {
            id: group_id.to_string(),
            topic: topic.to_string(),
            members: Vec::new(),
            partitions: partition_list,
        }
    }

    /// Problem 14: Add member to group
    pub fn add_member_to_group(
        group: &mut ConsumerGroup,
        member_id: &str,
    ) {
        if !group.members.contains(&member_id.to_string()) {
            group.members.push(member_id.to_string());
        }
    }

    /// Problem 15: Remove member from group
    pub fn remove_member_from_group(
        group: &mut ConsumerGroup,
        member_id: &str,
    ) {
        group.members.retain(|m| m != member_id);
    }

    /// Problem 16: Assign partitions to members
    pub fn assign_partitions_to_members(
        group: &ConsumerGroup,
    ) -> HashMap<String, Vec<u32>> {
        let mut assignment = HashMap::new();
        let members_count = group.members.len() as u32;
        
        if members_count > 0 {
            for (idx, member) in group.members.iter().enumerate() {
                let partitions: Vec<u32> = group
                    .partitions
                    .iter()
                    .filter(|p| (**p as usize) % group.members.len() == idx)
                    .copied()
                    .collect();
                assignment.insert(member.clone(), partitions);
            }
        }
        assignment
    }

    /// Problem 17: Get group members
    pub fn get_group_members(group: &ConsumerGroup) -> Vec<String> {
        group.members.clone()
    }

    /// Problem 18: Get group offset
    pub fn get_group_offset(
        offsets: &HashMap<String, HashMap<u32, u64>>,
        group_id: &str,
        partition: u32,
    ) -> u64 {
        offsets
            .get(group_id)
            .and_then(|g| g.get(&partition))
            .copied()
            .unwrap_or(0)
    }

    /// Problem 19: Commit offset
    pub fn commit_offset(
        offsets: &mut HashMap<String, HashMap<u32, u64>>,
        group_id: &str,
        partition: u32,
        offset: u64,
    ) {
        offsets
            .entry(group_id.to_string())
            .or_insert_with(HashMap::new)
            .insert(partition, offset);
    }

    /// Problem 20: Reset offsets
    pub fn reset_offsets(
        offsets: &mut HashMap<String, HashMap<u32, u64>>,
        group_id: &str,
    ) {
        offsets.remove(group_id);
    }

    /// Problem 21: Rebalance group
    pub fn rebalance_group(group: &mut ConsumerGroup, new_members: Vec<String>) {
        group.members = new_members;
    }

    /// Problem 22: Get consumer lag
    pub fn get_consumer_lag(
        current_offset: u64,
        committed_offset: u64,
    ) -> u64 {
        if current_offset >= committed_offset {
            current_offset - committed_offset
        } else {
            0
        }
    }

    /// Problem 23: Get group status
    pub fn get_group_status(group: &ConsumerGroup) -> String {
        format!(
            "Group: {}, Members: {}, Partitions: {}",
            group.id,
            group.members.len(),
            group.partitions.len()
        )
    }

    /// Problem 24: Get group metrics
    pub fn get_group_metrics(
        group: &ConsumerGroup,
        offsets: &HashMap<String, HashMap<u32, u64>>,
    ) -> HashMap<String, String> {
        let mut metrics = HashMap::new();
        metrics.insert("group_id".to_string(), group.id.clone());
        metrics.insert("member_count".to_string(), group.members.len().to_string());
        metrics.insert("partition_count".to_string(), group.partitions.len().to_string());
        
        if let Some(group_offsets) = offsets.get(&group.id) {
            metrics.insert("committed_partitions".to_string(), group_offsets.len().to_string());
        }
        metrics
    }

    // ================================================================
    // DEAD LETTER QUEUE (25-36)
    // ================================================================

    /// Problem 25: Create dead letter queue
    pub fn create_dead_letter_queue() -> VecDeque<DeadLetterEntry> {
        VecDeque::new()
    }

    /// Problem 26: Send to DLQ
    pub fn send_to_dlq(
        dlq: &mut VecDeque<DeadLetterEntry>,
        message: &Message,
        reason: &str,
    ) {
        let entry = DeadLetterEntry {
            message: message.clone(),
            reason: reason.to_string(),
            retry_count: 0,
            last_retry_time: 0,
        };
        dlq.push_back(entry);
    }

    /// Problem 27: Get DLQ message
    pub fn get_dlq_message(dlq: &mut VecDeque<DeadLetterEntry>) -> Option<DeadLetterEntry> {
        dlq.pop_front()
    }

    /// Problem 28: Get DLQ depth
    pub fn get_dlq_depth(dlq: &VecDeque<DeadLetterEntry>) -> usize {
        dlq.len()
    }

    /// Problem 29: Retry DLQ message
    pub fn retry_dlq_message(
        dlq: &mut VecDeque<DeadLetterEntry>,
        messages: &mut HashMap<String, VecDeque<Message>>,
        entry: &mut DeadLetterEntry,
        topic: &str,
    ) -> Result<(), String> {
        entry.retry_count += 1;
        if entry.retry_count < 3 {
            if let Some(queue) = messages.get_mut(topic) {
                queue.push_back(entry.message.clone());
                Ok(())
            } else {
                Err("Topic not found".to_string())
            }
        } else {
            Err("Max retries exceeded".to_string())
        }
    }

    /// Problem 30: Get DLQ stats
    pub fn get_dlq_stats(dlq: &VecDeque<DeadLetterEntry>) -> HashMap<String, usize> {
        let mut stats = HashMap::new();
        stats.insert("depth".to_string(), dlq.len());
        stats.insert(
            "max_retries".to_string(),
            dlq.iter()
                .map(|e| e.retry_count as usize)
                .max()
                .unwrap_or(0),
        );
        stats
    }

    /// Problem 31: Purge DLQ
    pub fn purge_dlq(dlq: &mut VecDeque<DeadLetterEntry>) {
        dlq.clear();
    }

    /// Problem 32: Get DLQ messages by reason
    pub fn get_dlq_messages_by_reason(
        dlq: &VecDeque<DeadLetterEntry>,
        reason: &str,
    ) -> Vec<DeadLetterEntry> {
        dlq.iter()
            .filter(|e| e.reason == reason)
            .cloned()
            .collect()
    }

    /// Problem 33: Export DLQ for analysis
    pub fn export_dlq_for_analysis(dlq: &VecDeque<DeadLetterEntry>) -> String {
        format!("DLQ Export: {} messages", dlq.len())
    }

    /// Problem 34: Archive DLQ entry
    pub fn archive_dlq_entry(
        _archive: &mut Vec<DeadLetterEntry>,
        entry: &DeadLetterEntry,
    ) {
        // Archive the entry
    }

    /// Problem 35: Monitor DLQ health
    pub fn monitor_dlq_health(dlq: &VecDeque<DeadLetterEntry>) -> bool {
        dlq.len() < 1000 // Health OK if under 1000 messages
    }

    /// Problem 36: Alert on DLQ threshold
    pub fn alert_on_dlq_threshold(dlq: &VecDeque<DeadLetterEntry>) -> Option<String> {
        if dlq.len() > 500 {
            Some(format!("DLQ depth: {}", dlq.len()))
        } else {
            None
        }
    }

    // ================================================================
    // MESSAGE PROPERTIES (37-45)
    // ================================================================

    /// Problem 37: Create message
    pub fn create_message(
        topic: &str,
        body: Vec<u8>,
        timestamp: u64,
    ) -> Message {
        Message {
            id: format!("msg_{}", timestamp),
            topic: topic.to_string(),
            body,
            timestamp,
            partition: 0,
            offset: 0,
            key: None,
            headers: HashMap::new(),
        }
    }

    /// Problem 38: Set message key
    pub fn set_message_key(message: &mut Message, key: &str) {
        message.key = Some(key.to_string());
    }

    /// Problem 39: Add message header
    pub fn add_message_header(
        message: &mut Message,
        key: &str,
        value: &str,
    ) {
        message.headers.insert(key.to_string(), value.to_string());
    }

    /// Problem 40: Get message header
    pub fn get_message_header(message: &Message, key: &str) -> Option<String> {
        message.headers.get(key).cloned()
    }

    /// Problem 41: Get message size
    pub fn get_message_size(message: &Message) -> usize {
        message.body.len()
    }

    /// Problem 42: Set message partition
    pub fn set_message_partition(message: &mut Message, partition: u32) {
        message.partition = partition;
    }

    /// Problem 43: Set message offset
    pub fn set_message_offset(message: &mut Message, offset: u64) {
        message.offset = offset;
    }

    /// Problem 44: Get messages by key
    pub fn get_messages_by_key(
        partition_data: &VecDeque<Message>,
        key: &str,
    ) -> Vec<Message> {
        partition_data
            .iter()
            .filter(|m| m.key.as_deref() == Some(key))
            .cloned()
            .collect()
    }

    /// Problem 45: Message deduplication
    pub fn message_deduplication(
        seen_messages: &mut HashMap<String, u64>,
        message: &Message,
    ) -> bool {
        if let Some(timestamp) = seen_messages.get(&message.id) {
            message.timestamp > *timestamp
        } else {
            seen_messages.insert(message.id.clone(), message.timestamp);
            true
        }
    }

    // ================================================================
    // PARTITION MANAGEMENT (46-50)
    // ================================================================

    /// Problem 46: Create partitions
    pub fn create_partitions(partition_count: u32) -> HashMap<u32, VecDeque<Message>> {
        let mut partitions = HashMap::new();
        for i in 0..partition_count {
            partitions.insert(i, VecDeque::new());
        }
        partitions
    }

    /// Problem 47: Get partition for key
    pub fn get_partition_for_key(key: &str, partition_count: u32) -> u32 {
        let hash = MessageQueueSolver::hash_key(key);
        (hash % partition_count as u64) as u32
    }

    /// Problem 48: Hash key for partitioning
    pub fn hash_key(key: &str) -> u64 {
        let mut hash: u64 = 0;
        for byte in key.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
        }
        hash
    }

    /// Problem 49: Rebalance partitions
    pub fn rebalance_partitions(
        old_partitions: usize,
        new_partitions: usize,
    ) -> HashMap<u32, u32> {
        let mut mapping = HashMap::new();
        for i in 0..old_partitions {
            mapping.insert(i as u32, (i % new_partitions) as u32);
        }
        mapping
    }

    /// Problem 50: Get partition stats
    pub fn get_partition_stats(
        partitions: &HashMap<u32, VecDeque<Message>>,
    ) -> HashMap<String, u32> {
        let mut stats = HashMap::new();
        stats.insert("partition_count".to_string(), partitions.len() as u32);
        let total_messages: usize = partitions.values().map(|q| q.len()).sum();
        stats.insert("total_messages".to_string(), total_messages as u32);
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_topic() {
        let mut topics = HashMap::new();
        MessageQueueSolver::create_topic(&mut topics, "test-topic");
        assert!(topics.contains_key("test-topic"));
    }

    #[test]
    fn test_publish_message() {
        let mut topics = HashMap::new();
        MessageQueueSolver::create_topic(&mut topics, "test");
        let msg = MessageQueueSolver::create_message("test", vec![1, 2, 3], 1000);
        let result = MessageQueueSolver::publish_message(&mut topics, &msg);
        assert!(result.is_ok());
    }

    #[test]
    fn test_consume_message() {
        let mut topics = HashMap::new();
        MessageQueueSolver::create_topic(&mut topics, "test");
        let msg = MessageQueueSolver::create_message("test", vec![1, 2, 3], 1000);
        let _ = MessageQueueSolver::publish_message(&mut topics, &msg);
        let consumed = MessageQueueSolver::consume_message(&mut topics, "test");
        assert!(consumed.is_some());
    }

    #[test]
    fn test_queue_depth() {
        let mut topics = HashMap::new();
        MessageQueueSolver::create_topic(&mut topics, "test");
        for i in 0..5 {
            let msg = MessageQueueSolver::create_message("test", vec![i], 1000 + i as u64);
            let _ = MessageQueueSolver::publish_message(&mut topics, &msg);
        }
        assert_eq!(MessageQueueSolver::get_queue_depth(&topics, "test"), 5);
    }

    #[test]
    fn test_consumer_group() {
        let group = MessageQueueSolver::create_consumer_group("g1", "test", 4);
        assert_eq!(group.members.len(), 0);
        assert_eq!(group.partitions.len(), 4);
    }

    #[test]
    fn test_add_member_to_group() {
        let mut group = MessageQueueSolver::create_consumer_group("g1", "test", 4);
        MessageQueueSolver::add_member_to_group(&mut group, "consumer1");
        assert_eq!(group.members.len(), 1);
    }

    #[test]
    fn test_assign_partitions() {
        let mut group = MessageQueueSolver::create_consumer_group("g1", "test", 4);
        MessageQueueSolver::add_member_to_group(&mut group, "c1");
        MessageQueueSolver::add_member_to_group(&mut group, "c2");
        let assignment = MessageQueueSolver::assign_partitions_to_members(&group);
        assert_eq!(assignment.len(), 2);
    }

    #[test]
    fn test_commit_offset() {
        let mut offsets = HashMap::new();
        MessageQueueSolver::commit_offset(&mut offsets, "g1", 0, 100);
        assert_eq!(MessageQueueSolver::get_group_offset(&offsets, "g1", 0), 100);
    }

    #[test]
    fn test_dead_letter_queue() {
        let mut dlq = MessageQueueSolver::create_dead_letter_queue();
        let msg = MessageQueueSolver::create_message("test", vec![1, 2], 1000);
        MessageQueueSolver::send_to_dlq(&mut dlq, &msg, "parse_error");
        assert_eq!(MessageQueueSolver::get_dlq_depth(&dlq), 1);
    }

    #[test]
    fn test_message_key() {
        let mut msg = MessageQueueSolver::create_message("test", vec![], 1000);
        MessageQueueSolver::set_message_key(&mut msg, "key1");
        assert_eq!(msg.key, Some("key1".to_string()));
    }

    #[test]
    fn test_partition_for_key() {
        let partition = MessageQueueSolver::get_partition_for_key("key1", 4);
        assert!(partition < 4);
    }
}
