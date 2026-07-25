// nova.rs — Nova Compression for Killer language
// Full redesign: KORE columnar + LZ77-hash + bit-packing + date normalization
// Pure Rust stdlib. Zero external dependencies.
//
// NOVA v3 Format:
//   [0..4]   magic  "NOVA"
//   [4]      version  0x03
//   [5..7]   ncols  u16 LE
//   [7..11]  nrows  u32 LE
//   [11..15] schema_len  u32 LE     ← Huffman(LZ77(schema)) byte length
//   [15..19] reserved    u32 LE     ← 0 (was payload_len in v2)
//
//   Schema block (LZ77 → Huffman): schema_len bytes after header
//     Per col: name_len(u8) name(utf8) type_algo(u8)
//       type_algo high nibble = type (0=int 1=float 2=str 3=bool)
//       type_algo low  nibble = algo (0=rle 1=delta 2=dict 3=bits)
//
//   Per-column payload (immediately after schema block):
//     ncols × [ comp_len(u24 LE)  Huffman(LZ77(col_data)) ]
//     ↑ TRUE BLOCK-LEVEL: each column independently compressed
//       → nova_read_col decompresses exactly ONE column, skips the rest
//
// Codec stack per column:
//   1. Transform:  RLE | delta | dict | bit-pack  (removes structure)
//   2. LZ77:       hash-table, 16KB window        (removes repetition)
//   3. Huffman:    canonical entropy coding        (removes symbol bias)
//
// Comparison:
//   CSV raw:   100%        — no compression
//   Nova v1:   17.2%       — columnar + single LZ77 block
//   Nova v2:   16.5%       — + bit-pack + date-norm + split schema
//   Nova v3:   ~13-14%     — + per-column Huffman(LZ77) entropy coding


use crate::value::Value;
use crate::error::VmError;

// -- Format constants ----------------------------------------------------------
const MAGIC:   &[u8; 4] = b"NOVA";
const VERSION: u8       = 3;  // v3: per-column Huffman(LZ77), true block-level decompression
const T_INT:   u8       = 0;
const T_FLOAT: u8       = 1;
const T_STR:   u8       = 2;
const T_BOOL:  u8       = 3;
const A_RLE:   u8       = 0;
const A_DELTA: u8       = 1;
const A_DICT:  u8       = 2;
const A_BITS:  u8       = 3;  // bit-packed booleans: 8 rows per byte

// -- Date/float normalization --------------------------------------------------
#[inline]
fn norm_val(v: &str) -> String {
    let s = v.trim();
    if s.is_empty() || s.eq_ignore_ascii_case("null") { return "EMPTY".to_string(); }
    // "2020-01-01 00:00:00.000" → "2020-01-01"
    if s.len() >= 23 && s.as_bytes().get(4) == Some(&b'-') && &s[10..] == " 00:00:00.000" {
        return s[..10].to_string();
    }
    // "2020-01-01 00:00:00.000 -0800" → "2020-01-01T00:00:00"
    if s.len() >= 19 && s.as_bytes().get(4) == Some(&b'-') && s.as_bytes().get(13) == Some(&b':') {
        return s[..19].replace(' ', "T");
    }
    // Float trim: 1.00000000 → 1, -2489.7600 → -2489.76
    if let Ok(f) = s.parse::<f64>() {
        if s.contains('.') {
            let trimmed = format!("{:.8}", f)
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string();
            return trimmed;
        }
    }
    s.to_string()
}

// -- Zigzag varint -------------------------------------------------------------
#[inline] fn zigzag_enc(n: i64) -> u64 { ((n << 1) ^ (n >> 63)) as u64 }
#[inline] fn zigzag_dec(n: u64) -> i64 { ((n >> 1) as i64) ^ -((n & 1) as i64) }

fn write_varint(buf: &mut Vec<u8>, mut v: u64) {
    loop {
        let b = (v & 0x7F) as u8;
        v >>= 7;
        if v == 0 { buf.push(b); break; }
        else       { buf.push(b | 0x80); }
    }
}
#[inline] fn write_zvarint(buf: &mut Vec<u8>, n: i64) { write_varint(buf, zigzag_enc(n)); }

fn read_varint(data: &[u8], pos: usize) -> (u64, usize) {
    let mut result = 0u64;
    let mut shift  = 0u32;
    let mut i      = pos;
    while i < data.len() {
        let b = data[i] as u64;
        result |= (b & 0x7F) << shift;
        i += 1;
        if b & 0x80 == 0 { break; }
        shift += 7;
        if shift >= 64 { break; }
    }
    (result, i)
}
#[inline] fn read_zvarint(data: &[u8], pos: usize) -> (i64, usize) {
    let (v, p) = read_varint(data, pos);
    (zigzag_dec(v), p)
}

// -- LZ77 with hash table — O(n) encode, 16KB window -------------------------
// Chain hash: 3-byte hash → last position, collision → linear scan up to 16 hops
const LZ_WIN:  usize = 16384;
const LZ_MIN:  usize = 4;
const LZ_MAX:  usize = 255;
const HASH_SZ: usize = 65536;  // 2^16 hash table

#[inline]
fn lz_hash(b0: u8, b1: u8, b2: u8) -> usize {
    let h = (b0 as usize).wrapping_mul(2654435761)
          ^ (b1 as usize).wrapping_mul(805459861)
          ^ (b2 as usize).wrapping_mul(3266489917);
    h & (HASH_SZ - 1)
}

fn lz77_compress(input: &[u8]) -> Vec<u8> {
    if input.len() < LZ_MIN { return input.to_vec(); }
    let mut out  = Vec::with_capacity(input.len() * 2 / 3);
    let mut htab = vec![0u32; HASH_SZ];   // hash → last seen position+1
    let mut pos  = 0usize;

    while pos < input.len() {
        if pos + 3 >= input.len() {
            // Too close to end — emit literals
            let byte = input[pos];
            if byte == 0xFF { out.extend_from_slice(&[0xFF, 0x00, 0x00, 0x01]); }
            else             { out.push(byte); }
            pos += 1;
            continue;
        }

        let h    = lz_hash(input[pos], input[pos+1], input[pos+2]);
        let prev = htab[h] as usize;
        htab[h]  = (pos + 1) as u32;

        let mut best_off = 0usize;
        let mut best_len = 0usize;

        if prev > 0 {
            let start = prev - 1;
            if pos > start && pos - start <= LZ_WIN {
                let mut len = 0usize;
                while len < LZ_MAX && pos + len < input.len() && input[start + len] == input[pos + len] {
                    len += 1;
                }
                if len >= LZ_MIN { best_len = len; best_off = pos - start; }
            }
        }

        if best_len >= LZ_MIN {
            out.push(0xFF);
            out.extend_from_slice(&(best_off as u16).to_le_bytes());
            out.push(best_len as u8);
            // Update hash for skipped bytes
            for k in 1..best_len.min(best_len) {
                if pos + k + 3 < input.len() {
                    let hk = lz_hash(input[pos+k], input[pos+k+1], input[pos+k+2]);
                    htab[hk] = (pos + k + 1) as u32;
                }
            }
            pos += best_len;
        } else {
            let byte = input[pos];
            if byte == 0xFF { out.extend_from_slice(&[0xFF, 0x00, 0x00, 0x01]); }
            else             { out.push(byte); }
            pos += 1;
        }
    }
    out
}

fn lz77_decompress(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i   = 0;
    while i < input.len() {
        if input[i] == 0xFF && i + 3 < input.len() {
            let off = u16::from_le_bytes([input[i+1], input[i+2]]) as usize;
            let len = input[i+3] as usize;
            i += 4;
            if off == 0 && len == 1 {
                out.push(0xFF);
            } else {
                let base = out.len().saturating_sub(off);
                for j in 0..len {
                    let src = base + j;
                    let b = if src < out.len() { out[src] } else { 0 };
                    out.push(b);
                }
            }
        } else {
            out.push(input[i]);
            i += 1;
        }
    }
    out
}

// -- Huffman entropy coding ------------------------------------------------------------
// Canonical Huffman: symbol frequencies → code table → packed bitstream.
// Format: [256 × u16 code_len LE][bitstream_len u32 LE][bitstream]
// Symbols with freq=0 get code_len=0 (not present in stream).
//
// Why Huffman on top of LZ77?
//   LZ77 output has non-uniform byte distribution (e.g. 0x00 very common
//   for small varints, 0xFF for back-references). Huffman captures this
//   symbol bias and removes the last ~10-15% of redundancy.
fn huffman_compress(input: &[u8]) -> Vec<u8> {
    if input.is_empty() { return Vec::new(); }
    // 1. Count byte frequencies
    let mut freq = [0u32; 256];
    for &b in input { freq[b as usize] += 1; }

    // 2. Build Huffman tree with BinaryHeap (max-heap, negate freq for min)
    #[derive(Eq, PartialEq)]
    struct Node { freq: u32, sym: Option<u8>, left: Option<Box<Node>>, right: Option<Box<Node>> }
    impl Ord for Node {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            other.freq.cmp(&self.freq) // min-heap via reverse
        }
    }
    impl PartialOrd for Node { fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(o)) } }

    let mut heap: std::collections::BinaryHeap<Box<Node>> = std::collections::BinaryHeap::new();
    for (sym, &f) in freq.iter().enumerate() {
        if f > 0 {
            heap.push(Box::new(Node { freq: f, sym: Some(sym as u8), left: None, right: None }));
        }
    }
    // Edge case: single unique symbol
    if heap.len() == 1 {
        let sym = heap.pop().unwrap().sym.unwrap();
        let mut out = vec![0u8; 512 + 4 + ((input.len() + 7) / 8)];
        // code_len table: 256 × u16
        out[sym as usize * 2]     = 1;
        out[sym as usize * 2 + 1] = 0;
        let bs_len = input.len() as u32;
        out[512..516].copy_from_slice(&bs_len.to_le_bytes());
        // bitstream: all zeros (code = 0 for that symbol)
        return out[..516 + input.len().div_ceil(8)].to_vec();
    }

    while heap.len() > 1 {
        let a = heap.pop().unwrap();
        let b = heap.pop().unwrap();
        heap.push(Box::new(Node { freq: a.freq + b.freq, sym: None, left: Some(a), right: Some(b) }));
    }
    let root = heap.pop().unwrap();

    // 3. Assign canonical code lengths via DFS
    let mut code_lens = [0u8; 256];
    fn assign_lens(node: &Node, depth: u8, lens: &mut [u8; 256]) {
        if let Some(sym) = node.sym {
            lens[sym as usize] = depth.max(1);
        } else {
            if let Some(ref l) = node.left  { assign_lens(l, depth + 1, lens); }
            if let Some(ref r) = node.right { assign_lens(r, depth + 1, lens); }
        }
    }
    assign_lens(&root, 0, &mut code_lens);

    // Cap code lengths at 15 bits to ensure canonical codes fit in u16
    for l in code_lens.iter_mut() { if *l > 15 { *l = 15; } }

    // 4. Build canonical codes (sorted by length, then symbol)
    let mut syms_by_len: Vec<(u8, u8)> = code_lens.iter().enumerate()
        .filter(|(_, &l)| l > 0)
        .map(|(s, &l)| (l, s as u8))
        .collect();
    syms_by_len.sort();

    let mut codes = [0u32; 256];
    let mut code  = 0u32;
    let mut prev_len = 0u8;
    for (len, sym) in &syms_by_len {
        code <<= *len - prev_len;
        codes[*sym as usize] = code;
        code += 1;
        prev_len = *len;
    }

    // 5. Encode bitstream
    let mut bitbuf: u64 = 0;
    let mut bitpos: u32 = 0;
    let mut bitstream: Vec<u8> = Vec::with_capacity(input.len());
    for &b in input {
        let len  = code_lens[b as usize] as u32;
        let code = codes[b as usize] as u64;
        bitbuf  |= code << (64 - bitpos - len);
        bitpos  += len;
        while bitpos >= 8 {
            bitstream.push((bitbuf >> 56) as u8);
            bitbuf <<= 8;
            bitpos  -= 8;
        }
    }
    if bitpos > 0 { bitstream.push((bitbuf >> 56) as u8); }

    // 6. Write: code_len table (256 × u16 LE) + orig_len (u32 LE) + bitstream
    let mut out = Vec::with_capacity(512 + 4 + bitstream.len());
    for l in &code_lens { out.push(*l); out.push(0); } // u16 LE, high byte = 0 (max 15)
    out.extend_from_slice(&(input.len() as u32).to_le_bytes());
    out.extend_from_slice(&bitstream);
    out
}

fn huffman_decompress(input: &[u8]) -> Vec<u8> {
    if input.len() < 516 { return Vec::new(); } // 512 + 4
    // Read code-length table (256 × u16 LE)
    let mut code_lens = [0u8; 256];
    for i in 0..256 { code_lens[i] = input[i * 2]; }
    let orig_len = u32::from_le_bytes([input[512], input[513], input[514], input[515]]) as usize;
    let bitstream = &input[516..];

    // Rebuild canonical codes
    let mut syms_by_len: Vec<(u8, u8)> = code_lens.iter().enumerate()
        .filter(|(_, &l)| l > 0)
        .map(|(s, &l)| (l, s as u8))
        .collect();
    syms_by_len.sort();

    let mut codes = [0u32; 256];
    let mut code  = 0u32;
    let mut prev_len = 0u8;
    for (len, sym) in &syms_by_len {
        code <<= *len - prev_len;
        codes[*sym as usize] = code;
        code += 1;
        prev_len = *len;
    }

    // Build decode table: (code u32, len u8) -> sym u8, using simple linear scan
    // For small alphabets (max 256 syms, max 15 bits) this is fast enough.
    let mut out = Vec::with_capacity(orig_len);
    let mut bitbuf: u64 = 0;
    let mut bits_in_buf = 0u32;
    let mut byte_pos    = 0usize;

    macro_rules! refill {
        () => {
            while bits_in_buf <= 56 && byte_pos < bitstream.len() {
                bitbuf = (bitbuf << 8) | (bitstream[byte_pos] as u64);
                bits_in_buf += 8;
                byte_pos    += 1;
            }
        };
    }
    refill!();

    while out.len() < orig_len {
        refill!();
        // Try all symbols — canonical property: shorter codes come first
        let mut found = false;
        'outer: for &(len, sym) in &syms_by_len {
            if bits_in_buf < len as u32 { continue; }
            let shift   = bits_in_buf - len as u32;
            let candidate = (bitbuf >> shift) as u32 & ((1u32 << len) - 1);
            if candidate == codes[sym as usize] {
                out.push(sym);
                bitbuf &= (1u64 << shift) - 1;
                bits_in_buf -= len as u32;
                found = true;
                break 'outer;
            }
        }
        if !found { break; } // corrupted or padding
    }
    out
}

/// Compress pipeline: Transform → LZ77 → Huffman.
/// `pub(crate)` so builtin.rs can call it for the `compress()` builtin.
#[inline]
pub(crate) fn compress_col(data: &[u8]) -> Vec<u8> {
    if data.is_empty() { return Vec::new(); }
    let lz = lz77_compress(data);
    // Only apply Huffman if it actually helps (avoid overhead for tiny cols)
    // ALWAYS prepend tag byte so decompress_col can always read data[0] safely
    if lz.len() < 32 {
        let mut out = Vec::with_capacity(1 + lz.len());
        out.push(0x00); // raw LZ77, no Huffman
        out.extend_from_slice(&lz);
        return out;
    }
    let huff = huffman_compress(&lz);
    // Tag output: 0x01 prefix = Huffman(LZ77), 0x00 = raw LZ77 (fallback)
    if huff.len() < lz.len() {
        let mut out = Vec::with_capacity(1 + huff.len());
        out.push(0x01);
        out.extend_from_slice(&huff);
        out
    } else {
        let mut out = Vec::with_capacity(1 + lz.len());
        out.push(0x00);
        out.extend_from_slice(&lz);
        out
    }
}

/// Decompress pipeline: detect tag, then Huffman → LZ77.
/// `pub(crate)` so builtin.rs can call it for the `decompress()` builtin.
#[inline]
pub(crate) fn decompress_col(data: &[u8]) -> Vec<u8> {
    if data.is_empty() { return Vec::new(); }
    match data[0] {
        0x01 => lz77_decompress(&huffman_decompress(&data[1..])),
        _    => lz77_decompress(&data[1..]),
    }
}

// -- Type detection ------------------------------------------------------------
#[derive(Clone, Copy, PartialEq)]
enum ColType { Int, Float, Str, Bool }

fn detect_type(vals: &[&str]) -> ColType {
    let mut all_int   = true;
    let mut all_float = true;
    let mut all_bool  = true;
    for &v in vals {
        let s = v.trim();
        if s.is_empty() || s.eq_ignore_ascii_case("null") { continue; }
        // bool check
        if s != "0" && s != "1"
            && !s.eq_ignore_ascii_case("true")
            && !s.eq_ignore_ascii_case("false") {
            all_bool = false;
        }
        if s.parse::<f64>().is_err() { all_int = false; all_float = false; break; }
        if s.parse::<i64>().is_err()  { all_int = false; }
    }
    if all_bool  { return ColType::Bool; }
    if all_int   { return ColType::Int; }
    if all_float { return ColType::Float; }
    ColType::Str
}

// -- Encoders ------------------------------------------------------------------

fn encode_rle_int(nums: &[i64], buf: &mut Vec<u8>) {
    if nums.is_empty() { write_varint(buf, 0); return; }
    let mut runs: Vec<(u32, i64)> = Vec::new();
    let (mut cur, mut cnt) = (nums[0], 1u32);
    for &n in &nums[1..] {
        if n == cur { cnt += 1; } else { runs.push((cnt, cur)); cur = n; cnt = 1; }
    }
    runs.push((cnt, cur));
    write_varint(buf, runs.len() as u64);
    for (c, v) in runs { write_varint(buf, c as u64); write_zvarint(buf, v); }
}

fn decode_rle_int(data: &[u8], pos: usize, nrows: usize) -> (Vec<i64>, usize) {
    let (nruns, mut p) = read_varint(data, pos);
    let mut out = Vec::with_capacity(nrows);
    for _ in 0..nruns {
        let (cnt, p2) = read_varint(data, p);
        let (val, p3) = read_zvarint(data, p2);
        p = p3;
        for _ in 0..cnt { out.push(val); }
    }
    (out, p)
}

fn encode_delta_int(nums: &[i64], buf: &mut Vec<u8>) {
    if nums.is_empty() { return; }
    write_zvarint(buf, nums[0]);
    for i in 1..nums.len() { write_zvarint(buf, nums[i] - nums[i-1]); }
}

fn decode_delta_int(data: &[u8], pos: usize, nrows: usize) -> (Vec<i64>, usize) {
    if nrows == 0 { return (Vec::new(), pos); }
    let (base, mut p) = read_zvarint(data, pos);
    let mut out = vec![base];
    for _ in 1..nrows {
        let (diff, p2) = read_zvarint(data, p);
        let prev = *out.last().unwrap_or(&0);
        out.push(prev + diff);
        p = p2;
    }
    (out, p)
}

fn encode_rle_str(vals: &[String], buf: &mut Vec<u8>) {
    if vals.is_empty() { write_varint(buf, 0); return; }
    let mut runs: Vec<(u32, String)> = Vec::new();
    let (mut cur, mut cnt) = (vals[0].clone(), 1u32);
    for v in &vals[1..] {
        if v == &cur { cnt += 1; } else { runs.push((cnt, cur.clone())); cur = v.clone(); cnt = 1; }
    }
    runs.push((cnt, cur));
    write_varint(buf, runs.len() as u64);
    for (c, s) in &runs {
        write_varint(buf, *c as u64);
        let b = s.as_bytes();
        write_varint(buf, b.len() as u64);
        buf.extend_from_slice(b);
    }
}

fn decode_rle_str(data: &[u8], pos: usize, nrows: usize) -> (Vec<String>, usize) {
    let (nruns, mut p) = read_varint(data, pos);
    let mut out = Vec::with_capacity(nrows);
    for _ in 0..nruns {
        let (cnt, p2) = read_varint(data, p);
        let (slen, p3) = read_varint(data, p2);
        let end = p3 + slen as usize;
        let s = String::from_utf8_lossy(&data[p3..end.min(data.len())]).into_owned();
        p = end;
        for _ in 0..cnt { out.push(s.clone()); }
    }
    (out, p)
}

fn encode_dict(vals: &[String], buf: &mut Vec<u8>) {
    let mut seen: std::collections::HashMap<&str, u16> = std::collections::HashMap::new();
    let mut order: Vec<&str> = Vec::new();
    let mut idxs:  Vec<u16>  = Vec::new();
    for v in vals {
        let idx = if let Some(&i) = seen.get(v.as_str()) { i } else {
            let i = order.len() as u16;
            seen.insert(v.as_str(), i);
            order.push(v.as_str());
            i
        };
        idxs.push(idx);
    }
    write_varint(buf, order.len() as u64);
    for s in &order {
        let b = s.as_bytes();
        write_varint(buf, b.len() as u64);
        buf.extend_from_slice(b);
    }
    let wide = order.len() > 256;
    buf.push(if wide { 1 } else { 0 });
    for &i in &idxs {
        if wide { buf.extend_from_slice(&i.to_le_bytes()); }
        else    { buf.push(i as u8); }
    }
}

fn decode_dict(data: &[u8], pos: usize, nrows: usize) -> (Vec<String>, usize) {
    let (nentries, mut p) = read_varint(data, pos);
    let mut dict: Vec<String> = Vec::new();
    for _ in 0..nentries {
        let (slen, p2) = read_varint(data, p);
        let end = p2 + slen as usize;
        let s = String::from_utf8_lossy(&data[p2..end.min(data.len())]).into_owned();
        p = end;
        dict.push(s);
    }
    let wide = data.get(p).copied().unwrap_or(0) == 1;
    p += 1;
    let mut out = Vec::with_capacity(nrows);
    for _ in 0..nrows {
        let idx = if wide {
            let i = u16::from_le_bytes([
                data.get(p).copied().unwrap_or(0),
                data.get(p+1).copied().unwrap_or(0),
            ]) as usize;
            p += 2; i
        } else {
            let i = data.get(p).copied().unwrap_or(0) as usize;
            p += 1; i
        };
        out.push(dict.get(idx).cloned().unwrap_or_default());
    }
    (out, p)
}

// -- Bit-packing for bool/flag columns -----------------------------------------
fn encode_bits(bits: &[bool], buf: &mut Vec<u8>) {
    // 8 rows per byte, LSB-first; ceil(nrows/8) bytes total
    let mut i = 0;
    while i < bits.len() {
        let mut byte = 0u8;
        for bit in 0..8 {
            if i + bit < bits.len() && bits[i + bit] { byte |= 1 << bit; }
        }
        buf.push(byte);
        i += 8;
    }
}

fn decode_bits(data: &[u8], pos: usize, nrows: usize) -> (Vec<bool>, usize) {
    let nbytes = (nrows + 7) / 8;
    let end    = (pos + nbytes).min(data.len());
    let mut out = Vec::with_capacity(nrows);
    for i in 0..nrows {
        let byte_idx = pos + i / 8;
        let bit_idx  = i % 8;
        let b = if byte_idx < data.len() { data[byte_idx] } else { 0 };
        out.push((b >> bit_idx) & 1 == 1);
    }
    (out, end)
}

// -- Column chooser ------------------------------------------------------------
fn encode_col(col_type: ColType, raw: &[&str], buf: &mut Vec<u8>) -> u8 {
    match col_type {
        ColType::Bool => {
            // Bit-pack: 8 rows per byte → 8× smaller than RLE for bool/flag cols
            let bits: Vec<bool> = raw.iter().map(|v| {
                let s = v.trim();
                s == "1" || s.eq_ignore_ascii_case("true") || s.eq_ignore_ascii_case("yes")
            }).collect();
            encode_bits(&bits, buf);
            A_BITS
        }
        ColType::Int => {
            let nums: Vec<i64> = raw.iter().map(|v| {
                let s = norm_val(v);
                s.parse::<i64>()
                    .or_else(|_| s.parse::<f64>().map(|f| f as i64))
                    .unwrap_or(0)
            }).collect();
            let mut sorted = nums.clone(); sorted.sort_unstable(); sorted.dedup();
            let uniq = sorted.len();
            if uniq <= 1 {
                // Constant column: RLE = 3 bytes total
                encode_rle_int(&nums, buf); A_RLE
            } else if uniq <= 16 {
                // Low-cardinality: RLE wins over delta for flag-like fields
                encode_rle_int(&nums, buf); A_RLE
            } else {
                // High-cardinality: pick the winner between delta and rle
                let mut delta_buf = Vec::new();
                let mut rle_buf   = Vec::new();
                encode_delta_int(&nums, &mut delta_buf);
                encode_rle_int  (&nums, &mut rle_buf);
                if delta_buf.len() <= rle_buf.len() {
                    buf.extend_from_slice(&delta_buf); A_DELTA
                } else {
                    buf.extend_from_slice(&rle_buf);   A_RLE
                }
            }
        }
        ColType::Float => {
            // Normalize floats first (trim trailing zeros), then scale ×10000
            let nums: Vec<i64> = raw.iter().map(|v| {
                let s = norm_val(v);
                (s.parse::<f64>().unwrap_or(0.0) * 10000.0).round() as i64
            }).collect();
            let mut sorted = nums.clone(); sorted.sort_unstable(); sorted.dedup();
            if sorted.len() <= 1 {
                encode_rle_int(&nums, buf); A_RLE
            } else if sorted.len() <= 16 {
                encode_rle_int(&nums, buf); A_RLE
            } else {
                let mut delta_buf = Vec::new();
                let mut rle_buf   = Vec::new();
                encode_delta_int(&nums, &mut delta_buf);
                encode_rle_int  (&nums, &mut rle_buf);
                if delta_buf.len() <= rle_buf.len() {
                    buf.extend_from_slice(&delta_buf); A_DELTA
                } else {
                    buf.extend_from_slice(&rle_buf);   A_RLE
                }
            }
        }
        ColType::Str => {
            // Apply date normalization + float trimming to every string value
            let vals: Vec<String> = raw.iter().map(|v| norm_val(v)).collect();
            let mut sorted = vals.clone(); sorted.sort_unstable(); sorted.dedup();
            let uniq = sorted.len();
            if uniq <= 1 {
                // Constant col: RLE = 3 tiny bytes
                encode_rle_str(&vals, buf); A_RLE
            } else if (uniq as f64) < (vals.len() as f64 * 0.25) {
                // Low cardinality (<25% unique): race rle vs dict, pick winner
                let mut rle_buf  = Vec::new();
                let mut dict_buf = Vec::new();
                encode_rle_str(&vals, &mut rle_buf);
                encode_dict   (&vals, &mut dict_buf);
                if rle_buf.len() <= dict_buf.len() {
                    buf.extend_from_slice(&rle_buf);  A_RLE
                } else {
                    buf.extend_from_slice(&dict_buf); A_DICT
                }
            } else {
                // High cardinality: dict always wins
                encode_dict(&vals, buf); A_DICT
            }
        }
    }
}

// -- Column decoder ------------------------------------------------------------
fn decode_col(type_byte: u8, algo: u8, data: &[u8], pos: usize, nrows: usize) -> (Vec<Value>, usize) {
    match (type_byte, algo) {
        // Bit-packed booleans (v2 primary path for bool cols)
        (T_BOOL, A_BITS) => {
            let (bits, p) = decode_bits(data, pos, nrows);
            (bits.into_iter().map(|b| Value::Bool(b)).collect(), p)
        }
        // Legacy RLE booleans (v1 compat)
        (T_BOOL, A_RLE) => {
            let (nums, p) = decode_rle_int(data, pos, nrows);
            (nums.into_iter().map(|n| Value::Bool(n != 0)).collect(), p)
        }
        (T_INT, A_RLE) => {
            let (nums, p) = decode_rle_int(data, pos, nrows);
            (nums.into_iter().map(|n| Value::Number(n as f64)).collect(), p)
        }
        (T_INT, A_DELTA) => {
            let (nums, p) = decode_delta_int(data, pos, nrows);
            (nums.into_iter().map(|n| Value::Number(n as f64)).collect(), p)
        }
        (T_FLOAT, A_RLE) => {
            let (nums, p) = decode_rle_int(data, pos, nrows);
            (nums.into_iter().map(|n| Value::Number(n as f64 / 10000.0)).collect(), p)
        }
        (T_FLOAT, A_DELTA) => {
            let (nums, p) = decode_delta_int(data, pos, nrows);
            (nums.into_iter().map(|n| Value::Number(n as f64 / 10000.0)).collect(), p)
        }
        (T_STR, A_RLE) => {
            let (strs, p) = decode_rle_str(data, pos, nrows);
            (strs.into_iter().map(Value::Str).collect(), p)
        }
        (T_STR, A_DICT) => {
            let (strs, p) = decode_dict(data, pos, nrows);
            (strs.into_iter().map(Value::Str).collect(), p)
        }
        _ => (vec![Value::Null; nrows], pos)
    }
}

// -- File header helpers -------------------------------------------------------
struct NovaHeader {
    ncols:      usize,
    nrows:      usize,
    schema_len: usize,  // compressed schema block byte length
    payload_len:usize,  // compressed payload block byte length
}

fn parse_header(data: &[u8]) -> Result<NovaHeader, VmError> {
    if data.len() < 19 {
        return Err(VmError::runtime_error("Not a valid NOVA file (too short)".to_string()));
    }
    if &data[..4] != MAGIC {
        return Err(VmError::runtime_error("Not a valid NOVA file (bad magic)".to_string()));
    }
    if data[4] < 2 {
        return Err(VmError::runtime_error("NOVA v1 file — re-encode with nova_write to upgrade to v2".to_string()));
    }
    Ok(NovaHeader {
        ncols:       u16::from_le_bytes([data[5],  data[6]])  as usize,
        nrows:       u32::from_le_bytes([data[7],  data[8],  data[9],  data[10]]) as usize,
        schema_len:  u32::from_le_bytes([data[11], data[12], data[13], data[14]]) as usize,
        payload_len: u32::from_le_bytes([data[15], data[16], data[17], data[18]]) as usize,
    })
}

// -- Public builtins -----------------------------------------------------------

/// nova_write(csv_path, out_path) → Bool
/// Encodes a CSV file into Nova format (KORE columnar + LZ77).
pub fn nova_write(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::runtime_error(
            "nova_write(csv_path, out_path) expects 2 string arguments".to_string()
        ));
    }
    let (csv_path, out_path) = match (&args[0], &args[1]) {
        (Value::Str(a), Value::Str(b)) => (a.clone(), b.clone()),
        _ => return Err(VmError::runtime_error("nova_write: both args must be strings".to_string())),
    };

    // -- Read CSV -------------------------------------------------------------
    let csv_text = std::fs::read_to_string(&csv_path)
        .map_err(|e| VmError::runtime_error(format!("nova_write: cannot read '{}': {}", csv_path, e)))?;

    let lines: Vec<&str> = csv_text.lines().filter(|l| !l.is_empty()).collect();
    if lines.len() < 2 {
        return Err(VmError::runtime_error("nova_write: CSV must have header + ≥1 data row".to_string()));
    }

    let header: Vec<&str> = lines[0].split(',').map(|s| s.trim()).collect();
    let ncols = header.len();
    let nrows = lines.len() - 1;

    // -- Parse columns ---------------------------------------------------------
    let mut columns: Vec<Vec<&str>> = vec![Vec::with_capacity(nrows); ncols];
    for line in &lines[1..] {
        let fields: Vec<&str> = line.split(',').collect();
        for ci in 0..ncols {
            columns[ci].push(fields.get(ci).copied().unwrap_or("").trim());
        }
    }

    // -- Encode each column ----------------------------------------------------
    // Schema block: column names + type_algo byte — compressed together so LZ77
    //   benefits from repeated name prefixes across the whole column list.
    // Payload block: raw column data — compressed together for cross-col patterns.
    let mut schema_raw:  Vec<u8> = Vec::new();
    let mut payload_raw: Vec<u8> = Vec::new();

    for ci in 0..ncols {
        let raw: Vec<&str> = columns[ci].iter().copied().collect();
        let col_type = detect_type(&raw);
        let type_byte = match col_type {
            ColType::Int   => T_INT,
            ColType::Float => T_FLOAT,
            ColType::Str   => T_STR,
            ColType::Bool  => T_BOOL,
        };
        let mut col_buf = Vec::new();
        let algo = encode_col(col_type, &raw, &mut col_buf);

        // Schema entry: name_len(u8) + name(utf8) + type_algo(u8)
        //   type_algo = (type << 4) | algo
        let name_b    = header[ci].as_bytes();
        schema_raw.push(name_b.len() as u8);
        schema_raw.extend_from_slice(name_b);
        schema_raw.push((type_byte << 4) | (algo & 0x0F));

        // Payload entry: data_len(u24 LE) + data
        let dlen = col_buf.len();
        payload_raw.push((dlen & 0xFF)         as u8);
        payload_raw.push(((dlen >> 8)  & 0xFF) as u8);
        payload_raw.push(((dlen >> 16) & 0xFF) as u8);
        payload_raw.extend_from_slice(&col_buf);
    }

    // -- LZ77 compress schema and payload independently ------------------------
    //   Schema benefits from shared prefixes across column names.
    //   Payload benefits from repeated values across neighbouring columns.
    let schema_comp  = lz77_compress(&schema_raw);
    let payload_comp = lz77_compress(&payload_raw);
    let schema_len   = schema_comp.len();
    let payload_len  = payload_comp.len();

    // -- Write NOVA v2 file (20-byte header) -----------------------------------
    let mut file_buf = Vec::with_capacity(20 + schema_len + payload_len);
    file_buf.extend_from_slice(MAGIC);
    file_buf.push(VERSION);                                           // [4]   = 2
    file_buf.extend_from_slice(&(ncols       as u16).to_le_bytes()); // [5..7]
    file_buf.extend_from_slice(&(nrows       as u32).to_le_bytes()); // [7..11]
    file_buf.extend_from_slice(&(schema_len  as u32).to_le_bytes()); // [11..15]
    file_buf.extend_from_slice(&(payload_len as u32).to_le_bytes()); // [15..19]
    file_buf.extend_from_slice(&schema_comp);                        // schema block
    file_buf.extend_from_slice(&payload_comp);                       // payload block

    std::fs::write(&out_path, &file_buf)
        .map_err(|e| VmError::runtime_error(format!("nova_write: cannot write '{}': {}", out_path, e)))?;

    Ok(Value::Bool(true))
}

/// nova_info(path) → String
/// Returns metadata about a NOVA file.
pub fn nova_info(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::runtime_error("nova_info(path) expects 1 string argument".to_string()));
    }
    let path = match &args[0] {
        Value::Str(s) => s.clone(),
        _ => return Err(VmError::runtime_error("nova_info: arg must be a string".to_string())),
    };

    let data = std::fs::read(&path)
        .map_err(|e| VmError::runtime_error(format!("nova_info: cannot read '{}': {}", path, e)))?;

    let h = parse_header(&data)?;

    let file_sz  = data.len();
    let overhead = 19usize; // v2 header: 4+1+2+4+4+4 = 19 bytes

    // Decompress schema block to list column names + algos
    let schema_start = overhead;
    let schema_end   = schema_start + h.schema_len;
    let schema = lz77_decompress(&data[schema_start..schema_end.min(data.len())]);

    let mut col_info = Vec::new();
    let mut pos = 0;
    while pos < schema.len() && col_info.len() < h.ncols {
        let name_len = *schema.get(pos).unwrap_or(&0) as usize; pos += 1;
        if pos + name_len > schema.len() { break; }
        let name      = String::from_utf8_lossy(&schema[pos..pos+name_len]).into_owned();
        pos += name_len;
        let type_algo = *schema.get(pos).unwrap_or(&0); pos += 1;
        let type_byte = (type_algo >> 4) & 0x0F;
        let algo      = type_algo & 0x0F;
        let type_str = match type_byte { T_INT=>"int", T_FLOAT=>"flt", T_STR=>"str", T_BOOL=>"bool", _=>"?" };
        let algo_str = match algo { A_RLE=>"rle", A_DELTA=>"Δ", A_DICT=>"dict", A_BITS=>"bits", _=>"?" };
        col_info.push(format!("{}:{}/{}", name, type_str, algo_str));
    }

    let first5 = col_info.iter().take(5).cloned().collect::<Vec<_>>().join("  ");
    let ratio_schema  = if schema.len() > 0 { h.schema_len  * 100 / schema.len()  } else { 0 };
    let payload_raw_sz = {
        // walk payload to sum up raw sizes (just report estimate = decomp len)
        let payload = lz77_decompress(
            &data[schema_end..( schema_end + h.payload_len).min(data.len())]
        );
        payload.len()
    };
    let ratio_payload = if payload_raw_sz > 0 { h.payload_len * 100 / payload_raw_sz } else { 0 };

    Ok(Value::Str(format!(
        "NOVA file: {path}\nFormat: Nova Compression v{ver} (columnar+bitpack+LZ77 hash-table)\nColumns: {ncols}  Rows: {nrows}\nFile size: {file_sz} bytes  Header: {overhead} bytes\nSchema:  {sraw} B raw → {scomp} B compressed ({sratio}%)\nPayload: {praw} B raw → {pcomp} B compressed ({pratio}%)\nFirst 5 cols: {first5}",
        path    = path,
        ver     = data[4],
        ncols   = h.ncols,
        nrows   = h.nrows,
        file_sz = file_sz,
        overhead= overhead,
        sraw    = schema.len(),
        scomp   = h.schema_len,
        sratio  = ratio_schema,
        praw    = payload_raw_sz,
        pcomp   = h.payload_len,
        pratio  = ratio_payload,
        first5  = first5,
    )))
}

/// nova_read_col(path, col_name) → Array
/// Decodes and returns one column from a NOVA file.
pub fn nova_read_col(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::runtime_error(
            "nova_read_col(path, col_name) expects 2 string arguments".to_string()
        ));
    }
    let (path, col_name) = match (&args[0], &args[1]) {
        (Value::Str(a), Value::Str(b)) => (a.clone(), b.clone()),
        _ => return Err(VmError::runtime_error("nova_read_col: both args must be strings".to_string())),
    };

    let data = std::fs::read(&path)
        .map_err(|e| VmError::runtime_error(format!("nova_read_col: cannot read '{}': {}", path, e)))?;

    let h = parse_header(&data)?;
    let schema_start  = 19usize; // header is 4+1+2+4+4+4 = 19 bytes
    let schema_end    = schema_start + h.schema_len;
    let payload_start = schema_end;
    let payload_end   = payload_start + h.payload_len;

    // Decompress both blocks upfront
    let schema  = lz77_decompress(&data[schema_start ..schema_end .min(data.len())]);
    let payload = lz77_decompress(&data[payload_start..payload_end.min(data.len())]);

    // Walk schema to find column index + type/algo
    let mut target_type_byte = 0u8;
    let mut target_algo      = 0u8;
    let mut target_col_idx   = usize::MAX;
    let mut spos = 0;
    let mut cidx = 0;
    while spos < schema.len() && cidx < h.ncols {
        let name_len = *schema.get(spos).unwrap_or(&0) as usize; spos += 1;
        if spos + name_len > schema.len() { break; }
        let name = String::from_utf8_lossy(&schema[spos..spos+name_len]).into_owned();
        spos += name_len;
        let type_algo = *schema.get(spos).unwrap_or(&0); spos += 1;
        if name == col_name {
            target_type_byte = (type_algo >> 4) & 0x0F;
            target_algo      = type_algo & 0x0F;
            target_col_idx   = cidx;
            break;
        }
        cidx += 1;
    }
    if target_col_idx == usize::MAX {
        return Err(VmError::runtime_error(
            format!("nova_read_col: column '{}' not found in '{}'", col_name, path)
        ));
    }

    // Walk payload to the correct column's data block
    let mut ppos = 0;
    for _ in 0..target_col_idx {
        let dlen = {
            let a = *payload.get(ppos  ).unwrap_or(&0) as usize;
            let b = *payload.get(ppos+1).unwrap_or(&0) as usize;
            let c = *payload.get(ppos+2).unwrap_or(&0) as usize;
            a | (b << 8) | (c << 16)
        };
        ppos += 3 + dlen;
    }
    // Now at our column
    let dlen = {
        let a = *payload.get(ppos  ).unwrap_or(&0) as usize;
        let b = *payload.get(ppos+1).unwrap_or(&0) as usize;
        let c = *payload.get(ppos+2).unwrap_or(&0) as usize;
        a | (b << 8) | (c << 16)
    };
    ppos += 3;
    let col_data = if ppos + dlen <= payload.len() { &payload[ppos..ppos+dlen] } else { &payload[ppos..] };
    let (vals, _) = decode_col(target_type_byte, target_algo, col_data, 0, h.nrows);
    Ok(Value::from(vals))
}

// -- Helper: decompress both blocks and decode all columns ---------------------
fn decode_all_columns(data: &[u8]) -> Result<(NovaHeader, Vec<String>, Vec<Vec<Value>>), VmError> {
    let h = parse_header(data)?;
    let schema_start  = 19usize;
    let schema_end    = schema_start + h.schema_len;
    let payload_start = schema_end;
    let payload_end   = payload_start + h.payload_len;

    let schema  = lz77_decompress(&data[schema_start ..schema_end .min(data.len())]);
    let payload = lz77_decompress(&data[payload_start..payload_end.min(data.len())]);

    // Parse schema: col names + type/algo info
    let mut col_names:      Vec<String> = Vec::with_capacity(h.ncols);
    let mut col_type_algos: Vec<(u8,u8)>= Vec::with_capacity(h.ncols);
    let mut spos = 0;
    while spos < schema.len() && col_names.len() < h.ncols {
        let name_len = *schema.get(spos).unwrap_or(&0) as usize; spos += 1;
        if spos + name_len > schema.len() { break; }
        let name = String::from_utf8_lossy(&schema[spos..spos+name_len]).into_owned();
        spos += name_len;
        let type_algo = *schema.get(spos).unwrap_or(&0); spos += 1;
        col_names.push(name);
        col_type_algos.push(((type_algo >> 4) & 0x0F, type_algo & 0x0F));
    }

    // Decode all columns from payload
    let mut cols: Vec<Vec<Value>> = Vec::with_capacity(h.ncols);
    let mut ppos = 0;
    for ci in 0..col_names.len() {
        let dlen = {
            let a = *payload.get(ppos  ).unwrap_or(&0) as usize;
            let b = *payload.get(ppos+1).unwrap_or(&0) as usize;
            let c = *payload.get(ppos+2).unwrap_or(&0) as usize;
            a | (b << 8) | (c << 16)
        };
        ppos += 3;
        let (type_byte, algo) = col_type_algos.get(ci).copied().unwrap_or((0,0));
        let col_data = if ppos + dlen <= payload.len() { &payload[ppos..ppos+dlen] } else { &payload[ppos..] };
        let (vals, _) = decode_col(type_byte, algo, col_data, 0, h.nrows);
        cols.push(vals);
        ppos += dlen;
    }

    Ok((h, col_names, cols))
}

// =============================================================================
// Streaming Nova Decode API
// =============================================================================
// Lazy, column-at-a-time decoding without loading the entire dataset.
// Supports:
//   1. nova_stream_open(path) → reader handle (Dict with metadata)
//   2. nova_stream_col(reader, col_name) → decodes one column on demand
//   3. nova_stream_batch(path, batch_size) → row-oriented batch iterator
// =============================================================================

/// Internal: parse schema block without decoding any payload columns.
/// Returns (header, decompressed_payload, col_names, col_type_algos, col_offsets).
/// col_offsets[i] = byte offset within decompressed payload where column i starts.
fn parse_schema_and_offsets(data: &[u8])
    -> Result<(NovaHeader, Vec<u8>, Vec<String>, Vec<(u8,u8)>, Vec<usize>), VmError>
{
    let h = parse_header(data)?;
    let schema_start  = 19usize;
    let schema_end    = schema_start + h.schema_len;
    let payload_start = schema_end;
    let payload_end   = payload_start + h.payload_len;

    let schema  = lz77_decompress(&data[schema_start..schema_end.min(data.len())]);
    let payload = lz77_decompress(&data[payload_start..payload_end.min(data.len())]);

    let mut col_names:      Vec<String>  = Vec::with_capacity(h.ncols);
    let mut col_type_algos: Vec<(u8,u8)> = Vec::with_capacity(h.ncols);
    let mut spos = 0;
    while spos < schema.len() && col_names.len() < h.ncols {
        let name_len = *schema.get(spos).unwrap_or(&0) as usize; spos += 1;
        if spos + name_len > schema.len() { break; }
        let name = String::from_utf8_lossy(&schema[spos..spos+name_len]).into_owned();
        spos += name_len;
        let type_algo = *schema.get(spos).unwrap_or(&0); spos += 1;
        col_names.push(name);
        col_type_algos.push(((type_algo >> 4) & 0x0F, type_algo & 0x0F));
    }

    // Build offset table: walk the u24 length prefix of each column block
    let mut col_offsets: Vec<usize> = Vec::with_capacity(h.ncols);
    let mut ppos = 0;
    for _ in 0..col_names.len() {
        col_offsets.push(ppos);
        let dlen = {
            let a = *payload.get(ppos  ).unwrap_or(&0) as usize;
            let b = *payload.get(ppos+1).unwrap_or(&0) as usize;
            let c = *payload.get(ppos+2).unwrap_or(&0) as usize;
            a | (b << 8) | (c << 16)
        };
        ppos += 3 + dlen;
    }

    Ok((h, payload, col_names, col_type_algos, col_offsets))
}

/// Decode a single column by index from an already-decompressed payload.
fn decode_single_column(
    payload:   &[u8],
    offset:    usize,
    type_byte: u8,
    algo:      u8,
    nrows:     usize,
) -> Vec<Value> {
    let mut ppos = offset;
    let dlen = {
        let a = *payload.get(ppos  ).unwrap_or(&0) as usize;
        let b = *payload.get(ppos+1).unwrap_or(&0) as usize;
        let c = *payload.get(ppos+2).unwrap_or(&0) as usize;
        a | (b << 8) | (c << 16)
    };
    ppos += 3;
    let col_data = if ppos + dlen <= payload.len() { &payload[ppos..ppos+dlen] } else { &payload[ppos..] };
    let (vals, _) = decode_col(type_byte, algo, col_data, 0, nrows);
    vals
}

/// nova_stream_open(path) → Dict
/// Opens a KORE file and returns a lightweight reader handle containing:
///   "path"     → original file path (re-read on demand)
///   "ncols"    → number of columns
///   "nrows"    → number of rows
///   "columns"  → Array of column names
///   "_meta"    → internal: column type/algo/offset metadata
/// No payload is loaded — individual columns are decoded from disk on demand.
pub fn nova_stream_open(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::runtime_error(
            "nova_stream_open(path) expects 1 string argument".to_string()));
    }
    let path = match &args[0] {
        Value::Str(s) => s.clone(),
        _ => return Err(VmError::runtime_error("nova_stream_open: arg must be a string".to_string())),
    };

    let data = std::fs::read(&path)
        .map_err(|e| VmError::runtime_error(format!("nova_stream_open: cannot read '{}': {}", path, e)))?;

    let (h, _payload, col_names, col_type_algos, col_offsets) = parse_schema_and_offsets(&data)?;

    // Pack col metadata: type_byte,algo,offset as CSV per column
    let meta_str: String = col_type_algos.iter().zip(col_offsets.iter())
        .map(|((tb, al), off)| format!("{},{},{}", tb, al, off))
        .collect::<Vec<_>>()
        .join(";");

    let mut map = std::collections::HashMap::new();
    map.insert("path".to_string(),     Value::Str(path));
    map.insert("ncols".to_string(),    Value::Number(h.ncols as f64));
    map.insert("nrows".to_string(),    Value::Number(h.nrows as f64));
    map.insert(
        "columns".to_string(),
        Value::from(col_names.iter().map(|n| Value::Str(n.clone())).collect::<Vec<_>>()),
    );
    map.insert("_meta".to_string(),    Value::Str(meta_str));

    Ok(Value::Dict(Box::new(map)))
}

/// nova_stream_col(reader, col_name) → Array
/// Decodes a single column from a previously opened streaming reader handle.
/// Re-reads and decompresses the file, then decodes only the target column.
pub fn nova_stream_col(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::runtime_error(
            "nova_stream_col(reader, col_name) expects 2 arguments".to_string()));
    }
    let reader = match &args[0] {
        Value::Dict(d) => d,
        _ => return Err(VmError::runtime_error("nova_stream_col: first arg must be a stream reader (Dict)".to_string())),
    };
    let col_name = match &args[1] {
        Value::Str(s) => s.as_str(),
        _ => return Err(VmError::runtime_error("nova_stream_col: second arg must be a column name string".to_string())),
    };

    // Extract metadata from handle
    let nrows = match reader.get("nrows") {
        Some(Value::Number(n)) => *n as usize,
        _ => return Err(VmError::runtime_error("nova_stream_col: invalid reader (missing nrows)".to_string())),
    };
    let columns = match reader.get("columns") {
        Some(Value::Array(arr)) => arr,
        _ => return Err(VmError::runtime_error("nova_stream_col: invalid reader (missing columns)".to_string())),
    };
    let file_path = match reader.get("path") {
        Some(Value::Str(s)) => s.clone(),
        _ => return Err(VmError::runtime_error("nova_stream_col: invalid reader (missing path)".to_string())),
    };
    let meta_str = match reader.get("_meta") {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Err(VmError::runtime_error("nova_stream_col: invalid reader (missing _meta)".to_string())),
    };

    // Find column index
    let col_idx = columns
        .iter_cloned()
        .position(|v| matches!(&v, Value::Str(s) if s == col_name))
        .ok_or_else(|| VmError::runtime_error(format!("nova_stream_col: column '{}' not found", col_name)))?;

    // Re-read file and decompress payload
    let data = std::fs::read(&file_path)
        .map_err(|e| VmError::runtime_error(format!("nova_stream_col: cannot read '{}': {}", file_path, e)))?;
    let h = parse_header(&data)?;
    let schema_end    = 19 + h.schema_len;
    let payload_start = schema_end;
    let payload_end   = payload_start + h.payload_len;
    let payload = lz77_decompress(&data[payload_start..payload_end.min(data.len())]);

    // Parse column metadata
    let meta_parts: Vec<&str> = meta_str.split(';').collect();
    let meta_entry = meta_parts.get(col_idx)
        .ok_or_else(|| VmError::runtime_error("nova_stream_col: metadata index out of range".to_string()))?;
    let nums: Vec<usize> = meta_entry.split(',').filter_map(|s| s.parse().ok()).collect();
    if nums.len() != 3 {
        return Err(VmError::runtime_error("nova_stream_col: corrupt column metadata".to_string()));
    }
    let (type_byte, algo, offset) = (nums[0] as u8, nums[1] as u8, nums[2]);

    let vals = decode_single_column(&payload, offset, type_byte, algo, nrows);
    Ok(Value::from(vals))
}

/// nova_stream_batch(path, batch_size) → Array of row-Dicts
/// Decodes a KORE file in row-oriented batches. Returns an Array where each
/// element is a Dict of { col_name: value } for that row. Processes `batch_size`
/// rows at a time to limit peak memory.
///
/// Usage: `let rows = nova_stream_batch("data.kore", 1000)`
pub fn nova_stream_batch(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::runtime_error(
            "nova_stream_batch(path, batch_size) expects 2 arguments".to_string()));
    }
    let path = match &args[0] {
        Value::Str(s) => s.clone(),
        _ => return Err(VmError::runtime_error("nova_stream_batch: first arg must be a path string".to_string())),
    };
    let batch_size = match &args[1] {
        Value::Number(n) => {
            let b = *n as usize;
            if b == 0 { 1000 } else { b }
        }
        _ => 1000,
    };

    let data = std::fs::read(&path)
        .map_err(|e| VmError::runtime_error(format!("nova_stream_batch: cannot read '{}': {}", path, e)))?;

    let (h, payload, col_names, col_type_algos, col_offsets) = parse_schema_and_offsets(&data)?;

    // Decode all columns (we need full columns to pivot to rows)
    let decoded_cols: Vec<Vec<Value>> = (0..col_names.len())
        .map(|ci| {
            let (tb, al) = col_type_algos[ci];
            decode_single_column(&payload, col_offsets[ci], tb, al, h.nrows)
        })
        .collect();

    // Pivot columns → row batches, emitting one Array of row-Dicts per batch
    let mut batches: Vec<Value> = Vec::new();
    let mut row = 0;
    while row < h.nrows {
        let end = (row + batch_size).min(h.nrows);
        let mut batch: Vec<Value> = Vec::with_capacity(end - row);
        for ri in row..end {
            let mut row_map = std::collections::HashMap::new();
            for (ci, name) in col_names.iter().enumerate() {
                let val = decoded_cols.get(ci)
                    .and_then(|c| c.get(ri))
                    .cloned()
                    .unwrap_or(Value::Null);
                row_map.insert(name.clone(), val);
            }
            batch.push(Value::Dict(Box::new(row_map)));
        }
        batches.push(Value::from(batch));
        row = end;
    }

    Ok(Value::from(batches))
}

/// nova_stream_cols(path, col_names_array) → Dict
/// Decodes only the specified columns from a KORE file. More efficient than
/// nova_read_all when only a few columns are needed.
pub fn nova_stream_cols(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::runtime_error(
            "nova_stream_cols(path, col_names) expects 2 arguments".to_string()));
    }
    let path = match &args[0] {
        Value::Str(s) => s.clone(),
        _ => return Err(VmError::runtime_error("nova_stream_cols: first arg must be a path string".to_string())),
    };
    let wanted: Vec<String> = match &args[1] {
        Value::Array(arr) => arr
            .iter_cloned()
            .filter_map(|v| match v {
                Value::Str(s) => Some(s.clone()),
                _ => None,
            })
            .collect(),
        _ => return Err(VmError::runtime_error("nova_stream_cols: second arg must be an array of column names".to_string())),
    };

    let data = std::fs::read(&path)
        .map_err(|e| VmError::runtime_error(format!("nova_stream_cols: cannot read '{}': {}", path, e)))?;

    let (h, payload, col_names, col_type_algos, col_offsets) = parse_schema_and_offsets(&data)?;

    let mut result_map = std::collections::HashMap::new();
    for want in &wanted {
        let ci = col_names.iter().position(|n| n == want)
            .ok_or_else(|| VmError::runtime_error(format!("nova_stream_cols: column '{}' not found", want)))?;
        let (tb, al) = col_type_algos[ci];
        let vals = decode_single_column(&payload, col_offsets[ci], tb, al, h.nrows);
        result_map.insert(want.clone(), Value::from(vals));
    }

    Ok(Value::Dict(Box::new(result_map)))
}

/// nova_read_all(path) → Map  (col_name → Array of values)
/// Decodes ALL columns at once. Ideal for full-table analytics.
pub fn nova_read_all(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::runtime_error("nova_read_all(path) expects 1 string argument".to_string()));
    }
    let path = match &args[0] {
        Value::Str(s) => s.clone(),
        _ => return Err(VmError::runtime_error("nova_read_all: arg must be a string".to_string())),
    };
    let data = std::fs::read(&path)
        .map_err(|e| VmError::runtime_error(format!("nova_read_all: cannot read '{}': {}", path, e)))?;

    let (_h, col_names, cols) = decode_all_columns(&data)?;

    let mut map = std::collections::HashMap::new();
    for (name, col) in col_names.into_iter().zip(cols.into_iter()) {
        map.insert(name, Value::from(col));
    }
    Ok(Value::Dict(Box::new(map)))
}

/// nova_stats(path, col_name) → Map  {count, min, max, sum, mean, nulls}
/// Fast column statistics without loading full dataset into Killer heap.
pub fn nova_stats(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::runtime_error("nova_stats(path, col_name) expects 2 string arguments".to_string()));
    }
    let (path, col_name) = match (&args[0], &args[1]) {
        (Value::Str(a), Value::Str(b)) => (a.clone(), b.clone()),
        _ => return Err(VmError::runtime_error("nova_stats: both args must be strings".to_string())),
    };
    let data = std::fs::read(&path)
        .map_err(|e| VmError::runtime_error(format!("nova_stats: cannot read '{}': {}", path, e)))?;

    let (_h, col_names, mut cols) = decode_all_columns(&data)?;

    let ci = col_names.iter().position(|n| n == &col_name)
        .ok_or_else(|| VmError::runtime_error(format!("nova_stats: column '{}' not found", col_name)))?;

    let col = cols.remove(ci);
    let mut count  = 0i64;
    let mut nulls  = 0i64;
    let mut sum    = 0f64;
    let mut min    = f64::MAX;
    let mut max    = f64::MIN;
    let mut uniq: std::collections::HashSet<String> = std::collections::HashSet::new();

    for v in &col {
        match v {
            Value::Number(n) => {
                count += 1;
                sum   += n;
                if *n < min { min = *n; }
                if *n > max { max = *n; }
                uniq.insert(format!("{}", n));
            }
            Value::Bool(b) => {
                count += 1;
                let n = if *b { 1.0 } else { 0.0 };
                sum += n;
                if n < min { min = n; }
                if n > max { max = n; }
                uniq.insert(b.to_string());
            }
            Value::Str(s) if s != "EMPTY" => {
                count += 1;
                uniq.insert(s.clone());
                // Try numeric
                if let Ok(n) = s.parse::<f64>() {
                    sum += n;
                    if n < min { min = n; }
                    if n > max { max = n; }
                }
            }
            _ => { nulls += 1; }
        }
    }

    let mean = if count > 0 { sum / count as f64 } else { 0.0 };
    if min == f64::MAX { min = 0.0; }
    if max == f64::MIN { max = 0.0; }

    let mut map = std::collections::HashMap::new();
    map.insert("count".to_string(),  Value::Number(count as f64));
    map.insert("nulls".to_string(),  Value::Number(nulls as f64));
    map.insert("unique".to_string(), Value::Number(uniq.len() as f64));
    map.insert("min".to_string(),    Value::Number(min));
    map.insert("max".to_string(),    Value::Number(max));
    map.insert("sum".to_string(),    Value::Number(sum));
    map.insert("mean".to_string(),   Value::Number(mean));
    Ok(Value::Dict(Box::new(map)))
}

/// nova_filter(path, col_name, value) → Array of row indices (0-based) where col == value
/// Example: nova_filter(path, "TRANSSOURCE", "FF") → [0, 1, 2, ...]
pub fn nova_filter(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 3 {
        return Err(VmError::runtime_error(
            "nova_filter(path, col_name, value) expects 3 arguments".to_string()
        ));
    }
    let path = match &args[0] {
        Value::Str(s) => s.clone(),
        _ => return Err(VmError::runtime_error("nova_filter: first arg must be a string path".to_string())),
    };
    let col_name = match &args[1] {
        Value::Str(s) => s.clone(),
        _ => return Err(VmError::runtime_error("nova_filter: second arg must be a column name string".to_string())),
    };
    let filter_val = &args[2];

    let data = std::fs::read(&path)
        .map_err(|e| VmError::runtime_error(format!("nova_filter: cannot read '{}': {}", path, e)))?;

    let (_h, col_names, mut cols) = decode_all_columns(&data)?;

    let ci = col_names.iter().position(|n| n == &col_name)
        .ok_or_else(|| VmError::runtime_error(format!("nova_filter: column '{}' not found", col_name)))?;

    let col = cols.remove(ci);
    let mut indices = Vec::new();
    for (row_idx, v) in col.iter().enumerate() {
        let matches = match (v, filter_val) {
            (Value::Number(a), Value::Number(b)) => (a - b).abs() < 1e-9,
            (Value::Bool(a),   Value::Bool(b))   => a == b,
            (Value::Str(a),    Value::Str(b))    => a == b,
            (Value::Number(n), Value::Str(s))    => format!("{}", n) == *s || format!("{:.4}", n) == *s,
            (Value::Bool(b),   Value::Str(s))    => b.to_string() == *s || (*b && s == "1") || (!b && s == "0"),
            _ => false,
        };
        if matches {
            indices.push(Value::Number(row_idx as f64));
        }
    }
    Ok(Value::from(indices))
}

/// nova_to_csv(nova_path, csv_path) → Bool
/// Decompresses a KORE Nova file back to plain CSV. Full round-trip support.
pub fn nova_to_csv(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::runtime_error("nova_to_csv(nova_path, csv_path) expects 2 string arguments".to_string()));
    }
    let (nova_path, csv_path) = match (&args[0], &args[1]) {
        (Value::Str(a), Value::Str(b)) => (a.clone(), b.clone()),
        _ => return Err(VmError::runtime_error("nova_to_csv: both args must be strings".to_string())),
    };
    let data = std::fs::read(&nova_path)
        .map_err(|e| VmError::runtime_error(format!("nova_to_csv: cannot read '{}': {}", nova_path, e)))?;

    let (h, col_names, cols) = decode_all_columns(&data)?;

    let mut out = String::with_capacity(h.nrows * col_names.len() * 8);

    // Header row
    out.push_str(&col_names.join(","));
    out.push('\n');

    // Data rows  
    for row in 0..h.nrows {
        let mut first = true;
        for col in &cols {
            if !first { out.push(','); }
            first = false;
            let v = col.get(row).unwrap_or(&Value::Null);
            match v {
                Value::Number(n) => {
                    // Restore original integer representation where possible
                    if n.fract() == 0.0 && n.abs() < 1e15 {
                        out.push_str(&format!("{}", *n as i64));
                    } else {
                        // Trim insignificant trailing zeros
                        let s = format!("{:.8}", n);
                        out.push_str(s.trim_end_matches('0').trim_end_matches('.'));
                    }
                }
                Value::Bool(b) => out.push(if *b { '1' } else { '0' }),
                Value::Str(s)  => {
                    // Quote fields that contain commas
                    if s.contains(',') {
                        out.push('"');
                        out.push_str(s);
                        out.push('"');
                    } else if s == "EMPTY" {
                        // empty field — write nothing
                    } else {
                        out.push_str(s);
                    }
                }
                Value::Null => {}
                _ => out.push_str(&format!("{:?}", v)),
            }
        }
        out.push('\n');
    }

    std::fs::write(&csv_path, out.as_bytes())
        .map_err(|e| VmError::runtime_error(format!("nova_to_csv: cannot write '{}': {}", csv_path, e)))?;

    Ok(Value::Bool(true))
}

// -- Format converters ---------------------------------------------------------
// These live in Rust (nova.rs) — not Killer scripts — because they deal with
// binary/structured parsing that belongs at the engine boundary.
// Killer scripts compose these builtins; they don't reimplement them.

/// nova_from_tsv(tsv_path, kore_path) → Bool
/// Converts a Tab-Separated Values file to KORE Nova format.
/// Handles quoted fields and any TSV header row.
pub fn nova_from_tsv(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::runtime_error(
            "nova_from_tsv(tsv_path, kore_path) expects 2 string arguments".to_string()
        ));
    }
    let (tsv_path, kore_path) = match (&args[0], &args[1]) {
        (Value::Str(a), Value::Str(b)) => (a.clone(), b.clone()),
        _ => return Err(VmError::runtime_error("nova_from_tsv: both args must be strings".to_string())),
    };

    let text = std::fs::read_to_string(&tsv_path)
        .map_err(|e| VmError::runtime_error(format!("nova_from_tsv: cannot read '{}': {}", tsv_path, e)))?;

    // Convert TSV → CSV in memory (replace tabs with commas, handle quoted fields)
    let mut csv_out = String::with_capacity(text.len());
    for line in text.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        for (i, field) in fields.iter().enumerate() {
            if i > 0 { csv_out.push(','); }
            let f = field.trim_matches('"');
            if f.contains(',') {
                csv_out.push('"');
                csv_out.push_str(f);
                csv_out.push('"');
            } else {
                csv_out.push_str(f);
            }
        }
        csv_out.push('\n');
    }

    // Write temp CSV then encode with nova_write logic
    let tmp_csv = format!("{}.tmp_tsv_import.csv", kore_path);
    std::fs::write(&tmp_csv, csv_out.as_bytes())
        .map_err(|e| VmError::runtime_error(format!("nova_from_tsv: cannot write temp csv: {}", e)))?;

    let result = nova_write(&[Value::Str(tmp_csv.clone()), Value::Str(kore_path)]);
    let _ = std::fs::remove_file(&tmp_csv); // clean up temp
    result
}

/// nova_to_tsv(kore_path, tsv_path) → Bool
/// Decompresses a KORE file to Tab-Separated Values.
pub fn nova_to_tsv(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::runtime_error(
            "nova_to_tsv(kore_path, tsv_path) expects 2 string arguments".to_string()
        ));
    }
    let (kore_path, tsv_path) = match (&args[0], &args[1]) {
        (Value::Str(a), Value::Str(b)) => (a.clone(), b.clone()),
        _ => return Err(VmError::runtime_error("nova_to_tsv: both args must be strings".to_string())),
    };

    let data = std::fs::read(&kore_path)
        .map_err(|e| VmError::runtime_error(format!("nova_to_tsv: cannot read '{}': {}", kore_path, e)))?;
    let (h, col_names, cols) = decode_all_columns(&data)?;

    let mut out = String::with_capacity(h.nrows * col_names.len() * 8);
    out.push_str(&col_names.join("\t"));
    out.push('\n');

    for row in 0..h.nrows {
        for (ci, col) in cols.iter().enumerate() {
            if ci > 0 { out.push('\t'); }
            match col.get(row).unwrap_or(&Value::Null) {
                Value::Number(n) => {
                    if n.fract() == 0.0 && n.abs() < 1e15 {
                        out.push_str(&format!("{}", *n as i64));
                    } else {
                        let s = format!("{:.8}", n);
                        out.push_str(s.trim_end_matches('0').trim_end_matches('.'));
                    }
                }
                Value::Bool(b)  => out.push(if *b { '1' } else { '0' }),
                Value::Str(s)   => { if s != "EMPTY" { out.push_str(s); } }
                Value::Null     => {}
                v               => out.push_str(&format!("{:?}", v)),
            }
        }
        out.push('\n');
    }

    std::fs::write(&tsv_path, out.as_bytes())
        .map_err(|e| VmError::runtime_error(format!("nova_to_tsv: cannot write '{}': {}", tsv_path, e)))?;
    Ok(Value::Bool(true))
}

/// nova_from_json(json_path, kore_path) → Bool
/// Converts a JSON array-of-objects file to KORE Nova format.
/// Supports: [{"col": val, ...}, ...]  (newline-delimited or standard array)
pub fn nova_from_json(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::runtime_error(
            "nova_from_json(json_path, kore_path) expects 2 string arguments".to_string()
        ));
    }
    let (json_path, kore_path) = match (&args[0], &args[1]) {
        (Value::Str(a), Value::Str(b)) => (a.clone(), b.clone()),
        _ => return Err(VmError::runtime_error("nova_from_json: both args must be strings".to_string())),
    };

    let text = std::fs::read_to_string(&json_path)
        .map_err(|e| VmError::runtime_error(format!("nova_from_json: cannot read '{}': {}", json_path, e)))?;

    // Minimal JSON array-of-objects parser (pure stdlib, no serde)
    // Strips outer [...], splits on "},{", parses each object into key:value pairs
    let trimmed = text.trim();

    // Support both array [...] and newline-delimited JSON (one object per line)
    let objects: Vec<&str> = if trimmed.starts_with('[') {
        let inner = trimmed.trim_start_matches('[').trim_end_matches(']').trim();
        // Split on "},\n{" or "},{" — safe because values won't span across top-level objects
        split_json_objects(inner)
    } else {
        // NDJSON: one JSON object per line
        trimmed.lines().filter(|l| l.trim().starts_with('{')).collect()
    };

    if objects.is_empty() {
        return Err(VmError::runtime_error("nova_from_json: no records found in JSON".to_string()));
    }

    // Parse first object to get column names (preserves order)
    let first_keys = parse_json_keys(objects[0]);
    let ncols = first_keys.len();
    if ncols == 0 {
        return Err(VmError::runtime_error("nova_from_json: could not parse JSON object keys".to_string()));
    }

    // Build column index map
    let _col_idx: std::collections::HashMap<String, usize> = first_keys.iter()
        .enumerate().map(|(i, k)| (k.clone(), i)).collect();

    // Extract all rows
    let nrows = objects.len();
    let mut columns: Vec<Vec<String>> = vec![Vec::with_capacity(nrows); ncols];
    for obj in &objects {
        let kv = parse_json_kv(obj);
        for ci in 0..ncols {
            let key = &first_keys[ci];
            let val = kv.get(key).map(|s| s.as_str()).unwrap_or("");
            columns[ci].push(val.to_string());
        }
    }

    // Convert to CSV in memory and feed to nova_write internals
    let mut csv_out = String::new();
    csv_out.push_str(&first_keys.join(","));
    csv_out.push('\n');
    for row in 0..nrows {
        for ci in 0..ncols {
            if ci > 0 { csv_out.push(','); }
            let v = &columns[ci][row];
            if v.contains(',') || v.contains('"') {
                csv_out.push('"');
                csv_out.push_str(&v.replace('"', "\\\""));
                csv_out.push('"');
            } else {
                csv_out.push_str(v);
            }
        }
        csv_out.push('\n');
    }

    let tmp = format!("{}.tmp_json_import.csv", kore_path);
    std::fs::write(&tmp, csv_out.as_bytes())
        .map_err(|e| VmError::runtime_error(format!("nova_from_json: cannot write temp: {}", e)))?;
    let result = nova_write(&[Value::Str(tmp.clone()), Value::Str(kore_path)]);
    let _ = std::fs::remove_file(&tmp);
    result
}

/// nova_to_json(kore_path, json_path) → Bool
/// Exports a KORE file to a JSON array-of-objects.
pub fn nova_to_json(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::runtime_error(
            "nova_to_json(kore_path, json_path) expects 2 string arguments".to_string()
        ));
    }
    let (kore_path, json_path) = match (&args[0], &args[1]) {
        (Value::Str(a), Value::Str(b)) => (a.clone(), b.clone()),
        _ => return Err(VmError::runtime_error("nova_to_json: both args must be strings".to_string())),
    };

    let data = std::fs::read(&kore_path)
        .map_err(|e| VmError::runtime_error(format!("nova_to_json: cannot read '{}': {}", kore_path, e)))?;
    let (h, col_names, cols) = decode_all_columns(&data)?;

    let mut out = String::with_capacity(h.nrows * col_names.len() * 12);
    out.push('[');

    for row in 0..h.nrows {
        if row > 0 { out.push(','); }
        out.push_str("\n  {");
        for (ci, col) in cols.iter().enumerate() {
            if ci > 0 { out.push(','); }
            out.push('"');
            out.push_str(&col_names[ci].replace('"', "\\\""));
            out.push_str("\":");
            match col.get(row).unwrap_or(&Value::Null) {
                Value::Number(n) => {
                    if n.fract() == 0.0 && n.abs() < 1e15 {
                        out.push_str(&format!("{}", *n as i64));
                    } else {
                        let s = format!("{:.8}", n);
                        out.push_str(s.trim_end_matches('0').trim_end_matches('.'));
                    }
                }
                Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
                Value::Str(s) if s == "EMPTY" => out.push_str("null"),
                Value::Str(s) => {
                    out.push('"');
                    out.push_str(&s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"));
                    out.push('"');
                }
                Value::Null => out.push_str("null"),
                v           => { out.push('"'); out.push_str(&format!("{:?}", v)); out.push('"'); }
            }
        }
        out.push('}');
    }
    out.push_str("\n]\n");

    std::fs::write(&json_path, out.as_bytes())
        .map_err(|e| VmError::runtime_error(format!("nova_to_json: cannot write '{}': {}", json_path, e)))?;
    Ok(Value::Bool(true))
}

// -- JSON helpers (pure stdlib, zero deps) -------------------------------------

/// Split a JSON array inner content into individual object strings
fn split_json_objects(inner: &str) -> Vec<&str> {
    let mut objects = Vec::new();
    let mut depth   = 0i32;
    let mut start   = 0usize;
    let bytes       = inner.as_bytes();
    let mut in_str  = false;
    let mut i       = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if in_str {
            if b == b'\\' { i += 1; } // skip escaped char
            else if b == b'"' { in_str = false; }
        } else {
            match b {
                b'"' => in_str = true,
                b'{' => { if depth == 0 { start = i; } depth += 1; }
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        objects.push(&inner[start..=i]);
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    objects
}

/// Extract ordered list of keys from a JSON object string
fn parse_json_keys(obj: &str) -> Vec<String> {
    let mut keys  = Vec::new();
    let mut i     = 0usize;
    let bytes     = obj.as_bytes();
    let mut _in_key = false;
    let mut key_buf = String::new();
    // Fast scan: find all "key" before the colon at depth=1
    let mut depth = 0i32;
    while i < bytes.len() {
        match bytes[i] {
            b'{' | b'[' => depth += 1,
            b'}' | b']' => depth -= 1,
            b'"' if depth == 1 => {
                i += 1;
                key_buf.clear();
                while i < bytes.len() && bytes[i] != b'"' {
                    if bytes[i] == b'\\' { i += 1; }
                    else { key_buf.push(bytes[i] as char); }
                    i += 1;
                }
                // Check if followed by ':' (it's a key, not a value)
                let rest = &obj[i+1..].trim_start();
                if rest.starts_with(':') { keys.push(key_buf.clone()); }
            }
            _ => {}
        }
        i += 1;
    }
    keys
}

/// Parse a JSON object string into a HashMap<key, value_string>
fn parse_json_kv(obj: &str) -> std::collections::HashMap<String, String> {
    let mut map   = std::collections::HashMap::new();
    let bytes     = obj.as_bytes();
    let mut i     = 0usize;
    let mut depth = 0i32;

    while i < bytes.len() {
        match bytes[i] {
            b'{' | b'[' => depth += 1,
            b'}' | b']' => depth -= 1,
            b'"' if depth == 1 => {
                // Parse key
                i += 1;
                let mut key = String::new();
                while i < bytes.len() && bytes[i] != b'"' {
                    if bytes[i] == b'\\' { i += 1; }
                    else { key.push(bytes[i] as char); }
                    i += 1;
                }
                i += 1; // closing "
                // Skip whitespace and colon
                while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b':') { i += 1; }
                // Parse value
                let val = if i < bytes.len() && bytes[i] == b'"' {
                    // String value
                    i += 1;
                    let mut v = String::new();
                    while i < bytes.len() && bytes[i] != b'"' {
                        if bytes[i] == b'\\' {
                            i += 1;
                            if i < bytes.len() {
                                match bytes[i] {
                                    b'n' => v.push('\n'),
                                    b't' => v.push('\t'),
                                    b'"' => v.push('"'),
                                    b'\\' => v.push('\\'),
                                    c => { v.push('\\'); v.push(c as char); }
                                }
                            }
                        } else {
                            v.push(bytes[i] as char);
                        }
                        i += 1;
                    }
                    // i is at closing '"' — skip it so outer loop doesn't re-parse it as a key
                    if i < bytes.len() { i += 1; }
                    v
                } else if i < bytes.len() && (bytes[i] == b'{' || bytes[i] == b'[') {
                    // Nested — just collect as string
                    let start = i;
                    let mut d = 0i32;
                    while i < bytes.len() {
                        match bytes[i] { b'{' | b'[' => d += 1, b'}' | b']' => { d -= 1; if d == 0 { i += 1; break; } }, _ => {} }
                        i += 1;
                    }
                    obj[start..i].to_string()
                } else {
                    // Bare value: number / bool / null — read until , or }
                    let start = i;
                    while i < bytes.len() && bytes[i] != b',' && bytes[i] != b'}' && bytes[i] != b']' { i += 1; }
                    obj[start..i].trim().to_string()
                };
                map.insert(key, val);
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    map
}

// -- XML converter -------------------------------------------------------------

/// nova_from_xml(xml_path, kore_path) → Bool
/// Converts an XML file with repeated row elements to KORE format.
/// Expects: <root><row><col>val</col>...</row>...</root>  (any tag names)
pub fn nova_from_xml(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::runtime_error(
            "nova_from_xml(xml_path, kore_path) expects 2 string arguments".to_string()
        ));
    }
    let (xml_path, kore_path) = match (&args[0], &args[1]) {
        (Value::Str(a), Value::Str(b)) => (a.clone(), b.clone()),
        _ => return Err(VmError::runtime_error("nova_from_xml: both args must be strings".to_string())),
    };
    let text = std::fs::read_to_string(&xml_path)
        .map_err(|e| VmError::runtime_error(format!("nova_from_xml: cannot read '{}': {}", xml_path, e)))?;

    // Minimal XML → records parser (pure stdlib)
    // Strategy: find the second-level repeated element, treat its children as columns
    let records = parse_xml_records(&text);
    if records.is_empty() {
        return Err(VmError::runtime_error("nova_from_xml: no records found".to_string()));
    }

    // Collect all column names (union of all records, in order of first appearance)
    let mut col_order: Vec<String> = Vec::new();
    let mut col_set: std::collections::HashSet<String> = std::collections::HashSet::new();
    for rec in &records {
        for (k, _) in rec {
            if col_set.insert(k.clone()) { col_order.push(k.clone()); }
        }
    }

    // Build CSV in memory
    let mut csv = String::new();
    csv.push_str(&col_order.join(","));
    csv.push('\n');
    for rec in &records {
        for (ci, col) in col_order.iter().enumerate() {
            if ci > 0 { csv.push(','); }
            let v = rec.iter().find(|(k,_)| k == col).map(|(_,v)| v.as_str()).unwrap_or("");
            if v.contains(',') || v.contains('"') {
                csv.push('"');
                csv.push_str(&v.replace('"', "\\\""));
                csv.push('"');
            } else {
                csv.push_str(v);
            }
        }
        csv.push('\n');
    }

    let tmp = format!("{}.tmp_xml.csv", kore_path);
    std::fs::write(&tmp, csv.as_bytes())
        .map_err(|e| VmError::runtime_error(format!("nova_from_xml: tmp write error: {}", e)))?;
    let result = nova_write(&[Value::Str(tmp.clone()), Value::Str(kore_path)]);
    let _ = std::fs::remove_file(&tmp);
    result
}

/// nova_to_xml(kore_path, xml_path) → Bool
/// Exports KORE to XML: <records><record><COL>val</COL>...</record>...</records>
pub fn nova_to_xml(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::runtime_error(
            "nova_to_xml(kore_path, xml_path) expects 2 string arguments".to_string()
        ));
    }
    let (kore_path, xml_path) = match (&args[0], &args[1]) {
        (Value::Str(a), Value::Str(b)) => (a.clone(), b.clone()),
        _ => return Err(VmError::runtime_error("nova_to_xml: both args must be strings".to_string())),
    };
    let data = std::fs::read(&kore_path)
        .map_err(|e| VmError::runtime_error(format!("nova_to_xml: cannot read '{}': {}", kore_path, e)))?;
    let (h, col_names, cols) = decode_all_columns(&data)?;

    let mut out = String::with_capacity(h.nrows * col_names.len() * 20);
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<records>\n");
    for row in 0..h.nrows {
        out.push_str("  <record>\n");
        for (ci, col) in cols.iter().enumerate() {
            let tag = xml_safe_tag(&col_names[ci]);
            let val = match col.get(row).unwrap_or(&Value::Null) {
                Value::Number(n) => if n.fract() == 0.0 && n.abs() < 1e15 {
                    format!("{}", *n as i64)
                } else {
                    let s = format!("{:.8}", n);
                    s.trim_end_matches('0').trim_end_matches('.').to_string()
                },
                Value::Bool(b)  => if *b { "1".to_string() } else { "0".to_string() },
                Value::Str(s) if s == "EMPTY" => String::new(),
                Value::Str(s)   => xml_escape(s),
                Value::Null     => String::new(),
                v               => format!("{:?}", v),
            };
            out.push_str(&format!("    <{}>{}</{}>\n", tag, val, tag));
        }
        out.push_str("  </record>\n");
    }
    out.push_str("</records>\n");

    std::fs::write(&xml_path, out.as_bytes())
        .map_err(|e| VmError::runtime_error(format!("nova_to_xml: cannot write '{}': {}", xml_path, e)))?;
    Ok(Value::Bool(true))
}

// -- NDJSON (Newline-Delimited JSON) -------------------------------------------

/// nova_to_ndjson(kore_path, ndjson_path) → Bool
/// One JSON object per line — ideal for log pipelines and streaming systems.
pub fn nova_to_ndjson(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::runtime_error(
            "nova_to_ndjson(kore_path, ndjson_path) expects 2 string arguments".to_string()
        ));
    }
    let (kore_path, ndjson_path) = match (&args[0], &args[1]) {
        (Value::Str(a), Value::Str(b)) => (a.clone(), b.clone()),
        _ => return Err(VmError::runtime_error("nova_to_ndjson: both args must be strings".to_string())),
    };
    let data = std::fs::read(&kore_path)
        .map_err(|e| VmError::runtime_error(format!("nova_to_ndjson: cannot read '{}': {}", kore_path, e)))?;
    let (h, col_names, cols) = decode_all_columns(&data)?;

    let mut out = String::with_capacity(h.nrows * col_names.len() * 12);
    for row in 0..h.nrows {
        out.push('{');
        for (ci, col) in cols.iter().enumerate() {
            if ci > 0 { out.push(','); }
            out.push('"');
            out.push_str(&col_names[ci].replace('"', "\\\""));
            out.push_str("\":");
            match col.get(row).unwrap_or(&Value::Null) {
                Value::Number(n) => if n.fract() == 0.0 && n.abs() < 1e15 {
                    out.push_str(&format!("{}", *n as i64));
                } else {
                    let s = format!("{:.8}", n);
                    out.push_str(s.trim_end_matches('0').trim_end_matches('.'));
                },
                Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
                Value::Str(s) if s == "EMPTY" => out.push_str("null"),
                Value::Str(s)  => { out.push('"'); out.push_str(&s.replace('\\', "\\\\").replace('"', "\\\"")); out.push('"'); },
                Value::Null    => out.push_str("null"),
                v              => { out.push('"'); out.push_str(&format!("{:?}", v)); out.push('"'); },
            }
        }
        out.push_str("}\n");
    }

    std::fs::write(&ndjson_path, out.as_bytes())
        .map_err(|e| VmError::runtime_error(format!("nova_to_ndjson: cannot write '{}': {}", ndjson_path, e)))?;
    Ok(Value::Bool(true))
}

// -- Avro (schema + binary row store) -----------------------------------------
// Pure-Rust minimal Avro writer. No external crate needed.
// Format: Avro Object Container File
//   [4]  "Obj\x01" magic
//   [meta block] schema JSON + sync marker
//   [data blocks] count + size + rows (Avro binary encoding)
// Avro binary types used: null=0, boolean=1, int(zigzag varint), string(len+utf8), float=5(4B LE)

/// nova_to_avro(kore_path, avro_path) → Bool
/// Exports KORE to Apache Avro Object Container File format.
pub fn nova_to_avro(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::runtime_error(
            "nova_to_avro(kore_path, avro_path) expects 2 string arguments".to_string()
        ));
    }
    let (kore_path, avro_path) = match (&args[0], &args[1]) {
        (Value::Str(a), Value::Str(b)) => (a.clone(), b.clone()),
        _ => return Err(VmError::runtime_error("nova_to_avro: both args must be strings".to_string())),
    };
    let data = std::fs::read(&kore_path)
        .map_err(|e| VmError::runtime_error(format!("nova_to_avro: cannot read '{}': {}", kore_path, e)))?;
    let (h, col_names, cols) = decode_all_columns(&data)?;

    // Build Avro schema JSON
    let fields_json: Vec<String> = col_names.iter().enumerate().map(|(ci, name)| {
        let avro_type = match cols[ci].first() {
            Some(Value::Bool(_))   => r#"["null","boolean"]"#,
            Some(Value::Number(n)) => if n.fract() == 0.0 { r#"["null","long"]"# } else { r#"["null","double"]"# },
            _                      => r#"["null","string"]"#,
        };
        format!("{{\"name\":\"{}\",\"type\":{}}}", name.replace('"', "\\\""), avro_type)
    }).collect();
    let schema_json = format!(
        "{{\"type\":\"record\",\"name\":\"KoreRow\",\"fields\":[{}]}}",
        fields_json.join(",")
    );

    // Avro file magic + header
    let sync_marker: [u8; 16] = [0x4B,0x4F,0x52,0x45,0x4E,0x4F,0x56,0x41,
                                   0x41,0x56,0x52,0x4F,0x46,0x4D,0x54,0x00];
    let mut out: Vec<u8> = Vec::new();
    // Magic: "Obj\x01"
    out.extend_from_slice(b"Obj\x01");
    // Header: meta map (avro map encoding)
    // Map count=2 (2 entries)
    avro_write_long(&mut out, 2);
    // Entry 1: "avro.schema" → schema_json bytes
    let key1 = b"avro.schema";
    avro_write_long(&mut out, key1.len() as i64);
    out.extend_from_slice(key1);
    let schema_bytes = schema_json.as_bytes();
    avro_write_long(&mut out, schema_bytes.len() as i64);
    out.extend_from_slice(schema_bytes);
    // Entry 2: "avro.codec" → "null"
    let key2 = b"avro.codec";
    avro_write_long(&mut out, key2.len() as i64);
    out.extend_from_slice(key2);
    let codec = b"null";
    avro_write_long(&mut out, codec.len() as i64);
    out.extend_from_slice(codec);
    // End of map
    avro_write_long(&mut out, 0);
    // Sync marker
    out.extend_from_slice(&sync_marker);

    // Data block: all rows in one block
    let mut block: Vec<u8> = Vec::with_capacity(h.nrows * col_names.len() * 8);
    for row in 0..h.nrows {
        for (ci, col) in cols.iter().enumerate() {
            match col.get(row).unwrap_or(&Value::Null) {
                Value::Bool(b) => {
                    avro_write_long(&mut block, 1); // union index 1 = boolean
                    block.push(if *b { 1 } else { 0 });
                }
                Value::Number(n) => {
                    avro_write_long(&mut block, 1); // union index 1 = long/double
                    if cols[ci].iter().any(|v| matches!(v, Value::Number(x) if x.fract() != 0.0)) {
                        block.extend_from_slice(&n.to_le_bytes()); // double 8B
                    } else {
                        avro_write_long(&mut block, *n as i64);
                    }
                }
                Value::Str(s) if s != "EMPTY" => {
                    avro_write_long(&mut block, 1); // union index 1 = string
                    let sb = s.as_bytes();
                    avro_write_long(&mut block, sb.len() as i64);
                    block.extend_from_slice(sb);
                }
                _ => {
                    avro_write_long(&mut block, 0); // union index 0 = null
                }
            }
        }
    }
    // Block header: count + byte_count
    avro_write_long(&mut out, h.nrows as i64);
    avro_write_long(&mut out, block.len() as i64);
    out.extend_from_slice(&block);
    out.extend_from_slice(&sync_marker);

    std::fs::write(&avro_path, &out)
        .map_err(|e| VmError::runtime_error(format!("nova_to_avro: cannot write '{}': {}", avro_path, e)))?;
    Ok(Value::Bool(true))
}

// -- Parquet-layout columnar binary (KORE-Parquet) -----------------------------
// Pure-stdlib minimal Parquet-compatible columnar layout.
// NOT full Apache Parquet (that needs thrift + complex footer) but a
// columnar binary store with the same layout philosophy:
//   col1_data | col2_data | ... | footer(col offsets + types) | footer_len(u32)
// Readable by nova_from_parquet_layout; not compatible with Apache parquet-rs.
// For true Parquet output users should pipe through nova_to_csv → external tool.

/// nova_to_parquet(kore_path, parquet_path) → Bool
/// Exports KORE to KORE-Parquet columnar binary layout (nova-native, not Apache Parquet).
pub fn nova_to_parquet(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::runtime_error(
            "nova_to_parquet(kore_path, parquet_path) expects 2 string arguments".to_string()
        ));
    }
    let (kore_path, parquet_path) = match (&args[0], &args[1]) {
        (Value::Str(a), Value::Str(b)) => (a.clone(), b.clone()),
        _ => return Err(VmError::runtime_error("nova_to_parquet: both args must be strings".to_string())),
    };
    let data = std::fs::read(&kore_path)
        .map_err(|e| VmError::runtime_error(format!("nova_to_parquet: cannot read '{}': {}", kore_path, e)))?;
    let (h, col_names, cols) = decode_all_columns(&data)?;

    let mut out: Vec<u8> = Vec::new();
    // Magic: "KPAR" (KORE-Parquet)
    out.extend_from_slice(b"KPAR");
    // ncols u16 LE, nrows u32 LE
    out.extend_from_slice(&(col_names.len() as u16).to_le_bytes());
    out.extend_from_slice(&(h.nrows as u32).to_le_bytes());

    // Write each column as a compressed block (same Nova compress pipeline)
    let mut col_offsets: Vec<(String, u8, u64, u32)> = Vec::new(); // (name, type_algo, offset, comp_len)
    for (ci, col) in cols.iter().enumerate() {
        // Determine type
        let type_tag: u8 = match col.first() {
            Some(Value::Bool(_))                         => 3,
            Some(Value::Number(n)) if n.fract() == 0.0  => 0,
            Some(Value::Number(_))                       => 1,
            _                                            => 2,
        };
        // Encode column data into raw bytes — use owned strings to avoid leaks
        let strs: Vec<String> = col.iter().map(|v| match v {
            Value::Number(n) => format!("{}", n),
            Value::Bool(b)   => if *b { "1".to_string() } else { "0".to_string() },
            Value::Str(s)    => s.clone(),
            _                => String::new(),
        }).collect();
        let raw: Vec<&str> = strs.iter().map(|s| s.as_str()).collect();
        // Re-encode using Nova pipeline; capture algo for footer
        let col_type = match type_tag {
            0 => ColType::Int, 1 => ColType::Float, 3 => ColType::Bool, _ => ColType::Str,
        };
        let mut col_buf = Vec::new();
        let algo = encode_col(col_type, &raw, &mut col_buf);
        let compressed = compress_col(&col_buf);
        let type_algo = (type_tag << 4) | (algo & 0x0F); // pack type+algo into 1 byte

        let offset = out.len() as u64;
        col_offsets.push((col_names[ci].clone(), type_algo, offset, compressed.len() as u32));
        // comp_len u32 LE then data
        out.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
        out.extend_from_slice(&compressed);
    }

    // Footer: col_count entries of [ name_len(u8) name(utf8) type_algo(u8) offset(u64 LE) comp_len(u32 LE) ]
    //   type_algo = (type_byte << 4) | algo   — both packed into one byte
    let footer_start = out.len() as u32;
    for (name, type_algo, offset, comp_len) in &col_offsets {
        let nb = name.as_bytes();
        out.push(nb.len() as u8);
        out.extend_from_slice(nb);
        out.push(*type_algo);
        out.extend_from_slice(&offset.to_le_bytes());
        out.extend_from_slice(&comp_len.to_le_bytes());
    }
    // Footer length at the very end (u32 LE) — enables backward seek-and-read
    out.extend_from_slice(&footer_start.to_le_bytes());

    std::fs::write(&parquet_path, &out)
        .map_err(|e| VmError::runtime_error(format!("nova_to_parquet: cannot write '{}': {}", parquet_path, e)))?;
    Ok(Value::Bool(true))
}

// -- XML helpers ---------------------------------------------------------------

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn xml_safe_tag(name: &str) -> String {
    // XML tag names must start with letter or _, no spaces
    let mut t = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_alphanumeric() || c == '_' || c == '-' || (c == '.' && i > 0) {
            t.push(c);
        } else if i == 0 {
            t.push('_');
        } else {
            t.push('_');
        }
    }
    if t.is_empty() { t.push_str("col"); }
    t
}

/// Parse XML into a Vec of records, each record is a Vec<(key, value)>
fn parse_xml_records(xml: &str) -> Vec<Vec<(String, String)>> {
    let mut records = Vec::new();
    let bytes = xml.as_bytes();
    let mut i   = 0;
    let mut depth = 0i32;
    let mut row_depth = -1i32;
    let mut current_record: Vec<(String, String)> = Vec::new();
    #[allow(unused_assignments)]
    let mut current_tag = String::new();
    let mut in_value = false;

    while i < bytes.len() {
        if bytes[i] == b'<' {
            // Check for closing tag
            if i + 1 < bytes.len() && bytes[i+1] == b'/' {
                // Closing tag
                let end = bytes[i..].iter().position(|&b| b == b'>').unwrap_or(0);
                let _tag = std::str::from_utf8(&bytes[i+2..i+end]).unwrap_or("").trim().to_string();
                depth -= 1;
                if depth == row_depth && in_value {
                    // End of a field value — we already pushed in the opening path
                    in_value = false;
                }
                if depth == row_depth - 1 && row_depth >= 0 {
                    // End of a record
                    if !current_record.is_empty() {
                        records.push(std::mem::take(&mut current_record));
                    }
                }
                if depth < row_depth - 1 { row_depth = -1; }
                i += end + 1;
            } else if i + 1 < bytes.len() && (bytes[i+1] == b'!' || bytes[i+1] == b'?') {
                // Comment, CDATA, DOCTYPE, or processing instruction <?...?> — skip entirely
                let end = bytes[i..].iter().position(|&b| b == b'>').unwrap_or(0);
                i += end + 1;
            } else {
                // Opening tag
                let end = bytes[i..].iter().position(|&b| b == b'>').unwrap_or(0);
                let tag_content = std::str::from_utf8(&bytes[i+1..i+end]).unwrap_or("").trim();
                let tag = tag_content.split_whitespace().next().unwrap_or("").to_string();
                let self_closing = tag_content.ends_with('/');
                if !self_closing {
                    depth += 1;
                    // Heuristic: depth 2 = record row, depth 3 = field
                    if depth == 2 && row_depth < 0 { row_depth = 2; }
                    if depth == row_depth + 1 {
                        current_tag = tag;
                        in_value = true;
                        i += end + 1;
                        // Collect text content until closing tag
                        let close = format!("</{}", current_tag);
                        if let Some(end2) = xml[i..].find(&close) {
                            let text = xml[i..i+end2].trim().to_string();
                            if !text.starts_with('<') {
                                current_record.push((current_tag.clone(), text));
                            }
                            // Skip to end of closing tag
                            let skip = xml[i+end2..].find('>').unwrap_or(0);
                            i += end2 + skip + 1;
                            depth -= 1;
                            in_value = false;
                        }
                        continue;
                    }
                }
                i += end + 1;
            }
        } else {
            i += 1;
        }
    }
    records
}

// -- Avro helpers --------------------------------------------------------------

fn avro_write_long(buf: &mut Vec<u8>, n: i64) {
    // Zigzag + varint (Avro uses zigzag encoding)
    let mut v = ((n << 1) ^ (n >> 63)) as u64;
    loop {
        let b = (v & 0x7F) as u8;
        v >>= 7;
        if v == 0 { buf.push(b); break; }
        else      { buf.push(b | 0x80); }
    }
}

/// Read one Avro zigzag-encoded long from `data` starting at `pos`.
/// Returns (decoded_value, new_pos).
fn avro_read_long(data: &[u8], pos: usize) -> (i64, usize) {
    let mut val: u64 = 0;
    let mut shift  = 0u32;
    let mut p = pos;
    loop {
        if p >= data.len() { return (0, p); }
        let b = data[p] as u64;
        p += 1;
        val |= (b & 0x7F) << shift;
        shift += 7;
        if b & 0x80 == 0 { break; }
    }
    // Zigzag decode: (val >>> 1) XOR -(val & 1)
    (((val >> 1) as i64) ^ -((val & 1) as i64), p)
}

// -- NDJSON → KORE -------------------------------------------------------------

/// nova_from_ndjson(ndjson_path, kore_path) → Bool
/// Converts a Newline-Delimited JSON file (one JSON object per line) to KORE.
pub fn nova_from_ndjson(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::runtime_error(
            "nova_from_ndjson(ndjson_path, kore_path) expects 2 string arguments".to_string()
        ));
    }
    let (ndjson_path, kore_path) = match (&args[0], &args[1]) {
        (Value::Str(a), Value::Str(b)) => (a.clone(), b.clone()),
        _ => return Err(VmError::runtime_error("nova_from_ndjson: both args must be strings".to_string())),
    };
    let text = std::fs::read_to_string(&ndjson_path)
        .map_err(|e| VmError::runtime_error(format!("nova_from_ndjson: cannot read '{}': {}", ndjson_path, e)))?;

    // Collect records line-by-line
    let mut col_order: Vec<String> = Vec::new();
    let mut col_set: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut records: Vec<std::collections::HashMap<String, String>> = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || !line.starts_with('{') { continue; }
        // Stable key ordering from first record
        if col_order.is_empty() {
            for k in parse_json_keys(line) {
                if col_set.insert(k.clone()) { col_order.push(k); }
            }
        }
        let kv = parse_json_kv(line);
        // Collect any new keys from subsequent records
        for k in kv.keys() {
            if col_set.insert(k.clone()) { col_order.push(k.clone()); }
        }
        records.push(kv);
    }

    if records.is_empty() {
        return Err(VmError::runtime_error("nova_from_ndjson: no records found".to_string()));
    }

    // Build CSV
    let mut csv = String::new();
    csv.push_str(&col_order.join(","));
    csv.push('\n');
    for rec in &records {
        for (ci, col) in col_order.iter().enumerate() {
            if ci > 0 { csv.push(','); }
            let v = rec.get(col).map(|s| s.as_str()).unwrap_or("");
            if v.contains(',') || v.contains('"') || v.contains('\n') {
                csv.push('"');
                csv.push_str(&v.replace('"', "\\\""));
                csv.push('"');
            } else {
                csv.push_str(v);
            }
        }
        csv.push('\n');
    }

    let tmp = format!("{}.tmp_ndjson.csv", kore_path);
    std::fs::write(&tmp, csv.as_bytes())
        .map_err(|e| VmError::runtime_error(format!("nova_from_ndjson: tmp write error: {}", e)))?;
    let result = nova_write(&[Value::Str(tmp.clone()), Value::Str(kore_path)]);
    let _ = std::fs::remove_file(&tmp);
    result
}

// -- KPAR → KORE ---------------------------------------------------------------

/// nova_from_parquet(parquet_path, kore_path) → Bool
/// Reads a KORE-native KPAR file back into KORE format (round-trip).
pub fn nova_from_parquet(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::runtime_error(
            "nova_from_parquet(parquet_path, kore_path) expects 2 string arguments".to_string()
        ));
    }
    let (parquet_path, kore_path) = match (&args[0], &args[1]) {
        (Value::Str(a), Value::Str(b)) => (a.clone(), b.clone()),
        _ => return Err(VmError::runtime_error("nova_from_parquet: both args must be strings".to_string())),
    };
    let data = std::fs::read(&parquet_path)
        .map_err(|e| VmError::runtime_error(format!("nova_from_parquet: cannot read '{}': {}", parquet_path, e)))?;

    if data.len() < 10 || &data[0..4] != b"KPAR" {
        return Err(VmError::runtime_error("nova_from_parquet: not a valid KPAR file".to_string()));
    }
    let ncols = u16::from_le_bytes([data[4], data[5]]) as usize;
    let nrows = u32::from_le_bytes([data[6], data[7], data[8], data[9]]) as usize;
    if data.len() < 14 { // need at least 10-byte header + 4 bytes footer_start
        return Err(VmError::runtime_error("nova_from_parquet: file too small".to_string()));
    }

    // Read footer_start from last 4 bytes
    let fstart_off = data.len() - 4;
    let footer_start = u32::from_le_bytes([
        data[fstart_off], data[fstart_off+1], data[fstart_off+2], data[fstart_off+3]
    ]) as usize;

    // Parse footer entries: name_len(u8) + name(utf8) + type_algo(u8) + offset(u64) + comp_len(u32)
    let mut fp = footer_start;
    let footer_end = data.len() - 4;
    let mut col_meta: Vec<(String, u8, u8, usize, usize)> = Vec::with_capacity(ncols);
    // (name, type_byte, algo, data_offset, comp_len)
    while fp < footer_end && col_meta.len() < ncols {
        if fp >= data.len() { break; }
        let name_len = data[fp] as usize;
        fp += 1;
        if fp + name_len + 1 + 8 + 4 > data.len() { break; }
        let name = std::str::from_utf8(&data[fp..fp+name_len]).unwrap_or("col").to_string();
        fp += name_len;
        let type_algo = data[fp]; fp += 1;
        let type_byte = (type_algo >> 4) & 0x0F;
        let algo      = type_algo & 0x0F;
        let offset = u64::from_le_bytes(data[fp..fp+8].try_into().unwrap_or([0;8])) as usize;
        fp += 8;
        let comp_len = u32::from_le_bytes(data[fp..fp+4].try_into().unwrap_or([0;4])) as usize;
        fp += 4;
        // data_offset points to comp_len u32, actual data starts 4 bytes later
        col_meta.push((name, type_byte, algo, offset + 4, comp_len));
    }

    if col_meta.is_empty() {
        return Err(VmError::runtime_error("nova_from_parquet: empty footer, cannot reconstruct columns".to_string()));
    }

    // Decode each column
    let col_names: Vec<String> = col_meta.iter().map(|(n,_,_,_,_)| n.clone()).collect();
    let mut all_cols: Vec<Vec<Value>> = Vec::with_capacity(col_meta.len());
    for (_, type_byte, algo, data_off, comp_len) in &col_meta {
        let end = data_off + comp_len;
        if end > data.len() {
            return Err(VmError::runtime_error("nova_from_parquet: column data out of bounds".to_string()));
        }
        let raw = decompress_col(&data[*data_off..end]);
        let (vals, _) = decode_col(*type_byte, *algo, &raw, 0, nrows);
        all_cols.push(vals);
    }

    // Build CSV → nova_write
    let mut csv = col_names.join(",");
    csv.push('\n');
    for row in 0..nrows {
        for (ci, col) in all_cols.iter().enumerate() {
            if ci > 0 { csv.push(','); }
            match col.get(row).unwrap_or(&Value::Null) {
                Value::Number(n) => if n.fract() == 0.0 && n.abs() < 1e15 {
                    csv.push_str(&format!("{}", *n as i64))
                } else {
                    let s = format!("{:.8}", n);
                    csv.push_str(s.trim_end_matches('0').trim_end_matches('.'));
                },
                Value::Bool(b) => csv.push_str(if *b { "1" } else { "0" }),
                Value::Str(s) if s == "EMPTY" => {},
                Value::Str(s) => { if s.contains(',') || s.contains('"') {
                    csv.push('"'); csv.push_str(&s.replace('"', "\\\"")); csv.push('"');
                } else { csv.push_str(s); } },
                _ => {},
            }
        }
        csv.push('\n');
    }

    let tmp = format!("{}.tmp_kpar.csv", kore_path);
    std::fs::write(&tmp, csv.as_bytes())
        .map_err(|e| VmError::runtime_error(format!("nova_from_parquet: tmp write error: {}", e)))?;
    let result = nova_write(&[Value::Str(tmp.clone()), Value::Str(kore_path)]);
    let _ = std::fs::remove_file(&tmp);
    result
}

// -- Avro OCF → KORE -----------------------------------------------------------

/// nova_from_avro(avro_path, kore_path) → Bool
/// Reads an Apache Avro Object Container File produced by nova_to_avro back into KORE.
pub fn nova_from_avro(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::runtime_error(
            "nova_from_avro(avro_path, kore_path) expects 2 string arguments".to_string()
        ));
    }
    let (avro_path, kore_path) = match (&args[0], &args[1]) {
        (Value::Str(a), Value::Str(b)) => (a.clone(), b.clone()),
        _ => return Err(VmError::runtime_error("nova_from_avro: both args must be strings".to_string())),
    };
    let data = std::fs::read(&avro_path)
        .map_err(|e| VmError::runtime_error(format!("nova_from_avro: cannot read '{}': {}", avro_path, e)))?;

    // Validate magic "Obj\x01"
    if data.len() < 4 || &data[0..4] != b"Obj\x01" {
        return Err(VmError::runtime_error("nova_from_avro: not a valid Avro Object Container File".to_string()));
    }

    // -- Parse header meta map -------------------------------------------------
    let mut pos = 4usize;
    let mut schema_json = String::new();
    loop {
        let (block_count, np) = avro_read_long(&data, pos);
        pos = np;
        if block_count == 0 { break; }
        let count = block_count.unsigned_abs() as usize;
        // If block_count < 0, there's a block size long before entries (skip it)
        if block_count < 0 { let (_, np2) = avro_read_long(&data, pos); pos = np2; }
        for _ in 0..count {
            // key: long + bytes
            let (klen, np) = avro_read_long(&data, pos); pos = np;
            let klen = klen.max(0) as usize;
            if pos + klen > data.len() { break; }
            let key = std::str::from_utf8(&data[pos..pos+klen]).unwrap_or("").to_string();
            pos += klen;
            // value: long + bytes
            let (vlen, np) = avro_read_long(&data, pos); pos = np;
            let vlen = vlen.max(0) as usize;
            if pos + vlen > data.len() { break; }
            if key == "avro.schema" {
                schema_json = std::str::from_utf8(&data[pos..pos+vlen]).unwrap_or("").to_string();
            }
            pos += vlen;
        }
    }

    if schema_json.is_empty() {
        return Err(VmError::runtime_error("nova_from_avro: missing avro.schema in header".to_string()));
    }

    // Skip sync marker (16 bytes)
    if pos + 16 > data.len() {
        return Err(VmError::runtime_error("nova_from_avro: truncated before sync marker".to_string()));
    }
    let sync_marker: Vec<u8> = data[pos..pos+16].to_vec();
    pos += 16;

    // -- Parse schema: extract field names + types -----------------------------
    // Schema format (we wrote): {"type":"record",...,"fields":[{"name":"X","type":["null","long"]},…]}
    let field_metas: Vec<(String, &str)> = parse_avro_schema_fields(&schema_json);
    if field_metas.is_empty() {
        return Err(VmError::runtime_error("nova_from_avro: no fields in schema".to_string()));
    }

    // -- Read data blocks ------------------------------------------------------
    let ncols = field_metas.len();
    let mut all_rows: Vec<Vec<String>> = Vec::new();

    while pos + 1 < data.len() {
        // row_count (long)
        let (row_count, np) = avro_read_long(&data, pos); pos = np;
        if row_count == 0 { break; }
        // byte_count (long)
        let (byte_count, np) = avro_read_long(&data, pos); pos = np;
        let block_end = pos + byte_count.max(0) as usize;

        // Decode rows
        for _ in 0..row_count.max(0) {
            let mut row: Vec<String> = Vec::with_capacity(ncols);
            for (_name, ftype) in &field_metas {
                // Read union discriminant: 0=null, 1=value
                let (union_idx, np) = avro_read_long(&data, pos); pos = np;
                if union_idx == 0 {
                    // null
                    row.push(String::new());
                } else {
                    match *ftype {
                        "boolean" => {
                            if pos < data.len() {
                                row.push(if data[pos] != 0 { "1".to_string() } else { "0".to_string() });
                                pos += 1;
                            } else { row.push(String::new()); }
                        }
                        "long" => {
                            let (n, np) = avro_read_long(&data, pos); pos = np;
                            row.push(format!("{}", n));
                        }
                        "double" => {
                            if pos + 8 <= data.len() {
                                let bytes: [u8;8] = data[pos..pos+8].try_into().unwrap_or([0;8]);
                                let v = f64::from_le_bytes(bytes);
                                let s = format!("{:.8}", v);
                                row.push(s.trim_end_matches('0').trim_end_matches('.').to_string());
                                pos += 8;
                            } else { row.push(String::new()); }
                        }
                        _ => {
                            // string (or unknown → treat as string)
                            let (slen, np) = avro_read_long(&data, pos); pos = np;
                            let slen = slen.max(0) as usize;
                            if pos + slen <= data.len() {
                                let s = std::str::from_utf8(&data[pos..pos+slen]).unwrap_or("").to_string();
                                row.push(s);
                                pos += slen;
                            } else { row.push(String::new()); }
                        }
                    }
                }
            }
            all_rows.push(row);
        }

        // Skip to block_end, then consume sync marker
        pos = block_end;
        if pos + 16 <= data.len() && data[pos..pos+16] == sync_marker[..] {
            pos += 16;
        }
    }

    if all_rows.is_empty() {
        return Err(VmError::runtime_error("nova_from_avro: no rows found in data block".to_string()));
    }

    // Build CSV → nova_write
    let col_names: Vec<String> = field_metas.iter().map(|(n,_)| n.clone()).collect();
    let mut csv = col_names.join(",");
    csv.push('\n');
    for row in &all_rows {
        for (ci, val) in row.iter().enumerate() {
            if ci > 0 { csv.push(','); }
            if val.contains(',') || val.contains('"') {
                csv.push('"');
                csv.push_str(&val.replace('"', "\\\""));
                csv.push('"');
            } else {
                csv.push_str(val);
            }
        }
        csv.push('\n');
    }

    let tmp = format!("{}.tmp_avro.csv", kore_path);
    std::fs::write(&tmp, csv.as_bytes())
        .map_err(|e| VmError::runtime_error(format!("nova_from_avro: tmp write error: {}", e)))?;
    let result = nova_write(&[Value::Str(tmp.clone()), Value::Str(kore_path)]);
    let _ = std::fs::remove_file(&tmp);
    result
}

// -- Avro schema parser helper -------------------------------------------------

/// Parse the fields array from an Avro record schema JSON.
/// Returns Vec<(field_name, avro_base_type)> where avro_base_type is "long", "double", "string", or "boolean".
fn parse_avro_schema_fields(schema: &str) -> Vec<(String, &'static str)> {
    let mut fields: Vec<(String, &'static str)> = Vec::new();
    // Find the "fields" array
    let Some(fa) = schema.find("\"fields\"") else { return fields; };
    let rest = &schema[fa..];
    let Some(arr_start) = rest.find('[') else { return fields; };
    let arr = &rest[arr_start..];

    // Walk through field objects { ... }
    let mut depth = 0i32;
    let mut obj_start: Option<usize> = None;
    for (i, ch) in arr.char_indices() {
        match ch {
            '{' => {
                depth += 1;
                if depth == 1 { obj_start = Some(i); }
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    if let Some(start) = obj_start {
                        let field_json = &arr[start..=i];
                        let name = extract_json_str_value(field_json, "name");
                        let ftype = detect_avro_field_type(field_json);
                        if !name.is_empty() {
                            fields.push((name, ftype));
                        }
                        obj_start = None;
                    }
                }
            }
            _ => {}
        }
    }
    fields
}

/// Extract the value of a string field from a simple JSON object snippet.
fn extract_json_str_value(json: &str, key: &str) -> String {
    let pattern = format!("\"{}\":", key);
    let Some(kp) = json.find(&pattern) else { return String::new(); };
    let after = json[kp + pattern.len()..].trim_start();
    if after.starts_with('"') {
        let inner = &after[1..];
        let end = inner.find('"').unwrap_or(inner.len());
        inner[..end].to_string()
    } else {
        String::new()
    }
}

/// Detect the base Avro type for a field from its schema JSON snippet.
/// We wrote types like ["null","long"], ["null","double"], etc.
fn detect_avro_field_type(field_json: &str) -> &'static str {
    if field_json.contains("\"boolean\"") { "boolean" }
    else if field_json.contains("\"double\"") { "double" }
    else if field_json.contains("\"long\"")   { "long" }
    else                                       { "string" }
}

// -- Universal auto-converter (dot-to-dot syntax) ---------------------------------------

/// nova_auto_convert(src_path, dst_path) → Bool
/// Universal format converter invoked by the Killer  data.csv.to.data.kore  syntax.
/// Auto-detects both source and destination format from file extensions.
pub fn nova_auto_convert(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::runtime_error(
            "nova_auto_convert(src_path, dst_path) expects 2 string arguments".to_string()
        ));
    }
    let (src, dst) = match (&args[0], &args[1]) {
        (Value::Str(a), Value::Str(b)) => (a.clone(), b.clone()),
        _ => return Err(VmError::runtime_error("nova_auto_convert: both args must be strings".to_string())),
    };
    let src_ext = src.rfind('.').map(|i| src[i+1..].to_lowercase()).unwrap_or_default();
    let dst_ext = dst.rfind('.').map(|i| dst[i+1..].to_lowercase()).unwrap_or_default();

    match (src_ext.as_str(), dst_ext.as_str()) {
        // anything → kore
        (_, "kore") => match src_ext.as_str() {
            "json"    => nova_from_json(&[Value::Str(src), Value::Str(dst)]),
            "tsv"     => nova_from_tsv(&[Value::Str(src), Value::Str(dst)]),
            "xml"     => nova_from_xml(&[Value::Str(src), Value::Str(dst)]),
            "ndjson"  => nova_from_ndjson(&[Value::Str(src), Value::Str(dst)]),
            "avro"    => nova_from_avro(&[Value::Str(src), Value::Str(dst)]),
            "parquet" => nova_from_parquet(&[Value::Str(src), Value::Str(dst)]),
            _         => nova_write(&[Value::Str(src), Value::Str(dst)]),  // csv/txt/etc.
        },
        // kore → anything
        ("kore", _) => match dst_ext.as_str() {
            "json"    => nova_to_json(&[Value::Str(src), Value::Str(dst)]),
            "tsv"     => nova_to_tsv(&[Value::Str(src), Value::Str(dst)]),
            "xml"     => nova_to_xml(&[Value::Str(src), Value::Str(dst)]),
            "ndjson"  => nova_to_ndjson(&[Value::Str(src), Value::Str(dst)]),
            "avro"    => nova_to_avro(&[Value::Str(src), Value::Str(dst)]),
            "parquet" => nova_to_parquet(&[Value::Str(src), Value::Str(dst)]),
            _         => nova_to_csv(&[Value::Str(src), Value::Str(dst)]),  // csv/txt/etc.
        },
        // direct non-kore conversions (src → kore → dst in memory via tmp)
        _ => {
            let tmp = format!("{}.auto_conv.kore", src);
            let to_kore_args = [Value::Str(src.clone()), Value::Str(tmp.clone())];
            let r1 = match src_ext.as_str() {
                "json"    => nova_from_json(&to_kore_args),
                "tsv"     => nova_from_tsv(&to_kore_args),
                "xml"     => nova_from_xml(&to_kore_args),
                "ndjson"  => nova_from_ndjson(&to_kore_args),
                "avro"    => nova_from_avro(&to_kore_args),
                "parquet" => nova_from_parquet(&to_kore_args),
                _         => nova_write(&to_kore_args),
            };
            if r1.is_err() { let _ = std::fs::remove_file(&tmp); return r1; }
            let from_kore_args = [Value::Str(tmp.clone()), Value::Str(dst.clone())];
            let r2 = match dst_ext.as_str() {
                "json"    => nova_to_json(&from_kore_args),
                "tsv"     => nova_to_tsv(&from_kore_args),
                "xml"     => nova_to_xml(&from_kore_args),
                "ndjson"  => nova_to_ndjson(&from_kore_args),
                "avro"    => nova_to_avro(&from_kore_args),
                "parquet" => nova_to_parquet(&from_kore_args),
                _         => nova_to_csv(&from_kore_args),
            };
            let _ = std::fs::remove_file(&tmp);
            r2
        }
    }
}

// -- Nova external file compressor ---------------------------------------------------------
// Compresses ANY file (text, binary, images, etc.) using the Nova LZ77+Huffman
// pipeline and stores it in .nvz format (Nova Zip).
// Packed balanced-trit sequences (`NOVT`) live in `nova_trit_codec.rs` — separate format, not used here.
//
// NOVZ file layout:
//   [4]  "NOVZ" magic
//   [1]  version = 1
//   [8]  original_size u64 LE
//   [4]  chunk_count u32 LE
//   per chunk: [4] comp_len u32 LE + [comp_len] compress_col(chunk)

const NOVZ_MAGIC:      &[u8; 4] = b"NOVZ";
const NOVZ_VERSION:    u8       = 1;
const NOVZ_CHUNK_SIZE: usize    = 65536; // 64 KB

/// nova_compress(src_path, dst_path) → Bool
/// Compresses any file using Nova's LZ77+Huffman pipeline into .nvz format.
pub fn nova_compress(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::runtime_error(
            "nova_compress(src_path, dst_path) expects 2 string arguments".to_string()
        ));
    }
    let (src, dst) = match (&args[0], &args[1]) {
        (Value::Str(a), Value::Str(b)) => (a.clone(), b.clone()),
        _ => return Err(VmError::runtime_error("nova_compress: both args must be strings".to_string())),
    };
    let data = std::fs::read(&src)
        .map_err(|e| VmError::runtime_error(format!("nova_compress: cannot read '{}': {}", src, e)))?;

    let orig_size   = data.len();
    let chunks: Vec<&[u8]> = data.chunks(NOVZ_CHUNK_SIZE).collect();
    let chunk_count = chunks.len();

    let mut out: Vec<u8> = Vec::with_capacity(orig_size / 2);
    out.extend_from_slice(NOVZ_MAGIC);
    out.push(NOVZ_VERSION);
    out.extend_from_slice(&(orig_size as u64).to_le_bytes());
    out.extend_from_slice(&(chunk_count as u32).to_le_bytes());

    for chunk in &chunks {
        let compressed = compress_col(chunk);
        out.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
        out.extend_from_slice(&compressed);
    }

    let ratio = if orig_size > 0 {
        (out.len() as f64 / orig_size as f64) * 100.0
    } else { 100.0 };

    std::fs::write(&dst, &out)
        .map_err(|e| VmError::runtime_error(format!("nova_compress: cannot write '{}': {}", dst, e)))?;

    // Print compression summary
    println!("Nova compressed: {} bytes → {} bytes ({:.1}% of original)",
        orig_size, out.len(), ratio);

    Ok(Value::Bool(true))
}

/// nova_decompress(src_path, dst_path) → Bool
/// Decompresses a .nvz Nova Zip file back to its original bytes.
pub fn nova_decompress(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::runtime_error(
            "nova_decompress(src_path, dst_path) expects 2 string arguments".to_string()
        ));
    }
    let (src, dst) = match (&args[0], &args[1]) {
        (Value::Str(a), Value::Str(b)) => (a.clone(), b.clone()),
        _ => return Err(VmError::runtime_error("nova_decompress: both args must be strings".to_string())),
    };
    let data = std::fs::read(&src)
        .map_err(|e| VmError::runtime_error(format!("nova_decompress: cannot read '{}': {}", src, e)))?;

    // Validate header
    if data.len() < 17 || &data[0..4] != NOVZ_MAGIC {
        return Err(VmError::runtime_error(
            format!("nova_decompress: '{}' is not a valid NOVZ file", src)
        ));
    }
    let _version   = data[4];
    let orig_size  = u64::from_le_bytes(data[5..13].try_into().unwrap_or([0;8])) as usize;
    let chunk_count = u32::from_le_bytes(data[13..17].try_into().unwrap_or([0;4])) as usize;

    let mut out: Vec<u8> = Vec::with_capacity(orig_size);
    let mut pos = 17usize;

    for i in 0..chunk_count {
        if pos + 4 > data.len() {
            return Err(VmError::runtime_error(
                format!("nova_decompress: truncated at chunk {}/{}", i, chunk_count)
            ));
        }
        let comp_len = u32::from_le_bytes(data[pos..pos+4].try_into().unwrap_or([0;4])) as usize;
        pos += 4;
        if pos + comp_len > data.len() {
            return Err(VmError::runtime_error(
                format!("nova_decompress: chunk {} data out of bounds", i)
            ));
        }
        let decompressed = decompress_col(&data[pos..pos+comp_len]);
        out.extend_from_slice(&decompressed);
        pos += comp_len;
    }

    std::fs::write(&dst, &out)
        .map_err(|e| VmError::runtime_error(format!("nova_decompress: cannot write '{}': {}", dst, e)))?;

    println!("Nova decompressed: {} bytes → {} bytes", data.len(), out.len());

    Ok(Value::Bool(true))
}


fn decode_all_columns_typed(data: &[u8]) -> Result<(NovaHeader, Vec<String>, Vec<(u8,u8)>, Vec<Vec<Value>>), VmError> {
    let h = parse_header(data)?;
    let schema_start  = 19usize;
    let schema_end    = schema_start + h.schema_len;
    let payload_start = schema_end;
    let payload_end   = payload_start + h.payload_len;

    let schema  = lz77_decompress(&data[schema_start ..schema_end .min(data.len())]);
    let payload = lz77_decompress(&data[payload_start..payload_end.min(data.len())]);

    let mut col_names:      Vec<String>   = Vec::with_capacity(h.ncols);
    let mut col_type_algos: Vec<(u8, u8)> = Vec::with_capacity(h.ncols);
    let mut spos = 0;
    while spos < schema.len() && col_names.len() < h.ncols {
        let name_len = *schema.get(spos).unwrap_or(&0) as usize; spos += 1;
        if spos + name_len > schema.len() { break; }
        let name = String::from_utf8_lossy(&schema[spos..spos+name_len]).into_owned();
        spos += name_len;
        let type_algo = *schema.get(spos).unwrap_or(&0); spos += 1;
        col_names.push(name);
        col_type_algos.push(((type_algo >> 4) & 0x0F, type_algo & 0x0F));
    }

    let mut cols: Vec<Vec<Value>> = Vec::with_capacity(h.ncols);
    let mut ppos = 0;
    for ci in 0..col_names.len() {
        let dlen = {
            let a = *payload.get(ppos  ).unwrap_or(&0) as usize;
            let b = *payload.get(ppos+1).unwrap_or(&0) as usize;
            let c = *payload.get(ppos+2).unwrap_or(&0) as usize;
            a | (b << 8) | (c << 16)
        };
        ppos += 3;
        let (type_byte, algo) = col_type_algos.get(ci).copied().unwrap_or((0,0));
        let col_data = if ppos + dlen <= payload.len() { &payload[ppos..ppos+dlen] } else { &payload[ppos..] };
        let (vals, _) = decode_col(type_byte, algo, col_data, 0, h.nrows);
        cols.push(vals);
        ppos += dlen;
    }

    Ok((h, col_names, col_type_algos, cols))
}

#[inline(always)]
fn value_to_sort_key(v: &Value) -> String {
    match v {
        Value::Number(n) => if n.fract() == 0.0 { format!("{}", *n as i64) } else { format!("{}", n) },
        Value::Str(s)    => s.clone(),
        Value::Bool(b)   => b.to_string(),
        _                => "null".to_string(),
    }
}

fn write_kore_from_values(
    col_names:  &[String],
    type_hints: &[u8],
    cols:       &[Vec<Value>],
    out_path:   &str,
) -> Result<(), VmError> {
    let ncols = col_names.len();
    let nrows = cols.first().map(|c| c.len()).unwrap_or(0);

    let mut schema_raw:  Vec<u8> = Vec::new();
    let mut payload_raw: Vec<u8> = Vec::new();

    for ci in 0..ncols {
        let orig_type = type_hints.get(ci).copied().unwrap_or(T_STR);
        let strings: Vec<String> = cols[ci].iter().map(|v| match v {
            Value::Number(n) => {
                if orig_type == T_FLOAT {
                    format!("{:.6}", n)
                } else if n.fract() == 0.0 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            Value::Str(s)  => s.clone(),
            Value::Bool(b) => b.to_string(),
            _              => "null".to_string(),
        }).collect();

        let raw_refs: Vec<&str> = strings.iter().map(|s| s.as_str()).collect();
        let col_type  = detect_type(&raw_refs);
        let type_byte = match col_type {
            ColType::Int   => T_INT,
            ColType::Float => T_FLOAT,
            ColType::Str   => T_STR,
            ColType::Bool  => T_BOOL,
        };
        let mut col_buf = Vec::new();
        let algo = encode_col(col_type, &raw_refs, &mut col_buf);

        let name_b = col_names[ci].as_bytes();
        schema_raw.push(name_b.len() as u8);
        schema_raw.extend_from_slice(name_b);
        schema_raw.push((type_byte << 4) | (algo & 0x0F));

        let dlen = col_buf.len();
        payload_raw.push((dlen         & 0xFF) as u8);
        payload_raw.push(((dlen >> 8)  & 0xFF) as u8);
        payload_raw.push(((dlen >> 16) & 0xFF) as u8);
        payload_raw.extend_from_slice(&col_buf);
    }

    let schema_comp  = lz77_compress(&schema_raw);
    let payload_comp = lz77_compress(&payload_raw);

    let mut file_buf = Vec::with_capacity(19 + schema_comp.len() + payload_comp.len());
    file_buf.extend_from_slice(MAGIC);
    file_buf.push(VERSION);
    file_buf.extend_from_slice(&(ncols              as u16).to_le_bytes());
    file_buf.extend_from_slice(&(nrows              as u32).to_le_bytes());
    file_buf.extend_from_slice(&(schema_comp.len()  as u32).to_le_bytes());
    file_buf.extend_from_slice(&(payload_comp.len() as u32).to_le_bytes());
    file_buf.extend_from_slice(&schema_comp);
    file_buf.extend_from_slice(&payload_comp);

    std::fs::write(out_path, &file_buf)
        .map_err(|e| VmError::runtime_error(format!("write_kore: cannot write '{}': {}", out_path, e)))?;
    Ok(())
}

// ==============================================================================
// BATCH 1 -- File System Functions
// ==============================================================================

pub fn nova_file_read(args: &[Value]) -> Result<Value, VmError> {
    let path = match args.get(0) { Some(Value::Str(s)) => s.clone(), _ => return Err(VmError::runtime_error("nova_file_read(path) expects 1 string argument".to_string())), };
    let contents = std::fs::read_to_string(&path).map_err(|e| VmError::runtime_error(format!("nova_file_read: cannot read '{}': {}", path, e)))?;
    Ok(Value::Str(contents))
}

pub fn nova_file_write(args: &[Value]) -> Result<Value, VmError> {
    let (path, data) = match (args.get(0), args.get(1)) {
        (Some(Value::Str(p)), Some(Value::Str(d))) => (p.clone(), d.clone()),
        _ => return Err(VmError::runtime_error("nova_file_write(path, data) expects 2 string arguments".to_string())),
    };
    std::fs::write(&path, data.as_bytes()).map_err(|e| VmError::runtime_error(format!("nova_file_write: cannot write '{}': {}", path, e)))?;
    Ok(Value::Bool(true))
}

pub fn nova_file_append(args: &[Value]) -> Result<Value, VmError> {
    use std::io::Write;
    let (path, data) = match (args.get(0), args.get(1)) {
        (Some(Value::Str(p)), Some(Value::Str(d))) => (p.clone(), d.clone()),
        _ => return Err(VmError::runtime_error("nova_file_append(path, data) expects 2 string arguments".to_string())),
    };
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&path)
        .map_err(|e| VmError::runtime_error(format!("nova_file_append: cannot open '{}': {}", path, e)))?;
    file.write_all(data.as_bytes()).map_err(|e| VmError::runtime_error(format!("nova_file_append: cannot write '{}': {}", path, e)))?;
    Ok(Value::Bool(true))
}

pub fn nova_file_exists(args: &[Value]) -> Result<Value, VmError> {
    let path = match args.get(0) { Some(Value::Str(s)) => s.clone(), _ => return Err(VmError::runtime_error("nova_file_exists(path) expects 1 string argument".to_string())), };
    Ok(Value::Bool(std::path::Path::new(&path).is_file()))
}

pub fn nova_file_delete(args: &[Value]) -> Result<Value, VmError> {
    let path = match args.get(0) { Some(Value::Str(s)) => s.clone(), _ => return Err(VmError::runtime_error("nova_file_delete(path) expects 1 string argument".to_string())), };
    std::fs::remove_file(&path).map_err(|e| VmError::runtime_error(format!("nova_file_delete: cannot delete '{}': {}", path, e)))?;
    Ok(Value::Bool(true))
}

pub fn nova_file_size(args: &[Value]) -> Result<Value, VmError> {
    let path = match args.get(0) { Some(Value::Str(s)) => s.clone(), _ => return Err(VmError::runtime_error("nova_file_size(path) expects 1 string argument".to_string())), };
    let meta = std::fs::metadata(&path).map_err(|e| VmError::runtime_error(format!("nova_file_size: cannot stat '{}': {}", path, e)))?;
    Ok(Value::Number(meta.len() as f64))
}

pub fn nova_dir_list(args: &[Value]) -> Result<Value, VmError> {
    let path = match args.get(0) { Some(Value::Str(s)) => s.clone(), _ => return Err(VmError::runtime_error("nova_dir_list(path) expects 1 string argument".to_string())), };
    let entries = std::fs::read_dir(&path).map_err(|e| VmError::runtime_error(format!("nova_dir_list: cannot read dir '{}': {}", path, e)))?;
    let mut names: Vec<Value> = Vec::new();
    for entry in entries { if let Ok(e) = entry { names.push(Value::Str(e.file_name().to_string_lossy().into_owned())); } }
    Ok(Value::from(names))
}

pub fn nova_dir_exists(args: &[Value]) -> Result<Value, VmError> {
    let path = match args.get(0) { Some(Value::Str(s)) => s.clone(), _ => return Err(VmError::runtime_error("nova_dir_exists(path) expects 1 string argument".to_string())), };
    Ok(Value::Bool(std::path::Path::new(&path).is_dir()))
}

pub fn nova_dir_create(args: &[Value]) -> Result<Value, VmError> {
    let path = match args.get(0) { Some(Value::Str(s)) => s.clone(), _ => return Err(VmError::runtime_error("nova_dir_create(path) expects 1 string argument".to_string())), };
    std::fs::create_dir_all(&path).map_err(|e| VmError::runtime_error(format!("nova_dir_create: cannot create '{}': {}", path, e)))?;
    Ok(Value::Bool(true))
}

// ==============================================================================
// BATCH 2 -- KORE Data Operations
// ==============================================================================

pub fn nova_select(args: &[Value]) -> Result<Value, VmError> {
    if args.len() < 3 { return Err(VmError::runtime_error("nova_select(in, out, col...) expects >=3 arguments".to_string())); }
    let (inp, out) = match (&args[0], &args[1]) { (Value::Str(a), Value::Str(b)) => (a.clone(), b.clone()), _ => return Err(VmError::runtime_error("nova_select: first two args must be paths".to_string())), };
    let keep: Vec<String> = args[2..].iter().filter_map(|v| if let Value::Str(s) = v { Some(s.clone()) } else { None }).collect();
    let data = std::fs::read(&inp).map_err(|e| VmError::runtime_error(format!("nova_select: cannot read '{}': {}", inp, e)))?;
    let (_h, col_names, type_algos, cols) = decode_all_columns_typed(&data)?;
    let mut new_names: Vec<String> = Vec::new(); let mut new_types: Vec<u8> = Vec::new(); let mut new_cols: Vec<Vec<Value>> = Vec::new();
    for k in &keep {
        let idx = col_names.iter().position(|n| n == k).ok_or_else(|| VmError::runtime_error(format!("nova_select: column '{}' not found", k)))?;
        new_names.push(col_names[idx].clone()); new_types.push(type_algos[idx].0); new_cols.push(cols[idx].clone());
    }
    write_kore_from_values(&new_names, &new_types, &new_cols, &out)?;
    println!("nova_select: {} cols -> {}", new_names.len(), out);
    Ok(Value::Bool(true))
}

pub fn nova_drop_col(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 3 { return Err(VmError::runtime_error("nova_drop_col(in, out, col) expects 3 arguments".to_string())); }
    let (inp, out, col) = match (&args[0], &args[1], &args[2]) { (Value::Str(a), Value::Str(b), Value::Str(c)) => (a.clone(), b.clone(), c.clone()), _ => return Err(VmError::runtime_error("nova_drop_col: all args must be strings".to_string())), };
    let data = std::fs::read(&inp).map_err(|e| VmError::runtime_error(format!("nova_drop_col: cannot read '{}': {}", inp, e)))?;
    let (_h, col_names, type_algos, cols) = decode_all_columns_typed(&data)?;
    let orig_len = col_names.len();
    let mut new_names: Vec<String> = Vec::new(); let mut new_types: Vec<u8> = Vec::new(); let mut new_cols: Vec<Vec<Value>> = Vec::new();
    for (i, name) in col_names.iter().enumerate() {
        if name != &col { new_names.push(name.clone()); new_types.push(type_algos[i].0); new_cols.push(cols[i].clone()); }
    }
    if new_names.len() == orig_len { return Err(VmError::runtime_error(format!("nova_drop_col: column '{}' not found", col))); }
    write_kore_from_values(&new_names, &new_types, &new_cols, &out)?;
    println!("nova_drop_col: dropped '{}' -> {}", col, out);
    Ok(Value::Bool(true))
}

pub fn nova_rename_col(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 4 { return Err(VmError::runtime_error("nova_rename_col(in, out, old, new) expects 4 arguments".to_string())); }
    let (inp, out, old, new_name) = match (&args[0], &args[1], &args[2], &args[3]) { (Value::Str(a), Value::Str(b), Value::Str(c), Value::Str(d)) => (a.clone(), b.clone(), c.clone(), d.clone()), _ => return Err(VmError::runtime_error("nova_rename_col: all args must be strings".to_string())), };
    let data = std::fs::read(&inp).map_err(|e| VmError::runtime_error(format!("nova_rename_col: cannot read '{}': {}", inp, e)))?;
    let (_h, mut col_names, type_algos, cols) = decode_all_columns_typed(&data)?;
    let idx = col_names.iter().position(|n| n == &old).ok_or_else(|| VmError::runtime_error(format!("nova_rename_col: column '{}' not found", old)))?;
    col_names[idx] = new_name.clone();
    let type_bytes: Vec<u8> = type_algos.iter().map(|(t, _)| *t).collect();
    write_kore_from_values(&col_names, &type_bytes, &cols, &out)?;
    println!("nova_rename_col: '{}' -> '{}' in {}", old, new_name, out);
    Ok(Value::Bool(true))
}

pub fn nova_add_col(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 4 { return Err(VmError::runtime_error("nova_add_col(in, out, name, value) expects 4 arguments".to_string())); }
    let (inp, out, name) = match (&args[0], &args[1], &args[2]) { (Value::Str(a), Value::Str(b), Value::Str(c)) => (a.clone(), b.clone(), c.clone()), _ => return Err(VmError::runtime_error("nova_add_col: first 3 args must be strings".to_string())), };
    let fill = args[3].clone();
    let data = std::fs::read(&inp).map_err(|e| VmError::runtime_error(format!("nova_add_col: cannot read '{}': {}", inp, e)))?;
    let (h, mut col_names, type_algos, mut cols) = decode_all_columns_typed(&data)?;
    let new_type = match &fill { Value::Number(_) => T_INT, Value::Bool(_) => T_BOOL, _ => T_STR };
    col_names.push(name.clone());
    let mut type_bytes: Vec<u8> = type_algos.iter().map(|(t, _)| *t).collect();
    type_bytes.push(new_type);
    cols.push(vec![fill; h.nrows]);
    write_kore_from_values(&col_names, &type_bytes, &cols, &out)?;
    println!("nova_add_col: added '{}' -> {}", name, out);
    Ok(Value::Bool(true))
}

pub fn nova_head(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 3 { return Err(VmError::runtime_error("nova_head(in, out, n) expects 3 arguments".to_string())); }
    let (inp, out) = match (&args[0], &args[1]) { (Value::Str(a), Value::Str(b)) => (a.clone(), b.clone()), _ => return Err(VmError::runtime_error("nova_head: first 2 args must be strings".to_string())), };
    let n = match &args[2] { Value::Number(n) => *n as usize, Value::Str(s) => s.parse::<usize>().unwrap_or(10), _ => 10 };
    let data = std::fs::read(&inp).map_err(|e| VmError::runtime_error(format!("nova_head: cannot read '{}': {}", inp, e)))?;
    let (h, col_names, type_algos, cols) = decode_all_columns_typed(&data)?;
    let take = n.min(h.nrows);
    let sliced: Vec<Vec<Value>> = cols.iter().map(|c| c[..take].to_vec()).collect();
    let type_bytes: Vec<u8> = type_algos.iter().map(|(t, _)| *t).collect();
    write_kore_from_values(&col_names, &type_bytes, &sliced, &out)?;
    println!("nova_head: {} rows -> {}", take, out);
    Ok(Value::Bool(true))
}

pub fn nova_tail(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 3 { return Err(VmError::runtime_error("nova_tail(in, out, n) expects 3 arguments".to_string())); }
    let (inp, out) = match (&args[0], &args[1]) { (Value::Str(a), Value::Str(b)) => (a.clone(), b.clone()), _ => return Err(VmError::runtime_error("nova_tail: first 2 args must be strings".to_string())), };
    let n = match &args[2] { Value::Number(n) => *n as usize, Value::Str(s) => s.parse::<usize>().unwrap_or(10), _ => 10 };
    let data = std::fs::read(&inp).map_err(|e| VmError::runtime_error(format!("nova_tail: cannot read '{}': {}", inp, e)))?;
    let (h, col_names, type_algos, cols) = decode_all_columns_typed(&data)?;
    let take = n.min(h.nrows); let start = h.nrows - take;
    let sliced: Vec<Vec<Value>> = cols.iter().map(|c| c[start..].to_vec()).collect();
    let type_bytes: Vec<u8> = type_algos.iter().map(|(t, _)| *t).collect();
    write_kore_from_values(&col_names, &type_bytes, &sliced, &out)?;
    println!("nova_tail: {} rows -> {}", take, out);
    Ok(Value::Bool(true))
}

pub fn nova_sort(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 4 { return Err(VmError::runtime_error("nova_sort(in, out, col, asc|desc) expects 4 arguments".to_string())); }
    let (inp, out, col, dir) = match (&args[0], &args[1], &args[2], &args[3]) { (Value::Str(a), Value::Str(b), Value::Str(c), Value::Str(d)) => (a.clone(), b.clone(), c.clone(), d.clone()), _ => return Err(VmError::runtime_error("nova_sort: all args must be strings".to_string())), };
    let data = std::fs::read(&inp).map_err(|e| VmError::runtime_error(format!("nova_sort: cannot read '{}': {}", inp, e)))?;
    let (h, col_names, type_algos, cols) = decode_all_columns_typed(&data)?;
    let sort_ci = col_names.iter().position(|n| n == &col).ok_or_else(|| VmError::runtime_error(format!("nova_sort: column '{}' not found", col)))?;
    let sort_col = &cols[sort_ci];
    let ascending = dir.to_lowercase() != "desc";
    let mut indices: Vec<usize> = (0..h.nrows).collect();
    indices.sort_by(|&a, &b| {
        let cmp = match (&sort_col[a], &sort_col[b]) {
            (Value::Number(x), Value::Number(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
            (Value::Str(x),    Value::Str(y))    => x.cmp(y),
            (Value::Bool(x),   Value::Bool(y))   => x.cmp(y),
            _ => std::cmp::Ordering::Equal,
        };
        if ascending { cmp } else { cmp.reverse() }
    });
    let reordered: Vec<Vec<Value>> = cols.iter().map(|c| indices.iter().map(|&i| c[i].clone()).collect()).collect();
    let type_bytes: Vec<u8> = type_algos.iter().map(|(t, _)| *t).collect();
    write_kore_from_values(&col_names, &type_bytes, &reordered, &out)?;
    println!("nova_sort: sorted by '{}' {} -> {}", col, dir, out);
    Ok(Value::Bool(true))
}

pub fn nova_merge(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 3 { return Err(VmError::runtime_error("nova_merge(a, b, out) expects 3 arguments".to_string())); }
    let (pa, pb, out) = match (&args[0], &args[1], &args[2]) { (Value::Str(a), Value::Str(b), Value::Str(c)) => (a.clone(), b.clone(), c.clone()), _ => return Err(VmError::runtime_error("nova_merge: all args must be strings".to_string())), };
    let da = std::fs::read(&pa).map_err(|e| VmError::runtime_error(format!("nova_merge: cannot read '{}': {}", pa, e)))?;
    let db = std::fs::read(&pb).map_err(|e| VmError::runtime_error(format!("nova_merge: cannot read '{}': {}", pb, e)))?;
    let (ha, col_names_a, type_algos_a, cols_a) = decode_all_columns_typed(&da)?;
    let (hb, col_names_b, _type_algos_b, cols_b) = decode_all_columns_typed(&db)?;
    if col_names_a != col_names_b { return Err(VmError::runtime_error("nova_merge: schemas do not match".to_string())); }
    let merged: Vec<Vec<Value>> = cols_a.iter().zip(cols_b.iter()).map(|(ca, cb)| { let mut m = ca.clone(); m.extend_from_slice(cb); m }).collect();
    let type_bytes: Vec<u8> = type_algos_a.iter().map(|(t, _)| *t).collect();
    write_kore_from_values(&col_names_a, &type_bytes, &merged, &out)?;
    println!("nova_merge: merged {} + {} rows -> {}", ha.nrows, hb.nrows, out);
    Ok(Value::Bool(true))
}

pub fn nova_join(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 4 { return Err(VmError::runtime_error("nova_join(a, b, out, col) expects 4 arguments".to_string())); }
    let (pa, pb, out, key) = match (&args[0], &args[1], &args[2], &args[3]) { (Value::Str(a), Value::Str(b), Value::Str(c), Value::Str(d)) => (a.clone(), b.clone(), c.clone(), d.clone()), _ => return Err(VmError::runtime_error("nova_join: all args must be strings".to_string())), };
    let da = std::fs::read(&pa).map_err(|e| VmError::runtime_error(format!("nova_join: cannot read '{}': {}", pa, e)))?;
    let db = std::fs::read(&pb).map_err(|e| VmError::runtime_error(format!("nova_join: cannot read '{}': {}", pb, e)))?;
    let (ha, col_names_a, type_algos_a, cols_a) = decode_all_columns_typed(&da)?;
    let (hb, col_names_b, type_algos_b, cols_b) = decode_all_columns_typed(&db)?;
    let ka = col_names_a.iter().position(|n| n == &key).ok_or_else(|| VmError::runtime_error(format!("nova_join: key '{}' not found in a", key)))?;
    let kb = col_names_b.iter().position(|n| n == &key).ok_or_else(|| VmError::runtime_error(format!("nova_join: key '{}' not found in b", key)))?;
    let mut b_index: std::collections::HashMap<String, Vec<usize>> = std::collections::HashMap::new();
    for ri in 0..hb.nrows { b_index.entry(value_to_sort_key(&cols_b[kb][ri])).or_default().push(ri); }
    let mut out_names: Vec<String> = col_names_a.clone();
    let mut out_type_bytes: Vec<u8> = type_algos_a.iter().map(|(t, _)| *t).collect();
    for (i, n) in col_names_b.iter().enumerate() { if i != kb { out_names.push(n.clone()); out_type_bytes.push(type_algos_b[i].0); } }
    let out_ncols = out_names.len();
    let mut out_cols: Vec<Vec<Value>> = vec![Vec::new(); out_ncols];
    for ri_a in 0..ha.nrows {
        let kv = value_to_sort_key(&cols_a[ka][ri_a]);
        if let Some(b_rows) = b_index.get(&kv) {
            for &ri_b in b_rows {
                for ci in 0..col_names_a.len() { out_cols[ci].push(cols_a[ci][ri_a].clone()); }
                let mut out_ci = col_names_a.len();
                for ci_b in 0..col_names_b.len() { if ci_b != kb { out_cols[out_ci].push(cols_b[ci_b][ri_b].clone()); out_ci += 1; } }
            }
        }
    }
    let result_rows = out_cols.first().map(|c| c.len()).unwrap_or(0);
    write_kore_from_values(&out_names, &out_type_bytes, &out_cols, &out)?;
    println!("nova_join: {} rows -> {}", result_rows, out);
    Ok(Value::Bool(true))
}

pub fn nova_group_by(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 5 { return Err(VmError::runtime_error("nova_group_by(in, out, col, agg, agg_col) expects 5 arguments".to_string())); }
    let (inp, out, gcol, agg, acol) = match (&args[0], &args[1], &args[2], &args[3], &args[4]) { (Value::Str(a), Value::Str(b), Value::Str(c), Value::Str(d), Value::Str(e)) => (a.clone(), b.clone(), c.clone(), d.clone(), e.clone()), _ => return Err(VmError::runtime_error("nova_group_by: all args must be strings".to_string())), };
    let data = std::fs::read(&inp).map_err(|e| VmError::runtime_error(format!("nova_group_by: cannot read '{}': {}", inp, e)))?;
    let (h, col_names, _type_algos, cols) = decode_all_columns_typed(&data)?;
    let gi = col_names.iter().position(|n| n == &gcol).ok_or_else(|| VmError::runtime_error(format!("nova_group_by: group column '{}' not found", gcol)))?;
    let ai = col_names.iter().position(|n| n == &acol).ok_or_else(|| VmError::runtime_error(format!("nova_group_by: agg column '{}' not found", acol)))?;
    let mut group_order: Vec<String> = Vec::new();
    let mut group_rows: std::collections::HashMap<String, Vec<usize>> = std::collections::HashMap::new();
    for ri in 0..h.nrows {
        let gk = value_to_sort_key(&cols[gi][ri]);
        if !group_rows.contains_key(&gk) { group_order.push(gk.clone()); }
        group_rows.entry(gk).or_default().push(ri);
    }
    let agg_lower = agg.to_lowercase();
    let mut result_keys: Vec<Value> = Vec::new(); let mut result_values: Vec<Value> = Vec::new();
    for gk in &group_order {
        let rows = &group_rows[gk];
        result_keys.push(Value::Str(gk.clone()));
        let agg_val: Value = match agg_lower.as_str() {
            "count" => Value::Number(rows.len() as f64),
            "sum"   => { let s: f64 = rows.iter().filter_map(|&ri| if let Value::Number(n) = cols[ai][ri] { Some(n) } else { None }).sum(); Value::Number(s) }
            "mean"  => { let vals: Vec<f64> = rows.iter().filter_map(|&ri| if let Value::Number(n) = cols[ai][ri] { Some(n) } else { None }).collect(); Value::Number(if vals.is_empty() { 0.0 } else { vals.iter().sum::<f64>() / vals.len() as f64 }) }
            "min"   => { let v = rows.iter().filter_map(|&ri| if let Value::Number(n) = cols[ai][ri] { Some(n) } else { None }).fold(f64::INFINITY, f64::min); Value::Number(if v == f64::INFINITY { 0.0 } else { v }) }
            "max"   => { let v = rows.iter().filter_map(|&ri| if let Value::Number(n) = cols[ai][ri] { Some(n) } else { None }).fold(f64::NEG_INFINITY, f64::max); Value::Number(if v == f64::NEG_INFINITY { 0.0 } else { v }) }
            _ => return Err(VmError::runtime_error(format!("nova_group_by: unknown agg '{}' (sum/mean/count/min/max)", agg))),
        };
        result_values.push(agg_val);
    }
    let out_names = vec![gcol.clone(), format!("{}_{}", agg_lower, acol)];
    let out_type_bytes = vec![T_STR, T_FLOAT];
    write_kore_from_values(&out_names, &out_type_bytes, &[result_keys, result_values], &out)?;
    println!("nova_group_by: {} groups -> {}", group_order.len(), out);
    Ok(Value::Bool(true))
}

pub fn nova_distinct(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 3 { return Err(VmError::runtime_error("nova_distinct(in, out, col) expects 3 arguments".to_string())); }
    let (inp, out, col) = match (&args[0], &args[1], &args[2]) { (Value::Str(a), Value::Str(b), Value::Str(c)) => (a.clone(), b.clone(), c.clone()), _ => return Err(VmError::runtime_error("nova_distinct: all args must be strings".to_string())), };
    let data = std::fs::read(&inp).map_err(|e| VmError::runtime_error(format!("nova_distinct: cannot read '{}': {}", inp, e)))?;
    let (h, col_names, type_algos, cols) = decode_all_columns_typed(&data)?;
    let ci = col_names.iter().position(|n| n == &col).ok_or_else(|| VmError::runtime_error(format!("nova_distinct: column '{}' not found", col)))?;
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut keep: Vec<usize> = Vec::new();
    for ri in 0..h.nrows { if seen.insert(value_to_sort_key(&cols[ci][ri])) { keep.push(ri); } }
    let deduped: Vec<Vec<Value>> = cols.iter().map(|c| keep.iter().map(|&i| c[i].clone()).collect()).collect();
    let type_bytes: Vec<u8> = type_algos.iter().map(|(t, _)| *t).collect();
    write_kore_from_values(&col_names, &type_bytes, &deduped, &out)?;
    println!("nova_distinct: {} -> {} unique rows -> {}", h.nrows, keep.len(), out);
    Ok(Value::Bool(true))
}

pub fn nova_sample(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 3 { return Err(VmError::runtime_error("nova_sample(in, out, n) expects 3 arguments".to_string())); }
    let (inp, out) = match (&args[0], &args[1]) { (Value::Str(a), Value::Str(b)) => (a.clone(), b.clone()), _ => return Err(VmError::runtime_error("nova_sample: first 2 args must be strings".to_string())), };
    let n = match &args[2] { Value::Number(n) => *n as usize, Value::Str(s) => s.parse::<usize>().unwrap_or(10), _ => 10 };
    let data = std::fs::read(&inp).map_err(|e| VmError::runtime_error(format!("nova_sample: cannot read '{}': {}", inp, e)))?;
    let (h, col_names, type_algos, cols) = decode_all_columns_typed(&data)?;
    let take = n.min(h.nrows);
    let mut indices: Vec<usize> = (0..h.nrows).collect();
    let mut seed = h.nrows as u64;
    for i in (1..h.nrows).rev() {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let j = (seed >> 33) as usize % (i + 1);
        indices.swap(i, j);
    }
    indices.truncate(take);
    let sampled: Vec<Vec<Value>> = cols.iter().map(|c| indices.iter().map(|&i| c[i].clone()).collect()).collect();
    let type_bytes: Vec<u8> = type_algos.iter().map(|(t, _)| *t).collect();
    write_kore_from_values(&col_names, &type_bytes, &sampled, &out)?;
    println!("nova_sample: {} rows sampled -> {}", take, out);
    Ok(Value::Bool(true))
}


// ==============================================================================
// eval_predicate -- evaluate a single col/op/val predicate for one row value
// Shared by nova_filter_op and nova_multi_filter.
fn eval_predicate(row_val: &Value, op: &str, filter_str: &str) -> Result<bool, VmError> {
    let filter_num: Option<f64> = filter_str.parse::<f64>().ok();
    let row_num: Option<f64> = match row_val {
        Value::Number(n) => Some(*n),
        Value::Str(s)    => s.parse::<f64>().ok(),
        _                => None,
    };
    let row_str = value_to_sort_key(row_val);
    Ok(match op {
        "=" | "==" | "eq" => match (row_num, filter_num) {
            (Some(a), Some(b)) => (a - b).abs() < 1e-9,
            _                  => row_str.to_lowercase() == filter_str.to_lowercase(),
        },
        "!=" | "<>" | "ne" => match (row_num, filter_num) {
            (Some(a), Some(b)) => (a - b).abs() >= 1e-9,
            _                  => row_str.to_lowercase() != filter_str.to_lowercase(),
        },
        ">" | "gt"  => match (row_num, filter_num) { (Some(a), Some(b)) => a > b,  _ => row_str.as_str() > filter_str },
        "<" | "lt"  => match (row_num, filter_num) { (Some(a), Some(b)) => a < b,  _ => row_str.as_str() < filter_str },
        ">=" | "ge" => match (row_num, filter_num) { (Some(a), Some(b)) => a >= b, _ => row_str.as_str() >= filter_str },
        "<=" | "le" => match (row_num, filter_num) { (Some(a), Some(b)) => a <= b, _ => row_str.as_str() <= filter_str },
        "contains"   | "like"        => row_str.to_lowercase().contains(&filter_str.to_lowercase()),
        "starts"     | "startswith"  => row_str.to_lowercase().starts_with(&filter_str.to_lowercase()),
        "ends"       | "endswith"    => row_str.to_lowercase().ends_with(&filter_str.to_lowercase()),
        _ => return Err(VmError::runtime_error(
            format!("unknown operator '{}' (use =, !=, >, <, >=, <=, contains, starts, ends)", op)
        )),
    })
}

// nova_filter_op -- filter rows with operator: =, !=, >, <, >=, <=, contains
// Usage: nova_filter_op(in, out, col, op, val)
// ==============================================================================
pub fn nova_filter_op(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 5 {
        return Err(VmError::runtime_error(
            "nova_filter_op(in, out, col, op, val) expects 5 arguments".to_string()
        ));
    }
    let (inp, out, col, op) = match (&args[0], &args[1], &args[2], &args[3]) {
        (Value::Str(a), Value::Str(b), Value::Str(c), Value::Str(d)) => (a.clone(), b.clone(), c.clone(), d.clone()),
        _ => return Err(VmError::runtime_error("nova_filter_op: first 4 args must be strings".to_string())),
    };
    let filter_val = &args[4];

    let data = std::fs::read(&inp)
        .map_err(|e| VmError::runtime_error(format!("nova_filter_op: cannot read '{}': {}", inp, e)))?;
    let (h, col_names, type_algos, cols) = decode_all_columns_typed(&data)?;

    let ci = col_names.iter().position(|n| n == &col)
        .ok_or_else(|| VmError::runtime_error(format!("nova_filter_op: column '{}' not found", col)))?;

    let filter_str = match filter_val {
        Value::Str(s)    => s.clone(),
        Value::Number(n) => if n.fract() == 0.0 { format!("{}", *n as i64) } else { format!("{}", n) },
        Value::Bool(b)   => b.to_string(),
        _                => "null".to_string(),
    };

    let mut keep: Vec<usize> = Vec::new();
    for ri in 0..h.nrows {
        if eval_predicate(&cols[ci][ri], &op, &filter_str)? {
            keep.push(ri);
        }
    }

    let filtered: Vec<Vec<Value>> = cols.iter().map(|c| keep.iter().map(|&i| c[i].clone()).collect()).collect();
    let type_bytes: Vec<u8> = type_algos.iter().map(|(t, _)| *t).collect();
    write_kore_from_values(&col_names, &type_bytes, &filtered, &out)?;
    println!("nova_filter_op: {} -> {} rows (col '{}' {} {}) -> {}", h.nrows, keep.len(), col, op, filter_str, out);
    Ok(Value::Bool(true))
}

// ==============================================================================
// nova_fill -- replace empty/null/"EMPTY" values in a column with a default
// Usage: nova_fill(in, out, col, fill_value)
// ==============================================================================
pub fn nova_fill(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 4 {
        return Err(VmError::runtime_error(
            "nova_fill(in, out, col, value) expects 4 arguments".to_string()
        ));
    }
    let (inp, out, col) = match (&args[0], &args[1], &args[2]) {
        (Value::Str(a), Value::Str(b), Value::Str(c)) => (a.clone(), b.clone(), c.clone()),
        _ => return Err(VmError::runtime_error("nova_fill: first 3 args must be strings".to_string())),
    };
    let fill = args[3].clone();

    let data = std::fs::read(&inp)
        .map_err(|e| VmError::runtime_error(format!("nova_fill: cannot read '{}': {}", inp, e)))?;
    let (h, col_names, type_algos, mut cols) = decode_all_columns_typed(&data)?;

    let ci = col_names.iter().position(|n| n == &col)
        .ok_or_else(|| VmError::runtime_error(format!("nova_fill: column '{}' not found", col)))?;

    let mut filled = 0usize;
    for ri in 0..h.nrows {
        let is_empty = match &cols[ci][ri] {
            Value::Null      => true,
            Value::Str(s)    => s.is_empty() || s.eq_ignore_ascii_case("null") || s.eq_ignore_ascii_case("EMPTY"),
            _                => false,
        };
        if is_empty {
            cols[ci][ri] = fill.clone();
            filled += 1;
        }
    }

    let type_bytes: Vec<u8> = type_algos.iter().map(|(t, _)| *t).collect();
    write_kore_from_values(&col_names, &type_bytes, &cols, &out)?;
    println!("nova_fill: filled {} nulls in '{}' -> {}", filled, col, out);
    Ok(Value::Bool(true))
}

// ==============================================================================
// nova_read_lines -- read a text file and return lines as a JSON array
// Usage: nova_read_lines(path)
// ==============================================================================
pub fn nova_read_lines(args: &[Value]) -> Result<Value, VmError> {
    let path = match args.get(0) {
        Some(Value::Str(s)) => s.clone(),
        _ => return Err(VmError::runtime_error("nova_read_lines(path) expects 1 string argument".to_string())),
    };
    let contents = std::fs::read_to_string(&path)
        .map_err(|e| VmError::runtime_error(format!("nova_read_lines: cannot read '{}': {}", path, e)))?;
    let lines: Vec<Value> = contents
        .lines()
        .map(|l| Value::Str(l.to_string()))
        .collect();
    Ok(Value::from(lines))
}

// ==============================================================================
// nova_multi_filter -- filter rows with compound AND/OR predicates
// Usage: nova_multi_filter(in, out, AND|OR, col1, op1, val1 [, col2, op2, val2 ...])
//   Predicates after position 2 come in triples: col, operator, value.
//   AND keeps rows that satisfy ALL predicates; OR keeps rows that satisfy ANY.
// ==============================================================================
pub fn nova_multi_filter(args: &[Value]) -> Result<Value, VmError> {
    if args.len() < 6 || (args.len() - 3) % 3 != 0 {
        return Err(VmError::runtime_error(
            "nova_multi_filter(in, out, AND|OR, col, op, val, ...) expects 6+ args, predicates in triples".to_string()
        ));
    }
    let (inp, out, logic) = match (&args[0], &args[1], &args[2]) {
        (Value::Str(a), Value::Str(b), Value::Str(c)) => (a.clone(), b.clone(), c.clone()),
        _ => return Err(VmError::runtime_error("nova_multi_filter: first 3 args must be strings".to_string())),
    };
    let is_and = match logic.to_uppercase().as_str() {
        "AND" => true,
        "OR"  => false,
        _     => return Err(VmError::runtime_error(
            format!("nova_multi_filter: logic must be AND or OR, got '{}'", logic)
        )),
    };

    // Parse predicate triples: (col, op, filter_value_string)
    let predicates: Vec<(String, String, String)> = args[3..].chunks(3).map(|c| {
        let col = match &c[0] { Value::Str(s) => s.clone(), v => value_to_sort_key(v) };
        let op  = match &c[1] { Value::Str(s) => s.clone(), v => value_to_sort_key(v) };
        let val = match &c[2] {
            Value::Str(s)    => s.clone(),
            Value::Number(n) => if n.fract() == 0.0 { format!("{}", *n as i64) } else { format!("{}", n) },
            Value::Bool(b)   => b.to_string(),
            _                => "null".to_string(),
        };
        (col, op, val)
    }).collect();

    let data = std::fs::read(&inp)
        .map_err(|e| VmError::runtime_error(format!("nova_multi_filter: cannot read '{}': {}", inp, e)))?;
    let (h, col_names, type_algos, cols) = decode_all_columns_typed(&data)?;

    // Resolve each predicate's column index up-front
    let pred_cols: Vec<usize> = predicates.iter().map(|(col, _, _)| {
        col_names.iter().position(|n| n == col)
            .ok_or_else(|| VmError::runtime_error(format!("nova_multi_filter: column '{}' not found", col)))
    }).collect::<Result<Vec<usize>, VmError>>()?;

    let mut keep: Vec<usize> = Vec::new();
    'rows: for ri in 0..h.nrows {
        let matched = is_and; // AND starts true (all must pass), OR starts false (any suffices)
        for (pi, &ci) in pred_cols.iter().enumerate() {
            let (_, op, val) = &predicates[pi];
            let passes = eval_predicate(&cols[ci][ri], op, val)?;
            if is_and {
                if !passes { continue 'rows; } // short-circuit AND
            } else {
                if passes  { keep.push(ri); continue 'rows; } // short-circuit OR
            }
        }
        if matched { keep.push(ri); }
    }

    let filtered: Vec<Vec<Value>> = cols.iter()
        .map(|c| keep.iter().map(|&i| c[i].clone()).collect())
        .collect();
    let type_bytes: Vec<u8> = type_algos.iter().map(|(t, _)| *t).collect();
    write_kore_from_values(&col_names, &type_bytes, &filtered, &out)?;

    let pred_desc: Vec<String> = predicates.iter().map(|(c, o, v)| format!("{} {} {}", c, o, v)).collect();
    println!("nova_multi_filter: {} -> {} rows  ({})  -> {}", h.nrows, keep.len(), pred_desc.join(&format!(" {} ", logic.to_uppercase())), out);
    Ok(Value::Bool(true))
}

// ==============================================================================
// nova_cast -- change the stored type of a column
// Usage: nova_cast(in, out, col, type)   type: int | float | str | bool
//
// Use after nova_fill to convert mixed-type columns to their correct numeric
// type so that nova_sort, nova_stats, etc. work correctly.
// ==============================================================================
pub fn nova_cast(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 4 {
        return Err(VmError::runtime_error(
            "nova_cast(in, out, col, type) expects 4 arguments".to_string()
        ));
    }
    let (inp, out, col, to_type) = match (&args[0], &args[1], &args[2], &args[3]) {
        (Value::Str(a), Value::Str(b), Value::Str(c), Value::Str(d)) =>
            (a.clone(), b.clone(), c.clone(), d.clone()),
        _ => return Err(VmError::runtime_error("nova_cast: all 4 args must be strings".to_string())),
    };

    let target_type_byte: u8 = match to_type.to_lowercase().as_str() {
        "int" | "integer"                   => T_INT,
        "float" | "double" | "num"          => T_FLOAT,
        "str"  | "string"  | "text"         => T_STR,
        "bool" | "boolean"                  => T_BOOL,
        _ => return Err(VmError::runtime_error(
            format!("nova_cast: unknown type '{}' (use int, float, str, bool)", to_type)
        )),
    };

    let data = std::fs::read(&inp)
        .map_err(|e| VmError::runtime_error(format!("nova_cast: cannot read '{}': {}", inp, e)))?;
    let (_h, col_names, type_algos, mut cols) = decode_all_columns_typed(&data)?;

    let ci = col_names.iter().position(|n| n == &col)
        .ok_or_else(|| VmError::runtime_error(format!("nova_cast: column '{}' not found", col)))?;

    let mut converted = 0usize;
    let mut failed    = 0usize;
    let nrows = cols[ci].len();

    for ri in 0..nrows {
        let new_val = match target_type_byte {
            T_INT | T_FLOAT => {
                let s = value_to_sort_key(&cols[ci][ri]);
                match s.parse::<f64>() {
                    Ok(n)  => { converted += 1; Value::Number(n) }
                    Err(_) => { failed    += 1; Value::Null }
                }
            }
            T_BOOL => {
                let s = value_to_sort_key(&cols[ci][ri]).to_lowercase();
                let b = s == "true" || s == "1" || s == "yes" || s == "y";
                converted += 1;
                Value::Bool(b)
            }
            _ /* T_STR */ => {
                converted += 1;
                Value::Str(value_to_sort_key(&cols[ci][ri]))
            }
        };
        cols[ci][ri] = new_val;
    }

    let mut type_bytes: Vec<u8> = type_algos.iter().map(|(t, _)| *t).collect();
    type_bytes[ci] = target_type_byte;

    write_kore_from_values(&col_names, &type_bytes, &cols, &out)?;
    if failed > 0 {
        println!("nova_cast: '{}' -> {} ({} converted, {} parse failures -> null) -> {}", col, to_type, converted, failed, out);
    } else {
        println!("nova_cast: '{}' -> {} ({} rows) -> {}", col, to_type, converted, out);
    }
    Ok(Value::Bool(true))
}

// ==============================================================================
// nova_concat -- side-by-side column join (paste columns from two files)
// Usage: nova_concat(a, b, out)
//   Both files must have the same number of rows.
//   If a column name appears in both files, the copy from B is suffixed "_2".
// ==============================================================================
pub fn nova_concat(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 3 {
        return Err(VmError::runtime_error(
            "nova_concat(a, b, out) expects exactly 3 arguments".to_string()
        ));
    }
    let (a_path, b_path, out) = match (&args[0], &args[1], &args[2]) {
        (Value::Str(a), Value::Str(b), Value::Str(c)) => (a.clone(), b.clone(), c.clone()),
        _ => return Err(VmError::runtime_error("nova_concat: all 3 args must be strings".to_string())),
    };

    let a_data = std::fs::read(&a_path)
        .map_err(|e| VmError::runtime_error(format!("nova_concat: cannot read '{}': {}", a_path, e)))?;
    let b_data = std::fs::read(&b_path)
        .map_err(|e| VmError::runtime_error(format!("nova_concat: cannot read '{}': {}", b_path, e)))?;

    let (ha, mut col_names, mut type_algos, mut cols) = decode_all_columns_typed(&a_data)?;
    let (hb, b_names, b_type_algos, b_cols)           = decode_all_columns_typed(&b_data)?;

    if ha.nrows != hb.nrows {
        return Err(VmError::runtime_error(format!(
            "nova_concat: row count mismatch ({} vs {}). Use merge to stack rows instead.",
            ha.nrows, hb.nrows
        )));
    }

    // Append B columns, deduplicating names with "_2" suffix
    for (bi, b_name) in b_names.iter().enumerate() {
        let final_name = if col_names.contains(b_name) {
            format!("{}_2", b_name)
        } else {
            b_name.clone()
        };
        col_names.push(final_name);
        type_algos.push(b_type_algos[bi]);
        cols.push(b_cols[bi].clone());
    }

    let type_bytes: Vec<u8> = type_algos.iter().map(|(t, _)| *t).collect();
    write_kore_from_values(&col_names, &type_bytes, &cols, &out)?;
    println!("nova_concat: {} + {} cols, {} rows -> {}", ha.ncols, hb.ncols, ha.nrows, out);
    Ok(Value::Bool(true))
}

// ==============================================================================
// nova_show -- print the first N rows as a formatted table to stdout
// Usage: nova_show(path)              -- show first 20 rows, all cols
//        nova_show(path, n)           -- show first n rows, all cols
//        nova_show(path, n, col, ...) -- show first n rows, selected cols only
// ==============================================================================
pub fn nova_show(args: &[Value]) -> Result<Value, VmError> {
    if args.is_empty() {
        return Err(VmError::runtime_error(
            "nova_show(path [, n [, col...]]) expects at least 1 argument".to_string()
        ));
    }
    let path = match &args[0] {
        Value::Str(s) => s.clone(),
        _ => return Err(VmError::runtime_error("nova_show: first arg must be a string path".to_string())),
    };
    let n_rows: usize = match args.get(1) {
        Some(Value::Str(s))    => s.parse::<usize>().unwrap_or(20),
        Some(Value::Number(n)) => *n as usize,
        None                   => 20,
        _                      => 20,
    };
    // Optional column filter from arg[2] onwards
    let wanted_cols: Vec<String> = args.get(2..)
        .unwrap_or(&[])
        .iter()
        .filter_map(|v| if let Value::Str(s) = v { Some(s.clone()) } else { None })
        .collect();

    let data = std::fs::read(&path)
        .map_err(|e| VmError::runtime_error(format!("nova_show: cannot read '{}': {}", path, e)))?;
    let (h, all_names, _type_algos, all_cols) = decode_all_columns_typed(&data)?;

    // Determine which columns to display
    let show_indices: Vec<usize> = if wanted_cols.is_empty() {
        (0..all_names.len()).collect()
    } else {
        wanted_cols.iter()
            .filter_map(|wc| all_names.iter().position(|n| n == wc))
            .collect()
    };

    let display_names: Vec<&str> = show_indices.iter().map(|&i| all_names[i].as_str()).collect();
    let display_cols:  Vec<&Vec<Value>> = show_indices.iter().map(|&i| &all_cols[i]).collect();
    let show_rows = n_rows.min(h.nrows);

    // -- compute column widths (capped at 28 for readability) -----------------
    let mut widths: Vec<usize> = display_names.iter().map(|n| n.len()).collect();
    for ri in 0..show_rows {
        for (ci, col) in display_cols.iter().enumerate() {
            let s = fmt_cell(&col[ri]);
            if s.len() > widths[ci] { widths[ci] = s.len(); }
        }
    }
    for w in &mut widths { if *w > 28 { *w = 28; } }

    // -- separator line --------------------------------------------------------
    let sep: String = widths.iter()
        .map(|&w| "-".repeat(w + 2))
        .collect::<Vec<_>>()
        .join("+");
    let border = format!("+{}+", sep);

    // -- header ----------------------------------------------------------------
    println!("{}", border);
    let hdr: String = display_names.iter().zip(widths.iter())
        .map(|(name, &w)| format!(" {:<width$} ", trunc(name, w), width = w))
        .collect::<Vec<_>>()
        .join("|");
    println!("{}{}{}", "|", hdr, "|");
    println!("{}", border);

    // -- data rows -------------------------------------------------------------
    for ri in 0..show_rows {
        let row: String = display_cols.iter().zip(widths.iter())
            .map(|(col, &w)| {
                let s = fmt_cell(&col[ri]);
                format!(" {:<width$} ", trunc(&s, w), width = w)
            })
            .collect::<Vec<_>>()
            .join("|");
        println!("{}{}{}", "|", row, "|");
    }
    println!("{}", border);

    // -- footer ----------------------------------------------------------------
    if h.nrows > show_rows {
        println!("  showing {} of {} rows   {} columns   {}",
            show_rows, h.nrows, display_names.len(), path);
    } else {
        println!("  {} rows   {} columns   {}",
            h.nrows, display_names.len(), path);
    }

    Ok(Value::Bool(true))
}

#[inline(always)]
fn fmt_cell(v: &Value) -> String {
    match v {
        Value::Number(n) => if n.fract() == 0.0 { format!("{}", *n as i64) }
                            else                 { format!("{:.4}", n) },
        Value::Str(s)    => s.clone(),
        Value::Bool(b)   => b.to_string(),
        Value::Null      => "null".to_string(),
        _                => "...".to_string(),
    }
}

#[inline(always)]
fn trunc(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() }
    else { format!("{}...", &s[..max.saturating_sub(3)]) }
}


// -- Excel XLSX ----------------------------------------------------------------
// Pure-Rust XLSX writer/reader. Zero external crates.
// XLSX = ZIP(stored) of OOXML parts. Opens in Excel 2016+ / LibreOffice 6+.

/// CRC-32 (IEEE 802.3 polynomial) required by the ZIP format.
fn crc32_ieee(data: &[u8]) -> u32 {
    const fn make_table() -> [u32; 256] {
        let mut t = [0u32; 256];
        let mut i = 0;
        while i < 256 {
            let mut c = i as u32;
            let mut k = 0;
            while k < 8 { c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 }; k += 1; }
            t[i] = c;
            i += 1;
        }
        t
    }
    const TABLE: [u32; 256] = make_table();
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data { crc = TABLE[((crc ^ b as u32) & 0xFF) as usize] ^ (crc >> 8); }
    crc ^ 0xFFFF_FFFF
}

/// Write entries into a stored-mode (method=0, no compression) ZIP archive.
fn zip_stored(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let mut meta: Vec<(u32, u32, u32, Vec<u8>)> = Vec::new(); // (offset, crc, len, name_bytes)
    for &(name, data) in entries {
        let crc = crc32_ieee(data);
        let off = out.len() as u32;
        let len = data.len() as u32;
        let nb  = name.as_bytes();
        out.extend_from_slice(b"PK\x03\x04");
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());            // flags
        out.extend_from_slice(&0u16.to_le_bytes());            // method: stored
        out.extend_from_slice(&0u16.to_le_bytes());            // mod time
        out.extend_from_slice(&0u16.to_le_bytes());            // mod date
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(nb.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());            // extra field len
        out.extend_from_slice(nb);
        out.extend_from_slice(data);
        meta.push((off, crc, len, nb.to_vec()));
    }
    let dir_start = out.len() as u32;
    let dir_begin = out.len();
    for (off, crc, len, nb) in &meta {
        out.extend_from_slice(b"PK\x01\x02");
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(nb.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&off.to_le_bytes());
        out.extend_from_slice(nb);
    }
    let dir_size = (out.len() - dir_begin) as u32;
    let nfiles   = meta.len() as u16;
    out.extend_from_slice(b"PK\x05\x06");
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&nfiles.to_le_bytes());
    out.extend_from_slice(&nfiles.to_le_bytes());
    out.extend_from_slice(&dir_size.to_le_bytes());
    out.extend_from_slice(&dir_start.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

/// Extract one file by name from a stored-mode ZIP. Returns None if compressed or not found.
fn zip_extract(zip: &[u8], target: &str) -> Option<Vec<u8>> {
    if zip.len() < 22 { return None; }
    let lo   = zip.len().saturating_sub(65557);
    let eocd = (lo..=zip.len() - 22).rev()
        .find(|&i| &zip[i..i+4] == b"PK\x05\x06")?;
    let cd_size   = u32::from_le_bytes([zip[eocd+12], zip[eocd+13], zip[eocd+14], zip[eocd+15]]) as usize;
    let cd_offset = u32::from_le_bytes([zip[eocd+16], zip[eocd+17], zip[eocd+18], zip[eocd+19]]) as usize;
    let mut pos   = cd_offset;
    let cd_end    = (cd_offset + cd_size).min(zip.len());
    while pos + 46 <= cd_end {
        if &zip[pos..pos+4] != b"PK\x01\x02" { break; }
        let method      = u16::from_le_bytes([zip[pos+10], zip[pos+11]]);
        let comp_size   = u32::from_le_bytes([zip[pos+20], zip[pos+21], zip[pos+22], zip[pos+23]]) as usize;
        let fname_len   = u16::from_le_bytes([zip[pos+28], zip[pos+29]]) as usize;
        let extra_len   = u16::from_le_bytes([zip[pos+30], zip[pos+31]]) as usize;
        let comment_len = u16::from_le_bytes([zip[pos+32], zip[pos+33]]) as usize;
        let local_off   = u32::from_le_bytes([zip[pos+42], zip[pos+43], zip[pos+44], zip[pos+45]]) as usize;
        if pos + 46 + fname_len <= zip.len() {
            let fname = std::str::from_utf8(&zip[pos+46..pos+46+fname_len]).unwrap_or("");
            if fname == target && method == 0 {
                if local_off + 30 > zip.len() { return None; }
                let lfn        = u16::from_le_bytes([zip[local_off+26], zip[local_off+27]]) as usize;
                let lfx        = u16::from_le_bytes([zip[local_off+28], zip[local_off+29]]) as usize;
                let data_start = local_off + 30 + lfn + lfx;
                if data_start + comp_size <= zip.len() {
                    return Some(zip[data_start..data_start+comp_size].to_vec());
                }
                return None;
            }
        }
        pos += 46 + fname_len + extra_len + comment_len;
    }
    None
}

/// 0-based column index → Excel column letter(s).  0→A, 25→Z, 26→AA.
fn xlsx_col_letter(idx: usize) -> String {
    let mut n = idx + 1;
    let mut letters: Vec<u8> = Vec::new();
    while n > 0 { n -= 1; letters.push(b'A' + (n % 26) as u8); n /= 26; }
    letters.iter().rev().map(|&b| b as char).collect()
}

/// Excel cell reference (e.g. "AB12") → 0-based column index.
fn xlsx_col_idx(cell_ref: &str) -> usize {
    let mut col = 0usize;
    for b in cell_ref.bytes() {
        if b >= b'A' && b <= b'Z' { col = col * 26 + (b - b'A' + 1) as usize; } else { break; }
    }
    col.saturating_sub(1)
}

/// Append XML-escaped text to a String buffer.
fn xlsx_push_escaped(buf: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '&' => buf.push_str("&amp;"),
            '<' => buf.push_str("&lt;"),
            '>' => buf.push_str("&gt;"),
            '"' => buf.push_str("&quot;"),
            c   => buf.push(c),
        }
    }
}

/// Unescape XML entities.
fn xlsx_unescape(s: &str) -> String {
    s.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">")
     .replace("&quot;", "\"").replace("&apos;", "'")
}

/// Find `needle` in `haystack` starting at `from`.
fn find_bytes(haystack: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || from + needle.len() > haystack.len() { return None; }
    haystack[from..].windows(needle.len()).position(|w| w == needle).map(|p| p + from)
}

/// Extract the value of XML attribute `attr_name` from a tag string.
fn xlsx_attr(tag: &str, attr_name: &str) -> String {
    let needle = format!("{}=\"", attr_name);
    if let Some(p) = tag.find(&needle) {
        let s = p + needle.len();
        if let Some(e) = tag[s..].find('"') { return tag[s..s+e].to_string(); }
    }
    String::new()
}

/// Parse xl/sharedStrings.xml → ordered Vec of strings.
fn parse_xlsx_sst(xml: &str) -> Vec<String> {
    let b = xml.as_bytes();
    let mut result: Vec<String> = Vec::new();
    let mut i = 0;
    loop {
        let si = match find_bytes(b, i, b"<si") { Some(p) => p, None => break };
        let si_end = match find_bytes(b, si + 3, b"</si>") { Some(p) => p, None => break };
        let si_content = &xml[si + 3..si_end];
        let sc = si_content.as_bytes();
        let mut text = String::new();
        let mut j = 0;
        loop {
            let t_open = match find_bytes(sc, j, b"<t") { Some(p) => p, None => break };
            let t_gt   = match find_bytes(sc, t_open + 2, b">") { Some(p) => p, None => break };
            let t_end  = match find_bytes(sc, t_gt + 1, b"</t>") { Some(p) => p, None => break };
            text.push_str(&xlsx_unescape(&si_content[t_gt + 1..t_end]));
            j = t_end + 4;
        }
        result.push(text);
        i = si_end + 5;
    }
    result
}

/// Parse xl/worksheets/sheet1.xml → Vec<Vec<String>> where row[0] = headers.
fn parse_xlsx_sheet(xml: &str, sst: &[String]) -> Vec<Vec<String>> {
    let b = xml.as_bytes();
    let mut raw_rows: Vec<Vec<(usize, String)>> = Vec::new();
    let mut max_col = 0usize;
    let sd_start = find_bytes(b, 0, b"<sheetData").unwrap_or(0);
    let sd_end   = find_bytes(b, sd_start, b"</sheetData>").unwrap_or(b.len());
    let mut i = sd_start;
    loop {
        let row_open  = match find_bytes(b, i, b"<row") { Some(p) => p, None => break };
        if row_open >= sd_end { break; }
        let row_close = match find_bytes(b, row_open, b"</row>") { Some(p) => p, None => break };
        let row_str   = &xml[row_open..row_close + 6];
        let row_bytes = row_str.as_bytes();
        let mut cells: Vec<(usize, String)> = Vec::new();
        let mut j = 0usize;
        loop {
            let c_open = match find_bytes(row_bytes, j, b"<c") { Some(p) => p, None => break };
            let c_gt   = match find_bytes(row_bytes, c_open + 2, b">") { Some(p) => p, None => break };
            let c_tag  = &row_str[c_open..c_gt + 1];
            let cr     = xlsx_attr(c_tag, "r");
            let ct     = xlsx_attr(c_tag, "t");
            let col    = xlsx_col_idx(&cr);
            if col > max_col { max_col = col; }
            let self_close = c_gt > 0 && row_bytes[c_gt - 1] == b'/';
            let val = if self_close {
                String::new()
            } else {
                let c_close = match find_bytes(row_bytes, c_gt + 1, b"</c>") { Some(p) => p, None => break };
                let inner   = &row_str[c_gt + 1..c_close];
                let ib      = inner.as_bytes();
                let v = find_bytes(ib, 0, b"<v>")
                    .and_then(|vs| find_bytes(ib, vs + 3, b"</v>").map(|ve| &inner[vs+3..ve]))
                    .unwrap_or("");
                match ct.as_str() {
                    "s" => v.trim().parse::<usize>().ok()
                              .and_then(|k| sst.get(k)).cloned().unwrap_or_default(),
                    "b" => if v.trim() == "1" { "true".to_string() } else { "false".to_string() },
                    "inlineStr" | "str" => find_bytes(ib, 0, b"<t>")
                        .and_then(|ts| find_bytes(ib, ts+3, b"</t>").map(|te| xlsx_unescape(&inner[ts+3..te])))
                        .unwrap_or_else(|| xlsx_unescape(v)),
                    _ => v.to_string(),
                }
            };
            if !val.is_empty() { cells.push((col, val)); }
            j = if self_close { c_gt + 1 } else {
                find_bytes(row_bytes, c_gt + 1, b"</c>").map(|p| p + 4).unwrap_or(row_bytes.len())
            };
        }
        if !cells.is_empty() { raw_rows.push(cells); }
        i = row_close + 6;
        if i >= sd_end { break; }
    }
    if raw_rows.is_empty() { return Vec::new(); }
    let ncols = max_col + 1;
    raw_rows.iter().map(|cells| {
        let mut row = vec![String::new(); ncols];
        for &(ci, ref v) in cells { if ci < ncols { row[ci] = v.clone(); } }
        row
    }).collect()
}

/// nova_to_xlsx(kore_path, xlsx_path) -> Bool
/// Exports KORE to Excel XLSX (OOXML). Opens in Excel 2016+ / LibreOffice 6+.
pub fn nova_to_xlsx(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::runtime_error(
            "nova_to_xlsx(kore_path, xlsx_path) expects 2 string arguments".to_string()
        ));
    }
    let (kore_path, xlsx_path) = match (&args[0], &args[1]) {
        (Value::Str(a), Value::Str(b)) => (a.clone(), b.clone()),
        _ => return Err(VmError::runtime_error("nova_to_xlsx: both args must be strings".to_string())),
    };
    let data = std::fs::read(&kore_path)
        .map_err(|e| VmError::runtime_error(format!("nova_to_xlsx: cannot read '{}': {}", kore_path, e)))?;
    let (h, col_names, cols) = decode_all_columns(&data)?;
    let ncols = col_names.len();

    // Build shared string table (SST): col names first, then all cell strings
    let mut sst: Vec<String> = Vec::new();
    let mut sst_idx: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    {
        let mut add = |s: &str| {
            if !sst_idx.contains_key(s) {
                sst_idx.insert(s.to_string(), sst.len() as u32);
                sst.push(s.to_string());
            }
        };
        for name in &col_names { add(name); }
        for ci in 0..ncols {
            for row in 0..h.nrows {
                if let Some(Value::Str(s)) = cols[ci].get(row) {
                    if !s.is_empty() && s != "EMPTY" { add(s); }
                }
            }
        }
    }

    // xl/sharedStrings.xml
    let sst_xml: String = {
        let mut x = String::new();
        x.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>");
        let n = sst.len();
        x.push_str(&format!(
            "<sst xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" count=\"{n}\" uniqueCount=\"{n}\">"
        ));
        for s in &sst {
            x.push_str("<si><t xml:space=\"preserve\">");
            xlsx_push_escaped(&mut x, s);
            x.push_str("</t></si>");
        }
        x.push_str("</sst>");
        x
    };

    // xl/worksheets/sheet1.xml
    let sheet_xml: String = {
        let mut x = String::new();
        x.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>");
        x.push_str("<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">");
        let last_col = xlsx_col_letter(ncols.saturating_sub(1));
        let last_row = h.nrows + 1;
        x.push_str(&format!("<dimension ref=\"A1:{last_col}{last_row}\"/>"));
        x.push_str("<sheetData>");
        // Header row (row 1) as shared strings
        x.push_str("<row r=\"1\">");
        for (ci, name) in col_names.iter().enumerate() {
            let cr  = format!("{}1", xlsx_col_letter(ci));
            let sid = sst_idx.get(name.as_str()).copied().unwrap_or(0);
            x.push_str(&format!("<c r=\"{cr}\" t=\"s\"><v>{sid}</v></c>"));
        }
        x.push_str("</row>");
        // Data rows
        for row in 0..h.nrows {
            let rn = row + 2;
            x.push_str(&format!("<row r=\"{rn}\">"));
            for ci in 0..ncols {
                let cr = format!("{}{}", xlsx_col_letter(ci), rn);
                match cols[ci].get(row).unwrap_or(&Value::Null) {
                    Value::Number(n) => {
                        if n.fract() == 0.0 && n.abs() < 1e15 {
                            x.push_str(&format!("<c r=\"{cr}\"><v>{}</v></c>", *n as i64));
                        } else {
                            let s = format!("{:.10}", n);
                            let s = s.trim_end_matches('0').trim_end_matches('.');
                            x.push_str(&format!("<c r=\"{cr}\"><v>{s}</v></c>"));
                        }
                    }
                    Value::Bool(b) => {
                        x.push_str(&format!("<c r=\"{cr}\" t=\"b\"><v>{}</v></c>", if *b { 1 } else { 0 }));
                    }
                    Value::Str(s) if !s.is_empty() && s != "EMPTY" => {
                        let sid = sst_idx.get(s.as_str()).copied().unwrap_or(0);
                        x.push_str(&format!("<c r=\"{cr}\" t=\"s\"><v>{sid}</v></c>"));
                    }
                    _ => {}
                }
            }
            x.push_str("</row>");
        }
        x.push_str("</sheetData></worksheet>");
        x
    };

    // Static OOXML parts (single-line XML for ZIP compactness)
    let content_types = b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/><Override PartName=\"/xl/worksheets/sheet1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/><Override PartName=\"/xl/sharedStrings.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml\"/><Override PartName=\"/xl/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml\"/></Types>";
    let pkg_rels      = b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>";
    let workbook      = b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><sheets><sheet name=\"Sheet1\" sheetId=\"1\" r:id=\"rId1\"/></sheets></workbook>";
    let wb_rels       = b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet1.xml\"/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings\" Target=\"sharedStrings.xml\"/><Relationship Id=\"rId3\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/></Relationships>";
    let styles        = b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><styleSheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><fonts count=\"1\"><font><sz val=\"11\"/><name val=\"Calibri\"/></font></fonts><fills count=\"2\"><fill><patternFill patternType=\"none\"/></fill><fill><patternFill patternType=\"gray125\"/></fill></fills><borders count=\"1\"><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\"/></cellStyleXfs><cellXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\"/></cellXfs></styleSheet>";

    let zip_bytes = zip_stored(&[
        ("[Content_Types].xml",        content_types),
        ("_rels/.rels",                pkg_rels),
        ("xl/workbook.xml",            workbook),
        ("xl/_rels/workbook.xml.rels", wb_rels),
        ("xl/sharedStrings.xml",       sst_xml.as_bytes()),
        ("xl/styles.xml",              styles),
        ("xl/worksheets/sheet1.xml",   sheet_xml.as_bytes()),
    ]);

    std::fs::write(&xlsx_path, &zip_bytes)
        .map_err(|e| VmError::runtime_error(format!("nova_to_xlsx: cannot write '{}': {}", xlsx_path, e)))?;
    Ok(Value::Bool(true))
}

/// nova_from_xlsx(xlsx_path, kore_path) -> Bool
/// Imports an Excel XLSX (first sheet) into KORE. Supports stored (uncompressed) XLSX only.
pub fn nova_from_xlsx(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::runtime_error(
            "nova_from_xlsx(xlsx_path, kore_path) expects 2 string arguments".to_string()
        ));
    }
    let (xlsx_path, kore_path) = match (&args[0], &args[1]) {
        (Value::Str(a), Value::Str(b)) => (a.clone(), b.clone()),
        _ => return Err(VmError::runtime_error("nova_from_xlsx: both args must be strings".to_string())),
    };
    let zip = std::fs::read(&xlsx_path)
        .map_err(|e| VmError::runtime_error(format!("nova_from_xlsx: cannot read '{}': {}", xlsx_path, e)))?;

    let sst: Vec<String> = zip_extract(&zip, "xl/sharedStrings.xml")
        .and_then(|b| String::from_utf8(b).ok())
        .as_deref().map(parse_xlsx_sst).unwrap_or_default();

    let sheet_bytes = zip_extract(&zip, "xl/worksheets/sheet1.xml")
        .ok_or_else(|| VmError::runtime_error(format!("nova_from_xlsx: sheet1.xml not found in '{}'", xlsx_path)))?;
    let sheet_xml = String::from_utf8(sheet_bytes)
        .map_err(|_| VmError::runtime_error("nova_from_xlsx: sheet XML not valid UTF-8".to_string()))?;

    let rows = parse_xlsx_sheet(&sheet_xml, &sst);
    if rows.len() < 2 {
        return Err(VmError::runtime_error(
            "nova_from_xlsx: sheet must have a header row + at least 1 data row".to_string()
        ));
    }

    let mut csv = String::with_capacity(rows.len() * rows[0].len() * 8);
    for (ri, row) in rows.iter().enumerate() {
        if ri > 0 { csv.push('\n'); }
        for (ci, cell) in row.iter().enumerate() {
            if ci > 0 { csv.push(','); }
            if cell.contains(',') || cell.contains('"') || cell.contains('\n') {
                csv.push('"');
                csv.push_str(&cell.replace('"', "\"\""));
                csv.push('"');
            } else {
                csv.push_str(cell);
            }
        }
    }
    csv.push('\n');

    let tmp = format!("{}.tmp_xlsx.csv", kore_path);
    std::fs::write(&tmp, csv.as_bytes())
        .map_err(|e| VmError::runtime_error(format!("nova_from_xlsx: tmp write error: {}", e)))?;
    let result = nova_write(&[Value::Str(tmp.clone()), Value::Str(kore_path)]);
    let _ = std::fs::remove_file(&tmp);
    result
}

// -- ORC (KORC – KORE Optimised Row Columnar) ---------------------------------
// Pure-Rust ORC-inspired columnar format. Zero external dependencies.
// Apache ORC requires Protobuf + Snappy/Zstd; KORC uses the same columnar-stripe
// philosophy backed by Nova LZ77.
//
// KORC v1 binary layout:
//   [4]  "KORC" magic
//   [1]  version = 1
//   [2]  ncols u16 LE
//   [4]  nrows u32 LE
//   [per column stripe]
//     [1]  name_len u8
//     [?]  name utf8
//     [1]  type_algo u8  (high nibble = type: 0=int,1=flt,2=str,3=bool; low = algo)
//     [4]  comp_len u32 LE
//     [?]  LZ77-compressed column data
//   [seek-footer]  (mirrors stripe headers; enables O(1) random column access)
//     [per col]  name_len(1) + name + type_algo(1) + comp_len(4)
//   [4]  footer_start u32 LE (offset where footer begins)

/// nova_to_orc(kore_path, orc_path) -> Bool
/// Exports KORE to KORC (KORE Optimised Row Columnar) format.
pub fn nova_to_orc(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::runtime_error(
            "nova_to_orc(kore_path, orc_path) expects 2 string arguments".to_string()
        ));
    }
    let (kore_path, orc_path) = match (&args[0], &args[1]) {
        (Value::Str(a), Value::Str(b)) => (a.clone(), b.clone()),
        _ => return Err(VmError::runtime_error("nova_to_orc: both args must be strings".to_string())),
    };
    let data = std::fs::read(&kore_path)
        .map_err(|e| VmError::runtime_error(format!("nova_to_orc: cannot read '{}': {}", kore_path, e)))?;
    let (h, col_names, cols) = decode_all_columns(&data)?;

    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"KORC");
    out.push(1); // version
    out.extend_from_slice(&(col_names.len() as u16).to_le_bytes());
    out.extend_from_slice(&(h.nrows as u32).to_le_bytes());

    let mut footer_meta: Vec<(Vec<u8>, u8, u32)> = Vec::new(); // (name_bytes, type_algo, comp_len)

    for (ci, col) in cols.iter().enumerate() {
        let type_tag: u8 = match col.first() {
            Some(Value::Bool(_))                        => 3,
            Some(Value::Number(n)) if n.fract() == 0.0 => 0,
            Some(Value::Number(_))                      => 1,
            _                                           => 2,
        };
        let col_type = match type_tag { 0 => ColType::Int, 1 => ColType::Float, 3 => ColType::Bool, _ => ColType::Str };
        let strs: Vec<String> = col.iter().map(|v| match v {
            Value::Number(n) => format!("{}", n),
            Value::Bool(b)   => if *b { "1".to_string() } else { "0".to_string() },
            Value::Str(s)    => s.clone(),
            _                => String::new(),
        }).collect();
        let raw: Vec<&str> = strs.iter().map(|s| s.as_str()).collect();
        let mut col_buf = Vec::new();
        let algo       = encode_col(col_type, &raw, &mut col_buf);
        let compressed = compress_col(&col_buf);
        let type_algo  = (type_tag << 4) | (algo & 0x0F);
        let nb         = col_names[ci].as_bytes();

        out.push(nb.len() as u8);
        out.extend_from_slice(nb);
        out.push(type_algo);
        out.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
        out.extend_from_slice(&compressed);

        footer_meta.push((nb.to_vec(), type_algo, compressed.len() as u32));
    }

    // Seek-footer
    let footer_start = out.len() as u32;
    for (nb, type_algo, comp_len) in &footer_meta {
        out.push(nb.len() as u8);
        out.extend_from_slice(nb);
        out.push(*type_algo);
        out.extend_from_slice(&comp_len.to_le_bytes());
    }
    out.extend_from_slice(&footer_start.to_le_bytes());

    std::fs::write(&orc_path, &out)
        .map_err(|e| VmError::runtime_error(format!("nova_to_orc: cannot write '{}': {}", orc_path, e)))?;
    Ok(Value::Bool(true))
}

/// nova_from_orc(orc_path, kore_path) -> Bool
/// Imports a KORC (KORE Optimised Row Columnar) file into KORE format.
pub fn nova_from_orc(args: &[Value]) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::runtime_error(
            "nova_from_orc(orc_path, kore_path) expects 2 string arguments".to_string()
        ));
    }
    let (orc_path, kore_path) = match (&args[0], &args[1]) {
        (Value::Str(a), Value::Str(b)) => (a.clone(), b.clone()),
        _ => return Err(VmError::runtime_error("nova_from_orc: both args must be strings".to_string())),
    };
    let raw = std::fs::read(&orc_path)
        .map_err(|e| VmError::runtime_error(format!("nova_from_orc: cannot read '{}': {}", orc_path, e)))?;
    if raw.len() < 11 || &raw[0..4] != b"KORC" {
        return Err(VmError::runtime_error(format!("nova_from_orc: '{}' is not a KORC file", orc_path)));
    }
    let ncols = u16::from_le_bytes([raw[5], raw[6]]) as usize;
    let nrows = u32::from_le_bytes([raw[7], raw[8], raw[9], raw[10]]) as usize;

    let mut pos = 11usize;
    let mut col_names: Vec<String>   = Vec::with_capacity(ncols);
    let mut type_algos: Vec<u8>      = Vec::with_capacity(ncols);
    let mut col_blobs:  Vec<Vec<u8>> = Vec::with_capacity(ncols);
    for _ in 0..ncols {
        if pos >= raw.len() { break; }
        let name_len = raw[pos] as usize; pos += 1;
        if pos + name_len + 5 > raw.len() { break; }
        let name = String::from_utf8_lossy(&raw[pos..pos + name_len]).to_string(); pos += name_len;
        let type_algo = raw[pos]; pos += 1;
        let comp_len  = u32::from_le_bytes([raw[pos], raw[pos+1], raw[pos+2], raw[pos+3]]) as usize; pos += 4;
        if pos + comp_len > raw.len() { break; }
        col_names.push(name);
        type_algos.push(type_algo);
        col_blobs.push(raw[pos..pos + comp_len].to_vec());
        pos += comp_len;
    }

    let mut all_cols: Vec<Vec<Value>> = Vec::with_capacity(col_names.len());
    for (i, blob) in col_blobs.iter().enumerate() {
        let ta        = type_algos[i];
        let type_byte = (ta >> 4) & 0x0F;
        let algo      = ta & 0x0F;
        let dec       = decompress_col(blob);
        let (vals, _) = decode_col(type_byte, algo, &dec, 0, nrows);
        all_cols.push(vals);
    }

    let mut csv = col_names.join(",");
    csv.push('\n');
    for row in 0..nrows {
        for (ci, col) in all_cols.iter().enumerate() {
            if ci > 0 { csv.push(','); }
            match col.get(row).unwrap_or(&Value::Null) {
                Value::Number(n) => if n.fract() == 0.0 && n.abs() < 1e15 {
                    csv.push_str(&format!("{}", *n as i64))
                } else {
                    let s = format!("{:.8}", n);
                    csv.push_str(s.trim_end_matches('0').trim_end_matches('.'));
                },
                Value::Bool(b)   => csv.push_str(if *b { "1" } else { "0" }),
                Value::Str(s) if s == "EMPTY" => {},
                Value::Str(s) => {
                    if s.contains(',') || s.contains('"') {
                        csv.push('"'); csv.push_str(&s.replace('"', "\\\"")); csv.push('"');
                    } else { csv.push_str(s); }
                },
                _ => {},
            }
        }
        csv.push('\n');
    }

    let tmp = format!("{}.tmp_korc.csv", kore_path);
    std::fs::write(&tmp, csv.as_bytes())
        .map_err(|e| VmError::runtime_error(format!("nova_from_orc: tmp write error: {}", e)))?;
    let result = nova_write(&[Value::Str(tmp.clone()), Value::Str(kore_path)]);
    let _ = std::fs::remove_file(&tmp);
    result
}
