// Phase 16: Testing Framework - unit testing, property testing, benchmarking
// Features: Test runners, assertions, benchmarks, property-based testing, coverage

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Test result status
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TestResult {
    Passed,
    Failed(String),
    Skipped(String),
    Ignored,
}

impl TestResult {
    pub fn as_str(&self) -> &str {
        match self {
            TestResult::Passed => "passed",
            TestResult::Failed(_) => "failed",
            TestResult::Skipped(_) => "skipped",
            TestResult::Ignored => "ignored",
        }
    }

    pub fn is_passed(&self) -> bool {
        *self == TestResult::Passed
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, TestResult::Failed(_))
    }
}

/// Test case
#[derive(Clone, Debug)]
pub struct TestCase {
    pub name: String,
    pub description: String,
    pub result: TestResult,
    pub duration_ms: u64,
    pub assertions: u32,
}

impl TestCase {
    pub fn new(name: String) -> Self {
        TestCase {
            name,
            description: String::new(),
            result: TestResult::Passed,
            duration_ms: 0,
            assertions: 0,
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: String) -> Self {
        self.description = desc;
        self
    }

    /// Set result
    pub fn with_result(mut self, result: TestResult) -> Self {
        self.result = result;
        self
    }

    /// Set duration
    pub fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = ms;
        self
    }

    /// Increment assertions
    pub fn increment_assertions(mut self) -> Self {
        self.assertions += 1;
        self
    }

    /// Get status
    pub fn status(&self) -> &str {
        self.result.as_str()
    }
}

/// Test suite
#[derive(Clone, Debug)]
pub struct TestSuite {
    pub name: String,
    pub tests: Vec<TestCase>,
}

impl TestSuite {
    pub fn new(name: String) -> Self {
        TestSuite {
            name,
            tests: Vec::new(),
        }
    }

    /// Add test
    pub fn add_test(&mut self, test: TestCase) {
        self.tests.push(test);
    }

    /// Run suite
    pub fn run(&mut self) -> TestSuiteResult {
        let total = self.tests.len();
        let passed = self.tests.iter().filter(|t| t.result == TestResult::Passed).count();
        let failed = self.tests.iter().filter(|t| t.result.is_failed()).count();
        let skipped = self.tests.iter().filter(|t| matches!(t.result, TestResult::Skipped(_))).count();

        TestSuiteResult {
            suite_name: self.name.clone(),
            total_tests: total,
            passed,
            failed,
            skipped,
            duration_ms: self.tests.iter().map(|t| t.duration_ms).sum(),
            timestamp: current_timestamp(),
        }
    }

    /// Get pass rate
    pub fn pass_rate(&self) -> f32 {
        if self.tests.is_empty() {
            0.0
        } else {
            let passed = self.tests.iter().filter(|t| t.result == TestResult::Passed).count();
            (passed as f32 / self.tests.len() as f32) * 100.0
        }
    }

    /// Test count
    pub fn test_count(&self) -> usize {
        self.tests.len()
    }

    /// Get failed tests
    pub fn get_failed_tests(&self) -> Vec<TestCase> {
        self.tests.iter()
            .filter(|t| t.result.is_failed())
            .cloned()
            .collect()
    }
}

/// Test suite result
#[derive(Clone, Debug)]
pub struct TestSuiteResult {
    pub suite_name: String,
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub duration_ms: u64,
    pub timestamp: u64,
}

impl TestSuiteResult {
    /// Get success rate
    pub fn success_rate(&self) -> f32 {
        if self.total_tests == 0 {
            0.0
        } else {
            (self.passed as f32 / self.total_tests as f32) * 100.0
        }
    }

    /// Is successful
    pub fn is_successful(&self) -> bool {
        self.failed == 0
    }
}

/// Test runner
#[derive(Clone, Debug)]
pub struct TestRunner {
    pub suites: HashMap<String, TestSuite>,
    pub results: Vec<TestSuiteResult>,
}

impl TestRunner {
    pub fn new() -> Self {
        TestRunner {
            suites: HashMap::new(),
            results: Vec::new(),
        }
    }

    /// Register suite
    pub fn register_suite(&mut self, suite: TestSuite) -> Result<(), String> {
        if self.suites.contains_key(&suite.name) {
            return Err(format!("Suite {} already registered", suite.name));
        }
        self.suites.insert(suite.name.clone(), suite);
        Ok(())
    }

    /// Run suite
    pub fn run_suite(&mut self, suite_name: &str) -> Result<TestSuiteResult, String> {
        let mut suite = self.suites.get(suite_name)
            .ok_or_else(|| format!("Suite {} not found", suite_name))?
            .clone();

        let result = suite.run();
        self.results.push(result.clone());
        Ok(result)
    }

    /// Run all suites
    pub fn run_all(&mut self) -> Vec<TestSuiteResult> {
        let suite_names: Vec<String> = self.suites.keys().cloned().collect();
        for name in suite_names {
            if let Ok(result) = self.run_suite(&name) {
                // Result already added in run_suite
            }
        }
        self.results.clone()
    }

    /// Suite count
    pub fn suite_count(&self) -> usize {
        self.suites.len()
    }

    /// Get total stats
    pub fn get_total_stats(&self) -> (usize, usize, usize, usize) {
        let total: usize = self.results.iter().map(|r| r.total_tests).sum();
        let passed: usize = self.results.iter().map(|r| r.passed).sum();
        let failed: usize = self.results.iter().map(|r| r.failed).sum();
        let skipped: usize = self.results.iter().map(|r| r.skipped).sum();
        (total, passed, failed, skipped)
    }

    /// Get overall success rate
    pub fn overall_success_rate(&self) -> f32 {
        let (total, passed, _, _) = self.get_total_stats();
        if total == 0 {
            0.0
        } else {
            (passed as f32 / total as f32) * 100.0
        }
    }
}

impl Default for TestRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Benchmark result
#[derive(Clone, Debug)]
pub struct BenchmarkResult {
    pub name: String,
    pub iterations: u32,
    pub total_time_ns: u64,
    pub min_time_ns: u64,
    pub max_time_ns: u64,
    pub avg_time_ns: f64,
    pub throughput: f64, // iterations per second
}

impl BenchmarkResult {
    pub fn new(name: String, iterations: u32, total_time_ns: u64) -> Self {
        let avg = total_time_ns as f64 / iterations as f64;
        let throughput = (iterations as f64 / (total_time_ns as f64 / 1_000_000_000.0)).max(0.0);

        BenchmarkResult {
            name,
            iterations,
            total_time_ns,
            min_time_ns: u64::MAX,
            max_time_ns: 0,
            avg_time_ns: avg,
            throughput,
        }
    }

    /// Update min/max
    pub fn update_bounds(mut self, time_ns: u64) -> Self {
        self.min_time_ns = self.min_time_ns.min(time_ns);
        self.max_time_ns = self.max_time_ns.max(time_ns);
        self
    }

    /// Get average in milliseconds
    pub fn avg_ms(&self) -> f64 {
        self.avg_time_ns / 1_000_000.0
    }
}

/// Benchmarker
#[derive(Clone, Debug)]
pub struct Benchmarker {
    pub results: HashMap<String, BenchmarkResult>,
}

impl Benchmarker {
    pub fn new() -> Self {
        Benchmarker {
            results: HashMap::new(),
        }
    }

    /// Start benchmark
    pub fn start_benchmark(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }

    /// Record benchmark
    pub fn record(&mut self, name: String, iterations: u32, elapsed_ns: u64) {
        let result = BenchmarkResult::new(name.clone(), iterations, elapsed_ns);
        self.results.insert(name, result);
    }

    /// Get result
    pub fn get_result(&self, name: &str) -> Option<BenchmarkResult> {
        self.results.get(name).cloned()
    }

    /// List all results
    pub fn list_results(&self) -> Vec<BenchmarkResult> {
        self.results.values().cloned().collect()
    }

    /// Get fastest
    pub fn get_fastest(&self) -> Option<BenchmarkResult> {
        self.results.values()
            .min_by(|a, b| a.avg_time_ns.partial_cmp(&b.avg_time_ns).unwrap_or(std::cmp::Ordering::Equal))
            .cloned()
    }

    /// Get slowest
    pub fn get_slowest(&self) -> Option<BenchmarkResult> {
        self.results.values()
            .max_by(|a, b| a.avg_time_ns.partial_cmp(&b.avg_time_ns).unwrap_or(std::cmp::Ordering::Equal))
            .cloned()
    }
}

impl Default for Benchmarker {
    fn default() -> Self {
        Self::new()
    }
}

/// Property test case
#[derive(Clone, Debug)]
pub struct PropertyTestCase {
    pub name: String,
    pub iterations: u32,
    pub passed: u32,
    pub failed: u32,
}

impl PropertyTestCase {
    pub fn new(name: String, iterations: u32) -> Self {
        PropertyTestCase {
            name,
            iterations,
            passed: 0,
            failed: 0,
        }
    }

    /// Record pass
    pub fn record_pass(mut self) -> Self {
        self.passed += 1;
        self
    }

    /// Record failure
    pub fn record_failure(mut self) -> Self {
        self.failed += 1;
        self
    }

    /// Get pass rate
    pub fn pass_rate(&self) -> f32 {
        if self.iterations == 0 {
            0.0
        } else {
            (self.passed as f32 / self.iterations as f32) * 100.0
        }
    }

    /// Is successful
    pub fn is_successful(&self) -> bool {
        self.failed == 0 && self.passed == self.iterations
    }
}

/// Property-based test runner
#[derive(Clone, Debug)]
pub struct PropertyTestRunner {
    pub test_cases: HashMap<String, PropertyTestCase>,
}

impl PropertyTestRunner {
    pub fn new() -> Self {
        PropertyTestRunner {
            test_cases: HashMap::new(),
        }
    }

    /// Register test case
    pub fn register(&mut self, test_case: PropertyTestCase) {
        self.test_cases.insert(test_case.name.clone(), test_case);
    }

    /// Get test case
    pub fn get_test_case(&self, name: &str) -> Option<PropertyTestCase> {
        self.test_cases.get(name).cloned()
    }

    /// List all tests
    pub fn list_tests(&self) -> Vec<PropertyTestCase> {
        self.test_cases.values().cloned().collect()
    }

    /// Get success rate
    pub fn overall_success_rate(&self) -> f32 {
        if self.test_cases.is_empty() {
            0.0
        } else {
            let successful = self.test_cases.values()
                .filter(|t| t.is_successful())
                .count();
            (successful as f32 / self.test_cases.len() as f32) * 100.0
        }
    }
}

impl Default for PropertyTestRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Code coverage
#[derive(Clone, Debug)]
pub struct CodeCoverage {
    pub total_lines: u32,
    pub covered_lines: u32,
    pub total_branches: u32,
    pub covered_branches: u32,
    pub file_coverage: HashMap<String, f32>,
}

impl CodeCoverage {
    pub fn new(total_lines: u32, covered_lines: u32) -> Self {
        CodeCoverage {
            total_lines,
            covered_lines,
            total_branches: 0,
            covered_branches: 0,
            file_coverage: HashMap::new(),
        }
    }

    /// Get line coverage percentage
    pub fn line_coverage(&self) -> f32 {
        if self.total_lines == 0 {
            0.0
        } else {
            (self.covered_lines as f32 / self.total_lines as f32) * 100.0
        }
    }

    /// Get branch coverage percentage
    pub fn branch_coverage(&self) -> f32 {
        if self.total_branches == 0 {
            0.0
        } else {
            (self.covered_branches as f32 / self.total_branches as f32) * 100.0
        }
    }

    /// Add file coverage
    pub fn add_file_coverage(mut self, file: String, coverage: f32) -> Self {
        self.file_coverage.insert(file, coverage);
        self
    }

    /// Get overall coverage
    pub fn overall_coverage(&self) -> f32 {
        (self.line_coverage() + self.branch_coverage()) / 2.0
    }
}

/// Helper to get current timestamp
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_result_as_str() {
        assert_eq!(TestResult::Passed.as_str(), "passed");
        assert_eq!(TestResult::Failed("error".to_string()).as_str(), "failed");
    }

    #[test]
    fn test_test_result_is_passed() {
        assert!(TestResult::Passed.is_passed());
        assert!(!TestResult::Failed("error".to_string()).is_passed());
    }

    #[test]
    fn test_test_case_creation() {
        let test = TestCase::new("test_addition".to_string());
        assert_eq!(test.name, "test_addition");
        assert_eq!(test.result, TestResult::Passed);
    }

    #[test]
    fn test_test_case_with_description() {
        let test = TestCase::new("test".to_string())
            .with_description("Test description".to_string());
        assert_eq!(test.description, "Test description");
    }

    #[test]
    fn test_test_case_with_result() {
        let test = TestCase::new("test".to_string())
            .with_result(TestResult::Failed("assertion failed".to_string()));
        assert!(test.result.is_failed());
    }

    #[test]
    fn test_test_case_with_duration() {
        let test = TestCase::new("test".to_string()).with_duration(100);
        assert_eq!(test.duration_ms, 100);
    }

    #[test]
    fn test_test_case_increment_assertions() {
        let test = TestCase::new("test".to_string())
            .increment_assertions()
            .increment_assertions();
        assert_eq!(test.assertions, 2);
    }

    #[test]
    fn test_test_suite_creation() {
        let suite = TestSuite::new("suite1".to_string());
        assert_eq!(suite.name, "suite1");
        assert_eq!(suite.test_count(), 0);
    }

    #[test]
    fn test_test_suite_add_test() {
        let mut suite = TestSuite::new("suite1".to_string());
        suite.add_test(TestCase::new("test1".to_string()));
        suite.add_test(TestCase::new("test2".to_string()));
        assert_eq!(suite.test_count(), 2);
    }

    #[test]
    fn test_test_suite_run() {
        let mut suite = TestSuite::new("suite1".to_string());
        suite.add_test(TestCase::new("test1".to_string()));
        suite.add_test(TestCase::new("test2".to_string()));
        let result = suite.run();
        assert_eq!(result.total_tests, 2);
        assert_eq!(result.passed, 2);
    }

    #[test]
    fn test_test_suite_pass_rate() {
        let mut suite = TestSuite::new("suite1".to_string());
        suite.add_test(TestCase::new("test1".to_string()));
        suite.add_test(TestCase::new("test2".to_string())
            .with_result(TestResult::Failed("error".to_string())));
        assert_eq!(suite.pass_rate(), 50.0);
    }

    #[test]
    fn test_test_runner_register_suite() {
        let mut runner = TestRunner::new();
        let suite = TestSuite::new("suite1".to_string());
        assert!(runner.register_suite(suite).is_ok());
        assert_eq!(runner.suite_count(), 1);
    }

    #[test]
    fn test_test_runner_run_suite() {
        let mut runner = TestRunner::new();
        let suite = TestSuite::new("suite1".to_string());
        runner.register_suite(suite).unwrap();
        let result = runner.run_suite("suite1");
        assert!(result.is_ok());
    }

    #[test]
    fn test_test_suite_result_is_successful() {
        let result = TestSuiteResult {
            suite_name: "suite1".to_string(),
            total_tests: 10,
            passed: 10,
            failed: 0,
            skipped: 0,
            duration_ms: 100,
            timestamp: 0,
        };
        assert!(result.is_successful());
    }

    #[test]
    fn test_test_suite_result_success_rate() {
        let result = TestSuiteResult {
            suite_name: "suite1".to_string(),
            total_tests: 10,
            passed: 8,
            failed: 2,
            skipped: 0,
            duration_ms: 100,
            timestamp: 0,
        };
        assert_eq!(result.success_rate(), 80.0);
    }

    #[test]
    fn test_benchmark_result_creation() {
        let bench = BenchmarkResult::new("operation".to_string(), 1000, 1_000_000);
        assert_eq!(bench.iterations, 1000);
    }

    #[test]
    fn test_benchmark_result_avg_ms() {
        let bench = BenchmarkResult::new("operation".to_string(), 100, 100_000_000);
        assert_eq!(bench.avg_ms(), 1.0);
    }

    #[test]
    fn test_benchmarker_record() {
        let mut benchmarker = Benchmarker::new();
        benchmarker.record("test".to_string(), 1000, 1_000_000);
        assert_eq!(benchmarker.results.len(), 1);
    }

    #[test]
    fn test_benchmarker_get_result() {
        let mut benchmarker = Benchmarker::new();
        benchmarker.record("test".to_string(), 1000, 1_000_000);
        let result = benchmarker.get_result("test");
        assert!(result.is_some());
    }

    #[test]
    fn test_benchmarker_get_fastest() {
        let mut benchmarker = Benchmarker::new();
        benchmarker.record("slow".to_string(), 100, 10_000_000);
        benchmarker.record("fast".to_string(), 100, 1_000_000);
        let fastest = benchmarker.get_fastest();
        assert_eq!(fastest.unwrap().name, "fast");
    }

    #[test]
    fn test_benchmarker_get_slowest() {
        let mut benchmarker = Benchmarker::new();
        benchmarker.record("slow".to_string(), 100, 10_000_000);
        benchmarker.record("fast".to_string(), 100, 1_000_000);
        let slowest = benchmarker.get_slowest();
        assert_eq!(slowest.unwrap().name, "slow");
    }

    #[test]
    fn test_property_test_case_creation() {
        let test = PropertyTestCase::new("fuzzing".to_string(), 1000);
        assert_eq!(test.iterations, 1000);
        assert_eq!(test.passed, 0);
    }

    #[test]
    fn test_property_test_case_record() {
        let mut test = PropertyTestCase::new("fuzzing".to_string(), 3);
        test = test.record_pass().record_pass().record_failure();
        assert_eq!(test.passed, 2);
        assert_eq!(test.failed, 1);
    }

    #[test]
    fn test_property_test_case_pass_rate() {
        let mut test = PropertyTestCase::new("fuzzing".to_string(), 100);
        for _ in 0..90 {
            test = test.record_pass();
        }
        for _ in 0..10 {
            test = test.record_failure();
        }
        assert_eq!(test.pass_rate(), 90.0);
    }

    #[test]
    fn test_property_test_case_is_successful() {
        let mut test = PropertyTestCase::new("fuzzing".to_string(), 2);
        test = test.record_pass().record_pass();
        assert!(test.is_successful());
    }

    #[test]
    fn test_property_test_runner_register() {
        let mut runner = PropertyTestRunner::new();
        let test = PropertyTestCase::new("test".to_string(), 100);
        runner.register(test);
        assert_eq!(runner.test_cases.len(), 1);
    }

    #[test]
    fn test_property_test_runner_overall_success_rate() {
        let mut runner = PropertyTestRunner::new();
        let mut test1 = PropertyTestCase::new("test1".to_string(), 2);
        test1 = test1.record_pass().record_pass();
        
        let mut test2 = PropertyTestCase::new("test2".to_string(), 2);
        test2 = test2.record_pass().record_failure();
        
        runner.register(test1);
        runner.register(test2);
        assert_eq!(runner.overall_success_rate(), 50.0);
    }

    #[test]
    fn test_code_coverage_creation() {
        let coverage = CodeCoverage::new(100, 80);
        assert_eq!(coverage.total_lines, 100);
        assert_eq!(coverage.covered_lines, 80);
    }

    #[test]
    fn test_code_coverage_line_coverage() {
        let coverage = CodeCoverage::new(100, 75);
        assert_eq!(coverage.line_coverage(), 75.0);
    }

    #[test]
    fn test_code_coverage_branch_coverage() {
        let mut coverage = CodeCoverage::new(100, 75);
        coverage.total_branches = 50;
        coverage.covered_branches = 40;
        assert_eq!(coverage.branch_coverage(), 80.0);
    }

    #[test]
    fn test_code_coverage_overall_coverage() {
        let mut coverage = CodeCoverage::new(100, 80);
        coverage.total_branches = 50;
        coverage.covered_branches = 50;
        assert_eq!(coverage.overall_coverage(), 90.0);
    }

    #[test]
    fn test_code_coverage_add_file_coverage() {
        let coverage = CodeCoverage::new(100, 80)
            .add_file_coverage("main.rs".to_string(), 85.0);
        assert_eq!(coverage.file_coverage.get("main.rs"), Some(&85.0));
    }
}
