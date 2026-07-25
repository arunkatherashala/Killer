// src/v2-rust/killer_vm/src/json_csv.rs
// JSON and CSV serialization/deserialization module for Killer language
// Provides pretty-printing, CSV parsing, and data format conversion

use std::collections::HashMap;

/// JSON pretty printer (indented output)
pub fn json_pretty(json_str: &str, indent: usize) -> Result<String, String> {
    let mut result = String::new();
    let mut current_indent = 0;
    let indent_str = " ".repeat(indent);
    let chars: Vec<char> = json_str.chars().collect();
    let mut i = 0;
    let mut in_string = false;
    let mut escape_next = false;

    while i < chars.len() {
        let ch = chars[i];

        // Handle string escaping
        if escape_next {
            result.push(ch);
            escape_next = false;
            i += 1;
            continue;
        }

        if ch == '\\' && in_string {
            result.push(ch);
            escape_next = true;
            i += 1;
            continue;
        }

        if ch == '"' {
            in_string = !in_string;
            result.push(ch);
            i += 1;
            continue;
        }

        if in_string {
            result.push(ch);
            i += 1;
            continue;
        }

        match ch {
            '{' | '[' => {
                result.push(ch);
                current_indent += 1;
                result.push('\n');
                result.push_str(&indent_str.repeat(current_indent));
            }
            '}' | ']' => {
                current_indent = if current_indent > 0 {
                    current_indent - 1
                } else {
                    0
                };
                result.push('\n');
                result.push_str(&indent_str.repeat(current_indent));
                result.push(ch);
            }
            ',' => {
                result.push(ch);
                result.push('\n');
                result.push_str(&indent_str.repeat(current_indent));
            }
            ':' => {
                result.push(ch);
                result.push(' ');
            }
            ' ' | '\n' | '\t' | '\r' => {
                // Skip whitespace in compact JSON
            }
            _ => result.push(ch),
        }

        i += 1;
    }

    Ok(result)
}

/// CSV row representation
#[derive(Clone, Debug)]
pub struct CsvRow {
    pub fields: Vec<String>,
}

impl CsvRow {
    pub fn new() -> Self {
        CsvRow {
            fields: Vec::new(),
        }
    }

    pub fn add_field(&mut self, field: &str) {
        self.fields.push(field.to_string());
    }

    pub fn to_csv_line(&self, delimiter: char) -> String {
        self.fields
            .iter()
            .map(|field| {
                if field.contains(',') || field.contains('"') || field.contains('\n') {
                    format!("\"{}\"", field.replace("\"", "\"\""))
                } else {
                    field.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(&delimiter.to_string())
    }
}

/// Parse CSV string to list of dicts
/// First line is treated as headers
pub fn parse_csv(csv_str: &str, delimiter: char) -> Result<Vec<HashMap<String, String>>, String> {
    let lines: Vec<&str> = csv_str.lines().collect();
    if lines.is_empty() {
        return Ok(Vec::new());
    }

    // Parse header line
    let headers = parse_csv_line(lines[0], delimiter)?;
    if headers.is_empty() {
        return Err("CSV has no header row".to_string());
    }

    // Parse data rows
    let mut result = Vec::new();
    for line in &lines[1..] {
        if line.trim().is_empty() {
            continue;
        }

        let fields = parse_csv_line(line, delimiter)?;
        let mut row = HashMap::new();

        for (i, header) in headers.iter().enumerate() {
            let value = if i < fields.len() {
                fields[i].clone()
            } else {
                String::new()
            };
            row.insert(header.clone(), value);
        }

        result.push(row);
    }

    Ok(result)
}

/// Parse a single CSV line
fn parse_csv_line(line: &str, delimiter: char) -> Result<Vec<String>, String> {
    let mut fields = Vec::new();
    let mut current_field = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes {
                    // Check for escaped quote
                    if chars.peek() == Some(&'"') {
                        chars.next();
                        current_field.push('"');
                    } else {
                        in_quotes = false;
                    }
                } else {
                    in_quotes = true;
                }
            }
            c if c == delimiter && !in_quotes => {
                fields.push(current_field.trim().to_string());
                current_field = String::new();
            }
            _ => current_field.push(ch),
        }
    }

    // Add last field
    fields.push(current_field.trim().to_string());

    Ok(fields)
}

/// Convert list of dicts to CSV string
pub fn to_csv(
    rows: &[HashMap<String, String>],
    delimiter: char,
) -> Result<String, String> {
    if rows.is_empty() {
        return Ok(String::new());
    }

    // Get headers from first row
    let mut headers: Vec<String> = rows[0].keys().cloned().collect();
    headers.sort(); // For consistent ordering

    let mut csv = String::new();
    
    // Write header line
    csv.push_str(&headers.join(&delimiter.to_string()));
    csv.push('\n');

    // Write data rows
    for row in rows {
        let mut line_fields = Vec::new();
        for header in &headers {
            let value = row.get(header).cloned().unwrap_or_default();
            if value.contains(',') || value.contains('"') || value.contains('\n') {
                line_fields.push(format!("\"{}\"", value.replace("\"", "\"\"")));
            } else {
                line_fields.push(value);
            }
        }
        csv.push_str(&line_fields.join(&delimiter.to_string()));
        csv.push('\n');
    }

    Ok(csv)
}

/// Convert dict to YAML-like format
pub fn to_yaml(dict: &HashMap<String, String>, indent: usize) -> String {
    let mut result = String::new();
    let indent_str = " ".repeat(indent);

    for (key, value) in dict {
        result.push_str(&indent_str);
        result.push_str(key);
        result.push_str(": ");
        
        // Quote strings with special characters
        if value.contains('\n') || value.contains(':') {
            result.push('|');
            result.push('\n');
            for line in value.lines() {
                result.push_str(&indent_str);
                result.push_str("  ");
                result.push_str(line);
                result.push('\n');
            }
        } else {
            result.push_str(value);
            result.push('\n');
        }
    }

    result
}

/// Convert YAML-like string to dict (simple parser)
pub fn parse_yaml(yaml_str: &str) -> Result<HashMap<String, String>, String> {
    let mut result = HashMap::new();
    
    for line in yaml_str.lines() {
        let trimmed = line.trim();
        
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        
        if let Some(colon_pos) = trimmed.find(':') {
            let key = trimmed[..colon_pos].trim().to_string();
            let value = trimmed[colon_pos + 1..].trim().to_string();
            
            if !key.is_empty() {
                result.insert(key, value);
            }
        }
    }
    
    Ok(result)
}

/// Validate JSON structure
pub fn is_valid_json(json_str: &str) -> bool {
    let mut brace_count = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for ch in json_str.chars() {
        if escape_next {
            escape_next = false;
            continue;
        }

        if ch == '\\' && in_string {
            escape_next = true;
            continue;
        }

        if ch == '"' {
            in_string = !in_string;
            continue;
        }

        if !in_string {
            match ch {
                '{' => brace_count += 1,
                '}' => {
                    brace_count -= 1;
                    if brace_count < 0 {
                        return false;
                    }
                }
                _ => {}
            }
        }
    }

    brace_count == 0 && !in_string
}

/// Get JSON value by path (simple implementation for dot notation)
/// Example: "user.name" or "data.items.0"
pub fn json_get_path(json_dict: &HashMap<String, String>, path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('.').collect();
    
    if parts.is_empty() {
        return None;
    }
    
    // For now, simple implementation - just get first level
    json_dict.get(parts[0]).cloned()
}

/// Merge two dicts
pub fn merge_dicts(
    dict1: &HashMap<String, String>,
    dict2: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut result = dict1.clone();
    for (key, value) in dict2 {
        result.insert(key.clone(), value.clone());
    }
    result
}

/// Filter CSV rows by condition (simple implementation)
pub fn filter_csv_rows(
    rows: &[HashMap<String, String>],
    field: &str,
    value: &str,
) -> Vec<HashMap<String, String>> {
    rows.iter()
        .filter(|row| row.get(field).map_or(false, |v| v.contains(value)))
        .cloned()
        .collect()
}

/// Sort CSV rows by field
pub fn sort_csv_rows(
    rows: &mut [HashMap<String, String>],
    field: &str,
) {
    rows.sort_by(|a, b| {
        let a_val = a.get(field).cloned().unwrap_or_default();
        let b_val = b.get(field).cloned().unwrap_or_default();
        a_val.cmp(&b_val)
    });
}
