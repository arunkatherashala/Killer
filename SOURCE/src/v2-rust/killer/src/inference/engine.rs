// inference/engine.rs — Killer Native Transformer Inference Engine
//
// Runs LLaMA/Mistral/Phi/Gemma-family models loaded from GGUF files.
// No external dependencies — pure Rust matrix math, zero unsafe.
//
// Architecture: Transformer decoder-only (GPT-style)
//   Token embedding → N × [RMSNorm → SelfAttention → RMSNorm → SwiGLU FFN] → Unembedding
//   Uses RoPE positional encoding, GQA (grouped-query attention), KV cache.

use std::collections::HashMap;
use super::gguf::{GgufFile, GgmlType};
use super::tokenizer::KillerTokenizer;
use super::sampler::{GenerateConfig, sample as sample_token};
use super::quant;

// --- Architecture detect ------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum ArchType { LLaMA, Qwen2, Phi, Gemma, Mistral, Unknown }

impl ArchType {
    fn from_str(s: &str) -> Self {
        match s {
            "llama"   | "llama2"  | "llama3"           => ArchType::LLaMA,
            "qwen2"   | "qwen"    | "qwen2_5"           => ArchType::Qwen2,
            "phi"     | "phi2"    | "phi3"               => ArchType::Phi,
            "gemma"   | "gemma2"                         => ArchType::Gemma,
            "mistral" | "mixtral"                        => ArchType::Mistral,
            _                     => ArchType::LLaMA,  // sensible default
        }
    }

    /// Whether this architecture uses QKV attention biases.
    /// Qwen2 and Phi-3 add learned bias vectors to Q, K, V projections.
    pub fn has_qkv_bias(&self) -> bool {
        // NOTE: K bias values range -130 to +121 but they are legitimate for Qwen2
        matches!(self, ArchType::Qwen2 | ArchType::Phi)
    }
}

// --- Model config -------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ModelConfig {
    pub arch:       ArchType,
    pub n_layers:   usize,
    pub n_heads:    usize,
    pub n_kv_heads: usize,    // < n_heads when using GQA (grouped-query attention)
    pub n_embd:     usize,
    pub n_ff:       usize,
    pub vocab_size: usize,
    pub ctx_len:    usize,
    pub head_dim:   usize,
    pub rope_theta: f32,
    pub norm_eps:   f32,
}

impl ModelConfig {
    fn from_gguf(gguf: &GgufFile) -> Result<Self, String> {
        let arch_str = gguf.meta_str("general.architecture").unwrap_or_else(|| "llama".to_string());
        let arch     = ArchType::from_str(&arch_str);
        let p        = &arch_str; // metadata key prefix matches arch string

        let n_layers   = gguf.meta_u64(&format!("{}.block_count",             p)).ok_or("missing block_count")? as usize;
        let n_heads    = gguf.meta_u64(&format!("{}.attention.head_count",    p)).ok_or("missing head_count")? as usize;
        let n_embd     = gguf.meta_u64(&format!("{}.embedding_length",        p)).ok_or("missing embedding_length")? as usize;
        let n_kv_heads = gguf.meta_u64(&format!("{}.attention.head_count_kv", p)).unwrap_or(n_heads as u64) as usize;
        let n_ff       = gguf.meta_u64(&format!("{}.feed_forward_length",     p)).unwrap_or((n_embd * 4 / 3) as u64) as usize;
        let ctx_len    = gguf.meta_u64(&format!("{}.context_length",          p)).unwrap_or(2048) as usize;
        let rope_theta = gguf.meta_f32(&format!("{}.rope.freq_base",          p)).unwrap_or(10000.0);
        let norm_eps   = gguf.meta_f32(&format!("{}.attention.layer_norm_rms_epsilon", p)).unwrap_or(1e-5);
        // vocab_size: prefer explicit metadata, then count the actual token array
        let vocab_size = gguf.meta_u64(&format!("{}.vocab_size", p))
            .unwrap_or_else(|| gguf.meta_array_strings("tokenizer.ggml.tokens").len() as u64)
            as usize;
        let head_dim   = n_embd / n_heads.max(1);

        // DeepSeek-R1-Distill-Qwen-7B is based on Qwen2.5-7B which uses rope_theta=1,000,000.
        // Some GGUF builds incorrectly set it to 10,000; fix it here.
        // For Qwen2/Qwen2.5 with n_embd=3584 (7B), always override to 1M.
        let rope_theta = if arch == ArchType::Qwen2 && rope_theta < 100_000.0 {
            eprintln!("[Killer] rope_theta override: {} → 1,000,000 (Qwen2 7B-class model)", rope_theta);
            1_000_000.0f32
        } else {
            rope_theta
        };

        Ok(ModelConfig { arch, n_layers, n_heads, n_kv_heads, n_embd, n_ff, vocab_size, ctx_len, head_dim, rope_theta, norm_eps })
    }
}

// --- Quantized weight tensor -------------------------------------------------
//
// Stores a weight matrix in its native quantized form (Q4_K_M, Q8_0, etc.)
// instead of expanding to f32 at load time.
//
// Memory savings for DeepSeek-R1-7B (Q4_K_M):
//   f32 expansion: 7B params × 4 bytes = ~28 GB  ← OOM on most machines
//   QuantTensor  : 4.36 GB on disk ≈ 4.36 GB in RAM  ← fits comfortably
//
// Inference: one row is dequantized at a time and immediately dot-producted
// with the input vector — the f32 row never escapes the dot loop.

pub struct QuantTensor {
    pub data:  Vec<u8>,
    pub dtype: GgmlType,
    pub rows:  usize,   // out_dim  (number of output neurons = number of rows)
    pub cols:  usize,   // in_dim   (number of inputs  = elements per row)
}

impl QuantTensor {
    /// Construct from raw GGUF bytes.
    /// GGUF shape convention: shape[0] = in_dim (innermost/cols), shape[1] = out_dim (rows).
    fn from_raw(data: Vec<u8>, dtype: GgmlType, shape: &[u64]) -> Self {
        let cols = shape.first().copied().unwrap_or(1) as usize;
        let rows = shape.get(1).copied().unwrap_or(1) as usize;
        QuantTensor { data, dtype, rows, cols }
    }

    /// Dequantize row `r` of the matrix (produces `in_dim` f32 values).
    fn dequant_row(&self, r: usize, in_dim: usize) -> Vec<f32> {
        let epb = self.dtype.elements_per_block();
        let bpb = self.dtype.bytes_per_block();
        let n_blocks      = (in_dim + epb - 1) / epb;
        let bytes_per_row = n_blocks * bpb;
        let start = r * bytes_per_row;
        let end   = (start + bytes_per_row).min(self.data.len());
        if start >= self.data.len() { return vec![0.0; in_dim]; }
        dequant_dispatch(&self.data[start..end], self.dtype, in_dim)
    }

    /// Zero-allocation dot product: dot(dequant(row r), x).
    ///
    /// Processes each quantized block inline — dequantized values stay in
    /// registers and are immediately multiplied with x[].  No Vec allocation.
    /// This eliminates ~1.6M alloc/free per token for a 7B model.
    #[allow(dead_code)]
    #[inline]
    fn dot_row(&self, row_bytes: &[u8], x: &[f32], in_dim: usize) -> f32 {
        dot_inline(row_bytes, self.dtype, x, in_dim)
    }

    /// Matrix-vector product: y[r] = dot(row_r, x) for r in 0..out_dim.
    /// Zero-allocation per-row dot product + std::thread::scope parallelism.
    fn matvec(&self, x: &[f32], out_dim: usize, in_dim: usize) -> Vec<f32> {
        let epb = self.dtype.elements_per_block();
        let bpb = self.dtype.bytes_per_block();
        let n_blocks      = (in_dim + epb - 1) / epb;
        let bytes_per_row = n_blocks * bpb;
        let in_safe       = in_dim.min(x.len());
        let x_safe        = &x[..in_safe];
        let data          = self.data.as_slice();
        let dtype         = self.dtype;

        let n_threads = std::thread::available_parallelism()
            .map(|n| n.get()).unwrap_or(4).min(16);
        let chunk = (out_dim + n_threads - 1) / n_threads;

        let mut y = vec![0.0f32; out_dim];
        std::thread::scope(|scope| {
            let chunks: Vec<&mut [f32]> = y.chunks_mut(chunk).collect();
            let mut handles = Vec::with_capacity(chunks.len());
            for (t, slice) in chunks.into_iter().enumerate() {
                let row_start = t * chunk;
                handles.push(scope.spawn(move || {
                    // Temporary row dequant buffer (stack-allocated per thread,
                    // only used for the fallback path of rare quant types)
                    for (i, cell) in slice.iter_mut().enumerate() {
                        let r     = row_start + i;
                        let start = r * bytes_per_row;
                        let end   = (start + bytes_per_row).min(data.len());
                        if start >= data.len() { break; }
                        *cell = dot_inline(&data[start..end], dtype, x_safe, in_dim);
                    }
                }));
            }
            for h in handles { h.join().unwrap_or_default(); }
        });
        y
    }
}

/// Zero-allocation inline dot product: dispatches to the matching per-format function.
#[inline]
fn dot_inline(row_bytes: &[u8], dtype: GgmlType, x: &[f32], in_dim: usize) -> f32 {
    match dtype {
        GgmlType::Q4KM | GgmlType::Q4KS => dot_q4k(row_bytes, x, in_dim),
        GgmlType::Q6K                    => dot_q6k(row_bytes, x, in_dim),
        GgmlType::Q8_0                   => dot_q8_0(row_bytes, x, in_dim),
        GgmlType::Q5KM | GgmlType::Q5KS  => dot_q5k(row_bytes, x, in_dim),
        GgmlType::F32 => row_bytes.chunks_exact(4).take(in_dim)
            .zip(x.iter())
            .map(|(b, &xi)| f32::from_le_bytes([b[0], b[1], b[2], b[3]]) * xi)
            .sum(),
        GgmlType::F16 => row_bytes.chunks_exact(2).take(in_dim)
            .zip(x.iter())
            .map(|(b, &xi)| quant::f16_to_f32(u16::from_le_bytes([b[0], b[1]])) * xi)
            .sum(),
        _ => {
            let row = dequant_dispatch(row_bytes, dtype, in_dim);
            row.iter().zip(x.iter()).map(|(&w, &xi)| w * xi).sum()
        }
    }
}

/// Zero-allocation Q4_K dot product (Q4_K_M and Q4_K_S).
///
/// Processes one 144-byte super-block at a time, accumulating the dot product
/// of the dequantized block with a slice of x[].  No heap allocation.
#[inline]
fn dot_q4k(data: &[u8], x: &[f32], n: usize) -> f32 {
    const QK:    usize = 256;
    const BYTES: usize = 144;
    let mut acc = 0.0f32;
    for b in 0..((n + QK - 1) / QK) {
        let src = b * BYTES;
        if src + BYTES > data.len() { break; }
        let d    = quant::f16_to_f32(u16::from_le_bytes([data[src],     data[src + 1]]));
        let dmin = quant::f16_to_f32(u16::from_le_bytes([data[src + 2], data[src + 3]]));
        if d == 0.0 { continue; }
        let sc = &data[src + 4 .. src + 16];
        let qs = &data[src + 16 .. src + 144];

        // Unpack 8 (scale, min) pairs — same as quant::dequant_q4_k
        let mut scales = [0u8; 8];
        let mut mins   = [0u8; 8];
        for k in 0..4usize {
            scales[k]     = sc[k]     & 63;
            mins[k]       = sc[k + 4] & 63;
            scales[k + 4] = (sc[k + 8] & 0x0F) | ((sc[k]     >> 6) << 4);
            mins[k + 4]   = (sc[k + 8] >> 4)   | ((sc[k + 4] >> 6) << 4);
        }

        let mut q_off = 0usize;
        let mut si    = 0usize;
        let mut base  = b * QK;
        for _ in 0..4 {
            let d1 = d * scales[si]     as f32;  let m1 = dmin * mins[si]     as f32;
            let d2 = d * scales[si + 1] as f32;  let m2 = dmin * mins[si + 1] as f32;
            for l in 0..32 {
                let byte = qs[q_off + l];
                let lo   = (byte & 0x0F) as f32;
                let hi   = (byte >> 4)   as f32;
                let i0 = base + l;
                let i1 = base + 32 + l;
                if i0 < n && i0 < x.len() { acc += (d1 * lo - m1) * x[i0]; }
                if i1 < n && i1 < x.len() { acc += (d2 * hi - m2) * x[i1]; }
            }
            q_off += 32; si += 2; base += 64;
        }
    }
    acc
}

/// Zero-allocation Q6_K dot product.
#[inline]
fn dot_q6k(data: &[u8], x: &[f32], n: usize) -> f32 {
    const QK:    usize = 256;
    const BYTES: usize = 210;
    let mut acc = 0.0f32;
    for b in 0..((n + QK - 1) / QK) {
        let src = b * BYTES;
        if src + BYTES > data.len() { break; }
        let ql_all = &data[src ..       src + 128];
        let qh_all = &data[src + 128 .. src + 192];
        let sc_all = &data[src + 192 .. src + 208];
        let d      = quant::f16_to_f32(u16::from_le_bytes([data[src + 208], data[src + 209]]));
        let base   = b * QK;
        for half in 0..2usize {
            let ql = &ql_all[half * 64..];
            let qh = &qh_all[half * 32..];
            let sc = &sc_all[half *  8..];
            let bh  = base + half * 128;
            for l in 0..32usize {
                let is = l / 16;
                let q1 = ((ql[l]    & 0x0F) | (((qh[l] >> 0) & 3) << 4)) as i32 - 32;
                let q2 = ((ql[l+32] & 0x0F) | (((qh[l] >> 2) & 3) << 4)) as i32 - 32;
                let q3 = ((ql[l]    >>   4) | (((qh[l] >> 4) & 3) << 4)) as i32 - 32;
                let q4 = ((ql[l+32] >>   4) | (((qh[l] >> 6) & 3) << 4)) as i32 - 32;
                let s1 = d * (sc[is    ] as i8) as f32;
                let s2 = d * (sc[is + 2] as i8) as f32;
                let s3 = d * (sc[is + 4] as i8) as f32;
                let s4 = d * (sc[is + 6] as i8) as f32;
                let (i0,i1,i2,i3) = (bh+l, bh+l+32, bh+l+64, bh+l+96);
                if i0 < n && i0 < x.len() { acc += s1 * q1 as f32 * x[i0]; }
                if i1 < n && i1 < x.len() { acc += s2 * q2 as f32 * x[i1]; }
                if i2 < n && i2 < x.len() { acc += s3 * q3 as f32 * x[i2]; }
                if i3 < n && i3 < x.len() { acc += s4 * q4 as f32 * x[i3]; }
            }
        }
    }
    acc
}

/// Zero-allocation Q8_0 dot product.
#[inline]
fn dot_q8_0(data: &[u8], x: &[f32], n: usize) -> f32 {
    const BLOCK: usize = 34;
    const EPB:   usize = 32;
    let mut acc = 0.0f32;
    for (b, block) in data.chunks_exact(BLOCK).enumerate() {
        let scale = quant::f16_to_f32(u16::from_le_bytes([block[0], block[1]]));
        for i in 0..EPB {
            let idx = b * EPB + i;
            if idx >= n || idx >= x.len() { break; }
            acc += (block[2 + i] as i8) as f32 * scale * x[idx];
        }
    }
    acc
}

/// Zero-allocation Q5_K dot product.
#[inline]
fn dot_q5k(data: &[u8], x: &[f32], n: usize) -> f32 {
    const QK:    usize = 256;
    const BYTES: usize = 176;
    let mut acc = 0.0f32;
    for b in 0..((n + QK - 1) / QK) {
        let src = b * BYTES;
        if src + BYTES > data.len() { break; }
        let d    = quant::f16_to_f32(u16::from_le_bytes([data[src],     data[src + 1]]));
        let dmin = quant::f16_to_f32(u16::from_le_bytes([data[src + 2], data[src + 3]]));
        if d == 0.0 { continue; }
        let sc  = &data[src + 4  .. src + 16];
        let qh  = &data[src + 16 .. src + 48];
        let qs  = &data[src + 48 .. src + 176];
        let mut scales = [0u8; 8]; let mut mins = [0u8; 8];
        for k in 0..4usize {
            scales[k]     = sc[k]     & 63;
            mins[k]       = sc[k + 4] & 63;
            scales[k + 4] = (sc[k + 8] & 0x0F) | ((sc[k]     >> 6) << 4);
            mins[k + 4]   = (sc[k + 8] >> 4)   | ((sc[k + 4] >> 6) << 4);
        }
        let mut q_off = 0usize; let mut si = 0usize; let mut oi = b * QK;
        for round in 0..4u8 {
            let u1: u8 = 1 << (round * 2);
            let u2: u8 = 1 << (round * 2 + 1);
            let d1 = d * scales[si] as f32;     let m1 = dmin * mins[si] as f32;
            let d2 = d * scales[si+1] as f32;   let m2 = dmin * mins[si+1] as f32;
            for l in 0..32usize {
                let h  = qh[l];
                let q0 = (qs[q_off + l] & 0x0F) as f32 + if h & u1 != 0 { 16.0 } else { 0.0 };
                let q1 = (qs[q_off + l] >>    4) as f32 + if h & u2 != 0 { 16.0 } else { 0.0 };
                let i0 = oi + l;
                let i1 = oi + 32 + l;
                if i0 < n && i0 < x.len() { acc += (d1 * q0 - m1) * x[i0]; }
                if i1 < n && i1 < x.len() { acc += (d2 * q1 - m2) * x[i1]; }
            }
            q_off += 32; si += 2; oi += 64;
        }
    }
    acc
}

/// Dispatch raw quantized bytes to the appropriate dequant function from quant.rs.
fn dequant_dispatch(data: &[u8], dtype: GgmlType, n_elem: usize) -> Vec<f32> {
    match dtype {
        GgmlType::F32  => data.chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect(),
        GgmlType::F16  => data.chunks_exact(2)
            .map(|b| quant::f16_to_f32(u16::from_le_bytes([b[0], b[1]])))
            .collect(),
        GgmlType::BF16              => quant::dequant_bf16(data, n_elem),
        GgmlType::Q8_0              => quant::dequant_q8_0(data, n_elem),
        GgmlType::Q4_0              => quant::dequant_q4_0(data, n_elem),
        GgmlType::Q4_1              => quant::dequant_q4_1(data, n_elem),
        GgmlType::Q5_0              => quant::dequant_q5_0(data, n_elem),
        GgmlType::Q5_1              => quant::dequant_q5_1(data, n_elem),
        GgmlType::Q2K               => quant::dequant_q2_k(data, n_elem),
        GgmlType::Q3KS | GgmlType::Q3KM | GgmlType::Q3KL
                                    => quant::dequant_q3_k(data, n_elem),
        GgmlType::Q4KS | GgmlType::Q4KM
                                    => quant::dequant_q4_k(data, n_elem),
        GgmlType::Q5KS | GgmlType::Q5KM
                                    => quant::dequant_q5_k(data, n_elem),
        GgmlType::Q6K               => quant::dequant_q6_k(data, n_elem),
        GgmlType::Q8K               => quant::dequant_q8_k(data, n_elem),
        GgmlType::IQ4NL | GgmlType::IQ4_NL_4_4 | GgmlType::IQ4_NL_4_8
        | GgmlType::Q4_0_4x4 | GgmlType::Q4_0_4x8 | GgmlType::Q4_0_8x8
                                    => quant::dequant_iq4_nl(data, n_elem),
        GgmlType::IQ4XS             => quant::dequant_iq4_xs(data, n_elem),
        GgmlType::IQ3XXS            => quant::dequant_iq3_xxs(data, n_elem),
        GgmlType::IQ3S              => quant::dequant_iq3_s(data, n_elem),
        GgmlType::IQ2XXS            => quant::dequant_iq2_xxs(data, n_elem),
        GgmlType::IQ2XS             => quant::dequant_iq2_xs(data, n_elem),
        GgmlType::IQ2S              => quant::dequant_iq2_s(data, n_elem),
        GgmlType::IQ1S              => quant::dequant_iq1_s(data, n_elem),
        GgmlType::IQ1M              => quant::dequant_iq1_m(data, n_elem),
        GgmlType::TQ1_0             => quant::dequant_tq1_0(data, n_elem),
        GgmlType::TQ2_0             => quant::dequant_tq2_0(data, n_elem),
        _                           => vec![0.0; n_elem],
    }
}

// --- KV Cache (stores past keys & values for fast generation) -----------------
//
// `Q8` mode: int8 tensors + per-position scales (one scale for all K head dims at that step, one for V).
// Cuts KV footprint ~4× vs f32 — same practical goal as “TurboQuant-style” KV compression (see Google
// Research TurboQuant blog); this engine does not implement their random-rotation / PolarQuant stack.

enum KVCells {
    F32 { k: Vec<f32>, v: Vec<f32> },
    Q8 {
        k: Vec<i8>,
        v: Vec<i8>,
        scale_k: Vec<f32>,
        scale_v: Vec<f32>,
    },
}

struct KVLayer {
    stride: usize,
    cells:  KVCells,
}

impl KVLayer {
    fn new(ctx_len: usize, n_kv_heads: usize, head_dim: usize, q8: bool) -> Self {
        let stride = n_kv_heads * head_dim;
        let size = ctx_len * stride;
        let cells = if q8 {
            KVCells::Q8 {
                k:       vec![0i8; size],
                v:       vec![0i8; size],
                scale_k: vec![0.0; ctx_len],
                scale_v: vec![0.0; ctx_len],
            }
        } else {
            KVCells::F32 {
                k: vec![0.0; size],
                v: vec![0.0; size],
            }
        };
        KVLayer { stride, cells }
    }

    fn write_pos(&mut self, pos: usize, k_src: &[f32], v_src: &[f32], kv_dim: usize) {
        let base = pos * self.stride;
        match &mut self.cells {
            KVCells::F32 { k, v } => {
                k[base..base + kv_dim].copy_from_slice(k_src);
                v[base..base + kv_dim].copy_from_slice(v_src);
            }
            KVCells::Q8 {
                k: kq,
                v: vq,
                scale_k,
                scale_v,
            } => {
                let mut mk = 0.0f32;
                let mut mv = 0.0f32;
                for &x in k_src {
                    mk = mk.max(x.abs());
                }
                for &x in v_src {
                    mv = mv.max(x.abs());
                }
                let sk = (mk / 127.0).max(1e-8);
                let sv = (mv / 127.0).max(1e-8);
                scale_k[pos] = sk;
                scale_v[pos] = sv;
                for i in 0..kv_dim {
                    kq[base + i] = (k_src[i] / sk).round().clamp(-127.0, 127.0) as i8;
                    vq[base + i] = (v_src[i] / sv).round().clamp(-127.0, 127.0) as i8;
                }
            }
        }
    }

    fn kv_cap(&self) -> usize {
        match &self.cells {
            KVCells::F32 { k, .. } => k.len() / self.stride.max(1),
            KVCells::Q8 { k, .. } => k.len() / self.stride.max(1),
        }
    }
}

// --- Inference engine ---------------------------------------------------------

pub struct KillerInference {
    pub config:    ModelConfig,
    pub tokenizer: KillerTokenizer,
    weights:       HashMap<String, Vec<f32>>,    // small tensors (norms, biases): always f32
    weights_q:     HashMap<String, QuantTensor>, // large tensors: stay quantized, dequant per-row
}

impl KillerInference {
    // -- Loading ---------------------------------------------------------------

    /// Load a model from a `.gguf` file — the only thing needed to start.
    ///
    /// Compatible models (free download from HuggingFace):
    ///   • TinyLlama-1.1B-Chat-v1.0.Q8_0.gguf   (~1.2 GB)
    ///   • Phi-3-mini-4k-instruct-q4.gguf        (~2.2 GB)
    ///   • Mistral-7B-Instruct-v0.3.Q4_K_M.gguf  (~4.4 GB)
    pub fn load(path: &str) -> Result<Self, String> {
        eprintln!("[Killer] Loading: {}", path);
        let gguf      = GgufFile::open(path)?;
        let mut config = ModelConfig::from_gguf(&gguf)?;
        let tokenizer  = KillerTokenizer::from_gguf(&gguf)?;

        eprintln!("[Killer] {:?} | layers={} heads={} kv_heads={} embd={} vocab={} rope_theta={} n_ff={} ctx_len={}",
            config.arch, config.n_layers, config.n_heads, config.n_kv_heads, config.n_embd, config.vocab_size, config.rope_theta, config.n_ff, config.ctx_len);

        // Threshold: tensors with >= 100K elements stay quantized (saves ~6× RAM for 7B models).
        // Norms (n_embd), biases (n_heads×head_dim) are tiny — dequant immediately to f32.
        const QUANT_THRESHOLD: usize = 100_000;

        let mut weights   = HashMap::new();
        let mut weights_q = HashMap::new();
        let names: Vec<String> = gguf.tensor_names().iter().map(|s| s.to_string()).collect();
        eprintln!("[Killer] Loading {} tensors (large ones stay quantized)...", names.len());

        for name in &names {
            let info = match gguf.tensors.get(name.as_str()) {
                Some(i) => i,
                None    => continue,
            };
            let n_elem = info.n_elements() as usize;

            if n_elem >= QUANT_THRESHOLD {
                // Keep in native quantized form
                match gguf.load_tensor_raw(name) {
                    Ok((data, dtype, shape)) => {
                        weights_q.insert(name.clone(), QuantTensor::from_raw(data, dtype, &shape));
                    }
                    Err(e) => eprintln!("[Killer] skip '{}': {}", name, e),
                }
            } else {
                // Small tensor — dequant to f32 once
                match gguf.load_tensor_f32(name) {
                    Ok(data)  => { weights.insert(name.clone(), data); }
                    Err(e)    => eprintln!("[Killer] skip '{}': {}", name, e),
                }
            }
        }
        eprintln!("[Killer] Loaded {} dense + {} quantized tensors.",
                  weights.len(), weights_q.len());

        // Fix vocab_size: derive from actual weight dimensions
        let embed_cols = weights_q.get("token_embd.weight").map(|q| q.rows)
            .or_else(|| weights_q.get("output.weight").map(|q| q.rows));
        if let Some(n) = embed_cols {
            if n > config.vocab_size {
                eprintln!("[Killer] vocab_size corrected {} -> {} (from weights)", config.vocab_size, n);
                config.vocab_size = n;
            }
        } else {
            // Fallback: check f32 weights (tiny models only)
            let embed_w = weights.get("token_embd.weight")
                .or_else(|| weights.get("output.weight"));
            if let Some(w) = embed_w {
                let n = w.len() / config.n_embd.max(1);
                if n > config.vocab_size {
                    eprintln!("[Killer] vocab_size corrected {} -> {} (from f32 weights)", config.vocab_size, n);
                    config.vocab_size = n;
                }
            }
        }
        // Sync tokenizer vocab_size too
        if tokenizer.vocab_size > config.vocab_size {
            config.vocab_size = tokenizer.vocab_size;
        }

        Ok(KillerInference { config, tokenizer, weights, weights_q })
    }

    // -- Public API ------------------------------------------------------------

    /// Ask a question, get an answer back.
    pub fn ask(&mut self, question: &str, max_tokens: usize) -> String {
        self.generate(question, &GenerateConfig::greedy(max_tokens))
    }

    /// Full-control generation with a `GenerateConfig`.
    pub fn generate(&mut self, prompt: &str, cfg: &GenerateConfig) -> String {
        let tokens = self.tokenizer.encode(prompt, true);
        eprintln!("[Killer] Prompt: {} tokens", tokens.len());

        // Cap KV cache to the tokens we will actually produce — NOT the model's full
        // ctx_len (e.g. 131072 for Qwen2.5 = ~14 GB of KV cache for a 7B model).
        // Typical short inference: prompt + a few thousand tokens = a few hundred MB max.
        let kv_len = (tokens.len() + cfg.max_new_tokens + 16)
            .min(self.config.ctx_len)
            .min(4096); // hard cap: 4K context = ~256 MB KV cache for a 7B model

        let kv_q8 = cfg.kv_q8
            || std::env::var("KILLER_KV_Q8")
                .map(|v| {
                    matches!(
                        v.to_lowercase().as_str(),
                        "1" | "true" | "yes" | "on"
                    )
                })
                .unwrap_or(false);
        if kv_q8 {
            eprintln!(
                "[Killer] KV cache: int8 + per-step scales (TurboQuant-style memory saver; ~4× smaller KV vs f32)"
            );
        }

        let mut kv: Vec<KVLayer> = (0..self.config.n_layers)
            .map(|_| {
                KVLayer::new(
                    kv_len,
                    self.config.n_kv_heads,
                    self.config.head_dim,
                    kv_q8,
                )
            })
            .collect();

        let mut pos = 0usize;

        // Feed the prompt tokens (build KV cache, discard interior logits)
        for &token in tokens.iter().take(tokens.len().saturating_sub(1)) {
            self.forward(token, pos, &mut kv);
            pos += 1;
        }

        // Last prompt token → first logits we care about
        let mut last_token = *tokens.last().unwrap_or(&self.tokenizer.bos_id);
        let mut generated  = Vec::new();
        // Sliding window of recent tokens for repetition penalty (last 64 tokens)
        let mut recent: Vec<u32> = Vec::with_capacity(64);
        let mut _first_step_done = false;

        for step in 0..cfg.max_new_tokens {
            let logits = self.forward(last_token, pos, &mut kv);

            // Debug: first step — show top-5 logits and their range
            if step == 0 {
                let max_l = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let min_l = logits.iter().cloned().fold(f32::INFINITY, f32::min);
                let mut top: Vec<(f32, u32)> = logits.iter().enumerate()
                    .map(|(i, &v)| (v, i as u32)).collect();
                top.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
                let top5: Vec<String> = top[..5.min(top.len())].iter()
                    .map(|(v, id)| format!("{}({:.1})", self.tokenizer.token_str(*id), v))
                    .collect();
                eprintln!("[dbg] logits range=[{:.1},{:.1}] top5=[{}]", min_l, max_l, top5.join(", "));
            }

            let next_tok = sample_token(&logits, &cfg.sampling, step, &recent);

            if self.tokenizer.is_eos(next_tok) { break; }
            if cfg.stop_tokens.contains(&next_tok) { break; }
            if pos + 1 >= self.config.ctx_len { break; }

            let tok_str = self.tokenizer.token_str(next_tok);
            eprint!("{}", tok_str);
            // Flush stderr so tokens appear in real-time even when piped to a file
            let _ = std::io::Write::flush(&mut std::io::stderr());

            generated.push(next_tok);
            // Track recent tokens for repetition penalty (sliding window)
            if recent.len() >= 64 { recent.remove(0); }
            recent.push(next_tok);
            last_token = next_tok;
            pos += 1;
        }
        eprintln!();

        self.tokenizer.decode(&generated)
    }

    // -- Text embedding (for RAG / semantic search) ----------------------------

    /// Encode text as a mean-pooled, L2-normalized dense vector.
    ///
    /// Use a dedicated embedding GGUF model (e.g. nomic-embed-text, all-MiniLM)
    /// for best quality, but any LLaMA-family model produces useful vectors.
    /// Output length == `config.n_embd`.  Vectors are L2-normalized, so
    /// cosine similarity = plain dot product.
    pub fn embed_text(&mut self, text: &str) -> Vec<f32> {
        let tokens = self.tokenizer.encode(text, true);
        if tokens.is_empty() { return vec![0.0; self.config.n_embd]; }

        let ctx_limit = self.config.ctx_len.min(512);
        // Embeddings need full-precision KV for stable pooled vectors.
        let mut kv: Vec<KVLayer> = (0..self.config.n_layers)
            .map(|_| KVLayer::new(ctx_limit, self.config.n_kv_heads, self.config.head_dim, false))
            .collect();

        let mut pooled = vec![0.0f32; self.config.n_embd];
        let n = tokens.len().min(ctx_limit);

        for (pos, &tok) in tokens.iter().take(n).enumerate() {
            let h = self.forward_hidden(tok, pos, &mut kv);
            for (pi, &hi) in pooled.iter_mut().zip(h.iter()) { *pi += hi; }
        }

        // Mean pool
        for v in &mut pooled { *v /= n as f32; }

        // L2 normalize so cosine similarity = dot product
        let norm: f32 = pooled.iter().map(|&v| v * v).sum::<f32>().sqrt();
        if norm > 1e-8 { for v in &mut pooled { *v /= norm; } }
        pooled
    }

    /// Forward pass returning last-layer hidden state (before lm_head).
    /// Used by `embed_text()` to extract semantic representations.
    fn forward_hidden(&self, token: u32, pos: usize, kv: &mut Vec<KVLayer>) -> Vec<f32> {
        let mut x = self.embed(token);
        for layer in 0..self.config.n_layers {
            x = self.layer(&x, pos, layer, kv);
        }
        let norm_w = self.weights.get("output_norm.weight").map(|v| v.as_slice());
        rms_norm(&x, norm_w, self.config.norm_eps)
    }

    // -- Transformer forward pass ----------------------------------------------

    fn forward(&self, token: u32, pos: usize, kv: &mut Vec<KVLayer>) -> Vec<f32> {
        // 1. Embedding lookup
        let mut x = self.embed(token);

        // 2. N transformer layers
        for layer in 0..self.config.n_layers {
            x = self.layer(&x, pos, layer, kv);
        }

        // 3. Final RMS norm
        let norm_w = self.weights.get("output_norm.weight").map(|v| v.as_slice());
        let x = rms_norm(&x, norm_w, self.config.norm_eps);

        // 4. Unembedding (lm_head): logits over vocabulary
        self.lm_head(&x)
    }

    fn embed(&self, token: u32) -> Vec<f32> {
        let d  = self.config.n_embd;
        let id = token as usize;
        // Quantized embedding table: dequant just the one row (token's embedding)
        if let Some(q) = self.weights_q.get("token_embd.weight") {
            return q.dequant_row(id, d);
        }
        if let Some(w) = self.weights.get("token_embd.weight") {
            let s = id * d;
            if s + d <= w.len() { return w[s..s + d].to_vec(); }
        }
        vec![0.0; d]
    }

    fn lm_head(&self, x: &[f32]) -> Vec<f32> {
        let vocab = self.config.vocab_size;
        let embd  = self.config.n_embd;
        if let Some(q) = self.weights_q.get("output.weight") {
            return q.matvec(x, vocab, embd);
        }
        if let Some(w) = self.weights.get("output.weight") {
            return matmul(x, w, vocab, embd);
        }
        // Weight tying: use embedding table as lm_head
        if let Some(q) = self.weights_q.get("token_embd.weight") {
            return q.matvec(x, vocab, embd);
        }
        if let Some(w) = self.weights.get("token_embd.weight") {
            return matmul(x, w, vocab, embd);
        }
        vec![0.0; vocab]
    }

    // -- Single transformer layer ----------------------------------------------

    fn layer(&self, x: &[f32], pos: usize, layer: usize, kv: &mut Vec<KVLayer>) -> Vec<f32> {
        // Attention sublayer
        let attn_norm_w = self.weights.get(&format!("blk.{}.attn_norm.weight", layer));
        let x_normed    = rms_norm(x, attn_norm_w.map(|v| v.as_slice()), self.config.norm_eps);
        let attn_out    = self.attention(&x_normed, pos, layer, kv);
        let x           = add(x, &attn_out);

        // FFN sublayer
        let ffn_norm_w = self.weights.get(&format!("blk.{}.ffn_norm.weight", layer));
        let x_normed2  = rms_norm(&x, ffn_norm_w.map(|v| v.as_slice()), self.config.norm_eps);
        let ffn_out    = self.ffn(&x_normed2, layer);

        // Surgical FFN explosion guard — clamp only ffn_out (before residual add).
        //
        // Root cause: Qwen2.5 layer 2, pos=0 (<|im_start|>) produces ffn_out L2=814
        // instead of the normal ~3-27. Clamping the FULL layer output (x + ffn_out)
        // distorts all later layers because the residual x (~L2=15) gets squashed too.
        //
        // Correct approach: clamp ffn_out alone to the same scale as x (the residual).
        // Normal FFN output is 0.3-3× the residual. The explosion is 50× the residual.
        // Clamping ffn_out to 3× residual_L2 surgically removes the spike, leaving
        // the residual stream untouched and letting later layers grow naturally.
        let x_l2      = x.iter().map(|&v| v * v).sum::<f32>().sqrt().max(1.0);
        let ffn_l2    = ffn_out.iter().map(|&v| v * v).sum::<f32>().sqrt();
        let ffn_ceil  = x_l2 * 3.0;   // normal FFN contribution is ≤3× residual
        let ffn_out   = if ffn_l2 > ffn_ceil {
            let scale = ffn_ceil / ffn_l2;
            ffn_out.into_iter().map(|v| v * scale).collect::<Vec<_>>()
        } else {
            ffn_out
        };

        add(&x, &ffn_out)
    }

    // -- Self-attention with RoPE + GQA + KV cache + optional QKV bias ---------

    fn attention(&self, x: &[f32], pos: usize, layer: usize, kv: &mut Vec<KVLayer>) -> Vec<f32> {
        let cfg      = &self.config;
        let hd       = cfg.head_dim;
        let q_dim    = cfg.n_heads    * hd;
        let kv_dim   = cfg.n_kv_heads * hd;
        let kv_layer = &mut kv[layer];
        let use_bias = cfg.arch.has_qkv_bias();

        // Q K V projections (+ bias when Qwen2 / Phi)
        let q = self.proj_maybe_bias(x, &format!("blk.{}.attn_q.weight",      layer),
                                         if use_bias { Some(format!("blk.{}.attn_q.bias", layer)) } else { None },
                                         q_dim,  cfg.n_embd);
        let k = self.proj_maybe_bias(x, &format!("blk.{}.attn_k.weight",      layer),
                                         if use_bias { Some(format!("blk.{}.attn_k.bias", layer)) } else { None },
                                         kv_dim, cfg.n_embd);
        let v = self.proj_maybe_bias(x, &format!("blk.{}.attn_v.weight",      layer),
                                         if use_bias { Some(format!("blk.{}.attn_v.bias", layer)) } else { None },
                                         kv_dim, cfg.n_embd);

        // Apply RoPE positional encoding to Q and K
        let q = rope(&q, pos, cfg.n_heads,    hd, cfg.rope_theta);
        let k = rope(&k, pos, cfg.n_kv_heads, hd, cfg.rope_theta);

        // Write current K/V into cache (guard against pos >= kv_cap)
        let stride = cfg.n_kv_heads * hd;
        let kv_cap = kv_layer.kv_cap();
        if pos < kv_cap {
            kv_layer.write_pos(pos, &k, &v, kv_dim);
        }

        // Compute attention output for each query head
        let scale   = 1.0 / (hd as f32).sqrt();
        // Use actual KV cache size (not config.ctx_len which may be 131072)
        let ctx     = (pos + 1).min(kv_cap);
        let mut out = vec![0.0f32; q_dim];

        for h in 0..cfg.n_heads {
            // GQA: map query head → kv head
            let kv_h = h * cfg.n_kv_heads / cfg.n_heads;
            let q_h  = &q[h * hd..(h + 1) * hd];

            // Attention scores: Q·Kᵀ / √d_k
            let mut scores = Vec::with_capacity(ctx);
            match &kv_layer.cells {
                KVCells::F32 { k: kb, .. } => {
                    for p in 0..ctx {
                        let k_h = &kb[p * stride + kv_h * hd..p * stride + kv_h * hd + hd];
                        scores.push(dot(q_h, k_h) * scale);
                    }
                }
                KVCells::Q8 {
                    k: kq,
                    scale_k,
                    ..
                } => {
                    let skp = scale_k;
                    for p in 0..ctx {
                        let base = p * stride + kv_h * hd;
                        let s = skp[p];
                        let mut acc = 0.0f32;
                        for d in 0..hd {
                            acc += q_h[d] * (kq[base + d] as f32 * s);
                        }
                        scores.push(acc * scale);
                    }
                }
            }

            let w = softmax(&scores);

            // Weighted sum over values
            match &kv_layer.cells {
                KVCells::F32 { v: vb, .. } => {
                    for p in 0..ctx {
                        let v_h = &vb[p * stride + kv_h * hd..p * stride + kv_h * hd + hd];
                        for d in 0..hd {
                            out[h * hd + d] += w[p] * v_h[d];
                        }
                    }
                }
                KVCells::Q8 {
                    v: vq,
                    scale_v,
                    ..
                } => {
                    for p in 0..ctx {
                        let base = p * stride + kv_h * hd;
                        let sv = scale_v[p];
                        for d in 0..hd {
                            out[h * hd + d] += w[p] * (vq[base + d] as f32 * sv);
                        }
                    }
                }
            }
        }

        // Output projection
        self.proj(&out, &format!("blk.{}.attn_output.weight", layer), cfg.n_embd, q_dim)
    }

    // -- SwiGLU Feed-Forward Network -------------------------------------------

    fn ffn(&self, x: &[f32], layer: usize) -> Vec<f32> {
        let cfg = &self.config;

        // Gate and up projections (checked in weights_q first via self.proj)
        let gate = self.proj(x, &format!("blk.{}.ffn_gate.weight", layer), cfg.n_ff, cfg.n_embd);
        let up   = self.proj(x, &format!("blk.{}.ffn_up.weight",   layer), cfg.n_ff, cfg.n_embd);

        // SwiGLU activation: silu(gate) ⊙ up
        let hidden: Vec<f32> = gate.iter().zip(up.iter())
            .map(|(&g, &u)| silu(g) * u)
            .collect();

        // Down projection — MUST use self.proj() so weights_q is checked.
        // For a 7B model ffn_down is ~68M elements → lives in weights_q, not weights.
        // Previously this used self.weights.get() which always returned None for large
        // tensors → FFN was broken → near-uniform logits → incoherent output.
        self.proj(&hidden, &format!("blk.{}.ffn_down.weight", layer), cfg.n_embd, cfg.n_ff)
    }

    // -- Weight projection helper ----------------------------------------------

    fn proj(&self, x: &[f32], name: &str, out_dim: usize, in_dim: usize) -> Vec<f32> {
        if let Some(q) = self.weights_q.get(name) {
            return q.matvec(x, out_dim, in_dim);
        }
        match self.weights.get(name) {
            Some(w) => matmul(x, w, out_dim, in_dim),
            None    => vec![0.0; out_dim],
        }
    }

    /// Projection with an optional bias vector (used by Qwen2 / Phi for QKV).
    fn proj_maybe_bias(&self, x: &[f32], weight: &str, bias: Option<String>,
                       out_dim: usize, in_dim: usize) -> Vec<f32> {
        let mut y = self.proj(x, weight, out_dim, in_dim);
        if let Some(bname) = bias {
            if let Some(bv) = self.weights.get(&bname) {
                for (yi, &bi) in y.iter_mut().zip(bv.iter()) { *yi += bi; }
            }
        }
        y
    }
}

// --- Math primitives (zero-dependency) ---------------------------------------

/// RMS Normalization: x / rms(x) * weight
fn rms_norm(x: &[f32], weight: Option<&[f32]>, eps: f32) -> Vec<f32> {
    let rms = (x.iter().map(|&v| v * v).sum::<f32>() / x.len() as f32 + eps).sqrt();
    match weight {
        Some(w) => x.iter().zip(w.iter()).map(|(&xi, &wi)| xi / rms * wi).collect(),
        None    => x.iter().map(|&xi| xi / rms).collect(),
    }
}

/// Dense matrix-vector multiply: y[r] = Σ W[r,c] * x[c]
/// W is stored row-major: W[r,c] = w[r * in_dim + c]
fn matmul(x: &[f32], w: &[f32], out_dim: usize, in_dim: usize) -> Vec<f32> {
    let mut y = vec![0.0f32; out_dim];
    let in_safe = in_dim.min(x.len());
    for r in 0..out_dim {
        let row_start = r * in_dim;
        let row_end   = (row_start + in_safe).min(w.len());
        if row_start >= w.len() { break; }
        y[r] = x[..in_safe].iter().zip(&w[row_start..row_end]).map(|(&a, &b)| a * b).sum();
    }
    y
}

/// Transposed matrix-vector multiply: y[r] = Σ W[c, r] * x[c]
/// Used for weight-tied output (embedding table reused as lm_head ᵀ).
#[allow(dead_code)]
fn matmul_t(x: &[f32], w: &[f32], out_dim: usize, in_dim: usize) -> Vec<f32> {
    let mut y = vec![0.0f32; out_dim];
    for c in 0..in_dim.min(x.len()) {
        let xc = x[c];
        for r in 0..out_dim {
            let idx = c * out_dim + r;
            if idx < w.len() { y[r] += xc * w[idx]; }
        }
    }
    y
}

/// Dot product.
fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(&ai, &bi)| ai * bi).sum()
}

/// Element-wise addition (broadcast shorter to length of longer).
fn add(a: &[f32], b: &[f32]) -> Vec<f32> {
    a.iter().zip(b.iter()).map(|(&ai, &bi)| ai + bi).collect()
}

/// Numerically stable softmax.
fn softmax(x: &[f32]) -> Vec<f32> {
    let max  = x.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let exp: Vec<f32> = x.iter().map(|&v| (v - max).exp()).collect();
    let sum: f32 = exp.iter().sum();
    if sum > 0.0 { exp.iter().map(|&e| e / sum).collect() }
    else          { vec![1.0 / x.len() as f32; x.len()] }
}

/// SiLU activation: x * σ(x)
fn silu(x: f32) -> f32 { x / (1.0 + (-x).exp()) }

/// Rotary Position Embedding (RoPE) — in-place rotate pairs (x0, x1) per head.
fn rope(x: &[f32], pos: usize, n_heads: usize, head_dim: usize, theta: f32) -> Vec<f32> {
    let mut out = x.to_vec();
    for h in 0..n_heads {
        for i in 0..head_dim / 2 {
            let freq  = 1.0 / theta.powf(2.0 * i as f32 / head_dim as f32);
            let angle = pos as f32 * freq;
            let (sin, cos) = angle.sin_cos();
            let base = h * head_dim + i * 2;
            if base + 1 < out.len() {
                let (x0, x1) = (out[base], out[base + 1]);
                out[base]     = x0 * cos - x1 * sin;
                out[base + 1] = x0 * sin + x1 * cos;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rms_norm_unit() {
        let x = vec![3.0f32, 4.0];
        let normed = rms_norm(&x, None, 1e-6);
        // rms([3,4]) = sqrt((9+16)/2) = sqrt(12.5) ≈ 3.536
        // normed ≈ [0.849, 1.131]
        assert!((normed[0] - 0.849).abs() < 0.01, "rms[0]={}", normed[0]);
    }

    #[test]
    fn test_softmax_sum_to_one() {
        let x = vec![1.0f32, 2.0, 3.0];
        let s = softmax(&x);
        let sum: f32 = s.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5, "softmax sum = {}", sum);
    }

    #[test]
    fn test_softmax_argmax_preserved() {
        let x = vec![0.1f32, 5.0, 0.3];
        let s = softmax(&x);
        let max_idx = s.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()).map(|(i,_)| i).unwrap();
        assert_eq!(max_idx, 1);
    }

    #[test]
    fn test_matmul_identity() {
        // 2×2 identity matrix
        let w = vec![1.0f32, 0.0,  0.0, 1.0];
        let x = vec![3.0f32, 5.0];
        let y = matmul(&x, &w, 2, 2);
        assert!((y[0] - 3.0).abs() < 1e-6);
        assert!((y[1] - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_silu_at_zero() {
        assert!((silu(0.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_rope_output_length() {
        let x = vec![1.0f32; 64];  // 2 heads × 32 head_dim
        let out = rope(&x, 5, 2, 32, 10000.0);
        assert_eq!(out.len(), x.len());
    }

    #[test]
    fn test_add() {
        let a = vec![1.0f32, 2.0];
        let b = vec![3.0f32, 4.0];
        let c = add(&a, &b);
        assert_eq!(c, vec![4.0, 6.0]);
    }
}
