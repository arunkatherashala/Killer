use std::collections::HashMap;

fn main() {
    println!("{}", [to_string(&Value::Str("=== Killer Calculator ===".to_string()))].join(" "));
    let mut a = Value::Number(10f64);
    let mut b = Value::Number(3f64);
    println!("{}", [to_string(&Value::Str("Numbers:".to_string())), to_string(&a), to_string(&b)].join(" "));
    let mut result = bin_op(&a, "Add", &b);
    println!("{}", [to_string(&Value::Str("Addition:".to_string())), to_string(&a), to_string(&Value::Str("+".to_string())), to_string(&b), to_string(&Value::Str("=".to_string())), to_string(&result)].join(" "));
    result = bin_op(&a, "Sub", &b);
    println!("{}", [to_string(&Value::Str("Subtraction:".to_string())), to_string(&a), to_string(&Value::Str("-".to_string())), to_string(&b), to_string(&Value::Str("=".to_string())), to_string(&result)].join(" "));
    result = bin_op(&a, "Mul", &b);
    println!("{}", [to_string(&Value::Str("Multiplication:".to_string())), to_string(&a), to_string(&Value::Str("*".to_string())), to_string(&b), to_string(&Value::Str("=".to_string())), to_string(&result)].join(" "));
    result = bin_op(&a, "Div", &b);
    println!("{}", [to_string(&Value::Str("Division:".to_string())), to_string(&a), to_string(&Value::Str("/".to_string())), to_string(&b), to_string(&Value::Str("=".to_string())), to_string(&result)].join(" "));
    result = bin_op(&a, "Mod", &b);
    println!("{}", [to_string(&Value::Str("Modulo:".to_string())), to_string(&a), to_string(&Value::Str("%".to_string())), to_string(&b), to_string(&Value::Str("=".to_string())), to_string(&result)].join(" "));
    println!("{}", [to_string(&Value::Str("---".to_string()))].join(" "));
    println!("{}", [to_string(&Value::Str("Comparing numbers:".to_string()))].join(" "));
    if is_truthy(&bin_op(&a, "Gt", &b)) {
        println!("{}", [to_string(&a), to_string(&Value::Str("is greater than".to_string())), to_string(&b)].join(" "));
    }
    if is_truthy(&bin_op(&a, "Eq", &b)) {
        println!("{}", [to_string(&a), to_string(&Value::Str("equals".to_string())), to_string(&b)].join(" "));
    } else {
        println!("{}", [to_string(&a), to_string(&Value::Str("does not equal".to_string())), to_string(&b)].join(" "));
    }
    println!("{}", [to_string(&Value::Str("---".to_string()))].join(" "));
    println!("{}", [to_string(&Value::Str("Sum from 1 to 5:".to_string()))].join(" "));
    let mut sum_result = Value::Number(0f64);
    let mut num = Value::Number(1f64);
    while is_truthy(&bin_op(&num, "Le", &Value::Number(5f64))) {
        sum_result = bin_op(&sum_result, "Add", &num);
        num = bin_op(&num, "Add", &Value::Number(1f64));
    }
    println!("{}", [to_string(&Value::Str("Sum:".to_string())), to_string(&sum_result)].join(" "));
    println!("{}", [to_string(&Value::Str("--- End of Example 4 ---".to_string()))].join(" "));
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