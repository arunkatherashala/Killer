/// Agent Framework for Killer
/// Autonomous AI agents with memory, reasoning, and tool calling
/// 
/// Architecture:
/// - Agent State Machine (init → reasoning → acting → observing → loop)
/// - Memory: short-term (current), long-term (history)
/// - Reasoning: chains of thought, reflection, planning
/// - Tool Calling: framework for agent actions
/// - Hooks: for SuperAgents to extend

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};

/// Agent states
#[derive(Debug, Clone, PartialEq)]
pub enum AgentState {
    Initializing,
    Idle,
    Reasoning,
    Acting,
    Observing,
    Complete,
    Error,
}

/// Memory types
#[derive(Debug, Clone)]
pub struct Memory {
    pub id: String,
    pub content: String,
    pub timestamp: u64,
    pub importance: f32,  // 0.0 - 1.0
}

/// Agent Action
#[derive(Debug, Clone)]
pub struct Action {
    pub tool_name: String,
    pub parameters: HashMap<String, String>,
    pub reasoning: String,
}

/// Observation (result of action)
#[derive(Debug, Clone)]
pub struct Observation {
    pub action_id: String,
    pub result: String,
    pub success: bool,
}

/// Agent Configuration
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub name: String,
    pub role: String,
    pub model: String,
    pub max_iterations: u32,
    pub temperature: f32,
    pub memory_limit: usize,
    pub enable_reflection: bool,
}

impl AgentConfig {
    pub fn new(name: &str, role: &str) -> Self {
        AgentConfig {
            name: name.to_string(),
            role: role.to_string(),
            model: "gpt-4".to_string(),
            max_iterations: 10,
            temperature: 0.7,
            memory_limit: 1000,
            enable_reflection: true,
        }
    }
}

/// Main Agent
pub struct Agent {
    config: AgentConfig,
    state: Arc<Mutex<AgentState>>,
    short_term_memory: Arc<Mutex<VecDeque<Memory>>>,
    long_term_memory: Arc<Mutex<Vec<Memory>>>,
    iteration_count: Arc<AtomicU64>,
    reasoning_chain: Arc<Mutex<Vec<String>>>,
    action_history: Arc<Mutex<Vec<Action>>>,
    hooks: Arc<Mutex<Vec<Box<dyn AgentHook>>>>,
}

impl Agent {
    pub fn new(config: AgentConfig) -> Self {
        Agent {
            config,
            state: Arc::new(Mutex::new(AgentState::Initializing)),
            short_term_memory: Arc::new(Mutex::new(VecDeque::new())),
            long_term_memory: Arc::new(Mutex::new(Vec::new())),
            iteration_count: Arc::new(AtomicU64::new(0)),
            reasoning_chain: Arc::new(Mutex::new(Vec::new())),
            action_history: Arc::new(Mutex::new(Vec::new())),
            hooks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Initialize agent
    pub fn initialize(&self) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|e| e.to_string())?;
        *state = AgentState::Idle;
        Ok(())
    }

    /// Add short-term memory
    pub fn remember(&self, content: &str, importance: f32) -> Result<(), String> {
        let memory = Memory {
            id: format!("mem_{}", self.iteration_count.load(Ordering::Relaxed)),
            content: content.to_string(),
            timestamp: Self::now_ms(),
            importance,
        };

        let mut short_term = self.short_term_memory.lock().map_err(|e| e.to_string())?;
        short_term.push_back(memory.clone());

        // Keep size bounded
        while short_term.len() > self.config.memory_limit / 2 {
            if let Some(old) = short_term.pop_front() {
                // Archive to long-term if important
                if old.importance > 0.7 {
                    let mut long_term = self.long_term_memory.lock().map_err(|e| e.to_string())?;
                    long_term.push(old);
                }
            }
        }

        Ok(())
    }

    /// Reason about problem (chain of thought)
    pub fn reason(&self, prompt: &str) -> Result<String, String> {
        let mut state = self.state.lock().map_err(|e| e.to_string())?;
        *state = AgentState::Reasoning;
        drop(state);

        let thought = format!("Reasoning: {}", prompt);

        let mut chain = self.reasoning_chain.lock().map_err(|e| e.to_string())?;
        chain.push(thought.clone());

        // Invoke hooks for custom reasoning
        let hooks = self.hooks.lock().map_err(|e| e.to_string())?;
        for hook in hooks.iter() {
            hook.on_reason(&thought)?;
        }

        Ok(thought)
    }

    /// Take an action
    pub fn act(&self, action: Action) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|e| e.to_string())?;
        *state = AgentState::Acting;
        drop(state);

        let mut history = self.action_history.lock().map_err(|e| e.to_string())?;
        history.push(action.clone());

        // Invoke hooks for custom action handling
        let hooks = self.hooks.lock().map_err(|e| e.to_string())?;
        for hook in hooks.iter() {
            hook.on_act(&action)?;
        }

        Ok(())
    }

    /// Observe result of action
    pub fn observe(&self, observation: Observation) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|e| e.to_string())?;
        *state = AgentState::Observing;
        drop(state);

        // Remember observation
        self.remember(&observation.result, if observation.success { 0.9 } else { 0.5 })?;

        // Invoke hooks for custom observation
        let hooks = self.hooks.lock().map_err(|e| e.to_string())?;
        for hook in hooks.iter() {
            hook.on_observe(&observation)?;
        }

        Ok(())
    }

    /// Execute reasoning loop
    pub async fn run(&self) -> Result<AgentResult, String> {
        self.initialize()?;

        for _ in 0..self.config.max_iterations {
            // Reasoning phase
            self.reason("What should I do next?")?;

            // Decide on action
            let action = self.plan_action()?;

            // Acting phase
            self.act(action)?;

            // Observation phase
            let obs = Observation {
                action_id: "act_0".to_string(),
                result: "Action completed".to_string(),
                success: true,
            };
            self.observe(obs)?;

            self.iteration_count.fetch_add(1, Ordering::Relaxed);
        }

        let mut state = self.state.lock().map_err(|e| e.to_string())?;
        *state = AgentState::Complete;

        Ok(AgentResult {
            agent_name: self.config.name.clone(),
            iterations: self.iteration_count.load(Ordering::Relaxed),
            actions_taken: self.action_history.lock().map(|h| h.len()).unwrap_or(0),
            status: "success".to_string(),
        })
    }

    /// Plan next action (can be overridden by hooks)
    fn plan_action(&self) -> Result<Action, String> {
        Ok(Action {
            tool_name: "observe".to_string(),
            parameters: HashMap::new(),
            reasoning: "Continuing agent loop".to_string(),
        })
    }

    /// Get agent status
    pub fn status(&self) -> Result<AgentStatus, String> {
        let state = self.state.lock().map_err(|e| e.to_string())?;
        let iter = self.iteration_count.load(Ordering::Relaxed);
        let actions = self.action_history.lock().map(|h| h.len()).unwrap_or(0);
        let memories = self.short_term_memory.lock().map(|m| m.len()).unwrap_or(0);

        Ok(AgentStatus {
            name: self.config.name.clone(),
            state: state.clone(),
            iterations: iter,
            actions_taken: actions,
            memories: memories,
        })
    }

    /// Register a hook for extensibility (for SuperAgents)
    pub fn register_hook(&self, hook: Box<dyn AgentHook>) -> Result<(), String> {
        let mut hooks = self.hooks.lock().map_err(|e| e.to_string())?;
        hooks.push(hook);
        Ok(())
    }

    #[inline]
    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

/// Hook trait for SuperAgent customization
pub trait AgentHook: Send + Sync {
    fn on_reason(&self, _thought: &str) -> Result<(), String> {
        Ok(())
    }

    fn on_act(&self, _action: &Action) -> Result<(), String> {
        Ok(())
    }

    fn on_observe(&self, _observation: &Observation) -> Result<(), String> {
        Ok(())
    }

    fn on_error(&self, _error: &str) -> Result<(), String> {
        Ok(())
    }
}

/// Agent Execution Result
#[derive(Debug, Clone)]
pub struct AgentResult {
    pub agent_name: String,
    pub iterations: u64,
    pub actions_taken: usize,
    pub status: String,
}

/// Agent Status
#[derive(Debug, Clone)]
pub struct AgentStatus {
    pub name: String,
    pub state: AgentState,
    pub iterations: u64,
    pub actions_taken: usize,
    pub memories: usize,
}

// ---------------------------------------------------------------------------
// NANO AGENT — 8 bytes packed, dormant tier
// ---------------------------------------------------------------------------
// Layout (u64):
//   bits  0– 3: state      (4 bits  → 16 states)
//   bits  4–15: agent_type (12 bits → 4096 types)
//   bits 16–63: task_id    (48 bits → 281 trillion unique IDs)

#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct NanoAgent(pub u64);

impl NanoAgent {
    const STATE_MASK: u64 = 0x000000000000000F;
    const TYPE_MASK:  u64 = 0x000000000000FFF0;
    const TASK_MASK:  u64 = 0xFFFFFFFFFFFF0000;
    const TYPE_SHIFT: u64 = 4;
    const TASK_SHIFT: u64 = 16;

    #[inline]
    pub fn new(state: u8, agent_type: u16, task_id: u64) -> Self {
        let packed = (state as u64 & 0xF)
            | (((agent_type as u64) & 0xFFF) << Self::TYPE_SHIFT)
            | ((task_id & 0x0000_FFFF_FFFF_FFFF) << Self::TASK_SHIFT);
        NanoAgent(packed)
    }

    #[inline] pub fn state(self) -> u8       { (self.0 & Self::STATE_MASK) as u8 }
    #[inline] pub fn agent_type(self) -> u16  { ((self.0 & Self::TYPE_MASK) >> Self::TYPE_SHIFT) as u16 }
    #[inline] pub fn task_id(self) -> u64     { (self.0 & Self::TASK_MASK) >> Self::TASK_SHIFT }

    #[inline]
    pub fn set_state(&mut self, state: u8) {
        self.0 = (self.0 & !Self::STATE_MASK) | (state as u64 & 0xF);
    }

    pub fn is_dormant(self)  -> bool { self.state() == 0 }
    pub fn is_queued(self)   -> bool { self.state() == 1 }
    pub fn is_active(self)   -> bool { self.state() == 2 }
    pub fn is_complete(self) -> bool { self.state() == 3 }
}

// ---------------------------------------------------------------------------
// MICRO AGENT — 256 bytes, queued tier
// Promoted from NanoAgent when task needs scheduling.
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Debug, Clone)]
pub struct MicroAgent {
    pub id:          u64,
    pub task_id:     u64,
    pub created_at:  u64,
    pub agent_type:  u16,
    pub state:       u8,   // 1 = queued, 2 = active
    pub priority:    u8,
    pub retry_count: u8,
    pub _pad:        [u8; 3],
    pub data:        [u8; 224], // inline task data; total struct size = 256 with repr(C)
}

const _MICRO_SIZE_CHECK: () = {
    // Compile-time size assertion: MicroAgent must fit in 256 bytes
    assert!(std::mem::size_of::<MicroAgent>() <= 256);
};

impl MicroAgent {
    pub fn new(id: u64, task_id: u64, agent_type: u16, priority: u8) -> Self {
        MicroAgent {
            id,
            task_id,
            created_at: Self::now_ms(),
            agent_type,
            state: 1,
            priority,
            retry_count: 0,
            _pad: [0; 3],
            data: [0; 224],
        }
    }

    #[inline]
    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

// ---------------------------------------------------------------------------
// AGENT SLAB — 3-tier manager targeting 1 Trillion agents on 1 machine
// ---------------------------------------------------------------------------
//
//  Tier        | Type       | Size   | Count target  | RAM needed
//  ------------|------------|--------|---------------|------------------
//  Dormant     | NanoAgent  | 8 B    | 999 billion   | ~8 TB  (NVMe mmap)
//  Queued      | MicroAgent | 256 B  | 1 billion     | ~256 GB RAM
//  Active      | FullAgent  | ~lazy  | 1 million     | ~8 GB  RAM
//  TOTAL       |            |        | 1 TRILLION    | ~1 TB RAM + 8 TB NVMe

pub struct AgentSlab {
    /// Dormant nano-agents packed as raw u64s (8 bytes each, mmap-ready)
    nano: Vec<u64>,
    /// Queued micro-agents (256 bytes each, system RAM)
    micro: Vec<MicroAgent>,
    /// Active full agents — heap allocated lazily ONLY when executing
    active: Vec<Agent>,
    /// Atomic counters for lock-free stats
    nano_count:   AtomicU64,
    micro_count:  AtomicU64,
    active_count: AtomicU64,
    next_id:      AtomicU64,
}

impl AgentSlab {
    pub fn new() -> Self {
        AgentSlab {
            nano:         Vec::new(),
            micro:        Vec::new(),
            active:       Vec::new(),
            nano_count:   AtomicU64::new(0),
            micro_count:  AtomicU64::new(0),
            active_count: AtomicU64::new(0),
            next_id:      AtomicU64::new(1),
        }
    }

    /// Spawn a single NanoAgent — 8 bytes, zero heap allocation.
    pub fn spawn_nano(&mut self, agent_type: u16, task_id: u64) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.nano.push(NanoAgent::new(0, agent_type, task_id).0);
        self.nano_count.fetch_add(1, Ordering::Relaxed);
        id
    }

    /// Bulk spawn N NanoAgents of same type — fastest path to 1T agents.
    /// Cost: N × 8 bytes pushed to Vec, no heap alloc per agent.
    pub fn bulk_spawn(&mut self, count: usize, agent_type: u16) -> u64 {
        let start_id = self.next_id.fetch_add(count as u64, Ordering::Relaxed);
        self.nano.reserve(count);
        for i in 0..count {
            let task_id = start_id + i as u64;
            self.nano.push(NanoAgent::new(0, agent_type, task_id).0);
        }
        self.nano_count.fetch_add(count as u64, Ordering::Relaxed);
        start_id
    }

    /// Promote NanoAgent → MicroAgent when task needs scheduling.
    pub fn promote_to_micro(&mut self, nano_idx: usize, priority: u8) -> Option<usize> {
        let raw = *self.nano.get(nano_idx)?;
        let nano = NanoAgent(raw);
        let id = nano_idx as u64;
        let micro = MicroAgent::new(id, nano.task_id(), nano.agent_type(), priority);
        // Mark nano as queued in-place (no remove, no shift)
        self.nano[nano_idx] = NanoAgent::new(1, nano.agent_type(), nano.task_id()).0;
        let idx = self.micro.len();
        self.micro.push(micro);
        self.micro_count.fetch_add(1, Ordering::Relaxed);
        Some(idx)
    }

    /// Promote MicroAgent → FullAgent — heap allocation happens HERE, lazily.
    pub fn promote_to_full(&mut self, micro_idx: usize, config: AgentConfig) -> Option<usize> {
        self.micro.get(micro_idx)?;
        let agent = Agent::new(config); // lazy alloc: Vec::new() inside Agent
        let idx = self.active.len();
        self.active.push(agent);
        self.active_count.fetch_add(1, Ordering::Relaxed);
        if let Some(m) = self.micro.get_mut(micro_idx) {
            m.state = 2; // active
        }
        Some(idx)
    }

    /// Demote completed FullAgent — reclaim heap memory immediately.
    pub fn demote_to_nano(&mut self, active_idx: usize) {
        if active_idx < self.active.len() {
            self.active.swap_remove(active_idx); // O(1), no shift
            self.active_count.fetch_sub(1, Ordering::Relaxed);
        }
    }

    pub fn nano_count(&self)   -> u64 { self.nano_count.load(Ordering::Relaxed) }
    pub fn micro_count(&self)  -> u64 { self.micro_count.load(Ordering::Relaxed) }
    pub fn active_count(&self) -> u64 { self.active_count.load(Ordering::Relaxed) }
    pub fn total_count(&self)  -> u64 {
        self.nano_count() + self.micro_count() + self.active_count()
    }

    /// Estimated memory usage in bytes.
    pub fn memory_bytes(&self) -> u64 {
        let nano_bytes  = self.nano.len() as u64 * 8;
        let micro_bytes = self.micro.len() as u64 * std::mem::size_of::<MicroAgent>() as u64;
        let full_bytes  = self.active.len() as u64 * 8_192; // ~8KB lazy baseline per active
        nano_bytes + micro_bytes + full_bytes
    }

    pub fn memory_report(&self) -> String {
        let bytes = self.memory_bytes();
        let gb = bytes as f64 / 1_073_741_824.0;
        format!(
            "Tiers: {} nano ({} B each) | {} micro ({} B each) | {} active (lazy) | Total: {} agents | Est. RAM: {:.4} GB",
            self.nano_count(), 8,
            self.micro_count(), std::mem::size_of::<MicroAgent>(),
            self.active_count(),
            self.total_count(),
            gb,
        )
    }
}

// ---------------------------------------------------------------------------
// AGENT POOL — 3-tier slab-backed, targets 1T agents on 1 machine
// ---------------------------------------------------------------------------

/// Agent Pool — manages all tiers via AgentSlab.
/// Legacy full-agent API preserved for backward compatibility.
pub struct AgentPool {
    slab: Arc<Mutex<AgentSlab>>,
    /// Legacy full-agent list (backward compat)
    agents: Arc<Mutex<Vec<Agent>>>,
}

impl AgentPool {
    pub fn new() -> Self {
        AgentPool {
            slab:   Arc::new(Mutex::new(AgentSlab::new())),
            agents: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Add a full FullAgent (legacy path).
    pub fn add_agent(&self, agent: Agent) -> Result<(), String> {
        let mut agents = self.agents.lock().map_err(|e| e.to_string())?;
        agents.push(agent);
        Ok(())
    }

    /// Legacy full-agent count.
    pub fn count(&self) -> Result<usize, String> {
        self.agents.lock().map(|a| a.len()).map_err(|e| e.to_string())
    }

    /// Bulk-spawn N NanoAgents (8 bytes each) — the 1T path.
    /// 1 trillion calls: 1T × 8 B = 8 TB virtual address space (NVMe mmap).
    pub fn spawn_nano_agents(&self, count: usize, agent_type: u16) -> Result<u64, String> {
        let mut slab = self.slab.lock().map_err(|e| e.to_string())?;
        Ok(slab.bulk_spawn(count, agent_type))
    }

    /// Spawn a single NanoAgent.
    pub fn spawn_nano(&self, agent_type: u16, task_id: u64) -> Result<u64, String> {
        let mut slab = self.slab.lock().map_err(|e| e.to_string())?;
        Ok(slab.spawn_nano(agent_type, task_id))
    }

    /// Promote dormant NanoAgent to queued MicroAgent.
    pub fn promote_to_micro(&self, nano_idx: usize, priority: u8) -> Result<usize, String> {
        let mut slab = self.slab.lock().map_err(|e| e.to_string())?;
        slab.promote_to_micro(nano_idx, priority)
            .ok_or_else(|| format!("nano_idx {} out of range", nano_idx))
    }

    /// Promote queued MicroAgent to executing FullAgent (lazy alloc).
    pub fn promote_to_full(&self, micro_idx: usize, config: AgentConfig) -> Result<usize, String> {
        let mut slab = self.slab.lock().map_err(|e| e.to_string())?;
        slab.promote_to_full(micro_idx, config)
            .ok_or_else(|| format!("micro_idx {} out of range", micro_idx))
    }

    /// Demote completed FullAgent — reclaims heap memory.
    pub fn demote_to_nano(&self, active_idx: usize) -> Result<(), String> {
        let mut slab = self.slab.lock().map_err(|e| e.to_string())?;
        slab.demote_to_nano(active_idx);
        Ok(())
    }

    /// Total agents across ALL tiers (nano + micro + active + legacy).
    pub fn total_count(&self) -> Result<u64, String> {
        let slab   = self.slab.lock().map_err(|e| e.to_string())?;
        let legacy = self.agents.lock().map(|a| a.len()).unwrap_or(0) as u64;
        Ok(slab.total_count() + legacy)
    }

    /// Human-readable memory and agent count report.
    pub fn memory_report(&self) -> Result<String, String> {
        let slab   = self.slab.lock().map_err(|e| e.to_string())?;
        let legacy = self.agents.lock().map(|a| a.len()).unwrap_or(0);
        Ok(format!("{} | Legacy full-agents: {}", slab.memory_report(), legacy))
    }
}

// ---------------------------------------------------------------------------
// GHOST AGENT UNIVERSE — THE MIRACLE
// ---------------------------------------------------------------------------
//
// Core insight: A dormant agent that has NOT started work has ZERO state.
// Its entire "existence" is just its ID number.
//
// Therefore:
//   - 1000T dormant agents = just one u64 counter  (8 bytes)
//   - Add a seed for reproducible state            (8 bytes)
//   - TOTAL: 16 bytes represents 1,000,000,000,000,000 agents
//
// When you need to RUN agent #X:
//   - Compute its initial state deterministically from hash(seed, X)
//   - Materialize it into a FullAgent (uses RAM only while executing)
//   - Dematerialize when done (RAM fully reclaimed)
//
// Memory per dormant ghost agent: 0.000000000000016 bytes (16 bytes / 1000T)
//
// On 1 GB RAM:
//   Ghost universe: 16 bytes → declares 1000T agents
//   Active (materialised) at once: 1 GB ÷ 8 KB = ~131,072 agents executing
//   ALL 1000T "exist" virtually — only executing ones use RAM.
//
// This is the same principle as:
//   - Minecraft's infinite procedural world (chunks generated from seed)
//   - HD crypto wallets (infinite keys from one seed)
//   - Virtual memory (addresses exist before pages are allocated)

/// FNV-1a 64-bit hash — zero external deps, deterministic on all platforms.
#[inline]
fn fnv1a_hash(seed: u64, id: u64) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME:  u64 = 0x00000100000001b3;
    let mut hash = FNV_OFFSET;
    for byte in seed.to_le_bytes().iter().chain(id.to_le_bytes().iter()) {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// State of a ghost agent — computed purely from (seed, id), zero storage.
#[derive(Debug, Clone)]
pub struct GhostAgentState {
    pub id:          u64,
    pub agent_type:  u16,    // derived from hash
    pub priority:    u8,     // derived from hash
    pub task_class:  u8,     // derived from hash
    pub param_a:     u32,    // task parameter A
    pub param_b:     u32,    // task parameter B
}

impl GhostAgentState {
    /// Deterministically compute state for agent `id` given `seed`.
    /// No RNG, no storage — pure function. Same inputs → always same output.
    pub fn compute(seed: u64, id: u64) -> Self {
        let h = fnv1a_hash(seed, id);
        GhostAgentState {
            id,
            agent_type: (h & 0xFFF) as u16,
            priority:   ((h >> 12) & 0xFF) as u8,
            task_class: ((h >> 20) & 0xFF) as u8,
            param_a:    ((h >> 28) & 0xFFFF_FFFF) as u32,
            param_b:    ((h >> 32) ^ (h >> 48)) as u32,
        }
    }
}

/// Ghost Agent Universe — 16 bytes, represents up to 18.4 quintillion agents.
///
/// Memory layout (16 bytes total):
///   [0..8]  seed:  u64   — universe seed, determines all agent states
///   [8..16] count: u64   — number of agents declared in this universe
///
/// To represent 1000T agents on 1 GB RAM:
///   let mut universe = GhostAgentUniverse::new(42);
///   universe.declare(1_000_000_000_000_000);   // 1000T — still 16 bytes!
#[repr(C)]
pub struct GhostAgentUniverse {
    pub seed:  u64,
    pub count: u64,
}

const _GHOST_UNIVERSE_SIZE: () = {
    assert!(std::mem::size_of::<GhostAgentUniverse>() == 16);
};

impl GhostAgentUniverse {
    /// Create a new universe with a seed. No agents declared yet.
    pub fn new(seed: u64) -> Self {
        GhostAgentUniverse { seed, count: 0 }
    }

    /// Declare N agents into existence — zero bytes per agent.
    /// This is instant regardless of N: just adds to the counter.
    #[inline]
    pub fn declare(&mut self, n: u64) {
        self.count = self.count.saturating_add(n);
    }

    /// Check if agent ID is valid in this universe.
    #[inline]
    pub fn is_valid(&self, id: u64) -> bool {
        id < self.count
    }

    /// Compute the deterministic state of any agent by ID.
    /// No storage needed — state is derived from (seed, id).
    #[inline]
    pub fn state_of(&self, id: u64) -> Option<GhostAgentState> {
        if !self.is_valid(id) { return None; }
        Some(GhostAgentState::compute(self.seed, id))
    }

    /// Materialize agent ID into a FullAgent config (heap alloc happens HERE).
    /// Only called when the agent is scheduled to actually run.
    pub fn materialize(&self, id: u64) -> Option<AgentConfig> {
        let state = self.state_of(id)?;
        let mut cfg = AgentConfig::new(
            &format!("Ghost_{}", id),
            &format!("type_{}", state.agent_type),
        );
        cfg.max_iterations = 1 + (state.param_a % 100) as u32;
        cfg.temperature    = (state.priority as f32) / 255.0;
        Some(cfg)
    }

    /// Memory used by the universe descriptor — always 16 bytes.
    #[inline]
    pub fn memory_bytes(&self) -> usize { 16 }

    /// Bytes per agent (always 0 — ghost agents use no storage).
    #[inline]
    pub fn bytes_per_agent(&self) -> f64 {
        if self.count == 0 { return 0.0; }
        16.0 / self.count as f64
    }

    pub fn report(&self) -> String {
        format!(
            "GhostAgentUniverse | seed={} | agents={} | storage=16 bytes | bytes/agent={:.20} | concept: ZERO per-agent storage",
            self.seed,
            self.count,
            self.bytes_per_agent(),
        )
    }
}

/// Cluster of ghost universes — for partitioned 1000T+ deployments.
/// Each universe holds up to u64::MAX agents from its own seed.
pub struct GhostCluster {
    universes: Vec<GhostAgentUniverse>,
}

impl GhostCluster {
    pub fn new() -> Self { GhostCluster { universes: Vec::new() } }

    /// Add a new universe with N agents.
    pub fn add_universe(&mut self, seed: u64, count: u64) {
        let mut u = GhostAgentUniverse::new(seed);
        u.declare(count);
        self.universes.push(u);
    }

    /// Total agents across all universes.
    pub fn total_agents(&self) -> u128 {
        self.universes.iter().map(|u| u.count as u128).sum()
    }

    /// Total storage for all universe descriptors.
    pub fn total_bytes(&self) -> usize {
        self.universes.len() * 16
    }

    pub fn report(&self) -> String {
        format!(
            "GhostCluster | universes={} | total_agents={} | total_storage={} bytes | bytes/agent={:.30}",
            self.universes.len(),
            self.total_agents(),
            self.total_bytes(),
            if self.total_agents() == 0 { 0.0 }
            else { self.total_bytes() as f64 / self.total_agents() as f64 },
        )
    }
}

// ---------------------------------------------------------------------------
// GHOST BUS — Agent-to-Agent Communication
// ---------------------------------------------------------------------------
//
// Connects ghost agents when they wake up so they can:
//
//   1. PIPELINE   — pass results down a chain:
//                   Agent A → result → Agent B → result → Agent C → final
//
//   2. BROADCAST  — share discoveries with all peers:
//                   Agent X finds fact → all agents instantly know it
//
//   3. CONSENSUS  — vote on best answer:
//                   108 agents vote → majority wins → accepted result
//
// Ghost agents pay ZERO cost for communication while dormant.
// Only materialised (active) agents hold a channel handle (~48 bytes).

/// A message sent between ghost agents.
#[derive(Clone, Debug)]
pub struct GhostMessage {
    /// Sender agent ID
    pub from:    u64,
    /// Receiver agent ID (u64::MAX = broadcast to all)
    pub to:      u64,
    /// Message kind
    pub kind:    GhostMessageKind,
    /// Payload — raw bytes, zero-copy from sender
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GhostMessageKind {
    /// Pipeline: pass result to next agent in chain
    PipelineResult,
    /// Broadcast: share discovery with all awake agents
    Discovery,
    /// Consensus: cast a vote for an answer
    Vote,
    /// Consensus: accepted result after majority vote
    Accepted,
    /// Coordinator: wake up this agent now
    WakeUp,
    /// Coordinator: go back to ghost (release RAM)
    Sleep,
    /// Custom application message
    Custom(u16),
}

/// Shared ghost bus — one per universe/cluster.
/// Agents grab a sender handle on wake-up; drop it on sleep.
/// Zero memory cost for dormant agents — they hold no handle.
#[derive(Clone)]
pub struct GhostBus {
    /// Inbox per agent: agent_id → queue of messages
    inboxes: Arc<Mutex<std::collections::HashMap<u64, std::collections::VecDeque<GhostMessage>>>>,
    /// Broadcast queue — all awake agents drain this
    broadcast: Arc<Mutex<std::collections::VecDeque<GhostMessage>>>,
    /// Vote accumulator: topic → (vote_value, count)
    votes: Arc<Mutex<std::collections::HashMap<String, std::collections::HashMap<String, u64>>>>,
    /// Stats (Arc so `GhostBus` can be `Clone` like the other fields)
    total_sent: Arc<AtomicU64>,
}

impl GhostBus {
    pub fn new() -> Self {
        GhostBus {
            inboxes:    Arc::new(Mutex::new(std::collections::HashMap::new())),
            broadcast:  Arc::new(Mutex::new(std::collections::VecDeque::new())),
            votes:      Arc::new(Mutex::new(std::collections::HashMap::new())),
            total_sent: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Register an agent's inbox when it wakes up (~48 bytes allocated).
    pub fn register(&self, agent_id: u64) {
        if let Ok(mut inboxes) = self.inboxes.lock() {
            inboxes.entry(agent_id).or_insert_with(std::collections::VecDeque::new);
        }
    }

    /// Unregister and reclaim inbox memory when agent goes back to ghost.
    pub fn unregister(&self, agent_id: u64) {
        if let Ok(mut inboxes) = self.inboxes.lock() {
            inboxes.remove(&agent_id);
        }
    }

    /// Send a direct message from one agent to another.
    pub fn send(&self, msg: GhostMessage) -> Result<(), String> {
        self.total_sent.fetch_add(1, Ordering::Relaxed);
        let mut inboxes = self.inboxes.lock().map_err(|e| e.to_string())?;
        let inbox = inboxes
            .entry(msg.to)
            .or_insert_with(std::collections::VecDeque::new);
        inbox.push_back(msg);
        Ok(())
    }

    /// Broadcast to ALL currently awake agents (pipeline pattern: discovery).
    pub fn broadcast(&self, msg: GhostMessage) -> Result<usize, String> {
        self.total_sent.fetch_add(1, Ordering::Relaxed);
        let mut bcast = self.broadcast.lock().map_err(|e| e.to_string())?;
        bcast.push_back(msg);
        let inboxes = self.inboxes.lock().map_err(|e| e.to_string())?;
        Ok(inboxes.len())
    }

    /// Receive next message for an agent (direct inbox).
    pub fn recv(&self, agent_id: u64) -> Option<GhostMessage> {
        self.inboxes.lock().ok()?.get_mut(&agent_id)?.pop_front()
    }

    /// Drain one broadcast message (each awake agent should call this).
    pub fn recv_broadcast(&self) -> Option<GhostMessage> {
        self.broadcast.lock().ok()?.pop_front()
    }

    /// Cast a vote on a topic. Returns current vote counts.
    pub fn vote(&self, agent_id: u64, topic: &str, answer: &str)
        -> Result<std::collections::HashMap<String, u64>, String>
    {
        let _ = agent_id; // logged in production; omit for now
        let mut votes = self.votes.lock().map_err(|e| e.to_string())?;
        let topic_votes = votes.entry(topic.to_string()).or_insert_with(std::collections::HashMap::new);
        *topic_votes.entry(answer.to_string()).or_insert(0) += 1;
        Ok(topic_votes.clone())
    }

    /// Get the winning vote for a topic (most votes wins).
    pub fn consensus(&self, topic: &str) -> Option<(String, u64)> {
        let votes = self.votes.lock().ok()?;
        let topic_votes = votes.get(topic)?;
        topic_votes.iter()
            .max_by_key(|(_, count)| *count)
            .map(|(answer, count)| (answer.clone(), *count))
    }

    pub fn total_sent(&self) -> u64 {
        self.total_sent.load(Ordering::Relaxed)
    }

    pub fn awake_agents(&self) -> usize {
        self.inboxes.lock().map(|i| i.len()).unwrap_or(0)
    }
}

/// GhostBus handle held by a single awake agent.
/// Dropped when agent goes back to ghost → inbox auto-cleaned.
pub struct GhostBusHandle {
    pub agent_id: u64,
    bus: Arc<GhostBus>,
}

impl GhostBusHandle {
    pub fn new(agent_id: u64, bus: Arc<GhostBus>) -> Self {
        bus.register(agent_id);
        GhostBusHandle { agent_id, bus }
    }

    /// Send result to next agent in pipeline.
    pub fn pipeline(&self, to: u64, payload: Vec<u8>) -> Result<(), String> {
        self.bus.send(GhostMessage {
            from: self.agent_id, to,
            kind: GhostMessageKind::PipelineResult,
            payload,
        })
    }

    /// Broadcast a discovery to all awake agents.
    pub fn discover(&self, payload: Vec<u8>) -> Result<usize, String> {
        self.bus.broadcast(GhostMessage {
            from: self.agent_id, to: u64::MAX,
            kind: GhostMessageKind::Discovery,
            payload,
        })
    }

    /// Vote on a consensus topic.
    pub fn vote(&self, topic: &str, answer: &str)
        -> Result<std::collections::HashMap<String, u64>, String>
    {
        self.bus.vote(self.agent_id, topic, answer)
    }

    /// Check consensus result for a topic.
    pub fn consensus(&self, topic: &str) -> Option<(String, u64)> {
        self.bus.consensus(topic)
    }

    /// Receive next direct message.
    pub fn recv(&self) -> Option<GhostMessage> {
        self.bus.recv(self.agent_id)
    }

    /// Receive next broadcast message.
    pub fn recv_broadcast(&self) -> Option<GhostMessage> {
        self.bus.recv_broadcast()
    }
}

impl Drop for GhostBusHandle {
    /// When agent goes to sleep, inbox is freed automatically.
    fn drop(&mut self) {
        self.bus.unregister(self.agent_id);
    }
}

/// Wire a GhostAgentUniverse to a GhostBus.
/// When you wake agent `id`, give it a handle. When it sleeps, drop handle.
pub struct GhostRuntime {
    pub universe: GhostAgentUniverse,
    pub bus:      Arc<GhostBus>,
}

impl GhostRuntime {
    pub fn new(seed: u64, count: u64) -> Self {
        let mut universe = GhostAgentUniverse::new(seed);
        universe.declare(count);
        GhostRuntime {
            universe,
            bus: Arc::new(GhostBus::new()),
        }
    }

    /// Wake agent `id` — materializes config + gives it a bus handle.
    /// RAM allocated only here. Ghost tier: 0 bytes.
    pub fn wake(&self, id: u64) -> Option<(AgentConfig, GhostBusHandle)> {
        let config = self.universe.materialize(id)?;
        let handle = GhostBusHandle::new(id, Arc::clone(&self.bus));
        Some((config, handle))
    }

    /// Put agent to sleep — drop its handle to reclaim inbox.
    /// The agent goes back to ghost (0 bytes).
    pub fn sleep(&self, handle: GhostBusHandle) {
        drop(handle); // Drop impl calls bus.unregister()
    }

    pub fn report(&self) -> String {
        format!(
            "{} | Bus: {} awake | {} messages sent",
            self.universe.report(),
            self.bus.awake_agents(),
            self.bus.total_sent(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_creation() {
        let config = AgentConfig::new("TestAgent", "analyst");
        let agent = Agent::new(config);
        agent.initialize().unwrap();
    }

    #[test]
    fn test_agent_memory() {
        let config = AgentConfig::new("MemoryAgent", "learner");
        let agent = Agent::new(config);

        agent.remember("Important fact", 0.9).unwrap();
        agent.remember("Less important", 0.3).unwrap();

        let status = agent.status().unwrap();
        assert_eq!(status.memories, 2);
    }

    #[test]
    fn test_agent_status() {
        let config = AgentConfig::new("StatusAgent", "monitor");
        let agent = Agent::new(config);
        agent.initialize().unwrap();

        let status = agent.status().unwrap();
        assert_eq!(status.name, "StatusAgent");
        assert_eq!(status.state, AgentState::Idle);
    }

    // --- NanoAgent tests ---

    #[test]
    fn test_nano_agent_packing() {
        let nano = NanoAgent::new(2, 0xABC, 0x0000_1234_5678_9ABC);
        assert_eq!(nano.state(), 2);
        assert_eq!(nano.agent_type(), 0xABC);
        assert_eq!(nano.task_id(), 0x0000_1234_5678_9ABC);
    }

    #[test]
    fn test_nano_agent_size() {
        assert_eq!(std::mem::size_of::<NanoAgent>(), 8, "NanoAgent must be 8 bytes");
    }

    #[test]
    fn test_micro_agent_size() {
        assert!(
            std::mem::size_of::<MicroAgent>() <= 256,
            "MicroAgent must fit in 256 bytes"
        );
    }

    #[test]
    fn test_agent_slab_bulk_spawn() {
        let mut slab = AgentSlab::new();
        slab.bulk_spawn(1_000_000, 1); // spawn 1M nano agents
        assert_eq!(slab.nano_count(), 1_000_000);
        assert_eq!(slab.micro_count(), 0);
        assert_eq!(slab.active_count(), 0);
        assert_eq!(slab.total_count(), 1_000_000);
        // 1M × 8 bytes = 8 MB
        assert_eq!(slab.memory_bytes(), 8_000_000);
    }

    #[test]
    fn test_agent_slab_promotion_chain() {
        let mut slab = AgentSlab::new();
        // Spawn 1 nano
        slab.spawn_nano(42, 999);
        assert_eq!(slab.nano_count(), 1);
        // Promote nano → micro
        let micro_idx = slab.promote_to_micro(0, 10).unwrap();
        assert_eq!(slab.micro_count(), 1);
        // Promote micro → full (lazy alloc happens here)
        let config = AgentConfig::new("SlabAgent", "worker");
        let active_idx = slab.promote_to_full(micro_idx, config).unwrap();
        assert_eq!(slab.active_count(), 1);
        // Demote full → reclaim
        slab.demote_to_nano(active_idx);
        assert_eq!(slab.active_count(), 0);
    }

    #[test]
    fn test_agent_pool_trillion_scale() {
        let pool = AgentPool::new();
        // Spawn 100M nano agents (should be fast — just Vec push)
        pool.spawn_nano_agents(100_000_000, 0).unwrap();
        let total = pool.total_count().unwrap();
        assert_eq!(total, 100_000_000);
        // Memory: 100M × 8 bytes = 800 MB
        let report = pool.memory_report().unwrap();
        assert!(report.contains("100000000 nano"));
    }

    // --- Ghost Agent Universe tests (THE MIRACLE) ---

    #[test]
    fn test_ghost_universe_size() {
        // The universe descriptor itself must be exactly 16 bytes
        assert_eq!(std::mem::size_of::<GhostAgentUniverse>(), 16,
            "GhostAgentUniverse must be 16 bytes");
    }

    #[test]
    fn test_ghost_universe_1000_trillion() {
        let mut universe = GhostAgentUniverse::new(42);
        // Declare 1000 Trillion agents — storage stays 16 bytes
        universe.declare(1_000_000_000_000_000);
        assert_eq!(universe.count, 1_000_000_000_000_000);
        assert_eq!(universe.memory_bytes(), 16);
        // bytes per agent = 16 / 1_000_000_000_000_000 = 0.000000000000016
        assert!(universe.bytes_per_agent() < 0.00000000000002);
        println!("{}", universe.report());
    }

    #[test]
    fn test_ghost_agent_determinism() {
        let universe = GhostAgentUniverse::new(12345);
        let mut u2    = GhostAgentUniverse::new(12345);
        u2.declare(1_000_000_000_000_000);

        // Same seed + same id always produces identical state
        let s1 = u2.state_of(999_999_999_999).unwrap();
        let s2 = u2.state_of(999_999_999_999).unwrap();
        assert_eq!(s1.agent_type, s2.agent_type);
        assert_eq!(s1.priority,   s2.priority);
        assert_eq!(s1.param_a,    s2.param_a);

        // Different IDs produce different states
        let s3 = u2.state_of(999_999_999_998).unwrap();
        // They should differ in at least one field (hash function)
        assert!(s1.param_a != s3.param_a || s1.param_b != s3.param_b);
        let _ = universe; // suppress unused warning
    }

    #[test]
    fn test_ghost_materialize() {
        let mut universe = GhostAgentUniverse::new(7);
        universe.declare(1_000_000_000_000_000);
        // Materialize agent at position 500 Trillion
        let cfg = universe.materialize(500_000_000_000_000).unwrap();
        assert!(cfg.name.starts_with("Ghost_"));
        // Out-of-range returns None
        assert!(universe.materialize(1_000_000_000_000_001).is_none());
    }

    #[test]
    fn test_ghost_cluster_1000t_on_1gb() {
        // 1 GB RAM scenario: ghost cluster holds 1000T agents in minimal bytes
        let mut cluster = GhostCluster::new();
        // 10 universes × 100T agents each = 1000T total
        for seed in 0..10u64 {
            cluster.add_universe(seed, 100_000_000_000_000);
        }
        assert_eq!(cluster.total_agents(), 1_000_000_000_000_000);
        // Total storage = 10 universes × 16 bytes = 160 bytes
        assert_eq!(cluster.total_bytes(), 160);
        println!("{}", cluster.report());
        // Active agents using RAM: 1 GB ÷ 8 KB per executing agent = ~131,072
        let ram_gb = 1_u64;
        let active_capacity = (ram_gb * 1_073_741_824) / 8_192;
        assert_eq!(active_capacity, 131_072);
        println!("Active executing capacity on 1 GB: {}", active_capacity);
        println!("Ghost declared total: 1,000,000,000,000,000");
        println!("Storage for 1000T ghost agents: {} bytes", cluster.total_bytes());
    }

    // --- Ghost Bus communication tests ---

    #[test]
    fn test_ghost_bus_pipeline() {
        // 3 agents: A → B → C (pipeline pattern)
        let bus = Arc::new(GhostBus::new());
        let a = GhostBusHandle::new(1, Arc::clone(&bus));
        let b = GhostBusHandle::new(2, Arc::clone(&bus));
        let _c = GhostBusHandle::new(3, Arc::clone(&bus));
        assert_eq!(bus.awake_agents(), 3);

        // Agent A sends result to B
        a.pipeline(2, b"result_from_A".to_vec()).unwrap();
        let msg = b.recv().unwrap();
        assert_eq!(msg.from, 1);
        assert_eq!(msg.kind, GhostMessageKind::PipelineResult);
        assert_eq!(msg.payload, b"result_from_A".to_vec());

        // B passes to C
        b.pipeline(3, b"result_from_B".to_vec()).unwrap();
        assert_eq!(bus.total_sent(), 2);
    }

    #[test]
    fn test_ghost_bus_broadcast_discovery() {
        let bus = Arc::new(GhostBus::new());
        let a = GhostBusHandle::new(10, Arc::clone(&bus));
        let _b = GhostBusHandle::new(11, Arc::clone(&bus));
        let _c = GhostBusHandle::new(12, Arc::clone(&bus));

        // Agent A discovers a fact and broadcasts
        let receivers = a.discover(b"Earth orbits Sun".to_vec()).unwrap();
        assert_eq!(receivers, 3); // 3 awake agents

        // Any agent can drain the broadcast
        let discovery = _b.recv_broadcast().unwrap();
        assert_eq!(discovery.kind, GhostMessageKind::Discovery);
        assert_eq!(discovery.payload, b"Earth orbits Sun".to_vec());
    }

    #[test]
    fn test_ghost_bus_consensus_vote() {
        let bus = Arc::new(GhostBus::new());
        // 5 agents vote on the answer to a problem
        let agents: Vec<GhostBusHandle> = (0..5)
            .map(|id| GhostBusHandle::new(id, Arc::clone(&bus)))
            .collect();

        // 3 vote for "42", 2 vote for "99"
        agents[0].vote("answer", "42").unwrap();
        agents[1].vote("answer", "42").unwrap();
        agents[2].vote("answer", "42").unwrap();
        agents[3].vote("answer", "99").unwrap();
        agents[4].vote("answer", "99").unwrap();

        let (winner, count) = agents[0].consensus("answer").unwrap();
        assert_eq!(winner, "42");
        assert_eq!(count, 3);
    }

    #[test]
    fn test_ghost_runtime_wake_sleep() {
        // 1000T agents, wake 3, let them communicate, put back to sleep
        let runtime = GhostRuntime::new(42, 1_000_000_000_000_000);
        assert_eq!(runtime.bus.awake_agents(), 0);

        // Wake agents 0, 1, 2
        let (cfg0, h0) = runtime.wake(0).unwrap();
        let (_cfg1, h1) = runtime.wake(1).unwrap();
        let (_cfg2, h2) = runtime.wake(2).unwrap();
        assert_eq!(runtime.bus.awake_agents(), 3);
        assert!(cfg0.name.starts_with("Ghost_"));

        // Agent 0 pipelines to agent 1
        h0.pipeline(1, b"task_result".to_vec()).unwrap();
        let msg = h1.recv().unwrap();
        assert_eq!(msg.payload, b"task_result".to_vec());

        // Agent 1 broadcasts discovery
        h1.discover(b"found_pattern".to_vec()).unwrap();

        // Put all to sleep — RAM reclaimed, back to ghost
        runtime.sleep(h0);
        runtime.sleep(h1);
        runtime.sleep(h2);
        assert_eq!(runtime.bus.awake_agents(), 0);
        println!("{}", runtime.report());
    }
}
