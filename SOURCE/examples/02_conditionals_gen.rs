use std::collections::HashMap;

fn main() {
    let mut age = Value::Number(20f64);
    println!("{}", [to_string(&Value::Str("Testing conditionals...".to_string()))].join(" "));
    if is_truthy(&bin_op(&age, "Ge", &Value::Number(18f64))) {
        println!("{}", [to_string(&Value::Str("You are an adult!".to_string()))].join(" "));
    }
    age = Value::Number(10f64);
    if is_truthy(&bin_op(&age, "Ge", &Value::Number(18f64))) {
        println!("{}", [to_string(&Value::Str("Adult".to_string()))].join(" "));
    } else {
        println!("{}", [to_string(&Value::Str("Minor".to_string()))].join(" "));
    }
    let mut num = Value::Number(15f64);
    if is_truthy(&bin_op(&num, "Gt", &Value::Number(20f64))) {
        println!("{}", [to_string(&Value::Str("Number is greater than 20".to_string()))].join(" "));
    } else {
        if is_truthy(&bin_op(&num, "Gt", &Value::Number(10f64))) {
            println!("{}", [to_string(&Value::Str("Number is between 10 and 20".to_string()))].join(" "));
        } else {
            println!("{}", [to_string(&Value::Str("Number is 10 or less".to_string()))].join(" "));
        }
    }
    println!("{}", [to_string(&Value::Str("--- End of Example 2 ---".to_string()))].join(" "));
}

fn to_string(val: &Value) -> String {
    match val {
        Value::Number(n) => {
            if n.fract() == 0.0 { (*n as i64).to_string() } else { n.to_string() }
        }
        Value::Str(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(|v| to_string(v)).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Dict(dict) => {
            let items: Vec<String> = dict.iter()
                .map(|(k, v)| format!("{}: {}", k, to_string(v)))
                .collect();
            format!("{{ {} }}", items.join(", "))
        }
        Value::Null => "null".to_string(),
    }
}

fn bin_op(left: &Value, op: &str, right: &Value) -> Value {
    match (left, right) {
        (Value::Number(l), Value::Number(r)) => {
            match op {
                "Add" | "+" => Value::Number(l + r),
                "Sub" | "-" => Value::Number(l - r),
                "Mul" | "*" => Value::Number(l * r),
                "Div" | "/" => Value::Number(l / r),
                "Mod" | "%" => Value::Number(l % r),
                "Eq" | "==" => Value::Bool((l - r).abs() < f64::EPSILON),
                "Ne" | "!=" => Value::Bool((l - r).abs() >= f64::EPSILON),
                "Lt" | "<" => Value::Bool(l < r),
                "Gt" | ">" => Value::Bool(l > r),
                "Le" | "<=" => Value::Bool(l <= r),
                "Ge" | ">=" => Value::Bool(l >= r),
                _ => Value::Null,
            }
        }
        (Value::Str(l), Value::Str(r)) => {
            match op {
                "Add" | "+" => Value::Str(format!("{}{}", l, r)),
                "Eq" | "==" => Value::Bool(l == r),
                "Ne" | "!=" => Value::Bool(l != r),
                _ => Value::Null,
            }
        }
        _ => Value::Null,
    }
}

fn is_truthy(val: &Value) -> bool {
    match val {
        Value::Bool(b) => *b,
        Value::Null => false,
        Value::Number(n) => *n != 0.0,
        Value::Str(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Dict(d) => !d.is_empty(),
    }
}

#[derive(Clone, Debug)]
enum Value {
    Number(f64),
    Str(String),
    Bool(bool),
    Array(Vec<Value>),
    Dict(HashMap<String, Value>),
    Null,
}