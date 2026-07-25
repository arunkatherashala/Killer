use crate::error::VmError;
use crate::value::Value;

/// Arithmetic and comparison operations for the Virtual Machine
pub struct Operations;

impl Operations {
    /// Binary addition (supports number + number, string concatenation)
    pub fn add(lhs: &Value, rhs: &Value) -> Result<Value, VmError> {
        match (lhs, rhs) {
            (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l + r)),
            (Value::Str(l), _) => Ok(Value::Str(format!("{}{}", l, rhs))),
            (_, Value::Str(r)) => Ok(Value::Str(format!("{}{}", lhs, r))),
            (Value::Array(l), Value::Array(r)) => {
                let mut v = l.to_vec();
                v.extend(r.to_vec());
                Ok(Value::from(v))
            }
            _ => Err(VmError::runtime_error("Type error in +".to_string())),
        }
    }

    /// Binary subtraction (numbers only)
    pub fn sub(lhs: f64, rhs: f64) -> Value {
        Value::Number(lhs - rhs)
    }

    /// Binary multiplication (numbers only, or string repetition)
    pub fn mul(lhs: &Value, rhs: &Value) -> Result<Value, VmError> {
        match (lhs, rhs) {
            (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l * r)),
            (Value::Str(s), Value::Number(n)) | (Value::Number(n), Value::Str(s)) => {
                let count = *n as usize;
                Ok(Value::Str(s.repeat(count)))
            }
            _ => Err(VmError::runtime_error("Type error in *".to_string())),
        }
    }

    /// Binary division (numbers only)
    pub fn div(lhs: f64, rhs: f64) -> Result<Value, VmError> {
        if rhs == 0.0 {
            Ok(Value::Number(f64::INFINITY))
        } else {
            Ok(Value::Number(lhs / rhs))
        }
    }

    /// Binary modulo (numbers only)
    pub fn modulo(lhs: f64, rhs: f64) -> Result<Value, VmError> {
        if rhs == 0.0 {
            Ok(Value::Number(f64::NAN))
        } else {
            Ok(Value::Number(lhs % rhs))
        }
    }

    /// Binary exponentiation (numbers only)
    pub fn pow(lhs: f64, rhs: f64) -> Value {
        Value::Number(lhs.powf(rhs))
    }

    /// Equality comparison
    pub fn eq(lhs: &Value, rhs: &Value) -> Value {
        Value::Bool(lhs == rhs)
    }

    /// Inequality comparison
    pub fn ne(lhs: &Value, rhs: &Value) -> Value {
        Value::Bool(lhs != rhs)
    }

    /// Greater than (numbers only)
    pub fn gt(lhs: f64, rhs: f64) -> Value {
        Value::Bool(lhs > rhs)
    }

    /// Less than (numbers only)
    pub fn lt(lhs: f64, rhs: f64) -> Value {
        Value::Bool(lhs < rhs)
    }

    /// Greater than or equal (numbers only)
    pub fn ge(lhs: f64, rhs: f64) -> Value {
        Value::Bool(lhs >= rhs)
    }

    /// Less than or equal (numbers only)
    pub fn le(lhs: f64, rhs: f64) -> Value {
        Value::Bool(lhs <= rhs)
    }

    /// Logical AND
    pub fn and(lhs: bool, rhs: bool) -> Value {
        Value::Bool(lhs && rhs)
    }

    /// Logical OR
    pub fn or(lhs: bool, rhs: bool) -> Value {
        Value::Bool(lhs || rhs)
    }

    /// Logical NOT
    pub fn not(val: bool) -> Value {
        Value::Bool(!val)
    }

    /// Unary negation (for numbers)
    pub fn negate(val: &Value) -> Result<Value, VmError> {
        match val {
            Value::Number(n) => Ok(Value::Number(-n)),
            _ => Err(VmError::runtime_error(
                "Cannot negate non-number".to_string(),
            )),
        }
    }

    /// Prefix increment
    pub fn prefix_inc(val: &Value) -> Result<Value, VmError> {
        match val {
            Value::Number(n) => Ok(Value::Number(n + 1.0)),
            _ => Err(VmError::runtime_error(
                "Cannot increment non-number".to_string(),
            )),
        }
    }

    /// Prefix decrement
    pub fn prefix_dec(val: &Value) -> Result<Value, VmError> {
        match val {
            Value::Number(n) => Ok(Value::Number(n - 1.0)),
            _ => Err(VmError::runtime_error(
                "Cannot decrement non-number".to_string(),
            )),
        }
    }

    /// Postfix increment (returns original value)
    pub fn postfix_inc(val: &Value) -> Result<Value, VmError> {
        match val {
            Value::Number(n) => Ok(Value::Number(*n)), // Returns original
            _ => Err(VmError::runtime_error(
                "Cannot increment non-number".to_string(),
            )),
        }
    }

    /// Postfix decrement (returns original value)
    pub fn postfix_dec(val: &Value) -> Result<Value, VmError> {
        match val {
            Value::Number(n) => Ok(Value::Number(*n)), // Returns original
            _ => Err(VmError::runtime_error(
                "Cannot decrement non-number".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_numbers() {
        let result = Operations::add(&Value::Number(2.0), &Value::Number(3.0)).unwrap();
        assert_eq!(result, Value::Number(5.0));
    }

    #[test]
    fn test_add_strings() {
        let result = Operations::add(&Value::Str("Hello".to_string()), &Value::Str(" World".to_string())).unwrap();
        assert_eq!(result, Value::Str("Hello World".to_string()));
    }

    #[test]
    fn test_mul_repeat_string() {
        let result = Operations::mul(&Value::Str("a".to_string()), &Value::Number(3.0)).unwrap();
        assert_eq!(result, Value::Str("aaa".to_string()));
    }

    #[test]
    fn test_comparison() {
        assert_eq!(Operations::gt(5.0, 3.0), Value::Bool(true));
        assert_eq!(Operations::lt(5.0, 3.0), Value::Bool(false));
    }
}
