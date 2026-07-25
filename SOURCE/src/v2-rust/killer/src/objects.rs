use crate::value::{Value, ObjectInstance};
use crate::error::VmError;
use crate::bytecode::Program;
use std::collections::HashMap;

/// Handles object creation, method dispatch, and property access
pub struct ObjectManager;

impl ObjectManager {
    /// Create a new object instance of a given class
    pub fn create_instance(
        class_name: &str,
        class_info: &(Option<String>, Vec<(String, Vec<String>, Vec<crate::ast::Stmt>)>),
        constructor_args: Vec<Value>,
    ) -> Result<ObjectInstance, VmError> {
        let mut instance = ObjectInstance {
            class_name: class_name.to_string(),
            fields: HashMap::new(),
        };

        // Extract methods from class_info
        let (_parent, methods) = class_info;
        
        // Look for constructor (init method)
        if let Some((_method_name, params, body)) = methods.iter().find(|(name, _, _)| name == "init") {
            // Lightweight constructor: extract simple field assignments
            let mut param_bindings: HashMap<String, Value> = HashMap::new();
            for (idx, p) in params.iter().enumerate() {
                let value = constructor_args.get(idx).cloned().unwrap_or(Value::Null);
                param_bindings.insert(p.clone(), value);
            }

            for stmt in body {
                if let crate::ast::Stmt::IndexAssign { object, index, value } = stmt {
                    if object != "this" {
                        continue;
                    }

                    let field_name = match index {
                        crate::ast::Expr::String(s) => s.clone(),
                        crate::ast::Expr::Identifier(s) => s.clone(),
                        _ => continue,
                    };

                    let assigned = match value {
                        crate::ast::Expr::Identifier(name) => {
                            param_bindings.get(name).cloned().unwrap_or(Value::Null)
                        }
                        crate::ast::Expr::String(s) => Value::Str(s.clone()),
                        crate::ast::Expr::Number(n) => Value::Number(*n),
                        crate::ast::Expr::Bool(b) => Value::Bool(*b),
                        crate::ast::Expr::Null => Value::Null,
                        _ => continue,
                    };

                    instance.fields.insert(field_name, assigned);
                }
            }
        }

        Ok(instance)
    }

    /// Dispatch a method call with proper inheritance chain walking
    pub fn dispatch_method(
        object: &Value,
        method_name: &str,
        args: Vec<Value>,
        program: &Program,
    ) -> Result<Option<(usize, Value)>, VmError> {
        match object {
            Value::Object(obj_inst) => {
                // Walk inheritance chain
                let mut current_class = obj_inst.class_name.clone();
                let mut visited = std::collections::HashSet::new();

                loop {
                    if visited.contains(&current_class) {
                        return Err(VmError::runtime_error(
                            "Circular inheritance detected".to_string(),
                        ));
                    }
                    visited.insert(current_class.clone());

                    // Try to find method in current class
                    let method_key = (current_class.clone(), method_name.to_string());
                    if let Some(&bytecode_start) = program.method_bytecode.get(&method_key) {
                        return Ok(Some((bytecode_start, object.clone())));
                    }

                    // Check parent class
                    if let Some((parent, _)) = program.classes.get(&current_class) {
                        if let Some(parent_name) = parent {
                            current_class = parent_name.clone();
                            continue;
                        }
                    }

                    // No parent or method not found
                    // Fall back to field access for zero-arg calls
                    if args.is_empty() {
                        if obj_inst.fields.contains_key(method_name) {
                            return Ok(None);
                        }
                    }

                    return Err(VmError::runtime_error(format!(
                        "Method {} not found in class {} or its parents",
                        method_name, current_class
                    )));
                }
            }
            _ => Ok(None),
        }
    }

    /// Call method on string value
    pub fn call_string_method(
        s: &str,
        method_name: &str,
        args: &[Value],
    ) -> Result<Value, VmError> {
        match method_name {
            "length" => {
                if !args.is_empty() {
                    return Err(VmError::runtime_error(
                        "string.length takes no arguments".to_string(),
                    ));
                }
                Ok(Value::Number(s.chars().count() as f64))
            }
            "upper" => {
                if !args.is_empty() {
                    return Err(VmError::runtime_error(
                        "upper() expects no arguments in method form".to_string(),
                    ));
                }
                Ok(Value::Str(s.to_uppercase()))
            }
            "lower" => {
                if !args.is_empty() {
                    return Err(VmError::runtime_error(
                        "lower() expects no arguments in method form".to_string(),
                    ));
                }
                Ok(Value::Str(s.to_lowercase()))
            }
            "startsWith" => {
                if args.len() != 1 {
                    return Err(VmError::runtime_error("startsWith() expects 1 argument".to_string()));
                }
                let prefix = match &args[0] {
                    Value::Str(v) => v,
                    _ => return Err(VmError::runtime_error(
                        "startsWith() expects string argument".to_string(),
                    )),
                };
                Ok(Value::Bool(s.starts_with(prefix.as_str())))
            }
            "endsWith" => {
                if args.len() != 1 {
                    return Err(VmError::runtime_error("endsWith() expects 1 argument".to_string()));
                }
                let suffix = match &args[0] {
                    Value::Str(v) => v,
                    _ => return Err(VmError::runtime_error(
                        "endsWith() expects string argument".to_string(),
                    )),
                };
                Ok(Value::Bool(s.ends_with(suffix.as_str())))
            }
            "repeat" => {
                if args.len() != 1 {
                    return Err(VmError::runtime_error("repeat() expects 1 argument".to_string()));
                }
                let times = match &args[0] {
                    Value::Number(n) => (*n as usize).max(0),
                    _ => return Err(VmError::runtime_error(
                        "repeat() expects numeric argument".to_string(),
                    )),
                };
                Ok(Value::Str(s.repeat(times)))
            }
            "charAt" => {
                if args.len() != 1 {
                    return Err(VmError::runtime_error("charAt() expects 1 argument".to_string()));
                }
                let idx = match &args[0] {
                    Value::Number(n) => *n as usize,
                    _ => {
                        return Err(VmError::runtime_error(
                            "charAt() index must be a number".to_string(),
                        ))
                    }
                };
                Ok(Value::Str(s.chars().nth(idx).map(|c| c.to_string()).unwrap_or_default()))
            }
            "substring" => {
                if args.len() != 2 {
                    return Err(VmError::runtime_error("substring() expects 2 arguments".to_string()));
                }
                let start = match &args[0] {
                    Value::Number(n) => *n as usize,
                    _ => 0,
                };
                let end = match &args[1] {
                    Value::Number(n) => *n as usize,
                    _ => s.len(),
                };
                let len = s.chars().count();
                let from = start.min(len);
                let to = end.min(len);
                let (begin, finish) = if from <= to { (from, to) } else { (to, from) };
                Ok(Value::Str(s.chars().skip(begin).take(finish - begin).collect()))
            }
            "replace" => {
                if args.len() != 2 {
                    return Err(VmError::runtime_error("replace() expects 2 arguments".to_string()));
                }
                let old = match &args[0] {
                    Value::Str(v) => v,
                    _ => {
                        return Err(VmError::runtime_error(
                            "replace() old value must be string".to_string(),
                        ))
                    }
                };
                let new = match &args[1] {
                    Value::Str(v) => v,
                    _ => {
                        return Err(VmError::runtime_error(
                            "replace() new value must be string".to_string(),
                        ))
                    }
                };
                Ok(Value::Str(s.replace(old.as_str(), new.as_str())))
            }
            "split" => {
                if args.len() != 1 {
                    return Err(VmError::runtime_error("split() expects 1 argument".to_string()));
                }
                let sep = match &args[0] {
                    Value::Str(v) => v,
                    _ => {
                        return Err(VmError::runtime_error(
                            "split() separator must be string".to_string(),
                        ))
                    }
                };
                Ok(Value::from(
                    s.split(sep.as_str())
                        .map(|part| Value::Str(part.to_string()))
                        .collect::<Vec<_>>(),
                ))
            }
            _ => Err(VmError::runtime_error(format!(
                "Cannot call method {} on string",
                method_name
            ))),
        }
    }

    /// Call method on array value
    pub fn call_array_method(
        arr: &[Value],
        method_name: &str,
        args: &[Value],
    ) -> Result<Value, VmError> {
        match method_name {
            "length" => {
                if !args.is_empty() {
                    return Err(VmError::runtime_error("length() expects no arguments".to_string()));
                }
                Ok(Value::Number(arr.len() as f64))
            }
            "push" => {
                let mut updated = arr.to_vec();
                for arg in args {
                    updated.push(arg.clone());
                }
                Ok(Value::from(updated))
            }
            "pop" => {
                if !args.is_empty() {
                    return Err(VmError::runtime_error("pop() expects no arguments".to_string()));
                }
                let mut updated = arr.to_vec();
                let popped = updated.pop().unwrap_or(Value::Null);
                Ok(popped)
            }
            "join" => {
                if args.len() != 1 {
                    return Err(VmError::runtime_error(
                        "join() expects 1 separator argument".to_string(),
                    ));
                }
                let sep = match &args[0] {
                    Value::Str(v) => v,
                    _ => {
                        return Err(VmError::runtime_error("join() separator must be string".to_string()))
                    }
                };
                let parts: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
                Ok(Value::Str(parts.join(sep.as_str())))
            }
            "includes" => {
                if args.len() != 1 {
                    return Err(VmError::runtime_error("includes() expects 1 argument".to_string()));
                }
                Ok(Value::Bool(arr.contains(&args[0])))
            }
            "concat" => {
                if args.len() != 1 {
                    return Err(VmError::runtime_error("concat() expects 1 array argument".to_string()));
                }
                let rhs = match &args[0] {
                    Value::Array(v) => v,
                    _ => return Err(VmError::runtime_error(
                        "concat() expects array argument".to_string(),
                    )),
                };
                let mut out = arr.to_vec();
                out.extend(rhs.to_vec());
                Ok(Value::from(out))
            }
            "indexOf" => {
                if args.len() != 1 {
                    return Err(VmError::runtime_error("indexOf() expects 1 argument".to_string()));
                }
                let mut idx = -1.0;
                for (i, item) in arr.iter().enumerate() {
                    if item == &args[0] {
                        idx = i as f64;
                        break;
                    }
                }
                Ok(Value::Number(idx))
            }
            "sort" => {
                if !args.is_empty() {
                    return Err(VmError::runtime_error("sort() expects no arguments".to_string()));
                }
                let mut out = arr.to_vec();
                out.sort_by(|a, b| format!("{}", a).cmp(&format!("{}", b)));
                Ok(Value::from(out))
            }
            "reverse" => {
                if !args.is_empty() {
                    return Err(VmError::runtime_error("reverse() expects no arguments".to_string()));
                }
                let mut out = arr.to_vec();
                out.reverse();
                Ok(Value::from(out))
            }
            _ => Err(VmError::runtime_error(format!(
                "Cannot call method {} on array",
                method_name
            ))),
        }
    }

    /// Call method on dict/object property access
    pub fn call_dict_method(
        dict: &std::collections::HashMap<String, Value>,
        method_name: &str,
        args: &[Value],
    ) -> Result<Value, VmError> {
        match method_name {
            "length" => {
                if !args.is_empty() {
                    return Err(VmError::runtime_error("length() expects no arguments".to_string()));
                }
                Ok(Value::Number(dict.len() as f64))
            }
            "keys" => {
                if !args.is_empty() {
                    return Err(VmError::runtime_error("keys() expects no arguments".to_string()));
                }
                let keys: Vec<Value> = dict
                    .keys()
                    .map(|k| Value::Str(k.clone()))
                    .collect();
                Ok(Value::from(keys))
            }
            "values" => {
                if !args.is_empty() {
                    return Err(VmError::runtime_error("values() expects no arguments".to_string()));
                }
                let values: Vec<Value> = dict.values().cloned().collect();
                Ok(Value::from(values))
            }
            "entries" => {
                if !args.is_empty() {
                    return Err(VmError::runtime_error("entries() expects no arguments".to_string()));
                }
                let mut entries = Vec::new();
                for (k, v) in dict.iter() {
                    entries.push(Value::from(vec![Value::Str(k.clone()), v.clone()]));
                }
                Ok(Value::from(entries))
            }
            _ => Err(VmError::runtime_error(format!(
                "Property or method {} not found on object",
                method_name
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_simple_instance() {
        let class_name = "TestClass";
        let class_info = (None, vec![]);
        let instance = ObjectManager::create_instance(class_name, &class_info, vec![]);
        
        assert!(instance.is_ok());
        let obj = instance.unwrap();
        assert_eq!(obj.class_name, "TestClass");
        assert_eq!(obj.fields.len(), 0);
    }

    #[test]
    fn test_string_method_upper() {
        let result = ObjectManager::call_string_method("hello", "upper", &[]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Str("HELLO".to_string()));
    }

    #[test]
    fn test_string_method_length() {
        let result = ObjectManager::call_string_method("hello", "length", &[]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Number(5.0));
    }

    #[test]
    fn test_array_method_length() {
        let arr = vec![Value::Number(1.0), Value::Number(2.0), Value::Number(3.0)];
        let result = ObjectManager::call_array_method(&arr, "length", &[]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Number(3.0));
    }

    #[test]
    fn test_array_method_includes() {
        let arr = vec![Value::Number(1.0), Value::Number(2.0), Value::Number(3.0)];
        let result = ObjectManager::call_array_method(&arr, "includes", &[Value::Number(2.0)]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Bool(true));
    }
}
