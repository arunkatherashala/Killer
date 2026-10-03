//! Lazy generators (coroutines).
//!
//! A function whose body contains `yield` starts with `Instruction::MakeGenerator`. Calling it
//! binds the arguments as usual, then `MakeGenerator` lifts the whole call frame (instruction
//! pointer, scope, local slots, operand stack, `try` handlers, closure captures) out of the VM
//! into a [`GenFrame`] stored here and returns `Value::Generator(id)` without running the body.
//!
//! `VirtualMachine::gen_resume` swaps the frame back in, runs until the next `Yield` (which saves
//! the frame again) or `Ret` (the generator is finished and removed). An id that is no longer
//! present therefore means "exhausted".

use crate::fast_hash::FastMap;
use crate::value::{Captures, Value};
use crate::vm::TryHandler;
use std::collections::HashMap;

/// A suspended generator call frame. `TryHandler` positions are stored relative to the frame's
/// bases (operand stack, scopes, locals) so the frame can be resumed at any VM depth.
pub struct GenFrame {
    /// The frame's scope plus any block scopes opened inside it.
    pub scopes: Vec<FastMap<String, Value>>,
    pub locals: Vec<Value>,
    pub stack: Vec<Value>,
    pub(crate) handlers: Vec<TryHandler>,
    pub captured: Option<Captures>,
    /// Where execution continues (the instruction after the `Yield`, or after `MakeGenerator`).
    pub ip: usize,
}

struct GenSlot {
    /// `None` while the generator is running (a re-entrant `next` is an error).
    frame: Option<GenFrame>,
    /// A value produced early by `has_next`, handed out by the next `next`.
    peeked: Option<Value>,
}

/// What `take_frame` found.
pub enum Taken {
    Frame(GenFrame),
    /// Finished, or never existed.
    Exhausted,
    Running,
}

/// Owns every live generator of a VM.
pub struct GeneratorManager {
    slots: HashMap<String, GenSlot>,
    gen_counter: usize,
}

impl GeneratorManager {
    pub fn new() -> Self {
        Self { slots: HashMap::new(), gen_counter: 0 }
    }

    /// Register a new, not yet started generator.
    pub fn create(&mut self, frame: GenFrame) -> String {
        let id = format!("gen_{}", self.gen_counter);
        self.gen_counter += 1;
        self.slots.insert(id.clone(), GenSlot { frame: Some(frame), peeked: None });
        id
    }

    /// A value `has_next` already produced, if any.
    pub fn take_peeked(&mut self, id: &str) -> Option<Value> {
        self.slots.get_mut(id).and_then(|s| s.peeked.take())
    }

    pub fn set_peeked(&mut self, id: &str, value: Value) {
        if let Some(s) = self.slots.get_mut(id) {
            s.peeked = Some(value);
        }
    }

    pub fn has_peeked(&self, id: &str) -> bool {
        self.slots.get(id).map_or(false, |s| s.peeked.is_some())
    }

    /// Take the frame out to run it.
    pub fn take_frame(&mut self, id: &str) -> Taken {
        match self.slots.get_mut(id) {
            None => Taken::Exhausted,
            Some(slot) => match slot.frame.take() {
                Some(f) => Taken::Frame(f),
                None => Taken::Running,
            },
        }
    }

    /// Put a suspended frame back after a `Yield`.
    pub fn put_frame(&mut self, id: &str, frame: GenFrame) {
        if let Some(slot) = self.slots.get_mut(id) {
            slot.frame = Some(frame);
        }
    }

    /// The generator finished (or failed): forget it.
    pub fn finish(&mut self, id: &str) {
        self.slots.remove(id);
    }

    /// True while the generator exists and has not finished (it may still turn out to be empty).
    pub fn is_live(&self, id: &str) -> bool {
        self.slots.contains_key(id)
    }

    pub fn live_count(&self) -> usize {
        self.slots.len()
    }

    pub fn clear(&mut self) {
        self.slots.clear();
        self.gen_counter = 0;
    }
}

impl Default for GeneratorManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(ip: usize) -> GenFrame {
        GenFrame {
            scopes: vec![Default::default()],
            locals: vec![Value::Number(1.0)],
            stack: Vec::new(),
            handlers: Vec::new(),
            captured: None,
            ip,
        }
    }

    #[test]
    fn create_gives_distinct_ids() {
        let mut gm = GeneratorManager::new();
        let a = gm.create(frame(1));
        let b = gm.create(frame(2));
        assert_ne!(a, b);
        assert!(a.starts_with("gen_"));
        assert_eq!(gm.live_count(), 2);
        assert!(gm.is_live(&a));
    }

    #[test]
    fn take_and_put_frame_round_trip() {
        let mut gm = GeneratorManager::new();
        let id = gm.create(frame(7));
        match gm.take_frame(&id) {
            Taken::Frame(f) => {
                assert_eq!(f.ip, 7);
                // while taken, a second take is a re-entrant resume
                assert!(matches!(gm.take_frame(&id), Taken::Running));
                gm.put_frame(&id, GenFrame { ip: 9, ..f });
            }
            _ => panic!("expected a frame"),
        }
        match gm.take_frame(&id) {
            Taken::Frame(f) => assert_eq!(f.ip, 9),
            _ => panic!("expected a frame"),
        }
    }

    #[test]
    fn finished_or_unknown_is_exhausted() {
        let mut gm = GeneratorManager::new();
        let id = gm.create(frame(1));
        gm.finish(&id);
        assert!(!gm.is_live(&id));
        assert!(matches!(gm.take_frame(&id), Taken::Exhausted));
        assert!(matches!(gm.take_frame("gen_nope"), Taken::Exhausted));
    }

    #[test]
    fn peeked_value_is_handed_out_once() {
        let mut gm = GeneratorManager::new();
        let id = gm.create(frame(1));
        assert!(!gm.has_peeked(&id));
        gm.set_peeked(&id, Value::Number(5.0));
        assert!(gm.has_peeked(&id));
        assert_eq!(gm.take_peeked(&id), Some(Value::Number(5.0)));
        assert_eq!(gm.take_peeked(&id), None);
    }

    #[test]
    fn clear_resets_everything() {
        let mut gm = GeneratorManager::new();
        gm.create(frame(1));
        gm.clear();
        assert_eq!(gm.live_count(), 0);
        assert_eq!(gm.create(frame(1)), "gen_0");
    }
}
