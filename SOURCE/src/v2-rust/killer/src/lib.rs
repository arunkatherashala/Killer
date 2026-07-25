// lib.rs - killer_rcore: High-performance Rust backend for Killer language
// Killer Advanced v4.0 - Benchmarking & Optimization Infrastructure
// Week 3-5: JIT compilation, benchmarking, and loop optimizations

//! Unsafe is denied crate-wide; modules that need `unsafe` (JIT, FFI, VM bridges) use `#![allow(unsafe_code)]`.

#![deny(unsafe_code)]
// VmError carries rich diagnostics; boxing every `Err` would touch most call sites for little benefit.
#![allow(clippy::result_large_err)]
// Style / API-shape lints that are intentionally relaxed in this large VM + stdlib codebase.
#![allow(clippy::empty_line_after_doc_comments)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::new_without_default)]
// Pedantic / style lints: low signal for this codebase; tighten per-module when refactoring.
#![allow(clippy::manual_clamp)]
#![allow(clippy::useless_format)]
#![allow(clippy::no_effect_replace)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::unwrap_or_default)]
#![allow(clippy::should_implement_trait)]
#![allow(clippy::write_literal)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::match_like_matches_macro)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::needless_lifetimes)]
#![allow(clippy::unnecessary_cast)]
#![allow(clippy::inherent_to_string)]
#![allow(clippy::manual_flatten)]
#![allow(clippy::manual_find)]
#![allow(clippy::manual_div_ceil)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::unnecessary_map_or)]
#![allow(clippy::nonminimal_bool)]
#![allow(clippy::redundant_field_names)]
#![allow(clippy::manual_pattern_char_comparison)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::collapsible_str_replace)]
#![allow(clippy::format_in_format_args)]
#![allow(clippy::manual_contains)]
#![allow(clippy::manual_is_multiple_of)]
#![allow(clippy::unnecessary_min_or_max)]
#![allow(clippy::needless_return)]
#![allow(clippy::manual_next_back)]
#![allow(clippy::lines_filter_map_ok)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::ptr_arg)]

// v4.3 Refactoring: Source location tracking and safe mutex operations
pub mod source_location;
pub mod safe_mutex;

// Module declarations (Week 3-5 focus)
pub mod jit;
pub mod benchmark;
pub mod optimization;

// Error handling system with rich error types and source locations (v4.3)
pub mod error_handling;
pub mod error;

// SECURITY: Input validation, path safety, recursion limits (March 2026)
pub mod security;

// MODULE SYSTEM: Import/export, module registry, module loader
pub mod module_loader;

// HTTP CLIENT: REST API integration
pub mod http_client;

// LSP SERVER: IDE integration (autocomplete, diagnostics, etc)
pub mod lsp_server;

// CORE VM MODULES: Required by optimization_engine and vm (v4.3)
pub mod ast;
pub mod stmt_parser;
pub mod bytecode;
pub mod value;
pub mod nanbox;         // NaN-boxing: 8-byte inline scalars, zero heap alloc for f64/bool/trit
pub mod jit_x86;       // Native x86-64 JIT: hot loop detection + machine code generation
pub mod data_quality;
pub mod instruction_cache;
pub mod jit_compiler;
pub mod native_codegen;
pub mod variable_caching;
pub mod runtime_optimization;
pub mod call_site_cache;
pub mod allocation_pool;
pub mod loop_pattern_detection;

// ARCHITECTURE: Consolidated optimization engine (March 2026 refactoring)
pub mod optimization_engine;

// MONITORING & OBSERVABILITY: SaaS production features (March-April 2026)
pub mod telemetry;           // Metrics collection (requests, latency, resources)
pub mod circuit_breaker;     // Circuit breaker for error recovery
pub mod logging;             // Structured logging with correlation IDs
pub mod retry;               // Retry policies with exponential backoff
pub mod health_check;        // Health check system (liveness/readiness/startup probes)
pub mod audit_logging;       // Compliance audit trails (financial/security)
pub mod encryption;          // Cryptography (AES-256-GCM, password hashing, key management)

// Killer Standard Library - File I/O, Networking, JSON, HTTP
pub mod stdlib;

// Concurrency primitives - channels, mutexes, locks
pub mod concurrency;

// Nova Galaxy Engine v1 — Polyglot @lang{} runtime bridge
pub mod polyglot;

// Nova Galaxy Engine v1 — AI Assassin Assist Layer (logging, auto-debug, token budget, cost control)
pub mod assassin_assist;

// KhLM-Polyglot — 5-tier AI intelligence router for polyglot code ops (CAG→LLM→RLM→Ghost-108)
pub mod khlm_polyglot;

// Vision Engine — image_load / image_describe / khlm_vision (PNG/JPEG/BMP/GIF/WebP + LLM vision)
pub mod vision;

// Kala UI — world-class chat UI served at http://127.0.0.1:PORT/ via pure TCP
pub mod kala_ui;

// Killer UI engine — native panels / operator-style UI (TouchDesigner-adjacent *goals*; phased). See SOURCE/docs/KILLER_UI_ENGINE.md
pub mod killer_ui;

// Kala Generator — image generation (DALL-E 3 / Stability AI) + video generation (Runway Gen-3)
pub mod image_gen;
// Nova Native Image Generator — pure Rust procedural image synthesis (zero network, zero crates)
pub mod nova_gen;
// Nova Audio Engine — pure Rust WAV synthesis: ambient · nature · space · beat · ocean
pub mod nova_audio;
// Nova Video Engine — pure Rust animated GIF generator (GIF89a + LZW encoder)
pub mod nova_video;

// ── Android Native Modules — Pure Killer call recording engine ───────────────
// Microphone recording via AAudio NDK (Android) / simulated (Desktop)
pub mod android_audio;
// Phone state monitoring — call detection, VoIP apps, auto-record
pub mod android_phone;
// Foreground service, permissions, notifications, device info
pub mod android_service;
// Encryption, PIN lock, evidence hashing, secure wipe, integrity check
pub mod android_security;

// ══════════════════════════════════════════════════════════════════════════════
// Production Module — Regex engine, Help/Docs, File DB, Formatter, Linter
// Score booster: takes Killer from 3/10 → 7/10 production readiness
// ══════════════════════════════════════════════════════════════════════════════
pub mod production;

// ══════════════════════════════════════════════════════════════════════════════
// 10x Module — Package Manager, LSP Server, DAP Debugger, Docs Site Generator
// Score booster: takes Killer from 7/10 → 10/10 production readiness
// ══════════════════════════════════════════════════════════════════════════════
pub mod killer_10x;

// ══════════════════════════════════════════════════════════════════════════════
// Improve Module — Enhanced Errors, Import System, Watch Mode, Stack Traces,
// REPL Completions, Perf Baseline, Doc Comments Parser
// Final push: 8/10 → 9.5/10
// ══════════════════════════════════════════════════════════════════════════════
pub mod killer_improve;

// AI/ML Engine — native Rust primitives & demos (math, classical ML, NN building blocks, NLP, tabular RL, agents, etc.) — not a full PyTorch-scale stack; AGI/ASI are not here
pub mod ml_module;

// Nova Galaxy Engine v1 — HTTP Client (http_get, http_post, http_post_json, http_head, http_status, http_download)
pub mod http_client;

// Nova Galaxy Engine v1 — Vector Memory (TF-IDF embeddings, cosine similarity, KhLM auto-recall)
pub mod vector_memory;

// Affect Engine — 6-dimensional emotional state that colors all AI responses
pub mod affect;

// Imagination Engine — counterfactual reasoning, conceptual bridges, extrapolation
pub mod imagination;

// Guardian Engine — Human Protection Principle — Killer's first and final law
// Created by Sai Arun Kumar Katherashala
pub mod guardian;

// Generics system - type parameters and constraints
pub mod generics;

// Advanced features: JIT, Package Manager, Memory Profiling, FFI (v4.0+), GC, REPL, Debugger
pub mod advanced_features;

// Foreign Function Interface - Call C libraries from Killer (PLANNED v4.0)
// pub mod ffi;

// Dynamic FFI - Runtime library loading with callbacks (PLANNED v4.0)
// pub mod ffi_dynamic;

// Standard Library Builder - 220+ functions (Math, String, Collections, I/O, Time, Type, Concurrency)
pub mod stdlib_builder;

// Standard Library Implementation - Phase 21-22 Solvers
// 600+ functions across all scientific, technical, and infrastructure domains (PLANNED v4.0 with FFI)
/*
pub mod stdlib_impl {
    /// Math library: 71+ functions (trig, exponential, stats, special, RNG)
    pub mod math_impl;
    
    /// Linear Algebra: 25+ functions (matrix ops, decomposition, eigenvalues)
    pub mod linear_algebra;
    
    /// Statistics: 50+ functions (descriptive, distributions, hypothesis testing, correlation)
    pub mod statistics_solver;
    
    /// Game Theory: 35+ functions (Nash equilibrium, cooperative games, auctions, voting)
    pub mod game_theory;
    
    /// Cryptography: 50+ functions (RSA, ECC, hash, DH key exchange, signatures)
    pub mod cryptography_solver;
    
    /// Network Science: 40+ functions (centrality, clustering, algorithms, community detection)
    pub mod network_science;
    
    /// Signal Processing: 45+ functions (FFT, filtering, windowing, spectral analysis, features)
    pub mod signal_processing;
    
    /// Medical & Biomedical: 43+ functions (pharmacokinetics, epidemiology, diagnostics, clinical metrics)
    pub mod medical_biomedical;
    
    /// Millennium Prize Problems: 20+ functions (P vs NP, Riemann hypothesis, Navier-Stokes, Yang-Mills)
    pub mod millennium_prize;
    
    /// File I/O & Streams: 42+ functions (file ops, streams, buffering, serialization, binary I/O)
    pub mod io_solver;
    
    /// Time & Scheduling: 40+ functions (current time, calculations, scheduling, timers, formatting)
    pub mod time_solver;
    
    /// Type System: 40+ functions (type introspection, reflection, classification, constraints)
    pub mod type_solver;
    
    /// Concurrency: 40+ functions (atomic ops, synchronization, thread-safe primitives, memory barriers)
    pub mod concurrency_solver;
    
    /// MongoDB Database: 42+ functions (connection pool, CRUD, aggregation, indexing, transactions)
    pub mod database_mongodb;
    
    /// PostgreSQL Database: 45+ functions (connection pool, queries, prepared statements, transactions, DDL, indexing)
    pub mod database_postgresql;
    
    /// Query Builder & ORM: 40+ functions (generic query DSL, filter builder, pagination, joins, result mapping)
    pub mod database_query;
    
    /// HTTP Server: 50+ functions (server lifecycle, routing, connections, static files, keep-alive)
    pub mod http_server;
    
    /// Request/Response HTTP Protocol: 55+ functions (parsing, headers, cookies, encoding/decoding, content types)
    pub mod request_response;
    
    /// Middleware: 50+ functions (CORS, logging, compression, security, rate limiting, request/response filtering)
    pub mod middleware;
    
    /// Template Engine: 55+ functions (parsing, variable interpolation, filters, loops, rendering, caching)
    pub mod template_engine;
    
    /// Session Management: 50+ functions (session lifecycle, storage, serialization, TTL, config)
    pub mod session;
    
    /// Authentication & Authorization: 50+ functions (basic auth, bearer tokens, JWT, permissions, roles)
    pub mod auth;
    
    /// WebSocket: 50+ functions (handshake, frame parsing, messaging, connection management, extensions)
    pub mod websocket;
    
    /// GraphQL: 50+ functions (schema definition, query parsing, execution, types, response formatting)
    pub mod graphql;
    
    /// File Upload: 45+ functions (multipart parsing, file handling, form processing, progress tracking, security)
    pub mod file_upload;
    
    /// Streaming: 45+ functions (response streaming, stream processing, buffering, composition, error handling)
    pub mod streaming;
    
    /// Server-Sent Events: 50+ functions (connection management, event publishing, formats, client management, channels)
    pub mod sse;
    
    /// OAuth 2.0 & OpenID Connect: 50+ functions (auth flows, token management, PKCE, identity verification)
    pub mod oauth2;
    
    /// Role-Based Access Control: 50+ functions (roles, permissions, hierarchy, audit)
    pub mod rbac;
    
    /// Attribute-Based Access Control: 50+ functions (policies, conditions, attributes, decision making)
    pub mod abac;
    
    /// Distributed Session Management: 50+ functions (session storage, multi-device, synchronization)
    pub mod sessions;
    
    /// Token Introspection & Revocation: 40+ functions (validation, revocation tracking, JTI management)
    pub mod token_introspection;
    
    /// Service Discovery: 50+ functions (registry, DNS, health checks, service watch)
    pub mod service_discovery;
    
    /// Load Balancing: 50+ functions (round robin, least connections, weighted, consistent hashing, health-aware)
    pub mod load_balancer;
    
    /// Circuit Breaker: 50+ functions (state machine, failure detection, recovery, multi-circuit management)
    pub mod circuit_breaker;
    
    /// Message Queues: 50+ functions (pub/sub, consumer groups, dead letter queues, partitioning)
    pub mod message_queue;
    
    /// Distributed Tracing: 50+ functions (spans, instrumentation, context propagation, sampling)
    pub mod distributed_tracing;
    
    /// PHASE 28: Distributed Consensus - 5 modules, 250+ functions
    
    /// Raft Consensus: 46+ functions (leader election, log replication, safety)
    pub mod raft;
    
    /// Paxos for Byzantine Resilience: 50+ functions (proposer, acceptor, learner, Byzantine handling)
    pub mod paxos;
    
    /// Hybrid Logical Clocks: 50+ functions (HLC management, causality tracking, gap handling, timestamp ordering)
    pub mod hlc;
    
    /// Distributed Locks: 50+ functions (basic locking, expiration, RW locks, lock manager, deadlock detection)
    pub mod locks;
    
    /// Consensus State Machines: 50+ functions (state management, command logs, snapshots, queries)
    pub mod state_machines;
}
*/  // END stdlib_impl - PLANNED v4.0

// Async/Await - Non-blocking I/O and async functions
pub mod async_await;

// Type system for static type checking and inference
pub mod type_system;

// Monitoring and metrics framework
pub mod monitoring;

// Killer Super: Advanced 16-stage production compiler
pub mod killer_super;

// Temporal computing: event log, causality, time-series, reversible execution
pub mod time_machine;

// -- LLM Integration (native, zero external deps) -----------------------------
pub mod llm;                   // Ollama (TCP) + OpenAI/Anthropic/Groq (curl) — complete/embed/ask

// -- Native Inference Engine — Killer runs its own LLMs, no Ollama, no cloud --
pub mod inference;             // GGUF loader + BPE tokenizer + transformer forward pass

// -- Native AI subsystem -------------------------------------------------------
pub mod ai;                    // Core AI runtime: providers, cache, batching, config
pub mod ai_annotations;        // @ai_assist / @ai_schedule / @ai_validate annotations
pub mod ai_analyzer;           // Pattern detector + hint generator (Phase 2)
pub mod ai_code_analyzer;      // AST-level code analyzer with optimization hints
pub mod ai_optimizer;          // ML-driven JIT threshold tuning
pub mod ai_workflow_engine;    // Secure workflow orchestration (Phase 3)

// LLM client façade, multi-agent framework, super-agent workflows (see tests/ai_integration_tests.rs)
pub mod llm_client;
pub mod agent_framework;
pub mod super_agent_layer;

// -- Compression primitives (RLE, base64, hex) — wired as builtins -------------
pub mod compression_module;

// JSON (used by `web_framework` request helpers)
pub mod json_module;

// SuperProcessor stack + HTTP helpers (std only) — integration tests in `tests/superprocessor_*`, `tests/http_server_tests.rs`
pub mod web_framework;
pub mod stream_processing;
pub mod batch_processing;
pub mod data_sharding;
pub mod lazy_evaluation;
pub mod spill_to_disk;
pub mod distributed_queues;
pub mod super_processor;
pub mod http_server;

// -- Debug Intelligence — "Developer Can Relax" system ------------------------
// debug_check / auto_fix / explain_error / suggest_refactor / auto_test /
// perf_profile / ai_pair / killer_debug_agent / watch / watch_report
pub mod debug_intelligence;

// -- Core interpreter pipeline ------------------------------------------------
pub mod lexer;           // Tokenizer: source text → Vec<Token>
#[path = "optimizer.rs"]
pub mod optimizer;       // Bytecode optimizer (dead-code, CSE)
pub mod exception;       // Exception manager for try/catch/throw
pub mod generator;       // Generator/yield manager
pub mod builtin;         // Built-in functions (len, push, etc.)
pub mod builtin_dispatch; // Forward `BuiltinFunctions::call` for modules that must not cycle with `builtin`
pub mod tool_calling;    // KhLM tool registry + dispatch (uses `builtin_dispatch`, not `builtin` directly)
pub mod objects;         // Object / class instance support
pub mod operations;      // Arithmetic & comparison helpers
pub mod stack;           // Value stack helpers
pub mod simd_ops;        // SIMD-accelerated operations
pub mod net;             // Networking stdlib
pub mod datetime;        // Date/time operations
pub mod http;            // HTTP client utils
pub mod json_csv;        // JSON and CSV parsing
pub mod websocket;       // WebSocket support
pub mod compiler;        // Killer → bytecode compiler
pub mod vm;              // Bytecode virtual machine
pub mod ghost_vm;        // Ghost VM — cold-resumable capsule (GHST) + fuel-bounded interpreter
pub mod ghost_lang;      // GhostLang — high-level language that compiles to Ghost assembly
pub mod ghost_hive;      // Ghost Hive — 1M-agent evolution engine on Ghost VM
pub mod ghost_world;     // Ghost WorldHost — file I/O, HTTP, time, pages, data processing
pub mod version;         // Version constants

// Supernova Lightning Engine — native built-in runtime (no external .killer file needed)
pub mod supernova;

// -- KORE — Killer Optimized Record Exchange -----------------------------------
// Binary columnar format: PAX layout + bloom filters + delta/dict/LZ77 + per-column encryption
// Beats Parquet+Snappy in read speed; beats CSV in size; unique AI metadata block
pub mod kore;
// -- KORE v2 — World-Class Columnar Format (beats Parquet on every dimension) ----
// Per-column Huffman(LZ77), 9 codecs, CRC32, bloom filters, predicate pushdown
pub mod kore_v2;
// -- KORE Query Engine — SQL-like queries on KORE files (SELECT/WHERE/GROUP BY) --
pub mod kore_query;
// -- KORE Transactions — ACID writes + Time Travel versioning --------------------
pub mod kore_txn;
// -- Nova Compression — KORE columnar encoding + LZ77 (no external deps) ---------
// nova_write / nova_info / nova_read_col builtins
pub mod nova;
/// Trit packing (NOVT) — orthogonal to [`nova`] NOVZ file compression; see module docs.
pub mod nova_trit_codec;
pub mod test_runner;     // `killer-native --test` runner
pub mod formatter;       // `killer-native --format` code formatter
pub mod linter;          // Source linter (INFO/WARN/ERROR)

// -- World-Class Stdlib Modules ------------------------------------------------
pub mod killer_collections;    // Set, OrderedMap, PriorityQueue, Deque, LinkedList, Iterator
pub mod killer_async;          // Executor pool, TaskHandle, select/join, AsyncChannel
pub mod killer_testing;        // Mock, Fixture, PropTest, SnapshotStore, ParamTest
pub mod killer_cli;            // CLI arg parser: flags, options, subcommands, help gen
pub mod killer_subprocess;     // ProcessBuilder, Pipeline, shell, capture, which
pub mod killer_serialization;  // TOML parser/writer + MessagePack encoder/decoder
// 
// Re-export main components at crate level
pub use jit::{JITCache, JITLoader, RustCompiler, LoopSignature};
pub use benchmark::{BenchmarkHarness, BenchmarkRunner, BenchmarkMetrics, PerformanceReport, LoopType};
pub use optimization::{LoopAnalyzer, LoopUnroller, OptimizedCodeGenerator, OptimizationConfig};
pub use error_handling::{KillerError, ErrorKind, ErrorSeverity, Result, ErrorRecovery, suggest_similar};
// pub use parser::{Parser, Lexer, AstNode, AstNodeType, Token, TokenType};  // Use killer_super parser
pub use type_system::{TypeKind, TypeAnnotation, FunctionSignature, TypeEnvironment, TypeChecker};
pub use monitoring::{Monitor, Measurement, PhaseMetrics, ResourceMetrics, HealthMonitor, HealthStatus};
// pub use ffi::{CType, CValue, CFunction, FFIError, FFIBindings, load_library, call_c_function};  // PLANNED v4.0

// Re-export Killer Super components
pub use killer_super::{
    KillerSuper, CompilationResult, CompilerStats, KillerSuperConfig, OptimizationLevel, CompilerMode, TargetArch,
    LLVMModule, LLVMFunction, LLVMInstruction,
    GpuDevice, GpuKernel, GpuExecutionPlan, GpuDeviceType,
    CompilationProfile, StageProfiler, CacheAnalyzer, MemoryAccessAnalyzer
};

// Re-export core interpreter types for binary and external consumers
pub use bytecode::Program;
pub use vm::VirtualMachine;
pub use error::VmError;
pub use compiler::{compile_killer_ast, compile_killer_default, compile_killer_subset};
pub use stmt_parser::{parse_killer_program, run_killer_parsed};

/// KhLM / RLM helpers for embedders and tools (model path heuristics, synthesis context budget).
pub use llm::{is_rlm_model_path, khlm_synth_context_char_limit};

/// Run Killer **source text** through the default line-oriented pipeline → VM.
///
/// For the **token parser → full AST compiler** path (classes, richer `Stmt` coverage when parsed),
/// use [`run_killer_parsed`]. See `SOURCE/docs/LANGUAGE_PIPELINE.md`.
pub fn run_killer_source(source: &str) -> std::result::Result<(), error::VmError> {
    let program = compiler::compile_killer_default(source)?;
    let mut machine = vm::VirtualMachine::new();
    machine.run(&program)
}

/// Run a pre-built [`ast::Stmt`] program through the **AST** compiler → VM (classes, methods, full `Stmt` surface).
///
/// Use when constructing the AST in Rust or when a future parser feeds [`ast::Stmt`].
pub fn run_killer_ast(statements: &[ast::Stmt]) -> std::result::Result<(), error::VmError> {
    let program = compiler::compile_killer_ast(statements)?;
    let mut machine = vm::VirtualMachine::new();
    machine.run(&program)
}

// Version string
pub const VERSION: &str = "4.0.0-week5";

/// Main entry point for JIT compilation pipeline
/// 
/// # Example
/// ```ignore
/// use killer_rcore::{LoopDetector, RustCodegen, RustCompiler, JITLoader, JITCache};
/// use std::path::Path;
/// 
/// // Step 1: Detect hot loops in AST
/// let detector = LoopDetector::new();
/// let hot_loops = detector.detect_hot_loops(&ast);
/// 
/// // Step 2: Generate Rust code
/// for loop_profile in hot_loops {
///     let generator = RustCodegen::new();
///     let rust_code = generator.generate(&loop_profile);
///     
///     // Step 3: Compile to native binary
///     let compiler = RustCompiler::new();
///     let result = compiler.compile(&rust_code, "killer_jit_loop");
///     
///     if result.success {
///         // Step 4: Load and execute
///         let loader = JITLoader::new();
///         if let Some(binary) = result.binary_path {
///             let _result = loader.execute_loop_function(&binary, "killer_jit_loop_0");
///         }
///     }
/// }
/// ```
pub fn compile_pipeline_version() -> &'static str {
    "Week 3-5: JIT compilation + Benchmarking + Loop optimization (unrolling, vectorization)"
}

#[cfg(test)]
mod lib_tests {
    use super::*;
    
    #[test]
    fn test_version_string() {
        assert_eq!(VERSION, "4.0.0-week5");
    }
    
    #[test]
    fn test_pipeline_description() {
        let desc = compile_pipeline_version();
        assert!(desc.contains("Week 3"));
        assert!(desc.contains("optimization") || desc.contains("week 5"));
    }
}
