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
// Block layout (84 bytes per super-block, 256 elements), ggml block_q2_K:
//   [0..16]   16 bytes - scales: lo nibble = scale, hi nibble = min (one per 16 elements)
//   [16..80]  64 bytes - qs: 2-bit quants; within each 128-element half, 32 bytes hold
//             4 shift-planes (bits 0-1, 2-3, 4-5, 6-7), each plane = 32 consecutive elements
//   [80..82]  f16 d, [82..84] f16 dmin
// value = d*(sc&15)*q - dmin*(sc>>4)
// Reference: ggml-quants.c :: dequantize_row_q2_K

pub fn dequant_q2_k(data: &[u8], n: usize) -> Vec<f32> {
    const QK:    usize = 256;
    const BYTES: usize = 84;
    let mut out = vec![0.0f32; n];

    for b in 0..((n + QK - 1) / QK) {
        let src = b * BYTES;
        if src + BYTES > data.len() { break; }

        let sc   = &data[src .. src + 16];
        let qs   = &data[src + 16 .. src + 80];
        let d    = f16_to_f32(u16::from_le_bytes([data[src + 80], data[src + 81]]));
        let dmin = f16_to_f32(u16::from_le_bytes([data[src + 82], data[src + 83]]));

        let base = b * QK;
        for i in 0..QK {
            let idx = base + i;
            if idx >= n { break; }
            let (chunk, r) = (i / 128, i % 128);
            let (j, l) = (r / 32, r % 32); // shift plane j, byte l within the 32-byte half
            let q2    = (qs[chunk * 32 + l] >> (2 * j)) & 3;
            let sc_b  = sc[i >> 4];
            out[idx]  = d * (sc_b & 0x0F) as f32 * q2 as f32 - dmin * (sc_b >> 4) as f32;
        }
    }
    out
}

// --- Q3_K  (Q3_K_S, Q3_K_M, Q3_K_L) -----------------------------------------
//
// Block layout (110 bytes per super-block, 256 elements):
//   [0..32]   32 bytes — high bits: 1 bit per element packed as u1 (256 bits = 32 bytes)
//   [32..96]  64 bytes — low 2 bits per element, packed 4 per byte (sequential)
//   [96..108] 12 bytes — 16 x 6-bit scales (ggml aux/kmask packing, see code)
//   [108..110] 2 bytes — f16 d (super-block scale)
//
// Dequant: q3 = (low2 | (high_bit << 2)) - 4  → signed -4..+3
//          value = d * scale[i/16] * q3
//
// Reference: ggml-quants.c :: dequantize_row_q3_K

pub fn dequant_q3_k(data: &[u8], n: usize) -> Vec<f32> {
    // ggml block_q3_K: hmask[32], qs[64], scales[12], f16 d. Same shift-plane layout as Q2_K;
    // hmask bit (chunk*4 + plane) of byte l is the high bit (set => no -4 offset).
    const QK:    usize = 256;
    const BYTES: usize = 110;
    let mut out = vec![0.0f32; n];

    for b in 0..((n + QK - 1) / QK) {
        let src = b * BYTES;
        if src + BYTES > data.len() { break; }

        let hmask = &data[src      .. src + 32];
        let qs    = &data[src + 32 .. src + 96];
        let sc    = &data[src + 96 .. src + 108];
        let d     = f16_to_f32(u16::from_le_bytes([data[src + 108], data[src + 109]]));

        // 16 x 6-bit scales: low nibbles in sc[0..8], high 2 bits in sc[8..12].
        let mut scales = [0i32; 16];
        for j in 0..4usize {
            scales[j]      = ((sc[j]     & 0x0F) | ((sc[8 + j]      & 3) << 4)) as i32;
            scales[j + 4]  = ((sc[4 + j] & 0x0F) | (((sc[8 + j] >> 2) & 3) << 4)) as i32;
            scales[j + 8]  = ((sc[j]     >> 4)   | (((sc[8 + j] >> 4) & 3) << 4)) as i32;
            scales[j + 12] = ((sc[4 + j] >> 4)   | (((sc[8 + j] >> 6) & 3) << 4)) as i32;
        }

        let base = b * QK;
        for i in 0..QK {
            let idx = base + i;
            if idx >= n { break; }
            let (chunk, r) = (i / 128, i % 128);
            let (j, l) = (r / 32, r % 32);
            let low2  = ((qs[chunk * 32 + l] >> (2 * j)) & 3) as i32;
            let high  = (hmask[l] >> (chunk * 4 + j)) & 1;
            let q3    = low2 - if high != 0 { 0 } else { 4 };
            out[idx]  = d * (scales[i >> 4] - 32) as f32 * q3 as f32;
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
// Only IQ4_NL / IQ4_XS are implemented: they need just this 16-entry table (kvalues_iq4nl).
// IQ1/IQ2/IQ3 need large codebook grids that are NOT verified here; those types error out
// at load (see GgmlType::unsupported_reason).

// IQ4_NL codebook (16 entries, non-linear 4-bit)
const IQ4NL_VALUES: [f32; 16] = [
    -127.0, -104.0, -83.0, -65.0, -49.0, -35.0, -22.0, -10.0,
      1.0,   13.0,  25.0,  38.0,  53.0,  69.0,  89.0, 113.0,
];

// --- IQ4_NL ------------------------------------------------------------------
//
// Block layout (18 bytes, 32 elements): f16 d + 16 bytes of nibbles; element j = lo nibble of
// byte j, element j+16 = hi nibble.
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
            // ls = nibble (sub%2) of scales_l[sub/2] | 2 high bits from scales_h << 4
            let sc_low  = (scales_l[sub / 2] >> (4 * (sub % 2))) & 0x0F;
            let sc_high = ((scales_h >> (sub * 2)) & 3) as u8;
            let scale   = d * ((sc_low | (sc_high << 4)) as i32 - 32) as f32;
            for j in 0..16usize {
                let byte = qs[sub * 16 + j];
                let (i0, i1) = (base + sub * 32 + j, base + sub * 32 + j + 16);
                if i0 < n { out[i0] = scale * IQ4NL_VALUES[(byte & 0x0F) as usize]; }
                if i1 < n { out[i1] = scale * IQ4NL_VALUES[(byte >> 4) as usize]; }
            }
        }
    }
    out
}

// --- TQ2_0 (Ternary 2.06 bpw) ------------------------------------------------
//
// ggml block_tq2_0 (66 bytes, 256 elements): u8 qs[64], f16 d. For each 32-byte half j, for
// 2-bit plane l (0..3), byte m (0..32): element = j*4 + l*32 + m, value = (q - 1) * d.

pub fn dequant_tq2_0(data: &[u8], n: usize) -> Vec<f32> {
    const QK:    usize = 256;
    const BYTES: usize = 66;
    let mut out = vec![0.0f32; n];

    for b in 0..((n + QK - 1) / QK) {
        let src = b * BYTES;
        if src + BYTES > data.len() { break; }

        let d  = f16_to_f32(u16::from_le_bytes([data[src + 64], data[src + 65]]));
        let qs = &data[src .. src + 64];

        for i in 0..QK {
            let idx = b * QK + i;
            if idx >= n { break; }
            let (half, r) = (i / 128, i % 128);
            let (l, m) = (r / 32, r % 32);
            let t = (qs[half * 32 + m] >> (l * 2)) & 3;
            out[idx] = (t as i32 - 1) as f32 * d;
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

    // ---- literal ports of the ggml reference loops (ggml-quants.c dequantize_row_*) ----
    fn lcg_bytes(n: usize, seed: u32) -> Vec<u8> {
        let mut x = seed;
        (0..n).map(|_| { x = x.wrapping_mul(1664525).wrapping_add(1013904223); (x >> 24) as u8 }).collect()
    }

    fn ref_q2_k(blk: &[u8]) -> Vec<f32> {
        // struct { u8 scales[16]; u8 qs[64]; half d; half dmin; }
        let scales = &blk[0..16];
        let qs = &blk[16..80];
        let d = f16_to_f32(u16::from_le_bytes([blk[80], blk[81]]));
        let min = f16_to_f32(u16::from_le_bytes([blk[82], blk[83]]));
        let (mut y, mut is, mut q) = (Vec::new(), 0, 0);
        for _n in (0..256).step_by(128) {
            let mut shift = 0;
            for _j in 0..4 {
                for half in 0..2 {
                    let sc = scales[is]; is += 1;
                    let (dl, ml) = (d * (sc & 0xF) as f32, min * (sc >> 4) as f32);
                    for l in 0..16 { y.push(dl * ((qs[q + 16 * half + l] >> shift) & 3) as f32 - ml); }
                }
                shift += 2;
            }
            q += 32;
        }
        y
    }

    fn ref_q3_k(blk: &[u8]) -> Vec<f32> {
        // struct { u8 hmask[32]; u8 qs[64]; u8 scales[12]; half d; }
        let (hm, qs, sc) = (&blk[0..32], &blk[32..96], &blk[96..108]);
        let d = f16_to_f32(u16::from_le_bytes([blk[108], blk[109]]));
        let mut aux = [0u32; 4];
        for i in 0..3 { aux[i] = u32::from_le_bytes([sc[4*i], sc[4*i+1], sc[4*i+2], sc[4*i+3]]); }
        let (k1, k2) = (0x03030303u32, 0x0f0f0f0fu32);
        let tmp = aux[2];
        aux[2] = ((aux[0] >> 4) & k2) | (((tmp >> 4) & k1) << 4);
        aux[3] = ((aux[1] >> 4) & k2) | (((tmp >> 6) & k1) << 4);
        aux[0] = (aux[0] & k2) | (((tmp >> 0) & k1) << 4);
        aux[1] = (aux[1] & k2) | (((tmp >> 2) & k1) << 4);
        let scales: Vec<i32> = aux.iter().flat_map(|w| w.to_le_bytes()).map(|b| b as i8 as i32).collect();
        let (mut y, mut is, mut q, mut m) = (Vec::new(), 0, 0, 1u8);
        for _n in (0..256).step_by(128) {
            let mut shift = 0;
            for _j in 0..4 {
                for half in 0..2 {
                    let dl = d * (scales[is] - 32) as f32; is += 1;
                    for l in 0..16 {
                        let lo = ((qs[q + 16 * half + l] >> shift) & 3) as i32;
                        let sub = if hm[16 * half + l] & m != 0 { 0 } else { 4 };
                        y.push(dl * (lo - sub) as f32);
                    }
                }
                shift += 2; m <<= 1;
            }
            q += 32;
        }
        y
    }

    fn ref_iq4_xs(blk: &[u8]) -> Vec<f32> {
        // struct { half d; u16 scales_h; u8 scales_l[4]; u8 qs[128]; }
        let d = f16_to_f32(u16::from_le_bytes([blk[0], blk[1]]));
        let sh = u16::from_le_bytes([blk[2], blk[3]]);
        let (sl, qs) = (&blk[4..8], &blk[8..136]);
        let mut y = Vec::new();
        for ib in 0..8 {
            let ls = ((sl[ib / 2] >> (4 * (ib % 2))) & 0xf) as i32 | ((((sh >> (2 * ib)) & 3) as i32) << 4);
            let dl = d * (ls - 32) as f32;
            let q = &qs[16 * ib..16 * ib + 16];
            for j in 0..16 { y.push(dl * IQ4NL_VALUES[(q[j] & 0xf) as usize]); }
            for j in 0..16 { y.push(dl * IQ4NL_VALUES[(q[j] >> 4) as usize]); }
        }
        y
    }

    fn ref_tq2_0(blk: &[u8]) -> Vec<f32> {
        // struct { u8 qs[64]; half d; }
        let d = f16_to_f32(u16::from_le_bytes([blk[64], blk[65]]));
        let mut y = Vec::new();
        for j in (0..64).step_by(32) {
            for l in 0..4 { for m in 0..32 { y.push((((blk[j + m] >> (l * 2)) & 3) as i32 - 1) as f32 * d); } }
        }
        y
    }

    fn check(name: &str, got: Vec<f32>, want: Vec<f32>) {
        assert_eq!(got.len(), want.len());
        for (i, (g, w)) in got.iter().zip(&want).enumerate() {
            assert!((g - w).abs() <= 1e-3 * w.abs().max(1.0), "{} elem {}: got {} want {}", name, i, g, w);
        }
    }

    #[test]
    fn q2_k_matches_ggml_layout() {
        let mut blk = lcg_bytes(84, 1);
        blk[80] = 0x00; blk[81] = 0x3C; blk[82] = 0x00; blk[83] = 0x38; // d=1.0 dmin=0.5
        check("q2_k", dequant_q2_k(&blk, 256), ref_q2_k(&blk));
    }

    #[test]
    fn q3_k_matches_ggml_layout() {
        let mut blk = lcg_bytes(110, 2);
        blk[108] = 0x00; blk[109] = 0x3C;
        check("q3_k", dequant_q3_k(&blk, 256), ref_q3_k(&blk));
    }

    #[test]
    fn iq4_xs_matches_ggml_layout() {
        let mut blk = lcg_bytes(136, 3);
        blk[0] = 0x00; blk[1] = 0x3C;
        check("iq4_xs", dequant_iq4_xs(&blk, 256), ref_iq4_xs(&blk));
    }

    #[test]
    fn tq2_0_matches_ggml_layout() {
        let mut blk = lcg_bytes(66, 4);
        blk[64] = 0x00; blk[65] = 0x3C;
        check("tq2_0", dequant_tq2_0(&blk, 256), ref_tq2_0(&blk));
    }

    #[test]
    fn iq4_nl_block_is_32_elements_18_bytes() {
        use super::super::gguf::GgmlType;
        assert_eq!(GgmlType::IQ4NL.bytes_per_block(), 18);
        assert_eq!(GgmlType::IQ4NL.elements_per_block(), 32);
        let mut blk = vec![0u8; 18];
        blk[1] = 0x3C; blk[2] = 0x80; // d=1, byte0 = lo 0, hi 8
        let out = dequant_iq4_nl(&blk, 32);
        assert_eq!(out[0], -127.0); assert_eq!(out[16], 1.0);
    }
}

