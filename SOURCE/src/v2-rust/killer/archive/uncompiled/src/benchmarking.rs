// Benchmarking Framework for Killer Standard Library
// Comprehensive performance measurement and analysis
// Version: 2.1.0

use std::time::Instant;
use std::collections::HashMap;

/// Benchmark result data
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub name: String,
    pub iterations: u64,
    pub total_time_ns: u128,
    pub avg_time_ns: f64,
    pub min_time_ns: u128,
    pub max_time_ns: u128,
    pub throughput: f64,  // ops/second
}

impl BenchmarkResult {
    pub fn format(&self) -> String {
        format!(
            "{}: {} iterations, avg {:.2}ns, throughput {:.0} ops/sec",
            self.name,
            self.iterations,
            self.avg_time_ns,
            self.throughput
        )
    }
}

/// Benchmarking module with common utilities
pub struct BenchmarkModule;

impl BenchmarkModule {
    /// Run a simple benchmark
    /// Returns (iterations completed, time in nanoseconds)
    pub fn quick_bench<F>(name: &str, mut f: F, target_time_ms: u64) -> BenchmarkResult
    where
        F: FnMut(),
    {
        let target_ns = (target_time_ms as u128) * 1_000_000;
        let mut iterations = 0u64;
        
        let start = Instant::now();
        let mut elapsed_ns = 0u128;
        
        while elapsed_ns < target_ns {
            f();
            iterations += 1;
            elapsed_ns = start.elapsed().as_nanos();
        }
        
        let throughput = iterations as f64 / (elapsed_ns as f64 / 1e9);
        
        BenchmarkResult {
            name: name.to_string(),
            iterations,
            total_time_ns: elapsed_ns,
            avg_time_ns: elapsed_ns as f64 / iterations as f64,
            min_time_ns: 0,
            max_time_ns: 0,
            throughput,
        }
    }
    
    /// Detailed benchmark with warm-up and statistics
    pub fn detailed_bench<F>(
        name: &str,
        mut f: F,
        iterations: u64,
        warmup: u64,
    ) -> BenchmarkResult
    where
        F: FnMut(),
    {
        // Warmup phase
        for _ in 0..warmup {
            f();
        }
        
        // Measurement phase
        let mut times = Vec::new();
        for _ in 0..iterations {
            let start = Instant::now();
            f();
            times.push(start.elapsed().as_nanos());
        }
        
        let total_time_ns: u128 = times.iter().sum();
        let avg_time_ns = total_time_ns as f64 / iterations as f64;
        let min_time_ns = *times.iter().min().unwrap_or(&(0u128));
        let max_time_ns = *times.iter().max().unwrap_or(&(0u128));
        let throughput = iterations as f64 / (total_time_ns as f64 / 1e9);
        
        BenchmarkResult {
            name: name.to_string(),
            iterations,
            total_time_ns,
            avg_time_ns,
            min_time_ns,
            max_time_ns,
            throughput,
        }
    }
    
    /// Comparative benchmarking
    /// Run two operations and compare
    pub fn compare<F, G>(
        name_a: &str,
        f_a: F,
        name_b: &str,
        f_b: G,
        iterations: u64,
    ) -> (BenchmarkResult, BenchmarkResult, f64)
    where
        F: FnMut(),
        G: FnMut(),
    {
        let bench_a = Self::detailed_bench(name_a, f_a, iterations, iterations / 10);
        let bench_b = Self::detailed_bench(name_b, f_b, iterations, iterations / 10);
        
        let ratio = bench_a.avg_time_ns / bench_b.avg_time_ns;
        
        (bench_a, bench_b, ratio)
    }
    
    /// Format benchmark results for display
    pub fn format_results(results: &[BenchmarkResult]) -> String {
        let mut output = String::from("=== Benchmark Results ===\n");
        
        for result in results {
            output.push_str(&format!("{}\n", result.format()));
        }
        
        output
    }
    
    /// Analyze multiple results
    pub fn analyze(results: &[BenchmarkResult]) -> HashMap<String, f64> {
        let mut stats = HashMap::new();
        
        for result in results {
            stats.insert(format!("{}_avg_ns", result.name), result.avg_time_ns);
            stats.insert(format!("{}_throughput", result.name), result.throughput);
        }
        
        stats
    }
}

/// STDLIB Benchmarks - Compare against baseline
pub struct StdlibBenchmarks;

impl StdlibBenchmarks {
    /// Benchmark Math module operations
    pub fn bench_math() -> Vec<BenchmarkResult> {
        use crate::math::MathModule;
        
        vec![
            BenchmarkModule::quick_bench("math_add", || {
                let _ = 2.0 + 3.0;
            }, 100),
            
            BenchmarkModule::quick_bench("math_sqrt", || {
                let _ = MathModule::sqrt(16.0);
            }, 100),
            
            BenchmarkModule::quick_bench("math_sin", || {
                let _ = MathModule::sin(1.57);
            }, 100),
            
            BenchmarkModule::quick_bench("math_pow", || {
                let _ = MathModule::pow(2.0, 10.0);
            }, 100),
        ]
    }
    
    /// Benchmark String module operations
    pub fn bench_string() -> Vec<BenchmarkResult> {
        use crate::string_utils::StringModule;
        
        let test_str = "Hello, World!";
        let test_str2 = "The quick brown fox jumps over the lazy dog";
        
        vec![
            BenchmarkModule::quick_bench("string_uppercase", || {
                let _ = StringModule::uppercase(test_str);
            }, 100),
            
            BenchmarkModule::quick_bench("string_contains", || {
                let _ = StringModule::contains(test_str2, "fox");
            }, 100),
            
            BenchmarkModule::quick_bench("string_replace", || {
                let _ = StringModule::replace_all(test_str2, "o", "0");
            }, 100),
            
            BenchmarkModule::quick_bench("string_split", || {
                let _ = StringModule::split("a,b,c,d,e", ",");
            }, 100),
        ]
    }
    
    /// Benchmark Array module operations
    pub fn bench_array() -> Vec<BenchmarkResult> {
        use crate::value::Value;
        use crate::array_utils::ArrayModule;
        
        let arr = vec![
            Value::Number(3.0),
            Value::Number(1.0),
            Value::Number(4.0),
            Value::Number(1.0),
            Value::Number(5.0),
        ];
        let arr_clone = arr.clone();
        let arr_clone2 = arr.clone();
        let arr_clone3 = arr.clone();
        
        vec![
            BenchmarkModule::quick_bench("array_length", || {
                let _ = ArrayModule::length(&arr);
            }, 100),
            
            BenchmarkModule::quick_bench("array_sort", || {
                let _ = ArrayModule::sort(&arr_clone);
            }, 100),
            
            BenchmarkModule::quick_bench("array_contains", || {
                let _ = ArrayModule::contains(&arr_clone2, &Value::Number(3.0));
            }, 100),
            
            BenchmarkModule::quick_bench("array_sum", || {
                let _ = ArrayModule::sum(&arr_clone3);
            }, 100),
        ]
    }
    
    /// Benchmark JSON operations
    pub fn bench_json() -> Vec<BenchmarkResult> {
        use crate::json_module::JsonModule;
        
        let json_str = r#"{"name": "Alice", "age": 30, "city": "Toronto"}"#;
        let complex_json = r#"[{"id": 1, "name": "Alice"}, {"id": 2, "name": "Bob"}]"#;
        
        let json_str_clone = json_str.to_string();
        let json_str_clone2 = json_str.to_string();
        
        vec![
            BenchmarkModule::quick_bench("json_parse", || {
                let _ = JsonModule::parse(json_str);
            }, 100),
            
            BenchmarkModule::quick_bench("json_is_valid", || {
                let _ = JsonModule::is_valid(&json_str_clone);
            }, 100),
            
            BenchmarkModule::quick_bench("json_parse_complex", || {
                let _ = JsonModule::parse(complex_json);
            }, 100),
        ]
    }
    
    /// Benchmark Type module operations
    pub fn bench_types() -> Vec<BenchmarkResult> {
        use crate::value::Value;
        use crate::types_module::TypeModule;
        
        let num = Value::Number(42.0);
        let str_val = Value::Str("hello".to_string());
        
        vec![
            BenchmarkModule::quick_bench("types_typeof", || {
                let _ = TypeModule::typeof_value(&num);
            }, 100),
            
            BenchmarkModule::quick_bench("types_to_number", || {
                let _ = TypeModule::to_number(&str_val);
            }, 100),
            
            BenchmarkModule::quick_bench("types_equals", || {
                let _ = TypeModule::equals(&num, &Value::Number(42.0));
            }, 100),
        ]
    }
    
    /// Benchmark DateTime operations
    pub fn bench_datetime() -> Vec<BenchmarkResult> {
        use crate::datetime_module::DateTimeModule;
        
        vec![
            BenchmarkModule::quick_bench("datetime_now", || {
                let _ = DateTimeModule::now();
            }, 100),
            
            BenchmarkModule::quick_bench("datetime_format_iso", || {
                let _ = DateTimeModule::format_iso(1710288000);
            }, 100),
            
            BenchmarkModule::quick_bench("datetime_add_days", || {
                let dt = DateTimeModule::from_timestamp(1710288000);
                let _ = DateTimeModule::add_days(&dt, 1);
            }, 100),
        ]
    }
    
    /// Benchmark Logging operations
    pub fn bench_logging() -> Vec<BenchmarkResult> {
        use crate::logging_module::{Logger, LogLevel};
        
        let logger = Logger::new(LogLevel::Debug);
        
        vec![
            BenchmarkModule::quick_bench("logging_debug", || {
                logger.debug("Test message");
            }, 100),
            
            BenchmarkModule::quick_bench("logging_search", || {
                let _ = logger.search("Test");
            }, 100),
        ]
    }
    
    /// Benchmark Regex operations
    pub fn bench_regex() -> Vec<BenchmarkResult> {
        use crate::regex_module::RegexModule;
        
        let text = "The quick brown fox jumps over the lazy dog";
        
        vec![
            BenchmarkModule::quick_bench("regex_find", || {
                let _ = RegexModule::find(text, "fox");
            }, 100),
            
            BenchmarkModule::quick_bench("regex_contains", || {
                let _ = RegexModule::contains(text, "brown");
            }, 100),
            
            BenchmarkModule::quick_bench("regex_split", || {
                let _ = RegexModule::split(text, " ");
            }, 100),
        ]
    }
    
    /// Benchmark Compression operations
    pub fn bench_compression() -> Vec<BenchmarkResult> {
        use crate::compression_module::CompressionModule;
        
        let text = "aaabbbccccdddddeeeeeeeffffffff";
        let text2 = "Hello, World! This is a test message.";
        
        vec![
            BenchmarkModule::quick_bench("compression_rle_encode", || {
                let _ = CompressionModule::rle_encode(text);
            }, 100),
            
            BenchmarkModule::quick_bench("compression_hex_encode", || {
                let _ = CompressionModule::hex_encode(text2);
            }, 100),
            
            BenchmarkModule::quick_bench("compression_base64_encode", || {
                let _ = CompressionModule::base64_encode(text2);
            }, 100),
        ]
    }
    
    /// Run all benchmarks
    pub fn run_all() -> HashMap<String, Vec<BenchmarkResult>> {
        let mut results = HashMap::new();
        
        results.insert("Math".to_string(), Self::bench_math());
        results.insert("String".to_string(), Self::bench_string());
        results.insert("Array".to_string(), Self::bench_array());
        results.insert("JSON".to_string(), Self::bench_json());
        results.insert("Types".to_string(), Self::bench_types());
        results.insert("DateTime".to_string(), Self::bench_datetime());
        results.insert("Logging".to_string(), Self::bench_logging());
        results.insert("Regex".to_string(), Self::bench_regex());
        results.insert("Compression".to_string(), Self::bench_compression());
        
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_quick_bench() {
        let result = BenchmarkModule::quick_bench("test", || {
            let _ = 2 + 2;
        }, 10);
        
        assert!(result.iterations > 0);
        assert!(result.avg_time_ns > 0.0);
    }
    
    #[test]
    fn test_detailed_bench() {
        let result = BenchmarkModule::detailed_bench("test", || {
            let _ = 3 * 4;
        }, 100, 10);
        
        assert_eq!(result.iterations, 100);
        assert!(result.avg_time_ns > 0.0);
    }
    
    #[test]
    fn test_benchmark_format() {
        let result = BenchmarkResult {
            name: "test".to_string(),
            iterations: 1000,
            total_time_ns: 100000,
            avg_time_ns: 100.0,
            min_time_ns: 50,
            max_time_ns: 200,
            throughput: 1e7,
        };
        
        let formatted = result.format();
        assert!(formatted.contains("test"));
        assert!(formatted.contains("1000"));
    }
}
