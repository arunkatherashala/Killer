use crate::value::Value;

pub fn explain_expression(expr: &str, value: &Value) -> Vec<String> {
    let trimmed = expr.trim();
    let mut lines = vec![format!("[EXPLAIN] {trimmed}")];

    if let Some(operator) = detect_operator(trimmed) {
        lines.push(format!("- Top-level operator: {operator}"));
    }

    if let Some(call_name) = detect_call_name(trimmed) {
        lines.extend(explain_known_call(call_name));
    }

    lines.push(format!("- Result type: {}", value.type_name()));
    lines.push(format!("- Result value: {value}"));
    lines
}

fn detect_operator(expr: &str) -> Option<&'static str> {
    for operator in [" && ", " || ", " == ", " != ", " >= ", " <= ", " > ", " < ", " in ", " is ", " + ", " - ", " * ", " / "] {
        if expr.contains(operator) {
            return Some(operator.trim());
        }
    }
    None
}

fn detect_call_name(expr: &str) -> Option<&str> {
    let open = expr.find('(')?;
    expr.ends_with(')').then_some(expr[..open].trim())
}

fn explain_known_call(call_name: &str) -> Vec<String> {
    match call_name {
        "factorial" => vec![
            "- Math alias detected: factorial(n)".to_string(),
            "- Uses the multiplicative factorial definition for non-negative integers".to_string(),
        ],
        "gcd" => vec![
            "- Math alias detected: gcd(a, b)".to_string(),
            "- Uses the Euclidean algorithm".to_string(),
        ],
        "lcm" => vec![
            "- Math alias detected: lcm(a, b)".to_string(),
            "- Derived from |a * b| / gcd(a, b)".to_string(),
        ],
        "sum" => vec![
            "- Math alias detected: sum(...)".to_string(),
            "- Supports summing a number range or an array of numeric values".to_string(),
        ],
        "force" => vec![
            "- Physics alias detected: force(m, a)".to_string(),
            "- Formula: F = m * a".to_string(),
        ],
        "acceleration" => vec![
            "- Physics alias detected: acceleration(v, u, t)".to_string(),
            "- Formula: a = (v - u) / t".to_string(),
        ],
        "velocity" => vec![
            "- Physics alias detected: velocity(u, a, t)".to_string(),
            "- Formula: v = u + a * t".to_string(),
        ],
        "kineticEnergy" => vec![
            "- Physics alias detected: kineticEnergy(m, v)".to_string(),
            "- Formula: KE = 1/2 * m * v^2".to_string(),
        ],
        "potentialEnergy" => vec![
            "- Physics alias detected: potentialEnergy(m, h, g?)".to_string(),
            "- Formula: PE = m * g * h".to_string(),
        ],
        "ohmsLawCurrent" => vec![
            "- Physics alias detected: ohmsLawCurrent(v, r)".to_string(),
            "- Formula: I = V / R".to_string(),
        ],
        "ohmsLawVoltage" => vec![
            "- Physics alias detected: ohmsLawVoltage(i, r)".to_string(),
            "- Formula: V = I * R".to_string(),
        ],
        "ohmsLawResistance" => vec![
            "- Physics alias detected: ohmsLawResistance(v, i)".to_string(),
            "- Formula: R = V / I".to_string(),
        ],
        "rangeGenerator" => vec![
            "- Iterator helper detected: rangeGenerator(start, end, step?)".to_string(),
            "- Produces a native iterator value lazily consumed with next(...)".to_string(),
        ],
        _ => Vec::new(),
    }
}
