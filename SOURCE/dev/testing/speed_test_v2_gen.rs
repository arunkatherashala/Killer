use std::collections::HashMap;

fn main() {
    println!("{}", [to_string(&Value::Str("Starting Killer Speed Test...".to_string()))].join(" "));
    println!("{}", [to_string(&Value::Str("================================".to_string()))].join(" "));
    println!("{}", [to_string(&Value::Str("Test 1: Loop (100,000 iterations)".to_string()))].join(" "));
    let mut count = Value::Number(0f64);
    while is_truthy(&bin_op(&count, "Lt", &Value::Number(100000f64))) {
        count = bin_op(&count, "Add", &Value::Number(1f64));
    }
    println!("{}", [to_string(&Value::Str("Result: Complete".to_string()))].join(" "));
    println!("{}", [to_string(&Value::Str("Test 2: Arithmetic (50,000 ops)".to_string()))].join(" "));
    let mut result = Value::Number(0f64);
    let mut i = Value::Number(0f64);
    while is_truthy(&bin_op(&i, "Lt", &Value::Number(50000f64))) {
        result = bin_op(&bin_op(&bin_op(&result, "Add", &i), "Mul", &Value::Number(2f64)), "Div", &Value::Number(2f64));
        i = bin_op(&i, "Add", &Value::Number(1f64));
    }
    println!("{}", [to_string(&Value::Str("Result:".to_string())), to_string(&result)].join(" "));
    println!("{}", [to_string(&Value::Str("Test 3: Array (10,000 elements)".to_string()))].join(" "));
    let mut arr = Value::Array(vec![]);
    let mut j = Value::Number(0f64);
    while is_truthy(&bin_op(&j, "Lt", &Value::Number(10000f64))) {
        // TODO: Expr(MethodCall { object: Identifier("arr"), method: "push", args: [Identifier("j")] })
        j = bin_op(&j, "Add", &Value::Number(1f64));
    }
    println!("{}", [to_string(&Value::Str("Result: Array length =".to_string())), to_string(&Value::Null)].join(" "));
    println!("{}", [to_string(&Value::Str("Test 4: Recursion (fibonacci(20))".to_string()))].join(" "));
    fn fib(n: Value) -> Value {
        if is_truthy(&bin_op(&n, "Le", &Value::Number(1f64))) {
            return n;
        }
        return bin_op(&fib(bin_op(&n, "Sub", &Value::Number(1f64))), "Add", &fib(bin_op(&n, "Sub", &Value::Number(2f64))));
        Value::Null
    }
    let mut fib_val = fib(Value::Number(20f64));
    println!("{}", [to_string(&Value::Str("Result:".to_string())), to_string(&fib_val)].join(" "));
    println!("{}", [to_string(&Value::Str("Test 5: String Concatenation".to_string()))].join(" "));
    let mut s = Value::Str("".to_string());
    let mut k = Value::Number(0f64);
    while is_truthy(&bin_op(&k, "Lt", &Value::Number(1000f64))) {
        s = bin_op(&s, "Add", &Value::Str("a".to_string()));
        k = bin_op(&k, "Add", &Value::Number(1f64));
    }
    println!("{}", [to_string(&Value::Str("Result: String length =".to_string())), to_string(&Value::Null)].join(" "));
    println!("{}", [to_string(&Value::Str("Test 6: Nested Loops".to_string()))].join(" "));
    let mut sum_val = Value::Number(0f64);
    let mut x = Value::Number(0f64);
    while is_truthy(&bin_op(&x, "Lt", &Value::Number(100f64))) {
        let mut y = Value::Number(0f64);
        while is_truthy(&bin_op(&y, "Lt", &Value::Number(100f64))) {
            sum_val = bin_op(&sum_val, "Add", &Value::Number(1f64));
            y = bin_op(&y, "Add", &Value::Number(1f64));
        }
        x = bin_op(&x, "Add", &Value::Number(1f64));
    }
    println!("{}", [to_string(&Value::Str("Result: Total iterations =".to_string())), to_string(&sum_val)].join(" "));
    println!("{}", [to_string(&Value::Str("================================".to_string()))].join(" "));
    println!("{}", [to_string(&Value::Str("Speed Test Complete!".to_string()))].join(" "));
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