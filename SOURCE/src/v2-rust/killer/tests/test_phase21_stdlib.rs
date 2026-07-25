#![cfg(feature = "legacy-killer-rcore-tests")]
// Phase 21 Standard Library Tests
// File: _TOOLS/killer_rcore/tests/test_phase21_stdlib.rs
// Purpose: Comprehensive validation of 220+ stdlib functions
// Status: COMPLETE TEST FRAMEWORK

use killer_rcore::stdlib_builder::{StdlibBuilder, StdlibFunction};

// ================================================================
// STDLIB BUILDER TESTS
// ================================================================

#[test]
fn test_stdlib_instantiation() {
    let builder = StdlibBuilder::new();
    assert!(!builder.count() == 0);
    println!("✓ Stdlib builder initialized with {} functions", builder.count());
}

#[test]
fn test_stdlib_total_count() {
    let builder = StdlibBuilder::new();
    // Expected: 80 math + 60 string + 50 collections + 10 io + 4 time + 4 type + 3 concurrency = 211+
    let count = builder.count();
    println!("✓ Total functions: {}", count);
    assert!(count > 200, "Expected 200+ functions, got {}", count);
}

#[test]
fn test_stdlib_math_count() {
    let builder = StdlibBuilder::new();
    let math_count = builder.count_by_category("math");
    println!("✓ Math functions: {}", math_count);
    assert!(math_count >= 80, "Expected 80+ math functions");
}

#[test]
fn test_stdlib_string_count() {
    let builder = StdlibBuilder::new();
    let string_count = builder.count_by_category("string");
    println!("✓ String functions: {}", string_count);
    assert!(string_count >= 60, "Expected 60+ string functions");
}

#[test]
fn test_stdlib_collections_count() {
    let builder = StdlibBuilder::new();
    let collections_count = builder.count_by_category("collections");
    println!("✓ Collections functions: {}", collections_count);
    assert!(collections_count >= 50, "Expected 50+ collections functions");
}

// ================================================================
// MATH LIBRARY TESTS
// ================================================================

#[test]
fn test_math_trigonometric_functions() {
    let builder = StdlibBuilder::new();
    let trig_funcs = vec!["sin", "cos", "tan", "asin", "acos", "atan", "sinh", "cosh", "tanh", "atan2"];
    
    for func_name in trig_funcs {
        let func = builder.get_function(func_name);
        assert!(func.is_some(), "Missing trigonometric function: {}", func_name);
        assert_eq!(func.unwrap().category, "math");
        println!("✓ Trigonometric: {}", func_name);
    }
}

#[test]
fn test_math_exponential_functions() {
    let builder = StdlibBuilder::new();
    let exp_funcs = vec!["exp", "log", "log10", "log2", "pow", "sqrt", "cbrt", "hypot", "expm1", "logp1"];
    
    for func_name in exp_funcs {
        let func = builder.get_function(func_name);
        assert!(func.is_some(), "Missing exponential function: {}", func_name);
        println!("✓ Exponential: {}", func_name);
    }
}

#[test]
fn test_math_rounding_functions() {
    let builder = StdlibBuilder::new();
    let round_funcs = vec!["abs", "fabs", "ceil", "floor", "round", "trunc", "fmod", "remainder"];
    
    for func_name in round_funcs {
        let func = builder.get_function(func_name);
        assert!(func.is_some(), "Missing rounding function: {}", func_name);
        println!("✓ Rounding: {}", func_name);
    }
}

#[test]
fn test_math_special_functions() {
    let builder = StdlibBuilder::new();
    let special_funcs = vec!["factorial", "combinations", "permutations", "is_prime", "gcd", "lcm"];
    
    for func_name in special_funcs {
        let func = builder.get_function(func_name);
        assert!(func.is_some(), "Missing special function: {}", func_name);
        println!("✓ Special: {}", func_name);
    }
}

#[test]
fn test_math_random_functions() {
    let builder = StdlibBuilder::new();
    let random_funcs = vec!["random", "random_int", "random_range", "random_float", "randn", "seed"];
    
    for func_name in random_funcs {
        let func = builder.get_function(func_name);
        assert!(func.is_some(), "Missing random function: {}", func_name);
        println!("✓ Random: {}", func_name);
    }
}

#[test]
fn test_math_statistical_functions() {
    let builder = StdlibBuilder::new();
    let stat_funcs = vec!["mean", "median", "stddev", "variance", "sum", "product", "min_of", "max_of", "percentile"];
    
    for func_name in stat_funcs {
        let func = builder.get_function(func_name);
        assert!(func.is_some(), "Missing statistical function: {}", func_name);
        println!("✓ Statistical: {}", func_name);
    }
}

// ================================================================
// STRING LIBRARY TESTS
// ================================================================

#[test]
fn test_string_basic_operations() {
    let builder = StdlibBuilder::new();
    let basic_funcs = vec!["length", "concat", "substring", "index_of", "contains", "replace", "split", "join"];
    
    for func_name in basic_funcs {
        let func = builder.get_function(func_name);
        assert!(func.is_some(), "Missing string function: {}", func_name);
        println!("✓ String basic: {}", func_name);
    }
}

#[test]
fn test_string_case_functions() {
    let builder = StdlibBuilder::new();
    let case_funcs = vec!["to_upper", "to_lower", "to_title_case", "capitalize", "camel_case", "snake_case"];
    
    for func_name in case_funcs {
        let func = builder.get_function(func_name);
        assert!(func.is_some(), "Missing case function: {}", func_name);
        println!("✓ String case: {}", func_name);
    }
}

#[test]
fn test_string_testing_functions() {
    let builder = StdlibBuilder::new();
    let test_funcs = vec!["is_empty", "is_blank", "is_numeric", "is_alpha", "is_alphanumeric"];
    
    for func_name in test_funcs {
        let func = builder.get_function(func_name);
        assert!(func.is_some(), "Missing test function: {}", func_name);
        println!("✓ String test: {}", func_name);
    }
}

#[test]
fn test_string_parsing_functions() {
    let builder = StdlibBuilder::new();
    let parse_funcs = vec!["parse_int", "parse_float", "parse_bool", "to_string", "format"];
    
    for func_name in parse_funcs {
        let func = builder.get_function(func_name);
        assert!(func.is_some(), "Missing parsing function: {}", func_name);
        println!("✓ String parse: {}", func_name);
    }
}

#[test]
fn test_string_pattern_functions() {
    let builder = StdlibBuilder::new();
    let pattern_funcs = vec!["match", "matches", "split_pattern", "replace_pattern", "count_pattern"];
    
    for func_name in pattern_funcs {
        let func = builder.get_function(func_name);
        assert!(func.is_some(), "Missing pattern function: {}", func_name);
        println!("✓ String pattern: {}", func_name);
    }
}

// ================================================================
// COLLECTIONS LIBRARY TESTS
// ================================================================

#[test]
fn test_collections_list_functions() {
    let builder = StdlibBuilder::new();
    let list_funcs = vec!["list_push", "list_pop", "list_get", "list_set", "list_sort", "list_map", "list_filter"];
    
    for func_name in list_funcs {
        let func = builder.get_function(func_name);
        assert!(func.is_some(), "Missing list function: {}", func_name);
        println!("✓ Collections list: {}", func_name);
    }
}

#[test]
fn test_collections_map_functions() {
    let builder = StdlibBuilder::new();
    let map_funcs = vec!["map_put", "map_get", "map_remove", "map_keys", "map_values"];
    
    for func_name in map_funcs {
        let func = builder.get_function(func_name);
        assert!(func.is_some(), "Missing map function: {}", func_name);
        println!("✓ Collections map: {}", func_name);
    }
}

#[test]
fn test_collections_set_functions() {
    let builder = StdlibBuilder::new();
    let set_funcs = vec!["set_add", "set_remove", "set_contains", "set_union", "set_intersection"];
    
    for func_name in set_funcs {
        let func = builder.get_function(func_name);
        assert!(func.is_some(), "Missing set function: {}", func_name);
        println!("✓ Collections set: {}", func_name);
    }
}

// ================================================================
// I/O LIBRARY TESTS
// ================================================================

#[test]
fn test_io_functions() {
    let builder = StdlibBuilder::new();
    let io_funcs = vec!["print", "println", "read_line", "read_file", "write_file", "file_exists"];
    
    for func_name in io_funcs {
        let func = builder.get_function(func_name);
        assert!(func.is_some(), "Missing I/O function: {}", func_name);
        println!("✓ I/O: {}", func_name);
    }
}

// ================================================================
// TIME LIBRARY TESTS
// ================================================================

#[test]
fn test_time_functions() {
    let builder = StdlibBuilder::new();
    let time_funcs = vec!["now_ms", "now_s", "sleep", "sleep_seconds"];
    
    for func_name in time_funcs {
        let func = builder.get_function(func_name);
        assert!(func.is_some(), "Missing time function: {}", func_name);
        println!("✓ Time: {}", func_name);
    }
}

// ================================================================
// TYPE LIBRARY TESTS
// ================================================================

#[test]
fn test_type_functions() {
    let builder = StdlibBuilder::new();
    let type_funcs = vec!["type_of", "is_int", "is_float", "is_string"];
    
    for func_name in type_funcs {
        let func = builder.get_function(func_name);
        assert!(func.is_some(), "Missing type function: {}", func_name);
        println!("✓ Type: {}", func_name);
    }
}

// ================================================================
// FUNCTION METADATA TESTS
// ================================================================

#[test]
fn test_function_metadata_completeness() {
    let builder = StdlibBuilder::new();
    let funcs = builder.list_all();
    
    for func in funcs {
        assert!(!func.name.is_empty(), "Function has empty name");
        assert!(!func.category.is_empty(), "Function {} has empty category", func.name);
        assert!(!func.signature.is_empty(), "Function {} has empty signature", func.name);
        assert!(!func.description.is_empty(), "Function {} has empty description", func.name);
        assert!(!func.complexity.is_empty(), "Function {} has empty complexity", func.name);
    }
    println!("✓ All {} functions have complete metadata", funcs.len());
}

#[test]
fn test_function_complexity_notation() {
    let builder = StdlibBuilder::new();
    let valid_complexities = vec!["O(1)", "O(n)", "O(n log n)", "O(n^2)", "O(sqrt(n))", "O(log n)", "O(min(n,m))", "O(n+m)", "O(n*m)"];
    
    for func in builder.list_all() {
        let complexity = &func.complexity;
        let is_valid = valid_complexities.iter().any(|v| complexity.contains(v)) 
                    || complexity.contains("amortized")
                    || complexity.contains("hash")
                    || complexity.contains("euclidean");
        assert!(is_valid, "Invalid complexity notation for {}: {}", func.name, complexity);
    }
    println!("✓ All functions have valid complexity notation");
}

// ================================================================
// CATEGORY ORGANIZATION TESTS
// ================================================================

#[test]
fn test_categories_coverage() {
    let builder = StdlibBuilder::new();
    let categories = vec!["math", "string", "collections", "io", "time", "type", "concurrency"];
    
    for category in categories {
        let count = builder.count_by_category(category);
        assert!(count > 0, "Category {} has no functions", category);
        println!("✓ Category {}: {} functions", category, count);
    }
}

#[test]
fn test_list_functions_by_category() {
    let builder = StdlibBuilder::new();
    
    let math_funcs = builder.list_functions("math");
    println!("✓ Math functions ({}): {:?}", math_funcs.len(), 
        math_funcs.iter().map(|f| f.name.as_str()).collect::<Vec<_>>()[..std::cmp::min(5, math_funcs.len())].to_vec());
    
    let string_funcs = builder.list_functions("string");
    println!("✓ String functions ({}): {:?}", string_funcs.len(),
        string_funcs.iter().map(|f| f.name.as_str()).collect::<Vec<_>>()[..std::cmp::min(5, string_funcs.len())].to_vec());
    
    let collection_funcs = builder.list_functions("collections");
    println!("✓ Collections functions ({}): {:?}", collection_funcs.len(),
        collection_funcs.iter().map(|f| f.name.as_str()).collect::<Vec<_>>()[..std::cmp::min(5, collection_funcs.len())].to_vec());
}

// ================================================================
// GENERATION TESTS
// ================================================================

#[test]
fn test_killer_module_generation() {
    let builder = StdlibBuilder::new();
    let killer_module = builder.generate_killer_module();
    
    assert!(killer_module.contains("AUTO-GENERATED"));
    assert!(killer_module.contains("Standard Library v1.0"));
    assert!(killer_module.contains("Total Functions"));
    println!("✓ Generated Killer module:\n{}", killer_module);
}

// ================================================================
// INTEGRATION TESTS
// ================================================================

#[test]
fn test_stdlib_math_sqrt_metadata() {
    let builder = StdlibBuilder::new();
    let sqrt = builder.get_function("sqrt").unwrap();
    
    assert_eq!(sqrt.name, "sqrt");
    assert_eq!(sqrt.category, "math");
    assert!(sqrt.signature.contains("Float"));
    assert_eq!(sqrt.complexity, "O(1)");
    println!("✓ sqrt metadata: {:?}", sqrt);
}

#[test]
fn test_stdlib_string_split_metadata() {
    let builder = StdlibBuilder::new();
    let split = builder.get_function("split").unwrap();
    
    assert_eq!(split.name, "split");
    assert_eq!(split.category, "string");
    assert!(split.signature.contains("String"));
    assert!(split.signature.contains("List"));
    println!("✓ split metadata: {:?}", split);
}

#[test]
fn test_stdlib_list_map_metadata() {
    let builder = StdlibBuilder::new();
    let list_map = builder.get_function("list_map").unwrap();
    
    assert_eq!(list_map.name, "list_map");
    assert_eq!(list_map.category, "collections");
    assert!(list_map.signature.contains("Fn(T)->U"));
    println!("✓ list_map metadata: {:?}", list_map);
}

// ================================================================
// SUMMARY TEST
// ================================================================

#[test]
fn test_phase21_stdlib_summary() {
    let builder = StdlibBuilder::new();
    
    println!("\n+========================================+");
    println!("|     PHASE 21 STDLIB SUMMARY            |");
    println!("+========================================+");
    println!("Total Functions: {}", builder.count());
    println!("Math:            {} functions", builder.count_by_category("math"));
    println!("String:          {} functions", builder.count_by_category("string"));
    println!("Collections:     {} functions", builder.count_by_category("collections"));
    println!("I/O:             {} functions", builder.count_by_category("io"));
    println!("Time:            {} functions", builder.count_by_category("time"));
    println!("Type:            {} functions", builder.count_by_category("type"));
    println!("Concurrency:     {} functions", builder.count_by_category("concurrency"));
    println!("+========================================+\n");
    
    assert!(builder.count() >= 210);
}
