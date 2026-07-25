//! Ghost VM — cold-resumable capsule: bytecode + linear RAM + stack, bounded fuel per host touch.
//!
//! Capsules serialize to a small binary blob for disk / NFC-sized payloads later.
//!
//! **GHST v2** adds a `capabilities` syscall bitmask after `fuel_per_touch`. v1 capsules decode with
//! `capabilities = !0` (all syscalls allowed except enforcement uses the same mask). [`SYS_NOP`] is always allowed.
//!
//! API: [`Capsule`] encode/decode, [`run`], [`assemble_capsule`], [`disassemble_code`], [`GhostHost`], [`sign_capsule`], [`verify_capsule`].
//!
//! Signed capsules append `GSIG` + length + HMAC-SHA256 (32 bytes) over the core payload.

#[path = "ghost_vm_sha.rs"]
mod ghost_vm_sha;

use std::io::Write as IoWrite;

/// Max signature length accepted on decode (HMAC-SHA256 = 32).
pub const MAX_SIGNATURE_LEN: usize = 512;
const SIG_MAGIC: &[u8; 4] = b"GSIG";

/// Max nested `.include` depth.
pub const MAX_INCLUDE_DEPTH: usize = 16;

/// File / blob magic (`GHST`).
pub const MAGIC: &[u8; 4] = b"GHST";
/// GHST envelope v1 (no capability dword).
pub const FORMAT_VERSION_V1: u16 = 1;
/// GHST envelope v2 (`capabilities` u32 after `fuel_per_touch`).
pub const FORMAT_VERSION_V2: u16 = 2;
/// GHST envelope v3 (i64 stack, 64KB RAM, float opcodes).
pub const FORMAT_VERSION_V3: u16 = 3;
/// Default format for new capsules.
pub const FORMAT_VERSION_LATEST: u16 = FORMAT_VERSION_V3;
/// All syscall bits set (including extended / id ≥ 32 via bit 31).
pub const CAPABILITIES_ALL: u32 = !0;
/// Bit for syscall ids ≥ 32 (host-defined extended syscalls).
pub const CAPABILITY_SYSCALL_EXTENDED: u32 = 1 << 31;
/// VM semantics revision; bump when opcode meanings change.
pub const VM_REVISION: u32 = 1;
pub const MAX_RAM: usize = 65_536;
pub const MAX_STACK: usize = 4096;
pub const MAX_CODE: usize = 65_536;

// --- Opcodes (fixed interpretation for VM_REVISION 1) ---

pub const OP_NOP: u8 = 0x00;
pub const OP_PUSH: u8 = 0x01;
pub const OP_POP: u8 = 0x02;
pub const OP_DUP: u8 = 0x03;
pub const OP_SWAP: u8 = 0x04;
pub const OP_ROT: u8 = 0x05;
pub const OP_ADD: u8 = 0x10;
pub const OP_SUB: u8 = 0x11;
pub const OP_MUL: u8 = 0x12;
pub const OP_DIV: u8 = 0x13;
pub const OP_MOD: u8 = 0x14;
pub const OP_EQ: u8 = 0x15;
pub const OP_LT: u8 = 0x16;
pub const OP_GT: u8 = 0x17;
pub const OP_LOAD: u8 = 0x20;
pub const OP_STORE: u8 = 0x21;
pub const OP_JMP: u8 = 0x30;
pub const OP_JMPIF: u8 = 0x31;
pub const OP_FCONST: u8 = 0x18;
pub const OP_FADD: u8 = 0x19;
pub const OP_FSUB: u8 = 0x1A;
pub const OP_FMUL: u8 = 0x1B;
pub const OP_FDIV: u8 = 0x1C;
pub const OP_ITOF: u8 = 0x1D;
pub const OP_FTOI: u8 = 0x1E;
pub const OP_SYSCALL: u8 = 0x40;
pub const OP_HALT: u8 = 0xFF;

// Syscall IDs (host-defined above this range reserved)
pub const SYS_NOP: u8 = 0;
pub const SYS_HOST_LOG: u8 = 1;
pub const SYS_CHECKPOINT: u8 = 2;
pub const SYS_PRINT_NUM: u8 = 3;
pub const SYS_PRINT_STR: u8 = 4;
pub const SYS_READ_LINE: u8 = 5;
pub const SYS_PRINT_CHAR: u8 = 6;
pub const SYS_PROMPT: u8 = 7;
pub const SYS_YIELD: u8 = 8;
pub const SYS_TICK: u8 = 9;
pub const SYS_STACK_DEPTH: u8 = 10;
pub const SYS_CODE_LEN: u8 = 11;
pub const SYS_CODE_READ: u8 = 12;
pub const SYS_CODE_WRITE: u8 = 13;
pub const SYS_RAM_LEN: u8 = 14;
pub const SYS_PC: u8 = 15;
pub const SYS_SPAWN_INIT: u8 = 16;
pub const SYS_SPAWN_RUN: u8 = 17;
pub const SYS_SPAWN_KILL: u8 = 18;
pub const SYS_SPAWN_READ: u8 = 19;
pub const SYS_SPAWN_WRITE: u8 = 20;
pub const SYS_SPAWN_COUNT: u8 = 21;
pub const SYS_RANDOM: u8 = 22;
pub const SYS_COPY_CODE: u8 = 23;
pub const SYS_HASH: u8 = 24;

// File I/O syscalls (WorldHost)
pub const SYS_FILE_READ: u8 = 30;
pub const SYS_FILE_WRITE: u8 = 31;
pub const SYS_FILE_APPEND: u8 = 32;
pub const SYS_FILE_EXISTS: u8 = 33;
pub const SYS_FILE_SIZE: u8 = 34;
pub const SYS_FILE_LIST: u8 = 35;

// HTTP syscalls (WorldHost, http:// only, std::net::TcpStream)
pub const SYS_HTTP_GET: u8 = 40;
pub const SYS_HTTP_POST: u8 = 41;
pub const SYS_DNS_RESOLVE: u8 = 42;

// Time & System syscalls (WorldHost)
pub const SYS_TIME_NOW: u8 = 50;
pub const SYS_TIME_MS: u8 = 51;
pub const SYS_SLEEP_MS: u8 = 52;
pub const SYS_ENV_GET: u8 = 53;
pub const SYS_ARGV: u8 = 54;
pub const SYS_PLATFORM: u8 = 55;

// Data processing syscalls (WorldHost)
pub const SYS_PARSE_INT: u8 = 60;
pub const SYS_FORMAT_INT: u8 = 61;
pub const SYS_MEM_COPY: u8 = 62;
pub const SYS_MEM_FIND: u8 = 63;
pub const SYS_MEM_FILL: u8 = 64;
pub const SYS_HASH_RAM: u8 = 65;

// Extended RAM page syscalls (WorldHost)
pub const SYS_PAGE_ALLOC: u8 = 70;
pub const SYS_PAGE_FREE: u8 = 71;
pub const SYS_PAGE_READ: u8 = 72;
pub const SYS_PAGE_WRITE: u8 = 73;
pub const SYS_PAGE_COPY: u8 = 74;

// Knowledge / AI syscalls (WorldHost)
pub const SYS_ASK: u8 = 80;
pub const SYS_MATH_EVAL: u8 = 81;
pub const SYS_STR_LOWER: u8 = 82;
pub const SYS_STR_EQ: u8 = 83;

// v3.0 new syscalls
pub const SYS_FPRINT: u8 = 84;
pub const SYS_RAND_SEED: u8 = 85;
pub const SYS_RAND_RANGE: u8 = 86;
pub const SYS_HTTP_HEADER: u8 = 87;

/// Capability bit 13 — required for SYS_CODE_WRITE (self-modifying code).
pub const CAPABILITY_SELF_MODIFY: u32 = 1 << 13;
pub const MAX_CHILDREN: usize = 16;
pub const PAGE_SIZE: usize = 4096;
pub const MAX_PAGES: usize = 256;
pub const MAX_FILE_READ: usize = 65_536;  // 64KB per file/HTTP read into base RAM
const GPAG_MAGIC: &[u8; 4] = b"GPAG";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugMode { None, Trace, Step }

impl Default for DebugMode {
    fn default() -> Self { DebugMode::None }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GhostError {
    CapsuleCorrupt(&'static str),
    AssembleError(String),
    UndefinedLabel(String),
    SignatureMissing,
    SignatureInvalid,
    StackOverflow,
    StackUnderflow,
    PcOutOfBounds,
    RamOutOfBounds,
    UnknownOpcode(u8),
    FuelExhausted,
    SyscallDenied(u8),
    Interrupted,
}

impl std::fmt::Display for GhostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GhostError::AssembleError(s) => write!(f, "assemble: {s}"),
            GhostError::UndefinedLabel(s) => write!(f, "assemble: undefined label {s:?}"),
            GhostError::SignatureMissing => write!(f, "signature missing"),
            GhostError::SignatureInvalid => write!(f, "signature invalid"),
            GhostError::SyscallDenied(id) => write!(f, "syscall {id} not allowed by capsule capabilities"),
            GhostError::Interrupted => write!(f, "interrupted by user"),
            _ => write!(f, "{self:?}"),
        }
    }
}

impl std::error::Error for GhostError {}

#[derive(Debug, Clone)]
pub struct Capsule {
    /// GHST envelope: [`FORMAT_VERSION_V1`] or [`FORMAT_VERSION_V2`].
    pub format_version: u16,
    pub vm_revision: u32,
    pub pc: u32,
    pub stack: Vec<i64>,
    pub ram: Vec<u8>,
    pub code: Vec<u8>,
    /// Default fuel for `run` when not overridden.
    pub fuel_per_touch: u32,
    /// Syscall allow mask: bit `id` enables syscall `id` for `id < 32`; [`CAPABILITY_SYSCALL_EXTENDED`] enables id ≥ 32.
    /// [`SYS_NOP`] (0) is always allowed regardless of bits.
    pub capabilities: u32,
    /// Extended RAM pages (4KB each, up to 256). Serialized via GPAG section.
    pub pages: Vec<Option<Vec<u8>>>,
    /// Present when loaded from a signed blob (`GSIG` trailer); see [`sign_capsule`] / [`verify_capsule`].
    pub signature: Option<Vec<u8>>,
}

impl Default for Capsule {
    fn default() -> Self {
        Self::with_ram_and_fuel(MAX_RAM, 10_000)
    }
}

impl Capsule {
    /// Empty stack, `pc = 0`, code is single `HALT`.
    pub fn with_ram_and_fuel(ram_len: usize, fuel_per_touch: u32) -> Self {
        let ram_len = ram_len.min(MAX_RAM);
        Self {
            format_version: FORMAT_VERSION_LATEST,
            vm_revision: VM_REVISION,
            pc: 0,
            stack: Vec::new(),
            ram: vec![0u8; ram_len],
            code: vec![OP_HALT],
            fuel_per_touch,
            capabilities: CAPABILITIES_ALL,
            pages: Vec::new(),
            signature: None,
        }
    }

    /// Whether syscall `id` may invoke the host (always `true` for [`SYS_NOP`]).
    pub fn syscall_allowed(&self, id: u8) -> bool {
        if id == SYS_NOP {
            return true;
        }
        let u = id as u32;
        if u < 32 {
            (self.capabilities & (1u32 << u)) != 0
        } else {
            (self.capabilities & CAPABILITY_SYSCALL_EXTENDED) != 0
        }
    }

    /// Core GHST blob (no `GSIG` trailer). This is what HMAC signs.
    pub fn encode_payload(&self) -> Result<Vec<u8>, GhostError> {
        validate_sizes(self)?;
        let mut out = Vec::new();
        Self::write_core_bytes(self, &mut out);
        Ok(out)
    }

    fn write_core_bytes(c: &Capsule, out: &mut Vec<u8>) {
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&c.format_version.to_le_bytes());
        out.extend_from_slice(&c.vm_revision.to_le_bytes());
        out.extend_from_slice(&c.fuel_per_touch.to_le_bytes());
        if c.format_version >= FORMAT_VERSION_V2 {
            out.extend_from_slice(&c.capabilities.to_le_bytes());
        }
        out.extend_from_slice(&c.pc.to_le_bytes());
        let sl = c.stack.len() as u32;
        out.extend_from_slice(&sl.to_le_bytes());
        for v in &c.stack {
            if c.format_version >= FORMAT_VERSION_V3 {
                out.extend_from_slice(&v.to_le_bytes());
            } else {
                out.extend_from_slice(&(*v as i32).to_le_bytes());
            }
        }
        let rl = c.ram.len() as u32;
        out.extend_from_slice(&rl.to_le_bytes());
        out.extend_from_slice(&c.ram);
        let cl = c.code.len() as u32;
        out.extend_from_slice(&cl.to_le_bytes());
        out.extend_from_slice(&c.code);
        // GPAG section: only written when pages are allocated
        let alloc_count = c.pages.iter().filter(|p| p.is_some()).count();
        if alloc_count > 0 {
            out.extend_from_slice(GPAG_MAGIC);
            out.extend_from_slice(&(alloc_count as u16).to_le_bytes());
            for (idx, page) in c.pages.iter().enumerate() {
                if let Some(data) = page {
                    out.push(idx as u8);
                    out.extend_from_slice(data);
                }
            }
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, GhostError> {
        validate_sizes(self)?;
        if let Some(sig) = &self.signature {
            if sig.is_empty() || sig.len() > MAX_SIGNATURE_LEN {
                return Err(GhostError::CapsuleCorrupt("bad signature length"));
            }
        }
        let mut out = self.encode_payload()?;
        if let Some(sig) = &self.signature {
            out.extend_from_slice(SIG_MAGIC);
            out.extend_from_slice(&(sig.len() as u32).to_le_bytes());
            out.extend_from_slice(sig);
        }
        Ok(out)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, GhostError> {
        if bytes.len() < 4 + 2 + 4 + 4 + 4 + 4 + 4 + 4 {
            return Err(GhostError::CapsuleCorrupt("truncated header"));
        }
        if &bytes[0..4] != MAGIC {
            return Err(GhostError::CapsuleCorrupt("bad magic"));
        }
        let format_version = u16::from_le_bytes([bytes[4], bytes[5]]);
        if format_version != FORMAT_VERSION_V1 && format_version != FORMAT_VERSION_V2 && format_version != FORMAT_VERSION_V3 {
            return Err(GhostError::CapsuleCorrupt("unsupported format version"));
        }
        let mut i = 6;
        let vm_revision = read_u32(bytes, &mut i)?;
        let fuel_per_touch = read_u32(bytes, &mut i)?;
        let capabilities = if format_version >= FORMAT_VERSION_V2 {
            read_u32(bytes, &mut i)?
        } else {
            CAPABILITIES_ALL
        };
        let pc = read_u32(bytes, &mut i)?;
        let stack_len = read_u32(bytes, &mut i)? as usize;
        if stack_len > MAX_STACK {
            return Err(GhostError::CapsuleCorrupt("stack too large"));
        }
        let val_size = if format_version >= FORMAT_VERSION_V3 { 8 } else { 4 };
        let need_stack = i + stack_len * val_size;
        if need_stack > bytes.len() {
            return Err(GhostError::CapsuleCorrupt("truncated stack"));
        }
        let mut stack = Vec::with_capacity(stack_len);
        if format_version >= FORMAT_VERSION_V3 {
            for _ in 0..stack_len {
                stack.push(read_i64(bytes, &mut i)?);
            }
        } else {
            for _ in 0..stack_len {
                stack.push(read_u32(bytes, &mut i)? as i32 as i64);
            }
        }
        let ram_len = read_u32(bytes, &mut i)? as usize;
        if ram_len > MAX_RAM {
            return Err(GhostError::CapsuleCorrupt("ram too large"));
        }
        let need_ram = i + ram_len;
        if need_ram > bytes.len() {
            return Err(GhostError::CapsuleCorrupt("truncated ram"));
        }
        let ram = bytes[i..i + ram_len].to_vec();
        i += ram_len;
        let code_len = read_u32(bytes, &mut i)? as usize;
        if code_len > MAX_CODE {
            return Err(GhostError::CapsuleCorrupt("code too large"));
        }
        let need_code = i + code_len;
        if need_code > bytes.len() {
            return Err(GhostError::CapsuleCorrupt("truncated code"));
        }
        let code = bytes[i..i + code_len].to_vec();
        i += code_len;

        // Optional GPAG (pages) section
        let mut pages: Vec<Option<Vec<u8>>> = Vec::new();
        if i + 4 <= bytes.len() && &bytes[i..i + 4] == GPAG_MAGIC {
            i += 4;
            if i + 2 > bytes.len() {
                return Err(GhostError::CapsuleCorrupt("truncated GPAG header"));
            }
            let alloc_count = u16::from_le_bytes([bytes[i], bytes[i + 1]]) as usize;
            i += 2;
            for _ in 0..alloc_count {
                if i >= bytes.len() {
                    return Err(GhostError::CapsuleCorrupt("truncated page index"));
                }
                let page_idx = bytes[i] as usize;
                i += 1;
                if page_idx >= MAX_PAGES {
                    return Err(GhostError::CapsuleCorrupt("page index too large"));
                }
                if i + PAGE_SIZE > bytes.len() {
                    return Err(GhostError::CapsuleCorrupt("truncated page data"));
                }
                while pages.len() <= page_idx {
                    pages.push(None);
                }
                pages[page_idx] = Some(bytes[i..i + PAGE_SIZE].to_vec());
                i += PAGE_SIZE;
            }
        }

        let mut signature = None;
        if i < bytes.len() {
            if bytes.len() < i + 8 {
                return Err(GhostError::CapsuleCorrupt("truncated signature header"));
            }
            if &bytes[i..i + 4] != SIG_MAGIC {
                return Err(GhostError::CapsuleCorrupt("trailing garbage after code"));
            }
            let sig_len = u32::from_le_bytes([
                bytes[i + 4],
                bytes[i + 5],
                bytes[i + 6],
                bytes[i + 7],
            ]) as usize;
            if sig_len > MAX_SIGNATURE_LEN {
                return Err(GhostError::CapsuleCorrupt("signature too large"));
            }
            let end = i + 8 + sig_len;
            if end != bytes.len() {
                return Err(GhostError::CapsuleCorrupt("bad signed blob length"));
            }
            signature = Some(bytes[i + 8..end].to_vec());
        }

        Ok(Capsule {
            format_version,
            vm_revision,
            pc,
            stack,
            ram,
            code,
            fuel_per_touch,
            capabilities,
            pages,
            signature,
        })
    }
}

fn read_u32(bytes: &[u8], i: &mut usize) -> Result<u32, GhostError> {
    if *i + 4 > bytes.len() {
        return Err(GhostError::CapsuleCorrupt("read u32"));
    }
    let v = u32::from_le_bytes([
        bytes[*i],
        bytes[*i + 1],
        bytes[*i + 2],
        bytes[*i + 3],
    ]);
    *i += 4;
    Ok(v)
}

fn read_i64(bytes: &[u8], i: &mut usize) -> Result<i64, GhostError> {
    if *i + 8 > bytes.len() {
        return Err(GhostError::CapsuleCorrupt("read i64"));
    }
    let v = i64::from_le_bytes([
        bytes[*i], bytes[*i + 1], bytes[*i + 2], bytes[*i + 3],
        bytes[*i + 4], bytes[*i + 5], bytes[*i + 6], bytes[*i + 7],
    ]);
    *i += 8;
    Ok(v)
}

fn validate_sizes(c: &Capsule) -> Result<(), GhostError> {
    if c.stack.len() > MAX_STACK {
        return Err(GhostError::StackOverflow);
    }
    if c.ram.len() > MAX_RAM {
        return Err(GhostError::RamOutOfBounds);
    }
    if c.code.len() > MAX_CODE {
        return Err(GhostError::PcOutOfBounds);
    }
    Ok(())
}

fn ct_eq_sig(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut d = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        d |= x ^ y;
    }
    d == 0
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    ghost_vm_sha::hmac_sha256(key, data).to_vec()
}

/// SHA-256 digest (32 bytes). Used by SYS_HASH_RAM in WorldHost.
pub fn sha256_digest(data: &[u8]) -> [u8; 32] {
    ghost_vm_sha::sha256(data)
}

/// Set [`Capsule::signature`] to HMAC-SHA256 of [`Capsule::encode_payload`].
pub fn sign_capsule(capsule: &mut Capsule, key: &[u8]) -> Result<(), GhostError> {
    let payload = capsule.encode_payload()?;
    let tag = hmac_sha256(key, &payload);
    capsule.signature = Some(tag);
    Ok(())
}

/// Verifies HMAC over [`Capsule::encode_payload`] against [`Capsule::signature`].
pub fn verify_capsule(capsule: &Capsule, key: &[u8]) -> Result<(), GhostError> {
    let sig = capsule.signature.as_ref().ok_or(GhostError::SignatureMissing)?;
    let payload = capsule.encode_payload()?;
    let expected = hmac_sha256(key, &payload);
    if ct_eq_sig(sig, &expected) {
        Ok(())
    } else {
        Err(GhostError::SignatureInvalid)
    }
}

/// Host hooks for syscalls. Return `Ok(true)` to keep running, `Ok(false)` to stop cleanly.
pub trait GhostHost {
    fn syscall(&mut self, id: u8, capsule: &mut Capsule) -> Result<bool, GhostError>;
}

/// No-op host: SYS_NOP continues; unknown syscalls stop.
pub struct NullHost;

impl GhostHost for NullHost {
    fn syscall(&mut self, id: u8, _capsule: &mut Capsule) -> Result<bool, GhostError> {
        match id {
            SYS_NOP => Ok(true),
            _ => Ok(false),
        }
    }
}

/// Counting host for tests.
pub struct LogHost {
    pub logged: Vec<i64>,
}

impl GhostHost for LogHost {
    fn syscall(&mut self, id: u8, capsule: &mut Capsule) -> Result<bool, GhostError> {
        match id {
            SYS_NOP => Ok(true),
            SYS_HOST_LOG => {
                let v = capsule.stack.pop().ok_or(GhostError::StackUnderflow)?;
                self.logged.push(v);
                Ok(true)
            }
            SYS_CHECKPOINT => Ok(true),
            _ => Ok(false),
        }
    }
}

/// Write a string into capsule RAM starting at `addr`. Returns bytes written.
pub fn ram_write_str(capsule: &mut Capsule, addr: usize, s: &str) -> Result<usize, GhostError> {
    let bytes = s.as_bytes();
    if addr + bytes.len() > capsule.ram.len() {
        return Err(GhostError::RamOutOfBounds);
    }
    capsule.ram[addr..addr + bytes.len()].copy_from_slice(bytes);
    Ok(bytes.len())
}

/// Read a string from capsule RAM at `addr` for `len` bytes.
pub fn ram_read_str(capsule: &Capsule, addr: usize, len: usize) -> Result<String, GhostError> {
    if addr + len > capsule.ram.len() {
        return Err(GhostError::RamOutOfBounds);
    }
    String::from_utf8(capsule.ram[addr..addr + len].to_vec())
        .map_err(|_| GhostError::CapsuleCorrupt("invalid UTF-8 in RAM"))
}

/// Interactive host for live mode with I/O, child capsule spawning, and evolution primitives.
pub struct InteractiveHost {
    pub input_buffer: std::collections::VecDeque<String>,
    pub output_buffer: Vec<u8>,
    pub children: Vec<Option<Capsule>>,
    pub rng_seed: u64,
}

impl InteractiveHost {
    pub fn new() -> Self {
        Self {
            input_buffer: std::collections::VecDeque::new(),
            output_buffer: Vec::new(),
            children: Vec::new(),
            rng_seed: 0,
        }
    }

    fn next_random(&mut self) -> i64 {
        if self.rng_seed == 0 {
            self.rng_seed = 12345;
        }
        self.rng_seed = self.rng_seed.wrapping_mul(1103515245).wrapping_add(12345);
        ((self.rng_seed >> 16) & 0x7FFF) as i64
    }

    fn alloc_child_slot(&mut self) -> Option<usize> {
        for (i, slot) in self.children.iter().enumerate() {
            if slot.is_none() {
                return Some(i);
            }
        }
        if self.children.len() < MAX_CHILDREN {
            let idx = self.children.len();
            self.children.push(None);
            return Some(idx);
        }
        None
    }

    fn read_line_into_ram(&mut self, capsule: &mut Capsule) -> Result<bool, GhostError> {
        let addr = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as usize;
        let line = if let Some(buffered) = self.input_buffer.pop_front() {
            buffered
        } else {
            let mut buf = String::new();
            std::io::stdin()
                .read_line(&mut buf)
                .map_err(|_| GhostError::CapsuleCorrupt("stdin read failed"))?;
            buf.trim_end_matches('\n')
                .trim_end_matches('\r')
                .to_string()
        };
        let bytes = line.as_bytes();
        let len = bytes.len().min(4096);
        if addr + len > capsule.ram.len() {
            return Err(GhostError::RamOutOfBounds);
        }
        capsule.ram[addr..addr + len].copy_from_slice(&bytes[..len]);
        if capsule.stack.len() >= MAX_STACK {
            return Err(GhostError::StackOverflow);
        }
        capsule.stack.push(len as i64);
        Ok(true)
    }
}

impl GhostHost for InteractiveHost {
    fn syscall(&mut self, id: u8, capsule: &mut Capsule) -> Result<bool, GhostError> {
        match id {
            SYS_NOP => Ok(true),
            SYS_HOST_LOG => {
                let v = capsule.stack.pop().ok_or(GhostError::StackUnderflow)?;
                let _ = write!(self.output_buffer, "{v}");
                Ok(true)
            }
            SYS_CHECKPOINT => Ok(true),
            SYS_PRINT_NUM => {
                let v = capsule.stack.pop().ok_or(GhostError::StackUnderflow)?;
                let msg = format!("{v}\n");
                self.output_buffer.extend_from_slice(msg.as_bytes());
                let _ = std::io::stdout().write_all(msg.as_bytes());
                let _ = std::io::stdout().flush();
                Ok(true)
            }
            SYS_PRINT_STR => {
                let len = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as usize;
                let addr = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as usize;
                if addr + len > capsule.ram.len() {
                    return Err(GhostError::RamOutOfBounds);
                }
                let bytes = &capsule.ram[addr..addr + len];
                self.output_buffer.extend_from_slice(bytes);
                let _ = std::io::stdout().write_all(bytes);
                let _ = std::io::stdout().flush();
                Ok(true)
            }
            SYS_READ_LINE => self.read_line_into_ram(capsule),
            SYS_PRINT_CHAR => {
                let v = capsule.stack.pop().ok_or(GhostError::StackUnderflow)?;
                let ch = (v & 0x7F) as u8;
                self.output_buffer.push(ch);
                let _ = std::io::stdout().write_all(&[ch]);
                let _ = std::io::stdout().flush();
                Ok(true)
            }
            SYS_PROMPT => {
                let prompt = b"ghost> ";
                self.output_buffer.extend_from_slice(prompt);
                let _ = std::io::stdout().write_all(prompt);
                let _ = std::io::stdout().flush();
                self.read_line_into_ram(capsule)
            }
            SYS_FPRINT => {
                let v = capsule.stack.pop().ok_or(GhostError::StackUnderflow)?;
                let f = f64::from_bits(v as u64);
                let msg = format!("{f}\n");
                self.output_buffer.extend_from_slice(msg.as_bytes());
                let _ = std::io::stdout().write_all(msg.as_bytes());
                let _ = std::io::stdout().flush();
                Ok(true)
            }
            SYS_STR_LOWER => {
                let len = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as usize;
                let addr = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as usize;
                if addr.saturating_add(len) > capsule.ram.len() {
                    capsule.stack.push(0);
                } else {
                    for i in addr..addr + len {
                        capsule.ram[i] = capsule.ram[i].to_ascii_lowercase();
                    }
                    capsule.stack.push(len as i64);
                }
                Ok(true)
            }
            SYS_STR_EQ => {
                let len2 = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as usize;
                let addr2 = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as usize;
                let len1 = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as usize;
                let addr1 = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as usize;
                if len1 != len2
                    || addr1.saturating_add(len1) > capsule.ram.len()
                    || addr2.saturating_add(len2) > capsule.ram.len()
                {
                    capsule.stack.push(0);
                } else {
                    let eq = capsule.ram[addr1..addr1 + len1] == capsule.ram[addr2..addr2 + len2];
                    capsule.stack.push(if eq { 1 } else { 0 });
                }
                Ok(true)
            }
            SYS_SPAWN_INIT => {
                let ram_addr = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as usize;
                let code_len = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as usize;
                if ram_addr.saturating_add(code_len) > capsule.ram.len() {
                    return Err(GhostError::RamOutOfBounds);
                }
                let child_code = capsule.ram[ram_addr..ram_addr + code_len].to_vec();
                let idx = self.alloc_child_slot().ok_or(GhostError::StackOverflow)?;
                let mut child = Capsule::with_ram_and_fuel(MAX_RAM, 10_000);
                child.code = child_code;
                self.children[idx] = Some(child);
                capsule.stack.push(idx as i64);
                Ok(true)
            }
            SYS_SPAWN_RUN => {
                let fuel = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as u32;
                let ci = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as usize;
                let child = self.children.get_mut(ci)
                    .and_then(|s| s.as_mut())
                    .ok_or(GhostError::CapsuleCorrupt("bad child index"))?;
                let mut null = NullHost;
                let _ = run(child, &mut null, Some(fuel));
                let result = child.stack.last().copied().unwrap_or(0);
                capsule.stack.push(result);
                Ok(true)
            }
            SYS_SPAWN_KILL => {
                let ci = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as usize;
                if ci < self.children.len() {
                    self.children[ci] = None;
                }
                capsule.stack.push(1);
                Ok(true)
            }
            SYS_SPAWN_READ => {
                let off = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as usize;
                let ci = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as usize;
                let child = self.children.get(ci)
                    .and_then(|s| s.as_ref())
                    .ok_or(GhostError::CapsuleCorrupt("bad child index"))?;
                if off + 8 > child.ram.len() {
                    capsule.stack.push(0);
                } else {
                    let v = i64::from_le_bytes([
                        child.ram[off], child.ram[off+1], child.ram[off+2], child.ram[off+3],
                        child.ram[off+4], child.ram[off+5], child.ram[off+6], child.ram[off+7],
                    ]);
                    capsule.stack.push(v);
                }
                Ok(true)
            }
            SYS_SPAWN_WRITE => {
                let val = capsule.stack.pop().ok_or(GhostError::StackUnderflow)?;
                let off = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as usize;
                let ci = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as usize;
                let child = self.children.get_mut(ci)
                    .and_then(|s| s.as_mut())
                    .ok_or(GhostError::CapsuleCorrupt("bad child index"))?;
                if off + 8 <= child.ram.len() {
                    child.ram[off..off+8].copy_from_slice(&val.to_le_bytes());
                }
                Ok(true)
            }
            SYS_SPAWN_COUNT => {
                let count = self.children.iter().filter(|s| s.is_some()).count();
                capsule.stack.push(count as i64);
                Ok(true)
            }
            SYS_RANDOM => {
                let v = self.next_random();
                capsule.stack.push(v);
                Ok(true)
            }
            SYS_HASH => {
                let ram_addr = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as usize;
                let length = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as usize;
                if ram_addr.saturating_add(length) > capsule.ram.len() {
                    capsule.stack.push(0);
                } else {
                    let mut h: u32 = 5381;
                    for &b in &capsule.ram[ram_addr..ram_addr + length] {
                        h = h.wrapping_mul(33).wrapping_add(b as u32);
                    }
                    capsule.stack.push(h as i64);
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

/// Run up to `fuel` instructions or until HALT / host stop / error.
/// `ctrlc_flag`: if provided, checked every 1024 instructions to allow graceful interruption.
/// `debug_mode`: Trace prints each instruction; Step also waits for Enter.
pub fn run<H: GhostHost>(
    capsule: &mut Capsule,
    host: &mut H,
    fuel: Option<u32>,
) -> Result<RunStatus, GhostError> {
    run_ex(capsule, host, fuel, None, DebugMode::None)
}

pub fn run_ex<H: GhostHost>(
    capsule: &mut Capsule,
    host: &mut H,
    fuel: Option<u32>,
    ctrlc_flag: Option<&std::sync::atomic::AtomicBool>,
    debug_mode: DebugMode,
) -> Result<RunStatus, GhostError> {
    if capsule.vm_revision != VM_REVISION {
        return Err(GhostError::CapsuleCorrupt("vm_revision mismatch"));
    }
    let mut fuel_left = fuel.unwrap_or(capsule.fuel_per_touch);
    let mut tick: u32 = 0;

    loop {
        if fuel_left == 0 {
            return Ok(RunStatus::FuelExhausted);
        }
        fuel_left -= 1;
        tick += 1;

        if tick & 0x3FF == 0 {
            if let Some(flag) = ctrlc_flag {
                if flag.load(std::sync::atomic::Ordering::Relaxed) {
                    return Err(GhostError::Interrupted);
                }
            }
        }

        let pci = capsule.pc as usize;
        if pci >= capsule.code.len() {
            return Err(GhostError::PcOutOfBounds);
        }
        let op = capsule.code[pci];

        if matches!(debug_mode, DebugMode::Trace | DebugMode::Step) {
            let instr = disassemble_instruction(&capsule.code, pci);
            let slen = capsule.stack.len().min(8);
            let stail: Vec<String> = capsule.stack[capsule.stack.len()-slen..].iter().map(|v| v.to_string()).collect();
            let rlen = capsule.ram.len().min(8);
            eprintln!("[PC:{pci:04}] {:<20} stack: [{}]  ram[0..{rlen}]: {:?}",
                instr, stail.join(", "), &capsule.ram[..rlen]);
            if debug_mode == DebugMode::Step {
                let mut input = String::new();
                let _ = std::io::stdin().read_line(&mut input);
                let cmd = input.trim();
                if cmd == "q" { return Ok(RunStatus::Stopped); }
            }
        }

        capsule.pc = pci as u32 + 1;

        match op {
            OP_NOP => {}
            OP_PUSH => {
                let start = capsule.pc as usize;
                if start + 4 > capsule.code.len() {
                    return Err(GhostError::PcOutOfBounds);
                }
                let imm = i32::from_le_bytes([
                    capsule.code[start],
                    capsule.code[start + 1],
                    capsule.code[start + 2],
                    capsule.code[start + 3],
                ]);
                capsule.pc += 4;
                if capsule.stack.len() >= MAX_STACK {
                    return Err(GhostError::StackOverflow);
                }
                capsule.stack.push(imm as i64);
            }
            OP_POP => {
                capsule.stack.pop().ok_or(GhostError::StackUnderflow)?;
            }
            OP_DUP => {
                let v = *capsule
                    .stack
                    .last()
                    .ok_or(GhostError::StackUnderflow)?;
                if capsule.stack.len() >= MAX_STACK {
                    return Err(GhostError::StackOverflow);
                }
                capsule.stack.push(v);
            }
            OP_SWAP => {
                let len = capsule.stack.len();
                if len < 2 {
                    return Err(GhostError::StackUnderflow);
                }
                capsule.stack.swap(len - 1, len - 2);
            }
            OP_ROT => {
                let len = capsule.stack.len();
                if len < 3 {
                    return Err(GhostError::StackUnderflow);
                }
                // a b c → b c a  (bottom element goes to top)
                let a = capsule.stack[len - 3];
                capsule.stack[len - 3] = capsule.stack[len - 2];
                capsule.stack[len - 2] = capsule.stack[len - 1];
                capsule.stack[len - 1] = a;
            }
            OP_ADD => {
                let b = capsule.stack.pop().ok_or(GhostError::StackUnderflow)?;
                let a = capsule.stack.pop().ok_or(GhostError::StackUnderflow)?;
                capsule.stack.push(a.wrapping_add(b));
            }
            OP_SUB => {
                let b = capsule.stack.pop().ok_or(GhostError::StackUnderflow)?;
                let a = capsule.stack.pop().ok_or(GhostError::StackUnderflow)?;
                capsule.stack.push(a.wrapping_sub(b));
            }
            OP_MUL => {
                let b = capsule.stack.pop().ok_or(GhostError::StackUnderflow)?;
                let a = capsule.stack.pop().ok_or(GhostError::StackUnderflow)?;
                capsule.stack.push(a.wrapping_mul(b));
            }
            OP_DIV => {
                let b = capsule.stack.pop().ok_or(GhostError::StackUnderflow)?;
                let a = capsule.stack.pop().ok_or(GhostError::StackUnderflow)?;
                capsule.stack.push(if b == 0 { 0 } else { a.wrapping_div(b) });
            }
            OP_MOD => {
                let b = capsule.stack.pop().ok_or(GhostError::StackUnderflow)?;
                let a = capsule.stack.pop().ok_or(GhostError::StackUnderflow)?;
                capsule.stack.push(if b == 0 { 0 } else { a.wrapping_rem(b) });
            }
            OP_EQ => {
                let b = capsule.stack.pop().ok_or(GhostError::StackUnderflow)?;
                let a = capsule.stack.pop().ok_or(GhostError::StackUnderflow)?;
                capsule.stack.push(if a == b { 1 } else { 0 });
            }
            OP_LT => {
                let b = capsule.stack.pop().ok_or(GhostError::StackUnderflow)?;
                let a = capsule.stack.pop().ok_or(GhostError::StackUnderflow)?;
                capsule.stack.push(if a < b { 1 } else { 0 });
            }
            OP_GT => {
                let b = capsule.stack.pop().ok_or(GhostError::StackUnderflow)?;
                let a = capsule.stack.pop().ok_or(GhostError::StackUnderflow)?;
                capsule.stack.push(if a > b { 1 } else { 0 });
            }
            OP_LOAD => {
                let start = capsule.pc as usize;
                if start + 2 > capsule.code.len() {
                    return Err(GhostError::PcOutOfBounds);
                }
                let off =
                    u16::from_le_bytes([capsule.code[start], capsule.code[start + 1]]) as usize;
                capsule.pc += 2;
                if off + 8 > capsule.ram.len() {
                    return Err(GhostError::RamOutOfBounds);
                }
                let v = i64::from_le_bytes([
                    capsule.ram[off], capsule.ram[off + 1],
                    capsule.ram[off + 2], capsule.ram[off + 3],
                    capsule.ram[off + 4], capsule.ram[off + 5],
                    capsule.ram[off + 6], capsule.ram[off + 7],
                ]);
                if capsule.stack.len() >= MAX_STACK {
                    return Err(GhostError::StackOverflow);
                }
                capsule.stack.push(v);
            }
            OP_STORE => {
                let start = capsule.pc as usize;
                if start + 2 > capsule.code.len() {
                    return Err(GhostError::PcOutOfBounds);
                }
                let off =
                    u16::from_le_bytes([capsule.code[start], capsule.code[start + 1]]) as usize;
                capsule.pc += 2;
                let v = capsule.stack.pop().ok_or(GhostError::StackUnderflow)?;
                if off + 8 > capsule.ram.len() {
                    return Err(GhostError::RamOutOfBounds);
                }
                capsule.ram[off..off + 8].copy_from_slice(&v.to_le_bytes());
            }
            OP_JMP => {
                let start = capsule.pc as usize;
                if start + 2 > capsule.code.len() {
                    return Err(GhostError::PcOutOfBounds);
                }
                let rel = i16::from_le_bytes([capsule.code[start], capsule.code[start + 1]]) as i32;
                capsule.pc = (capsule.pc as i32 + rel) as u32;
            }
            OP_JMPIF => {
                let start = capsule.pc as usize;
                if start + 2 > capsule.code.len() {
                    return Err(GhostError::PcOutOfBounds);
                }
                let rel = i16::from_le_bytes([capsule.code[start], capsule.code[start + 1]]) as i32;
                capsule.pc += 2;
                let cond = capsule.stack.pop().ok_or(GhostError::StackUnderflow)?;
                if cond != 0 {
                    // Use `start` (not capsule.pc) so jmpif is consistent with jmp:
                    // both interpret rel as offset from the operand start.
                    capsule.pc = (start as i32 + rel) as u32;
                }
            }
            OP_SYSCALL => {
                let pci = capsule.pc as usize;
                if pci >= capsule.code.len() {
                    return Err(GhostError::PcOutOfBounds);
                }
                let id = capsule.code[pci];
                capsule.pc += 1;
                if !capsule.syscall_allowed(id) {
                    return Err(GhostError::SyscallDenied(id));
                }
                match id {
                    SYS_YIELD => return Ok(RunStatus::Yielded),
                    SYS_TICK => {
                        if capsule.stack.len() >= MAX_STACK {
                            return Err(GhostError::StackOverflow);
                        }
                        capsule.stack.push(fuel_left as i64);
                        continue;
                    }
                    SYS_STACK_DEPTH => {
                        if capsule.stack.len() >= MAX_STACK {
                            return Err(GhostError::StackOverflow);
                        }
                        capsule.stack.push(capsule.stack.len() as i64);
                        continue;
                    }
                    SYS_CODE_LEN => {
                        if capsule.stack.len() >= MAX_STACK {
                            return Err(GhostError::StackOverflow);
                        }
                        capsule.stack.push(capsule.code.len() as i64);
                        continue;
                    }
                    SYS_CODE_READ => {
                        let off = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as usize;
                        let val: i64 = if off < capsule.code.len() {
                            capsule.code[off] as i64
                        } else {
                            -1
                        };
                        capsule.stack.push(val);
                        continue;
                    }
                    SYS_CODE_WRITE => {
                        if capsule.capabilities & CAPABILITY_SELF_MODIFY == 0 {
                            return Err(GhostError::SyscallDenied(SYS_CODE_WRITE));
                        }
                        let off = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as usize;
                        let val = capsule.stack.pop().ok_or(GhostError::StackUnderflow)?;
                        let ok = if off < capsule.code.len() {
                            capsule.code[off] = (val & 0xFF) as u8;
                            1
                        } else {
                            0
                        };
                        capsule.stack.push(ok);
                        continue;
                    }
                    SYS_RAM_LEN => {
                        if capsule.stack.len() >= MAX_STACK {
                            return Err(GhostError::StackOverflow);
                        }
                        capsule.stack.push(capsule.ram.len() as i64);
                        continue;
                    }
                    SYS_PC => {
                        if capsule.stack.len() >= MAX_STACK {
                            return Err(GhostError::StackOverflow);
                        }
                        capsule.stack.push(capsule.pc as i64);
                        continue;
                    }
                    SYS_COPY_CODE => {
                        let dest_ram = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as usize;
                        let src_off = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as usize;
                        let length = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as usize;
                        let src_end = src_off.saturating_add(length).min(capsule.code.len());
                        let actual = src_end.saturating_sub(src_off);
                        if dest_ram.saturating_add(actual) > capsule.ram.len() {
                            capsule.stack.push(0);
                        } else {
                            capsule.ram[dest_ram..dest_ram + actual]
                                .copy_from_slice(&capsule.code[src_off..src_off + actual]);
                            capsule.stack.push(actual as i64);
                        }
                        continue;
                    }
                    _ => {}
                }
                let cont = host.syscall(id, capsule)?;
                if !cont {
                    return Ok(RunStatus::Stopped);
                }
            }
            OP_FCONST => {
                let start = capsule.pc as usize;
                if start + 8 > capsule.code.len() {
                    return Err(GhostError::PcOutOfBounds);
                }
                let lo = u32::from_le_bytes([
                    capsule.code[start], capsule.code[start+1],
                    capsule.code[start+2], capsule.code[start+3],
                ]);
                let hi = u32::from_le_bytes([
                    capsule.code[start+4], capsule.code[start+5],
                    capsule.code[start+6], capsule.code[start+7],
                ]);
                capsule.pc += 8;
                let bits = ((hi as u64) << 32) | (lo as u64);
                if capsule.stack.len() >= MAX_STACK {
                    return Err(GhostError::StackOverflow);
                }
                capsule.stack.push(bits as i64);
            }
            OP_FADD => {
                let b_bits = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as u64;
                let a_bits = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as u64;
                let r = f64::from_bits(a_bits) + f64::from_bits(b_bits);
                capsule.stack.push(r.to_bits() as i64);
            }
            OP_FSUB => {
                let b_bits = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as u64;
                let a_bits = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as u64;
                let r = f64::from_bits(a_bits) - f64::from_bits(b_bits);
                capsule.stack.push(r.to_bits() as i64);
            }
            OP_FMUL => {
                let b_bits = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as u64;
                let a_bits = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as u64;
                let r = f64::from_bits(a_bits) * f64::from_bits(b_bits);
                capsule.stack.push(r.to_bits() as i64);
            }
            OP_FDIV => {
                let b_bits = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as u64;
                let a_bits = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as u64;
                let b = f64::from_bits(b_bits);
                let r = if b == 0.0 { f64::NAN } else { f64::from_bits(a_bits) / b };
                capsule.stack.push(r.to_bits() as i64);
            }
            OP_ITOF => {
                let v = capsule.stack.pop().ok_or(GhostError::StackUnderflow)?;
                let f = v as f64;
                capsule.stack.push(f.to_bits() as i64);
            }
            OP_FTOI => {
                let bits = capsule.stack.pop().ok_or(GhostError::StackUnderflow)? as u64;
                let f = f64::from_bits(bits);
                capsule.stack.push(f as i64);
            }
            OP_HALT => return Ok(RunStatus::Halted),
            _ => return Err(GhostError::UnknownOpcode(op)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Halted,
    Stopped,
    FuelExhausted,
    Yielded,
}

fn asm_err(line: usize, msg: impl Into<String>) -> GhostError {
    GhostError::AssembleError(format!("line {}: {}", line, msg.into()))
}

/// One line per instruction; `#` starts a comment. Directives: `.fuel N`, `.ram N` (bytes, capped at [`MAX_RAM`]).
///
/// **Labels:** `name:` at the start of a line (before the mnemonic), e.g. `loop: push 1` or `loop:` alone.
///
/// Opcodes: `nop` `push N` `pop` `dup` `add` `sub` `mul` `load U16` `store U16` `jmp` `jmpif` `syscall U8` `halt`
///
/// `jmp` / `jmpif` take either an **`i16` literal** (relative offset from the first byte of the encoded offset) or a **label** name.
#[derive(Debug)]
struct Fixup {
    rel_offset: usize,
    label: String,
}

fn label_ident_ok(s: &str) -> bool {
    let mut ch = s.chars();
    let Some(first) = ch.next() else {
        return false;
    };
    if !matches!(first, 'a'..='z' | 'A'..='Z' | '_') {
        return false;
    }
    ch.all(|c| matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_'))
}

/// `name: rest` → label `name` and remaining text (may be empty).
fn split_leading_label(line: &str) -> Option<(&str, &str)> {
    let t = line.trim_start();
    let first = t.split_whitespace().next()?;
    let name = first.strip_suffix(':')?;
    if name.is_empty() || !label_ident_ok(name) {
        return None;
    }
    let rest = t[first.len()..].trim_start();
    Some((name, rest))
}

fn parse_jmp_operand(tok: &str) -> Result<EitherRelOrLabel, String> {
    if let Ok(v) = tok.parse::<i16>() {
        return Ok(EitherRelOrLabel::Rel(v));
    }
    if label_ident_ok(tok) {
        return Ok(EitherRelOrLabel::Label(tok.to_string()));
    }
    Err(format!("bad jmp operand {tok:?}"))
}

#[derive(Debug)]
enum EitherRelOrLabel {
    Rel(i16),
    Label(String),
}

fn parse_caps_operand(s: &str) -> Result<u32, String> {
    let t = s.trim();
    let tl = t.to_ascii_lowercase();
    match tl.as_str() {
        "all" => Ok(CAPABILITIES_ALL),
        "none" => Ok(0),
        _ if t.len() > 2 && (t.starts_with("0x") || t.starts_with("0X")) => {
            u32::from_str_radix(t.trim_start_matches("0x").trim_start_matches("0X"), 16)
                .map_err(|_| "bad hex .caps".to_string())
        }
        _ => t
            .parse::<u32>()
            .map_err(|_| "bad .caps (use u32, 0x.., all, or none)".to_string()),
    }
}

fn parse_data_string(s: &str) -> Result<String, String> {
    let s = s.trim();
    if !s.starts_with('"') {
        return Err("expected string in double quotes".to_string());
    }
    let mut out = String::new();
    let mut chars = s[1..].chars();
    let mut closed = false;
    while let Some(c) = chars.next() {
        if c == '"' {
            closed = true;
            break;
        }
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some('0') => out.push('\0'),
                Some(other) => return Err(format!("unknown escape \\{other}")),
                None => return Err("unterminated string".to_string()),
            }
        } else {
            out.push(c);
        }
    }
    if !closed {
        return Err("unterminated string literal".to_string());
    }
    Ok(out)
}

pub fn assemble_capsule(source: &str) -> Result<Capsule, GhostError> {
    let mut code: Vec<u8> = Vec::new();
    let mut fuel_per_touch: u32 = 10_000;
    let mut ram_len: usize = MAX_RAM;
    let mut capabilities: u32 = CAPABILITIES_ALL;
    let mut labels: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    let mut fixups: Vec<Fixup> = Vec::new();
    let mut ram_init: Vec<(usize, Vec<u8>)> = Vec::new();

    for (idx, raw) in source.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        let line = if let Some((name, rest)) = split_leading_label(line) {
            if labels.insert(name.to_string(), code.len() as u32).is_some() {
                return Err(asm_err(
                    line_no,
                    format!("duplicate label {name:?}"),
                ));
            }
            if rest.is_empty() {
                continue;
            }
            rest
        } else {
            line
        };

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        let head = parts[0].to_lowercase();
        match head.as_str() {
            ".fuel" => {
                let n = parts
                    .get(1)
                    .and_then(|s| s.parse::<u32>().ok())
                    .ok_or_else(|| asm_err(line_no, ".fuel needs u32"))?;
                fuel_per_touch = n;
            }
            ".ram" => {
                let n = parts
                    .get(1)
                    .and_then(|s| s.parse::<usize>().ok())
                    .ok_or_else(|| asm_err(line_no, ".ram needs usize"))?;
                if n > MAX_RAM {
                    return Err(asm_err(line_no, format!(".ram max {MAX_RAM}")));
                }
                ram_len = n;
            }
            ".caps" => {
                let v = parts
                    .get(1)
                    .ok_or_else(|| asm_err(line_no, ".caps needs value (u32, 0x.., all, none)"))?;
                if parts.len() != 2 {
                    return Err(asm_err(line_no, ".caps takes one operand"));
                }
                capabilities =
                    parse_caps_operand(v).map_err(|e| asm_err(line_no, e))?;
            }
            ".data" => {
                // Re-parse from raw line to preserve strings containing #
                let raw_lower = raw.to_lowercase();
                let dpos = raw_lower.find(".data").unwrap();
                let after = raw[dpos + 5..].trim_start();
                let sp = after
                    .find(|c: char| c.is_whitespace())
                    .ok_or_else(|| asm_err(line_no, ".data needs addr and string"))?;
                let addr: usize = after[..sp]
                    .parse()
                    .map_err(|_| asm_err(line_no, ".data addr must be a number"))?;
                let str_part = after[sp..].trim_start();
                let s = parse_data_string(str_part).map_err(|e| asm_err(line_no, e))?;
                ram_init.push((addr, s.into_bytes()));
            }
            "nop" => {
                no_operands(&parts, line_no)?;
                code.push(OP_NOP);
            }
            "push" => {
                let n: i32 = one_i32(&parts, line_no)?;
                code.push(OP_PUSH);
                code.extend_from_slice(&n.to_le_bytes());
            }
            "pop" => {
                no_operands(&parts, line_no)?;
                code.push(OP_POP);
            }
            "dup" => {
                no_operands(&parts, line_no)?;
                code.push(OP_DUP);
            }
            "swap" => {
                no_operands(&parts, line_no)?;
                code.push(OP_SWAP);
            }
            "rot" => {
                no_operands(&parts, line_no)?;
                code.push(OP_ROT);
            }
            "add" => {
                no_operands(&parts, line_no)?;
                code.push(OP_ADD);
            }
            "sub" => {
                no_operands(&parts, line_no)?;
                code.push(OP_SUB);
            }
            "mul" => {
                no_operands(&parts, line_no)?;
                code.push(OP_MUL);
            }
            "div" => {
                no_operands(&parts, line_no)?;
                code.push(OP_DIV);
            }
            "mod" => {
                no_operands(&parts, line_no)?;
                code.push(OP_MOD);
            }
            "eq" => {
                no_operands(&parts, line_no)?;
                code.push(OP_EQ);
            }
            "lt" => {
                no_operands(&parts, line_no)?;
                code.push(OP_LT);
            }
            "gt" => {
                no_operands(&parts, line_no)?;
                code.push(OP_GT);
            }
            "load" => {
                let u = one_u16(&parts, line_no)?;
                code.push(OP_LOAD);
                code.extend_from_slice(&u.to_le_bytes());
            }
            "store" => {
                let u = one_u16(&parts, line_no)?;
                code.push(OP_STORE);
                code.extend_from_slice(&u.to_le_bytes());
            }
            "jmp" => {
                if parts.len() != 2 {
                    return Err(asm_err(line_no, "jmp needs one operand (i16 or label)"));
                }
                match parse_jmp_operand(parts[1]).map_err(|s| asm_err(line_no, s))? {
                    EitherRelOrLabel::Rel(r) => {
                        code.push(OP_JMP);
                        code.extend_from_slice(&r.to_le_bytes());
                    }
                    EitherRelOrLabel::Label(lab) => {
                        code.push(OP_JMP);
                        let rel_pos = code.len();
                        code.extend_from_slice(&0i16.to_le_bytes());
                        fixups.push(Fixup {
                            rel_offset: rel_pos,
                            label: lab,
                        });
                    }
                }
            }
            "jmpif" => {
                if parts.len() != 2 {
                    return Err(asm_err(line_no, "jmpif needs one operand (i16 or label)"));
                }
                match parse_jmp_operand(parts[1]).map_err(|s| asm_err(line_no, s))? {
                    EitherRelOrLabel::Rel(r) => {
                        code.push(OP_JMPIF);
                        code.extend_from_slice(&r.to_le_bytes());
                    }
                    EitherRelOrLabel::Label(lab) => {
                        code.push(OP_JMPIF);
                        let rel_pos = code.len();
                        code.extend_from_slice(&0i16.to_le_bytes());
                        fixups.push(Fixup {
                            rel_offset: rel_pos,
                            label: lab,
                        });
                    }
                }
            }
            "syscall" => {
                let id = parts
                    .get(1)
                    .and_then(|s| s.parse::<u8>().ok())
                    .ok_or_else(|| asm_err(line_no, "syscall needs u8"))?;
                if parts.len() != 2 {
                    return Err(asm_err(line_no, "extra operands"));
                }
                code.push(OP_SYSCALL);
                code.push(id);
            }
            "fconst" => {
                if parts.len() != 2 {
                    return Err(asm_err(line_no, "fconst needs one f64 operand"));
                }
                let f: f64 = parts[1].parse().map_err(|_| asm_err(line_no, "bad f64"))?;
                let bits = f.to_bits();
                let lo = (bits & 0xFFFF_FFFF) as u32;
                let hi = (bits >> 32) as u32;
                code.push(OP_FCONST);
                code.extend_from_slice(&lo.to_le_bytes());
                code.extend_from_slice(&hi.to_le_bytes());
            }
            "fadd" => { no_operands(&parts, line_no)?; code.push(OP_FADD); }
            "fsub" => { no_operands(&parts, line_no)?; code.push(OP_FSUB); }
            "fmul" => { no_operands(&parts, line_no)?; code.push(OP_FMUL); }
            "fdiv" => { no_operands(&parts, line_no)?; code.push(OP_FDIV); }
            "itof" => { no_operands(&parts, line_no)?; code.push(OP_ITOF); }
            "ftoi" => { no_operands(&parts, line_no)?; code.push(OP_FTOI); }
            "halt" => {
                no_operands(&parts, line_no)?;
                code.push(OP_HALT);
            }
            _ => {
                return Err(asm_err(
                    line_no,
                    format!("unknown mnemonic {:?}", parts[0]),
                ));
            }
        }
        if code.len() > MAX_CODE {
            return Err(asm_err(line_no, format!("code exceeds {MAX_CODE} bytes")));
        }
    }

    for f in &fixups {
        let target = *labels
            .get(&f.label)
            .ok_or(GhostError::UndefinedLabel(f.label.clone()))?;
        let rel_i32 = (target as i32) - (f.rel_offset as i32);
        let rel = i16::try_from(rel_i32).map_err(|_| {
            GhostError::AssembleError(format!(
                "jump to {} out of i16 range",
                f.label
            ))
        })?;
        code[f.rel_offset..f.rel_offset + 2].copy_from_slice(&rel.to_le_bytes());
    }

    if code.is_empty() {
        code.push(OP_HALT);
    }

    let mut capsule = Capsule {
        format_version: FORMAT_VERSION_LATEST,
        vm_revision: VM_REVISION,
        pc: 0,
        stack: Vec::new(),
        ram: vec![0u8; ram_len],
        code,
        fuel_per_touch,
        capabilities,
        pages: Vec::new(),
        signature: None,
    };

    for (addr, bytes) in &ram_init {
        if addr + bytes.len() > capsule.ram.len() {
            return Err(GhostError::AssembleError(format!(
                ".data at offset {} (len {}) overflows RAM ({})",
                addr,
                bytes.len(),
                capsule.ram.len()
            )));
        }
        capsule.ram[*addr..*addr + bytes.len()].copy_from_slice(bytes);
    }

    Ok(capsule)
}

fn no_operands(parts: &[&str], line_no: usize) -> Result<(), GhostError> {
    if parts.len() != 1 {
        return Err(asm_err(line_no, "unexpected operands"));
    }
    Ok(())
}

fn one_i32(parts: &[&str], line_no: usize) -> Result<i32, GhostError> {
    if parts.len() != 2 {
        return Err(asm_err(line_no, "expected one i32 operand"));
    }
    parts[1]
        .parse()
        .map_err(|_| asm_err(line_no, "bad i32"))
}

fn one_u16(parts: &[&str], line_no: usize) -> Result<u16, GhostError> {
    if parts.len() != 2 {
        return Err(asm_err(line_no, "expected one u16 operand"));
    }
    parts[1]
        .parse()
        .map_err(|_| asm_err(line_no, "bad u16"))
}

/// Pretty-print bytecode with offsets (for `dump` / debugging).
pub fn disassemble_code(code: &[u8]) -> String {
    let mut i = 0usize;
    let mut out = String::new();
    while i < code.len() {
        let start = i;
        let op = code[i];
        i += 1;
        let text = match op {
            OP_NOP => "nop".to_string(),
            OP_PUSH => {
                if i + 4 > code.len() {
                    format!("push <truncated@{start}>")
                } else {
                    let imm = i32::from_le_bytes([code[i], code[i + 1], code[i + 2], code[i + 3]]);
                    i += 4;
                    format!("push {imm}")
                }
            }
            OP_POP => "pop".to_string(),
            OP_DUP => "dup".to_string(),
            OP_SWAP => "swap".to_string(),
            OP_ROT => "rot".to_string(),
            OP_ADD => "add".to_string(),
            OP_SUB => "sub".to_string(),
            OP_MUL => "mul".to_string(),
            OP_DIV => "div".to_string(),
            OP_MOD => "mod".to_string(),
            OP_EQ => "eq".to_string(),
            OP_LT => "lt".to_string(),
            OP_GT => "gt".to_string(),
            OP_LOAD => {
                if i + 2 > code.len() {
                    format!("load <truncated@{start}>")
                } else {
                    let u = u16::from_le_bytes([code[i], code[i + 1]]);
                    i += 2;
                    format!("load {u}")
                }
            }
            OP_STORE => {
                if i + 2 > code.len() {
                    format!("store <truncated@{start}>")
                } else {
                    let u = u16::from_le_bytes([code[i], code[i + 1]]);
                    i += 2;
                    format!("store {u}")
                }
            }
            OP_JMP => {
                if i + 2 > code.len() {
                    format!("jmp <truncated@{start}>")
                } else {
                    let r = i16::from_le_bytes([code[i], code[i + 1]]);
                    i += 2;
                    format!("jmp {r}")
                }
            }
            OP_JMPIF => {
                if i + 2 > code.len() {
                    format!("jmpif <truncated@{start}>")
                } else {
                    let r = i16::from_le_bytes([code[i], code[i + 1]]);
                    i += 2;
                    format!("jmpif {r}")
                }
            }
            OP_SYSCALL => {
                if i >= code.len() {
                    format!("syscall <truncated@{start}>")
                } else {
                    let id = code[i];
                    i += 1;
                    format!("syscall {id}")
                }
            }
            OP_FCONST => {
                if i + 8 > code.len() {
                    format!("fconst <truncated@{start}>")
                } else {
                    let lo = u32::from_le_bytes([code[i], code[i+1], code[i+2], code[i+3]]);
                    let hi = u32::from_le_bytes([code[i+4], code[i+5], code[i+6], code[i+7]]);
                    i += 8;
                    let bits = ((hi as u64) << 32) | (lo as u64);
                    let f = f64::from_bits(bits);
                    format!("fconst {f}")
                }
            }
            OP_FADD => "fadd".to_string(),
            OP_FSUB => "fsub".to_string(),
            OP_FMUL => "fmul".to_string(),
            OP_FDIV => "fdiv".to_string(),
            OP_ITOF => "itof".to_string(),
            OP_FTOI => "ftoi".to_string(),
            OP_HALT => "halt".to_string(),
            x => format!(".byte 0x{x:02x}"),
        };
        out.push_str(&format!("{start:04x}: {text}\n"));
    }
    out
}

/// Disassemble a single instruction at `pc` and return human-readable string.
pub fn disassemble_instruction(code: &[u8], pc: usize) -> String {
    if pc >= code.len() { return "<past end>".to_string(); }
    let op = code[pc];
    let i = pc + 1;
    match op {
        OP_NOP => "nop".into(),
        OP_PUSH => {
            if i + 4 > code.len() { "push <trunc>".into() }
            else {
                let v = i32::from_le_bytes([code[i], code[i+1], code[i+2], code[i+3]]);
                format!("push {v}")
            }
        }
        OP_POP => "pop".into(), OP_DUP => "dup".into(),
        OP_SWAP => "swap".into(), OP_ROT => "rot".into(),
        OP_ADD => "add".into(), OP_SUB => "sub".into(),
        OP_MUL => "mul".into(), OP_DIV => "div".into(),
        OP_MOD => "mod".into(), OP_EQ => "eq".into(),
        OP_LT => "lt".into(), OP_GT => "gt".into(),
        OP_LOAD => {
            if i + 2 > code.len() { "load <trunc>".into() }
            else { let u = u16::from_le_bytes([code[i], code[i+1]]); format!("load {u}") }
        }
        OP_STORE => {
            if i + 2 > code.len() { "store <trunc>".into() }
            else { let u = u16::from_le_bytes([code[i], code[i+1]]); format!("store {u}") }
        }
        OP_JMP => {
            if i + 2 > code.len() { "jmp <trunc>".into() }
            else { let r = i16::from_le_bytes([code[i], code[i+1]]); format!("jmp {r}") }
        }
        OP_JMPIF => {
            if i + 2 > code.len() { "jmpif <trunc>".into() }
            else { let r = i16::from_le_bytes([code[i], code[i+1]]); format!("jmpif {r}") }
        }
        OP_SYSCALL => {
            if i >= code.len() { "syscall <trunc>".into() }
            else { format!("syscall {}", code[i]) }
        }
        OP_FCONST => {
            if i + 8 > code.len() { "fconst <trunc>".into() }
            else {
                let lo = u32::from_le_bytes([code[i], code[i+1], code[i+2], code[i+3]]);
                let hi = u32::from_le_bytes([code[i+4], code[i+5], code[i+6], code[i+7]]);
                let f = f64::from_bits(((hi as u64) << 32) | lo as u64);
                format!("fconst {f}")
            }
        }
        OP_FADD => "fadd".into(), OP_FSUB => "fsub".into(),
        OP_FMUL => "fmul".into(), OP_FDIV => "fdiv".into(),
        OP_ITOF => "itof".into(), OP_FTOI => "ftoi".into(),
        OP_HALT => "halt".into(),
        x => format!(".byte 0x{x:02x}"),
    }
}

/// Save capsule recovery state (PC + stack + RAM) to a .recover file.
pub fn save_recovery(capsule: &Capsule, path: &str) -> Result<(), std::io::Error> {
    let mut out = Vec::new();
    out.extend_from_slice(b"GRCV");
    out.extend_from_slice(&capsule.pc.to_le_bytes());
    let sl = capsule.stack.len() as u32;
    out.extend_from_slice(&sl.to_le_bytes());
    for v in &capsule.stack {
        out.extend_from_slice(&v.to_le_bytes());
    }
    let rl = capsule.ram.len() as u32;
    out.extend_from_slice(&rl.to_le_bytes());
    out.extend_from_slice(&capsule.ram);
    std::fs::write(path, &out)
}

/// Load recovery state into a capsule from a .recover file.
pub fn load_recovery(capsule: &mut Capsule, path: &str) -> Result<(), String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    if data.len() < 12 || &data[0..4] != b"GRCV" {
        return Err("invalid recovery file".into());
    }
    let mut i = 4usize;
    capsule.pc = u32::from_le_bytes([data[i], data[i+1], data[i+2], data[i+3]]);
    i += 4;
    let sl = u32::from_le_bytes([data[i], data[i+1], data[i+2], data[i+3]]) as usize;
    i += 4;
    capsule.stack.clear();
    for _ in 0..sl {
        if i + 8 > data.len() { return Err("truncated recovery stack".into()); }
        let v = i64::from_le_bytes([
            data[i], data[i+1], data[i+2], data[i+3],
            data[i+4], data[i+5], data[i+6], data[i+7],
        ]);
        capsule.stack.push(v);
        i += 8;
    }
    if i + 4 > data.len() { return Err("truncated recovery ram len".into()); }
    let rl = u32::from_le_bytes([data[i], data[i+1], data[i+2], data[i+3]]) as usize;
    i += 4;
    if i + rl > data.len() { return Err("truncated recovery ram".into()); }
    capsule.ram = data[i..i+rl].to_vec();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capsule_roundtrip() {
        let mut c = Capsule::default();
        c.stack.push(1);
        c.stack.push(2);
        c.pc = 0;
        c.code = vec![OP_PUSH, 3, 0, 0, 0, OP_HALT];
        let bytes = c.encode().unwrap();
        let c2 = Capsule::decode(&bytes).unwrap();
        assert_eq!(c2.stack, c.stack);
        assert_eq!(c2.pc, c.pc);
        assert_eq!(c2.code, c.code);
    }

    #[test]
    fn run_push_add_halt() {
        let mut c = Capsule {
            code: vec![
                OP_PUSH, 10, 0, 0, 0, OP_PUSH, 32, 0, 0, 0, OP_ADD, OP_HALT,
            ],
            ram: vec![0u8; 16],
            ..Default::default()
        };
        let mut h = NullHost;
        let st = run(&mut c, &mut h, Some(100)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![42]);
    }

    #[test]
    fn syscall_log() {
        let mut c = Capsule {
            code: vec![
                OP_PUSH, 7, 0, 0, 0,
                OP_SYSCALL,
                SYS_HOST_LOG,
                OP_HALT,
            ],
            ..Default::default()
        };
        let mut h = LogHost { logged: Vec::new() };
        let st = run(&mut c, &mut h, Some(50)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(h.logged, vec![7]);
        assert!(c.stack.is_empty());
    }

    #[test]
    fn syscall_denied_when_caps_clear() {
        let mut c = Capsule {
            capabilities: 0,
            code: vec![OP_PUSH, 1, 0, 0, 0, OP_SYSCALL, SYS_HOST_LOG, OP_HALT],
            ..Default::default()
        };
        let mut h = LogHost { logged: Vec::new() };
        let e = run(&mut c, &mut h, Some(20)).unwrap_err();
        assert!(matches!(e, GhostError::SyscallDenied(SYS_HOST_LOG)));
    }

    #[test]
    fn assemble_push_add() {
        let src = r#"
            .ram 16
            push 10
            push 32
            add
            halt
        "#;
        let mut c = assemble_capsule(src).unwrap();
        assert_eq!(c.ram.len(), 16);
        let mut h = NullHost;
        let st = run(&mut c, &mut h, Some(50)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![42]);
    }

    #[test]
    fn disassemble_contains_mnemonics() {
        let src = "push -1\nsyscall 0\nhalt\n";
        let c = assemble_capsule(src).unwrap();
        let d = disassemble_code(&c.code);
        assert!(d.contains("push -1"));
        assert!(d.contains("syscall 0"));
        assert!(d.contains("halt"));
    }

    #[test]
    fn assemble_jmp_label() {
        let src = r#"
            .ram 8
            push 99
            jmp there
        there:
            halt
        "#;
        let mut c = assemble_capsule(src).unwrap();
        let mut h = NullHost;
        let st = run(&mut c, &mut h, Some(100)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![99]);
    }

    #[test]
    fn sign_verify_roundtrip() {
        let mut c = assemble_capsule("halt").unwrap();
        let key = b"ghost-test-key";
        sign_capsule(&mut c, key).unwrap();
        verify_capsule(&c, key).unwrap();
        let bytes = c.encode().unwrap();
        let c2 = Capsule::decode(&bytes).unwrap();
        verify_capsule(&c2, key).unwrap();
        assert!(verify_capsule(&c2, b"wrong").is_err());
    }

    #[test]
    fn decode_rejects_trailing_garbage() {
        let mut c = Capsule::default();
        c.code = vec![OP_HALT];
        let mut v = c.encode().unwrap();
        v.push(0xAB);
        assert!(Capsule::decode(&v).is_err());
    }

    // --- New opcode tests ---

    #[test]
    fn run_div() {
        let mut c = Capsule {
            code: vec![OP_PUSH, 10, 0, 0, 0, OP_PUSH, 3, 0, 0, 0, OP_DIV, OP_HALT],
            ..Default::default()
        };
        let st = run(&mut c, &mut NullHost, Some(50)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![3]); // 10 / 3 = 3
    }

    #[test]
    fn run_div_by_zero_pushes_zero() {
        let mut c = Capsule {
            code: vec![OP_PUSH, 7, 0, 0, 0, OP_PUSH, 0, 0, 0, 0, OP_DIV, OP_HALT],
            ..Default::default()
        };
        let st = run(&mut c, &mut NullHost, Some(50)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![0]);
    }

    #[test]
    fn run_mod() {
        let mut c = Capsule {
            code: vec![OP_PUSH, 10, 0, 0, 0, OP_PUSH, 3, 0, 0, 0, OP_MOD, OP_HALT],
            ..Default::default()
        };
        let st = run(&mut c, &mut NullHost, Some(50)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![1]); // 10 % 3 = 1
    }

    #[test]
    fn run_mod_by_zero_pushes_zero() {
        let mut c = Capsule {
            code: vec![OP_PUSH, 7, 0, 0, 0, OP_PUSH, 0, 0, 0, 0, OP_MOD, OP_HALT],
            ..Default::default()
        };
        let st = run(&mut c, &mut NullHost, Some(50)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![0]);
    }

    #[test]
    fn run_eq() {
        let mut c = Capsule {
            code: vec![
                OP_PUSH, 5, 0, 0, 0, OP_PUSH, 5, 0, 0, 0, OP_EQ,
                OP_PUSH, 5, 0, 0, 0, OP_PUSH, 3, 0, 0, 0, OP_EQ,
                OP_HALT,
            ],
            ..Default::default()
        };
        let st = run(&mut c, &mut NullHost, Some(100)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![1, 0]);
    }

    #[test]
    fn run_lt() {
        let mut c = Capsule {
            code: vec![
                OP_PUSH, 3, 0, 0, 0, OP_PUSH, 5, 0, 0, 0, OP_LT,
                OP_PUSH, 5, 0, 0, 0, OP_PUSH, 3, 0, 0, 0, OP_LT,
                OP_HALT,
            ],
            ..Default::default()
        };
        let st = run(&mut c, &mut NullHost, Some(100)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![1, 0]); // 3<5 → 1, 5<3 → 0
    }

    #[test]
    fn run_gt() {
        let mut c = Capsule {
            code: vec![
                OP_PUSH, 5, 0, 0, 0, OP_PUSH, 3, 0, 0, 0, OP_GT,
                OP_PUSH, 3, 0, 0, 0, OP_PUSH, 5, 0, 0, 0, OP_GT,
                OP_HALT,
            ],
            ..Default::default()
        };
        let st = run(&mut c, &mut NullHost, Some(100)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![1, 0]); // 5>3 → 1, 3>5 → 0
    }

    #[test]
    fn run_swap() {
        let mut c = Capsule {
            code: vec![OP_PUSH, 1, 0, 0, 0, OP_PUSH, 2, 0, 0, 0, OP_SWAP, OP_HALT],
            ..Default::default()
        };
        let st = run(&mut c, &mut NullHost, Some(50)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![2, 1]);
    }

    #[test]
    fn run_rot() {
        let mut c = Capsule {
            code: vec![
                OP_PUSH, 1, 0, 0, 0,
                OP_PUSH, 2, 0, 0, 0,
                OP_PUSH, 3, 0, 0, 0,
                OP_ROT, OP_HALT,
            ],
            ..Default::default()
        };
        let st = run(&mut c, &mut NullHost, Some(50)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![2, 3, 1]); // 1 2 3 → 2 3 1
    }

    // --- New syscall tests ---

    #[test]
    fn syscall_yield_returns_yielded() {
        let mut c = Capsule {
            code: vec![OP_SYSCALL, SYS_YIELD, OP_HALT],
            ..Default::default()
        };
        let mut h = InteractiveHost::new();
        let st = run(&mut c, &mut h, Some(50)).unwrap();
        assert_eq!(st, RunStatus::Yielded);
        // PC should be past the syscall instruction, ready to resume at HALT
        assert_eq!(c.pc, 2);
    }

    #[test]
    fn syscall_yield_resume_then_halt() {
        let mut c = Capsule {
            code: vec![OP_SYSCALL, SYS_YIELD, OP_HALT],
            ..Default::default()
        };
        let mut h = InteractiveHost::new();
        let st = run(&mut c, &mut h, Some(50)).unwrap();
        assert_eq!(st, RunStatus::Yielded);
        let st2 = run(&mut c, &mut h, Some(50)).unwrap();
        assert_eq!(st2, RunStatus::Halted);
    }

    #[test]
    fn syscall_tick() {
        let mut c = Capsule {
            code: vec![OP_SYSCALL, SYS_TICK, OP_HALT],
            ..Default::default()
        };
        let mut h = NullHost;
        let st = run(&mut c, &mut h, Some(100)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack.len(), 1);
        // After 1 instruction (syscall opcode fetch) + 1 byte (id) counted, fuel_left < 100
        assert!(c.stack[0] > 0 && c.stack[0] <= 100);
    }

    #[test]
    fn syscall_stack_depth() {
        let mut c = Capsule {
            code: vec![
                OP_PUSH, 10, 0, 0, 0,
                OP_PUSH, 20, 0, 0, 0,
                OP_SYSCALL, SYS_STACK_DEPTH,
                OP_HALT,
            ],
            ..Default::default()
        };
        let mut h = NullHost;
        let st = run(&mut c, &mut h, Some(50)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![10, 20, 2]); // depth was 2 before push
    }

    #[test]
    fn syscall_print_num() {
        let mut c = Capsule {
            code: vec![OP_PUSH, 42, 0, 0, 0, OP_SYSCALL, SYS_PRINT_NUM, OP_HALT],
            ..Default::default()
        };
        let mut h = InteractiveHost::new();
        let st = run(&mut c, &mut h, Some(50)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(String::from_utf8_lossy(&h.output_buffer), "42\n");
    }

    #[test]
    fn syscall_print_str() {
        let msg = b"hello";
        let mut c = Capsule {
            code: vec![
                OP_PUSH, 0, 0, 0, 0,   // addr = 0
                OP_PUSH, 5, 0, 0, 0,   // len = 5
                OP_SYSCALL, SYS_PRINT_STR, OP_HALT,
            ],
            ..Default::default()
        };
        c.ram[..5].copy_from_slice(msg);
        let mut h = InteractiveHost::new();
        let st = run(&mut c, &mut h, Some(50)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(String::from_utf8_lossy(&h.output_buffer), "hello");
    }

    #[test]
    fn syscall_print_char() {
        let mut c = Capsule {
            code: vec![OP_PUSH, 65, 0, 0, 0, OP_SYSCALL, SYS_PRINT_CHAR, OP_HALT],
            ..Default::default()
        };
        let mut h = InteractiveHost::new();
        let st = run(&mut c, &mut h, Some(50)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(h.output_buffer, vec![b'A']);
    }

    #[test]
    fn syscall_read_line_from_buffer() {
        let mut c = Capsule {
            code: vec![
                OP_PUSH, 0, 1, 0, 0,   // addr = 256
                OP_SYSCALL, SYS_READ_LINE,
                OP_HALT,
            ],
            ..Default::default()
        };
        let mut h = InteractiveHost::new();
        h.input_buffer.push_back("hi there".to_string());
        let st = run(&mut c, &mut h, Some(50)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![8]); // "hi there" = 8 bytes
        assert_eq!(&c.ram[256..264], b"hi there");
    }

    #[test]
    fn syscall_prompt_from_buffer() {
        let mut c = Capsule {
            code: vec![
                OP_PUSH, 0, 2, 0, 0,   // addr = 512
                OP_SYSCALL, SYS_PROMPT,
                OP_HALT,
            ],
            ..Default::default()
        };
        let mut h = InteractiveHost::new();
        h.input_buffer.push_back("test input".to_string());
        let st = run(&mut c, &mut h, Some(50)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![10]); // "test input" = 10 bytes
        assert_eq!(&c.ram[512..522], b"test input");
        // output should contain the prompt
        assert!(h.output_buffer.starts_with(b"ghost> "));
    }

    // --- ram_write_str / ram_read_str tests ---

    #[test]
    fn ram_write_read_str_roundtrip() {
        let mut c = Capsule::default();
        let n = ram_write_str(&mut c, 100, "Ghost VM").unwrap();
        assert_eq!(n, 8);
        let s = ram_read_str(&c, 100, 8).unwrap();
        assert_eq!(s, "Ghost VM");
    }

    #[test]
    fn ram_write_str_out_of_bounds() {
        let mut c = Capsule::with_ram_and_fuel(16, 100);
        let err = ram_write_str(&mut c, 10, "this is too long").unwrap_err();
        assert!(matches!(err, GhostError::RamOutOfBounds));
    }

    #[test]
    fn ram_read_str_out_of_bounds() {
        let c = Capsule::with_ram_and_fuel(16, 100);
        let err = ram_read_str(&c, 10, 10).unwrap_err();
        assert!(matches!(err, GhostError::RamOutOfBounds));
    }

    // --- .data directive tests ---

    #[test]
    fn assemble_data_directive() {
        let src = r#"
            .data 0 "Hello\n"
            halt
        "#;
        let c = assemble_capsule(src).unwrap();
        assert_eq!(&c.ram[0..6], b"Hello\n");
    }

    #[test]
    fn assemble_data_multiple() {
        let src = r#"
            .data 0 "abc"
            .data 100 "xyz"
            halt
        "#;
        let c = assemble_capsule(src).unwrap();
        assert_eq!(&c.ram[0..3], b"abc");
        assert_eq!(&c.ram[100..103], b"xyz");
    }

    #[test]
    fn assemble_data_escapes() {
        let src = r#"
            .data 0 "a\tb\nc\\"
            halt
        "#;
        let c = assemble_capsule(src).unwrap();
        assert_eq!(&c.ram[0..6], b"a\tb\nc\\");
    }

    // --- Assembler mnemonic tests ---

    #[test]
    fn assemble_div_mod() {
        let src = "push 10\npush 3\ndiv\npush 10\npush 3\nmod\nhalt\n";
        let mut c = assemble_capsule(src).unwrap();
        let st = run(&mut c, &mut NullHost, Some(100)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![3, 1]);
    }

    #[test]
    fn assemble_eq_lt_gt() {
        let src = "push 5\npush 5\neq\npush 3\npush 5\nlt\npush 5\npush 3\ngt\nhalt\n";
        let mut c = assemble_capsule(src).unwrap();
        let st = run(&mut c, &mut NullHost, Some(100)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![1, 1, 1]);
    }

    #[test]
    fn assemble_swap_rot() {
        let src = "push 1\npush 2\nswap\npush 3\nrot\nhalt\n";
        let mut c = assemble_capsule(src).unwrap();
        let st = run(&mut c, &mut NullHost, Some(100)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        // push 1, push 2 → [1,2], swap → [2,1], push 3 → [2,1,3], rot → [1,3,2]
        assert_eq!(c.stack, vec![1, 3, 2]);
    }

    #[test]
    fn disassemble_new_opcodes() {
        let src = "swap\nrot\ndiv\nmod\neq\nlt\ngt\nhalt\n";
        let c = assemble_capsule(src).unwrap();
        let d = disassemble_code(&c.code);
        for m in &["swap", "rot", "div", "mod", "eq", "lt", "gt", "halt"] {
            assert!(d.contains(m), "disassembly missing {m}");
        }
    }

    // --- Interactive roundtrip test ---

    #[test]
    fn interactive_print_str_from_data() {
        let src = r#"
            .data 0 "Ghost says hi"
            push 0
            push 13
            syscall 4
            halt
        "#;
        let mut c = assemble_capsule(src).unwrap();
        let mut h = InteractiveHost::new();
        let st = run(&mut c, &mut h, Some(100)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(String::from_utf8_lossy(&h.output_buffer), "Ghost says hi");
    }

    #[test]
    fn interactive_yield_resume_roundtrip() {
        let src = r#"
            .data 0 "before"
            .data 100 "after"
            push 0
            push 6
            syscall 4
            syscall 8
            push 100
            push 5
            syscall 4
            halt
        "#;
        let mut c = assemble_capsule(src).unwrap();
        let mut h = InteractiveHost::new();
        let st = run(&mut c, &mut h, Some(200)).unwrap();
        assert_eq!(st, RunStatus::Yielded);
        assert_eq!(String::from_utf8_lossy(&h.output_buffer), "before");

        let st2 = run(&mut c, &mut h, Some(200)).unwrap();
        assert_eq!(st2, RunStatus::Halted);
        assert_eq!(String::from_utf8_lossy(&h.output_buffer), "beforeafter");
    }

    // === PHASE 1: Code introspection tests ===

    #[test]
    fn syscall_code_len() {
        let code = vec![
            OP_SYSCALL, SYS_CODE_LEN,
            OP_HALT,
        ];
        let code_len = code.len() as i64;
        let mut c = Capsule { code, ..Default::default() };
        let mut h = InteractiveHost::new();
        let st = run(&mut c, &mut h, Some(50)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![code_len]);
    }

    #[test]
    fn syscall_code_read() {
        let mut c = Capsule {
            code: vec![
                OP_PUSH, 0, 0, 0, 0,       // push offset 0
                OP_SYSCALL, SYS_CODE_READ,
                OP_HALT,
            ],
            ..Default::default()
        };
        let mut h = InteractiveHost::new();
        let st = run(&mut c, &mut h, Some(50)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![OP_PUSH as i64]);
    }

    #[test]
    fn syscall_code_read_out_of_bounds() {
        let mut c = Capsule {
            code: vec![
                OP_PUSH, 0xFF, 0xFF, 0, 0, // push 65535
                OP_SYSCALL, SYS_CODE_READ,
                OP_HALT,
            ],
            ..Default::default()
        };
        let mut h = InteractiveHost::new();
        let st = run(&mut c, &mut h, Some(50)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![-1]); // out of bounds → -1
    }

    #[test]
    fn syscall_code_write_self_modify() {
        // Layout: push 99 (val), push 12 (offset), syscall CODE_WRITE, <target>, halt
        // <target> is at code offset 12 — we'll place PUSH 0 there initially,
        // then the CODE_WRITE overwrites byte 13 (the immediate) to 42.
        // After the write, execution continues and pushes 42.
        let mut c = Capsule {
            code: vec![
                OP_PUSH, 42, 0, 0, 0,      // value to write (42)
                OP_PUSH, 13, 0, 0, 0,      // offset to write (byte 13 = the first imm byte of the next push)
                OP_SYSCALL, SYS_CODE_WRITE, // writes 42 to code[13]
                OP_PUSH, 0, 0, 0, 0,       // offset 12: push 0 — but byte 13 gets overwritten to 42
                OP_HALT,                    // offset 17
            ],
            ..Default::default()
        };
        let mut h = InteractiveHost::new();
        let st = run(&mut c, &mut h, Some(100)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        // Stack: [1 (success from CODE_WRITE), 42 (the modified push)]
        assert_eq!(c.stack, vec![1, 42]);
    }

    #[test]
    fn syscall_code_write_denied_without_capability() {
        let mut c = Capsule {
            capabilities: CAPABILITIES_ALL & !CAPABILITY_SELF_MODIFY,
            code: vec![
                OP_PUSH, 0, 0, 0, 0,
                OP_PUSH, 0, 0, 0, 0,
                OP_SYSCALL, SYS_CODE_WRITE,
                OP_HALT,
            ],
            ..Default::default()
        };
        let mut h = InteractiveHost::new();
        let e = run(&mut c, &mut h, Some(50)).unwrap_err();
        assert!(matches!(e, GhostError::SyscallDenied(SYS_CODE_WRITE)));
    }

    #[test]
    fn syscall_ram_len() {
        let mut c = Capsule {
            code: vec![OP_SYSCALL, SYS_RAM_LEN, OP_HALT],
            ram: vec![0u8; 1024],
            ..Default::default()
        };
        let mut h = InteractiveHost::new();
        let st = run(&mut c, &mut h, Some(50)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![1024]);
    }

    #[test]
    fn syscall_pc() {
        let mut c = Capsule {
            code: vec![
                OP_NOP,                    // offset 0, pc becomes 1
                OP_SYSCALL, SYS_PC,        // offset 1, after reading id pc=3
                OP_HALT,                   // offset 3
            ],
            ..Default::default()
        };
        let mut h = InteractiveHost::new();
        let st = run(&mut c, &mut h, Some(50)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![3]); // PC is 3 after consuming the syscall operand
    }

    // === PHASE 2: Spawn tests ===

    #[test]
    fn spawn_init_and_run() {
        // Parent writes "push 42; halt" into RAM, spawns child, runs it
        let child_code: Vec<u8> = vec![OP_PUSH, 42, 0, 0, 0, OP_HALT];
        let code_len = child_code.len();
        let mut c = Capsule {
            code: vec![
                // push code_len (6)
                OP_PUSH, code_len as u8, 0, 0, 0,
                // push ram_addr (0)
                OP_PUSH, 0, 0, 0, 0,
                OP_SYSCALL, SYS_SPAWN_INIT,    // → child_index on stack
                // push fuel (100) — SPAWN_RUN pops fuel first, then child_index
                OP_PUSH, 100, 0, 0, 0,
                OP_SYSCALL, SYS_SPAWN_RUN,     // → child's TOS (42)
                OP_HALT,
            ],
            ..Default::default()
        };
        c.ram[..code_len].copy_from_slice(&child_code);
        let mut h = InteractiveHost::new();
        let st = run(&mut c, &mut h, Some(200)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(*c.stack.last().unwrap(), 42);
    }

    #[test]
    fn spawn_write_read_child_ram() {
        let mut c = Capsule {
            code: vec![
                // push code_len (1), push ram_addr (0) → spawn
                OP_PUSH, 1, 0, 0, 0,
                OP_PUSH, 0, 0, 0, 0,
                OP_SYSCALL, SYS_SPAWN_INIT,     // child_index=0
                // dup child_index for later reads
                OP_DUP,
                OP_DUP,
                // SYS_SPAWN_WRITE: pop val, pop offset, pop child_index
                // push child_index (already on stack from dup)
                OP_PUSH, 0, 0, 0, 0,       // offset = 0
                OP_PUSH, 99, 0, 0, 0,      // value = 99
                OP_SYSCALL, SYS_SPAWN_WRITE,
                // SYS_SPAWN_READ: pop offset, pop child_index
                OP_PUSH, 0, 0, 0, 0,       // offset = 0
                OP_SYSCALL, SYS_SPAWN_READ,
                OP_HALT,
            ],
            ..Default::default()
        };
        c.ram[0] = OP_HALT;
        let mut h = InteractiveHost::new();
        let st = run(&mut c, &mut h, Some(200)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(*c.stack.last().unwrap(), 99);
    }

    #[test]
    fn spawn_kill() {
        let mut c = Capsule {
            code: vec![
                OP_PUSH, 1, 0, 0, 0,
                OP_PUSH, 0, 0, 0, 0,
                OP_SYSCALL, SYS_SPAWN_INIT,
                OP_SYSCALL, SYS_SPAWN_KILL,     // kill child 0
                OP_SYSCALL, SYS_SPAWN_COUNT,
                OP_HALT,
            ],
            ..Default::default()
        };
        c.ram[0] = OP_HALT;
        let mut h = InteractiveHost::new();
        let st = run(&mut c, &mut h, Some(200)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        // spawn_kill pushes 1, spawn_count pushes 0 (child was killed)
        assert_eq!(*c.stack.last().unwrap(), 0);
    }

    #[test]
    fn spawn_count() {
        let mut c = Capsule {
            code: vec![
                // Spawn child 1
                OP_PUSH, 1, 0, 0, 0,
                OP_PUSH, 0, 0, 0, 0,
                OP_SYSCALL, SYS_SPAWN_INIT,
                OP_POP,
                // Spawn child 2
                OP_PUSH, 1, 0, 0, 0,
                OP_PUSH, 0, 0, 0, 0,
                OP_SYSCALL, SYS_SPAWN_INIT,
                OP_POP,
                OP_SYSCALL, SYS_SPAWN_COUNT,
                OP_HALT,
            ],
            ..Default::default()
        };
        c.ram[0] = OP_HALT;
        let mut h = InteractiveHost::new();
        let st = run(&mut c, &mut h, Some(300)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![2]);
    }

    // === PHASE 3: Evolution tests ===

    #[test]
    fn syscall_random_produces_different_values() {
        let mut c = Capsule {
            code: vec![
                OP_SYSCALL, SYS_RANDOM,
                OP_SYSCALL, SYS_RANDOM,
                OP_SYSCALL, SYS_RANDOM,
                OP_HALT,
            ],
            ..Default::default()
        };
        let mut h = InteractiveHost::new();
        let st = run(&mut c, &mut h, Some(100)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack.len(), 3);
        // At least two of the three values should differ
        assert!(c.stack[0] != c.stack[1] || c.stack[1] != c.stack[2],
            "RNG produced identical values: {:?}", c.stack);
    }

    #[test]
    fn syscall_copy_code() {
        let code = vec![
            OP_PUSH, 5, 0, 0, 0,       // length = 5
            OP_PUSH, 0, 0, 0, 0,       // src_offset = 0
            OP_PUSH, 0, 1, 0, 0,       // dest_ram = 256
            OP_SYSCALL, SYS_COPY_CODE,
            OP_HALT,
        ];
        let expected_first5 = code[0..5].to_vec();
        let mut c = Capsule { code, ..Default::default() };
        let mut h = InteractiveHost::new();
        let st = run(&mut c, &mut h, Some(100)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![5]); // 5 bytes copied
        assert_eq!(&c.ram[256..261], &expected_first5);
    }

    #[test]
    fn syscall_hash_consistent() {
        let mut c = Capsule {
            code: vec![
                OP_PUSH, 5, 0, 0, 0,       // length = 5
                OP_PUSH, 0, 0, 0, 0,       // addr = 0
                OP_SYSCALL, SYS_HASH,
                OP_PUSH, 5, 0, 0, 0,
                OP_PUSH, 0, 0, 0, 0,
                OP_SYSCALL, SYS_HASH,
                OP_HALT,
            ],
            ..Default::default()
        };
        c.ram[0..5].copy_from_slice(b"hello");
        let mut h = InteractiveHost::new();
        let st = run(&mut c, &mut h, Some(100)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack.len(), 2);
        assert_eq!(c.stack[0], c.stack[1]); // same input → same hash
        assert_ne!(c.stack[0], 0);
    }

    #[test]
    fn self_modify_overwrites_upcoming_code() {
        // Ghost writes PUSH 99 + HALT over its own upcoming bytes, then executes them.
        // Code layout:
        //  0: push 0x01           (OP_PUSH = value to write at code[20])
        //  5: push 20             (offset 20)
        //  10: syscall CODE_WRITE (write OP_PUSH at offset 20) → pushes 1
        //  12: pop                (discard success flag)
        //  13: push 99            (value 99 to write at offset 21)
        //  18: <placeholder — will be overwritten>
        //  But this is getting complex. Simpler: overwrite a single byte.
        //
        // Simplest: write OP_PUSH immediate.
        // offset 0..5 = push 42; offset 5..10 = push 18; offset 10..12 = syscall CODE_WRITE;
        // offset 12 = pop; offset 13..18 = push 0; offset 18 = halt
        // We overwrite code[14] (the first imm byte of the second push) from 0 to 42.
        let mut c = Capsule {
            code: vec![
                OP_PUSH, 42, 0, 0, 0,      // 0-4: value=42
                OP_PUSH, 14, 0, 0, 0,      // 5-9: offset=14 (first imm byte of target push)
                OP_SYSCALL, SYS_CODE_WRITE, // 10-11: write code[14]=42
                OP_POP,                     // 12: discard success flag
                OP_PUSH, 0, 0, 0, 0,       // 13-17: initially pushes 0 (byte 14 will be 42 after write)
                OP_HALT,                    // 18
            ],
            ..Default::default()
        };
        let mut h = InteractiveHost::new();
        let st = run(&mut c, &mut h, Some(100)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![42]); // the modified push now pushes 42 instead of 0
    }

    #[test]
    fn spawn_run_child_computes() {
        // Parent creates child that computes 3+4=7, gets result back
        let child_code: Vec<u8> = vec![
            OP_PUSH, 3, 0, 0, 0,
            OP_PUSH, 4, 0, 0, 0,
            OP_ADD,
            OP_HALT,
        ];
        let cl = child_code.len();
        let mut c = Capsule {
            code: vec![
                OP_PUSH, cl as u8, 0, 0, 0,
                OP_PUSH, 0, 0, 0, 0,
                OP_SYSCALL, SYS_SPAWN_INIT,
                OP_PUSH, 200, 0, 0, 0,     // fuel=200
                OP_SYSCALL, SYS_SPAWN_RUN,
                OP_HALT,
            ],
            ..Default::default()
        };
        c.ram[..cl].copy_from_slice(&child_code);
        let mut h = InteractiveHost::new();
        let st = run(&mut c, &mut h, Some(500)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(*c.stack.last().unwrap(), 7);
    }

    // === v3.0 Float opcode tests ===

    #[test]
    fn test_float_basic() {
        let src = "fconst 3.14\nfconst 2.0\nfmul\nhalt\n";
        let mut c = assemble_capsule(src).unwrap();
        let st = run(&mut c, &mut NullHost, Some(50)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        let bits = *c.stack.last().unwrap() as u64;
        let result = f64::from_bits(bits);
        assert!((result - 6.28).abs() < 0.001);
    }

    #[test]
    fn test_float_itof() {
        let src = "push 42\nitof\nfconst 2.0\nfmul\nhalt\n";
        let mut c = assemble_capsule(src).unwrap();
        let st = run(&mut c, &mut NullHost, Some(50)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        let bits = *c.stack.last().unwrap() as u64;
        let result = f64::from_bits(bits);
        assert!((result - 84.0).abs() < 0.001);
    }

    #[test]
    fn test_float_ftoi() {
        let src = "fconst 9.99\nftoi\nhalt\n";
        let mut c = assemble_capsule(src).unwrap();
        let st = run(&mut c, &mut NullHost, Some(50)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        assert_eq!(c.stack, vec![9]);
    }

    #[test]
    fn test_float_fdiv_zero() {
        let src = "fconst 1.0\nfconst 0.0\nfdiv\nhalt\n";
        let mut c = assemble_capsule(src).unwrap();
        let st = run(&mut c, &mut NullHost, Some(50)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        let bits = *c.stack.last().unwrap() as u64;
        assert!(f64::from_bits(bits).is_nan());
    }

    #[test]
    fn test_fprint() {
        let src = "fconst 3.14\nsyscall 84\nhalt\n";
        let mut c = assemble_capsule(src).unwrap();
        let mut h = InteractiveHost::new();
        let st = run(&mut c, &mut h, Some(50)).unwrap();
        assert_eq!(st, RunStatus::Halted);
        let out = String::from_utf8_lossy(&h.output_buffer);
        assert!(out.contains("3.14"));
    }

    // === Ctrl+C interrupt test ===

    #[test]
    fn test_ctrlc_interrupt() {
        let src = "loop: push 1\npop\njmp loop\n";
        let mut c = assemble_capsule(src).unwrap();
        let flag = std::sync::atomic::AtomicBool::new(true);
        let mut h = NullHost;
        let err = run_ex(&mut c, &mut h, Some(100_000), Some(&flag), DebugMode::None).unwrap_err();
        assert!(matches!(err, GhostError::Interrupted));
    }

    #[test]
    fn test_disassemble_instruction() {
        let code = vec![OP_PUSH, 42, 0, 0, 0, OP_ADD, OP_HALT];
        assert_eq!(disassemble_instruction(&code, 0), "push 42");
        assert_eq!(disassemble_instruction(&code, 5), "add");
        assert_eq!(disassemble_instruction(&code, 6), "halt");
    }

    #[test]
    fn test_capsule_v3_roundtrip() {
        let mut c = Capsule::default();
        assert_eq!(c.format_version, FORMAT_VERSION_V3);
        c.stack.push(42);
        c.stack.push(i64::MAX);
        let bytes = c.encode().unwrap();
        let c2 = Capsule::decode(&bytes).unwrap();
        assert_eq!(c2.stack, vec![42i64, i64::MAX]);
        assert_eq!(c2.format_version, FORMAT_VERSION_V3);
    }
}
