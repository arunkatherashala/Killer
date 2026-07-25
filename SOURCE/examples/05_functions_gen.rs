use std::collections::HashMap;

fn main() {
    fn add(a: Value, b: Value) -> Value {
        return bin_op(&a, "Add", &b);
        Value::Null
    }
    fn greet(name: Value) -> Value {
        println!("{}", [format_display(&Value::Str("Hello, ".to_string())), format_display(&name)].join(" "));
        Value::Null
    }
    fn factorial(n: Value) -> Value {
        if is_truthy(&bin_op(&n, "Le", &Value::Number(1f64))) {
            return Value::Number(1f64);
        } else {
            return bin_op(&n, "Mul", &factorial(bin_op(&n, "Sub", &Value::Number(1f64))));
        }
        Value::Null
    }
    let mut result = add(Value::Number(5f64), Value::Number(3f64));
    println!("{}", [format_display(&Value::Str("5 + 3 =".to_string())), format_display(&result)].join(" "));
    // TODO: Expr(Call { callee: "greet", args: [String("World")] })
    // TODO: Expr(Call { callee: "greet", args: [String("Killer Language")] })
    let mut fac5 = factorial(Value::Number(5f64));
    println!("{}", [format_display(&Value::Str("5! =".to_string())), format_display(&fac5)].join(" "));
    println!("{}", [format_display(&Value::Str("--- Functions work! ---".to_string()))].join(" "));
}

fn format_display(val: &Value) -> String {
    match val {
        Value::Number(n) => {
            if n.fract() == 0.0 { (*n as i64).to_string() } else { n.to_string() }
        }
        Value::Str(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(|v| format_display(v)).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Dict(dict) => {
            let items: Vec<String> = dict.iter()
                .map(|(k, v)| format!("{}: {}", k, format_display(v)))
                .collect();
            format!("{{ {} }}", items.join(", "))
        }
        Value::Null => "null".to_string(),
    }
}

fn to_string(val: &Value) -> String { format_display(val) }

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