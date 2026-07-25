// ================================================================
// TYPE SYSTEM SOLVER - Phase 21.5
// Type introspection, reflection, type operations
// ================================================================

use std::any::{TypeId, type_name};

/// Type System Operations Solver
pub struct TypeSolver;

/// Simple type information struct
#[derive(Clone, Debug)]
pub struct TypeInfo {
    pub name: String,
    pub size: usize,
    pub alignment: usize,
}

impl TypeSolver {
    // ================================================================
    // TYPE INTROSPECTION (1-20)
    // ================================================================

    /// Problem 1: Get type name of T
    pub fn type_name_of<T: ?Sized>() -> &'static str {
        type_name::<T>()
    }

    /// Problem 2: Get type ID
    pub fn type_id<T: 'static + ?Sized>() -> TypeId {
        TypeId::of::<T>()
    }

    /// Problem 3: Size of type in bytes
    pub fn size_of<T>() -> usize {
        std::mem::size_of::<T>()
    }

    /// Problem 4: Alignment of type
    pub fn align_of<T>() -> usize {
        std::mem::align_of::<T>()
    }

    /// Problem 5: Size of value
    pub fn size_of_value<T: ?Sized>(_val: &T) -> usize {
        std::mem::size_of_val(_val)
    }

    /// Problem 6: Check if types equal
    pub fn types_equal<T: 'static, U: 'static>() -> bool {
        TypeId::of::<T>() == TypeId::of::<U>()
    }

    /// Problem 7: Type is integer
    pub fn is_integer_type(name: &str) -> bool {
        matches!(name, "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16" | "u32" | "u64" | "u128" | "isize" | "usize")
    }

    /// Problem 8: Type is floating point
    pub fn is_float_type(name: &str) -> bool {
        matches!(name, "f32" | "f64")
    }

    /// Problem 9: Type is boolean
    pub fn is_bool_type(name: &str) -> bool {
        name == "bool"
    }

    /// Problem 10: Type is pointer
    pub fn is_pointer_type(name: &str) -> bool {
        name.contains("*") || name.contains("&")
    }

    // ================================================================
    // TYPE CLASSIFICATION (11-25)
    // ================================================================

    /// Problem 11: Get numeric type width in bits
    pub fn numeric_type_bits(type_name: &str) -> usize {
        match type_name {
            "i8" | "u8" => 8,
            "i16" | "u16" => 16,
            "i32" | "u32" | "f32" => 32,
            "i64" | "u64" | "f64" => 64,
            "i128" | "u128" => 128,
            _ => 0,
        }
    }

    /// Problem 12: Check if type is signed
    pub fn is_signed_type(name: &str) -> bool {
        matches!(name, "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "f32" | "f64")
    }

    /// Problem 13: Check if type is unsigned
    pub fn is_unsigned_type(name: &str) -> bool {
        matches!(name, "u8" | "u16" | "u32" | "u64" | "u128" | "usize")
    }

    /// Problem 14: Check if type is numeric
    pub fn is_numeric_type(name: &str) -> bool {
        Self::is_integer_type(name) || Self::is_float_type(name)
    }

    /// Problem 15: Check if type is tuple
    pub fn is_tuple_type(name: &str) -> bool {
        name.starts_with('(') && name.ends_with(')')
    }

    /// Problem 16: Check if type is array
    pub fn is_array_type(name: &str) -> bool {
        name.starts_with('[') && name.ends_with(']')
    }

    /// Problem 17: Check if type is vector
    pub fn is_vector_type(name: &str) -> bool {
        name.contains("Vec<") || name.contains("vec!")
    }

    /// Problem 18: Check if type is string
    pub fn is_string_type(name: &str) -> bool {
        matches!(name, "str" | "String") || name.contains("String")
    }

    /// Problem 19: Check if type is map
    pub fn is_map_type(name: &str) -> bool {
        name.contains("HashMap<") || name.contains("BTreeMap<")
    }

    /// Problem 20: Generic type parameter count
    pub fn generic_param_count(name: &str) -> usize {
        name.matches(',').count() + if name.contains('<') { 1 } else { 0 }
    }

    // ================================================================
    // TYPE CONSTRAINTS & VALIDATION (21-35)
    // ================================================================

    /// Problem 21: Check if value fits in u8
    pub fn fits_in_u8(val: i64) -> bool {
        val >= 0 && val <= 255
    }

    /// Problem 22: Check if value fits in u16
    pub fn fits_in_u16(val: i64) -> bool {
        val >= 0 && val <= 65535
    }

    /// Problem 23: Check if value fits in u32
    pub fn fits_in_u32(val: i64) -> bool {
        val >= 0 && val <= 4294967295
    }

    /// Problem 24: Check if value fits in i8
    pub fn fits_in_i8(val: i64) -> bool {
        val >= -128 && val <= 127
    }

    /// Problem 25: Check if value fits in i16
    pub fn fits_in_i16(val: i64) -> bool {
        val >= -32768 && val <= 32767
    }

    /// Problem 26: Check if value fits in i32
    pub fn fits_in_i32(val: i64) -> bool {
        val >= -2147483648 && val <= 2147483647
    }

    /// Problem 27: Safe type conversion check
    pub fn can_convert(from: &str, to: &str) -> bool {
        // Simplified: numeric types can often convert
        if Self::is_numeric_type(from) && Self::is_numeric_type(to) {
            return true;
        }
        from == to
    }

    /// Problem 28: Type casting safety level (0 = unsafe to 3 = safe)
    pub fn cast_safety_level(from: &str, to: &str) -> u8 {
        if from == to { return 3; } // Same type = safest
        if Self::is_numeric_type(from) && Self::is_numeric_type(to) { return 2; } // Numeric conversion
        1 // Potentially unsafe
    }

    /// Problem 29: Numeric type max value
    pub fn numeric_max_value(type_name: &str) -> i128 {
        match type_name {
            "i8" => 127,
            "i16" => 32767,
            "i32" => 2147483647,
            "i64" => 9223372036854775807,
            "u8" => 255,
            "u16" => 65535,
            "u32" => 4294967295,
            "u64" => 18446744073709551615i128,
            _ => 0,
        }
    }

    /// Problem 30: Numeric type min value
    pub fn numeric_min_value(type_name: &str) -> i128 {
        match type_name {
            "i8" => -128,
            "i16" => -32768,
            "i32" => -2147483648,
            "i64" => -9223372036854775808,
            "u8" | "u16" | "u32" | "u64" => 0,
            _ => 0,
        }
    }

    // ================================================================
    // TYPE OPERATIONS (31-40)
    // ================================================================

    /// Problem 31: Get type representation
    pub fn create_type_info<T: 'static>() -> TypeInfo {
        TypeInfo {
            name: Self::type_name_of::<T>().to_string(),
            size: Self::size_of::<T>(),
            alignment: Self::align_of::<T>(),
        }
    }

    /// Problem 32: Scalar type rank (for promotion)
    pub fn numeric_rank(type_name: &str) -> u8 {
        match type_name {
            "i8" | "u8" => 0,
            "i16" | "u16" => 1,
            "i32" | "u32" | "f32" => 2,
            "i64" | "u64" | "f64" => 3,
            "i128" | "u128" => 4,
            _ => 255,
        }
    }

    /// Problem 33: Should promote for binary op
    pub fn should_promote(left: &str, right: &str) -> (String, String) {
        let left_rank = Self::numeric_rank(left);
        let right_rank = Self::numeric_rank(right);
        
        if left_rank == right_rank {
            (left.to_string(), right.to_string())
        } else if left_rank > right_rank {
            (left.to_string(), left.to_string())
        } else {
            (right.to_string(), right.to_string())
        }
    }

    /// Problem 34: Display type constraint
    pub fn type_constraint_string<T: 'static>() -> String {
        format!("T: {}", Self::type_name_of::<T>())
    }

    /// Problem 35: Common supertype (simplified)
    pub fn common_supertype(t1: &str, t2: &str) -> String {
        if t1 == t2 {
            t1.to_string()
        } else if Self::numeric_rank(t1) > Self::numeric_rank(t2) {
            t1.to_string()
        } else {
            t2.to_string()
        }
    }

    /// Problem 36: Type family (what category)
    pub fn type_family(name: &str) -> &'static str {
        if Self::is_integer_type(name) { "integer" }
        else if Self::is_float_type(name) { "float" }
        else if Self::is_bool_type(name) { "bool" }
        else if Self::is_string_type(name) { "string" }
        else if Self::is_vector_type(name) { "collection" }
        else { "other" }
    }

    /// Problem 37: Zero value for type
    pub fn zero_value_str(type_name: &str) -> &'static str {
        match type_name {
            "bool" => "false",
            t if Self::is_numeric_type(t) => "0",
            t if Self::is_string_type(t) => "\"\"",
            _ => "null",
        }
    }

    /// Problem 38: Default display for type
    pub fn default_display_format(type_name: &str) -> &'static str {
        match type_name {
            "f32" | "f64" => "{:.2}",
            "i32" | "i64" | "u32" | "u64" => "{}",
            _ => "{}",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_info() {
        let info = TypeSolver::create_type_info::<i32>();
        assert_eq!(info.size, 4);
        assert!(info.alignment <= 4);
    }

    #[test]
    fn test_type_classification() {
        assert!(TypeSolver::is_integer_type("i32"));
        assert!(TypeSolver::is_float_type("f64"));
        assert!(TypeSolver::is_signed_type("i32"));
        assert!(!TypeSolver::is_unsigned_type("i32"));
    }

    #[test]
    fn test_value_fit() {
        assert!(TypeSolver::fits_in_u8(255));
        assert!(!TypeSolver::fits_in_u8(256));
        assert!(TypeSolver::fits_in_i32(100));
    }

    #[test]
    fn test_numeric_rank() {
        assert!(TypeSolver::numeric_rank("i32") > TypeSolver::numeric_rank("i16"));
    }
}
