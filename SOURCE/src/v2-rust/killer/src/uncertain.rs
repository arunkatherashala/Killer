//! Uncertainty-aware arithmetic and comparison.
//!
//! Two kinds of uncertain number exist:
//!
//! * `Value::Uncertain { value, margin }` (`believe x = v ± m`) is the *guaranteed* interval
//!   `[v - m, v + m]`. Arithmetic returns an interval that encloses every possible result.
//! * `Value::Gauss { mean, sigma }` (`gauss(mean, sigma)`) is *statistical* uncertainty:
//!   independent, normally distributed errors that combine in quadrature (first order for division
//!   and `sqrt`). It assumes the errors are independent, so `x - x` is not zero.
//!
//! The two kinds cannot be mixed in one expression. A plain number is exact (spread 0) and adopts
//! the kind of the other operand.
//!
//! Comparisons are three-valued and return a `Trit`:
//!
//! * `T_POS`  (+1) — certainly true (every value in the interval / at least 95% confidence)
//! * `T_NEG`  (-1) — certainly false
//! * `T_ZERO` ( 0) — cannot be decided
//!
//! `&&`, `||`, `!` on trits use Kleene logic, and `if` takes its branch only for `T_POS`.

use crate::value::Value;

/// Two-sided 95% critical value of the standard normal distribution.
pub const Z95: f64 = 1.959964;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CmpOp {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Interval,
    Gauss,
}

#[inline]
pub fn is_uncertain(v: &Value) -> bool {
    matches!(v, Value::Uncertain { .. } | Value::Gauss { .. })
}

/// (center, spread, kind) of a numeric value; plain numbers have no kind. `None` for non-numbers.
fn parts(v: &Value) -> Option<(f64, f64, Option<Kind>)> {
    match v {
        Value::Number(n) => Some((*n, 0.0, None)),
        Value::Integer(i) => Some((*i as f64, 0.0, None)),
        Value::Uncertain { value, margin } => Some((*value, margin.abs(), Some(Kind::Interval))),
        Value::Gauss { mean, sigma } => Some((*mean, sigma.abs(), Some(Kind::Gauss))),
        _ => None,
    }
}

fn make(kind: Kind, center: f64, spread: f64) -> Value {
    match kind {
        Kind::Interval => Value::Uncertain { value: center, margin: spread.abs() },
        Kind::Gauss => Value::Gauss { mean: center, sigma: spread.abs() },
    }
}

/// The kind two operands share (a plain number adopts the other's kind).
fn common_kind(k1: Option<Kind>, k2: Option<Kind>) -> Result<Kind, String> {
    match (k1, k2) {
        (Some(a), Some(b)) if a != b => Err(
            "Cannot combine an interval (\u{b1}) uncertainty with a statistical (gauss) one".to_string(),
        ),
        (Some(a), _) | (_, Some(a)) => Ok(a),
        (None, None) => Ok(Kind::Interval),
    }
}

/// Arithmetic where at least one operand is uncertain. `None` means neither operand is
/// uncertain (the caller handles it); `Some(Err)` is a type or domain error.
pub fn binary(op: BinOp, a: &Value, b: &Value) -> Option<Result<Value, String>> {
    if !is_uncertain(a) && !is_uncertain(b) {
        return None;
    }
    let symbol = match op {
        BinOp::Add => '+',
        BinOp::Sub => '-',
        BinOp::Mul => '*',
        BinOp::Div => '/',
    };
    let (Some((v1, m1, k1)), Some((v2, m2, k2))) = (parts(a), parts(b)) else {
        return Some(Err(format!(
            "Type error in '{}': uncertain values combine only with numbers or other uncertain values",
            symbol
        )));
    };
    let kind = match common_kind(k1, k2) {
        Ok(k) => k,
        Err(e) => return Some(Err(e)),
    };
    Some(match kind {
        Kind::Interval => interval_binary(op, v1, m1, v2, m2),
        Kind::Gauss => gauss_binary(op, v1, m1, v2, m2),
    })
}

fn interval_binary(op: BinOp, v1: f64, m1: f64, v2: f64, m2: f64) -> Result<Value, String> {
    let k = Kind::Interval;
    match op {
        BinOp::Add => Ok(make(k, v1 + v2, m1 + m2)),
        BinOp::Sub => Ok(make(k, v1 - v2, m1 + m2)),
        // (v1±m1)(v2±m2) lies within v1*v2 ± (|v1|m2 + |v2|m1 + m1m2)
        BinOp::Mul => Ok(make(k, v1 * v2, v1.abs() * m2 + v2.abs() * m1 + m1 * m2)),
        BinOp::Div => {
            if v2.abs() <= m2 {
                Err("Division by an uncertain value whose range includes zero".to_string())
            } else {
                let center = v1 / v2;
                let mut lo = f64::INFINITY;
                let mut hi = f64::NEG_INFINITY;
                for n in [v1 - m1, v1 + m1] {
                    for d in [v2 - m2, v2 + m2] {
                        let q = n / d;
                        lo = lo.min(q);
                        hi = hi.max(q);
                    }
                }
                Ok(make(k, center, (hi - center).max(center - lo)))
            }
        }
    }
}

/// Independent normal errors: variances add. Products use the exact variance of independent
/// variables; quotients use the first-order (delta-method) approximation.
fn gauss_binary(op: BinOp, v1: f64, s1: f64, v2: f64, s2: f64) -> Result<Value, String> {
    let k = Kind::Gauss;
    match op {
        BinOp::Add => Ok(make(k, v1 + v2, s1.hypot(s2))),
        BinOp::Sub => Ok(make(k, v1 - v2, s1.hypot(s2))),
        BinOp::Mul => {
            let var = (v2 * s1).powi(2) + (v1 * s2).powi(2) + (s1 * s2).powi(2);
            Ok(make(k, v1 * v2, var.sqrt()))
        }
        BinOp::Div => {
            if v2 == 0.0 || v2.abs() <= Z95 * s2 {
                Err("Division by a statistical value that is not clearly different from zero".to_string())
            } else {
                let var = (s1 / v2).powi(2) + (v1 * s2 / (v2 * v2)).powi(2);
                Ok(make(k, v1 / v2, var.sqrt()))
            }
        }
    }
}

/// Three-valued comparison where at least one operand is uncertain. Returns a `Value::Trit`.
pub fn compare(op: CmpOp, a: &Value, b: &Value) -> Option<Result<Value, String>> {
    if !is_uncertain(a) && !is_uncertain(b) {
        return None;
    }
    let (Some((v1, m1, k1)), Some((v2, m2, k2))) = (parts(a), parts(b)) else {
        return Some(Err("Cannot compare an uncertain value with a non-number".to_string()));
    };
    let kind = match common_kind(k1, k2) {
        Ok(k) => k,
        Err(e) => return Some(Err(e)),
    };
    let t = match kind {
        Kind::Interval => interval_compare(op, v1, m1, v2, m2),
        Kind::Gauss => gauss_compare(op, v1, m1, v2, m2),
    };
    Some(Ok(Value::Trit(t)))
}

fn interval_compare(op: CmpOp, v1: f64, m1: f64, v2: f64, m2: f64) -> i8 {
    let (lo1, hi1, lo2, hi2) = (v1 - m1, v1 + m1, v2 - m2, v2 + m2);
    if lo1.is_nan() || hi1.is_nan() || lo2.is_nan() || hi2.is_nan() {
        return 0;
    }
    // Interval relations as (certainly true, certainly false)
    let lt = |lo_a: f64, hi_a: f64, lo_b: f64, hi_b: f64| (hi_a < lo_b, lo_a >= hi_b);
    let le = |lo_a: f64, hi_a: f64, lo_b: f64, hi_b: f64| (hi_a <= lo_b, lo_a > hi_b);
    let (yes, no) = match op {
        CmpOp::Lt => lt(lo1, hi1, lo2, hi2),
        CmpOp::Le => le(lo1, hi1, lo2, hi2),
        CmpOp::Gt => lt(lo2, hi2, lo1, hi1),
        CmpOp::Ge => le(lo2, hi2, lo1, hi1),
        CmpOp::Eq | CmpOp::Ne => {
            let disjoint = hi1 < lo2 || hi2 < lo1;
            let exact_equal = m1 == 0.0 && m2 == 0.0 && v1 == v2;
            let (eq_yes, eq_no) = (exact_equal, disjoint);
            if op == CmpOp::Eq { (eq_yes, eq_no) } else { (eq_no, eq_yes) }
        }
    };
    if yes {
        1
    } else if no {
        -1
    } else {
        0
    }
}

/// Statistical comparison: decided only when the means differ by at least `Z95` combined sigmas.
fn gauss_compare(op: CmpOp, v1: f64, s1: f64, v2: f64, s2: f64) -> i8 {
    if v1.is_nan() || v2.is_nan() || s1.is_nan() || s2.is_nan() {
        return 0;
    }
    let sd = s1.hypot(s2);
    if sd == 0.0 {
        // both exact: ordinary comparison
        let r = match op {
            CmpOp::Lt => v1 < v2,
            CmpOp::Le => v1 <= v2,
            CmpOp::Gt => v1 > v2,
            CmpOp::Ge => v1 >= v2,
            CmpOp::Eq => v1 == v2,
            CmpOp::Ne => v1 != v2,
        };
        return if r { 1 } else { -1 };
    }
    let d = (v2 - v1) / sd; // standardised difference, positive when v1 < v2
    let ordered = |toward_less: bool| {
        let z = if toward_less { d } else { -d };
        if z >= Z95 {
            1
        } else if z <= -Z95 {
            -1
        } else {
            0
        }
    };
    match op {
        CmpOp::Lt | CmpOp::Le => ordered(true),
        CmpOp::Gt | CmpOp::Ge => ordered(false),
        CmpOp::Eq => if d.abs() >= Z95 { -1 } else { 0 },
        CmpOp::Ne => if d.abs() >= Z95 { 1 } else { 0 },
    }
}

/// Standard normal CDF.
pub fn normal_cdf(x: f64) -> f64 {
    0.5 * erfc(-x / std::f64::consts::SQRT_2)
}

/// Complementary error function, accurate to about 1e-15 relative error.
/// Near zero: the all-positive series erf(z) = 2/sqrt(pi) e^(-z^2) sum 2^n z^(2n+1) / (2n+1)!!
/// (no cancellation). For z >= 3: the continued fraction for erfc.
fn erfc(x: f64) -> f64 {
    let z = x.abs();
    let c = if z < 3.0 {
        let mut term = z;
        let mut sum = z;
        let mut n = 0.0;
        while n < 400.0 {
            n += 1.0;
            term *= 2.0 * z * z / (2.0 * n + 1.0);
            sum += term;
            if term <= 1e-17 * sum {
                break;
            }
        }
        1.0 - 2.0 / std::f64::consts::PI.sqrt() * (-z * z).exp() * sum
    } else {
        let mut f = z;
        for k in (1..=80).rev() {
            f = z + (k as f64 / 2.0) / f;
        }
        (-z * z).exp() / (std::f64::consts::PI.sqrt() * f)
    };
    if x >= 0.0 {
        c
    } else {
        2.0 - c
    }
}

/// P(a > b) for statistical values (plain numbers are exact). Errors for interval values.
pub fn prob_greater(a: &Value, b: &Value) -> Result<f64, String> {
    let (Some((v1, s1, k1)), Some((v2, s2, k2))) = (parts(a), parts(b)) else {
        return Err("expects numbers or statistical (gauss) values".to_string());
    };
    if k1 == Some(Kind::Interval) || k2 == Some(Kind::Interval) {
        return Err("a probability needs statistical (gauss) values, not an interval".to_string());
    }
    let sd = s1.hypot(s2);
    if sd == 0.0 {
        return Ok(if v1 > v2 {
            1.0
        } else if v1 < v2 {
            0.0
        } else {
            0.5
        });
    }
    Ok(normal_cdf((v1 - v2) / sd))
}

/// `sqrt` of an interval value (range must be non-negative).
pub fn sqrt(v: f64, m: f64) -> Result<Value, String> {
    let m = m.abs();
    if v - m < 0.0 {
        return Err("sqrt of an uncertain value whose range includes negative numbers".to_string());
    }
    let center = v.sqrt();
    let (lo, hi) = ((v - m).sqrt(), (v + m).sqrt());
    Ok(make(Kind::Interval, center, (hi - center).max(center - lo)))
}

/// `abs` of an interval value.
pub fn abs(v: f64, m: f64) -> Value {
    let m = m.abs();
    if v - m >= 0.0 {
        make(Kind::Interval, v, m)
    } else if v + m <= 0.0 {
        make(Kind::Interval, -v, m)
    } else {
        // the range straddles zero: result lies in [0, |v| + m]
        let half = (v.abs() + m) / 2.0;
        make(Kind::Interval, half, half)
    }
}

/// `sqrt` of a statistical value (first order: sigma / (2 sqrt(mean))).
pub fn sqrt_gauss(mean: f64, sigma: f64) -> Result<Value, String> {
    if mean <= 0.0 || mean < Z95 * sigma.abs() {
        return Err("sqrt of a statistical value that is not clearly positive".to_string());
    }
    Ok(make(Kind::Gauss, mean.sqrt(), sigma.abs() / (2.0 * mean.sqrt())))
}

/// `abs` of a statistical value (first order: valid when the mean is clear of zero).
pub fn abs_gauss(mean: f64, sigma: f64) -> Value {
    make(Kind::Gauss, mean.abs(), sigma.abs())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(v: f64, m: f64) -> Value {
        Value::Uncertain { value: v, margin: m }
    }

    fn g(mean: f64, sigma: f64) -> Value {
        Value::Gauss { mean, sigma }
    }

    fn range(r: &Value) -> (f64, f64) {
        match r {
            Value::Uncertain { value, margin } => (value - margin, value + margin),
            Value::Number(n) => (*n, *n),
            other => panic!("not numeric: {:?}", other),
        }
    }

    fn gauss_parts(r: &Value) -> (f64, f64) {
        match r {
            Value::Gauss { mean, sigma } => (*mean, *sigma),
            other => panic!("not gauss: {:?}", other),
        }
    }

    /// tiny deterministic generator so the property tests need no crates
    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> f64 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
        }
        fn between(&mut self, lo: f64, hi: f64) -> f64 {
            lo + (hi - lo) * self.next()
        }
        /// standard normal via Box-Muller
        fn normal(&mut self) -> f64 {
            let (u1, u2) = (self.next().max(1e-12), self.next());
            (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
        }
    }

    #[test]
    fn plain_numbers_are_not_handled() {
        assert!(binary(BinOp::Add, &Value::Number(1.0), &Value::Number(2.0)).is_none());
        assert!(compare(CmpOp::Lt, &Value::Number(1.0), &Value::Number(2.0)).is_none());
    }

    #[test]
    fn basic_arithmetic() {
        let r = binary(BinOp::Add, &u(10.0, 1.0), &u(20.0, 2.0)).unwrap().unwrap();
        assert_eq!(r, u(30.0, 3.0));
        let r = binary(BinOp::Sub, &u(20.0, 2.0), &u(10.0, 1.0)).unwrap().unwrap();
        assert_eq!(r, u(10.0, 3.0));
        let r = binary(BinOp::Mul, &u(10.0, 1.0), &Value::Number(2.0)).unwrap().unwrap();
        assert_eq!(r, u(20.0, 2.0));
        let r = binary(BinOp::Add, &u(10.0, 1.0), &Value::Number(5.0)).unwrap().unwrap();
        assert_eq!(r, u(15.0, 1.0));
    }

    #[test]
    fn results_always_enclose_every_possible_outcome() {
        let mut gen = Lcg(42);
        for _ in 0..4000 {
            let (v1, m1) = (gen.between(-50.0, 50.0), gen.between(0.0, 5.0));
            let (v2, m2) = (gen.between(-50.0, 50.0), gen.between(0.0, 5.0));
            let (a, b) = (u(v1, m1), u(v2, m2));
            for op in [BinOp::Add, BinOp::Sub, BinOp::Mul, BinOp::Div] {
                let Ok(r) = binary(op, &a, &b).unwrap() else { continue };
                let (lo, hi) = range(&r);
                for _ in 0..8 {
                    let x = gen.between(v1 - m1, v1 + m1);
                    let y = gen.between(v2 - m2, v2 + m2);
                    let z = match op {
                        BinOp::Add => x + y,
                        BinOp::Sub => x - y,
                        BinOp::Mul => x * y,
                        BinOp::Div => x / y,
                    };
                    let slack = 1e-9 * (1.0 + z.abs());
                    assert!(z >= lo - slack && z <= hi + slack, "{:?}: {} not in [{}, {}] for ({}±{}) ({}±{})", op, z, lo, hi, v1, m1, v2, m2);
                }
            }
        }
    }

    #[test]
    fn division_by_a_range_containing_zero_is_an_error() {
        assert!(binary(BinOp::Div, &Value::Number(1.0), &u(1.0, 2.0)).unwrap().is_err());
        assert!(binary(BinOp::Div, &Value::Number(1.0), &u(0.0, 0.0)).unwrap().is_err());
        assert!(binary(BinOp::Div, &Value::Number(1.0), &u(5.0, 1.0)).unwrap().is_ok());
    }

    #[test]
    fn non_numbers_are_rejected() {
        assert!(binary(BinOp::Add, &u(1.0, 0.1), &Value::Str("x".into())).unwrap().is_err());
        assert!(compare(CmpOp::Lt, &u(1.0, 0.1), &Value::Null).unwrap().is_err());
        assert!(binary(BinOp::Add, &g(1.0, 0.1), &Value::Str("x".into())).unwrap().is_err());
    }

    fn trit(op: CmpOp, a: Value, b: Value) -> i8 {
        match compare(op, &a, &b).unwrap().unwrap() {
            Value::Trit(t) => t,
            other => panic!("expected a Trit, got {:?}", other),
        }
    }

    #[test]
    fn comparisons_are_three_valued() {
        // [9, 11] vs 5: certainly greater
        assert_eq!(trit(CmpOp::Gt, u(10.0, 1.0), Value::Number(5.0)), 1);
        assert_eq!(trit(CmpOp::Lt, u(10.0, 1.0), Value::Number(5.0)), -1);
        // [9, 11] vs 10: cannot tell
        assert_eq!(trit(CmpOp::Gt, u(10.0, 1.0), Value::Number(10.0)), 0);
        assert_eq!(trit(CmpOp::Lt, u(10.0, 1.0), Value::Number(10.0)), 0);
        // overlapping ranges
        assert_eq!(trit(CmpOp::Lt, u(10.0, 2.0), u(11.0, 2.0)), 0);
        // disjoint ranges
        assert_eq!(trit(CmpOp::Lt, u(10.0, 1.0), u(20.0, 2.0)), 1);
        assert_eq!(trit(CmpOp::Ge, u(10.0, 1.0), u(20.0, 2.0)), -1);
    }

    #[test]
    fn boundary_cases_are_not_over_claimed() {
        // [9, 11] <= 11 is certain, [9, 11] < 11 is not (the value could be exactly 11)
        assert_eq!(trit(CmpOp::Le, u(10.0, 1.0), Value::Number(11.0)), 1);
        assert_eq!(trit(CmpOp::Lt, u(10.0, 1.0), Value::Number(11.0)), 0);
        // [9, 11] > 11 is certainly false; [9, 11] >= 11 is undecided
        assert_eq!(trit(CmpOp::Gt, u(10.0, 1.0), Value::Number(11.0)), -1);
        assert_eq!(trit(CmpOp::Ge, u(10.0, 1.0), Value::Number(11.0)), 0);
    }

    #[test]
    fn equality() {
        assert_eq!(trit(CmpOp::Eq, u(10.0, 1.0), Value::Number(50.0)), -1);
        assert_eq!(trit(CmpOp::Ne, u(10.0, 1.0), Value::Number(50.0)), 1);
        assert_eq!(trit(CmpOp::Eq, u(10.0, 1.0), Value::Number(10.0)), 0);
        assert_eq!(trit(CmpOp::Ne, u(10.0, 1.0), Value::Number(10.0)), 0);
        assert_eq!(trit(CmpOp::Eq, u(7.0, 0.0), Value::Number(7.0)), 1);
    }

    #[test]
    fn comparison_agrees_with_every_sampled_pair() {
        let mut gen = Lcg(7);
        for _ in 0..3000 {
            let (v1, m1) = (gen.between(-20.0, 20.0), gen.between(0.0, 4.0));
            let (v2, m2) = (gen.between(-20.0, 20.0), gen.between(0.0, 4.0));
            for op in [CmpOp::Lt, CmpOp::Le, CmpOp::Gt, CmpOp::Ge] {
                let t = trit(op, u(v1, m1), u(v2, m2));
                for _ in 0..8 {
                    let x = gen.between(v1 - m1, v1 + m1);
                    let y = gen.between(v2 - m2, v2 + m2);
                    let truth = match op {
                        CmpOp::Lt => x < y,
                        CmpOp::Le => x <= y,
                        CmpOp::Gt => x > y,
                        _ => x >= y,
                    };
                    if t == 1 {
                        assert!(truth, "{:?} claimed certain but {} vs {} is false", op, x, y);
                    }
                    if t == -1 {
                        assert!(!truth, "{:?} claimed impossible but {} vs {} is true", op, x, y);
                    }
                }
            }
        }
    }

    #[test]
    fn sqrt_and_abs() {
        let r = sqrt(100.0, 21.0).unwrap();
        let (lo, hi) = range(&r);
        assert!(lo <= 79f64.sqrt() + 1e-12 && hi >= 121f64.sqrt() - 1e-12);
        assert!(sqrt(1.0, 2.0).is_err());
        assert_eq!(abs(5.0, 1.0), u(5.0, 1.0));
        assert_eq!(abs(-5.0, 1.0), u(5.0, 1.0));
        assert_eq!(abs(0.5, 1.0), u(0.75, 0.75)); // straddles zero: [0, 1.5]
    }

    // ---- statistical (gauss) values ----

    #[test]
    fn normal_cdf_matches_known_values() {
        let known = [
            (0.0, 0.5),
            (1.0, 0.8413447460685429),
            (2.0, 0.9772498680518208),
            (3.0, 0.9986501019683699),
            (-1.0, 0.15865525393145707),
            (1.6448536269514722, 0.95),
            (-3.5, 0.00023262907903552504),
        ];
        for (x, want) in known {
            let got = normal_cdf(x);
            assert!((got - want).abs() < 1e-12, "Phi({}) = {} want {}", x, got, want);
        }
        assert!(normal_cdf(-40.0) >= 0.0 && normal_cdf(40.0) <= 1.0);
    }

    #[test]
    fn gauss_errors_add_in_quadrature() {
        let r = binary(BinOp::Add, &g(10.0, 3.0), &g(20.0, 4.0)).unwrap().unwrap();
        assert_eq!(gauss_parts(&r), (30.0, 5.0)); // 3-4-5
        let r = binary(BinOp::Sub, &g(20.0, 4.0), &g(10.0, 3.0)).unwrap().unwrap();
        assert_eq!(gauss_parts(&r), (10.0, 5.0));
        let r = binary(BinOp::Mul, &g(10.0, 1.0), &Value::Number(3.0)).unwrap().unwrap();
        assert_eq!(gauss_parts(&r), (30.0, 3.0));
        let r = binary(BinOp::Add, &g(10.0, 1.0), &Value::Number(5.0)).unwrap().unwrap();
        assert_eq!(gauss_parts(&r), (15.0, 1.0));
    }

    #[test]
    fn gauss_results_match_a_monte_carlo_simulation() {
        let mut gen = Lcg(2024);
        let cases = [(100.0, 2.0, 50.0, 1.5), (30.0, 1.0, 12.0, 0.4), (-40.0, 3.0, 25.0, 2.0)];
        for (m1, s1, m2, s2) in cases {
            for op in [BinOp::Add, BinOp::Sub, BinOp::Mul, BinOp::Div] {
                let (mean, sigma) = gauss_parts(&binary(op, &g(m1, s1), &g(m2, s2)).unwrap().unwrap());
                let n = 60000;
                let (mut sum, mut sumsq) = (0.0, 0.0);
                for _ in 0..n {
                    let x = m1 + s1 * gen.normal();
                    let y = m2 + s2 * gen.normal();
                    let z = match op {
                        BinOp::Add => x + y,
                        BinOp::Sub => x - y,
                        BinOp::Mul => x * y,
                        BinOp::Div => x / y,
                    };
                    sum += z;
                    sumsq += z * z;
                }
                let emp_mean = sum / n as f64;
                let emp_sigma = (sumsq / n as f64 - emp_mean * emp_mean).sqrt();
                let tol = 0.05 * sigma.max(1e-9);
                assert!((emp_mean - mean).abs() <= 0.05 * sigma + 0.002 * mean.abs(), "{:?} mean {} vs {}", op, emp_mean, mean);
                assert!((emp_sigma - sigma).abs() <= tol + 0.02 * sigma, "{:?} sigma {} vs {} for ({}±{}) ({}±{})", op, emp_sigma, sigma, m1, s1, m2, s2);
            }
        }
    }

    #[test]
    fn gauss_comparisons_need_about_95_percent_confidence() {
        // combined sigma = 5; difference 10 is 2 sigma -> decided; 5 is 1 sigma -> undecided
        assert_eq!(trit(CmpOp::Lt, g(0.0, 3.0), g(10.0, 4.0)), 1);
        assert_eq!(trit(CmpOp::Gt, g(0.0, 3.0), g(10.0, 4.0)), -1);
        assert_eq!(trit(CmpOp::Lt, g(0.0, 3.0), g(5.0, 4.0)), 0);
        assert_eq!(trit(CmpOp::Gt, g(10.0, 3.0), Value::Number(0.0)), 1); // 3.3 sigma vs exact 0
        assert_eq!(trit(CmpOp::Eq, g(10.0, 3.0), Value::Number(0.0)), -1);
        assert_eq!(trit(CmpOp::Ne, g(10.0, 3.0), Value::Number(0.0)), 1);
        assert_eq!(trit(CmpOp::Eq, g(10.0, 3.0), Value::Number(9.0)), 0);
    }

    #[test]
    fn exact_gauss_values_compare_like_numbers() {
        assert_eq!(trit(CmpOp::Lt, g(1.0, 0.0), g(2.0, 0.0)), 1);
        assert_eq!(trit(CmpOp::Ge, g(1.0, 0.0), g(2.0, 0.0)), -1);
        assert_eq!(trit(CmpOp::Eq, g(2.0, 0.0), Value::Number(2.0)), 1);
    }

    #[test]
    fn certain_gauss_claims_hold_in_nearly_all_samples() {
        let mut gen = Lcg(99);
        let mut decided = 0;
        let mut wrong = 0;
        for _ in 0..2000 {
            let (m1, s1) = (gen.between(-10.0, 10.0), gen.between(0.1, 3.0));
            let (m2, s2) = (gen.between(-10.0, 10.0), gen.between(0.1, 3.0));
            let t = trit(CmpOp::Lt, g(m1, s1), g(m2, s2));
            if t == 0 {
                continue;
            }
            decided += 1;
            for _ in 0..20 {
                let truth = m1 + s1 * gen.normal() < m2 + s2 * gen.normal();
                if (t == 1) != truth {
                    wrong += 1;
                }
            }
        }
        assert!(decided > 200, "too few decided cases: {}", decided);
        // a 95%-confidence claim can be wrong at most ~2.5% of the time (plus slack)
        assert!((wrong as f64) < 0.03 * (decided * 20) as f64, "wrong {} of {}", wrong, decided * 20);
    }

    #[test]
    fn probabilities_for_gauss_values() {
        let p = prob_greater(&g(10.0, 3.0), &g(0.0, 4.0)).unwrap(); // Phi(2)
        assert!((p - 0.9772499).abs() < 1e-5);
        assert!((prob_greater(&g(5.0, 1.0), &g(5.0, 2.0)).unwrap() - 0.5).abs() < 1e-9);
        assert_eq!(prob_greater(&Value::Number(2.0), &Value::Number(1.0)).unwrap(), 1.0);
        assert!(prob_greater(&u(1.0, 0.5), &Value::Number(0.0)).is_err());
    }

    #[test]
    fn interval_and_gauss_do_not_mix() {
        assert!(binary(BinOp::Add, &u(1.0, 0.1), &g(1.0, 0.1)).unwrap().is_err());
        assert!(compare(CmpOp::Lt, &u(1.0, 0.1), &g(1.0, 0.1)).unwrap().is_err());
    }

    #[test]
    fn gauss_division_and_sqrt_domains() {
        assert!(binary(BinOp::Div, &Value::Number(1.0), &g(1.0, 1.0)).unwrap().is_err());
        assert!(binary(BinOp::Div, &Value::Number(1.0), &g(10.0, 1.0)).unwrap().is_ok());
        let (m, s) = gauss_parts(&sqrt_gauss(100.0, 4.0).unwrap());
        assert!((m - 10.0).abs() < 1e-12 && (s - 0.2).abs() < 1e-12);
        assert!(sqrt_gauss(1.0, 1.0).is_err());
        assert_eq!(gauss_parts(&abs_gauss(-5.0, 1.0)), (5.0, 1.0));
    }
}
