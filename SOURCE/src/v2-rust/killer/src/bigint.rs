//! Arbitrary-precision signed integers (sign + magnitude, 32-bit limbs, little endian).
//!
//! Written from scratch (no external crates). Operations:
//! * parse decimal / `0x` hexadecimal, print decimal (divide and conquer) / hexadecimal;
//! * `+ - *` (schoolbook below [`KARATSUBA_THRESHOLD`] limbs, Karatsuba above);
//! * division with Python *floor* semantics (`divmod`): the quotient rounds toward negative
//!   infinity and the remainder takes the sign of the divisor (Knuth algorithm D underneath);
//! * `pow` (with a size guard), `powmod`, `gcd`, `isqrt`, `bit_length`, comparison, `abs`, negation;
//! * conversion to / from `f64` (exact when the float is integral) and `i64`.

use std::cmp::Ordering;
use std::fmt;

/// Limb count from which multiplication switches from schoolbook to Karatsuba.
pub const KARATSUBA_THRESHOLD: usize = 32;
/// `pow` refuses results larger than this many bits (about 2.5 million decimal digits).
pub const MAX_POW_BITS: u64 = 8_388_608;
/// `factorial` refuses arguments above this.
pub const MAX_FACTORIAL: u64 = 200_000;

/// An arbitrary-precision integer. Invariant: `mag` has no trailing zero limb, and zero is
/// `mag == []` with `neg == false`.
#[derive(Clone, PartialEq, Eq, Hash, Default)]
pub struct BigInt {
    neg: bool,
    mag: Vec<u32>,
}

type Mag = Vec<u32>;

// ---------------------------------------------------------------------------------------------
// magnitude helpers
// ---------------------------------------------------------------------------------------------

fn trim(v: &mut Mag) {
    while v.last() == Some(&0) {
        v.pop();
    }
}

fn cmp_mag(a: &[u32], b: &[u32]) -> Ordering {
    if a.len() != b.len() {
        return a.len().cmp(&b.len());
    }
    for i in (0..a.len()).rev() {
        if a[i] != b[i] {
            return a[i].cmp(&b[i]);
        }
    }
    Ordering::Equal
}

fn add_mag(a: &[u32], b: &[u32]) -> Mag {
    let (long, short) = if a.len() >= b.len() { (a, b) } else { (b, a) };
    let mut out = Vec::with_capacity(long.len() + 1);
    let mut carry = 0u64;
    for i in 0..long.len() {
        let s = long[i] as u64 + *short.get(i).unwrap_or(&0) as u64 + carry;
        out.push(s as u32);
        carry = s >> 32;
    }
    if carry != 0 {
        out.push(carry as u32);
    }
    out
}

/// `a - b` for `a >= b`.
fn sub_mag(a: &[u32], b: &[u32]) -> Mag {
    let mut out = Vec::with_capacity(a.len());
    let mut borrow = 0i64;
    for i in 0..a.len() {
        let mut d = a[i] as i64 - *b.get(i).unwrap_or(&0) as i64 - borrow;
        if d < 0 {
            d += 1 << 32;
            borrow = 1;
        } else {
            borrow = 0;
        }
        out.push(d as u32);
    }
    trim(&mut out);
    out
}

fn mul_school(a: &[u32], b: &[u32]) -> Mag {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut out = vec![0u32; a.len() + b.len()];
    for (i, &x) in a.iter().enumerate() {
        if x == 0 {
            continue;
        }
        let mut carry = 0u64;
        for (j, &y) in b.iter().enumerate() {
            let t = out[i + j] as u64 + x as u64 * y as u64 + carry;
            out[i + j] = t as u32;
            carry = t >> 32;
        }
        out[i + b.len()] = carry as u32;
    }
    trim(&mut out);
    out
}

fn shift_limbs(a: &[u32], n: usize) -> Mag {
    if a.is_empty() {
        return Vec::new();
    }
    let mut out = vec![0u32; n];
    out.extend_from_slice(a);
    out
}

fn trimmed(s: &[u32]) -> &[u32] {
    let mut n = s.len();
    while n > 0 && s[n - 1] == 0 {
        n -= 1;
    }
    &s[..n]
}

fn mul_mag(a: &[u32], b: &[u32]) -> Mag {
    let (a, b) = (trimmed(a), trimmed(b));
    let (a, b) = if a.len() >= b.len() { (a, b) } else { (b, a) };
    if b.len() < KARATSUBA_THRESHOLD {
        return mul_school(a, b);
    }
    if a.len() >= 2 * b.len() {
        // unbalanced: multiply `b` by chunks of `a` and add the shifted partial products
        let mut acc: Mag = Vec::new();
        let mut off = 0;
        while off < a.len() {
            let end = (off + b.len()).min(a.len());
            let part = mul_mag(&a[off..end], b);
            acc = add_mag(&acc, &shift_limbs(&part, off));
            off = end;
        }
        trim(&mut acc);
        return acc;
    }
    let m = a.len() / 2;
    let (a0, a1) = (trimmed(&a[..m]), &a[m..]);
    let (b0, b1) = if b.len() > m { (trimmed(&b[..m]), &b[m..]) } else { (b, &b[..0]) };
    let z0 = mul_mag(a0, b0);
    let z2 = mul_mag(a1, b1);
    let sa = add_mag(a0, a1);
    let sb = add_mag(b0, b1);
    let z1full = mul_mag(&sa, &sb);
    let z1 = sub_mag(&sub_mag(&z1full, &z0), &z2);
    let mut out = add_mag(&z0, &shift_limbs(&z1, m));
    out = add_mag(&out, &shift_limbs(&z2, 2 * m));
    trim(&mut out);
    out
}

fn shl_bits_mag(a: &[u32], bits: usize) -> Mag {
    if a.is_empty() {
        return Vec::new();
    }
    let (limbs, rem) = (bits / 32, (bits % 32) as u32);
    let mut out = vec![0u32; limbs];
    if rem == 0 {
        out.extend_from_slice(a);
    } else {
        let mut carry = 0u32;
        for &x in a {
            out.push((x << rem) | carry);
            carry = x >> (32 - rem);
        }
        if carry != 0 {
            out.push(carry);
        }
    }
    out
}

fn shr_bits_mag(a: &[u32], bits: usize) -> Mag {
    let (limbs, rem) = (bits / 32, (bits % 32) as u32);
    if limbs >= a.len() {
        return Vec::new();
    }
    let src = &a[limbs..];
    let mut out = Vec::with_capacity(src.len());
    for i in 0..src.len() {
        let lo = src[i] >> rem;
        let hi = if rem != 0 && i + 1 < src.len() { src[i + 1] << (32 - rem) } else { 0 };
        out.push(lo | hi);
    }
    trim(&mut out);
    out
}

/// Divide by a single limb: (quotient, remainder).
fn divmod_small(a: &[u32], d: u32) -> (Mag, u32) {
    let mut q = vec![0u32; a.len()];
    let mut rem = 0u64;
    for i in (0..a.len()).rev() {
        let cur = (rem << 32) | a[i] as u64;
        q[i] = (cur / d as u64) as u32;
        rem = cur % d as u64;
    }
    trim(&mut q);
    (q, rem as u32)
}

/// Knuth algorithm D: (quotient, remainder) of magnitudes; `b` must be non-zero.
fn divmod_mag(a: &[u32], b: &[u32]) -> (Mag, Mag) {
    let (a, b) = (trimmed(a), trimmed(b));
    assert!(!b.is_empty(), "division by zero");
    if cmp_mag(a, b) == Ordering::Less {
        return (Vec::new(), a.to_vec());
    }
    if b.len() == 1 {
        let (q, r) = divmod_small(a, b[0]);
        let mut rm = vec![r];
        trim(&mut rm);
        return (q, rm);
    }
    let n = b.len();
    let m = a.len() - n;
    let s = b[n - 1].leading_zeros() as usize;
    let vn = shl_bits_mag(b, s);
    let mut un = shl_bits_mag(a, s);
    if un.len() == a.len() {
        un.push(0);
    }
    debug_assert_eq!(vn.len(), n);
    let mut q = vec![0u32; m + 1];
    const BASE: u64 = 1 << 32;
    for j in (0..=m).rev() {
        let num = ((un[j + n] as u64) << 32) | un[j + n - 1] as u64;
        let mut qhat = num / vn[n - 1] as u64;
        let mut rhat = num % vn[n - 1] as u64;
        while qhat >= BASE || qhat * vn[n - 2] as u64 > ((rhat << 32) | un[j + n - 2] as u64) {
            qhat -= 1;
            rhat += vn[n - 1] as u64;
            if rhat >= BASE {
                break;
            }
        }
        // multiply and subtract
        let mut k: i64 = 0;
        for i in 0..n {
            let p = qhat * vn[i] as u64;
            let t = un[i + j] as i64 - k - (p & 0xFFFF_FFFF) as i64;
            un[i + j] = t as u32;
            k = (p >> 32) as i64 - (t >> 32);
        }
        let t = un[j + n] as i64 - k;
        un[j + n] = t as u32;
        if t < 0 {
            // qhat was one too large: add the divisor back
            qhat -= 1;
            let mut carry = 0u64;
            for i in 0..n {
                let s2 = un[i + j] as u64 + vn[i] as u64 + carry;
                un[i + j] = s2 as u32;
                carry = s2 >> 32;
            }
            un[j + n] = un[j + n].wrapping_add(carry as u32);
        }
        q[j] = qhat as u32;
    }
    trim(&mut q);
    un.truncate(n);
    trim(&mut un);
    let r = shr_bits_mag(&un, s);
    (q, r)
}

fn bit_length_mag(a: &[u32]) -> u64 {
    match a.last() {
        None => 0,
        Some(top) => (a.len() as u64 - 1) * 32 + (32 - top.leading_zeros() as u64),
    }
}

// ---------------------------------------------------------------------------------------------
// BigInt
// ---------------------------------------------------------------------------------------------

impl BigInt {
    pub fn zero() -> BigInt {
        BigInt { neg: false, mag: Vec::new() }
    }

    pub fn one() -> BigInt {
        BigInt::from_u64(1)
    }

    fn from_parts(neg: bool, mut mag: Mag) -> BigInt {
        trim(&mut mag);
        let neg = neg && !mag.is_empty();
        BigInt { neg, mag }
    }

    pub fn from_u64(v: u64) -> BigInt {
        BigInt::from_parts(false, vec![v as u32, (v >> 32) as u32])
    }

    pub fn from_i64(v: i64) -> BigInt {
        let mut b = BigInt::from_u64(v.unsigned_abs());
        b.neg = v < 0;
        b
    }

    pub fn is_zero(&self) -> bool {
        self.mag.is_empty()
    }

    pub fn is_negative(&self) -> bool {
        self.neg
    }

    pub fn is_even(&self) -> bool {
        self.mag.first().map_or(true, |l| l & 1 == 0)
    }

    /// -1, 0 or 1.
    pub fn signum(&self) -> i32 {
        if self.is_zero() { 0 } else if self.neg { -1 } else { 1 }
    }

    pub fn neg(&self) -> BigInt {
        BigInt::from_parts(!self.neg, self.mag.clone())
    }

    pub fn abs(&self) -> BigInt {
        BigInt { neg: false, mag: self.mag.clone() }
    }

    /// Number of bits of the magnitude (0 for zero).
    pub fn bit_length(&self) -> u64 {
        bit_length_mag(&self.mag)
    }

    pub fn to_i64(&self) -> Option<i64> {
        if self.mag.len() > 2 {
            return None;
        }
        let v = self.mag.iter().rev().fold(0u64, |acc, &l| (acc << 32) | l as u64);
        if self.neg {
            if v <= 1u64 << 63 { Some((v as i64).wrapping_neg()) } else { None }
        } else if v <= i64::MAX as u64 {
            Some(v as i64)
        } else {
            None
        }
    }

    /// Nearest `f64` (ties to even); `inf` when the value is out of range.
    pub fn to_f64(&self) -> f64 {
        let bits = self.bit_length();
        let magnitude = if bits <= 64 {
            self.mag.iter().rev().fold(0u64, |acc, &l| (acc << 32) | l as u64) as f64
        } else {
            let shift = (bits - 64) as usize;
            let top = shr_bits_mag(&self.mag, shift);
            let mut v = top.iter().rev().fold(0u64, |acc, &l| (acc << 32) | l as u64);
            // sticky bit: anything dropped below makes a tie round correctly
            let dropped_nonzero = {
                let (limbs, rem) = (shift / 32, shift % 32);
                self.mag[..limbs].iter().any(|&l| l != 0)
                    || (rem != 0 && self.mag[limbs] & ((1u32 << rem) - 1) != 0)
            };
            if dropped_nonzero {
                v |= 1;
            }
            if shift > 1100 { f64::INFINITY } else { v as f64 * 2f64.powi(shift as i32) }
        };
        if self.neg { -magnitude } else { magnitude }
    }

    /// The exact integer value of `f`, or `None` when it is not finite or has a fraction.
    pub fn from_f64(f: f64) -> Option<BigInt> {
        if !f.is_finite() || f.fract() != 0.0 {
            return None;
        }
        let bits = f.to_bits();
        let neg = (bits >> 63) != 0;
        let exp = ((bits >> 52) & 0x7ff) as i64;
        let frac = bits & ((1u64 << 52) - 1);
        if exp == 0 {
            return Some(BigInt::zero()); // 0.0 (subnormals have a fraction)
        }
        let mant = frac | (1u64 << 52);
        let e = exp - 1075;
        let m = BigInt::from_u64(mant);
        let mag = if e >= 0 { shl_bits_mag(&m.mag, e as usize) } else { shr_bits_mag(&m.mag, (-e) as usize) };
        Some(BigInt::from_parts(neg, mag))
    }

    // -- arithmetic -------------------------------------------------------------------------

    pub fn add(&self, o: &BigInt) -> BigInt {
        if self.neg == o.neg {
            return BigInt::from_parts(self.neg, add_mag(&self.mag, &o.mag));
        }
        match cmp_mag(&self.mag, &o.mag) {
            Ordering::Equal => BigInt::zero(),
            Ordering::Greater => BigInt::from_parts(self.neg, sub_mag(&self.mag, &o.mag)),
            Ordering::Less => BigInt::from_parts(o.neg, sub_mag(&o.mag, &self.mag)),
        }
    }

    pub fn sub(&self, o: &BigInt) -> BigInt {
        self.add(&o.neg())
    }

    pub fn mul(&self, o: &BigInt) -> BigInt {
        BigInt::from_parts(self.neg != o.neg, mul_mag(&self.mag, &o.mag))
    }

    /// Floor division and modulo (Python semantics). `None` when `d` is zero.
    pub fn divmod(&self, d: &BigInt) -> Option<(BigInt, BigInt)> {
        if d.is_zero() {
            return None;
        }
        let (q, r) = divmod_mag(&self.mag, &d.mag);
        let mut quot = BigInt::from_parts(self.neg != d.neg, q);
        let mut rem = BigInt::from_parts(self.neg, r);
        if !rem.is_zero() && (rem.neg != d.neg) {
            quot = quot.sub(&BigInt::one());
            rem = rem.add(d);
        }
        Some((quot, rem))
    }

    /// `self ** exp` for a non-negative exponent; errors when the result would be absurdly large.
    pub fn pow(&self, exp: u64) -> Result<BigInt, String> {
        if exp == 0 {
            return Ok(BigInt::one());
        }
        if self.is_zero() || (self.mag == [1]) {
            return Ok(if self.neg && exp % 2 == 1 { self.clone() } else { self.abs() });
        }
        // (bits - 1) * exp is a lower bound of the result's bit length
        if (self.bit_length() - 1).saturating_mul(exp) > MAX_POW_BITS {
            return Err(format!(
                "exponent too large: the result would need more than {} bits",
                MAX_POW_BITS
            ));
        }
        let mut result = BigInt::one();
        let mut base = self.clone();
        let mut e = exp;
        while e > 0 {
            if e & 1 == 1 {
                result = result.mul(&base);
            }
            e >>= 1;
            if e > 0 {
                base = base.mul(&base);
            }
        }
        Ok(result)
    }

    pub fn gcd(&self, o: &BigInt) -> BigInt {
        let mut a = self.abs();
        let mut b = o.abs();
        while !b.is_zero() {
            let (_, r) = divmod_mag(&a.mag, &b.mag);
            a = b;
            b = BigInt::from_parts(false, r);
        }
        a
    }

    /// Integer square root (floor). `None` for negative values.
    pub fn isqrt(&self) -> Option<BigInt> {
        if self.neg {
            return None;
        }
        if self.is_zero() {
            return Some(BigInt::zero());
        }
        let mut x = BigInt::from_parts(false, shl_bits_mag(&[1], ((self.bit_length() + 1) / 2) as usize));
        loop {
            let (q, _) = divmod_mag(&self.mag, &x.mag);
            let y = BigInt::from_parts(false, shr_bits_mag(&add_mag(&x.mag, &q), 1));
            if cmp_mag(&y.mag, &x.mag) != Ordering::Less {
                return Some(x);
            }
            x = y;
        }
    }

    fn bit(&self, i: u64) -> bool {
        let (limb, off) = ((i / 32) as usize, (i % 32) as u32);
        self.mag.get(limb).map_or(false, |l| (l >> off) & 1 == 1)
    }

    /// `self ** exp mod m` (Python `pow(a, b, m)`): `exp >= 0`, `m != 0`, result takes the sign of `m`.
    pub fn powmod(&self, exp: &BigInt, m: &BigInt) -> Result<BigInt, String> {
        if m.is_zero() {
            return Err("powmod: modulus is zero".to_string());
        }
        if exp.neg {
            return Err("powmod: negative exponent is not supported".to_string());
        }
        let (_, mut base) = self.divmod(m).expect("non-zero modulus");
        let (_, mut result) = BigInt::one().divmod(m).expect("non-zero modulus");
        for i in 0..exp.bit_length() {
            if exp.bit(i) {
                result = result.mul(&base).divmod(m).expect("non-zero modulus").1;
            }
            if i + 1 < exp.bit_length() {
                base = base.mul(&base).divmod(m).expect("non-zero modulus").1;
            }
        }
        Ok(result)
    }

    /// `n!` by a balanced product tree (so Karatsuba applies).
    pub fn factorial(n: u64) -> Result<BigInt, String> {
        if n > MAX_FACTORIAL {
            return Err(format!("factorial: argument too large (limit {})", MAX_FACTORIAL));
        }
        fn product(lo: u64, hi: u64) -> BigInt {
            // product of lo..=hi
            if hi - lo < 8 {
                let mut acc = BigInt::from_u64(lo);
                for k in lo + 1..=hi {
                    acc = acc.mul(&BigInt::from_u64(k));
                }
                return acc;
            }
            let mid = lo + (hi - lo) / 2;
            product(lo, mid).mul(&product(mid + 1, hi))
        }
        if n < 2 {
            return Ok(BigInt::one());
        }
        Ok(product(2, n))
    }

    // -- text -------------------------------------------------------------------------------

    /// Parse an optionally signed decimal or `0x` hexadecimal literal (`_` separators allowed).
    pub fn parse(s: &str) -> Result<BigInt, String> {
        let t = s.trim();
        let (neg, body) = match t.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, t.strip_prefix('+').unwrap_or(t)),
        };
        let cleaned: String = body.chars().filter(|c| *c != '_').collect();
        if cleaned.is_empty() {
            return Err(format!("invalid integer literal `{}`", s));
        }
        let mag = if let Some(hex) = cleaned.strip_prefix("0x").or_else(|| cleaned.strip_prefix("0X")) {
            if hex.is_empty() || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(format!("invalid hexadecimal literal `{}`", s));
            }
            let mut out: Mag = Vec::new();
            let digits: Vec<u32> = hex.chars().map(|c| c.to_digit(16).unwrap()).collect();
            let mut i = digits.len();
            while i > 0 {
                let start = i.saturating_sub(8);
                let mut limb = 0u32;
                for &d in &digits[start..i] {
                    limb = (limb << 4) | d;
                }
                out.push(limb);
                i = start;
            }
            out
        } else {
            if !cleaned.chars().all(|c| c.is_ascii_digit()) {
                return Err(format!("invalid integer literal `{}`", s));
            }
            parse_decimal(cleaned.as_bytes())
        };
        Ok(BigInt::from_parts(neg, mag))
    }

    pub fn to_hex(&self) -> String {
        if self.is_zero() {
            return "0".to_string();
        }
        let mut s = String::new();
        if self.neg {
            s.push('-');
        }
        s.push_str("0x");
        let top = self.mag.len() - 1;
        s.push_str(&format!("{:x}", self.mag[top]));
        for i in (0..top).rev() {
            s.push_str(&format!("{:08x}", self.mag[i]));
        }
        s
    }

    pub fn to_decimal(&self) -> String {
        if self.is_zero() {
            return "0".to_string();
        }
        let mut out = String::new();
        if self.neg {
            out.push('-');
        }
        // 10^9, 10^18, 10^36, ... (each the square of the previous)
        let mut pows: Vec<Mag> = vec![vec![1_000_000_000]];
        while cmp_mag(&mul_mag(pows.last().unwrap(), pows.last().unwrap()), &self.mag) != Ordering::Greater {
            let sq = mul_mag(pows.last().unwrap(), pows.last().unwrap());
            pows.push(sq);
        }
        // largest index whose power is <= value
        let mut top = pows.len() as isize - 1;
        while top >= 0 && cmp_mag(&pows[top as usize], &self.mag) == Ordering::Greater {
            top -= 1;
        }
        dec_rec(&self.mag, &pows, top, false, &mut out);
        out
    }
}

fn dec_rec(x: &[u32], pows: &[Mag], k: isize, padded: bool, out: &mut String) {
    if k < 0 {
        let v = x.first().copied().unwrap_or(0);
        if padded {
            out.push_str(&format!("{:09}", v));
        } else {
            out.push_str(&v.to_string());
        }
        return;
    }
    let (q, r) = divmod_mag(x, &pows[k as usize]);
    if q.is_empty() && !padded {
        dec_rec(&r, pows, k - 1, false, out);
    } else {
        dec_rec(&q, pows, k - 1, padded, out);
        dec_rec(&r, pows, k - 1, true, out);
    }
}

/// Decimal digits (ASCII) to a magnitude; divide and conquer above a few thousand digits.
fn parse_decimal(digits: &[u8]) -> Mag {
    if digits.len() <= 2000 {
        let mut mag: Mag = Vec::new();
        let first = digits.len() % 9;
        let mut chunks: Vec<&[u8]> = Vec::new();
        if first > 0 {
            chunks.push(&digits[..first]);
        }
        let mut i = first;
        while i < digits.len() {
            chunks.push(&digits[i..i + 9]);
            i += 9;
        }
        for c in chunks {
            let v = c.iter().fold(0u32, |a, &d| a * 10 + (d - b'0') as u32);
            let scale = 10u32.pow(c.len() as u32);
            // mag = mag * scale + v
            let mut carry = v as u64;
            for limb in mag.iter_mut() {
                let t = *limb as u64 * scale as u64 + carry;
                *limb = t as u32;
                carry = t >> 32;
            }
            if carry != 0 {
                mag.push(carry as u32);
            }
        }
        trim(&mut mag);
        return mag;
    }
    let lo_len = (digits.len() / 2) / 9 * 9;
    let split = digits.len() - lo_len;
    let hi = parse_decimal(&digits[..split]);
    let lo = parse_decimal(&digits[split..]);
    let pow = BigInt::from_u64(10).pow(lo_len as u64).expect("small power");
    add_mag(&mul_mag(&hi, &pow.mag), &lo)
}

impl fmt::Display for BigInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_decimal())
    }
}

impl fmt::Debug for BigInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BigInt({})", self.to_decimal())
    }
}

impl PartialOrd for BigInt {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BigInt {
    fn cmp(&self, o: &Self) -> Ordering {
        match (self.neg, o.neg) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => cmp_mag(&self.mag, &o.mag),
            (true, true) => cmp_mag(&o.mag, &self.mag),
        }
    }
}

impl BigInt {
    /// Compare with an `f64` exactly (NaN is unordered).
    pub fn cmp_f64(&self, f: f64) -> Option<Ordering> {
        if f.is_nan() {
            return None;
        }
        if f == f64::INFINITY {
            return Some(Ordering::Less);
        }
        if f == f64::NEG_INFINITY {
            return Some(Ordering::Greater);
        }
        let floor = f.floor();
        let fb = BigInt::from_f64(floor).expect("finite integral");
        match self.cmp(&fb) {
            Ordering::Equal => Some(if f > floor { Ordering::Less } else { Ordering::Equal }),
            other => Some(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(s: &str) -> BigInt {
        BigInt::parse(s).unwrap()
    }

    #[test]
    fn parse_and_print_round_trip() {
        for s in ["0", "1", "-1", "4294967296", "-18446744073709551616", "123456789012345678901234567890"] {
            assert_eq!(b(s).to_string(), s);
        }
        assert_eq!(b("0xFF").to_string(), "255");
        assert_eq!(b("-0x1_0000_0000").to_string(), "-4294967296");
        assert_eq!(b("000123").to_string(), "123");
        assert!(BigInt::parse("12x").is_err());
        assert!(BigInt::parse("").is_err());
        assert!(BigInt::parse("0x").is_err());
    }

    #[test]
    fn large_decimal_round_trip_uses_divide_and_conquer() {
        let digits: String = (0..30_000).map(|i| char::from(b'1' + (i * 7 % 9) as u8)).collect();
        let n = b(&digits);
        assert_eq!(n.to_string(), digits);
    }

    #[test]
    fn floor_division_semantics() {
        for (a, d, q, r) in [(7, 2, 3, 1), (-7, 2, -4, 1), (7, -2, -4, -1), (-7, -2, 3, -1), (6, 3, 2, 0), (-6, 3, -2, 0)] {
            let (qq, rr) = BigInt::from_i64(a).divmod(&BigInt::from_i64(d)).unwrap();
            assert_eq!((qq.to_i64().unwrap(), rr.to_i64().unwrap()), (q, r), "{a} divmod {d}");
        }
        assert!(BigInt::one().divmod(&BigInt::zero()).is_none());
    }

    #[test]
    fn multiplication_and_division_agree_on_big_operands() {
        let x = b("98765432109876543210987654321098765432109876543210").pow(9).unwrap();
        let y = b("12345678901234567890123456789").pow(7).unwrap();
        let p = x.mul(&y);
        let (q, r) = p.divmod(&y).unwrap();
        assert_eq!(q, x);
        assert!(r.is_zero());
        let (q2, r2) = p.add(&BigInt::from_i64(12345)).divmod(&x).unwrap();
        assert_eq!(q2, y);
        assert_eq!(r2.to_i64(), Some(12345));
    }

    #[test]
    fn karatsuba_matches_schoolbook() {
        let a: Mag = (0..200u32).map(|i| i.wrapping_mul(2654435761).wrapping_add(7)).collect();
        let c: Mag = (0..150u32).map(|i| i.wrapping_mul(40503).wrapping_add(11) | 1).collect();
        let mut s = mul_school(&a, &c);
        let mut k = mul_mag(&a, &c);
        trim(&mut s);
        trim(&mut k);
        assert_eq!(s, k);
    }

    #[test]
    fn pow_gcd_isqrt_powmod() {
        assert_eq!(BigInt::from_i64(2).pow(128).unwrap().to_string(), "340282366920938463463374607431768211456");
        assert!(BigInt::from_i64(10).pow(10_000_000_000).is_err());
        assert_eq!(BigInt::from_i64(-3).pow(3).unwrap().to_i64(), Some(-27));
        assert_eq!(b("123456789123456789").gcd(&b("987654321987654321")).to_string(), "9000000009");
        assert_eq!(b("1000000000000000000000000000000").isqrt().unwrap().to_string(), "1000000000000000");
        assert_eq!(BigInt::from_i64(4).powmod(&BigInt::from_i64(13), &BigInt::from_i64(497)).unwrap().to_i64(), Some(445));
    }

    #[test]
    fn float_conversions() {
        assert_eq!(BigInt::from_f64(1e20).unwrap().to_string(), "100000000000000000000");
        assert!(BigInt::from_f64(0.5).is_none());
        assert_eq!(b("100000000000000000000").to_f64(), 1e20);
        assert_eq!(BigInt::from_i64(-5).to_f64(), -5.0);
        assert_eq!(BigInt::from_i64(2).pow(2000).unwrap().to_f64(), f64::INFINITY);
        assert_eq!(b("5").cmp_f64(5.5), Some(Ordering::Less));
        assert_eq!(b("-5").cmp_f64(-5.5), Some(Ordering::Greater));
        assert_eq!(b("5").cmp_f64(5.0), Some(Ordering::Equal));
    }

    #[test]
    fn factorial_values() {
        assert_eq!(BigInt::factorial(20).unwrap().to_string(), "2432902008176640000");
        assert_eq!(BigInt::factorial(0).unwrap().to_string(), "1");
        assert_eq!(BigInt::factorial(25).unwrap().to_string(), "15511210043330985984000000");
    }
}
