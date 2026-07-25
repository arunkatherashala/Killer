/// NaN-boxing for Killer VM hot-path scalar values.
///
/// A 64-bit IEEE 754 double has space in its NaN payload.
/// We use the "NaN-tagging" trick used by LuaJIT, JavaScriptCore, etc.
///
/// Layout:
///   Normal f64:    any value where bits [63..51] are NOT all set to NaN pattern
///   NaN-tagged:    bits [63..51] = 0x7FF8 (quiet NaN marker)
///                 bits [50..48] = 3-bit TYPE TAG
///                 bits [47.. 0] = payload (48 bits)
///
/// Type tags:
///   TAG_FLOAT  = 0  — not used (normal f64 is never a NaN-tagged value)
///   TAG_BOOL   = 1  — payload: 0 = false, 1 = true
///   TAG_NULL   = 2  — payload: 0
///   TAG_TRIT   = 3  — payload: 0xFF = -1 (T_NEG), 0x00 = 0 (T_ZERO), 0x01 = 1 (T_POS)
///   TAG_INT    = 4  — payload: i32 as two's complement in low 32 bits
///   TAG_HEAP   = 5  — payload: index into heap (Arc<HeapValue>)
///
/// A value that is a real f64 (not NaN, or not our sentinel NaN) is stored directly.
///
/// Performance:
///   copy:   8 bytes, stack-allocated (like f64 itself)
///   clone:  memcpy 8 bytes — no heap allocation for scalars
///   match:  bit-extract tag — 1-2 CPU instructions

const NAN_MASK: u64  = 0x7FF8_0000_0000_0000; // quiet NaN mask
const TAG_BITS: u64  = 0x0007_0000_0000_0000; // 3 tag bits at [50..48]
const PAY_MASK: u64  = 0x0000_FFFF_FFFF_FFFF; // 48 payload bits

const TAG_BOOL: u64  = 1;
const TAG_NULL: u64  = 2;
const TAG_TRIT: u64  = 3;
const TAG_INT:  u64  = 4;

/// A scalar value that fits in 8 bytes with zero heap allocation.
#[derive(Copy, Clone, PartialEq)]
#[repr(transparent)]
pub struct NanBox(u64);

impl NanBox {
    // ---- constructors -------------------------------------------------------

    #[inline(always)]
    pub fn from_f64(v: f64) -> Self {
        let bits = v.to_bits();
        // If the bits look like our sentinel NaN pattern, store as is — the
        // consumer checks via is_tagged() first.
        NanBox(bits)
    }

    #[inline(always)]
    pub fn from_bool(b: bool) -> Self {
        NanBox(NAN_MASK | (TAG_BOOL << 48) | (b as u64))
    }

    #[inline(always)]
    pub fn null() -> Self {
        NanBox(NAN_MASK | (TAG_NULL << 48))
    }

    #[inline(always)]
    pub fn from_trit(t: i8) -> Self {
        // store trit as u8: 0xFF=-1, 0x00=0, 0x01=+1
        let pay = (t as u8) as u64;
        NanBox(NAN_MASK | (TAG_TRIT << 48) | pay)
    }

    #[inline(always)]
    pub fn from_i32(v: i32) -> Self {
        NanBox(NAN_MASK | (TAG_INT << 48) | ((v as u32) as u64))
    }

    // ---- tag detection ------------------------------------------------------

    #[inline(always)]
    fn is_tagged(self) -> bool {
        // A tagged value has the quiet-NaN pattern in bits [63..51]
        (self.0 & NAN_MASK) == NAN_MASK
    }

    #[inline(always)]
    fn tag(self) -> u64 {
        (self.0 & TAG_BITS) >> 48
    }

    #[inline(always)]
    fn payload(self) -> u64 {
        self.0 & PAY_MASK
    }

    // ---- type queries -------------------------------------------------------

    #[inline(always)]
    pub fn is_number(self) -> bool { !self.is_tagged() }

    #[inline(always)]
    pub fn is_bool(self) -> bool  { self.is_tagged() && self.tag() == TAG_BOOL }

    #[inline(always)]
    pub fn is_null(self) -> bool  { self.is_tagged() && self.tag() == TAG_NULL }

    #[inline(always)]
    pub fn is_trit(self) -> bool  { self.is_tagged() && self.tag() == TAG_TRIT }

    #[inline(always)]
    pub fn is_int(self) -> bool   { self.is_tagged() && self.tag() == TAG_INT  }

    // ---- extractors ---------------------------------------------------------

    #[inline(always)]
    pub fn as_f64(self) -> f64 {
        debug_assert!(self.is_number());
        f64::from_bits(self.0)
    }

    #[inline(always)]
    pub fn as_bool(self) -> bool {
        debug_assert!(self.is_bool());
        self.payload() != 0
    }

    #[inline(always)]
    pub fn as_trit(self) -> i8 {
        debug_assert!(self.is_trit());
        self.payload() as u8 as i8
    }

    #[inline(always)]
    pub fn as_i32(self) -> i32 {
        debug_assert!(self.is_int());
        self.payload() as u32 as i32
    }

    // ---- arithmetic fast-paths (number only) --------------------------------

    /// Add two NanBox values. Both must be numbers (f64).
    /// Returns None if not both numbers — caller should fall back.
    #[inline(always)]
    pub fn try_add(self, rhs: NanBox) -> Option<NanBox> {
        if self.is_number() && rhs.is_number() {
            Some(NanBox::from_f64(self.as_f64() + rhs.as_f64()))
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn try_sub(self, rhs: NanBox) -> Option<NanBox> {
        if self.is_number() && rhs.is_number() {
            Some(NanBox::from_f64(self.as_f64() - rhs.as_f64()))
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn try_mul(self, rhs: NanBox) -> Option<NanBox> {
        if self.is_number() && rhs.is_number() {
            Some(NanBox::from_f64(self.as_f64() * rhs.as_f64()))
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn try_lt(self, rhs: NanBox) -> Option<NanBox> {
        if self.is_number() && rhs.is_number() {
            Some(NanBox::from_bool(self.as_f64() < rhs.as_f64()))
        } else {
            None
        }
    }

    // ---- trit fast-paths ----------------------------------------------------

    #[inline(always)]
    pub fn trit_and(self, rhs: NanBox) -> Option<NanBox> {
        if self.is_trit() && rhs.is_trit() {
            Some(NanBox::from_trit(self.as_trit().min(rhs.as_trit())))
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn trit_or(self, rhs: NanBox) -> Option<NanBox> {
        if self.is_trit() && rhs.is_trit() {
            Some(NanBox::from_trit(self.as_trit().max(rhs.as_trit())))
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn trit_not(self) -> Option<NanBox> {
        if self.is_trit() {
            Some(NanBox::from_trit(-self.as_trit()))
        } else {
            None
        }
    }

    // ---- fuzzy fast-paths (f64 in [0,1]) ------------------------------------

    #[inline(always)]
    pub fn fuzzy_and(self, rhs: NanBox) -> Option<NanBox> {
        if self.is_number() && rhs.is_number() {
            let a = self.as_f64().clamp(0.0, 1.0);
            let b = rhs.as_f64().clamp(0.0, 1.0);
            Some(NanBox::from_f64(a.min(b)))
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn fuzzy_or(self, rhs: NanBox) -> Option<NanBox> {
        if self.is_number() && rhs.is_number() {
            let a = self.as_f64().clamp(0.0, 1.0);
            let b = rhs.as_f64().clamp(0.0, 1.0);
            Some(NanBox::from_f64(a.max(b)))
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn fuzzy_not(self) -> Option<NanBox> {
        if self.is_number() {
            Some(NanBox::from_f64(1.0 - self.as_f64().clamp(0.0, 1.0)))
        } else {
            None
        }
    }

    // ---- truthiness ---------------------------------------------------------

    #[inline(always)]
    pub fn is_truthy(self) -> bool {
        if self.is_number()  { return self.as_f64() != 0.0; }
        if self.is_bool()    { return self.as_bool(); }
        if self.is_null()    { return false; }
        if self.is_trit()    { return self.as_trit() > 0; }
        if self.is_int()     { return self.as_i32() != 0; }
        true // heap types are truthy
    }
}

impl std::fmt::Debug for NanBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_number()     { write!(f, "NanBox(f64={})", self.as_f64()) }
        else if self.is_bool()  { write!(f, "NanBox(bool={})", self.as_bool()) }
        else if self.is_null()  { write!(f, "NanBox(null)") }
        else if self.is_trit()  { write!(f, "NanBox(trit={})", self.as_trit()) }
        else if self.is_int()   { write!(f, "NanBox(i32={})", self.as_i32()) }
        else                    { write!(f, "NanBox(heap={:#018x})", self.0) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nanbox_size() {
        assert_eq!(std::mem::size_of::<NanBox>(), 8);
    }

    #[test]
    fn test_round_trip() {
        assert_eq!(NanBox::from_f64(42.0).as_f64(), 42.0);
        assert_eq!(NanBox::from_f64(0.0).as_f64(), 0.0);
        assert_eq!(NanBox::from_bool(true).as_bool(), true);
        assert_eq!(NanBox::from_bool(false).as_bool(), false);
        assert!(NanBox::null().is_null());
        assert_eq!(NanBox::from_trit(1).as_trit(), 1);
        assert_eq!(NanBox::from_trit(-1).as_trit(), -1);
        assert_eq!(NanBox::from_trit(0).as_trit(), 0);
        assert_eq!(NanBox::from_i32(12345).as_i32(), 12345);
        assert_eq!(NanBox::from_i32(-1).as_i32(), -1);
    }

    #[test]
    fn test_type_detection() {
        assert!(NanBox::from_f64(1.5).is_number());
        assert!(!NanBox::from_f64(1.5).is_bool());
        assert!(NanBox::from_bool(true).is_bool());
        assert!(!NanBox::from_bool(true).is_number());
        assert!(NanBox::null().is_null());
        assert!(NanBox::from_trit(1).is_trit());
    }

    #[test]
    fn test_arithmetic() {
        let a = NanBox::from_f64(3.0);
        let b = NanBox::from_f64(4.0);
        assert_eq!(a.try_add(b).unwrap().as_f64(), 7.0);
        assert_eq!(a.try_lt(b).unwrap().as_bool(), true);
    }

    #[test]
    fn test_trit_ops() {
        let pos = NanBox::from_trit(1);
        let neg = NanBox::from_trit(-1);
        assert_eq!(pos.trit_and(neg).unwrap().as_trit(), -1);
        assert_eq!(pos.trit_or(neg).unwrap().as_trit(), 1);
        assert_eq!(pos.trit_not().unwrap().as_trit(), -1);
    }

    #[test]
    fn test_fuzzy_ops() {
        let a = NanBox::from_f64(0.8);
        let b = NanBox::from_f64(0.6);
        let r = a.fuzzy_and(b).unwrap().as_f64();
        assert!((r - 0.6).abs() < 1e-10);
        assert!((a.fuzzy_not().unwrap().as_f64() - 0.2).abs() < 1e-10);
    }
}
