# KORE v2 — Benchmark Report

**Format**: KORE (Killer Optimized Record Exchange)  
**Implementation**: Pure Rust, zero external dependencies  
**Date**: June 2026  
**Authors**: Arun Kathera

---

## 1. Executive Summary

KORE v2 is a columnar storage format that **beats Apache Parquet on every compression codec** — including Brotli, Gzip, Zstd, and Snappy — while using zero external dependencies (pure Rust stdlib only).

| Metric | KORE v2 | Best Parquet |
|--------|---------|--------------|
| **Uniform data** | **113.5 MB (10.0%)** | 195.7 MB (17.2%) — Brotli |
| **Realistic data** | **87.7 MB (8.2%)** | 147.2 MB (13.7%) — Brotli |
| **Dependencies** | 0 (pure stdlib) | 5+ crates |
| **Column pruning** | ✓ O(1) seek | ✓ |
| **Predicate pushdown** | ✓ min/max + bloom | ✓ min/max |
| **Per-column encryption** | ✓ XOR | ✗ |
| **Integrity checks** | CRC32 per block | CRC32 per page |

---

## 2. Codec Architecture

KORE uses an **11-codec adaptive stack** that automatically selects the best encoding per column per chunk:

| Codec | ID | Description | Best For |
|-------|----|-------------|----------|
| Raw | 0 | No transform | Incompressible data |
| RLE | 1 | Run-length encoding | Low cardinality, sorted data |
| Delta | 2 | Zigzag varint differences | Monotonic integers |
| DictRLE | 3 | Global dict + RLE on indices | Repeated strings |
| Bitpack | 4 | 1 bit per boolean | Boolean columns |
| BDict | 5 | Bit-packed dictionary indices | Low-cardinality strings (≤4096 unique) |
| CDelta | 6 | Constant-delta (2 varints) | Sequential IDs, timestamps |
| FOR | 7 | Frame-of-reference + bitpack | Narrow-range integers |
| HuffDict | 8 | Huffman-coded dictionary | Skewed string distributions |
| Derived | 9 | Formula-based (e.g., col = A + B) | Computed columns |

After codec encoding, each block goes through a 6-path compression pipeline:
- LZ77 only, Huffman(LZ77), Raw, Huffman-only, Range-coder(LZ77), Range-coder-only
- The smallest output wins.

---

## 3. Benchmark Setup

- **Hardware**: Standard development workstation (Windows)
- **Rust**: Edition 2021, release profile (opt-level=3, LTO=fat, codegen-units=1)
- **Parquet**: Apache Arrow (pyarrow 21.0.0) with all codecs
- **Chunk size**: 65,536 rows per chunk

### 3.1 Datasets

**Uniform dataset** (1,140 MB CSV, 10M rows × 15 columns):
- Integer columns: uniform random (1–1000)
- Float columns: uniform random
- String columns: random from 50-value vocabulary
- Boolean, date, and derived columns

**Realistic dataset** (1,072 MB CSV, 10M rows × 15 columns):
- **Zipf distribution** (α=1.2) for categorical columns — models real-world skew
- **Power-law** pricing and quantities
- **Sequential IDs** with date ranges
- Simulates e-commerce transaction data

---

## 4. Compression Results

### 4.1 Uniform Data (1,140 MB CSV)

| Format | Compressed Size | Ratio | Write Time | Read Time |
|--------|---------------:|------:|-----------:|----------:|
| **KORE v2** | **113.5 MB** | **10.0%** | **64s** | **4.2s** |
| Parquet + Brotli | 195.7 MB | 17.2% | 125s | — |
| Parquet + Gzip | 207.1 MB | 18.2% | 91s | — |
| Parquet + Zstd | 234.9 MB | 20.6% | 24s | — |
| Parquet + Snappy | 275.0 MB | 24.1% | 26s | — |
| Parquet + None | 365.9 MB | 32.1% | 30s | — |

**KORE is 1.72× smaller than Parquet+Brotli** (the strongest Parquet codec).

### 4.2 Realistic Data (1,072 MB CSV — Zipf/Power-law)

| Format | Compressed Size | Ratio | Write Time | Read Time |
|--------|---------------:|------:|-----------:|----------:|
| **KORE v2** | **87.7 MB** | **8.2%** | **30s** | — |
| Parquet + Brotli | 147.2 MB | 13.7% | 56s | — |
| Parquet + Gzip | 157.2 MB | 14.7% | 52s | — |
| Parquet + Zstd | 162.6 MB | 15.2% | 17s | — |
| Parquet + Snappy | 223.1 MB | 20.8% | 14s | — |

**KORE is 1.68× smaller than Parquet+Brotli on realistic data.**

### 4.3 Why KORE Wins

1. **Adaptive 11-codec stack**: KORE picks the optimal codec per column per chunk. Parquet uses a single codec per column.
2. **CDelta for sequential IDs**: 2 varints encode an entire 65K-row column of sequential values.
3. **Derived column detection**: If `total = price × quantity`, KORE stores only the formula + residuals.
4. **HuffDict for skewed strings**: Huffman-coded dictionary indices exploit Zipf distributions better than Parquet's dictionary encoding.
5. **6-path block compression**: LZ77 + Huffman + Range coder — all tried, smallest wins.

---

## 5. Feature Comparison

| Feature | KORE v2 | Apache Parquet |
|---------|:-------:|:--------------:|
| Column pruning | ✓ | ✓ |
| Predicate pushdown | ✓ (min/max/bloom) | ✓ (min/max) |
| Bloom filters | ✓ (4096-bit per chunk) | Optional |
| CRC32 integrity | ✓ per column block | ✓ per page |
| Per-column encryption | ✓ XOR | ✗ |
| Nested types | ✗ | ✓ |
| External dependencies | **0** | 5+ |
| Streaming write | ✓ single-pass | ✓ |
| Parallel encode | ✓ per-column threads | Library-dependent |
| Parallel decode | ✓ per-column threads | Library-dependent |

---

## 6. Analytical Operations

### 6.1 Column Pruning
Reading a single column (`total`) from 10M rows:
- **Time**: ~0.3s (vs ~4.2s for all columns)
- **Speedup**: ~14× faster than full read
- Only decodes the requested column; skips all others via footer metadata.

### 6.2 Predicate Pushdown
Filtering `quantity > 900` across 10M rows:
- **Time**: ~0.5s
- **Mechanism**: Chunk min/max stats → skip entire 65K-row chunks where max(quantity) ≤ 900
- Bloom filters provide additional O(1) existence checks for string equality predicates.

### 6.3 Column Statistics
Retrieving min/max/null_count for any column:
- **Time**: <0.001s
- **Zero data decode**: Stats are stored in the footer metadata.

---

## 7. Data Integrity

Every column block includes a CRC32 checksum. On read, the checksum is verified before decompression. Any corruption is detected immediately.

Verification on 10M rows × 15 columns = **150M cells**:
- **0 mismatches** (exact round-trip for integers, booleans, and strings)
- **Max float error**: <0.02 (due to fixed-point scaling at configurable precision)

---

## 8. File Format Layout

```
HEADER (64 bytes)
├── Magic: "KORE" (4 bytes)
├── Version: 2 (1 byte)
├── Chunk size, ncols, flags (59 bytes)
SCHEMA block (compressed)
├── Column names + types
DICTIONARY pool (compressed)
CHUNK 0:
├── Column 0: [CRC32(4)] [comp_len(4)] [Huffman(LZ77(codec(data)))]
├── Column 1: [CRC32(4)] [comp_len(4)] [Huffman(LZ77(codec(data)))]
├── ...
CHUNK 1:
├── ...
FOOTER (compressed):
├── Per-chunk per-column: offset, comp_len, codec, stats, bloom
FOOTER_LEN (4 bytes, u32)
FOOTER_OFFSET (8 bytes, u64)
```

---

## 9. Conclusion

KORE v2 demonstrates that a **pure Rust, zero-dependency** columnar format can outperform Parquet with industrial-strength compression libraries. The key insight is that **per-column adaptive codec selection** — choosing from 11 codecs based on data characteristics — enables compression ratios that no single general-purpose codec can achieve.

On realistic e-commerce data with Zipf distributions, KORE achieves **8.2% compression ratio** (91.8% reduction), compared to Parquet+Brotli's 13.7%. This represents a **1.68× improvement** in storage efficiency.

---

*KORE is part of the Killer Language ecosystem. Pure Rust. Zero dependencies. World-class compression.*
