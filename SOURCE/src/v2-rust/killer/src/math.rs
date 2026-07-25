// Math Module for Killer Language
// Comprehensive mathematical functions and utilities
// Version: 2.1.0

use std::f64;

/// Math module providing 30+ mathematical functions
/// Includes: basic operations, trigonometry, logarithms, random numbers, and special functions
pub struct MathModule;

/// Mathematical constants
pub mod constants {
    
    pub const PI: f64 = std::f64::consts::PI;
    pub const E: f64 = std::f64::consts::E;
    pub const LN2: f64 = std::f64::consts::LN_2;
    pub const LN10: f64 = std::f64::consts::LN_10;
    pub const LOG2E: f64 = std::f64::consts::LOG2_E;
    pub const LOG10E: f64 = std::f64::consts::LOG10_E;
    pub const SQRT2: f64 = std::f64::consts::SQRT_2;
    pub const SQRT_HALF: f64 = std::f64::consts::FRAC_1_SQRT_2;
    pub const TAU: f64 = std::f64::consts::TAU; // 2 * PI
    pub const INF: f64 = f64::INFINITY;
    pub const NEG_INF: f64 = f64::NEG_INFINITY;
    pub const NAN: f64 = f64::NAN;
}

impl MathModule {
    // ==================== Basic Operations ====================
    
    /// Absolute value
    /// abs(-5) => 5
    pub fn abs(n: f64) -> f64 {
        n.abs()
    }
    
    /// Minimum of two numbers
    /// min(5, 3) => 3
    pub fn min(a: f64, b: f64) -> f64 {
        a.min(b)
    }
    
    /// Maximum of two numbers
    /// max(5, 3) => 5
    pub fn max(a: f64, b: f64) -> f64 {
        a.max(b)
    }
    
    /// Sign of a number: -1, 0, or 1
    /// sign(-5) => -1
    pub fn sign(n: f64) -> f64 {
        if n > 0.0 { 1.0 } else if n < 0.0 { -1.0 } else { 0.0 }
    }
    
    /// Clamp value between min and max
    /// clamp(5, 0, 3) => 3
    pub fn clamp(n: f64, min: f64, max: f64) -> f64 {
        n.max(min).min(max)
    }
    
    /// Sum of all numbers
    /// sum([1, 2, 3, 4]) => 10
    pub fn sum(nums: &[f64]) -> f64 {
        nums.iter().sum()
    }
    
    /// Average of all numbers
    /// average([1, 2, 3, 4]) => 2.5
    pub fn average(nums: &[f64]) -> f64 {
        if nums.is_empty() { 0.0 } else { Self::sum(nums) / nums.len() as f64 }
    }
    
    /// Product of all numbers
    /// product([1, 2, 3, 4]) => 24
    pub fn product(nums: &[f64]) -> f64 {
        nums.iter().product()
    }
    
    /// Greatest common divisor
    /// gcd(48, 18) => 6
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
    
    /// Least common multiple
    /// lcm(12, 18) => 36
    pub fn lcm(a: i64, b: i64) -> i64 {
        if a == 0 || b == 0 {
            0
        } else {
            (a / Self::gcd(a, b)) * b
        }
    }
    
    // ==================== Rounding ====================
    
    /// Ceiling (round up)
    /// ceil(4.3) => 5.0
    pub fn ceil(n: f64) -> f64 {
        n.ceil()
    }
    
    /// Floor (round down)
    /// floor(4.7) => 4.0
    pub fn floor(n: f64) -> f64 {
        n.floor()
    }
    
    /// Round to nearest integer
    /// round(4.5) => 5.0 (banker's rounding)
    pub fn round(n: f64) -> f64 {
        n.round()
    }
    
    /// Truncate (remove decimal part)
    /// trunc(4.7) => 4.0
    pub fn trunc(n: f64) -> f64 {
        n.trunc()
    }
    
    /// Round to decimal places
    /// round_to(3.14159, 2) => 3.14
    pub fn round_to(n: f64, places: u32) -> f64 {
        let multiplier = 10f64.powi(places as i32);
        (n * multiplier).round() / multiplier
    }
    
    // ==================== Powers & Roots ====================
    
    /// Power (exponentiation)
    /// pow(2, 10) => 1024.0
    pub fn pow(base: f64, exponent: f64) -> f64 {
        base.powf(exponent)
    }
    
    /// Square root
    /// sqrt(16) => 4.0
    pub fn sqrt(n: f64) -> f64 {
        n.sqrt()
    }
    
    /// Cube root
    /// cbrt(27) => 3.0
    pub fn cbrt(n: f64) -> f64 {
        n.cbrt()
    }
    
    /// Nth root
    /// nthroot(16, 4) => 2.0
    pub fn nthroot(n: f64, root: f64) -> f64 {
        n.powf(1.0 / root)
    }
    
    /// Exponential (e^n)
    /// exp(1) => e ≈ 2.718
    pub fn exp(n: f64) -> f64 {
        n.exp()
    }
    
    /// Exponential base 2 (2^n)
    /// exp2(3) => 8.0
    pub fn exp2(n: f64) -> f64 {
        n.exp2()
    }
    
    /// Exponential base 10 (10^n)
    /// exp10(2) => 100.0
    pub fn exp10(n: f64) -> f64 {
        (10.0_f64).powf(n)
    }
    
    // ==================== Logarithms ====================
    
    /// Natural logarithm (base e)
    /// ln(e) => 1.0
    pub fn ln(n: f64) -> f64 {
        n.ln()
    }
    
    /// Logarithm base 10
    /// log10(100) => 2.0
    pub fn log10(n: f64) -> f64 {
        n.log10()
    }
    
    /// Logarithm base 2
    /// log2(8) => 3.0
    pub fn log2(n: f64) -> f64 {
        n.log2()
    }
    
    /// Logarithm with custom base
    /// log(100, 10) => 2.0
    pub fn log(n: f64, base: f64) -> f64 {
        n.log(base)
    }
    
    // ==================== Trigonometric ====================
    
    /// Sine (radians)
    /// sin(pi/2) => 1.0
    pub fn sin(radians: f64) -> f64 {
        radians.sin()
    }
    
    /// Cosine (radians)
    /// cos(0) => 1.0
    pub fn cos(radians: f64) -> f64 {
        radians.cos()
    }
    
    /// Tangent (radians)
    /// tan(pi/4) => 1.0
    pub fn tan(radians: f64) -> f64 {
        radians.tan()
    }
    
    /// Arcsine (returns radians)
    /// asin(1) => pi/2
    pub fn asin(x: f64) -> f64 {
        x.asin()
    }
    
    /// Arccosine (returns radians)
    /// acos(0) => pi/2
    pub fn acos(x: f64) -> f64 {
        x.acos()
    }
    
    /// Arctangent (returns radians)
    /// atan(1) => pi/4
    pub fn atan(x: f64) -> f64 {
        x.atan()
    }
    
    /// Two-argument arctangent (atan2)
    /// atan2(1, 1) => pi/4
    pub fn atan2(y: f64, x: f64) -> f64 {
        y.atan2(x)
    }
    
    /// Hyperbolic sine
    pub fn sinh(x: f64) -> f64 {
        x.sinh()
    }
    
    /// Hyperbolic cosine
    pub fn cosh(x: f64) -> f64 {
        x.cosh()
    }
    
    /// Hyperbolic tangent
    pub fn tanh(x: f64) -> f64 {
        x.tanh()
    }
    
    // ==================== Angle Conversion ====================
    
    /// Convert degrees to radians
    /// to_radians(180) => pi
    pub fn to_radians(degrees: f64) -> f64 {
        degrees.to_radians()
    }
    
    /// Convert radians to degrees
    /// to_degrees(pi) => 180.0
    pub fn to_degrees(radians: f64) -> f64 {
        radians.to_degrees()
    }
    
    // ==================== Random Numbers ====================
    
    /// Random float between 0.0 and 1.0
    /// random() => 0.723...
    pub fn random() -> f64 {
        // Simple LCG random number generator
        // Seeds with current time for unpredictability
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        
        // LCG parameters (glibc)
        let a: u64 = 1103515245;
        let c: u64 = 12345;
        let m: u64 = 2u64.pow(31);
        
        let state = ((seed.wrapping_mul(a).wrapping_add(c)) % m) as f64;
        state / m as f64
    }
    
    /// Random integer between min (inclusive) and max (exclusive)
    /// random_int(1, 10) => 5
    pub fn random_int(min: i64, max: i64) -> i64 {
        if min >= max { return min; }
        let range = (max - min) as f64;
        min + (Self::random() * range) as i64
    }
    
    /// Random float between min (inclusive) and max (exclusive)
    /// random_range(1.5, 2.5) => 2.1...
    pub fn random_range(min: f64, max: f64) -> f64 {
        if min >= max { return min; }
        min + Self::random() * (max - min)
    }
    
    // ==================== Special Functions ====================
    
    /// Fibonacci number at index
    /// fibonacci(10) => 55
    pub fn fibonacci(n: u32) -> u64 {
        match n {
            0 => 0,
            1 => 1,
            _ => {
                let mut a = 0u64;
                let mut b = 1u64;
                for _ in 2..=n {
                    let temp = a + b;
                    a = b;
                    b = temp;
                }
                b
            }
        }
    }
    
    /// Factorial
    /// factorial(5) => 120
    pub fn factorial(n: u32) -> u64 {
        (1..=n as u64).product()
    }
    
    /// Check if number is even
    pub fn is_even(n: i64) -> bool {
        n % 2 == 0
    }
    
    /// Check if number is odd
    pub fn is_odd(n: i64) -> bool {
        n % 2 != 0
    }
    
    /// Check if number is prime
    pub fn is_prime(n: u32) -> bool {
        if n < 2 { return false; }
        if n == 2 { return true; }
        if n % 2 == 0 { return false; }
        for i in (3..=((n as f64).sqrt() as u32)).step_by(2) {
            if n % i == 0 { return false; }
        }
        true
    }
    
    /// Check if number is perfect square
    pub fn is_perfect_square(n: u32) -> bool {
        let sqrt = (n as f64).sqrt() as u32;
        sqrt * sqrt == n
    }
    
    /// Check if number is NaN
    pub fn is_nan(n: f64) -> bool {
        n.is_nan()
    }
    
    /// Check if number is infinite
    pub fn is_infinite(n: f64) -> bool {
        n.is_infinite()
    }
    
    /// Check if number is finite
    pub fn is_finite(n: f64) -> bool {
        n.is_finite()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        assert_eq!(MathModule::abs(-5.0), 5.0);
        assert_eq!(MathModule::min(5.0, 3.0), 3.0);
        assert_eq!(MathModule::max(5.0, 3.0), 5.0);
        assert_eq!(MathModule::sign(-5.0), -1.0);
        assert_eq!(MathModule::clamp(5.0, 0.0, 3.0), 3.0);
    }

    #[test]
    fn test_rounding() {
        assert_eq!(MathModule::ceil(4.3), 5.0);
        assert_eq!(MathModule::floor(4.7), 4.0);
        assert_eq!(MathModule::round(4.5), 5.0); // rounds half away from zero
        assert_eq!(MathModule::trunc(4.7), 4.0);
    }

    #[test]
    fn test_powers() {
        assert_eq!(MathModule::pow(2.0, 10.0), 1024.0);
        assert_eq!(MathModule::sqrt(16.0), 4.0);
        assert_eq!(MathModule::cbrt(27.0), 3.0);
    }

    #[test]
    fn test_logarithms() {
        assert!((MathModule::ln(constants::E) - 1.0).abs() < 1e-10);
        assert_eq!(MathModule::log10(100.0), 2.0);
        assert_eq!(MathModule::log2(8.0), 3.0);
    }

    #[test]
    fn test_trigonometric() {
        assert!(MathModule::sin(constants::PI / 2.0) - 1.0 < 1e-10);
        assert!(MathModule::cos(0.0) - 1.0 < 1e-10);
        assert!((MathModule::tan(constants::PI / 4.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_gcd_lcm() {
        assert_eq!(MathModule::gcd(48, 18), 6);
        assert_eq!(MathModule::lcm(12, 18), 36);
    }

    #[test]
    fn test_special_functions() {
        assert_eq!(MathModule::fibonacci(10), 55);
        assert_eq!(MathModule::factorial(5), 120);
        assert!(MathModule::is_even(4));
        assert!(MathModule::is_odd(5));
        assert!(MathModule::is_prime(13));
        assert!(MathModule::is_perfect_square(16));
    }

    #[test]
    fn test_random() {
        let r = MathModule::random();
        assert!(r >= 0.0 && r <= 1.0);
        
        let ri = MathModule::random_int(1, 10);
        assert!(ri >= 1 && ri < 10);
    }

    #[test]
    fn test_constants() {
        assert!((constants::PI - 3.14159265359).abs() < 1e-10);
        assert!((constants::E - 2.71828182846).abs() < 1e-10);
    }
}
