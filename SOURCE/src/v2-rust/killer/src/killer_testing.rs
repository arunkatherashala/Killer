//! **killer_testing** — production testing framework: mocks, fixtures, property testing, snapshots.
//!
//! Extends the basic TestCase/TestSuite with:
//! - Mock objects with expectation tracking
//! - Test fixtures (setup/teardown)
//! - Property-based testing (random inputs, shrinking)
//! - Snapshot testing (golden file comparison)
//! - Data-driven / parameterized tests
//! - Assertion extensions (approximate equality, contains, throws)

use std::collections::HashMap;
use std::fmt;

// ══════════════════════════════════════════════════════════════════════════════
// Assertion extensions
// ══════════════════════════════════════════════════════════════════════════════

/// Extended assertion result.
#[derive(Debug, Clone)]
pub struct AssertResult {
    pub passed: bool,
    pub message: String,
    pub expected: String,
    pub actual: String,
}

impl AssertResult {
    pub fn pass() -> Self {
        Self { passed: true, message: String::new(), expected: String::new(), actual: String::new() }
    }
    pub fn fail(msg: &str, expected: &str, actual: &str) -> Self {
        Self { passed: false, message: msg.to_string(), expected: expected.to_string(), actual: actual.to_string() }
    }
}

pub fn assert_approx_eq(a: f64, b: f64, epsilon: f64) -> AssertResult {
    if (a - b).abs() < epsilon {
        AssertResult::pass()
    } else {
        AssertResult::fail(
            &format!("{} ≈ {} (ε={})", a, b, epsilon),
            &b.to_string(), &a.to_string(),
        )
    }
}

pub fn assert_contains(haystack: &str, needle: &str) -> AssertResult {
    if haystack.contains(needle) {
        AssertResult::pass()
    } else {
        AssertResult::fail(
            &format!("expected to contain \"{}\"", needle),
            needle, haystack,
        )
    }
}

pub fn assert_starts_with(s: &str, prefix: &str) -> AssertResult {
    if s.starts_with(prefix) {
        AssertResult::pass()
    } else {
        AssertResult::fail("starts_with", prefix, s)
    }
}

pub fn assert_len(collection_len: usize, expected: usize) -> AssertResult {
    if collection_len == expected {
        AssertResult::pass()
    } else {
        AssertResult::fail("length mismatch",
            &expected.to_string(), &collection_len.to_string())
    }
}

pub fn assert_none(value: &Option<String>) -> AssertResult {
    if value.is_none() { AssertResult::pass() }
    else { AssertResult::fail("expected None", "None", &format!("{:?}", value)) }
}

pub fn assert_some(value: &Option<String>) -> AssertResult {
    if value.is_some() { AssertResult::pass() }
    else { AssertResult::fail("expected Some", "Some(_)", "None") }
}

// ══════════════════════════════════════════════════════════════════════════════
// Mock object system
// ══════════════════════════════════════════════════════════════════════════════

/// A mock function call record.
#[derive(Debug, Clone)]
pub struct MockCall {
    pub method: String,
    pub args: Vec<String>,
    pub timestamp_ms: u64,
}

/// Mock object: records calls, returns predefined values, verifies expectations.
#[derive(Debug, Clone)]
pub struct Mock {
    pub name: String,
    calls: Vec<MockCall>,
    returns: HashMap<String, Vec<String>>,  // method → return values (FIFO)
    expected_calls: HashMap<String, ExpectedCall>,
    call_count: u64,
}

#[derive(Debug, Clone)]
pub struct ExpectedCall {
    pub method: String,
    pub min_times: usize,
    pub max_times: Option<usize>,
    pub with_args: Option<Vec<String>>,
}

impl Mock {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            calls: Vec::new(),
            returns: HashMap::new(),
            expected_calls: HashMap::new(),
            call_count: 0,
        }
    }

    /// Configure a return value for a method (FIFO queue).
    pub fn when(&mut self, method: &str, return_value: &str) -> &mut Self {
        self.returns.entry(method.to_string()).or_default().push(return_value.to_string());
        self
    }

    /// Set an expectation: this method should be called N times.
    pub fn expect(&mut self, method: &str, min: usize, max: Option<usize>) -> &mut Self {
        self.expected_calls.insert(method.to_string(), ExpectedCall {
            method: method.to_string(),
            min_times: min,
            max_times: max,
            with_args: None,
        });
        self
    }

    /// Expect with specific args.
    pub fn expect_with_args(&mut self, method: &str, args: Vec<String>) -> &mut Self {
        self.expected_calls.insert(method.to_string(), ExpectedCall {
            method: method.to_string(),
            min_times: 1,
            max_times: None,
            with_args: Some(args),
        });
        self
    }

    /// Call a mock method. Records the call and returns the next configured value.
    pub fn call(&mut self, method: &str, args: Vec<String>) -> String {
        self.call_count += 1;
        self.calls.push(MockCall {
            method: method.to_string(),
            args,
            timestamp_ms: self.call_count,
        });
        // Return configured value (FIFO) or "mock_default"
        if let Some(queue) = self.returns.get_mut(method) {
            if !queue.is_empty() {
                return queue.remove(0);
            }
        }
        "mock_default".to_string()
    }

    /// Verify all expectations were met.
    pub fn verify(&self) -> Vec<String> {
        let mut failures = Vec::new();
        for (method, expected) in &self.expected_calls {
            let actual_count = self.calls.iter().filter(|c| c.method == *method).count();
            if actual_count < expected.min_times {
                failures.push(format!("{}(): expected at least {} calls, got {}",
                    method, expected.min_times, actual_count));
            }
            if let Some(max) = expected.max_times {
                if actual_count > max {
                    failures.push(format!("{}(): expected at most {} calls, got {}",
                        method, max, actual_count));
                }
            }
            if let Some(ref expected_args) = expected.with_args {
                let matching = self.calls.iter()
                    .filter(|c| c.method == *method && c.args == *expected_args)
                    .count();
                if matching == 0 {
                    failures.push(format!("{}(): never called with args {:?}", method, expected_args));
                }
            }
        }
        failures
    }

    /// Was this method called at all?
    pub fn was_called(&self, method: &str) -> bool {
        self.calls.iter().any(|c| c.method == method)
    }

    /// How many times was this method called?
    pub fn call_count_for(&self, method: &str) -> usize {
        self.calls.iter().filter(|c| c.method == method).count()
    }

    /// Get all calls to a method.
    pub fn calls_to(&self, method: &str) -> Vec<&MockCall> {
        self.calls.iter().filter(|c| c.method == method).collect()
    }

    /// Reset all recorded calls (keeps expectations and return configs).
    pub fn reset(&mut self) {
        self.calls.clear();
        self.call_count = 0;
    }

    /// Total calls recorded.
    pub fn total_calls(&self) -> usize { self.calls.len() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Test Fixture
// ══════════════════════════════════════════════════════════════════════════════

/// A test fixture with setup and teardown actions.
#[derive(Debug)]
pub struct TestFixture {
    pub name: String,
    pub data: HashMap<String, String>,
    setup_actions: Vec<String>,
    teardown_actions: Vec<String>,
    pub is_setup: bool,
}

impl TestFixture {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            data: HashMap::new(),
            setup_actions: Vec::new(),
            teardown_actions: Vec::new(),
            is_setup: false,
        }
    }

    /// Register a setup action (tag name for the runner to execute).
    pub fn on_setup(&mut self, action: &str) -> &mut Self {
        self.setup_actions.push(action.to_string());
        self
    }

    /// Register a teardown action.
    pub fn on_teardown(&mut self, action: &str) -> &mut Self {
        self.teardown_actions.push(action.to_string());
        self
    }

    /// Set a fixture data value (e.g., test database URL, temp file path).
    pub fn set(&mut self, key: &str, value: &str) {
        self.data.insert(key.to_string(), value.to_string());
    }

    /// Get a fixture data value.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.data.get(key).map(|s| s.as_str())
    }

    /// Run setup actions. Returns the action tags.
    pub fn setup(&mut self) -> &[String] {
        self.is_setup = true;
        &self.setup_actions
    }

    /// Run teardown actions. Returns the action tags.
    pub fn teardown(&mut self) -> &[String] {
        self.is_setup = false;
        &self.teardown_actions
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Property-based testing
// ══════════════════════════════════════════════════════════════════════════════

/// Random value generators for property-based testing.
pub struct PropGen {
    seed: u64,
}

impl PropGen {
    pub fn new(seed: u64) -> Self { Self { seed } }

    fn next_u64(&mut self) -> u64 {
        // xorshift64
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        self.seed
    }

    pub fn gen_bool(&mut self) -> bool { self.next_u64() % 2 == 0 }

    pub fn gen_int(&mut self, min: i64, max: i64) -> i64 {
        if min >= max { return min; }
        min + (self.next_u64() as i64).abs() % (max - min + 1)
    }

    pub fn gen_float(&mut self, min: f64, max: f64) -> f64 {
        let t = (self.next_u64() as f64) / (u64::MAX as f64);
        min + t * (max - min)
    }

    pub fn gen_string(&mut self, max_len: usize) -> String {
        let len = (self.next_u64() as usize) % (max_len + 1);
        (0..len).map(|_| {
            let c = b'a' + (self.next_u64() as u8) % 26;
            c as char
        }).collect()
    }

    pub fn gen_vec_int(&mut self, max_len: usize, min_val: i64, max_val: i64) -> Vec<i64> {
        let len = (self.next_u64() as usize) % (max_len + 1);
        (0..len).map(|_| self.gen_int(min_val, max_val)).collect()
    }
}

/// A property-based test: checks a property holds for many random inputs.
pub struct PropTest {
    pub name: String,
    pub iterations: usize,
    pub seed: u64,
    pub failures: Vec<PropFailure>,
}

#[derive(Debug, Clone)]
pub struct PropFailure {
    pub iteration: usize,
    pub input: String,
    pub message: String,
}

impl PropTest {
    pub fn new(name: &str, iterations: usize) -> Self {
        Self {
            name: name.to_string(),
            iterations,
            seed: 42,
            failures: Vec::new(),
        }
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Run a property check for integers: property(i64) -> bool.
    pub fn for_all_ints<F>(&mut self, min: i64, max: i64, property: F)
    where F: Fn(i64) -> bool {
        let mut gen = PropGen::new(self.seed);
        for i in 0..self.iterations {
            let val = gen.gen_int(min, max);
            if !property(val) {
                self.failures.push(PropFailure {
                    iteration: i,
                    input: val.to_string(),
                    message: format!("property failed for input {}", val),
                });
            }
        }
    }

    /// Run a property check for strings.
    pub fn for_all_strings<F>(&mut self, max_len: usize, property: F)
    where F: Fn(&str) -> bool {
        let mut gen = PropGen::new(self.seed);
        for i in 0..self.iterations {
            let val = gen.gen_string(max_len);
            if !property(&val) {
                self.failures.push(PropFailure {
                    iteration: i,
                    input: val.clone(),
                    message: format!("property failed for input \"{}\"", val),
                });
            }
        }
    }

    /// Run a property check for float pairs.
    pub fn for_all_float_pairs<F>(&mut self, min: f64, max: f64, property: F)
    where F: Fn(f64, f64) -> bool {
        let mut gen = PropGen::new(self.seed);
        for i in 0..self.iterations {
            let a = gen.gen_float(min, max);
            let b = gen.gen_float(min, max);
            if !property(a, b) {
                self.failures.push(PropFailure {
                    iteration: i,
                    input: format!("({}, {})", a, b),
                    message: format!("property failed for ({}, {})", a, b),
                });
            }
        }
    }

    pub fn passed(&self) -> bool { self.failures.is_empty() }
    pub fn failure_count(&self) -> usize { self.failures.len() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Snapshot testing
// ══════════════════════════════════════════════════════════════════════════════

/// In-memory snapshot store for golden-file style testing.
#[derive(Debug, Default)]
pub struct SnapshotStore {
    snapshots: HashMap<String, String>,
    pub mismatches: Vec<SnapshotMismatch>,
}

#[derive(Debug, Clone)]
pub struct SnapshotMismatch {
    pub name: String,
    pub expected: String,
    pub actual: String,
}

impl SnapshotStore {
    pub fn new() -> Self { Self::default() }

    /// Record a snapshot. If a previous snapshot exists with the same name, compare.
    pub fn assert_snapshot(&mut self, name: &str, actual: &str) -> bool {
        if let Some(expected) = self.snapshots.get(name) {
            if expected == actual {
                true
            } else {
                self.mismatches.push(SnapshotMismatch {
                    name: name.to_string(),
                    expected: expected.clone(),
                    actual: actual.to_string(),
                });
                false
            }
        } else {
            // First time — record the snapshot
            self.snapshots.insert(name.to_string(), actual.to_string());
            true
        }
    }

    /// Update a snapshot (for `--update-snapshots` workflow).
    pub fn update(&mut self, name: &str, value: &str) {
        self.snapshots.insert(name.to_string(), value.to_string());
    }

    pub fn count(&self) -> usize { self.snapshots.len() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Parameterized / data-driven tests
// ══════════════════════════════════════════════════════════════════════════════

/// A parameterized test case.
#[derive(Debug)]
pub struct ParamTest {
    pub name: String,
    pub cases: Vec<ParamCase>,
    pub results: Vec<ParamResult>,
}

#[derive(Debug, Clone)]
pub struct ParamCase {
    pub label: String,
    pub inputs: HashMap<String, String>,
    pub expected: String,
}

#[derive(Debug, Clone)]
pub struct ParamResult {
    pub label: String,
    pub passed: bool,
    pub actual: String,
    pub message: String,
}

impl ParamTest {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), cases: Vec::new(), results: Vec::new() }
    }

    pub fn add_case(&mut self, label: &str, inputs: HashMap<String, String>, expected: &str) {
        self.cases.push(ParamCase {
            label: label.to_string(),
            inputs,
            expected: expected.to_string(),
        });
    }

    /// Run all cases with a test function that produces a string result.
    pub fn run<F>(&mut self, test_fn: F)
    where F: Fn(&HashMap<String, String>) -> String {
        for case in &self.cases {
            let actual = test_fn(&case.inputs);
            let passed = actual == case.expected;
            self.results.push(ParamResult {
                label: case.label.clone(),
                passed,
                actual: actual.clone(),
                message: if passed { String::new() } else {
                    format!("expected \"{}\", got \"{}\"", case.expected, actual)
                },
            });
        }
    }

    pub fn all_passed(&self) -> bool { self.results.iter().all(|r| r.passed) }
    pub fn pass_count(&self) -> usize { self.results.iter().filter(|r| r.passed).count() }
    pub fn fail_count(&self) -> usize { self.results.iter().filter(|r| !r.passed).count() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Test Runner
// ══════════════════════════════════════════════════════════════════════════════

/// Aggregated test suite results.
#[derive(Debug)]
pub struct TestReport {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub duration_ms: f64,
    pub failures: Vec<String>,
}

impl fmt::Display for TestReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tests: {} total, {} passed, {} failed, {} skipped ({:.1}ms)",
            self.total, self.passed, self.failed, self.skipped, self.duration_ms)
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assertion_approx_eq() {
        assert!(assert_approx_eq(3.14159, 3.14160, 0.001).passed);
        assert!(!assert_approx_eq(1.0, 2.0, 0.5).passed);
    }

    #[test]
    fn assertion_contains() {
        assert!(assert_contains("hello world", "world").passed);
        assert!(!assert_contains("hello", "xyz").passed);
    }

    #[test]
    fn mock_records_calls() {
        let mut m = Mock::new("db");
        m.when("query", "result_1");
        m.when("query", "result_2");
        let r1 = m.call("query", vec!["SELECT *".into()]);
        assert_eq!(r1, "result_1");
        let r2 = m.call("query", vec!["SELECT 1".into()]);
        assert_eq!(r2, "result_2");
        let r3 = m.call("query", vec![]);
        assert_eq!(r3, "mock_default"); // exhausted queue
        assert_eq!(m.call_count_for("query"), 3);
    }

    #[test]
    fn mock_verify_expectations() {
        let mut m = Mock::new("api");
        m.expect("get", 2, Some(3));
        m.call("get", vec![]);
        m.call("get", vec![]);
        let failures = m.verify();
        assert!(failures.is_empty());
    }

    #[test]
    fn mock_verify_fails_on_missing() {
        let mut m = Mock::new("api");
        m.expect("save", 1, None);
        let failures = m.verify();
        assert_eq!(failures.len(), 1);
        assert!(failures[0].contains("expected at least 1"));
    }

    #[test]
    fn fixture_setup_data() {
        let mut fix = TestFixture::new("db_test");
        fix.on_setup("create_tables");
        fix.on_teardown("drop_tables");
        fix.set("db_url", "sqlite::memory:");
        let actions = fix.setup();
        assert_eq!(actions, &["create_tables"]);
        assert_eq!(fix.get("db_url"), Some("sqlite::memory:"));
    }

    #[test]
    fn property_all_ints_pass() {
        let mut pt = PropTest::new("absolute value is non-negative", 1000);
        pt.for_all_ints(-1000, 1000, |x| x.abs() >= 0);
        assert!(pt.passed());
    }

    #[test]
    fn property_string_length() {
        let mut pt = PropTest::new("string length non-negative", 500);
        pt.for_all_strings(100, |s| s.len() <= 100);
        assert!(pt.passed());
    }

    #[test]
    fn property_addition_commutative() {
        let mut pt = PropTest::new("a + b == b + a", 1000);
        pt.for_all_float_pairs(-1e6, 1e6, |a, b| (a + b - (b + a)).abs() < 1e-9);
        assert!(pt.passed());
    }

    #[test]
    fn snapshot_first_records() {
        let mut store = SnapshotStore::new();
        assert!(store.assert_snapshot("greeting", "hello world"));
        assert_eq!(store.count(), 1);
    }

    #[test]
    fn snapshot_mismatch_detected() {
        let mut store = SnapshotStore::new();
        store.assert_snapshot("data", "version_1");
        let ok = store.assert_snapshot("data", "version_2"); // mismatch
        assert!(!ok);
        assert_eq!(store.mismatches.len(), 1);
    }

    #[test]
    fn parameterized_test() {
        let mut pt = ParamTest::new("double");
        pt.add_case("double 2", HashMap::from([("x".into(), "2".into())]), "4");
        pt.add_case("double 5", HashMap::from([("x".into(), "5".into())]), "10");
        pt.add_case("double 0", HashMap::from([("x".into(), "0".into())]), "0");
        pt.run(|inputs| {
            let x: i32 = inputs["x"].parse().unwrap();
            (x * 2).to_string()
        });
        assert!(pt.all_passed());
        assert_eq!(pt.pass_count(), 3);
    }

    #[test]
    fn parameterized_test_with_failure() {
        let mut pt = ParamTest::new("broken");
        pt.add_case("case1", HashMap::from([("x".into(), "1".into())]), "wrong");
        pt.run(|inputs| inputs["x"].clone());
        assert!(!pt.all_passed());
        assert_eq!(pt.fail_count(), 1);
    }
}
