// inference/gguf.rs — GGUF v3 file format parser
//
// GGUF is the native weight format used by llama.cpp and all popular quantized LLMs.
// Every model on HuggingFace (TinyLlama, Phi-3-mini, Mistral, Gemma...) ships as .gguf.
//
// Format:
//   [magic u32][version u32][n_tensors u64][n_kv u64]
//   [n_kv × key-value metadata pairs]
//   [n_tensors × tensor info (name + shape + type + offset)]
//   [alignment padding to 32 bytes]
//   [raw tensor data]

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};

pub const GGUF_MAGIC: u32 = 0x46554747;
pub const GGUF_VERSION_MAX: u32 = 3;
const MAX_TENSOR_DIMS: usize = 8;
const MAX_ARRAY_DEPTH: usize = 4;

// --- Value types -------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum GgufValue {
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    F32(f32),
    Bool(bool),
    String(String),
    Array(Vec<GgufValue>),
    U64(u64),
    I64(i64),
    F64(f64),
}

// --- Tensor quantization types ------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GgmlType {
    F32  = 0,
    F16  = 1,
    Q4_0 = 2,
    Q4_1 = 3,
    Q5_0 = 6,
    Q5_1 = 7,
    Q8_0 = 8,
    Q8_1 = 9,
    Q2K  = 10,
    Q3KS = 11,
    Q3KM = 12,
    Q3KL = 13,
    Q4KS = 14,
    Q4KM = 15,
    Q5KS = 16,
    Q5KM = 17,
    Q6K  = 18,
    Q8K  = 19,
    I8   = 24,
    I16  = 25,
    I32  = 26,
    I64  = 27,
    F64  = 28,
    // IQ (imatrix importance-weighted) formats — type IDs 29–45
    IQ2XXS = 29,  // 2.06 bpw
    IQ2XS  = 30,  // 2.31 bpw
    IQ3XXS = 31,  // 3.06 bpw
    IQ1S   = 32,  // 1.56 bpw  (smallest possible)
    IQ4NL  = 33,  // 4.50 bpw  non-linear 4-bit
    IQ3S   = 34,  // 3.44 bpw
    IQ2S   = 35,  // 2.50 bpw
    IQ4XS  = 36,  // 4.25 bpw  (popular: Phi/Qwen imatrix)
    IQ1M   = 37,  // 1.75 bpw
    BF16   = 38,  // bfloat16
    Q4_0_4x4 = 39,
    Q4_0_4x8 = 40,
    Q4_0_8x8 = 41,
    TQ1_0  = 42,  // ternary 1.69 bpw
    TQ2_0  = 43,  // ternary 2.06 bpw
    #[allow(non_camel_case_types, dead_code)]
    IQ4_NL_4_4 = 44,
    #[allow(non_camel_case_types, dead_code)]
    IQ4_NL_4_8 = 45,
}

impl GgmlType {
    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            0  => Some(GgmlType::F32),  1  => Some(GgmlType::F16),
            2  => Some(GgmlType::Q4_0), 3  => Some(GgmlType::Q4_1),
            6  => Some(GgmlType::Q5_0), 7  => Some(GgmlType::Q5_1),
            8  => Some(GgmlType::Q8_0), 9  => Some(GgmlType::Q8_1),
            10 => Some(GgmlType::Q2K),
            // K-quants: GGUF spec type IDs 11-15 (one type per bitwidth, S/M/L are
            // the same block format — only differ in which tensors use which).
            11 => Some(GgmlType::Q3KS),   // Q3_K  (Q3_K_S / Q3_K_M / Q3_K_L)
            12 => Some(GgmlType::Q4KS),   // Q4_K  (Q4_K_S / Q4_K_M)
            13 => Some(GgmlType::Q5KS),   // Q5_K  (Q5_K_S / Q5_K_M)
            14 => Some(GgmlType::Q6K),    // Q6_K
            15 => Some(GgmlType::Q8K),    // Q8_K
            // IQ (imatrix importance-weighted) formats: GGUF type IDs 16-23
            16 => Some(GgmlType::IQ2XXS),
            17 => Some(GgmlType::IQ2XS),
            18 => Some(GgmlType::IQ3XXS),
            19 => Some(GgmlType::IQ1S),
            20 => Some(GgmlType::IQ4NL),
            21 => Some(GgmlType::IQ3S),
            22 => Some(GgmlType::IQ2S),
            23 => Some(GgmlType::IQ4XS),
            24 => Some(GgmlType::I8),   25 => Some(GgmlType::I16),
            26 => Some(GgmlType::I32),  27 => Some(GgmlType::I64),
            28 => Some(GgmlType::F64),
            29 => Some(GgmlType::IQ1M),
            30 => Some(GgmlType::BF16),
            31 => Some(GgmlType::Q4_0_4x4),
            32 => Some(GgmlType::Q4_0_4x8),
            33 => Some(GgmlType::Q4_0_8x8),
            34 => Some(GgmlType::TQ1_0),
            35 => Some(GgmlType::TQ2_0),
            _  => None,
        }
    }

    /// Raw bytes per block (the atomic unit this quantization format uses).
    pub fn bytes_per_block(&self) -> usize {
        match self {
            GgmlType::F32  => 4,
            GgmlType::F16  => 2,
            GgmlType::Q4_0 => 18,   // f16 scale + 16B nibbles
            GgmlType::Q4_1 => 20,   // f16 scale + f16 min + 16B nibbles
            GgmlType::Q5_0 => 22,   // f16 scale + 4B high-bits + 16B nibbles
            GgmlType::Q5_1 => 24,   // f16 scale + f16 min + 4B high-bits + 16B nibbles
            GgmlType::Q8_0 => 34,   // f16 scale + 32×i8
            GgmlType::Q8_1 => 36,   // 2×f16 + 32×i8
            // K-quants — 256 elements per super-block
            GgmlType::Q2K               => 84,   // 16+64+2+2
            GgmlType::Q3KS
            | GgmlType::Q3KM
            | GgmlType::Q3KL            => 110,  // 32+64+12+2
            GgmlType::Q4KS
            | GgmlType::Q4KM            => 144,  // 2+2+12+128
            GgmlType::Q5KS
            | GgmlType::Q5KM            => 176,  // 2+2+12+32+128
            GgmlType::Q6K               => 210,  // 128+64+16+2
            GgmlType::Q8K               => 292,  // 4+256+32
            // IQ formats — all use QK_K=256 super-blocks
            // Byte counts from ggml-quants.h sizeof(block_iq*)
            GgmlType::IQ2XXS            => 66,   // 2+64  (f16 d + 64 packed 2-bit)
            GgmlType::IQ2XS             => 74,   // 2+64+8 (d + u16 qs[32] + 8 scales)
            GgmlType::IQ2S              => 82,   // 2+64+8+8 (d + qs + qh + scales)
            GgmlType::IQ3XXS            => 98,   // 2+96  scrambled 3-bit grid
            GgmlType::IQ3S              => 110,  // 2+64+8+32+4 (d + qs + qh + signs + scales)
            GgmlType::IQ1S              => 50,   // 2+48  1.5-bit ternary grid
            GgmlType::IQ1M              => 56,   // no d field — scale embedded
            GgmlType::IQ4NL             => 18,   // 32-element block: f16 d + 16B nibbles
            GgmlType::IQ4XS             => 136,  // 2+2+128 with 4-bit sub-scales
            GgmlType::BF16              => 2,    // bfloat16 — 1 element per "block"
            GgmlType::Q4_0_4x4 | GgmlType::Q4_0_4x8 | GgmlType::Q4_0_8x8 => 18,
            GgmlType::TQ1_0             => 54,   // 2+52 ternary
            GgmlType::TQ2_0             => 66,   // 2+64 ternary
            GgmlType::IQ4_NL_4_4 | GgmlType::IQ4_NL_4_8 => 144,
            _                           => 4,
        }
    }

    /// `Some(reason)` if this type cannot be dequantized correctly. Verified types: F32 F16 BF16
    /// Q4_0 Q4_1 Q5_0 Q5_1 Q8_0 Q2_K Q3_K Q4_K Q5_K Q6_K Q8_K IQ4_NL IQ4_XS TQ2_0. The IQ1/IQ2/IQ3
    /// families (and TQ1_0) need codebook grids that are not verified against ggml reference
    /// data, so they error instead of producing garbage.
    pub fn unsupported_reason(&self) -> Option<String> {
        let name = match self {
            GgmlType::IQ2XXS => "IQ2_XXS", GgmlType::IQ2XS => "IQ2_XS", GgmlType::IQ2S => "IQ2_S",
            GgmlType::IQ3XXS => "IQ3_XXS", GgmlType::IQ3S => "IQ3_S",
            GgmlType::IQ1S => "IQ1_S", GgmlType::IQ1M => "IQ1_M",
            GgmlType::TQ1_0 => "TQ1_0",
            _ => "",
        };
        if !name.is_empty() {
            return Some(format!("unsupported quantization {}: lookup table not verified", name));
        }
        match self {
            GgmlType::Q8_1 | GgmlType::I8 | GgmlType::I16 | GgmlType::I32 | GgmlType::I64 | GgmlType::F64
            | GgmlType::Q4_0_4x4 | GgmlType::Q4_0_4x8 | GgmlType::Q4_0_8x8
            | GgmlType::IQ4_NL_4_4 | GgmlType::IQ4_NL_4_8 =>
                Some(format!("unsupported quantization {:?}: no dequantizer", self)),
            _ => None,
        }
    }

    /// Number of elements packed into one block.
    pub fn elements_per_block(&self) -> usize {
        match self {
            GgmlType::F32 | GgmlType::F16 => 1,
            // Legacy small-block formats: 32 elements per block
            GgmlType::Q4_0 | GgmlType::Q4_1
            | GgmlType::Q5_0 | GgmlType::Q5_1
            | GgmlType::Q8_0 | GgmlType::Q8_1 => 32,
            // BF16 and tiled Q4_0 variants: 1 element per block
            GgmlType::IQ4NL => 32,
            GgmlType::BF16
            | GgmlType::Q4_0_4x4 | GgmlType::Q4_0_4x8 | GgmlType::Q4_0_8x8 => 1,
            // All K-quant and IQ super-blocks use QK_K = 256
            _ => 256,
        }
    }
}

// --- Tensor descriptor --------------------------------------------------------

#[derive(Debug, Clone)]
pub struct TensorInfo {
    pub name:   String,
    pub shape:  Vec<u64>,  // dimensions (row-major)
    pub dtype:  GgmlType,
    pub offset: u64,       // byte offset within the tensor data section
}

impl TensorInfo {
    /// Element count, `None` if the shape product overflows u64.
    pub fn checked_n_elements(&self) -> Option<u64> {
        self.shape.iter().try_fold(1u64, |acc, &d| acc.checked_mul(d))
    }

    /// Byte size of the tensor data, `None` on arithmetic overflow.
    pub fn checked_n_bytes(&self) -> Option<u64> {
        let n = self.checked_n_elements()?;
        let bpb = self.dtype.bytes_per_block() as u64;
        let epb = self.dtype.elements_per_block() as u64;
        match self.dtype {
            GgmlType::F32 | GgmlType::F16 => n.checked_mul(bpb),
            _ => n.checked_add(epb - 1)?.checked_div(epb)?.checked_mul(bpb),
        }
    }

    /// Saturating element count (see `checked_n_elements`).
    pub fn n_elements(&self) -> u64 {
        self.checked_n_elements().unwrap_or(u64::MAX)
    }

    /// Saturating byte size (see `checked_n_bytes`).
    pub fn n_bytes(&self) -> u64 {
        self.checked_n_bytes().unwrap_or(u64::MAX)
    }
}

// --- Parsed GGUF file ---------------------------------------------------------

pub struct GgufFile {
    pub metadata:           HashMap<String, GgufValue>,
    pub tensors:            HashMap<String, TensorInfo>,
    pub tensor_data_offset: u64,  // absolute byte offset in file
    pub file_path:          String,
    pub n_tensors:          usize,
}

impl GgufFile {
    /// Parse GGUF header + metadata + tensor descriptors.
    /// Tensor data is NOT read into RAM — call `load_tensor_f32()` to load on demand.
    pub fn open(path: &str) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| format!("Cannot open '{}': {}", path, e))?;
        let file_len = file.metadata().map_err(|e| format!("Cannot stat '{}': {}", path, e))?.len();
        let mut r = BufReader::new(file);

        // Header
        let magic = read_u32(&mut r)?;
        if magic != GGUF_MAGIC {
            return Err(format!("Not a GGUF file (magic=0x{:08x})", magic));
        }
        let version = read_u32(&mut r)?;
        if version > GGUF_VERSION_MAX {
            return Err(format!("Unsupported GGUF version {} (max {})", version, GGUF_VERSION_MAX));
        }

        let n_tensors = read_u64(&mut r)? as usize;
        let n_kv      = read_u64(&mut r)? as usize;

        // Metadata
        // Untrusted counts: never pre-allocate more than the file could possibly hold
        // (every entry needs at least 8 bytes).
        let max_entries = (file_len / 8) as usize;
        let mut metadata = HashMap::with_capacity(n_kv.min(max_entries));
        for _ in 0..n_kv {
            let key = read_string(&mut r)?;
            let val = read_value(&mut r)?;
            metadata.insert(key, val);
        }

        // Tensor info
        let mut tensors = HashMap::with_capacity(n_tensors.min(max_entries));
        for _ in 0..n_tensors {
            let name   = read_string(&mut r)?;
            let n_dims = read_u32(&mut r)? as usize;
            if n_dims > MAX_TENSOR_DIMS {
                return Err(format!("Tensor '{}' has too many dimensions ({})", name, n_dims));
            }
            let mut shape = Vec::with_capacity(n_dims);
            for _ in 0..n_dims { shape.push(read_u64(&mut r)?); }
            let type_id = read_u32(&mut r)?;
            let dtype   = GgmlType::from_u32(type_id)
                .ok_or_else(|| format!("Unknown tensor type {} for '{}'", type_id, name))?;
            let offset = read_u64(&mut r)?;
            tensors.insert(name.clone(), TensorInfo { name, shape, dtype, offset });
        }

        // Data starts at the next multiple of alignment after current file position.
        // Alignment is either stored in metadata as `general.alignment` (uint32) or defaults to 32.
        let pos = r.seek(SeekFrom::Current(0)).map_err(|e| format!("Seek: {}", e))?;
        let alignment: u64 = match metadata.get("general.alignment") {
            Some(GgufValue::U32(v)) => *v as u64,
            Some(GgufValue::U64(v)) => *v,
            _ => 32,
        };
        if alignment == 0 {
            return Err("Invalid GGUF: general.alignment is 0".to_string());
        }
        let tensor_data_offset = pos
            .checked_add(alignment - 1)
            .map(|v| v / alignment * alignment)
            .ok_or_else(|| "Invalid GGUF: alignment overflow".to_string())?;

        // Validate every tensor against the real file size so later loads cannot
        // overflow or allocate attacker-controlled amounts of memory.
        for t in tensors.values() {
            let n_bytes = t.checked_n_bytes()
                .ok_or_else(|| format!("Tensor '{}' size overflows", t.name))?;
            let end = tensor_data_offset.checked_add(t.offset)
                .and_then(|v| v.checked_add(n_bytes))
                .ok_or_else(|| format!("Tensor '{}' offset overflows", t.name))?;
            if end > file_len {
                return Err(format!("Tensor '{}' extends past end of file", t.name));
            }
        }

        Ok(GgufFile {
            metadata,
            tensors,
            tensor_data_offset,
            file_path: path.to_string(),
            n_tensors,
        })
    }

    // -- Tensor loading ------------------------------------------------------

    /// Checked (absolute offset, byte length) of a tensor in the file.
    fn tensor_span(&self, info: &TensorInfo) -> Result<(u64, usize), String> {
        let n_bytes = info.checked_n_bytes()
            .ok_or_else(|| format!("Tensor '{}' size overflows", info.name))?;
        let abs = self.tensor_data_offset.checked_add(info.offset)
            .ok_or_else(|| format!("Tensor '{}' offset overflows", info.name))?;
        abs.checked_add(n_bytes)
            .ok_or_else(|| format!("Tensor '{}' extends past u64", info.name))?;
        let n = usize::try_from(n_bytes).map_err(|_| format!("Tensor '{}' too large", info.name))?;
        Ok((abs, n))
    }

    /// Load raw quantized bytes for a tensor — no dequantization.
    /// Returns `(raw_bytes, dtype, shape)`.  Use for large weight matrices
    /// that you want to keep compact and dequantize one row at a time.
    pub fn load_tensor_raw(&self, name: &str) -> Result<(Vec<u8>, GgmlType, Vec<u64>), String> {
        let info = self.tensors.get(name)
            .ok_or_else(|| format!("Tensor '{}' not found", name))?;
        if let Some(why) = info.dtype.unsupported_reason() {
            return Err(format!("Tensor '{}': {}", name, why));
        }
        let (abs_offset, n_bytes) = self.tensor_span(info)?;
        let mut file = File::open(&self.file_path)
            .map_err(|e| format!("Reopen '{}': {}", self.file_path, e))?;
        file.seek(SeekFrom::Start(abs_offset))
            .map_err(|e| format!("Seek for '{}': {}", name, e))?;
        let mut raw = vec![0u8; n_bytes];
        file.read_exact(&mut raw)
            .map_err(|e| format!("Read '{}': {}", name, e))?;
        Ok((raw, info.dtype, info.shape.clone()))
    }

    /// Load a tensor and dequantize to F32.
    pub fn load_tensor_f32(&self, name: &str) -> Result<Vec<f32>, String> {
        let info = self.tensors.get(name)
            .ok_or_else(|| format!("Tensor '{}' not found", name))?;

        if let Some(why) = info.dtype.unsupported_reason() {
            return Err(format!("Tensor '{}': {}", name, why));
        }
        let (abs_offset, n_bytes) = self.tensor_span(info)?;

        let mut file = File::open(&self.file_path)
            .map_err(|e| format!("Reopen '{}': {}", self.file_path, e))?;
        file.seek(SeekFrom::Start(abs_offset))
            .map_err(|e| format!("Seek for '{}': {}", name, e))?;
        let mut raw = vec![0u8; n_bytes];
        file.read_exact(&mut raw)
            .map_err(|e| format!("Read '{}': {}", name, e))?;

        use super::quant;
        let n_elem = info.n_elements() as usize;

        match info.dtype {
            GgmlType::F32  => Ok(raw.chunks_exact(4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .collect()),
            GgmlType::F16  => Ok(raw.chunks_exact(2)
                .map(|b| quant::f16_to_f32(u16::from_le_bytes([b[0], b[1]])))
                .collect()),
            GgmlType::Q8_0              => Ok(quant::dequant_q8_0(&raw, n_elem)),
            GgmlType::Q4_0              => Ok(quant::dequant_q4_0(&raw, n_elem)),
            GgmlType::Q4_1              => Ok(quant::dequant_q4_1(&raw, n_elem)),
            GgmlType::Q5_0              => Ok(quant::dequant_q5_0(&raw, n_elem)),
            GgmlType::Q5_1              => Ok(quant::dequant_q5_1(&raw, n_elem)),
            GgmlType::Q2K               => Ok(quant::dequant_q2_k(&raw, n_elem)),
            GgmlType::Q3KS | GgmlType::Q3KM | GgmlType::Q3KL
                                        => Ok(quant::dequant_q3_k(&raw, n_elem)),
            GgmlType::Q4KS | GgmlType::Q4KM => Ok(quant::dequant_q4_k(&raw, n_elem)),
            GgmlType::Q5KS | GgmlType::Q5KM => Ok(quant::dequant_q5_k(&raw, n_elem)),
            GgmlType::Q6K               => Ok(quant::dequant_q6_k(&raw, n_elem)),
            GgmlType::Q8K               => Ok(quant::dequant_q8_k(&raw, n_elem)),
            GgmlType::BF16              => Ok(quant::dequant_bf16(&raw, n_elem)),
            GgmlType::IQ4NL             => Ok(quant::dequant_iq4_nl(&raw, n_elem)),
            GgmlType::IQ4XS             => Ok(quant::dequant_iq4_xs(&raw, n_elem)),
            GgmlType::TQ2_0             => Ok(quant::dequant_tq2_0(&raw, n_elem)),
            t => Err(format!(
                "Tensor '{}' uses unsupported type {:?}.",
                name, t
            )),
        }
    }

    // -- Metadata helpers ----------------------------------------------------

    pub fn meta_str(&self, key: &str) -> Option<String> {
        match self.metadata.get(key)? {
            GgufValue::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    pub fn meta_u64(&self, key: &str) -> Option<u64> {
        match self.metadata.get(key)? {
            GgufValue::U32(v) => Some(*v as u64),
            GgufValue::U64(v) => Some(*v),
            GgufValue::I32(v) => Some(*v as u64),
            GgufValue::I64(v) => Some(*v as u64),
            _ => None,
        }
    }

    pub fn meta_f32(&self, key: &str) -> Option<f32> {
        match self.metadata.get(key)? {
            GgufValue::F32(v) => Some(*v),
            GgufValue::F64(v) => Some(*v as f32),
            _ => None,
        }
    }

    pub fn meta_array_strings(&self, key: &str) -> Vec<String> {
        match self.metadata.get(key) {
            Some(GgufValue::Array(arr)) => arr.iter().filter_map(|v| {
                if let GgufValue::String(s) = v { Some(s.clone()) } else { None }
            }).collect(),
            _ => Vec::new(),
        }
    }

    pub fn meta_array_f32(&self, key: &str) -> Vec<f32> {
        match self.metadata.get(key) {
            Some(GgufValue::Array(arr)) => arr.iter().filter_map(|v| {
                if let GgufValue::F32(f) = v { Some(*f) } else { None }
            }).collect(),
            _ => Vec::new(),
        }
    }

    pub fn meta_array_u32(&self, key: &str) -> Vec<u32> {
        match self.metadata.get(key) {
            Some(GgufValue::Array(arr)) => arr.iter().filter_map(|v| {
                match v {
                    GgufValue::U32(x) => Some(*x),
                    GgufValue::I32(x) => Some(*x as u32),
                    _ => None,
                }
            }).collect(),
            _ => Vec::new(),
        }
    }

    pub fn has_tensor(&self, name: &str) -> bool { self.tensors.contains_key(name) }
    pub fn tensor_names(&self) -> Vec<&str>       { self.tensors.keys().map(|s| s.as_str()).collect() }
}

// --- Binary read helpers ------------------------------------------------------

fn read_u8<R: Read>(r: &mut R) -> Result<u8, String> {
    let mut b = [0u8; 1];
    r.read_exact(&mut b).map_err(|e| format!("read_u8: {}", e))?;
    Ok(b[0])
}
fn read_u16<R: Read>(r: &mut R) -> Result<u16, String> {
    let mut b = [0u8; 2];
    r.read_exact(&mut b).map_err(|e| format!("read_u16: {}", e))?;
    Ok(u16::from_le_bytes(b))
}
fn read_u32<R: Read>(r: &mut R) -> Result<u32, String> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b).map_err(|e| format!("read_u32: {}", e))?;
    Ok(u32::from_le_bytes(b))
}
fn read_u64<R: Read>(r: &mut R) -> Result<u64, String> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b).map_err(|e| format!("read_u64: {}", e))?;
    Ok(u64::from_le_bytes(b))
}
fn read_i8<R: Read>(r: &mut R)  -> Result<i8,  String> { Ok(read_u8(r)?  as i8)  }
fn read_i16<R: Read>(r: &mut R) -> Result<i16, String> { Ok(read_u16(r)? as i16) }
fn read_i32<R: Read>(r: &mut R) -> Result<i32, String> { Ok(read_u32(r)? as i32) }
fn read_i64<R: Read>(r: &mut R) -> Result<i64, String> { Ok(read_u64(r)? as i64) }
fn read_f32<R: Read>(r: &mut R) -> Result<f32, String> { Ok(f32::from_bits(read_u32(r)?)) }
fn read_f64<R: Read>(r: &mut R) -> Result<f64, String> { Ok(f64::from_bits(read_u64(r)?)) }
fn read_bool<R: Read>(r: &mut R) -> Result<bool, String> { Ok(read_u8(r)? != 0) }

fn read_string<R: Read>(r: &mut R) -> Result<String, String> {
    let len = read_u64(r)?;
    if len > 4 * 1024 * 1024 {
        return Err(format!("String too long ({} bytes) — corrupt file?", len));
    }
    // read through `take` so a bogus length cannot force a large up-front allocation
    let mut bytes = Vec::new();
    Read::take(&mut *r, len).read_to_end(&mut bytes).map_err(|e| format!("read_string data: {}", e))?;
    if bytes.len() as u64 != len {
        return Err("read_string data: unexpected end of file".to_string());
    }
    String::from_utf8(bytes).map_err(|e| format!("String UTF-8: {}", e))
}

fn read_value<R: Read>(r: &mut R) -> Result<GgufValue, String> {
    let type_id = read_u32(r)?;
    read_typed_value(r, type_id, 0)
}

fn read_typed_value<R: Read>(r: &mut R, type_id: u32, depth: usize) -> Result<GgufValue, String> {
    match type_id {
        0  => Ok(GgufValue::U8(read_u8(r)?)),
        1  => Ok(GgufValue::I8(read_i8(r)?)),
        2  => Ok(GgufValue::U16(read_u16(r)?)),
        3  => Ok(GgufValue::I16(read_i16(r)?)),
        4  => Ok(GgufValue::U32(read_u32(r)?)),
        5  => Ok(GgufValue::I32(read_i32(r)?)),
        6  => Ok(GgufValue::F32(read_f32(r)?)),
        7  => Ok(GgufValue::Bool(read_bool(r)?)),
        8  => Ok(GgufValue::String(read_string(r)?)),
        9  => {
            if depth >= MAX_ARRAY_DEPTH {
                return Err("GGUF array nesting too deep".to_string());
            }
            let elem_type = read_u32(r)?;
            let count     = read_u64(r)? as usize;
            let cap = count.min(4096); // guard against corrupt; grows as data is really read
            let mut arr = Vec::with_capacity(cap);
            for _ in 0..count { arr.push(read_typed_value(r, elem_type, depth + 1)?); }
            Ok(GgufValue::Array(arr))
        }
        10 => Ok(GgufValue::U64(read_u64(r)?)),
        11 => Ok(GgufValue::I64(read_i64(r)?)),
        12 => Ok(GgufValue::F64(read_f64(r)?)),
        t  => Err(format!("Unknown GGUF value type_id: {}", t)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr(n_tensors: u64, n_kv: u64) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&GGUF_MAGIC.to_le_bytes());
        b.extend_from_slice(&3u32.to_le_bytes());
        b.extend_from_slice(&n_tensors.to_le_bytes());
        b.extend_from_slice(&n_kv.to_le_bytes());
        b
    }
    fn gstr(b: &mut Vec<u8>, s: &str) {
        b.extend_from_slice(&(s.len() as u64).to_le_bytes());
        b.extend_from_slice(s.as_bytes());
    }
    fn open_bytes(tag: &str, bytes: &[u8]) -> Result<GgufFile, String> {
        let p = std::env::temp_dir().join(format!("killer_gguf_{}_{}.gguf", tag, std::process::id()));
        std::fs::write(&p, bytes).unwrap();
        let r = GgufFile::open(p.to_str().unwrap());
        let _ = std::fs::remove_file(&p);
        r
    }

    #[test]
    fn rejects_zero_alignment() {
        let mut b = hdr(0, 1);
        gstr(&mut b, "general.alignment");
        b.extend_from_slice(&4u32.to_le_bytes()); // U32
        b.extend_from_slice(&0u32.to_le_bytes());
        assert!(open_bytes("align0", &b).is_err());
    }

    #[test]
    fn rejects_overflowing_tensor_shape() {
        let mut b = hdr(1, 0);
        gstr(&mut b, "t");
        b.extend_from_slice(&2u32.to_le_bytes());
        b.extend_from_slice(&(1u64 << 40).to_le_bytes());
        b.extend_from_slice(&(1u64 << 40).to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes()); // F32
        b.extend_from_slice(&0u64.to_le_bytes());
        assert!(open_bytes("shape", &b).is_err());
    }

    #[test]
    fn rejects_tensor_past_eof_and_huge_counts() {
        let mut b = hdr(1, 0);
        gstr(&mut b, "t");
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&1_000_000u64.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&u64::MAX.to_le_bytes()); // offset overflow
        assert!(open_bytes("eof", &b).is_err());
        assert!(open_bytes("counts", &hdr(u64::MAX, u64::MAX)).is_err());
    }

    #[test]
    fn rejects_deeply_nested_arrays() {
        let mut b = hdr(0, 1);
        gstr(&mut b, "k");
        b.extend_from_slice(&9u32.to_le_bytes());
        for _ in 0..10_000 {
            b.extend_from_slice(&9u32.to_le_bytes()); // element type: array
            b.extend_from_slice(&1u64.to_le_bytes()); // count 1
        }
        assert!(open_bytes("deep", &b).is_err());
    }

    #[test]
    fn test_tensor_n_bytes_f32() {
        let t = TensorInfo {
            name: "w".to_string(), shape: vec![4, 8],
            dtype: GgmlType::F32, offset: 0,
        };
        assert_eq!(t.n_bytes(), 4 * 8 * 4);
    }

    #[test]
    fn test_tensor_n_bytes_q8_0() {
        let t = TensorInfo {
            name: "w".to_string(), shape: vec![32],
            dtype: GgmlType::Q8_0, offset: 0,
        };
        // 32 elements = 1 block = 34 bytes
        assert_eq!(t.n_bytes(), 34);
    }

    #[test]
    fn test_tensor_n_bytes_q4_0() {
        let t = TensorInfo {
            name: "w".to_string(), shape: vec![64],
            dtype: GgmlType::Q4_0, offset: 0,
        };
        // 64 elements = 2 blocks × 18 bytes = 36
        assert_eq!(t.n_bytes(), 36);
    }
}

#[cfg(test)]
mod type_tests {
    use super::*;

    #[test]
    fn unverified_iq_types_error_and_sizes_match_ggml() {
        for t in [GgmlType::IQ2XXS, GgmlType::IQ2XS, GgmlType::IQ2S, GgmlType::IQ3XXS, GgmlType::IQ3S, GgmlType::IQ1S, GgmlType::IQ1M] {
            let r = t.unsupported_reason().expect("must be unsupported");
            assert!(r.contains("lookup table not verified"), "{}", r);
        }
        assert!(GgmlType::IQ4XS.unsupported_reason().is_none() && GgmlType::Q2K.unsupported_reason().is_none());
        let sizes = [(GgmlType::IQ2XXS, 66), (GgmlType::IQ2XS, 74), (GgmlType::IQ2S, 82), (GgmlType::IQ3XXS, 98),
                     (GgmlType::IQ3S, 110), (GgmlType::IQ4XS, 136), (GgmlType::Q2K, 84), (GgmlType::Q3KS, 110)];
        for (t, n) in sizes { assert_eq!(t.bytes_per_block(), n, "{:?}", t); }
    }
}
