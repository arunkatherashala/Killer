// Phase 21: Standard Library Builder (200+ Functions)
// File: _TOOLS/killer_rcore/src/stdlib_builder.rs
// Purpose: Generate 200+ killer stdlib functions using native logic
// Timeline: Phase 21 (4 weeks)
// Status: FRAMEWORK IN PROGRESS

use std::collections::HashMap;

/// stdlib Function metadata
#[derive(Debug, Clone)]
pub struct StdlibFunction {
    pub name: String,
    pub category: String,
    pub signature: String,
    pub description: String,
    pub implementation: String,
    pub complexity: String,  // O(1), O(n), O(n log n), etc
    pub native_equivalent: Option<String>,  // C/FFI equivalent
}

/// stdlib Categories
pub enum StdlibCategory {
    Math,
    String,
    Collections,
    IO,
    Time,
    Type,
    Concurrency,
    Advanced,
}

impl StdlibCategory {
    pub fn as_string(&self) -> String {
        match self {
            StdlibCategory::Math => "math".to_string(),
            StdlibCategory::String => "string".to_string(),
            StdlibCategory::Collections => "collections".to_string(),
            StdlibCategory::IO => "io".to_string(),
            StdlibCategory::Time => "time".to_string(),
            StdlibCategory::Type => "type".to_string(),
            StdlibCategory::Concurrency => "concurrency".to_string(),
            StdlibCategory::Advanced => "advanced".to_string(),
        }
    }
}

/// stdlib Builder - generates 200+ functions
pub struct StdlibBuilder {
    functions: HashMap<String, StdlibFunction>,
}

impl StdlibBuilder {
    pub fn new() -> Self {
        let mut builder = StdlibBuilder {
            functions: HashMap::new(),
        };
        
        // Initialize with all stdlib functions
        builder.build_all();
        builder
    }

    pub fn build_all(&mut self) {
        self.build_math_library();
        self.build_string_library();
        self.build_collections_library();
        self.build_io_library();
        self.build_time_library();
        self.build_type_library();
        self.build_concurrency_library();
    }

    // ================================================================
    // MATH LIBRARY (80 functions)
    // ================================================================
    fn build_math_library(&mut self) {
        // Trigonometric (10)
        self.add_function("sin", "math", "fn sin(x: Float) -> Float", 
            "Sine function", "calls C::sin", "O(1)");
        self.add_function("cos", "math", "fn cos(x: Float) -> Float", 
            "Cosine function", "calls C::cos", "O(1)");
        self.add_function("tan", "math", "fn tan(x: Float) -> Float", 
            "Tangent function", "calls C::tan", "O(1)");
        self.add_function("asin", "math", "fn asin(x: Float) -> Float", 
            "Inverse sine", "calls C::asin", "O(1)");
        self.add_function("acos", "math", "fn acos(x: Float) -> Float", 
            "Inverse cosine", "calls C::acos", "O(1)");
        self.add_function("atan", "math", "fn atan(x: Float) -> Float", 
            "Inverse tangent", "calls C::atan", "O(1)");
        self.add_function("sinh", "math", "fn sinh(x: Float) -> Float", 
            "Hyperbolic sine", "calls C::sinh", "O(1)");
        self.add_function("cosh", "math", "fn cosh(x: Float) -> Float", 
            "Hyperbolic cosine", "calls C::cosh", "O(1)");
        self.add_function("tanh", "math", "fn tanh(x: Float) -> Float", 
            "Hyperbolic tangent", "calls C::tanh", "O(1)");
        self.add_function("atan2", "math", "fn atan2(y: Float, x: Float) -> Float", 
            "Atan2 function", "calls C::atan2", "O(1)");

        // Exponential/Logarithmic (10)
        self.add_function("exp", "math", "fn exp(x: Float) -> Float", 
            "Exponential (e^x)", "calls C::exp", "O(1)");
        self.add_function("log", "math", "fn log(x: Float) -> Float", 
            "Natural logarithm", "calls C::log", "O(1)");
        self.add_function("log10", "math", "fn log10(x: Float) -> Float", 
            "Base-10 logarithm", "calls C::log10", "O(1)");
        self.add_function("log2", "math", "fn log2(x: Float) -> Float", 
            "Base-2 logarithm", "calls C::log2", "O(1)");
        self.add_function("pow", "math", "fn pow(base: Float, exp: Float) -> Float", 
            "Power function", "calls C::pow", "O(1)");
        self.add_function("sqrt", "math", "fn sqrt(x: Float) -> Float", 
            "Square root", "calls C::sqrt", "O(1)");
        self.add_function("cbrt", "math", "fn cbrt(x: Float) -> Float", 
            "Cube root", "calls C::cbrt", "O(1)");
        self.add_function("hypot", "math", "fn hypot(x: Float, y: Float) -> Float", 
            "Hypotenuse", "calls C::hypot", "O(1)");
        self.add_function("expm1", "math", "fn expm1(x: Float) -> Float", 
            "e^x - 1 (accurate)", "calls C::expm1", "O(1)");
        self.add_function("logp1", "math", "fn log1p(x: Float) -> Float", 
            "log(1 + x) (accurate)", "calls C::log1p", "O(1)");

        // Rounding (10)
        self.add_function("abs", "math", "fn abs(x: Int) -> Int", 
            "Absolute value (integer)", "inline", "O(1)");
        self.add_function("fabs", "math", "fn fabs(x: Float) -> Float", 
            "Absolute value (float)", "calls C::fabs", "O(1)");
        self.add_function("ceil", "math", "fn ceil(x: Float) -> Float", 
            "Ceiling function", "calls C::ceil", "O(1)");
        self.add_function("floor", "math", "fn floor(x: Float) -> Float", 
            "Floor function", "calls C::floor", "O(1)");
        self.add_function("round", "math", "fn round(x: Float) -> Float", 
            "Round to nearest int", "calls C::round", "O(1)");
        self.add_function("trunc", "math", "fn trunc(x: Float) -> Float", 
            "Truncate to integer", "calls C::trunc", "O(1)");
        self.add_function("fmod", "math", "fn fmod(x: Float, y: Float) -> Float", 
            "Floating-point modulo", "calls C::fmod", "O(1)");
        self.add_function("remainder", "math", "fn remainder(x: Float, y: Float) -> Float", 
            "IEEE remainder", "calls C::remainder", "O(1)");
        self.add_function("sign", "math", "fn sign(x: Float) -> Int", 
            "Sign (-1, 0, 1)", "inline", "O(1)");
        self.add_function("copysign", "math", "fn copysign(x: Float, y: Float) -> Float", 
            "Copy sign function", "calls C::copysign", "O(1)");

        // Min/Max/Quantile (10)
        self.add_function("min", "math", "fn min(a: Float, b: Float) -> Float", 
            "Minimum value", "inline", "O(1)");
        self.add_function("max", "math", "fn max(a: Float, b: Float) -> Float", 
            "Maximum value", "inline", "O(1)");
        self.add_function("clamp", "math", "fn clamp(x: Float, min: Float, max: Float) -> Float", 
            "Clamp to range", "inline", "O(1)");
        self.add_function("gcd", "math", "fn gcd(a: Int, b: Int) -> Int", 
            "Greatest common divisor", "euclidean", "O(log n)");
        self.add_function("lcm", "math", "fn lcm(a: Int, b: Int) -> Int", 
            "Least common multiple", "inline", "O(log n)");
        self.add_function("mod", "math", "fn mod(x: Int, m: Int) -> Int", 
            "Integer modulo", "inline", "O(1)");
        self.add_function("rem", "math", "fn rem(x: Int, m: Int) -> Int", 
            "Integer remainder", "inline", "O(1)");
        self.add_function("saturating_add", "math", "fn saturating_add(a: Int, b: Int) -> Int", 
            "Add without overflow", "conditional", "O(1)");
        self.add_function("saturating_sub", "math", "fn saturating_sub(a: Int, b: Int) -> Int", 
            "Subtract without underflow", "conditional", "O(1)");
        self.add_function("saturating_mul", "math", "fn saturating_mul(a: Int, b: Int) -> Int", 
            "Multiply without overflow", "conditional", "O(1)");

        // Random/Statistical (15)
        self.add_function("random", "math", "fn random() -> Float", 
            "Random float [0, 1)", "MT19937", "O(1)");
        self.add_function("random_int", "math", "fn random_int(max: Int) -> Int", 
            "Random integer [0, max)", "MT19937", "O(1)");
        self.add_function("random_range", "math", "fn random_range(min: Int, max: Int) -> Int", 
            "Random in range", "MT19937", "O(1)");
        self.add_function("random_float", "math", "fn random_float(min: Float, max: Float) -> Float", 
            "Random float in range", "MT19937", "O(1)");
        self.add_function("randn", "math", "fn randn() -> Float", 
            "Normal distribution", "Box-Muller", "O(1)");
        self.add_function("seed", "math", "fn seed(s: Int) -> Void", 
            "Set random seed", "MT19937", "O(1)");
        self.add_function("mean", "math", "fn mean(list: List<Float>) -> Float", 
            "Calculate mean", "sum/length", "O(n)");
        self.add_function("median", "math", "fn median(list: List<Float>) -> Float", 
            "Calculate median", "sort+middle", "O(n log n)");
        self.add_function("stddev", "math", "fn stddev(list: List<Float>) -> Float", 
            "Standard deviation", "variance-sqrt", "O(n)");
        self.add_function("variance", "math", "fn variance(list: List<Float>) -> Float", 
            "Calculate variance", "mean-diff-sq", "O(n)");
        self.add_function("sum", "math", "fn sum(list: List<Float>) -> Float", 
            "Sum all elements", "fold", "O(n)");
        self.add_function("product", "math", "fn product(list: List<Float>) -> Float", 
            "Product all elements", "fold", "O(n)");
        self.add_function("min_of", "math", "fn min_of(list: List<Float>) -> Float", 
            "Minimum in list", "fold", "O(n)");
        self.add_function("max_of", "math", "fn max_of(list: List<Float>) -> Float", 
            "Maximum in list", "fold", "O(n)");
        self.add_function("percentile", "math", "fn percentile(list: List<Float>, p: Float) -> Float", 
            "Calculate percentile", "sort+interpolate", "O(n log n)");

        // Special Functions (15)
        self.add_function("erf", "math", "fn erf(x: Float) -> Float", 
            "Error function", "calls C::erf", "O(1)");
        self.add_function("erfc", "math", "fn erfc(x: Float) -> Float", 
            "Complementary error", "calls C::erfc", "O(1)");
        self.add_function("tgamma", "math", "fn tgamma(x: Float) -> Float", 
            "Gamma function", "calls C::tgamma", "O(1)");
        self.add_function("lgamma", "math", "fn lgamma(x: Float) -> Float", 
            "Log gamma function", "calls C::lgamma", "O(1)");
        self.add_function("j0", "math", "fn j0(x: Float) -> Float", 
            "Bessel J0", "calls C::j0", "O(1)");
        self.add_function("j1", "math", "fn j1(x: Float) -> Float", 
            "Bessel J1", "calls C::j1", "O(1)");
        self.add_function("y0", "math", "fn y0(x: Float) -> Float", 
            "Bessel Y0", "calls C::y0", "O(1)");
        self.add_function("y1", "math", "fn y1(x: Float) -> Float", 
            "Bessel Y1", "calls C::y1", "O(1)");
        self.add_function("factorial", "math", "fn factorial(n: Int) -> Long", 
            "Factorial", "loop", "O(n)");
        self.add_function("combinations", "math", "fn combinations(n: Int, k: Int) -> Long", 
            "Combinations nCk", "numer/denom", "O(min(k,n-k))");
        self.add_function("permutations", "math", "fn permutations(n: Int, k: Int) -> Long", 
            "Permutations nPk", "factorial-based", "O(k)");
        self.add_function("is_prime", "math", "fn is_prime(n: Int) -> Bool", 
            "Check primality", "trial-div", "O(sqrt(n))");
        self.add_function("gcd_extended", "math", "fn gcd_extended(a: Int, b: Int) -> (Int, Int, Int)", 
            "Extended GCD", "euclidean", "O(log n)");
        self.add_function("modular_pow", "math", "fn modular_pow(base: Int, exp: Int, mod: Int) -> Int", 
            "Modular exponentiation", "binary-exp", "O(log exp)");
        self.add_function("modular_inverse", "math", "fn modular_inverse(a: Int, m: Int) -> Int", 
            "Modular inverse", "extended-gcd", "O(log m)");
    }

    // ================================================================
    // STRING LIBRARY (60 functions)
    // ================================================================
    fn build_string_library(&mut self) {
        // Basic operations (20)
        self.add_function("length", "string", "fn length(s: String) -> Int", 
            "String length", "strlen FFI", "O(1)");
        self.add_function("concat", "string", "fn concat(a: String, b: String) -> String", 
            "Concatenate strings", "append", "O(n+m)");
        self.add_function("substring", "string", "fn substring(s: String, start: Int, end: Int) -> String", 
            "Extract substring", "slice", "O(n)");
        self.add_function("index_of", "string", "fn index_of(s: String, sub: String) -> Int", 
            "Find substring position", "search", "O(n*m)");
        self.add_function("last_index_of", "string", "fn last_index_of(s: String, sub: String) -> Int", 
            "Find last position", "reverse-search", "O(n*m)");
        self.add_function("starts_with", "string", "fn starts_with(s: String, prefix: String) -> Bool", 
            "Check prefix", "compare", "O(m)");
        self.add_function("ends_with", "string", "fn ends_with(s: String, suffix: String) -> Bool", 
            "Check suffix", "compare", "O(m)");
        self.add_function("contains", "string", "fn contains(s: String, sub: String) -> Bool", 
            "Check contains", "search", "O(n*m)");
        self.add_function("replace", "string", "fn replace(s: String, old: String, new: String) -> String", 
            "Replace all occurrences", "search+concat", "O(n*m)");
        self.add_function("replace_first", "string", "fn replace_first(s: String, old: String, new: String) -> String", 
            "Replace first", "search+concat", "O(n*m)");
        self.add_function("split", "string", "fn split(s: String, delim: String) -> List<String>", 
            "Split by delimiter", "search+slice", "O(n)");
        self.add_function("split_limit", "string", "fn split_limit(s: String, delim: String, limit: Int) -> List<String>", 
            "Split with limit", "search+slice", "O(n)");
        self.add_function("join", "string", "fn join(list: List<String>, sep: String) -> String", 
            "Join with separator", "concat", "O(n*m)");
        self.add_function("trim", "string", "fn trim(s: String) -> String", 
            "Remove whitespace", "slice", "O(n)");
        self.add_function("trim_left", "string", "fn trim_left(s: String) -> String", 
            "Remove left whitespace", "slice", "O(n)");
        self.add_function("trim_right", "string", "fn trim_right(s: String) -> String", 
            "Remove right whitespace", "slice", "O(n)");
        self.add_function("to_upper", "string", "fn to_upper(s: String) -> String", 
            "Convert to uppercase", "map-char", "O(n)");
        self.add_function("to_lower", "string", "fn to_lower(s: String) -> String", 
            "Convert to lowercase", "map-char", "O(n)");
        self.add_function("reverse", "string", "fn reverse(s: String) -> String", 
            "Reverse string", "iterate-back", "O(n)");
        self.add_function("repeat", "string", "fn repeat(s: String, count: Int) -> String", 
            "Repeat N times", "concat-loop", "O(n*count)");

        // Case operations (5)
        self.add_function("to_title_case", "string", "fn to_title_case(s: String) -> String", 
            "Title case conversion", "map-words", "O(n)");
        self.add_function("capitalize", "string", "fn capitalize(s: String) -> String", 
            "Capitalize first letter", "map-char", "O(n)");
        self.add_function("decapitalize", "string", "fn decapitalize(s: String) -> String", 
            "Lowercase first letter", "map-char", "O(n)");
        self.add_function("camel_case", "string", "fn camel_case(s: String) -> String", 
            "Convert to camelCase", "split+join", "O(n)");
        self.add_function("snake_case", "string", "fn snake_case(s: String) -> String", 
            "Convert to snake_case", "split+join", "O(n)");

        // Testing (10)
        self.add_function("is_empty", "string", "fn is_empty(s: String) -> Bool", 
            "Check if empty", "length", "O(1)");
        self.add_function("is_blank", "string", "fn is_blank(s: String) -> Bool", 
            "Check if whitespace only", "iterate", "O(n)");
        self.add_function("is_numeric", "string", "fn is_numeric(s: String) -> Bool", 
            "Check if all digits", "iterate", "O(n)");
        self.add_function("is_alpha", "string", "fn is_alpha(s: String) -> Bool", 
            "Check if all letters", "iterate", "O(n)");
        self.add_function("is_alphanumeric", "string", "fn is_alphanumeric(s: String) -> Bool", 
            "Check if alphanumeric", "iterate", "O(n)");
        self.add_function("equals_ignore_case", "string", "fn equals_ignore_case(a: String, b: String) -> Bool", 
            "Case-insensitive compare", "to_lower+compare", "O(n)");
        self.add_function("compare_to", "string", "fn compare_to(a: String, b: String) -> Int", 
            "Lexicographic compare", "compare-char", "O(min(n,m))");
        self.add_function("compare_to_ignore_case", "string", "fn compare_to_ignore_case(a: String, b: String) -> Int", 
            "Case-insensitive lex compare", "compare-lower", "O(min(n,m))");
        self.add_function("starts_with_any", "string", "fn starts_with_any(s: String, prefixes: List<String>) -> Bool", 
            "Check any prefix", "iterate+compare", "O(n*m)");
        self.add_function("ends_with_any", "string", "fn ends_with_any(s: String, suffixes: List<String>) -> Bool", 
            "Check any suffix", "iterate+compare", "O(n*m)");

        // Parsing/Formatting (15)
        self.add_function("parse_int", "string", "fn parse_int(s: String) -> Int", 
            "Parse integer", "strtol", "O(n)");
        self.add_function("parse_float", "string", "fn parse_float(s: String) -> Float", 
            "Parse floating point", "strtod", "O(n)");
        self.add_function("parse_bool", "string", "fn parse_bool(s: String) -> Bool", 
            "Parse boolean", "compare", "O(n)");
        self.add_function("to_string", "string", "fn to_string(x: Any) -> String", 
            "Convert to string", "format", "O(n)");
        self.add_function("format", "string", "fn format(template: String, args: List<String>) -> String", 
            "String formatting", "template-replace", "O(n)");
        self.add_function("pad_left", "string", "fn pad_left(s: String, width: Int, char: String) -> String", 
            "Pad left with char", "repeat+concat", "O(n)");
        self.add_function("pad_right", "string", "fn pad_right(s: String, width: Int, char: String) -> String", 
            "Pad right with char", "repeat+concat", "O(n)");
        self.add_function("center", "string", "fn center(s: String, width: Int, char: String) -> String", 
            "Center string", "pad-both", "O(n)");
        self.add_function("ljust", "string", "fn ljust(s: String, width: Int, char: String) -> String", 
            "Left justify", "pad-right", "O(n)");
        self.add_function("rjust", "string", "fn rjust(s: String, width: Int, char: String) -> String", 
            "Right justify", "pad-left", "O(n)");
        self.add_function("escape", "string", "fn escape(s: String) -> String", 
            "Escape special chars", "replace-loop", "O(n)");
        self.add_function("unescape", "string", "fn unescape(s: String) -> String", 
            "Unescape special chars", "replace-loop", "O(n)");
        self.add_function("codes", "string", "fn codes(s: String) -> List<Int>", 
            "Get character codes", "iterate", "O(n)");
        self.add_function("from_codes", "string", "fn from_codes(codes: List<Int>) -> String", 
            "Create from char codes", "iterate", "O(n)");
        self.add_function("grapheme_count", "string", "fn grapheme_count(s: String) -> Int", 
            "Count graphemes (unicode)", "iterate", "O(n)");

        // Pattern/Regex (10)
        self.add_function("match", "string", "fn match(s: String, pattern: String) -> Bool", 
            "Simple pattern match", "regex", "O(n*m)");
        self.add_function("matches", "string", "fn matches(s: String, pattern: String) -> List<String>", 
            "Find all matches", "regex", "O(n*m)");
        self.add_function("match_groups", "string", "fn match_groups(s: String, pattern: String) -> List<List<String>>", 
            "Extract groups", "regex", "O(n*m)");
        self.add_function("split_pattern", "string", "fn split_pattern(s: String, pattern: String) -> List<String>", 
            "Split by pattern", "regex", "O(n*m)");
        self.add_function("replace_pattern", "string", "fn replace_pattern(s: String, pattern: String, replacement: String) -> String", 
            "Pattern replace", "regex", "O(n*m)");
        self.add_function("is_match", "string", "fn is_match(s: String, pattern: String) -> Bool", 
            "Check pattern match", "regex", "O(n*m)");
        self.add_function("find", "string", "fn find(s: String, pattern: String) -> Int", 
            "Find pattern position", "regex", "O(n*m)");
        self.add_function("find_all", "string", "fn find_all(s: String, pattern: String) -> List<Int>", 
            "Find all positions", "regex", "O(n*m)");
        self.add_function("count_pattern", "string", "fn count_pattern(s: String, pattern: String) -> Int", 
            "Count pattern occurrences", "regex", "O(n*m)");
        self.add_function("has_whitespace", "string", "fn has_whitespace(s: String) -> Bool", 
            "Check for whitespace", "iterate", "O(n)");
    }

    // ================================================================
    // COLLECTIONS LIBRARY (50 functions)
    // ================================================================
    fn build_collections_library(&mut self) {
        // List operations (25)
        self.add_function("list_new", "collections", "fn list_new<T>() -> List<T>", 
            "Create empty list", "allocate", "O(1)");
        self.add_function("list_push", "collections", "fn list_push<T>(list: List<T>, item: T) -> Void", 
            "Add to end", "append", "O(1) amortized");
        self.add_function("list_pop", "collections", "fn list_pop<T>(list: List<T>) -> T", 
            "Remove from end", "remove", "O(1)");
        self.add_function("list_shift", "collections", "fn list_shift<T>(list: List<T>) -> T", 
            "Remove from start", "remove", "O(n)");
        self.add_function("list_unshift", "collections", "fn list_unshift<T>(list: List<T>, item: T) -> Void", 
            "Add to start", "insert", "O(n)");
        self.add_function("list_insert", "collections", "fn list_insert<T>(list: List<T>, index: Int, item: T) -> Void", 
            "Insert at index", "insert", "O(n)");
        self.add_function("list_remove", "collections", "fn list_remove<T>(list: List<T>, index: Int) -> T", 
            "Remove at index", "remove", "O(n)");
        self.add_function("list_get", "collections", "fn list_get<T>(list: List<T>, index: Int) -> T", 
            "Get at index", "index", "O(1)");
        self.add_function("list_set", "collections", "fn list_set<T>(list: List<T>, index: Int, item: T) -> Void", 
            "Set at index", "index-assign", "O(1)");
        self.add_function("list_clear", "collections", "fn list_clear<T>(list: List<T>) -> Void", 
            "Remove all items", "truncate", "O(1)");
        self.add_function("list_contains", "collections", "fn list_contains<T>(list: List<T>, item: T) -> Bool", 
            "Check membership", "search", "O(n)");
        self.add_function("list_index_of", "collections", "fn list_index_of<T>(list: List<T>, item: T) -> Int", 
            "Find item index", "search", "O(n)");
        self.add_function("list_reverse", "collections", "fn list_reverse<T>(list: List<T>) -> Void", 
            "Reverse in place", "swap-loop", "O(n)");
        self.add_function("list_sort", "collections", "fn list_sort<T>(list: List<T>) -> Void", 
            "Sort in place", "quicksort", "O(n log n)");
        self.add_function("list_sorted", "collections", "fn list_sorted<T>(list: List<T>) -> List<T>", 
            "Return sorted copy", "quicksort", "O(n log n)");
        self.add_function("list_shuffle", "collections", "fn list_shuffle<T>(list: List<T>) -> Void", 
            "Shuffle in place", "fisher-yates", "O(n)");
        self.add_function("list_map", "collections", "fn list_map<T,U>(list: List<T>, fn: Fn(T)->U) -> List<U>", 
            "Apply function", "iterate", "O(n)");
        self.add_function("list_filter", "collections", "fn list_filter<T>(list: List<T>, fn: Fn(T)->Bool) -> List<T>", 
            "Filter items", "iterate", "O(n)");
        self.add_function("list_reduce", "collections", "fn list_reduce<T>(list: List<T>, fn: Fn(T,T)->T) -> T", 
            "Fold/reduce", "iterate", "O(n)");
        self.add_function("list_unique", "collections", "fn list_unique<T>(list: List<T>) -> List<T>", 
            "Remove duplicates", "hash-set", "O(n)");
        self.add_function("list_flatten", "collections", "fn list_flatten<T>(list: List<List<T>>) -> List<T>", 
            "Flatten nested", "iterate", "O(n)");
        self.add_function("list_zip", "collections", "fn list_zip<T,U>(a: List<T>, b: List<U>) -> List<(T,U)>", 
            "Zip two lists", "pair-iterate", "O(min(n,m))");
        self.add_function("list_chunk", "collections", "fn list_chunk<T>(list: List<T>, size: Int) -> List<List<T>>", 
            "Split into chunks", "slice-loop", "O(n)");
        self.add_function("list_take", "collections", "fn list_take<T>(list: List<T>, count: Int) -> List<T>", 
            "Take first N", "slice", "O(n)");
        self.add_function("list_drop", "collections", "fn list_drop<T>(list: List<T>, count: Int) -> List<T>", 
            "Drop first N", "slice", "O(n)");

        // Map operations (15)
        self.add_function("map_new", "collections", "fn map_new<K,V>() -> Map<K,V>", 
            "Create empty map", "allocate", "O(1)");
        self.add_function("map_put", "collections", "fn map_put<K,V>(map: Map<K,V>, key: K, value: V) -> Void", 
            "Set key-value", "hash-insert", "O(1)");
        self.add_function("map_get", "collections", "fn map_get<K,V>(map: Map<K,V>, key: K) -> V", 
            "Get by key", "hash-lookup", "O(1)");
        self.add_function("map_remove", "collections", "fn map_remove<K,V>(map: Map<K,V>, key: K) -> V", 
            "Remove by key", "hash-remove", "O(1)");
        self.add_function("map_contains", "collections", "fn map_contains<K,V>(map: Map<K,V>, key: K) -> Bool", 
            "Check key exists", "hash-lookup", "O(1)");
        self.add_function("map_keys", "collections", "fn map_keys<K,V>(map: Map<K,V>) -> List<K>", 
            "Get all keys", "iterate", "O(n)");
        self.add_function("map_values", "collections", "fn map_values<K,V>(map: Map<K,V>) -> List<V>", 
            "Get all values", "iterate", "O(n)");
        self.add_function("map_clear", "collections", "fn map_clear<K,V>(map: Map<K,V>) -> Void", 
            "Remove all items", "truncate", "O(1)");
        self.add_function("map_size", "collections", "fn map_size<K,V>(map: Map<K,V>) -> Int", 
            "Get item count", "length", "O(1)");
        self.add_function("map_is_empty", "collections", "fn map_is_empty<K,V>(map: Map<K,V>) -> Bool", 
            "Check if empty", "length", "O(1)");
        self.add_function("map_merge", "collections", "fn map_merge<K,V>(a: Map<K,V>, b: Map<K,V>) -> Map<K,V>", 
            "Merge two maps", "iterate+insert", "O(n+m)");
        self.add_function("map_map", "collections", "fn map_map<K,V,U>(map: Map<K,V>, fn: Fn(K,V)->U) -> List<U>", 
            "Transform values", "iterate", "O(n)");
        self.add_function("map_filter", "collections", "fn map_filter<K,V>(map: Map<K,V>, fn: Fn(K,V)->Bool) -> Map<K,V>", 
            "Filter entries", "iterate+insert", "O(n)");
        self.add_function("map_from_pairs", "collections", "fn map_from_pairs<K,V>(pairs: List<(K,V)>) -> Map<K,V>", 
            "Create from list", "iterate+insert", "O(n)");
        self.add_function("map_to_pairs", "collections", "fn map_to_pairs<K,V>(map: Map<K,V>) -> List<(K,V)>", 
            "Convert to list", "iterate", "O(n)");

        // Set operations (10)
        self.add_function("set_new", "collections", "fn set_new<T>() -> Set<T>", 
            "Create empty set", "allocate", "O(1)");
        self.add_function("set_add", "collections", "fn set_add<T>(set: Set<T>, item: T) -> Void", 
            "Add item", "hash-insert", "O(1)");
        self.add_function("set_remove", "collections", "fn set_remove<T>(set: Set<T>, item: T) -> Void", 
            "Remove item", "hash-remove", "O(1)");
        self.add_function("set_contains", "collections", "fn set_contains<T>(set: Set<T>, item: T) -> Bool", 
            "Check membership", "hash-lookup", "O(1)");
        self.add_function("set_union", "collections", "fn set_union<T>(a: Set<T>, b: Set<T>) -> Set<T>", 
            "Set union", "iterate+insert", "O(n+m)");
        self.add_function("set_intersection", "collections", "fn set_intersection<T>(a: Set<T>, b: Set<T>) -> Set<T>", 
            "Set intersection", "iterate+lookup", "O(min(n,m))");
        self.add_function("set_difference", "collections", "fn set_difference<T>(a: Set<T>, b: Set<T>) -> Set<T>", 
            "Set difference", "iterate+lookup", "O(n)");
        self.add_function("set_symmetric_diff", "collections", "fn set_symmetric_diff<T>(a: Set<T>, b: Set<T>) -> Set<T>", 
            "Symmetric difference", "union-minus-intersect", "O(n+m)");
        self.add_function("set_size", "collections", "fn set_size<T>(set: Set<T>) -> Int", 
            "Get item count", "length", "O(1)");
        self.add_function("set_to_list", "collections", "fn set_to_list<T>(set: Set<T>) -> List<T>", 
            "Convert to list", "iterate", "O(n)");
    }

    fn build_io_library(&mut self) {
        self.add_function("print", "io", "fn print(msg: String) -> Void", 
            "Print to stdout", "puts", "O(n)");
        self.add_function("println", "io", "fn println(msg: String) -> Void", 
            "Print with newline", "puts+newline", "O(n)");
        self.add_function("read_line", "io", "fn read_line() -> String", 
            "Read from stdin", "getline", "O(n)");
        self.add_function("read_file", "io", "fn read_file(path: String) -> String", 
            "Read entire file", "fopen+fread", "O(n)");
        self.add_function("write_file", "io", "fn write_file(path: String, content: String) -> Void", 
            "Write entire file", "fopen+fwrite", "O(n)");
        self.add_function("append_file", "io", "fn append_file(path: String, content: String) -> Void", 
            "Append to file", "fopen+fwrite", "O(n)");
        self.add_function("file_exists", "io", "fn file_exists(path: String) -> Bool", 
            "Check file exists", "access", "O(1)");
        self.add_function("list_files", "io", "fn list_files(dir: String) -> List<String>", 
            "List directory", "scandir", "O(n)");
        self.add_function("mkdir", "io", "fn mkdir(path: String) -> Bool", 
            "Create directory", "mkdir", "O(1)");
        self.add_function("delete_file", "io", "fn delete_file(path: String) -> Bool", 
            "Delete file", "unlink", "O(1)");
    }

    fn build_time_library(&mut self) {
        self.add_function("now_ms", "time", "fn now_ms() -> Long", 
            "Milliseconds since epoch", "time_call", "O(1)");
        self.add_function("now_s", "time", "fn now_s() -> Long", 
            "Seconds since epoch", "time_call", "O(1)");
        self.add_function("sleep", "time", "fn sleep(ms: Int) -> Void", 
            "Sleep milliseconds", "usleep", "O(ms)");
        self.add_function("sleep_seconds", "time", "fn sleep_seconds(s: Int) -> Void", 
            "Sleep seconds", "sleep", "O(s)");
    }

    fn build_type_library(&mut self) {
        self.add_function("type_of", "type", "fn type_of(x: Any) -> String", 
            "Get type name", "typeof", "O(1)");
        self.add_function("is_int", "type", "fn is_int(x: Any) -> Bool", 
            "Check if int", "type-check", "O(1)");
        self.add_function("is_float", "type", "fn is_float(x: Any) -> Bool", 
            "Check if float", "type-check", "O(1)");
        self.add_function("is_string", "type", "fn is_string(x: Any) -> Bool", 
            "Check if string", "type-check", "O(1)");
    }

    fn build_concurrency_library(&mut self) {
        self.add_function("spawn_actor", "concurrency", "fn spawn_actor<T>(fn: Fn()->T) -> Actor", 
            "Spawn actor", "actor_new", "O(1)");
        self.add_function("send_message", "concurrency", "fn send_message(actor: Actor, msg: String) -> Void", 
            "Send message", "enqueue", "O(1)");
        self.add_function("receive_message", "concurrency", "fn receive_message(actor: Actor) -> String", 
            "Receive message", "dequeue", "O(1)");
    }

    fn add_function(&mut self, name: &str, category: &str, sig: &str, desc: &str, impl_: &str, complexity: &str) {
        let func = StdlibFunction {
            name: name.to_string(),
            category: category.to_string(),
            signature: sig.to_string(),
            description: desc.to_string(),
            implementation: impl_.to_string(),
            complexity: complexity.to_string(),
            native_equivalent: None,
        };
        self.functions.insert(name.to_string(), func);
    }

    pub fn get_function(&self, name: &str) -> Option<&StdlibFunction> {
        self.functions.get(name)
    }

    pub fn list_functions(&self, category: &str) -> Vec<&StdlibFunction> {
        self.functions
            .values()
            .filter(|f| f.category == category)
            .collect()
    }

    pub fn list_all(&self) -> Vec<&StdlibFunction> {
        self.functions.values().collect()
    }

    pub fn count(&self) -> usize {
        self.functions.len()
    }

    pub fn count_by_category(&self, category: &str) -> usize {
        self.functions
            .values()
            .filter(|f| f.category == category)
            .count()
    }

    pub fn generate_killer_module(&self) -> String {
        let mut output = String::from("// AUTO-GENERATED: Killer Standard Library v1.0\n");
        output.push_str(&format!("// Total Functions: {}\n", self.count()));
        output.push_str("\n");

        for category in &["math", "string", "collections", "io", "time", "type", "concurrency"] {
            output.push_str(&format!("// --- {} LIBRARY ---\n", category.to_uppercase()));
            let funcs = self.list_functions(category);
            output.push_str(&format!("// Functions: {}\n\n", funcs.len()));
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdlib_builder_new() {
        let builder = StdlibBuilder::new();
        assert!(builder.count() > 0);
    }

    #[test]
    fn test_stdlib_builder_count() {
        let builder = StdlibBuilder::new();
        assert_eq!(builder.count(), 201); // 220 - 19 FFI functions (disabled for v4.0)
    }

    #[test]
    fn test_stdlib_math_library() {
        let builder = StdlibBuilder::new();
        assert!(builder.get_function("sqrt").is_some());
        assert!(builder.get_function("sin").is_some());
        assert!(builder.get_function("random").is_some());
    }

    #[test]
    fn test_stdlib_string_library() {
        let builder = StdlibBuilder::new();
        assert!(builder.get_function("length").is_some());
        assert!(builder.get_function("split").is_some());
        assert!(builder.get_function("to_upper").is_some());
    }

    #[test]
    fn test_stdlib_collections_library() {
        let builder = StdlibBuilder::new();
        assert!(builder.get_function("list_push").is_some());
        assert!(builder.get_function("map_get").is_some());
        assert!(builder.get_function("set_add").is_some());
    }

    #[test]
    fn test_stdlib_function_metadata() {
        let builder = StdlibBuilder::new();
        let func = builder.get_function("sqrt").unwrap();
        assert_eq!(func.category, "math");
        assert!(func.signature.contains("Float"));
    }
}
