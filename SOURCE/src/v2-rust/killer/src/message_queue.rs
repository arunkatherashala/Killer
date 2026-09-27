//! Message Queue — Asynchronous message passing

use std::collections::VecDeque;
use std::sync::Mutex;

pub struct MessageQueue {
    _messages: Mutex<VecDeque<String>>,
}

impl MessageQueue {
    pub fn new() -> Self { Self { _messages: Mutex::new(VecDeque::new()) } }
}
