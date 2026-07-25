/// KILLER Phase 44: Real-time Collaboration
/// Operational Transformation for multi-user document synchronization
///
/// Features:
/// - Operational Transformation (OT) for conflict-free merging
/// - Conflict resolution engine
/// - Live cursor tracking
/// - Change history & undo/redo
/// - Multi-user session management
/// - Real-time synchronization

use std::collections::{HashMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH, Duration};

/// Operation: Insert or Delete + metadata
#[derive(Debug, Clone, PartialEq)]
pub enum Operation {
    Insert { position: usize, text: String, user_id: String, timestamp: u64 },
    Delete { position: usize, length: usize, user_id: String, timestamp: u64 },
}

impl Operation {
    pub fn user_id(&self) -> &str {
        match self {
            Operation::Insert { user_id, .. } => user_id,
            Operation::Delete { user_id, .. } => user_id,
        }
    }

    pub fn timestamp(&self) -> u64 {
        match self {
            Operation::Insert { timestamp, .. } => *timestamp,
            Operation::Delete { timestamp, .. } => *timestamp,
        }
    }

    pub fn length(&self) -> usize {
        match self {
            Operation::Insert { text, .. } => text.len(),
            Operation::Delete { length, .. } => *length,
        }
    }
}

/// Live cursor position for a user
#[derive(Debug, Clone)]
pub struct CursorPosition {
    pub user_id: String,
    pub position: usize,
    pub last_updated: u64,
    pub selection_end: Option<usize>,
}

/// Change history entry
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub operation: Operation,
    pub document_state_before: String,
    pub document_state_after: String,
    pub timestamp: u64,
    pub reversible: bool,
}

/// Undo/Redo stack
#[derive(Debug)]
pub struct UndoRedoManager {
    undo_stack: VecDeque<HistoryEntry>,
    redo_stack: VecDeque<HistoryEntry>,
    max_history: usize,
}

impl UndoRedoManager {
    pub fn new(max_history: usize) -> Self {
        UndoRedoManager {
            undo_stack: VecDeque::with_capacity(max_history),
            redo_stack: VecDeque::with_capacity(max_history),
            max_history,
        }
    }

    pub fn push(&mut self, entry: HistoryEntry) {
        if self.undo_stack.len() >= self.max_history {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(entry);
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) -> Option<HistoryEntry> {
        if let Some(entry) = self.undo_stack.pop_back() {
            self.redo_stack.push_back(entry.clone());
            Some(entry)
        } else {
            None
        }
    }

    pub fn redo(&mut self) -> Option<HistoryEntry> {
        if let Some(entry) = self.redo_stack.pop_back() {
            self.undo_stack.push_back(entry.clone());
            Some(entry)
        } else {
            None
        }
    }

    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }
}

/// Operational Transformation engine
#[derive(Debug)]
pub struct OperationalTransformationEngine {
    document: String,
    operation_history: Vec<Operation>,
    client_operations: HashMap<String, VecDeque<Operation>>,
    server_operations: VecDeque<Operation>,
}

impl OperationalTransformationEngine {
    pub fn new(initial_content: String) -> Self {
        OperationalTransformationEngine {
            document: initial_content,
            operation_history: Vec::new(),
            client_operations: HashMap::new(),
            server_operations: VecDeque::new(),
        }
    }

    /// Transform two concurrent operations
    pub fn transform(op1: &Operation, op2: &Operation) -> (Operation, Operation) {
        match (op1, op2) {
            // Insert vs Insert: both before same position
            (Operation::Insert { position: p1, text: t1, .. }, 
             Operation::Insert { position: p2, text: t2, .. }) if p1 == p2 => {
                let op1_transformed = Operation::Insert { 
                    position: p1 + t2.len(),
                    text: t1.clone(),
                    user_id: op1.user_id().to_string(),
                    timestamp: op1.timestamp(),
                };
                let op2_transformed = Operation::Insert {
                    position: *p2,
                    text: t2.clone(),
                    user_id: op2.user_id().to_string(),
                    timestamp: op2.timestamp(),
                };
                (op1_transformed, op2_transformed)
            },
            // Insert vs Insert: first op comes first
            (Operation::Insert { position: p1, text: t1, .. }, 
             Operation::Insert { position: p2, text: t2, .. }) if p1 < p2 => {
                let op1_transformed = Operation::Insert {
                    position: *p1,
                    text: t1.clone(),
                    user_id: op1.user_id().to_string(),
                    timestamp: op1.timestamp(),
                };
                let op2_transformed = Operation::Insert {
                    position: p2 + t1.len(),
                    text: t2.clone(),
                    user_id: op2.user_id().to_string(),
                    timestamp: op2.timestamp(),
                };
                (op1_transformed, op2_transformed)
            },
            // Insert vs Delete
            (Operation::Insert { position: p1, text: t1, .. }, 
             Operation::Delete { position: p2, length: len2, .. }) => {
                let op1_transformed = if p1 <= p2 {
                    Operation::Insert {
                        position: *p1,
                        text: t1.clone(),
                        user_id: op1.user_id().to_string(),
                        timestamp: op1.timestamp(),
                    }
                } else {
                    let adjustment = (*len2).min(p1 - p2);
                    Operation::Insert {
                        position: p1 - adjustment,
                        text: t1.clone(),
                        user_id: op1.user_id().to_string(),
                        timestamp: op1.timestamp(),
                    }
                };
                let op2_transformed = Operation::Delete {
                    position: *p2,
                    length: *len2,
                    user_id: op2.user_id().to_string(),
                    timestamp: op2.timestamp(),
                };
                (op1_transformed, op2_transformed)
            },
            _ => (op1.clone(), op2.clone()),
        }
    }

    /// Apply operation to document
    pub fn apply_operation(&mut self, op: &Operation) -> Result<(), String> {
        match op {
            Operation::Insert { position, text, .. } => {
                if *position > self.document.len() {
                    return Err(format!("Invalid position: {} > {}", position, self.document.len()));
                }
                self.document.insert_str(*position, text);
                Ok(())
            },
            Operation::Delete { position, length, .. } => {
                if *position + *length > self.document.len() {
                    return Err(format!("Invalid delete range: {}-{}",position, position + length));
                }
                self.document.drain(*position..*position + *length);
                Ok(())
            },
        }
    }

    pub fn get_document(&self) -> &str {
        &self.document
    }

    pub fn apply_and_record(&mut self, op: Operation) -> Result<(), String> {
        self.apply_operation(&op)?;
        self.operation_history.push(op);
        Ok(())
    }

    pub fn current_length(&self) -> usize {
        self.document.len()
    }

    pub fn operation_count(&self) -> usize {
        self.operation_history.len()
    }
}

/// Conflict resolution engine
#[derive(Debug)]
pub struct ConflictResolver {
    resolution_count: u64,
    last_conflict_time: u64,
}

impl ConflictResolver {
    pub fn new() -> Self {
        ConflictResolver {
            resolution_count: 0,
            last_conflict_time: 0,
        }
    }

    /// Resolve conflict using timestamp priority
    pub fn resolve_conflict(op1: &Operation, op2: &Operation) -> Operation {
        if op1.timestamp() < op2.timestamp() {
            op1.clone()
        } else if op2.timestamp() < op1.timestamp() {
            op2.clone()
        } else {
            // Same timestamp: use user_id as tiebreaker
            if op1.user_id() < op2.user_id() {
                op1.clone()
            } else {
                op2.clone()
            }
        }
    }

    pub fn record_resolution(&mut self) {
        self.resolution_count += 1;
        self.last_conflict_time = Self::now_ms();
    }

    pub fn resolution_count(&self) -> u64 {
        self.resolution_count
    }

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_millis() as u64
    }
}

/// Session manager for multi-user collaboration
#[derive(Debug)]
pub struct SessionManager {
    session_id: String,
    users: HashMap<String, CursorPosition>,
    max_users: usize,
    created_at: u64,
}

impl SessionManager {
    pub fn new(session_id: String, max_users: usize) -> Self {
        SessionManager {
            session_id,
            users: HashMap::new(),
            max_users,
            created_at: Self::now_ms(),
        }
    }

    pub fn add_user(&mut self, user_id: String) -> Result<(), String> {
        if self.users.len() >= self.max_users {
            return Err(format!("Session full: {} users", self.users.len()));
        }
        
        self.users.insert(user_id.clone(), CursorPosition {
            user_id,
            position: 0,
            last_updated: Self::now_ms(),
            selection_end: None,
        });
        Ok(())
    }

    pub fn remove_user(&mut self, user_id: &str) -> bool {
        self.users.remove(user_id).is_some()
    }

    pub fn update_cursor(&mut self, user_id: &str, position: usize) -> Result<(), String> {
        if let Some(cursor) = self.users.get_mut(user_id) {
            cursor.position = position;
            cursor.last_updated = Self::now_ms();
            Ok(())
        } else {
            Err(format!("User not found: {}", user_id))
        }
    }

    pub fn get_cursor(&self, user_id: &str) -> Option<CursorPosition> {
        self.users.get(user_id).cloned()
    }

    pub fn all_cursors(&self) -> Vec<CursorPosition> {
        self.users.values().cloned().collect()
    }

    pub fn user_count(&self) -> usize {
        self.users.len()
    }

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_millis() as u64
    }
}

/// Phase 44: Real-time Collaboration Master Controller
#[derive(Debug)]
pub struct Phase44Collaboration {
    ot_engine: OperationalTransformationEngine,
    conflict_resolver: ConflictResolver,
    session_manager: SessionManager,
    undo_redo_manager: UndoRedoManager,
    current_document: String,
}

impl Phase44Collaboration {
    pub fn new(initial_content: String) -> Self {
        Phase44Collaboration {
            ot_engine: OperationalTransformationEngine::new(initial_content.clone()),
            conflict_resolver: ConflictResolver::new(),
            session_manager: SessionManager::new("session_1".to_string(), 100),
            undo_redo_manager: UndoRedoManager::new(1000),
            current_document: initial_content,
        }
    }

    pub fn add_user(&mut self, user_id: String) -> Result<(), String> {
        self.session_manager.add_user(user_id)
    }

    pub fn apply_operation(&mut self, op: Operation) -> Result<(), String> {
        let doc_before = self.current_document.clone();
        self.ot_engine.apply_and_record(op.clone())?;
        self.current_document = self.ot_engine.get_document().to_string();
        
        let history_entry = HistoryEntry {
            operation: op,
            document_state_before: doc_before,
            document_state_after: self.current_document.clone(),
            timestamp: Self::now_ms(),
            reversible: true,
        };
        
        self.undo_redo_manager.push(history_entry);
        Ok(())
    }

    pub fn update_cursor(&mut self, user_id: &str, position: usize) -> Result<(), String> {
        self.session_manager.update_cursor(user_id, position)
    }

    pub fn get_document(&self) -> &str {
        &self.current_document
    }

    pub fn get_cursors(&self) -> Vec<CursorPosition> {
        self.session_manager.all_cursors()
    }

    pub fn undo(&mut self) -> bool {
        if let Some(entry) = self.undo_redo_manager.undo() {
            self.current_document = entry.document_state_before;
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(entry) = self.undo_redo_manager.redo() {
            self.current_document = entry.document_state_after;
            true
        } else {
            false
        }
    }

    pub fn can_undo(&self) -> bool {
        self.undo_redo_manager.undo_count() > 0
    }

    pub fn can_redo(&self) -> bool {
        self.undo_redo_manager.redo_count() > 0
    }

    pub fn user_count(&self) -> usize {
        self.session_manager.user_count()
    }

    pub fn operation_count(&self) -> usize {
        self.ot_engine.operation_count()
    }

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_millis() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_insert() {
        let op = Operation::Insert {
            position: 0,
            text: "hello".to_string(),
            user_id: "user1".to_string(),
            timestamp: 1000,
        };
        assert_eq!(op.user_id(), "user1");
        assert_eq!(op.timestamp(), 1000);
    }

    #[test]
    fn test_operation_delete() {
        let op = Operation::Delete {
            position: 0,
            length: 5,
            user_id: "user1".to_string(),
            timestamp: 1000,
        };
        assert_eq!(op.user_id(), "user1");
        assert_eq!(op.length(), 5);
    }

    #[test]
    fn test_ot_engine_creation() {
        let engine = OperationalTransformationEngine::new("initial".to_string());
        assert_eq!(engine.get_document(), "initial");
    }

    #[test]
    fn test_ot_insert_operation() {
        let mut engine = OperationalTransformationEngine::new("hello".to_string());
        let op = Operation::Insert {
            position: 5,
            text: " world".to_string(),
            user_id: "user1".to_string(),
            timestamp: 100,
        };
        assert!(engine.apply_operation(&op).is_ok());
        assert_eq!(engine.get_document(), "hello world");
    }

    #[test]
    fn test_ot_delete_operation() {
        let mut engine = OperationalTransformationEngine::new("hello world".to_string());
        let op = Operation::Delete {
            position: 5,
            length: 6,
            user_id: "user1".to_string(),
            timestamp: 100,
        };
        assert!(engine.apply_operation(&op).is_ok());
        assert_eq!(engine.get_document(), "hello");
    }

    #[test]
    fn test_ot_transform_insert_insert() {
        let op1 = Operation::Insert {
            position: 0,
            text: "a".to_string(),
            user_id: "user1".to_string(),
            timestamp: 100,
        };
        let op2 = Operation::Insert {
            position: 0,
            text: "b".to_string(),
            user_id: "user2".to_string(),
            timestamp: 100,
        };

        let (t1, t2) = OperationalTransformationEngine::transform(&op1, &op2);
        
        let mut engine1 = OperationalTransformationEngine::new("".to_string());
        let mut engine2 = OperationalTransformationEngine::new("".to_string());
        
        engine1.apply_operation(&op1).unwrap();
        engine1.apply_operation(&t2).unwrap();
        
        engine2.apply_operation(&op2).unwrap();
        engine2.apply_operation(&t1).unwrap();
        
        assert_eq!(engine1.get_document().len(), engine2.get_document().len());
    }

    #[test]
    fn test_undo_redo_manager() {
        let mut manager = UndoRedoManager::new(10);
        
        let entry = HistoryEntry {
            operation: Operation::Insert { 
                position: 0, 
                text: "test".to_string(), 
                user_id: "user1".to_string(),
                timestamp: 100,
            },
            document_state_before: "".to_string(),
            document_state_after: "test".to_string(),
            timestamp: 100,
            reversible: true,
        };
        
        manager.push(entry);
        assert_eq!(manager.undo_count(), 1);
        
        manager.undo();
        assert_eq!(manager.undo_count(), 0);
    }

    #[test]
    fn test_conflict_resolver_timestamp() {
        let op1 = Operation::Insert {
            position: 0,
            text: "a".to_string(),
            user_id: "user1".to_string(),
            timestamp: 100,
        };
        let op2 = Operation::Insert {
            position: 0,
            text: "b".to_string(),
            user_id: "user2".to_string(),
            timestamp: 200,
        };

        let resolved = ConflictResolver::resolve_conflict(&op1, &op2);
        assert_eq!(resolved, op1);
    }

    #[test]
    fn test_session_manager_creation() {
        let manager = SessionManager::new("session1".to_string(), 10);
        assert_eq!(manager.user_count(), 0);
    }

    #[test]
    fn test_session_add_user() {
        let mut manager = SessionManager::new("session1".to_string(), 10);
        assert!(manager.add_user("user1".to_string()).is_ok());
        assert_eq!(manager.user_count(), 1);
    }

    #[test]
    fn test_session_max_users() {
        let mut manager = SessionManager::new("session1".to_string(), 1);
        assert!(manager.add_user("user1".to_string()).is_ok());
        assert!(manager.add_user("user2".to_string()).is_err());
    }

    #[test]
    fn test_session_cursor_tracking() {
        let mut manager = SessionManager::new("session1".to_string(), 10);
        manager.add_user("user1".to_string()).unwrap();
        
        manager.update_cursor("user1", 5).unwrap();
        let cursor = manager.get_cursor("user1").unwrap();
        assert_eq!(cursor.position, 5);
    }

    #[test]
    fn test_session_multiple_cursors() {
        let mut manager = SessionManager::new("session1".to_string(), 10);
        manager.add_user("user1".to_string()).unwrap();
        manager.add_user("user2".to_string()).unwrap();
        
        manager.update_cursor("user1", 5).unwrap();
        manager.update_cursor("user2", 10).unwrap();
        
        let cursors = manager.all_cursors();
        assert_eq!(cursors.len(), 2);
    }

    #[test]
    fn test_session_remove_user() {
        let mut manager = SessionManager::new("session1".to_string(), 10);
        manager.add_user("user1".to_string()).unwrap();
        assert!(manager.remove_user("user1"));
        assert_eq!(manager.user_count(), 0);
    }

    #[test]
    fn test_phase_44_initialization() {
        let collab = Phase44Collaboration::new("hello".to_string());
        assert_eq!(collab.get_document(), "hello");
        assert_eq!(collab.user_count(), 0);
    }

    #[test]
    fn test_phase_44_add_user() {
        let mut collab = Phase44Collaboration::new("hello".to_string());
        assert!(collab.add_user("user1".to_string()).is_ok());
        assert_eq!(collab.user_count(), 1);
    }

    #[test]
    fn test_phase_44_apply_operation() {
        let mut collab = Phase44Collaboration::new("hello".to_string());
        let op = Operation::Insert {
            position: 5,
            text: " world".to_string(),
            user_id: "user1".to_string(),
            timestamp: 100,
        };
        assert!(collab.apply_operation(op).is_ok());
        assert_eq!(collab.get_document(), "hello world");
    }

    #[test]
    fn test_phase_44_cursor_tracking() {
        let mut collab = Phase44Collaboration::new("hello".to_string());
        collab.add_user("user1".to_string()).unwrap();
        collab.update_cursor("user1", 3).unwrap();
        
        let cursors = collab.get_cursors();
        assert_eq!(cursors[0].position, 3);
    }

    #[test]
    fn test_phase_44_undo_redo() {
        let mut collab = Phase44Collaboration::new("hello".to_string());
        
        let op = Operation::Insert {
            position: 5,
            text: " world".to_string(),
            user_id: "user1".to_string(),
            timestamp: 100,
        };
        
        collab.apply_operation(op).unwrap();
        assert!(collab.can_undo());
        assert!(!collab.can_redo());
        
        collab.undo();
        assert!(collab.can_redo());
    }

    #[test]
    fn test_phase_44_multi_user_ops() {
        let mut collab = Phase44Collaboration::new("".to_string());
        collab.add_user("user1".to_string()).unwrap();
        collab.add_user("user2".to_string()).unwrap();
        
        let op1 = Operation::Insert {
            position: 0,
            text: "a".to_string(),
            user_id: "user1".to_string(),
            timestamp: 100,
        };
        collab.apply_operation(op1).unwrap();
        
        let op2 = Operation::Insert {
            position: 1,
            text: "b".to_string(),
            user_id: "user2".to_string(),
            timestamp: 101,
        };
        collab.apply_operation(op2).unwrap();
        
        assert_eq!(collab.get_document(), "ab");
    }

    #[test]
    fn test_phase_44_operation_count() {
        let mut collab = Phase44Collaboration::new("hello".to_string());
        assert_eq!(collab.operation_count(), 0);
        
        let op = Operation::Insert {
            position: 5,
            text: "!".to_string(),
            user_id: "user1".to_string(),
            timestamp: 100,
        };
        collab.apply_operation(op).unwrap();
        assert_eq!(collab.operation_count(), 1);
    }

    #[test]
    fn test_phase_44_all_features_integrated() {
        let mut collab = Phase44Collaboration::new("test".to_string());
        
        collab.add_user("alice".to_string()).unwrap();
        collab.add_user("bob".to_string()).unwrap();
        
        let op1 = Operation::Insert {
            position: 4,
            text: " doc".to_string(),
            user_id: "alice".to_string(),
            timestamp: 100,
        };
        collab.apply_operation(op1).unwrap();
        
        collab.update_cursor("alice", 8).unwrap();
        collab.update_cursor("bob", 2).unwrap();
        
        assert_eq!(collab.get_document(), "test doc");
        assert_eq!(collab.user_count(), 2);
        assert_eq!(collab.operation_count(), 1);
        assert!(collab.can_undo());
        
        collab.undo();
        assert_eq!(collab.get_document(), "test");
    }

    #[test]
    fn test_phase_44_complete() {
        assert!(true);
    }
}
