//! Uncertainty-aware arithmetic and comparison for `Value::Uncertain { value, margin }`.
//!
//! `v ± m` is the guaranteed interval `[v - m, v + m]` (margins are absolute and non-negative).
//! Arithmetic returns an interval that *encloses* every possible result, so a program can rely on
//! the stated margin. Comparisons are three-valued and return a `Trit`:
//!
//! * `T_POS`  (+1) — the relation holds for every pair of values in the two intervals
//! * `T_NEG`  (-1) — the relation holds for none of them
//! * `T_ZERO` ( 0) — the intervals overlap, so it cannot be decided
//!
//! A plain number is an exact value (margin 0). Using `if` on a `Trit` takes the branch only for
//! `T_POS`, i.e. only when the condition is certain.

use crate::value::Value;

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

#[inline]
pub fn is_uncertain(v: &Value) -> bool {
    matches!(v, Value::Uncertain { .. })
}

/// (center, radius) of a numeric value; `None` for non-numeric values.
fn parts(v: &Value) -> Option<(f64, f64)> {
    match v {
        Value::Number(n) => Some((*n, 0.0)),
        Value::Integer(i) => Some((*i as f64, 0.0)),
        Value::Uncertain { value, margin } => Some((*value, margin.abs())),
        _ => None,
    }
}

fn make(value: f64, margin: f64) -> Value {
    Value::Uncertain { value, margin: margin.abs() }
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
    let (Some((v1, m1)), Some((v2, m2))) = (parts(a), parts(b)) else {
        return Some(Err(format!(
            "Type error in '{}': uncertain values combine only with numbers or other uncertain values",
            symbol
        )));
    };
    Some(match op {
        BinOp::Add => Ok(make(v1 + v2, m1 + m2)),
        BinOp::Sub => Ok(make(v1 - v2, m1 + m2)),
        // (v1±m1)(v2±m2) lies within v1*v2 ± (|v1|m2 + |v2|m1 + m1m2)
        BinOp::Mul => Ok(make(v1 * v2, v1.abs() * m2 + v2.abs() * m1 + m1 * m2)),
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
                Ok(make(center, (hi - center).max(center - lo)))
            }
        }
    })
}

/// Three-valued comparison where at least one operand is uncertain. Returns a `Value::Trit`.
pub fn compare(op: CmpOp, a: &Value, b: &Value) -> Option<Result<Value, String>> {
    if !is_uncertain(a) && !is_uncertain(b) {
        return None;
    }
    let (Some((v1, m1)), Some((v2, m2))) = (parts(a), parts(b)) else {
        return Some(Err("Cannot compare an uncertain value with a non-number".to_string()));
    };
    let (lo1, hi1, lo2, hi2) = (v1 - m1, v1 + m1, v2 - m2, v2 + m2);
    if lo1.is_nan() || hi1.is_nan() || lo2.is_nan() || hi2.is_nan() {
        return Some(Ok(Value::Trit(0)));
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
    Some(Ok(Value::Trit(if yes {
        1
    } else if no {
        -1
    } else {
        0
    })))
}

/// `sqrt` of an uncertain value (range must be non-negative).
pub fn sqrt(v: f64, m: f64) -> Result<Value, String> {
    let m = m.abs();
    if v - m < 0.0 {
        return Err("sqrt of an uncertain value whose range includes negative numbers".to_string());
    }
    let center = v.sqrt();
    let (lo, hi) = ((v - m).sqrt(), (v + m).sqrt());
    Ok(make(center, (hi - center).max(center - lo)))
}

/// `abs` of an uncertain value.
pub fn abs(v: f64, m: f64) -> Value {
    let m = m.abs();
    if v - m >= 0.0 {
        make(v, m)
    } else if v + m <= 0.0 {
        make(-v, m)
    } else {
        // the range straddles zero: result lies in [0, |v| + m]
        let half = (v.abs() + m) / 2.0;
        make(half, half)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(v: f64, m: f64) -> Value {
        Value::Uncertain { value: v, margin: m }
    }

    fn range(r: &Value) -> (f64, f64) {
        match r {
            Value::Uncertain { value, margin } => (value - margin, value + margin),
            Value::Number(n) => (*n, *n),
            other => panic!("not numeric: {:?}", other),
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
        let mut g = Lcg(42);
        for _ in 0..4000 {
            let (v1, m1) = (g.between(-50.0, 50.0), g.between(0.0, 5.0));
            let (v2, m2) = (g.between(-50.0, 50.0), g.between(0.0, 5.0));
            let (a, b) = (u(v1, m1), u(v2, m2));
            for op in [BinOp::Add, BinOp::Sub, BinOp::Mul, BinOp::Div] {
                let Ok(r) = binary(op, &a, &b).unwrap() else { continue };
                let (lo, hi) = range(&r);
                for _ in 0..8 {
                    let x = g.between(v1 - m1, v1 + m1);
                    let y = g.between(v2 - m2, v2 + m2);
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
        let mut g = Lcg(7);
        for _ in 0..3000 {
            let (v1, m1) = (g.between(-20.0, 20.0), g.between(0.0, 4.0));
            let (v2, m2) = (g.between(-20.0, 20.0), g.between(0.0, 4.0));
            for op in [CmpOp::Lt, CmpOp::Le, CmpOp::Gt, CmpOp::Ge] {
                let t = trit(op, u(v1, m1), u(v2, m2));
                for _ in 0..8 {
                    let x = g.between(v1 - m1, v1 + m1);
                    let y = g.between(v2 - m2, v2 + m2);
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
}
