#![cfg(feature = "legacy-killer-rcore-tests")]
// Phase 20.1 FFI Tests
// File: _TOOLS/killer_rcore/tests/test_phase20_ffi.rs

use killer_rcore::ffi::*;

#[test]
fn test_cvalue_conversion_i32() {
    let val = CValue::from_killer("123", CType::I32).expect("Should parse i32");
    assert!(matches!(val, CValue::I32(123)));
    assert_eq!(val.to_killer(), "123");
}

#[test]
fn test_cvalue_conversion_f64() {
    let val = CValue::from_killer("3.14159", CType::F64).expect("Should parse f64");
    match val {
        CValue::F64(x) => assert!((x - 3.14159).abs() < 0.00001),
        _ => panic!("Expected F64"),
    }
}

#[test]
fn test_cvalue_conversion_bool() {
    let true_val = CValue::from_killer("true", CType::Bool).expect("Should parse bool");
    assert!(matches!(true_val, CValue::Bool(true)));
    
    let false_val = CValue::from_killer("false", CType::Bool).expect("Should parse bool");
    assert!(matches!(false_val, CValue::Bool(false)));
}

#[test]
fn test_cvalue_conversion_string() {
    let val = CValue::from_killer("hello", CType::CStr).expect("Should parse string");
    match val {
        CValue::CStr(s) => assert_eq!(s, "hello"),
        _ => panic!("Expected CStr"),
    }
}

#[test]
fn test_cvalue_invalid_i32() {
    let result = CValue::from_killer("not_a_number", CType::I32);
    assert!(result.is_err());
}

#[test]
fn test_cfunction_registration() {
    let mut bindings = FFIBindings::new();
    
    let func = CFunction {
        name: "test_func".to_string(),
        return_type: CType::I32,
        param_types: vec![CType::I32, CType::I32],
    };
    
    bindings.register(func);
    assert!(bindings.get("test_func").is_some());
}

#[test]
fn test_math_bindings() {
    let bindings = create_math_bindings();
    let funcs = bindings.list_all();
    
    assert!(funcs.contains(&"sqrt".to_string()));
    assert!(funcs.contains(&"sin".to_string()));
    assert!(funcs.contains(&"cos".to_string()));
    assert!(funcs.contains(&"pow".to_string()));
    assert!(funcs.contains(&"log".to_string()));
}

#[test]
fn test_string_bindings() {
    let bindings = create_string_bindings();
    let funcs = bindings.list_all();
    
    assert!(funcs.contains(&"strlen".to_string()));
}

#[test]
fn test_utils_bindings() {
    let bindings = create_utils_bindings();
    let funcs = bindings.list_all();
    
    assert!(funcs.contains(&"abs".to_string()));
}

#[test]
fn test_ffi_abs() {
    // Mock library for testing
    let mock_lib = std::sync::Arc::new(std::sync::Mutex::new(()));
    
    // Test abs with negative number
    let args = vec![CValue::I32(-42)];
    let lib = unsafe {
        libloading::Library::new("").ok()
    };
    
    // We'll test with built-in functions
    let result = call_c_function(
        &CLibrary {
            library: lib.unwrap(),
            path: "mock".to_string(),
        },
        "abs",
        args,
        CType::I32,
    );
    
    assert!(matches!(result, Ok(CValue::I32(42))));
}

#[test]
fn test_ffi_sqrt() {
    let args = vec![CValue::F64(4.0)];
    let lib = unsafe { libloading::Library::new("").ok() };
    
    let result = call_c_function(
        &CLibrary {
            library: lib.unwrap(),
            path: "mock".to_string(),
        },
        "sqrt",
        args,
        CType::F64,
    );
    
    match result {
        Ok(CValue::F64(x)) => assert!((x - 2.0).abs() < 0.0001),
        _ => panic!("Expected F64 with sqrt(4) = 2"),
    }
}

#[test]
fn test_ffi_pow() {
    let args = vec![CValue::F64(2.0), CValue::F64(8.0)];
    let lib = unsafe { libloading::Library::new("").ok() };
    
    let result = call_c_function(
        &CLibrary {
            library: lib.unwrap(),
            path: "mock".to_string(),
        },
        "pow",
        args,
        CType::F64,
    );
    
    match result {
        Ok(CValue::F64(x)) => assert!((x - 256.0).abs() < 0.0001), // 2^8 = 256
        _ => panic!("Expected F64 with pow(2,8) = 256"),
    }
}

#[test]
fn test_ffi_strlen() {
    let args = vec![CValue::CStr("hello world".to_string())];
    let lib = unsafe { libloading::Library::new("").ok() };
    
    let result = call_c_function(
        &CLibrary {
            library: lib.unwrap(),
            path: "mock".to_string(),
        },
        "strlen",
        args,
        CType::U64,
    );
    
    assert!(matches!(result, Ok(CValue::U64(11))));
}

#[test]
fn test_ffi_sin() {
    let args = vec![CValue::F64(0.0)];
    let lib = unsafe { libloading::Library::new("").ok() };
    
    let result = call_c_function(
        &CLibrary {
            library: lib.unwrap(),
            path: "mock".to_string(),
        },
        "sin",
        args,
        CType::F64,
    );
    
    assert!(matches!(result, Ok(CValue::F64(x)) if (x - 0.0).abs() < 0.0001));
}

#[test]
fn test_ffi_cos() {
    let args = vec![CValue::F64(0.0)];
    let lib = unsafe { libloading::Library::new("").ok() };
    
    let result = call_c_function(
        &CLibrary {
            library: lib.unwrap(),
            path: "mock".to_string(),
        },
        "cos",
        args,
        CType::F64,
    );
    
    assert!(matches!(result, Ok(CValue::F64(x)) if (x - 1.0).abs() < 0.0001));
}

#[test]
fn test_ffi_log() {
    let args = vec![CValue::F64(std::f64::consts::E)];
    let lib = unsafe { libloading::Library::new("").ok() };
    
    let result = call_c_function(
        &CLibrary {
            library: lib.unwrap(),
            path: "mock".to_string(),
        },
        "log",
        args,
        CType::F64,
    );
    
    assert!(matches!(result, Ok(CValue::F64(x)) if (x - 1.0).abs() < 0.0001));
}

#[test]
fn test_ffi_error_symbol_not_found() {
    let lib = unsafe { libloading::Library::new("").ok() };
    let result = call_c_function(
        &CLibrary {
            library: lib.unwrap(),
            path: "mock".to_string(),
        },
        "nonexistent",
        vec![],
        CType::Void,
    );
    
    assert!(matches!(result, Err(FFIError::SymbolNotFound(_))));
}

#[test]
fn test_ffi_error_invalid_args() {
    let lib = unsafe { libloading::Library::new("").ok() };
    let result = call_c_function(
        &CLibrary {
            library: lib.unwrap(),
            path: "mock".to_string(),
        },
        "strlen",
        vec![], // strlen needs 1 arg
        CType::U64,
    );
    
    assert!(matches!(result, Err(FFIError::InvalidArgument(_))));
}

#[test]
fn test_ffi_error_type_mismatch() {
    let lib = unsafe { libloading::Library::new("").ok() };
    let result = call_c_function(
        &CLibrary {
            library: lib.unwrap(),
            path: "mock".to_string(),
        },
        "strlen",
        vec![CValue::I32(123)], // strlen expects string
        CType::U64,
    );
    
    assert!(matches!(result, Err(FFIError::TypeMismatch(_))));
}
