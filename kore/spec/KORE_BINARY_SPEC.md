# KORE v2 Binary Format Specification

**Version**: 2  
**Magic**: `KORE` (4 bytes: 0x4B 0x4F 0x52 0x45)  
**Status**: Stable  
**Date**: April 2026  

---

## 1. File Layout

```
┌──────────────────────────────────────────────┐
│ HEADER (64 bytes, fixed)                     │
├──────────────────────────────────────────────┤
│ SCHEMA block (variable, compressed)          │
├──────────────────────────────────────────────┤
│ DICTIONARY pool (variable, compressed)       │
├──────────────────────────────────────────────┤
│ CHUNK 0                                      │
│   Column 0: [CRC32][comp_len][compressed]    │
│   Column 1: [CRC32][comp_len][compressed]    │
│   ...                                        │
├──────────────────────────────────────────────┤
│ CHUNK 1                                      │
│   ...                                        │
├──────────────────────────────────────────────┤
│ FOOTER (variable, compressed)                │
├──────────────────────────────────────────────┤
│ FOOTER_COMP_LEN (4 bytes, u32 LE)           │
│ FOOTER_OFFSET   (8 bytes, u64 LE)           │
└──────────────────────────────────────────────┘
```

**Reading order**: Read last 12 bytes → get footer offset → decompress footer → seek to any column.

---

## 2. Header (64 bytes)

| Offset | Size | Type | Description |
|--------|------|------|-------------|
| 0 | 4 | `[u8;4]` | Magic: `KORE` (0x4B4F5245) |
| 4 | 1 | `u8` | Version (must be 2) |
| 5 | 1 | `u8` | Flags (reserved, must be 0) |
| 6 | 2 | `u16 LE` | Number of columns (ncols) |
| 8 | 8 | `u64 LE` | Number of rows (nrows) |
| 16 | 4 | `u32 LE` | Number of chunks (nchunks) |
| 20 | 4 | `u32 LE` | Rows per chunk (chunk_size, default 65536) |
| 24 | 8 | `u64 LE` | Created timestamp (Unix epoch seconds) |
| 32 | 32 | `[u8;32]` | Reserved (zero-padded) |

---

## 3. Schema Block

Immediately after the header. Stored as:

```
[schema_comp_len: u32 LE] [compressed_schema_data]
```

**Uncompressed schema format** (per column, repeated `ncols` times):

| Field | Encoding | Description |
|-------|----------|-------------|
| name_len | varint | Length of column name in bytes |
| name | UTF-8 bytes | Column name |
| ktype | u8 | Column type (see §3.1) |
| flags | u8 | Bit 0 = encrypted |

### 3.1 Column Types (KType)

| Value | Name | Description |
|-------|------|-------------|
| 1 | Int | 64-bit signed integer |
| 2 | Float | 64-bit floating point (stored as fixed-point) |
| 3 | Bool | Boolean (true/false) |
| 4 | Str | UTF-8 string |
| 5 | Bytes | Raw byte array |
| 6 | Struct | Nested struct (child columns inline) |
| 7 | List | Variable-length list (offsets + child) |
| 8 | Map | Key-value map (offsets + key/value) |

---

## 4. Dictionary Pool

Immediately after the schema block:

```
[dict_comp_len: u32 LE] [compressed_dict_data]
```

**Uncompressed dictionary format**:

| Field | Encoding | Description |
|-------|----------|-------------|
| count | varint | Number of dictionary entries |
| For each entry: | | |
| → str_len | varint | Length in bytes |
| → str_data | UTF-8 bytes | String value |

---

## 5. Chunk Data

Each chunk contains `chunk_size` rows (last chunk may have fewer). Data is stored column-by-column within each chunk.

### 5.1 Column Block

Each column block within a chunk:

```
[crc32: u32 LE] [comp_len: u32 LE] [compressed_data: comp_len bytes]
```

- **crc32**: CRC32 of the compressed data (for integrity verification)
- **comp_len**: Length of compressed data in bytes
- **compressed_data**: Block-compressed codec output

### 5.2 Block Compression Pipeline

Each column's codec output goes through a 6-path compression pipeline. The tag byte (first byte) identifies the compression used:

| Tag | Method | Description |
|-----|--------|-------------|
| 0x00 | LZ77 only | LZ77 with 64KB window |
| 0x01 | Huffman(LZ77) | Canonical Huffman on LZ77 output |
| 0x02 | Raw | No compression (passthrough) |
| 0x03 | Huffman only | Huffman on raw data (no LZ77) |
| 0x04 | Range coder only | Arithmetic coding on raw data |
| 0x05 | Range(LZ77) | Arithmetic coding on LZ77 output |

The writer tries all applicable paths and picks the smallest output.

---

## 6. Codecs

### 6.0 Raw (codec=0)
No transform. Data stored as-is.

### 6.1 RLE — Run-Length Encoding (codec=1)

**Integer RLE**:
```
[count: varint] [value: zigzag varint]   (repeated)
```

**String RLE**:
```
[count: varint] [str_len: varint] [str_data: UTF-8]   (repeated)
```

### 6.2 Delta (codec=2)
```
[base: zigzag varint] [delta_1: zigzag varint] [delta_2: zigzag varint] ...
```
Values reconstructed as: `val[i] = val[i-1] + delta[i]`

### 6.3 DictRLE (codec=3)
Global dictionary indices encoded with RLE.

### 6.4 Bitpack (codec=4)
Booleans packed 8-per-byte, LSB-first.
```
Byte 0: rows 0-7,  Byte 1: rows 8-15, ...
Bit 0 = row N, Bit 1 = row N+1, etc.
```

### 6.5 BDict — Bit-Packed Dictionary (codec=5)
```
[num_unique: varint]
For each unique value:
  [str_len: varint] [str_data: UTF-8]
[bits_per_index: u8]
[packed indices: ceil(bits_per_index × nrows / 8) bytes, LSB-first]
```

### 6.6 CDelta — Constant Delta (codec=6)
For perfectly sequential data (e.g., row IDs):
```
[base: zigzag varint] [step: zigzag varint]
```
Values: `val[i] = base + step × i`. Entire column in 2 varints.

### 6.7 FOR — Frame of Reference (codec=7)
```
[min_value: zigzag varint] [bits: u8]
[packed residuals: ceil(bits × nrows / 8) bytes, LSB-first]
```
Values: `val[i] = min_value + residual[i]`

### 6.8 HuffDict — Huffman Dictionary (codec=8)
Like BDict but dictionary indices are Huffman-coded instead of uniform-bit-packed. Better for skewed distributions where some values are much more frequent.

```
[num_unique: varint]
For each unique value:
  [str_len: varint] [str_data: UTF-8]
[huffman_coded_indices]
```

### 6.9 Derived (codec=9)
Cross-column formula encoding. Stores a formula type + source column indices + residuals.
```
[formula_type: u8]   (0=sum, 1=product, 2=diff, 3=ratio)
[num_sources: u8]
[source_indices: u8 × num_sources]
[scale_exp: u8]
[residuals: codec-encoded i64 array]
```
Values reconstructed as: `val[i] = formula(source_cols[i]) + residual[i]`

### 6.10 Float Encoding
Float columns are stored as fixed-point integers. The first byte of the codec data is the **scale exponent**:

| Scale Exp | Multiplier | Precision |
|-----------|------------|-----------|
| 0 | ×1 | Integer |
| 1 | ×10 | 1 decimal |
| 2 | ×100 | 2 decimals |
| 3 | ×1000 | 3 decimals |
| 4 | ×10000 | 4 decimals (default) |

`stored_value = round(float_value × 10^scale_exp)`

---

## 7. Footer

Located at `FOOTER_OFFSET` (read from last 12 bytes of file).

```
[footer_comp_len: u32 LE]   ← at file_size - 12
[footer_offset: u64 LE]     ← at file_size - 8
```

**Uncompressed footer format**:

| Field | Encoding | Description |
|-------|----------|-------------|
| nchunks | u32 LE | Number of chunks |
| ncols | u16 LE | Number of columns |
| chunk_nrows[] | u32 LE × nchunks | Row count per chunk |
| **Per-chunk per-column metadata** (nchunks × ncols entries): | | |
| → file_offset | u64 LE | Byte offset of column block in file |
| → comp_len | u32 LE | Compressed length in bytes |
| → codec | u8 | Codec ID (see §6) |
| → null_count | u32 LE | Number of NULL values |
| → min_i64 | zigzag varint | Minimum value (for Int/Float) |
| → max_i64 | zigzag varint | Maximum value (for Int/Float) |
| → min_str_len | varint | Length of min string |
| → min_str | UTF-8 bytes | Minimum string value |
| → max_str_len | varint | Length of max string |
| → max_str | UTF-8 bytes | Maximum string value |
| → bloom_filter | 512 bytes | 4096-bit Bloom filter |

### 7.1 Bloom Filter

512-byte (4096-bit) Bloom filter per chunk per column. Uses 3 hash functions with seeds 0x1234, 0x5678, 0x9ABC.

```
hash(seed, string) = fold(seed ⊕ byte × 0x517cc1b727220a95) mod 4096
```

---

## 8. Varint Encoding

Standard LEB128 (Little-Endian Base 128):
```
if value < 128: [value]
if value < 16384: [value & 0x7F | 0x80] [value >> 7]
...
```

**Zigzag encoding** for signed integers:
```
encode: (n << 1) ^ (n >> 63)
decode: (v >> 1) ^ -(v & 1)
```

---

## 9. LZ77 Format

Window size: 65535 bytes. Minimum match: 6 bytes.

```
Literal byte: [byte]               (if byte ≠ 0xFF)
Match:        [0xFF] [offset: u16 LE] [length: u16 LE]
Escape 0xFF:  [0xFF] [0x00 0x00] [0x01 0x00]   (offset=0, length=1)
```

---

## 10. Huffman Coding

Canonical Huffman with two header formats:

**Sparse format** (≤64 symbols): `[orig_len: u32 LE] [nsyms: u16 LE] [sym, code_len] × nsyms [bitstream]`

**Full format** (>64 symbols): `[orig_len: u32 LE] [nsyms: u16 LE] [code_len × 256] [bitstream]`

Codes are canonical: sorted by (code_length, symbol), assigned sequentially.

---

## 11. Encryption

Per-column XOR stream cipher (legacy) or AES-256-CTR (v2.1+).

**XOR cipher**: `state = state ⊕ key[i%32]; state = state × 0x9e3779b97f4a7c15 <<< 17; out = byte ⊕ (state >> 32)`

**AES-256-CTR**: Standard AES-256 in counter mode. Nonce (12 bytes) + counter (4 bytes big-endian).

---

## 12. Sidecar Files

| Extension | Purpose |
|-----------|---------|
| `.kore.del` | Delete bitmap (soft-deleted rows) |
| `.kore.versions` | Version manifest for time travel |
| `.kore.vN` | Versioned snapshot (N = version number) |

---

*KORE v2 — Killer Optimized Record Exchange. Pure Rust. Zero dependencies.*
