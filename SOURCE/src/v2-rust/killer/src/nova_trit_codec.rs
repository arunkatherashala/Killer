//! Trit **packing** codec — **separate** from `nova_compress` / NOVZ (LZ77+Huffman).
//!
//! - **NOVT** = fixed **2 bits per trit** (~4× smaller than raw `i8` payload).
//! - **NOVD** = **dense** packing: one minimal big-endian integer over base 3 (maps trits to digits `0,1,2`),
//!   averaging **~log₂(3) ≈ 1.585 bits/trit** on the payload (information-theoretic minimum for i.i.d. uniform trits on binary media).
//! - **Stacking compressors:** dense or fixed-width trit blobs are still **bytes**. If the data (or headers) have
//!   repetition, run **`nova_compress` / NOVZ** on that file afterward for another shrink; high-entropy packed trits may grow slightly (NOVZ overhead).
//! - Does **not** call [`crate::nova::nova_compress`] from this module; keep codec pure — compose at the call site (see `csv_format_compare` binary).

use crate::error::VmError;

/// Magic: Nova Trit packed blob (not NOVZ).
pub const NOVT_MAGIC: &[u8; 4] = b"NOVT";
/// Magic: Nova Trit **dense** base-3 integer blob (same logical trits as NOVT, tighter encoding).
pub const NOVD_MAGIC: &[u8; 4] = b"NOVD";
const NOVT_VERSION: u8 = 1;
const NOVD_VERSION: u8 = 1;
const HEADER_LEN: usize = 4 + 1 + 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NovaTritCodecError {
    InvalidTrit(i8),
    InvalidPayloadByte(u8),
    BadMagic,
    UnsupportedVersion(u8),
    Truncated,
    PayloadTritMismatch,
}

impl std::fmt::Display for NovaTritCodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTrit(t) => write!(f, "trit must be -1, 0, or 1, got {t}"),
            Self::InvalidPayloadByte(b) => write!(f, "invalid 2-bit trit code {b} (expected 0–2)"),
            Self::BadMagic => write!(f, "not a NOVT blob (bad magic)"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported NOVT version {v}"),
            Self::Truncated => write!(f, "truncated NOVT blob"),
            Self::PayloadTritMismatch => write!(f, "NOVD payload does not match trit count (corrupt or trimmed)"),
        }
    }
}

impl std::error::Error for NovaTritCodecError {}

impl From<NovaTritCodecError> for VmError {
    fn from(e: NovaTritCodecError) -> Self {
        VmError::runtime_error(e.to_string())
    }
}

#[inline]
fn trit_to_2bit(t: i8) -> Result<u8, NovaTritCodecError> {
    match t {
        -1 => Ok(0),
        0 => Ok(1),
        1 => Ok(2),
        _ => Err(NovaTritCodecError::InvalidTrit(t)),
    }
}

#[inline]
fn trit_from_2bit(b: u8) -> Result<i8, NovaTritCodecError> {
    match b & 3 {
        0 => Ok(-1),
        1 => Ok(0),
        2 => Ok(1),
        _ => Err(NovaTritCodecError::InvalidPayloadByte(b & 3)),
    }
}

/// Pack trits into a **NOVT** blob: `magic | version | trit_count u64 LE | packed bits`.
pub fn pack_trits_novt(trits: &[i8]) -> Result<Vec<u8>, NovaTritCodecError> {
    let mut packed: Vec<u8> = Vec::new();
    let mut cur = 0u8;
    let mut bits_in_cur = 0u8;

    for &t in trits {
        let enc = trit_to_2bit(t)?;
        cur |= enc << bits_in_cur;
        bits_in_cur += 2;
        if bits_in_cur == 8 {
            packed.push(cur);
            cur = 0;
            bits_in_cur = 0;
        }
    }
    if bits_in_cur > 0 {
        packed.push(cur);
    }

    let mut blob = Vec::with_capacity(HEADER_LEN + packed.len());
    blob.extend_from_slice(NOVT_MAGIC);
    blob.push(NOVT_VERSION);
    blob.extend_from_slice(&(trits.len() as u64).to_le_bytes());
    blob.extend_from_slice(&packed);
    Ok(blob)
}

/// Decode a **NOVT** blob (see [`pack_trits_novt`]). Returns only the `trit_count` trits from the header.
pub fn unpack_novt_trits(blob: &[u8]) -> Result<Vec<i8>, NovaTritCodecError> {
    if blob.len() < HEADER_LEN {
        return Err(NovaTritCodecError::Truncated);
    }
    if blob[0..4] != *NOVT_MAGIC {
        return Err(NovaTritCodecError::BadMagic);
    }
    if blob[4] != NOVT_VERSION {
        return Err(NovaTritCodecError::UnsupportedVersion(blob[4]));
    }
    let trit_count = u64::from_le_bytes(blob[5..13].try_into().map_err(|_| NovaTritCodecError::Truncated)?) as usize;
    let payload = &blob[HEADER_LEN..];
    let need_bits = trit_count.saturating_mul(2);
    let need_bytes = (need_bits + 7) / 8;
    if payload.len() < need_bytes {
        return Err(NovaTritCodecError::Truncated);
    }

    let mut out = Vec::with_capacity(trit_count);
    for i in 0..trit_count {
        let bit_start = i * 2;
        let b0 = bit_start / 8;
        let off = bit_start % 8;
        let two = if off <= 6 {
            (payload[b0] >> off) & 3
        } else {
            let part0 = (payload[b0] >> 7) & 1;
            let part1 = payload.get(b0 + 1).map(|x| x & 1).unwrap_or(0);
            part0 | (part1 << 1)
        };
        out.push(trit_from_2bit(two)?);
    }
    Ok(out)
}

// -- NOVD: Horner on base 3, arbitrary-precision LE bytes -------------------------------------

#[inline]
fn le_mul_add(v: &mut Vec<u8>, mul: u64, add: u64) {
    let mut carry: u128 = add as u128;
    for byte in v.iter_mut() {
        carry += (*byte as u128) * (mul as u128);
        *byte = (carry & 0xff) as u8;
        carry >>= 8;
    }
    while carry > 0 {
        v.push((carry & 0xff) as u8);
        carry >>= 8;
    }
}

/// LE base-256 → minimal big-endian (MSB first, no leading zero; `[]` if all zero).
fn le_to_minimal_be(le: &[u8]) -> Vec<u8> {
    let mut end = le.len();
    while end > 0 && le[end - 1] == 0 {
        end -= 1;
    }
    if end == 0 {
        return Vec::new();
    }
    let mut be = Vec::with_capacity(end);
    for i in (0..end).rev() {
        be.push(le[i]);
    }
    be
}

fn be_to_le_for_value(be: &[u8]) -> Vec<u8> {
    if be.is_empty() {
        return Vec::new();
    }
    let mut le = Vec::with_capacity(be.len());
    for &b in be.iter().rev() {
        le.push(b);
    }
    while le.len() > 1 && *le.last().unwrap() == 0 {
        le.pop();
    }
    le
}

fn le_divmod_u32(le: &mut Vec<u8>, divisor: u32) -> u32 {
    let mut rem = 0u64;
    if le.is_empty() {
        return 0;
    }
    for i in (0..le.len()).rev() {
        let cur = rem * 256 + le[i] as u64;
        le[i] = (cur / divisor as u64) as u8;
        rem = cur % divisor as u64;
    }
    while le.len() > 1 && *le.last().unwrap() == 0 {
        le.pop();
    }
    rem as u32
}

fn le_is_zero(le: &[u8]) -> bool {
    le.is_empty() || le.iter().all(|&b| b == 0)
}

/// Pack trits into **NOVD**: same semantics as NOVT, payload **≈ ceil(n · log₂(3)) / 8** bytes (asymptotic).
///
/// Trit order: `trits[0]` is the least significant base-3 digit (same reconstruction order as iterating unpack).
pub fn pack_trits_novd(trits: &[i8]) -> Result<Vec<u8>, NovaTritCodecError> {
    let mut acc: Vec<u8> = vec![0];
    for &t in trits.iter().rev() {
        let d = trit_to_2bit(t)? as u64;
        le_mul_add(&mut acc, 3, d);
    }
    let payload = le_to_minimal_be(&acc);
    let mut blob = Vec::with_capacity(HEADER_LEN + payload.len());
    blob.extend_from_slice(NOVD_MAGIC);
    blob.push(NOVD_VERSION);
    blob.extend_from_slice(&(trits.len() as u64).to_le_bytes());
    blob.extend_from_slice(&payload);
    Ok(blob)
}

/// Decode a **NOVD** blob from [`pack_trits_novd`].
pub fn unpack_novd_trits(blob: &[u8]) -> Result<Vec<i8>, NovaTritCodecError> {
    if blob.len() < HEADER_LEN {
        return Err(NovaTritCodecError::Truncated);
    }
    if blob[0..4] != *NOVD_MAGIC {
        return Err(NovaTritCodecError::BadMagic);
    }
    if blob[4] != NOVD_VERSION {
        return Err(NovaTritCodecError::UnsupportedVersion(blob[4]));
    }
    let trit_count = u64::from_le_bytes(blob[5..13].try_into().map_err(|_| NovaTritCodecError::Truncated)?) as usize;
    let payload = &blob[HEADER_LEN..];

    let mut le = be_to_le_for_value(payload);
    if le.is_empty() {
        le.push(0);
    }
    let mut out = Vec::with_capacity(trit_count);
    for _ in 0..trit_count {
        let r = le_divmod_u32(&mut le, 3);
        debug_assert!(r <= 2);
        out.push(trit_from_2bit(r as u8)?);
    }
    if !le_is_zero(&le) {
        return Err(NovaTritCodecError::PayloadTritMismatch);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_empty() {
        let a: [i8; 0] = [];
        let b = pack_trits_novt(&a).unwrap();
        assert_eq!(unpack_novt_trits(&b).unwrap(), vec![]);
    }

    #[test]
    fn round_trip_single_and_few() {
        for t in [-1i8, 0, 1] {
            let b = pack_trits_novt(&[t]).unwrap();
            assert_eq!(unpack_novt_trits(&b).unwrap(), vec![t]);
        }
        let v = vec![1i8, 0, -1, 1, 1];
        let b = pack_trits_novt(&v).unwrap();
        assert_eq!(unpack_novt_trits(&b).unwrap(), v);
    }

    #[test]
    fn round_trip_non_multiple_of_four_len() {
        let v: Vec<i8> = (0..19).map(|i| match i % 3 {
            0 => -1,
            1 => 0,
            _ => 1,
        }).collect();
        let b = pack_trits_novt(&v).unwrap();
        assert_eq!(unpack_novt_trits(&b).unwrap(), v);
    }

    #[test]
    fn reject_invalid_trit() {
        assert!(pack_trits_novt(&[2]).is_err());
        assert!(pack_trits_novt(&[-2]).is_err());
    }

    #[test]
    fn reject_bad_magic() {
        let mut b = pack_trits_novt(&[1, -1]).unwrap();
        b[0] = b'X';
        assert_eq!(unpack_novt_trits(&b), Err(NovaTritCodecError::BadMagic));
    }

    #[test]
    fn novd_round_trip_empty_and_few() {
        let a: [i8; 0] = [];
        let b = pack_trits_novd(&a).unwrap();
        assert_eq!(&b[0..4], NOVD_MAGIC.as_slice());
        assert_eq!(unpack_novd_trits(&b).unwrap(), vec![]);

        for t in [-1i8, 0, 1] {
            let b = pack_trits_novd(&[t]).unwrap();
            assert_eq!(unpack_novd_trits(&b).unwrap(), vec![t]);
        }
        let v = vec![1i8, 0, -1, 1, 1];
        let b = pack_trits_novd(&v).unwrap();
        assert_eq!(unpack_novd_trits(&b).unwrap(), v);
    }

    #[test]
    fn novd_round_trip_long_pattern() {
        let v: Vec<i8> = (0..500)
            .map(|i| match i % 3 {
                0 => -1i8,
                1 => 0,
                _ => 1,
            })
            .collect();
        let b = pack_trits_novd(&v).unwrap();
        assert_eq!(unpack_novd_trits(&b).unwrap(), v);
        let novt = pack_trits_novt(&v).unwrap();
        assert!(
            b.len() < novt.len(),
            "NOVD should be smaller than NOVT for this length: novd={} novt={}",
            b.len(),
            novt.len()
        );
    }

    #[test]
    fn novd_tighter_than_novt_meta() {
        // 10 trits: NOVT payload ceil(20/8)=3 bytes + header; NOVD ceil(log2(3^10)/8)=ceil(15.85/8)=2 payload bytes
        let v: Vec<i8> = (0..10).map(|i| [-1i8, 0, 1][i % 3]).collect();
        let d = pack_trits_novd(&v).unwrap();
        let t = pack_trits_novt(&v).unwrap();
        assert!(d.len() <= t.len());
    }
}
