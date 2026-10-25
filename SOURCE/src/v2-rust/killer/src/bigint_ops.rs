//! Operators and builtins on [`Value::BigInt`].
//!
//! Rules (also in DOCS.md):
//! * `Number` arithmetic is untouched: nothing promotes to `BigInt` unless an operand already is one.
//! * `BigInt` with `BigInt` or with an *integral* `Number`/`Integer`: exact `BigInt` result.
//! * `BigInt` with a non-integral (or non-finite) `Number`: the `BigInt` becomes an `f64` and the
//!   result is a `Number`.
//! * `/`: exact `BigInt` when the division is exact, otherwise a `Number` (float) quotient.
//! * `%` (and `mod`) and `idiv`/`divmod` use floor semantics (the remainder has the sign of the divisor);
//!   `rem` is the truncated remainder (sign of the dividend, like Rust `%` or C `fmod`).
//! * `**`: a non-negative integral exponent gives an exact `BigInt` (guarded against absurd sizes);
//!   a negative or fractional exponent gives a `Number`.
//! * comparisons with a `Number` are exact.

use crate::bigint::BigInt;
use crate::value::Value;
use std::cmp::Ordering;
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    /// Truncated remainder (`rem` builtin): the sign of the dividend.
    Rem,
    Pow,
}

impl Op {
    fn symbol(self) -> &'static str {
        match self {
            Op::Add => "+",
            Op::Sub => "-",
            Op::Mul => "*",
            Op::Div => "/",
            Op::Mod => "%",
            Op::Rem => "rem",
            Op::Pow => "**",
        }
    }
}

/// The digits of a bigint literal (`123n` -> `123`, `0xFFn` -> `0xFF`), or `None` if `expr` is not one.
pub fn literal_text(expr: &str) -> Option<&str> {
    let body = expr.strip_suffix('n')?;
    let valid = if let Some(hex) = body.strip_prefix("0x").or_else(|| body.strip_prefix("0X")) {
        !hex.is_empty() && hex.bytes().all(|c| c.is_ascii_hexdigit() || c == b'_')
    } else {
        !body.is_empty() && body.bytes().all(|c| c.is_ascii_digit() || c == b'_') && body.as_bytes()[0].is_ascii_digit()
    };
    if valid { Some(body) } else { None }
}

/// True when either operand is a `BigInt` (the only case this module handles).
#[inline]
pub fn involves(a: &Value, b: &Value) -> bool {
    matches!(a, Value::BigInt(_)) || matches!(b, Value::BigInt(_))
}

/// A bigint that fits comfortably in an `f64` mantissa (|v| <= 2^53) as a plain number: used so
/// that an index, slice bound, repeat count or builtin argument may be a bigint.
pub fn small_number(v: &Value) -> Option<f64> {
    match v {
        Value::BigInt(b) => b.to_i64().filter(|i| i.unsigned_abs() <= (1u64 << 53)).map(|i| i as f64),
        _ => None,
    }
}

/// `v` as a `Number` when it is a small bigint, otherwise unchanged.
pub fn demote_index(v: Value) -> Value {
    match small_number(&v) {
        Some(n) => Value::Number(n),
        None => v,
    }
}

/// A whole-number text (`[+-]digits`) too large for an exact f64 (|n| > 2^53) as a `BigInt`
/// value; `None` for anything else (callers then use the float path). Used by `int("...")` and
/// JSON parsing so big integers keep every digit.
pub fn big_from_digits(text: &str) -> Option<Value> {
    let t = text.trim();
    let digits = t.strip_prefix(|c| c == '-' || c == '+').unwrap_or(t);
    if digits.len() < 16 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let b = BigInt::parse(t.strip_prefix('+').unwrap_or(t)).ok()?;
    match b.to_i64() {
        Some(i) if i.unsigned_abs() <= (1u64 << 53) => None,
        _ => Some(big(b)),
    }
}

pub fn big(v: BigInt) -> Value {
    Value::BigInt(Rc::new(v))
}

// ---- automatic promotion of whole-number results -------------------------------------------------
//
// A plain number is an f64, exact for whole numbers only up to 2^53. When both operands of `+ - * **`
// are whole numbers and the float result reaches 2^53 (so it may be rounded), the exact result is
// computed as a BigInt instead, like Python's int. Anything else (a fractional operand, an operand
// beyond +-2^63, infinities, results below 2^53) stays a plain float, exactly as before.

/// 2^53: whole floats at or above this may already be rounded.
pub const EXACT_LIMIT: f64 = 9_007_199_254_740_992.0;
/// Operands up to about 2^63 take part in promotion (larger floats are treated as floats).
const OPERAND_LIMIT: f64 = 9.3e18;

#[inline(always)]
fn whole_operands(x: f64, y: f64) -> bool {
    x.fract() == 0.0 && y.fract() == 0.0 && x.abs() < OPERAND_LIMIT && y.abs() < OPERAND_LIMIT
}

/// `x + 0`, `x - 0`, `0 - x`, `x * 1` and `x * -1` (negation) never lose precision, so they never promote:
/// `-9223372036854775807` stays a plain number.
#[inline(always)]
fn identity_op(op: Op, x: f64, y: f64) -> bool {
    match op {
        Op::Add | Op::Sub => x == 0.0 || y == 0.0,
        _ => x.abs() == 1.0 || y.abs() == 1.0,
    }
}

#[cold]
fn exact(op: Op, x: f64, y: f64, fallback: f64) -> Value {
    arith(op, &Value::Number(x), &Value::Number(y)).unwrap_or(Value::Number(fallback))
}

/// `x + y` for two plain numbers.
#[inline(always)]
pub fn add(x: f64, y: f64) -> Value {
    let r = x + y;
    if r.abs() >= EXACT_LIMIT && r.is_finite() && whole_operands(x, y) && !identity_op(Op::Add, x, y) {
        return exact(Op::Add, x, y, r);
    }
    Value::Number(r)
}

/// `x - y` for two plain numbers.
#[inline(always)]
pub fn sub(x: f64, y: f64) -> Value {
    let r = x - y;
    if r.abs() >= EXACT_LIMIT && r.is_finite() && whole_operands(x, y) && !identity_op(Op::Sub, x, y) {
        return exact(Op::Sub, x, y, r);
    }
    Value::Number(r)
}

/// `x * y` for two plain numbers.
#[inline(always)]
pub fn mul(x: f64, y: f64) -> Value {
    let r = x * y;
    if r.abs() >= EXACT_LIMIT && r.is_finite() && whole_operands(x, y) && !identity_op(Op::Mul, x, y) {
        return exact(Op::Mul, x, y, r);
    }
    Value::Number(r)
}

/// `base ** exp` / `pow(base, exp)` for two plain numbers. A whole base with a whole exponent from 0 to
/// 4096 whose float result reaches 2^53 (including results too large for a float) is computed exactly.
#[inline]
pub fn pow(base: f64, exp: f64) -> Value {
    let r = base.powf(exp);
    if r.abs() >= EXACT_LIMIT
        && exp >= 0.0
        && exp <= 4096.0
        && base.abs() < OPERAND_LIMIT
        && whole_operands(base, exp)
    {
        return exact(Op::Pow, base, exp, r);
    }
    Value::Number(r)
}

/// Would `x op y` be promoted? Used by constant folding to leave such operations to run time.
pub fn would_promote(op_char: char, x: f64, y: f64) -> bool {
    let (r, op) = match op_char {
        '+' => (x + y, Op::Add),
        '-' => (x - y, Op::Sub),
        '*' => (x * y, Op::Mul),
        _ => return false,
    };
    r.abs() >= EXACT_LIMIT && r.is_finite() && whole_operands(x, y) && !identity_op(op, x, y)
}

/// An operand reduced to what the arithmetic needs.
enum Operand {
    Int(BigInt),
    Float(f64),
}

fn operand(v: &Value, op: Op, side: &str) -> Result<Operand, String> {
    match v {
        Value::BigInt(b) => Ok(Operand::Int((**b).clone())),
        Value::Number(n) => Ok(match BigInt::from_f64(*n) {
            Some(b) => Operand::Int(b),
            None => Operand::Float(*n),
        }),
        Value::Integer(i) => Ok(Operand::Int(BigInt::from_i64(*i))),
        Value::Bool(b) => Ok(Operand::Int(BigInt::from_i64(*b as i64))),
        other => Err(format!(
            "Type error in '{}': {} operand must be a number or bigint, got {}",
            op.symbol(),
            side,
            other.type_name()
        )),
    }
}

fn as_f64(o: &Operand) -> f64 {
    match o {
        Operand::Int(b) => b.to_f64(),
        Operand::Float(f) => *f,
    }
}

/// `lhs op rhs` where at least one side is a `BigInt`.
pub fn arith(op: Op, lhs: &Value, rhs: &Value) -> Result<Value, String> {
    let a = operand(lhs, op, "left")?;
    let b = operand(rhs, op, "right")?;
    let (x, y) = match (&a, &b) {
        (Operand::Int(x), Operand::Int(y)) => (x, y),
        _ => return Ok(float_arith(op, as_f64(&a), as_f64(&b))),
    };
    Ok(match op {
        Op::Add => big(x.add(y)),
        Op::Sub => big(x.sub(y)),
        Op::Mul => big(x.mul(y)),
        Op::Div => {
            let (q, r) = x.divmod(y).ok_or_else(|| "Division by zero".to_string())?;
            if r.is_zero() {
                big(q)
            } else {
                Value::Number(x.to_f64() / y.to_f64())
            }
        }
        Op::Mod => {
            let (_, r) = x.divmod(y).ok_or_else(|| "Modulo by zero".to_string())?;
            big(r)
        }
        Op::Rem => {
            // divmod is floor (r has the sign of y); shift to the sign of x when they differ
            let (_, r) = x.divmod(y).ok_or_else(|| "Modulo by zero".to_string())?;
            if r.is_zero() || x.is_negative() == r.is_negative() {
                big(r)
            } else {
                big(r.sub(y))
            }
        }
        Op::Pow => {
            if y.is_negative() {
                Value::Number(x.to_f64().powf(y.to_f64()))
            } else {
                let exp = y.to_i64().ok_or_else(|| "exponent too large".to_string())? as u64;
                big(x.pow(exp)?)
            }
        }
    })
}

/// Floor modulo on floats (Python `%`): the result has the sign of the divisor, so
/// `-7 % 3 == 2`, `7 % -3 == -2`, `-5.5 % 2 == 0.5`.
///
/// Built from the truncated remainder `r` (Rust `%` / C `fmod`): when `r != 0` and its sign differs
/// from the divisor's, `r += b`. Special values follow Python: `x % inf` is `x` for `x >= 0`,
/// `-x % inf` is `inf` (and `x % -inf` is `-inf` for `x > 0`); `inf % x` and `x % NaN` are NaN. A zero
/// result is always `+0.0` (never `-0.0`), so it prints as `0`. The caller handles a zero divisor.
#[inline]
pub fn floor_mod(a: f64, b: f64) -> f64 {
    let r = a % b + 0.0; // + 0.0 turns -0.0 into +0.0 (NaN stays NaN)
    if r != 0.0 && (r < 0.0) != (b < 0.0) {
        r + b
    } else {
        r
    }
}

/// Truncated remainder (`rem` builtin; C `fmod`, Rust `%`): the result has the sign of the dividend.
#[inline]
pub fn trunc_rem(a: f64, b: f64) -> f64 {
    a % b
}

fn float_arith(op: Op, x: f64, y: f64) -> Value {
    Value::Number(match op {
        Op::Add => x + y,
        Op::Sub => x - y,
        Op::Mul => x * y,
        Op::Div => x / y,
        Op::Mod => floor_mod(x, y),
        Op::Rem => trunc_rem(x, y),
        Op::Pow => x.powf(y),
    })
}

/// Exact ordering of a `BigInt` against another number. `None` for NaN or a non-numeric operand.
pub fn compare(lhs: &Value, rhs: &Value) -> Option<Ordering> {
    match (lhs, rhs) {
        (Value::BigInt(a), Value::BigInt(b)) => Some(a.cmp(b)),
        (Value::BigInt(a), Value::Number(n)) => a.cmp_f64(*n),
        (Value::Number(n), Value::BigInt(b)) => b.cmp_f64(*n).map(Ordering::reverse),
        (Value::BigInt(a), Value::Integer(i)) => Some((**a).cmp(&BigInt::from_i64(*i))),
        (Value::Integer(i), Value::BigInt(b)) => Some(BigInt::from_i64(*i).cmp(b)),
        _ => None,
    }
}

/// `==` with at least one `BigInt` operand.
pub fn equals(lhs: &Value, rhs: &Value) -> bool {
    compare(lhs, rhs) == Some(Ordering::Equal)
}

/// Ordering for `< <= > >=`; an error message when the operands cannot be compared.
pub fn order(lhs: &Value, rhs: &Value) -> Result<Option<Ordering>, String> {
    match (lhs, rhs) {
        (Value::BigInt(_) | Value::Number(_) | Value::Integer(_), Value::BigInt(_) | Value::Number(_) | Value::Integer(_)) => {
            Ok(compare(lhs, rhs))
        }
        _ => Err(format!("Cannot compare {} with {}", lhs.type_name(), rhs.type_name())),
    }
}

// ---------------------------------------------------------------------------------------------
// builtins
// ---------------------------------------------------------------------------------------------

/// Convert a builtin argument to a `BigInt` (integral numbers, bigints, integer strings).
fn to_big(v: &Value, fname: &str) -> Result<BigInt, String> {
    match v {
        Value::BigInt(b) => Ok((**b).clone()),
        Value::Number(n) => BigInt::from_f64(*n)
            .ok_or_else(|| format!("{}(): expected an integer, got {}", fname, v)),
        Value::Integer(i) => Ok(BigInt::from_i64(*i)),
        Value::Str(s) => BigInt::parse(s).map_err(|e| format!("{}(): {}", fname, e)),
        other => Err(format!("{}(): expected an integer, got {}", fname, other.type_name())),
    }
}

fn arity(args: &[Value], n: usize, usage: &str) -> Result<(), String> {
    if args.len() != n {
        Err(format!("{} expects {} argument{}", usage, n, if n == 1 { "" } else { "s" }))
    } else {
        Ok(())
    }
}

/// A result narrowed back to a `Number` when every input was one.
fn narrow(b: BigInt) -> Value {
    Value::Number(b.to_f64())
}

/// The bigint builtins: `big`, `factorial`, `divmod`, `idiv`, `powmod`, `isqrt`, `bit_length`, `gcd`.
pub fn call(name: &str, args: &[Value]) -> Result<Value, String> {
    let any_big = args.iter().any(|a| matches!(a, Value::BigInt(_)));
    match name {
        "big" => {
            arity(args, 1, "big(x)")?;
            Ok(big(to_big(&args[0], "big")?))
        }
        "factorial" => {
            arity(args, 1, "factorial(n)")?;
            let n = to_big(&args[0], "factorial")?;
            if n.is_negative() {
                return Err("factorial(): negative argument".to_string());
            }
            let n = n.to_i64().ok_or_else(|| "factorial(): argument too large".to_string())? as u64;
            Ok(big(BigInt::factorial(n)?))
        }
        "mod" | "rem" => {
            arity(args, 2, &format!("{}(a, b)", name))?;
            let op = if name == "mod" { Op::Mod } else { Op::Rem };
            if any_big {
                return arith(op, &args[0], &args[1]);
            }
            let num = |v: &Value| match v {
                Value::Number(n) => Ok(*n),
                Value::Integer(i) => Ok(*i as f64),
                Value::Bool(b) => Ok(*b as i64 as f64),
                other => Err(format!("{}(): expected numbers, got {}", name, other.type_name())),
            };
            let (x, y) = (num(&args[0])?, num(&args[1])?);
            if y == 0.0 {
                return Err("Modulo by zero".to_string());
            }
            Ok(Value::Number(if op == Op::Mod { floor_mod(x, y) } else { trunc_rem(x, y) }))
        }
        "divmod" | "idiv" => {
            arity(args, 2, &format!("{}(a, b)", name))?;
            let (a, b) = (&args[0], &args[1]);
            let all_numbers = !any_big && matches!(a, Value::Number(_)) && matches!(b, Value::Number(_));
            if all_numbers || (!any_big && (matches!(a, Value::Number(n) if BigInt::from_f64(*n).is_none()) || matches!(b, Value::Number(n) if BigInt::from_f64(*n).is_none()))) {
                // plain floats: floor semantics on f64
                let (x, y) = match (a, b) {
                    (Value::Number(x), Value::Number(y)) => (*x, *y),
                    _ => return Err(format!("{}(): expected numbers", name)),
                };
                if y == 0.0 {
                    return Err("Division by zero".to_string());
                }
                let q = (x / y).floor();
                return Ok(if name == "idiv" {
                    Value::Number(q)
                } else {
                    Value::from(vec![Value::Number(q), Value::Number(floor_mod(x, y))])
                });
            }
            let (x, y) = (to_big(a, name)?, to_big(b, name)?);
            let (q, r) = x.divmod(&y).ok_or_else(|| "Division by zero".to_string())?;
            Ok(if name == "idiv" { big(q) } else { Value::from(vec![big(q), big(r)]) })
        }
        "powmod" => {
            arity(args, 3, "powmod(base, exp, mod)")?;
            let all_numbers = !any_big && args.iter().all(|a| matches!(a, Value::Number(_)));
            let r = to_big(&args[0], "powmod")?.powmod(&to_big(&args[1], "powmod")?, &to_big(&args[2], "powmod")?)?;
            Ok(if all_numbers { narrow(r) } else { big(r) })
        }
        "isqrt" => {
            arity(args, 1, "isqrt(n)")?;
            let r = to_big(&args[0], "isqrt")?.isqrt().ok_or_else(|| "isqrt(): negative argument".to_string())?;
            Ok(if any_big { big(r) } else { narrow(r) })
        }
        "bit_length" => {
            arity(args, 1, "bit_length(n)")?;
            Ok(Value::Number(to_big(&args[0], "bit_length")?.bit_length() as f64))
        }
        "gcd" => {
            if args.len() < 2 {
                return Err("gcd() expects at least 2 arguments".to_string());
            }
            let mut acc = to_big(&args[0], "gcd")?;
            for a in &args[1..] {
                acc = acc.gcd(&to_big(a, "gcd")?);
            }
            Ok(if any_big { big(acc) } else { narrow(acc) })
        }
        _ => Err(format!("unknown bigint builtin `{}`", name)),
    }
}

/// JSON text of an array value (numbers, bigints as plain number text, strings, bools, null,
/// nested arrays and dicts).
pub fn json_array(v: &Value) -> String {
    fn item(v: &Value) -> String {
        match v {
            Value::Str(s) => format!("\"{}\"", s.replace('"', "\\\"")),
            Value::Number(n) => n.to_string(),
            Value::BigInt(b) => b.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Array(_) => json_array(v),
            Value::Dict(d) => {
                let parts: Vec<String> = d.iter().map(|(k, x)| format!("\"{}\":{}", k, item(&x))).collect();
                format!("{{{}}}", parts.join(","))
            }
            other => format!("\"{}\"", other),
        }
    }
    match v {
        Value::Array(a) => format!("[{}]", a.to_vec().iter().map(item).collect::<Vec<_>>().join(",")),
        other => item(other),
    }
}

fn order_any(a: &Value, b: &Value) -> Option<Ordering> {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.partial_cmp(y),
        _ => compare(a, b),
    }
}

/// Builtins that must understand a bigint argument natively: `int`, `float`, `round`, `floor`,
/// `ceil`, `trunc` (a bigint is already whole), and `min`, `max`, `sum` over bigints (exact).
/// Returns `None` when the call has no bigint in it or the builtin is not one of these, so the
/// normal implementation runs.
pub fn native_builtin(name: &str, args: &[Value]) -> Option<Result<Value, String>> {
    if !matches!(name, "int" | "float" | "round" | "floor" | "ceil" | "trunc" | "min" | "max" | "sum") {
        return None;
    }
    let scan_arrays = matches!(name, "min" | "max" | "sum");
    if !args.iter().any(|a| matches!(a, Value::BigInt(_)) || (scan_arrays && matches!(a, Value::Array(arr) if arr.to_vec().iter().any(|x| matches!(x, Value::BigInt(_)))))) {
        return None;
    }
    match (name, args) {
        ("int" | "round" | "floor" | "ceil" | "trunc", [v @ Value::BigInt(_)]) => Some(Ok(v.clone())),
        ("float", [Value::BigInt(b)]) => Some(Ok(Value::Number(b.to_f64()))),
        ("min" | "max", _) => {
            let items: Vec<Value> = match args {
                [Value::Array(a)] => a.to_vec(),
                _ => args.to_vec(),
            };
            let mut it = items.into_iter();
            let mut best = match it.next() {
                Some(v) => v,
                None => return Some(Err(format!("{}() of an empty sequence", name))),
            };
            for v in it {
                match order_any(&v, &best) {
                    Some(o) => {
                        if (name == "min" && o == Ordering::Less) || (name == "max" && o == Ordering::Greater) {
                            best = v;
                        }
                    }
                    None => return Some(Err(format!("{}() expects numbers", name))),
                }
            }
            Some(Ok(best))
        }
        ("sum", [Value::Array(a)]) => {
            let mut acc = Value::Number(0.0);
            for v in a.to_vec() {
                acc = match arith(Op::Add, &acc, &v) {
                    Ok(r) => r,
                    Err(e) => return Some(Err(e)),
                };
            }
            Some(Ok(acc))
        }
        _ => None,
    }
}

/// `args` with every small bigint replaced by a number (also inside a single array argument is
/// NOT done); `None` when nothing changes. Used to retry a builtin that rejected a bigint.
pub fn demote_args(args: &[Value]) -> Option<Vec<Value>> {
    if !args.iter().any(|a| small_number(a).is_some()) {
        return None;
    }
    Some(args.iter().map(|a| demote_index(a.clone())).collect())
}

/// Builtins whose arguments are counts, sizes or codes: a small bigint is just a number there.
const DEMOTE_NAMES: &[&str] = &["range", "chr", "pad_left", "pad_right", "repeat", "insert", "sleep_ms", "sleep", "sqrt"];

/// VM entry hook for every builtin call: native bigint builtins return a value; for the names in
/// `DEMOTE_NAMES` small bigints in `args` are replaced by numbers. `None`: carry on normally.
pub fn prepass(name: &str, args: &mut [Value]) -> Option<Result<Value, String>> {
    if !args.iter().any(|a| matches!(a, Value::BigInt(_) | Value::Array(_))) {
        return None;
    }
    if let Some(r) = native_builtin(name, args) {
        return Some(r);
    }
    if DEMOTE_NAMES.contains(&name) {
        for a in args.iter_mut() {
            if let Some(n) = small_number(a) {
                *a = Value::Number(n);
            }
        }
    }
    None
}
