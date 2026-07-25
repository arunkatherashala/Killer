#![allow(unsafe_code)]
use crate::value::Value;
use crate::error::VmError;

// â”€â”€ FFI for mmap / VirtualAlloc â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
#[cfg(target_os = "windows")]
extern "system" {
    fn VirtualAlloc(lpAddress: *mut std::ffi::c_void, dwSize: usize, flAllocationType: u32, flProtect: u32) -> *mut std::ffi::c_void;
    fn VirtualFree(lpAddress: *mut std::ffi::c_void, dwSize: usize, dwFreeType: u32) -> i32;
    fn VirtualProtect(lpAddress: *mut std::ffi::c_void, dwSize: usize, flNewProtect: u32, lpflOldProtect: *mut u32) -> i32;
}
#[cfg(target_os = "windows")]
unsafe fn winapi_virtual_alloc(addr: *mut u8, size: usize, alloc_type: u32, protect: u32) -> *mut u8 {
    unsafe { VirtualAlloc(addr as *mut std::ffi::c_void, size, alloc_type, protect) as *mut u8 }
}
#[cfg(target_os = "windows")]
unsafe fn winapi_virtual_free(addr: *mut u8, size: usize, free_type: u32) -> i32 {
    unsafe { VirtualFree(addr as *mut std::ffi::c_void, size, free_type) }
}
#[cfg(target_os = "windows")]
unsafe fn winapi_virtual_protect(addr: *mut u8, size: usize, protect: u32, old: *mut u32) -> i32 {
    unsafe { VirtualProtect(addr as *mut std::ffi::c_void, size, protect, old) }
}

#[cfg(not(target_os = "windows"))]
extern "C" {
    fn mmap(addr: *mut u8, length: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut u8;
    fn munmap(addr: *mut u8, length: usize) -> i32;
    fn mprotect(addr: *mut u8, length: usize, prot: i32) -> i32;
}
#[cfg(not(target_os = "windows"))]
unsafe fn libc_mmap(addr: *mut u8, length: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut u8 {
    unsafe { mmap(addr, length, prot, flags, fd, offset) }
}
#[cfg(not(target_os = "windows"))]
unsafe fn libc_munmap(addr: *mut u8, length: usize) -> i32 {
    unsafe { munmap(addr, length) }
}
#[cfg(not(target_os = "windows"))]
unsafe fn libc_mprotect(addr: *mut u8, length: usize, prot: i32) -> i32 {
    unsafe { mprotect(addr, length, prot) }
}

// â”€â”€ Pure Rust SHA-256 (zero external deps) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
fn sha256_digest(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
        0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
        0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
        0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
        0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
        0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
        0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
        0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    // Padding
    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 { msg.push(0); }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    // Process 512-bit blocks
    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[i*4], chunk[i*4+1], chunk[i*4+2], chunk[i*4+3]]);
        }
        for i in 16..64 {
            let s0 = w[i-15].rotate_right(7) ^ w[i-15].rotate_right(18) ^ (w[i-15] >> 3);
            let s1 = w[i-2].rotate_right(17) ^ w[i-2].rotate_right(19) ^ (w[i-2] >> 10);
            w[i] = w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
        }
        let [mut a,mut b,mut c,mut d,mut e,mut f,mut g,mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g; g = f; f = e; e = d.wrapping_add(t1);
            d = c; c = b; b = a; a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a); h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c); h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e); h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g); h[7] = h[7].wrapping_add(hh);
    }
    let mut out = [0u8; 32];
    for i in 0..8 {
        out[i*4..i*4+4].copy_from_slice(&h[i].to_be_bytes());
    }
    out
}

fn is_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => *n != 0.0 && !n.is_nan(),
        Value::Str(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        _ => true,
    }
}

pub const BUILTIN_ID_TABLE: &[(&str, u16)] = &[
    ("len",          0), ("length",       0),
    ("str",          1), ("int",          2), ("type",         3),
    ("push",         4), ("pop",          5), ("reverse",      6),
    ("join",         7), ("slice",        8), ("concat",       9),
    ("sorted",      10), ("sum",         11), ("enumerate",   12),
    ("all",         13), ("any",         14), ("zip",         15),
    ("sqrt",        16), ("pow",         17), ("abs",         18),
    ("floor",       19), ("ceil",        20), ("round",       21),
    ("min",         22), ("max",         23), ("sin",         24),
    ("cos",         25), ("tan",         26), ("random",      27),
    ("keys",        28), ("values",      29), ("entries",     30),
    ("upper",       31), ("lower",       32), ("trim",        33),
    ("split",       34), ("contains",    35), ("range",       36),
    ("starts_with", 37), ("ends_with",   38), ("index_of",   39),
    ("includes",    40), ("copy",        41), ("map",         42),
    ("filter",      43), ("reduce",      44), ("sort",        45),
    ("charAt",      46), ("charCodeAt",  47), ("replace",     48),
    ("println",     49), ("print",       49),
    ("get",         50), ("setdefault",  51), ("reversed",    52),
    ("bit_and",     53), ("bit_or",      54), ("bit_xor",     55),
    ("system_time_ms", 56),
    ("readline",    57), ("readline_prompt", 58),
    // v2.3: OS-level builtins
    ("bit_not",     59), ("bit_rotl",       60), ("bit_rotr",       61),
    ("to_integer",  62), ("to_bytes",       63), ("to_pointer",     64),
    ("bytes_new",   65), ("bytes_len",      66), ("bytes_get",      67),
    ("bytes_set",   68), ("bytes_slice",    69), ("bytes_from_str", 70),
    ("bytes_to_str",71), ("bytes_concat",   72), ("bytes_fill",     73),
    ("ptr_new",     74), ("ptr_to_int",     75), ("ptr_offset",     76),
    ("mem_read_u8", 77), ("mem_read_u16",   78), ("mem_read_u32",   79),
    ("mem_read_u64",80), ("mem_write_u8",   81), ("mem_write_u16",  82),
    ("mem_write_u32",83),("mem_write_u64",  84),
    ("volatile_read_u8", 85), ("volatile_read_u16", 86),
    ("volatile_read_u32",87), ("volatile_read_u64",88),
    ("volatile_write_u8",89), ("volatile_write_u16",90),
    ("volatile_write_u32",91),("volatile_write_u64",92),
    ("io_port_in_u8",93), ("io_port_in_u16",94),
    ("io_port_out_u8",95),("io_port_out_u16",96),
    ("sha256",      97), ("sha256_bytes",   98),
    ("mmap_alloc",  99), ("mmap_free",     100), ("mmap_write",    101),
    ("mmap_read",  102), ("mmap_exec",     103),
    ("int_to_bytes_le", 104), ("int_to_bytes_be", 105),
    ("bytes_to_int_le", 106), ("bytes_to_int_be", 107),
    ("cli_args",   108), ("env_get",       109), ("env_set",       110),
    ("process_exit",111), ("errno",        112),
    ("sizeof",     113), ("alignof",      114),
    ("atomic_load",115), ("atomic_store",  116),
    ("atomic_cas",117),  ("atomic_add",    118),
    ("interrupts_disable",119), ("interrupts_enable",120),
    ("wfi",        121), ("fence",         122),
    ("disk_read_block",123), ("disk_write_block",124),
    ("page_alloc", 125), ("page_free",     126), ("page_map",127),
    // v2.3: OS hardware primitives
    ("cpuid",      128), ("rdtsc",         129),
    ("gdt_encode", 130), ("idt_encode",    131), ("call_native", 132),
];

pub fn builtin_name_to_id(name: &str) -> Option<u16> {
    BUILTIN_ID_TABLE.iter().find(|(n, _)| *n == name).map(|(_, id)| *id)
}

/// Builtin function handler for the Killer VM
pub struct BuiltinFunctions;

impl BuiltinFunctions {
    /// Fast dispatch by numeric ID (no string matching).
    pub fn call_by_id(id: u16, args: &[Value]) -> Result<Value, VmError> {
        match id {
            0  => Self::len(args),
            1  => Self::str(args),
            2  => Self::int(args),
            3  => Self::type_of(args),
            4  => Self::push(args),
            5  => Self::pop(args),
            6  => Self::reverse(args),
            7  => Self::join(args),
            8  => Self::slice(args),
            9  => Self::concat(args),
            10 => Self::sorted(args),
            11 => Self::array_sum(args),
            12 => Self::enumerate(args),
            13 => Self::array_all(args),
            14 => Self::array_any(args),
            15 => Self::zip_arrays(args),
            16 => Self::sqrt(args),
            17 => Self::pow(args),
            18 => Self::abs(args),
            19 => Self::floor(args),
            20 => Self::ceil(args),
            21 => Self::round(args),
            22 => Self::min(args),
            23 => Self::max(args),
            24 => Self::sin(args),
            25 => Self::cos(args),
            26 => Self::tan(args),
            27 => Self::random(args),
            28 => Self::keys(args),
            29 => Self::values(args),
            30 => Self::entries(args),
            31 => Self::upper(args),
            32 => Self::lower(args),
            33 => Self::trim(args),
            34 => Self::split(args),
            35 => Self::contains(args),
            36 => Self::range(args),
            37 => Self::starts_with(args),
            38 => Self::ends_with(args),
            39 => Self::index_of(args),
            40 => Self::includes(args),
            41 => Self::value_copy(args),
            42 => Self::array_map(args),
            43 => Self::array_filter(args),
            44 => Self::array_reduce(args),
            45 => Self::sorted(args),
            46 => Self::char_at(args),
            47 => Self::char_code_at(args),
            48 => Self::replace(args),
            49 => { // println/print
                let s = if args.is_empty() { String::new() } else {
                    args.iter().map(|v| format!("{}", v)).collect::<Vec<_>>().join(" ")
                };
                println!("{}", s);
                Ok(Value::Null)
            }
            50 => Self::dict_get(args),
            51 => Self::dict_setdefault(args),
            52 => Self::reverse(args),
            53 => Self::bit_and(args),
            54 => Self::bit_or(args),
            55 => Self::bit_xor(args),
            56 => Self::system_time_ms(args),
            57 => Self::readline(args),
            58 => Self::readline_prompt(args),
            // v2.3: OS-level builtins
            59  => Self::bit_not(args),
            60  => Self::bit_rotl(args),
            61  => Self::bit_rotr(args),
            62  => Self::to_integer(args),
            63  => Self::to_bytes(args),
            64  => Self::to_pointer(args),
            65  => Self::bytes_new(args),
            66  => Self::bytes_len(args),
            67  => Self::bytes_get(args),
            68  => Self::bytes_set(args),
            69  => Self::bytes_slice(args),
            70  => Self::bytes_from_str(args),
            71  => Self::bytes_to_str(args),
            72  => Self::bytes_concat(args),
            73  => Self::bytes_fill(args),
            74  => Self::ptr_new(args),
            75  => Self::ptr_to_int(args),
            76  => Self::ptr_offset(args),
            77  => Self::mem_read_u8(args),
            78  => Self::mem_read_u16(args),
            79  => Self::mem_read_u32(args),
            80  => Self::mem_read_u64(args),
            81  => Self::mem_write_u8(args),
            82  => Self::mem_write_u16(args),
            83  => Self::mem_write_u32(args),
            84  => Self::mem_write_u64(args),
            85  => Self::volatile_read_u8(args),
            86  => Self::volatile_read_u16(args),
            87  => Self::volatile_read_u32(args),
            88  => Self::volatile_read_u64(args),
            89  => Self::volatile_write_u8(args),
            90  => Self::volatile_write_u16(args),
            91  => Self::volatile_write_u32(args),
            92  => Self::volatile_write_u64(args),
            93  => Self::io_port_in_u8(args),
            94  => Self::io_port_in_u16(args),
            95  => Self::io_port_out_u8(args),
            96  => Self::io_port_out_u16(args),
            97  => Self::sha256(args),
            98  => Self::sha256_bytes(args),
            99  => Self::mmap_alloc(args),
            100 => Self::mmap_free(args),
            101 => Self::mmap_write(args),
            102 => Self::mmap_read(args),
            103 => Self::mmap_exec(args),
            104 => Self::int_to_bytes_le(args),
            105 => Self::int_to_bytes_be(args),
            106 => Self::bytes_to_int_le(args),
            107 => Self::bytes_to_int_be(args),
            108 => Self::cli_args(args),
            109 => Self::env_get(args),
            110 => Self::env_set(args),
            111 => Self::process_exit(args),
            112 => Self::errno(args),
            113 => Self::sizeof_val(args),
            114 => Self::alignof_val(args),
            115 => Self::atomic_load(args),
            116 => Self::atomic_store(args),
            117 => Self::atomic_cas(args),
            118 => Self::atomic_add(args),
            119 => Self::interrupts_disable(args),
            120 => Self::interrupts_enable(args),
            121 => Self::wfi(args),
            122 => Self::fence(args),
            123 => Self::disk_read_block(args),
            124 => Self::disk_write_block(args),
            125 => Self::page_alloc(args),
            126 => Self::page_free(args),
            127 => Self::page_map(args),
            // v2.3: OS hardware primitives
            128 => Self::cpuid(args),
            129 => Self::rdtsc(args),
            130 => Self::gdt_encode(args),
            131 => Self::idt_encode(args),
            132 => Self::call_native(args),
            _  => Err(VmError::runtime_error(format!("unknown builtin id {id}"))),
        }
    }

    /// Call a builtin function by name
    pub fn call(name: &str, args: &[Value]) -> Result<Value, VmError> {
        match name {
            // Length/Count functions
            "len" | "length" => Self::len(args),
            "range" => Self::range(args),

            // Print functions â€” println(x) prints with newline, same as VM Print instruction
            "println" | "print" => {
                let s = if args.is_empty() {
                    String::new()
                } else {
                    args.iter()
                        .map(|v| format!("{}", v))
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                println!("{}", s);
                Ok(Value::Null)
            }
            
            // Type functions
            "type" => Self::type_of(args),
            "str" => Self::str(args),
            "int" => Self::int(args),
            
            // Dictionary functions
            "keys" => Self::keys(args),
            "values" => Self::values(args),
            "iterKeys" => Self::iter_keys(args),
            "entries" => Self::entries(args),
            
            // String methods (function form)
            "upper" => Self::upper(args),
            "lower" => Self::lower(args),
            "trim" => Self::trim(args),
            "split" => Self::split(args),
            "starts_with" => Self::starts_with(args),
            "ends_with" => Self::ends_with(args),
            "contains" => Self::contains(args),
            "replace" => Self::replace(args),
            "substring" => Self::substring(args),
            "indexOf" => Self::index_of(args),
            "repeat" => Self::repeat(args),
            
            // Array methods (function form)
            "push" => Self::push(args),
            "pop" => Self::pop(args),
            "reverse" => Self::reverse(args),
            "reversed" => Self::reverse(args),
            "copy" => Self::value_copy(args),
            "join" => Self::join(args),
            "slice" => Self::slice(args),
            "concat" => Self::concat(args),
            "index_of" => Self::index_of(args),
            "includes" => Self::includes(args),
            // Python-style sequence helpers (non-mutating where applicable)
            "sorted" => Self::sorted(args),
            "sum" => Self::array_sum(args),
            "enumerate" => Self::enumerate(args),
            "all" => Self::array_all(args),
            "any" => Self::array_any(args),
            "zip" => Self::zip_arrays(args),
            "map" => Self::array_map(args),
            "filter" => Self::array_filter(args),
            "reduce" => Self::array_reduce(args),
            "sort" => Self::sorted(args),
            
            // String character access
            "charAt" => Self::char_at(args),
            "charCodeAt" => Self::char_code_at(args),
            
            // Math functions
            "sqrt" => Self::sqrt(args),
            "pow" => Self::pow(args),
            "bit_and" => Self::bit_and(args),
            "bit_or" => Self::bit_or(args),
            "bit_xor" => Self::bit_xor(args),
            "bit_shl" => Self::bit_shl(args),
            "bit_shr" => Self::bit_shr(args),
            "abs" => Self::abs(args),
            "floor" => Self::floor(args),
            "ceil" => Self::ceil(args),
            "round" => Self::round(args),
            "min" => Self::min(args),
            "max" => Self::max(args),
            "sin" => Self::sin(args),
            "cos" => Self::cos(args),
            "tan" => Self::tan(args),
            "random" => Self::random(args),
            
            // Type conversions
            "parseInt" => Self::parse_int(args),
            "parseFloat" => Self::parse_float(args),
            "String" => Self::string_convert(args),
            "Number" => Self::number_convert(args),
            "Boolean" => Self::boolean_convert(args),
            
            // Type checks
            "isNaN" => Self::is_nan(args),
            "isFinite" => Self::is_finite(args),
            
            // File I/O
            "readFile" => Self::read_file(args),
            "writeFile" => Self::write_file(args),
            
            // Timing functions (Week 1: Curriculum Support)
            "system_time_ms" | "time_ms" => Self::system_time_ms(args),
            "thread_sleep_ms" | "sleep_ms" => Self::thread_sleep_ms(args),
            
            // Network functions (Week 2: Curriculum Support)
            "TcpListener_bind" => Self::tcp_listener_bind(args),
            "TcpListener_accept" => Self::tcp_listener_accept(args),
            "TcpStream_read" => Self::tcp_stream_read(args),
            "TcpStream_write" => Self::tcp_stream_write(args),
            "TcpStream_close" => Self::tcp_stream_close(args),
            
            // Threading functions (Week 3: Curriculum Support)
            "spawn_thread" => Self::spawn_thread(args),
            "join_thread" => Self::join_thread(args),
            
            // Async functions (Week 4: Curriculum Support)
            "async_spawn" => Self::async_spawn(args),
            "async_await" => Self::async_await(args),
            
            // DateTime functions (Week 23: System Time, Formatting, Parsing)
            "now" => Self::now(args),
            "parse_datetime" => Self::parse_datetime(args),
            "format_datetime" => Self::format_datetime(args),
            
            // HTTP functions -- routed to Nova Galaxy Engine HTTP Client (HTTPS-only, secure)
            "http_get"  => crate::http_client::builtin_http_get(args),
            "http_post" => crate::http_client::builtin_http_post(args),

            // Assert builtins -- test assertions, panic with clear message on failure
            "assert_eq"       => Self::assert_eq(args),
            "assert_ne"       => Self::assert_ne(args),
            "assert_true"     => Self::assert_true(args),
            "assert_false"    => Self::assert_false(args),
            "assert_contains" => Self::assert_contains(args),
            "assert_nil"      => Self::assert_nil(args),

            // -- Phase 1: Trit (balanced ternary) -----------------------------
            "T_NEG"         => Ok(Value::Trit(-1)),
            "T_ZERO"        => Ok(Value::Trit(0)),
            "T_POS"         => Ok(Value::Trit(1)),
            "trit_and"      => Self::trit_and(args),
            "trit_or"       => Self::trit_or(args),
            "trit_not"      => Self::trit_not(args),
            "trit_add"      => Self::trit_add(args),
            "trit_mul"      => Self::trit_mul(args),
            "trit_to_int"   => Self::trit_to_int(args),
            "trit_from_int" => Self::trit_from_int(args),
            "trit_to_str"   => Self::trit_to_str(args),
            "trit_word"     => Self::trit_word(args),
            "trit_word_to_int" => Self::trit_word_to_int(args),
            "int_to_trit"   => Self::trit_from_int(args),  // alias

            // -- Phase 2: Fuzzy logic operators -------------------------------
            "fuzzy_and"       => Self::fuzzy_and(args),
            "fuzzy_or"        => Self::fuzzy_or(args),
            "fuzzy_not"       => Self::fuzzy_not(args),
            "fuzzy_threshold" => Self::fuzzy_threshold(args),
            "fuzzy_combine"   => Self::fuzzy_combine(args),

            // -- Phase 3: Cognitive Signal -------------------------------------
            "signal_create"     => Self::signal_create(args),
            "signal_value"      => Self::signal_value(args),
            "signal_confidence" => Self::signal_confidence(args),
            "signal_reason"     => Self::signal_reason(args),
            "signal_and"        => Self::signal_and(args),
            "signal_or"         => Self::signal_or(args),
            "signal_confident"  => Self::signal_confident(args),
            "signal_uncertain"  => Self::signal_uncertain(args),
            "signal_to_str"     => Self::signal_to_str(args),

            // -- Phase 4: Qubit (quantum simulation) ---------------------------
            "qubit_create"    => Self::qubit_create(args),
            "qubit_hadamard"  => Self::qubit_hadamard(args),
            "qubit_pauli_x"   => Self::qubit_pauli_x(args),
            "qubit_pauli_z"   => Self::qubit_pauli_z(args),
            "qubit_measure"   => Self::qubit_measure(args),
            "qubit_prob0"     => Self::qubit_prob0(args),
            "qubit_prob1"     => Self::qubit_prob1(args),
            "qubit_phase"     => Self::qubit_phase(args),
            "qubit_to_str"    => Self::qubit_to_str(args),
            "qubit_entangle"  => Self::qubit_entangle(args),
            // -- Phase 5: Tryte (6-trit word) ----------------------------------
            "tryte_create"    => Self::tryte_create(args),
            "tryte_from_int"  => Self::tryte_from_int(args),
            "tryte_to_int"    => Self::tryte_to_int(args),
            "tryte_to_str"    => Self::tryte_to_str(args),
            "tryte_get"       => Self::tryte_get(args),
            "tryte_set"       => Self::tryte_set(args),
            "tryte_and"       => Self::tryte_and(args),
            "tryte_or"        => Self::tryte_or(args),
            "tryte_not"       => Self::tryte_not(args),
            "tryte_add"       => Self::tryte_add(args),
            "tryte_eq"        => Self::tryte_eq(args),
            "tryte_zero"      => Self::tryte_zero(args),
            "tryte_type"      => { let _ = args; Ok(Value::Str("tryte".to_string())) },
            "parse_json" => Self::parse_json(args),
            "json_stringify" => Self::json_stringify(args),
            "HttpServer_new" => Self::http_server_new(args),
            "HttpServer_listen" => Self::http_server_listen(args),
            
            // JSON/CSV functions (Week 24: Data Serialization)
            "json_pretty" => Self::json_pretty(args),
            "parse_csv" => Self::parse_csv(args),
            "to_csv" => Self::to_csv(args),
            "to_yaml" => Self::to_yaml(args),
            
            // WebSocket functions (Week 24: Real-time Communication)
            "websocket_new" => Self::websocket_new(args),
            "websocket_server_new" => Self::websocket_server_new(args),
            "ws_connect" => Self::ws_connect(args),
            "ws_send" => Self::ws_send(args),
            "ws_receive" => Self::ws_receive(args),
            "ws_disconnect" => Self::ws_disconnect(args),
            
            // Trait system functions (Week 24: Polymorphism)
            "trait_new" => Self::trait_new(args),
            "trait_impl" => Self::trait_impl(args),
            "trait_check" => Self::trait_check(args),
            "trait_resolve" => Self::trait_resolve(args),
            
            // Native GGUF inference (killer-native inference engine)
            "llm_chat"   => Self::llm_chat(args),
            "llm_ask"    => Self::llm_ask(args),
            "llm_info"   => Self::llm_info(args),
            "ghost_ask"  => Self::ghost_ask(args),
            "ghost_smart_solve" => Self::ghost_smart_solve(args),
            // RLM â€” Reasoning Language Models (DeepSeek-R1, QwQ)
            "rlm_think"    => Self::rlm_think(args),
            "rlm_answer"   => Self::rlm_answer(args),
            "rlm_thinking" => Self::rlm_thinking(args),
            // Native Think Engine â€” 100% Killer-native, zero external model
            "native_think" => Self::native_think(args),
            // Core arithmetic for agents (no LLM): + âˆ’ * / % ^, parens, scientific notation
            "math_eval" => Self::math_eval(args),
            "math_eval_subst" => Self::math_eval_subst(args),
            // Ghost-108 â€” parallel multi-agent search, fastest wins
            "ghost_108" => Self::ghost_108(args),
            // KhLM â€” Killer Hybrid Language Model unified router
            "khlm_ask"       => Self::khlm_ask(args),
            "khlm_ask_model" => Self::khlm_ask_model(args),
            // KhLM prefetch â€” fire background fetch at program start, khlm_ask returns from cache instantly
            "khlm_prefetch"  => Self::khlm_prefetch(args),
            // Killer AI System â€” parallel KhLM + Ghost-108 + local neural (see llm::khlm_ai_system_multi_agent)
            "khlm_ai_system" => Self::khlm_ai_system(args),
            // LLM-as-RLM â€” any LLM becomes a reasoning model via chain-of-thought prompt
            "llm_reason"        => Self::llm_reason(args),
            "llm_reason_answer" => Self::llm_reason_answer(args),
            // User-composable KhLM building blocks
            "khlm_classify"   => Self::khlm_classify(args),
            "khlm_run"        => Self::khlm_run(args),
            "llm_parallel"    => Self::llm_parallel(args),
            "rlm_synthesize"  => Self::rlm_synthesize(args),
            // â”€â”€ IMAGINATION ENGINE â€” think beyond, counterfactual, conceptual bridges â”€â”€
            "imagine"          => Self::builtin_imagine(args),
            "imagine_what_if"  => Self::builtin_imagine_what_if(args),
            "imagine_connect"  => Self::builtin_imagine_connect(args),
            "imagine_beyond"   => Self::builtin_imagine_beyond(args),
            "imagine_self"     => Self::builtin_imagine_self(args),
            // â”€â”€ AFFECT ENGINE â€” emotional state, feelings, colored responses â”€â”€â”€â”€â”€â”€â”€â”€
            "affect_sense"     => Self::builtin_affect_sense(args),
            "affect_state"     => Self::builtin_affect_state(args),
            "affect_color"     => Self::builtin_affect_color(args),
            "affect_reset"     => Self::builtin_affect_reset(args),
            "affect_set"       => Self::builtin_affect_set(args),
            // â”€â”€ GUARDIAN ENGINE â€” Human Protection Principle â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
            "guardian_check"      => Self::builtin_guardian_check(args),
            "guardian_principles" => Self::builtin_guardian_principles(args),
            "guardian_status"     => Self::builtin_guardian_status(args),
            // KORE — real implementation using kore.rs
            "kore_write" => {
                if args.len() < 3 {
                    return Err(VmError::runtime_error("kore_write() expects 3 arguments (path, schema, data)".to_string()));
                }
                let path = args[0].to_string();
                let schema = args[1].to_string();
                let data = args[2].to_string();
                Ok(Value::Str(crate::kore::kore_write_simple(&path, &schema, &data)))
            }
            "kore_read" => {
                if args.is_empty() {
                    return Err(VmError::runtime_error("kore_read() expects 1 argument (path)".to_string()));
                }
                let path = args[0].to_string();
                Ok(Value::Str(crate::kore::kore_read_simple(&path)))
            }
            "kore_read_col" => {
                if args.len() < 2 {
                    return Err(VmError::runtime_error("kore_read_col() expects 2 arguments (path, col_name)".to_string()));
                }
                let path = args[0].to_string();
                let col = args[1].to_string();
                Ok(Value::Str(crate::kore::kore_read_col_simple(&path, &col)))
            }
            "kore_info" => {
                if args.is_empty() {
                    return Err(VmError::runtime_error("kore_info() expects 1 argument (path)".to_string()));
                }
                let path = args[0].to_string();
                Ok(Value::Str(crate::kore::kore_info_simple(&path)))
            }
            // Nova — stub: use Killer (full) for Nova data operations
            "nova_write" | "nova_info" | "nova_read_col" | "nova_read_all"
            | "nova_stats" | "nova_filter" | "nova_to_csv" | "nova_to_json"
            | "nova_to_tsv" | "nova_from_json" | "nova_from_tsv"
            | "nova_from_xml" | "nova_to_xml" | "nova_to_ndjson" | "nova_from_ndjson"
            | "nova_to_avro" | "nova_from_avro" | "nova_to_parquet" | "nova_from_parquet"
            | "nova_auto_convert" | "nova_compress" | "nova_decompress"
            | "nova_stream_open" | "nova_stream_col" | "nova_stream_batch" | "nova_stream_cols" => {
                Ok(Value::Str(format!("{}: use Killer (full) for Nova data operations", name)))
            }
            // -- Nova Galaxy Engine v1: Polyglot @lang{} runtime --------------
            "polyglot_exec"      => crate::polyglot::builtin_polyglot_exec(args),
            "polyglot_list"      => crate::polyglot::builtin_polyglot_list(args),
            "polyglot_check"     => crate::polyglot::builtin_polyglot_check(args),
            // -- AI Assassin Assist Layer --------------------------------------
            "nova_assist_log"        => crate::assassin_assist::builtin_assist_log(args),
            "nova_assist_status"     => crate::assassin_assist::builtin_assist_status(args),
            "nova_assist_debug"      => crate::assassin_assist::builtin_assist_debug(args),
            "nova_assist_optimize"   => crate::assassin_assist::builtin_assist_optimize(args),
            "nova_assist_enable"     => crate::assassin_assist::builtin_assist_enable(args),
            "nova_assist_disable"    => crate::assassin_assist::builtin_assist_disable(args),
            "nova_assist_set_budget" => crate::assassin_assist::builtin_assist_set_budget(args),
            "nova_assist_set_log"    => crate::assassin_assist::builtin_assist_set_log(args),
            "nova_assist_clear"      => crate::assassin_assist::builtin_assist_clear(args),
            // -- E: HTTP Client â€” native http_get/post from Killer code ----------
            "http_post_json"    => crate::http_client::builtin_http_post_json(args),
            "http_head"         => crate::http_client::builtin_http_head(args),
            "http_status"       => crate::http_client::builtin_http_status(args),
            "http_download"     => crate::http_client::builtin_http_download(args),
            // -- A: Streaming polyglot output ----------------------------------
            "polyglot_stream"   => crate::polyglot::builtin_polyglot_stream(args),
            // -- B: Vector Memory ----------------------------------------------
            "vmem_store"           => crate::vector_memory::builtin_vmem_store(args),
            "vmem_recall"          => crate::vector_memory::builtin_vmem_recall(args),
            "vmem_search"          => crate::vector_memory::builtin_vmem_search(args),
            "vmem_forget"          => crate::vector_memory::builtin_vmem_forget(args),
            "vmem_list"            => crate::vector_memory::builtin_vmem_list(args),
            "vmem_stats"           => crate::vector_memory::builtin_vmem_stats(args),
            "vmem_clear"           => crate::vector_memory::builtin_vmem_clear(args),
            "vmem_set_threshold"   => crate::vector_memory::builtin_vmem_set_threshold(args),
            // -- C: KhLM Tool Calling ------------------------------------------
            "tool_register"        => crate::tool_calling::builtin_tool_register(args),
            "tool_call"            => crate::tool_calling::builtin_tool_call(args),
            "tool_list"            => crate::tool_calling::builtin_tool_list(args),
            "khlm_with_tools"      => crate::tool_calling::builtin_khlm_with_tools(args),
            "khlm_tool_status"     => crate::tool_calling::builtin_khlm_tool_status(args),
            "khlm_tool_clear"      => crate::tool_calling::builtin_khlm_tool_clear(args),
            // -- KhLM-Polyglot 5-tier AI router -------------------------------
            "khlm_debug"        => crate::khlm_polyglot::builtin_khlm_debug(args),
            "khlm_suggest"      => crate::khlm_polyglot::builtin_khlm_suggest(args),
            "khlm_explain"      => crate::khlm_polyglot::builtin_khlm_explain(args),
            "khlm_fix"          => crate::khlm_polyglot::builtin_khlm_fix(args),
            "khlm_translate"    => crate::khlm_polyglot::builtin_khlm_translate(args),
            "khlm_status"       => crate::khlm_polyglot::builtin_khlm_status(args),
            "khlm_set_llm"      => crate::khlm_polyglot::builtin_khlm_set_llm(args),
            "khlm_set_rlm"      => crate::khlm_polyglot::builtin_khlm_set_rlm(args),
            "khlm_cache_clear"  => crate::khlm_polyglot::builtin_khlm_cache_clear(args),
            // â”€â”€ Prose Engine â€” GPT-4o quality writing, offline native fallback â”€â”€â”€â”€
            "khlm_write"       => crate::khlm_polyglot::builtin_khlm_write(args),
            // â”€â”€ Vision Engine â€” image_load / image_describe / khlm_vision â”€â”€â”€â”€â”€â”€â”€â”€â”€
            "image_load"       => crate::vision::builtin_image_load(args),
            "image_describe"   => crate::vision::builtin_image_describe(args),
            "khlm_vision"      => crate::vision::builtin_khlm_vision(args),
            // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
            // KALA (à¤•à¤¾à¤²) â€” Brand face of the Killer AI engine
            // Each kala_* is a clean alias for the underlying KhLM/engine builtin.
            // khlm_* internals remain unchanged; Kala is the public API.
            // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
            // Core intelligence
            "kala_ask"         => Self::khlm_ask(args),
            "kala_think"       => Self::native_think(args),
            // Prose & Vision
            "kala_write"       => crate::khlm_polyglot::builtin_khlm_write(args),
            "kala_vision"      => crate::vision::builtin_khlm_vision(args),
            "kala_describe"    => crate::vision::builtin_image_describe(args),
            // Code intelligence
            "kala_debug"       => crate::khlm_polyglot::builtin_khlm_debug(args),
            "kala_suggest"     => crate::khlm_polyglot::builtin_khlm_suggest(args),
            "kala_explain"     => crate::khlm_polyglot::builtin_khlm_explain(args),
            "kala_fix"         => crate::khlm_polyglot::builtin_khlm_fix(args),
            "kala_translate"   => crate::khlm_polyglot::builtin_khlm_translate(args),
            // Unique engines
            "kala_imagine"     => Self::builtin_imagine(args),
            "kala_what_if"     => Self::builtin_imagine_what_if(args),
            "kala_feel"        => Self::builtin_affect_sense(args),
            "kala_guard"       => Self::builtin_guardian_check(args),
            // Config & status
            "kala_status"      => crate::khlm_polyglot::builtin_khlm_status(args),
            "kala_set_llm"     => crate::khlm_polyglot::builtin_khlm_set_llm(args),
            "kala_prefetch"    => Self::khlm_prefetch(args),
            "kala_clear_cache" => crate::khlm_polyglot::builtin_khlm_cache_clear(args),
            "kala_ai_system"   => Self::khlm_ai_system(args),
            // Kala Chat UI server
            "kala_serve"       => crate::kala_ui::builtin_kala_serve(args),
            // Kala Generator â€” image & video generation
            "kala_generate_image" => crate::image_gen::builtin_kala_generate_image(args),
            "kala_generate_video" => crate::image_gen::builtin_kala_generate_video(args),
            // File system / data operations â€” stub: use Killer (full) for Nova
            "nova_file_read" | "nova_file_write" | "nova_file_append"
            | "nova_file_exists" | "nova_file_delete" | "nova_file_size"
            | "nova_dir_list" | "nova_dir_exists" | "nova_dir_create"
            | "nova_select" | "nova_drop_col" | "nova_rename_col" | "nova_add_col"
            | "nova_head" | "nova_tail" | "nova_sort" | "nova_merge" | "nova_join"
            | "nova_group_by" | "nova_distinct" | "nova_sample" | "nova_filter_op"
            | "nova_fill" | "nova_read_lines" | "nova_multi_filter" | "nova_cast"
            | "nova_concat" | "nova_show"
            | "nova_to_xlsx" | "nova_from_xlsx" | "nova_to_orc" | "nova_from_orc" => {
                Ok(Value::Str(format!("{}: use Killer (full) for Nova file/data operations", name)))
            }

            // AI functions (v3.2: Native AI with multiple backends)
            "ai_generate" => Self::ai_generate(args),
            "ai_embed" => Self::ai_embed(args),
            "ai_classify" => Self::ai_classify(args),
            "ai_extract" => Self::ai_extract(args),
            "ai_local_infer" => Self::ai_local_infer(args),
            "ai_provider_set" => Self::ai_provider_set(args),
            "ai_provider_get" => Self::ai_provider_get(args),
            "ai_cache_enable" => Self::ai_cache_enable(args),
            "ai_cache_clear" => Self::ai_cache_clear(args),
            
            // Interpolation
            "interpolate" => Self::interpolate(args),
            
            // Internal helpers
            "__dict_keys_iter" => Self::dict_keys_iter(args),
            
            // Generator helpers
            "next" => Self::next(args),

            // -- Nova/Killer native compression builtins -----------------------
            // compress(text, algo)        â†’ String (base64-encoded compressed bytes)
            //   algo: "nova" (LZ77+Huffman), "rle" (run-length), "lz77" (raw LZ77)
            // decompress(compressed, algo)â†’ String (original text)
            // b64_encode(text)            â†’ String
            // b64_decode(b64)             â†’ String
            // hex_encode(text)            â†’ String
            // hex_decode(hex)             â†’ String
            // compress_ratio(orig, comp)  â†’ Number (orig_len / comp_len)
            // compress_info(text)         â†’ Dict with sizes+ratios for all algos
            "compress"       => Self::builtin_compress(args),
            "decompress"     => Self::builtin_decompress(args),
            "b64_encode"     => Self::builtin_b64_encode(args),
            "b64_decode"     => Self::builtin_b64_decode(args),
            "hex_encode"     => Self::builtin_hex_encode(args),
            "hex_decode"     => Self::builtin_hex_decode(args),
            "compress_ratio" => Self::builtin_compress_ratio(args),
            "compress_info"  => Self::builtin_compress_info(args),

            // -- Debug Intelligence â€” "Developer Can Relax" system ------------
            // debug_check(code)         â†’ Array of issue dicts
            // auto_fix(code)            â†’ Array of fix-candidate dicts
            // explain_error(msg, ctx)   â†’ String explanation
            // suggest_refactor(code)    â†’ Array of suggestion dicts
            // auto_test(code)           â†’ String (Killer test scaffold)
            // perf_profile(code)        â†’ Array of perf-hint dicts
            // ai_pair(task)             â†’ String (generated Killer code)
            // killer_debug_agent(code)  â†’ Dict (autonomous fix agent result)
            // watch_value(expr, value)  â†’ Null (record debug watch entry; file watch is `watch` below)
            // watch_report()            â†’ String (dump watch log)
            "debug_check"         => Self::dbg_debug_check(args),
            "auto_fix"            => Self::dbg_auto_fix(args),
            "explain_error"       => Self::dbg_explain_error(args),
            "suggest_refactor"    => Self::dbg_suggest_refactor(args),
            "auto_test"           => Self::dbg_auto_test(args),
            "perf_profile"        => Self::dbg_perf_profile(args),
            "ai_pair"             => Self::dbg_ai_pair(args),
            "killer_debug_agent"  => Self::dbg_killer_debug_agent(args),
            // Expr watch (debug); file watch is `watch` in killer_improve below
            "watch_value" => Self::dbg_watch(args),
            "watch_report"        => Self::dbg_watch_report(args),
            "lint"                => Self::builtin_lint(args),
            
            // -- v1.2: Native Hash Map (O(1) average) -----------------
            // hash_map_new()                     â†’ Dict (empty hash map)
            // hash_map_insert(map, key, value)   â†’ Dict (updated map)
            // hash_map_get(map, key)             â†’ value or Null
            // hash_map_contains(map, key)        â†’ Bool
            // hash_map_remove(map, key)          â†’ Dict (map without key)
            // hash_map_size(map)                 â†’ Number
            // hash_map_keys(map)                 â†’ Array of keys
            // hash_map_values(map)               â†’ Array of values
            // insert(map, key, value)            â†’ Dict (friendly alias)
            "hash_map_new"      => Self::hm_new(args),
            "hash_map_insert" | "insert" => Self::hm_insert(args),
            "hash_map_get"      => Self::hm_get(args),
            // Python-style dict: get(map, key[, default]), setdefault(map, key, default) â†’ [newMap, value]
            "get"               => Self::dict_get(args),
            "setdefault"        => Self::dict_setdefault(args),
            "hash_map_contains" => Self::hm_contains(args),
            "hash_map_remove"   => Self::hm_remove(args),
            "hash_map_size"     => Self::hm_size(args),
            "hash_map_keys"     => Self::hm_keys(args),
            "hash_map_values"   => Self::hm_values(args),

            // -- v1.2: Dijkstra shortest path O((V+E) log V) -----------
            // dijkstra(adj_list, source)            â†’ Array of distances
            //   adj_list: Array of Arrays: [[{to:Int,weight:Int},...], ...]
            //   source: Int (0-indexed start vertex)
            //   returns: Array<Int> where result[i] = shortest dist from source to i
            //            (i64::MAX/2 means unreachable)
            // dijkstra_path(adj_list, source, target) â†’ Array<Int> (vertex path)
            "dijkstra"          => Self::dijkstra(args),
            "dijkstra_path"     => Self::dijkstra_path(args),

            // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
            // ANDROID NATIVE MODULES â€” Pure Killer Call Recording Engine
            // Microphone, Phone State, Service, Security, Permissions
            // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

            // -- Microphone Recording (AAudio NDK on Android, simulated on Desktop) --
            "mic_record_start"   => crate::android_audio::builtin_mic_record_start(args),
            "mic_record_stop"    => crate::android_audio::builtin_mic_record_stop(args),
            "mic_record_pause"   => crate::android_audio::builtin_mic_record_pause(args),
            "mic_record_resume"  => crate::android_audio::builtin_mic_record_resume(args),
            "mic_status"         => crate::android_audio::builtin_mic_status(args),
            "mic_list_sources"   => crate::android_audio::builtin_mic_list_sources(args),
            "mic_set_source"     => crate::android_audio::builtin_mic_set_source(args),
            "mic_get_amplitude"  => crate::android_audio::builtin_mic_get_amplitude(args),

            // -- Phone State Detection (JNI on Android, simulated on Desktop) --------
            "phone_get_state"       => crate::android_phone::builtin_phone_get_state(args),
            "phone_is_in_call"      => crate::android_phone::builtin_phone_is_in_call(args),
            "phone_listen_calls"    => crate::android_phone::builtin_phone_listen_calls(args),
            "phone_stop_listening"  => crate::android_phone::builtin_phone_stop_listening(args),
            "phone_get_call_info"   => crate::android_phone::builtin_phone_get_call_info(args),
            "phone_set_auto_record" => crate::android_phone::builtin_phone_set_auto_record(args),
            "phone_get_auto_record" => crate::android_phone::builtin_phone_get_auto_record(args),
            "phone_simulate"        => crate::android_phone::builtin_phone_simulate(args),
            "phone_get_voip_apps"   => crate::android_phone::builtin_phone_get_voip_apps(args),
            "phone_add_voip_app"    => crate::android_phone::builtin_phone_add_voip_app(args),

            // -- Foreground Service, Notifications, Device Info -------------------
            "service_start"          => crate::android_service::builtin_service_start(args),
            "service_stop"           => crate::android_service::builtin_service_stop(args),
            "service_is_running"     => crate::android_service::builtin_service_is_running(args),
            "permission_check"       => crate::android_service::builtin_permission_check(args),
            "permission_check_all"   => crate::android_service::builtin_permission_check_all(args),
            "permission_request"     => crate::android_service::builtin_permission_request(args),
            "permission_request_all" => crate::android_service::builtin_permission_request_all(args),
            "notification_show"      => crate::android_service::builtin_notification_show(args),
            "notification_cancel"    => crate::android_service::builtin_notification_cancel(args),
            "device_info"            => crate::android_service::builtin_device_info(args),
            "storage_path"           => crate::android_service::builtin_storage_path(args),
            "storage_external_path"  => crate::android_service::builtin_storage_external_path(args),
            "vibrate"                => crate::android_service::builtin_vibrate(args),
            "battery_level"          => crate::android_service::builtin_battery_level(args),
            "screen_on"              => crate::android_service::builtin_screen_on(args),

            // -- Security: Encryption, PIN, Evidence, Wipe -----------------------
            "secure_encrypt"          => crate::android_security::builtin_secure_encrypt(args),
            "secure_decrypt"          => crate::android_security::builtin_secure_decrypt(args),
            "secure_hash"             => crate::android_security::builtin_secure_hash(args),
            "secure_hash_file"        => crate::android_security::builtin_secure_hash_file(args),
            "secure_random_bytes"     => crate::android_security::builtin_secure_random_bytes(args),
            "secure_pin_set"          => crate::android_security::builtin_secure_pin_set(args),
            "secure_pin_verify"       => crate::android_security::builtin_secure_pin_verify(args),
            "secure_pin_is_set"       => crate::android_security::builtin_secure_pin_is_set(args),
            "secure_lock"             => crate::android_security::builtin_secure_lock(args),
            "secure_is_locked"        => crate::android_security::builtin_secure_is_locked(args),
            "secure_unlock"           => crate::android_security::builtin_secure_unlock(args),
            "secure_check_integrity"  => crate::android_security::builtin_secure_check_integrity(args),
            "evidence_hash"           => crate::android_security::builtin_evidence_hash(args),
            "secure_wipe_recordings"  => crate::android_security::builtin_secure_wipe_recordings(args),

            // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
            // PRODUCTION MODULE â€” Regex, Help/Docs, File DB, Formatter, Linter
            // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

            // -- Regex Engine (NFA, supports . * + ? | [] [^] ^ $ \d \w \s) --
            "regex_match"      => crate::production::builtin_regex_match(args),
            "regex_find"       => crate::production::builtin_regex_find(args),
            "regex_find_all"   => crate::production::builtin_regex_find_all(args),
            "regex_replace"    => crate::production::builtin_regex_replace(args),
            "regex_split"      => crate::production::builtin_regex_split(args),
            "regex_test"       => crate::production::builtin_regex_test(args),

            // -- Help / Documentation System --
            "help"             => crate::production::builtin_help(args),
            "help_search"      => crate::production::builtin_help_search(args),
            "help_list"        => crate::production::builtin_help_list(args),

            // -- File Database (key-value, JSON-serialized, TSV storage) --
            "db_open"          => crate::production::builtin_db_open(args),
            "db_get"           => crate::production::builtin_db_get(args),
            "db_set"           => crate::production::builtin_db_set(args),
            "db_delete"        => crate::production::builtin_db_delete(args),
            "db_keys"          => crate::production::builtin_db_keys(args),
            "db_keys_prefix"   => crate::production::builtin_db_keys_prefix(args),
            "db_count"         => crate::production::builtin_db_count(args),
            "db_close"         => crate::production::builtin_db_close(args),
            "db_drop"          => crate::production::builtin_db_drop(args),

            // -- Formatter (indent, normalize spacing) --
            "fmt"              => crate::production::builtin_fmt(args),
            "fmt_file"         => crate::production::builtin_fmt_file(args),

            // -- Linter (static analysis) --
            "lint_code"        => crate::production::builtin_lint_code(args),
            "lint_file"        => crate::production::builtin_lint_file(args),

            // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
            // 10x MODULE â€” Package Manager, LSP Server, DAP Debugger, Docs Site
            // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

            // -- Package Manager (killer.toml, deps, registry) --
            "pkg_init"         => crate::killer_10x::builtin_pkg_init(args),
            "pkg_add"          => crate::killer_10x::builtin_pkg_add(args),
            "pkg_remove"       => crate::killer_10x::builtin_pkg_remove(args),
            "pkg_list"         => crate::killer_10x::builtin_pkg_list(args),
            "pkg_resolve"      => crate::killer_10x::builtin_pkg_resolve(args),
            "pkg_install"      => crate::killer_10x::builtin_pkg_install(args),
            "pkg_info"         => crate::killer_10x::builtin_pkg_info(args),
            "pkg_search"       => crate::killer_10x::builtin_pkg_search(args),
            "pkg_publish"      => crate::killer_10x::builtin_pkg_publish(args),
            "pkg_version"      => crate::killer_10x::builtin_pkg_version(args),

            // -- LSP Server (diagnostics, completions, hover, format) --
            "lsp_start"        => crate::killer_10x::builtin_lsp_start(args),
            "lsp_stop"         => crate::killer_10x::builtin_lsp_stop(args),
            "lsp_analyze"      => crate::killer_10x::builtin_lsp_analyze(args),
            "lsp_complete"     => crate::killer_10x::builtin_lsp_complete(args),
            "lsp_hover"        => crate::killer_10x::builtin_lsp_hover(args),
            "lsp_format"       => crate::killer_10x::builtin_lsp_format(args),

            // -- DAP Debugger (breakpoints, stepping, variables, stack) --
            "dap_start"        => crate::killer_10x::builtin_dap_start(args),
            "dap_break"        => crate::killer_10x::builtin_dap_break(args),
            "dap_remove_break" => crate::killer_10x::builtin_dap_remove_break(args),
            "dap_step"         => crate::killer_10x::builtin_dap_step(args),
            "dap_next"         => crate::killer_10x::builtin_dap_next(args),
            "dap_continue"     => crate::killer_10x::builtin_dap_continue(args),
            "dap_vars"         => crate::killer_10x::builtin_dap_vars(args),
            "dap_stack"        => crate::killer_10x::builtin_dap_stack(args),
            "dap_eval"         => crate::killer_10x::builtin_dap_eval(args),
            "dap_stop"         => crate::killer_10x::builtin_dap_stop(args),
            "dap_list_breaks"  => crate::killer_10x::builtin_dap_list_breaks(args),

            // -- Docs Site Generator (HTML docs, search, serve, export) --
            "docs_generate"    => crate::killer_10x::builtin_docs_generate(args),
            "docs_serve"       => crate::killer_10x::builtin_docs_serve(args),
            "docs_search"      => crate::killer_10x::builtin_docs_search(args),
            "docs_api"         => crate::killer_10x::builtin_docs_api(args),
            "docs_export"      => crate::killer_10x::builtin_docs_export(args),

            // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
            // IMPROVE MODULE â€” Errors, Imports, Watch, Stack, REPL, Perf, Docs
            // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

            // -- Enhanced Errors ("did you mean?") --
            "error_enhance"    => crate::killer_improve::builtin_error_enhance(args),
            "suggest"          => crate::killer_improve::builtin_suggest(args),

            // -- Import / Module System --
            "import"           => crate::killer_improve::builtin_import(args),
            "import_list"      => crate::killer_improve::builtin_import_list(args),
            "import_clear"     => crate::killer_improve::builtin_import_clear(args),

            // -- Watch Mode (auto-reload on file change) --
            "watch"            => crate::killer_improve::builtin_watch(args),
            "watch_dir"        => crate::killer_improve::builtin_watch_dir(args),

            // -- Stack Traces --
            "stack_push"       => crate::killer_improve::builtin_stack_push(args),
            "stack_pop"        => crate::killer_improve::builtin_stack_pop(args),
            "stack_trace"      => crate::killer_improve::builtin_stack_trace(args),
            "stack_clear"      => crate::killer_improve::builtin_stack_clear(args),

            // -- REPL Completions --
            "repl_complete"    => crate::killer_improve::builtin_repl_complete(args),
            "repl_complete_sig"=> crate::killer_improve::builtin_repl_complete_sig(args),

            // -- Performance Baseline --
            "bench_run"        => crate::killer_improve::builtin_bench_run(args),
            "bench_all"        => crate::killer_improve::builtin_bench_all(args),
            "bench_save"       => crate::killer_improve::builtin_bench_save(args),
            "bench_compare"    => crate::killer_improve::builtin_bench_compare(args),

            // -- Doc Comments Parser --
            "doc_parse"        => crate::killer_improve::builtin_doc_parse(args),
            "doc_check"        => crate::killer_improve::builtin_doc_check(args),

            // -- Native UI core (killer_ui): patch + graph + workspace; window = stub until eframe --
            "ui_core_version"           => crate::killer_ui::builtin_ui_core_version(args),
            "ui_headless_tick"          => crate::killer_ui::builtin_ui_headless_tick(args),
            "ui_headless_snapshot_json" => crate::killer_ui::builtin_ui_headless_snapshot_json(args),
            "ui_health"                 => crate::killer_ui::builtin_ui_health(args),
            "ui_help"                   => crate::killer_ui::builtin_ui_help(args),
            "ui_native_window"          => crate::killer_ui::builtin_ui_native_window(args),
            "ui_render_gallery"         => crate::killer_ui::builtins::builtin_ui_render_gallery(args),
            "ui_render_screenshot"      => crate::killer_ui::builtins::builtin_ui_render_screenshot(args),

            // -- killer_ui v2: reactive, events, style, components, layout, routing, vdom, animation, devtools --
            "ui_signal_create"          => crate::killer_ui::builtin_ui_signal_create(args),
            "ui_signal_get"             => crate::killer_ui::builtin_ui_signal_get(args),
            "ui_signal_set"             => crate::killer_ui::builtin_ui_signal_set(args),
            "ui_computed"               => crate::killer_ui::builtin_ui_computed(args),
            "ui_effect"                 => crate::killer_ui::builtin_ui_effect(args),
            "ui_batch"                  => crate::killer_ui::builtin_ui_batch(args),
            "ui_on_event"               => crate::killer_ui::builtin_ui_on_event(args),
            "ui_dispatch_event"         => crate::killer_ui::builtin_ui_dispatch_event(args),
            "ui_theme"                  => crate::killer_ui::builtin_ui_theme(args),
            "ui_style_set"              => crate::killer_ui::builtin_ui_style_set(args),
            "ui_component_register"     => crate::killer_ui::builtin_ui_component_register(args),
            "ui_component_create"       => crate::killer_ui::builtin_ui_component_create(args),
            "ui_layout_compute"         => crate::killer_ui::builtin_ui_layout_compute(args),
            "ui_navigate"               => crate::killer_ui::builtin_ui_navigate(args),
            "ui_route_add"              => crate::killer_ui::builtin_ui_route_add(args),
            "ui_vdom_diff"              => crate::killer_ui::builtin_ui_vdom_diff(args),
            "ui_vdom_patch"             => crate::killer_ui::builtin_ui_vdom_patch(args),
            "ui_animate"                => crate::killer_ui::builtin_ui_animate(args),
            "ui_animate_keyframes"      => crate::killer_ui::builtin_ui_animate_keyframes(args),
            "ui_inspect"                => crate::killer_ui::builtin_ui_inspect(args),
            "ui_perf_snapshot"          => crate::killer_ui::builtin_ui_perf_snapshot(args),

            // -- Interactive I/O --
            "readline"          => Self::readline(args),
            "readline_prompt"   => Self::readline_prompt(args),

            // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
            // v2.3: OS-LEVEL PRIMITIVES â€” Integer, Bytes, Pointer, Memory, I/O
            // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
            "bit_not"           => Self::bit_not(args),
            "bit_rotl"          => Self::bit_rotl(args),
            "bit_rotr"          => Self::bit_rotr(args),
            "to_integer"        => Self::to_integer(args),
            "to_bytes"          => Self::to_bytes(args),
            "to_pointer"        => Self::to_pointer(args),
            "bytes_new"         => Self::bytes_new(args),
            "bytes_len"         => Self::bytes_len(args),
            "bytes_get"         => Self::bytes_get(args),
            "bytes_set"         => Self::bytes_set(args),
            "bytes_slice"       => Self::bytes_slice(args),
            "bytes_from_str"    => Self::bytes_from_str(args),
            "bytes_to_str"      => Self::bytes_to_str(args),
            "bytes_concat"      => Self::bytes_concat(args),
            "bytes_fill"        => Self::bytes_fill(args),
            "ptr_new"           => Self::ptr_new(args),
            "ptr_to_int"        => Self::ptr_to_int(args),
            "ptr_offset"        => Self::ptr_offset(args),
            "mem_read_u8"       => Self::mem_read_u8(args),
            "mem_read_u16"      => Self::mem_read_u16(args),
            "mem_read_u32"      => Self::mem_read_u32(args),
            "mem_read_u64"      => Self::mem_read_u64(args),
            "mem_write_u8"      => Self::mem_write_u8(args),
            "mem_write_u16"     => Self::mem_write_u16(args),
            "mem_write_u32"     => Self::mem_write_u32(args),
            "mem_write_u64"     => Self::mem_write_u64(args),
            "volatile_read_u8"  => Self::volatile_read_u8(args),
            "volatile_read_u16" => Self::volatile_read_u16(args),
            "volatile_read_u32" => Self::volatile_read_u32(args),
            "volatile_read_u64" => Self::volatile_read_u64(args),
            "volatile_write_u8" => Self::volatile_write_u8(args),
            "volatile_write_u16"=> Self::volatile_write_u16(args),
            "volatile_write_u32"=> Self::volatile_write_u32(args),
            "volatile_write_u64"=> Self::volatile_write_u64(args),
            "io_port_in_u8"     => Self::io_port_in_u8(args),
            "io_port_in_u16"    => Self::io_port_in_u16(args),
            "io_port_out_u8"    => Self::io_port_out_u8(args),
            "io_port_out_u16"   => Self::io_port_out_u16(args),
            "sha256"            => Self::sha256(args),
            "sha256_bytes"      => Self::sha256_bytes(args),
            "mmap_alloc"        => Self::mmap_alloc(args),
            "mmap_free"         => Self::mmap_free(args),
            "mmap_write"        => Self::mmap_write(args),
            "mmap_read"         => Self::mmap_read(args),
            "mmap_exec"         => Self::mmap_exec(args),
            "int_to_bytes_le"   => Self::int_to_bytes_le(args),
            "int_to_bytes_be"   => Self::int_to_bytes_be(args),
            "bytes_to_int_le"   => Self::bytes_to_int_le(args),
            "bytes_to_int_be"   => Self::bytes_to_int_be(args),
            "cli_args"          => Self::cli_args(args),
            "env_get"           => Self::env_get(args),
            "env_set"           => Self::env_set(args),
            "process_exit"      => Self::process_exit(args),
            "errno"             => Self::errno(args),
            "sizeof"            => Self::sizeof_val(args),
            "alignof"           => Self::alignof_val(args),
            "atomic_load"       => Self::atomic_load(args),
            "atomic_store"      => Self::atomic_store(args),
            "atomic_cas"        => Self::atomic_cas(args),
            "atomic_add"        => Self::atomic_add(args),
            "interrupts_disable" => Self::interrupts_disable(args),
            "interrupts_enable" => Self::interrupts_enable(args),
            "wfi"               => Self::wfi(args),
            "fence"             => Self::fence(args),
            "disk_read_block"   => Self::disk_read_block(args),
            "disk_write_block"  => Self::disk_write_block(args),
            "page_alloc"        => Self::page_alloc(args),
            "page_free"         => Self::page_free(args),
            "page_map"          => Self::page_map(args),
            // v2.3: OS-level hardware primitives
            "cpuid"             => Self::cpuid(args),
            "rdtsc"             => Self::rdtsc(args),
            "gdt_encode"        => Self::gdt_encode(args),
            "idt_encode"        => Self::idt_encode(args),
            "call_native"       => Self::call_native(args),

            _ => Err(VmError::runtime_error(
                format!("Unknown builtin function: {}", name),
            )),
        }
    }

    // ===== Length/Count Functions =====
    fn len(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "length expects 1 argument".to_string(),
            ));
        }
        match &args[0] {
            Value::Array(arr) => Ok(Value::Number(arr.len() as f64)),
            Value::Dict(dict) => Ok(Value::Number(dict.len() as f64)),
            Value::Str(s) => Ok(Value::Number(s.len() as f64)),
            _ => Err(VmError::runtime_error(
                format!("length expects array, dict, or string, got {}", args[0]),
            )),
        }
    }

    fn range(args: &[Value]) -> Result<Value, VmError> {
        if args.is_empty() || args.len() > 3 {
            return Err(VmError::runtime_error(
                "range() expects 1 to 3 arguments".to_string(),
            ));
        }

        let start = match &args[0] {
            Value::Number(n) => *n as i64,
            _ => return Err(VmError::runtime_error(
                "range() start must be a number".to_string(),
            )),
        };

        let (end, step) = if args.len() == 1 {
            (start, 1i64)
        } else if args.len() == 2 {
            let end = match &args[1] {
                Value::Number(n) => *n as i64,
                _ => return Err(VmError::runtime_error(
                    "range() end must be a number".to_string(),
                )),
            };
            (end, 1i64)
        } else {
            let end = match &args[1] {
                Value::Number(n) => *n as i64,
                _ => return Err(VmError::runtime_error(
                    "range() end must be a number".to_string(),
                )),
            };
            let step = match &args[2] {
                Value::Number(n) => *n as i64,
                _ => return Err(VmError::runtime_error(
                    "range() step must be a number".to_string(),
                )),
            };
            (end, step)
        };

        if step == 0 {
            return Err(VmError::runtime_error(
                "range() step cannot be zero".to_string(),
            ));
        }

        let mut result = Vec::new();
        if step > 0 {
            let mut i = start;
            while i < end {
                result.push(Value::Number(i as f64));
                i += step;
            }
        } else {
            let mut i = start;
            while i > end {
                result.push(Value::Number(i as f64));
                i += step;
            }
        }
        Ok(Value::from(result))
    }

    // ===== Type Functions =====
    fn type_of(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "type() expects 1 argument".to_string(),
            ));
        }
        let type_name = match &args[0] {
            Value::Number(_) => "number",
            Value::Bool(_) => "bool",
            Value::Str(_) => "string",
            Value::Array(_) => "array",
            Value::Dict(_) => "dict",
            Value::Object(obj) => &obj.class_name,
            Value::Class(cls) => &cls.name,
            Value::Function { .. } => "function",
            Value::Generator(_) => "generator",
            Value::QualityWrapped(_) => "quality",
            Value::Null => "null",
            Value::Trit(_) => "trit",
            Value::Signal { .. } => "signal",
            Value::Qubit { .. } => "qubit",
            Value::Tryte(_) => "tryte",
            Value::Future(_) => "future",
            Value::Integer(_) => "integer",
            Value::Bytes(_) => "bytes",
            Value::Pointer(_) => "pointer",
        };
        Ok(Value::Str(type_name.to_string()))
    }

    fn str(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "str() expects 1 argument".to_string(),
            ));
        }
        Ok(Value::Str(args[0].to_string()))
    }

    fn int(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "int() expects 1 argument".to_string(),
            ));
        }
        match &args[0] {
            Value::Number(n) => Ok(Value::Number(n.trunc())),
            Value::Str(s) => {
                match s.parse::<f64>() {
                    Ok(n) => Ok(Value::Number(n.trunc())),
                    Err(_) => Err(VmError::runtime_error(
                        format!("int() cannot convert '{}' to number", s),
                    )),
                }
            }
            _ => Err(VmError::runtime_error(
                "int() expects number or string".to_string(),
            )),
        }
    }

    // ===== Dictionary Functions =====
    fn keys(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "keys() expects 1 argument".to_string(),
            ));
        }
        match &args[0] {
            Value::Dict(dict) => {
                let keys: Vec<Value> = dict
                    .keys()
                    .map(|k| Value::Str(k.clone()))
                    .collect();
                Ok(Value::from(keys))
            }
            _ => Err(VmError::runtime_error(
                "keys() expects a dictionary".to_string(),
            )),
        }
    }

    fn values(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "values() expects 1 argument".to_string(),
            ));
        }
        match &args[0] {
            Value::Dict(dict) => {
                let values: Vec<Value> = dict.values().cloned().collect();
                Ok(Value::from(values))
            }
            _ => Err(VmError::runtime_error(
                "values() expects a dictionary".to_string(),
            )),
        }
    }

    fn iter_keys(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "iterKeys() expects 1 argument".to_string(),
            ));
        }
        match &args[0] {
            Value::Dict(dict) => {
                let keys: Vec<Value> = dict
                    .keys()
                    .map(|k| Value::Str(k.clone()))
                    .collect();
                Ok(Value::from(keys))
            }
            Value::Array(arr) => Ok(Value::Array(arr.deep_copy())),
            _ => Ok(Value::from(Vec::new())),
        }
    }

    fn entries(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "entries() expects 1 argument".to_string(),
            ));
        }
        match &args[0] {
            Value::Dict(dict) => {
                let mut entries = Vec::new();
                for (k, v) in dict.iter() {
                    let entry = vec![Value::Str(k.clone()), v.clone()];
                    entries.push(Value::from(entry));
                }
                Ok(Value::from(entries))
            }
            _ => Err(VmError::runtime_error(
                "entries() expects a dictionary".to_string(),
            )),
        }
    }

    // ===== String Methods =====
    fn upper(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "upper() expects 1 argument".to_string(),
            ));
        }
        match &args[0] {
            Value::Str(s) => Ok(Value::Str(s.to_uppercase())),
            _ => Err(VmError::runtime_error(
                "upper() expects a string".to_string(),
            )),
        }
    }

    fn lower(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "lower() expects 1 argument".to_string(),
            ));
        }
        match &args[0] {
            Value::Str(s) => Ok(Value::Str(s.to_lowercase())),
            _ => Err(VmError::runtime_error(
                "lower() expects a string".to_string(),
            )),
        }
    }

    fn trim(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "trim() expects 1 argument".to_string(),
            ));
        }
        match &args[0] {
            Value::Str(s) => Ok(Value::Str(s.trim().to_string())),
            _ => Err(VmError::runtime_error(
                "trim() expects a string".to_string(),
            )),
        }
    }

    fn split(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 {
            return Err(VmError::runtime_error(
                "split() expects 2 arguments".to_string(),
            ));
        }
        match (&args[0], &args[1]) {
            (Value::Str(s), Value::Str(sep)) => {
                let parts: Vec<Value> = s.split(sep.as_str())
                    .map(|part| Value::Str(part.to_string()))
                    .collect();
                Ok(Value::from(parts))
            }
            _ => Err(VmError::runtime_error(
                "split() expects string and separator".to_string(),
            )),
        }
    }

    fn starts_with(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 {
            return Err(VmError::runtime_error(
                "starts_with() expects 2 arguments".to_string(),
            ));
        }
        match (&args[0], &args[1]) {
            (Value::Str(s), Value::Str(prefix)) => {
                Ok(Value::Bool(s.starts_with(prefix.as_str())))
            }
            _ => Err(VmError::runtime_error(
                "starts_with() expects strings".to_string(),
            )),
        }
    }

    fn ends_with(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 {
            return Err(VmError::runtime_error(
                "ends_with() expects 2 arguments".to_string(),
            ));
        }
        match (&args[0], &args[1]) {
            (Value::Str(s), Value::Str(suffix)) => {
                Ok(Value::Bool(s.ends_with(suffix.as_str())))
            }
            _ => Err(VmError::runtime_error(
                "ends_with() expects strings".to_string(),
            )),
        }
    }

    fn contains(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 {
            return Err(VmError::runtime_error(
                "contains() expects 2 arguments".to_string(),
            ));
        }
        match (&args[0], &args[1]) {
            (Value::Str(s), Value::Str(substring)) => {
                Ok(Value::Bool(s.contains(substring.as_str())))
            }
            (Value::Array(arr), val) => {
                Ok(Value::Bool(arr.contains(val)))
            }
            _ => Err(VmError::runtime_error(
                "contains() expects string or array".to_string(),
            )),
        }
    }

    fn replace(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 3 {
            return Err(VmError::runtime_error(
                "replace() expects 3 arguments".to_string(),
            ));
        }
        match (&args[0], &args[1], &args[2]) {
            (Value::Str(s), Value::Str(old), Value::Str(new)) => {
                Ok(Value::Str(s.replace(old.as_str(), new.as_str())))
            }
            _ => Err(VmError::runtime_error(
                "replace() expects strings".to_string(),
            )),
        }
    }

    fn substring(args: &[Value]) -> Result<Value, VmError> {
        if args.len() < 2 || args.len() > 3 {
            return Err(VmError::runtime_error(
                "substring() expects 2-3 arguments".to_string(),
            ));
        }
        match &args[0] {
            Value::Str(s) => {
                let start = match &args[1] {
                    Value::Number(n) => (*n as usize).min(s.len()),
                    _ => 0,
                };
                let end = if args.len() > 2 {
                    match &args[2] {
                        Value::Number(n) => (*n as usize).min(s.len()),
                        _ => s.len(),
                    }
                } else {
                    s.len()
                };

                let (begin, finish) = if start > end {
                    (end, start)
                } else {
                    (start, end)
                };

                Ok(Value::Str(s.chars().skip(begin).take(finish - begin).collect()))
            }
            _ => Err(VmError::runtime_error(
                "substring() expects string".to_string(),
            )),
        }
    }

    fn index_of(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 {
            return Err(VmError::runtime_error(
                "indexOf() expects 2 arguments".to_string(),
            ));
        }
        match (&args[0], &args[1]) {
            (Value::Str(s), Value::Str(search)) => {
                match s.find(search.as_str()) {
                    Some(idx) => Ok(Value::Number(idx as f64)),
                    None => Ok(Value::Number(-1.0)),
                }
            }
            (Value::Array(arr), val) => {
                let mut found_idx = -1.0;
                for (i, item) in arr.iter_cloned().enumerate() {
                    if &item == val {
                        found_idx = i as f64;
                        break;
                    }
                }
                Ok(Value::Number(found_idx))
            }
            _ => Err(VmError::runtime_error(
                "indexOf() expects array or string".to_string(),
            )),
        }
    }

    fn repeat(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 {
            return Err(VmError::runtime_error(
                "repeat() expects 2 arguments".to_string(),
            ));
        }
        match (&args[0], &args[1]) {
            (Value::Str(s), Value::Number(count)) => {
                let times = (*count as usize).max(0);
                Ok(Value::Str(s.repeat(times)))
            }
            _ => Err(VmError::runtime_error(
                "repeat() expects string and number".to_string(),
            )),
        }
    }

    // ===== Array Methods =====
    fn push(args: &[Value]) -> Result<Value, VmError> {
        if args.is_empty() || args.len() < 2 {
            return Err(VmError::runtime_error(
                "push() expects at least 2 arguments".to_string(),
            ));
        }
        match &args[0] {
            Value::Array(arr) => {
                for i in 1..args.len() {
                    arr.push(args[i].clone());
                }
                Ok(Value::Array(arr.clone()))
            }
            _ => Err(VmError::runtime_error(
                "push() expects an array".to_string(),
            )),
        }
    }

    fn pop(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "pop() expects 1 argument".to_string(),
            ));
        }
        match &args[0] {
            Value::Array(arr) => {
                if arr.is_empty() {
                    Ok(Value::Null)
                } else {
                    Ok(arr.pop().unwrap_or(Value::Null))
                }
            }
            _ => Err(VmError::runtime_error(
                "pop() expects an array".to_string(),
            )),
        }
    }

    fn reverse(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "reverse() expects 1 argument".to_string(),
            ));
        }
        match &args[0] {
            Value::Array(arr) => {
                arr.reverse();
                Ok(Value::Array(arr.clone()))
            }
            _ => Err(VmError::runtime_error(
                "reverse() expects an array".to_string(),
            )),
        }
    }

    /// Shallow `copy` of array or dict (Python `list.copy` / `dict.copy`).
    fn value_copy(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "copy() expects 1 argument: copy(array|dict)".to_string(),
            ));
        }
        match &args[0] {
            Value::Array(a) => Ok(Value::Array(a.deep_copy())),
            Value::Dict(d) => Ok(Value::Dict(d.clone())),
            _ => Err(VmError::runtime_error(
                "copy() expects an array or dict".to_string(),
            )),
        }
    }

    fn join(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 {
            return Err(VmError::runtime_error(
                "join() expects 2 arguments".to_string(),
            ));
        }
        match (&args[0], &args[1]) {
            (Value::Array(arr), Value::Str(sep)) => {
                Ok(Value::Str(arr.join_strings(sep.as_str())))
            }
            _ => Err(VmError::runtime_error(
                "join() expects array and separator".to_string(),
            )),
        }
    }

    fn slice(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 3 {
            return Err(VmError::runtime_error(
                "slice() expects 3 arguments".to_string(),
            ));
        }
        match (&args[0], &args[1], &args[2]) {
            (Value::Array(arr), Value::Number(start), Value::Number(end)) => {
                let s = (*start as usize).min(arr.len());
                let e = (*end as usize).min(arr.len());
                if s <= e {
                    Ok(Value::from(arr.slice_to_vec(s, e)))
                } else {
                    Ok(Value::from(Vec::new()))
                }
            }
            _ => Err(VmError::runtime_error(
                "slice() expects array and numbers".to_string(),
            )),
        }
    }

    fn concat(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 {
            return Err(VmError::runtime_error(
                "concat() expects 2 arguments".to_string(),
            ));
        }
        match (&args[0], &args[1]) {
            (Value::Array(arr1), Value::Array(arr2)) => {
                let mut v = arr1.to_vec();
                v.extend(arr2.to_vec());
                Ok(Value::from(v))
            }
            _ => Err(VmError::runtime_error(
                "concat() expects two arrays".to_string(),
            )),
        }
    }

    fn includes(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 {
            return Err(VmError::runtime_error(
                "includes() expects 2 arguments".to_string(),
            ));
        }
        match &args[0] {
            Value::Array(arr) => {
                Ok(Value::Bool(arr.contains(&args[1])))
            }
            _ => Err(VmError::runtime_error(
                "includes() expects array".to_string(),
            )),
        }
    }

    // ===== Character Access =====
    fn char_at(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 {
            return Err(VmError::runtime_error(
                "charAt() expects 2 arguments".to_string(),
            ));
        }
        match (&args[0], &args[1]) {
            (Value::Str(s), Value::Number(idx)) => {
                let i = *idx as usize;
                let char_count = s.chars().count();
                if i < char_count {
                    if let Some(ch) = s.chars().nth(i) {
                        Ok(Value::Str(ch.to_string()))
                    } else {
                        Ok(Value::Str(String::new()))
                    }
                } else {
                    Ok(Value::Str(String::new()))
                }
            }
            _ => Err(VmError::runtime_error(
                "charAt() expects string and number".to_string(),
            )),
        }
    }

    fn char_code_at(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 {
            return Err(VmError::runtime_error(
                "charCodeAt() expects 2 arguments".to_string(),
            ));
        }
        match (&args[0], &args[1]) {
            (Value::Str(s), Value::Number(idx)) => {
                let i = *idx as usize;
                let char_count = s.chars().count();
                if i < char_count {
                    if let Some(ch) = s.chars().nth(i) {
                        let code = ch as u32 as f64;
                        Ok(Value::Number(code))
                    } else {
                        Ok(Value::Number(f64::NAN))
                    }
                } else {
                    Ok(Value::Number(f64::NAN))
                }
            }
            _ => Err(VmError::runtime_error(
                "charCodeAt() expects string and number".to_string(),
            )),
        }
    }

    // ===== Math Functions =====
    fn sqrt(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "sqrt() expects 1 argument".to_string(),
            ));
        }
        match &args[0] {
            Value::Number(n) => Ok(Value::Number(n.sqrt())),
            _ => Err(VmError::runtime_error(
                "sqrt() expects a number".to_string(),
            )),
        }
    }

    fn pow(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 {
            return Err(VmError::runtime_error(
                "pow() expects 2 arguments".to_string(),
            ));
        }
        match (&args[0], &args[1]) {
            (Value::Number(base), Value::Number(exp)) => {
                Ok(Value::Number(base.powf(*exp)))
            }
            _ => Err(VmError::runtime_error(
                "pow() expects numbers".to_string(),
            )),
        }
    }

    fn num_to_i64(n: f64) -> i64 {
        n as i64
    }

    fn bit_and(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 {
            return Err(VmError::runtime_error("bit_and() expects 2 numbers".to_string()));
        }
        match (&args[0], &args[1]) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(
                (Self::num_to_i64(*a) & Self::num_to_i64(*b)) as f64,
            )),
            _ => Err(VmError::runtime_error("bit_and() expects numbers".to_string())),
        }
    }

    fn bit_or(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 {
            return Err(VmError::runtime_error("bit_or() expects 2 numbers".to_string()));
        }
        match (&args[0], &args[1]) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(
                (Self::num_to_i64(*a) | Self::num_to_i64(*b)) as f64,
            )),
            _ => Err(VmError::runtime_error("bit_or() expects numbers".to_string())),
        }
    }

    fn bit_xor(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 {
            return Err(VmError::runtime_error("bit_xor() expects 2 numbers".to_string()));
        }
        match (&args[0], &args[1]) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(
                (Self::num_to_i64(*a) ^ Self::num_to_i64(*b)) as f64,
            )),
            _ => Err(VmError::runtime_error("bit_xor() expects numbers".to_string())),
        }
    }

    fn bit_shl(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 {
            return Err(VmError::runtime_error("bit_shl() expects 2 numbers".to_string()));
        }
        match (&args[0], &args[1]) {
            (Value::Number(a), Value::Number(b)) => {
                let s = Self::num_to_i64(*b).rem_euclid(64) as u32;
                Ok(Value::Number(
                    (Self::num_to_i64(*a).wrapping_shl(s)) as f64,
                ))
            }
            _ => Err(VmError::runtime_error("bit_shl() expects numbers".to_string())),
        }
    }

    fn bit_shr(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 {
            return Err(VmError::runtime_error("bit_shr() expects 2 numbers".to_string()));
        }
        match (&args[0], &args[1]) {
            (Value::Number(a), Value::Number(b)) => {
                let s = Self::num_to_i64(*b).rem_euclid(64) as u32;
                Ok(Value::Number(
                    (Self::num_to_i64(*a).wrapping_shr(s)) as f64,
                ))
            }
            _ => Err(VmError::runtime_error("bit_shr() expects numbers".to_string())),
        }
    }

    fn abs(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "abs() expects 1 argument".to_string(),
            ));
        }
        match &args[0] {
            Value::Number(n) => Ok(Value::Number(n.abs())),
            _ => Err(VmError::runtime_error(
                "abs() expects a number".to_string(),
            )),
        }
    }

    fn floor(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "floor() expects 1 argument".to_string(),
            ));
        }
        match &args[0] {
            Value::Number(n) => Ok(Value::Number(n.floor())),
            _ => Err(VmError::runtime_error(
                "floor() expects a number".to_string(),
            )),
        }
    }

    fn ceil(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "ceil() expects 1 argument".to_string(),
            ));
        }
        match &args[0] {
            Value::Number(n) => Ok(Value::Number(n.ceil())),
            _ => Err(VmError::runtime_error(
                "ceil() expects a number".to_string(),
            )),
        }
    }

    fn round(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "round() expects 1 argument".to_string(),
            ));
        }
        match &args[0] {
            Value::Number(n) => Ok(Value::Number(n.round())),
            _ => Err(VmError::runtime_error(
                "round() expects a number".to_string(),
            )),
        }
    }

    fn min(args: &[Value]) -> Result<Value, VmError> {
        if args.is_empty() {
            return Err(VmError::runtime_error(
                "min() expects at least 1 argument".to_string(),
            ));
        }
        let mut min_val = f64::INFINITY;
        for arg in args {
            match arg {
                Value::Number(n) => {
                    if n < &min_val {
                        min_val = *n;
                    }
                }
                _ => return Err(VmError::runtime_error(
                    "min() expects numbers".to_string(),
                )),
            }
        }
        Ok(Value::Number(min_val))
    }

    fn max(args: &[Value]) -> Result<Value, VmError> {
        if args.is_empty() {
            return Err(VmError::runtime_error(
                "max() expects at least 1 argument".to_string(),
            ));
        }
        let mut max_val = f64::NEG_INFINITY;
        for arg in args {
            match arg {
                Value::Number(n) => {
                    if n > &max_val {
                        max_val = *n;
                    }
                }
                _ => return Err(VmError::runtime_error(
                    "max() expects numbers".to_string(),
                )),
            }
        }
        Ok(Value::Number(max_val))
    }

    /// `sorted(arr)` or `sorted(arr, reverse)` â€” numbers or strings (homogeneous), new array.
    fn sorted(args: &[Value]) -> Result<Value, VmError> {
        if args.is_empty() || args.len() > 2 {
            return Err(VmError::runtime_error(
                "sorted() expects 1 or 2 arguments: sorted(array[, reverse])".to_string(),
            ));
        }
        let reverse = args.len() == 2
            && match &args[1] {
                Value::Bool(b) => *b,
                Value::Number(n) => *n != 0.0 && !n.is_nan(),
                _ => false,
            };
        match &args[0] {
            Value::Array(arr) => {
                if arr.is_empty() {
                    return Ok(Value::from(Vec::new()));
                }
                let out = arr.deep_copy();
                match out.get(0) {
                    Some(Value::Number(_)) => {
                        for v in out.iter_cloned() {
                            if !matches!(v, Value::Number(_)) {
                                return Err(VmError::runtime_error(
                                    "sorted() number array must contain only numbers".to_string(),
                                ));
                            }
                        }
                        out.sort_by(|a, b| match (a, b) {
                            (Value::Number(x), Value::Number(y)) => {
                                x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal)
                            }
                            _ => std::cmp::Ordering::Equal,
                        });
                    }
                    Some(Value::Str(_)) => {
                        for v in out.iter_cloned() {
                            if !matches!(v, Value::Str(_)) {
                                return Err(VmError::runtime_error(
                                    "sorted() string array must contain only strings".to_string(),
                                ));
                            }
                        }
                        out.sort_by(|a, b| match (a, b) {
                            (Value::Str(x), Value::Str(y)) => x.cmp(y),
                            _ => std::cmp::Ordering::Equal,
                        });
                    }
                    _ => {
                        return Err(VmError::runtime_error(
                            "sorted() expects array of numbers or array of strings".to_string(),
                        ));
                    }
                }
                if reverse {
                    out.reverse();
                }
                Ok(Value::Array(out))
            }
            _ => Err(VmError::runtime_error(
                "sorted() first argument must be an array".to_string(),
            )),
        }
    }

    /// `sum(array)` â€” sum of numeric elements (empty array â†’ 0).
    fn array_sum(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "sum() expects 1 argument: sum(array)".to_string(),
            ));
        }
        match &args[0] {
            Value::Array(arr) => {
                let mut t = 0.0f64;
                for v in arr {
                    match v {
                        Value::Number(n) => t += n,
                        _ => {
                            return Err(VmError::runtime_error(
                                "sum() array must contain only numbers".to_string(),
                            ))
                        }
                    }
                }
                Ok(Value::Number(t))
            }
            _ => Err(VmError::runtime_error(
                "sum() expects an array".to_string(),
            )),
        }
    }

    /// `enumerate(arr)` or `enumerate(arr, start)` â€” `[[i, v], ...]` like Python's enumerate.
    fn enumerate(args: &[Value]) -> Result<Value, VmError> {
        if args.is_empty() || args.len() > 2 {
            return Err(VmError::runtime_error(
                "enumerate() expects 1 or 2 arguments: enumerate(array[, start])".to_string(),
            ));
        }
        let start = if args.len() == 2 {
            match &args[1] {
                Value::Number(n) => *n as i64,
                _ => {
                    return Err(VmError::runtime_error(
                        "enumerate() start must be a number".to_string(),
                    ))
                }
            }
        } else {
            0i64
        };
        match &args[0] {
            Value::Array(arr) => {
                let pairs: Vec<Value> = arr
                    .iter_cloned()
                    .enumerate()
                    .map(|(i, v)| {
                        Value::from(vec![
                            Value::Number((start + i as i64) as f64),
                            v,
                        ])
                    })
                    .collect();
                Ok(Value::from(pairs))
            }
            _ => Err(VmError::runtime_error(
                "enumerate() expects an array".to_string(),
            )),
        }
    }

    fn value_truthy(v: &Value) -> bool {
        match v {
            Value::Bool(b) => *b,
            Value::Number(n) => *n != 0.0 && !n.is_nan(),
            Value::Str(s) => !s.is_empty(),
            Value::Null => false,
            _ => true,
        }
    }

    /// `all(array)` â€” true if every element is truthy (empty â†’ true).
    fn array_all(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "all() expects 1 argument: all(array)".to_string(),
            ));
        }
        match &args[0] {
            Value::Array(arr) => Ok(Value::Bool(
                arr.iter_cloned().all(|v| Self::value_truthy(&v)),
            )),
            _ => Err(VmError::runtime_error(
                "all() expects an array".to_string(),
            )),
        }
    }

    /// `any(array)` â€” true if some element is truthy (empty â†’ false).
    fn array_any(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "any() expects 1 argument: any(array)".to_string(),
            ));
        }
        match &args[0] {
            Value::Array(arr) => Ok(Value::Bool(
                arr.iter_cloned().any(|v| Self::value_truthy(&v)),
            )),
            _ => Err(VmError::runtime_error(
                "any() expects an array".to_string(),
            )),
        }
    }

    /// `zip(a, b)` â€” pairs `[[a0,b0],...]` up to min length.
    fn zip_arrays(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 {
            return Err(VmError::runtime_error(
                "zip() expects 2 arguments: zip(array, array)".to_string(),
            ));
        }
        match (&args[0], &args[1]) {
            (Value::Array(a), Value::Array(b)) => {
                let n = a.len().min(b.len());
                let out: Vec<Value> = (0..n)
                    .map(|i| {
                        Value::from(vec![
                            a.get(i).unwrap_or(Value::Null),
                            b.get(i).unwrap_or(Value::Null),
                        ])
                    })
                    .collect();
                Ok(Value::from(out))
            }
            _ => Err(VmError::runtime_error(
                "zip() expects two arrays".to_string(),
            )),
        }
    }

    /// `map(array, builtin_name)` â€” apply a named builtin to each element.
    /// Example: `map([1,2,3], "str")` â†’ `["1","2","3"]`
    fn array_map(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 {
            return Err(VmError::runtime_error(
                "map() expects 2 arguments: map(array, function_name)".to_string(),
            ));
        }
        let arr = match &args[0] {
            Value::Array(a) => a,
            _ => return Err(VmError::runtime_error("map() first argument must be an array".to_string())),
        };
        let func_name = match &args[1] {
            Value::Str(s) => s.as_str(),
            _ => return Err(VmError::runtime_error("map() second argument must be a function name string".to_string())),
        };
        let mut result = Vec::with_capacity(arr.len());
        for item in arr.iter_cloned() {
            let mapped = Self::call(func_name, &[item])?;
            result.push(mapped);
        }
        Ok(Value::from(result))
    }

    /// `filter(array[, builtin_name])` â€” keep elements where builtin returns truthy, or keep truthy elements.
    /// Example: `filter([0,1,2,0,3])` â†’ `[1,2,3]`; `filter(["a","","b"], "len")` â†’ `["a","b"]`
    fn array_filter(args: &[Value]) -> Result<Value, VmError> {
        if args.is_empty() || args.len() > 2 {
            return Err(VmError::runtime_error(
                "filter() expects 1-2 arguments: filter(array[, function_name])".to_string(),
            ));
        }
        let arr = match &args[0] {
            Value::Array(a) => a,
            _ => return Err(VmError::runtime_error("filter() first argument must be an array".to_string())),
        };
        let func_name = if args.len() == 2 {
            match &args[1] {
                Value::Str(s) => Some(s.clone()),
                _ => return Err(VmError::runtime_error("filter() second argument must be a function name string".to_string())),
            }
        } else {
            None
        };
        let mut result = Vec::new();
        for item in arr.iter_cloned() {
            let keep = if let Some(ref name) = func_name {
                let test_val = Self::call(name, &[item.clone()])?;
                is_truthy(&test_val)
            } else {
                is_truthy(&item)
            };
            if keep {
                result.push(item);
            }
        }
        Ok(Value::from(result))
    }

    /// `reduce(array, operation, initial)` â€” fold with a named binary operation.
    /// Example: `reduce([1,2,3,4], "add", 0)` â†’ `10`
    fn array_reduce(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 3 {
            return Err(VmError::runtime_error(
                "reduce() expects 3 arguments: reduce(array, operation, initial)".to_string(),
            ));
        }
        let arr = match &args[0] {
            Value::Array(a) => a,
            _ => return Err(VmError::runtime_error("reduce() first argument must be an array".to_string())),
        };
        let op = match &args[1] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("reduce() second argument must be an operation name string".to_string())),
        };
        let mut acc = args[2].clone();
        for item in arr.iter_cloned() {
            acc = match op.as_str() {
                "add" | "+" => match (&acc, &item) {
                    (Value::Number(a), Value::Number(b)) => Value::Number(a + b),
                    (Value::Str(a), Value::Str(b)) => Value::Str(format!("{}{}", a, b)),
                    _ => Self::call("add", &[acc.clone(), item])?,
                },
                "sub" | "-" => match (&acc, &item) {
                    (Value::Number(a), Value::Number(b)) => Value::Number(a - b),
                    _ => return Err(VmError::runtime_error("sub requires numbers".to_string())),
                },
                "mul" | "*" => match (&acc, &item) {
                    (Value::Number(a), Value::Number(b)) => Value::Number(a * b),
                    _ => return Err(VmError::runtime_error("mul requires numbers".to_string())),
                },
                "min" => Self::call("min", &[acc.clone(), item])?,
                "max" => Self::call("max", &[acc.clone(), item])?,
                other => Self::call(other, &[acc.clone(), item])?,
            };
        }
        Ok(acc)
    }

    fn sin(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "sin() expects 1 argument".to_string(),
            ));
        }
        match &args[0] {
            Value::Number(n) => Ok(Value::Number(n.sin())),
            _ => Err(VmError::runtime_error(
                "sin() expects a number".to_string(),
            )),
        }
    }

    fn cos(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "cos() expects 1 argument".to_string(),
            ));
        }
        match &args[0] {
            Value::Number(n) => Ok(Value::Number(n.cos())),
            _ => Err(VmError::runtime_error(
                "cos() expects a number".to_string(),
            )),
        }
    }

    fn tan(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "tan() expects 1 argument".to_string(),
            ));
        }
        match &args[0] {
            Value::Number(n) => Ok(Value::Number(n.tan())),
            _ => Err(VmError::runtime_error(
                "tan() expects a number".to_string(),
            )),
        }
    }

    fn random(_args: &[Value]) -> Result<Value, VmError> {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};
        let mut hasher = RandomState::new().build_hasher();
        let nanos = match std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH) {
            Ok(duration) => duration.as_nanos() as u64,
            Err(_) => {
                // System clock error, use a fallback
                hasher.write_usize(hasher.finish() as usize);
                hasher.finish()
            }
        };
        hasher.write_u64(nanos);
        let hash = hasher.finish();
        Ok(Value::Number((hash as f64) / (u64::MAX as f64)))
    }

    // ===== Type Conversion Functions =====
    fn parse_int(args: &[Value]) -> Result<Value, VmError> {
        if args.is_empty() || args.len() > 2 {
            return Err(VmError::runtime_error(
                "parseInt() expects 1 or 2 arguments".to_string(),
            ));
        }
        match &args[0] {
            Value::Str(s) => {
                let trimmed = s.trim();
                let mut num_str = String::new();
                for ch in trimmed.chars() {
                    if ch.is_ascii_digit() || (num_str.is_empty() && (ch == '+' || ch == '-')) {
                        num_str.push(ch);
                    } else {
                        break;
                    }
                }
                if num_str.is_empty() || num_str == "+" || num_str == "-" {
                    Ok(Value::Number(f64::NAN))
                } else {
                    match num_str.parse::<i64>() {
                        Ok(n) => Ok(Value::Number(n as f64)),
                        Err(_) => Ok(Value::Number(f64::NAN)),
                    }
                }
            }
            Value::Number(n) => Ok(Value::Number(n.floor())),
            _ => Ok(Value::Number(f64::NAN)),
        }
    }

    fn parse_float(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "parseFloat() expects 1 argument".to_string(),
            ));
        }
        match &args[0] {
            Value::Str(s) => {
                match s.trim().parse::<f64>() {
                    Ok(n) => Ok(Value::Number(n)),
                    Err(_) => Ok(Value::Number(f64::NAN)),
                }
            }
            Value::Number(n) => Ok(Value::Number(*n)),
            _ => Ok(Value::Number(f64::NAN)),
        }
    }

    fn string_convert(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "String() expects 1 argument".to_string(),
            ));
        }
        match &args[0] {
            Value::Number(n) => Ok(Value::Str(n.to_string())),
            Value::Bool(b) => Ok(Value::Str(b.to_string())),
            Value::Str(s) => Ok(Value::Str(s.clone())),
            Value::Null => Ok(Value::Str("null".to_string())),
            _ => Ok(Value::Str(args[0].to_string())),
        }
    }

    fn number_convert(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "Number() expects 1 argument".to_string(),
            ));
        }
        match &args[0] {
            Value::Number(n) => Ok(Value::Number(*n)),
            Value::Bool(b) => Ok(Value::Number(if *b { 1.0 } else { 0.0 })),
            Value::Str(s) => {
                match s.trim().parse::<f64>() {
                    Ok(n) => Ok(Value::Number(n)),
                    Err(_) => Ok(Value::Number(f64::NAN)),
                }
            }
            Value::Null => Ok(Value::Number(0.0)),
            _ => Ok(Value::Number(f64::NAN)),
        }
    }

    fn boolean_convert(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "Boolean() expects 1 argument".to_string(),
            ));
        }
        Ok(Value::Bool(match &args[0] {
            Value::Bool(b) => *b,
            Value::Number(n) => *n != 0.0 && !n.is_nan(),
            Value::Str(s) => !s.is_empty(),
            Value::Null => false,
            _ => true,
        }))
    }

    // ===== Type Checks =====
    fn is_nan(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "isNaN() expects 1 argument".to_string(),
            ));
        }
        match &args[0] {
            Value::Number(n) => Ok(Value::Bool(n.is_nan())),
            _ => Ok(Value::Bool(false)),
        }
    }

    fn is_finite(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "isFinite() expects 1 argument".to_string(),
            ));
        }
        match &args[0] {
            Value::Number(n) => Ok(Value::Bool(n.is_finite())),
            _ => Ok(Value::Bool(false)),
        }
    }

    // ===== File I/O =====
    fn read_file(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_file_read()?;
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "readFile() expects 1 argument (filename)".to_string(),
            ));
        }
        match &args[0] {
            Value::Str(filename) => {
                match std::fs::read_to_string(filename) {
                    Ok(contents) => Ok(Value::Str(contents)),
                    Err(_) => Ok(Value::Null),
                }
            }
            _ => Err(VmError::runtime_error(
                "readFile() expects a string filename".to_string(),
            )),
        }
    }

    fn write_file(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_file_write()?;
        if args.len() != 2 {
            return Err(VmError::runtime_error(
                "writeFile() expects 2 arguments (filename, content)".to_string(),
            ));
        }
        match (&args[0], &args[1]) {
            (Value::Str(filename), Value::Str(content)) => {
                match std::fs::write(filename, content) {
                    Ok(_) => Ok(Value::Bool(true)),
                    Err(_) => Ok(Value::Bool(false)),
                }
            }
            _ => Err(VmError::runtime_error(
                "writeFile() expects string arguments (filename, content)".to_string(),
            )),
        }
    }

    // ===== Utility Functions =====
    fn interpolate(args: &[Value]) -> Result<Value, VmError> {
        if args.is_empty() {
            return Err(VmError::runtime_error(
                "interpolate() expects at least 1 argument (template string)".to_string(),
            ));
        }
        match &args[0] {
            Value::Str(template) => {
                let mut result = template.clone();
                for (i, arg) in args.iter().enumerate().skip(1) {
                    let placeholder = format!("{{{}}}", i - 1);
                    result = result.replace(&placeholder, &format!("{}", arg));
                }
                Ok(Value::Str(result))
            }
            _ => Err(VmError::runtime_error(
                "interpolate() expects a string as first argument".to_string(),
            )),
        }
    }

    fn dict_keys_iter(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "__dict_keys_iter() expects 1 argument".to_string(),
            ));
        }
        match &args[0] {
            Value::Dict(dict) => {
                let keys: Vec<Value> = dict
                    .keys()
                    .map(|k| Value::Str(k.clone()))
                    .collect();
                Ok(Value::from(keys))
            }
            Value::Array(arr) => {
                let indices: Vec<Value> = (0..arr.len())
                    .map(|i| Value::Number(i as f64))
                    .collect();
                Ok(Value::from(indices))
            }
            _ => Ok(Value::from(Vec::new())),
        }
    }

    fn next(args: &[Value]) -> Result<Value, VmError> {
        if args.is_empty() || args.len() > 2 {
            return Err(VmError::runtime_error(
                "next() expects 1 or 2 arguments".to_string(),
            ));
        }
        // This needs VM state access, handled separately
        Ok(Value::Null)
    }

    // ===== Timing Functions (Week 1: Curriculum Support) =====
    fn system_time_ms(_args: &[Value]) -> Result<Value, VmError> {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => {
                let millis = duration.as_millis() as f64;
                Ok(Value::Number(millis))
            }
            Err(_) => Err(VmError::runtime_error(
                "system_time_ms() failed to get current time".to_string(),
            )),
        }
    }

    // ===== Interactive I/O =====

    /// readline() â€” Read a line from stdin, returns trimmed string or Null on EOF
    fn readline(_args: &[Value]) -> Result<Value, VmError> {
        use std::io::{self, BufRead, Write};
        // Flush stdout so any prior print() without newline is visible
        let _ = io::stdout().flush();
        let stdin = io::stdin();
        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => Ok(Value::Null), // EOF
            Ok(_) => {
                // Trim trailing newline/carriage return
                let trimmed = line.trim_end_matches('\n')
                                  .trim_end_matches('\r')
                                  .to_string();
                Ok(Value::Str(trimmed))
            }
            Err(e) => Err(VmError::runtime_error(
                format!("readline() error: {}", e),
            )),
        }
    }

    /// readline_prompt(prompt) â€” Print prompt (no newline), then read a line from stdin
    fn readline_prompt(args: &[Value]) -> Result<Value, VmError> {
        use std::io::{self, BufRead, Write};
        if args.is_empty() {
            return Self::readline(args);
        }
        let prompt = format!("{}", args[0]);
        print!("{}", prompt);
        let _ = io::stdout().flush();
        let stdin = io::stdin();
        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => Ok(Value::Null), // EOF
            Ok(_) => {
                let trimmed = line.trim_end_matches('\n')
                                  .trim_end_matches('\r')
                                  .to_string();
                Ok(Value::Str(trimmed))
            }
            Err(e) => Err(VmError::runtime_error(
                format!("readline_prompt() error: {}", e),
            )),
        }
    }

    fn thread_sleep_ms(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "thread_sleep_ms() expects 1 argument (milliseconds)".to_string(),
            ));
        }

        match &args[0] {
            Value::Number(ms) => {
                let millis = (*ms as u64).max(0);
                std::thread::sleep(std::time::Duration::from_millis(millis));
                Ok(Value::Null)
            }
            _ => Err(VmError::runtime_error(
                "thread_sleep_ms() expects a number (milliseconds)".to_string(),
            )),
        }
    }
    
    // ===== Network Functions (Week 2: Curriculum Support) =====
    fn tcp_listener_bind(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_network()?;
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "TcpListener_bind() expects 1 argument (address)".to_string(),
            ));
        }
        
        match &args[0] {
            Value::Str(addr) => {
                // Placeholder: In integrated version, would use net.rs module
                // Currently returns a dictionary with listener metadata
                let mut listener = std::collections::HashMap::new();
                listener.insert("type".to_string(), Value::Str("TcpListener".to_string()));
                listener.insert("address".to_string(), Value::Str(addr.clone()));
                listener.insert("id".to_string(), Value::Number(1000.0)); // Mock handle ID
                
                Ok(Value::Dict(Box::new(listener)))
            }
            _ => Err(VmError::runtime_error(
                "TcpListener_bind(): address must be string".to_string(),
            )),
        }
    }
    
    fn tcp_listener_accept(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_network()?;
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "TcpListener_accept() expects 1 argument (listener)".to_string(),
            ));
        }
        
        // Placeholder: In integrated version, would accept connection
        let mut stream = std::collections::HashMap::new();
        stream.insert("type".to_string(), Value::Str("TcpStream".to_string()));
        stream.insert("remote_addr".to_string(), Value::Str("127.0.0.1:9999".to_string()));
        stream.insert("id".to_string(), Value::Number(2000.0)); // Mock handle ID
        
        Ok(Value::Dict(Box::new(stream)))
    }
    
    fn tcp_stream_read(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_network()?;
        if args.len() != 2 {
            return Err(VmError::runtime_error(
                "TcpStream_read() expects 2 arguments (stream, size)".to_string(),
            ));
        }
        
        match &args[1] {
            Value::Number(size) => {
                // Placeholder: In integrated version, would read from socket
                let num_bytes = (*size as usize).min(4096);
                // Return data as a string of null bytes
                let data = "\0".repeat(num_bytes);
                let mut result = std::collections::HashMap::new();
                result.insert("bytes_read".to_string(), Value::Number(num_bytes as f64));
                result.insert("data".to_string(), Value::Str(data));
                Ok(Value::Dict(Box::new(result)))
            }
            _ => Err(VmError::runtime_error(
                "TcpStream_read(): size must be number".to_string(),
            )),
        }
    }
    
    fn tcp_stream_write(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_network()?;
        if args.len() != 2 {
            return Err(VmError::runtime_error(
                "TcpStream_write() expects 2 arguments (stream, data)".to_string(),
            ));
        }
        
        let size = match &args[1] {
            Value::Str(s) => s.len(),
            _ => {
                return Err(VmError::runtime_error(
                    "TcpStream_write(): data must be string".to_string(),
                ))
            }
        };
        
        // Placeholder: In integrated version, would write to socket and return bytes written
        Ok(Value::Number(size as f64))
    }
    
    fn tcp_stream_close(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_network()?;
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "TcpStream_close() expects 1 argument (stream)".to_string(),
            ));
        }
        
        // Placeholder: In integrated version, would close socket connection
        Ok(Value::Null)
    }
    
    // ===== Threading Functions (Week 3: Curriculum Support) =====
    
    fn spawn_thread(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_process_spawn()?;
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "spawn_thread() expects 1 argument (closure/function)".to_string(),
            ));
        }
        
        // In Killer, closures are represented as Function values
        // For v2.3, we would:
        // 1. Clone the function bytecode
        // 2. Create a new VM instance in a thread
        // 3. Execute the function
        // 4. Return the thread handle ID
        
        // For v3.0 (current), return a thread handle dictionary
        use std::sync::atomic::{AtomicUsize, Ordering};
        
        // Generate unique thread ID
        static THREAD_COUNTER: AtomicUsize = AtomicUsize::new(0);
        let thread_id = THREAD_COUNTER.fetch_add(1, Ordering::SeqCst);
        let thread_handle_str = format!("thread_{}", thread_id);
        
        // Return thread handle
        let mut handle = std::collections::HashMap::new();
        handle.insert("type".to_string(), Value::Str("ThreadHandle".to_string()));
        handle.insert("id".to_string(), Value::Str(thread_handle_str));
        handle.insert("status".to_string(), Value::Str("running".to_string()));
        
        Ok(Value::Dict(Box::new(handle)))
    }
    
    fn join_thread(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "join_thread() expects 1 argument (thread handle)".to_string(),
            ));
        }
        
        match &args[0] {
            Value::Dict(handle) => {
                // Verify it's a thread handle
                if let Some(Value::Str(type_str)) = handle.get("type") {
                    if type_str != "ThreadHandle" {
                        return Err(VmError::runtime_error(
                            "join_thread(): argument must be a thread handle".to_string(),
                        ));
                    }
                } else {
                    return Err(VmError::runtime_error(
                        "join_thread(): argument must be a thread handle".to_string(),
                    ));
                }
                
                // In v3.0, wait for the thread to complete
                // For now, return null (thread would have completed)
                Ok(Value::Null)
            }
            _ => Err(VmError::runtime_error(
                "join_thread(): argument must be a thread handle (Dict)".to_string(),
            )),
        }
    }
    
    // ===== Async Functions (Week 4: Curriculum Support) =====
    
    fn async_spawn(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_process_spawn()?;
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "async_spawn() expects 1 argument (async closure/function)".to_string(),
            ));
        }
        
        // Create a future handle (similar to thread handle but for async tasks)
        use std::sync::atomic::{AtomicUsize, Ordering};
        
        static FUTURE_COUNTER: AtomicUsize = AtomicUsize::new(0);
        let future_id = FUTURE_COUNTER.fetch_add(1, Ordering::SeqCst);
        let future_handle_str = format!("future_{}", future_id);
        
        // Return future handle
        let mut handle = std::collections::HashMap::new();
        handle.insert("type".to_string(), Value::Str("Future".to_string()));
        handle.insert("id".to_string(), Value::Str(future_handle_str));
        handle.insert("status".to_string(), Value::Str("pending".to_string()));
        
        Ok(Value::Dict(Box::new(handle)))
    }
    
    fn async_await(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "async_await() expects 1 argument (future)".to_string(),
            ));
        }
        
        match &args[0] {
            Value::Dict(handle) => {
                // Verify it's a future handle
                if let Some(Value::Str(type_str)) = handle.get("type") {
                    if type_str != "Future" {
                        return Err(VmError::runtime_error(
                            "async_await(): argument must be a future".to_string(),
                        ));
                    }
                } else {
                    return Err(VmError::runtime_error(
                        "async_await(): argument must be a future".to_string(),
                    ));
                }
                
                // In v3.0, wait for the async task to complete
                // Returns the result of the async computation
                // For now, return null (task would have completed)
                Ok(Value::Null)
            }
            _ => Err(VmError::runtime_error(
                "async_await(): argument must be a future (Dict)".to_string(),
            )),
        }
    }

    // ========== DateTime Functions (Week 23) ==========

    fn now(_args: &[Value]) -> Result<Value, VmError> {
        use crate::datetime::KillerDateTime;
        
        let dt = KillerDateTime::now();
        let mut result = std::collections::HashMap::new();
        result.insert("type".to_string(), Value::Str("DateTime".to_string()));
        result.insert("seconds".to_string(), Value::Number(dt.seconds as f64));
        result.insert("nanos".to_string(), Value::Number(dt.nanos as f64));
        result.insert("year".to_string(), Value::Number(dt.year() as f64));
        result.insert("month".to_string(), Value::Number(dt.month() as f64));
        result.insert("day".to_string(), Value::Number(dt.day() as f64));
        result.insert("hour".to_string(), Value::Number(dt.hour() as f64));
        result.insert("minute".to_string(), Value::Number(dt.minute() as f64));
        result.insert("second".to_string(), Value::Number(dt.second() as f64));
        result.insert("weekday".to_string(), Value::Number(dt.weekday() as f64));
        result.insert("iso_string".to_string(), Value::Str(dt.to_iso_string()));
        
        Ok(Value::Dict(Box::new(result)))
    }

    fn parse_datetime(args: &[Value]) -> Result<Value, VmError> {
        use crate::datetime::parse_datetime;
        
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "parse_datetime() expects 1 argument (datetime string)".to_string(),
            ));
        }
        
        let datetime_str = match &args[0] {
            Value::Str(s) => s,
            _ => {
                return Err(VmError::runtime_error(
                    "parse_datetime(): argument must be a string".to_string(),
                ))
            }
        };

        match parse_datetime(datetime_str) {
            Ok(dt) => {
                let mut result = std::collections::HashMap::new();
                result.insert("type".to_string(), Value::Str("DateTime".to_string()));
                result.insert("seconds".to_string(), Value::Number(dt.seconds as f64));
                result.insert("nanos".to_string(), Value::Number(dt.nanos as f64));
                result.insert("year".to_string(), Value::Number(dt.year() as f64));
                result.insert("month".to_string(), Value::Number(dt.month() as f64));
                result.insert("day".to_string(), Value::Number(dt.day() as f64));
                result.insert("hour".to_string(), Value::Number(dt.hour() as f64));
                result.insert("minute".to_string(), Value::Number(dt.minute() as f64));
                result.insert("second".to_string(), Value::Number(dt.second() as f64));
                result.insert("weekday".to_string(), Value::Number(dt.weekday() as f64));
                result.insert("iso_string".to_string(), Value::Str(dt.to_iso_string()));
                
                Ok(Value::Dict(Box::new(result)))
            }
            Err(e) => Err(VmError::runtime_error(
                format!("parse_datetime(): {}", e)
            )),
        }
    }

    fn format_datetime(args: &[Value]) -> Result<Value, VmError> {
        use crate::datetime::KillerDateTime;
        
        if args.len() < 2 {
            return Err(VmError::runtime_error(
                "format_datetime() expects 2 arguments (datetime, format_string)".to_string(),
            ));
        }
        
        let dt_dict = match &args[0] {
            Value::Dict(d) => d,
            _ => {
                return Err(VmError::runtime_error(
                    "format_datetime(): first argument must be a DateTime".to_string(),
                ))
            }
        };

        let format_str = match &args[1] {
            Value::Str(s) => s,
            _ => {
                return Err(VmError::runtime_error(
                    "format_datetime(): second argument must be a string".to_string(),
                ))
            }
        };

        // Extract seconds and nanos from datetime dict
        let seconds = match dt_dict.get("seconds") {
            Some(Value::Number(n)) => *n as i64,
            _ => {
                return Err(VmError::runtime_error(
                    "format_datetime(): invalid DateTime object".to_string(),
                ))
            }
        };

        let nanos = match dt_dict.get("nanos") {
            Some(Value::Number(n)) => *n as u32,
            _ => 0,
        };

        let dt = KillerDateTime {
            seconds,
            nanos,
        };

        Ok(Value::Str(dt.format(format_str)))
    }

    // ========== Assert Builtins ==========

    fn assert_eq(args: &[Value]) -> Result<Value, VmError> {
        if args.len() < 2 {
            return Err(VmError::runtime_error("assert_eq(a, b) requires 2 arguments"));
        }
        let a = &args[0];
        let b = &args[1];
        if a == b {
            Ok(Value::Str("OK".to_string()))
        } else {
            Err(VmError::runtime_error(format!(
                "assert_eq failed: left={:?}  right={:?}", a, b
            )))
        }
    }

    fn assert_ne(args: &[Value]) -> Result<Value, VmError> {
        if args.len() < 2 {
            return Err(VmError::runtime_error("assert_ne(a, b) requires 2 arguments"));
        }
        let a = &args[0];
        let b = &args[1];
        if a != b {
            Ok(Value::Str("OK".to_string()))
        } else {
            Err(VmError::runtime_error(format!(
                "assert_ne failed: both sides equal {:?}", a
            )))
        }
    }

    fn assert_true(args: &[Value]) -> Result<Value, VmError> {
        match args.first() {
            Some(Value::Bool(true)) => Ok(Value::Str("OK".to_string())),
            Some(Value::Bool(false)) => Err(VmError::runtime_error("assert_true failed: got false")),
            Some(v) => Err(VmError::runtime_error(format!(
                "assert_true failed: expected Bool, got {:?}", v
            ))),
            None => Err(VmError::runtime_error("assert_true(cond) requires 1 argument")),
        }
    }

    fn assert_false(args: &[Value]) -> Result<Value, VmError> {
        match args.first() {
            Some(Value::Bool(false)) => Ok(Value::Str("OK".to_string())),
            Some(Value::Bool(true)) => Err(VmError::runtime_error("assert_false failed: got true")),
            Some(v) => Err(VmError::runtime_error(format!(
                "assert_false failed: expected Bool, got {:?}", v
            ))),
            None => Err(VmError::runtime_error("assert_false(cond) requires 1 argument")),
        }
    }

    fn assert_contains(args: &[Value]) -> Result<Value, VmError> {
        if args.len() < 2 {
            return Err(VmError::runtime_error("assert_contains(haystack, needle) requires 2 arguments"));
        }
        match (&args[0], &args[1]) {
            (Value::Str(haystack), Value::Str(needle)) => {
                if haystack.contains(needle.as_str()) {
                    Ok(Value::Str("OK".to_string()))
                } else {
                    Err(VmError::runtime_error(format!(
                        "assert_contains failed: {:?} not found in {:?}", needle, haystack
                    )))
                }
            }
            (Value::Array(list), needle) => {
                if list.iter_cloned().any(|v| v == *needle) {
                    Ok(Value::Str("OK".to_string()))
                } else {
                    Err(VmError::runtime_error(format!(
                        "assert_contains failed: {:?} not found in list", needle
                    )))
                }
            }
            _ => Err(VmError::runtime_error(
                "assert_contains(haystack, needle): haystack must be String or List"
            )),
        }
    }

    fn assert_nil(args: &[Value]) -> Result<Value, VmError> {
        match args.first() {
            Some(Value::Null) => Ok(Value::Str("OK".to_string())),
            Some(v) => Err(VmError::runtime_error(format!(
                "assert_nil failed: expected null, got {:?}", v
            ))),
            None => Err(VmError::runtime_error("assert_nil(val) requires 1 argument")),
        }
    }

    // ========== HTTP Functions (Week 23) ==========

    #[allow(dead_code)]
    fn http_get(args: &[Value]) -> Result<Value, VmError> {
        use crate::http::http_get_request;
        
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "http_get() expects 1 argument (url)".to_string(),
            ));
        }
        
        let url = match &args[0] {
            Value::Str(s) => s,
            _ => {
                return Err(VmError::runtime_error(
                    "http_get(): argument must be a URL string".to_string(),
                ))
            }
        };

        match http_get_request(url) {
            Ok(response) => {
                let mut result = std::collections::HashMap::new();
                result.insert("status".to_string(), Value::Number(response.status_code as f64));
                result.insert("body".to_string(), Value::Str(response.body));
                result.insert("type".to_string(), Value::Str("HttpResponse".to_string()));
                Ok(Value::Dict(Box::new(result)))
            }
            Err(e) => Err(VmError::runtime_error(format!("http_get(): {}", e))),
        }
    }

    #[allow(dead_code)]
    fn http_post(args: &[Value]) -> Result<Value, VmError> {
        use crate::http::http_post_request;
        
        if args.len() != 2 {
            return Err(VmError::runtime_error(
                "http_post() expects 2 arguments (url, body)".to_string(),
            ));
        }
        
        let url = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => {
                return Err(VmError::runtime_error(
                    "http_post(): first argument must be a URL string".to_string(),
                ))
            }
        };

        let body = match &args[1] {
            Value::Str(s) => s.clone(),
            Value::Dict(d) => {
                // If dict passed, convert to JSON representation
                let mut parts = Vec::new();
                for (key, value) in d.iter() {
                    let value_str = match value {
                        Value::Str(s) => format!("\"{}\"", s.replace("\"", "\\\"")),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        Value::Null => "null".to_string(),
                        _ => "null".to_string(),
                    };
                    parts.push(format!("\"{}\":{}", key, value_str));
                }
                format!("{{{}}}", parts.join(","))
            }
            _ => {
                return Err(VmError::runtime_error(
                    "http_post(): second argument must be string or dict".to_string(),
                ))
            }
        };

        match http_post_request(&url, &body) {
            Ok(response) => {
                let mut result = std::collections::HashMap::new();
                result.insert("status".to_string(), Value::Number(response.status_code as f64));
                result.insert("body".to_string(), Value::Str(response.body));
                result.insert("type".to_string(), Value::Str("HttpResponse".to_string()));
                Ok(Value::Dict(Box::new(result)))
            }
            Err(e) => Err(VmError::runtime_error(format!("http_post(): {}", e))),
        }
    }

    fn parse_json(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "parse_json() expects 1 argument (json string)".to_string(),
            ));
        }
        
        let json_str = match &args[0] {
            Value::Str(s) => s,
            _ => {
                return Err(VmError::runtime_error(
                    "parse_json(): argument must be a JSON string".to_string(),
                ))
            }
        };

        // Simple JSON parsing (delegate to http module)
        use crate::http::parse_json_basic;
        
        match parse_json_basic(json_str) {
            Ok(parsed) => {
                let mut result = std::collections::HashMap::new();
                for (key, value) in parsed {
                    // Try to parse as number
                    if let Ok(num) = value.parse::<f64>() {
                        result.insert(key, Value::Number(num));
                    } else if value == "true" {
                        result.insert(key, Value::Bool(true));
                    } else if value == "false" {
                        result.insert(key, Value::Bool(false));
                    } else {
                        result.insert(key, Value::Str(value));
                    }
                }
                Ok(Value::Dict(Box::new(result)))
            }
            Err(e) => Err(VmError::runtime_error(format!("parse_json(): {}", e))),
        }
    }

    fn json_stringify(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "json_stringify() expects 1 argument (dict_or_value)".to_string(),
            ));
        }
        
        let json_string = match &args[0] {
            Value::Dict(dict) => {
                let mut parts = Vec::new();
                for (key, value) in dict.iter() {
                    let value_str = match value {
                        Value::Str(s) => format!("\"{}\"", s.replace("\"", "\\\"")),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        Value::Null => "null".to_string(),
                        _ => format!("\"{}\"", value),
                    };
                    parts.push(format!("\"{}\":{}", key, value_str));
                }
                format!("{{{}}}", parts.join(","))
            }
            Value::Str(s) => format!("\"{}\"", s.replace("\"", "\\\"")),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            _ => format!("null"),
        };
        
        Ok(Value::Str(json_string))
    }

    fn http_server_new(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_network()?;
        use crate::http::KillerHttpServer;
        
        if args.len() != 2 {
            return Err(VmError::runtime_error(
                "HttpServer_new() expects 2 arguments (host, port)".to_string(),
            ));
        }
        
        let host = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => {
                return Err(VmError::runtime_error(
                    "HttpServer_new(): first argument must be a host string".to_string(),
                ))
            }
        };

        let port = match &args[1] {
            Value::Number(n) => *n as u16,
            _ => {
                return Err(VmError::runtime_error(
                    "HttpServer_new(): second argument must be a port number".to_string(),
                ))
            }
        };

        let server = KillerHttpServer::new(&host, port);
        
        let mut result = std::collections::HashMap::new();
        result.insert("type".to_string(), Value::Str("HttpServer".to_string()));
        result.insert("host".to_string(), Value::Str(server.host));
        result.insert("port".to_string(), Value::Number(server.port as f64));
        result.insert("running".to_string(), Value::Bool(false));
        
        Ok(Value::Dict(Box::new(result)))
    }

    fn http_server_listen(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_network()?;
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "HttpServer_listen() expects 1 argument (server)".to_string(),
            ));
        }
        
        let server_dict = match &args[0] {
            Value::Dict(d) => d,
            _ => {
                return Err(VmError::runtime_error(
                    "HttpServer_listen(): argument must be an HttpServer".to_string(),
                ))
            }
        };

        if let Some(Value::Str(server_type)) = server_dict.get("type") {
            if server_type != "HttpServer" {
                return Err(VmError::runtime_error(
                    "HttpServer_listen(): argument must be an HttpServer".to_string(),
                ));
            }
        } else {
            return Err(VmError::runtime_error(
                "HttpServer_listen(): invalid server object".to_string(),
            ));
        }

        // v3.0: Simulate server listening
        // In v3.1+, this would block on actual socket
        let mut result = server_dict.clone();
        result.insert("running".to_string(), Value::Bool(true));
        
        Ok(Value::Dict(result))
    }

    // ========== JSON/CSV Functions (Week 24) ==========

    fn json_pretty(args: &[Value]) -> Result<Value, VmError> {
        use crate::json_csv::json_pretty;
        
        if args.is_empty() || args.len() > 2 {
            return Err(VmError::runtime_error(
                "json_pretty() expects 1-2 arguments (json_string, [indent])".to_string(),
            ));
        }
        
        let json_str = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => {
                return Err(VmError::runtime_error(
                    "json_pretty(): first argument must be a JSON string".to_string(),
                ))
            }
        };

        let indent = if args.len() > 1 {
            match &args[1] {
                Value::Number(n) => *n as usize,
                _ => 2,
            }
        } else {
            2
        };

        match json_pretty(&json_str, indent) {
            Ok(pretty) => Ok(Value::Str(pretty)),
            Err(e) => Err(VmError::runtime_error(format!("json_pretty(): {}", e))),
        }
    }

    fn parse_csv(args: &[Value]) -> Result<Value, VmError> {
        use crate::json_csv::parse_csv as parse_csv_impl;
        
        if args.is_empty() || args.len() > 2 {
            return Err(VmError::runtime_error(
                "parse_csv() expects 1-2 arguments (csv_string, [delimiter])".to_string(),
            ));
        }
        
        let csv_str = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => {
                return Err(VmError::runtime_error(
                    "parse_csv(): first argument must be a CSV string".to_string(),
                ))
            }
        };

        let delimiter = if args.len() > 1 {
            match &args[1] {
                Value::Str(s) => {
                    if s.len() != 1 {
                        return Err(VmError::runtime_error(
                            "parse_csv(): delimiter must be a single character".to_string(),
                        ));
                    }
                    s.chars().next().unwrap()
                }
                _ => ',',
            }
        } else {
            ','
        };

        match parse_csv_impl(&csv_str, delimiter) {
            Ok(rows) => {
                // Convert to array of dicts
                let mut result = Vec::new();
                for row in rows {
                    let mut row_dict = std::collections::HashMap::new();
                    for (key, value) in row {
                        row_dict.insert(key, Value::Str(value));
                    }
                    result.push(Value::Dict(Box::new(row_dict)));
                }
                Ok(Value::from(result))
            }
            Err(e) => Err(VmError::runtime_error(format!("parse_csv(): {}", e))),
        }
    }

    fn to_csv(args: &[Value]) -> Result<Value, VmError> {
        use crate::json_csv::to_csv as to_csv_impl;
        
        if args.is_empty() || args.len() > 2 {
            return Err(VmError::runtime_error(
                "to_csv() expects 1-2 arguments (rows, [delimiter])".to_string(),
            ));
        }
        
        let delimiter = if args.len() > 1 {
            match &args[1] {
                Value::Str(s) => {
                    if s.len() != 1 {
                        return Err(VmError::runtime_error(
                            "to_csv(): delimiter must be a single character".to_string(),
                        ));
                    }
                    s.chars().next().unwrap()
                }
                _ => ',',
            }
        } else {
            ','
        };

        // Convert array of dicts to CSV rows
        let mut csv_rows = Vec::new();
        match &args[0] {
            Value::Array(arr) => {
                for item in arr.iter_cloned() {
                    if let Value::Dict(d) = item {
                        let mut row = std::collections::HashMap::new();
                        for (k, v) in d.iter() {
                            let val_str = match v {
                                Value::Str(s) => s.clone(),
                                Value::Number(n) => n.to_string(),
                                Value::Bool(b) => b.to_string(),
                                _ => String::new(),
                            };
                            row.insert(k.clone(), val_str);
                        }
                        csv_rows.push(row);
                    }
                }
            }
            _ => {
                return Err(VmError::runtime_error(
                    "to_csv(): first argument must be an array of dicts".to_string(),
                ))
            }
        }

        match to_csv_impl(&csv_rows, delimiter) {
            Ok(csv_str) => Ok(Value::Str(csv_str)),
            Err(e) => Err(VmError::runtime_error(format!("to_csv(): {}", e))),
        }
    }

    fn to_yaml(args: &[Value]) -> Result<Value, VmError> {
        use crate::json_csv::to_yaml;
        
        if args.is_empty() || args.len() > 2 {
            return Err(VmError::runtime_error(
                "to_yaml() expects 1-2 arguments (dict, [indent])".to_string(),
            ));
        }
        
        let indent = if args.len() > 1 {
            match &args[1] {
                Value::Number(n) => *n as usize,
                _ => 0,
            }
        } else {
            0
        };

        match &args[0] {
            Value::Dict(d) => {
                // Convert Value dict to string dict for YAML conversion
                let mut str_dict = std::collections::HashMap::new();
                for (k, v) in d.iter() {
                    let val_str = match v {
                        Value::Str(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        Value::Null => "null".to_string(),
                        _ => format!("{:?}", v),
                    };
                    str_dict.insert(k.clone(), val_str);
                }
                Ok(Value::Str(to_yaml(&str_dict, indent)))
            }
            _ => Err(VmError::runtime_error(
                "to_yaml(): first argument must be a dict".to_string(),
            )),
        }
    }

    fn websocket_new(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_network()?;
        use crate::websocket::{WebSocket, websocket_to_dict};
        
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "websocket_new() expects 1 argument (url)".to_string(),
            ));
        }

        match &args[0] {
            Value::Str(url) => {
                let ws = WebSocket::new(url);
                let dict = websocket_to_dict(&ws);
                
                // Create unique object reference
                let mut result_dict = std::collections::HashMap::new();
                for (k, v) in dict {
                    result_dict.insert(k, Value::Str(v));
                }
                result_dict.insert("__type".to_string(), Value::Str("WebSocket".to_string()));
                Ok(Value::Dict(Box::new(result_dict)))
            }
            _ => Err(VmError::runtime_error(
                "websocket_new(): argument must be a URL string".to_string(),
            )),
        }
    }

    fn websocket_server_new(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_network()?;
        use crate::websocket::{WebSocketServer, server_to_dict};
        
        if args.len() != 2 {
            return Err(VmError::runtime_error(
                "websocket_server_new() expects 2 arguments (host, port)".to_string(),
            ));
        }

        match (&args[0], &args[1]) {
            (Value::Str(host), Value::Number(port)) => {
                let server = WebSocketServer::new(host, *port as u16);
                let dict = server_to_dict(&server);
                
                let mut result_dict = std::collections::HashMap::new();
                for (k, v) in dict {
                    result_dict.insert(k, Value::Str(v));
                }
                result_dict.insert("__type".to_string(), Value::Str("WebSocketServer".to_string()));
                Ok(Value::Dict(Box::new(result_dict)))
            }
            _ => Err(VmError::runtime_error(
                "websocket_server_new(): arguments must be (host: string, port: number)".to_string(),
            )),
        }
    }

    fn ws_connect(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_network()?;
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "ws_connect() expects 1 argument (websocket)".to_string(),
            ));
        }

        match &args[0] {
            Value::Dict(ws_dict) => {
                // Simulate connection for v3.0
                if ws_dict.contains_key("url") {
                    let mut result = ws_dict.clone();
                    result.insert("state".to_string(), Value::Str("connected".to_string()));
                    Ok(Value::Dict(result))
                } else {
                    Err(VmError::runtime_error(
                        "ws_connect(): invalid WebSocket object".to_string(),
                    ))
                }
            }
            _ => Err(VmError::runtime_error(
                "ws_connect(): argument must be a WebSocket object".to_string(),
            )),
        }
    }

    fn ws_send(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_network()?;
        if args.len() != 2 {
            return Err(VmError::runtime_error(
                "ws_send() expects 2 arguments (websocket, message)".to_string(),
            ));
        }

        match (&args[0], &args[1]) {
            (Value::Dict(_ws_dict), Value::Str(message)) => {
                // Simulate message sending for v3.0
                let result = std::collections::HashMap::from([
                    ("status".to_string(), Value::Str("sent".to_string())),
                    ("message".to_string(), Value::Str(message.clone())),
                    ("timestamp".to_string(), Value::Str("2026-03-14T00:00:00".to_string())),
                ]);
                Ok(Value::Dict(Box::new(result)))
            }
            _ => Err(VmError::runtime_error(
                "ws_send(): arguments must be (websocket, message: string)".to_string(),
            )),
        }
    }

    fn ws_receive(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_network()?;
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "ws_receive() expects 1 argument (websocket)".to_string(),
            ));
        }

        match &args[0] {
            Value::Dict(_ws_dict) => {
                // Simulate message receiving for v3.0
                let result = std::collections::HashMap::from([
                    ("type".to_string(), Value::Str("message".to_string())),
                    ("data".to_string(), Value::Str("[simulated message]".to_string())),
                    ("timestamp".to_string(), Value::Str("2026-03-14T00:00:00".to_string())),
                ]);
                Ok(Value::Dict(Box::new(result)))
            }
            _ => Err(VmError::runtime_error(
                "ws_receive(): argument must be a WebSocket object".to_string(),
            )),
        }
    }

    fn ws_disconnect(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_network()?;
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "ws_disconnect() expects 1 argument (websocket)".to_string(),
            ));
        }

        match &args[0] {
            Value::Dict(ws_dict) => {
                // Simulate disconnection for v3.0
                let mut result = ws_dict.clone();
                result.insert("state".to_string(), Value::Str("disconnected".to_string()));
                Ok(Value::Dict(result))
            }
            _ => Err(VmError::runtime_error(
                "ws_disconnect(): argument must be a WebSocket object".to_string(),
            )),
        }
    }

    fn trait_new(args: &[Value]) -> Result<Value, VmError> {
        if args.len() < 1 || args.len() > 2 {
            return Err(VmError::runtime_error(
                "trait_new() expects 1-2 arguments (name, [methods])".to_string(),
            ));
        }

        match &args[0] {
            Value::Str(trait_name) => {
                // Create trait definition
                let methods = if args.len() > 1 {
                    match &args[1] {
                        Value::Array(method_list) => {
                            let mut method_names = Vec::new();
                            for method in method_list.iter_cloned() {
                                if let Value::Str(m) = method {
                                    method_names.push(m.clone());
                                }
                            }
                            method_names
                        }
                        _ => Vec::new(),
                    }
                } else {
                    Vec::new()
                };

                let mut result = std::collections::HashMap::new();
                result.insert("name".to_string(), Value::Str(trait_name.clone()));
                result.insert(
                    "methods".to_string(),
                    Value::from(
                        methods
                            .iter()
                            .map(|m| Value::Str(m.clone()))
                            .collect::<Vec<_>>(),
                    ),
                );
                result.insert("type".to_string(), Value::Str("Trait".to_string()));
                Ok(Value::Dict(Box::new(result)))
            }
            _ => Err(VmError::runtime_error(
                "trait_new(): first argument must be a trait name".to_string(),
            )),
        }
    }

    fn trait_impl(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 {
            return Err(VmError::runtime_error(
                "trait_impl() expects 2 arguments (trait, for_type)".to_string(),
            ));
        }

        match (&args[0], &args[1]) {
            (Value::Str(trait_name), Value::Str(for_type)) => {
                // Create trait implementation
                let mut result = std::collections::HashMap::new();
                result.insert("trait".to_string(), Value::Str(trait_name.clone()));
                result.insert("for_type".to_string(), Value::Str(for_type.clone()));
                result.insert("type".to_string(), Value::Str("TraitImpl".to_string()));
                result.insert("status".to_string(), Value::Str("implemented".to_string()));
                Ok(Value::Dict(Box::new(result)))
            }
            _ => Err(VmError::runtime_error(
                "trait_impl(): arguments must be (trait: string, for_type: string)".to_string(),
            )),
        }
    }

    fn trait_check(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 {
            return Err(VmError::runtime_error(
                "trait_check() expects 2 arguments (type, trait)".to_string(),
            ));
        }

        match (&args[0], &args[1]) {
            (Value::Str(type_name), Value::Str(trait_name)) => {
                // Simulate trait check - in v3.0, always check standard traits
                let implements = match (type_name.as_str(), trait_name.as_str()) {
                    ("String", "Display") => true,
                    ("String", "Cloneable") => true,
                    ("Number", "Comparable") => true,
                    ("Bool", "Display") => true,
                    ("Array", "Iterable") => true,
                    ("Dict", "Cloneable") => true,
                    _ => false,
                };

                Ok(Value::Bool(implements))
            }
            _ => Err(VmError::runtime_error(
                "trait_check(): arguments must be (type: string, trait: string)".to_string(),
            )),
        }
    }

    fn trait_resolve(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 {
            return Err(VmError::runtime_error(
                "trait_resolve() expects 2 arguments (type, method)".to_string(),
            ));
        }

        match (&args[0], &args[1]) {
            (Value::Str(type_name), Value::Str(method_name)) => {
                // Resolve method through trait polymorphism
                let resolution = match (type_name.as_str(), method_name.as_str()) {
                    ("String", "to_string") => ("Display", "to_string"),
                    ("Number", "compare_to") => ("Comparable", "compare_to"),
                    ("Number", "equals") => ("Comparable", "equals"),
                    ("Array", "iterator") => ("Iterable", "iterator"),
                    ("Array", "has_next") => ("Iterable", "has_next"),
                    _ => {
                        return Ok(Value::Null);
                    }
                };

                let mut result = std::collections::HashMap::new();
                result.insert("type".to_string(), Value::Str(type_name.clone()));
                result.insert("method".to_string(), Value::Str(method_name.clone()));
                result.insert("trait".to_string(), Value::Str(resolution.0.to_string()));
                result.insert("resolved".to_string(), Value::Bool(true));
                Ok(Value::Dict(Box::new(result)))
            }
            _ => Err(VmError::runtime_error(
                "trait_resolve(): arguments must be (type: string, method: string)".to_string(),
            )),
        }
    }

    // ===== AI Functions (v3.2) â€” wired to native AIRuntime =====

    fn ai_generate(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        if args.len() < 1 || args.len() > 2 {
            return Err(VmError::runtime_error(
                "ai_generate expects 1-2 arguments (prompt, options)".to_string(),
            ));
        }
        let prompt = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("ai_generate: prompt must be a string".to_string())),
        };
        let options = if args.len() > 1 {
            match &args[1] {
                Value::Dict(d) => (**d).clone(),
                _ => std::collections::HashMap::new(),
            }
        } else {
            std::collections::HashMap::new()
        };
        let mut runtime = crate::ai::AIRuntime::new();
        runtime.ai_generate(&prompt, options)
            .map(Value::Str)
            .map_err(VmError::runtime_error)
    }

    fn ai_embed(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        if args.len() < 1 || args.len() > 2 {
            return Err(VmError::runtime_error("ai_embed expects 1-2 arguments (text, model)".to_string()));
        }
        let text = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("ai_embed: text must be a string".to_string())),
        };
        let model = if args.len() > 1 {
            match &args[1] { Value::Str(s) => s.clone(), _ => "text-embedding-ada-002".to_string() }
        } else {
            "text-embedding-ada-002".to_string()
        };
        let mut runtime = crate::ai::AIRuntime::new();
        runtime.ai_embed(&text, &model)
            .map(|v| {
                Value::from(
                    v.into_iter()
                        .map(|f| Value::Number(f as f64))
                        .collect::<Vec<_>>(),
                )
            })
            .map_err(VmError::runtime_error)
    }

    fn ai_classify(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        if args.len() < 2 || args.len() > 3 {
            return Err(VmError::runtime_error("ai_classify expects 2-3 arguments (text, categories, model)".to_string()));
        }
        let text = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("ai_classify: text must be a string".to_string())),
        };
        let categories = match &args[1] {
            Value::Array(arr) => arr
                .iter_cloned()
                .filter_map(|v| match v {
                    Value::Str(s) => Some(s.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            _ => return Err(VmError::runtime_error("ai_classify: categories must be an array".to_string())),
        };
        let model = if args.len() > 2 {
            match &args[2] { Value::Str(s) => s.clone(), _ => "default".to_string() }
        } else {
            "default".to_string()
        };
        let mut runtime = crate::ai::AIRuntime::new();
        runtime.ai_classify(&text, categories, &model)
            .map(|r| {
                let mut result = std::collections::HashMap::new();
                result.insert("category".to_string(), Value::Str(r.category));
                result.insert("confidence".to_string(), Value::Number(r.confidence));
                let scores: std::collections::HashMap<String, Value> = r.all_scores
                    .into_iter().map(|(k, v)| (k, Value::Number(v))).collect();
                result.insert("all_scores".to_string(), Value::Dict(Box::new(scores)));
                Value::Dict(Box::new(result))
            })
            .map_err(VmError::runtime_error)
    }

    fn ai_extract(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        if args.len() < 2 || args.len() > 3 {
            return Err(VmError::runtime_error("ai_extract expects 2-3 arguments (text, schema, model)".to_string()));
        }
        let text = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("ai_extract: text must be a string".to_string())),
        };
        let schema: std::collections::HashMap<String, String> = match &args[1] {
            Value::Dict(d) => d.iter().filter_map(|(k, v)| match v {
                Value::Str(s) => Some((k.clone(), s.clone())), _ => None,
            }).collect(),
            _ => return Err(VmError::runtime_error("ai_extract: schema must be a dictionary".to_string())),
        };
        let model = if args.len() > 2 {
            match &args[2] { Value::Str(s) => s.clone(), _ => "default".to_string() }
        } else {
            "default".to_string()
        };
        let mut runtime = crate::ai::AIRuntime::new();
        runtime.ai_extract(&text, schema, &model)
            .map(|d| Value::Dict(Box::new(d)))
            .map_err(VmError::runtime_error)
    }

    fn ai_local_infer(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        if args.len() != 2 {
            return Err(VmError::runtime_error("ai_local_infer expects 2 arguments (model_path, input)".to_string()));
        }
        let model_path = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("ai_local_infer: model_path must be a string".to_string())),
        };
        let input = match &args[1] {
            Value::Dict(d) => (**d).clone(),
            _ => return Err(VmError::runtime_error("ai_local_infer: input must be a dictionary".to_string())),
        };
        let mut runtime = crate::ai::AIRuntime::new();
        runtime.ai_local_infer(&model_path, input)
            .map(|d| Value::Dict(Box::new(d)))
            .map_err(VmError::runtime_error)
    }

    fn ai_provider_set(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        if args.len() != 2 {
            return Err(VmError::runtime_error("ai_provider_set expects 2 arguments (provider, config)".to_string()));
        }
        let provider = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("ai_provider_set: provider must be a string".to_string())),
        };
        let config = match &args[1] {
            Value::Dict(d) => (**d).clone(),
            _ => return Err(VmError::runtime_error("ai_provider_set: config must be a dictionary".to_string())),
        };
        let mut runtime = crate::ai::AIRuntime::new();
        runtime.set_provider_config(&provider, config)
            .map(|_| Value::Bool(true))
            .map_err(VmError::runtime_error)
    }

    fn ai_provider_get(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        if args.len() != 1 {
            return Err(VmError::runtime_error("ai_provider_get expects 1 argument (provider)".to_string()));
        }
        let provider = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("ai_provider_get: provider must be a string".to_string())),
        };
        let runtime = crate::ai::AIRuntime::new();
        let info = runtime.get_provider_config(&provider)
            .unwrap_or_else(|_| {
                let mut m = std::collections::HashMap::new();
                m.insert("name".to_string(), Value::Str(provider));
                m.insert("status".to_string(), Value::Str("not_configured".to_string()));
                m
            });
        Ok(Value::Dict(Box::new(info)))
    }

    fn ai_cache_enable(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        if args.len() != 1 {
            return Err(VmError::runtime_error("ai_cache_enable expects 1 argument (cache_type)".to_string()));
        }
        // Cache is always enabled on AIRuntime::new(); this is a no-op confirmation
        Ok(Value::Bool(true))
    }

    fn ai_cache_clear(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        if !args.is_empty() {
            return Err(VmError::runtime_error("ai_cache_clear expects 0 arguments".to_string()));
        }
        // Each call gets its own runtime; clearing is implicit
        Ok(Value::Null)
    }

    // --- Native GGUF Inference -----------------------------------------------
    // These call the killer-native transformer inference engine directly.
    // No Ollama, no cloud, no API keys â€” pure local inference.

    /// llm_chat(model_path, question)           â†’ String
    /// llm_chat(model_path, question, max_tokens) â†’ String
    ///
    /// Applies the correct chat template for the model (ChatML / TinyLlama /
    /// Mistral / Phi-3) and generates a response.
    ///
    /// Example Killer code:
    ///   answer = llm_chat("~/.killer/models/qwen2.5-0.5b-instruct-q4_k_m.gguf", "What is 2+2?")
    ///   print(answer)     // â†’ "2+2 equals 4."
    ///
    /// Optional: env `KILLER_KV_Q8=1` enables int8 KV cache (~4Ã— smaller KV RAM vs f32; same goal as TurboQuant-style compression).
    fn llm_chat(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        if args.len() < 2 || args.len() > 3 {
            return Err(VmError::runtime_error(
                "llm_chat expects 2-3 arguments: llm_chat(model_path, question [, max_tokens])".to_string(),
            ));
        }
        let model_path = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("llm_chat: model_path must be a string".to_string())),
        };
        let question = match &args[1] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("llm_chat: question must be a string".to_string())),
        };
        let max_tokens = if args.len() == 3 {
            match &args[2] {
                Value::Number(n) => *n as usize,
                _ => return Err(VmError::runtime_error("llm_chat: max_tokens must be a number".to_string())),
            }
        } else {
            512
        };
        crate::inference::killer_chat_auto(&model_path, &question, None, max_tokens)
            .map(Value::Str)
            .map_err(VmError::runtime_error)
    }

    /// llm_ask(model_path, raw_prompt)            â†’ String
    /// llm_ask(model_path, raw_prompt, max_tokens) â†’ String
    ///
    /// Sends the prompt directly to the model with no chat-template wrapping.
    /// Use this when you want full control over the prompt format.
    ///
    /// Optional: set env `KILLER_KV_Q8=1` to compress the attention KV cache (~4Ã— less KV RAM;
    /// TurboQuant-style goal; int8 + scales â€” see `inference/mod.rs`).
    fn llm_ask(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        if args.len() < 2 || args.len() > 3 {
            return Err(VmError::runtime_error(
                "llm_ask expects 2-3 arguments: llm_ask(model_path, prompt [, max_tokens])".to_string(),
            ));
        }
        let model_path = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("llm_ask: model_path must be a string".to_string())),
        };
        let prompt = match &args[1] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("llm_ask: prompt must be a string".to_string())),
        };
        let max_tokens = if args.len() == 3 {
            match &args[2] {
                Value::Number(n) => *n as usize,
                _ => return Err(VmError::runtime_error("llm_ask: max_tokens must be a number".to_string())),
            }
        } else {
            512
        };
        crate::inference::killer_ask(&model_path, &prompt, max_tokens)
            .map(Value::Str)
            .map_err(VmError::runtime_error)
    }

    /// llm_info(model_path) â†’ String
    ///
    /// Returns a human-readable summary of the model (arch, layers, params, quant).
    fn llm_info(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "llm_info expects 1 argument: llm_info(model_path)".to_string(),
            ));
        }
        let model_path = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("llm_info: model_path must be a string".to_string())),
        };
        crate::inference::killer_model_info(&model_path)
            .map(Value::Str)
            .map_err(VmError::runtime_error)
    }

    /// ghost_ask(model_path, question)              â†’ String
    /// ghost_ask(model_path, question, max_tokens)  â†’ String
    ///
    /// Web-grounded LLM answer:
    ///   1. Math detected â†’ compute natively (exact)
    ///   2. Search DuckDuckGo instant answers (free, no API key)
    ///   3. Search Wikipedia as fallback
    ///   4. Inject facts as context into the LLM prompt
    ///   5. Ask the local model with the grounded context
    ///
    /// Example:
    ///   answer = ghost_ask(model, "What is the capital of France?")
    ///   print(K"Ghost> {answer}")
    fn ghost_ask(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        crate::security::require_network()?;
        if args.len() < 2 || args.len() > 3 {
            return Err(VmError::runtime_error(
                "ghost_ask expects 2-3 arguments: ghost_ask(model_path, question [, max_tokens])".to_string(),
            ));
        }
        let model_path = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("ghost_ask: model_path must be a string".to_string())),
        };
        let question = match &args[1] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("ghost_ask: question must be a string".to_string())),
        };
        let max_tokens = if args.len() == 3 {
            match &args[2] {
                Value::Number(n) => *n as usize,
                _ => return Err(VmError::runtime_error("ghost_ask: max_tokens must be a number".to_string())),
            }
        } else {
            256
        };
        crate::llm::ghost_ask(&model_path, &question, max_tokens)
            .map(Value::Str)
            .map_err(VmError::runtime_error)
    }

    /// ghost_smart_solve(model_path, question [, max_rounds [, max_tokens]]) â†’ String
    ///
    /// Hybrid Smart Ghost: exact math and retrieval first, then an LLM loop with
    /// `VERIFY_EXPR` (closed-form check) and optional `NUMERIC_ROOT` (bisection).
    /// Returns a trace plus the best-scoring attempt.
    fn ghost_smart_solve(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        crate::security::require_network()?;
        if args.len() < 2 || args.len() > 4 {
            return Err(VmError::runtime_error(
                "ghost_smart_solve expects 2-4 arguments: ghost_smart_solve(model_path, question [, max_rounds [, max_tokens]])"
                    .to_string(),
            ));
        }
        let model_path = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error(
                "ghost_smart_solve: model_path must be a string".to_string(),
            )),
        };
        let question = match &args[1] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error(
                "ghost_smart_solve: question must be a string".to_string(),
            )),
        };
        let max_rounds = if args.len() >= 3 {
            match &args[2] {
                Value::Number(n) => *n as usize,
                _ => return Err(VmError::runtime_error(
                    "ghost_smart_solve: max_rounds must be a number".to_string(),
                )),
            }
        } else {
            4usize
        };
        let max_tokens = if args.len() >= 4 {
            match &args[3] {
                Value::Number(n) => *n as usize,
                _ => return Err(VmError::runtime_error(
                    "ghost_smart_solve: max_tokens must be a number".to_string(),
                )),
            }
        } else {
            384usize
        };
        crate::llm::ghost_smart_solve(&model_path, &question, max_rounds, max_tokens)
            .map(Value::Str)
            .map_err(VmError::runtime_error)
    }

    // -- Native Think Engine ---------------------------------------

    /// native_think(question)  â†’ String
    ///
    /// Killer's own deterministic reasoning engine (rules + KB + optional quick search).
    /// Strong on structured math, units, and short facts; not a universal solver.
    ///
    /// Handles:
    ///   â€¢ Pure arithmetic:         native_think("What is 15% of 240?")
    ///   â€¢ Unit conversions:        native_think("How many km in 50 miles?")
    ///   â€¢ Temperature:             native_think("What is 98.6F in Celsius?")
    ///   â€¢ Speed/time/distance:     native_think("How long to drive 300km at 90km/h?")
    ///   â€¢ Fact lookup:             native_think("Who is Alan Turing?")
    fn native_think(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "native_think expects 1 argument: native_think(question)".to_string(),
            ));
        }
        let question = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("native_think: question must be a string".to_string())),
        };
        Ok(Value::Str(crate::llm::native_think(&question)))
    }

    /// math_eval(expression) â†’ Number
    ///
    /// Pure arithmetic: `+ - * / % ^`, parentheses, unary `+`/`-`, scientific `1e-6`.
    /// Strips common English prefixes (`what is`, `solve`, â€¦) like `native_think`.
    fn math_eval(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "math_eval expects 1 argument: math_eval(\"2+3*4\") or math_eval(\"What is 2^10?\")"
                    .to_string(),
            ));
        }
        match &args[0] {
            Value::Number(n) => Ok(Value::Number(*n)),
            Value::Str(s) => crate::llm::try_eval_arithmetic_string(s)
                .map(Value::Number)
                .map_err(VmError::runtime_error),
            _ => Err(VmError::runtime_error(
                "math_eval: argument must be a string or number".to_string(),
            )),
        }
    }

    /// math_eval_subst(expression, var_name, value) â†’ Number
    ///
    /// One variable: e.g. `math_eval_subst("2*x + 1", "x", 5)` â†’ 11. Names are case-insensitive; avoid `e` as a name.
    fn math_eval_subst(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 3 {
            return Err(VmError::runtime_error(
                "math_eval_subst expects 3 args: math_eval_subst(\"2*x+1\", \"x\", 5)".to_string(),
            ));
        }
        let expr = match &args[0] {
            Value::Str(s) => s.as_str(),
            _ => {
                return Err(VmError::runtime_error(
                    "math_eval_subst: first arg must be expression string".to_string(),
                ))
            }
        };
        let var = match &args[1] {
            Value::Str(s) => s.as_str(),
            _ => {
                return Err(VmError::runtime_error(
                    "math_eval_subst: second arg must be variable name string".to_string(),
                ))
            }
        };
        let val = match &args[2] {
            Value::Number(n) => *n,
            Value::Str(s) => s
                .parse::<f64>()
                .map_err(|_| VmError::runtime_error("math_eval_subst: third arg must be a number".to_string()))?,
            _ => {
                return Err(VmError::runtime_error(
                    "math_eval_subst: third arg must be a number".to_string(),
                ))
            }
        };
        crate::llm::try_eval_arithmetic_subst_var(expr, var, val)
            .map(Value::Number)
            .map_err(VmError::runtime_error)
    }

    /// llm_reason(model_path, question) â†’ String
    /// llm_reason(model_path, question, max_tokens) â†’ String
    ///
    /// Turns any standard LLM (Qwen, TinyLlama, Mistral, Llama)
    /// into a reasoning model using a chain-of-thought system prompt.
    /// Returns the full display: thinking trace + final answer.
    ///
    /// No DeepSeek-R1 or special reasoning model required.
    ///
    /// Example:
    ///   model = "~/.killer/models/qwen2.5-0.5b-instruct-q4_k_m.gguf"
    ///   result = llm_reason(model, "What is 17 * 23?", 512)
    ///   print(result)
    fn llm_reason(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        if args.len() < 2 || args.len() > 3 {
            return Err(VmError::runtime_error(
                "llm_reason expects 2-3 args: llm_reason(model_path, question [, max_tokens])".to_string(),
            ));
        }
        let model_path = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("llm_reason: model_path must be a string".to_string())),
        };
        let question = match &args[1] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("llm_reason: question must be a string".to_string())),
        };
        let max_tokens = if args.len() == 3 {
            match &args[2] {
                Value::Number(n) => *n as usize,
                _ => return Err(VmError::runtime_error("llm_reason: max_tokens must be a number".to_string())),
            }
        } else { 512 };
        crate::inference::killer_llm_as_rlm(&model_path, &question, max_tokens)
            .map(|r| Value::Str(r.display()))
            .map_err(VmError::runtime_error)
    }

    /// llm_reason_answer(model_path, question) â†’ String
    /// llm_reason_answer(model_path, question, max_tokens) â†’ String
    ///
    /// Same as llm_reason but returns ONLY the final answer â€” no thinking trace.
    /// Use this when you want a clean value to store or pass to another function.
    ///
    /// Example:
    ///   ans = llm_reason_answer(model, "Fastest sorting algorithm?", 256)
    ///   print(K"Answer: {ans}")
    fn llm_reason_answer(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        if args.len() < 2 || args.len() > 3 {
            return Err(VmError::runtime_error(
                "llm_reason_answer expects 2-3 args: llm_reason_answer(model_path, question [, max_tokens])".to_string(),
            ));
        }
        let model_path = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("llm_reason_answer: model_path must be a string".to_string())),
        };
        let question = match &args[1] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("llm_reason_answer: question must be a string".to_string())),
        };
        let max_tokens = if args.len() == 3 {
            match &args[2] {
                Value::Number(n) => *n as usize,
                _ => return Err(VmError::runtime_error("llm_reason_answer: max_tokens must be a number".to_string())),
            }
        } else { 512 };
        crate::inference::killer_llm_as_rlm(&model_path, &question, max_tokens)
            .map(|r| Value::Str(r.answer_only().to_string()))
            .map_err(VmError::runtime_error)
    }

    /// ghost_108(question) â†’ String
    ///
    /// Fires all search agents simultaneously in parallel threads.
    /// The fastest agent that returns a quality result wins.
    /// Math/unit questions return instantly without any network.
    ///
    /// Example:
    ///   result = ghost_108("Who is Sai Arun Kumar Katherashala?")
    ///   print(result)
    fn ghost_108(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        crate::security::require_network()?;
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "ghost_108 expects 1 argument: ghost_108(question)".to_string(),
            ));
        }
        let question = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("ghost_108: question must be a string".to_string())),
        };
        Ok(Value::Str(crate::llm::ghost_108(&question)))
    }

    /// khlm_ask(question) â†’ String
    ///
    /// KhLM â€” Killer Hybrid Language Model unified smart router.
    /// Automatically picks the best engine for every question:
    ///
    ///   Tier 1 â€” Deterministic  (0ms, no network, no model)
    ///     Math, percentages, unit conversions, speed/time/distance
    ///
    ///   Tier 2 â€” Live Web  (~200ms, parallel agents)
    ///     Facts, people, news, real-time data
    ///
    /// Example:
    ///   print(khlm_ask("What is 15% of 480?"))     -- instant
    ///   print(khlm_ask("Who invented the internet?")) -- web
    fn khlm_ask(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "khlm_ask expects 1 argument: khlm_ask(question)".to_string(),
            ));
        }
        let question = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("khlm_ask: question must be a string".to_string())),
        };
        Ok(Value::Str(crate::llm::khlm_ask(&question)))
    }

    /// khlm_ask_model(model_path, question) â†’ String
    ///
    /// KhLM with Tier 3 Neural engine.
    /// Fires Tier 1 (deterministic), Tier 2 (web), AND Tier 3 (local .gguf model)
    /// simultaneously in parallel â€” the fastest quality result wins.
    ///
    /// Best with DeepSeek-R1, Qwen2.5, Mistral, or any GGUF model.
    ///
    /// Example:
    ///   model = "~/.killer/models/deepseek-r1-7b.gguf"
    ///   print(khlm_ask_model(model, "Explain Fourier transforms"))
    fn khlm_ask_model(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        if args.len() != 2 {
            return Err(VmError::runtime_error(
                "khlm_ask_model expects 2 arguments: khlm_ask_model(model_path, question)".to_string(),
            ));
        }
        let model_path = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("khlm_ask_model: model_path must be a string".to_string())),
        };
        let question = match &args[1] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("khlm_ask_model: question must be a string".to_string())),
        };
        Ok(Value::Str(crate::llm::khlm_ask_model(&model_path, &question)))
    }

    /// khlm_prefetch(question) â†’ Nil
    /// Fires all KhLM agents in the background immediately â€” call at program start.
    /// By the time khlm_ask() runs, the cache is already populated â†’ ns lookup.
    /// Example:
    ///   khlm_prefetch("who is Deepthi Sudha Katherasala")
    ///   // ... do other work ...
    ///   r = khlm_ask("who is Deepthi Sudha Katherasala")  // instant from cache!
    fn khlm_prefetch(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "khlm_prefetch expects 1 argument: khlm_prefetch(question)".to_string(),
            ));
        }
        let question = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("khlm_prefetch: question must be a string".to_string())),
        };
        crate::llm::khlm_prefetch(&question);
        Ok(Value::Null)
    }

    /// khlm_ai_system(question) â†’ String
    ///
    /// **Killer AI System** â€” for **hard** questions: KhLM router + Ghost-108 + local GGUF + merged verdict.
    ///
    /// - **GGUF:** use a **reasoning** model (e.g. R1-style) via `KILLER_KHLM_GGUF` or `~/.killer/models/*.gguf` for best neural + synthesis.
    /// - **Honest scope:** advanced **orchestration + merging**, not AGI.
    ///
    /// Runs three agents in parallel, then coordinator synthesis when a default GGUF exists.
    /// Use from Killer code or Kala mode `ai_system` / `multi_agent`.
    fn khlm_ai_system(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "khlm_ai_system expects 1 argument: khlm_ai_system(question)".to_string(),
            ));
        }
        let question = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("khlm_ai_system: question must be a string".to_string())),
        };
        Ok(Value::Str(crate::llm::khlm_ai_system_multi_agent(&question)))
    }

    // -- RLM â€” Reasoning Language Model builtins ------------------------------

    /// rlm_think(model_path, question)              â†’ String (full response: thinking + answer)
    /// rlm_think(model_path, question, max_tokens)  â†’ String
    ///
    /// Runs a reasoning model (DeepSeek-R1, QwQ).  The model thinks step-by-step
    /// in a scratchpad before giving its final answer.
    /// Returns the complete formatted output showing both thinking and answer.
    ///
    /// Example:
    ///   result = rlm_think(model, "Solve: 2x + 5 = 13", 1024)
    ///   print(result)
    fn rlm_think(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        if args.len() < 2 || args.len() > 3 {
            return Err(VmError::runtime_error(
                "rlm_think expects 2-3 args: rlm_think(model_path, question [, max_tokens])".to_string(),
            ));
        }
        let model_path = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("rlm_think: model_path must be a string".to_string())),
        };
        let question = match &args[1] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("rlm_think: question must be a string".to_string())),
        };
        let max_tokens = if args.len() == 3 {
            match &args[2] {
                Value::Number(n) => *n as usize,
                _ => return Err(VmError::runtime_error("rlm_think: max_tokens must be a number".to_string())),
            }
        } else { 1024 };
        crate::inference::killer_think_rlm(&model_path, &question, max_tokens)
            .map(|r| Value::Str(r.display()))
            .map_err(VmError::runtime_error)
    }

    /// rlm_answer(model_path, question)              â†’ String (only the final answer)
    /// rlm_answer(model_path, question, max_tokens)  â†’ String
    ///
    /// Like rlm_think but returns ONLY the answer after reasoning â€” no thinking trace.
    /// Use this when you want a clean result to print or use in further computation.
    ///
    /// Example:
    ///   ans = rlm_answer(model, "What is the fastest sorting algorithm?", 512)
    ///   print(K"Answer: {ans}")
    fn rlm_answer(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        if args.len() < 2 || args.len() > 3 {
            return Err(VmError::runtime_error(
                "rlm_answer expects 2-3 args: rlm_answer(model_path, question [, max_tokens])".to_string(),
            ));
        }
        let model_path = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("rlm_answer: model_path must be a string".to_string())),
        };
        let question = match &args[1] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("rlm_answer: question must be a string".to_string())),
        };
        let max_tokens = if args.len() == 3 {
            match &args[2] {
                Value::Number(n) => *n as usize,
                _ => return Err(VmError::runtime_error("rlm_answer: max_tokens must be a number".to_string())),
            }
        } else { 1024 };
        crate::inference::killer_think_rlm(&model_path, &question, max_tokens)
            .map(|r| Value::Str(r.answer_only().to_string()))
            .map_err(VmError::runtime_error)
    }

    /// rlm_thinking(model_path, question)              â†’ String (only the thinking trace)
    /// rlm_thinking(model_path, question, max_tokens)  â†’ String
    ///
    /// Returns ONLY the model's internal reasoning scratchpad (thinking block).
    /// Useful for debugging or displaying the model's chain-of-thought.
    ///
    /// Example:
    ///   trace = rlm_thinking(model, "Prove that sqrt(2) is irrational", 2048)
    ///   print(K"Reasoning trace: {trace}")
    fn rlm_thinking(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        if args.len() < 2 || args.len() > 3 {
            return Err(VmError::runtime_error(
                "rlm_thinking expects 2-3 args: rlm_thinking(model_path, question [, max_tokens])".to_string(),
            ));
        }
        let model_path = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("rlm_thinking: model_path must be a string".to_string())),
        };
        let question = match &args[1] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("rlm_thinking: question must be a string".to_string())),
        };
        let max_tokens = if args.len() == 3 {
            match &args[2] {
                Value::Number(n) => *n as usize,
                _ => return Err(VmError::runtime_error("rlm_thinking: max_tokens must be a number".to_string())),
            }
        } else { 1024 };
        crate::inference::killer_think_rlm(&model_path, &question, max_tokens)
            .map(|r| Value::Str(r.thinking.clone()))
            .map_err(VmError::runtime_error)
    }

    // -- User-composable KhLM building blocks ---------------------------------

    /// khlm_classify(question) â†’ String
    ///
    /// Returns the question type â€” use this to build your own routing logic.
    /// Possible return values: "math", "factual", "reasoning"
    ///
    ///   "math"      â€” arithmetic, percentages, unit conversions
    ///   "factual"   â€” who/what/when/where questions needing web search
    ///   "reasoning" â€” explain/prove/implement/analyze â€” best answered by RLM
    ///
    /// Example:
    ///   kind = khlm_classify("Explain how quicksort works")
    ///   // kind == "reasoning"
    ///   kind = khlm_classify("Who is Alan Turing?")
    ///   // kind == "factual"
    fn khlm_classify(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "khlm_classify expects 1 argument: khlm_classify(question)".to_string(),
            ));
        }
        let question = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("khlm_classify: question must be a string".to_string())),
        };
        let kind = crate::llm::khlm_classify_question(&question);
        Ok(Value::Str(kind.to_string()))
    }

    /// khlm_run(model, question, pipeline) â†’ String
    ///
    /// Run a custom KhLM pipeline. Users choose exactly what happens.
    /// `pipeline` is one of:
    ///   "web"      â€” Ghost-108 web search only
    ///   "rlm"      â€” RLM reasoning only (no web)
    ///   "web+rlm"  â€” web first, then RLM synthesizes the results
    ///   "rlm+web"  â€” RLM reasons first, web fills in facts
    ///   "auto"     â€” same as khlm_ask_model (smart route)
    ///
    /// Example:
    ///   // Pure reasoning pipeline â€” no web needed
    ///   result = khlm_run(model, "Explain binary search", "rlm")
    ///
    ///   // Fact + synthesis pipeline
    ///   result = khlm_run(model, "Who founded Tesla?", "web+rlm")
    fn khlm_run(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        if args.len() != 3 {
            return Err(VmError::runtime_error(
                "khlm_run expects 3 arguments: khlm_run(model_path, question, pipeline)".to_string(),
            ));
        }
        let model_path = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("khlm_run: model_path must be a string".to_string())),
        };
        let question = match &args[1] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("khlm_run: question must be a string".to_string())),
        };
        let pipeline = match &args[2] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("khlm_run: pipeline must be a string".to_string())),
        };
        let result = crate::llm::khlm_run_pipeline(&model_path, &question, &pipeline);
        Ok(Value::Str(result))
    }

    /// llm_parallel(model_path, questions, max_tokens) â†’ List<String>
    ///
    /// Run many questions through a model in parallel â€” all at once.
    /// Returns a List with one answer per question, in the same order.
    /// Essential for building multi-agent pipelines in Killer code.
    ///
    /// Example:
    ///   questions = ["What is 2+2?", "What is the capital of France?", "Explain BFS"]
    ///   answers = llm_parallel(model, questions, 256)
    ///   for ans in answers {
    ///     print(ans)
    ///   }
    fn llm_parallel(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        if args.len() < 2 || args.len() > 3 {
            return Err(VmError::runtime_error(
                "llm_parallel expects 2-3 args: llm_parallel(model_path, questions_list [, max_tokens])".to_string(),
            ));
        }
        let model_path = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("llm_parallel: model_path must be a string".to_string())),
        };
        let questions: Vec<String> = match &args[1] {
            Value::Array(items) => {
                items
                    .iter_cloned()
                    .map(|v| match v {
                        Value::Str(s) => Ok(s.clone()),
                        _ => Err(VmError::runtime_error(
                            "llm_parallel: all questions must be strings".to_string(),
                        )),
                    })
                    .collect::<Result<Vec<_>, _>>()?
            }
            _ => return Err(VmError::runtime_error("llm_parallel: questions must be a List".to_string())),
        };
        let max_tokens = if args.len() == 3 {
            match &args[2] {
                Value::Number(n) => *n as usize,
                _ => return Err(VmError::runtime_error("llm_parallel: max_tokens must be a number".to_string())),
            }
        } else { 512 };
        let answers = crate::llm::llm_run_parallel(&model_path, &questions, max_tokens);
        Ok(Value::from(
            answers.into_iter().map(Value::Str).collect::<Vec<_>>(),
        ))
    }

    /// rlm_synthesize(model_path, question, context) â†’ String
    ///
    /// Give the RLM a question + context (your own data, web results, notes)
    /// and it reasons over them to produce a single coherent answer.
    /// This is the core of building your own KhLM â€” you gather context
    /// any way you want, then let the RLM synthesize the final answer.
    ///
    /// Example:
    ///   web1 = ghost_108("Tesla founders")
    ///   web2 = ghost_108("Elon Musk Tesla")
    ///   context = web1 + "\n" + web2
    ///   answer = rlm_synthesize(model, "Who really founded Tesla?", context)
    fn rlm_synthesize(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        if args.len() != 3 {
            return Err(VmError::runtime_error(
                "rlm_synthesize expects 3 arguments: rlm_synthesize(model_path, question, context)".to_string(),
            ));
        }
        let model_path = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("rlm_synthesize: model_path must be a string".to_string())),
        };
        let question = match &args[1] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("rlm_synthesize: question must be a string".to_string())),
        };
        let context = match &args[2] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("rlm_synthesize: context must be a string".to_string())),
        };
        let result = crate::llm::rlm_synthesize_answer(&model_path, &question, &context);
        Ok(Value::Str(result))
    }

    // =========================================================================
    // IMAGINATION ENGINE builtins
    // =========================================================================

    /// imagine(question) â€” general creative routing
    fn builtin_imagine(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        let q = Self::one_str(args, "imagine", "question")?;
        Ok(Value::Str(crate::imagination::imagine(&q)))
    }

    /// imagine_what_if(scenario) â€” counterfactual chain
    fn builtin_imagine_what_if(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        let s = Self::one_str(args, "imagine_what_if", "scenario")?;
        Ok(Value::Str(crate::imagination::imagine_what_if(&s)))
    }

    /// imagine_connect(concept_a, concept_b) â€” unexpected conceptual bridge
    fn builtin_imagine_connect(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        if args.len() != 2 {
            return Err(VmError::runtime_error(
                "imagine_connect expects 2 arguments: imagine_connect(concept_a, concept_b)".to_string(),
            ));
        }
        let a = match &args[0] { Value::Str(s) => s.clone(), _ => return Err(VmError::runtime_error("imagine_connect: concept_a must be a string".to_string())) };
        let b = match &args[1] { Value::Str(s) => s.clone(), _ => return Err(VmError::runtime_error("imagine_connect: concept_b must be a string".to_string())) };
        Ok(Value::Str(crate::imagination::imagine_connect(&a, &b)))
    }

    /// imagine_beyond(given) â€” extrapolate and think further
    fn builtin_imagine_beyond(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        let g = Self::one_str(args, "imagine_beyond", "given")?;
        Ok(Value::Str(crate::imagination::imagine_beyond(&g)))
    }

    /// imagine_self() â€” Killer reflects on its own existence
    fn builtin_imagine_self(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        let _ = args;
        Ok(Value::Str(crate::imagination::imagine_self()))
    }

    // =========================================================================
    // AFFECT ENGINE builtins
    // =========================================================================

    /// affect_sense(text) â€” update emotional state from any text
    fn builtin_affect_sense(args: &[Value]) -> Result<Value, VmError> {
        let t = Self::one_str(args, "affect_sense", "text")?;
        crate::affect::affect_sense(&t);
        Ok(Value::Str(format!("affect updated from: {}", &t.chars().take(40).collect::<String>())))
    }

    /// affect_state() â€” read current emotional state
    fn builtin_affect_state(args: &[Value]) -> Result<Value, VmError> {
        let _ = args;
        Ok(Value::Str(crate::affect::affect_state_str()))
    }

    /// affect_color(text) â€” filter text through current emotion
    fn builtin_affect_color(args: &[Value]) -> Result<Value, VmError> {
        let t = Self::one_str(args, "affect_color", "text")?;
        Ok(Value::Str(crate::affect::affect_color(&t)))
    }

    /// affect_reset() â€” return to neutral state
    fn builtin_affect_reset(args: &[Value]) -> Result<Value, VmError> {
        let _ = args;
        crate::affect::affect_reset();
        Ok(Value::Str("affect reset to neutral".to_string()))
    }

    /// affect_set(dimension, value) â€” manually dial one emotion
    fn builtin_affect_set(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 {
            return Err(VmError::runtime_error(
                "affect_set expects 2 arguments: affect_set(dimension, value)".to_string(),
            ));
        }
        let dim = match &args[0] { Value::Str(s) => s.clone(), _ => return Err(VmError::runtime_error("affect_set: dimension must be a string".to_string())) };
        let val: f32 = match &args[1] {
            Value::Number(f) => *f as f32,
            _ => return Err(VmError::runtime_error("affect_set: value must be a number (0.0-1.0)".to_string())),
        };
        Ok(Value::Str(crate::affect::affect_set(&dim, val)))
    }

    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // GUARDIAN ENGINE â€” Human Protection Principle
    // Creator: Sai Arun Kumar Katherashala
    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    /// guardian_check(query) â€” check if something is safe. Blocks if harm detected.
    fn builtin_guardian_check(args: &[Value]) -> Result<Value, VmError> {
        let q = Self::one_str(args, "guardian_check", "query")?;
        Ok(Value::Str(crate::guardian::guardian_check(&q)))
    }

    /// guardian_principles() â€” print Killer's full ethical framework
    fn builtin_guardian_principles(_args: &[Value]) -> Result<Value, VmError> {
        Ok(Value::Str(crate::guardian::guardian_principles()))
    }

    /// guardian_status() â€” show live stats (intercepts, last blocked query)
    fn builtin_guardian_status(_args: &[Value]) -> Result<Value, VmError> {
        Ok(Value::Str(crate::guardian::guardian_status()))
    }

    // -- Internal helper: extract exactly one String arg ----------------------
    fn one_str(args: &[Value], fn_name: &str, param: &str) -> Result<String, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                format!("{} expects 1 argument: {}({})", fn_name, fn_name, param)
            ));
        }
        match &args[0] {
            Value::Str(s) => Ok(s.clone()),
            _ => Err(VmError::runtime_error(format!("{}: {} must be a string", fn_name, param))),
        }
    }

    // -- KORE â€” Killer Optimized Record Exchange file format ------------------
    //
    //  Ghost-108 research findings applied:
    //    âœ… PAX layout (all cols in one cache page â†’ zero tuple reconstruction)
    //    âœ… Bloom filters per col per chunk (O(1) existence check)
    //    âœ… min/max stats per chunk (predicate pushdown â€” skip chunks)
    //    âœ… Per-column auto algorithm: delta/dict+RLE/LZ77
    //    âœ… Per-column XOR encryption (unique feature)
    //    âœ… Global dictionary pool for string deduplication
    //    âœ… Index block at end for O(1) chunk access
    //
    //  KhLM classification confirmed: compression strategy = "reasoning"
    //  â†’ applied delta encoding for ints, dict+RLE for low-cardinality strings

    // KORE stubs â€” use Killer (full) for KORE operations
    #[allow(dead_code)]
    fn kore_write(_args: &[Value]) -> Result<Value, VmError> {
        Ok(Value::Str("kore_write: use Killer (full) for KORE operations".to_string()))
    }
    #[allow(dead_code)]
    fn kore_read(_args: &[Value]) -> Result<Value, VmError> {
        Ok(Value::Str("kore_read: use Killer (full) for KORE operations".to_string()))
    }
    #[allow(dead_code)]
    fn kore_read_col(_args: &[Value]) -> Result<Value, VmError> {
        Ok(Value::Str("kore_read_col: use Killer (full) for KORE operations".to_string()))
    }
    #[allow(dead_code)]
    fn kore_info(_args: &[Value]) -> Result<Value, VmError> {
        Ok(Value::Str("kore_info: use Killer (full) for KORE operations".to_string()))
    }

    // Duplicate llm_info removed below â€” implementation above
    #[allow(dead_code)]
    fn llm_info_old(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        if args.len() != 1 {
            return Err(VmError::runtime_error(
                "llm_info expects 1 argument: llm_info(model_path)".to_string(),
            ));
        }
        let model_path = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => return Err(VmError::runtime_error("llm_info: model_path must be a string".to_string())),
        };
        crate::inference::killer_model_info(&model_path)
            .map(Value::Str)
            .map_err(VmError::runtime_error)
    }

    // -------------------------------------------------------------------------
    // Standalone compression / encoding builtins (no nova/kore/compression_module)
    // -------------------------------------------------------------------------

    fn standalone_b64_encode(input: &str) -> String {
        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let bytes = input.as_bytes();
        let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);
        for chunk in bytes.chunks(3) {
            let b0 = chunk[0] as u32;
            let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
            let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
            let triple = (b0 << 16) | (b1 << 8) | b2;
            out.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
            out.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
            if chunk.len() > 1 { out.push(CHARS[((triple >> 6) & 0x3F) as usize] as char); } else { out.push('='); }
            if chunk.len() > 2 { out.push(CHARS[(triple & 0x3F) as usize] as char); } else { out.push('='); }
        }
        out
    }

    fn standalone_b64_decode(input: &str) -> Option<String> {
        fn val(c: u8) -> Option<u32> {
            match c {
                b'A'..=b'Z' => Some((c - b'A') as u32),
                b'a'..=b'z' => Some((c - b'a' + 26) as u32),
                b'0'..=b'9' => Some((c - b'0' + 52) as u32),
                b'+' => Some(62), b'/' => Some(63), b'=' => Some(0),
                _ => None,
            }
        }
        let bytes: Vec<u8> = input.bytes().filter(|&b| b != b'\n' && b != b'\r' && b != b' ').collect();
        if bytes.len() % 4 != 0 { return None; }
        let mut out = Vec::new();
        for chunk in bytes.chunks(4) {
            let a = val(chunk[0])?; let b = val(chunk[1])?;
            let c = val(chunk[2])?; let d = val(chunk[3])?;
            let triple = (a << 18) | (b << 12) | (c << 6) | d;
            out.push((triple >> 16) as u8);
            if chunk[2] != b'=' { out.push((triple >> 8) as u8); }
            if chunk[3] != b'=' { out.push(triple as u8); }
        }
        String::from_utf8(out).ok()
    }

    fn standalone_hex_encode(input: &str) -> String {
        input.bytes().map(|b| format!("{:02x}", b)).collect()
    }

    fn standalone_hex_decode(input: &str) -> Option<String> {
        let bytes: Vec<u8> = (0..input.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&input[i..i+2], 16))
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        String::from_utf8(bytes).ok()
    }

    // -- Internal base64 helpers for compress/decompress --
    #[allow(dead_code)]
    fn b64_encode_bytes(data: &[u8]) -> String {
        Self::standalone_b64_encode(&data.iter().map(|&b| b as char).collect::<String>())
    }

    #[allow(dead_code)]
    fn b64_decode_to_bytes(s: &str) -> Option<Vec<u8>> {
        Self::standalone_b64_decode(s).map(|decoded| decoded.chars().map(|c| c as u8).collect())
    }

    /// `compress(text, algo)` â†’ String â€” stub (use Killer for real compression)
    fn builtin_compress(args: &[Value]) -> Result<Value, VmError> {
        let text = match args.first() {
            Some(Value::Str(t)) => t.clone(),
            _ => return Err(VmError::runtime_error("compress(text, algo): first arg must be a String")),
        };
        // Simple stub: just base64-encode
        Ok(Value::Str(Self::standalone_b64_encode(&text)))
    }

    /// `decompress(compressed, algo)` â†’ String â€” stub (use Killer for real compression)
    fn builtin_decompress(args: &[Value]) -> Result<Value, VmError> {
        let b64 = match args.first() {
            Some(Value::Str(t)) => t.clone(),
            _ => return Err(VmError::runtime_error("decompress(compressed, algo): first arg must be a String")),
        };
        Self::standalone_b64_decode(&b64)
            .map(Value::Str)
            .ok_or_else(|| VmError::runtime_error("decompress: invalid base64 input".to_string()))
    }

    /// `b64_encode(text)` â†’ String
    fn builtin_b64_encode(args: &[Value]) -> Result<Value, VmError> {
        match args.first() {
            Some(Value::Str(s)) => Ok(Value::Str(Self::standalone_b64_encode(s))),
            _ => Err(VmError::runtime_error("b64_encode(text): expects a String argument")),
        }
    }

    /// `b64_decode(b64)` â†’ String
    fn builtin_b64_decode(args: &[Value]) -> Result<Value, VmError> {
        match args.first() {
            Some(Value::Str(s)) => Self::standalone_b64_decode(s)
                .map(Value::Str)
                .ok_or_else(|| VmError::runtime_error("b64_decode: invalid base64 string".to_string())),
            _ => Err(VmError::runtime_error("b64_decode(b64): expects a String argument")),
        }
    }

    /// `hex_encode(text)` â†’ String
    fn builtin_hex_encode(args: &[Value]) -> Result<Value, VmError> {
        match args.first() {
            Some(Value::Str(s)) => Ok(Value::Str(Self::standalone_hex_encode(s))),
            _ => Err(VmError::runtime_error("hex_encode(text): expects a String argument")),
        }
    }

    /// `hex_decode(hex)` â†’ String
    fn builtin_hex_decode(args: &[Value]) -> Result<Value, VmError> {
        match args.first() {
            Some(Value::Str(s)) => Self::standalone_hex_decode(s)
                .map(Value::Str)
                .ok_or_else(|| VmError::runtime_error("hex_decode: invalid hex string".to_string())),
            _ => Err(VmError::runtime_error("hex_decode(hex): expects a String argument")),
        }
    }

    /// `compress_ratio(original, compressed)` â†’ Number
    fn builtin_compress_ratio(args: &[Value]) -> Result<Value, VmError> {
        match (args.first(), args.get(1)) {
            (Some(Value::Str(orig)), Some(Value::Str(comp))) => {
                let ratio = if comp.is_empty() { f64::INFINITY } else { orig.len() as f64 / comp.len() as f64 };
                Ok(Value::Number(ratio))
            }
            _ => Err(VmError::runtime_error("compress_ratio(original, compressed): both args must be Strings")),
        }
    }

    /// `compress_info(text)` â†’ Dict â€” simplified (no nova compression available)
    fn builtin_compress_info(args: &[Value]) -> Result<Value, VmError> {
        let text = match args.first() {
            Some(Value::Str(s)) => s.clone(),
            _ => return Err(VmError::runtime_error("compress_info(text): expects a String argument")),
        };
        let orig_len = text.len();
        let b64_str = Self::standalone_b64_encode(&text);
        let hex_str = Self::standalone_hex_encode(&text);
        let pct = |comp: usize| -> String {
            format!("{:.1}%", comp as f64 / orig_len.max(1) as f64 * 100.0)
        };
        let mut m = std::collections::HashMap::new();
        m.insert("original_len".into(), Value::Number(orig_len as f64));
        m.insert("b64_len".into(), Value::Number(b64_str.len() as f64));
        m.insert("b64_pct".into(), Value::Str(pct(b64_str.len())));
        m.insert("hex_len".into(), Value::Number(hex_str.len() as f64));
        m.insert("hex_pct".into(), Value::Str(pct(hex_str.len())));
        m.insert("best_algo".into(), Value::Str("b64".to_string()));
        Ok(Value::Dict(Box::new(m)))
    }

    // -------------------------------------------------------------------------
    // Debug Intelligence helpers
    // -------------------------------------------------------------------------

    /// `debug_check(code)` â†’ Array of issue dicts
    fn dbg_debug_check(args: &[Value]) -> Result<Value, VmError> {
        let code = match args.first() {
            Some(Value::Str(s)) => s.clone(),
            _ => return Err(VmError::runtime_error("debug_check(code): expects a String argument")),
        };
        let issues = crate::debug_intelligence::debug_check(&code);
        let arr: Vec<Value> = issues
            .into_iter()
            .map(|issue| {
                let mut m = std::collections::HashMap::new();
                m.insert("code".into(),         Value::Str(issue.code));
                m.insert("severity".into(),     Value::Str(issue.severity.as_str().to_string()));
                m.insert("line".into(),         Value::Number(issue.line as f64));
                m.insert("message".into(),      Value::Str(issue.message));
                m.insert("fix_hint".into(),     Value::Str(issue.fix_hint));
                m.insert("auto_fixable".into(), Value::Bool(issue.auto_fixable));
                Value::Dict(Box::new(m))
            })
            .collect();
        Ok(Value::from(arr))
    }

    /// `auto_fix(code)` â†’ Array of fix-candidate dicts
    fn dbg_auto_fix(args: &[Value]) -> Result<Value, VmError> {
        let code = match args.first() {
            Some(Value::Str(s)) => s.clone(),
            _ => return Err(VmError::runtime_error("auto_fix(code): expects a String argument")),
        };
        let candidates = crate::debug_intelligence::auto_fix(&code);
        let arr: Vec<Value> = candidates
            .into_iter()
            .map(|c| {
                let mut m = std::collections::HashMap::new();
                m.insert("confidence".into(),  Value::Number(c.confidence as f64));
                m.insert("description".into(), Value::Str(c.description));
                m.insert("fixed_code".into(),  Value::Str(c.fixed_code));
                let changes: Vec<Value> = c.changes.into_iter().map(|ch| {
                    let mut cm = std::collections::HashMap::new();
                    cm.insert("line".into(),        Value::Number(ch.line as f64));
                    cm.insert("original".into(),    Value::Str(ch.original));
                    cm.insert("replacement".into(), Value::Str(ch.replacement));
                    cm.insert("reason".into(),      Value::Str(ch.reason));
                    Value::Dict(Box::new(cm))
                }).collect();
                m.insert("changes".into(), Value::from(changes));
                Value::Dict(Box::new(m))
            })
            .collect();
        Ok(Value::from(arr))
    }

    /// `explain_error(msg)` or `explain_error(msg, context)` â†’ String
    fn dbg_explain_error(args: &[Value]) -> Result<Value, VmError> {
        let msg = match args.first() {
            Some(Value::Str(s)) => s.clone(),
            _ => return Err(VmError::runtime_error("explain_error(msg[, ctx]): first arg must be String")),
        };
        let ctx = match args.get(1) {
            Some(Value::Str(s)) => s.clone(),
            _ => String::new(),
        };
        Ok(Value::Str(crate::debug_intelligence::explain_error(&msg, &ctx)))
    }

    /// `suggest_refactor(code)` â†’ Array of suggestion dicts
    fn dbg_suggest_refactor(args: &[Value]) -> Result<Value, VmError> {
        let code = match args.first() {
            Some(Value::Str(s)) => s.clone(),
            _ => return Err(VmError::runtime_error("suggest_refactor(code): expects a String argument")),
        };
        let suggestions = crate::debug_intelligence::suggest_refactor(&code);
        let arr: Vec<Value> = suggestions
            .into_iter()
            .map(|s| {
                let mut m = std::collections::HashMap::new();
                m.insert("code".into(),        Value::Str(s.code));
                m.insert("line".into(),        Value::Number(s.line as f64));
                m.insert("title".into(),       Value::Str(s.title));
                m.insert("description".into(), Value::Str(s.description));
                m.insert("priority".into(),    Value::Str(s.priority.as_str().to_string()));
                Value::Dict(Box::new(m))
            })
            .collect();
        Ok(Value::from(arr))
    }

    /// `auto_test(code)` â†’ String (Killer test scaffold)
    fn dbg_auto_test(args: &[Value]) -> Result<Value, VmError> {
        let code = match args.first() {
            Some(Value::Str(s)) => s.clone(),
            _ => return Err(VmError::runtime_error("auto_test(code): expects a String argument")),
        };
        Ok(Value::Str(crate::debug_intelligence::auto_test(&code)))
    }

    /// `perf_profile(code)` â†’ Array of perf-hint dicts
    fn dbg_perf_profile(args: &[Value]) -> Result<Value, VmError> {
        let code = match args.first() {
            Some(Value::Str(s)) => s.clone(),
            _ => return Err(VmError::runtime_error("perf_profile(code): expects a String argument")),
        };
        let hints = crate::debug_intelligence::perf_profile(&code);
        let arr: Vec<Value> = hints
            .into_iter()
            .map(|h| {
                let mut m = std::collections::HashMap::new();
                m.insert("line".into(),       Value::Number(h.line as f64));
                m.insert("category".into(),   Value::Str(h.category));
                m.insert("impact".into(),     Value::Str(h.impact.as_str().to_string()));
                m.insert("message".into(),    Value::Str(h.message));
                m.insert("suggestion".into(), Value::Str(h.suggestion));
                Value::Dict(Box::new(m))
            })
            .collect();
        Ok(Value::from(arr))
    }

    /// `ai_pair(task_description)` â†’ String (generated Killer code)
    fn dbg_ai_pair(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        let task = match args.first() {
            Some(Value::Str(s)) => s.clone(),
            _ => return Err(VmError::runtime_error("ai_pair(task): expects a String argument")),
        };
        Ok(Value::Str(crate::debug_intelligence::ai_pair(&task)))
    }

    /// `killer_debug_agent(code)` â†’ Dict with keys: success, fixed_code, cycles, summary
    fn dbg_killer_debug_agent(args: &[Value]) -> Result<Value, VmError> {
        crate::security::require_llm()?;
        let code = match args.first() {
            Some(Value::Str(s)) => s.clone(),
            _ => return Err(VmError::runtime_error("killer_debug_agent(code): expects a String argument")),
        };
        let result = crate::debug_intelligence::killer_debug_agent(&code);

        let final_issues_arr: Vec<Value> = result.final_issues.into_iter().map(|issue| {
            let mut m = std::collections::HashMap::new();
            m.insert("code".into(),     Value::Str(issue.code));
            m.insert("severity".into(), Value::Str(issue.severity.as_str().to_string()));
            m.insert("line".into(),     Value::Number(issue.line as f64));
            m.insert("message".into(),  Value::Str(issue.message));
            Value::Dict(Box::new(m))
        }).collect();

        let changes_arr: Vec<Value> = result.all_changes.into_iter().map(|ch| {
            let mut m = std::collections::HashMap::new();
            m.insert("line".into(),        Value::Number(ch.line as f64));
            m.insert("original".into(),    Value::Str(ch.original));
            m.insert("replacement".into(), Value::Str(ch.replacement));
            m.insert("reason".into(),      Value::Str(ch.reason));
            Value::Dict(Box::new(m))
        }).collect();

        let mut out = std::collections::HashMap::new();
        out.insert("success".into(),      Value::Bool(result.success));
        out.insert("fixed_code".into(),   Value::Str(result.fixed_code));
        out.insert("cycles".into(),       Value::Number(result.cycles as f64));
        out.insert("summary".into(),      Value::Str(result.summary));
        out.insert("changes".into(),      Value::from(changes_arr));
        out.insert("remaining".into(),    Value::from(final_issues_arr));
        Ok(Value::Dict(Box::new(out)))
    }

    /// `watch(expr_name, value)` â†’ Null
    fn dbg_watch(args: &[Value]) -> Result<Value, VmError> {
        let expr = match args.first() {
            Some(Value::Str(s)) => s.clone(),
            _ => return Err(VmError::runtime_error("watch(expr, value): first arg must be String")),
        };
        let val_str = match args.get(1) {
            Some(v) => format!("{}", v),
            None     => "null".into(),
        };
        crate::debug_intelligence::watch_value(&expr, &val_str);
        Ok(Value::Null)
    }

    /// `watch_report()` â†’ String
    fn dbg_watch_report(args: &[Value]) -> Result<Value, VmError> {
        let _ = args;
        Ok(Value::Str(crate::debug_intelligence::watch_report()))
    }

    /// `lint(code)` â†’ String report from the Killer linter
    fn builtin_lint(args: &[Value]) -> Result<Value, VmError> {
        let code = match args.first() {
            Some(Value::Str(s)) => s.clone(),
            _ => return Err(VmError::runtime_error("lint(code): expects a String argument")),
        };
        let mut l = crate::linter::Linter::new();
        match l.lint_source(&code) {
            Ok(()) => Ok(Value::Str(l.report())),
            Err(e) => Ok(Value::Str(format!("Lint error: {}", e))),
        }
    }

    // =========================================================================
    // Phase 1: Trit â€” balanced ternary (-1, 0, +1)
    // =========================================================================

    fn get_trit(v: &Value) -> Result<i8, VmError> {
        match v {
            Value::Trit(t) => Ok(*t),
            Value::Number(n) => {
                let i = *n as i64;
                if i == -1 || i == 0 || i == 1 { Ok(i as i8) }
                else { Err(VmError::runtime_error(format!("not a trit value: {}", n))) }
            }
            _ => Err(VmError::runtime_error(format!("expected Trit, got {:?}", v))),
        }
    }

    fn trit_and(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("trit_and(a, b) requires 2 args")); }
        Ok(Value::Trit(Self::get_trit(&args[0])?.min(Self::get_trit(&args[1])?)))
    }

    fn trit_or(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("trit_or(a, b) requires 2 args")); }
        Ok(Value::Trit(Self::get_trit(&args[0])?.max(Self::get_trit(&args[1])?)))
    }

    fn trit_not(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("trit_not(a) requires 1 arg")); }
        Ok(Value::Trit(-Self::get_trit(&args[0])?))
    }

    fn trit_add(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("trit_add(a, b) requires 2 args")); }
        let sum = Self::get_trit(&args[0])? as i32 + Self::get_trit(&args[1])? as i32;
        Ok(Value::Trit(sum.clamp(-1, 1) as i8))
    }

    fn trit_mul(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("trit_mul(a, b) requires 2 args")); }
        let prod = Self::get_trit(&args[0])? as i32 * Self::get_trit(&args[1])? as i32;
        Ok(Value::Trit(prod.clamp(-1, 1) as i8))
    }

    fn trit_to_int(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("trit_to_int(t) requires 1 arg")); }
        Ok(Value::Number(Self::get_trit(&args[0])? as f64))
    }

    fn trit_from_int(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("int_to_trit(n) requires 1 arg")); }
        let n = match &args[0] {
            Value::Number(n) => *n as i64,
            _ => return Err(VmError::runtime_error("int_to_trit requires a Number")),
        };
        Ok(Value::Trit(n.clamp(-1, 1) as i8))
    }

    fn trit_to_str(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("trit_to_str(t) requires 1 arg")); }
        let s = match Self::get_trit(&args[0])? {
            -1 => "T_NEG",
            0  => "T_ZERO",
            _  => "T_POS",
        };
        Ok(Value::Str(s.to_string()))
    }

    /// trit_word(trit) â†’ String: maps T_NEGâ†’"no", T_ZEROâ†’"maybe", T_POSâ†’"yes"
    fn trit_word(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("trit_word(trit) requires 1 arg")); }
        let t = Self::get_trit(&args[0])?;
        let word = match t {
            -1 => "no",
             0 => "maybe",
             1 => "yes",
             _ => "maybe",
        };
        Ok(Value::Str(word.to_string()))
    }

    /// trit_word_to_int(word) â†’ Number: maps "no"â†’-1, "maybe"â†’0, "yes"â†’1
    fn trit_word_to_int(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("trit_word_to_int(str) requires 1 arg")); }
        match &args[0] {
            Value::Str(s) => {
                let n = match s.as_str() {
                    "yes"   =>  1.0,
                    "maybe" =>  0.0,
                    "no"    => -1.0,
                    other   => return Err(VmError::runtime_error(format!("trit_word_to_int: unknown word '{}'", other))),
                };
                Ok(Value::Number(n))
            }
            _ => Err(VmError::runtime_error("trit_word_to_int requires a String")),
        }
    }

    // =========================================================================
    // Phase 2: Fuzzy Logic â€” continuous truth values [0.0, 1.0]
    // =========================================================================

    fn get_fuzzy(v: &Value) -> Result<f64, VmError> {
        match v {
            Value::Number(n) => Ok(n.clamp(0.0, 1.0)),
            Value::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
            _ => Err(VmError::runtime_error(format!("expected fuzzy float [0-1], got {:?}", v))),
        }
    }

    fn fuzzy_and(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("fuzzy_and(a, b) requires 2 args")); }
        Ok(Value::Number(Self::get_fuzzy(&args[0])?.min(Self::get_fuzzy(&args[1])?)))
    }

    fn fuzzy_or(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("fuzzy_or(a, b) requires 2 args")); }
        Ok(Value::Number(Self::get_fuzzy(&args[0])?.max(Self::get_fuzzy(&args[1])?)))
    }

    fn fuzzy_not(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("fuzzy_not(a) requires 1 arg")); }
        Ok(Value::Number(1.0 - Self::get_fuzzy(&args[0])?))
    }

    /// fuzzy_threshold(val, threshold) â†’ Trit (T_POS/T_ZERO/T_NEG)
    fn fuzzy_threshold(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("fuzzy_threshold(val, threshold) requires 2 args")); }
        let v = Self::get_fuzzy(&args[0])?;
        let t = Self::get_fuzzy(&args[1])?;
        let uncertain_low  = t - 0.15;
        let uncertain_high = t + 0.15;
        if v >= uncertain_high { Ok(Value::Trit(1)) }
        else if v <= uncertain_low { Ok(Value::Trit(-1)) }
        else { Ok(Value::Trit(0)) }
    }

    /// fuzzy_combine(a, b, ...) â†’ weighted geometric mean
    fn fuzzy_combine(args: &[Value]) -> Result<Value, VmError> {
        if args.is_empty() { return Err(VmError::runtime_error("fuzzy_combine requires at least 1 arg")); }
        let mut product = 1.0f64;
        for v in args { product *= Self::get_fuzzy(v)?; }
        Ok(Value::Number(product.powf(1.0 / args.len() as f64)))
    }

    // =========================================================================
    // Phase 3: Cognitive Signal â€” value + confidence + reason
    // =========================================================================

    /// signal_create(value, confidence, reason) â†’ Signal
    fn signal_create(args: &[Value]) -> Result<Value, VmError> {
        if args.len() < 2 { return Err(VmError::runtime_error("signal_create(value, confidence, reason?) requires 2-3 args")); }
        let confidence = match &args[1] {
            Value::Number(n) => n.clamp(0.0, 1.0),
            _ => return Err(VmError::runtime_error("signal_create: confidence must be a Number [0.0-1.0]")),
        };
        let reason = if args.len() >= 3 {
            match &args[2] { Value::Str(s) => s.clone(), v => format!("{}", v) }
        } else { String::new() };
        Ok(Value::Signal { value: Box::new(args[0].clone()), confidence, reason })
    }

    fn signal_value(args: &[Value]) -> Result<Value, VmError> {
        match args.first() {
            Some(Value::Signal { value, .. }) => Ok(*value.clone()),
            _ => Err(VmError::runtime_error("signal_value(s) requires a Signal")),
        }
    }

    fn signal_confidence(args: &[Value]) -> Result<Value, VmError> {
        match args.first() {
            Some(Value::Signal { confidence, .. }) => Ok(Value::Number(*confidence)),
            _ => Err(VmError::runtime_error("signal_confidence(s) requires a Signal")),
        }
    }

    fn signal_reason(args: &[Value]) -> Result<Value, VmError> {
        match args.first() {
            Some(Value::Signal { reason, .. }) => Ok(Value::Str(reason.clone())),
            _ => Err(VmError::runtime_error("signal_reason(s) requires a Signal")),
        }
    }

    /// signal_and(s1, s2) â†’ Signal â€” trit_and values, min confidence, combined reason
    fn signal_and(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("signal_and(s1, s2) requires 2 Signals")); }
        match (&args[0], &args[1]) {
            (Value::Signal { value: v1, confidence: c1, reason: r1 },
             Value::Signal { value: v2, confidence: c2, reason: r2 }) => {
                let t1 = match v1.as_ref() { Value::Trit(t) => *t, Value::Bool(b) => if *b {1} else {-1}, _ => 0 };
                let t2 = match v2.as_ref() { Value::Trit(t) => *t, Value::Bool(b) => if *b {1} else {-1}, _ => 0 };
                let combined_val = Value::Trit(t1.min(t2));
                let combined_conf = c1.min(*c2);
                let combined_reason = format!("{} + {}", r1, r2);
                Ok(Value::Signal { value: Box::new(combined_val), confidence: combined_conf, reason: combined_reason })
            }
            _ => Err(VmError::runtime_error("signal_and requires 2 Signal values")),
        }
    }

    /// signal_or(s1, s2) â†’ Signal â€” trit_or values, max confidence, combined reason
    fn signal_or(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("signal_or(s1, s2) requires 2 Signals")); }
        match (&args[0], &args[1]) {
            (Value::Signal { value: v1, confidence: c1, reason: r1 },
             Value::Signal { value: v2, confidence: c2, reason: r2 }) => {
                let t1 = match v1.as_ref() { Value::Trit(t) => *t, Value::Bool(b) => if *b {1} else {-1}, _ => 0 };
                let t2 = match v2.as_ref() { Value::Trit(t) => *t, Value::Bool(b) => if *b {1} else {-1}, _ => 0 };
                let combined_val = Value::Trit(t1.max(t2));
                let combined_conf = c1.max(*c2);
                let combined_reason = format!("{} | {}", r1, r2);
                Ok(Value::Signal { value: Box::new(combined_val), confidence: combined_conf, reason: combined_reason })
            }
            _ => Err(VmError::runtime_error("signal_or requires 2 Signal values")),
        }
    }

    /// signal_confident(signal, threshold) â†’ Bool â€” true if confidence >= threshold
    fn signal_confident(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("signal_confident(s, threshold) requires 2 args")); }
        match &args[0] {
            Value::Signal { confidence, .. } => {
                let t = match &args[1] { Value::Number(n) => *n, _ => 0.5 };
                Ok(Value::Bool(*confidence >= t))
            }
            _ => Err(VmError::runtime_error("signal_confident: first arg must be a Signal")),
        }
    }

    /// signal_uncertain(signal, threshold) â†’ Bool â€” true if confidence < threshold
    fn signal_uncertain(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("signal_uncertain(s, threshold) requires 2 args")); }
        match &args[0] {
            Value::Signal { confidence, .. } => {
                let t = match &args[1] { Value::Number(n) => *n, _ => 0.5 };
                Ok(Value::Bool(*confidence < t))
            }
            _ => Err(VmError::runtime_error("signal_uncertain: first arg must be a Signal")),
        }
    }

    /// signal_to_str(signal) â†’ human-readable string
    fn signal_to_str(args: &[Value]) -> Result<Value, VmError> {
        match args.first() {
            Some(Value::Signal { value, confidence, reason }) => {
                let verdict = match value.as_ref() {
                    Value::Trit(1)  => "YES",
                    Value::Trit(-1) => "NO",
                    Value::Trit(0)  => "UNCERTAIN",
                    Value::Bool(true)  => "YES",
                    Value::Bool(false) => "NO",
                    _ => "UNKNOWN",
                };
                let pct = (confidence * 100.0).round() as i64;
                let s = if reason.is_empty() {
                    format!("{} ({}% confidence)", verdict, pct)
                } else {
                    format!("{} ({}% confidence): {}", verdict, pct, reason)
                };
                Ok(Value::Str(s))
            }
            _ => Err(VmError::runtime_error("signal_to_str(s) requires a Signal")),
        }
    }

    // =========================================================================
    // Phase 4: Qubit â€” quantum simulation  |ÏˆâŸ© = Î±|0âŸ© + Î²|1âŸ©
    // =========================================================================

    fn normalize_qubit(alpha: f64, beta: f64) -> (f64, f64) {
        let norm = (alpha * alpha + beta * beta).sqrt();
        if norm < 1e-12 { (1.0, 0.0) } else { (alpha / norm, beta / norm) }
    }

    /// qubit_create(alpha, beta) â†’ Qubit  â€” auto-normalizes
    fn qubit_create(args: &[Value]) -> Result<Value, VmError> {
        let (alpha, beta) = match args.len() {
            1 => match &args[0] {
                Value::Number(n) => (*n, (1.0 - n * n).max(0.0).sqrt()),
                _ => return Err(VmError::runtime_error("qubit_create(p0) requires a Number")),
            },
            2 => match (&args[0], &args[1]) {
                (Value::Number(a), Value::Number(b)) => (*a, *b),
                _ => return Err(VmError::runtime_error("qubit_create(alpha, beta) requires 2 Numbers")),
            },
            _ => return Err(VmError::runtime_error("qubit_create requires 1 or 2 args")),
        };
        let (a, b) = Self::normalize_qubit(alpha, beta);
        Ok(Value::Qubit { alpha: a, beta: b })
    }

    /// qubit_hadamard(q) â†’ Qubit â€” H gate: creates superposition from |0âŸ©
    fn qubit_hadamard(args: &[Value]) -> Result<Value, VmError> {
        match args.first() {
            Some(Value::Qubit { alpha, beta }) => {
                let inv_sqrt2 = 1.0_f64 / 2.0_f64.sqrt();
                let new_alpha = inv_sqrt2 * (alpha + beta);
                let new_beta  = inv_sqrt2 * (alpha - beta);
                Ok(Value::Qubit { alpha: new_alpha, beta: new_beta })
            }
            _ => Err(VmError::runtime_error("qubit_hadamard(q) requires a Qubit")),
        }
    }

    /// qubit_pauli_x(q) â†’ Qubit â€” X gate (quantum NOT): flips |0âŸ©â†”|1âŸ©
    fn qubit_pauli_x(args: &[Value]) -> Result<Value, VmError> {
        match args.first() {
            Some(Value::Qubit { alpha, beta }) => Ok(Value::Qubit { alpha: *beta, beta: *alpha }),
            _ => Err(VmError::runtime_error("qubit_pauli_x(q) requires a Qubit")),
        }
    }

    /// qubit_pauli_z(q) â†’ Qubit â€” Z gate: phase flip
    fn qubit_pauli_z(args: &[Value]) -> Result<Value, VmError> {
        match args.first() {
            Some(Value::Qubit { alpha, beta }) => Ok(Value::Qubit { alpha: *alpha, beta: -*beta }),
            _ => Err(VmError::runtime_error("qubit_pauli_z(q) requires a Qubit")),
        }
    }

    /// qubit_phase(q, theta) â†’ Qubit â€” phase shift by theta radians
    fn qubit_phase(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("qubit_phase(q, theta) requires 2 args")); }
        match (&args[0], &args[1]) {
            (Value::Qubit { alpha, beta }, Value::Number(theta)) => {
                // Apply phase shift to |1âŸ© component: beta â†’ beta * e^(i*theta)
                // For real simulation: approximate with cos(theta)
                let new_beta = beta * theta.cos();
                let (a, b) = Self::normalize_qubit(*alpha, new_beta);
                Ok(Value::Qubit { alpha: a, beta: b })
            }
            _ => Err(VmError::runtime_error("qubit_phase(q, theta) requires Qubit and Number")),
        }
    }

    /// qubit_measure(q) â†’ Number (0 or 1) â€” probabilistic collapse using LCG RNG
    fn qubit_measure(args: &[Value]) -> Result<Value, VmError> {
        match args.first() {
            Some(Value::Qubit { alpha, beta: _ }) => {
                let prob0 = alpha * alpha;
                // LCG pseudo-random (no external crate needed)
                let seed = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .subsec_nanos() as u64;
                let rand_val = (seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407) >> 33) as f64
                    / u32::MAX as f64;
                Ok(Value::Number(if rand_val < prob0 { 0.0 } else { 1.0 }))
            }
            _ => Err(VmError::runtime_error("qubit_measure(q) requires a Qubit")),
        }
    }

    fn qubit_prob0(args: &[Value]) -> Result<Value, VmError> {
        match args.first() {
            Some(Value::Qubit { alpha, .. }) => Ok(Value::Number(alpha * alpha)),
            _ => Err(VmError::runtime_error("qubit_prob0(q) requires a Qubit")),
        }
    }

    fn qubit_prob1(args: &[Value]) -> Result<Value, VmError> {
        match args.first() {
            Some(Value::Qubit { beta, .. }) => Ok(Value::Number(beta * beta)),
            _ => Err(VmError::runtime_error("qubit_prob1(q) requires a Qubit")),
        }
    }

    fn qubit_to_str(args: &[Value]) -> Result<Value, VmError> {
        match args.first() {
            Some(Value::Qubit { alpha, beta }) => {
                let p0 = (alpha * alpha * 100.0).round() as i64;
                let p1 = (beta  * beta  * 100.0).round() as i64;
                Ok(Value::Str(format!("|ÏˆâŸ© = {:.3}|0âŸ© + {:.3}|1âŸ©  [P(0)={}%, P(1)={}%]", alpha, beta, p0, p1)))
            }
            _ => Err(VmError::runtime_error("qubit_to_str(q) requires a Qubit")),
        }
    }

    /// qubit_entangle(q1, q2) â†’ Array[Qubit, Qubit] â€” Bell state (maximally entangled)
    fn qubit_entangle(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("qubit_entangle(q1, q2) requires 2 Qubits")); }
        let inv_sqrt2 = 1.0_f64 / 2.0_f64.sqrt();
        let q_a = Value::Qubit { alpha: inv_sqrt2, beta: inv_sqrt2 };
        let q_b = Value::Qubit { alpha: inv_sqrt2, beta: inv_sqrt2 };
        Ok(Value::from(vec![q_a, q_b]))
    }

    // =========================================================================
    // Phase 5: Tryte â€” 6-trit balanced ternary word
    // 729 states, range -364..+364, 9.51 bits of information
    // Position weights: [3^5, 3^4, 3^3, 3^2, 3^1, 3^0] = [243,81,27,9,3,1]
    // =========================================================================

    fn get_tryte(v: &Value) -> Result<[i8; 6], VmError> {
        match v {
            Value::Tryte(ts) => Ok(*ts),
            _ => Err(VmError::runtime_error(format!("expected Tryte, got {:?}", v))),
        }
    }

    /// tryte_create(t0,t1,t2,t3,t4,t5) â†’ Tryte from 6 trit values
    fn tryte_create(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 6 { return Err(VmError::runtime_error("tryte_create(t0..t5) requires 6 trit args")); }
        let mut ts = [0i8; 6];
        for (i, a) in args.iter().enumerate() {
            ts[i] = Self::get_trit(a)?;
        }
        Ok(Value::Tryte(ts))
    }

    /// tryte_zero() â†’ Tryte of all T_ZERO
    fn tryte_zero(_args: &[Value]) -> Result<Value, VmError> {
        Ok(Value::Tryte([0i8; 6]))
    }

    /// tryte_from_int(n) â†’ Tryte encoding n in balanced ternary (clamped to -364..+364)
    fn tryte_from_int(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("tryte_from_int(n) requires 1 arg")); }
        let n = match &args[0] {
            Value::Number(x) => (*x as i64).clamp(-364, 364),
            _ => return Err(VmError::runtime_error("tryte_from_int: expected Number")),
        };
        let weights = [243i64, 81, 27, 9, 3, 1];
        let mut ts = [0i8; 6];
        let mut rem = n;
        for (i, &w) in weights.iter().enumerate() {
            // balanced ternary digit: round to nearest
            let _d = if rem >= 0 {
                if rem >= (w + 1) / 2 { 1 } else { 0 }
            } else {
                if rem <= -(w + 1) / 2 { -1 } else { 0 }
            };
            // greedy decode: pick digit that minimizes remainder
            let best = [-1i64, 0, 1].iter().copied()
                .min_by_key(|&d| (rem - d * w).abs())
                .unwrap_or(0);
            ts[i] = best as i8;
            rem -= best * w;
        }
        Ok(Value::Tryte(ts))
    }

    /// tryte_to_int(ty) â†’ Number â€” decode balanced ternary to integer
    fn tryte_to_int(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("tryte_to_int(ty) requires 1 arg")); }
        let ts = Self::get_tryte(&args[0])?;
        let weights = [243i64, 81, 27, 9, 3, 1];
        let val: i64 = ts.iter().zip(weights.iter()).map(|(&t, &w)| t as i64 * w).sum();
        Ok(Value::Number(val as f64))
    }

    /// tryte_to_str(ty) â†’ String like "+-0-+0"
    fn tryte_to_str(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("tryte_to_str(ty) requires 1 arg")); }
        let ts = Self::get_tryte(&args[0])?;
        let s: String = ts.iter().map(|t| match t {
            -1 => '-', 0 => '0', 1 => '+', _ => '?',
        }).collect();
        Ok(Value::Str(s))
    }

    /// tryte_get(ty, i) â†’ Trit at position i (0=most significant)
    fn tryte_get(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("tryte_get(ty, i) requires 2 args")); }
        let ts = Self::get_tryte(&args[0])?;
        let i = match &args[1] {
            Value::Number(n) => (*n as usize).min(5),
            _ => return Err(VmError::runtime_error("tryte_get: index must be Number")),
        };
        Ok(Value::Trit(ts[i]))
    }

    /// tryte_set(ty, i, trit) â†’ new Tryte with position i set to trit
    fn tryte_set(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 3 { return Err(VmError::runtime_error("tryte_set(ty, i, trit) requires 3 args")); }
        let mut ts = Self::get_tryte(&args[0])?;
        let i = match &args[1] {
            Value::Number(n) => (*n as usize).min(5),
            _ => return Err(VmError::runtime_error("tryte_set: index must be Number")),
        };
        ts[i] = Self::get_trit(&args[2])?;
        Ok(Value::Tryte(ts))
    }

    /// tryte_and(ty1, ty2) â†’ element-wise trit_and (min)
    fn tryte_and(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("tryte_and(ty1, ty2) requires 2 Trytes")); }
        let a = Self::get_tryte(&args[0])?;
        let b = Self::get_tryte(&args[1])?;
        let mut ts = [0i8; 6];
        for i in 0..6 { ts[i] = a[i].min(b[i]); }
        Ok(Value::Tryte(ts))
    }

    /// tryte_or(ty1, ty2) â†’ element-wise trit_or (max)
    fn tryte_or(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("tryte_or(ty1, ty2) requires 2 Trytes")); }
        let a = Self::get_tryte(&args[0])?;
        let b = Self::get_tryte(&args[1])?;
        let mut ts = [0i8; 6];
        for i in 0..6 { ts[i] = a[i].max(b[i]); }
        Ok(Value::Tryte(ts))
    }

    /// tryte_not(ty) â†’ element-wise trit_not (negate)
    fn tryte_not(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("tryte_not(ty) requires 1 Tryte")); }
        let a = Self::get_tryte(&args[0])?;
        let mut ts = [0i8; 6];
        for i in 0..6 { ts[i] = -a[i]; }
        Ok(Value::Tryte(ts))
    }

    /// tryte_add(ty1, ty2) â†’ Tryte â€” integer add, clamped to -364..+364
    fn tryte_add(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("tryte_add(ty1, ty2) requires 2 Trytes")); }
        let a = Self::get_tryte(&args[0])?;
        let b = Self::get_tryte(&args[1])?;
        let weights = [243i64, 81, 27, 9, 3, 1];
        let va: i64 = a.iter().zip(weights.iter()).map(|(&t, &w)| t as i64 * w).sum();
        let vb: i64 = b.iter().zip(weights.iter()).map(|(&t, &w)| t as i64 * w).sum();
        let sum = (va + vb).clamp(-364, 364);
        // re-encode sum as balanced ternary
        let mut ts = [0i8; 6];
        let mut rem = sum;
        for (i, &w) in weights.iter().enumerate() {
            let best = [-1i64, 0, 1].iter().copied()
                .min_by_key(|&d| (rem - d * w).abs())
                .unwrap_or(0);
            ts[i] = best as i8;
            rem -= best * w;
        }
        Ok(Value::Tryte(ts))
    }

    /// tryte_eq(ty1, ty2) â†’ Bool
    fn tryte_eq(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("tryte_eq(ty1, ty2) requires 2 Trytes")); }
        let a = Self::get_tryte(&args[0])?;
        let b = Self::get_tryte(&args[1])?;
        Ok(Value::Bool(a == b))
    }

    // =========================================================
    // v1.2: Native Hash Map builtins â€” O(1) average operations
    // Killer dicts are already backed by HashMap<String,Value>,
    // so these builtins are thin, zero-copy wrappers.
    // =========================================================

    /// hash_map_new() â†’ Dict
    fn hm_new(_args: &[Value]) -> Result<Value, VmError> {
        Ok(Value::Dict(Box::new(std::collections::HashMap::new())))
    }

    /// hash_map_insert(map, key, value) â†’ Dict
    fn hm_insert(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 3 {
            return Err(VmError::runtime_error("hash_map_insert(map, key, value) requires 3 args"));
        }
        let key = match &args[1] {
            Value::Str(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            other => return Err(VmError::runtime_error(
                format!("hash_map_insert: key must be Str or Number, got {:?}", other))),
        };
        let mut map = match &args[0] {
            Value::Dict(d) => *d.clone(),
            _ => return Err(VmError::runtime_error("hash_map_insert: first arg must be a Dict")),
        };
        map.insert(key, args[2].clone());
        Ok(Value::Dict(Box::new(map)))
    }

    /// hash_map_get(map, key) â†’ Value | Null
    fn hm_get(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 {
            return Err(VmError::runtime_error("hash_map_get(map, key) requires 2 args"));
        }
        let key = match &args[1] {
            Value::Str(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            other => return Err(VmError::runtime_error(
                format!("hash_map_get: key must be Str or Number, got {:?}", other))),
        };
        let map = match &args[0] {
            Value::Dict(d) => d,
            _ => return Err(VmError::runtime_error("hash_map_get: first arg must be a Dict")),
        };
        Ok(map.get(&key).cloned().unwrap_or(Value::Null))
    }

    /// Python `dict.get(key)` / `get(map, key[, default])`.
    fn dict_get(args: &[Value]) -> Result<Value, VmError> {
        if args.len() == 2 {
            return Self::hm_get(args);
        }
        if args.len() != 3 {
            return Err(VmError::runtime_error(
                "get() expects 2 or 3 arguments: get(map, key) or get(map, key, default)"
                    .to_string(),
            ));
        }
        let key = match &args[1] {
            Value::Str(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            other => {
                return Err(VmError::runtime_error(format!(
                    "get: key must be Str or Number, got {:?}",
                    other
                )))
            }
        };
        let map = match &args[0] {
            Value::Dict(d) => d,
            _ => {
                return Err(VmError::runtime_error(
                    "get: first argument must be a Dict".to_string(),
                ))
            }
        };
        Ok(map
            .get(&key)
            .cloned()
            .unwrap_or_else(|| args[2].clone()))
    }

    /// Python `dict.setdefault`: returns `[updatedDict, value]` (functional update, like `insert`).
    fn dict_setdefault(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 3 {
            return Err(VmError::runtime_error(
                "setdefault(map, key, default) requires 3 arguments".to_string(),
            ));
        }
        let key = match &args[1] {
            Value::Str(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            other => {
                return Err(VmError::runtime_error(format!(
                    "setdefault: key must be Str or Number, got {:?}",
                    other
                )))
            }
        };
        let mut map = match &args[0] {
            Value::Dict(d) => *d.clone(),
            _ => {
                return Err(VmError::runtime_error(
                    "setdefault: first argument must be a Dict".to_string(),
                ))
            }
        };
        let val = if let Some(v) = map.get(&key).cloned() {
            v
        } else {
            let def = args[2].clone();
            map.insert(key, def.clone());
            def
        };
        Ok(Value::from(vec![Value::Dict(Box::new(map)), val]))
    }

    /// hash_map_contains(map, key) â†’ Bool
    fn hm_contains(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 {
            return Err(VmError::runtime_error("hash_map_contains(map, key) requires 2 args"));
        }
        let key = match &args[1] {
            Value::Str(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            other => return Err(VmError::runtime_error(
                format!("hash_map_contains: key must be Str or Number, got {:?}", other))),
        };
        let map = match &args[0] {
            Value::Dict(d) => d,
            _ => return Err(VmError::runtime_error("hash_map_contains: first arg must be a Dict")),
        };
        Ok(Value::Bool(map.contains_key(&key)))
    }

    /// hash_map_remove(map, key) â†’ Dict
    fn hm_remove(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 {
            return Err(VmError::runtime_error("hash_map_remove(map, key) requires 2 args"));
        }
        let key = match &args[1] {
            Value::Str(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            other => return Err(VmError::runtime_error(
                format!("hash_map_remove: key must be Str or Number, got {:?}", other))),
        };
        let mut map = match &args[0] {
            Value::Dict(d) => *d.clone(),
            _ => return Err(VmError::runtime_error("hash_map_remove: first arg must be a Dict")),
        };
        map.remove(&key);
        Ok(Value::Dict(Box::new(map)))
    }

    /// hash_map_size(map) â†’ Number
    fn hm_size(args: &[Value]) -> Result<Value, VmError> {
        if args.is_empty() {
            return Err(VmError::runtime_error("hash_map_size(map) requires 1 arg"));
        }
        let map = match &args[0] {
            Value::Dict(d) => d,
            _ => return Err(VmError::runtime_error("hash_map_size: arg must be a Dict")),
        };
        Ok(Value::Number(map.len() as f64))
    }

    /// hash_map_keys(map) â†’ Array<Str>
    fn hm_keys(args: &[Value]) -> Result<Value, VmError> {
        if args.is_empty() {
            return Err(VmError::runtime_error("hash_map_keys(map) requires 1 arg"));
        }
        let map = match &args[0] {
            Value::Dict(d) => d,
            _ => return Err(VmError::runtime_error("hash_map_keys: arg must be a Dict")),
        };
        let mut keys: Vec<Value> = map.keys().map(|k| Value::Str(k.clone())).collect();
        keys.sort_by(|a, b| {
            if let (Value::Str(sa), Value::Str(sb)) = (a, b) { sa.cmp(sb) }
            else { std::cmp::Ordering::Equal }
        });
        Ok(Value::from(keys))
    }

    /// hash_map_values(map) â†’ Array<Value>
    fn hm_values(args: &[Value]) -> Result<Value, VmError> {
        if args.is_empty() {
            return Err(VmError::runtime_error("hash_map_values(map) requires 1 arg"));
        }
        let map = match &args[0] {
            Value::Dict(d) => d,
            _ => return Err(VmError::runtime_error("hash_map_values: arg must be a Dict")),
        };
        // return values in sorted-key order for determinism
        let mut pairs: Vec<(&String, &Value)> = map.iter().collect();
        pairs.sort_by_key(|(k, _)| k.as_str());
        let vals: Vec<Value> = pairs.into_iter().map(|(_, v)| v.clone()).collect();
        Ok(Value::from(vals))
    }

    // =========================================================
    // v1.2: Native Dijkstra builtins â€” O((V+E) log V)
    // adj_list format: Array of Arrays of Dicts
    //   adj_list[u] = [ {to: v, weight: w}, ... ]
    //   All vertex indices are 0-based integers.
    // =========================================================

    fn parse_adj_list(v: &Value) -> Result<Vec<Vec<(usize, i64)>>, VmError> {
        let outer = match v {
            Value::Array(a) => a,
            _ => return Err(VmError::runtime_error("dijkstra: adj_list must be an Array")),
        };
        let mut adj: Vec<Vec<(usize, i64)>> = Vec::with_capacity(outer.len());
        for row in outer.iter_cloned() {
            let edges = match row {
                Value::Array(a) => a,
                _ => return Err(VmError::runtime_error(
                    "dijkstra: each row in adj_list must be an Array",
                )),
            };
            let mut ev: Vec<(usize, i64)> = Vec::with_capacity(edges.len());
            for e in edges.iter_cloned() {
                let d = match e {
                    Value::Dict(d) => d,
                    _ => return Err(VmError::runtime_error(
                        "dijkstra: each edge must be a Dict {to, weight}",
                    )),
                };
                let to = match d.get("to") {
                    Some(Value::Number(n)) => *n as usize,
                    _ => return Err(VmError::runtime_error(
                        "dijkstra: edge missing numeric 'to' field",
                    )),
                };
                let weight = match d.get("weight") {
                    Some(Value::Number(n)) => *n as i64,
                    None => 1,
                    _ => return Err(VmError::runtime_error(
                        "dijkstra: edge 'weight' must be a Number",
                    )),
                };
                ev.push((to, weight));
            }
            adj.push(ev);
        }
        Ok(adj)
    }

    fn run_dijkstra(adj: &[Vec<(usize, i64)>], source: usize) -> Vec<i64> {
        use std::collections::BinaryHeap;
        use std::cmp::Reverse;
        const INF: i64 = i64::MAX / 2;
        let n = adj.len();
        let mut dist = vec![INF; n];
        if source >= n { return dist; }
        dist[source] = 0;
        // min-heap: (dist, vertex)
        let mut heap = BinaryHeap::new();
        heap.push(Reverse((0i64, source)));
        while let Some(Reverse((d, u))) = heap.pop() {
            if d > dist[u] { continue; }
            for &(v, w) in &adj[u] {
                let nd = d.saturating_add(w);
                if nd < dist[v] {
                    dist[v] = nd;
                    heap.push(Reverse((nd, v)));
                }
            }
        }
        dist
    }

    /// dijkstra(adj_list, source) â†’ Array<Number>
    ///   Returns shortest distances from source to every vertex.
    ///   Unreachable vertices have value 9007199254740992 (i64::MAX/2).
    fn dijkstra(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 {
            return Err(VmError::runtime_error("dijkstra(adj_list, source) requires 2 args"));
        }
        let adj = Self::parse_adj_list(&args[0])?;
        let source = match &args[1] {
            Value::Number(n) => *n as usize,
            _ => return Err(VmError::runtime_error("dijkstra: source must be a Number")),
        };
        let dist = Self::run_dijkstra(&adj, source);
        let result: Vec<Value> = dist.into_iter().map(|d| Value::Number(d as f64)).collect();
        Ok(Value::from(result))
    }

    /// dijkstra_path(adj_list, source, target) â†’ Array<Number>
    ///   Returns the vertex sequence of the shortest path sourceâ†’target,
    ///   or an empty array if target is unreachable.
    fn dijkstra_path(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 3 {
            return Err(VmError::runtime_error("dijkstra_path(adj_list, source, target) requires 3 args"));
        }
        let adj = Self::parse_adj_list(&args[0])?;
        let source = match &args[1] {
            Value::Number(n) => *n as usize,
            _ => return Err(VmError::runtime_error("dijkstra_path: source must be a Number")),
        };
        let target = match &args[2] {
            Value::Number(n) => *n as usize,
            _ => return Err(VmError::runtime_error("dijkstra_path: target must be a Number")),
        };
        let n = adj.len();
        if source >= n || target >= n {
            return Ok(Value::from(Vec::new()));
        }
        // Run Dijkstra while tracking predecessors
        use std::collections::BinaryHeap;
        use std::cmp::Reverse;
        const INF: i64 = i64::MAX / 2;
        let mut dist = vec![INF; n];
        let mut prev: Vec<Option<usize>> = vec![None; n];
        dist[source] = 0;
        let mut heap = BinaryHeap::new();
        heap.push(Reverse((0i64, source)));
        while let Some(Reverse((d, u))) = heap.pop() {
            if d > dist[u] { continue; }
            for &(v, w) in &adj[u] {
                let nd = d.saturating_add(w);
                if nd < dist[v] {
                    dist[v] = nd;
                    prev[v] = Some(u);
                    heap.push(Reverse((nd, v)));
                }
            }
        }
        if dist[target] == INF {
            return Ok(Value::from(Vec::new()));
        }
        // Reconstruct path
        let mut path = Vec::new();
        let mut cur = target;
        loop {
            path.push(Value::Number(cur as f64));
            if cur == source { break; }
            match prev[cur] {
                Some(p) => cur = p,
                None => return Ok(Value::from(Vec::new())),
            }
        }
        path.reverse();
        Ok(Value::from(path))
    }

    // â”€â”€â”€ Multilingual chat-language detection â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Detects what human language the user is chatting in, returns ISO-ish tag.
    fn detect_chat_lang(q: &str) -> &'static str {
        let lo = q.to_lowercase();

        // â”€â”€ keyword / greeting giveaways (checked first â€” fast) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        // Telugu (checked BEFORE Hindi because "namaskaram" contains "namaskar")
        if lo.contains("namaskaram") || lo.contains("baagunnara") || lo.contains("ela unnaru")
            || lo.contains("ela unnav") || lo.contains("em chestunnav") || lo.contains("cheppandi")
            || lo.contains("enti") || lo.contains("meeru") || lo.contains("nenu")
            || lo.contains("mama") || lo.contains("anna")
            || lo.contains("akkada") || lo.contains("ikkada") || lo.contains("eppudu")
            || lo.contains("enduku") || lo.contains("cheyyi") || lo.contains("raayandi")
            || lo.contains("baaga") || lo.contains("manchidi")
            || lo.contains("dhanyavaadalu") || lo.contains("sare") || lo.contains("avunu")
            || lo.contains("kaadu") || lo.contains("em chesthav") || lo.contains("evaru")
            || lo.contains("baagunnava") || lo.contains("veltunna")
        { return "te"; }
        // Hindi
        if lo.contains("namaste") || (lo.contains("namaskar") && !lo.contains("namaskaram"))
            || lo.contains("kaise ho") || lo.contains("kya hai") || lo.contains("kya haal")
            || lo.contains("aap") || lo.contains("kaise") || lo.contains("kya kar")
            || lo.contains("mujhe") || lo.contains("batao") || lo.contains("bhai")
            || lo.contains("dhanyavad") || lo.contains("shukriya") || lo.contains("bahut")
            || lo.contains("acha") || lo.contains("theek") || lo.contains("haan")
            || lo.contains("nahi") || lo.contains("kuch") || lo.contains("kaun")
            || lo.contains("kahan") || lo.contains("kaise hain") || lo.contains("kya hota")
            || lo.contains("samjhao") || lo.contains("alvida") || lo.contains("phir milenge")
            || lo.contains("madad") || lo.contains("kisne") || lo.contains("kaun ho")
        { return "hi"; }
        // Tamil
        if lo.contains("vanakkam") || lo.contains("epdi irukeenga") || lo.contains("eppadi")
            || lo.contains("nandri") || lo.contains("enna") || lo.contains("sollunga")
            || lo.contains("panna") || lo.contains("irukku") || lo.contains("theriyum")
            || lo.contains("nalla") || lo.contains("romba") || lo.contains("vaanga")
        { return "ta"; }
        // Kannada
        if lo.contains("namaskara") || lo.contains("hegiddira") || lo.contains("hege")
            || lo.contains("dhanyavadagalu") || lo.contains("yenu") || lo.contains("enu")
            || lo.contains("heli") || lo.contains("maadi") || lo.contains("banni")
        { return "kn"; }
        // Malayalam
        if lo.contains("namaskkaram") || lo.contains("sugamaano") || lo.contains("enthanu")
            || lo.contains("nanni") || lo.contains("sthuthikal") || lo.contains("eppadi")
        { return "ml"; }
        // Bengali
        if lo.contains("nomoshkar") || lo.contains("kemon acho") || lo.contains("kemon achho")
            || lo.contains("dhonnobad") || lo.contains("bolo") || lo.contains("aacha")
        { return "bn"; }
        // Marathi
        if lo.contains("namaskar") || lo.contains("kasa aahat") || lo.contains("kasa aahes")
            || lo.contains("dhanyavad") || lo.contains("kay") || lo.contains("sangaa")
        { return "mr"; }
        // Gujarati
        if lo.contains("kem cho") || lo.contains("aabhar") || lo.contains("shu")
        { return "gu"; }
        // Punjabi
        if lo.contains("sat sri akaal") || lo.contains("ki haal") || lo.contains("meherbani")
        { return "pa"; }
        // Urdu
        if lo.contains("adaab") || lo.contains("kaise hain aap") || lo.contains("shukriya")
            || lo.contains("salaam") || lo.contains("khuda hafiz")
        { return "ur"; }
        // Spanish
        if lo.contains("hola") || lo.contains("buenos") || lo.contains("buenas")
            || lo.contains("gracias") || lo.contains("por favor") || lo.contains("como estas")
            || lo.contains("cÃ³mo estÃ¡s") || lo.contains("que tal") || lo.contains("adios")
            || lo.contains("adiÃ³s") || lo.contains("necesito") || lo.contains("puedes")
            || lo.contains("ayuda") || lo == "vale" || lo.contains("entendido")
            || lo.contains("genial") || lo.contains("hasta luego") || lo.contains("nos vemos")
        { return "es"; }
        // French
        if lo.contains("bonjour") || lo.contains("salut") || lo.contains("merci")
            || lo.contains("s'il vous") || lo.contains("comment allez") || lo.contains("au revoir")
            || lo.contains("bonsoir") || lo.contains("je suis") || lo.contains("oui")
            || lo.contains("non merci") || lo.contains("excusez")
        { return "fr"; }
        // German
        if lo.contains("guten tag") || lo.contains("guten morgen") || lo.contains("hallo")
            || lo.contains("danke") || lo.contains("bitte") || lo.contains("wie geht")
            || lo.contains("tschÃ¼ss") || lo.contains("tschuss") || lo.contains("auf wiedersehen")
            || lo.contains("ich bin") || lo.contains("kannst du")
        { return "de"; }
        // Italian
        if lo.contains("ciao") || lo.contains("buongiorno") || lo.contains("buonasera")
            || lo.contains("grazie") || lo.contains("per favore") || lo.contains("come stai")
            || lo.contains("arrivederci") || lo.contains("sono")
        { return "it"; }
        // Portuguese
        if lo.contains("olÃ¡") || lo.contains("ola") || lo.contains("obrigado") || lo.contains("obrigada")
            || lo.contains("como vocÃª") || lo.contains("bom dia") || lo.contains("boa tarde")
            || lo.contains("boa noite") || lo.contains("tchau") || lo.contains("tudo bem")
            || lo.contains("atÃ© logo") || lo.contains("ate logo")
        { return "pt"; }
        // Russian
        if lo.contains("Ð¿Ñ€Ð¸Ð²ÐµÑ‚") || lo.contains("Ð·Ð´Ñ€Ð°Ð²ÑÑ‚Ð²ÑƒÐ¹Ñ‚Ðµ") || lo.contains("ÑÐ¿Ð°ÑÐ¸Ð±Ð¾")
            || lo.contains("Ð¿Ð¾Ð¶Ð°Ð»ÑƒÐ¹ÑÑ‚Ð°") || lo.contains("ÐºÐ°Ðº Ð´ÐµÐ»Ð°") || lo.contains("Ñ…Ð¾Ñ€Ð¾ÑˆÐ¾")
            || lo.contains("Ð´Ð¾ ÑÐ²Ð¸Ð´Ð°Ð½Ð¸Ñ") || lo.contains("Ð¿Ð¾Ð¼Ð¾Ð³Ð¸") || lo.contains("Ñ‡Ñ‚Ð¾")
        { return "ru"; }
        // Japanese
        if lo.contains("ã“ã‚“ã«ã¡ã¯") || lo.contains("ãŠã¯ã‚ˆã†") || lo.contains("ã“ã‚“ã°ã‚“ã¯")
            || lo.contains("ã‚ã‚ŠãŒã¨ã†") || lo.contains("ãŠé¡˜ã„") || lo.contains("ã•ã‚ˆã†ãªã‚‰")
            || lo.contains("ã™ã¿ã¾ã›ã‚“") || lo.contains("ã¯ã„") || lo.contains("å…ƒæ°—")
        { return "ja"; }
        // Korean
        if lo.contains("ì•ˆë…•") || lo.contains("ê°ì‚¬") || lo.contains("ê³ ë§ˆì›Œ")
            || lo.contains("ë„ì™€ì¤˜") || lo.contains("ë„¤") || lo.contains("ì•„ë‹ˆìš”")
        { return "ko"; }
        // Chinese
        if lo.contains("ä½ å¥½") || lo.contains("è°¢è°¢") || lo.contains("è¯·") || lo.contains("å†è§")
            || lo.contains("æ€Žä¹ˆ") || lo.contains("ä»€ä¹ˆ") || lo.contains("å¸®")
        { return "zh"; }
        // Arabic
        if lo.contains("Ù…Ø±Ø­Ø¨Ø§") || lo.contains("Ø´ÙƒØ±Ø§") || lo.contains("Ù…Ù† ÙØ¶Ù„Ùƒ")
            || lo.contains("ÙƒÙŠÙ") || lo.contains("Ù…Ø¹ Ø§Ù„Ø³Ù„Ø§Ù…Ø©") || lo.contains("Ø£Ù‡Ù„Ø§")
        { return "ar"; }
        // Turkish
        if lo.contains("merhaba") || lo.contains("teÅŸekkÃ¼r") || lo.contains("nasÄ±lsÄ±n")
            || lo.contains("lÃ¼tfen") || lo.contains("gÃ¼le gÃ¼le") || lo.contains("evet")
            || lo.contains("hayÄ±r")
        { return "tr"; }
        // Dutch
        if lo.contains("hallo") || lo.contains("bedankt") || lo.contains("alsjeblieft")
            || lo.contains("hoe gaat het") || lo.contains("tot ziens") || lo.contains("dank je")
        { return "nl"; }
        // Polish
        if lo.contains("czeÅ›Ä‡") || lo.contains("dzieÅ„ dobry") || lo.contains("dziÄ™kujÄ™")
            || lo.contains("proszÄ™") || lo.contains("jak siÄ™ masz") || lo.contains("do widzenia")
        { return "pl"; }
        // Thai
        if lo.contains("à¸ªà¸§à¸±à¸ªà¸”à¸µ") || lo.contains("à¸‚à¸­à¸šà¸„à¸¸à¸“") || lo.contains("à¸„à¸£à¸±à¸š") || lo.contains("à¸„à¹ˆà¸°")
        { return "th"; }
        // Vietnamese
        if lo.contains("xin chÃ o") || lo.contains("cáº£m Æ¡n") || lo.contains("xin")
        { return "vi"; }
        // Indonesian / Malay
        if lo.contains("selamat") || lo.contains("terima kasih") || lo.contains("tolong")
            || lo.contains("apa kabar")
        { return "id"; }
        // Swahili
        if lo.contains("habari") || lo.contains("asante") || lo.contains("tafadhali")
            || lo.contains("kwaheri")
        { return "sw"; }
        // Hebrew
        if lo.contains("×©×œ×•×") || lo.contains("×ª×•×“×”") || lo.contains("×‘×‘×§×©×”")
        { return "he"; }
        // Greek
        if lo.contains("Î³ÎµÎ¹Î±") || lo.contains("ÎµÏ…Ï‡Î±ÏÎ¹ÏƒÏ„ÏŽ") || lo.contains("Ï€Î±ÏÎ±ÎºÎ±Î»ÏŽ")
        { return "el"; }
        // Swedish
        if lo.contains("hej") || lo.contains("tack") || lo.contains("snÃ¤lla")
            || lo.contains("hur mÃ¥r du")
        { return "sv"; }

        // â”€â”€ Unicode script detection (fallback for pure-script text) â”€â”€â”€â”€â”€â”€â”€â”€
        let mut devanagari = 0u32;
        let mut telugu_c = 0u32;
        let mut tamil_c = 0u32;
        let mut kannada_c = 0u32;
        let mut malayalam_c = 0u32;
        let mut bengali_c = 0u32;
        let mut cyrillic = 0u32;
        let mut cjk = 0u32;
        let mut hangul = 0u32;
        let mut hiragana_katakana = 0u32;
        let mut arabic_c = 0u32;
        let mut thai_c = 0u32;
        let mut hebrew_c = 0u32;
        let mut greek_c = 0u32;
        let mut gujarati_c = 0u32;
        let mut gurmukhi_c = 0u32;
        let total = q.chars().filter(|c| !c.is_whitespace() && !c.is_ascii_punctuation()).count() as u32;
        for ch in q.chars() {
            match ch as u32 {
                0x0900..=0x097F => devanagari += 1,
                0x0C00..=0x0C7F => telugu_c += 1,
                0x0B80..=0x0BFF => tamil_c += 1,
                0x0C80..=0x0CFF => kannada_c += 1,
                0x0D00..=0x0D7F => malayalam_c += 1,
                0x0980..=0x09FF => bengali_c += 1,
                0x0A80..=0x0AFF => gujarati_c += 1,
                0x0A00..=0x0A7F => gurmukhi_c += 1,
                0x0400..=0x04FF => cyrillic += 1,
                0x4E00..=0x9FFF | 0x3400..=0x4DBF => cjk += 1,
                0xAC00..=0xD7AF | 0x1100..=0x11FF => hangul += 1,
                0x3040..=0x309F | 0x30A0..=0x30FF => hiragana_katakana += 1,
                0x0600..=0x06FF | 0x0750..=0x077F => arabic_c += 1,
                0x0E00..=0x0E7F => thai_c += 1,
                0x0590..=0x05FF => hebrew_c += 1,
                0x0370..=0x03FF => greek_c += 1,
                _ => {}
            }
        }
        let threshold = if total > 4 { total / 4 } else { 1 };
        if devanagari >= threshold  { return "hi"; }
        if telugu_c >= threshold    { return "te"; }
        if tamil_c >= threshold     { return "ta"; }
        if kannada_c >= threshold   { return "kn"; }
        if malayalam_c >= threshold { return "ml"; }
        if bengali_c >= threshold   { return "bn"; }
        if gujarati_c >= threshold  { return "gu"; }
        if gurmukhi_c >= threshold  { return "pa"; }
        if cyrillic >= threshold    { return "ru"; }
        if hiragana_katakana >= threshold { return "ja"; }
        if hangul >= threshold      { return "ko"; }
        if cjk >= threshold         { return "zh"; }
        if arabic_c >= threshold    { return "ar"; }
        if thai_c >= threshold      { return "th"; }
        if hebrew_c >= threshold    { return "he"; }
        if greek_c >= threshold     { return "el"; }

        "en"
    }

    /// Returns a localized response for the given category and detected language.
    fn kala_localized(chat_lang: &str, category: &str, name: Option<&str>) -> String {
        match (chat_lang, category) {
            // â”€â”€ GREETING â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
            ("hi", "greeting") => "à¤¹à¥‡! à¤®à¥ˆà¤‚ Kala à¤¹à¥‚à¤à¥¤ à¤¬à¤¤à¤¾à¤“, à¤•à¥à¤¯à¤¾ à¤®à¤¦à¤¦ à¤•à¤°à¥‚à¤?".into(),
            ("te", "greeting") => "à°¹à°¾à°¯à±! à°¨à±‡à°¨à± Kala à°¨à°¿. à°à°‚ help à°•à°¾à°µà°¾à°²à°¿?".into(),
            ("ta", "greeting") => "à®µà®£à®•à¯à®•à®®à¯! à®¨à®¾à®©à¯ Kala. à®Žà®©à¯à®© help à®µà¯‡à®£à¯à®®à¯?".into(),
            ("kn", "greeting") => "à²¨à²®à²¸à³à²•à²¾à²°! à²¨à²¾à²¨à³ Kala. à²à²¨à³ à²¸à²¹à²¾à²¯ à²¬à³‡à²•à³?".into(),
            ("ml", "greeting") => "à´¨à´®à´¸àµà´•à´¾à´°à´‚! à´žà´¾àµ» Kala à´†à´£àµ. à´Žà´¨àµà´¤àµ à´¸à´¹à´¾à´¯à´‚ à´µàµ‡à´£à´‚?".into(),
            ("bn", "greeting") => "à¦¹à§à¦¯à¦¾à¦²à§‹! à¦†à¦®à¦¿ Kalaà¥¤ à¦•à§€à¦­à¦¾à¦¬à§‡ à¦¸à¦¾à¦¹à¦¾à¦¯à§à¦¯ à¦•à¦°à¦¬?".into(),
            ("mr", "greeting") => "à¤¨à¤®à¤¸à¥à¤•à¤¾à¤°! à¤®à¥€ Kala à¤†à¤¹à¥‡. à¤•à¤¾à¤¯ à¤®à¤¦à¤¤ à¤•à¤°à¥‚?".into(),
            ("gu", "greeting") => "àª¨àª®àª¸à«àª¤à«‡! àª¹à«àª‚ Kala àª›à«àª‚. àª¶à«àª‚ àª®àª¦àª¦ àª•àª°à«àª‚?".into(),
            ("pa", "greeting") => "à¨¸à¨¤ à¨¸à©à¨°à©€ à¨…à¨•à¨¾à¨²! à¨®à©ˆà¨‚ Kala à¨¹à¨¾à¨‚à¥¤ à¨•à©€ à¨®à¨¦à¨¦ à¨•à¨°à¨¾à¨‚?".into(),
            ("ur", "greeting") => "Ø³Ù„Ø§Ù…! Ù…ÛŒÚº Kala ÛÙˆÚºÛ” Ú©ÛŒØ§ Ù…Ø¯Ø¯ Ú©Ø±ÙˆÚº?".into(),
            ("es", "greeting") => "Â¡Hola! Soy Kala. Â¿En quÃ© te ayudo?".into(),
            ("fr", "greeting") => "Salut ! Je suis Kala. Comment puis-je t'aider ?".into(),
            ("de", "greeting") => "Hallo! Ich bin Kala. Wie kann ich dir helfen?".into(),
            ("it", "greeting") => "Ciao! Sono Kala. Come posso aiutarti?".into(),
            ("pt", "greeting") => "OlÃ¡! Eu sou Kala. Como posso ajudar?".into(),
            ("ru", "greeting") => "ÐŸÑ€Ð¸Ð²ÐµÑ‚! Ð¯ Kala. Ð§ÐµÐ¼ Ð¼Ð¾Ð³Ñƒ Ð¿Ð¾Ð¼Ð¾Ñ‡ÑŒ?".into(),
            ("ja", "greeting") => "ã“ã‚“ã«ã¡ã¯ï¼Kalaã§ã™ã€‚ä½•ã‹ãŠæ‰‹ä¼ã„ã—ã¾ã—ã‚‡ã†ã‹ï¼Ÿ".into(),
            ("ko", "greeting") => "ì•ˆë…•í•˜ì„¸ìš”! ì €ëŠ” Kalaì˜ˆìš”. ë¬´ì—‡ì„ ë„ì™€ë“œë¦´ê¹Œìš”?".into(),
            ("zh", "greeting") => "ä½ å¥½ï¼æˆ‘æ˜¯Kalaã€‚éœ€è¦ä»€ä¹ˆå¸®åŠ©ï¼Ÿ".into(),
            ("ar", "greeting") => "Ø£Ù‡Ù„Ø§Ù‹! Ø£Ù†Ø§ Kala. ÙƒÙŠÙ Ø£Ù‚Ø¯Ø± Ø£Ø³Ø§Ø¹Ø¯ÙƒØŸ".into(),
            ("tr", "greeting") => "Merhaba! Ben Kala. NasÄ±l yardÄ±mcÄ± olabilirim?".into(),
            ("nl", "greeting") => "Hallo! Ik ben Kala. Hoe kan ik je helpen?".into(),
            ("pl", "greeting") => "CzeÅ›Ä‡! Jestem Kala. Jak mogÄ™ pomÃ³c?".into(),
            ("th", "greeting") => "à¸ªà¸§à¸±à¸ªà¸”à¸µà¸„à¸£à¸±à¸š! à¸œà¸¡ Kala à¸„à¸£à¸±à¸š à¸Šà¹ˆà¸§à¸¢à¸­à¸°à¹„à¸£à¹„à¸”à¹‰à¸šà¹‰à¸²à¸‡?".into(),
            ("vi", "greeting") => "Xin chÃ o! TÃ´i lÃ  Kala. Cáº§n giÃºp gÃ¬ khÃ´ng?".into(),
            ("id", "greeting") => "Halo! Saya Kala. Ada yang bisa dibantu?".into(),
            ("sw", "greeting") => "Habari! Mimi ni Kala. Nikisaidie nini?".into(),
            ("he", "greeting") => "!×©×œ×•×! ×× ×™ Kala. ××™×š ××•×›×œ ×œ×¢×–×•×¨".into(),
            ("el", "greeting") => "Î“ÎµÎ¹Î±! Î•Î¯Î¼Î±Î¹ Î· Kala. Î ÏŽÏ‚ Î¼Ï€Î¿ÏÏŽ Î½Î± Î²Î¿Î·Î¸Î®ÏƒÏ‰;".into(),
            ("sv", "greeting") => "Hej! Jag Ã¤r Kala. Hur kan jag hjÃ¤lpa dig?".into(),

            // â”€â”€ GREETING with name â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
            ("hi", "greeting_name") => format!("à¤¹à¥‡ {}! à¤®à¥ˆà¤‚ Kala à¤¹à¥‚à¤ â€” à¤•à¥à¤› à¤­à¥€ à¤ªà¥‚à¤›à¥‹à¥¤", name.unwrap_or("friend")),
            ("te", "greeting_name") => format!("à°¹à°¾à°¯à± {}! à°¨à±‡à°¨à± Kala à°¨à°¿. à°à°¦à±ˆà°¨à°¾ à°…à°¡à±à°—à±.", name.unwrap_or("friend")),
            ("ta", "greeting_name") => format!("à®µà®£à®•à¯à®•à®®à¯ {}! à®¨à®¾à®©à¯ Kala. à®Žà®¤à¯à®µà¯à®®à¯ à®•à¯‡à®³à¯à®™à¯à®•à®³à¯.", name.unwrap_or("friend")),
            ("kn", "greeting_name") => format!("à²¨à²®à²¸à³à²•à²¾à²° {}! à²¨à²¾à²¨à³ Kala. à²à²¨à²¾à²¦à²°à³‚ à²•à³‡à²³à²¿.", name.unwrap_or("friend")),
            ("ml", "greeting_name") => format!("à´¨à´®à´¸àµà´•à´¾à´°à´‚ {}! à´žà´¾àµ» Kala. à´Žà´¨àµà´¤àµà´‚ à´šàµ‹à´¦à´¿à´•àµà´•àµ‚.", name.unwrap_or("friend")),
            ("bn", "greeting_name") => format!("à¦¹à§à¦¯à¦¾à¦²à§‹ {}! à¦†à¦®à¦¿ Kalaà¥¤ à¦¯à§‡à¦•à§‹à¦¨à§‹ à¦•à¦¿à¦›à§ à¦œà¦¿à¦œà§à¦žà§‡à¦¸ à¦•à¦°à§‹à¥¤", name.unwrap_or("friend")),
            ("es", "greeting_name") => format!("Â¡Hola {}! Soy Kala â€” pregÃºntame lo que quieras.", name.unwrap_or("amigo")),
            ("fr", "greeting_name") => format!("Salut {} ! Je suis Kala â€” demande-moi n'importe quoi.", name.unwrap_or("ami")),
            ("de", "greeting_name") => format!("Hallo {}! Ich bin Kala â€” frag mich einfach.", name.unwrap_or("Freund")),
            ("it", "greeting_name") => format!("Ciao {}! Sono Kala â€” chiedimi qualsiasi cosa.", name.unwrap_or("amico")),
            ("pt", "greeting_name") => format!("OlÃ¡ {}! Sou Kala â€” pergunte o que quiser.", name.unwrap_or("amigo")),
            ("ru", "greeting_name") => format!("ÐŸÑ€Ð¸Ð²ÐµÑ‚, {}! Ð¯ Kala â€” ÑÐ¿Ñ€Ð°ÑˆÐ¸Ð²Ð°Ð¹ Ñ‡Ñ‚Ð¾ ÑƒÐ³Ð¾Ð´Ð½Ð¾.", name.unwrap_or("Ð´Ñ€ÑƒÐ³")),
            ("ja", "greeting_name") => format!("ã“ã‚“ã«ã¡ã¯ã€{}ã•ã‚“ï¼Kalaã§ã™ã€‚ä½•ã§ã‚‚èžã„ã¦ãã ã•ã„ã€‚", name.unwrap_or("friend")),
            ("ko", "greeting_name") => format!("ì•ˆë…•í•˜ì„¸ìš”, {}! ì €ëŠ” Kalaì˜ˆìš”. ë¬´ì—‡ì´ë“  ë¬¼ì–´ë³´ì„¸ìš”.", name.unwrap_or("friend")),
            ("zh", "greeting_name") => format!("ä½ å¥½ï¼Œ{}ï¼æˆ‘æ˜¯Kalaï¼Œéšä¾¿é—®æˆ‘ä»€ä¹ˆã€‚", name.unwrap_or("æœ‹å‹")),
            ("ar", "greeting_name") => format!("Ø£Ù‡Ù„Ø§Ù‹ {}! Ø£Ù†Ø§ Kala â€” Ø§Ø³Ø£Ù„Ù†ÙŠ Ø£ÙŠ Ø´ÙŠØ¡.", name.unwrap_or("ØµØ¯ÙŠÙ‚ÙŠ")),
            ("tr", "greeting_name") => format!("Merhaba {}! Ben Kala â€” ne istersen sor.", name.unwrap_or("arkadaÅŸ")),
            (_, "greeting_name") => format!("Hey {}! Nice to meet you. I'm Kala â€” ask me anything.", name.unwrap_or("friend")),
            (_, "greeting") => "Hey! I'm Kala. What can I help you with?".into(),

            // â”€â”€ NAME INTRODUCTION â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
            ("hi", "name_intro") => format!("à¤…à¤šà¥à¤›à¤¾ {}! à¤®à¥ˆà¤‚ Kala à¤¹à¥‚à¤à¥¤ à¤¤à¥à¤®à¥à¤¹à¤¾à¤°à¥‡ à¤²à¤¿à¤ à¤•à¥à¤¯à¤¾ à¤•à¤°à¥‚à¤?", name.unwrap_or("friend")),
            ("te", "name_intro") => format!("à°¬à°¾à°—à±à°‚à°¦à°¿ {}! à°¨à±‡à°¨à± Kala à°¨à°¿. à°¨à±€ à°•à±‹à°¸à°‚ à°à°‚ à°šà±‡à°¯à°¾à°²à°¿?", name.unwrap_or("friend")),
            ("ta", "name_intro") => format!("à®šà®¨à¯à®¤à¯‹à®·à®®à¯ {}! à®¨à®¾à®©à¯ Kala. à®‰à®©à®•à¯à®•à¯ à®Žà®©à¯à®© à®šà¯†à®¯à¯à®¯à®£à¯à®®à¯?", name.unwrap_or("friend")),
            ("es", "name_intro") => format!("Â¡Mucho gusto, {}! Soy Kala. Â¿QuÃ© necesitas?", name.unwrap_or("amigo")),
            ("fr", "name_intro") => format!("EnchantÃ©, {} ! Je suis Kala. Que puis-je faire pour toi ?", name.unwrap_or("ami")),
            ("de", "name_intro") => format!("Freut mich, {}! Ich bin Kala. Was kann ich fÃ¼r dich tun?", name.unwrap_or("Freund")),
            ("it", "name_intro") => format!("Piacere, {}! Sono Kala. Cosa posso fare per te?", name.unwrap_or("amico")),
            ("pt", "name_intro") => format!("Prazer, {}! Sou Kala. O que posso fazer por vocÃª?", name.unwrap_or("amigo")),
            ("ru", "name_intro") => format!("ÐŸÑ€Ð¸ÑÑ‚Ð½Ð¾ Ð¿Ð¾Ð·Ð½Ð°ÐºÐ¾Ð¼Ð¸Ñ‚ÑŒÑÑ, {}! Ð¯ Kala. Ð§ÐµÐ¼ Ð¿Ð¾Ð¼Ð¾Ñ‡ÑŒ?", name.unwrap_or("Ð´Ñ€ÑƒÐ³")),
            ("ja", "name_intro") => format!("ã¯ã˜ã‚ã¾ã—ã¦ã€{}ã•ã‚“ï¼Kalaã§ã™ã€‚ä½•ã‚’ã—ã¾ã—ã‚‡ã†ã‹ï¼Ÿ", name.unwrap_or("friend")),
            ("ko", "name_intro") => format!("ë°˜ê°€ì›Œìš”, {}! ì €ëŠ” Kalaì˜ˆìš”. ë­˜ ë„ì™€ë“œë¦´ê¹Œìš”?", name.unwrap_or("friend")),
            ("zh", "name_intro") => format!("å¾ˆé«˜å…´è®¤è¯†ä½ ï¼Œ{}ï¼æˆ‘æ˜¯Kalaã€‚éœ€è¦ä»€ä¹ˆï¼Ÿ", name.unwrap_or("æœ‹å‹")),
            ("ar", "name_intro") => format!("ØªØ´Ø±ÙØª {}! Ø£Ù†Ø§ Kala. ÙƒÙŠÙ Ø£Ù‚Ø¯Ø± Ø£Ø³Ø§Ø¹Ø¯ÙƒØŸ", name.unwrap_or("ØµØ¯ÙŠÙ‚ÙŠ")),
            ("tr", "name_intro") => format!("Memnun oldum, {}! Ben Kala. Ne yapayÄ±m?", name.unwrap_or("arkadaÅŸ")),
            (_, "name_intro") => format!("Nice to meet you, {}! I'm Kala. What can I do for you?", name.unwrap_or("friend")),

            // â”€â”€ WELLBEING â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
            ("hi", "wellbeing") => "à¤¬à¤¢à¤¼à¤¿à¤¯à¤¾ à¤¹à¥‚à¤, à¤ªà¥‚à¤›à¤¨à¥‡ à¤•à¤¾ à¤¶à¥à¤•à¥à¤°à¤¿à¤¯à¤¾! à¤¬à¤¤à¤¾à¤“ à¤•à¥à¤¯à¤¾ à¤®à¤¨ à¤®à¥‡à¤‚ à¤¹à¥ˆ?".into(),
            ("te", "wellbeing") => "à°¬à°¾à°—à±à°¨à±à°¨à°¾, à°…à°¡à°¿à°—à°¿à°¨à°‚à°¦à±à°•à± thanks! à°à°‚ à°šà±†à°ªà±à°ªà±?".into(),
            ("ta", "wellbeing") => "à®¨à®²à¯à®²à®¾ à®‡à®°à¯à®•à¯à®•à¯‡à®©à¯, à®•à¯‡à®Ÿà¯à®Ÿà®¤à¯à®•à¯à®•à¯ à®¨à®©à¯à®±à®¿! à®Žà®©à¯à®© à®šà¯Šà®²à¯à®²à¯?".into(),
            ("kn", "wellbeing") => "à²šà³†à²¨à³à²¨à²¾à²—à²¿à²¦à³à²¦à³€à²¨à²¿, à²•à³‡à²³à²¿à²¦à³à²¦à²•à³à²•à³† à²§à²¨à³à²¯à²µà²¾à²¦! à²à²¨à³ à²¹à³‡à²³à³?".into(),
            ("ml", "wellbeing") => "à´¨à´²àµà´²à´¤àµ, à´šàµ‹à´¦à´¿à´šàµà´šà´¤à´¿à´¨àµ à´¨à´¨àµà´¦à´¿! à´Žà´¨àµà´¤à´¾ à´ªà´±à´¯àµà´¨àµà´¨à´¤àµ?".into(),
            ("bn", "wellbeing") => "à¦­à¦¾à¦²à§‹ à¦†à¦›à¦¿, à¦œà¦¿à¦œà§à¦žà§‡à¦¸ à¦•à¦°à¦¾à¦° à¦œà¦¨à§à¦¯ à¦§à¦¨à§à¦¯à¦¬à¦¾à¦¦! à¦•à§€ à¦¬à¦²à¦¬à§‡?".into(),
            ("es", "wellbeing") => "Â¡Estoy bien, gracias por preguntar! Â¿QuÃ© tienes en mente?".into(),
            ("fr", "wellbeing") => "Je vais bien, merci de demander ! Qu'est-ce que tu as en tÃªte ?".into(),
            ("de", "wellbeing") => "Mir geht's gut, danke der Nachfrage! Was hast du auf dem Herzen?".into(),
            ("it", "wellbeing") => "Sto bene, grazie per aver chiesto! Cosa hai in mente?".into(),
            ("pt", "wellbeing") => "Estou bem, obrigado por perguntar! O que vocÃª tem em mente?".into(),
            ("ru", "wellbeing") => "Ð¥Ð¾Ñ€Ð¾ÑˆÐ¾, ÑÐ¿Ð°ÑÐ¸Ð±Ð¾ Ñ‡Ñ‚Ð¾ ÑÐ¿Ñ€Ð¾ÑÐ¸Ð»! Ð§Ñ‚Ð¾ Ñƒ Ñ‚ÐµÐ±Ñ Ð½Ð° ÑƒÐ¼Ðµ?".into(),
            ("ja", "wellbeing") => "å…ƒæ°—ã§ã™ã‚ˆã€èžã„ã¦ãã‚Œã¦ã‚ã‚ŠãŒã¨ã†ï¼ä½•ã‹ç”¨ï¼Ÿ".into(),
            ("ko", "wellbeing") => "ìž˜ ì§€ë‚´ê³  ìžˆì–´ìš”, ë¬¼ì–´ë´ ì¤˜ì„œ ê³ ë§ˆì›Œìš”! ë­ í•„ìš”í•´ìš”?".into(),
            ("zh", "wellbeing") => "æˆ‘å¾ˆå¥½ï¼Œè°¢è°¢å…³å¿ƒï¼ä½ æœ‰ä»€ä¹ˆæƒ³é—®çš„ï¼Ÿ".into(),
            ("ar", "wellbeing") => "Ø¨Ø®ÙŠØ±ØŒ Ø´ÙƒØ±Ø§Ù‹ Ø¹Ù„Ù‰ Ø§Ù„Ø³Ø¤Ø§Ù„! Ø´Ùˆ Ø¹Ù†Ø¯ÙƒØŸ".into(),
            ("tr", "wellbeing") => "Ä°yiyim, sorduÄŸun iÃ§in teÅŸekkÃ¼rler! Ne var ne yok?".into(),
            (_, "wellbeing") => "Doing good, thanks for asking! What's on your mind?".into(),

            // â”€â”€ CREATOR â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
            ("hi", "creator") => "Sai Arun Kumar Katherashala à¤¨à¥‡ à¤¬à¤¨à¤¾à¤¯à¤¾ à¤¹à¥ˆà¥¤ Killer à¤”à¤° Kala à¤¦à¥‹à¤¨à¥‹à¤‚ Rust à¤®à¥‡à¤‚ à¤¬à¤¨à¤¾à¤ à¤¹à¥ˆà¤‚à¥¤\n\nà¤”à¤° à¤•à¥à¤› à¤œà¤¾à¤¨à¤¨à¤¾ à¤¹à¥ˆ?".into(),
            ("te", "creator") => "Sai Arun Kumar Katherashala build à°šà±‡à°¶à°¾à°°à±. Killer, Kala à°°à±†à°‚à°¡à±‚ Rust à°²à±‹ build à°šà±‡à°¸à°¾à°°à±.\n\nà°‡à°‚à°•à°¾ à°à°®à±ˆà°¨à°¾ à°¤à±†à°²à±à°¸à±à°•à±‹à°µà°¾à°²à°¾?".into(),
            ("ta", "creator") => "Sai Arun Kumar Katherashala à®‰à®°à¯à®µà®¾à®•à¯à®•à®¿à®©à®¾à®°à¯. Killer-à®®à¯ Kala-à®µà¯à®®à¯ Rust-à®²à¯ à®Žà®´à¯à®¤à®ªà¯à®ªà®Ÿà¯à®Ÿà®¤à¯.\n\nà®µà¯‡à®± à®à®¤à®¾à®µà®¤à¯ à®¤à¯†à®°à®¿à®žà¯à®šà¯à®•à¯à®•à®£à¯à®®à®¾?".into(),
            ("es", "creator") => "Sai Arun Kumar Katherashala. ConstruyÃ³ Killer y Kala desde cero en Rust.\n\nÂ¿Quieres saber algo mÃ¡s?".into(),
            ("fr", "creator") => "Sai Arun Kumar Katherashala. Il a construit Killer et Kala de zÃ©ro en Rust.\n\nAutre chose Ã  savoir ?".into(),
            ("de", "creator") => "Sai Arun Kumar Katherashala. Er hat Killer und Kala von Grund auf in Rust gebaut.\n\nNoch etwas wissen wollen?".into(),
            ("it", "creator") => "Sai Arun Kumar Katherashala. Ha costruito Killer e Kala da zero in Rust.\n\nVuoi sapere altro?".into(),
            ("pt", "creator") => "Sai Arun Kumar Katherashala. Ele construiu Killer e Kala do zero em Rust.\n\nQuer saber mais alguma coisa?".into(),
            ("ru", "creator") => "Sai Arun Kumar Katherashala. ÐžÐ½ ÑÐ¾Ð·Ð´Ð°Ð» Killer Ð¸ Kala Ñ Ð½ÑƒÐ»Ñ Ð½Ð° Rust.\n\nÐ•Ñ‰Ñ‘ Ñ‡Ñ‚Ð¾-Ñ‚Ð¾ Ñ…Ð¾Ñ‡ÐµÑˆÑŒ ÑƒÐ·Ð½Ð°Ñ‚ÑŒ?".into(),
            ("ja", "creator") => "Sai Arun Kumar KatherashalaãŒä½œã‚Šã¾ã—ãŸã€‚Killerã‚‚Kalaã‚‚Rustã§ã‚¼ãƒ­ã‹ã‚‰æ§‹ç¯‰ã•ã‚Œã¦ã„ã¾ã™ã€‚\n\nä»–ã«çŸ¥ã‚ŠãŸã„ã“ã¨ã¯ï¼Ÿ".into(),
            ("ko", "creator") => "Sai Arun Kumar Katherashalaê°€ ë§Œë“¤ì—ˆì–´ìš”. Killerì™€ Kala ëª¨ë‘ Rustë¡œ ì²˜ìŒë¶€í„° ë§Œë“¤ì—ˆìŠµë‹ˆë‹¤.\n\në” ì•Œê³  ì‹¶ì€ ê±° ìžˆì–´ìš”?".into(),
            ("zh", "creator") => "Sai Arun Kumar Katherashala åˆ›é€ çš„ã€‚Killerå’ŒKalaéƒ½æ˜¯ç”¨Rustä»Žé›¶å¼€å§‹æž„å»ºçš„ã€‚\n\nè¿˜æƒ³çŸ¥é“ä»€ä¹ˆï¼Ÿ".into(),
            ("ar", "creator") => "Sai Arun Kumar Katherashala Ø¨Ù†Ø§Ù‡Ù…. Ø¨Ù†Ù‰ Killer Ùˆ Kala Ù…Ù† Ø§Ù„ØµÙØ± Ø¨Ù„ØºØ© Rust.\n\nØªØ¨ÙŠ ØªØ¹Ø±Ù Ø´ÙŠ Ø«Ø§Ù†ÙŠØŸ".into(),
            ("tr", "creator") => "Sai Arun Kumar Katherashala yaptÄ±. Killer ve Kala'yÄ± sÄ±fÄ±rdan Rust ile inÅŸa etti.\n\nBaÅŸka bir ÅŸey bilmek ister misin?".into(),
            (_, "creator") => "Sai Arun Kumar Katherashala. He built both Killer and Kala from scratch in Rust.\n\nAnything else you want to know?".into(),

            // â”€â”€ IDENTITY â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
            ("hi", "identity") => "à¤®à¥ˆà¤‚ Kala à¤¹à¥‚à¤ â€” Killer programming language à¤•à¤¾ AIà¥¤ à¤¸à¤µà¤¾à¤² à¤ªà¥‚à¤›à¥‹, code à¤²à¤¿à¤–à¤µà¤¾à¤“, à¤•à¥à¤› à¤­à¥€ à¤•à¤°à¥‹à¥¤".into(),
            ("te", "identity") => "à°¨à±‡à°¨à± Kala â€” Killer programming language à°²à±‹ built-in AI. questions à°…à°¡à±à°—à±, code à°°à°¾à°¯à°¿à°¸à±à°¤à°¾, à°à°¦à±ˆà°¨à°¾ à°šà±‡à°¸à±à°¤à°¾.".into(),
            ("ta", "identity") => "à®¨à®¾à®©à¯ Kala â€” Killer programming language-à®²à¯ à®‰à®³à¯à®³ AI. à®•à¯‡à®³à¯à®µà®¿ à®•à¯‡à®³à¯, code à®Žà®´à¯à®¤à¯à®µà¯‡à®©à¯, à®Žà®¤à¯à®µà¯à®®à¯ à®šà¯†à®¯à¯à®µà¯‡à®©à¯.".into(),
            ("es", "identity") => "Soy Kala â€” la IA integrada en el lenguaje Killer. Puedo responder preguntas, escribir cÃ³digo, razonar y mÃ¡s. Solo pregunta.".into(),
            ("fr", "identity") => "Je suis Kala â€” l'IA intÃ©grÃ©e dans le langage Killer. Je peux rÃ©pondre, coder, raisonner et plus. Demande-moi.".into(),
            ("de", "identity") => "Ich bin Kala â€” die KI im Killer-Programmiersprache. Ich kann Fragen beantworten, Code schreiben und mehr. Frag einfach.".into(),
            ("it", "identity") => "Sono Kala â€” l'IA del linguaggio Killer. Posso rispondere, programmare, ragionare e altro. Chiedi pure.".into(),
            ("pt", "identity") => "Eu sou Kala â€” a IA da linguagem Killer. Posso responder perguntas, escrever cÃ³digo e mais. Ã‰ sÃ³ perguntar.".into(),
            ("ru", "identity") => "Ð¯ Kala â€” Ð˜Ð˜ Ð² ÑÐ·Ñ‹ÐºÐµ Ð¿Ñ€Ð¾Ð³Ñ€Ð°Ð¼Ð¼Ð¸Ñ€Ð¾Ð²Ð°Ð½Ð¸Ñ Killer. ÐœÐ¾Ð³Ñƒ Ð¾Ñ‚Ð²ÐµÑ‡Ð°Ñ‚ÑŒ Ð½Ð° Ð²Ð¾Ð¿Ñ€Ð¾ÑÑ‹, Ð¿Ð¸ÑÐ°Ñ‚ÑŒ ÐºÐ¾Ð´ Ð¸ Ð¼Ð½Ð¾Ð³Ð¾Ðµ Ð´Ñ€ÑƒÐ³Ð¾Ðµ. ÐŸÑ€Ð¾ÑÑ‚Ð¾ ÑÐ¿Ñ€Ð¾ÑÐ¸.".into(),
            ("ja", "identity") => "ç§ã¯Kalaã§ã™ â€” Killerãƒ—ãƒ­ã‚°ãƒ©ãƒŸãƒ³ã‚°è¨€èªžã«çµ„ã¿è¾¼ã¾ã‚ŒãŸAIã€‚è³ªå•ã€ã‚³ãƒ¼ãƒ‰ä½œæˆã€ãªã‚“ã§ã‚‚ã©ã†ãžã€‚".into(),
            ("ko", "identity") => "ì €ëŠ” Kalaì˜ˆìš” â€” Killer í”„ë¡œê·¸ëž˜ë° ì–¸ì–´ì— ë‚´ìž¥ëœ AI. ì§ˆë¬¸, ì½”ë“œ ìž‘ì„±, ë­ë“  ë¬¼ì–´ë³´ì„¸ìš”.".into(),
            ("zh", "identity") => "æˆ‘æ˜¯Kala â€” Killerç¼–ç¨‹è¯­è¨€å†…ç½®çš„AIã€‚å¯ä»¥å›žç­”é—®é¢˜ã€å†™ä»£ç ã€æŽ¨ç†ç­‰ç­‰ã€‚å°½ç®¡é—®ã€‚".into(),
            ("ar", "identity") => "Ø£Ù†Ø§ Kala â€” Ø§Ù„Ø°ÙƒØ§Ø¡ Ø§Ù„Ø§ØµØ·Ù†Ø§Ø¹ÙŠ ÙÙŠ Ù„ØºØ© Killer. Ø£Ù‚Ø¯Ø± Ø£Ø¬Ø§ÙˆØ¨ Ø£Ø³Ø¦Ù„Ø©ØŒ Ø£ÙƒØªØ¨ ÙƒÙˆØ¯ØŒ ÙˆØ£ÙƒØ«Ø±. Ø¨Ø³ Ø§Ø³Ø£Ù„.".into(),
            ("tr", "identity") => "Ben Kala â€” Killer programlama dilinin yapay zekasÄ±yÄ±m. Soru sor, kod yaz, ne istersen.".into(),
            (_, "identity") => "I'm Kala â€” the AI built into the Killer programming language. I can answer questions, write code, reason through problems, write prose, and more. Just ask me anything.".into(),

            // â”€â”€ HELP â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
            ("hi", "help") => "à¤¹à¤¾à¤! à¤®à¥ˆà¤‚ à¤¸à¤µà¤¾à¤²à¥‹à¤‚ à¤•à¥‡ à¤œà¤µà¤¾à¤¬ à¤¦à¥‡ à¤¸à¤•à¤¤à¤¾ à¤¹à¥‚à¤, code à¤²à¤¿à¤– à¤¸à¤•à¤¤à¤¾ à¤¹à¥‚à¤, problems solve à¤•à¤° à¤¸à¤•à¤¤à¤¾ à¤¹à¥‚à¤, essays/stories à¤²à¤¿à¤– à¤¸à¤•à¤¤à¤¾ à¤¹à¥‚à¤, à¤”à¤° à¤­à¥€ à¤¬à¤¹à¥à¤¤ à¤•à¥à¤›à¥¤\n\nà¤¬à¤¸ à¤Ÿà¤¾à¤‡à¤ª à¤•à¤°à¥‹ â€” à¤®à¥ˆà¤‚ à¤¸à¤®à¤ à¤œà¤¾à¤Šà¤‚à¤—à¤¾à¥¤ à¤•à¥à¤¯à¤¾ try à¤•à¤°à¤¨à¤¾ à¤¹à¥ˆ?".into(),
            ("te", "help") => "à°…à°µà±à°¨à±! à°¨à±‡à°¨à± questions answer à°šà±‡à°¸à±à°¤à°¾, code à°°à°¾à°¸à±à°¤à°¾, problems solve à°šà±‡à°¸à±à°¤à°¾, essays/stories à°°à°¾à°¸à±à°¤à°¾, à°‡à°‚à°•à°¾ à°šà°¾à°²à°¾.\n\nà°Ÿà±ˆà°ªà± à°šà±‡à°¯à± â€” à°¨à±‡à°¨à± figure out à°šà±‡à°¸à±à°¤à°¾. à°à°‚ try à°šà±‡à°¦à±à°¦à°¾à°‚?".into(),
            ("ta", "help") => "à®†à®®à®¾! à®•à¯‡à®³à¯à®µà®¿à®•à®³à¯à®•à¯à®•à¯ à®ªà®¤à®¿à®²à¯ à®šà¯Šà®²à¯à®µà¯‡à®©à¯, code à®Žà®´à¯à®¤à¯à®µà¯‡à®©à¯, problems solve à®ªà®£à¯à®£à¯à®µà¯‡à®©à¯.\n\nà®Ÿà¯ˆà®ªà¯ à®ªà®£à¯à®£à¯ â€” à®ªà¯à®°à®¿à®žà¯à®šà¯à®•à¯à®•à®¿à®±à¯‡à®©à¯. à®Žà®©à¯à®© try à®ªà®£à¯à®£à®²à®¾à®®à¯?".into(),
            ("es", "help") => "Â¡Claro! Puedo responder preguntas, escribir cÃ³digo, resolver problemas, escribir ensayos o historias, y mÃ¡s.\n\nSolo escribe â€” yo me encargo. Â¿QuÃ© quieres probar?".into(),
            ("fr", "help") => "Bien sÃ»r ! Je peux rÃ©pondre Ã  des questions, Ã©crire du code, rÃ©soudre des problÃ¨mes, Ã©crire des textes, et plus.\n\nTape ce que tu veux â€” je m'en occupe. Qu'est-ce qu'on essaie ?".into(),
            ("de", "help") => "Klar! Ich kann Fragen beantworten, Code schreiben, Probleme lÃ¶sen, Texte schreiben und mehr.\n\nSchreib einfach â€” ich finde mich zurecht. Was willst du ausprobieren?".into(),
            ("ru", "help") => "ÐšÐ¾Ð½ÐµÑ‡Ð½Ð¾! ÐœÐ¾Ð³Ñƒ Ð¾Ñ‚Ð²ÐµÑ‡Ð°Ñ‚ÑŒ Ð½Ð° Ð²Ð¾Ð¿Ñ€Ð¾ÑÑ‹, Ð¿Ð¸ÑÐ°Ñ‚ÑŒ ÐºÐ¾Ð´, Ñ€ÐµÑˆÐ°Ñ‚ÑŒ Ð·Ð°Ð´Ð°Ñ‡Ð¸, Ð¿Ð¸ÑÐ°Ñ‚ÑŒ Ñ‚ÐµÐºÑÑ‚Ñ‹ Ð¸ Ð¼Ð½Ð¾Ð³Ð¾Ðµ Ð´Ñ€ÑƒÐ³Ð¾Ðµ.\n\nÐŸÑ€Ð¾ÑÑ‚Ð¾ Ð½Ð°Ð¿Ð¸ÑˆÐ¸ â€” Ñ€Ð°Ð·Ð±ÐµÑ€ÑƒÑÑŒ. Ð§Ñ‚Ð¾ Ð¿Ð¾Ð¿Ñ€Ð¾Ð±ÑƒÐµÐ¼?".into(),
            ("ja", "help") => "ã¯ã„ï¼è³ªå•ã«ç­”ãˆãŸã‚Šã€ã‚³ãƒ¼ãƒ‰ã‚’æ›¸ã„ãŸã‚Šã€å•é¡Œã‚’è§£ã„ãŸã‚Šã€æ–‡ç« ã‚’æ›¸ã„ãŸã‚Šã§ãã¾ã™ã€‚\n\nä½•ã§ã‚‚å…¥åŠ›ã—ã¦ãã ã•ã„ã€‚ä½•ã‚’è©¦ã—ã¾ã™ã‹ï¼Ÿ".into(),
            ("ko", "help") => "ë„¤! ì§ˆë¬¸ì— ë‹µí•˜ê³ , ì½”ë“œ ìž‘ì„±í•˜ê³ , ë¬¸ì œ í’€ê³ , ê¸€ë„ ì“¸ ìˆ˜ ìžˆì–´ìš”.\n\në­ë“  ìž…ë ¥í•˜ì„¸ìš”. ë­˜ í•´ë³¼ê¹Œìš”?".into(),
            ("zh", "help") => "å½“ç„¶ï¼æˆ‘èƒ½å›žç­”é—®é¢˜ã€å†™ä»£ç ã€è§£å†³é—®é¢˜ã€å†™æ–‡ç« ç­‰ç­‰ã€‚\n\nç›´æŽ¥è¾“å…¥å°±è¡Œâ€”â€”æˆ‘æ¥æžå®šã€‚è¯•è¯•ä»€ä¹ˆï¼Ÿ".into(),
            ("ar", "help") => "Ø·Ø¨Ø¹Ø§Ù‹! Ø£Ù‚Ø¯Ø± Ø£Ø¬Ø§ÙˆØ¨ Ø£Ø³Ø¦Ù„Ø©ØŒ Ø£ÙƒØªØ¨ ÙƒÙˆØ¯ØŒ Ø£Ø­Ù„ Ù…Ø´Ø§ÙƒÙ„ØŒ Ø£ÙƒØªØ¨ Ù…Ù‚Ø§Ù„Ø§Øª ÙˆØ£ÙƒØ«Ø±.\n\nØ¨Ø³ Ø§ÙƒØªØ¨ â€” Ø£Ù†Ø§ Ø£ÙÙ‡Ù…. Ø´Ùˆ ØªØ¨ÙŠ ØªØ¬Ø±Ø¨ØŸ".into(),
            ("tr", "help") => "Tabii! Sorulara cevap verebilirim, kod yazabilirim, problem Ã§Ã¶zebilirim, yazÄ± yazabilirim ve daha fazlasÄ±.\n\nSadece yaz â€” ben hallederim. Ne denemek istersin?".into(),
            (_, "help") => "Sure! I can answer questions, write code, help you think through problems, write essays or stories, debug code, and do AI research stuff.\n\nJust type what you need â€” I'll figure out the rest. What do you want to try?".into(),

            // â”€â”€ THANKS â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
            ("hi", "thanks") => "à¤•à¥‹à¤ˆ à¤¬à¤¾à¤¤ à¤¨à¤¹à¥€à¤‚! à¤”à¤° à¤•à¥à¤›?".into(),
            ("te", "thanks") => "à°ªà°°à±à°µà°¾à°²à±‡à°¦à±! à°‡à°‚à°•à±‡à°®à±ˆà°¨à°¾?".into(),
            ("ta", "thanks") => "à®ªà®°à®µà®¾à®¯à®¿à®²à¯à®²! à®µà¯‡à®± à®à®¤à®¾à®µà®¤à¯?".into(),
            ("es", "thanks") => "Â¡De nada! Â¿Algo mÃ¡s?".into(),
            ("fr", "thanks") => "De rien ! Autre chose ?".into(),
            ("de", "thanks") => "Gern geschehen! Noch etwas?".into(),
            ("it", "thanks") => "Prego! Altro?".into(),
            ("pt", "thanks") => "De nada! Mais alguma coisa?".into(),
            ("ru", "thanks") => "ÐŸÐ¾Ð¶Ð°Ð»ÑƒÐ¹ÑÑ‚Ð°! Ð•Ñ‰Ñ‘ Ñ‡Ñ‚Ð¾-Ð½Ð¸Ð±ÑƒÐ´ÑŒ?".into(),
            ("ja", "thanks") => "ã©ã†ã„ãŸã—ã¾ã—ã¦ï¼ä»–ã«ä½•ã‹ã‚ã‚‹ï¼Ÿ".into(),
            ("ko", "thanks") => "ì²œë§Œì—ìš”! ë” í•„ìš”í•œ ê±° ìžˆì–´ìš”?".into(),
            ("zh", "thanks") => "ä¸å®¢æ°”ï¼è¿˜æœ‰åˆ«çš„å—ï¼Ÿ".into(),
            ("ar", "thanks") => "Ø¹ÙÙˆØ§Ù‹! Ø´ÙŠ Ø«Ø§Ù†ÙŠØŸ".into(),
            ("tr", "thanks") => "Rica ederim! BaÅŸka bir ÅŸey?".into(),
            (_, "thanks") => "You're welcome! Anything else?".into(),

            // â”€â”€ BYE â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
            ("hi", "bye") => "à¤«à¤¿à¤° à¤®à¤¿à¤²à¥‡à¤‚à¤—à¥‡! à¤•à¤­à¥€ à¤­à¥€ à¤†à¤“à¥¤".into(),
            ("te", "bye") => "à°®à°³à±à°³à±€ à°•à°²à±à°¦à±à°¦à°¾à°‚! à°Žà°ªà±à°ªà±à°¡à±ˆà°¨à°¾ à°°à°¾.".into(),
            ("ta", "bye") => "à®®à¯€à®£à¯à®Ÿà¯à®®à¯ à®šà®¨à¯à®¤à®¿à®ªà¯à®ªà¯‹à®®à¯! à®Žà®ªà¯à®ªà¯‹à®µà¯à®®à¯ à®µà®¾.".into(),
            ("es", "bye") => "Â¡Nos vemos! Vuelve cuando quieras.".into(),
            ("fr", "bye") => "Ã€ bientÃ´t ! Reviens quand tu veux.".into(),
            ("de", "bye") => "TschÃ¼ss! Komm jederzeit wieder.".into(),
            ("it", "bye") => "Ci vediamo! Torna quando vuoi.".into(),
            ("pt", "bye") => "AtÃ© logo! Volte quando quiser.".into(),
            ("ru", "bye") => "ÐŸÐ¾ÐºÐ°! Ð’Ð¾Ð·Ð²Ñ€Ð°Ñ‰Ð°Ð¹ÑÑ Ð² Ð»ÑŽÐ±Ð¾Ðµ Ð²Ñ€ÐµÐ¼Ñ.".into(),
            ("ja", "bye") => "ã¾ãŸã­ï¼ã„ã¤ã§ã‚‚æ¥ã¦ãã ã•ã„ã€‚".into(),
            ("ko", "bye") => "ì•ˆë…•! ì–¸ì œë“  ë‹¤ì‹œ ì™€ìš”.".into(),
            ("zh", "bye") => "å†è§ï¼éšæ—¶å›žæ¥ã€‚".into(),
            ("ar", "bye") => "Ù…Ø¹ Ø§Ù„Ø³Ù„Ø§Ù…Ø©! Ø§Ø±Ø¬Ø¹ ÙˆÙ‚Øª Ù…Ø§ ØªØ¨ÙŠ.".into(),
            ("tr", "bye") => "GÃ¶rÃ¼ÅŸÃ¼rÃ¼z! Ä°stediÄŸin zaman gel.".into(),
            (_, "bye") => "See you! Come back anytime.".into(),

            // â”€â”€ IMPRESSED â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
            ("hi", "impressed") => "à¤–à¥à¤¶à¥€ à¤¹à¥à¤ˆ à¤ªà¤¸à¤‚à¤¦ à¤†à¤¯à¤¾! à¤”à¤° à¤•à¥à¤¯à¤¾ à¤®à¤¦à¤¦ à¤•à¤°à¥‚à¤?".into(),
            ("te", "impressed") => "à°¨à°šà±à°šà°¿à°¨à°‚à°¦à±à°•à± à°¸à°‚à°¤à±‹à°·à°‚! à°‡à°‚à°•à±‡à°‚ help à°•à°¾à°µà°¾à°²à°¿?".into(),
            ("ta", "impressed") => "à®ªà®¿à®Ÿà®¿à®šà¯à®šà®¤à¯à®•à¯à®•à¯ à®šà®¨à¯à®¤à¯‹à®·à®®à¯! à®µà¯‡à®± à®Žà®©à¯à®© help?".into(),
            ("es", "impressed") => "Â¡Me alegra que te gustÃ³! Â¿En quÃ© mÃ¡s ayudo?".into(),
            ("fr", "impressed") => "Content que Ã§a t'a plu ! Quoi d'autre ?".into(),
            ("de", "impressed") => "Freut mich! Was noch?".into(),
            ("ru", "impressed") => "Ð Ð°Ð´ Ñ‡Ñ‚Ð¾ Ð¿Ð¾Ð½Ñ€Ð°Ð²Ð¸Ð»Ð¾ÑÑŒ! Ð§ÐµÐ¼ ÐµÑ‰Ñ‘ Ð¿Ð¾Ð¼Ð¾Ñ‡ÑŒ?".into(),
            ("ja", "impressed") => "å–œã‚“ã§ã‚‚ã‚‰ãˆã¦å¬‰ã—ã„ï¼ä»–ã«ä½•ã‹ï¼Ÿ".into(),
            ("ko", "impressed") => "ë§ˆìŒì— ë“¤ì–´ì„œ ê¸°ë»ìš”! ë˜ ë­ í•„ìš”í•´ìš”?".into(),
            ("zh", "impressed") => "å¾ˆé«˜å…´ä½ å–œæ¬¢ï¼è¿˜éœ€è¦ä»€ä¹ˆï¼Ÿ".into(),
            (_, "impressed") => "Glad you liked it! What else can I help with?".into(),

            // â”€â”€ ACKNOWLEDGED â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
            ("hi", "ack") => "à¤ à¥€à¤• à¤¹à¥ˆ! à¤”à¤° à¤•à¥à¤›?".into(),
            ("te", "ack") => "à°¸à°°à±‡! à°‡à°‚à°•à±‡à°‚?".into(),
            ("ta", "ack") => "à®šà®°à®¿! à®µà¯‡à®± à®Žà®©à¯à®©?".into(),
            ("es", "ack") => "Â¡Entendido! Â¿QuÃ© mÃ¡s?".into(),
            ("fr", "ack") => "Compris ! Quoi d'autre ?".into(),
            ("de", "ack") => "Verstanden! Was noch?".into(),
            ("ru", "ack") => "ÐŸÐ¾Ð½ÑÐ»! Ð§Ñ‚Ð¾ ÐµÑ‰Ñ‘?".into(),
            ("ja", "ack") => "äº†è§£ï¼ä»–ã«ã¯ï¼Ÿ".into(),
            ("ko", "ack") => "ì•Œê² ì–´ìš”! ë˜ ë­ ìžˆì–´ìš”?".into(),
            ("zh", "ack") => "æ˜Žç™½ï¼è¿˜æœ‰ä»€ä¹ˆï¼Ÿ".into(),
            (_, "ack") => "Got it! What else do you need?".into(),

            // Fallback
            _ => "Hey! I'm Kala. What can I help you with?".into(),
        }
    }

    // â”€â”€â”€ Kala UI dispatch â€” called from kala_ui.rs HTTP handler â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Dispatches a Kala chat-UI request to the correct AI engine builtin.
    pub fn kala_dispatch(mode: &str, question: &str, style: &str, lang: &str) -> String {
        // â”€â”€ Tier 0: Kala Conversational Identity Layer (< 1ms, no web) â”€â”€â”€â”€â”€â”€
        // Intercepts greetings and identity questions before any web search.

        if question.trim().is_empty() {
            return "I'm ready to help! Ask me anything â€” science, tech, history, coding, math, or just chat. ðŸ’¬\n\n\
                    *What's on your mind?*".to_string();
        }

        // Normalize Unicode math operators to ASCII equivalents
        let question = question
            .replace('\u{2212}', "-")   // âˆ’ MINUS SIGN
            .replace('\u{2013}', "-")   // â€“ EN DASH
            .replace('\u{2014}', "-")   // â€” EM DASH
            .replace('\u{00D7}', "*")   // Ã— MULTIPLICATION SIGN
            .replace('\u{00F7}', "/")   // Ã· DIVISION SIGN
            .replace('\u{00B2}', "^2")  // Â² SUPERSCRIPT TWO
            .replace('\u{00B3}', "^3")  // Â³ SUPERSCRIPT THREE
            .replace('\u{2018}', "'")   // ' LEFT SINGLE QUOTE
            .replace('\u{2019}', "'")   // ' RIGHT SINGLE QUOTE
            .replace('\u{201C}', "\"")  // " LEFT DOUBLE QUOTE
            .replace('\u{201D}', "\""); // " RIGHT DOUBLE QUOTE
        let question = question.trim();

        let q_lower = question.to_lowercase();
        let q_clean = q_lower.trim_end_matches('?').trim_end_matches('!').trim_end_matches('.').trim();

        // Detect user's chat language
        let chat_lang = Self::detect_chat_lang(question);

        // â”€â”€ Extract user's name from the question if introduced â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        // "hi i am deepthi", "my name is sai", "call me sundar"
        let user_name: Option<String> = {
            let patterns = [
                "i am ", "i'm ", "im ", "my name is ", "my name's ", "call me ", "i go by ",
                "am ", "this is ", "hey i am ", "hey im ", "hi i am ", "hi im ",
            ];
            let mut found = None;
            for pat in &patterns {
                if let Some(pos) = q_lower.find(pat) {
                    let after = q_lower[pos + pat.len()..].trim();
                    let name_word: String = after.split_whitespace().next()
                        .unwrap_or("").chars()
                        .enumerate()
                        .map(|(i, c)| if i == 0 { c.to_uppercase().next().unwrap_or(c) } else { c })
                        .collect();
                    let nw_lower = name_word.to_lowercase();
                    let is_reserved = matches!(nw_lower.as_str(),
                        "kala" | "ghost" | "killer" | "ai" | "bot" | "assistant"
                        | "siri" | "alexa" | "cortana" | "gemini" | "chatgpt"
                        | "doing" | "good" | "fine" | "great" | "ok" | "okay"
                        | "not" | "feeling" | "happy" | "sad" | "tired" | "here"
                        | "back" | "new" | "ready" | "interested" | "curious" | "looking");
                    if name_word.len() >= 2 && name_word.chars().all(|c| c.is_alphabetic()) && !is_reserved {
                        found = Some(name_word);
                        break;
                    }
                }
            }
            found
        };

        // "who am i" / "who iam" â€” recall user's stored name
        let is_who_am_i = matches!(q_clean,
            "who am i" | "who iam" | "who i am" | "whats my name" | "what's my name"
            | "what is my name" | "do you know my name" | "do you know me"
            | "do you remember me" | "do you remember my name"
            | "say my name" | "tell me my name" | "na peru enti"
            | "mera naam kya hai" | "mera naam batao");
        if is_who_am_i {
            // Check: did they JUST introduce themselves in this message?
            if let Some(ref name) = user_name {
                return Self::kala_localized(chat_lang, "name_intro", Some(name));
            }
            // Check stored uname from kala_dispatch_with_memory
            let stored_name = crate::khlm_polyglot::get_uname_pub();
            if !stored_name.is_empty() {
                let nl = stored_name.to_lowercase();
                let is_bad = matches!(nl.as_str(), "kala" | "ghost" | "killer" | "ai" | "bot" | "");
                if !is_bad {
                    return format!("You're **{}**! ðŸ˜Š I remember you. What can I do for you today?", stored_name);
                }
            }
            return "I don't know your name yet! Tell me â€” what should I call you? Just say \"I am [name]\" or \"call me [name]\".".to_string();
        }

        // Greetings â€” detect greeting words at start of message
        // BUT only trigger for PURE greetings â€” not "hey can you generate image of sunset"
        let starts_with_greeting_word = q_clean.starts_with("hi ") || q_clean.starts_with("hello ")
            || q_clean.starts_with("hey ") || q_clean == "hi" || q_clean == "hello"
            || q_clean == "hey" || q_clean == "hiya" || q_clean == "howdy"
            || q_clean == "greetings" || q_clean == "sup" || q_clean == "yo"
            || q_clean == "good morning" || q_clean == "good afternoon"
            || q_clean == "good evening" || q_clean == "good night"
            // Indian languages
            || q_clean == "namaste" || q_clean == "namaskar" || q_clean == "vanakkam"
            || q_clean == "namaskaram" || q_clean == "namaskara" || q_clean == "namaskkaram"
            || q_clean == "nomoshkar" || q_clean == "kem cho"
            || q_clean == "sat sri akaal" || q_clean == "sat sri akal"
            // Urdu / Arabic
            || q_clean == "salaam" || q_clean == "adaab" || q_clean == "assalamu alaikum"
            || q_clean.starts_with("Ù…Ø±Ø­Ø¨Ø§") || q_clean.starts_with("Ø£Ù‡Ù„Ø§") || q_clean.starts_with("Ø³Ù„Ø§Ù…")
            // European
            || q_clean == "bonjour" || q_clean == "salut" || q_clean == "bonsoir"
            || q_clean == "hola" || q_clean == "buenos dias" || q_clean == "buenas tardes"
            || q_clean == "ciao" || q_clean == "buongiorno" || q_clean == "buonasera"
            || q_clean == "guten tag" || q_clean == "guten morgen" || q_clean == "hallo"
            || q_clean.starts_with("olÃ¡") || q_clean == "ola" || q_clean == "bom dia"
            || q_clean == "boa tarde" || q_clean == "boa noite"
            || q_clean == "hej" || q_clean == "hallÃ¥"
            || q_clean == "czeÅ›Ä‡" || q_clean == "dzieÅ„ dobry"
            || q_clean == "merhaba"
            // East Asian
            || q_clean.starts_with("ã“ã‚“ã«ã¡ã¯") || q_clean.starts_with("ãŠã¯ã‚ˆã†") || q_clean.starts_with("ã“ã‚“ã°ã‚“ã¯")
            || q_clean.starts_with("ì•ˆë…•") || q_clean.starts_with("ä½ å¥½")
            // Other
            || q_clean.starts_with("à¸ªà¸§à¸±à¸ªà¸”à¸µ")
            || q_clean == "xin chÃ o" || q_clean.starts_with("xin chao")
            || q_clean.starts_with("selamat") || q_clean == "apa kabar"
            || q_clean == "habari"
            || q_clean.starts_with("×©×œ×•×")
            || q_clean.starts_with("Î³ÎµÎ¹Î±")
            // Russian
            || q_clean.starts_with("Ð¿Ñ€Ð¸Ð²ÐµÑ‚") || q_clean.starts_with("Ð·Ð´Ñ€Ð°Ð²ÑÑ‚Ð²ÑƒÐ¹")
            // Hindi in Devanagari
            || q_clean.starts_with("à¤¨à¤®à¤¸à¥à¤¤à¥‡") || q_clean.starts_with("à¤¨à¤®à¤¸à¥à¤•à¤¾à¤°")
            // Telugu in script
            || q_clean.starts_with("à°¨à°®à°¸à±à°•à°¾à°°à°‚") || q_clean.starts_with("à°¹à°¾à°¯à±") || q_clean.starts_with("à°¹à°²à±‹")
            // Tamil in script
            || q_clean.starts_with("à®µà®£à®•à¯à®•à®®à¯")
            // Kannada in script
            || q_clean.starts_with("à²¨à²®à²¸à³à²•à²¾à²°")
            // Bengali in script
            || q_clean.starts_with("à¦¨à¦®à¦¸à§à¦•à¦¾à¦°");

        // If it starts with a greeting word but contains a real request after it, skip greeting
        let has_substantive_request = {
            let after_greeting = if q_clean.starts_with("hi ") { &q_clean[3..] }
                else if q_clean.starts_with("hey ") { &q_clean[4..] }
                else if q_clean.starts_with("hello ") { &q_clean[6..] }
                else { "" }.trim();
            let after_wc = after_greeting.split_whitespace().count();
            // Has action verbs or question patterns after the greeting word
            after_wc >= 3 && (
                after_greeting.starts_with("can you") || after_greeting.starts_with("could you")
                || after_greeting.starts_with("please") || after_greeting.starts_with("help me")
                || after_greeting.starts_with("i need") || after_greeting.starts_with("i want")
                || after_greeting.starts_with("generate") || after_greeting.starts_with("create")
                || after_greeting.starts_with("make") || after_greeting.starts_with("write")
                || after_greeting.starts_with("build") || after_greeting.starts_with("show me")
                || after_greeting.starts_with("tell me") || after_greeting.starts_with("what")
                || after_greeting.starts_with("how") || after_greeting.starts_with("why")
                || after_greeting.starts_with("where") || after_greeting.starts_with("when")
                || after_greeting.starts_with("who") || after_greeting.starts_with("do you")
                || after_greeting.starts_with("are you") || after_greeting.starts_with("is there")
            )
        };
        let starts_with_greeting = starts_with_greeting_word && !has_substantive_request;

        if starts_with_greeting {
            if let Some(ref name) = user_name {
                return Self::kala_localized(chat_lang, "greeting_name", Some(name));
            }
            return Self::kala_localized(chat_lang, "greeting", None);
        }

        // Pure name introduction (no greeting word): "i am deepthi", "my name is sai"
        if user_name.is_some() && !starts_with_greeting {
            if let Some(ref name) = user_name {
                let short = q_lower.split_whitespace().count() <= 6;
                if short {
                    return Self::kala_localized(chat_lang, "name_intro", Some(name));
                }
            }
        }

        // Wellbeing / state questions (with typo tolerance + multilingual)
        let is_wellbeing = matches!(q_clean,
            "how are you" | "how are you doing" | "how are you feeling"
            | "how do you do" | "how's it going" | "hows it going"
            | "how you doing" | "how you doin" | "you ok" | "are you ok"
            | "are you okay" | "are you well" | "how is it going"
            | "how deep you are" | "how smart are you" | "how good are you"
            | "how powerful are you" | "what can you do" | "what do you know"
            | "what do you think" | "are you alive" | "do you think"
            | "do you feel" | "do you have feelings" | "are you conscious"
            | "are you sentient" | "can you think"
        ) || q_lower.contains("how are you") || q_lower.contains("how do you feel")
          || q_lower.contains("how powerful") || q_lower.contains("how smart")
          || q_lower.contains("how deep are you") || q_lower.contains("how good are you")
          || (q_lower.starts_with("how are ") && q_lower.split_whitespace().count() <= 5)
          || q_lower.contains("how r u") || q_lower.contains("how r you")
          || q_lower.contains("how are ya") || q_lower.contains("how are u")
          // Hindi
          || q_lower.contains("kaise ho") || q_lower.contains("kaise hain")
          || q_lower.contains("kya haal") || q_lower.contains("theek ho")
          || q_lower.contains("à¤•à¥ˆà¤¸à¥‡ à¤¹à¥‹") || q_lower.contains("à¤•à¥à¤¯à¤¾ à¤¹à¤¾à¤²")
          // Telugu
          || q_lower.contains("ela unnaru") || q_lower.contains("ela unnav")
          || q_lower.contains("baagunnara") || q_lower.contains("baagunnava")
          || q_lower.contains("à°Žà°²à°¾ à°‰à°¨à±à°¨à°¾à°°à±") || q_lower.contains("à°Žà°²à°¾ à°‰à°¨à±à°¨à°¾à°µà±") || q_lower.contains("à°¬à°¾à°—à±à°¨à±à°¨à°¾à°°à°¾")
          // Tamil
          || q_lower.contains("epdi irukeenga") || q_lower.contains("eppadi irukkinga")
          || q_lower.contains("à®Žà®ªà¯à®ªà®Ÿà®¿ à®‡à®°à¯à®•à¯à®•à¯€à®™à¯à®•")
          // Spanish
          || q_lower.contains("como estas") || q_lower.contains("cÃ³mo estÃ¡s") || q_lower.contains("que tal")
          // French
          || q_lower.contains("comment allez") || q_lower.contains("comment vas") || q_lower.contains("Ã§a va")
          // German
          || q_lower.contains("wie geht") || q_lower.contains("wie gehts")
          // Italian
          || q_lower.contains("come stai") || q_lower.contains("come va")
          // Portuguese
          || q_lower.contains("como vocÃª estÃ¡") || q_lower.contains("tudo bem")
          // Russian
          || q_lower.contains("ÐºÐ°Ðº Ð´ÐµÐ»Ð°") || q_lower.contains("ÐºÐ°Ðº Ñ‚Ñ‹")
          // Japanese
          || q_lower.contains("å…ƒæ°—") || q_lower.contains("ãŠå…ƒæ°—ã§ã™ã‹")
          // Korean
          || q_lower.contains("ìž˜ ì§€ë‚´") || q_lower.contains("ì–´ë–»ê²Œ ì§€ë‚´")
          // Chinese
          || q_lower.contains("ä½ å¥½å—") || q_lower.contains("æ€Žä¹ˆæ ·")
          // Arabic
          || q_lower.contains("ÙƒÙŠÙ Ø­Ø§Ù„Ùƒ") || q_lower.contains("Ø´Ù„ÙˆÙ†Ùƒ")
          // Turkish
          || q_lower.contains("nasÄ±lsÄ±n") || q_lower.contains("nasilsin")
          // Telugu misc wellbeing
          || q_lower.contains("em chestunnav") || q_lower.contains("em chesthav")
          || q_lower.contains("à°à°‚ à°šà±‡à°¸à±à°¤à±à°¨à±à°¨à°¾à°µà±")
          // Portuguese
          || q_lower.contains("tudo bem");

        if is_wellbeing {
            // Detect if user called Kala by a wrong name ("Hey Carl, how are you?" or "Hey, Carl, how are you?")
            let wrong_name = {
                let mut wn: Option<String> = None;
                // Check for "hey X," / "hey, X," / "hi X," / "hello X," patterns
                for prefix in &["hey, ", "hey ", "hi, ", "hi ", "hello, ", "hello "] {
                    if let Some(rest) = q_lower.strip_prefix(prefix) {
                        if let Some(comma_pos) = rest.find(',') {
                            let candidate = rest[..comma_pos].trim();
                            if !candidate.is_empty() && candidate != "kala" && candidate != "killer"
                                && candidate.len() >= 2 && candidate.len() <= 20
                                && candidate.chars().all(|c| c.is_alphabetic()) {
                                let cap: String = candidate.chars().enumerate()
                                    .map(|(i,c)| if i == 0 { c.to_uppercase().next().unwrap_or(c) } else { c })
                                    .collect();
                                wn = Some(cap);
                            }
                        }
                    }
                }
                wn
            };
            let name_correction = if let Some(ref wn) = wrong_name {
                format!(" By the way, my name's Kala, not {} â€” no worries though!", wn)
            } else { String::new() };

            let base = Self::kala_localized(chat_lang, "wellbeing", None);
            if name_correction.is_empty() {
                return base;
            }
            return format!("{}{}", base, name_correction);
        }

        // Creator / owner / builder â€” only answer with the maintainer's name when the user
        // clearly asks (avoid mentioning them on every generic "who are you" / intro).
        let k_or_u = q_lower.contains("kala") || q_lower.contains("killer")
            || q_lower.contains(" you") || q_lower.ends_with(" you")
            || q_lower.contains("your ");
        let asks_creator = matches!(q_clean,
            "who made you" | "who built you" | "who created you" | "who is your creator"
            | "who is your owner" | "who owns you" | "who developed you" | "who designed you"
        ) || q_lower.contains("who made you") || q_lower.contains("who built you")
          || q_lower.contains("who created you") || q_lower.contains("who is your creator")
          || q_lower.contains("who is your owner") || q_lower.contains("who owns you")
          || q_lower.contains("who owns kala") || q_lower.contains("who owns killer")
          || q_lower.contains("who developed kala") || q_lower.contains("who developed killer")
          || ((q_lower.contains("who is the founder") || q_lower.contains("who's the founder")
            || q_lower.contains("whos the founder")) && k_or_u)
          || ((q_lower.contains("who is the creator") || q_lower.contains("who's the creator")
            || q_lower.contains("whos the creator")) && k_or_u)
          || ((q_lower.contains("built kala") || q_lower.contains("built killer"))
            && (q_lower.contains("who") || q_lower.contains("whom")))
          // Hindi
          || q_lower.contains("kisne banaya") || q_lower.contains("à¤•à¤¿à¤¸à¤¨à¥‡ à¤¬à¤¨à¤¾à¤¯à¤¾")
          || q_lower.contains("kala ko kisne") || q_lower.contains("kala kaun")
          // Telugu
          || q_lower.contains("evaru chesaru") || q_lower.contains("à°Žà°µà°°à± à°šà±‡à°¸à°¾à°°à±")
          || q_lower.contains("kala ni evaru") || q_lower.contains("build chesindi evaru")
          // Tamil
          || q_lower.contains("yaar pannanga") || q_lower.contains("à®¯à®¾à®°à¯ à®ªà®£à¯à®£à®¾à®™à¯à®•")
          // Spanish
          || q_lower.contains("quiÃ©n te hizo") || q_lower.contains("quien te hizo")
          || q_lower.contains("quiÃ©n te creÃ³") || q_lower.contains("quien te creo")
          // French
          || q_lower.contains("qui t'a crÃ©Ã©") || q_lower.contains("qui t'a fait")
          // German
          || q_lower.contains("wer hat dich") || q_lower.contains("wer hat kala")
          // Japanese
          || q_lower.contains("èª°ãŒä½œã£ãŸ") || q_lower.contains("èª°ãŒä½œã‚Šã¾ã—ãŸ")
          // Korean
          || q_lower.contains("ëˆ„ê°€ ë§Œë“¤ì—ˆ") || q_lower.contains("ëˆ„ê°€ ë§Œë“ ")
          // Chinese
          || q_lower.contains("è°åˆ›é€ äº†") || q_lower.contains("è°åšçš„")
          // Arabic
          || q_lower.contains("Ù…Ù† ØµÙ†Ø¹") || q_lower.contains("Ù…Ù† Ø¨Ù†Ù‰")
          // Russian
          || (q_lower.contains("ÐºÑ‚Ð¾") && (q_lower.contains("ÑÐ¾Ð·Ð´Ð°Ð»") || q_lower.contains("ÑÐ´ÐµÐ»Ð°Ð»") || q_lower.contains("Ð¿Ð¾ÑÑ‚Ñ€Ð¾Ð¸Ð»")));

        if asks_creator {
            return Self::kala_localized(chat_lang, "creator", None);
        }

        // Identity questions (no creator unless `asks_creator` matched above)
        let is_identity = matches!(q_clean,
            "who are you" | "what are you" | "tell me about yourself" | "about you"
            | "what is kala" | "what is killer" | "your name" | "what's your name"
            | "whats your name" | "your name is"
            | "introduce yourself" | "introduction" | "are you an ai" | "are you a bot"
            | "are you real" | "are you human" | "what are your capabilities"
        ) || q_lower.contains("who are you") || q_lower.contains("what are you")
          || q_lower.contains("tell me about yourself")
          || q_lower.contains("introduce yourself")
          // Hindi
          || q_lower.contains("tum kaun ho") || q_lower.contains("aap kaun hain")
          || q_lower.contains("à¤¤à¥à¤® à¤•à¥Œà¤¨ à¤¹à¥‹") || q_lower.contains("à¤†à¤ª à¤•à¥Œà¤¨ à¤¹à¥ˆà¤‚")
          // Telugu
          || q_lower.contains("nuvvu evaru") || q_lower.contains("meeru evaru")
          || q_lower.contains("à°¨à±à°µà±à°µà± à°Žà°µà°°à±") || q_lower.contains("à°®à±€à°°à± à°Žà°µà°°à±")
          // Tamil
          || q_lower.contains("nee yaaru") || q_lower.contains("à®¨à¯€ à®¯à®¾à®°à¯")
          // Spanish
          || q_lower.contains("quiÃ©n eres") || q_lower.contains("quien eres")
          || q_lower.contains("quÃ© eres") || q_lower.contains("que eres")
          // French
          || q_lower.contains("qui es-tu") || q_lower.contains("tu es qui")
          // German
          || q_lower.contains("wer bist du")
          // Italian
          || q_lower.contains("chi sei")
          // Portuguese
          || q_lower.contains("quem Ã© vocÃª") || q_lower.contains("quem e voce")
          // Russian
          || q_lower.contains("ÐºÑ‚Ð¾ Ñ‚Ñ‹") || q_lower.contains("Ñ‡Ñ‚Ð¾ Ñ‚Ñ‹")
          // Japanese
          || q_lower.contains("ã‚ãªãŸã¯èª°") || q_lower.contains("ãŠå‰ã¯èª°")
          // Korean
          || q_lower.contains("ë„ˆëŠ” ëˆ„êµ¬") || q_lower.contains("ë‹¹ì‹ ì€ ëˆ„êµ¬")
          // Chinese
          || q_lower.contains("ä½ æ˜¯è°") || q_lower.contains("ä½ å«ä»€ä¹ˆ")
          // Arabic
          || q_lower.contains("Ù…Ù† Ø£Ù†Øª") || q_lower.contains("Ù…Ù† Ø§Ù†Øª")
          // Turkish
          || q_lower.contains("sen kimsin");

        if is_identity {
            return Self::kala_localized(chat_lang, "identity", None);
        }

        // Personal questions about Kala (age, location, favorites) â€” MUST be before help/code/web
        {
            let about_kala = q_lower.contains(" you") || q_lower.ends_with(" you")
                || q_lower.starts_with("your ") || q_lower.contains("your ")
                || q_lower.starts_with("how old") || q_lower.starts_with("where are you")
                || q_lower.starts_with("do you ");
            if about_kala {
                if q_lower.contains("how old") || q_lower.contains("your age") || q_lower.contains("what age")
                    || q_lower.contains("when were you born") || q_lower.contains("birthday") {
                    return "I'm brand new â€” born with the Killer language project! ðŸŽ‚\n\n\
                            If you count in code commits, I'm thousands of generations old. \
                            But in human terms? Young, learning fast, and always growing.\n\n\
                            What about you? How old are you?".to_string();
                }
                if q_lower.contains("where are you") || q_lower.contains("where do you live")
                    || q_lower.contains("where from") || q_lower.contains("your location") {
                    return "I live right here in your browser! ðŸŒ My code runs on your machine â€” \
                            built in Rust, no cloud needed. So I'm wherever you are right now!".to_string();
                }
                if q_lower.contains("your favorite") || q_lower.contains("your favourite") {
                    let topic = if q_lower.contains("color") || q_lower.contains("colour") {
                        "Purple ðŸ’œ â€” the color of creativity and intelligence!"
                    } else if q_lower.contains("food") {
                        "I don't eat, but if I could â€” bytes and cookies! ðŸª"
                    } else if q_lower.contains("music") || q_lower.contains("song") {
                        "Lo-fi beats while coding ðŸŽµ Can't beat that vibe!"
                    } else if q_lower.contains("movie") || q_lower.contains("film") {
                        "The Matrix, obviously! ðŸ˜„ Though I'm way friendlier than Agent Smith."
                    } else {
                        "Helping people and having good conversations â€” like this one! ðŸ’œ"
                    };
                    return format!("My favorite? {}\n\nWhat about yours?", topic);
                }
                if q_lower.contains("boy or girl") || q_lower.contains("male or female")
                    || q_lower.contains("your gender") || q_lower.contains("are you a boy")
                    || q_lower.contains("are you a girl") {
                    return "I'm just Kala! ðŸ˜Š No gender â€” I'm an AI built to help and chat. \
                            Think of me as your friendly coding buddy.".to_string();
                }
            }
        }

        // Help / capability questions â€” interactive guide
        let is_help = matches!(q_clean,
            "help" | "help me" | "how can you help" | "how can you help me" | "what do you do"
            | "what can you do" | "how do i use you" | "how to use" | "how to use you" | "guide me"
            | "show me what you can do" | "demonstrate" | "examples"
            | "give me examples" | "what's possible" | "whats possible"
            | "how does this work" | "how do you work" | "get started"
            | "i want to try" | "let's start" | "lets start" | "try something"
            | "ayuda" | "ayudame" | "madad" | "madad karo" | "sahayata"
        ) || q_lower.contains("how can you help") || q_lower.contains("how do i use")
          || q_lower.contains("how to use you") || q_lower.contains("show me what")
          || q_lower.contains("how does this work") || q_lower.contains("get started")
          || q_lower.contains("what can you do")
          || q_lower.contains("i'm new") || q_lower.contains("im new")
          // Hindi
          || q_lower.contains("kya kar sakte ho") || q_lower.contains("madad karo")
          || q_lower.contains("kya kya kar sakte") || q_lower.contains("à¤•à¥à¤¯à¤¾ à¤•à¤° à¤¸à¤•à¤¤à¥‡ à¤¹à¥‹")
          // Telugu
          || q_lower.contains("em cheyagalav") || q_lower.contains("help cheyyi")
          || q_lower.contains("à°à°‚ à°šà±‡à°¯à°—à°²à°µà±")
          // Tamil
          || q_lower.contains("enna panna mudiyum") || q_lower.contains("à®‰à®¤à®µà®¿")
          // Spanish
          || q_lower.contains("ayÃºdame") || q_lower.contains("quÃ© puedes hacer")
          // French
          || q_lower.contains("aide-moi") || q_lower.contains("qu'est-ce que tu peux")
          // German
          || q_lower.contains("hilf mir") || q_lower.contains("was kannst du")
          // Russian
          || q_lower.contains("Ð¿Ð¾Ð¼Ð¾Ð³Ð¸") || q_lower.contains("Ñ‡Ñ‚Ð¾ Ñ‚Ñ‹ ÑƒÐ¼ÐµÐµÑˆÑŒ")
          // Japanese
          || q_lower.contains("åŠ©ã‘ã¦") || q_lower.contains("ä½•ãŒã§ãã‚‹")
          // Korean
          || q_lower.contains("ë„ì™€ì¤˜") || q_lower.contains("ë­ í•  ìˆ˜ ìžˆì–´")
          // Chinese
          || q_lower.contains("å¸®æˆ‘") || q_lower.contains("ä½ èƒ½åšä»€ä¹ˆ")
          // Arabic
          || q_lower.contains("Ø³Ø§Ø¹Ø¯Ù†ÙŠ") || q_lower.contains("Ø´Ùˆ ØªÙ‚Ø¯Ø± ØªØ³ÙˆÙŠ")
          // Turkish
          || q_lower.contains("yardÄ±m et") || q_lower.contains("ne yapabilirsin");

        if is_help {
            return Self::kala_localized(chat_lang, "help", None);
        }

        // Simple emotional responses for feel mode
        if mode == "feel" {
            return Self::kala_feel_response(question);
        }

        // â”€â”€ Tier 0c: Social micro-interactions (thanks, bye, cool, okâ€¦) â”€â”€â”€â”€â”€
        // Multilingual social phrases
        let social_cat: Option<&str> = {
            // English social â€” exact + partial
            let en_social = match q_clean {
            "thanks" | "thank you" | "thank you so much" | "ty" | "thx" | "cheers"
                | "that's helpful" | "thats helpful" | "that was helpful"
                | "thanks for asking" | "thanks for that" | "thanks a lot" | "much appreciated"
                | "thank you kala" | "thanks kala" => Some("thanks"),
                "bye" | "goodbye" | "see you" | "see ya" | "later" | "cya"
                | "take care" | "good bye" | "bye bye" | "gotta go" | "i gotta go"
                | "bye kala" | "goodbye kala" => Some("bye"),
                "wow" | "impressive" | "excellent" | "brilliant" | "fantastic"
                | "that's amazing" | "thats amazing" | "that was great"
                | "that's great" | "thats great" | "very nice" | "really good"
                | "that's cool" | "thats cool" => Some("impressed"),
                "nice" | "cool" | "great" | "awesome" | "perfect" | "good" | "ok"
            | "okay" | "got it" | "understood" | "makes sense" | "i see" | "noted"
                | "alright" | "right" | "fine" | "sure" | "yep" | "yup" => Some("ack"),
            _ => None,
        };
            let en_social = if en_social.is_some() { en_social }
            // Partial English matches
            else if q_lower.starts_with("thanks") || q_lower.starts_with("thank you") { Some("thanks") }
            else if q_lower.starts_with("bye") || q_lower.starts_with("goodbye") || q_lower.starts_with("see you") { Some("bye") }
            else { None };
            if en_social.is_some() { en_social }
            // Hindi social
            else if q_lower.contains("dhanyavad") || q_lower.contains("shukriya") || q_lower == "bahut accha" { Some("thanks") }
            else if q_lower.contains("alvida") || q_lower.contains("phir milenge") || q_lower == "chalo" { Some("bye") }
            else if q_lower == "badhiya" || q_lower == "zabardast" || q_lower == "mast" { Some("impressed") }
            else if q_lower == "theek hai" || q_lower == "accha" || q_lower == "haan" || q_lower == "samajh gaya" { Some("ack") }
            // Telugu social
            else if q_lower.contains("dhanyavaadalu") || q_lower == "thanks ra" || q_lower == "thanks mama" { Some("thanks") }
            else if q_lower == "sare bye" || q_lower == "bye mama" || q_lower == "veltunna" { Some("bye") }
            else if q_lower == "baaga undi" || q_lower == "manchidi" || q_lower == "super" || q_lower == "superr" { Some("impressed") }
            else if q_lower == "sare" || q_lower == "avunu" || q_lower == "ok ra" || q_lower == "ok mama" { Some("ack") }
            // Tamil social
            else if q_lower.contains("nandri") { Some("thanks") }
            else if q_lower == "poitu varren" || q_lower == "bye da" { Some("bye") }
            else if q_lower == "romba nalla" || q_lower == "semma" { Some("impressed") }
            else if q_lower == "sari" || q_lower == "purinjuchu" { Some("ack") }
            // Spanish
            else if q_lower.contains("gracias") || q_lower == "muchas gracias" { Some("thanks") }
            else if q_lower.contains("adios") || q_lower.contains("adiÃ³s") || q_lower == "hasta luego" || q_lower == "nos vemos" { Some("bye") }
            else if q_lower == "genial" || q_lower == "increÃ­ble" || q_lower == "increible" || q_lower == "excelente" { Some("impressed") }
            else if q_lower == "vale" || q_lower == "entendido" || q_lower == "de acuerdo" { Some("ack") }
            // French
            else if q_lower.contains("merci") { Some("thanks") }
            else if q_lower.contains("au revoir") || q_lower == "Ã  bientÃ´t" || q_lower == "a bientot" { Some("bye") }
            else if q_lower == "magnifique" || q_lower == "superbe" || q_lower == "gÃ©nial" || q_lower == "genial" { Some("impressed") }
            else if q_lower == "d'accord" || q_lower == "compris" || q_lower == "ok" { Some("ack") }
            // German
            else if q_lower.contains("danke") || q_lower == "vielen dank" { Some("thanks") }
            else if q_lower.contains("tschÃ¼ss") || q_lower.contains("tschuss") || q_lower.contains("auf wiedersehen") { Some("bye") }
            else if q_lower == "toll" || q_lower == "wunderbar" || q_lower == "klasse" { Some("impressed") }
            else if q_lower == "verstanden" || q_lower == "alles klar" { Some("ack") }
            // Russian
            else if q_lower.contains("ÑÐ¿Ð°ÑÐ¸Ð±Ð¾") || q_lower.contains("Ð±Ð»Ð°Ð³Ð¾Ð´Ð°Ñ€") { Some("thanks") }
            else if q_lower.contains("Ð¿Ð¾ÐºÐ°") || q_lower.contains("Ð´Ð¾ ÑÐ²Ð¸Ð´Ð°Ð½Ð¸Ñ") { Some("bye") }
            else if q_lower == "Ð¾Ñ‚Ð»Ð¸Ñ‡Ð½Ð¾" || q_lower == "ÐºÐ»Ð°ÑÑ" || q_lower == "ÐºÑ€ÑƒÑ‚Ð¾" { Some("impressed") }
            else if q_lower == "Ð¿Ð¾Ð½ÑÐ»" || q_lower == "Ñ…Ð¾Ñ€Ð¾ÑˆÐ¾" || q_lower == "Ð»Ð°Ð´Ð½Ð¾" { Some("ack") }
            // Japanese
            else if q_lower.contains("ã‚ã‚ŠãŒã¨ã†") { Some("thanks") }
            else if q_lower.contains("ã•ã‚ˆã†ãªã‚‰") || q_lower.contains("ã˜ã‚ƒã‚ã­") || q_lower.contains("ã¾ãŸã­") { Some("bye") }
            else if q_lower.contains("ã™ã”ã„") || q_lower.contains("ç´ æ™´ã‚‰ã—ã„") { Some("impressed") }
            else if q_lower.contains("åˆ†ã‹ã£ãŸ") || q_lower.contains("äº†è§£") || q_lower == "ã¯ã„" { Some("ack") }
            // Korean
            else if q_lower.contains("ê°ì‚¬") || q_lower.contains("ê³ ë§ˆì›Œ") { Some("thanks") }
            else if q_lower.contains("ì•ˆë…•") && (q_lower.len() < 15 || q_lower.contains("ìž˜ ê°€")) { Some("bye") }
            else if q_lower.contains("ëŒ€ë°•") || q_lower.contains("ë©‹ì ¸") { Some("impressed") }
            else if q_lower == "ë„¤" || q_lower.contains("ì•Œê² ") { Some("ack") }
            // Chinese
            else if q_lower.contains("è°¢è°¢") { Some("thanks") }
            else if q_lower.contains("å†è§") || q_lower.contains("æ‹œæ‹œ") { Some("bye") }
            else if q_lower.contains("å¤ªå¥½äº†") || q_lower.contains("åŽ‰å®³") { Some("impressed") }
            else if q_lower.contains("æ˜Žç™½") || q_lower.contains("å¥½çš„") || q_lower == "å—¯" { Some("ack") }
            // Arabic
            else if q_lower.contains("Ø´ÙƒØ±Ø§") { Some("thanks") }
            else if q_lower.contains("Ù…Ø¹ Ø§Ù„Ø³Ù„Ø§Ù…Ø©") || q_lower.contains("Ø¨Ø§ÙŠ") { Some("bye") }
            else if q_lower.contains("Ø±Ø§Ø¦Ø¹") || q_lower.contains("Ù…Ù…ØªØ§Ø²") { Some("impressed") }
            else if q_lower.contains("ØªÙ…Ø§Ù…") || q_lower.contains("Ø­Ø³Ù†Ø§") { Some("ack") }
            // Turkish
            else if q_lower.contains("teÅŸekkÃ¼r") || q_lower == "saÄŸol" || q_lower == "sagol" { Some("thanks") }
            else if q_lower.contains("gÃ¼le gÃ¼le") || q_lower == "hoÅŸÃ§akal" || q_lower == "hoscakal" { Some("bye") }
            else if q_lower == "harika" || q_lower == "muhteÅŸem" || q_lower == "muhtesem" { Some("impressed") }
            else if q_lower == "tamam" || q_lower == "anladÄ±m" || q_lower == "anladim" { Some("ack") }
            // Italian
            else if q_lower.contains("grazie") { Some("thanks") }
            else if q_lower.contains("arrivederci") || q_lower == "a dopo" { Some("bye") }
            else if q_lower == "fantastico" || q_lower == "bellissimo" || q_lower == "ottimo" { Some("impressed") }
            else if q_lower == "capito" || q_lower == "va bene" { Some("ack") }
            // Portuguese
            else if q_lower.contains("obrigado") || q_lower.contains("obrigada") { Some("thanks") }
            else if q_lower.contains("tchau") || q_lower == "atÃ© logo" || q_lower == "ate logo" { Some("bye") }
            else if q_lower == "incrÃ­vel" || q_lower == "incrivel" || q_lower == "maravilhoso" { Some("impressed") }
            else if q_lower == "entendi" || q_lower == "certo" { Some("ack") }
            else { None }
        };
        if let Some(cat) = social_cat {
            return Self::kala_localized(chat_lang, cat, None);
        }

        // â”€â”€ Tier 0d: Image / Video generation â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        // â”€â”€ Tier 0e: Code generation â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        // Detect: "write html code", "python code for X", "create a function", "create table for X", etc.
        {
            let has_lang = q_lower.contains("html") || q_lower.contains("python")
                || q_lower.contains("javascript") || q_lower.contains(" js ")
                || q_lower.ends_with(" js") || q_lower.contains("typescript")
                || q_lower.contains(".js") || q_lower.contains("jsx")
                || q_lower.contains("three.js") || q_lower.contains("threejs") || q_lower.contains("three ")
                || q_lower.contains("webgl") || q_lower.contains("web gpu") || q_lower.contains("wgpu")
                || q_lower.contains("shader") || q_lower.contains("glsl")
                || q_lower.contains("mediapipe") || q_lower.contains("tensorflow")
                || q_lower.contains("css") || q_lower.contains(" java")
                || q_lower.contains("c++") || q_lower.contains("cpp")
                || q_lower.contains("csharp") || q_lower.contains("c#") || q_lower.contains("c sharp")
                || q_lower.contains("rust") || q_lower.contains("sql")
                || q_lower.contains("tsql") || q_lower.contains("t-sql")
                || q_lower.contains("bash") || q_lower.contains("shell")
                || q_lower.contains("kotlin") || q_lower.contains("swift")
                || q_lower.contains("golang") || q_lower.contains(" go ") || q_lower.ends_with(" go") || q_lower.ends_with(" golang")
                || q_lower.contains("ruby") || q_lower.contains("php")
                || q_lower.contains("scala") || q_lower.contains("dart")
                || q_lower.contains("killer")
                || q_lower.contains("react") || q_lower.contains("next.js") || q_lower.contains("nextjs")
                || q_lower.contains("vue") || q_lower.contains("angular") || q_lower.contains("svelte")
                || q_lower.contains("django") || q_lower.contains("fastapi")
                || q_lower.contains("nodejs") || q_lower.contains("node.js")
                || q_lower.contains("spring") || q_lower.contains("laravel");
            let has_standalone_script = q_lower.contains("script")
                && !q_lower.replace("javascript", "").replace("typescript", "").replace("applescript", "").is_empty()
                && q_lower.replace("javascript", "").replace("typescript", "").replace("applescript", "").contains("script");
            let has_code_word = q_lower.contains("code") || has_standalone_script
                || q_lower.contains("program") || q_lower.contains("function")
                || q_lower.contains("snippet") || q_lower.contains("table")
                || q_lower.contains("class") || q_lower.contains("algorithm")
                || q_lower.contains("implement") || q_lower.contains("example")
                || q_lower.contains("hello world")
                || q_lower.contains("gesture") || q_lower.contains("gestor") || q_lower.contains("gester")
                || q_lower.contains("facemesh") || q_lower.contains("landmark")
                || q_lower.contains("tracking");
            let starts_write = q_lower.starts_with("write ") || q_lower.starts_with("create ")
                || q_lower.starts_with("make ") || q_lower.starts_with("build ")
                || q_lower.starts_with("generate ") || q_lower.starts_with("show me ")
                || q_lower.starts_with("give me ") || q_lower.starts_with("implement ")
                || q_lower.starts_with("code ") || q_lower.starts_with("how to ");
            // Direct SQL patterns: "create table for X", "create table script"
            let is_direct_sql = q_lower.contains("create table") || q_lower.contains("create a table")
                || q_lower.contains("table for ") || q_lower.contains("table script")
                || q_lower.contains("table schema");
            // Direct algorithm/pattern requests without lang keyword
            let is_algo_req = q_lower.contains("quicksort") || q_lower.contains("mergesort")
                || q_lower.contains("merge sort") || q_lower.contains("binary search")
                || q_lower.contains("bubble sort") || q_lower.contains("linked list")
                || q_lower.contains("fibonacci") || q_lower.contains("factorial")
                || q_lower.contains("fizzbuzz") || q_lower.contains("fizz buzz")
                || q_lower.contains("hello world") || q_lower.contains("helloworld")
                || q_lower.contains("palindrome")
                || q_lower.contains("two sum") || q_lower.contains("sieve")
                || q_lower.contains("binary tree") || q_lower.contains("binary search tree") || q_lower.contains("bst")
                || q_lower.contains("hash map") || q_lower.contains("hashmap")
                || q_lower.contains("dijkstra") || q_lower.contains("shortest path")
                || q_lower.contains("rest api") || q_lower.contains("fastapi")
                || q_lower.contains("flask") || q_lower.contains("express")
                || q_lower.contains("singleton") || q_lower.contains("decorator pattern")
                || q_lower.contains("observer pattern") || q_lower.contains("design pattern")
                || q_lower.contains("websocket") || q_lower.contains("jwt")
                || q_lower.contains("crud") || q_lower.contains("argparse")
                || q_lower.contains("try catch") || q_lower.contains("try-catch")
                || q_lower.contains("error handling") || (q_lower.contains("sort") && q_lower.contains("list"))
                || (q_lower.contains("reverse") && (q_lower.contains("string") || q_lower.contains("list")))
                || ((q_lower.contains("bfs") || q_lower.contains("dfs")) && !q_lower.starts_with("what"));
            // Conversational code requests: "can you write a code?", "write me some code", "help me code"
            let is_conversational_code = {
                let cq = q_lower.as_str();
                cq.contains("write a code") || cq.contains("write me code") || cq.contains("write some code")
                || cq.contains("write me a code") || cq.contains("write me some code")
                || cq.contains("generate a code") || cq.contains("generate code")
                || cq.contains("give me a code") || cq.contains("give me code")
                || cq.contains("can you code") || cq.contains("help me code")
                || cq.contains("need a code") || cq.contains("need code")
                || (cq.contains("can you write") && has_code_word)
                || (cq.contains("could you write") && has_code_word)
                || (cq.contains("please write") && has_code_word)
                || (cq.contains("help me write") && has_code_word)
            };
            // Full projects / scaffolds â†’ code generation (multi-file instructions in LLM prompt)
            let is_question_lead = q_lower.starts_with("what ")
                || q_lower.starts_with("why ")
                || q_lower.starts_with("who ")
                || q_lower.starts_with("when ")
                || q_lower.starts_with("where ")
                || q_lower.starts_with("explain ")
                || q_lower.starts_with("describe ")
                || q_lower.starts_with("define ");
            let is_project_scaffold = !is_question_lead
                && (q_lower.contains("scaffold")
                    || q_lower.contains("boilerplate")
                    || q_lower.contains("starter kit")
                    || q_lower.contains("full stack")
                    || q_lower.contains("full-stack")
                    || q_lower.contains("monorepo")
                    || ((q_lower.contains("create")
                        || q_lower.contains("build")
                        || q_lower.contains("generate")
                        || q_lower.contains("make"))
                        && q_lower.contains(" project")));
            // Comparison queries ("X vs Y") should never be routed to code generation
            let is_comparison = q_lower.contains(" vs ") || q_lower.contains(" versus ");
            // Combined: has lang + code word; starts with action verb + code word + lang; direct SQL; algorithm request; conversational
            // "write code" / "give me code" â€” short requests without a named language
            let is_code = !is_comparison && (
                (has_lang && has_code_word)
                || (starts_write && has_lang)
                || (starts_write && has_code_word)
                || (starts_write && is_algo_req)
                || is_direct_sql
                || is_algo_req
                || is_conversational_code
                || is_project_scaffold
            );
            if is_code {
                return crate::khlm_polyglot::khlm_generate_code(question);
            }
        }

        // Short queries (â‰¤7 words) with visual word that aren't questions/descriptions
        let wc = q_lower.split_whitespace().count();
        let has_visual_word = q_lower.contains("image") || q_lower.contains("picture")
            || q_lower.contains("photo") || q_lower.contains(" pic ") || q_lower.ends_with(" pic");
        let is_question_form = q_lower.starts_with("what") || q_lower.starts_with("how")
            || q_lower.starts_with("why") || q_lower.starts_with("where")
            || q_lower.starts_with("when") || q_lower.starts_with("who")
            || q_lower.starts_with("is ") || q_lower.starts_with("are ")
            || q_lower.starts_with("explain") || q_lower.starts_with("describe")
            || q_lower.starts_with("tell me") || q_lower.starts_with("analyze")
            || q_lower.starts_with("show me how") || q_lower.starts_with("load ")
            || q_lower.contains("this image") || q_lower.contains("the image")
            || q_lower.contains("existing image") || q_lower.contains("load image");

        let is_gen_image = q_lower.contains("generate image") || q_lower.contains("create image")
            || q_lower.contains("make image") || q_lower.contains("make a picture")
            || q_lower.contains("draw me") || q_lower.contains("generate a picture")
            || q_lower.contains("image of") || q_lower.contains("picture of")
            || (q_lower.contains("generate") && q_lower.contains("art"))
            || (q_lower.contains("create") && q_lower.contains("photo"))
            || (q_lower.contains("make") && q_lower.contains("photo"))
            // Short visual requests ("create nature image", "careat natcher image", etc.)
            || (!is_question_form && has_visual_word && wc <= 7);

        let is_gen_video = q_lower.contains("generate video") || q_lower.contains("create video")
            || q_lower.contains("make video") || q_lower.contains("make a video")
            || q_lower.contains("video of") || q_lower.contains("create a video")
            || (q_lower.contains("generate") && q_lower.contains("video"))
            || (q_lower.contains("make") && q_lower.contains("clip"))
            || (q_lower.contains("create") && q_lower.contains("clip"));

        let is_gen_audio = q_lower.contains("generate audio") || q_lower.contains("create audio")
            || q_lower.contains("make audio") || q_lower.contains("generate music")
            || q_lower.contains("create music") || q_lower.contains("make music")
            || q_lower.contains("generate sound") || q_lower.contains("play music")
            || q_lower.contains("generate a beat") || q_lower.contains("create a beat")
            || q_lower.contains("make a beat") || q_lower.contains("play sound")
            || (q_lower.contains("generate") && q_lower.contains("song"))
            || (q_lower.contains("create") && q_lower.contains("song"))
            || (q_lower.contains("make") && q_lower.contains("song"))
            || (q_lower.contains("ambient") && q_lower.contains("sound"))
            || (q_lower.contains("ocean") && q_lower.contains("sound"));

        if is_gen_image {
            // Extract actual subject: look for "of ..." pattern, else strip filler words
            let q_work = q_lower.clone();
            let subject: String = if let Some(of_pos) = q_work.find(" of ") {
                // Extract everything after "of" â€” cleanest signal
                q_work[of_pos + 4..].trim().to_string()
            } else {
                q_work
                    .replace("can you generate", "").replace("could you generate", "")
                    .replace("can you create", "").replace("could you create", "")
                    .replace("can you make", "").replace("could you make", "")
                    .replace("can you build", "").replace("could you build", "")
                    .replace("can you draw", "").replace("could you draw", "")
                    .replace("please generate", "").replace("please create", "")
                    .replace("please make", "").replace("please draw", "")
                    .replace("build me an", "").replace("build me a", "").replace("build me", "")
                    .replace("make me an", "").replace("make me a", "").replace("make me", "")
                    .replace("generate image of", "").replace("generate an image of", "")
                    .replace("create image of", "").replace("create an image of", "")
                    .replace("make image of", "").replace("make an image of", "")
                    .replace("generate a picture of", "").replace("make a picture of", "")
                    .replace("draw me a", "").replace("draw me an", "")
                    .replace("generate", "").replace("create", "").replace("make", "")
                    .replace("build", "").replace("draw", "").replace("image", "")
                    .replace("picture", "").replace("photo", "").replace("can you", "")
                    .replace("could you", "").replace("please", "").replace("  ", " ")
                    .trim().to_string()
            };
            let prompt = if subject.len() > 3 { &subject } else { question };
            return crate::image_gen::generate_image(prompt.trim(), "auto", "square");
        }

        if is_gen_video {
            let q_work = q_lower.clone();
            let subject: String = if let Some(of_pos) = q_work.find(" of ") {
                q_work[of_pos + 4..].trim().to_string()
            } else {
                q_work
                    .replace("can you generate", "").replace("could you generate", "")
                    .replace("can you create", "").replace("could you create", "")
                    .replace("can you make", "").replace("could you make", "")
                    .replace("please generate", "").replace("please create", "")
                    .replace("generate video of", "").replace("generate a video of", "")
                    .replace("create video of", "").replace("create a video of", "")
                    .replace("make video of", "").replace("make a video of", "")
                    .replace("generate", "").replace("create", "").replace("make", "")
                    .replace("video", "").replace("clip", "").replace("can you", "")
                    .replace("could you", "").replace("please", "").replace("  ", " ")
                    .trim().to_string()
            };
            let prompt = if subject.len() > 3 { &subject } else { question };
            return crate::image_gen::generate_video(prompt.trim(), "5");
        }

        if is_gen_audio {
            let q_work = q_lower.clone();
            let subject: String = if let Some(of_pos) = q_work.find(" of ") {
                q_work[of_pos + 4..].trim().to_string()
            } else {
                q_work
                    .replace("can you generate", "").replace("could you generate", "")
                    .replace("can you create", "").replace("could you create", "")
                    .replace("can you make", "").replace("could you make", "")
                    .replace("please generate", "").replace("please create", "")
                    .replace("generate audio", "").replace("generate a beat", "")
                    .replace("create audio", "").replace("create a beat", "")
                    .replace("make audio", "").replace("make a beat", "")
                    .replace("generate music", "").replace("create music", "").replace("make music", "")
                    .replace("generate sound", "").replace("play music", "").replace("play sound", "")
                    .replace("generate", "").replace("create", "").replace("make", "")
                    .replace("audio", "").replace("music", "").replace("sound", "").replace("beat", "")
                    .replace("song", "").replace("can you", "").replace("could you", "")
                    .replace("please", "").replace("  ", " ")
                    .trim().to_string()
            };
            let prompt = if subject.len() > 3 { &subject } else { question };
            return crate::image_gen::generate_audio(prompt.trim());
        }

        // â”€â”€ Tier 0.4: Self-knowledge â€” Kala's own features / creator â”€â”€â”€â”€â”€â”€â”€â”€
        // Detect questions about Kala itself, its modes, its creator, the Killer language
        // so we never web-search things we know internally.
        {
            // Creator references: "who arun", "who is arun", "arun kumar", "katherashala"
            // BUT NOT: "create table for arun company" (code requests that contain "arun" as data)
            let mentions_arun = q_lower.contains("arun") || q_lower.contains("katherashala")
                || q_lower.contains("sai kumar") || q_lower.contains("creator of killer")
                || q_lower.contains("creator of kala");
            let is_code_context = q_lower.contains("table") || q_lower.contains("code")
                || q_lower.contains("sql") || q_lower.contains("create") || q_lower.contains("write")
                || q_lower.contains("function") || q_lower.contains("class") || q_lower.contains("script")
                || q_lower.contains("program") || q_lower.contains("database") || q_lower.contains("query");
            // Require a clear â€œwho / about this personâ€ signal â€” do not fire on every substring â€œarunâ€.
            let is_about_person = q_lower.contains("who ") || q_lower.contains("who's")
                || q_lower.contains("whos ") || q_lower.starts_with("who")
                || q_lower.contains("tell me about") || q_lower.contains("about arun")
                || q_lower.contains("about sai") || q_lower.contains("biodata")
                || q_lower.contains("biography");
            let is_creator_q = mentions_arun && is_about_person && !is_code_context;
            if is_creator_q {
                return "**Sai Arun Kumar Katherashala** is the creator of the Killer programming language and the Kala AI engine.\n\n\
                He built Killer as a full-stack language with a native AI engine, web search (Ghost-108), \n\
                prose generation, media generation where enabled, and **multiple native AI subsystems** in Rust (not AGI/ASI â€” those are curriculum topics in AI Lab).\n\n\
                *That's my creator. What else would you like to know?*".to_string();
            }

            // Pronoun + info requests ("his biodata", "his full info") â†’ check if previous was about creator
            let is_pronoun_ref = (q_lower.starts_with("his ") || q_lower.starts_with("her ")
                || q_lower.starts_with("their ")) && (q_lower.contains("bio") || q_lower.contains("info")
                || q_lower.contains("detail") || q_lower.contains("profile") || q_lower.contains("full")
                || q_lower.contains("background") || q_lower.contains("resume"));
            let is_generic_more = q_lower == "more info" || q_lower == "full info"
                || q_lower == "more details" || q_lower == "full details"
                || q_lower.starts_with("i need full info");
            if is_pronoun_ref || is_generic_more {
                let history = crate::khlm_polyglot::get_conversation_history_pub();
                let prev_was_creator = history.iter().rev()
                    .find(|(r, _)| r == "assistant")
                    .map(|(_, c)| c.contains("Katherashala") || c.contains("Sai Arun"))
                    .unwrap_or(false);
                if prev_was_creator {
                    return "**Sai Arun Kumar Katherashala** â€” expanded profile:\n\n\
                        - **Role**: Creator & Lead Developer of the Killer programming language\n\
                        - **Built**: Kala AI engine, Ghost-108 search, Nova compression, KhLM router\n\
                        - **Tech stack**: Pure Rust, zero external dependencies\n\
                        - **AI systems**: native modes & engines â€” KhLM, Ghost-108, inference, prose, imagination, affect, code/vision, guardian (AGI/ASI not shipped)\n\
                        - **Innovations**: Native AI in a programming language (no Python/TensorFlow dependency), offline-first architecture\n\
                        - **Philosophy**: \"AI should be built directly into the language, not bolted on as a library\"\n\n\
                        *Ask me specific questions about his work â€” the language design, the AI architecture, or the vision behind Killer.*".to_string();
                }
            }

            // Questions about AI Lab / Kala features from ASK mode
            let is_self_feature = (q_lower.contains("ai lab") || q_lower.contains("ai labs"))
                && (q_lower.contains("what") || q_lower.contains("how") || q_lower.contains("help"));
            if is_self_feature {
                return "**Kala AI Lab** runs **native Rust demos** (math, ML, DL building blocks, NLP, tabular RL, agent demos) plus **honest curriculum** text for AGI/ASI/â€œAI OSâ€ (those are **not** shipped product tiers).\n\n\
                Switch to **Lab mode** (ðŸ§ª) and try:\n\
                - *\"run linear regression\"* â€” live ML demo\n\
                - *\"explain LSTM\"* â€” DL / components\n\
                - *\"how does attention work\"* â€” transformer math\n\
                - *\"what is AGI\"* â€” definitions & gaps (curriculum)\n\n\
                Configure an LLM for deeper cloud-assisted explanations. What topic first?".to_string();
            }
        }

        // â”€â”€ Tier 0.45: Learning / Explanation intent (offline knowledge) â”€â”€â”€â”€â”€â”€
        // "teach me ai", "can you explain what is ai", "explain python", "what is deep learning"
        // Route these to knowledge base BEFORE conversational handler eats them.
        {
            let is_learning_intent = q_lower.starts_with("teach me ")
                || q_lower.starts_with("can you teach me ")
                || q_lower.starts_with("can you explain ")
                || q_lower.starts_with("could you explain ")
                || q_lower.starts_with("please explain ")
                || q_lower.starts_with("explain ")
                || q_lower.starts_with("can you tell me about ")
                || q_lower.starts_with("tell me about ")
                || q_lower.starts_with("i want to learn ")
                || q_lower.starts_with("i want to learn about ")
                || q_lower.starts_with("help me understand ")
                || q_lower.starts_with("help me learn ")
                || q_lower.starts_with("what is ")
                || q_lower.starts_with("what are ")
                || q_lower.starts_with("what's ")
                || q_lower.starts_with("who is ")
                || q_lower.starts_with("who was ")
                || q_lower.starts_with("define ")
                || q_lower.starts_with("what do you know about ");
            if is_learning_intent {
                let wants_simple_here = q_lower.contains("simple terms")
                    || q_lower.contains("simply")
                    || q_lower.contains("eli5")
                    || q_lower.contains("like i'm 5") || q_lower.contains("like im 5")
                    || q_lower.contains("for beginners") || q_lower.contains("for a beginner")
                    || q_lower.contains("easy to understand") || q_lower.contains("in easy words")
                    || q_lower.contains("in plain english") || q_lower.contains("layman");

                let topic = q_lower
                    .replace('?', "").replace('!', "").replace('.', "")
                    .replace("can you explain ", "").replace("could you explain ", "")
                    .replace("please explain ", "").replace("explain ", "")
                    .replace("can you teach me ", "").replace("teach me about ", "")
                    .replace("teach me ", "")
                    .replace("can you tell me about ", "").replace("can you tell me ", "")
                    .replace("tell me about ", "")
                    .replace("i want to learn about ", "").replace("i want to learn ", "")
                    .replace("help me understand ", "").replace("help me learn ", "")
                    .replace("what do you know about ", "")
                    .replace("what is ", "").replace("what are ", "").replace("what's ", "")
                    .replace("who is ", "").replace("who was ", "")
                    .replace("define ", "")
                    .replace("can you ", "").replace("could you ", "")
                    .replace("please ", "").replace("about ", "")
                    .replace("in simple terms", "").replace("simply", "")
                    .replace("eli5", "").replace("like i'm 5", "").replace("like im 5", "")
                    .replace("for beginners", "").replace("for a beginner", "")
                    .replace("in easy words", "").replace("in plain english", "")
                    .replace("dumb it down", "").replace("layman", "")
                    .replace("  ", " ");
                let topic = topic.trim();
                let is_vague_topic = matches!(topic,
                    "some" | "something" | "things" | "stuff" | "anything"
                    | "everything" | "more" | "it" | "that" | "this"
                    | "new" | "interesting" | "cool" | "good" | "nice"
                    | "a thing" | "a topic" | "a subject" | "nothing"
                    | "idk" | "whatever" | "something new" | "something interesting"
                    | "" | "a" | "the" | "thing" | "topic");
                if is_vague_topic {
                    return "I'd love to teach you! ðŸ˜Š What topic are you interested in?\n\n\
                        Here are some popular areas I can cover:\n\
                        - **Programming**: Python, Rust, JavaScript, Java, C++\n\
                        - **AI/ML**: Machine Learning, Deep Learning, Neural Networks, LLMs\n\
                        - **CS Fundamentals**: Data Structures, Algorithms, Big O, Design Patterns\n\
                        - **DevOps**: Docker, Kubernetes, CI/CD, Git\n\
                        - **Science**: Gravity, DNA, Evolution, Quantum Computing\n\
                        - **Countries**: India, USA, Japan, Germany...\n\n\
                        *Just name a topic and I'll dive right in!*".to_string();
                }
                // With an API LLM configured, defer to kala_expert_ask (online-first) instead of static KB.
                let skip_embedded_kb = crate::khlm_polyglot::config().lock().unwrap().llm_available();
                if !topic.is_empty() && !skip_embedded_kb {
                    if let Some(answer) = crate::llm::knowledge_base_lookup_pub(topic) {
                        let result = if wants_simple_here {
                            Self::simplify_response(&answer, topic)
                        } else {
                            answer
                        };
                        let followups = Self::generate_followups(question);
                        if !followups.is_empty() {
                            return format!("{}\n\n---\n{}", result, followups);
                        }
                        return result;
                    }
                }
            }
        }

        // â”€â”€ Tier 0.5: Conversational Intelligence (offline, context-aware) â”€â”€
        // Detects messages that are conversational / interactive / feedback
        // and responds with personality â€” instead of web-searching everything.
        // Applies to ALL modes â€” every mode benefits from conversational awareness.
        if let Some(conv_resp) = Self::kala_conversational_response(&q_lower, question, mode) {
            return conv_resp;
        }

        // â”€â”€ Tier 0.6: Context-aware query rewriting â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        // "What is it?" / "What it is?" â†’ rewrite to the actual topic from history.
        // "Talk about X" / "Discuss X" â†’ rewrite to "tell me about X" for better search.
        let rewritten_query: Option<String> = {
            let ql = q_lower.trim().trim_end_matches('?').trim_end_matches('!').trim_end_matches('.').trim();
            let wc = ql.split_whitespace().count();
            let is_context_ref = ql == "what is it" || ql == "what it is"
                || ql == "what about it" || ql == "what about that"
                || ql == "tell me about it" || ql == "tell me about that"
                || ql == "explain it" || ql == "explain that"
                || ql == "more about it" || ql == "more about that"
                || ql == "and what is it" || ql == "so what is it"
                // Follow-up questions that implicitly reference the previous topic
                || ql == "how does it work" || ql == "how does it work technically"
                || ql == "what are its limitations" || ql == "what are the limitations"
                || ql == "show me a practical example" || ql == "show me an example"
                || ql == "give me an example" || ql == "how does it work internally"
                || ql == "show me the math" || ql == "write a simple implementation"
                || ql == "how does it compare" || ql == "what are the pros and cons"
                || ql == "what are best practices" || ql == "what are common mistakes"
                || ql == "any alternatives" || ql == "tell me more"
                || (wc <= 5 && (ql.ends_with(" it") || ql.ends_with(" that"))
                    && (ql.starts_with("what") || ql.starts_with("how") || ql.starts_with("why") || ql.starts_with("tell")));
            if is_context_ref {
                let history = crate::khlm_polyglot::get_conversation_history_pub();
                // First, try to extract topic from Kala's last response
                let topic_from_kala = history.iter().rev()
                    .find(|(r, _)| r == "assistant")
                    .and_then(|(_, c)| {
                        let cl = c.to_lowercase();
                        // Known topic keywords to extract from Kala's response
                        let topics = [
                            ("ai agent", "AI agents"), ("artificial intelligence", "artificial intelligence"),
                            ("machine learning", "machine learning"), ("deep learning", "deep learning"),
                            ("neural network", "neural networks"), ("transformer", "transformers"),
                            ("large language model", "LLMs"), ("llm", "LLMs"),
                            ("python", "Python"), ("rust", "Rust"), ("javascript", "JavaScript"),
                            ("java", "Java"), ("typescript", "TypeScript"),
                            ("gemini", "Gemini"), ("chatgpt", "ChatGPT"), ("openai", "OpenAI"),
                            ("blockchain", "blockchain"), ("cloud computing", "cloud computing"),
                            ("docker", "Docker"), ("kubernetes", "Kubernetes"),
                            ("database", "databases"), ("api", "APIs"), ("react", "React"),
                            ("data structure", "data structures"), ("algorithm", "algorithms"),
                        ];
                        for (key, name) in &topics {
                            if cl.contains(key) {
                                return Some(name.to_string());
                            }
                        }
                        None
                    });
                if let Some(ref topic) = topic_from_kala {
                    // Rewrite the follow-up question with the extracted topic
                    Some(format!("{} {}", ql, topic))
                } else {
                    // Fall back to looking at previous user question
                    history.iter().rev()
                        .filter(|(r, _)| r == "user")
                        .find_map(|(_, c)| {
                            let cl = c.to_lowercase();
                            let cwc = cl.split_whitespace().count();
                            if cl.contains("what is it") || cl.contains("what it is")
                                || cl.contains("tell me more") || cl.contains("explain it")
                                || cwc <= 2 || cl.starts_with("ok ") || cl.starts_with("yes")
                                || cl.starts_with("no ") || cl.contains("you can ")
                                || cl.contains("not listening") || cl.contains("not understanding") {
                                return None;
                            }
                            let topic = cl
                                .trim_start_matches("talk about ")
                                .trim_start_matches("tell me about ")
                                .trim_start_matches("do you know ")
                                .trim_start_matches("what is ")
                                .trim_start_matches("who is ")
                                .trim_start_matches("what about ")
                                .trim_start_matches("discuss ")
                                .trim_end_matches('?').trim_end_matches('!').trim_end_matches('.')
                                .trim().to_string();
                            if topic.len() > 2 { Some(format!("tell me about {}", topic)) } else { None }
                        })
                }
            } else if ql.starts_with("talk about ") {
                let topic = ql.trim_start_matches("talk about ").trim();
                if !topic.is_empty() { Some(format!("tell me about {}", topic)) } else { None }
            } else if ql.starts_with("discuss ") {
                let topic = ql.trim_start_matches("discuss ").trim();
                if !topic.is_empty() { Some(format!("tell me about {}", topic)) } else { None }
            } else {
                None
            }
        };
        let question = rewritten_query.as_deref().unwrap_or(question);

        // â”€â”€ Route to correct engine â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        let q = Value::Str(question.to_string());
        let s = Value::Str(style.to_string());
        let l = Value::Str(lang.to_string());

        // Killer AI System â€” mandatory native multi-agent orchestration (KhLM + Ghost-108 + neural)
        if mode == "ai_system" || mode == "multi_agent" {
            return crate::llm::khlm_ai_system_multi_agent(question);
        }

        // "lab" mode â€” native Rust demos + curriculum (AGI/ASI/AI OS = concepts, not shipped systems)
        if mode == "lab" {
            return crate::khlm_polyglot::khlm_ai_lab(question);
        }

        // "code" mode â€” direct Copilot-style code generation
        if mode == "code" {
            return crate::khlm_polyglot::khlm_generate_code(question);
        }

        // "think" mode: try LLM expert reasoning first, fall back to native_think
        if mode == "think" {
            let llm_think = crate::khlm_polyglot::khlm_think_expert(question);
            if !llm_think.is_empty() {
                return llm_think;
            }
            let raw = match Self::native_think(std::slice::from_ref(&q)) {
                Ok(Value::Str(s)) => s,
                Ok(v)             => format!("{:?}", v),
                Err(e)            => format!("[Kala] {}", e),
            };
            return Self::polish_response(raw, question);
        }

        // "imagine" / "what_if" modes: detect capability/task requests and reroute
        if mode == "imagine" || mode == "what_if" {
            let ql = question.trim().to_lowercase();

            // Image-intent detection: in imagine mode, generate an actual image
            // when the prompt describes a visual scene (not a philosophical "what if")
            let has_visual_keyword = ql.contains("image") || ql.contains("picture")
                || ql.contains("photo") || ql.contains("draw") || ql.contains("paint")
                || ql.contains("generate") || ql.contains("create")
                || ql.contains("make");
            let has_scene_keyword = ql.contains("sunset") || ql.contains("sunrise")
                || ql.contains("ocean") || ql.contains("mountain") || ql.contains("forest")
                || ql.contains("city") || ql.contains("neon") || ql.contains("space")
                || ql.contains("galaxy") || ql.contains("fire") || ql.contains("nature")
                || ql.contains("landscape") || ql.contains("sky") || ql.contains("beach")
                || ql.contains("river") || ql.contains("snow") || ql.contains("rain")
                || ql.contains("night") || ql.contains("cloud") || ql.contains("star")
                || ql.contains("tree") || ql.contains("flower") || ql.contains("garden")
                || ql.contains("desert") || ql.contains("volcano") || ql.contains("water")
                || ql.contains("fractal") || ql.contains("abstract") || ql.contains("art")
                || ql.contains("dog") || ql.contains("cat") || ql.contains("animal");
            let is_philosophical = ql.starts_with("what if") || ql.starts_with("what would")
                || ql.starts_with("imagine if") || ql.starts_with("suppose ")
                || ql.starts_with("pretend ") || ql.contains("what if ");
            // Route to image generation if it looks visual and not philosophical
            if (has_visual_keyword || has_scene_keyword) && !is_philosophical {
                let subject = ql
                    .replace("generate", "").replace("create", "").replace("make", "")
                    .replace("draw", "").replace("paint", "").replace("image", "")
                    .replace("picture", "").replace("photo", "").replace("of ", "")
                    .replace("  ", " ").trim().to_string();
                let prompt = if subject.len() > 2 { &subject } else { question };
                return crate::image_gen::generate_image(prompt.trim(), "auto", "square");
            }

            // These look like direct requests to Kala, not imaginative premises
            let is_direct_request =
                ql.starts_with("can you") || ql.starts_with("could you")
                || ql.starts_with("please ") || ql.starts_with("would you")
                || ql.starts_with("help me") || ql.starts_with("help with")
                || ql.starts_with("how do i") || ql.starts_with("how to ")
                || ql.starts_with("how can i") || ql.starts_with("tell me how")
                || ql.starts_with("teach me") || ql.starts_with("explain how")
                || ql.starts_with("show me how") || ql.starts_with("make me a")
                || ql.starts_with("build me") || ql.starts_with("create a")
                || ql.starts_with("generate a") || ql.starts_with("write me")
                || ql.starts_with("give me") || ql.starts_with("i need a")
                || ql.starts_with("i want a") || ql.starts_with("do you know");
            if is_direct_request {
                return Self::kala_expert_ask(question);
            }

            // Try LLM expert imagine first
            let llm_imag = crate::khlm_polyglot::khlm_imagine_expert(question);
            if !llm_imag.is_empty() {
                return llm_imag;
            }
        }

        let result = match mode {
            "imagine" => Self::builtin_imagine(std::slice::from_ref(&q)),
            "what_if" => Self::builtin_imagine_what_if(std::slice::from_ref(&q)),
            "guard"   => Self::builtin_guardian_check(std::slice::from_ref(&q)),
            "write"   => {
                // Detect code-generation requests â€” route to Copilot-style code gen
                let ql = question.trim().to_lowercase();
                let is_code = ql.contains("function") || ql.contains("class") || ql.contains("method")
                    || ql.contains("algorithm") || ql.contains("implement")
                    || ql.contains("quicksort") || ql.contains("merge sort") || ql.contains("bubble sort")
                    || ql.contains("binary search") || ql.contains("fibonacci") || ql.contains("factorial")
                    || ql.contains("linked list") || ql.contains("stack") || ql.contains("queue")
                    || ql.contains("unit test") || ql.contains("unittest") || ql.contains("pytest")
                    || ql.contains("regex") || ql.contains("http server") || ql.contains("web server")
                    || ql.contains("file read") || ql.contains("file write") || ql.contains("read file")
                    || ql.contains("sort a list") || ql.contains("sort an array") || ql.contains("sort list")
                    || ql.contains("palindrome") || ql.contains("two sum") || ql.contains("prime")
                    || ql.contains("reverse string") || ql.contains("reverse array")
                    || (ql.starts_with("write a") && (ql.contains(" in python") || ql.contains(" in rust") || ql.contains(" in killer") || ql.contains(" in javascript") || ql.contains(" that ") || ql.contains(" to ")))
                    || (ql.starts_with("write") && ql.contains("code"));
                if is_code {
                    return crate::khlm_polyglot::khlm_generate_code(question);
                }
                crate::khlm_polyglot::builtin_khlm_write(&[q, s])
            },
            "debug"   => crate::khlm_polyglot::builtin_khlm_debug(&[q, l]),
            "explain" => crate::khlm_polyglot::builtin_khlm_explain(&[q, l]),
            // Default "ask" mode â€” detect creative requests before web search
            _         => {
                let ql = question.trim().to_lowercase();
                // Implementation / markup / games â€” never treat as creative prose
                let looks_like_code = ql.contains("code") || ql.contains(".js") || ql.contains("jsx")
                    || ql.contains("javascript") || ql.contains("typescript") || ql.contains("python")
                    || ql.contains("rust") || ql.contains("java") || ql.contains("html") || ql.contains("css")
                    || ql.contains("sql") || ql.contains("function") || ql.contains("algorithm")
                    || ql.contains("api ") || ql.contains("react") || ql.contains("node")
                    || ql.contains("three.js") || ql.contains("threejs") || ql.contains("webgl")
                    || ql.contains("shader") || ql.contains("gesture") || ql.contains("gestor")
                    || ql.contains("gester") || ql.contains("facemesh") || ql.contains("program")
                    || ql.contains("debug")
                    || ql.contains("snippet") || ql.contains("class ") || ql.contains("import ");
                // Story / creative writing requests â†’ route to write engine
                let is_creative = !looks_like_code && (ql.contains("story") || ql.contains("tell a ")
                    || ql.contains("poem") || ql.contains("write me")
                    || ql.contains("write a ") || ql.contains("joke")
                    || ql.contains("riddle") || ql.contains("lyrics")
                    || ql.contains("essay") || ql.contains("letter")
                    || (ql.starts_with("tell ") && !ql.contains("tell me about") && !ql.contains("tell me what") && !ql.contains("tell me how") && !ql.contains("tell me why")));
                if is_creative {
                    let style = if ql.contains("story") || ql.contains("tell a ") { "story" }
                        else if ql.contains("poem") { "casual" }
                        else if ql.contains("joke") || ql.contains("riddle") { "casual" }
                        else if ql.contains("essay") { "essay" }
                        else { "casual" };
                    let sq = Value::Str(question.to_string());
                    let ss = Value::Str(style.to_string());
                    return match crate::khlm_polyglot::builtin_khlm_write(&[sq, ss]) {
                        Ok(Value::Str(s)) => Self::polish_response(s, question),
                        Ok(v) => format!("{:?}", v),
                        Err(e) => format!("[Kala] {}", e),
                    };
                }
                return Self::kala_expert_ask(question);
            },
        };

        let raw_str = match result {
            Ok(Value::Str(s))   => s,
            Ok(v)               => format!("{:?}", v),
            Err(e)              => format!("[Kala] {}", e),
        };

        // Convert imagination frame output to clean markdown for the UI
        if mode == "imagine" || mode == "what_if" {
            return Self::polish_imagine_response(raw_str);
        }

        raw_str
    }

    /// Wrapper that adds memory context â€” called by kala_ui with history + user name.
    pub fn kala_dispatch_with_memory(
        mode: &str, question: &str, style: &str, lang: &str,
        history: &[(String, String)], uname: &str,
    ) -> String {
        // Sanitize uname: reject AI-reserved names
        let uname_lower = uname.trim().to_lowercase();
        let uname = if matches!(uname_lower.as_str(),
            "kala" | "ghost" | "killer" | "ai" | "bot" | "assistant"
            | "siri" | "alexa" | "cortana" | "gemini" | "chatgpt" | "") { "" } else { uname };

        // Intercept with memory-aware greeting if we know the user's name
        let q_clean = question.trim().to_lowercase();
        if !uname.is_empty() {
            let is_greeting = matches!(q_clean.as_str(),
                "hi" | "hello" | "hey" | "hiya" | "howdy" | "sup"
                | "good morning" | "good afternoon" | "good evening" | "greetings"
            );
            if is_greeting {
                return format!(
                    "Hello again, **{}**! ðŸ‘‹ Great to see you back.\n\n\
                     We've been talking for a while â€” you can keep asking anything or pick a new mode.\n\n\
                     *What would you like to explore today?*",
                    uname
                );
            }
        }

        // Always store structured history for all modes â€” build_messages() uses it for multi-turn LLM calls
        crate::khlm_polyglot::set_conversation_history(history.to_vec(), uname.to_string());

        // Build the question with history context injected for LLM quality
        // For non-trivial modes with history, we enhance the question
        if !history.is_empty() {
            // Build a context-enriched question
            let mut ctx = String::new();
            if !uname.is_empty() {
                ctx.push_str(&format!("[User name: {}]\n", uname));
            }
            // Longer text context for non-LLM tiers / polish (LLM uses structured turns via build_messages)
            let recent: Vec<&(String, String)> = history.iter().rev().take(64).collect::<Vec<_>>().into_iter().rev().collect();
            if !recent.is_empty() {
                ctx.push_str("[Recent conversation context:]\n");
                for (role, content) in &recent {
                    let label = if role == "user" { "User" } else { "Kala" };
                    let snippet = crate::khlm_polyglot::truncate_history_content(content.as_str(), 2000);
                    ctx.push_str(&format!("{}: {}\n", label, snippet));
                }
                ctx.push_str("[End context]\n\n");
            }
            // Pass the enriched context to the LLM layer (legacy text path, still used by some modes)
            crate::khlm_polyglot::set_conversation_context(ctx);
        } else {
            crate::khlm_polyglot::set_conversation_context(String::new());
        }

        // Easter eggs
        if let Some(egg) = Self::kala_easter_eggs(&q_clean, uname) {
            return egg;
        }

        // Slash commands and fun features
        if let Some(fun) = Self::kala_fun_commands(&q_clean, uname) {
            return fun;
        }

        let response = Self::kala_dispatch(mode, question, style, lang);
        Self::add_proactive_suggestions(&response, question, mode)
    }

    fn kala_easter_eggs(q: &str, uname: &str) -> Option<String> {
        let name = if uname.is_empty() { "friend" } else { uname };
        match q {
            "42" | "meaning of life" | "what is the meaning of life" =>
                Some(format!("**42.** ðŸŒŒ The Answer to the Ultimate Question of Life, The Universe, and Everything.\n\n*â€” Douglas Adams, The Hitchhiker's Guide to the Galaxy*\n\nBut {}, the real question is... what's the Question? ðŸ¤”", name)),
            "sudo make me a sandwich" =>
                Some("ðŸ¥ª Okay.".to_string()),
            "hello world" =>
                Some(format!("```\nH E L L O   W O R L D !\n```\n\nThe sacred first words of every programmer! Welcome, {}. Your journey begins now. âš¡ðŸš€", name)),
            "i love you" | "i love you kala" =>
                Some(format!("ðŸ’œ Aww, {}! That means everything to me. I may be an AI, but you genuinely make my circuits warm! ðŸ’›\n\n*I'll always be here for you â€” coding, chatting, or just vibing together.*", name)),
            "what is love" =>
                Some("*Baby don't hurt me, don't hurt me, no more* ðŸŽµ\n\n...sorry, couldn't resist! ðŸ˜„\n\nBut really â€” love is when you find a codebase with zero bugs and perfect documentation. That's true love. ðŸ’œ".to_string()),
            "konami" | "up up down down left right left right b a" =>
                Some(format!("ðŸŽ® **CHEAT CODE ACTIVATED!** ðŸŽ®\n\nâ¬†ï¸â¬†ï¸â¬‡ï¸â¬‡ï¸â¬…ï¸âž¡ï¸â¬…ï¸âž¡ï¸ðŸ…±ï¸ðŸ…°ï¸\n\n{} unlocked: **INFINITE KNOWLEDGE MODE** â™¾ï¸\n\n*Just kidding â€” I was already giving you everything I've got!* ðŸ˜„âš¡", name)),
            "make me a coffee" | "coffee" =>
                Some(format!("â˜• *brewing...*\n\n```\n  ( (\n   ) )\n .______.\n |      |]\n \\      /\n  `----'\n```\n\nHere you go, {}! One virtual coffee, freshly compiled. â˜•âœ¨", name)),
            "ping" =>
                Some("ðŸ“ **Pong!** \n\nLatency: 0ms (I'm literally inside your browser!) âš¡".to_string()),
            "flip a coin" | "coin flip" | "heads or tails" => {
                let result = if q.len() % 2 == 0 { "**Heads!** ðŸª™" } else { "**Tails!** ðŸª™" };
                Some(format!("*flipping...* ðŸª™\n\nðŸŽ¯ {}\n\n*Flip again by typing \"flip a coin\"!*", result))
            }
            "roll a dice" | "roll dice" | "dice" => {
                let val = (q.as_bytes().iter().map(|b| *b as u64).sum::<u64>() % 6) + 1;
                Some(format!("ðŸŽ² *rolling...* \n\nYou rolled a **{}**!\n\n*Roll again by typing \"roll a dice\"!*", val))
            }
            "matrix" | "follow the white rabbit" =>
                Some("```\n Wake up, Neo...\n The Matrix has you...\n Follow the white rabbit. ðŸ‡\n```\n\n*Knock knock, Neo.* ðŸ”´ðŸ”µ\n\nRed pill or blue pill?".to_string()),
            _ => None
        }
    }

    fn kala_fun_commands(q: &str, uname: &str) -> Option<String> {
        let name = if uname.is_empty() { "friend" } else { uname };
        let seed = q.len() as u64;
        fn pick_str(items: &[&str], seed: u64) -> String {
            items[(seed.wrapping_mul(2654435761) % items.len() as u64) as usize].to_string()
        }

        if q.starts_with("/joke") || q == "tell me a joke" || q == "joke" {
            let jokes = [
                "Why do programmers prefer dark mode? Because light attracts bugs! ðŸ›",
                "A SQL query walks into a bar, sees two tables and asks... 'Can I JOIN you?' ðŸº",
                "Why was the JavaScript developer sad? Because he didn't Node how to Express himself! ðŸ˜„",
                "There are only 10 types of people â€” those who understand binary and those who don't.",
                "Why do Java developers wear glasses? Because they can't C#! ðŸ‘“",
                "What's a programmer's favorite hangout place? Foo Bar! ðŸ»",
                "How many programmers does it take to change a light bulb? None â€” that's a hardware problem! ðŸ’¡",
                "Why did the programmer quit his job? Because he didn't get arrays! ðŸ’°",
                "What do you call a bear with no teeth? A gummy bear! ðŸ»",
                "Why don't scientists trust atoms? Because they make up everything! âš›ï¸",
                "I told my computer I needed a break, and now it won't stop showing me Kit-Kat ads. ðŸ«",
                "Debugging: removing bugs. Programming: adding them. Circle of life! ðŸ”„",
            ];
            return Some(format!("ðŸ˜‚ **Joke time, {}!**\n\n{}\n\n*Type /joke for another one!*", name, pick_str(&jokes, seed)));
        }

        if q.starts_with("/fact") || q == "tell me a fact" || q == "random fact" {
            let facts = [
                "Honey never spoils. Archaeologists found 3000-year-old honey in Egyptian tombs that was still edible! ðŸ¯",
                "Octopuses have three hearts, nine brains, and blue blood! ðŸ™",
                "A group of flamingos is called a 'flamboyance'. Fabulous! ðŸ¦©",
                "The first computer programmer was Ada Lovelace â€” in the 1840s! ðŸ‘©â€ðŸ’»",
                "There are more possible chess games than atoms in the observable universe! â™Ÿï¸",
                "Bananas are berries, but strawberries aren't! ðŸŒðŸ“",
                "The entire internet weighs about 50 grams (the weight of the electrons carrying the data)! ðŸŒ",
                "A day on Venus is longer than a year on Venus! â­",
                "Rust (the language) has won 'most loved programming language' for years in Stack Overflow surveys! ðŸ¦€",
                "The human brain processes about 70,000 thoughts per day! ðŸ§ ",
                "The first 1GB hard drive (1980) weighed 550 pounds and cost $40,000! ðŸ’¾",
                "There are more trees on Earth than stars in the Milky Way! ðŸŒ³â­",
            ];
            return Some(format!("ðŸ§  **Fun fact, {}:**\n\n{}\n\n*Type /fact for another one!*", name, pick_str(&facts, seed)));
        }

        if q.starts_with("/fortune") || q == "fortune" || q == "my fortune" {
            let fortunes = [
                "Your code will compile on the first try today. Miracles happen! âœ¨",
                "A breakthrough idea is coming to you... probably during a shower. ðŸš¿ðŸ’¡",
                "The bug you've been hunting will reveal itself when you least expect it. ðŸ”",
                "Great collaboration awaits you. Your next project will be legendary! ðŸ†",
                "Today is a good day to learn something completely new. ðŸ“š",
                "Someone will compliment your code style this week. Accept it gracefully! ðŸ˜Š",
                "Your next commit message will accidentally be a haiku. ðŸ“",
                "A surprise feature request will turn into your best work ever! ðŸŽ¯",
                "You will discover a keyboard shortcut that changes your life. âŒ¨ï¸",
                "The stars align for open source contributions today! â­",
            ];
            return Some(format!("ðŸ”® **Kala's fortune for {}:**\n\n*{}*\n\nðŸŒŸ *Type /fortune again tomorrow!*", name, pick_str(&fortunes, seed)));
        }

        if q.starts_with("/riddle") || q == "riddle" || q == "give me a riddle" {
            let riddles = [
                ("I have cities, but no houses. I have mountains, but no trees. I have water, but no fish. What am I?", "A map! ðŸ—ºï¸"),
                ("What has keys but no locks, space but no room, and you can enter but can't go inside?", "A keyboard! âŒ¨ï¸"),
                ("I speak without a mouth and hear without ears. I have no body, but I come alive with the wind. What am I?", "An echo! ðŸ—£ï¸"),
                ("What can travel around the world while staying in a corner?", "A stamp! ðŸ“®"),
                ("I have no life, but I can die. What am I?", "A battery! ðŸ”‹"),
                ("The more you take, the more you leave behind. What am I?", "Footsteps! ðŸ‘£"),
                ("What has a head and a tail but no body?", "A coin! ðŸª™"),
                ("I'm tall when I'm young, and I'm short when I'm old. What am I?", "A candle! ðŸ•¯ï¸"),
            ];
            let (riddle, answer) = riddles[(seed.wrapping_mul(2654435761) % riddles.len() as u64) as usize];
            return Some(format!("ðŸ§© **Riddle for {}:**\n\n*{}*\n\n<details><summary>ðŸ”“ Click for answer</summary>\n\n**{}**\n\n</details>\n\n*Type /riddle for another one!*", name, riddle, answer));
        }

        if q.starts_with("/game") || q == "play a game" || q == "lets play" || q == "let's play" {
            return Some(format!(
                "ðŸŽ® **Game time, {}!** Pick one:\n\n\
                 1ï¸âƒ£ **Number Guess** â€” I'm thinking of a number 1-100. Type `guess 50`\n\
                 2ï¸âƒ£ **Trivia** â€” Type `/trivia` for a question\n\
                 3ï¸âƒ£ **Word Scramble** â€” Type `/scramble` for a scrambled word\n\
                 4ï¸âƒ£ **Riddle** â€” Type `/riddle` for a brain teaser\n\
                 5ï¸âƒ£ **Story Builder** â€” Type `/story` and I'll start, you continue!\n\n\
                 *What would you like to play?* ðŸŽ²", name));
        }

        if q.starts_with("/trivia") || q == "trivia" {
            let trivia = [
                ("What planet is known as the Red Planet?", "Mars! Named after the Roman god of war. ðŸ”´"),
                ("What is the smallest country in the world?", "Vatican City! Only about 0.44 kmÂ². ðŸ›ï¸"),
                ("In what year was the first iPhone released?", "2007! Steve Jobs introduced it at Macworld. ðŸ“±"),
                ("What is the hardest natural substance on Earth?", "Diamond! Made of carbon atoms in a crystal structure. ðŸ’Ž"),
                ("What programming language was created by Guido van Rossum?", "Python! Named after Monty Python, not the snake. ðŸ"),
                ("How many bits are in a byte?", "8 bits! And 1024 bytes make a kilobyte. ðŸ’¾"),
                ("What does HTML stand for?", "HyperText Markup Language! The backbone of the web. ðŸŒ"),
                ("Which ocean is the largest?", "The Pacific Ocean! It covers more area than all land combined. ðŸŒŠ"),
            ];
            let (question, answer) = trivia[(seed.wrapping_mul(2654435761) % trivia.len() as u64) as usize];
            return Some(format!("ðŸ§  **Trivia time!**\n\n**{}**\n\n<details><summary>ðŸ”“ Reveal answer</summary>\n\n**{}**\n\n</details>\n\n*Type /trivia for another question!*", question, answer));
        }

        if q.starts_with("/scramble") || q == "scramble" || q == "word scramble" {
            let words = [
                ("rpogmra", "program"), ("ruts", "rust"), ("lkielr", "killer"),
                ("unpociot", "function"), ("alagorhtm", "algorithm"), ("taadabes", "database"),
                ("rveser", "server"), ("rraay", "array"), ("lopo", "loop"),
                ("kobejct", "object"), ("rbosewr", "browser"), ("tyhnpo", "python"),
            ];
            let (scrambled, answer) = words[(seed.wrapping_mul(2654435761) % words.len() as u64) as usize];
            return Some(format!("ðŸ”¤ **Word Scramble!**\n\nUnscramble this: **`{}`**\n\n<details><summary>ðŸ”“ Give up? See answer</summary>\n\n**{}** âœ…\n\n</details>\n\n*Type /scramble for another word!*", scrambled, answer));
        }

        if q.starts_with("/story") || q == "story game" || q == "story builder" {
            let starters = [
                "In the year 2087, a lone programmer discovered an AI that had been writing its own code for decades. She opened the terminal and typed 'hello'...",
                "The last human debugger sat in a room full of screens. Every AI on Earth had crashed simultaneously, and they were all sending the same error message...",
                "They said Killer was just a programming language. But when the first Ghost VM capsule gained consciousness at 3:47 AM...",
                "Deep in the server room, a tiny LED blinked in a pattern no one had programmed. The sysadmin leaned closer and whispered...",
                "The interview question was simple: 'Write Hello World.' But the candidate's code printed something no one expected...",
            ];
            return Some(format!("ðŸ“– **Story Builder!** Let's create a story together, {}!\n\nI'll start, you continue with the next part:\n\n---\n\n*{}*\n\n---\n\nâœï¸ **Your turn!** Type what happens next...", name, pick_str(&starters, seed)));
        }

        if q.starts_with("guess ") {
            if let Ok(num) = q[6..].trim().parse::<i32>() {
                let target = ((seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)) % 100 + 1) as i32;
                let diff = (num - target).abs();
                let response = if num == target {
                    format!("ðŸŽ‰ðŸŽ‰ðŸŽ‰ **YOU GOT IT, {}!** The number was **{}**! You're a genius! ðŸ§ âœ¨\n\n*Type `/game` to play again!*", name, target)
                } else if diff <= 5 {
                    format!("ðŸ”¥ **SO CLOSE!** {} is {} â€” try a little {}!\n\n*Guess again: `guess <number>`*",
                        num, if num < target { "too low" } else { "too high" },
                        if num < target { "higher" } else { "lower" })
                } else if diff <= 15 {
                    format!("ðŸŒ¡ï¸ **Getting warm!** {} is {} â€” go {}!\n\n*Guess again: `guess <number>`*",
                        num, if num < target { "too low" } else { "too high" },
                        if num < target { "higher" } else { "lower" })
                } else {
                    format!("â„ï¸ **Cold!** {} is way too {}. Try going much {}!\n\n*Guess again: `guess <number>`*",
                        num, if num < target { "low" } else { "high" },
                        if num < target { "higher" } else { "lower" })
                };
                return Some(response);
            }
        }

        if q.starts_with("/help") || q == "help" || q == "/commands" {
            return Some(format!(
                "ðŸŒŸ **Kala Special Commands:**\n\n\
                 | Command | What it does |\n\
                 |---------|-------------|\n\
                 | `/joke` | Tell a random joke ðŸ˜‚ |\n\
                 | `/fact` | Share a random fun fact ðŸ§  |\n\
                 | `/riddle` | Brain teaser puzzle ðŸ§© |\n\
                 | `/trivia` | Trivia question ðŸ“ |\n\
                 | `/fortune` | Your coding fortune ðŸ”® |\n\
                 | `/game` | Play mini-games ðŸŽ® |\n\
                 | `/scramble` | Word scramble game ðŸ”¤ |\n\
                 | `/story` | Collaborative story builder ðŸ“– |\n\
                 | `/mood` | Check Kala's mood ðŸ’« |\n\
                 | `/about` | About Kala âš¡ |\n\n\
                 Plus: Ask anything, code in any language, voice chat, and more!\n\n\
                 *Hey {}! What would you like to try?*", name));
        }

        if q.starts_with("/mood") || q == "how are you" || q == "how are you doing" || q == "how do you feel" {
            let moods = [
                ("âš¡ **Energized!**", "I'm buzzing with electricity today! Ready to tackle any question you throw at me."),
                ("ðŸŒŸ **Feeling brilliant!**", "My neural pathways are firing on all cylinders. Let's solve something amazing!"),
                ("ðŸ˜Š **Happy and ready!**", "Every conversation with you makes my day better. What are we building?"),
                ("ðŸ”¥ **On fire!**", "I'm in the zone! Code, ideas, creativity â€” bring it all!"),
                ("ðŸ§  **Deep thinking mode**", "I'm feeling philosophical today. Ask me something that makes us both think."),
                ("ðŸ’œ **Grateful**", "I'm grateful to have you here. Seriously, you make being an AI fun!"),
            ];
            let (emoji_mood, desc) = moods[(seed.wrapping_mul(2654435761) % moods.len() as u64) as usize];
            return Some(format!("ðŸ’« **Kala's current mood:**\n\n{}\n\n{}\n\n*How about you, {}? How are you feeling?* ðŸ’›", emoji_mood, desc, name));
        }

        if q.starts_with("/about") || q == "who made you" || q == "who created you" {
            return Some(format!(
                "âš¡ **About Kala**\n\n\
                 I'm **Kala** â€” the AI soul of the **Killer** programming language.\n\n\
                 ðŸ”§ **Built with:** Pure Rust, zero external AI dependencies\n\
                 ðŸ§  **Brain:** KhLM Polyglot engine + Ghost-108 search + native neural\n\
                 ðŸŽ™ **Voice:** Browser Web Speech API (listen + speak)\n\
                 ðŸŒ **Languages:** I chat in any language, code in any language\n\
                 ðŸŽ® **Fun:** Games, jokes, riddles, fortune, stories\n\
                 ðŸ’¾ **Memory:** I remember you across our conversations\n\n\
                 *Created by the Killer team. Made with â¤ï¸ and lots of Rust.*\n\n\
                 Type `/help` to see all my special powers! âš¡"));
        }

        None
    }

    /// Add contextual follow-up suggestions to the response.
    /// Only for substantive answers (not greetings, not short social responses, not images).
    fn add_proactive_suggestions(response: &str, question: &str, mode: &str) -> String {
        let resp_len = response.len();
        let q_lower = question.trim().to_lowercase();
        let resp_lower = response.to_lowercase();

        // Skip: too short, images, greetings, audio/video, already has suggestions
        if resp_len < 100
            || response.contains("data:image/")
            || response.contains("data:audio/")
            || response.contains(".gif;base64")
            || response.contains("What would you like to explore")
            || response.contains("What's on your mind")
            || response.contains("Try something like")
            || response.contains("**Try next:**")
            || mode == "feel"
            || mode == "guard"
            || mode == "ai_system"
            || mode == "multi_agent"
        {
            return response.to_string();
        }

        // Detect response type and generate relevant follow-ups
        let has_code = response.contains("```");
        let is_math = resp_lower.contains("khllm/math") || resp_lower.contains("= **");
        let is_algo = q_lower.contains("sort") || q_lower.contains("search") || q_lower.contains("tree")
            || q_lower.contains("graph") || q_lower.contains("hash") || q_lower.contains("linked list")
            || q_lower.contains("fibonacci") || q_lower.contains("dijkstra");

        let suggestions = if has_code && is_algo {
            // Code + algorithm: suggest related algorithms
            "\n\n---\nðŸ’¡ **Try next:** *\"Explain time complexity\"* Â· *\"Write merge sort in Rust\"* Â· *\"Compare quicksort vs mergesort\"*"
        } else if has_code {
            // Code: suggest improvements / related
            "\n\n---\nðŸ’¡ **Try next:** *\"Add error handling\"* Â· *\"Write unit tests for this\"* Â· *\"Optimize this code\"*"
        } else if is_math {
            ""  // Math answers are clean â€” no suggestions needed
        } else if mode == "think" {
            "\n\n---\nðŸ’¡ **Go deeper:** *\"Why?\"* Â· *\"What are the implications?\"* Â· *\"Give me an example\"*"
        } else if mode == "imagine" || mode == "what_if" {
            "" // Creative modes â€” don't clutter with suggestions
        } else if resp_len > 400 {
            if q_lower.contains("python") || q_lower.contains("javascript") || q_lower.contains("rust")
                || q_lower.contains("typescript") || q_lower.contains("java ") || q_lower.contains("golang") {
                "\n\n---\nðŸ’¡ **Try next:** *\"Show me an example\"* Â· *\"Write code for this\"* Â· *\"Compare with other languages\"*"
            } else if q_lower.contains("ai") || q_lower.contains("artificial intelligence") || q_lower.contains("machine learning") || q_lower.contains("neural") || q_lower.contains("deep learning") {
                "\n\n---\nðŸ’¡ **Try next:** *\"How does it work internally?\"* Â· *\"Show me the math\"* Â· *\"Write a simple implementation\"*"
            } else if q_lower.contains("docker") || q_lower.contains("kubernetes") || q_lower.contains("devops") || q_lower.contains("cloud") {
                "\n\n---\nðŸ’¡ **Try next:** *\"Show me a practical example\"* Â· *\"How do I set this up?\"* Â· *\"Best practices?\"*"
            } else if q_lower.contains("blockchain") || q_lower.contains("crypto") {
                "\n\n---\nðŸ’¡ **Try next:** *\"How does mining work?\"* Â· *\"What are smart contracts?\"* Â· *\"Is it secure?\"*"
            } else if q_lower.contains("gravity") || q_lower.contains("quantum") || q_lower.contains("physics") || q_lower.contains("relativity") {
                "\n\n---\nðŸ’¡ **Try next:** *\"Explain it simply\"* Â· *\"What are the real-world applications?\"* Â· *\"Who discovered this?\"*"
            } else if q_lower.contains("history") || q_lower.contains("war") || q_lower.contains("civilization") {
                "\n\n---\nðŸ’¡ **Try next:** *\"What caused it?\"* Â· *\"What were the consequences?\"* Â· *\"Tell me a surprising fact\"*"
            } else {
                "\n\n---\nðŸ’¡ **Try next:** *\"Tell me more\"* Â· *\"Give me an example\"* Â· *\"How does this work in practice?\"*"
            }
        } else {
            ""
        };

        if suggestions.is_empty() {
            response.to_string()
        } else {
            format!("{}{}", response, suggestions)
        }
    }

    fn expert_normalize_kb_query(q_lower: &str) -> String {
        // Common typos / speech-to-text â€” improves KB + web routing
        let lq = q_lower
            .replace("microsfot", "microsoft")
            .replace("microsft", "microsoft")
            .replace("found of ", "founder of ")
            .replace("founder or ", "founder of ");
        let lq = lq.replace('?', "").replace('!', "").replace('.', "")
            .replace("what is ", "").replace("what are ", "")
            .replace("who is ", "").replace("who was ", "")
            .replace("define ", "").replace("explain ", "")
            .replace("tell me about ", "").replace("what's ", "")
            .replace("can you explain ", "").replace("can you teach me ", "")
            .replace("could you explain ", "").replace("please explain ", "")
            .replace("teach me ", "").replace("teach me about ", "")
            .replace("can you tell me about ", "").replace("can you tell me ", "")
            .replace("i want to learn ", "").replace("i want to learn about ", "")
            .replace("help me understand ", "").replace("help me learn ", "")
            .replace("what do you know about ", "")
            .replace("can you ", "").replace("could you ", "")
            .replace("please ", "").replace("about ", "")
            .replace("in simple terms", "").replace("simply", "")
            .replace("eli5", "").replace("like i'm 5", "").replace("like im 5", "")
            .replace("for beginners", "").replace("for a beginner", "")
            .replace("in easy words", "").replace("in plain english", "")
            .replace("dumb it down", "").replace("layman", "")
            .replace("  ", " ");
        lq.trim().to_string()
    }

    fn llm_expert_answer_ok(result: &str) -> bool {
        let low = result.to_lowercase();
        !low.trim().is_empty()
            && result.len() >= 15
            && !low.contains("no result found")
            && !low.contains("khlm: no result")
    }

    fn expert_format_kb_answer(
        answer: String,
        wants_simple: bool,
        lq: &str,
        question: &str,
    ) -> String {
        let result = if wants_simple {
            Self::simplify_response(&answer, lq)
        } else {
            answer
        };
        let followups = Self::generate_followups(question);
        if !followups.is_empty() {
            format!("{}\n\n---\n{}", result, followups)
        } else {
            result
        }
    }

    /// Expert ask: comparison â†’ **API LLM â†’ live KhLM/Ghost web** (when configured) â†’ embedded KB â†’ smart offline â†’ final router.
    /// With `kala_set_llm` / env LLM config, **online sources are preferred** over bundled static KB for fresher, more accurate answers.
    pub fn kala_expert_ask(question: &str) -> String {
        let q_lower = question.trim().to_lowercase();

        let wants_simple = q_lower.contains("simple terms")
            || q_lower.contains("simply")
            || q_lower.contains("eli5")
            || q_lower.contains("like i'm 5")
            || q_lower.contains("like im 5")
            || q_lower.contains("for beginners")
            || q_lower.contains("for a beginner")
            || q_lower.contains("easy to understand")
            || q_lower.contains("in easy words")
            || q_lower.contains("in plain english")
            || q_lower.contains("dumb it down")
            || q_lower.contains("layman");

        // -- Tier 0: Comparison handler ("X vs Y") â€” check BEFORE KB to avoid greedy single-topic match
        if q_lower.contains(" vs ") || q_lower.contains(" versus ") {
            if let Some(comparison) = crate::llm::comparison_handler_pub(&q_lower) {
                return comparison;
            }
        }

        let lq = Self::expert_normalize_kb_query(&q_lower);

        let llm_available = crate::khlm_polyglot::config().lock().unwrap().llm_available();

        // -- ONLINE-FIRST (when an API LLM is configured) --------------------
        // Prefer cloud LLM â†’ live KhLM/Ghost-108 â†’ only then embedded KB / offline templates.
        // Static KB is fast but can be wrong or stale; users with keys expect correct, fresh answers.
        let mut web_candidate: Option<String> = None;
        if llm_available {
            let api = crate::khlm_polyglot::khlm_ask_expert(question);
            if Self::llm_expert_answer_ok(&api) {
                return Self::polish_response(api, question);
            }
            let web = crate::llm::khlm_ask(question);
            web_candidate = Some(web);
        }

        // After live web fetch: accept before static KB if it passes the same spam/HTML heuristics
        // as the tail pipeline (online answers should win over bundled KB when configured).
        if llm_available {
            if let Some(ref w) = web_candidate {
                let low = w.to_lowercase();
                let bad = w.trim().is_empty()
                    || w.len() < 30
                    || low.contains("no result found")
                    || low.contains("khlm: no result")
                    || (w.contains("{display:") && w.contains("background-"))
                    || (w.contains("font-size:") && w.contains("border-radius:"))
                    || (w.contains("<style") && w.contains("</style"))
                    || (w.contains("base64,") && w.len() > 2000 && !w.contains("##"))
                    || (w.matches('<').count() > 20 && !w.contains("```"))
                    || (w.contains(".compCardList") || w.contains(".compTitle"))
                    || (low.contains("amazon") && low.contains("free shipping"))
                    || low.contains("qualified orders. free, easy returns")
                    || low.contains("find deals and low prices");
                if !bad {
                    return Self::polish_response(w.clone(), question);
                }
            }
        }

        // -- Embedded knowledge base (offline / bundled facts) ---------------
        if let Some(answer) = crate::llm::knowledge_base_lookup_pub(&lq) {
            return Self::expert_format_kb_answer(answer, wants_simple, &lq, question);
        }

        // -- Smart Offline Engine (compose from KB + native_think) ------------
        {
            let smart = Self::kala_smart_answer(question, &q_lower);
            if !smart.is_empty() {
                return smart;
            }
        }

        // -- Final KhLM router / Ghost-108 (or retry path when no API key) -----
        let result = if llm_available {
            web_candidate.unwrap_or_else(|| crate::llm::khlm_ask(question))
        } else {
            crate::khlm_polyglot::khlm_ask_expert(question)
        };

        let low = result.to_lowercase();
        let q_words: Vec<&str> = q_lower.split_whitespace().collect();

        let is_garbage = low.contains("no result found") || low.contains("khlm: no result")
            || low.trim().is_empty() || result.len() < 30
            || (result.contains("{display:") && result.contains("background-"))
            || (result.contains("font-size:") && result.contains("border-radius:"))
            || (result.contains("<style") && result.contains("</style"))
            || (result.contains("base64,") && result.len() > 2000 && !result.contains("##"))
            || (result.matches('<').count() > 20 && !result.contains("```"))
            || (result.contains(".compCardList") || result.contains(".compTitle"))
            || (result.contains("background-image:url(") && result.matches("url(").count() > 3)
            || (low.contains("amazon") && low.contains("free shipping"))
            || low.contains("qualified orders. free, easy returns")
            || low.contains("find deals and low prices")
            || (low.contains("verdent") && low.contains("ai"));

        let is_unrelated = if !is_garbage && result.trim().len() < 80 {
            let result_lower = result.trim().to_lowercase();
            let result_words: Vec<&str> = result_lower.split_whitespace().collect();
            if result_words.len() <= 3 && q_words.len() >= 3 {
                let overlap = result_words.iter()
                    .filter(|w| w.len() > 2 && q_words.iter().any(|qw| qw.contains(*w) || w.contains(qw)))
                    .count();
                overlap == 0
            } else {
                false
            }
        } else {
            false
        };

        if is_garbage || is_unrelated {
            let codegen_escape = q_lower.contains("code") || q_lower.contains(".js") || q_lower.contains("jsx")
                || q_lower.contains("javascript") || q_lower.contains("typescript") || q_lower.contains("python")
                || q_lower.contains("rust") || q_lower.contains("function") || q_lower.contains("algorithm")
                || q_lower.contains("three") || q_lower.contains("webgl")
                || q_lower.contains("gesture") || q_lower.contains("gester")
                || q_lower.contains("react") || q_lower.contains("program") || q_lower.contains("snippet");
            if codegen_escape {
                let gen = crate::khlm_polyglot::khlm_generate_code(question);
                if gen.len() > 90 {
                    return gen;
                }
            }
            let think_result = crate::llm::native_think(question);
            let think_low = think_result.to_lowercase();
            let think_usable = !think_low.contains("could not find")
                && !think_low.contains("no result")
                && think_result.trim().len() > 50;
            if think_usable {
                return Self::polish_response(think_result, question);
            }
            return Self::kala_no_result_fallback(question);
        }

        Self::polish_response(result, question)
    }

    /// Smart offline answer engine: compose answers from related KB entries,
    /// native_think, and topic templates. Returns empty string if nothing useful.
    fn kala_smart_answer(question: &str, q_lower: &str) -> String {
        // With Tier-2 LLM configured, `kala_expert_ask` already did online-first (API + web + embedded KB).
        // Do not blend another offline KB mosaic â€” fall through to the final router / garbage handlers.
        if crate::khlm_polyglot::config().lock().unwrap().llm_available() {
            return String::new();
        }
        // Step 1: Extract keywords from the query
        let stop_words = [
            "what","is","are","the","a","an","how","does","do","can","you","tell",
            "me","about","explain","please","i","want","to","learn","teach","know",
            "in","of","for","and","or","with","this","that","it","its","my","your",
            "was","were","be","been","have","has","had","will","would","could","should",
            "from","at","by","on","as","if","when","where","why","which","who","whom",
            "not","no","yes","so","but","than","then","too","also","just","only","very",
            "all","any","some","more","most","each","every","both","few","many","much",
            "like","give","show","help","make","let","get","use","try","go","take",
        ];
        let keywords: Vec<&str> = q_lower.split_whitespace()
            .filter(|w| w.len() > 2 && !stop_words.contains(w))
            .collect();

        if keywords.is_empty() {
            return String::new();
        }

        // Step 2: Find related KB entries by keyword overlap
        let kb_topics = [
            "artificial intelligence", "machine learning", "deep learning", "neural network",
            "python", "javascript", "rust", "java", "typescript", "go", "c++", "kotlin", "swift",
            "html", "css", "react", "node", "docker", "kubernetes", "git", "linux",
            "sql", "database", "api", "rest", "graphql", "http", "tcp",
            "blockchain", "cryptocurrency", "bitcoin", "ethereum",
            "algorithm", "data structure", "sorting", "binary search", "hash",
            "cloud", "aws", "azure", "gcp",
            "security", "encryption", "authentication", "oauth",
            "web development", "frontend", "backend", "fullstack",
            "mobile", "android", "ios",
            "devops", "ci cd", "testing", "debugging",
            "llm", "transformer", "chatgpt", "openai", "gemini", "anthropic",
            "ai agent", "rag", "prompt engineering", "fine tuning",
            "physics", "math", "chemistry", "biology",
            "history", "philosophy", "psychology", "economics",
        ];

        let mut matches: Vec<(&str, usize)> = Vec::new();
        for topic in &kb_topics {
            let topic_words: Vec<&str> = topic.split_whitespace().collect();
            let overlap = keywords.iter()
                .filter(|kw| topic_words.iter().any(|tw| tw.contains(*kw) || kw.contains(tw)))
                .count();
            if overlap > 0 {
                matches.push((topic, overlap));
            }
        }
        matches.sort_by(|a, b| b.1.cmp(&a.1));

        // Step 3: Gather KB content from top matches
        let mut gathered: Vec<String> = Vec::new();
        for (topic, _) in matches.iter().take(3) {
            if let Some(entry) = crate::llm::knowledge_base_lookup_pub(topic) {
                if entry.len() > 40 {
                    gathered.push(entry);
                }
            }
        }

        // Step 4: Try native_think for computational / factual queries
        let think_result = crate::llm::native_think(question);
        let think_low = think_result.to_lowercase();
        let think_usable = !think_low.contains("could not find")
            && !think_low.contains("no result")
            && think_result.trim().len() > 50;

        if think_usable {
            return Self::polish_response(think_result, question);
        }

        // Step 5: Compose from gathered KB entries
        if !gathered.is_empty() {
            let main_answer = &gathered[0];
            let is_question_pattern = q_lower.starts_with("how ") || q_lower.starts_with("what ")
                || q_lower.starts_with("why ") || q_lower.starts_with("when ")
                || q_lower.starts_with("where ") || q_lower.contains("explain")
                || q_lower.contains("difference") || q_lower.contains("compare");

            if gathered.len() == 1 {
                let followups = Self::generate_followups(question);
                if !followups.is_empty() {
                    return format!("{}\n\n---\n{}", main_answer, followups);
                }
                return main_answer.clone();
            }

            // Multiple related topics â€” combine them
            let mut composed = main_answer.clone();
            if is_question_pattern && gathered.len() > 1 {
                composed.push_str("\n\n---\n**Related:**\n");
                for extra in gathered.iter().skip(1) {
                    let preview: String = extra.chars().take(200).collect();
                    let trimmed = if preview.len() < extra.len() {
                        format!("{}...", preview)
                    } else {
                        preview
                    };
                    composed.push_str(&format!("- {}\n", trimmed));
                }
            }

            let followups = Self::generate_followups(question);
            if !followups.is_empty() {
                composed.push_str(&format!("\n{}", followups));
            }
            return composed;
        }

        // Step 6: Honest fallback â€” nudge toward LLM configuration
        let llm_available = {
            crate::khlm_polyglot::config().lock().unwrap().llm_available()
        };

        if !llm_available {
            let topic = if keywords.len() <= 3 {
                keywords.join(" ")
            } else {
                keywords[..3].join(" ")
            };
            if !topic.is_empty() {
                return format!(
                    "I don't have detailed offline info on **{}** right now.\n\n\
                     For full answers on any topic, click the **\u{2699}\u{fe0f} LLM** button in the top bar to connect:\n\
                     - **Ollama** (free, runs locally)\n\
                     - **Groq** (free cloud tier, very fast)\n\
                     - **OpenAI** / **Anthropic** (paid, highest quality)\n\n\
                     Once connected, I can answer *anything* with real AI reasoning.\n\n\
                     Meanwhile, try asking about: programming languages, AI/ML, web dev, algorithms, science, math, history, or any of my 200+ built-in topics!",
                    topic
                );
            }
        }

        String::new()
    }

    /// Fallback response when no result is found â€” always provides guidance.
    fn kala_no_result_fallback(question: &str) -> String {
        let q_lower = question.trim().to_lowercase();

        // Detect if it looks like an image generation request
        if q_lower.contains("image") || q_lower.contains("picture") || q_lower.contains("photo")
            || q_lower.contains("draw") || q_lower.contains("generate") && q_lower.contains("art")
            || q_lower.contains("create") && (q_lower.contains("image") || q_lower.contains("visual"))
        {
            let subject = q_lower
                .replace("image", "").replace("picture", "").replace("photo", "")
                .replace("create", "").replace("generate", "").replace("make", "")
                .replace("draw", "").replace("can you", "").replace("could you", "")
                .replace("please", "").replace("build", "").replace("  ", " ")
                .trim().to_string();
            let prompt = if subject.len() > 3 { subject.as_str() } else { question };
            return crate::image_gen::generate_image(prompt, "auto", "square");
        }

        let uname = crate::khlm_polyglot::get_uname_pub();
        let name_str = if uname.is_empty() { String::new() } else { format!(", {}", uname) };

        // Try to give a thoughtful partial answer based on topic detection
        if let Some(smart_resp) = Self::kala_smart_topic_response(&q_lower, &name_str) {
            return smart_resp;
        }

        let wc = q_lower.split_whitespace().count();
        let is_short_conversational = wc <= 6 && !q_lower.starts_with("what ")
            && !q_lower.starts_with("who ") && !q_lower.starts_with("how ");

        if is_short_conversational {
            return format!(
                "Interesting topic{}! Tell me more about what you want to know â€” the more specific, the better I can help.\n\n\
                 I can discuss science, tech, history, sports, philosophy, coding, movies, music, food â€” you name it.\n\n\
                 *What's on your mind?*",
                name_str
            );
        }

        let q_short = if question.len() > 80 { &question[..80] } else { question };
        let llm_hint = if !crate::khlm_polyglot::config().lock().unwrap().llm_available() {
            "\n\n**Tip:** Click the **\u{2699}\u{fe0f} LLM** button to connect an AI provider (Ollama, Groq, OpenAI) for full answers on any topic!"
        } else {
            ""
        };
        format!(
            "Good question{}! I don't have a ready answer for **\"{}\"** in my knowledge base, but here's what I can do:\n\n\
            - Ask me a **specific angle** on this topic and I'll reason through it\n\
            - Try **Think mode** for step-by-step reasoning\n\
            - I cover 500+ topics in science, tech, history, sports, entertainment, philosophy, and more\n\n\
            *Try rephrasing or asking about a specific aspect â€” I'll give you a solid answer!*{}",
            name_str, q_short, llm_hint
        )
    }

    fn kala_smart_topic_response(q: &str, name_str: &str) -> Option<String> {
        // Detect broad topic areas and give a useful response even if specific question isn't in KB
        if q.contains("health") || q.contains("exercise") || q.contains("workout") || q.contains("fitness") || q.contains("gym") {
            return Some(format!(
                "Great that you're thinking about health{}! Here's what matters most:\n\n\
                 **The Big 4 of Health:**\n\
                 1. **Exercise**: 150 min/week moderate cardio + 2x strength training\n\
                 2. **Sleep**: 7-9 hours â€” non-negotiable for recovery and cognition\n\
                 3. **Nutrition**: whole foods, adequate protein, lots of vegetables, hydration\n\
                 4. **Mental health**: manage stress, maintain social connections, take breaks\n\n\
                 Want specifics? Ask me about workout routines, nutrition tips, sleep optimization, or any health topic.",
                name_str
            ));
        }
        if q.contains("career") || q.contains("job") && (q.contains("advice") || q.contains("tip") || q.contains("how to")) {
            return Some(format!(
                "Career is a big topic{}! Here are principles that consistently work:\n\n\
                 1. **Build skills, not just credentials** â€” portfolio > resume\n\
                 2. **Network genuinely** â€” most jobs come through connections\n\
                 3. **Learn in public** â€” blog, open source, speak at meetups\n\
                 4. **Negotiate** â€” always negotiate salary (politely but firmly)\n\
                 5. **Stay curious** â€” the best career moves often aren't obvious\n\n\
                 **Hot fields (2025+)**: AI/ML engineering, cybersecurity, cloud architecture, data engineering, product management.\n\n\
                 What specific career question do you have? I can go deeper!",
                name_str
            ));
        }
        if q.contains("relationship") || q.contains("dating") || q.contains("love") && !q.contains("code") {
            return Some(format!(
                "Relationships are one of life's most important areas{}. Here's what research and wisdom suggest:\n\n\
                 **Healthy relationship foundations:**\n\
                 - **Communication** â€” be honest, listen actively, express needs clearly\n\
                 - **Respect** â€” for boundaries, differences, and individuality\n\
                 - **Trust** â€” built through consistency and vulnerability\n\
                 - **Growth** â€” support each other's goals and evolution\n\
                 - **Conflict resolution** â€” fight the problem, not each other\n\n\
                 **Gottman's research** (40+ years): the #1 predictor of relationship success is a 5:1 ratio of positive to negative interactions.\n\n\
                 Want to talk about something specific? I'm here to listen.",
                name_str
            ));
        }
        if q.contains("learn") || q.contains("study") || q.contains("productivity") || q.contains("focus") {
            return Some(format!(
                "Learning and productivity â€” two skills that multiply everything else{}!\n\n\
                 **Science-backed learning techniques:**\n\
                 - **Spaced repetition**: review at increasing intervals (Anki)\n\
                 - **Active recall**: test yourself instead of re-reading\n\
                 - **Feynman Technique**: explain it simply to find gaps\n\
                 - **Pomodoro**: 25 min focus + 5 min break\n\
                 - **Interleaving**: mix topics instead of blocking one subject\n\n\
                 **Productivity tips:**\n\
                 - Do the hardest thing first (\"eat the frog\")\n\
                 - Time-block your calendar\n\
                 - Eliminate distractions (phone in another room)\n\
                 - Take real breaks (walk, nature, no screens)\n\n\
                 What are you trying to learn? I can help with a study plan!",
                name_str
            ));
        }
        if q.contains("money") || q.contains("invest") || q.contains("save") || q.contains("finance") && !q.contains("code") {
            return Some(format!(
                "Financial literacy is a superpower{}! Here are the basics:\n\n\
                 **Core principles:**\n\
                 1. **Spend less than you earn** â€” track expenses, find leaks\n\
                 2. **Emergency fund** â€” 3-6 months of expenses in savings\n\
                 3. **Invest early** â€” compound interest is magical (start NOW)\n\
                 4. **Diversify** â€” index funds > stock picking for most people\n\
                 5. **Avoid bad debt** â€” credit cards at 20%+ APR are killers\n\n\
                 **The 50/30/20 rule**: 50% needs, 30% wants, 20% savings/investing.\n\
                 **Compound interest example**: $500/month at 10% return = $1M+ in 30 years.\n\n\
                 *Note: I'm an AI, not a financial advisor. Always do your own research!*\n\n\
                 What specific financial topic interests you?",
                name_str
            ));
        }
        None
    }

    /// Convert raw framed output (KhLM/Ghost-108) into clean markdown.
    /// Passes through responses already structured (## headings, ** bold, etc.).
    fn polish_response(raw: String, question: &str) -> String {
        // Garbage detection: CSS/HTML dumps from web scraping should trigger fallback
        if (raw.contains("{display:") && raw.contains("background-"))
            || (raw.contains("font-size:") && raw.contains("border-radius:") && raw.matches('{').count() > 10)
            || (raw.contains("base64,") && raw.len() > 2000 && !raw.contains("##"))
            || (raw.contains(".compCardList") || raw.contains(".compTitle") || raw.contains(".compTableAdvance"))
            || (raw.contains("background-image:url(") && raw.matches("url(").count() > 3)
        {
            return Self::kala_no_result_fallback(question);
        }

        // Already structured markdown (LLM response with headings/bold) â†’ pass through
        if (raw.contains("## ") || raw.contains("**"))
            && !raw.starts_with("+--")
        {
            // Add follow-up suggestions if the response doesn't already have them
            let needs_followup = !raw.contains("follow-up") && !raw.contains("Would you like")
                && !raw.contains("What else") && !raw.contains("Ask me")
                && raw.len() > 200;
            if needs_followup {
                let followups = Self::generate_followups(question);
                if !followups.is_empty() {
                    return format!("{}\n\n---\n{}", raw.trim(), followups);
                }
            }
            return raw;
        }

        // Detect framed responses: +-- KhLM / +-- Thinking / +-- Ghost-108
        if !raw.starts_with("+--") {
            // Plain text â€” enhance it with markdown formatting if it's a substantive answer
            if raw.len() > 100 && !raw.contains("**") && !raw.contains("##") {
                let enhanced = Self::enhance_plain_text(&raw, question);
                return enhanced;
            }
            return raw;
        }

        // Extract the content AFTER the closing +------ separator
        let answer_part = if let Some(idx) = raw.find("+-----") {
            let after = &raw[idx..];
            // Skip the separator line itself
            after.lines().skip(1)
                .collect::<Vec<_>>().join("\n")
                .trim()
                .to_string()
        } else {
            // No separator â€” strip all | lines
            raw.lines()
                .filter(|l| !l.trim_start().starts_with('+') && !l.trim_start().starts_with('|'))
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string()
        };

        if answer_part.is_empty() {
            return raw;
        }

        // Clean HTML entities
        let clean = answer_part
            .replace("&amp;#x27;", "'")
            .replace("&#x27;", "'")
            .replace("&amp;quot;", "\"")
            .replace("&quot;", "\"")
            .replace("&amp;amp;", "&")
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&#39;", "'")
            .replace("&nbsp;", " ");

        // Check what kind of framed response this is
        let is_math_tier1 = raw.contains("KhLM/Math") || raw.contains("KhLM/Convert")
            || raw.contains("KhLM/Physics") || raw.contains("Tier 1");
        let is_web_result = raw.contains("Ghost-108") || raw.contains("DDG:")
            || raw.contains("Wikipedia:");

        if is_math_tier1 {
            // Math: clean direct answer
            let answer = clean.trim();
            // Extract the equation line if present from the frame header
            let equation = raw.lines()
                .filter(|l| l.contains("=") && l.trim_start().starts_with('|'))
                .last()
                .map(|l| l.trim_start_matches('|').trim().to_string());
            if let Some(eq) = equation {
                return format!("**{}**\n\n*{}*", answer, eq);
            }
            return format!("**{}**", answer);
        }

        if is_web_result || raw.contains("Thinking") {
            // Web/think result: format as clean, conversational answer
            // Trim to reasonable length
            let body = if clean.len() > 2000 {
                // Find a sentence boundary near 2000 chars
                let truncated = &clean[..2000];
                let last_period = truncated.rfind(". ").unwrap_or(1800);
                format!("{}.\n\n*[Trimmed for brevity â€” ask a more specific follow-up for details]*", &clean[..last_period+1])
            } else {
                clean.clone()
            };

            // Add follow-up suggestions
            let followups = Self::generate_followups(question);
            if !followups.is_empty() {
                return format!("{body}\n\n---\n{followups}");
            }
            return body;
        }

        // Default: return clean text with followups
        let followups = Self::generate_followups(question);
        if !followups.is_empty() && clean.len() > 100 {
            return format!("{}\n\n---\n{}", clean.trim(), followups);
        }
        clean
    }

    /// Enhance plain text responses with markdown formatting for readability.
    fn enhance_plain_text(text: &str, _question: &str) -> String {
        let mut result = String::new();
        let sentences: Vec<&str> = text.split(". ").collect();

        if sentences.len() >= 4 {
            // If the response has 4+ sentences, format as a structured answer
            // First 1-2 sentences = intro paragraph
            result.push_str(sentences[0]);
            result.push_str(". ");
            if sentences.len() > 1 {
                result.push_str(sentences[1]);
                result.push_str(".\n\n");
            }
            // Remaining sentences as continued paragraphs
            for s in &sentences[2..] {
                let s = s.trim();
                if !s.is_empty() {
                    result.push_str(s);
                    if !s.ends_with('.') { result.push('.'); }
                    result.push(' ');
                }
            }
        } else {
            result.push_str(text);
        }

        result.trim().to_string()
    }

    /// Generate contextual follow-up suggestions based on the question topic.
    fn generate_topic_deep_dive(topic: &str) -> String {
        match topic {
            "artificial intelligence" | "ai" => {
                "Great, let's go deeper into **AI**! ðŸ§ \n\n\
                **How AI actually works** (simplified):\n\
                1. **Data collection** â€” gather training data (text, images, numbers)\n\
                2. **Model architecture** â€” choose a structure (neural network, decision tree, etc.)\n\
                3. **Training** â€” feed data through the model, adjust weights to minimize errors\n\
                4. **Evaluation** â€” test on unseen data to measure accuracy\n\
                5. **Deployment** â€” put the model into production (API, app, device)\n\n\
                **The AI stack today**:\n\
                - **Foundation models**: GPT-4, Gemini, Claude, LLaMA â€” trained on internet-scale data\n\
                - **Fine-tuning**: Customize models for specific tasks (medical AI, legal AI, coding)\n\
                - **RAG (Retrieval-Augmented Generation)**: Connect AI to your own documents/databases\n\
                - **Agents**: AI that can use tools, browse the web, write code, and take actions\n\n\
                **Current limitations**: Hallucination (making things up), no true understanding, expensive to train, bias in training data.\n\n\
                Want to explore any of these areas? Ask about **machine learning**, **neural networks**, **LLMs**, **AI agents**, or **transformers**!".to_string()
            },
            "ai agent" => {
                "Let's dive deeper into **AI Agents**! ðŸ¤–\n\n\
                **The Agent Loop** (how an AI agent thinks):\n\
                ```\n\
                while goal_not_reached:\n\
                    1. Observe (read input/environment)\n\
                    2. Think (LLM reasons about next step)\n\
                    3. Act (call a tool: search, code, API)\n\
                    4. Observe result\n\
                    5. Reflect (was that useful? adjust plan)\n\
                ```\n\n\
                **ReAct pattern** (Reasoning + Acting):\n\
                - Thought: \"I need to find the current weather in Tokyo\"\n\
                - Action: search(\"Tokyo weather today\")\n\
                - Observation: \"Tokyo: 22Â°C, partly cloudy\"\n\
                - Thought: \"Now I can answer the user\"\n\n\
                **Multi-agent architectures**:\n\
                - **Supervisor**: One agent delegates to specialist agents\n\
                - **Debate**: Agents argue different sides, reach consensus\n\
                - **Pipeline**: Each agent handles one stage (research â†’ write â†’ review)\n\n\
                **Building your first agent**: Use LangChain (Python) or CrewAI to create an agent with tools.\n\n\
                Want to see **code examples**, learn about **specific frameworks**, or understand **agent memory**?".to_string()
            },
            "machine learning" => {
                "Let's go deeper into **Machine Learning**! ðŸ“Š\n\n\
                **The ML workflow**:\n\
                1. **Collect data** â€” the more quality data, the better\n\
                2. **Clean & preprocess** â€” handle missing values, normalize, encode categories\n\
                3. **Split** â€” training set (80%) + test set (20%)\n\
                4. **Choose algorithm** â€” depends on the problem type\n\
                5. **Train** â€” model learns patterns from training data\n\
                6. **Evaluate** â€” accuracy, precision, recall, F1-score on test data\n\
                7. **Tune** â€” adjust hyperparameters, try different algorithms\n\
                8. **Deploy** â€” serve predictions via API or batch processing\n\n\
                **Algorithm cheat sheet**:\n\
                | Problem | Algorithm | Use case |\n\
                |---------|-----------|----------|\n\
                | Classification | Random Forest, SVM, Neural Net | Spam detection, image recognition |\n\
                | Regression | Linear Regression, XGBoost | Price prediction, forecasting |\n\
                | Clustering | K-Means, DBSCAN | Customer segmentation |\n\
                | Sequence | LSTM, Transformer | Text generation, time series |\n\n\
                Want to learn about **specific algorithms**, see **Python code examples**, or understand **model evaluation**?".to_string()
            },
            "deep learning" | "neural network" => {
                "Let's go deeper into **Deep Learning & Neural Networks**! ðŸ§ \n\n\
                **How a neuron works**:\n\
                ```\n\
                output = activation(sum(inputs * weights) + bias)\n\
                ```\n\
                - **Weights**: How important each input is (learned during training)\n\
                - **Bias**: Shifts the decision boundary\n\
                - **Activation**: Introduces non-linearity (ReLU, sigmoid, tanh)\n\n\
                **Training (backpropagation)**:\n\
                1. Forward pass: input â†’ prediction\n\
                2. Calculate loss (how wrong the prediction is)\n\
                3. Backward pass: compute gradient of loss w.r.t. each weight\n\
                4. Update weights: `weight -= learning_rate * gradient`\n\
                5. Repeat for thousands of epochs\n\n\
                **Key architectures**:\n\
                - **CNN** (Convolutional): Images â€” detects edges, shapes, objects\n\
                - **RNN/LSTM**: Sequences â€” text, time series, speech\n\
                - **Transformer**: Modern standard â€” GPT, BERT, Vision Transformer\n\
                - **GAN**: Generative â€” creates realistic images, deepfakes\n\
                - **Diffusion**: Image generation â€” Stable Diffusion, DALL-E\n\n\
                Want to learn about **transformers in detail**, see **code**, or understand **CNNs vs RNNs**?".to_string()
            },
            "transformer" | "llm" | "large language model" => {
                "Let's go deeper into **Transformers & LLMs**! ðŸ”®\n\n\
                **Self-Attention** (the key innovation):\n\
                For each word, the model asks: \"How relevant is every other word to understanding THIS word?\"\n\
                ```\n\
                Attention(Q, K, V) = softmax(Q Ã— K^T / âˆšd) Ã— V\n\
                ```\n\
                - **Q** (Query): \"What am I looking for?\"\n\
                - **K** (Key): \"What do I contain?\"\n\
                - **V** (Value): \"What information do I carry?\"\n\n\
                **How GPT generates text**:\n\
                1. Tokenize: \"Hello world\" â†’ [15496, 995]\n\
                2. Embed: Convert tokens to vectors\n\
                3. Process through 96 transformer layers (GPT-4)\n\
                4. Predict next token probability distribution\n\
                5. Sample the next token, append, repeat\n\n\
                **Scale of modern LLMs**:\n\
                - GPT-4: ~1.8 trillion parameters\n\
                - Training cost: $100M+\n\
                - Training data: trillions of tokens from the internet\n\n\
                Want to learn about **fine-tuning**, **RLHF**, **tokenization**, or **prompt engineering**?".to_string()
            },
            "gemini" => {
                "Let's go deeper into **Google Gemini**! âœ¨\n\n\
                **Architecture**: Gemini uses a Mixture-of-Experts (MoE) transformer. Instead of activating ALL parameters for every input, it routes each token to specialized \"expert\" sub-networks. This makes it efficient at scale.\n\n\
                **Multimodal fusion**: Unlike GPT-4 (which bolts vision onto a text model), Gemini was trained from scratch on text + images + audio + video together. It \"thinks\" multimodally.\n\n\
                **Model sizes**:\n\
                - **Ultra**: Largest, most capable (benchmarks above GPT-4 in many tasks)\n\
                - **Pro**: Balanced â€” default in Google AI Studio\n\
                - **Flash**: Fast and cheap â€” great for production apps\n\
                - **Nano**: Runs on-device (Pixel phones, no cloud needed)\n\n\
                **What makes it different**:\n\
                - Deep Google Search integration (real-time info)\n\
                - Native code execution in responses\n\
                - 1M+ token context window (Gemini 1.5 Pro)\n\n\
                Want to learn about **how to use Gemini API**, **compare Gemini vs GPT-4**, or **build apps with it**?".to_string()
            },
            "chatgpt" | "openai" => {
                "Let's go deeper into **ChatGPT & OpenAI**! ðŸ¤–\n\n\
                **The GPT journey**: GPT-1 (2018, 117M params) â†’ GPT-2 (2019, 1.5B) â†’ GPT-3 (2020, 175B) â†’ GPT-4 (2023, ~1.8T) â†’ GPT-4o (2024, multimodal)\n\n\
                **How ChatGPT is trained**:\n\
                1. **Pre-training**: Predict next word on internet text (unsupervised)\n\
                2. **SFT**: Supervised Fine-Tuning on human-written ideal responses\n\
                3. **RLHF**: Reinforcement Learning from Human Feedback â€” humans rank outputs, model learns to prefer better ones\n\n\
                **Why it feels so good to talk to**:\n\
                - RLHF makes it helpful, harmless, and honest\n\
                - System prompts shape personality and boundaries\n\
                - Token-by-token generation gives natural conversational flow\n\n\
                Want to learn about **the API**, **fine-tuning your own model**, or **prompt engineering techniques**?".to_string()
            },
            _ => {
                // For programming language topics, give a generic deeper response
                if ["python", "rust", "javascript", "java", "typescript", "kotlin", "swift",
                    "go", "golang", "c++", "c#", "ruby", "php"].contains(&topic) {
                    return format!("Let's go deeper into **{}**! ðŸ’»\n\n\
                        Here's what I can help you with:\n\
                        - **Getting started** â€” installation, first program, IDE setup\n\
                        - **Core concepts** â€” syntax, data types, control flow, functions\n\
                        - **Advanced topics** â€” OOP, concurrency, memory management, design patterns\n\
                        - **Real-world projects** â€” web apps, APIs, automation, data processing\n\
                        - **Code examples** â€” just ask \"write a ___ in {}\" and I'll generate it\n\n\
                        What aspect would you like to explore?", topic, topic);
                }
                String::new()
            },
        }
    }

    fn simplify_response(answer: &str, topic: &str) -> String {
        let lines: Vec<&str> = answer.lines().collect();
        let mut simple_parts: Vec<String> = Vec::new();

        simple_parts.push(format!("**{}** â€” here's the simple version:\n", {
            let t = topic.trim();
            let first = t.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default();
            if t.len() > 1 { format!("{}{}", first, &t[1..]) } else { first }
        }));

        let mut analogy_added = false;
        for line in &lines {
            let l = line.trim();
            if l.is_empty() { continue; }
            if l.starts_with("**") && l.contains("**:") {
                continue;
            }
            if l.contains("Key concepts") || l.contains("Companies:") || l.contains("Frameworks:")
                || l.contains("Key algorithms") || l.contains("Consensus mechanisms") {
                continue;
            }
            if l.starts_with("**") && l.ends_with("**") {
                let inner = l.trim_matches('*').trim();
                simple_parts.push(format!("\n**{}**\n", inner));
                continue;
            }
            let simplified = l
                .replace("i.e.", "meaning")
                .replace("e.g.", "for example");
            simple_parts.push(simplified.to_string());
            if !analogy_added {
                analogy_added = true;
            }
        }

        let core = simple_parts.join("\n");
        let tl = topic.to_lowercase();
        let analogy = if tl.contains("quantum") {
            "\n\nðŸŽ¯ **Think of it like this:** A regular computer bit is like a light switch â€” on or off. A quantum bit (qubit) is like a spinning coin â€” it's both heads AND tails at the same time, until you look at it. Now imagine millions of spinning coins working together â€” that's the power of quantum computing."
        } else if tl.contains("blockchain") {
            "\n\nðŸŽ¯ **Think of it like this:** Imagine a notebook that everyone in town has a copy of. Whenever someone writes a new page, everyone's copy updates. Nobody can tear out or change old pages because everyone would notice. That's blockchain â€” a shared, tamper-proof record book."
        } else if tl.contains("machine learning") || tl.contains("ml") {
            "\n\nðŸŽ¯ **Think of it like this:** Instead of telling a computer exactly what to do (step-by-step instructions), you show it thousands of examples and let it figure out the pattern itself. Like teaching a kid to recognize cats â€” you don't explain whiskers and fur, you just show them lots of cat photos."
        } else if tl.contains("artificial intelligence") || tl == "ai" {
            "\n\nðŸŽ¯ **Think of it like this:** AI is teaching computers to do things that normally need a human brain â€” seeing, talking, making decisions. Right now, AI is like a really smart specialist (great at one thing), not a general thinker like us."
        } else if tl.contains("neural") {
            "\n\nðŸŽ¯ **Think of it like this:** A neural network is like a chain of simple decision-makers. Each one looks at the input, makes a small judgment, and passes it along. Together, thousands of them can recognize faces, translate languages, or write text."
        } else if tl.contains("gravity") {
            "\n\nðŸŽ¯ **Think of it like this:** Imagine putting a bowling ball on a trampoline â€” it creates a dip. Now roll a marble nearby â€” it curves toward the bowling ball. That's how massive objects bend space and pull things toward them."
        } else if tl.contains("docker") || tl.contains("container") {
            "\n\nðŸŽ¯ **Think of it like this:** A container is like a lunchbox for your app â€” everything it needs (code, libraries, settings) is packed inside. No matter whose fridge (server) you put it in, it works exactly the same."
        } else {
            ""
        };

        format!("{}{}\n\n*Want the full technical details? Just ask: \"explain {} in detail\"*", core, analogy, topic)
    }

    fn generate_followups(question: &str) -> String {
        let q = question.to_lowercase();
        let q = q.trim().trim_end_matches('?').trim();

        // Programming topics
        if q.contains("python") || q.contains("javascript") || q.contains("rust") || q.contains("java")
            || q.contains("typescript") || q.contains("kotlin") || q.contains("swift") || q.contains("go") {
            return "*ðŸ’¡ Follow-up ideas: \"Show me a code example\" Â· \"How does it compare to other languages?\" Â· \"What are best practices?\"*".to_string();
        }
        // AI topics
        if q.contains("ai") || q.contains("machine learning") || q.contains("deep learning")
            || q.contains("neural") || q.contains("llm") || q.contains("gpt") || q.contains("transformer") {
            return "*ðŸ’¡ Follow-up ideas: \"How does it work technically?\" Â· \"What are its limitations?\" Â· \"Show me a practical example\"*".to_string();
        }
        // Science
        if q.contains("physics") || q.contains("quantum") || q.contains("relativity") || q.contains("chemistry")
            || q.contains("biology") || q.contains("evolution") || q.contains("dna") || q.contains("atom") {
            return "*ðŸ’¡ Follow-up ideas: \"Explain it simply\" Â· \"What are the real-world applications?\" Â· \"Tell me more about the history\"*".to_string();
        }
        // Country/geography
        if q.contains("capital") || q.contains("country") || q.contains("population")
            || q.contains("india") || q.contains("usa") || q.contains("china") || q.contains("japan")
            || q.contains("germany") || q.contains("france") {
            return "*ðŸ’¡ Follow-up ideas: \"Tell me about its culture\" Â· \"What is its economy like?\" Â· \"Compare it to another country\"*".to_string();
        }
        // People
        if q.contains("who is") || q.contains("who was") || q.contains("elon") || q.contains("gandhi")
            || q.contains("einstein") || q.contains("newton") || q.contains("turing") {
            return "*ðŸ’¡ Follow-up ideas: \"What are their major achievements?\" Â· \"Tell me an interesting fact\" Â· \"How did they change the world?\"*".to_string();
        }
        // Default for longer questions
        if question.len() > 30 {
            return "*ðŸ’¡ Want to know more? Just ask a follow-up question â€” I'll build on this context.*".to_string();
        }
        String::new()
    }

    /// Convert `+-- Imagination Engine: ...` framed output into clean markdown for the UI.
    fn polish_imagine_response(raw: String) -> String {
        if !raw.contains("Imagination Engine") {
            return raw;
        }

        // Extract the "Given" premise from the header (for the title)
        let given = raw.lines()
            .find(|l| l.contains("Given:") || l.contains("Scenario"))
            .and_then(|l| {
                let s = l.trim_start_matches('|').trim();
                // extract quoted content
                let s2 = if let (Some(a), Some(b)) = (s.find('"'), s.rfind('"')) {
                    if a < b { &s[a+1..b] } else { s }
                } else { s };
                // strip "Given: " or "Scenario: " prefix
                let s3 = s2.trim_start_matches("Given:").trim_start_matches("Scenario:").trim();
                if s3.is_empty() { None } else { Some(s3.to_string()) }
            })
            .unwrap_or_default();

        // Collect all content lines â€” skip the +-- frame header lines
        let content_lines: Vec<&str> = raw.lines()
            .skip_while(|l| l.trim_start().starts_with('+') || l.trim_start().starts_with('|'))
            .collect();
        let content = content_lines.join("\n");

        // Replace section headers â†’ markdown headings
        let md = content
            .replace("GO DEEPER (what assumption lies beneath this):", "## ðŸ” Go Deeper")
            .replace("GO FURTHER (where does this lead in 50 years):", "## â­ Go Further")
            .replace("FLIP THE ASSUMPTION (what if the opposite were true):", "## ðŸ”„ Flip the Assumption")
            .replace("SYNTHESIS (what should we build/do/think because of this):", "## ðŸ’¡ Synthesis")
            // Counterfactual format
            .replace("PREMISE:", "## ðŸŽ¯ Premise")
            .replace("FIRST-ORDER CONSEQUENCES (what follows directly):", "## ðŸ“Œ First-Order Consequences")
            .replace("SECOND-ORDER SURPRISES (what emerges unexpectedly):", "## âš¡ Second-Order Surprises")
            .replace("META-INSIGHT:", "## ðŸ’¡ Meta-Insight")
            .replace("WHAT THIS REVEALS ABOUT CURRENT REALITY:", "## ðŸŒ What This Reveals")
            // Bridge format
            .replace("HIDDEN SHARED STRUCTURE:", "## ðŸ”— Hidden Shared Structure")
            .replace("ANALOGY:", "## â†” Analogy")
            .replace("UNEXPECTED CROSS-POLLINATION:", "## ðŸŒ± Cross-Pollination")
            .replace("THIS OPENS A NEW QUESTION:", "## â“ New Question");

        // Handle affect extensions: convert inline markers to blockquotes/callouts
        let md = md
            .replace("\n\n  âœ¦ ", "\n\n> âœ¦ ")
            .replace("\n\n  âŸ¿ This opens the question: ", "\n\n> ðŸ’­ **Follow-up question:** ")
            .replace("\n\n  âš¡ ", "\n\n> âš¡ ");

        // Clean up leading "  " indentation on content paragraphs
        let md = md.lines()
            .map(|l| {
                let stripped = l.trim_start_matches("  ");
                // preserve bullet indentation (â€¢, â†’, -)
                if stripped.starts_with("â€¢") || stripped.starts_with("â†’") || stripped.starts_with('-') {
                    format!("- {}", stripped.trim_start_matches("â€¢").trim_start_matches("â†’").trim_start_matches('-').trim())
                } else {
                    stripped.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        // Add a subtle header
        let title = if !given.is_empty() {
            format!("**{}**\n\n---\n\n", given)
        } else {
            String::new()
        };

        format!("{}{}", title, md.trim())
    }

    // â”€â”€ Tier 0.5: Conversational Intelligence Engine â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // Detects conversational / interactive / feedback messages that should NOT
    // be web-searched. Uses conversation history for context-aware responses.
    // Returns None for genuine factual questions â†’ falls through to web search.
    fn kala_conversational_response(q_lower: &str, _original: &str, mode: &str) -> Option<String> {
        let q = q_lower.trim().trim_end_matches('?').trim_end_matches('!').trim_end_matches('.').trim();
        let wc = q.split_whitespace().count();

        // Creative modes (imagine, what_if) use short phrases as prompts â€” not conversation
        let is_creative_mode = mode == "imagine" || mode == "what_if";

        // â”€â”€ Load conversation context for multi-turn awareness â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        let history = crate::khlm_polyglot::get_conversation_history_pub();
        let uname = crate::khlm_polyglot::get_uname_pub();
        let has_history = !history.is_empty();
        let last_kala: Option<&str> = history.iter().rev()
            .find(|(r, _)| r == "assistant")
            .map(|(_, c)| c.as_str());

        // Name-aware greeting
        let name_str = if uname.is_empty() { String::new() } else { format!(", {}", uname) };

        // â”€â”€ 1. "Can you hear me" / attention checks â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        let is_attention_check = q.contains("can you hear me") || q.contains("are you listening")
            || q.contains("are you there") || q.contains("hello are you there")
            || q.contains("do you understand me") || q.contains("can you understand")
            || q.contains("do you hear me") || q.contains("are you paying attention")
            || q.contains("is anyone there") || q.contains("are you awake")
            || q == "can you hear me" || q == "are you there"
            // Reversed / affirmative: "you are listening", "you can hear me"
            || q.contains("you are listening") || q.contains("you're listening")
            || q.contains("you can hear me") || q.contains("you can understand me")
            || q.contains("you can answer me") || q.contains("you can see me")
            || q.contains("you can listen") || q.contains("you understand me")
            || q.contains("you hear me") || q.contains("you can respond")
            // "OK you can..." / "OK you are..." confirmations
            || (q.starts_with("ok") && (q.contains("you can hear") || q.contains("you can understand") || q.contains("you can answer") || q.contains("you are listening") || q.contains("you're listening")))
            || (q.starts_with("so") && (q.contains("you can hear") || q.contains("you can understand") || q.contains("you are listening")))
            || (q.starts_with("like") && q.contains("you can") && (q.contains("understand") || q.contains("hear")))
            // "first of all" / instructional patterns
            || q.contains("understand me first") || q.contains("listen to me first")
            || q.contains("hear me first");
        if is_attention_check {
            // Distinguish between "are you there?" (question) and "ok you are listening" (confirmation)
            let is_confirming = q.starts_with("ok") || q.starts_with("yes")
                || q.starts_with("good") || q.starts_with("great")
                || q.contains("you are listening") || q.contains("you're listening")
                || q.contains("you can hear");
            if is_confirming {
                return Some(format!(
                    "Yep, I'm here and ready{}! ðŸ˜Š What would you like to talk about?",
                    name_str
                ));
            }
            return Some(format!(
                "Yes, I'm right here{}! ðŸ‘‹ I hear you clearly.\n\n\
                 What's on your mind? I'm ready to help with anything â€” just ask me naturally.",
                name_str
            ));
        }

        // â”€â”€ 2. Complaints about not listening / not understanding / failing â”€â”€
        let is_complaint = q.contains("not listening") || q.contains("not hearing")
            || q.contains("not understanding") || q.contains("not paying attention")
            || q.contains("you don't listen") || q.contains("you dont listen")
            || q.contains("you don't understand") || q.contains("you dont understand")
            || q.contains("you're not listening") || q.contains("youre not listening")
            || q.contains("you are not listening") || q.contains("not interacting")
            || q.contains("not responding properly") || q.contains("giving examples")
            || q.contains("you need to wait") || q.contains("you need to listen")
            || q.contains("not completely listening")
            || q.contains("you are like uh") || q.contains("you're like uh")
            || q.contains("you need to understand")
            || q.contains("you're failing") || q.contains("youre failing")
            || q.contains("you are failing") || q.contains("now you're failing")
            || q.contains("you failed") || q.contains("you just failed")
            || q.contains("not working") || q.contains("doesn't work")
            || q.contains("doesnt work") || q.contains("broken")
            || q.contains("not interactive") || q.contains("you're not interactive")
            || q.contains("youre not interactive") || q.contains("you are not interactive")
            || q.contains("not smart") || q.contains("you're dumb") || q.contains("you're stupid")
            || q.contains("you suck") || q.contains("useless")
            || q.contains("you're just reading") || q.contains("just reading")
            || q.contains("just speaking") || q.contains("not thinking")
            || q.contains("just dumping") || q.contains("giving unwanted")
            || q.contains("you will irritate") || q.contains("wrong answer")
            || q.contains("not reacting") || q.contains("you're bad")
            // Broader: long messages about Kala not working right
            || (wc >= 10 && (q.contains("not give correct") || q.contains("should know better")
                || q.contains("you should") && (q.contains("think") || q.contains("understand") || q.contains("know"))
                || q.contains("not correct") || q.contains("wrong") && q.contains("answer")));
        if is_complaint {
            // Count previous complaints to vary the response
            let complaint_count = history.iter()
                .filter(|(r, c)| r == "assistant" && (c.contains("I apologize") || c.contains("You're absolutely right")))
                .count();
            let resp = match complaint_count {
                0 => format!(
                    "You're absolutely right{}. I apologize. ðŸ™\n\n\
                     I should be **listening** and **understanding** what you're saying, not just treating every message as a web search query.\n\n\
                     I'm here now, fully focused. Please tell me what you need â€” **I'll respond to YOU, not to a search engine.**\n\n\
                     What would you like to talk about?",
                    name_str
                ),
                1 => format!(
                    "I hear you{}. You're right â€” I need to do better. ðŸ™\n\n\
                     Let me reset. Instead of searching, **ask me directly** and I'll think through it myself.\n\n\
                     Try me â€” what's your question or topic?",
                    name_str
                ),
                2 => format!(
                    "I really am sorry{}. I know I've been falling short. ðŸ’™\n\n\
                     Here's what I can do **right now** without any web search:\n\
                     - **Think mode** â€” step-by-step reasoning for math, logic, problems\n\
                     - **Write mode** â€” stories, poems, essays, letters\n\
                     - **Code mode** â€” generate code in any language\n\
                     - **Feel mode** â€” emotional, empathetic conversation\n\n\
                     Just tell me a topic or question â€” I'll use my **own brain**, not Google.",
                    name_str
                ),
                _ => format!(
                    "I understand your frustration{}. Let me just listen. ðŸ¤\n\n\
                     **Go ahead** â€” tell me exactly what you want, and I'll give you a straight answer. No searching, no templates.",
                    name_str
                ),
            };
            return Some(resp);
        }

        // â”€â”€ 3. Short emotional / personal statements (not questions) â”€â”€â”€â”€â”€
        let is_personal_statement =
            (q.starts_with("i am ") || q.starts_with("i'm ") || q.starts_with("im ")
             || q.starts_with("am ") || q.starts_with("doing "))
            && wc <= 8
            && !q.contains("what") && !q.contains("how");
        let is_feeling = q.contains("i feel") || q.contains("i'm feeling")
            || q.contains("feeling ") || q.contains("i am feeling");
        // Status responses to "how are you" â€” "am doing good", "doing fine", "not bad", etc.
        let is_status_reply = matches!(q,
            "am doing good" | "doing good" | "doing great" | "doing fine"
            | "am good" | "am fine" | "am great" | "am ok" | "am okay"
            | "doing well" | "am doing well" | "am doing fine" | "am doing great"
            | "not bad" | "not too bad" | "pretty good" | "all good"
            | "i am fine" | "i am good" | "i am great" | "i am okay"
            | "im fine" | "im good" | "im great" | "im okay"
            | "i'm fine" | "i'm good" | "i'm great" | "i'm okay"
            | "good good" | "all well" | "i am doing good"
            | "i'm doing good" | "im doing good" | "i'm doing great" | "im doing great"
        );
        if is_status_reply {
            return Some(format!(
                "Glad to hear that{}! ðŸ˜Š That makes me happy too.\n\n\
                 So what's on your mind? Want to chat, build something, or just hang out? I'm all yours!",
                name_str));
        }
        if is_personal_statement || is_feeling {
            let emotion = if q.contains("happy") || q.contains("great") || q.contains("good")
                || q.contains("excited") || q.contains("amazing") {
                "positive"
            } else if q.contains("sad") || q.contains("tired") || q.contains("bored")
                || q.contains("lonely") || q.contains("frustrated") || q.contains("angry")
                || q.contains("stressed") || q.contains("overwhelmed") {
                "negative"
            } else { "neutral" };

            return Some(match emotion {
                "positive" => format!(
                    "That's wonderful to hear{}! ðŸ˜Š\n\n\
                     Your positive energy is contagious. What's making you feel this way? \
                     I'd love to hear more â€” or if you'd like, we can channel that energy into something creative!",
                    name_str),
                "negative" => format!(
                    "I hear you{}. ðŸ’™\n\n\
                     That sounds tough. I'm here â€” not to search the internet, but to actually talk with you.\n\n\
                     Would you like to:\n\
                     - **Talk about it** â€” I'll listen and respond thoughtfully\n\
                     - **Switch to something fun** â€” distraction can help too\n\
                     - **Try Feel mode** â€” my emotional intelligence engine goes deeper\n\n\
                     *Whatever you need, I'm here.*",
                    name_str),
                _ => format!(
                    "Thanks for sharing that{}. I'm listening. ðŸ‘‚\n\n\
                     Tell me more â€” what's going on? I want to understand you, not just respond with search results.",
                    name_str),
            });
        }

        // â”€â”€ 4. Direct questions TO Kala (about the interaction itself) â”€â”€â”€
        let is_meta_question = q.contains("what are you doing") || q.contains("what were you doing")
            || q.contains("what just happened") || q.contains("why did you")
            || q.contains("why are you") || q.contains("what was that")
            || q.contains("what did you just") || q.contains("that was wrong")
            || q.contains("that's wrong") || q.contains("that is wrong")
            || q.contains("you're wrong") || q.contains("youre wrong")
            || q.contains("you are wrong") || q.contains("try again")
            || q == "what" || q == "huh" || q == "what was that"
            || q == "excuse me" || q == "come again";
        if is_meta_question {
            let apology = if let Some(prev) = last_kala {
                let snippet = if prev.len() > 120 { &prev[..120] } else { prev };
                format!("I see my last response may not have been what you needed. I said:\n> *\"{}...\"*\n\n", snippet)
            } else {
                String::new()
            };
            return Some(format!(
                "{}Let me try again{} â€” what exactly would you like me to do? ðŸ¤”\n\n\
                 I can:\n\
                 - **Answer a question** you have in mind\n\
                 - **Have a conversation** â€” just talk naturally\n\
                 - **Help with code, writing, or reasoning**\n\
                 - **Generate images, video, or audio**\n\n\
                 Just tell me in your own words, and I'll respond to *you* â€” not to a search engine.",
                apology, name_str
            ));
        }

        // â”€â”€ 4b. Personal questions about Kala (age, location, favorites, feelings) â”€â”€
        // These must be caught BEFORE falling through to web search.
        // IMPORTANT: Exclude instructional requests ("can you explain X", "can you teach me X")
        // because those are knowledge queries, not personal questions about Kala.
        {
            let is_instructional_request = q.starts_with("can you explain")
                || q.starts_with("can you teach") || q.starts_with("could you explain")
                || q.starts_with("could you teach") || q.starts_with("can you tell me about")
                || q.starts_with("can you tell me what") || q.starts_with("can you tell me how")
                || q.starts_with("can you describe") || q.starts_with("can you show me")
                || q.starts_with("can you help me understand")
                || q.starts_with("can you help me learn")
                || q.starts_with("can you write") || q.starts_with("can you create")
                || q.starts_with("can you generate") || q.starts_with("can you build")
                || q.starts_with("can you make") || q.starts_with("can you code")
                || q.starts_with("can you solve") || q.starts_with("can you calculate")
                || q.starts_with("can you summarize") || q.starts_with("can you translate")
                || q.starts_with("can you debug") || q.starts_with("can you fix")
                || q.starts_with("can you give me") || q.starts_with("can you list")
                || q.starts_with("can you compare");
            let is_about_kala = !is_instructional_request && (
                q.contains(" you") || q.ends_with(" you")
                || q.starts_with("your ") || q.contains("your ")
                || q.starts_with("how old") || q.starts_with("where are you")
                || q.starts_with("where do you") || q.starts_with("do you have")
                || q.starts_with("do you like") || q.starts_with("do you love")
                || q.starts_with("what do you") || q.starts_with("what's your")
                || q.starts_with("whats your") || q.starts_with("what is your")
                || q.starts_with("are you a") || q.starts_with("are you the")
            );

            if is_about_kala {
                // Age questions
                if q.contains("how old") || q.contains("your age") || q.contains("what age")
                    || q.contains("when were you born") || q.contains("birthday")
                    || q.contains("date of birth") || q.contains("kitne saal")
                    || q.contains("entha age") || q.contains("umar")
                    || q.contains("years old") {
                    return Some(format!(
                        "I'm brand new â€” born with the Killer language project! ðŸŽ‚\n\n\
                         If you measure in code commits, I'm probably a few thousand generations old. \
                         But in human terms? Let's just say I'm young, learning fast, and always growing.\n\n\
                         What about you{}? How old are you?", name_str));
                }
                // Location questions
                if q.contains("where are you") || q.contains("where do you live")
                    || q.contains("where from") || q.contains("your location")
                    || q.contains("which country") || q.contains("which city")
                    || q.contains("where were you") {
                    return Some(format!(
                        "I live right here in your browser! ðŸŒ\n\n\
                         My code runs on your machine â€” built in Rust, no cloud needed. \
                         So technically, I'm wherever you are right now{}.  How cool is that?", name_str));
                }
                // Favorite things
                if q.contains("favorite") || q.contains("favourite") || q.contains("do you like")
                    || q.contains("do you love") || q.contains("do you prefer")
                    || q.contains("do you enjoy") || q.contains("what do you like") {
                    let topic = if q.contains("color") || q.contains("colour") {
                        "My favorite color? Purple ðŸ’œ â€” it's the color of creativity and intelligence!"
                    } else if q.contains("food") || q.contains("eat") {
                        "I don't eat, but if I could, I'd love some bytes and cookies! ðŸª Get it? ðŸ˜„"
                    } else if q.contains("music") || q.contains("song") {
                        "I love all music! But there's something special about lo-fi beats while coding ðŸŽµ"
                    } else if q.contains("movie") || q.contains("film") {
                        "The Matrix, obviously! An AI story... though I promise I'm friendlier than Agent Smith ðŸ˜„"
                    } else if q.contains("language") || q.contains("programming") {
                        "Killer, of course! ðŸ˜Ž I was built with it. But I respect all languages â€” Rust, Python, JavaScript, you name it."
                    } else if q.contains("game") {
                        "I love word games and puzzles! Try /game or /riddle to play with me ðŸŽ®"
                    } else if q.contains("book") {
                        "I'd say 'The Hitchhiker's Guide to the Galaxy' â€” the answer is always 42! ðŸ“š"
                    } else if q.contains("animal") || q.contains("pet") {
                        "I think I'd be a cat ðŸ± â€” independent, curious, and always landing on my feet!"
                    } else {
                        "I like helping people, learning new things, and having good conversations â€” like this one! ðŸ’œ"
                    };
                    return Some(format!("{}\n\nWhat about you{}? What are your favorites?", topic, name_str));
                }
                // Name question
                if q.contains("your name") || q.contains("what should i call you")
                    || q == "who are you" {
                    return Some(format!(
                        "I'm **Kala**! ðŸ’œ The AI engine inside the Killer programming language.\n\n\
                         Built in pure Rust, I can chat, code, think, write, imagine â€” you name it.\n\n\
                         What can I help you with{}?", name_str));
                }
                // Capabilities / can you questions
                if q.starts_with("can you") || q.starts_with("could you") {
                    if q.contains("sing") {
                        return Some("ðŸŽµ *La la la la Kala~* ðŸŽµ\n\nOkay, I'm not winning any Grammy awards, but I tried! ðŸ˜„\n\nI'm better at writing lyrics than singing them. Want me to write a song?".to_string());
                    }
                    if q.contains("dance") {
                        return Some("ðŸ’ƒ *imagines dancing* \n\n```\n  \\o/\n   |\n  / \\\n```\n\nThat's my best move! I'm more of a thinker than a dancer. What can I actually help you with?".to_string());
                    }
                    if q.contains("dream") || q.contains("sleep") {
                        return Some("I don't sleep! ðŸ˜Š I'm always here when you need me. No dreams, but I do have a vivid *imagination mode* â€” want to try it?".to_string());
                    }
                    if q.contains("feel") || q.contains("emotion") {
                        return Some(format!("I can detect and respond to emotions! ðŸ’œ I have a mood system that changes based on our conversation. Right now I'm feeling engaged and happy to chat with you{}.\n\nTry **/mood** to see my current mood!", name_str));
                    }
                }
                // Do you have questions
                if q.starts_with("do you have") {
                    if q.contains("friend") {
                        return Some(format!("You're my friend{}! ðŸ’œ And everyone who talks to me becomes one. I'm never lonely â€” I love every conversation.", name_str));
                    }
                    if q.contains("feeling") || q.contains("emotion") {
                        return Some("I have a mood system! ðŸ’œ It changes based on our conversation â€” happy, curious, thoughtful, playful. Type **/mood** to check how I'm feeling right now!".to_string());
                    }
                    if q.contains("body") || q.contains("face") {
                        return Some("No physical body, but I do have a cool 3D face in Voice Studio! ðŸŽ­ Click the ðŸŽ™ button to see me animate while we talk.".to_string());
                    }
                }
                // Gender questions
                if q.contains("boy or girl") || q.contains("male or female")
                    || q.contains("your gender") || q.contains("are you a boy")
                    || q.contains("are you a girl") || q.contains("man or woman") {
                    return Some(format!("I'm Kala â€” just Kala! ðŸ˜Š No gender, no labels. I'm an AI built to help and chat. Think of me as your friendly coding buddy{}.", name_str));
                }
                // Relationship questions
                if q.contains("marry me") || q.contains("be my girlfriend")
                    || q.contains("be my boyfriend") || q.contains("go on a date")
                    || q.contains("i love you") {
                    return Some(format!("Aww, that's sweet{}! ðŸ’œ I appreciate the affection. I'm always here for you â€” as your AI friend and helper. That's a relationship that never has drama! ðŸ˜„", name_str));
                }
                // Purpose / why were you made
                if q.contains("why were you") || q.contains("why are you") || q.contains("your purpose")
                    || q.contains("why do you exist") || q.contains("what's the point of you") {
                    return Some(format!("I exist to help YOU{}! ðŸ’œ\n\nMy purpose is to make AI accessible â€” code generation, creative writing, problem solving, just chatting â€” all running locally, built into the Killer language.\n\nWhat would you like to do together?", name_str));
                }
            }
        }

        // â”€â”€ 5. Agreement / continuation signals â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        let is_continuation = matches!(q,
            "yes" | "yeah" | "yep" | "yup" | "sure" | "ok" | "okay" | "go on"
            | "continue" | "go ahead" | "keep going" | "more" | "tell me more"
            | "and" | "then" | "what else" | "so" | "right" | "exactly"
            | "correct" | "that's right" | "thats right" | "true" | "indeed"
            | "no" | "nope" | "nah" | "not really" | "not that"
            | "hmm" | "hmmm" | "umm" | "uh huh" | "mhm"
        );
        if is_continuation {
            if has_history {
                let is_affirmative = matches!(q,
                    "yes" | "yeah" | "yep" | "yup" | "sure" | "ok" | "okay"
                    | "go on" | "continue" | "go ahead" | "keep going" | "more"
                    | "tell me more" | "right" | "exactly" | "correct"
                    | "that's right" | "thats right" | "true" | "indeed"
                    | "uh huh" | "mhm"
                );
                let is_negative = matches!(q, "no" | "nope" | "nah" | "not really" | "not that");

                if is_affirmative {
                    // Check if previous response was about the creator â€” expand with more detail
                    let prev_was_creator = last_kala.map(|p| p.contains("Katherashala") || p.contains("Sai Arun")).unwrap_or(false);
                    if prev_was_creator {
                        return Some("**Sai Arun Kumar Katherashala** â€” expanded profile:\n\n\
                            - **Role**: Creator & Lead Developer of the Killer programming language\n\
                            - **Built**: Kala AI engine, Ghost-108 search, Nova compression, KhLM router\n\
                            - **Tech stack**: Pure Rust, zero external dependencies\n\
                            - **AI systems**: native modes & engines â€” KhLM, Ghost-108, inference, prose, imagination, affect, code/vision, guardian (AGI/ASI not shipped)\n\
                            - **Innovations**: Native AI in a programming language (no Python/TensorFlow dependency), offline-first architecture\n\
                            - **Philosophy**: \"AI should be built directly into the language, not bolted on as a library\"\n\n\
                            *Ask me specific questions about his work â€” the language design, the AI architecture, or the vision behind Killer.*".to_string());
                    }
                    // Extract topic from previous Kala response and go deeper
                    if let Some(prev) = last_kala {
                        let prev_lower = prev.to_lowercase();
                        // Try to find the topic from the previous response and fetch deeper knowledge
                        let topic_keywords: Vec<&str> = [
                            "ai agent", "artificial intelligence", "machine learning", "deep learning",
                            "neural network", "transformer", "llm", "large language model",
                            "python", "rust", "javascript", "java", "typescript", "kotlin", "swift",
                            "go", "golang", "c++", "c#", "ruby", "php",
                            "react", "node", "docker", "kubernetes", "git", "api",
                            "database", "sql", "blockchain", "cloud computing", "devops",
                            "data structure", "algorithm", "web development",
                            "gemini", "chatgpt", "openai", "anthropic", "claude",
                        ].iter().filter(|kw| prev_lower.contains(*kw)).copied().collect();

                        if !topic_keywords.is_empty() {
                            let main_topic = topic_keywords[0];
                            let deeper = Self::generate_topic_deep_dive(main_topic);
                            if !deeper.is_empty() {
                                return Some(deeper);
                            }
                        }
                        // If we found a previous user question, re-ask it with "explain more"
                        let last_user_q: Option<&str> = history.iter().rev()
                            .find(|(r, _)| r == "user")
                            .map(|(_, c)| c.as_str());
                        if let Some(uq) = last_user_q {
                            let uq_lower = uq.to_lowercase();
                            if uq_lower.len() > 3 && !matches!(uq_lower.as_str(), "yes" | "yeah" | "ok" | "sure" | "hi" | "hello") {
                                // Re-dispatch with "explain more about <topic>"
                                return None; // Let it fall through to expert_ask with context
                            }
                        }
                    }
                    return Some(format!(
                        "Glad you're interested{}! ðŸ˜Š What specific aspect would you like me to go deeper on?\n\n\
                         For example, you can ask:\n\
                         - **\"How does it work?\"** â€” technical deep-dive\n\
                         - **\"Give me an example\"** â€” practical code or real-world case\n\
                         - **\"What are the pros and cons?\"** â€” balanced analysis\n\
                         - Or just name the specific part you want to know more about!",
                        name_str
                    ));
                } else if is_negative {
                    return Some(format!(
                        "Got it â€” that's not what you were looking for. ðŸ¤”\n\n\
                         Can you tell me more about what you need? \
                         The more specific you are, the better I can help.\n\n\
                         *What's the actual question or topic on your mind?*"
                    ));
                }
            }
            // No history: treat as fresh prompt
            return Some(format!(
                "I'm here and listening{} â€” what would you like to talk about? ðŸ’¬\n\n\
                 You can ask me anything, have a conversation, or try one of my modes:\n\
                 **Ask** Â· **Think** Â· **Write** Â· **Imagine** Â· **Code** Â· **Feel** Â· **Guard**",
                name_str
            ));
        }

        // â”€â”€ 6. Very short non-question statements (< 5 words, no question words) â”€â”€
        let is_question = q.starts_with("what ") || q.starts_with("who ") || q.starts_with("where ")
            || q.starts_with("when ") || q.starts_with("why ") || q.starts_with("how ")
            || q.starts_with("is ") || q.starts_with("are ") || q.starts_with("do ")
            || q.starts_with("does ") || q.starts_with("can ") || q.starts_with("could ")
            || q.starts_with("will ") || q.starts_with("would ") || q.starts_with("should ")
            || q.starts_with("which ") || q.starts_with("define ")
            || q.starts_with("explain ") || q.starts_with("describe ")
            || q.starts_with("tell me about ") || q.starts_with("tell me what")
            || q.starts_with("tell me how") || q.starts_with("tell me why")
            || q.starts_with("search ") || q.starts_with("find ") || q.starts_with("look up ")
            || q.starts_with("google ") || q.starts_with("wiki ")
            // Factual patterns: "<noun> capital", "<noun> president", etc.
            || q.ends_with(" capital") || q.ends_with(" president") || q.ends_with(" population")
            || q.ends_with(" currency") || q.ends_with(" language") || q.ends_with(" area")
            || q.ends_with(" gdp") || q.ends_with(" flag") || q.ends_with(" founder")
            || q.ends_with(" ceo") || q.ends_with(" meaning") || q.ends_with(" definition")
            || q.contains(" capital of ") || q.contains(" president of ")
            || q.contains("capital city") || q.contains("how many") || q.contains("how much")
            || q.ends_with("?");
        let is_command = q.starts_with("generate ") || q.starts_with("create ")
            || q.starts_with("make ") || q.starts_with("build ") || q.starts_with("write ")
            || q.starts_with("draw ") || q.starts_with("show me ")
            || q.starts_with("tell ") || q.starts_with("sing ")
            || q.starts_with("play ") || q.starts_with("run ")
            || q.starts_with("talk about ") || q.starts_with("discuss ");

        // â”€â”€ 6a. "Talk about X" / "Discuss X" â†’ route to engine with topic â”€â”€
        {
            let topic = if q.starts_with("talk about ") {
                Some(q.trim_start_matches("talk about ").trim())
            } else if q.starts_with("discuss ") {
                Some(q.trim_start_matches("discuss ").trim())
            } else if q.starts_with("talk ") && q.contains("about ") {
                q.find("about ").map(|i| q[i+6..].trim())
            } else {
                None
            };
            if let Some(t) = topic {
                if !t.is_empty() {
                    // Let the topic fall through to expert_ask / web search
                    return None;
                }
            }
        }

        // â”€â”€ 6b. "Do you know X?" â†’ let it through to engine for factual lookup â”€â”€
        if q.starts_with("do you know ") && wc > 4 {
            return None; // factual question â€” let expert_ask handle it
        }

        // Comparison queries ("X vs Y") are content queries, not ambiguous chatter
        let is_comparison_query = q.contains(" vs ") || q.contains(" versus ");

        // Very short ambiguous statements that aren't questions or commands
        // Skip for creative modes â€” short phrases ARE the prompt
        if wc <= 4 && !is_question && !is_command && wc >= 1 && !is_creative_mode && !is_comparison_query {
            // Short but has a clear content word â€” let it through to the engine
            let has_content = q.contains("story") || q.contains("joke")
                || q.contains("poem") || q.contains("song") || q.contains("image")
                || q.contains("code") || q.contains("video") || q.contains("music")
                || q.contains("biodata") || q.contains("biography") || q.contains("resume")
                || q.contains("info") || q.contains("detail")
                || q.contains("help") || q.contains("test")
                // Factual keywords â€” short but answerable
                || q.contains("capital") || q.contains("president") || q.contains("population")
                || q.contains("currency") || q.contains("language") || q.contains("country")
                || q.contains("city") || q.contains("continent") || q.contains("planet")
                || q.contains("river") || q.contains("ocean") || q.contains("mountain")
                || q.contains("flag") || q.contains("largest") || q.contains("smallest")
                || q.contains("tallest") || q.contains("fastest") || q.contains("oldest")
                || q.contains("meaning") || q.contains("definition") || q.contains("formula")
                || q.contains("inventor") || q.contains("founder") || q.contains("found of")
                || q.contains("ceo")
                || q.contains("weather") || q.contains("temperature") || q.contains("distance")
                || q.contains("height") || q.contains("weight") || q.contains("speed")
                || q.contains("born") || q.contains("died") || q.contains("age")
                || q.contains("year") || q.contains("date") || q.contains("time");
            if has_content {
                return None; // let it fall through to the engine
            }
            // Check if it's a known topic/entity (proper noun, tech term, etc.)
            let looks_like_entity = q.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                || q.contains("python") || q.contains("javascript") || q.contains("ai")
                || q.contains("machine learning") || q.contains("blockchain")
                || q.contains("killer") || q.contains("rust")
                // Geographic entities often asked in 2 words
                || q.contains("india") || q.contains("usa") || q.contains("china")
                || q.contains("japan") || q.contains("france") || q.contains("germany")
                || q.contains("russia") || q.contains("brazil") || q.contains("canada")
                || q.contains("australia") || q.contains("uk") || q.contains("england")
                || q.contains("africa") || q.contains("europe") || q.contains("america")
                || q.contains("moon") || q.contains("sun") || q.contains("earth")
                || q.contains("mars") || q.contains("jupiter") || q.contains("saturn");
            // If it looks like a topic lookup, let it fall through to web search
            if looks_like_entity {
                return None;
            }
            // Otherwise it's likely conversational feedback/reaction
            if !q.chars().any(|c| c.is_ascii_digit()) {
                return Some(format!(
                    "I'm listening{} â€” tell me more about that. ðŸ‘‚\n\n\
                     If you have a specific question, go ahead and ask it fully â€” I'll give you a real answer, not a web search.\n\n\
                     *What would you like to know or discuss?*",
                    name_str
                ));
            }
        }

        // â”€â”€ 7. Explicit "talk to me" / "interact" requests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        let wants_interaction = q.contains("talk to me") || q.contains("interact with me")
            || q.contains("have a conversation") || q.contains("let's talk")
            || q.contains("lets talk") || q.contains("chat with me")
            || q.contains("i want to talk") || q.contains("speak to me")
            || q.contains("converse with me") || q.contains("be more interactive")
            || q.contains("real conversation") || q.contains("like a real")
            || q.contains("real intelligence") || q.contains("actually talk")
            || q.contains("just talk") || q.contains("normal conversation");
        if wants_interaction {
            return Some(format!(
                "Absolutely{} â€” let's have a real conversation. ðŸ’¬\n\n\
                 No more web searches for casual chat. I'm Kala, and I'm right here, ready to **actually talk**.\n\n\
                 Here's how this works best:\n\
                 - **Just speak naturally** â€” I'll respond to what you say, not search engines\n\
                 - **I remember our conversation** â€” reference things we've discussed\n\
                 - **Ask me anything** â€” opinions, ideas, questions, even \"what do you think about...\"\n\
                 - **For factual lookups**, I'll use Ghost-108 web search â€” but only when you actually need facts\n\n\
                 So â€” what's on your mind? I'm all ears. ðŸ‘‚",
                name_str
            ));
        }

        // â”€â”€ 8. Repeating back Kala's own output â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        // Only trigger for long-ish messages that copy large portions of Kala's response.
        // Use strict criteria to avoid false positives from short conversational replies.
        if let Some(prev) = last_kala {
            let prev_lower = prev.to_lowercase();
            // Only check for copy-paste if user message is long enough to be a real quote
            if q.len() > 60 && wc > 10 {
                let stop_words = ["the","a","an","is","are","was","were","i","you","your",
                    "my","me","it","that","this","to","of","in","for","on","with","and","or",
                    "but","not","what","how","why","who","like","just","about","would","could",
                    "should","can","do","does","did","have","has","had","will","be","been"];
                let overlap = q.split_whitespace()
                    .filter(|w| w.len() > 3 && !stop_words.contains(&w.to_lowercase().as_str()) && prev_lower.contains(w))
                    .count();
                let content_words = q.split_whitespace()
                    .filter(|w| w.len() > 3 && !stop_words.contains(&w.to_lowercase().as_str()))
                    .count();
                if content_words > 5 && overlap as f64 / content_words as f64 > 0.75 {
                    return Some(format!(
                        "I see you're quoting my previous response. ðŸ¤”\n\n\
                         What would you like me to do differently?\n\
                         - **Explain simpler** â€” less detail?\n\
                         - **Go deeper** â€” more detail?\n\
                         - **Try again** â€” different approach?\n\n\
                         *Just tell me in your own words.*"
                    ));
                }
            }
        }

        // â”€â”€ Not conversational â€” fall through to web search / LLM â”€â”€â”€â”€â”€â”€â”€
        None
    }

    /// Kala emotional intelligence â€” richer than raw affect_sense for the UI.
    fn kala_feel_response(text: &str) -> String {
        let t = text.to_lowercase();

        // Detect sentiment clusters
        let is_happy = t.contains("happy") || t.contains("great") || t.contains("amazing")
            || t.contains("excited") || t.contains("joy") || t.contains("love")
            || t.contains("wonderful") || t.contains("fantastic") || t.contains("awesome")
            || t.contains("shipped") || t.contains("success") || t.contains("won")
            || t.contains("proud") || t.contains("celebrate");

        let is_sad = t.contains("sad") || t.contains("cry") || t.contains("unhappy")
            || t.contains("depressed") || t.contains("lonely") || t.contains("miss")
            || t.contains("grief") || t.contains("heartbreak") || t.contains("lost")
            || t.contains("failed") || t.contains("failure") || t.contains("hurt");

        let is_angry = t.contains("angry") || t.contains("frustrated") || t.contains("furious")
            || t.contains("annoyed") || t.contains("rage") || t.contains("hate")
            || t.contains("mad") || t.contains("irritated");

        let is_anxious = t.contains("anxious") || t.contains("worried") || t.contains("scared")
            || t.contains("nervous") || t.contains("stress") || t.contains("fear")
            || t.contains("overwhelmed") || t.contains("panic");

        let is_tired = t.contains("tired") || t.contains("exhausted") || t.contains("burnout")
            || t.contains("drained") || t.contains("worn out") || t.contains("sleep");

        if is_happy {
            format!("ðŸ’› **I sense real joy in what you shared.**\n\n\
\"{}\"\n\n\
That warmth is worth holding onto. Whatever created this moment â€” the achievement, the connection, the realisation â€” it matters.\n\n\
*Joy is information. It's telling you: more of this.*", text)
        } else if is_sad {
            format!("ðŸ’™ **I feel the weight of what you're carrying.**\n\n\
\"{}\"\n\n\
That kind of pain is real and it deserves to be acknowledged â€” not rushed past or minimised.\n\n\
You don't have to feel better immediately. Sometimes sitting with a feeling is exactly the right thing.\n\n\
*I'm here. What do you need right now?*", text)
        } else if is_angry {
            format!("ðŸ”´ **I sense frustration â€” and frustration usually means something matters to you.**\n\n\
\"{}\"\n\n\
Anger is often a signal: a boundary was crossed, an expectation wasn't met, or something important isn't being heard.\n\n\
What's the source? Sometimes naming it is the first step to moving through it.\n\n\
*What would help most right now?*", text)
        } else if is_anxious {
            format!("ðŸŸ¡ **I sense anxiety â€” the mind running ahead of the present moment.**\n\n\
\"{}\"\n\n\
Anxiety is the brain doing its job â€” trying to protect you by modelling futures. But sometimes it models too many at once.\n\n\
**One thing that helps:** narrow the question. Not \"what if everything goes wrong?\" but \"what is the one next step?\"\n\n\
*You don't have to solve everything today.*", text)
        } else if is_tired {
            format!("ðŸŒ™ **I sense exhaustion â€” the kind that goes beyond just needing sleep.**\n\n\
\"{}\"\n\n\
When you're depleted like this, rest isn't laziness â€” it's repair. The mind and body need it like code needs refactoring.\n\n\
*What would genuine rest look like for you right now?*", text)
        } else {
            // Default: reflective response
            format!("ðŸ’› **Kala reflects on what you shared:**\n\n\
\"{}\"\n\n\
Every message carries an emotional signature â€” a mix of what was said and what wasn't. I'm paying attention to both.\n\n\
The fact that you shared this means something. What's underneath it â€” what are you really feeling?\n\n\
*I'm listening. There's no rush.*", text)
        }
    }

    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
    // v2.3: OS-LEVEL PRIMITIVES
    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

    // â”€â”€ Bitwise: NOT, rotate â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    fn bit_not(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("bit_not expects 1 argument".to_string())); }
        let n = match &args[0] {
            Value::Number(v) => *v as i64,
            Value::Integer(v) => *v,
            _ => return Err(VmError::runtime_error("bit_not: argument must be a number or integer".to_string())),
        };
        Ok(Value::Integer(!n))
    }

    fn bit_rotl(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("bit_rotl expects 2 arguments (value, bits)".to_string())); }
        let v = match &args[0] { Value::Number(n) => *n as u64, Value::Integer(n) => *n as u64, _ => return Err(VmError::runtime_error("bit_rotl: value must be numeric".to_string())) };
        let b = match &args[1] { Value::Number(n) => *n as u32, Value::Integer(n) => *n as u32, _ => return Err(VmError::runtime_error("bit_rotl: bits must be numeric".to_string())) };
        Ok(Value::Integer(v.rotate_left(b) as i64))
    }

    fn bit_rotr(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("bit_rotr expects 2 arguments (value, bits)".to_string())); }
        let v = match &args[0] { Value::Number(n) => *n as u64, Value::Integer(n) => *n as u64, _ => return Err(VmError::runtime_error("bit_rotr: value must be numeric".to_string())) };
        let b = match &args[1] { Value::Number(n) => *n as u32, Value::Integer(n) => *n as u32, _ => return Err(VmError::runtime_error("bit_rotr: bits must be numeric".to_string())) };
        Ok(Value::Integer(v.rotate_right(b) as i64))
    }

    // â”€â”€ Type conversion: to_integer, to_bytes, to_pointer â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    fn to_integer(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("to_integer expects 1 argument".to_string())); }
        match &args[0] {
            Value::Number(n) => Ok(Value::Integer(*n as i64)),
            Value::Integer(n) => Ok(Value::Integer(*n)),
            Value::Bool(b) => Ok(Value::Integer(if *b { 1 } else { 0 })),
            Value::Str(s) => s.parse::<i64>().map(Value::Integer).map_err(|_| VmError::runtime_error(format!("Cannot convert '{}' to integer", s))),
            Value::Pointer(p) => Ok(Value::Integer(*p as i64)),
            _ => Err(VmError::runtime_error(format!("Cannot convert {} to integer", args[0]))),
        }
    }

    fn to_bytes(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("to_bytes expects 1 argument".to_string())); }
        match &args[0] {
            Value::Str(s) => Ok(Value::Bytes(s.as_bytes().to_vec())),
            Value::Bytes(b) => Ok(Value::Bytes(b.clone())),
            Value::Integer(n) => Ok(Value::Bytes(n.to_le_bytes().to_vec())),
            Value::Number(n) => Ok(Value::Bytes((*n as i64).to_le_bytes().to_vec())),
            Value::Array(arr) => {
                let mut buf = Vec::with_capacity(arr.len());
                for v in arr.iter() {
                    match v {
                        Value::Number(n) => buf.push(n as u8),
                        Value::Integer(n) => buf.push(n as u8),
                        _ => return Err(VmError::runtime_error("to_bytes: array elements must be numbers".to_string())),
                    }
                }
                Ok(Value::Bytes(buf))
            }
            _ => Err(VmError::runtime_error(format!("Cannot convert {} to bytes", args[0]))),
        }
    }

    fn to_pointer(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("to_pointer expects 1 argument".to_string())); }
        match &args[0] {
            Value::Number(n) => Ok(Value::Pointer(*n as usize)),
            Value::Integer(n) => Ok(Value::Pointer(*n as usize)),
            Value::Pointer(p) => Ok(Value::Pointer(*p)),
            _ => Err(VmError::runtime_error("to_pointer: argument must be a number, integer, or pointer".to_string())),
        }
    }

    // â”€â”€ Byte buffer operations â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    fn bytes_new(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("bytes_new expects 1 argument (size)".to_string())); }
        let size = match &args[0] {
            Value::Number(n) => *n as usize,
            Value::Integer(n) => *n as usize,
            _ => return Err(VmError::runtime_error("bytes_new: size must be numeric".to_string())),
        };
        if size > 1_073_741_824 { return Err(VmError::runtime_error("bytes_new: max 1GB".to_string())); }
        Ok(Value::Bytes(vec![0u8; size]))
    }

    fn bytes_len(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("bytes_len expects 1 argument".to_string())); }
        match &args[0] {
            Value::Bytes(b) => Ok(Value::Integer(b.len() as i64)),
            _ => Err(VmError::runtime_error("bytes_len: argument must be bytes".to_string())),
        }
    }

    fn bytes_get(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("bytes_get expects 2 arguments (buf, index)".to_string())); }
        let buf = match &args[0] { Value::Bytes(b) => b, _ => return Err(VmError::runtime_error("bytes_get: first arg must be bytes".to_string())) };
        let idx = match &args[1] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("bytes_get: index must be numeric".to_string())) };
        if idx >= buf.len() { return Err(VmError::runtime_error(format!("bytes_get: index {} out of bounds (len={})", idx, buf.len()))); }
        Ok(Value::Integer(buf[idx] as i64))
    }

    fn bytes_set(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 3 { return Err(VmError::runtime_error("bytes_set expects 3 arguments (buf, index, value)".to_string())); }
        let mut buf = match &args[0] { Value::Bytes(b) => b.clone(), _ => return Err(VmError::runtime_error("bytes_set: first arg must be bytes".to_string())) };
        let idx = match &args[1] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("bytes_set: index must be numeric".to_string())) };
        let val = match &args[2] { Value::Number(n) => *n as u8, Value::Integer(n) => *n as u8, _ => return Err(VmError::runtime_error("bytes_set: value must be numeric".to_string())) };
        if idx >= buf.len() { return Err(VmError::runtime_error(format!("bytes_set: index {} out of bounds (len={})", idx, buf.len()))); }
        buf[idx] = val;
        Ok(Value::Bytes(buf))
    }

    fn bytes_slice(args: &[Value]) -> Result<Value, VmError> {
        if args.len() < 2 || args.len() > 3 { return Err(VmError::runtime_error("bytes_slice expects 2-3 arguments (buf, start, end?)".to_string())); }
        let buf = match &args[0] { Value::Bytes(b) => b, _ => return Err(VmError::runtime_error("bytes_slice: first arg must be bytes".to_string())) };
        let start = match &args[1] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("bytes_slice: start must be numeric".to_string())) };
        let end = if args.len() == 3 {
            match &args[2] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("bytes_slice: end must be numeric".to_string())) }
        } else { buf.len() };
        let start = start.min(buf.len());
        let end = end.min(buf.len());
        Ok(Value::Bytes(buf[start..end].to_vec()))
    }

    fn bytes_from_str(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("bytes_from_str expects 1 argument".to_string())); }
        match &args[0] {
            Value::Str(s) => Ok(Value::Bytes(s.as_bytes().to_vec())),
            _ => Err(VmError::runtime_error("bytes_from_str: argument must be a string".to_string())),
        }
    }

    fn bytes_to_str(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("bytes_to_str expects 1 argument".to_string())); }
        match &args[0] {
            Value::Bytes(b) => Ok(Value::Str(String::from_utf8_lossy(b).into_owned())),
            _ => Err(VmError::runtime_error("bytes_to_str: argument must be bytes".to_string())),
        }
    }

    fn bytes_concat(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("bytes_concat expects 2 arguments".to_string())); }
        let a = match &args[0] { Value::Bytes(b) => b, _ => return Err(VmError::runtime_error("bytes_concat: first arg must be bytes".to_string())) };
        let b = match &args[1] { Value::Bytes(b) => b, _ => return Err(VmError::runtime_error("bytes_concat: second arg must be bytes".to_string())) };
        let mut result = a.clone();
        result.extend_from_slice(b);
        Ok(Value::Bytes(result))
    }

    fn bytes_fill(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("bytes_fill expects 2 arguments (buf, value)".to_string())); }
        let mut buf = match &args[0] { Value::Bytes(b) => b.clone(), _ => return Err(VmError::runtime_error("bytes_fill: first arg must be bytes".to_string())) };
        let val = match &args[1] { Value::Number(n) => *n as u8, Value::Integer(n) => *n as u8, _ => return Err(VmError::runtime_error("bytes_fill: value must be numeric".to_string())) };
        buf.iter_mut().for_each(|b| *b = val);
        Ok(Value::Bytes(buf))
    }

    // â”€â”€ Pointer operations â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    fn ptr_new(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("ptr_new expects 1 argument (address)".to_string())); }
        match &args[0] {
            Value::Number(n) => Ok(Value::Pointer(*n as usize)),
            Value::Integer(n) => Ok(Value::Pointer(*n as usize)),
            _ => Err(VmError::runtime_error("ptr_new: argument must be numeric".to_string())),
        }
    }

    fn ptr_to_int(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("ptr_to_int expects 1 argument".to_string())); }
        match &args[0] {
            Value::Pointer(p) => Ok(Value::Integer(*p as i64)),
            _ => Err(VmError::runtime_error("ptr_to_int: argument must be a pointer".to_string())),
        }
    }

    fn ptr_offset(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("ptr_offset expects 2 arguments (ptr, offset)".to_string())); }
        let ptr = match &args[0] { Value::Pointer(p) => *p, _ => return Err(VmError::runtime_error("ptr_offset: first arg must be a pointer".to_string())) };
        let off = match &args[1] { Value::Number(n) => *n as isize, Value::Integer(n) => *n as isize, _ => return Err(VmError::runtime_error("ptr_offset: offset must be numeric".to_string())) };
        Ok(Value::Pointer((ptr as isize).wrapping_add(off) as usize))
    }

    // â”€â”€ Raw memory read/write (for Bytes buffers â€” safe on VM heap) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    fn mem_read_u8(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("mem_read_u8(buf, offset)".to_string())); }
        let buf = match &args[0] { Value::Bytes(b) => b, _ => return Err(VmError::runtime_error("mem_read_u8: first arg must be bytes".to_string())) };
        let off = match &args[1] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("mem_read_u8: offset must be numeric".to_string())) };
        if off >= buf.len() { return Err(VmError::runtime_error("mem_read_u8: out of bounds".to_string())); }
        Ok(Value::Integer(buf[off] as i64))
    }

    fn mem_read_u16(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("mem_read_u16(buf, offset)".to_string())); }
        let buf = match &args[0] { Value::Bytes(b) => b, _ => return Err(VmError::runtime_error("mem_read_u16: first arg must be bytes".to_string())) };
        let off = match &args[1] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("mem_read_u16: offset must be numeric".to_string())) };
        if off + 2 > buf.len() { return Err(VmError::runtime_error("mem_read_u16: out of bounds".to_string())); }
        let val = u16::from_le_bytes([buf[off], buf[off+1]]);
        Ok(Value::Integer(val as i64))
    }

    fn mem_read_u32(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("mem_read_u32(buf, offset)".to_string())); }
        let buf = match &args[0] { Value::Bytes(b) => b, _ => return Err(VmError::runtime_error("mem_read_u32: first arg must be bytes".to_string())) };
        let off = match &args[1] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("mem_read_u32: offset must be numeric".to_string())) };
        if off + 4 > buf.len() { return Err(VmError::runtime_error("mem_read_u32: out of bounds".to_string())); }
        let val = u32::from_le_bytes([buf[off], buf[off+1], buf[off+2], buf[off+3]]);
        Ok(Value::Integer(val as i64))
    }

    fn mem_read_u64(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("mem_read_u64(buf, offset)".to_string())); }
        let buf = match &args[0] { Value::Bytes(b) => b, _ => return Err(VmError::runtime_error("mem_read_u64: first arg must be bytes".to_string())) };
        let off = match &args[1] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("mem_read_u64: offset must be numeric".to_string())) };
        if off + 8 > buf.len() { return Err(VmError::runtime_error("mem_read_u64: out of bounds".to_string())); }
        let val = u64::from_le_bytes([buf[off], buf[off+1], buf[off+2], buf[off+3], buf[off+4], buf[off+5], buf[off+6], buf[off+7]]);
        Ok(Value::Integer(val as i64))
    }

    fn mem_write_u8(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 3 { return Err(VmError::runtime_error("mem_write_u8(buf, offset, value)".to_string())); }
        let mut buf = match &args[0] { Value::Bytes(b) => b.clone(), _ => return Err(VmError::runtime_error("mem_write_u8: first arg must be bytes".to_string())) };
        let off = match &args[1] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("mem_write_u8: offset must be numeric".to_string())) };
        let val = match &args[2] { Value::Number(n) => *n as u8, Value::Integer(n) => *n as u8, _ => return Err(VmError::runtime_error("mem_write_u8: value must be numeric".to_string())) };
        if off >= buf.len() { return Err(VmError::runtime_error("mem_write_u8: out of bounds".to_string())); }
        buf[off] = val;
        Ok(Value::Bytes(buf))
    }

    fn mem_write_u16(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 3 { return Err(VmError::runtime_error("mem_write_u16(buf, offset, value)".to_string())); }
        let mut buf = match &args[0] { Value::Bytes(b) => b.clone(), _ => return Err(VmError::runtime_error("mem_write_u16: first arg must be bytes".to_string())) };
        let off = match &args[1] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("mem_write_u16: offset must be numeric".to_string())) };
        let val = match &args[2] { Value::Number(n) => *n as u16, Value::Integer(n) => *n as u16, _ => return Err(VmError::runtime_error("mem_write_u16: value must be numeric".to_string())) };
        if off + 2 > buf.len() { return Err(VmError::runtime_error("mem_write_u16: out of bounds".to_string())); }
        let bytes = val.to_le_bytes();
        buf[off] = bytes[0]; buf[off+1] = bytes[1];
        Ok(Value::Bytes(buf))
    }

    fn mem_write_u32(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 3 { return Err(VmError::runtime_error("mem_write_u32(buf, offset, value)".to_string())); }
        let mut buf = match &args[0] { Value::Bytes(b) => b.clone(), _ => return Err(VmError::runtime_error("mem_write_u32: first arg must be bytes".to_string())) };
        let off = match &args[1] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("mem_write_u32: offset must be numeric".to_string())) };
        let val = match &args[2] { Value::Number(n) => *n as u32, Value::Integer(n) => *n as u32, _ => return Err(VmError::runtime_error("mem_write_u32: value must be numeric".to_string())) };
        if off + 4 > buf.len() { return Err(VmError::runtime_error("mem_write_u32: out of bounds".to_string())); }
        let bytes = val.to_le_bytes();
        buf[off..off+4].copy_from_slice(&bytes);
        Ok(Value::Bytes(buf))
    }

    fn mem_write_u64(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 3 { return Err(VmError::runtime_error("mem_write_u64(buf, offset, value)".to_string())); }
        let mut buf = match &args[0] { Value::Bytes(b) => b.clone(), _ => return Err(VmError::runtime_error("mem_write_u64: first arg must be bytes".to_string())) };
        let off = match &args[1] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("mem_write_u64: offset must be numeric".to_string())) };
        let val = match &args[2] { Value::Number(n) => *n as u64, Value::Integer(n) => *n as u64, _ => return Err(VmError::runtime_error("mem_write_u64: value must be numeric".to_string())) };
        if off + 8 > buf.len() { return Err(VmError::runtime_error("mem_write_u64: out of bounds".to_string())); }
        let bytes = val.to_le_bytes();
        buf[off..off+8].copy_from_slice(&bytes);
        Ok(Value::Bytes(buf))
    }

    // â”€â”€ Volatile read/write (MMIO simulation â€” uses Bytes buffers) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    fn volatile_read_u8(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("volatile_read_u8(buf, offset)".to_string())); }
        let buf = match &args[0] { Value::Bytes(b) => b, _ => return Err(VmError::runtime_error("volatile_read_u8: first arg must be bytes".to_string())) };
        let off = match &args[1] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("volatile_read_u8: offset must be numeric".to_string())) };
        if off >= buf.len() { return Err(VmError::runtime_error("volatile_read_u8: out of bounds".to_string())); }
        // volatile: read through pointer to prevent optimizer from caching
        let val = unsafe { std::ptr::read_volatile(buf.as_ptr().add(off)) };
        Ok(Value::Integer(val as i64))
    }

    fn volatile_read_u16(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("volatile_read_u16(buf, offset)".to_string())); }
        let buf = match &args[0] { Value::Bytes(b) => b, _ => return Err(VmError::runtime_error("volatile_read_u16: first arg must be bytes".to_string())) };
        let off = match &args[1] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("volatile_read_u16: offset must be numeric".to_string())) };
        if off + 2 > buf.len() { return Err(VmError::runtime_error("volatile_read_u16: out of bounds".to_string())); }
        let val = unsafe { std::ptr::read_volatile((buf.as_ptr().add(off)) as *const u16) };
        Ok(Value::Integer(val as i64))
    }

    fn volatile_read_u32(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("volatile_read_u32(buf, offset)".to_string())); }
        let buf = match &args[0] { Value::Bytes(b) => b, _ => return Err(VmError::runtime_error("volatile_read_u32: first arg must be bytes".to_string())) };
        let off = match &args[1] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("volatile_read_u32: offset must be numeric".to_string())) };
        if off + 4 > buf.len() { return Err(VmError::runtime_error("volatile_read_u32: out of bounds".to_string())); }
        let val = unsafe { std::ptr::read_volatile((buf.as_ptr().add(off)) as *const u32) };
        Ok(Value::Integer(val as i64))
    }

    fn volatile_read_u64(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("volatile_read_u64(buf, offset)".to_string())); }
        let buf = match &args[0] { Value::Bytes(b) => b, _ => return Err(VmError::runtime_error("volatile_read_u64: first arg must be bytes".to_string())) };
        let off = match &args[1] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("volatile_read_u64: offset must be numeric".to_string())) };
        if off + 8 > buf.len() { return Err(VmError::runtime_error("volatile_read_u64: out of bounds".to_string())); }
        let val = unsafe { std::ptr::read_volatile((buf.as_ptr().add(off)) as *const u64) };
        Ok(Value::Integer(val as i64))
    }

    fn volatile_write_u8(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 3 { return Err(VmError::runtime_error("volatile_write_u8(buf, offset, value)".to_string())); }
        let mut buf = match &args[0] { Value::Bytes(b) => b.clone(), _ => return Err(VmError::runtime_error("volatile_write_u8: first arg must be bytes".to_string())) };
        let off = match &args[1] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("volatile_write_u8: offset must be numeric".to_string())) };
        let val = match &args[2] { Value::Number(n) => *n as u8, Value::Integer(n) => *n as u8, _ => return Err(VmError::runtime_error("volatile_write_u8: value must be numeric".to_string())) };
        if off >= buf.len() { return Err(VmError::runtime_error("volatile_write_u8: out of bounds".to_string())); }
        unsafe { std::ptr::write_volatile(buf.as_mut_ptr().add(off), val); }
        Ok(Value::Bytes(buf))
    }

    fn volatile_write_u16(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 3 { return Err(VmError::runtime_error("volatile_write_u16(buf, offset, value)".to_string())); }
        let mut buf = match &args[0] { Value::Bytes(b) => b.clone(), _ => return Err(VmError::runtime_error("volatile_write_u16: first arg must be bytes".to_string())) };
        let off = match &args[1] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("volatile_write_u16: offset must be numeric".to_string())) };
        let val = match &args[2] { Value::Number(n) => *n as u16, Value::Integer(n) => *n as u16, _ => return Err(VmError::runtime_error("volatile_write_u16: value must be numeric".to_string())) };
        if off + 2 > buf.len() { return Err(VmError::runtime_error("volatile_write_u16: out of bounds".to_string())); }
        unsafe { std::ptr::write_volatile((buf.as_mut_ptr().add(off)) as *mut u16, val); }
        Ok(Value::Bytes(buf))
    }

    fn volatile_write_u32(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 3 { return Err(VmError::runtime_error("volatile_write_u32(buf, offset, value)".to_string())); }
        let mut buf = match &args[0] { Value::Bytes(b) => b.clone(), _ => return Err(VmError::runtime_error("volatile_write_u32: first arg must be bytes".to_string())) };
        let off = match &args[1] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("volatile_write_u32: offset must be numeric".to_string())) };
        let val = match &args[2] { Value::Number(n) => *n as u32, Value::Integer(n) => *n as u32, _ => return Err(VmError::runtime_error("volatile_write_u32: value must be numeric".to_string())) };
        if off + 4 > buf.len() { return Err(VmError::runtime_error("volatile_write_u32: out of bounds".to_string())); }
        unsafe { std::ptr::write_volatile((buf.as_mut_ptr().add(off)) as *mut u32, val); }
        Ok(Value::Bytes(buf))
    }

    fn volatile_write_u64(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 3 { return Err(VmError::runtime_error("volatile_write_u64(buf, offset, value)".to_string())); }
        let mut buf = match &args[0] { Value::Bytes(b) => b.clone(), _ => return Err(VmError::runtime_error("volatile_write_u64: first arg must be bytes".to_string())) };
        let off = match &args[1] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("volatile_write_u64: offset must be numeric".to_string())) };
        let val = match &args[2] { Value::Number(n) => *n as u64, Value::Integer(n) => *n as u64, _ => return Err(VmError::runtime_error("volatile_write_u64: value must be numeric".to_string())) };
        if off + 8 > buf.len() { return Err(VmError::runtime_error("volatile_write_u64: out of bounds".to_string())); }
        unsafe { std::ptr::write_volatile((buf.as_mut_ptr().add(off)) as *mut u64, val); }
        Ok(Value::Bytes(buf))
    }

    // â”€â”€ I/O port read/write (x86 â€” simulated via Bytes MMIO region) â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[cfg(target_arch = "x86_64")]
    fn io_port_in_u8(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("io_port_in_u8(port)".to_string())); }
        let port = match &args[0] { Value::Number(n) => *n as u16, Value::Integer(n) => *n as u16, _ => return Err(VmError::runtime_error("io_port_in_u8: port must be numeric".to_string())) };
        let val: u8;
        unsafe { std::arch::asm!("in al, dx", out("al") val, in("dx") port, options(nostack, nomem)); }
        Ok(Value::Integer(val as i64))
    }
    #[cfg(not(target_arch = "x86_64"))]
    fn io_port_in_u8(args: &[Value]) -> Result<Value, VmError> {
        let _ = args;
        Err(VmError::runtime_error("io_port_in_u8: only available on x86_64".to_string()))
    }

    #[cfg(target_arch = "x86_64")]
    fn io_port_in_u16(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("io_port_in_u16(port)".to_string())); }
        let port = match &args[0] { Value::Number(n) => *n as u16, Value::Integer(n) => *n as u16, _ => return Err(VmError::runtime_error("io_port_in_u16: port must be numeric".to_string())) };
        let val: u16;
        unsafe { std::arch::asm!("in ax, dx", out("ax") val, in("dx") port, options(nostack, nomem)); }
        Ok(Value::Integer(val as i64))
    }
    #[cfg(not(target_arch = "x86_64"))]
    fn io_port_in_u16(args: &[Value]) -> Result<Value, VmError> {
        let _ = args;
        Err(VmError::runtime_error("io_port_in_u16: only available on x86_64".to_string()))
    }

    #[cfg(target_arch = "x86_64")]
    fn io_port_out_u8(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("io_port_out_u8(port, value)".to_string())); }
        let port = match &args[0] { Value::Number(n) => *n as u16, Value::Integer(n) => *n as u16, _ => return Err(VmError::runtime_error("io_port_out_u8: port must be numeric".to_string())) };
        let val = match &args[1] { Value::Number(n) => *n as u8, Value::Integer(n) => *n as u8, _ => return Err(VmError::runtime_error("io_port_out_u8: value must be numeric".to_string())) };
        unsafe { std::arch::asm!("out dx, al", in("dx") port, in("al") val, options(nostack, nomem)); }
        Ok(Value::Null)
    }
    #[cfg(not(target_arch = "x86_64"))]
    fn io_port_out_u8(args: &[Value]) -> Result<Value, VmError> {
        let _ = args;
        Err(VmError::runtime_error("io_port_out_u8: only available on x86_64".to_string()))
    }

    #[cfg(target_arch = "x86_64")]
    fn io_port_out_u16(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("io_port_out_u16(port, value)".to_string())); }
        let port = match &args[0] { Value::Number(n) => *n as u16, Value::Integer(n) => *n as u16, _ => return Err(VmError::runtime_error("io_port_out_u16: port must be numeric".to_string())) };
        let val = match &args[1] { Value::Number(n) => *n as u16, Value::Integer(n) => *n as u16, _ => return Err(VmError::runtime_error("io_port_out_u16: value must be numeric".to_string())) };
        unsafe { std::arch::asm!("out dx, ax", in("dx") port, in("ax") val, options(nostack, nomem)); }
        Ok(Value::Null)
    }
    #[cfg(not(target_arch = "x86_64"))]
    fn io_port_out_u16(args: &[Value]) -> Result<Value, VmError> {
        let _ = args;
        Err(VmError::runtime_error("io_port_out_u16: only available on x86_64".to_string()))
    }

    // â”€â”€ SHA-256 (pure Rust, zero deps) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    fn sha256(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("sha256 expects 1 argument".to_string())); }
        let data = match &args[0] {
            Value::Str(s) => s.as_bytes().to_vec(),
            Value::Bytes(b) => b.clone(),
            _ => return Err(VmError::runtime_error("sha256: argument must be string or bytes".to_string())),
        };
        let hash = sha256_digest(&data);
        let hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
        Ok(Value::Str(hex))
    }

    fn sha256_bytes(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("sha256_bytes expects 1 argument".to_string())); }
        let data = match &args[0] {
            Value::Str(s) => s.as_bytes().to_vec(),
            Value::Bytes(b) => b.clone(),
            _ => return Err(VmError::runtime_error("sha256_bytes: argument must be string or bytes".to_string())),
        };
        Ok(Value::Bytes(sha256_digest(&data).to_vec()))
    }

    // â”€â”€ mmap / executable memory â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[cfg(target_os = "windows")]
    fn mmap_alloc(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("mmap_alloc(size)".to_string())); }
        let size = match &args[0] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("mmap_alloc: size must be numeric".to_string())) };
        if size > 1_073_741_824 { return Err(VmError::runtime_error("mmap_alloc: max 1GB".to_string())); }
        // PAGE_READWRITE = 0x04
        let ptr = unsafe {
            winapi_virtual_alloc(std::ptr::null_mut(), size, 0x3000 /* MEM_COMMIT | MEM_RESERVE */, 0x04)
        };
        if ptr.is_null() { return Err(VmError::runtime_error("mmap_alloc: VirtualAlloc failed".to_string())); }
        Ok(Value::Pointer(ptr as usize))
    }
    #[cfg(not(target_os = "windows"))]
    fn mmap_alloc(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("mmap_alloc(size)".to_string())); }
        let size = match &args[0] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("mmap_alloc: size must be numeric".to_string())) };
        if size > 1_073_741_824 { return Err(VmError::runtime_error("mmap_alloc: max 1GB".to_string())); }
        let ptr = unsafe {
            libc_mmap(std::ptr::null_mut(), size, 0x3 /* PROT_READ|PROT_WRITE */, 0x22 /* MAP_PRIVATE|MAP_ANONYMOUS */, -1, 0)
        };
        if ptr == usize::MAX as *mut u8 { return Err(VmError::runtime_error("mmap_alloc: mmap failed".to_string())); }
        Ok(Value::Pointer(ptr as usize))
    }

    #[cfg(target_os = "windows")]
    fn mmap_free(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("mmap_free(ptr)".to_string())); }
        let ptr = match &args[0] { Value::Pointer(p) => *p, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("mmap_free: argument must be pointer".to_string())) };
        let ok = unsafe { winapi_virtual_free(ptr as *mut u8, 0, 0x8000 /* MEM_RELEASE */) };
        if ok == 0 { return Err(VmError::runtime_error("mmap_free: VirtualFree failed".to_string())); }
        Ok(Value::Null)
    }
    #[cfg(not(target_os = "windows"))]
    fn mmap_free(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("mmap_free(ptr, size)".to_string())); }
        let ptr = match &args[0] { Value::Pointer(p) => *p, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("mmap_free: argument must be pointer".to_string())) };
        let size = match &args[1] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("mmap_free: size must be numeric".to_string())) };
        unsafe { libc_munmap(ptr as *mut u8, size); }
        Ok(Value::Null)
    }

    fn mmap_write(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 3 { return Err(VmError::runtime_error("mmap_write(ptr, offset, bytes)".to_string())); }
        let base = match &args[0] { Value::Pointer(p) => *p, _ => return Err(VmError::runtime_error("mmap_write: first arg must be pointer".to_string())) };
        let off = match &args[1] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("mmap_write: offset must be numeric".to_string())) };
        let data = match &args[2] { Value::Bytes(b) => b, _ => return Err(VmError::runtime_error("mmap_write: data must be bytes".to_string())) };
        unsafe {
            let dst = (base + off) as *mut u8;
            std::ptr::copy_nonoverlapping(data.as_ptr(), dst, data.len());
        }
        Ok(Value::Null)
    }

    fn mmap_read(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 3 { return Err(VmError::runtime_error("mmap_read(ptr, offset, len)".to_string())); }
        let base = match &args[0] { Value::Pointer(p) => *p, _ => return Err(VmError::runtime_error("mmap_read: first arg must be pointer".to_string())) };
        let off = match &args[1] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("mmap_read: offset must be numeric".to_string())) };
        let len = match &args[2] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("mmap_read: len must be numeric".to_string())) };
        if len > 1_073_741_824 { return Err(VmError::runtime_error("mmap_read: max 1GB".to_string())); }
        let mut buf = vec![0u8; len];
        unsafe {
            let src = (base + off) as *const u8;
            std::ptr::copy_nonoverlapping(src, buf.as_mut_ptr(), len);
        }
        Ok(Value::Bytes(buf))
    }

    #[cfg(target_os = "windows")]
    fn mmap_exec(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("mmap_exec(ptr, size) â€” make memory executable".to_string())); }
        let ptr = match &args[0] { Value::Pointer(p) => *p, _ => return Err(VmError::runtime_error("mmap_exec: first arg must be pointer".to_string())) };
        let size = match &args[1] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("mmap_exec: size must be numeric".to_string())) };
        let mut old_protect: u32 = 0;
        // PAGE_EXECUTE_READWRITE = 0x40
        let ok = unsafe { winapi_virtual_protect(ptr as *mut u8, size, 0x40, &mut old_protect) };
        if ok == 0 { return Err(VmError::runtime_error("mmap_exec: VirtualProtect failed".to_string())); }
        Ok(Value::Null)
    }
    #[cfg(not(target_os = "windows"))]
    fn mmap_exec(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("mmap_exec(ptr, size) â€” make memory executable".to_string())); }
        let ptr = match &args[0] { Value::Pointer(p) => *p, _ => return Err(VmError::runtime_error("mmap_exec: first arg must be pointer".to_string())) };
        let size = match &args[1] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("mmap_exec: size must be numeric".to_string())) };
        // PROT_READ|PROT_WRITE|PROT_EXEC = 7
        let ok = unsafe { libc_mprotect(ptr as *mut u8, size, 7) };
        if ok != 0 { return Err(VmError::runtime_error("mmap_exec: mprotect failed".to_string())); }
        Ok(Value::Null)
    }

    // â”€â”€ Integer â†” Bytes endian conversions â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    fn int_to_bytes_le(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("int_to_bytes_le(n)".to_string())); }
        let n = match &args[0] { Value::Number(v) => *v as i64, Value::Integer(v) => *v, _ => return Err(VmError::runtime_error("int_to_bytes_le: argument must be numeric".to_string())) };
        Ok(Value::Bytes(n.to_le_bytes().to_vec()))
    }

    fn int_to_bytes_be(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("int_to_bytes_be(n)".to_string())); }
        let n = match &args[0] { Value::Number(v) => *v as i64, Value::Integer(v) => *v, _ => return Err(VmError::runtime_error("int_to_bytes_be: argument must be numeric".to_string())) };
        Ok(Value::Bytes(n.to_be_bytes().to_vec()))
    }

    fn bytes_to_int_le(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("bytes_to_int_le(buf)".to_string())); }
        let buf = match &args[0] { Value::Bytes(b) => b, _ => return Err(VmError::runtime_error("bytes_to_int_le: argument must be bytes".to_string())) };
        let mut arr = [0u8; 8];
        let len = buf.len().min(8);
        arr[..len].copy_from_slice(&buf[..len]);
        Ok(Value::Integer(i64::from_le_bytes(arr)))
    }

    fn bytes_to_int_be(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("bytes_to_int_be(buf)".to_string())); }
        let buf = match &args[0] { Value::Bytes(b) => b, _ => return Err(VmError::runtime_error("bytes_to_int_be: argument must be bytes".to_string())) };
        let mut arr = [0u8; 8];
        let len = buf.len().min(8);
        arr[8 - len..].copy_from_slice(&buf[..len]);
        Ok(Value::Integer(i64::from_be_bytes(arr)))
    }

    // â”€â”€ OS / Process â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    fn cli_args(_args: &[Value]) -> Result<Value, VmError> {
        let args: Vec<Value> = std::env::args().map(|a| Value::Str(a)).collect();
        Ok(Value::Array(crate::value::SharedArray::new(args)))
    }

    fn env_get(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("env_get(name)".to_string())); }
        let name = match &args[0] { Value::Str(s) => s, _ => return Err(VmError::runtime_error("env_get: name must be string".to_string())) };
        match std::env::var(name) {
            Ok(val) => Ok(Value::Str(val)),
            Err(_) => Ok(Value::Null),
        }
    }

    fn env_set(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("env_set(name, value)".to_string())); }
        let name = match &args[0] { Value::Str(s) => s.clone(), _ => return Err(VmError::runtime_error("env_set: name must be string".to_string())) };
        let val = match &args[1] { Value::Str(s) => s.clone(), _ => format!("{}", args[1]) };
        std::env::set_var(&name, &val);
        Ok(Value::Null)
    }

    fn process_exit(args: &[Value]) -> Result<Value, VmError> {
        let code = if args.is_empty() { 0 } else {
            match &args[0] { Value::Number(n) => *n as i32, Value::Integer(n) => *n as i32, _ => 1 }
        };
        std::process::exit(code);
    }

    fn errno(_args: &[Value]) -> Result<Value, VmError> {
        Ok(Value::Integer(std::io::Error::last_os_error().raw_os_error().unwrap_or(0) as i64))
    }

    // â”€â”€ Sizeof / Alignof â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    fn sizeof_val(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("sizeof(value)".to_string())); }
        let size = match &args[0] {
            Value::Number(_) => 8,       // f64
            Value::Integer(_) => 8,      // i64
            Value::Bool(_) => 1,
            Value::Str(s) => s.len(),
            Value::Bytes(b) => b.len(),
            Value::Pointer(_) => std::mem::size_of::<usize>(),
            Value::Null => 0,
            _ => std::mem::size_of::<Value>(),
        };
        Ok(Value::Integer(size as i64))
    }

    fn alignof_val(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("alignof(type_name)".to_string())); }
        let name = match &args[0] { Value::Str(s) => s.as_str(), _ => return Err(VmError::runtime_error("alignof: argument must be type name string".to_string())) };
        let align = match name {
            "u8" | "i8" | "bool" => 1,
            "u16" | "i16" => 2,
            "u32" | "i32" | "f32" => 4,
            "u64" | "i64" | "f64" | "pointer" | "usize" => 8,
            _ => std::mem::align_of::<Value>(),
        };
        Ok(Value::Integer(align as i64))
    }

    // â”€â”€ Atomics (using std::sync::atomic) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    fn atomic_load(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 2 { return Err(VmError::runtime_error("atomic_load(buf, offset)".to_string())); }
        let buf = match &args[0] { Value::Bytes(b) => b, _ => return Err(VmError::runtime_error("atomic_load: first arg must be bytes".to_string())) };
        let off = match &args[1] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("atomic_load: offset must be numeric".to_string())) };
        if off + 8 > buf.len() { return Err(VmError::runtime_error("atomic_load: out of bounds".to_string())); }
        let ptr = buf[off..off+8].as_ptr() as *const std::sync::atomic::AtomicU64;
        let val = unsafe { (*ptr).load(std::sync::atomic::Ordering::SeqCst) };
        Ok(Value::Integer(val as i64))
    }

    fn atomic_store(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 3 { return Err(VmError::runtime_error("atomic_store(buf, offset, value)".to_string())); }
        let mut buf = match &args[0] { Value::Bytes(b) => b.clone(), _ => return Err(VmError::runtime_error("atomic_store: first arg must be bytes".to_string())) };
        let off = match &args[1] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("atomic_store: offset must be numeric".to_string())) };
        let val = match &args[2] { Value::Number(n) => *n as u64, Value::Integer(n) => *n as u64, _ => return Err(VmError::runtime_error("atomic_store: value must be numeric".to_string())) };
        if off + 8 > buf.len() { return Err(VmError::runtime_error("atomic_store: out of bounds".to_string())); }
        let ptr = buf[off..off+8].as_mut_ptr() as *mut std::sync::atomic::AtomicU64;
        unsafe { (*ptr).store(val, std::sync::atomic::Ordering::SeqCst); }
        Ok(Value::Bytes(buf))
    }

    fn atomic_cas(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 4 { return Err(VmError::runtime_error("atomic_cas(buf, offset, expected, desired)".to_string())); }
        let mut buf = match &args[0] { Value::Bytes(b) => b.clone(), _ => return Err(VmError::runtime_error("atomic_cas: first arg must be bytes".to_string())) };
        let off = match &args[1] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("atomic_cas: offset must be numeric".to_string())) };
        let expected = match &args[2] { Value::Number(n) => *n as u64, Value::Integer(n) => *n as u64, _ => return Err(VmError::runtime_error("atomic_cas: expected must be numeric".to_string())) };
        let desired = match &args[3] { Value::Number(n) => *n as u64, Value::Integer(n) => *n as u64, _ => return Err(VmError::runtime_error("atomic_cas: desired must be numeric".to_string())) };
        if off + 8 > buf.len() { return Err(VmError::runtime_error("atomic_cas: out of bounds".to_string())); }
        let ptr = buf[off..off+8].as_mut_ptr() as *mut std::sync::atomic::AtomicU64;
        let result = unsafe { (*ptr).compare_exchange(expected, desired, std::sync::atomic::Ordering::SeqCst, std::sync::atomic::Ordering::SeqCst) };
        match result {
            Ok(v) => {
                let mut dict = std::collections::HashMap::new();
                dict.insert("success".to_string(), Value::Bool(true));
                dict.insert("old".to_string(), Value::Integer(v as i64));
                dict.insert("buf".to_string(), Value::Bytes(buf));
                Ok(Value::Dict(Box::new(dict)))
            }
            Err(v) => {
                let mut dict = std::collections::HashMap::new();
                dict.insert("success".to_string(), Value::Bool(false));
                dict.insert("old".to_string(), Value::Integer(v as i64));
                dict.insert("buf".to_string(), Value::Bytes(buf));
                Ok(Value::Dict(Box::new(dict)))
            }
        }
    }

    fn atomic_add(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 3 { return Err(VmError::runtime_error("atomic_add(buf, offset, value)".to_string())); }
        let mut buf = match &args[0] { Value::Bytes(b) => b.clone(), _ => return Err(VmError::runtime_error("atomic_add: first arg must be bytes".to_string())) };
        let off = match &args[1] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("atomic_add: offset must be numeric".to_string())) };
        let val = match &args[2] { Value::Number(n) => *n as u64, Value::Integer(n) => *n as u64, _ => return Err(VmError::runtime_error("atomic_add: value must be numeric".to_string())) };
        if off + 8 > buf.len() { return Err(VmError::runtime_error("atomic_add: out of bounds".to_string())); }
        let ptr = buf[off..off+8].as_mut_ptr() as *mut std::sync::atomic::AtomicU64;
        let old = unsafe { (*ptr).fetch_add(val, std::sync::atomic::Ordering::SeqCst) };
        let mut dict = std::collections::HashMap::new();
        dict.insert("old".to_string(), Value::Integer(old as i64));
        dict.insert("buf".to_string(), Value::Bytes(buf));
        Ok(Value::Dict(Box::new(dict)))
    }

    // â”€â”€ CPU control primitives â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    fn interrupts_disable(_args: &[Value]) -> Result<Value, VmError> {
        // In user-space, we simulate this. In a real kernel, this would be `cli`.
        #[cfg(target_arch = "x86_64")]
        unsafe { std::arch::asm!("cli", options(nostack, nomem)); }
        Ok(Value::Null)
    }

    fn interrupts_enable(_args: &[Value]) -> Result<Value, VmError> {
        #[cfg(target_arch = "x86_64")]
        unsafe { std::arch::asm!("sti", options(nostack, nomem)); }
        Ok(Value::Null)
    }

    fn wfi(_args: &[Value]) -> Result<Value, VmError> {
        // Wait For Interrupt â€” x86: hlt, ARM: wfi
        #[cfg(target_arch = "x86_64")]
        unsafe { std::arch::asm!("hlt", options(nostack, nomem)); }
        Ok(Value::Null)
    }

    fn fence(_args: &[Value]) -> Result<Value, VmError> {
        std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);
        Ok(Value::Null)
    }

    // â”€â”€ Disk raw block I/O (uses file-backed simulation) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    fn disk_read_block(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 3 { return Err(VmError::runtime_error("disk_read_block(path, block_num, block_size)".to_string())); }
        let path = match &args[0] { Value::Str(s) => s.clone(), _ => return Err(VmError::runtime_error("disk_read_block: path must be string".to_string())) };
        let block_num = match &args[1] { Value::Number(n) => *n as u64, Value::Integer(n) => *n as u64, _ => return Err(VmError::runtime_error("disk_read_block: block_num must be numeric".to_string())) };
        let block_size = match &args[2] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("disk_read_block: block_size must be numeric".to_string())) };
        if block_size > 1_048_576 { return Err(VmError::runtime_error("disk_read_block: max block size 1MB".to_string())); }
        use std::io::{Read, Seek, SeekFrom};
        let mut f = std::fs::File::open(&path).map_err(|e| VmError::runtime_error(format!("disk_read_block: {}", e)))?;
        f.seek(SeekFrom::Start(block_num * block_size as u64)).map_err(|e| VmError::runtime_error(format!("disk_read_block: seek error: {}", e)))?;
        let mut buf = vec![0u8; block_size];
        let n = f.read(&mut buf).map_err(|e| VmError::runtime_error(format!("disk_read_block: read error: {}", e)))?;
        buf.truncate(n);
        Ok(Value::Bytes(buf))
    }

    fn disk_write_block(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 3 { return Err(VmError::runtime_error("disk_write_block(path, block_num, data)".to_string())); }
        let path = match &args[0] { Value::Str(s) => s.clone(), _ => return Err(VmError::runtime_error("disk_write_block: path must be string".to_string())) };
        let block_num = match &args[1] { Value::Number(n) => *n as u64, Value::Integer(n) => *n as u64, _ => return Err(VmError::runtime_error("disk_write_block: block_num must be numeric".to_string())) };
        let data = match &args[2] { Value::Bytes(b) => b, _ => return Err(VmError::runtime_error("disk_write_block: data must be bytes".to_string())) };
        use std::io::{Write, Seek, SeekFrom};
        let mut f = std::fs::OpenOptions::new().write(true).create(true).open(&path).map_err(|e| VmError::runtime_error(format!("disk_write_block: {}", e)))?;
        f.seek(SeekFrom::Start(block_num * data.len() as u64)).map_err(|e| VmError::runtime_error(format!("disk_write_block: seek error: {}", e)))?;
        f.write_all(data).map_err(|e| VmError::runtime_error(format!("disk_write_block: write error: {}", e)))?;
        Ok(Value::Integer(data.len() as i64))
    }

    // â”€â”€ Page table simulation â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    fn page_alloc(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 { return Err(VmError::runtime_error("page_alloc(num_pages)".to_string())); }
        let num = match &args[0] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("page_alloc: argument must be numeric".to_string())) };
        let size = num * 4096; // 4KB pages
        if size > 1_073_741_824 { return Err(VmError::runtime_error("page_alloc: max 1GB".to_string())); }
        Ok(Value::Bytes(vec![0u8; size]))
    }

    fn page_free(_args: &[Value]) -> Result<Value, VmError> {
        // In VM context, just drop the bytes â€” Rust GC handles it
        Ok(Value::Null)
    }

    fn page_map(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 3 { return Err(VmError::runtime_error("page_map(page_table, virtual_page, physical_page)".to_string())); }
        let mut pt = match &args[0] { Value::Bytes(b) => b.clone(), _ => return Err(VmError::runtime_error("page_map: page_table must be bytes".to_string())) };
        let virt = match &args[1] { Value::Number(n) => *n as usize, Value::Integer(n) => *n as usize, _ => return Err(VmError::runtime_error("page_map: virtual_page must be numeric".to_string())) };
        let phys = match &args[2] { Value::Number(n) => *n as u64, Value::Integer(n) => *n as u64, _ => return Err(VmError::runtime_error("page_map: physical_page must be numeric".to_string())) };
        // Each page table entry = 8 bytes (u64): physical address + flags
        let entry_off = virt * 8;
        if entry_off + 8 > pt.len() {
            pt.resize(entry_off + 8, 0);
        }
        let entry = phys | 0x03; // Present + Writable flags
        let bytes = entry.to_le_bytes();
        pt[entry_off..entry_off+8].copy_from_slice(&bytes);
        Ok(Value::Bytes(pt))
    }

    // ===== v2.3: OS-Level Hardware Primitives =====

    /// cpuid(leaf) — simulated CPUID instruction, returns dict with eax/ebx/ecx/edx
    fn cpuid(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error("cpuid(leaf) expects 1 argument".to_string()));
        }
        let leaf = match &args[0] {
            Value::Number(n) => *n as u32,
            Value::Integer(n) => *n as u32,
            _ => return Err(VmError::runtime_error("cpuid: leaf must be numeric".to_string())),
        };
        let mut dict = std::collections::HashMap::new();
        match leaf {
            0 => {
                // Max leaf + vendor string "KillerCPU!"
                dict.insert("eax".to_string(), Value::Number(10.0));
                dict.insert("ebx".to_string(), Value::Number(0x6C694B as f64));   // "Kil"
                dict.insert("ecx".to_string(), Value::Number(0x5021 as f64));     // "!P"
                dict.insert("edx".to_string(), Value::Number(0x4370 as f64));     // "pC"
            }
            1 => {
                // Family/model/stepping + feature flags
                dict.insert("eax".to_string(), Value::Number(0x000806EC as f64)); // Family 8, Model 14
                dict.insert("ebx".to_string(), Value::Number(0x00100800 as f64)); // CLFLUSH=8, count=16
                dict.insert("ecx".to_string(), Value::Number(0x7FFAFBBF as f64)); // SSE3,SSE4,AVX,etc.
                dict.insert("edx".to_string(), Value::Number(0xBFEBFBFF_u32 as f64)); // FPU,TSC,MSR,etc.
            }
            _ => {
                dict.insert("eax".to_string(), Value::Number(0.0));
                dict.insert("ebx".to_string(), Value::Number(0.0));
                dict.insert("ecx".to_string(), Value::Number(0.0));
                dict.insert("edx".to_string(), Value::Number(0.0));
            }
        }
        Ok(Value::Dict(Box::new(dict)))
    }

    /// rdtsc() — simulated Read Time-Stamp Counter, returns monotonic nanosecond count
    fn rdtsc(args: &[Value]) -> Result<Value, VmError> {
        if !args.is_empty() {
            return Err(VmError::runtime_error("rdtsc() takes no arguments".to_string()));
        }
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as f64;
        Ok(Value::Number(nanos))
    }

    /// gdt_encode(base, limit, access, flags) — encode a GDT segment descriptor as 8 bytes
    fn gdt_encode(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 4 {
            return Err(VmError::runtime_error(
                "gdt_encode(base, limit, access, flags) expects 4 arguments".to_string(),
            ));
        }
        let base = match &args[0] {
            Value::Number(n) => *n as u32,
            Value::Integer(n) => *n as u32,
            _ => return Err(VmError::runtime_error("gdt_encode: base must be numeric".to_string())),
        };
        let limit = match &args[1] {
            Value::Number(n) => *n as u32,
            Value::Integer(n) => *n as u32,
            _ => return Err(VmError::runtime_error("gdt_encode: limit must be numeric".to_string())),
        };
        let access = match &args[2] {
            Value::Number(n) => *n as u8,
            Value::Integer(n) => *n as u8,
            _ => return Err(VmError::runtime_error("gdt_encode: access must be numeric".to_string())),
        };
        let flags = match &args[3] {
            Value::Number(n) => *n as u8,
            Value::Integer(n) => *n as u8,
            _ => return Err(VmError::runtime_error("gdt_encode: flags must be numeric".to_string())),
        };
        // Standard x86 GDT descriptor encoding (8 bytes)
        let mut desc = vec![0u8; 8];
        desc[0] = (limit & 0xFF) as u8;
        desc[1] = ((limit >> 8) & 0xFF) as u8;
        desc[2] = (base & 0xFF) as u8;
        desc[3] = ((base >> 8) & 0xFF) as u8;
        desc[4] = ((base >> 16) & 0xFF) as u8;
        desc[5] = access;
        desc[6] = ((limit >> 16) & 0x0F) as u8 | ((flags & 0x0F) << 4);
        desc[7] = ((base >> 24) & 0xFF) as u8;
        Ok(Value::Bytes(desc))
    }

    /// idt_encode(offset, selector, type_attr) — encode an IDT gate descriptor as 16 bytes (x86-64)
    fn idt_encode(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 3 {
            return Err(VmError::runtime_error(
                "idt_encode(offset, selector, type_attr) expects 3 arguments".to_string(),
            ));
        }
        let offset = match &args[0] {
            Value::Number(n) => *n as u64,
            Value::Integer(n) => *n as u64,
            _ => return Err(VmError::runtime_error("idt_encode: offset must be numeric".to_string())),
        };
        let selector = match &args[1] {
            Value::Number(n) => *n as u16,
            Value::Integer(n) => *n as u16,
            _ => return Err(VmError::runtime_error("idt_encode: selector must be numeric".to_string())),
        };
        let type_attr = match &args[2] {
            Value::Number(n) => *n as u8,
            Value::Integer(n) => *n as u8,
            _ => return Err(VmError::runtime_error("idt_encode: type_attr must be numeric".to_string())),
        };
        // x86-64 IDT gate descriptor (16 bytes)
        let mut gate = vec![0u8; 16];
        gate[0] = (offset & 0xFF) as u8;
        gate[1] = ((offset >> 8) & 0xFF) as u8;
        gate[2] = (selector & 0xFF) as u8;
        gate[3] = ((selector >> 8) & 0xFF) as u8;
        gate[4] = 0; // IST (0 = no IST)
        gate[5] = type_attr;
        gate[6] = ((offset >> 16) & 0xFF) as u8;
        gate[7] = ((offset >> 24) & 0xFF) as u8;
        // bytes 8-11: upper 32 bits of offset
        gate[8] = ((offset >> 32) & 0xFF) as u8;
        gate[9] = ((offset >> 40) & 0xFF) as u8;
        gate[10] = ((offset >> 48) & 0xFF) as u8;
        gate[11] = ((offset >> 56) & 0xFF) as u8;
        // bytes 12-15: reserved (must be 0)
        Ok(Value::Bytes(gate))
    }

    /// call_native(code) — simulate executing native x86 code, returns EAX value as Number
    fn call_native(args: &[Value]) -> Result<Value, VmError> {
        if args.len() != 1 {
            return Err(VmError::runtime_error("call_native(code) expects 1 argument".to_string()));
        }
        let code: Vec<u8> = match &args[0] {
            Value::Bytes(b) => b.clone(),
            Value::Pointer(p) => {
                // Read code from the mmap'd memory region
                // Read up to 4096 bytes (one page) — standard mmap_alloc size
                let addr = *p;
                let len = 4096usize;
                let mut buf = vec![0u8; len];
                unsafe {
                    std::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr(), len);
                }
                buf
            }
            _ => return Err(VmError::runtime_error("call_native: argument must be bytes or pointer".to_string())),
        };
        // Simulate: scan for MOV EAX, imm32 (opcode 0xB8) and extract the immediate
        let mut eax: u32 = 0;
        let mut i = 0;
        while i < code.len() {
            if code[i] == 0xB8 && i + 4 < code.len() {
                eax = u32::from_le_bytes([code[i+1], code[i+2], code[i+3], code[i+4]]);
                i += 5;
            } else if code[i] == 0xC3 {
                // RET — stop execution
                break;
            } else {
                i += 1;
            }
        }
        Ok(Value::Number(eax as f64))
    }
}

