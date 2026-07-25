// Array Module for Killer Language
// Comprehensive array/collection manipulation functions
// Version: 2.1.0

use crate::value::Value;
use std::cmp::Ordering;

/// Array module providing 20+ array manipulation functions
/// Includes: sorting, filtering, transforming, searching, and more
pub struct ArrayModule;

impl ArrayModule {
    // ==================== Basic Operations ====================
    
    /// Get length of array
    /// length([1, 2, 3]) => 3
    pub fn length(arr: &[Value]) -> usize {
        arr.len()
    }
    
    /// Check if array is empty
    /// is_empty([]) => true
    pub fn is_empty(arr: &[Value]) -> bool {
        arr.is_empty()
    }
    
    /// Get first element
    /// first([1, 2, 3]) => Some(1)
    pub fn first(arr: &[Value]) -> Option<Value> {
        arr.first().cloned()
    }
    
    /// Get last element
    /// last([1, 2, 3]) => Some(3)
    pub fn last(arr: &[Value]) -> Option<Value> {
        arr.last().cloned()
    }
    
    /// Get element at index
    /// at([1, 2, 3], 1) => Some(2)
    pub fn at(arr: &[Value], index: usize) -> Option<Value> {
        arr.get(index).cloned()
    }
    
    /// Create array with N copies of value
    /// fill(5, 3) => [5, 5, 5]
    pub fn fill(value: Value, count: usize) -> Vec<Value> {
        vec![value; count]
    }
    
    // ==================== Sorting & Ordering ====================
    
    /// Sort array (numeric or alphabetic)
    /// sort([3, 1, 2]) => [1, 2, 3]
    pub fn sort(arr: &[Value]) -> Vec<Value> {
        let mut sorted = arr.to_vec();
        sorted.sort_by(|a, b| Self::compare_values(a, b));
        sorted
    }
    
    /// Sort array in reverse order
    /// sort_reverse([1, 2, 3]) => [3, 2, 1]
    pub fn sort_reverse(arr: &[Value]) -> Vec<Value> {
        let mut sorted = arr.to_vec();
        sorted.sort_by(|a, b| Self::compare_values(b, a));
        sorted
    }
    
    /// Reverse array
    /// reverse([1, 2, 3]) => [3, 2, 1]
    pub fn reverse(arr: &[Value]) -> Vec<Value> {
        let mut reversed = arr.to_vec();
        reversed.reverse();
        reversed
    }
    
    // Helper function for comparison
    fn compare_values(a: &Value, b: &Value) -> Ordering {
        match (a, b) {
            (Value::Number(x), Value::Number(y)) => {
                if x < y { Ordering::Less }
                else if x > y { Ordering::Greater }
                else { Ordering::Equal }
            },
            (Value::Str(x), Value::Str(y)) => x.cmp(y),
            _ => Ordering::Equal,
        }
    }
    
    // ==================== Searching & Finding ====================
    
    /// Find index of value (first occurrence)
    /// index_of([1, 2, 3, 2], 2) => Some(1)
    pub fn index_of(arr: &[Value], search: &Value) -> Option<usize> {
        arr.iter().position(|v| Self::values_equal(v, search))
    }
    
    /// Find index of value (last occurrence)
    /// last_index_of([1, 2, 3, 2], 2) => Some(3)
    pub fn last_index_of(arr: &[Value], search: &Value) -> Option<usize> {
        arr.iter().rposition(|v| Self::values_equal(v, search))
    }
    
    /// Check if array contains value
    /// contains([1, 2, 3], 2) => true
    pub fn contains(arr: &[Value], search: &Value) -> bool {
        arr.iter().any(|v| Self::values_equal(v, search))
    }
    
    /// Count occurrences of value
    /// count([1, 2, 2, 3, 2], 2) => 3
    pub fn count(arr: &[Value], search: &Value) -> usize {
        arr.iter().filter(|v| Self::values_equal(v, search)).count()
    }
    
    /// Helper function for value equality
    fn values_equal(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Number(x), Value::Number(y)) => (x - y).abs() < 1e-10,
            (Value::Str(x), Value::Str(y)) => x == y,
            (Value::Bool(x), Value::Bool(y)) => x == y,
            _ => false,
        }
    }
    
    // ==================== Transformation ====================
    
    /// Add element to end of array
    /// push([1, 2], 3) => [1, 2, 3]
    pub fn push(mut arr: Vec<Value>, value: Value) -> Vec<Value> {
        arr.push(value);
        arr
    }
    
    /// Remove last element and return array
    /// pop([1, 2, 3]) => [1, 2]
    pub fn pop(mut arr: Vec<Value>) -> Vec<Value> {
        arr.pop();
        arr
    }
    
    /// Add element to start of array
    /// unshift([2, 3], 1) => [1, 2, 3]
    pub fn unshift(value: Value, mut arr: Vec<Value>) -> Vec<Value> {
        arr.insert(0, value);
        arr
    }
    
    /// Remove first element and return array
    /// shift([1, 2, 3]) => [2, 3]
    pub fn shift(mut arr: Vec<Value>) -> Vec<Value> {
        if !arr.is_empty() {
            arr.remove(0);
        }
        arr
    }
    
    /// Concatenate arrays
    /// concat([1, 2], [3, 4]) => [1, 2, 3, 4]
    pub fn concat(mut arr1: Vec<Value>, arr2: &[Value]) -> Vec<Value> {
        arr1.extend_from_slice(arr2);
        arr1
    }
    
    /// Flatten nested arrays one level
    /// flatten([[1, 2], [3, 4]]) => [1, 2, 3, 4]
    pub fn flatten(arr: &[Value]) -> Vec<Value> {
        let mut result = Vec::new();
        for item in arr {
            if let Value::Array(nested) = item {
                result.extend(nested.clone());
            } else {
                result.push(item.clone());
            }
        }
        result
    }
    
    /// Deep flatten all levels
    /// deep_flatten([[1, [2, 3]], [4]]) => [1, 2, 3, 4]
    pub fn deep_flatten(arr: &[Value]) -> Vec<Value> {
        let mut result = Vec::new();
        for item in arr {
            if let Value::Array(nested) = item {
                result.extend(Self::deep_flatten(&nested));
            } else {
                result.push(item.clone());
            }
        }
        result
    }
    
    // ==================== Slicing & Chunking ====================
    
    /// Get slice from start to end index
    /// slice([1, 2, 3, 4], 1, 3) => [2, 3]
    pub fn slice(arr: &[Value], start: usize, end: usize) -> Vec<Value> {
        let start = start.min(arr.len());
        let end = end.min(arr.len());
        if start >= end {
            return Vec::new();
        }
        arr[start..end].to_vec()
    }
    
    /// Split array into chunks of size N
    /// chunk([1, 2, 3, 4, 5], 2) => [[1, 2], [3, 4], [5]]
    pub fn chunk(arr: &[Value], size: usize) -> Vec<Vec<Value>> {
        if size == 0 {
            return Vec::new();
        }
        
        let mut chunks = Vec::new();
        for i in (0..arr.len()).step_by(size) {
            let end = (i + size).min(arr.len());
            chunks.push(arr[i..end].to_vec());
        }
        chunks
    }
    
    /// Get unique elements
    /// unique([1, 2, 2, 3, 1]) => [1, 2, 3]
    pub fn unique(arr: &[Value]) -> Vec<Value> {
        let mut unique = Vec::new();
        for item in arr {
            if !unique.iter().any(|v| Self::values_equal(v, item)) {
                unique.push(item.clone());
            }
        }
        unique
    }
    
    /// Get elements matching predicate (simple string search version)
    /// For Killer, filter would be passed as callback in VM
    pub fn filter_string(arr: &[Value], pattern: &str) -> Vec<Value> {
        arr.iter()
            .filter(|v| {
                if let Value::Str(s) = v {
                    s.contains(pattern)
                } else {
                    false
                }
            })
            .cloned()
            .collect()
    }
    
    // ==================== Aggregation ====================
    
    /// Sum numeric array
    /// sum([1, 2, 3, 4]) => 10.0
    pub fn sum(arr: &[Value]) -> f64 {
        arr.iter()
            .filter_map(|v| if let Value::Number(n) = v { Some(*n) } else { None })
            .sum()
    }
    
    /// Find minimum value
    /// min([3, 1, 4, 1, 5]) => 1.0
    pub fn min(arr: &[Value]) -> Option<f64> {
        arr.iter()
            .filter_map(|v| if let Value::Number(n) = v { Some(*n) } else { None })
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
    }
    
    /// Find maximum value
    /// max([3, 1, 4, 1, 5]) => 5.0
    pub fn max(arr: &[Value]) -> Option<f64> {
        arr.iter()
            .filter_map(|v| if let Value::Number(n) = v { Some(*n) } else { None })
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
    }
    
    /// Calculate average
    /// average([1, 2, 3, 4, 5]) => 3.0
    pub fn average(arr: &[Value]) -> Option<f64> {
        let nums: Vec<f64> = arr.iter()
            .filter_map(|v| if let Value::Number(n) = v { Some(*n) } else { None })
            .collect();
        
        if nums.is_empty() {
            return None;
        }
        Some(nums.iter().sum::<f64>() / nums.len() as f64)
    }
    
    /// Count matching values
    /// count_matching([1, 2, 2, 3, 2], 2) => 3
    pub fn count_matching(arr: &[Value], search: &Value) -> usize {
        arr.iter().filter(|v| Self::values_equal(v, search)).count()
    }
    
    // ==================== Utility ====================
    
    /// Create array from range
    /// range(1, 5) => [1, 2, 3, 4, 5]
    pub fn range(start: i32, end: i32) -> Vec<Value> {
        if start <= end {
            (start..=end).map(|i| Value::Number(i as f64)).collect()
        } else {
            (end..=start).rev().map(|i| Value::Number(i as f64)).collect()
        }
    }
    
    /// Rotate array left by N positions
    /// rotate_left([1, 2, 3, 4], 1) => [2, 3, 4, 1]
    pub fn rotate_left(mut arr: Vec<Value>, n: usize) -> Vec<Value> {
        if arr.is_empty() { return arr; }
        let n = n % arr.len();
        arr.rotate_left(n);
        arr
    }
    
    /// Rotate array right by N positions
    /// rotate_right([1, 2, 3, 4], 1) => [4, 1, 2, 3]
    pub fn rotate_right(mut arr: Vec<Value>, n: usize) -> Vec<Value> {
        if arr.is_empty() { return arr; }
        let n = n % arr.len();
        arr.rotate_right(n);
        arr
    }
    
    /// Join array elements into string
    /// join([1, 2, 3], ",") => "1,2,3"
    pub fn join(arr: &[Value], delimiter: &str) -> String {
        arr.iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(delimiter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn num(n: f64) -> Value { Value::Number(n) }
    fn str(s: &str) -> Value { Value::Str(s.to_string()) }
    
    #[test]
    fn test_basic_operations() {
        let arr = vec![num(1.0), num(2.0), num(3.0)];
        assert_eq!(ArrayModule::length(&arr), 3);
        assert!(!ArrayModule::is_empty(&arr));
        assert_eq!(ArrayModule::first(&arr), Some(num(1.0)));
        assert_eq!(ArrayModule::last(&arr), Some(num(3.0)));
    }
    
    #[test]
    fn test_sorting() {
        let arr = vec![num(3.0), num(1.0), num(2.0)];
        let sorted = ArrayModule::sort(&arr);
        assert_eq!(sorted, vec![num(1.0), num(2.0), num(3.0)]);
    }
    
    #[test]
    fn test_searching() {
        let arr = vec![num(1.0), num(2.0), num(3.0), num(2.0)];
        assert_eq!(ArrayModule::index_of(&arr, &num(2.0)), Some(1));
        assert_eq!(ArrayModule::last_index_of(&arr, &num(2.0)), Some(3));
        assert!(ArrayModule::contains(&arr, &num(2.0)));
    }
    
    #[test]
    fn test_transformation() {
        let arr = vec![num(1.0), num(2.0)];
        let with_push = ArrayModule::push(arr.clone(), num(3.0));
        assert_eq!(with_push.len(), 3);
    }
    
    #[test]
    fn test_aggregation() {
        let arr = vec![num(1.0), num(2.0), num(3.0), num(4.0)];
        assert_eq!(ArrayModule::sum(&arr), 10.0);
        assert_eq!(ArrayModule::min(&arr), Some(1.0));
        assert_eq!(ArrayModule::max(&arr), Some(4.0));
    }
}
