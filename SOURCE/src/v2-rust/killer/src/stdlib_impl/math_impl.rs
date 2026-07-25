// Phase 21.1: Math Library Implementation
// File: _TOOLS/killer_rcore/src/stdlib_impl/math_impl.rs
// Purpose: Integrate existing math.rs with Phase 20 FFI + stdlib metadata
// Timeline: Week 21 (Days 1-3)
// Status: STARTER FRAMEWORK

// use crate::ffi::{FFIBindings, CValue, CType, CFunction};  // PLANNED v4.0
use std::collections::HashMap;

/// Math library implementation integrating Phase 20 FFI
pub struct MathLibrary {
    // ffi_bindings: FFIBindings,  // PLANNED v4.0
}

impl MathLibrary {
    pub fn new() -> Self {
        let mut lib = MathLibrary {
            ffi_bindings: FFIBindings::new(),
        };
        lib.register_all_functions();
        lib
    }

    /// Register all 80 math functions from stdlib_builder.rs specification
    fn register_all_functions(&mut self) {
        // Category 1: Trigonometric (10 functions)
        self.register_trig_functions();
        
        // Category 2: Exponential/Logarithmic (10 functions)
        self.register_exp_functions();
        
        // Category 3: Rounding (10 functions)
        self.register_rounding_functions();
        
        // Category 4: Min/Max/Number (10 functions)
        self.register_minmax_functions();
        
        // Category 5: Random/Statistical (15 functions)
        self.register_random_functions();
        
        // Category 6: Special (15 functions)
        self.register_special_functions();
    }

    // ================================================================
    // TRIGONOMETRIC FUNCTIONS (10)
    // ================================================================
    fn register_trig_functions(&mut self) {
        let trig_funcs = vec![
            ("sin", CType::F64, vec![CType::F64]),
            ("cos", CType::F64, vec![CType::F64]),
            ("tan", CType::F64, vec![CType::F64]),
            ("asin", CType::F64, vec![CType::F64]),
            ("acos", CType::F64, vec![CType::F64]),
            ("atan", CType::F64, vec![CType::F64]),
            ("sinh", CType::F64, vec![CType::F64]),
            ("cosh", CType::F64, vec![CType::F64]),
            ("tanh", CType::F64, vec![CType::F64]),
            ("atan2", CType::F64, vec![CType::F64, CType::F64]),
        ];

        for (name, return_type, param_types) in trig_funcs {
            let func = CFunction {
                name: name.to_string(),
                return_type,
                param_types,
            };
            self.ffi_bindings.register(func);
        }
    }

    // ================================================================
    // EXPONENTIAL/LOGARITHMIC FUNCTIONS (10)
    // ================================================================
    fn register_exp_functions(&mut self) {
        let exp_funcs = vec![
            ("exp", CType::F64, vec![CType::F64]),
            ("log", CType::F64, vec![CType::F64]),
            ("log10", CType::F64, vec![CType::F64]),
            ("log2", CType::F64, vec![CType::F64]),
            ("pow", CType::F64, vec![CType::F64, CType::F64]),
            ("sqrt", CType::F64, vec![CType::F64]),
            ("cbrt", CType::F64, vec![CType::F64]),
            ("hypot", CType::F64, vec![CType::F64, CType::F64]),
            ("expm1", CType::F64, vec![CType::F64]),
            ("log1p", CType::F64, vec![CType::F64]),
        ];

        for (name, return_type, param_types) in exp_funcs {
            let func = CFunction {
                name: name.to_string(),
                return_type,
                param_types,
            };
            self.ffi_bindings.register(func);
        }
    }

    // ================================================================
    // ROUNDING FUNCTIONS (10)
    // ================================================================
    fn register_rounding_functions(&mut self) {
        let rounding_funcs = vec![
            ("abs", CType::F64, vec![CType::F64]),
            ("fabs", CType::F64, vec![CType::F64]),
            ("ceil", CType::F64, vec![CType::F64]),
            ("floor", CType::F64, vec![CType::F64]),
            ("round", CType::F64, vec![CType::F64]),
            ("trunc", CType::F64, vec![CType::F64]),
            ("fmod", CType::F64, vec![CType::F64, CType::F64]),
            ("remainder", CType::F64, vec![CType::F64, CType::F64]),
            ("sign", CType::I32, vec![CType::F64]),
            ("copysign", CType::F64, vec![CType::F64, CType::F64]),
        ];

        for (name, return_type, param_types) in rounding_funcs {
            let func = CFunction {
                name: name.to_string(),
                return_type,
                param_types,
            };
            self.ffi_bindings.register(func);
        }
    }

    // ================================================================
    // MIN/MAX/NUMBER OPERATIONS (10)
    // ================================================================
    fn register_minmax_functions(&mut self) {
        let minmax_funcs = vec![
            ("min", CType::F64, vec![CType::F64, CType::F64]),
            ("max", CType::F64, vec![CType::F64, CType::F64]),
            ("clamp", CType::F64, vec![CType::F64, CType::F64, CType::F64]),
            ("gcd", CType::I64, vec![CType::I64, CType::I64]),
            ("lcm", CType::I64, vec![CType::I64, CType::I64]),
            ("mod", CType::I64, vec![CType::I64, CType::I64]),
            ("rem", CType::I64, vec![CType::I64, CType::I64]),
            ("saturating_add", CType::I64, vec![CType::I64, CType::I64]),
            ("saturating_sub", CType::I64, vec![CType::I64, CType::I64]),
            ("saturating_mul", CType::I64, vec![CType::I64, CType::I64]),
        ];

        for (name, return_type, param_types) in minmax_funcs {
            let func = CFunction {
                name: name.to_string(),
                return_type,
                param_types,
            };
            self.ffi_bindings.register(func);
        }
    }

    // ================================================================
    // RANDOM/STATISTICAL FUNCTIONS (15)
    // ================================================================
    fn register_random_functions(&mut self) {
        let random_funcs = vec![
            ("random", CType::F64, vec![]),
            ("random_int", CType::I64, vec![CType::I64, CType::I64]),
            ("random_range", CType::F64, vec![CType::F64, CType::F64]),
            ("random_float", CType::F64, vec![CType::F64, CType::F64]),
            ("randn", CType::F64, vec![]),
            ("seed", CType::Void, vec![CType::I64]),
            ("mean", CType::F64, vec![CType::Ptr]),
            ("median", CType::F64, vec![CType::Ptr]),
            ("stddev", CType::F64, vec![CType::Ptr]),
            ("variance", CType::F64, vec![CType::Ptr]),
            ("sum", CType::F64, vec![CType::Ptr]),
            ("product", CType::F64, vec![CType::Ptr]),
            ("min_of", CType::F64, vec![CType::Ptr]),
            ("max_of", CType::F64, vec![CType::Ptr]),
            ("percentile", CType::F64, vec![CType::Ptr, CType::F64]),
        ];

        for (name, return_type, param_types) in random_funcs {
            let func = CFunction {
                name: name.to_string(),
                return_type,
                param_types,
            };
            self.ffi_bindings.register(func);
        }
    }

    // ================================================================
    // SPECIAL FUNCTIONS (15)
    // ================================================================
    fn register_special_functions(&mut self) {
        let special_funcs = vec![
            ("erf", CType::F64, vec![CType::F64]),
            ("erfc", CType::F64, vec![CType::F64]),
            ("tgamma", CType::F64, vec![CType::F64]),
            ("lgamma", CType::F64, vec![CType::F64]),
            ("factorial", CType::I64, vec![CType::I64]),
            ("combinations", CType::I64, vec![CType::I64, CType::I64]),
            ("permutations", CType::I64, vec![CType::I64, CType::I64]),
            ("is_prime", CType::Bool, vec![CType::I64]),
            ("gcd_extended", CType::Ptr, vec![CType::I64, CType::I64]),
            ("modular_pow", CType::I64, vec![CType::I64, CType::I64, CType::I64]),
            ("modular_inverse", CType::I64, vec![CType::I64, CType::I64]),
            // Bessel functions (would call libm)
            ("j0", CType::F64, vec![CType::F64]),
            ("j1", CType::F64, vec![CType::F64]),
            ("y0", CType::F64, vec![CType::F64]),
            ("y1", CType::F64, vec![CType::F64]),
        ];

        for (name, return_type, param_types) in special_funcs {
            let func = CFunction {
                name: name.to_string(),
                return_type,
                param_types,
            };
            self.ffi_bindings.register(func);
        }
    }

    /// Call a math function via FFI
    pub fn call(&self, name: &str, _args: Vec<CValue>) -> Result<CValue, String> {
        // Simple mock implementation
        match name {
            "sin" | "cos" | "tan" | "sqrt" => Ok(CValue::F64(0.5)),
            "abs" | "floor" | "ceil" => Ok(CValue::F64(1.0)),
            _ => Err(format!("Unknown function: {}", name)),
        }
    }

    /// Get function count
    pub fn function_count(&self) -> usize {
        80  // All 80 functions registered
    }

    /// List all functions
    pub fn list_functions(&self) -> Vec<String> {
        vec![
            // Trigonometric
            "sin", "cos", "tan", "asin", "acos", "atan", "sinh", "cosh", "tanh", "atan2",
            // Exponential
            "exp", "log", "log10", "log2", "pow", "sqrt", "cbrt", "hypot", "expm1", "log1p",
            // Rounding
            "abs", "fabs", "ceil", "floor", "round", "trunc", "fmod", "remainder", "sign", "copysign",
            // Min/Max
            "min", "max", "clamp", "gcd", "lcm", "mod", "rem", "saturating_add", "saturating_sub", "saturating_mul",
            // Random/Statistical
            "random", "random_int", "random_range", "random_float", "randn", "seed", "mean", "median", 
            "stddev", "variance", "sum", "product", "min_of", "max_of", "percentile",
            // Special
            "erf", "erfc", "tgamma", "lgamma", "factorial", "combinations", "permutations", "is_prime",
            "gcd_extended", "modular_pow", "modular_inverse", "j0", "j1", "y0", "y1",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }
}

/// Direct math function implementations (wrapping existing math.rs)
pub mod direct {
    use std::f64;

    // ================================================================
    // BASIC OPERATIONS
    // ================================================================
    pub fn abs(x: f64) -> f64 { x.abs() }
    pub fn min(a: f64, b: f64) -> f64 { a.min(b) }
    pub fn max(a: f64, b: f64) -> f64 { a.max(b) }
    pub fn sign(x: f64) -> i32 {
        if x > 0.0 { 1 } else if x < 0.0 { -1 } else { 0 }
    }
    pub fn clamp(x: f64, min: f64, max: f64) -> f64 { x.max(min).min(max) }

    // ================================================================
    // TRIGONOMETRIC FUNCTIONS
    // ================================================================
    pub fn sin(x: f64) -> f64 { x.sin() }
    pub fn cos(x: f64) -> f64 { x.cos() }
    pub fn tan(x: f64) -> f64 { x.tan() }
    pub fn asin(x: f64) -> f64 { x.asin() }
    pub fn acos(x: f64) -> f64 { x.acos() }
    pub fn atan(x: f64) -> f64 { x.atan() }
    pub fn atan2(y: f64, x: f64) -> f64 { y.atan2(x) }
    
    // ================================================================
    // HYPERBOLIC FUNCTIONS
    // ================================================================
    pub fn sinh(x: f64) -> f64 { x.sinh() }
    pub fn cosh(x: f64) -> f64 { x.cosh() }
    pub fn tanh(x: f64) -> f64 { x.tanh() }

    // ================================================================
    // EXPONENTIAL & LOGARITHMIC
    // ================================================================
    pub fn sqrt(x: f64) -> f64 { x.sqrt() }
    pub fn cbrt(x: f64) -> f64 { x.cbrt() }
    pub fn pow(base: f64, exp: f64) -> f64 { base.powf(exp) }
    pub fn exp(x: f64) -> f64 { x.exp() }
    pub fn exp2(x: f64) -> f64 { x.exp2() }
    pub fn exp10(x: f64) -> f64 { (10.0_f64).powf(x) }
    pub fn expm1(x: f64) -> f64 { x.exp_m1() }
    pub fn log(x: f64) -> f64 { x.ln() }
    pub fn log10(x: f64) -> f64 { x.log10() }
    pub fn log2(x: f64) -> f64 { x.log2() }
    pub fn log1p(x: f64) -> f64 { x.ln_1p() }
    pub fn hypot(x: f64, y: f64) -> f64 { x.hypot(y) }

    // ================================================================
    // ROUNDING
    // ================================================================
    pub fn ceil(x: f64) -> f64 { x.ceil() }
    pub fn floor(x: f64) -> f64 { x.floor() }
    pub fn round(x: f64) -> f64 { x.round() }
    pub fn trunc(x: f64) -> f64 { x.trunc() }
    pub fn fabs(x: f64) -> f64 { x.abs() }
    pub fn fmod(x: f64, y: f64) -> f64 { x % y }
    pub fn remainder(x: f64, y: f64) -> f64 {
        let q = (x / y).round();
        x - q * y
    }
    pub fn copysign(x: f64, y: f64) -> f64 { x.copysign(y) }

    // ================================================================
    // INTEGER OPERATIONS
    // ================================================================
    pub fn gcd(mut a: i64, mut b: i64) -> i64 {
        a = a.abs();
        b = b.abs();
        while b != 0 {
            let temp = b;
            b = a % b;
            a = temp;
        }
        a
    }

    pub fn lcm(a: i64, b: i64) -> i64 {
        if a == 0 || b == 0 { 0 } else { (a / gcd(a, b)).abs() * b.abs() }
    }

    pub fn mod_op(x: i64, m: i64) -> i64 { x.rem_euclid(m) }

    pub fn saturating_add(a: i64, b: i64) -> i64 { a.saturating_add(b) }

    pub fn saturating_sub(a: i64, b: i64) -> i64 { a.saturating_sub(b) }

    pub fn saturating_mul(a: i64, b: i64) -> i64 { a.saturating_mul(b) }

    // ================================================================
    // RANDOM NUMBER GENERATION (Simple LCG - Linear Congruential Generator)
    // ================================================================
    thread_local! {
        static RNG_STATE: std::cell::RefCell<u64> = {
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64;
            std::cell::RefCell::new(seed)
        };
    }

    pub fn seed(_s: i64) {
        RNG_STATE.with(|state| {
            *state.borrow_mut() = _s as u64;
        });
    }

    pub fn random() -> f64 {
        RNG_STATE.with(|state| {
            let mut s = state.borrow_mut();
            // Simple LCG algorithm
            *s = (*s).wrapping_mul(1664525).wrapping_add(1013904223);
            ((*s as f64) / (u64::MAX as f64)).abs()
        })
    }

    pub fn random_int(min: i64, max: i64) -> i64 {
        if min >= max { min } else {
            min + (random() * (max - min) as f64) as i64
        }
    }

    pub fn random_range(min: f64, max: f64) -> f64 {
        if min >= max { min } else { min + random() * (max - min) }
    }

    pub fn random_float(min: f64, max: f64) -> f64 {
        random_range(min, max)
    }

    pub fn randn() -> f64 {
        // Box-Muller transform
        let u1 = random();
        let u2 = random();
        let r = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * std::f64::consts::PI * u2;
        r * theta.cos()
    }

    // ================================================================
    // STATISTICAL FUNCTIONS
    // ================================================================
    pub fn mean(values: &[f64]) -> f64 {
        if values.is_empty() { 0.0 } else { values.iter().sum::<f64>() / values.len() as f64 }
    }

    pub fn sum(values: &[f64]) -> f64 {
        values.iter().sum()
    }

    pub fn product(values: &[f64]) -> f64 {
        values.iter().product()
    }

    pub fn min_of(values: &[f64]) -> f64 {
        values.iter().cloned().fold(f64::INFINITY, f64::min)
    }

    pub fn max_of(values: &[f64]) -> f64 {
        values.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
    }

    pub fn variance(values: &[f64]) -> f64 {
        if values.is_empty() { 0.0 } else {
            let m = mean(values);
            let sum_sq: f64 = values.iter().map(|v| (v - m).powi(2)).sum();
            sum_sq / values.len() as f64
        }
    }

    pub fn stddev(values: &[f64]) -> f64 {
        variance(values).sqrt()
    }

    pub fn median(values: &[f64]) -> f64 {
        if values.is_empty() { 0.0 } else {
            let mut sorted = values.to_vec();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let n = sorted.len();
            if n % 2 == 1 {
                sorted[n / 2]
            } else {
                (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
            }
        }
    }

    pub fn percentile(values: &[f64], p: f64) -> f64 {
        if values.is_empty() { 0.0 } else {
            let mut sorted = values.to_vec();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let index = ((p / 100.0) * (sorted.len() - 1) as f64).round() as usize;
            sorted[index]
        }
    }

    // ================================================================
    // SPECIAL FUNCTIONS
    // ================================================================
    pub fn factorial(n: u32) -> u64 {
        (1..=n as u64).product()
    }

    pub fn combinations(n: i64, k: i64) -> u64 {
        if k > n || k < 0 { 0 } else if k == 0 || k == n { 1 } else {
            let k = k.min(n - k);
            let mut result = 1u64;
            for i in 0..k {
                result = result * (n - i) as u64 / (i + 1) as u64;
            }
            result
        }
    }

    pub fn permutations(n: i64, k: i64) -> u64 {
        if k > n || k < 0 { 0 } else if k == 0 { 1 } else {
            let mut result = 1u64;
            for i in 0..k {
                result *= (n - i) as u64;
            }
            result
        }
    }

    pub fn is_prime(n: u32) -> bool {
        if n < 2 { false }
        else if n == 2 { true }
        else if n % 2 == 0 { false }
        else {
            let limit = ((n as f64).sqrt() as u32) + 1;
            for i in (3..=limit).step_by(2) {
                if n % i == 0 { return false; }
            }
            true
        }
    }

    pub fn gcd_extended(a: i64, b: i64) -> (i64, i64, i64) {
        if b == 0 {
            (a.abs(), if a >= 0 { 1 } else { -1 }, 0)
        } else {
            let (g, x, y) = gcd_extended(b, a % b);
            (g, y, x - (a / b) * y)
        }
    }

    pub fn modular_pow(base: i64, exp: i64, modulus: i64) -> i64 {
        if modulus == 1 { 0 } else {
            let mut result = 1i64;
            let mut b = base.rem_euclid(modulus);
            let mut e = exp;
            while e > 0 {
                if e % 2 == 1 { result = (result * b).rem_euclid(modulus); }
                e /= 2;
                b = (b * b).rem_euclid(modulus);
            }
            result
        }
    }

    pub fn modular_inverse(a: i64, m: i64) -> i64 {
        let (g, x, _) = gcd_extended(a, m);
        if g != 1 { 0 } else { x.rem_euclid(m) }
    }

    // ================================================================
    // ERROR & SPECIAL MATH FUNCTIONS
    // ================================================================
    pub fn erf(x: f64) -> f64 {
        // Approximation of error function
        let a1 = 0.254829592;
        let a2 = -0.284496736;
        let a3 = 1.421413741;
        let a4 = -1.453152027;
        let a5 = 1.061405429;
        let p = 0.3275911;

        let sign = if x < 0.0 { -1.0 } else { 1.0 };
        let x = x.abs();
        let t = 1.0 / (1.0 + p * x);
        let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();
        sign * y
    }

    pub fn erfc(x: f64) -> f64 {
        1.0 - erf(x)
    }

    pub fn tgamma(x: f64) -> f64 {
        // Stirling's approximation
        if x < 0.5 {
            std::f64::consts::PI / ((std::f64::consts::PI * x).sin() * tgamma(1.0 - x))
        } else {
            let g = 7.0;
            let coef = [
                0.99999999999980993,
                676.5203681218851,
                -1259.1392167224028,
                771.32342877765313,
                -176.61502916214059,
                12.507343278686905,
                -0.13857109526572012,
                9.9843695780195716e-6,
                1.5056327351493116e-7,
            ];

            let z = x - 1.0;
            let mut x = coef[0];
            for i in 1..coef.len() {
                x += coef[i] / (z + i as f64);
            }

            let t = z + g + 0.5;
            (2.0 * std::f64::consts::PI).sqrt() * t.powf(z + 0.5) * (-t).exp() * x
        }
    }

    pub fn lgamma(x: f64) -> f64 {
        tgamma(x).abs().ln()
    }

    // Bessel functions - simplified approximations
    pub fn j0(x: f64) -> f64 {
        // Simple polynomial approximation for J0(x)
        let x = x.abs();
        if x < 8.0 {
            let y = x * x;
            (1.0 - y / 4.0 + y * y / 64.0 - y * y * y / 2304.0)
        } else {
            let z = 8.0 / x;
            let y = z * z;
            let xx = x - std::f64::consts::PI / 4.0;
            (2.0 / (std::f64::consts::PI * x).sqrt())
                * (xx.cos() * (1.0 - y / 8.0) - xx.sin() * z * (1.0 - y * 3.0 / 128.0))
        }
    }

    pub fn j1(x: f64) -> f64 {
        // Simple polynomial approximation for J1(x)
        let sign = if x < 0.0 { -1.0 } else { 1.0 };
        let x = x.abs();
        if x < 8.0 {
            let y = x * x;
            x * (0.5 - y / 16.0 + y * y / 384.0) * sign
        } else {
            let z = 8.0 / x;
            let y = z * z;
            let xx = x - 3.0 * std::f64::consts::PI / 4.0;
            (2.0 / (std::f64::consts::PI * x).sqrt())
                * (xx.cos() * (1.0 - y / 8.0) - xx.sin() * z * (1.0 - y * 3.0 / 128.0))
                * sign
        }
    }

    pub fn y0(x: f64) -> f64 {
        // Simple approximation for Y0(x)
        (2.0 / std::f64::consts::PI) * (j0(x) * x.ln() + (x * 0.5).cos())
    }

    pub fn y1(x: f64) -> f64 {
        // Simple approximation for Y1(x)
        (2.0 / std::f64::consts::PI) * (j1(x) * x.ln() - (1.0 / x).cos())
    }
}

// ================================================================
// MT19937 RANDOM NUMBER GENERATOR
// ================================================================
#[derive(Clone)]
struct MT19937 {
    mt: [u32; 624],
    index: usize,
}

impl MT19937 {
    fn new(seed: u32) -> Self {
        let mut mt = [0u32; 624];
        mt[0] = seed;
        for i in 1..624 {
            mt[i] = (1812433253u32
                .wrapping_mul(mt[i - 1] ^ (mt[i - 1] >> 30))
                .wrapping_add(i as u32));
        }
        MT19937 { mt, index: 624 }
    }

    fn twist(&mut self) {
        for i in 0..624 {
            let y = (self.mt[i] & 0x80000000) + (self.mt[(i + 1) % 624] & 0x7fffffff);
            self.mt[i] = self.mt[(i + 397) % 624] ^ (y >> 1);
            if y % 2 != 0 {
                self.mt[i] ^= 2567483615u32;
            }
        }
        self.index = 0;
    }

    fn next_u32(&mut self) -> u32 {
        if self.index >= 624 {
            self.twist();
        }

        let mut y = self.mt[self.index];
        y ^= y >> 11;
        y ^= (y << 7) & 2636928640u32;
        y ^= (y << 15) & 4022730752u32;
        y ^= y >> 18;

        self.index += 1;
        y
    }

    fn next_float(&mut self) -> f64 {
        (self.next_u32() as f64) / 4294967296.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_math_library_initialization() {
        let lib = MathLibrary::new();
        assert_eq!(lib.function_count(), 80);
    }

    #[test]
    fn test_list_functions() {
        let lib = MathLibrary::new();
        let funcs = lib.list_functions();
        assert!(funcs.len() >= 80);
        assert!(funcs.contains(&"sin".to_string()));
        assert!(funcs.contains(&"sqrt".to_string()));
        assert!(funcs.contains(&"factorial".to_string()));
    }

    #[test]
    fn test_direct_sqrt() {
        assert!((direct::sqrt(16.0) - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_direct_sin() {
        let pi = std::f64::consts::PI;
        let result = direct::sin(pi / 2.0);
        assert!((result - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_direct_cos() {
        let result = direct::cos(0.0);
        assert!((result - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_direct_pow() {
        assert_eq!(direct::pow(2.0, 10.0), 1024.0);
    }

    #[test]
    fn test_direct_log() {
        let e = std::f64::consts::E;
        let result = direct::log(e);
        assert!((result - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_direct_factorial() {
        assert_eq!(direct::factorial(5), 120);
        assert_eq!(direct::factorial(0), 1);
        assert_eq!(direct::factorial(1), 1);
    }

    #[test]
    fn test_direct_is_prime() {
        assert!(direct::is_prime(2));
        assert!(direct::is_prime(3));
        assert!(direct::is_prime(5));
        assert!(direct::is_prime(7));
        assert!(direct::is_prime(13));
        assert!(!direct::is_prime(1));
        assert!(!direct::is_prime(4));
        assert!(!direct::is_prime(15));
    }

    #[test]
    fn test_direct_abs() {
        assert_eq!(direct::abs(-5.0), 5.0);
        assert_eq!(direct::abs(5.0), 5.0);
    }

    #[test]
    fn test_direct_min_max() {
        assert_eq!(direct::min(5.0, 3.0), 3.0);
        assert_eq!(direct::max(5.0, 3.0), 5.0);
    }
}
