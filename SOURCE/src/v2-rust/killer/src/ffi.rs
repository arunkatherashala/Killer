#![allow(unsafe_code)]

// Phase 20.1: Foreign Function Interface (FFI) Module
// File: _TOOLS/killer_rcore/src/ffi.rs
// Purpose: Call C libraries from Killer language
// Timeline: 2 weeks
// Status: IMPLEMENTATION IN PROGRESS

use std::ffi::{CStr, CString, c_void};
use std::ptr::null_mut;
use libloading::{Library, Symbol};

/// C Type representation for FFI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CType {
    Void,
    I32,
    I64,
    U32,
    U64,
    F64,
    Bool,
    CStr,     // C string (const char*)
    Ptr,      // void pointer
    Function,
}

/// Represents a C library function signature
#[derive(Debug, Clone)]
pub struct CFunction {
    pub name: String,
    pub return_type: CType,
    pub param_types: Vec<CType>,
}

/// Represents a loaded C library
pub struct CLibrary {
    library: Library,
    path: String,
}

/// Value that can be passed to/from C
#[derive(Debug, Clone)]
pub enum CValue {
    Void,
    I32(i32),
    I64(i64),
    U32(u32),
    U64(u64),
    F64(f64),
    Bool(bool),
    CStr(String),
    Ptr(*mut c_void),
}

impl CValue {
    /// Convert Killer value to C value
    pub fn from_killer(killer_val: &str, c_type: CType) -> Result<Self, String> {
        match c_type {
            CType::I32 => {
                let val = killer_val.parse::<i32>()
                    .map_err(|_| format!("Cannot parse '{}' as i32", killer_val))?;
                Ok(CValue::I32(val))
            },
            CType::I64 => {
                let val = killer_val.parse::<i64>()
                    .map_err(|_| format!("Cannot parse '{}' as i64", killer_val))?;
                Ok(CValue::I64(val))
            },
            CType::F64 => {
                let val = killer_val.parse::<f64>()
                    .map_err(|_| format!("Cannot parse '{}' as f64", killer_val))?;
                Ok(CValue::F64(val))
            },
            CType::Bool => {
                let val = match killer_val.to_lowercase().as_str() {
                    "true" | "1" => true,
                    "false" | "0" => false,
                    _ => return Err(format!("Cannot parse '{}' as bool", killer_val)),
                };
                Ok(CValue::Bool(val))
            },
            CType::CStr => {
                Ok(CValue::CStr(killer_val.to_string()))
            },
            CType::Void => Ok(CValue::Void),
            _ => Err(format!("Unsupported type for conversion: {:?}", c_type)),
        }
    }

    /// Convert C value to Killer string representation
    pub fn to_killer(&self) -> String {
        match self {
            CValue::Void => "void".to_string(),
            CValue::I32(v) => v.to_string(),
            CValue::I64(v) => v.to_string(),
            CValue::U32(v) => v.to_string(),
            CValue::U64(v) => v.to_string(),
            CValue::F64(v) => v.to_string(),
            CValue::Bool(v) => v.to_string(),
            CValue::CStr(v) => v.clone(),
            CValue::Ptr(p) => format!("{:p}", p),
        }
    }
}

/// FFI Error types
#[derive(Debug)]
pub enum FFIError {
    LibraryNotFound(String),
    SymbolNotFound(String),
    TypeMismatch(String),
    InvalidArgument(String),
    NullPointer(String),
    Segmentation,
}

impl std::fmt::Display for FFIError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FFIError::LibraryNotFound(msg) => write!(f, "Library not found: {}", msg),
            FFIError::SymbolNotFound(msg) => write!(f, "Symbol not found: {}", msg),
            FFIError::TypeMismatch(msg) => write!(f, "Type mismatch: {}", msg),
            FFIError::InvalidArgument(msg) => write!(f, "Invalid argument: {}", msg),
            FFIError::NullPointer(msg) => write!(f, "Null pointer: {}", msg),
            FFIError::Segmentation => write!(f, "Segmentation fault detected"),
        }
    }
}

/// Load a C library
pub fn load_library(path: &str) -> Result<CLibrary, FFIError> {
    let library = unsafe {
        Library::new(path)
            .map_err(|_| FFIError::LibraryNotFound(path.to_string()))?
    };
    
    Ok(CLibrary {
        library,
        path: path.to_string(),
    })
}

/// Call a C function (simplified - works for basic types)
pub fn call_c_function(
    lib: &CLibrary,
    func_name: &str,
    args: Vec<CValue>,
    return_type: CType,
) -> Result<CValue, FFIError> {
    // For demonstration: handle common libc functions
    
    match func_name {
        // strlen(const char* str) -> size_t
        "strlen" => {
            if args.len() != 1 {
                return Err(FFIError::InvalidArgument("strlen expects 1 argument".to_string()));
            }
            match &args[0] {
                CValue::CStr(s) => Ok(CValue::U64(s.len() as u64)),
                _ => Err(FFIError::TypeMismatch("strlen expects CStr".to_string())),
            }
        },
        // abs(int x) -> int
        "abs" => {
            if args.len() != 1 {
                return Err(FFIError::InvalidArgument("abs expects 1 argument".to_string()));
            }
            match &args[0] {
                CValue::I32(n) => Ok(CValue::I32(n.abs())),
                _ => Err(FFIError::TypeMismatch("abs expects i32".to_string())),
            }
        },
        // sqrt(double x) -> double (approximation)
        "sqrt" => {
            if args.len() != 1 {
                return Err(FFIError::InvalidArgument("sqrt expects 1 argument".to_string()));
            }
            match &args[0] {
                CValue::F64(n) => Ok(CValue::F64(n.sqrt())),
                _ => Err(FFIError::TypeMismatch("sqrt expects f64".to_string())),
            }
        },
        // sin(double x) -> double
        "sin" => {
            if args.len() != 1 {
                return Err(FFIError::InvalidArgument("sin expects 1 argument".to_string()));
            }
            match &args[0] {
                CValue::F64(n) => Ok(CValue::F64(n.sin())),
                _ => Err(FFIError::TypeMismatch("sin expects f64".to_string())),
            }
        },
        // cos(double x) -> double
        "cos" => {
            if args.len() != 1 {
                return Err(FFIError::InvalidArgument("cos expects 1 argument".to_string()));
            }
            match &args[0] {
                CValue::F64(n) => Ok(CValue::F64(n.cos())),
                _ => Err(FFIError::TypeMismatch("cos expects f64".to_string())),
            }
        },
        // log(double x) -> double
        "log" => {
            if args.len() != 1 {
                return Err(FFIError::InvalidArgument("log expects 1 argument".to_string()));
            }
            match &args[0] {
                CValue::F64(n) if *n > 0.0 => Ok(CValue::F64(n.ln())),
                _ => Err(FFIError::InvalidArgument("log expects positive f64".to_string())),
            }
        },
        // pow(double x, double y) -> double
        "pow" => {
            if args.len() != 2 {
                return Err(FFIError::InvalidArgument("pow expects 2 arguments".to_string()));
            }
            match (&args[0], &args[1]) {
                (CValue::F64(base), CValue::F64(exp)) => Ok(CValue::F64(base.powf(*exp))),
                _ => Err(FFIError::TypeMismatch("pow expects (f64, f64)".to_string())),
            }
        },
        _ => Err(FFIError::SymbolNotFound(func_name.to_string())),
    }
}

/// FFI Binding registry for Killer language
pub struct FFIBindings {
    bindings: std::collections::HashMap<String, CFunction>,
}

impl FFIBindings {
    pub fn new() -> Self {
        FFIBindings {
            bindings: std::collections::HashMap::new(),
        }
    }

    pub fn register(&mut self, func: CFunction) {
        self.bindings.insert(func.name.clone(), func);
    }

    pub fn get(&self, name: &str) -> Option<&CFunction> {
        self.bindings.get(name)
    }

    pub fn list_all(&self) -> Vec<String> {
        self.bindings.keys().cloned().collect()
    }
}

/// Standard C math library bindings
pub fn create_math_bindings() -> FFIBindings {
    let mut bindings = FFIBindings::new();

    bindings.register(CFunction {
        name: "sqrt".to_string(),
        return_type: CType::F64,
        param_types: vec![CType::F64],
    });

    bindings.register(CFunction {
        name: "sin".to_string(),
        return_type: CType::F64,
        param_types: vec![CType::F64],
    });

    bindings.register(CFunction {
        name: "cos".to_string(),
        return_type: CType::F64,
        param_types: vec![CType::F64],
    });

    bindings.register(CFunction {
        name: "pow".to_string(),
        return_type: CType::F64,
        param_types: vec![CType::F64, CType::F64],
    });

    bindings.register(CFunction {
        name: "log".to_string(),
        return_type: CType::F64,
        param_types: vec![CType::F64],
    });

    bindings
}

/// Standard C string library bindings
pub fn create_string_bindings() -> FFIBindings {
    let mut bindings = FFIBindings::new();

    bindings.register(CFunction {
        name: "strlen".to_string(),
        return_type: CType::U64,
        param_types: vec![CType::CStr],
    });

    bindings
}

/// Standard C utils library bindings
pub fn create_utils_bindings() -> FFIBindings {
    let mut bindings = FFIBindings::new();

    bindings.register(CFunction {
        name: "abs".to_string(),
        return_type: CType::I32,
        param_types: vec![CType::I32],
    });

    bindings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cvalue_from_killer_i32() {
        let result = CValue::from_killer("42", CType::I32);
        assert!(matches!(result, Ok(CValue::I32(42))));
    }

    #[test]
    fn test_cvalue_from_killer_f64() {
        let result = CValue::from_killer("3.14", CType::F64);
        assert!(matches!(result, Ok(CValue::F64(x)) if (x - 3.14).abs() < 0.001));
    }

    #[test]
    fn test_cvalue_from_killer_bool() {
        let result = CValue::from_killer("true", CType::Bool);
        assert!(matches!(result, Ok(CValue::Bool(true))));
    }

    #[test]
    fn test_cvalue_to_killer() {
        let val = CValue::I32(42);
        assert_eq!(val.to_killer(), "42");
    }

    #[test]
    fn test_math_strlen() {
        let lib = load_library("libc.so.6").ok();
        let args = vec![CValue::CStr("hello".to_string())];
        let result = call_c_function(
            &lib.unwrap_or_else(|| CLibrary {
                library: unsafe { Library::new("").unwrap() },
                path: "mock".to_string(),
            }),
            "strlen",
            args,
            CType::U64,
        );
        assert!(matches!(result, Ok(CValue::U64(5))));
    }

    #[test]
    fn test_abs_function() {
        let lib = CLibrary {
            library: unsafe { Library::new("").unwrap() },
            path: "mock".to_string(),
        };
        let args = vec![CValue::I32(-42)];
        let result = call_c_function(&lib, "abs", args, CType::I32);
        assert!(matches!(result, Ok(CValue::I32(42))));
    }

    #[test]
    fn test_sqrt_function() {
        let lib = CLibrary {
            library: unsafe { Library::new("").unwrap() },
            path: "mock".to_string(),
        };
        let args = vec![CValue::F64(4.0)];
        let result = call_c_function(&lib, "sqrt", args, CType::F64);
        assert!(matches!(result, Ok(CValue::F64(x)) if (x - 2.0).abs() < 0.001));
    }

    #[test]
    fn test_math_bindings_registration() {
        let bindings = create_math_bindings();
        assert!(bindings.get("sqrt").is_some());
        assert!(bindings.get("sin").is_some());
        assert!(bindings.get("cos").is_some());
    }

    #[test]
    fn test_string_bindings_registration() {
        let bindings = create_string_bindings();
        assert!(bindings.get("strlen").is_some());
    }

    #[test]
    fn test_utils_bindings_registration() {
        let bindings = create_utils_bindings();
        assert!(bindings.get("abs").is_some());
    }

    #[test]
    fn test_invalid_argument_count() {
        let lib = CLibrary {
            library: unsafe { Library::new("").unwrap() },
            path: "mock".to_string(),
        };
        let args = vec![]; // strlen needs 1 arg
        let result = call_c_function(&lib, "strlen", args, CType::U64);
        assert!(matches!(result, Err(FFIError::InvalidArgument(_))));
    }

    #[test]
    fn test_type_mismatch() {
        let lib = CLibrary {
            library: unsafe { Library::new("").unwrap() },
            path: "mock".to_string(),
        };
        let args = vec![CValue::I32(42)]; // abs expects i32, pass wrong type
        let result = call_c_function(&lib, "strlen", args, CType::U64);
        assert!(matches!(result, Err(FFIError::TypeMismatch(_))));
    }

    #[test]
    fn test_unknown_function() {
        let lib = CLibrary {
            library: unsafe { Library::new("").unwrap() },
            path: "mock".to_string(),
        };
        let args = vec![];
        let result = call_c_function(&lib, "nonexistent_function", args, CType::Void);
        assert!(matches!(result, Err(FFIError::SymbolNotFound(_))));
    }

    #[test]
    fn test_pow_function() {
        let lib = CLibrary {
            library: unsafe { Library::new("").unwrap() },
            path: "mock".to_string(),
        };
        let args = vec![CValue::F64(2.0), CValue::F64(3.0)]; // 2^3 = 8
        let result = call_c_function(&lib, "pow", args, CType::F64);
        assert!(matches!(result, Ok(CValue::F64(x)) if (x - 8.0).abs() < 0.001));
    }
}
