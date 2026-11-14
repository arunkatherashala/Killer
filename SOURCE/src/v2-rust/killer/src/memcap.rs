//! Per-evaluation memory cap used by `eval` / `exec`.
//!
//! A thin wrapper around the system allocator ([`CountingAlloc`], an opt-in type: the library does NOT install it
//! unless the `mem-cap` Cargo feature is on; the Killer CLI binaries declare it themselves) keeps a thread-local count of live bytes, but only while a cap is active on the thread (see [`enter`]); when no
//! cap is active the cost is one thread-local read per allocation. The VM compares the count with the limit every
//! 256 instructions, and the known big allocation sites (string repeat and concatenation, list repeat and
//! concatenation, `range`) call [`reserve`] first, so one huge request fails before the memory is requested.
//!
//! Precision: the count is the sum of requested allocation sizes made on this thread while the cap is active (no
//! allocator overhead, no fragmentation), minus what was freed. It does not cover memory allocated by other threads
//! the evaluated code starts, nor memory the operating system maps for the process in other ways (files, stack).
//! Between two periodic checks the evaluation can overshoot the limit a little, and a growing `Vec` briefly holds
//! both its old and new buffer, so treat the cap as a practical bound (plus up to a factor of about two), not as an
//! exact limit.
//!
//! Without the counting allocator (an embedder that uses the library with its own global allocator and without the
//! `mem-cap` feature) the live-byte counter never moves: [`is_installed`] returns false, the periodic [`check`] can
//! never fire (no false "limit exceeded"), and only the explicit [`reserve`] pre-checks, the step and time budgets
//! and the capability inheritance protect the caller.

use crate::error::VmError;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicU8, Ordering};

/// Default cap of one evaluation, in megabytes.
pub const DEFAULT_MAX_MB: u64 = 256;
/// Largest cap that can be configured, in megabytes (larger requests are rejected).
pub const HARD_MAX_MB: u64 = 2048;
/// Smallest cap that can be configured, in megabytes.
pub const MIN_MB: u64 = 1;

thread_local! {
    /// Absolute limit in the `LIVE` counter's space; 0 means no cap is active on this thread.
    static LIMIT: Cell<i64> = const { Cell::new(0) };
    /// Net bytes allocated on this thread since the outermost cap was entered.
    static LIVE: Cell<i64> = const { Cell::new(0) };
    /// Megabytes reported in the error message of the innermost active cap.
    static MB: Cell<u64> = const { Cell::new(0) };
}

/// The system allocator plus the counting described in the module documentation.
pub struct CountingAlloc;

#[inline(always)]
fn add(delta: i64) {
    // `try_with` so an allocation during thread teardown can never panic (const, drop-free cells never fail anyway)
    let _ = LIMIT.try_with(|l| {
        if l.get() != 0 {
            let _ = LIVE.try_with(|v| v.set(v.get() + delta));
        }
    });
}

// Implementing `GlobalAlloc` is unsafe by definition; every method only forwards to `System`. This is the only
// place of this module that needs `unsafe`, so the allowance is scoped to the impl.
#[allow(unsafe_code)]
unsafe impl GlobalAlloc for CountingAlloc {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = System.alloc(layout);
        if !p.is_null() {
            add(layout.size() as i64);
        }
        p
    }
    #[inline]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let p = System.alloc_zeroed(layout);
        if !p.is_null() {
            add(layout.size() as i64);
        }
        p
    }
    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        add(-(layout.size() as i64));
    }
    #[inline]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let p = System.realloc(ptr, layout, new_size);
        if !p.is_null() {
            add(new_size as i64 - layout.size() as i64);
        }
        p
    }
}

/// Keeps a cap active; dropping it restores the enclosing cap (or none).
pub struct CapGuard {
    prev_limit: i64,
    prev_mb: u64,
}

impl Drop for CapGuard {
    fn drop(&mut self) {
        LIMIT.with(|l| l.set(self.prev_limit));
        MB.with(|m| m.set(self.prev_mb));
    }
}

/// Start counting with a cap of `mb` megabytes (clamped to [`MIN_MB`], [`HARD_MAX_MB`]). Inside an active cap the
/// new limit can never exceed the enclosing one, so nested evaluations share an overall bound.
pub fn enter(mb: u64) -> CapGuard {
    let mb = mb.clamp(MIN_MB, HARD_MAX_MB);
    let prev_limit = LIMIT.with(|l| l.get());
    let prev_mb = MB.with(|m| m.get());
    if prev_limit == 0 {
        LIVE.with(|v| v.set(0));
    }
    let live = LIVE.with(|v| v.get());
    let mine = live.saturating_add((mb as i64) * 1024 * 1024);
    let (limit, shown) = if prev_limit != 0 && prev_limit < mine {
        (prev_limit, prev_mb)
    } else {
        (mine, mb)
    };
    MB.with(|m| m.set(shown));
    LIMIT.with(|l| l.set(limit));
    CapGuard { prev_limit, prev_mb }
}

#[cold]
fn exceeded_error() -> VmError {
    let mb = MB.with(|m| m.get());
    VmError::SecurityError {
        message: format!("memory limit exceeded ({} MB)", mb),
        location: None,
        suggestion: Some("Allocate less, or raise the cap (eval opts max_memory_mb, KILLER_EVAL_MAX_MEMORY).".into()),
    }
}

/// Error when the live byte count is above the active limit (no-op without an active cap).
#[inline]
pub fn check() -> Result<(), VmError> {
    let limit = LIMIT.with(|l| l.get());
    if limit != 0 && LIVE.with(|v| v.get()) > limit {
        return Err(exceeded_error());
    }
    Ok(())
}

/// Error when allocating `bytes` more would go over the active limit (no-op without an active cap).
#[inline]
pub fn reserve(bytes: usize) -> Result<(), VmError> {
    let limit = LIMIT.with(|l| l.get());
    if limit != 0 && LIVE.with(|v| v.get()).saturating_add(bytes.min(i64::MAX as usize) as i64) > limit {
        return Err(exceeded_error());
    }
    Ok(())
}

/// Live bytes counted since the outermost cap was entered (for tests and diagnostics).
pub fn live_bytes() -> i64 {
    LIVE.with(|v| v.get())
}

/// The cap in megabytes: an explicit `requested` value, else `KILLER_EVAL_MAX_MEMORY`, else the default. Values
/// outside `MIN_MB..=HARD_MAX_MB` are an error.
pub fn resolve_mb(requested: Option<f64>) -> Result<u64, String> {
    let raw = match requested {
        Some(n) => n,
        None => match std::env::var("KILLER_EVAL_MAX_MEMORY") {
            Ok(s) if !s.trim().is_empty() => s.trim().parse::<f64>().map_err(|_| format!("KILLER_EVAL_MAX_MEMORY must be a number of megabytes, got \"{}\"", s.trim()))?,
            _ => return Ok(DEFAULT_MAX_MB),
        },
    };
    if !raw.is_finite() || raw.fract() != 0.0 || raw < MIN_MB as f64 || raw > HARD_MAX_MB as f64 {
        return Err(format!("the memory cap must be a whole number of megabytes from {} to {}, got {}", MIN_MB, HARD_MAX_MB, raw));
    }
    Ok(raw as u64)
}

/// 0 = not probed yet, 1 = counting allocator is installed, 2 = it is not.
static INSTALLED: AtomicU8 = AtomicU8::new(0);

/// True when [`CountingAlloc`] is the process's global allocator, so the periodic live-byte check works. When it
/// is false only the [`reserve`] pre-checks apply. Detected once by making a small allocation inside a temporary
/// counted scope on the calling thread and seeing whether the counter moved (the thread's own cap state is
/// restored afterwards).
pub fn is_installed() -> bool {
    match INSTALLED.load(Ordering::Relaxed) {
        1 => return true,
        2 => return false,
        _ => {}
    }
    let (pl, pv) = (LIMIT.with(|l| l.get()), LIVE.with(|v| v.get()));
    LIMIT.with(|l| l.set(i64::MAX));
    LIVE.with(|v| v.set(0));
    let b = std::hint::black_box(Box::new([0u8; 64]));
    let moved = LIVE.with(|v| v.get()) >= 64;
    drop(b);
    LIMIT.with(|l| l.set(pl));
    LIVE.with(|v| v.set(pv));
    INSTALLED.store(if moved { 1 } else { 2 }, Ordering::Relaxed);
    moved
}
