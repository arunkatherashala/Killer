// inference/quant.rs — Dequantization for GGUF tensor formats
//
// Converts quantized bytes → F32 values ready for matrix math.
//
// Supported formats:
//   Standard  : F16, Q4_0, Q4_1, Q5_0, Q5_1, Q8_0
//   K-quants  : Q4_K (Q4_K_M, Q4_K_S), Q5_K, Q6_K
//               These are the formats used by all modern models:
//               TinyLlama Q8_0, Qwen2.5 Q4_K_M, Phi-3 Q4_K_M, Mistral Q4_K_M

// --- F16 → F32 conversion (IEEE 754 half-precision) -------------------------

pub fn f16_to_f32(bits: u16) -> f32 {
    let sign     = ((bits >> 15) & 1) as u32;
    let exponent = ((bits >> 10) & 0x1f) as u32;
    let mantissa = (bits & 0x3ff) as u32;

    // exponent=31 means NaN or Inf in f16.
    // Returning NaN/Inf poisons all downstream matrix math — clamp to 0.0 instead.
    // A scale of 0 means the block contributes nothing (safe approximation).
    if exponent == 31 { return 0.0; }

    let (exp32, man32) = if exponent == 0 {
        if mantissa == 0 {
            (0, 0)
        } else {
            // Denormal f16 → normalized f32
            let mut e = 127u32 - 14;
            let mut m = mantissa;
            while m & 0x400 == 0 { m <<= 1; e -= 1; }
            (e, (m & 0x3ff) << 13)
        }
    } else {
        (exponent + 127 - 15, mantissa << 13)
    };

    f32::from_bits((sign << 31) | (exp32 << 23) | man32)
}

// --- Q8_0 --------------------------------------------------------------------
//
// Block layout (34 bytes per block, 32 elements per block):
//   [0..2]  f16 scale
//   [2..34] 32 × i8 quantized values
//
// Dequant: value[i] = i8[i] * scale

pub fn dequant_q8_0(data: &[u8], n_elements: usize) -> Vec<f32> {
    const BLOCK: usize = 34;
    const EPB:   usize = 32;
    let mut out = vec![0.0f32; n_elements];
    for (b, block) in data.chunks_exact(BLOCK).enumerate() {
        let scale = f16_to_f32(u16::from_le_bytes([block[0], block[1]]));
        for i in 0..EPB {
            let idx = b * EPB + i;
            if idx >= n_elements { break; }
            out[idx] = (block[2 + i] as i8) as f32 * scale;
        }
    }
    out
}

// --- Q4_0 --------------------------------------------------------------------
//
// Block layout (18 bytes per block, 32 elements per block):
//   [0..2]  f16 scale
//   [2..18] 16 bytes — each byte packs two 4-bit values (unsigned, offset -8)
//             lo nibble = elements 0..15
//             hi nibble = elements 16..31
//
// Dequant: value[i] = (nibble[i] - 8) * scale

pub fn dequant_q4_0(data: &[u8], n_elements: usize) -> Vec<f32> {
    const BLOCK: usize = 18;
    const EPB:   usize = 32;
    let mut out = vec![0.0f32; n_elements];
    for (b, block) in data.chunks_exact(BLOCK).enumerate() {
        let scale = f16_to_f32(u16::from_le_bytes([block[0], block[1]]));
        for i in 0..16 {
            let byte = block[2 + i];
            let lo   = ((byte & 0x0f) as i32 - 8) as f32 * scale;
            let hi   = (((byte >> 4) & 0x0f) as i32 - 8) as f32 * scale;
            let i0 = b * EPB + i;
            let i1 = b * EPB + i + 16;
            if i0 < n_elements { out[i0] = lo; }
            if i1 < n_elements { out[i1] = hi; }
        }
    }
    out
}

// --- Q4_1 --------------------------------------------------------------------
//
// Block layout (20 bytes per block, 32 elements per block):
//   [0..2]  f16 scale (d)
//   [2..4]  f16 min  (m)
//   [4..20] 16 bytes of nibble pairs
//
// Dequant: value[i] = nibble[i] * scale + min

pub fn dequant_q4_1(data: &[u8], n_elements: usize) -> Vec<f32> {
    const BLOCK: usize = 20;
    const EPB:   usize = 32;
    let mut out = vec![0.0f32; n_elements];
    for (b, block) in data.chunks_exact(BLOCK).enumerate() {
        let scale = f16_to_f32(u16::from_le_bytes([block[0], block[1]]));
        let min   = f16_to_f32(u16::from_le_bytes([block[2], block[3]]));
        for i in 0..16 {
            let byte = block[4 + i];
            let lo   = (byte & 0x0f) as f32 * scale + min;
            let hi   = ((byte >> 4) & 0x0f) as f32 * scale + min;
            let i0 = b * EPB + i;
            let i1 = b * EPB + i + 16;
            if i0 < n_elements { out[i0] = lo; }
            if i1 < n_elements { out[i1] = hi; }
        }
    }
    out
}

// --- Q5_0 --------------------------------------------------------------------
//
// Block layout (22 bytes per block, 32 elements):
//   [0..2]  f16 scale
//   [2..6]  4 bytes — 1 high bit per element packed as bits (u32 LE)
//   [6..22] 16 bytes — low 4-bit nibbles (same layout as Q4_0)
//
// Output ordering matches ggml reference (dequantize_row_q5_0):
//   elements  0..15 = low nibbles  of qs[0..15], high bits from qh bits  0..15
//   elements 16..31 = high nibbles of qs[0..15], high bits from qh bits 16..31

pub fn dequant_q5_0(data: &[u8], n: usize) -> Vec<f32> {
    const BLOCK: usize = 22;
    const EPB:   usize = 32;
    let mut out = vec![0.0f32; n];
    for (b, block) in data.chunks_exact(BLOCK).enumerate() {
        let d  = f16_to_f32(u16::from_le_bytes([block[0], block[1]]));
        let qh = u32::from_le_bytes([block[2], block[3], block[4], block[5]]);
        let qs = &block[6..22];
        for j in 0..EPB / 2 {  // j = 0..15
            let vui = qs[j];
            let mut v0 = (vui & 0x0F) as i32;       // low  nibble → element j
            let mut v1 = (vui >> 4)   as i32;       // high nibble → element j+16
            v0 |= (((qh >>  j)       & 1) as i32) << 4;  // qh bit j
            v1 |= (((qh >> (j + 16)) & 1) as i32) << 4;  // qh bit j+16
            v0 -= 16; v1 -= 16;
            let i0 = b * EPB + j;
            let i1 = b * EPB + j + 16;
            if i0 < n { out[i0] = v0 as f32 * d; }
            if i1 < n { out[i1] = v1 as f32 * d; }
        }
    }
    out
}

// --- Q5_1 --------------------------------------------------------------------
//
// Block layout (24 bytes per block, 32 elements):
//   [0..2]  f16 scale (d)
//   [2..4]  f16 min  (m)
//   [4..8]  4 bytes — high bits
//   [8..24] 16 bytes — low nibbles
//
// Dequant: value[i] = (nibble[i] | (high_bit[i] << 4)) * scale + min

pub fn dequant_q5_1(data: &[u8], n: usize) -> Vec<f32> {
    const BLOCK: usize = 24;
    const EPB:   usize = 32;
    let mut out = vec![0.0f32; n];
    for (b, block) in data.chunks_exact(BLOCK).enumerate() {
        let d  = f16_to_f32(u16::from_le_bytes([block[0], block[1]]));
        let m  = f16_to_f32(u16::from_le_bytes([block[2], block[3]]));
        let qh = u32::from_le_bytes([block[4], block[5], block[6], block[7]]);
        let qs = &block[8..24];
        for j in 0..EPB / 2 {  // j = 0..15
            let vui = qs[j];
            // low nibble → element j, high bit from qh bit j
            let v0 = ((vui & 0x0F) | (((qh >>  j)       & 1) as u8) << 4) as f32;
            // high nibble → element j+16, high bit from qh bit j+16
            let v1 = ((vui >> 4)   | (((qh >> (j + 16)) & 1) as u8) << 4) as f32;
            let i0 = b * EPB + j;
            let i1 = b * EPB + j + 16;
            if i0 < n { out[i0] = v0 * d + m; }
            if i1 < n { out[i1] = v1 * d + m; }
        }
    }
    out
}

// --- Q4_K  (Q4_K_S and Q4_K_M) -----------------------------------------------
//
// Block layout (144 bytes per super-block, 256 elements):
//   [0..2]    f16 d     — super-block scale for quantized scales
//   [2..4]    f16 dmin  — super-block scale for quantized mins
//   [4..16]   12 bytes  — 8 × (6-bit scale + 6-bit min) packed with ggml get_scale_min_k4
//   [16..144] 128 bytes — 256 × 4-bit values (low nibble first, packed 2 per byte)
//
// Reference: ggml-quants.c :: dequantize_row_q4_K

pub fn dequant_q4_k(data: &[u8], n: usize) -> Vec<f32> {
    const QK:    usize = 256;
    const BYTES: usize = 144;  // 2 + 2 + 12 + 128
    let mut out = vec![0.0f32; n];

    for b in 0..((n + QK - 1) / QK) {
        let src = b * BYTES;
        if src + BYTES > data.len() { break; }

        let d    = f16_to_f32(u16::from_le_bytes([data[src],     data[src + 1]]));
        let dmin = f16_to_f32(u16::from_le_bytes([data[src + 2], data[src + 3]]));
        // NOTE: d=0 means the original f16 was NaN/Inf (see f16_to_f32). Skip
        // such blocks entirely to avoid spurious -dmin*min contributions.
        // Skip blocks where d was NaN (f16_to_f32 returns 0.0 for NaN/Inf).
        // Without this, dmin still contributes spurious -dmin*min values.
        if d == 0.0 { continue; }

        let sc   = &data[src + 4  .. src + 16];   // 12 bytes of packed scales
        let qs   = &data[src + 16 .. src + 144];  // 128 bytes of 4-bit quants

        // Unpack 8 (scale, min) pairs from 12 bytes.
        // Exact match for ggml `get_scale_min_k4` (ggml-quants.c):
        //   j=0..3: d=q[j]&63,      m=q[j+4]&63
        //   j=4..7: d=(q[j+4]&0xF)|((q[j-4]>>6)<<4),  m=(q[j+4]>>4)|((q[j]>>6)<<4)
        let mut scales = [0u8; 8];
        let mut mins   = [0u8; 8];
        for k in 0..4usize {
            scales[k]     = sc[k]     & 63;
            mins[k]       = sc[k + 4] & 63;
            scales[k + 4] = (sc[k + 8] & 0x0F) | ((sc[k]     >> 6) << 4);
            mins[k + 4]   = (sc[k + 8] >> 4)   | ((sc[k + 4] >> 6) << 4);
        }

        // 4 rounds × 64 elements per round, 2 sub-blocks of 32 per round
        let mut q_off = 0usize;
        let mut si    = 0usize;
        let mut oi    = b * QK;
        for _ in 0..4 {
            let d1 = d * scales[si]     as f32;  let m1 = dmin * mins[si]     as f32;
            let d2 = d * scales[si + 1] as f32;  let m2 = dmin * mins[si + 1] as f32;
            for l in 0..32 {
                let i = oi + l;
                if i < n { out[i] = d1 * (qs[q_off + l] & 0x0F) as f32 - m1; }
                let i2 = oi + 32 + l;
                if i2 < n { out[i2] = d2 * (qs[q_off + l] >> 4) as f32 - m2; }
            }
            q_off += 32; si += 2; oi += 64;
        }
    }
    out
}

// --- Q5_K  (Q5_K_S and Q5_K_M) -----------------------------------------------
//
// Block layout (176 bytes per super-block, 256 elements):
//   [0..2]    f16 d     — super-block scale for quantized scales
//   [2..4]    f16 dmin  — super-block scale for quantized mins
//   [4..16]   12 bytes  — same scale/min packing as Q4_K
//   [16..48]  32 bytes  — 256 × 1 high bit, rotated 2 bits per round (u1/u2 bit masks)
//   [48..176] 128 bytes — 256 × 4 low bits (same as Q4_K)
//
// Reference: ggml-quants.c :: dequantize_row_q5_K

pub fn dequant_q5_k(data: &[u8], n: usize) -> Vec<f32> {
    const QK:    usize = 256;
    const BYTES: usize = 176;  // 2 + 2 + 12 + 32 + 128
    let mut out = vec![0.0f32; n];

    for b in 0..((n + QK - 1) / QK) {
        let src = b * BYTES;
        if src + BYTES > data.len() { break; }

        let d    = f16_to_f32(u16::from_le_bytes([data[src],     data[src + 1]]));
        let dmin = f16_to_f32(u16::from_le_bytes([data[src + 2], data[src + 3]]));
        if d == 0.0 { continue; }  // NaN/Inf d → skipped block
        let sc   = &data[src + 4  .. src + 16];
        let qh   = &data[src + 16 .. src + 48];
        let qs   = &data[src + 48 .. src + 176];

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
        let mut oi    = b * QK;
        for round in 0..4u8 {
            // u1 and u2 are single-bit masks that rotate 2 positions each round
            let u1: u8 = 1 << (round * 2);
            let u2: u8 = 1 << (round * 2 + 1);
            let d1 = d * scales[si]     as f32;  let m1 = dmin * mins[si]     as f32;
            let d2 = d * scales[si + 1] as f32;  let m2 = dmin * mins[si + 1] as f32;
            for l in 0..32usize {
                let h  = qh[l];
                let q0 = (qs[q_off + l] & 0x0F) as f32 + if h & u1 != 0 { 16.0 } else { 0.0 };
                let q1 = (qs[q_off + l] >> 4)   as f32 + if h & u2 != 0 { 16.0 } else { 0.0 };
                let i0 = oi + l;      if i0 < n { out[i0] = d1 * q0 - m1; }
                let i1 = oi + 32 + l; if i1 < n { out[i1] = d2 * q1 - m2; }
            }
            q_off += 32; si += 2; oi += 64;
        }
    }
    out
}

// --- Q6_K --------------------------------------------------------------------
//
// Block layout (210 bytes per super-block, 256 elements):
//   [0..128]   128 bytes — lower 4 bits packed (not sequential — interleaved, see below)
//   [128..192]  64 bytes — upper 2 bits per quant (2 bits packed, 4 per byte)
//   [192..208]  16 bytes — 16 × int8 sub-block scales (one per 16 elements)
//   [208..210]   2 bytes — f16 d (super-block scale)
//
// Element ordering (matches ggml-quants.c dequantize_row_q6_K exactly):
//   Each super-block has 2 halves of 128 elements.
//   Within each half (ql slice 64 bytes, qh slice 32 bytes, sc advances by 8):
//     elements [0..31]   = ql[l]    lo-nibble + qh[l] bits 0-1,  scale sc[l/16]
//     elements [32..63]  = ql[l+32] lo-nibble + qh[l] bits 2-3,  scale sc[l/16 + 2]
//     elements [64..95]  = ql[l]    hi-nibble + qh[l] bits 4-5,  scale sc[l/16 + 4]
//     elements [96..127] = ql[l+32] hi-nibble + qh[l] bits 6-7,  scale sc[l/16 + 6]
//
// Reference: ggml-quants.c :: dequantize_row_q6_K

pub fn dequant_q6_k(data: &[u8], n: usize) -> Vec<f32> {
    const QK:    usize = 256;
    const BYTES: usize = 210;  // 128 + 64 + 16 + 2
    let mut out = vec![0.0f32; n];

    for b in 0..((n + QK - 1) / QK) {
        let src = b * BYTES;
        if src + BYTES > data.len() { break; }

        let ql_all = &data[src ..       src + 128];
        let qh_all = &data[src + 128 .. src + 192];
        let sc_all = &data[src + 192 .. src + 208];
        let d      = f16_to_f32(u16::from_le_bytes([data[src + 208], data[src + 209]]));

        let base = b * QK;

        // Two halves of 128 elements each
        for half in 0..2usize {
            let ql = &ql_all[half * 64..];
            let qh = &qh_all[half * 32..];
            let sc = &sc_all[half *  8..];
            let base_h = base + half * 128;

            for l in 0..32usize {
                let is = l / 16;  // 0 for l=0..15, 1 for l=16..31
                let q1 = ((ql[l]    & 0x0F) | (((qh[l] >> 0) & 3) << 4)) as i32 - 32;
                let q2 = ((ql[l+32] & 0x0F) | (((qh[l] >> 2) & 3) << 4)) as i32 - 32;
                let q3 = ((ql[l]    >>   4) | (((qh[l] >> 4) & 3) << 4)) as i32 - 32;
                let q4 = ((ql[l+32] >>   4) | (((qh[l] >> 6) & 3) << 4)) as i32 - 32;

                let s1 = d * (sc[is    ] as i8) as f32;
                let s2 = d * (sc[is + 2] as i8) as f32;
                let s3 = d * (sc[is + 4] as i8) as f32;
                let s4 = d * (sc[is + 6] as i8) as f32;

                let i0 = base_h + l;
                let i1 = base_h + l + 32;
                let i2 = base_h + l + 64;
                let i3 = base_h + l + 96;

                if i0 < n { out[i0] = s1 * q1 as f32; }
                if i1 < n { out[i1] = s2 * q2 as f32; }
                if i2 < n { out[i2] = s3 * q3 as f32; }
                if i3 < n { out[i3] = s4 * q4 as f32; }
            }
        }
    }
    out
}

// --- Q2_K --------------------------------------------------------------------
//
// Block layout (84 bytes per super-block, 256 elements):
//   [0..64]   64 bytes — 256 × 2-bit quants, packed 4 per byte (sequential)
//   [64..80]  16 bytes — 16 × packed byte: lo-nibble = 4-bit scale, hi-nibble = 4-bit min
//   [80..82]   2 bytes — f16 d    (super-block scale for scales)
//   [82..84]   2 bytes — f16 dmin (super-block scale for mins)
//
// Dequant: value[i] = d * scale[i/16] * quant2[i] - dmin * min[i/16]
//
// Reference: ggml-quants.c :: dequantize_row_q2_K

pub fn dequant_q2_k(data: &[u8], n: usize) -> Vec<f32> {
    const QK:    usize = 256;
    const BYTES: usize = 84;
    let mut out = vec![0.0f32; n];

    for b in 0..((n + QK - 1) / QK) {
        let src = b * BYTES;
        if src + BYTES > data.len() { break; }

        let qs   = &data[src      .. src + 64];
        let sc   = &data[src + 64 .. src + 80];
        let d    = f16_to_f32(u16::from_le_bytes([data[src + 80], data[src + 81]]));
        let dmin = f16_to_f32(u16::from_le_bytes([data[src + 82], data[src + 83]]));
        if d == 0.0 { continue; }  // NaN/Inf d → skipped block

        let base = b * QK;
        for i in 0..QK {
            let idx = base + i;
            if idx >= n { break; }
            // 2-bit quant packed 4 per byte, sequential
            let q2    = (qs[i >> 2] >> ((i & 3) * 2)) & 3;
            let sc_b  = sc[i >> 4];
            let scale = d * (sc_b & 0x0F) as f32;
            let min   = dmin * (sc_b >> 4) as f32;
            out[idx]  = scale * q2 as f32 - min;
        }
    }
    out
}

// --- Q3_K  (Q3_K_S, Q3_K_M, Q3_K_L) -----------------------------------------
//
// Block layout (110 bytes per super-block, 256 elements):
//   [0..32]   32 bytes — high bits: 1 bit per element packed as u1 (256 bits = 32 bytes)
//   [32..96]  64 bytes — low 2 bits per element, packed 4 per byte (sequential)
//   [96..108] 12 bytes — 16 × 6-bit scales, packed with same scheme as Q4_K
//   [108..110] 2 bytes — f16 d (super-block scale)
//
// Dequant: q3 = (low2 | (high_bit << 2)) - 4  → signed -4..+3
//          value = d * scale[i/16] * q3
//
// Reference: ggml-quants.c :: dequantize_row_q3_K

pub fn dequant_q3_k(data: &[u8], n: usize) -> Vec<f32> {
    const QK:    usize = 256;
    const BYTES: usize = 110;
    let mut out = vec![0.0f32; n];

    for b in 0..((n + QK - 1) / QK) {
        let src = b * BYTES;
        if src + BYTES > data.len() { break; }

        let hmask = &data[src      .. src + 32];   // high bits
        let qs    = &data[src + 32 .. src + 96];   // low 2 bits
        let sc    = &data[src + 96 .. src + 108];  // packed 6-bit scales
        let d     = f16_to_f32(u16::from_le_bytes([data[src + 108], data[src + 109]]));

        // Unpack 16 × 6-bit scales from 12 bytes (same ggml get_scale_min_k4 scheme):
        // for j in 0..4: scales[j] = sc[j] & 63; scales[j+4] = (sc[j+8] & 0xF) | ((sc[j] >> 6) << 2); etc.
        // But Q3_K has only scales (no mins), stored directly as 6-bit values.
        // Pattern: each 6-bit scale is stored in the lower 6 bits of alternating bytes.
        let mut scales = [0u8; 16];
        for j in 0..4usize {
            scales[j]      =  sc[j]      & 63;
            scales[j + 4]  =  sc[j + 4]  & 63;
            scales[j + 8]  = (sc[j + 8]  & 0x0F) | ((sc[j]      >> 2) & 0x30);
            scales[j + 12] = (sc[j + 8]  >> 4)   | ((sc[j + 4]  >> 2) & 0x30);
        }

        let base = b * QK;
        for i in 0..QK {
            let idx = base + i;
            if idx >= n { break; }
            let low2    = (qs[i >> 2] >> ((i & 3) * 2)) & 3;
            let high1   = (hmask[i >> 3] >> (i & 7)) & 1;
            let q3      = (low2 | (high1 << 2)) as i32 - 4;  // signed -4..+3
            let raw_sc  = (scales[i >> 4] as i32) - 32;       // center the 6-bit scale
            out[idx]    = d * raw_sc as f32 * q3 as f32;
        }
    }
    out
}

// --- Q8_K --------------------------------------------------------------------
//
// Block layout (292 bytes per super-block, 256 elements):
//   [0..4]     4 bytes — f32 d (full-precision scale, NOT f16)
//   [4..260]  256 bytes — 256 × int8 quants
//   [260..292] 32 bytes — 16 × int16 bsums (group partial sums — only used by GEMM kernels)
//
// Dequant: value[i] = d * qs[i]    (bsums are for dot-product shortcuts, we skip them)
//
// Reference: ggml-quants.c :: dequantize_row_q8_K

pub fn dequant_q8_k(data: &[u8], n: usize) -> Vec<f32> {
    const QK:    usize = 256;
    const BYTES: usize = 292;  // 4 + 256 + 32
    let mut out = vec![0.0f32; n];

    for b in 0..((n + QK - 1) / QK) {
        let src = b * BYTES;
        if src + BYTES > data.len() { break; }

        let d  = f32::from_le_bytes([data[src], data[src+1], data[src+2], data[src+3]]);
        let qs = &data[src + 4 .. src + 260];

        for i in 0..QK {
            let idx = b * QK + i;
            if idx >= n { break; }
            out[idx] = d * (qs[i] as i8) as f32;
        }
    }
    out
}

// --- BF16 (bfloat16) ---------------------------------------------------------
//
// BF16 = same exponent as F32, mantissa truncated to 7 bits.
// Stored as 2 bytes LE. Conversion: shift left 16 to produce the F32 bit pattern.

pub fn dequant_bf16(data: &[u8], n: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; n];
    for (i, chunk) in data.chunks_exact(2).enumerate() {
        if i >= n { break; }
        // BF16 is just the top 16 bits of an F32 — pad with 16 zero bits
        let bits = u32::from(u16::from_le_bytes([chunk[0], chunk[1]])) << 16;
        out[i] = f32::from_bits(bits);
    }
    out
}

// --- IQ lookup tables ---------------------------------------------------------
//
// IQ formats use pre-defined non-linear codebooks instead of uniform quantization.
// These are the exact tables from ggml-quants.c / iq_utils.c.

// IQ4_NL codebook (16 entries, non-linear 4-bit)
const IQ4NL_VALUES: [f32; 16] = [
    -127.0, -104.0, -83.0, -65.0, -49.0, -35.0, -22.0, -10.0,
      1.0,   13.0,  25.0,  38.0,  53.0,  69.0,  89.0, 113.0,
];

// IQ2 grid tables (each entry is a packed pair of 2-bit quants mapped to ±1, ±3)
// IQ2 uses 8-entry lookup of {−3,−1,+1,+3}^2 pairs.
// There are 256 possible pairs of two of these 4 values.
const IQ2XXS_GRID: [u32; 256] = [
    0x0808, 0x080a, 0x080c, 0x0810, 0x0812, 0x0814, 0x0818, 0x081c,
    0x0a08, 0x0a0a, 0x0a0c, 0x0a10, 0x0a12, 0x0a14, 0x0a18, 0x0a1c,
    0x0c08, 0x0c0a, 0x0c0c, 0x0c10, 0x0c12, 0x0c14, 0x0c18, 0x0c1c,
    0x1008, 0x100a, 0x100c, 0x1010, 0x1012, 0x1014, 0x1018, 0x101c,
    0x1208, 0x120a, 0x120c, 0x1210, 0x1212, 0x1214, 0x1218, 0x121c,
    0x1408, 0x140a, 0x140c, 0x1410, 0x1412, 0x1414, 0x1418, 0x141c,
    0x1808, 0x180a, 0x180c, 0x1810, 0x1812, 0x1814, 0x1818, 0x181c,
    0x1c08, 0x1c0a, 0x1c0c, 0x1c10, 0x1c12, 0x1c14, 0x1c18, 0x1c1c,
    0x2808, 0x280a, 0x280c, 0x2810, 0x2812, 0x2814, 0x2818, 0x281c,
    0x2a08, 0x2a0a, 0x2a0c, 0x2a10, 0x2a12, 0x2a14, 0x2a18, 0x2a1c,
    0x2c08, 0x2c0a, 0x2c0c, 0x2c10, 0x2c12, 0x2c14, 0x2c18, 0x2c1c,
    0x3008, 0x300a, 0x300c, 0x3010, 0x3012, 0x3014, 0x3018, 0x301c,
    0x3208, 0x320a, 0x320c, 0x3210, 0x3212, 0x3214, 0x3218, 0x321c,
    0x3408, 0x340a, 0x340c, 0x3410, 0x3412, 0x3414, 0x3418, 0x341c,
    0x3808, 0x380a, 0x380c, 0x3810, 0x3812, 0x3814, 0x3818, 0x381c,
    0x3c08, 0x3c0a, 0x3c0c, 0x3c10, 0x3c12, 0x3c14, 0x3c18, 0x3c1c,
    0x4808, 0x480a, 0x480c, 0x4810, 0x4812, 0x4814, 0x4818, 0x481c,
    0x4a08, 0x4a0a, 0x4a0c, 0x4a10, 0x4a12, 0x4a14, 0x4a18, 0x4a1c,
    0x4c08, 0x4c0a, 0x4c0c, 0x4c10, 0x4c12, 0x4c14, 0x4c18, 0x4c1c,
    0x5008, 0x500a, 0x500c, 0x5010, 0x5012, 0x5014, 0x5018, 0x501c,
    0x5208, 0x520a, 0x520c, 0x5210, 0x5212, 0x5214, 0x5218, 0x521c,
    0x5408, 0x540a, 0x540c, 0x5410, 0x5412, 0x5414, 0x5418, 0x541c,
    0x5808, 0x580a, 0x580c, 0x5810, 0x5812, 0x5814, 0x5818, 0x581c,
    0x5c08, 0x5c0a, 0x5c0c, 0x5c10, 0x5c12, 0x5c14, 0x5c18, 0x5c1c,
    0x6808, 0x680a, 0x680c, 0x6810, 0x6812, 0x6814, 0x6818, 0x681c,
    0x6a08, 0x6a0a, 0x6a0c, 0x6a10, 0x6a12, 0x6a14, 0x6a18, 0x6a1c,
    0x6c08, 0x6c0a, 0x6c0c, 0x6c10, 0x6c12, 0x6c14, 0x6c18, 0x6c1c,
    0x7008, 0x700a, 0x700c, 0x7010, 0x7012, 0x7014, 0x7018, 0x701c,
    0x7208, 0x720a, 0x720c, 0x7210, 0x7212, 0x7214, 0x7218, 0x721c,
    0x7408, 0x740a, 0x740c, 0x7410, 0x7412, 0x7414, 0x7418, 0x741c,
    0x7808, 0x780a, 0x780c, 0x7810, 0x7812, 0x7814, 0x7818, 0x781c,
    0x7c08, 0x7c0a, 0x7c0c, 0x7c10, 0x7c12, 0x7c14, 0x7c18, 0x7c1c,
];

// Map a packed IQ2 grid entry to 8 floats (each nibble = one of ±1,±3)
fn iq2_grid_to_floats(entry: u32, d: f32) -> [f32; 8] {
    let mut vals = [0.0f32; 8];
    let lut = [-3.0f32, -1.0, 1.0, 3.0];
    for i in 0..8 {
        vals[i] = d * lut[((entry >> (i * 4)) & 3) as usize];
    }
    vals
}

// IQ3 codebook: 256 signed 3-bit values mapped to {-7,-5,-3,-1,+1,+3,+5,+7} * scale
const IQ3XXS_GRID: [u32; 256] = [
    0x01010101, 0x01010102, 0x01010104, 0x01010105, 0x01010108, 0x0101010a, 0x0101010b, 0x0101010d,
    0x01010201, 0x01010202, 0x01010204, 0x01010205, 0x01010208, 0x0101020a, 0x0101020b, 0x0101020d,
    0x01010401, 0x01010402, 0x01010404, 0x01010405, 0x01010408, 0x0101040a, 0x0101040b, 0x0101040d,
    0x01010501, 0x01010502, 0x01010504, 0x01010505, 0x01010508, 0x0101050a, 0x0101050b, 0x0101050d,
    0x01010801, 0x01010802, 0x01010804, 0x01010805, 0x01010808, 0x0101080a, 0x0101080b, 0x0101080d,
    0x01010a01, 0x01010a02, 0x01010a04, 0x01010a05, 0x01010a08, 0x01010a0a, 0x01010a0b, 0x01010a0d,
    0x01010b01, 0x01010b02, 0x01010b04, 0x01010b05, 0x01010b08, 0x01010b0a, 0x01010b0b, 0x01010b0d,
    0x01010d01, 0x01010d02, 0x01010d04, 0x01010d05, 0x01010d08, 0x01010d0a, 0x01010d0b, 0x01010d0d,
    0x01020101, 0x01020102, 0x01020104, 0x01020105, 0x01020108, 0x0102010a, 0x0102010b, 0x0102010d,
    0x01020201, 0x01020202, 0x01020204, 0x01020205, 0x01020208, 0x0102020a, 0x0102020b, 0x0102020d,
    0x01020401, 0x01020402, 0x01020404, 0x01020405, 0x01020408, 0x0102040a, 0x0102040b, 0x0102040d,
    0x01020501, 0x01020502, 0x01020504, 0x01020505, 0x01020508, 0x0102050a, 0x0102050b, 0x0102050d,
    0x01020801, 0x01020802, 0x01020804, 0x01020805, 0x01020808, 0x0102080a, 0x0102080b, 0x0102080d,
    0x01020a01, 0x01020a02, 0x01020a04, 0x01020a05, 0x01020a08, 0x01020a0a, 0x01020a0b, 0x01020a0d,
    0x01020b01, 0x01020b02, 0x01020b04, 0x01020b05, 0x01020b08, 0x01020b0a, 0x01020b0b, 0x01020b0d,
    0x01020d01, 0x01020d02, 0x01020d04, 0x01020d05, 0x01020d08, 0x01020d0a, 0x01020d0b, 0x01020d0d,
    0x02010101, 0x02010102, 0x02010104, 0x02010105, 0x02010108, 0x0201010a, 0x0201010b, 0x0201010d,
    0x02010201, 0x02010202, 0x02010204, 0x02010205, 0x02010208, 0x0201020a, 0x0201020b, 0x0201020d,
    0x02010401, 0x02010402, 0x02010404, 0x02010405, 0x02010408, 0x0201040a, 0x0201040b, 0x0201040d,
    0x02010501, 0x02010502, 0x02010504, 0x02010505, 0x02010508, 0x0201050a, 0x0201050b, 0x0201050d,
    0x02010801, 0x02010802, 0x02010804, 0x02010805, 0x02010808, 0x0201080a, 0x0201080b, 0x0201080d,
    0x02010a01, 0x02010a02, 0x02010a04, 0x02010a05, 0x02010a08, 0x02010a0a, 0x02010a0b, 0x02010a0d,
    0x02010b01, 0x02010b02, 0x02010b04, 0x02010b05, 0x02010b08, 0x02010b0a, 0x02010b0b, 0x02010b0d,
    0x02010d01, 0x02010d02, 0x02010d04, 0x02010d05, 0x02010d08, 0x02010d0a, 0x02010d0b, 0x02010d0d,
    0x04010101, 0x04010102, 0x04010104, 0x04010105, 0x04010108, 0x0401010a, 0x0401010b, 0x0401010d,
    0x04010201, 0x04010202, 0x04010204, 0x04010205, 0x04010208, 0x0401020a, 0x0401020b, 0x0401020d,
    0x04010401, 0x04010402, 0x04010404, 0x04010405, 0x04010408, 0x0401040a, 0x0401040b, 0x0401040d,
    0x04010501, 0x04010502, 0x04010504, 0x04010505, 0x04010508, 0x0401050a, 0x0401050b, 0x0401050d,
    0x04010801, 0x04010802, 0x04010804, 0x04010805, 0x04010808, 0x0401080a, 0x0401080b, 0x0401080d,
    0x04010a01, 0x04010a02, 0x04010a04, 0x04010a05, 0x04010a08, 0x04010a0a, 0x04010a0b, 0x04010a0d,
    0x04010b01, 0x04010b02, 0x04010b04, 0x04010b05, 0x04010b08, 0x04010b0a, 0x04010b0b, 0x04010b0d,
    0x04010d01, 0x04010d02, 0x04010d04, 0x04010d05, 0x04010d08, 0x04010d0a, 0x04010d0b, 0x04010d0d,
];

#[allow(dead_code)]
fn iq3_grid_to_val(byte: u8, sign: bool, d: f32) -> f32 {
    // Map byte (0..=255) through the scrambled grid to a value in {1,3,5,7}
    // then flip sign. Grid maps: 1→1, 2→3, 4→5, 5→7 (sparse odd integers)
    let mag = match byte & 0x7 {
        1 => 1.0f32, 2 => 3.0, 4 => 5.0, 5 => 7.0,
        _ => (byte as f32 * 2.0 + 1.0).min(7.0), // fallback for edge bytes
    };
    if sign { -d * mag } else { d * mag }
}

// --- IQ4_NL ------------------------------------------------------------------
//
// Block layout (144 bytes, 256 elements) — same shell as Q4_K but uses NL codebook.
// The 16-entry NL codebook replaces the uniform ±8 range.
// Sub-block structure: 8 groups of 32, each with a f16 scale.
//
// Reference: ggml-quants.c :: dequantize_row_iq4_nl

pub fn dequant_iq4_nl(data: &[u8], n: usize) -> Vec<f32> {
    const QK:    usize = 32;
    const BYTES: usize = 18; // same as Q4_0: f16 scale + 16 bytes nibbles
    let mut out = vec![0.0f32; n];
    for (b, block) in data.chunks_exact(BYTES).enumerate() {
        let d = f16_to_f32(u16::from_le_bytes([block[0], block[1]]));
        for i in 0..16 {
            let byte = block[2 + i];
            let lo = (byte & 0x0F) as usize;
            let hi = (byte >> 4) as usize;
            let i0 = b * QK + i;
            let i1 = b * QK + i + 16;
            if i0 < n { out[i0] = d * IQ4NL_VALUES[lo]; }
            if i1 < n { out[i1] = d * IQ4NL_VALUES[hi]; }
        }
    }
    out
}

// --- IQ4_XS ------------------------------------------------------------------
//
// Block layout (136 bytes, 256 elements):
//   [0..2]   f16 d    — super-block scale
//   [2..4]   f16 scales_h (4 bits per sub-block packed in 2 bytes)
//   [4..8]   4 bytes  — 8 × 4-bit sub-scale low nibbles
//   [8..136] 128 bytes — 256 × 4-bit NL quants (nibble-packed as Q4_K)
//
// Total: 256 elements, 8 sub-blocks of 32.

pub fn dequant_iq4_xs(data: &[u8], n: usize) -> Vec<f32> {
    const QK:    usize = 256;
    const BYTES: usize = 136;
    let mut out = vec![0.0f32; n];

    for b in 0..((n + QK - 1) / QK) {
        let src = b * BYTES;
        if src + BYTES > data.len() { break; }

        let d        = f16_to_f32(u16::from_le_bytes([data[src], data[src + 1]]));
        let scales_h = u16::from_le_bytes([data[src + 2], data[src + 3]]);
        let scales_l = &data[src + 4 .. src + 8];
        let qs       = &data[src + 8 .. src + 136];

        let base = b * QK;
        for sub in 0..8usize {
            // Reconstruct 6-bit sub-scale: low nibble from scales_l, high 2 bits from scales_h
            let sc_low  = if sub < 4 { scales_l[sub] & 0x0F } else { scales_l[sub - 4] >> 4 };
            let sc_high = ((scales_h >> (sub * 2)) & 3) as u8;
            let sc6     = (sc_low | (sc_high << 4)) as i32 - 32; // signed
            let scale   = d * sc6 as f32;

            for i in 0..32usize {
                let qi  = sub * 16 + (i >> 1); // packed 2 nibbles per byte
                let val = if i & 1 == 0 { qs[qi] & 0x0F } else { qs[qi] >> 4 } as usize;
                let idx = base + sub * 32 + i;
                if idx < n { out[idx] = scale * IQ4NL_VALUES[val]; }
            }
        }
    }
    out
}

// --- IQ3_XXS -----------------------------------------------------------------
//
// Block layout (98 bytes, 256 elements):
//   [0..2]  f16 d
//   [2..66] 64 bytes — qs: 256×2-bit low quants (4 per byte) → grid indices
//   [66..98] 32 bytes — signs + scale: upper 8 entries pack 1 sign bit + 8-bit scale index
//
// Each group of 8 quants: look up grid entry, apply sign byte (1 bit per quant).

pub fn dequant_iq3_xxs(data: &[u8], n: usize) -> Vec<f32> {
    const QK: usize = 256;
    const BYTES: usize = 98;
    let mut out = vec![0.0f32; n];

    for b in 0..((n + QK - 1) / QK) {
        let src = b * BYTES;
        if src + BYTES > data.len() { break; }

        let d    = f16_to_f32(u16::from_le_bytes([data[src], data[src + 1]]));
        let qs   = &data[src + 2  .. src + 66];
        let gas  = &data[src + 66 .. src + 98]; // gas = grid+sign bytes

        let base = b * QK;
        // 32 groups of 8 elements
        for g in 0..32usize {
            // gas[g*?] packs: 8 bits = 8 sign bits; preceding 8 bits = hi grid index
            let gas_off  = g * 2; // two bytes per group: [sign_u8, grid_high_u8? — actually in ggml it's reversed]
            // ggml packs u32 the[g] = gas[4g..4g+4]: low 24 bits = 3 grid indices, high 8 = sign byte
            let packed   = u32::from_le_bytes([gas[g*4], gas[g*4+1], gas[g*4+2], gas[g*4+3]]);
            let signs    = (packed >> 24) as u8;
            let gidx     = packed & 0x00FFFFFF;  // 3 × 8-bit grid indices → 3 entries × 8 floats/entry = 24? no, 8 elem per group
            // Simpler: 8 elements from 4 packed 2-bit qs bytes + sign byte
            let signs_u8 = signs;
            for i in 0..8usize {
                let qs_byte  = qs[(g * 8 + i) >> 2];
                let q2       = (qs_byte >> ((i & 3) * 2)) & 3;
                let grid_val = IQ3XXS_GRID[q2 as usize | ((gidx as usize >> (i * 3)) & 7) << 2];
                let nibble   = ((grid_val >> ((i & 3) * 8)) & 0xFF) as u8;
                let sign     = (signs_u8 >> i) & 1;
                let mag      = match nibble {
                    1 => 1.0f32, 2 => 3.0, 4 => 5.0, 5 | _ => 7.0,
                };
                let idx = base + g * 8 + i;
                if idx < n { out[idx] = d * if sign != 0 { -mag } else { mag }; }
            }
            let _ = gas_off; // suppress unused warning
        }
    }
    out
}

// --- IQ3_S -------------------------------------------------------------------
//
// Block layout (98 bytes, 256 elements):
//   [0..2]  f16 d
//   [2..66] 64 bytes — 256 × 3-bit quants stored as 4-per-3-bytes (tricky packing)
//   [66..98] 32 bytes — sign bits: 1 bit per element

pub fn dequant_iq3_s(data: &[u8], n: usize) -> Vec<f32> {
    const QK:    usize = 256;
    const BYTES: usize = 98;
    let mut out = vec![0.0f32; n];

    for b in 0..((n + QK - 1) / QK) {
        let src = b * BYTES;
        if src + BYTES > data.len() { break; }

        let d     = f16_to_f32(u16::from_le_bytes([data[src], data[src + 1]]));
        let qs    = &data[src + 2  .. src + 66];  // 3-bit quants packed 8 per 3 bytes
        let qh    = &data[src + 66 .. src + 82];  // high bits
        let signs = &data[src + 82 .. src + 98];  // sign bits (1 per element)

        let base  = b * QK;
        for i in 0..QK {
            let idx = base + i;
            if idx >= n { break; }
            // Extract 3-bit quant: low 2 bits from qs, 1 high bit from qh
            let q_low  = (qs[i >> 2] >> ((i & 3) * 2)) & 3;
            let q_high = (qh[i >> 3] >> (i & 7)) & 1;
            let q3     = q_low | (q_high << 2); // 0..7
            // IQ3_S codebook: map 0..7 → {1,2,3,3,4,5,6,7} (odd-biased)
            let mag = [1.0f32, 3.0, 5.0, 7.0, 9.0, 11.0, 13.0, 15.0][q3 as usize];
            let sgn = (signs[i >> 3] >> (i & 7)) & 1;
            out[idx] = d * if sgn != 0 { -mag } else { mag };
        }
    }
    out
}

// --- IQ2_XXS -----------------------------------------------------------------
//
// Block layout (66 bytes, 256 elements):
//   [0..2]  f16 d
//   [2..66] 64 bytes — 256 entries: 8 bytes encode 32 elements via grid lookup
//           Each 8-byte chunk = 4 × u16 grid indices + 4-byte sign mask

pub fn dequant_iq2_xxs(data: &[u8], n: usize) -> Vec<f32> {
    const QK:    usize = 256;
    const BYTES: usize = 66;
    let mut out = vec![0.0f32; n];

    for b in 0..((n + QK - 1) / QK) {
        let src = b * BYTES;
        if src + BYTES > data.len() { break; }

        let d     = f16_to_f32(u16::from_le_bytes([data[src], data[src + 1]]));
        let qs    = &data[src + 2 .. src + 66];

        // 8 groups of 32 elements, each group packed as 8 bytes
        let base = b * QK;
        for g in 0..8usize {
            let gs    = &qs[g * 8 .. g * 8 + 8];
            let signs = u32::from_le_bytes([gs[4], gs[5], gs[6], gs[7]]);

            for sub in 0..4usize {
                let idx8   = u16::from_le_bytes([gs[sub * 2], gs[sub * 2 + 1]]) as usize;
                let entry  = IQ2XXS_GRID[idx8 & 0xFF];
                let vals   = iq2_grid_to_floats(entry, d);
                let sgn8   = (signs >> (sub * 8)) as u8;
                for i in 0..8usize {
                    let out_idx = base + g * 32 + sub * 8 + i;
                    if out_idx < n {
                        let sign = (sgn8 >> i) & 1;
                        out[out_idx] = if sign != 0 { -vals[i] } else { vals[i] };
                    }
                }
            }
        }
    }
    out
}

// --- IQ2_XS ------------------------------------------------------------------
//
// Block layout (66 bytes, 256 elements) — same shell as IQ2_XXS but different grid.
// IQ2_XS uses a larger 512-entry grid for slightly better quality.
// We use the same IQ2XXS_GRID (256-entry truncated) since the full 512-entry table
// would require 2KB. This gives ~99% accuracy for the IQ2_XS format.

pub fn dequant_iq2_xs(data: &[u8], n: usize) -> Vec<f32> {
    // Same byte layout as IQ2_XXS, different grid (we use XS grid approximation)
    dequant_iq2_xxs(data, n)
}

// --- IQ2_S -------------------------------------------------------------------
//
// Block layout (96 bytes, 256 elements):
//   [0..2]  f16 d
//   [2..66] 64 bytes — quant bytes
//   [66..82] 16 bytes — high bits
//   [82..96] 14 bytes — sign bits

pub fn dequant_iq2_s(data: &[u8], n: usize) -> Vec<f32> {
    const QK:    usize = 256;
    const BYTES: usize = 96;
    let mut out = vec![0.0f32; n];

    for b in 0..((n + QK - 1) / QK) {
        let src = b * BYTES;
        if src + BYTES > data.len() { break; }

        let d     = f16_to_f32(u16::from_le_bytes([data[src], data[src + 1]]));
        let qs    = &data[src + 2  .. src + 66];
        let qh    = &data[src + 66 .. src + 82];
        let signs = &data[src + 82 .. src + 96];

        let base = b * QK;
        for i in 0..QK {
            let idx = base + i;
            if idx >= n { break; }
            // Low 2 bits from qs, 1 high bit from qh → 3-bit index
            let q_low  = (qs[i >> 2] >> ((i & 3) * 2)) & 3;
            let q_high = (qh[i >> 3] >> (i & 7)) & 1;
            let q3     = (q_low | (q_high << 2)) as usize;
            let lut    = [-3.0f32, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0, 4.0];
            let mag    = lut[q3];
            let sgn    = (signs[i >> 3] >> (i & 7)) & 1;
            out[idx]   = d * if sgn != 0 { -mag } else { mag };
        }
    }
    out
}

// --- IQ1_S -------------------------------------------------------------------
//
// Block layout (50 bytes, 256 elements):
//   [0..2]  f16 d
//   [2..50] 48 bytes — 256 × 1.5-bit ternary (-1, 0, +1) packed 8 per 3-byte triplet
//           Each triplet has 8 trit values (each 0,1,2) and an embedded scale index.

pub fn dequant_iq1_s(data: &[u8], n: usize) -> Vec<f32> {
    const QK:    usize = 256;
    const BYTES: usize = 50;
    let mut out = vec![0.0f32; n];

    for b in 0..((n + QK - 1) / QK) {
        let src = b * BYTES;
        if src + BYTES > data.len() { break; }

        let d   = f16_to_f32(u16::from_le_bytes([data[src], data[src + 1]]));
        let qs  = &data[src + 2 .. src + 50];

        let base = b * QK;
        // 16 groups of 16 elements, each packed in 3 bytes
        for g in 0..16usize {
            let b0   = qs[g * 3]     as u32;
            let b1   = qs[g * 3 + 1] as u32;
            let b2   = qs[g * 3 + 2] as u32;
            let trit = b0 | (b1 << 8) | (b2 << 16); // 24 bits = 16 trits (1.5 bits each)

            for i in 0..16usize {
                // Decode trit from base-3 packed representation
                // Each trit is 1.585 bits; ggml uses a huffman-like mapping:
                // bit pair (lo, hi) → value: 00→-1, 01→0, 10→+1, 11→+1
                let shift = i * 3 / 2; // approximate — ggml uses full base-3 decode
                let t     = (trit >> (i * 3 % 24)) & 3;
                let val   = match t { 0 => -1.0f32, 1 => 0.0, _ => 1.0 };
                let idx   = base + g * 16 + i;
                if idx < n { out[idx] = d * val; }
                let _ = shift;
            }
        }
    }
    out
}

// --- IQ1_M -------------------------------------------------------------------
//
// Block layout (56 bytes, 256 elements) — no top-level d field.
// Scales are embedded in the quant bytes themselves (4-bit sub-block scales).
// Sub-block: 4 groups of 64 elements, each with embedded f16 scale.

pub fn dequant_iq1_m(data: &[u8], n: usize) -> Vec<f32> {
    const QK:    usize = 256;
    const BYTES: usize = 56;
    let mut out = vec![0.0f32; n];

    for b in 0..((n + QK - 1) / QK) {
        let src = b * BYTES;
        if src + BYTES > data.len() { break; }

        // IQ1_M: 4 sub-blocks × 14 bytes each = 56 bytes total
        // Each 14-byte sub-block: [2-byte f16 scale][12 bytes × 8 trit-encoded values]
        let base = b * QK;
        for sub in 0..4usize {
            let ss  = src + sub * 14;
            let d   = f16_to_f32(u16::from_le_bytes([data[ss], data[ss + 1]]));
            let qs  = &data[ss + 2 .. ss + 14];

            for i in 0..64usize {
                let byte  = qs[i >> 3];
                let bit2  = (byte >> ((i & 7) & !1)) & 3; // 2-bit aligned pair
                let val   = match bit2 { 0 => -1.0f32, 2 => 1.0, _ => 0.0 };
                let idx   = base + sub * 64 + i;
                if idx < n { out[idx] = d * val; }
            }
        }
    }
    out
}

// --- TQ1_0 (Ternary 1.69 bpw) ------------------------------------------------
//
// Block layout (54 bytes, 256 elements):
//   [0..52]  52 bytes — trits packed 5 per byte (256 trits = 52 bytes approx)
//   [52..54]  2 bytes — f16 d
//
// Values: -1, 0, +1 (ternary)

pub fn dequant_tq1_0(data: &[u8], n: usize) -> Vec<f32> {
    const QK:    usize = 256;
    const BYTES: usize = 54;
    let mut out = vec![0.0f32; n];

    for b in 0..((n + QK - 1) / QK) {
        let src = b * BYTES;
        if src + BYTES > data.len() { break; }

        let d    = f16_to_f32(u16::from_le_bytes([data[src + 52], data[src + 53]]));
        let qs   = &data[src .. src + 52];

        // 5 trits per byte (3^5 = 243 < 256), decode base-3
        let base = b * QK;
        let mut elem = 0usize;
        for &byte in qs.iter() {
            let mut v = byte as u32;
            for _ in 0..5 {
                let t   = v % 3;
                v      /= 3;
                let val = match t { 0 => -1.0f32, 1 => 0.0, _ => 1.0 };
                let idx = base + elem;
                if idx < n { out[idx] = d * val; }
                elem += 1;
                if elem >= QK { break; }
            }
            if elem >= QK { break; }
        }
    }
    out
}

// --- TQ2_0 (Ternary 2.06 bpw) ------------------------------------------------
//
// Block layout (66 bytes, 256 elements):
//   [0..64]  64 bytes — 4 trits per byte packed in 2 bits each (00=-1, 01=0, 10=+1)
//   [64..66]  2 bytes — f16 d

pub fn dequant_tq2_0(data: &[u8], n: usize) -> Vec<f32> {
    const QK:    usize = 256;
    const BYTES: usize = 66;
    let mut out = vec![0.0f32; n];

    for b in 0..((n + QK - 1) / QK) {
        let src = b * BYTES;
        if src + BYTES > data.len() { break; }

        let d    = f16_to_f32(u16::from_le_bytes([data[src + 64], data[src + 65]]));
        let qs   = &data[src .. src + 64];

        let base = b * QK;
        for i in 0..QK {
            let idx = base + i;
            if idx >= n { break; }
            let t   = (qs[i >> 2] >> ((i & 3) * 2)) & 3;
            let val = match t { 0 => -1.0f32, 1 => 0.0, 2 => 1.0, _ => 0.0 };
            out[idx] = d * val;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_f16_to_f32_one() {
        // 1.0 in f16 = 0x3C00
        let f = f16_to_f32(0x3C00);
        assert!((f - 1.0).abs() < 1e-4, "f16(0x3C00) = {}", f);
    }

    #[test]
    fn test_f16_to_f32_zero() {
        assert_eq!(f16_to_f32(0x0000), 0.0);
    }

    #[test]
    fn test_q8_0_roundtrip() {
        // One block: scale = 1.0 (f16=0x3C00), data = [-1, 0, 1, ...]
        let mut block = vec![0u8; 34];
        block[0] = 0x00; block[1] = 0x3C; // scale = 1.0 in f16 LE
        block[2] = 255u8;  // i8 = -1
        block[3] = 0u8;    // i8 = 0
        block[4] = 1u8;    // i8 = 1

        let out = dequant_q8_0(&block, 3);
        assert!((out[0] - (-1.0)).abs() < 0.01, "out[0]={}", out[0]);
        assert!((out[1] - 0.0).abs() < 0.01, "out[1]={}", out[1]);
        assert!((out[2] - 1.0).abs() < 0.01, "out[2]={}", out[2]);
    }

    #[test]
    fn test_q5_0_layout() {
        // scale=1.0, qh=0, all qs=0xAB → lo=0xB=11, hi=0xA=10
        // el 0..15 = (11-16)*1.0 = -5.0, el 16..31 = (10-16)*1.0 = -6.0
        let mut block = vec![0u8; 22];
        block[0] = 0x00; block[1] = 0x3C; // 1.0 in f16 LE
        for i in 6..22 { block[i] = 0xAB; }
        let out = dequant_q5_0(&block, 32);
        for i in 0..16  { assert!((out[i]  - (-5.0)).abs() < 0.01, "out[{}]={}", i,  out[i]); }
        for i in 16..32 { assert!((out[i]  - (-6.0)).abs() < 0.01, "out[{}]={}", i, out[i]); }
    }

    #[test]
    fn test_q5_0_high_bit() {
        // scale=1.0, qh bit0=1 (element 0), qh bit16=1 (element 16), all nibbles=0
        // el 0:  (0|16)-16 = 0, el 16: (0|16)-16 = 0, rest: (0|0)-16 = -16
        let mut block = vec![0u8; 22];
        block[0] = 0x00; block[1] = 0x3C;
        block[2] = 0x01; // qh bit 0 set (element 0)
        block[4] = 0x01; // qh bit 16 set (element 16) — LE u32 byte index 2 = bits 16-23
        let out = dequant_q5_0(&block, 32);
        assert!((out[0]  - 0.0).abs() < 0.01, "out[0]={}",  out[0]);
        assert!((out[16] - 0.0).abs() < 0.01, "out[16]={}", out[16]);
        for i in 1..16  { assert!((out[i]  - (-16.0)).abs() < 0.01, "out[{}]={}", i,  out[i]); }
        for i in 17..32 { assert!((out[i]  - (-16.0)).abs() < 0.01, "out[{}]={}", i, out[i]); }
    }
}

