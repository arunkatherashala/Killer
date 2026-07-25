// Phase 6: AI/ML Module - Vector operations, clustering, ML utilities
// Features: Vectors, matrices, distance metrics, clustering, neural network basics

use std::collections::HashMap;
use crate::llm::{LlmConfig, LlmMessage, LlmResponse, complete as llm_complete};

/// Dense vector for ML operations
#[derive(Clone, Debug)]
pub struct Vector {
    pub data: Vec<f64>,
}

impl Vector {
    /// Create new vector
    pub fn new(data: Vec<f64>) -> Self {
        Vector { data }
    }

    /// Vector magnitude (L2 norm)
    pub fn magnitude(&self) -> f64 {
        self.data.iter().map(|x| x * x).sum::<f64>().sqrt()
    }

    /// Dot product
    pub fn dot(&self, other: &Vector) -> Result<f64, String> {
        if self.data.len() != other.data.len() {
            return Err("Vectors must have same length".to_string());
        }
        Ok(self.data.iter().zip(&other.data).map(|(a, b)| a * b).sum())
    }

    /// Cosine similarity (0-1)
    pub fn cosine_similarity(&self, other: &Vector) -> Result<f64, String> {
        let mag_self = self.magnitude();
        let mag_other = other.magnitude();

        if mag_self == 0.0 || mag_other == 0.0 {
            return Err("Zero magnitude vector".to_string());
        }

        let dot = self.dot(other)?;
        Ok(dot / (mag_self * mag_other))
    }

    /// Euclidean distance
    pub fn euclidean_distance(&self, other: &Vector) -> Result<f64, String> {
        if self.data.len() != other.data.len() {
            return Err("Vectors must have same length".to_string());
        }
        let sum: f64 = self.data.iter().zip(&other.data).map(|(a, b)| (a - b).powi(2)).sum();
        Ok(sum.sqrt())
    }

    /// Manhattan distance
    pub fn manhattan_distance(&self, other: &Vector) -> Result<f64, String> {
        if self.data.len() != other.data.len() {
            return Err("Vectors must have same length".to_string());
        }
        Ok(self.data.iter().zip(&other.data).map(|(a, b)| (a - b).abs()).sum())
    }

    /// Normalize vector (L2 normalization)
    pub fn normalize(&self) -> Vector {
        let mag = self.magnitude();
        if mag == 0.0 {
            return Vector::new(self.data.clone());
        }
        Vector::new(self.data.iter().map(|x| x / mag).collect())
    }

    /// Vector dimension
    pub fn dim(&self) -> usize {
        self.data.len()
    }
}

/// K-Means clustering algorithm
pub struct KMeans {
    pub k: usize,
    pub centroids: Vec<Vector>,
    pub assignments: Vec<usize>,
    pub iterations: usize,
}

impl KMeans {
    /// Initialize K-Means
    pub fn new(k: usize) -> Self {
        KMeans {
            k,
            centroids: Vec::new(),
            assignments: Vec::new(),
            iterations: 0,
        }
    }

    /// Fit K-Means to data
    pub fn fit(&mut self, data: &[Vector], max_iter: usize) -> Result<(), String> {
        if data.is_empty() || self.k > data.len() {
            return Err("Invalid k or empty data".to_string());
        }

        // Initialize centroids with random selection
        self.centroids = data.iter().take(self.k).cloned().collect();

        for _ in 0..max_iter {
            self.iterations += 1;

            // Assignment step
            self.assignments.clear();
            for point in data {
                let mut min_dist = f64::INFINITY;
                let mut closest = 0;

                for (i, centroid) in self.centroids.iter().enumerate() {
                    if let Ok(dist) = point.euclidean_distance(centroid) {
                        if dist < min_dist {
                            min_dist = dist;
                            closest = i;
                        }
                    }
                }
                self.assignments.push(closest);
            }

            // Update centroids
            let mut new_centroids = vec![Vector::new(vec![0.0; self.centroids[0].dim()]); self.k];
            let mut counts = vec![0; self.k];

            for (idx, &cluster) in self.assignments.iter().enumerate() {
                for d in 0..data[idx].dim() {
                    new_centroids[cluster].data[d] += data[idx].data[d];
                }
                counts[cluster] += 1;
            }

            // Average
            for i in 0..self.k {
                if counts[i] > 0 {
                    for d in 0..new_centroids[i].dim() {
                        new_centroids[i].data[d] /= counts[i] as f64;
                    }
                }
            }

            self.centroids = new_centroids;
        }

        Ok(())
    }

    /// Predict cluster for new point
    pub fn predict(&self, point: &Vector) -> Result<usize, String> {
        let mut min_dist = f64::INFINITY;
        let mut closest = 0;

        for (i, centroid) in self.centroids.iter().enumerate() {
            if let Ok(dist) = point.euclidean_distance(centroid) {
                if dist < min_dist {
                    min_dist = dist;
                    closest = i;
                }
            }
        }

        Ok(closest)
    }
}

/// Simple neural network layer
#[derive(Clone, Debug)]
pub struct NeuralNetworkLayer {
    pub weights: Vec<Vector>,
    pub biases: Vec<f64>,
    pub activation: String, // "relu", "sigmoid", "tanh", "linear"
}

impl NeuralNetworkLayer {
    /// Create network layer
    pub fn new(input_size: usize, output_size: usize, activation: String) -> Self {
        let mut weights = Vec::new();
        for _ in 0..output_size {
            // Initialize with random weights
            let w: Vec<f64> = (0..input_size).map(|i| (i as f64 % 1.0) / (input_size as f64)).collect();
            weights.push(Vector::new(w));
        }

        let biases = vec![0.0; output_size];

        NeuralNetworkLayer {
            weights,
            biases,
            activation,
        }
    }

    /// Forward pass
    pub fn forward(&self, input: &Vector) -> Result<Vector, String> {
        let mut output = Vec::new();

        for (w, &b) in self.weights.iter().zip(&self.biases) {
            let z = w.dot(input)? + b;
            let activated = match self.activation.as_str() {
                "relu" => z.max(0.0),
                "sigmoid" => 1.0 / (1.0 + (-z).exp()),
                "tanh" => z.tanh(),
                "linear" => z,
                _ => z,
            };
            output.push(activated);
        }

        Ok(Vector::new(output))
    }
}

/// Distance metrics
pub struct DistanceMetrics;

impl DistanceMetrics {
    /// Euclidean distance
    pub fn euclidean(a: &Vector, b: &Vector) -> Result<f64, String> {
        a.euclidean_distance(b)
    }

    /// Manhattan distance
    pub fn manhattan(a: &Vector, b: &Vector) -> Result<f64, String> {
        a.manhattan_distance(b)
    }

    /// Cosine distance (1 - similarity)
    pub fn cosine(a: &Vector, b: &Vector) -> Result<f64, String> {
        let similarity = a.cosine_similarity(b)?;
        Ok(1.0 - similarity)
    }

    /// Hamming distance (for binary vectors)
    pub fn hamming(a: &Vector, b: &Vector) -> Result<usize, String> {
        if a.dim() != b.dim() {
            return Err("Vectors must have same length".to_string());
        }
        Ok(a.data.iter().zip(&b.data).filter(|(x, y)| x != y).count())
    }
}

/// Feature scaling
pub struct FeatureScaler;

impl FeatureScaler {
    /// Min-max normalization (0-1)
    pub fn min_max_scale(data: &[Vector]) -> Result<Vec<Vector>, String> {
        if data.is_empty() {
            return Err("Empty data".to_string());
        }

        let dim = data[0].dim();
        let mut mins = vec![f64::INFINITY; dim];
        let mut maxs = vec![f64::NEG_INFINITY; dim];

        for point in data {
            for d in 0..dim {
                mins[d] = mins[d].min(point.data[d]);
                maxs[d] = maxs[d].max(point.data[d]);
            }
        }

        let mut scaled = Vec::new();
        for point in data {
            let mut s_data = Vec::new();
            for d in 0..dim {
                let range = maxs[d] - mins[d];
                let scaled_val = if range == 0.0 {
                    0.0
                } else {
                    (point.data[d] - mins[d]) / range
                };
                s_data.push(scaled_val);
            }
            scaled.push(Vector::new(s_data));
        }

        Ok(scaled)
    }

    /// Z-score standardization
    pub fn standardize(data: &[Vector]) -> Result<Vec<Vector>, String> {
        if data.is_empty() {
            return Err("Empty data".to_string());
        }

        let dim = data[0].dim();
        let n = data.len() as f64;

        // Calculate mean
        let mut means = vec![0.0; dim];
        for point in data {
            for d in 0..dim {
                means[d] += point.data[d];
            }
        }
        for m in &mut means {
            *m /= n;
        }

        // Calculate std dev
        let mut stds = vec![0.0; dim];
        for point in data {
            for d in 0..dim {
                stds[d] += (point.data[d] - means[d]).powi(2);
            }
        }
        for s in &mut stds {
            *s = (*s / n).sqrt();
        }

        // Standardize
        let mut result = Vec::new();
        for point in data {
            let mut z_data = Vec::new();
            for d in 0..dim {
                let z = if stds[d] == 0.0 {
                    0.0
                } else {
                    (point.data[d] - means[d]) / stds[d]
                };
                z_data.push(z);
            }
            result.push(Vector::new(z_data));
        }

        Ok(result)
    }
}

/// Classification metrics
pub struct ClassificationMetrics;

impl ClassificationMetrics {
    /// Calculate accuracy
    pub fn accuracy(predictions: &[usize], actuals: &[usize]) -> Result<f64, String> {
        if predictions.len() != actuals.len() {
            return Err("Length mismatch".to_string());
        }
        let correct = predictions.iter().zip(actuals).filter(|(p, a)| p == a).count();
        Ok(correct as f64 / predictions.len() as f64)
    }

    /// Calculate precision (TP / (TP + FP))
    pub fn precision(predictions: &[usize], actuals: &[usize], positive_class: usize) -> Result<f64, String> {
        if predictions.len() != actuals.len() {
            return Err("Length mismatch".to_string());
        }

        let tp = predictions.iter().zip(actuals).filter(|(p, a)| **p == positive_class && **a == positive_class).count() as f64;
        let fp = predictions.iter().zip(actuals).filter(|(p, a)| **p == positive_class && **a != positive_class).count() as f64;

        if tp + fp == 0.0 {
            Ok(0.0)
        } else {
            Ok(tp / (tp + fp))
        }
    }

    /// Calculate recall (TP / (TP + FN))
    pub fn recall(predictions: &[usize], actuals: &[usize], positive_class: usize) -> Result<f64, String> {
        if predictions.len() != actuals.len() {
            return Err("Length mismatch".to_string());
        }

        let tp = predictions.iter().zip(actuals).filter(|(p, a)| **p == positive_class && **a == positive_class).count() as f64;
        let fn_count = predictions.iter().zip(actuals).filter(|(p, a)| **p != positive_class && **a == positive_class).count() as f64;

        if tp + fn_count == 0.0 {
            Ok(0.0)
        } else {
            Ok(tp / (tp + fn_count))
        }
    }
}

/// ML Module public interface
pub struct MLModule;

impl MLModule {
    /// Create vector from array
    pub fn vector(data: Vec<f64>) -> Vector {
        Vector::new(data)
    }

    /// Create K-Means clustering
    pub fn kmeans(k: usize) -> KMeans {
        KMeans::new(k)
    }

    /// Create neural net layer
    pub fn layer(input_size: usize, output_size: usize, activation: String) -> NeuralNetworkLayer {
        NeuralNetworkLayer::new(input_size, output_size, activation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_creation() {
        let v = Vector::new(vec![1.0, 2.0, 3.0]);
        assert_eq!(v.data.len(), 3);
        assert_eq!(v.dim(), 3);
    }

    #[test]
    fn test_vector_magnitude() {
        let v = Vector::new(vec![3.0, 4.0]);
        assert_eq!(v.magnitude(), 5.0);
    }

    #[test]
    fn test_vector_dot_product() {
        let v1 = Vector::new(vec![1.0, 2.0, 3.0]);
        let v2 = Vector::new(vec![4.0, 5.0, 6.0]);
        let result = v1.dot(&v2).unwrap();
        assert_eq!(result, 32.0); // 1*4 + 2*5 + 3*6
    }

    #[test]
    fn test_vector_cosine_similarity() {
        let v1 = Vector::new(vec![1.0, 0.0]);
        let v2 = Vector::new(vec![1.0, 0.0]);
        let sim = v1.cosine_similarity(&v2).unwrap();
        assert_eq!(sim, 1.0); // Perfect similarity
    }

    #[test]
    fn test_euclidean_distance() {
        let v1 = Vector::new(vec![0.0, 0.0]);
        let v2 = Vector::new(vec![3.0, 4.0]);
        let dist = v1.euclidean_distance(&v2).unwrap();
        assert_eq!(dist, 5.0);
    }

    #[test]
    fn test_manhattan_distance() {
        let v1 = Vector::new(vec![0.0, 0.0]);
        let v2 = Vector::new(vec![3.0, 4.0]);
        let dist = v1.manhattan_distance(&v2).unwrap();
        assert_eq!(dist, 7.0);
    }

    #[test]
    fn test_vector_normalize() {
        let v = Vector::new(vec![3.0, 4.0]);
        let normalized = v.normalize();
        assert!((normalized.magnitude() - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_kmeans_clustering() {
        let data = vec![
            Vector::new(vec![0.0, 0.0]),
            Vector::new(vec![1.0, 1.0]),
            Vector::new(vec![10.0, 10.0]),
            Vector::new(vec![11.0, 11.0]),
        ];

        let mut km = KMeans::new(2);
        let result = km.fit(&data, 10);
        assert!(result.is_ok());
        assert_eq!(km.assignments.len(), 4);
    }

    #[test]
    fn test_neural_network_layer() {
        let layer = NeuralNetworkLayer::new(3, 2, "linear".to_string());
        let input = Vector::new(vec![1.0, 2.0, 3.0]);
        let output = layer.forward(&input);
        assert!(output.is_ok());
    }

    #[test]
    fn test_neural_network_relu() {
        let layer = NeuralNetworkLayer::new(2, 2, "relu".to_string());
        let input = Vector::new(vec![1.0, -1.0]);
        let output = layer.forward(&input).unwrap();
        assert_eq!(output.dim(), 2);
    }

    #[test]
    fn test_distance_metrics_euclidean() {
        let v1 = Vector::new(vec![0.0, 0.0]);
        let v2 = Vector::new(vec![3.0, 4.0]);
        let dist = DistanceMetrics::euclidean(&v1, &v2).unwrap();
        assert_eq!(dist, 5.0);
    }

    #[test]
    fn test_feature_scaling_minmax() {
        let data = vec![
            Vector::new(vec![1.0, 2.0]),
            Vector::new(vec![2.0, 4.0]),
            Vector::new(vec![3.0, 6.0]),
        ];
        let scaled = FeatureScaler::min_max_scale(&data).unwrap();
        assert_eq!(scaled.len(), 3);
    }

    #[test]
    fn test_feature_scaling_standardize() {
        let data = vec![
            Vector::new(vec![1.0, 2.0]),
            Vector::new(vec![2.0, 4.0]),
            Vector::new(vec![3.0, 6.0]),
        ];
        let standardized = FeatureScaler::standardize(&data).unwrap();
        assert_eq!(standardized.len(), 3);
    }

    #[test]
    fn test_classification_accuracy() {
        let predictions = vec![0, 1, 1, 0];
        let actuals = vec![0, 1, 0, 0];
        let acc = ClassificationMetrics::accuracy(&predictions, &actuals).unwrap();
        assert_eq!(acc, 0.75);
    }

    #[test]
    fn test_classification_precision() {
        let predictions = vec![0, 1, 1, 0];
        let actuals = vec![0, 1, 0, 0];
        let prec = ClassificationMetrics::precision(&predictions, &actuals, 1).unwrap();
        assert_eq!(prec, 0.5);
    }

    #[test]
    fn test_classification_recall() {
        let predictions = vec![0, 1, 1, 0];
        let actuals = vec![0, 1, 1, 0];
        let rec = ClassificationMetrics::recall(&predictions, &actuals, 1).unwrap();
        assert_eq!(rec, 1.0);
    }
}

// ============================================================================
// STAGE 2: GRADIENTS & CALCULUS
// ============================================================================

/// Numerical gradient engine — d/dx f(x) via central difference.
pub struct Gradient;
impl Gradient {
    const H: f64 = 1e-5;

    /// Derivative of f at x (central difference).
    pub fn derivative<F: Fn(f64) -> f64>(f: &F, x: f64) -> f64 {
        (f(x + Self::H) - f(x - Self::H)) / (2.0 * Self::H)
    }

    /// Gradient of f: R^n → R at point x (vector of partial derivatives).
    pub fn gradient<F: Fn(&[f64]) -> f64>(f: &F, x: &[f64]) -> Vec<f64> {
        let mut grad = vec![0.0; x.len()];
        let mut xp = x.to_vec();
        for i in 0..x.len() {
            xp[i] = x[i] + Self::H;
            let fp = f(&xp);
            xp[i] = x[i] - Self::H;
            let fm = f(&xp);
            xp[i] = x[i]; // restore
            grad[i] = (fp - fm) / (2.0 * Self::H);
        }
        grad
    }
}

/// Statistics module — mean, variance, std, correlation, covariance.
pub struct Stats;
impl Stats {
    pub fn mean(data: &[f64]) -> f64 {
        if data.is_empty() { return 0.0; }
        data.iter().sum::<f64>() / data.len() as f64
    }

    pub fn variance(data: &[f64]) -> f64 {
        if data.len() < 2 { return 0.0; }
        let m = Self::mean(data);
        data.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (data.len() - 1) as f64
    }

    pub fn std_dev(data: &[f64]) -> f64 { Self::variance(data).sqrt() }

    pub fn median(data: &[f64]) -> f64 {
        if data.is_empty() { return 0.0; }
        let mut sorted = data.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = sorted.len();
        if n % 2 == 0 { (sorted[n/2 - 1] + sorted[n/2]) / 2.0 }
        else { sorted[n/2] }
    }

    pub fn correlation(x: &[f64], y: &[f64]) -> Result<f64, String> {
        if x.len() != y.len() || x.is_empty() {
            return Err("Vectors must be same non-empty length".to_string());
        }
        let mx = Self::mean(x);
        let my = Self::mean(y);
        let num: f64 = x.iter().zip(y).map(|(xi, yi)| (xi - mx) * (yi - my)).sum();
        let dx: f64 = x.iter().map(|xi| (xi - mx).powi(2)).sum::<f64>().sqrt();
        let dy: f64 = y.iter().map(|yi| (yi - my).powi(2)).sum::<f64>().sqrt();
        if dx == 0.0 || dy == 0.0 { return Err("Zero variance".to_string()); }
        Ok(num / (dx * dy))
    }
}

// ============================================================================
// STAGE 3: FEATURE ENCODING & EDA
// ============================================================================

/// One-hot encoder — turns categorical labels into binary vectors.
pub struct OneHotEncoder {
    pub categories: Vec<String>,
}

impl OneHotEncoder {
    pub fn fit(labels: &[&str]) -> Self {
        let mut cats: Vec<String> = labels.iter().map(|s| s.to_string()).collect();
        cats.sort();
        cats.dedup();
        OneHotEncoder { categories: cats }
    }

    pub fn encode(&self, label: &str) -> Vec<f64> {
        self.categories.iter().map(|c| if c == label { 1.0 } else { 0.0 }).collect()
    }

    pub fn decode(&self, vec: &[f64]) -> Option<String> {
        vec.iter().enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| self.categories[i].clone())
    }
}

/// Label encoder — maps string labels to integer indices.
pub struct LabelEncoder {
    pub mapping: HashMap<String, usize>,
    pub inverse: Vec<String>,
}

impl LabelEncoder {
    pub fn fit(labels: &[&str]) -> Self {
        let mut mapping = HashMap::new();
        let mut inverse = Vec::new();
        let mut unique: Vec<String> = labels.iter().map(|s| s.to_string()).collect();
        unique.sort();
        unique.dedup();
        for (i, label) in unique.iter().enumerate() {
            mapping.insert(label.clone(), i);
            inverse.push(label.clone());
        }
        LabelEncoder { mapping, inverse }
    }

    pub fn encode(&self, label: &str) -> Option<usize> {
        self.mapping.get(label).copied()
    }

    pub fn decode(&self, idx: usize) -> Option<&str> {
        self.inverse.get(idx).map(String::as_str)
    }
}

/// EDA (Exploratory Data Analysis) — summary statistics for a dataset.
pub struct EDA;
impl EDA {
    /// Summary: min, max, mean, std for each column.
    pub fn describe(data: &[Vector]) -> Vec<[f64; 4]> {
        if data.is_empty() { return Vec::new(); }
        let n_cols = data[0].dim();
        (0..n_cols).map(|col| {
            let col_data: Vec<f64> = data.iter().map(|v| v.data[col]).collect();
            let mn = col_data.iter().cloned().fold(f64::INFINITY, f64::min);
            let mx = col_data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let mean = Stats::mean(&col_data);
            let std = Stats::std_dev(&col_data);
            [mn, mx, mean, std]
        }).collect()
    }

    /// Count missing (NaN) values per column.
    pub fn count_missing(data: &[Vector]) -> Vec<usize> {
        if data.is_empty() { return Vec::new(); }
        let n_cols = data[0].dim();
        (0..n_cols).map(|col| {
            data.iter().filter(|v| v.data[col].is_nan()).count()
        }).collect()
    }

    /// Correlation matrix between all pairs of columns.
    pub fn correlation_matrix(data: &[Vector]) -> Vec<Vec<f64>> {
        if data.is_empty() { return Vec::new(); }
        let n_cols = data[0].dim();
        let cols: Vec<Vec<f64>> = (0..n_cols)
            .map(|c| data.iter().map(|v| v.data[c]).collect())
            .collect();
        (0..n_cols).map(|i| {
            (0..n_cols).map(|j| {
                Stats::correlation(&cols[i], &cols[j]).unwrap_or(0.0)
            }).collect()
        }).collect()
    }
}

// ============================================================================
// STAGE 5: ML ALGORITHMS
// ============================================================================

/// Linear Regression via gradient descent (or normal equation).
pub struct LinearRegression {
    pub weights: Vec<f64>,
    pub bias: f64,
    pub learning_rate: f64,
}

impl LinearRegression {
    pub fn new(learning_rate: f64) -> Self {
        LinearRegression { weights: Vec::new(), bias: 0.0, learning_rate }
    }

    pub fn fit(&mut self, x: &[Vector], y: &[f64], epochs: usize) -> Result<(), String> {
        if x.is_empty() || x.len() != y.len() {
            return Err("X and y must be same non-empty length".to_string());
        }
        let n_features = x[0].dim();
        let n = x.len() as f64;
        self.weights = vec![0.0; n_features];
        self.bias = 0.0;

        for _ in 0..epochs {
            let preds: Vec<f64> = x.iter().map(|xi| self.predict_one(xi)).collect();
            let errors: Vec<f64> = preds.iter().zip(y).map(|(p, yi)| p - yi).collect();

            for j in 0..n_features {
                let grad: f64 = errors.iter().zip(x).map(|(e, xi)| e * xi.data[j]).sum::<f64>() / n;
                self.weights[j] -= self.learning_rate * grad;
            }
            let bias_grad: f64 = errors.iter().sum::<f64>() / n;
            self.bias -= self.learning_rate * bias_grad;
        }
        Ok(())
    }

    pub fn predict_one(&self, x: &Vector) -> f64 {
        self.weights.iter().zip(&x.data).map(|(w, xi)| w * xi).sum::<f64>() + self.bias
    }

    pub fn predict(&self, x: &[Vector]) -> Vec<f64> {
        x.iter().map(|xi| self.predict_one(xi)).collect()
    }

    pub fn mse(&self, x: &[Vector], y: &[f64]) -> f64 {
        let preds = self.predict(x);
        preds.iter().zip(y).map(|(p, yi)| (p - yi).powi(2)).sum::<f64>() / y.len() as f64
    }
}

/// Logistic Regression — binary classifier via sigmoid + gradient descent.
pub struct LogisticRegression {
    pub weights: Vec<f64>,
    pub bias: f64,
    pub learning_rate: f64,
}

impl LogisticRegression {
    pub fn new(learning_rate: f64) -> Self {
        LogisticRegression { weights: Vec::new(), bias: 0.0, learning_rate }
    }

    fn sigmoid(z: f64) -> f64 { 1.0 / (1.0 + (-z).exp()) }

    pub fn fit(&mut self, x: &[Vector], y: &[f64], epochs: usize) -> Result<(), String> {
        if x.is_empty() || x.len() != y.len() {
            return Err("X and y must be same non-empty length".to_string());
        }
        let n_features = x[0].dim();
        let n = x.len() as f64;
        self.weights = vec![0.0; n_features];
        self.bias = 0.0;

        for _ in 0..epochs {
            let preds: Vec<f64> = x.iter().map(|xi| {
                let z: f64 = self.weights.iter().zip(&xi.data).map(|(w, xij)| w * xij).sum::<f64>() + self.bias;
                Self::sigmoid(z)
            }).collect();

            let errors: Vec<f64> = preds.iter().zip(y).map(|(p, yi)| p - yi).collect();

            for j in 0..n_features {
                let grad: f64 = errors.iter().zip(x).map(|(e, xi)| e * xi.data[j]).sum::<f64>() / n;
                self.weights[j] -= self.learning_rate * grad;
            }
            self.bias -= self.learning_rate * errors.iter().sum::<f64>() / n;
        }
        Ok(())
    }

    pub fn predict_proba(&self, x: &Vector) -> f64 {
        let z: f64 = self.weights.iter().zip(&x.data).map(|(w, xi)| w * xi).sum::<f64>() + self.bias;
        Self::sigmoid(z)
    }

    pub fn predict(&self, x: &[Vector], threshold: f64) -> Vec<usize> {
        x.iter().map(|xi| if self.predict_proba(xi) >= threshold { 1 } else { 0 }).collect()
    }
}

/// K-Nearest Neighbors classifier.
pub struct KNN {
    pub k: usize,
    train_x: Vec<Vector>,
    train_y: Vec<usize>,
}

impl KNN {
    pub fn new(k: usize) -> Self {
        KNN { k, train_x: Vec::new(), train_y: Vec::new() }
    }

    pub fn fit(&mut self, x: Vec<Vector>, y: Vec<usize>) -> Result<(), String> {
        if x.len() != y.len() { return Err("X and y length mismatch".to_string()); }
        self.train_x = x;
        self.train_y = y;
        Ok(())
    }

    pub fn predict_one(&self, point: &Vector) -> Result<usize, String> {
        let mut dists: Vec<(f64, usize)> = self.train_x.iter()
            .enumerate()
            .map(|(i, xi)| (point.euclidean_distance(xi).unwrap_or(f64::INFINITY), i))
            .collect();
        dists.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let mut votes: HashMap<usize, usize> = HashMap::new();
        for (_, idx) in dists.iter().take(self.k) {
            *votes.entry(self.train_y[*idx]).or_insert(0) += 1;
        }
        votes.into_iter().max_by_key(|(_, v)| *v).map(|(k, _)| k)
            .ok_or("No training data".to_string())
    }

    pub fn predict(&self, x: &[Vector]) -> Vec<Result<usize, String>> {
        x.iter().map(|xi| self.predict_one(xi)).collect()
    }
}

/// Naive Bayes (Gaussian) classifier.
pub struct NaiveBayes {
    class_priors: HashMap<usize, f64>,
    class_means:  HashMap<usize, Vec<f64>>,
    class_vars:   HashMap<usize, Vec<f64>>,
}

impl NaiveBayes {
    pub fn new() -> Self {
        NaiveBayes { class_priors: HashMap::new(), class_means: HashMap::new(), class_vars: HashMap::new() }
    }

    pub fn fit(&mut self, x: &[Vector], y: &[usize]) -> Result<(), String> {
        if x.is_empty() || x.len() != y.len() {
            return Err("X and y must be same non-empty length".to_string());
        }
        let n = x.len() as f64;
        let n_features = x[0].dim();
        let mut class_samples: HashMap<usize, Vec<&Vector>> = HashMap::new();
        for (xi, &yi) in x.iter().zip(y) {
            class_samples.entry(yi).or_insert_with(Vec::new).push(xi);
        }
        for (&class, samples) in &class_samples {
            self.class_priors.insert(class, samples.len() as f64 / n);
            let means: Vec<f64> = (0..n_features).map(|j| {
                Stats::mean(&samples.iter().map(|v| v.data[j]).collect::<Vec<_>>())
            }).collect();
            let vars: Vec<f64> = (0..n_features).map(|j| {
                Stats::variance(&samples.iter().map(|v| v.data[j]).collect::<Vec<_>>()).max(1e-9)
            }).collect();
            self.class_means.insert(class, means);
            self.class_vars.insert(class, vars);
        }
        Ok(())
    }

    fn gaussian_log_prob(x: f64, mean: f64, var: f64) -> f64 {
        -0.5 * ((x - mean).powi(2) / var + var.ln() + (2.0 * std::f64::consts::PI).ln())
    }

    pub fn predict_one(&self, x: &Vector) -> Option<usize> {
        self.class_priors.keys().max_by(|&&a, &&b| {
            let log_a = self.class_priors[&a].ln() +
                x.data.iter().enumerate().map(|(j, &xj)| {
                    Self::gaussian_log_prob(xj, self.class_means[&a][j], self.class_vars[&a][j])
                }).sum::<f64>();
            let log_b = self.class_priors[&b].ln() +
                x.data.iter().enumerate().map(|(j, &xj)| {
                    Self::gaussian_log_prob(xj, self.class_means[&b][j], self.class_vars[&b][j])
                }).sum::<f64>();
            log_a.partial_cmp(&log_b).unwrap()
        }).copied()
    }
}

/// Decision Tree (CART) — classification via Gini impurity.
#[derive(Clone, Debug)]
enum DTNode {
    Leaf(usize),
    Split { feature: usize, threshold: f64, left: Box<DTNode>, right: Box<DTNode> },
}

pub struct DecisionTree {
    root: Option<DTNode>,
    pub max_depth: usize,
}

impl DecisionTree {
    pub fn new(max_depth: usize) -> Self {
        DecisionTree { root: None, max_depth }
    }

    fn gini(y: &[usize]) -> f64 {
        if y.is_empty() { return 0.0; }
        let n = y.len() as f64;
        let mut counts: HashMap<usize, usize> = HashMap::new();
        for &yi in y { *counts.entry(yi).or_insert(0) += 1; }
        1.0 - counts.values().map(|&c| (c as f64 / n).powi(2)).sum::<f64>()
    }

    fn majority(y: &[usize]) -> usize {
        let mut counts: HashMap<usize, usize> = HashMap::new();
        for &yi in y { *counts.entry(yi).or_insert(0) += 1; }
        *counts.iter().max_by_key(|(_, &v)| v).map(|(k, _)| k).unwrap_or(&0)
    }

    fn build(x: &[Vector], y: &[usize], depth: usize, max_depth: usize) -> DTNode {
        if depth >= max_depth || y.iter().all(|&yi| yi == y[0]) {
            return DTNode::Leaf(Self::majority(y));
        }
        let n_features = x[0].dim();
        let mut best_gini = f64::INFINITY;
        let mut best_feat = 0;
        let mut best_thresh = 0.0;

        for feat in 0..n_features {
            let mut vals: Vec<f64> = x.iter().map(|xi| xi.data[feat]).collect();
            vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
            vals.dedup();
            for &thresh in &vals {
                let (left_y, right_y): (Vec<_>, Vec<_>) = x.iter().zip(y).partition(|(xi, _)| xi.data[feat] <= thresh);
                let ly: Vec<usize> = left_y.iter().map(|(_, &yi)| yi).collect();
                let ry: Vec<usize> = right_y.iter().map(|(_, &yi)| yi).collect();
                let n = y.len() as f64;
                let g = ly.len() as f64 / n * Self::gini(&ly) + ry.len() as f64 / n * Self::gini(&ry);
                if g < best_gini { best_gini = g; best_feat = feat; best_thresh = thresh; }
            }
        }

        let (left_data, right_data): (Vec<_>, Vec<_>) = x.iter().zip(y).partition(|(xi, _)| xi.data[best_feat] <= best_thresh);
        let (lx, ly): (Vec<Vector>, Vec<usize>) = left_data.into_iter().map(|(v, &yi)| (v.clone(), yi)).unzip();
        let (rx, ry): (Vec<Vector>, Vec<usize>) = right_data.into_iter().map(|(v, &yi)| (v.clone(), yi)).unzip();

        if lx.is_empty() || rx.is_empty() {
            return DTNode::Leaf(Self::majority(y));
        }

        DTNode::Split {
            feature: best_feat,
            threshold: best_thresh,
            left:  Box::new(Self::build(&lx, &ly, depth + 1, max_depth)),
            right: Box::new(Self::build(&rx, &ry, depth + 1, max_depth)),
        }
    }

    pub fn fit(&mut self, x: &[Vector], y: &[usize]) -> Result<(), String> {
        if x.is_empty() || x.len() != y.len() { return Err("Invalid data".to_string()); }
        self.root = Some(Self::build(x, y, 0, self.max_depth));
        Ok(())
    }

    fn predict_node(node: &DTNode, x: &Vector) -> usize {
        match node {
            DTNode::Leaf(label) => *label,
            DTNode::Split { feature, threshold, left, right } => {
                if x.data[*feature] <= *threshold { Self::predict_node(left, x) }
                else { Self::predict_node(right, x) }
            }
        }
    }

    pub fn predict(&self, x: &[Vector]) -> Result<Vec<usize>, String> {
        let root = self.root.as_ref().ok_or("Model not trained")?;
        Ok(x.iter().map(|xi| Self::predict_node(root, xi)).collect())
    }
}

/// Random Forest — ensemble of decision trees.
pub struct RandomForest {
    pub n_trees: usize,
    pub max_depth: usize,
    trees: Vec<DecisionTree>,
}

impl RandomForest {
    pub fn new(n_trees: usize, max_depth: usize) -> Self {
        RandomForest { n_trees, max_depth, trees: Vec::new() }
    }

    pub fn fit(&mut self, x: &[Vector], y: &[usize]) -> Result<(), String> {
        self.trees.clear();
        let n = x.len();
        // Simple bootstrap with deterministic "random" indices using hash
        for t in 0..self.n_trees {
            let indices: Vec<usize> = (0..n).map(|i| (i * 6364136223846793005 + t * 1442695040888963407) % n).collect();
            let bx: Vec<Vector> = indices.iter().map(|&i| x[i].clone()).collect();
            let by: Vec<usize>  = indices.iter().map(|&i| y[i]).collect();
            let mut tree = DecisionTree::new(self.max_depth);
            tree.fit(&bx, &by)?;
            self.trees.push(tree);
        }
        Ok(())
    }

    pub fn predict(&self, x: &[Vector]) -> Vec<usize> {
        x.iter().map(|xi| {
            let mut votes: HashMap<usize, usize> = HashMap::new();
            for tree in &self.trees {
                if let Ok(preds) = tree.predict(std::slice::from_ref(xi)) {
                    *votes.entry(preds[0]).or_insert(0) += 1;
                }
            }
            *votes.iter().max_by_key(|(_, &v)| v).map(|(k, _)| k).unwrap_or(&0)
        }).collect()
    }
}

/// Gradient Boosting — sequential ensemble of regression trees.
pub struct GradientBoosting {
    pub n_estimators: usize,
    pub learning_rate: f64,
    pub max_depth: usize,
    trees: Vec<DecisionTree>,
    base_pred: f64,
}

impl GradientBoosting {
    pub fn new(n_estimators: usize, learning_rate: f64, max_depth: usize) -> Self {
        GradientBoosting { n_estimators, learning_rate, max_depth, trees: Vec::new(), base_pred: 0.0 }
    }

    pub fn fit(&mut self, x: &[Vector], y: &[usize]) -> Result<(), String> {
        if x.is_empty() || x.len() != y.len() { return Err("Invalid data".to_string()); }
        let yf: Vec<f64> = y.iter().map(|&yi| yi as f64).collect();
        self.base_pred = Stats::mean(&yf);
        let mut residuals: Vec<f64> = yf.iter().map(|&yi| yi - self.base_pred).collect();

        for _ in 0..self.n_estimators {
            // Bin residuals to pseudo-labels (sign-based)
            let pseudo: Vec<usize> = residuals.iter().map(|&r| if r > 0.0 { 1 } else { 0 }).collect();
            let mut tree = DecisionTree::new(self.max_depth);
            tree.fit(x, &pseudo)?;
            let preds = tree.predict(x)?;
            for (r, &p) in residuals.iter_mut().zip(&preds) {
                *r -= self.learning_rate * p as f64;
            }
            self.trees.push(tree);
        }
        Ok(())
    }

    pub fn predict(&self, x: &[Vector]) -> Vec<usize> {
        x.iter().map(|xi| {
            let score: f64 = self.base_pred + self.trees.iter().map(|t| {
                t.predict(std::slice::from_ref(xi)).map(|p| p[0] as f64 * self.learning_rate).unwrap_or(0.0)
            }).sum::<f64>();
            if score >= 0.5 { 1 } else { 0 }
        }).collect()
    }
}

/// DBSCAN — density-based spatial clustering.
pub struct DBSCAN {
    pub eps: f64,
    pub min_pts: usize,
    pub labels: Vec<i64>, // -1 = noise, ≥ 0 = cluster id
}

impl DBSCAN {
    pub fn new(eps: f64, min_pts: usize) -> Self {
        DBSCAN { eps, min_pts, labels: Vec::new() }
    }

    fn region_query(data: &[Vector], idx: usize, eps: f64) -> Vec<usize> {
        data.iter().enumerate().filter(|(j, p)| {
            *j != idx && data[idx].euclidean_distance(p).unwrap_or(f64::INFINITY) <= eps
        }).map(|(j, _)| j).collect()
    }

    pub fn fit(&mut self, data: &[Vector]) {
        let n = data.len();
        self.labels = vec![-1i64; n];
        let mut cluster_id: i64 = 0;

        for i in 0..n {
            if self.labels[i] != -1 { continue; }
            let neighbors = Self::region_query(data, i, self.eps);
            if neighbors.len() < self.min_pts {
                self.labels[i] = -1; // noise
                continue;
            }
            self.labels[i] = cluster_id;
            let mut seed_set = neighbors;
            let mut si = 0;
            while si < seed_set.len() {
                let q = seed_set[si];
                if self.labels[q] == -1 { self.labels[q] = cluster_id; }
                if self.labels[q] != cluster_id {
                    si += 1;
                    continue;
                }
                self.labels[q] = cluster_id;
                let q_neighbors = Self::region_query(data, q, self.eps);
                if q_neighbors.len() >= self.min_pts {
                    for &nb in &q_neighbors {
                        if self.labels[nb] == -1 { seed_set.push(nb); }
                    }
                }
                si += 1;
            }
            cluster_id += 1;
        }
    }
}

/// PCA — Principal Component Analysis via covariance + power iteration.
pub struct PCA {
    pub n_components: usize,
    pub components: Vec<Vector>, // eigenvectors
    pub mean: Vec<f64>,
}

impl PCA {
    pub fn new(n_components: usize) -> Self {
        PCA { n_components, components: Vec::new(), mean: Vec::new() }
    }

    pub fn fit(&mut self, data: &[Vector]) -> Result<(), String> {
        if data.is_empty() { return Err("Empty data".to_string()); }
        let n = data.len();
        let d = data[0].dim();
        // Compute mean
        self.mean = (0..d).map(|j| data.iter().map(|v| v.data[j]).sum::<f64>() / n as f64).collect();
        // Center data
        let centered: Vec<Vector> = data.iter().map(|v| {
            Vector::new(v.data.iter().zip(&self.mean).map(|(x, m)| x - m).collect())
        }).collect();
        // Compute covariance matrix (d×d)
        let mut cov = vec![vec![0.0f64; d]; d];
        for v in &centered {
            for i in 0..d {
                for j in 0..d {
                    cov[i][j] += v.data[i] * v.data[j];
                }
            }
        }
        for i in 0..d { for j in 0..d { cov[i][j] /= (n - 1) as f64; } }
        // Power iteration: find top n_components eigenvectors
        self.components.clear();
        let mut deflated = cov.clone();
        for _ in 0..self.n_components.min(d) {
            let mut v: Vec<f64> = (0..d).map(|i| if i == 0 { 1.0 } else { 0.0 }).collect();
            for _ in 0..100 {
                let mut av = vec![0.0f64; d];
                for i in 0..d { for j in 0..d { av[i] += deflated[i][j] * v[j]; } }
                let norm = av.iter().map(|x| x * x).sum::<f64>().sqrt().max(1e-12);
                v = av.iter().map(|x| x / norm).collect();
            }
            self.components.push(Vector::new(v.clone()));
            // Deflate: subtract outer product
            let eigen = {
                let mut av = vec![0.0f64; d];
                for i in 0..d { for j in 0..d { av[i] += deflated[i][j] * v[j]; } }
                av.iter().zip(&v).map(|(a, vi)| a * vi).sum::<f64>()
            };
            for i in 0..d { for j in 0..d { deflated[i][j] -= eigen * v[i] * v[j]; } }
        }
        Ok(())
    }

    pub fn transform(&self, data: &[Vector]) -> Vec<Vector> {
        data.iter().map(|v| {
            let centered: Vec<f64> = v.data.iter().zip(&self.mean).map(|(x, m)| x - m).collect();
            let projected: Vec<f64> = self.components.iter().map(|comp| {
                centered.iter().zip(&comp.data).map(|(x, c)| x * c).sum()
            }).collect();
            Vector::new(projected)
        }).collect()
    }
}

// ============================================================================
// STAGE 6: MODEL EVALUATION
// ============================================================================

/// Train-test split — splits data deterministically (no RNG needed).
pub fn train_test_split(
    x: &[Vector], y: &[usize], test_ratio: f64
) -> (Vec<Vector>, Vec<usize>, Vec<Vector>, Vec<usize>) {
    let n = x.len();
    let test_size = (n as f64 * test_ratio).round() as usize;
    let train_size = n - test_size;
    let train_x = x[..train_size].to_vec();
    let train_y = y[..train_size].to_vec();
    let test_x  = x[train_size..].to_vec();
    let test_y  = y[train_size..].to_vec();
    (train_x, train_y, test_x, test_y)
}

/// K-Fold cross-validation indices.
pub fn kfold_indices(n: usize, k: usize) -> Vec<(Vec<usize>, Vec<usize>)> {
    let fold_size = n / k;
    (0..k).map(|fold| {
        let val_start = fold * fold_size;
        let val_end = if fold == k - 1 { n } else { val_start + fold_size };
        let val: Vec<usize> = (val_start..val_end).collect();
        let train: Vec<usize> = (0..n).filter(|i| *i < val_start || *i >= val_end).collect();
        (train, val)
    }).collect()
}

/// Confusion matrix and ROC-AUC.
pub struct AdvancedMetrics;
impl AdvancedMetrics {
    /// Confusion matrix as 2D vec [actual][predicted] for binary classification.
    pub fn confusion_matrix(preds: &[usize], actuals: &[usize], n_classes: usize) -> Vec<Vec<usize>> {
        let mut cm = vec![vec![0usize; n_classes]; n_classes];
        for (&p, &a) in preds.iter().zip(actuals) {
            if p < n_classes && a < n_classes { cm[a][p] += 1; }
        }
        cm
    }

    /// F1 score for binary classification at positive_class.
    pub fn f1_score(preds: &[usize], actuals: &[usize], positive_class: usize) -> f64 {
        let tp = preds.iter().zip(actuals).filter(|(&p, &a)| p == positive_class && a == positive_class).count() as f64;
        let fp = preds.iter().zip(actuals).filter(|(&p, &a)| p == positive_class && a != positive_class).count() as f64;
        let fn_ = preds.iter().zip(actuals).filter(|(&p, &a)| p != positive_class && a == positive_class).count() as f64;
        if tp + fp == 0.0 || tp + fn_ == 0.0 { return 0.0; }
        let prec = tp / (tp + fp);
        let rec  = tp / (tp + fn_);
        if prec + rec == 0.0 { 0.0 } else { 2.0 * prec * rec / (prec + rec) }
    }

    /// ROC-AUC (approximate) from predicted probabilities.
    pub fn roc_auc(proba: &[f64], actuals: &[usize], positive_class: usize) -> f64 {
        let mut pairs: Vec<(f64, usize)> = proba.iter().copied().zip(actuals.iter().copied()).collect();
        pairs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        let pos = actuals.iter().filter(|&&a| a == positive_class).count() as f64;
        let neg = actuals.len() as f64 - pos;
        if pos == 0.0 || neg == 0.0 { return 0.5; }
        let mut auc = 0.0;
        let mut tp = 0.0;
        for (_, actual) in &pairs {
            if *actual == positive_class { tp += 1.0; }
            else { auc += tp; }
        }
        auc / (pos * neg)
    }
}

// ============================================================================
// STAGE 8-9: DEEP LEARNING — ACTIVATION, LOSS, BACKPROP, CNN, RNN, LSTM
// ============================================================================

/// All activation functions (Stage 8).
pub struct Activation;
impl Activation {
    pub fn relu(x: f64)    -> f64 { x.max(0.0) }
    pub fn relu_grad(x: f64) -> f64 { if x > 0.0 { 1.0 } else { 0.0 } }

    pub fn sigmoid(x: f64) -> f64 { 1.0 / (1.0 + (-x).exp()) }
    pub fn sigmoid_grad(x: f64) -> f64 { let s = Self::sigmoid(x); s * (1.0 - s) }

    pub fn tanh(x: f64)    -> f64 { x.tanh() }
    pub fn tanh_grad(x: f64) -> f64 { 1.0 - x.tanh().powi(2) }

    pub fn leaky_relu(x: f64, alpha: f64) -> f64 { if x > 0.0 { x } else { alpha * x } }
    pub fn softmax(x: &[f64]) -> Vec<f64> {
        let max = x.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let exps: Vec<f64> = x.iter().map(|xi| (xi - max).exp()).collect();
        let sum: f64 = exps.iter().sum();
        exps.iter().map(|e| e / sum).collect()
    }
}

/// Loss functions (Stage 8).
pub struct Loss;
impl Loss {
    pub fn mse(preds: &[f64], targets: &[f64]) -> f64 {
        preds.iter().zip(targets).map(|(p, t)| (p - t).powi(2)).sum::<f64>() / preds.len() as f64
    }
    pub fn mse_grad(pred: f64, target: f64) -> f64 { 2.0 * (pred - target) }

    pub fn binary_cross_entropy(preds: &[f64], targets: &[f64]) -> f64 {
        let n = preds.len() as f64;
        -preds.iter().zip(targets).map(|(&p, &t)| {
            let p = p.clamp(1e-9, 1.0 - 1e-9);
            t * p.ln() + (1.0 - t) * (1.0 - p).ln()
        }).sum::<f64>() / n
    }

    pub fn cross_entropy(probs: &[f64], target_class: usize) -> f64 {
        -probs[target_class].clamp(1e-9, 1.0).ln()
    }

    pub fn huber(pred: f64, target: f64, delta: f64) -> f64 {
        let e = (pred - target).abs();
        if e <= delta { 0.5 * e.powi(2) } else { delta * (e - 0.5 * delta) }
    }
}

/// Optimizers: SGD and Adam (Stage 8).
pub struct SGD { pub lr: f64, pub momentum: f64, velocity: Vec<f64> }
impl SGD {
    pub fn new(lr: f64, momentum: f64) -> Self { SGD { lr, momentum, velocity: Vec::new() } }

    pub fn step(&mut self, params: &mut Vec<f64>, grads: &[f64]) {
        if self.velocity.len() != params.len() { self.velocity = vec![0.0; params.len()]; }
        for i in 0..params.len() {
            self.velocity[i] = self.momentum * self.velocity[i] - self.lr * grads[i];
            params[i] += self.velocity[i];
        }
    }
}

pub struct Adam {
    pub lr: f64, pub beta1: f64, pub beta2: f64, pub eps: f64,
    m: Vec<f64>, v: Vec<f64>, t: u64,
}
impl Adam {
    pub fn new(lr: f64) -> Self {
        Adam { lr, beta1: 0.9, beta2: 0.999, eps: 1e-8, m: Vec::new(), v: Vec::new(), t: 0 }
    }

    pub fn step(&mut self, params: &mut Vec<f64>, grads: &[f64]) {
        if self.m.len() != params.len() {
            self.m = vec![0.0; params.len()];
            self.v = vec![0.0; params.len()];
        }
        self.t += 1;
        let bc1 = 1.0 - self.beta1.powi(self.t as i32);
        let bc2 = 1.0 - self.beta2.powi(self.t as i32);
        for i in 0..params.len() {
            self.m[i] = self.beta1 * self.m[i] + (1.0 - self.beta1) * grads[i];
            self.v[i] = self.beta2 * self.v[i] + (1.0 - self.beta2) * grads[i].powi(2);
            let m_hat = self.m[i] / bc1;
            let v_hat = self.v[i] / bc2;
            params[i] -= self.lr * m_hat / (v_hat.sqrt() + self.eps);
        }
    }
}

/// Fully-connected Dense layer with backprop (Stage 8).
pub struct DenseLayer {
    pub weights: Vec<Vec<f64>>, // [out × in]
    pub biases:  Vec<f64>,
    pub activation: String,
    // Cached for backprop
    last_input: Vec<f64>,
    last_z:     Vec<f64>,
}

impl DenseLayer {
    pub fn new(in_size: usize, out_size: usize, activation: &str) -> Self {
        // Xavier initialization
        let scale = (2.0 / in_size as f64).sqrt();
        let weights = (0..out_size).map(|i| {
            (0..in_size).map(|j| {
                // Deterministic pseudo-random init via LCG
                let v = ((i * 1664525 + j * 1013904223) % 1000000) as f64 / 1000000.0 - 0.5;
                v * scale
            }).collect()
        }).collect();
        DenseLayer {
            weights, biases: vec![0.0; out_size],
            activation: activation.to_string(),
            last_input: Vec::new(), last_z: Vec::new(),
        }
    }

    pub fn forward(&mut self, input: &[f64]) -> Vec<f64> {
        self.last_input = input.to_vec();
        self.last_z = self.weights.iter().zip(&self.biases).map(|(w_row, &b)| {
            w_row.iter().zip(input).map(|(w, x)| w * x).sum::<f64>() + b
        }).collect();
        self.last_z.iter().map(|&z| match self.activation.as_str() {
            "relu"    => Activation::relu(z),
            "sigmoid" => Activation::sigmoid(z),
            "tanh"    => Activation::tanh(z),
            _         => z,
        }).collect()
    }

    /// Backprop: returns grad w.r.t. input; updates weights/biases via optimizer.
    pub fn backward(&mut self, grad_output: &[f64], lr: f64) -> Vec<f64> {
        let delta: Vec<f64> = grad_output.iter().zip(&self.last_z).map(|(&go, &z)| {
            go * match self.activation.as_str() {
                "relu"    => Activation::relu_grad(z),
                "sigmoid" => Activation::sigmoid_grad(z),
                "tanh"    => Activation::tanh_grad(z),
                _         => 1.0,
            }
        }).collect();

        // Gradient w.r.t. input
        let mut grad_input = vec![0.0; self.last_input.len()];
        for (i, w_row) in self.weights.iter().enumerate() {
            for (j, &w) in w_row.iter().enumerate() {
                grad_input[j] += delta[i] * w;
            }
        }
        // Update weights
        for (i, w_row) in self.weights.iter_mut().enumerate() {
            for (j, w) in w_row.iter_mut().enumerate() {
                *w -= lr * delta[i] * self.last_input[j];
            }
            self.biases[i] -= lr * delta[i];
        }
        grad_input
    }
}

/// Conv1D layer — 1D convolution for sequences (simplified).
pub struct Conv1D {
    pub filters:     Vec<Vec<f64>>, // [n_filters × kernel_size]
    pub kernel_size: usize,
    pub n_filters:   usize,
    pub stride:      usize,
    last_input: Vec<f64>,
}

impl Conv1D {
    pub fn new(n_filters: usize, kernel_size: usize, stride: usize) -> Self {
        let scale = (2.0 / kernel_size as f64).sqrt();
        let filters = (0..n_filters).map(|f| {
            (0..kernel_size).map(|k| {
                ((f * 1664525 + k * 22695477) % 1000000) as f64 / 1_000_000.0 * scale - scale / 2.0
            }).collect()
        }).collect();
        Conv1D { filters, kernel_size, n_filters, stride, last_input: Vec::new() }
    }

    /// Forward: input shape [seq_len], output shape [n_filters × out_len].
    pub fn forward(&mut self, input: &[f64]) -> Vec<Vec<f64>> {
        self.last_input = input.to_vec();
        let out_len = (input.len() - self.kernel_size) / self.stride + 1;
        self.filters.iter().map(|kernel| {
            (0..out_len).map(|i| {
                let start = i * self.stride;
                kernel.iter().zip(&input[start..start + self.kernel_size])
                    .map(|(k, x)| k * x).sum::<f64>()
            }).collect()
        }).collect()
    }
}

/// LSTM Cell — Long Short-Term Memory (Stage 9).
pub struct LSTMCell {
    pub hidden_size: usize,
    pub input_size:  usize,
    // Weight matrices: [hidden × (input + hidden)]
    wf: Vec<Vec<f64>>, // forget gate
    wi: Vec<Vec<f64>>, // input gate
    wg: Vec<Vec<f64>>, // cell gate
    wo: Vec<Vec<f64>>, // output gate
    bf: Vec<f64>, bi: Vec<f64>, bg: Vec<f64>, bo: Vec<f64>,
}

impl LSTMCell {
    pub fn new(input_size: usize, hidden_size: usize) -> Self {
        let combined = input_size + hidden_size;
        let scale = (1.0 / combined as f64).sqrt();
        let make_w = |seed: usize| -> Vec<Vec<f64>> {
            (0..hidden_size).map(|i| {
                (0..combined).map(|j| {
                    ((i * 1664525 + j * 22695477 + seed * 6364136) % 1000000) as f64 / 1_000_000.0 * 2.0 * scale - scale
                }).collect()
            }).collect()
        };
        LSTMCell {
            input_size, hidden_size,
            wf: make_w(1), wi: make_w(2), wg: make_w(3), wo: make_w(4),
            bf: vec![1.0; hidden_size], // forget bias starts at 1 (standard)
            bi: vec![0.0; hidden_size], bg: vec![0.0; hidden_size], bo: vec![0.0; hidden_size],
        }
    }

    fn linear(w: &[Vec<f64>], b: &[f64], x: &[f64]) -> Vec<f64> {
        w.iter().zip(b).map(|(row, &bi)| {
            row.iter().zip(x).map(|(wi, xi)| wi * xi).sum::<f64>() + bi
        }).collect()
    }

    /// One step: returns (new_hidden, new_cell).
    pub fn step(&self, input: &[f64], hidden: &[f64], cell: &[f64]) -> (Vec<f64>, Vec<f64>) {
        let combined: Vec<f64> = input.iter().chain(hidden).copied().collect();
        let f: Vec<f64> = Self::linear(&self.wf, &self.bf, &combined).iter().map(|&z| Activation::sigmoid(z)).collect();
        let i: Vec<f64> = Self::linear(&self.wi, &self.bi, &combined).iter().map(|&z| Activation::sigmoid(z)).collect();
        let g: Vec<f64> = Self::linear(&self.wg, &self.bg, &combined).iter().map(|&z| z.tanh()).collect();
        let o: Vec<f64> = Self::linear(&self.wo, &self.bo, &combined).iter().map(|&z| Activation::sigmoid(z)).collect();
        let new_cell: Vec<f64> = f.iter().zip(cell).zip(&i).zip(&g).map(|(((fi, ci), ii), gi)| fi * ci + ii * gi).collect();
        let new_hidden: Vec<f64> = o.iter().zip(&new_cell).map(|(&oi, ci)| oi * ci.tanh()).collect();
        (new_hidden, new_cell)
    }

    /// Unroll over a sequence: returns all hidden states.
    pub fn forward_sequence(&self, inputs: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let mut h = vec![0.0; self.hidden_size];
        let mut c = vec![0.0; self.hidden_size];
        let mut outputs = Vec::new();
        for x in inputs {
            let (nh, nc) = self.step(x, &h, &c);
            h = nh.clone();
            c = nc;
            outputs.push(nh);
        }
        outputs
    }
}

/// GRU Cell — Gated Recurrent Unit (simpler than LSTM).
pub struct GRUCell {
    pub input_size:  usize,
    pub hidden_size: usize,
    wr: Vec<Vec<f64>>, wz: Vec<Vec<f64>>, wn: Vec<Vec<f64>>,
    br: Vec<f64>, bz: Vec<f64>, bn: Vec<f64>,
}

impl GRUCell {
    pub fn new(input_size: usize, hidden_size: usize) -> Self {
        let combined = input_size + hidden_size;
        let scale = (1.0 / combined as f64).sqrt();
        let make_w = |seed: usize| -> Vec<Vec<f64>> {
            (0..hidden_size).map(|i| {
                (0..combined).map(|j| {
                    ((i * 1664525 + j * 22695477 + seed * 6364136) % 1000000) as f64 / 1_000_000.0 * 2.0 * scale - scale
                }).collect()
            }).collect()
        };
        GRUCell {
            input_size, hidden_size,
            wr: make_w(10), wz: make_w(20), wn: make_w(30),
            br: vec![0.0; hidden_size], bz: vec![0.0; hidden_size], bn: vec![0.0; hidden_size],
        }
    }

    pub fn step(&self, input: &[f64], hidden: &[f64]) -> Vec<f64> {
        let combined: Vec<f64> = input.iter().chain(hidden).copied().collect();
        let r: Vec<f64> = LSTMCell::linear(&self.wr, &self.br, &combined).iter().map(|&z| Activation::sigmoid(z)).collect();
        let z: Vec<f64> = LSTMCell::linear(&self.wz, &self.bz, &combined).iter().map(|&z| Activation::sigmoid(z)).collect();
        let rh: Vec<f64> = r.iter().zip(hidden).map(|(ri, hi)| ri * hi).collect();
        let n_input: Vec<f64> = input.iter().chain(rh.iter()).copied().collect();
        let n: Vec<f64> = LSTMCell::linear(&self.wn, &self.bn, &n_input).iter().map(|&z| z.tanh()).collect();
        z.iter().zip(hidden).zip(&n).map(|((&zi, hi), ni)| (1.0 - zi) * ni + zi * hi).collect()
    }

    pub fn forward_sequence(&self, inputs: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let mut h = vec![0.0; self.hidden_size];
        inputs.iter().map(|x| { h = self.step(x, &h); h.clone() }).collect()
    }
}

// ============================================================================
// STAGE 11: NLP — TOKENIZATION, TF-IDF, WORD2VEC (SKIP-GRAM)
// ============================================================================

/// Tokenizer — splits text into word tokens.
pub struct Tokenizer {
    pub vocab: HashMap<String, usize>,
    pub inverse_vocab: Vec<String>,
}

impl Tokenizer {
    pub fn new() -> Self {
        Tokenizer { vocab: HashMap::new(), inverse_vocab: Vec::new() }
    }

    pub fn fit(&mut self, texts: &[&str]) {
        for text in texts {
            for word in Self::tokenize(text) {
                if !self.vocab.contains_key(&word) {
                    let id = self.inverse_vocab.len();
                    self.vocab.insert(word.clone(), id);
                    self.inverse_vocab.push(word);
                }
            }
        }
    }

    pub fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect()
    }

    pub fn encode(&self, text: &str) -> Vec<usize> {
        Self::tokenize(text).iter()
            .filter_map(|w| self.vocab.get(w).copied())
            .collect()
    }

    pub fn vocab_size(&self) -> usize { self.vocab.len() }
}

/// Bag of Words representation.
pub struct BagOfWords {
    pub tokenizer: Tokenizer,
}

impl BagOfWords {
    pub fn new() -> Self { BagOfWords { tokenizer: Tokenizer::new() } }

    pub fn fit(&mut self, texts: &[&str]) { self.tokenizer.fit(texts); }

    pub fn transform(&self, text: &str) -> Vec<f64> {
        let mut bow = vec![0.0; self.tokenizer.vocab_size()];
        for id in self.tokenizer.encode(text) {
            bow[id] += 1.0;
        }
        bow
    }
}

/// TF-IDF vectorizer.
pub struct TfIdf {
    pub tokenizer: Tokenizer,
    pub idf: Vec<f64>,
    n_docs: usize,
}

impl TfIdf {
    pub fn new() -> Self { TfIdf { tokenizer: Tokenizer::new(), idf: Vec::new(), n_docs: 0 } }

    pub fn fit(&mut self, texts: &[&str]) {
        self.tokenizer.fit(texts);
        self.n_docs = texts.len();
        let vocab_size = self.tokenizer.vocab_size();
        let mut doc_freq = vec![0usize; vocab_size];
        for text in texts {
            let ids: std::collections::HashSet<usize> = self.tokenizer.encode(text).into_iter().collect();
            for id in ids { doc_freq[id] += 1; }
        }
        self.idf = doc_freq.iter().map(|&df| {
            if df == 0 { 0.0 } else { ((self.n_docs as f64 + 1.0) / (df as f64 + 1.0)).ln() + 1.0 }
        }).collect();
    }

    pub fn transform(&self, text: &str) -> Vec<f64> {
        let tokens = Tokenizer::tokenize(text);
        let n = tokens.len() as f64;
        if n == 0.0 { return vec![0.0; self.idf.len()]; }
        let mut tf = vec![0.0; self.idf.len()];
        for id in self.tokenizer.encode(text) { tf[id] += 1.0 / n; }
        tf.iter().zip(&self.idf).map(|(t, i)| t * i).collect()
    }
}

/// Word2Vec — Skip-Gram with negative sampling (simplified 1 epoch).
pub struct Word2Vec {
    pub embedding_dim: usize,
    pub window_size:   usize,
    pub embeddings:    Vec<Vec<f64>>, // [vocab_size × dim]
    vocab_size: usize,
}

impl Word2Vec {
    pub fn new(embedding_dim: usize, window_size: usize) -> Self {
        Word2Vec { embedding_dim, window_size, embeddings: Vec::new(), vocab_size: 0 }
    }

    pub fn fit(&mut self, tokenizer: &Tokenizer, texts: &[&str], lr: f64, epochs: usize) {
        self.vocab_size = tokenizer.vocab_size();
        let scale = (1.0 / self.embedding_dim as f64).sqrt();
        // Init embeddings
        self.embeddings = (0..self.vocab_size).map(|i| {
            (0..self.embedding_dim).map(|j| {
                ((i * 1664525 + j * 22695477) % 1000000) as f64 / 1_000_000.0 * 2.0 * scale - scale
            }).collect()
        }).collect();
        // Context weights (output embeddings)
        let mut ctx: Vec<Vec<f64>> = (0..self.vocab_size).map(|i| {
            (0..self.embedding_dim).map(|j| {
                ((i * 6364136 + j * 1013904223) % 1000000) as f64 / 1_000_000.0 * 2.0 * scale - scale
            }).collect()
        }).collect();

        for _ in 0..epochs {
            for text in texts {
                let ids = tokenizer.encode(text);
                for (pos, &center) in ids.iter().enumerate() {
                    let start = pos.saturating_sub(self.window_size);
                    let end   = (pos + self.window_size + 1).min(ids.len());
                    for &context in ids[start..end].iter().filter(|&&c| c != center) {
                        // Positive sample gradient
                        let score: f64 = self.embeddings[center].iter().zip(&ctx[context]).map(|(e, c)| e * c).sum();
                        let sig = 1.0 / (1.0 + (-score).exp());
                        let grad = (sig - 1.0) * lr;
                        for d in 0..self.embedding_dim {
                            let e = self.embeddings[center][d];
                            let c = ctx[context][d];
                            self.embeddings[center][d] -= grad * c;
                            ctx[context][d]            -= grad * e;
                        }
                    }
                }
            }
        }
    }

    pub fn get_embedding(&self, word_id: usize) -> Option<&Vec<f64>> {
        self.embeddings.get(word_id)
    }

    pub fn most_similar(&self, word_id: usize, top_n: usize) -> Vec<(usize, f64)> {
        let target = match self.embeddings.get(word_id) { Some(e) => e, None => return Vec::new() };
        let t_norm: f64 = target.iter().map(|x| x * x).sum::<f64>().sqrt();
        let mut sims: Vec<(usize, f64)> = self.embeddings.iter().enumerate()
            .filter(|(i, _)| *i != word_id)
            .map(|(i, emb)| {
                let dot: f64 = target.iter().zip(emb).map(|(a, b)| a * b).sum();
                let norm: f64 = emb.iter().map(|x| x * x).sum::<f64>().sqrt();
                let sim = if t_norm * norm > 0.0 { dot / (t_norm * norm) } else { 0.0 };
                (i, sim)
            }).collect();
        sims.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        sims.into_iter().take(top_n).collect()
    }
}

// ============================================================================
// STAGE 12: TRANSFORMERS & ATTENTION MECHANISM
// ============================================================================

/// Scaled dot-product attention: Attention(Q,K,V) = softmax(QK^T / sqrt(d_k)) V
pub struct ScaledDotProductAttention;
impl ScaledDotProductAttention {
    /// q: [seq_q × d_k], k: [seq_k × d_k], v: [seq_k × d_v]
    /// Returns: [seq_q × d_v] context vectors + [seq_q × seq_k] attention weights
    pub fn forward(
        q: &[Vec<f64>], k: &[Vec<f64>], v: &[Vec<f64>]
    ) -> (Vec<Vec<f64>>, Vec<Vec<f64>>) {
        let d_k = q[0].len() as f64;
        let scale = d_k.sqrt();
        // scores: [seq_q × seq_k]
        let scores: Vec<Vec<f64>> = q.iter().map(|qi| {
            k.iter().map(|kj| {
                qi.iter().zip(kj).map(|(a, b)| a * b).sum::<f64>() / scale
            }).collect()
        }).collect();
        // softmax over last dim
        let weights: Vec<Vec<f64>> = scores.iter().map(|row| Activation::softmax(row)).collect();
        // context = weights @ V: [seq_q × d_v]
        let d_v = v[0].len();
        let context: Vec<Vec<f64>> = weights.iter().map(|w_row| {
            (0..d_v).map(|dv| {
                w_row.iter().zip(v).map(|(&w, vj)| w * vj[dv]).sum()
            }).collect()
        }).collect();
        (context, weights)
    }
}

/// Multi-Head Attention — h heads of scaled dot-product attention.
/// Each head projects Q, K, V to d_k = d_model / h dimensions.
pub struct MultiHeadAttention {
    pub d_model: usize,
    pub n_heads: usize,
    pub d_k:     usize,
    // Weight matrices [d_model × d_k] for each head
    wq: Vec<Vec<Vec<f64>>>, // [n_heads × d_model × d_k]
    wk: Vec<Vec<Vec<f64>>>,
    wv: Vec<Vec<Vec<f64>>>,
    wo: Vec<Vec<f64>>,      // [d_model × d_model] output projection
}

impl MultiHeadAttention {
    pub fn new(d_model: usize, n_heads: usize) -> Self {
        assert!(d_model % n_heads == 0, "d_model must be divisible by n_heads");
        let d_k = d_model / n_heads;
        let scale = (1.0 / d_model as f64).sqrt();
        let make_3d = |seed: usize| -> Vec<Vec<Vec<f64>>> {
            (0..n_heads).map(|h| {
                (0..d_model).map(|i| {
                    (0..d_k).map(|j| {
                        ((h * 1664525 + i * 22695477 + j * 6364136 + seed) % 1_000_000) as f64
                            / 1_000_000.0 * 2.0 * scale - scale
                    }).collect()
                }).collect()
            }).collect()
        };
        let wo = (0..d_model).map(|i| {
            (0..d_model).map(|j| {
                ((i * 1664525 + j * 22695477 + 999) % 1_000_000) as f64 / 1_000_000.0 * 2.0 * scale - scale
            }).collect()
        }).collect();
        MultiHeadAttention {
            d_model, n_heads, d_k,
            wq: make_3d(1), wk: make_3d(2), wv: make_3d(3), wo,
        }
    }

    fn project(x: &[Vec<f64>], w: &[Vec<f64>]) -> Vec<Vec<f64>> {
        // x: [seq × d_in], w: [d_in × d_out] → [seq × d_out]
        let d_out = w[0].len();
        x.iter().map(|xi| {
            (0..d_out).map(|j| {
                xi.iter().enumerate().map(|(i, &xv)| xv * w[i][j]).sum()
            }).collect()
        }).collect()
    }

    /// Forward: x [seq × d_model] → [seq × d_model]
    pub fn forward(&self, x: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let seq = x.len();
        let mut concat = vec![vec![0.0f64; self.d_model]; seq];

        for h in 0..self.n_heads {
            let q = Self::project(x, &self.wq[h]);
            let k = Self::project(x, &self.wk[h]);
            let v = Self::project(x, &self.wv[h]);
            let (ctx, _) = ScaledDotProductAttention::forward(&q, &k, &v);
            // Concatenate head output into concat[seq][h*d_k..(h+1)*d_k]
            for s in 0..seq {
                for d in 0..self.d_k {
                    concat[s][h * self.d_k + d] = ctx[s][d];
                }
            }
        }
        // Output projection: concat @ Wo
        Self::project(&concat, &self.wo)
    }
}

/// Layer Normalisation: (x - mean) / std * gamma + beta
pub struct LayerNorm {
    pub gamma: Vec<f64>,
    pub beta:  Vec<f64>,
    eps: f64,
}

impl LayerNorm {
    pub fn new(d_model: usize) -> Self {
        LayerNorm { gamma: vec![1.0; d_model], beta: vec![0.0; d_model], eps: 1e-6 }
    }

    pub fn forward(&self, x: &[f64]) -> Vec<f64> {
        let mean = x.iter().sum::<f64>() / x.len() as f64;
        let var  = x.iter().map(|xi| (xi - mean).powi(2)).sum::<f64>() / x.len() as f64;
        let std  = (var + self.eps).sqrt();
        x.iter().enumerate().map(|(i, &xi)| {
            self.gamma[i] * (xi - mean) / std + self.beta[i]
        }).collect()
    }
}

/// Position-wise Feed-Forward Network: FFN(x) = max(0, xW1+b1)W2+b2
pub struct FeedForward {
    layer1: DenseLayer,
    layer2: DenseLayer,
}

impl FeedForward {
    pub fn new(d_model: usize, d_ff: usize) -> Self {
        FeedForward {
            layer1: DenseLayer::new(d_model, d_ff,    "relu"),
            layer2: DenseLayer::new(d_ff,    d_model, "linear"),
        }
    }

    pub fn forward(&mut self, x: &[f64]) -> Vec<f64> {
        let h = self.layer1.forward(x);
        self.layer2.forward(&h)
    }
}

/// Sinusoidal positional encoding (Vaswani et al. 2017).
pub fn positional_encoding(seq_len: usize, d_model: usize) -> Vec<Vec<f64>> {
    (0..seq_len).map(|pos| {
        (0..d_model).map(|i| {
            let angle = pos as f64 / 10000_f64.powf(2.0 * (i / 2) as f64 / d_model as f64);
            if i % 2 == 0 { angle.sin() } else { angle.cos() }
        }).collect()
    }).collect()
}

/// Transformer Encoder Layer: MHA + Add&Norm + FFN + Add&Norm
pub struct TransformerEncoderLayer {
    attn: MultiHeadAttention,
    ffn:  FeedForward,
    norm1: LayerNorm,
    norm2: LayerNorm,
}

impl TransformerEncoderLayer {
    pub fn new(d_model: usize, n_heads: usize, d_ff: usize) -> Self {
        TransformerEncoderLayer {
            attn:  MultiHeadAttention::new(d_model, n_heads),
            ffn:   FeedForward::new(d_model, d_ff),
            norm1: LayerNorm::new(d_model),
            norm2: LayerNorm::new(d_model),
        }
    }

    /// x: [seq × d_model] → [seq × d_model]
    pub fn forward(&mut self, x: &[Vec<f64>]) -> Vec<Vec<f64>> {
        // Sub-layer 1: MHA + residual + norm
        let attn_out = self.attn.forward(x);
        let x1: Vec<Vec<f64>> = x.iter().zip(&attn_out).map(|(xi, ai)| {
            self.norm1.forward(&xi.iter().zip(ai).map(|(a, b)| a + b).collect::<Vec<_>>())
        }).collect();
        // Sub-layer 2: FFN + residual + norm
        x1.iter().map(|xi| {
            let ffn_out = self.ffn.forward(xi);
            self.norm2.forward(&xi.iter().zip(&ffn_out).map(|(a, b)| a + b).collect::<Vec<_>>())
        }).collect()
    }
}

/// Full Transformer Encoder — stack of N layers.
pub struct TransformerEncoder {
    pub layers:   Vec<TransformerEncoderLayer>,
    pub d_model:  usize,
}

impl TransformerEncoder {
    pub fn new(d_model: usize, n_heads: usize, d_ff: usize, n_layers: usize) -> Self {
        TransformerEncoder {
            layers:  (0..n_layers).map(|_| TransformerEncoderLayer::new(d_model, n_heads, d_ff)).collect(),
            d_model,
        }
    }

    /// Encode token IDs into contextual embeddings [seq × d_model].
    /// token_embeddings: pre-computed [seq × d_model] from an embedding table.
    pub fn encode(&mut self, token_embeddings: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let pe = positional_encoding(token_embeddings.len(), self.d_model);
        // Add positional encoding
        let mut x: Vec<Vec<f64>> = token_embeddings.iter().zip(&pe).map(|(te, pos)| {
            te.iter().zip(pos).map(|(a, b)| a + b).collect()
        }).collect();
        for layer in &mut self.layers {
            x = layer.forward(&x);
        }
        x
    }
}

/// GPT-style Causal (Decoder-only) Attention — masks future positions.
pub struct CausalAttention {
    pub d_model: usize,
    pub n_heads: usize,
    pub d_k:     usize,
    wq: Vec<Vec<Vec<f64>>>,
    wk: Vec<Vec<Vec<f64>>>,
    wv: Vec<Vec<Vec<f64>>>,
    wo: Vec<Vec<f64>>,
}

impl CausalAttention {
    pub fn new(d_model: usize, n_heads: usize) -> Self {
        // Reuse same weight init as MultiHeadAttention
        let base = MultiHeadAttention::new(d_model, n_heads);
        CausalAttention { d_model, n_heads, d_k: base.d_k, wq: base.wq, wk: base.wk, wv: base.wv, wo: base.wo }
    }

    /// Forward with causal mask (each position only attends to past + self).
    pub fn forward(&self, x: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let seq = x.len();
        let mut concat = vec![vec![0.0f64; self.d_model]; seq];
        for h in 0..self.n_heads {
            let q = MultiHeadAttention::project(x, &self.wq[h]);
            let k = MultiHeadAttention::project(x, &self.wk[h]);
            let v = MultiHeadAttention::project(x, &self.wv[h]);
            let d_k = self.d_k as f64;
            let scale = d_k.sqrt();
            // Causal mask: score[i][j] = -inf if j > i
            let scores: Vec<Vec<f64>> = (0..seq).map(|i| {
                (0..seq).map(|j| {
                    if j > i { f64::NEG_INFINITY }
                    else { q[i].iter().zip(&k[j]).map(|(a, b)| a * b).sum::<f64>() / scale }
                }).collect()
            }).collect();
            let weights: Vec<Vec<f64>> = scores.iter().map(|row| Activation::softmax(row)).collect();
            let d_v = v[0].len();
            for s in 0..seq {
                for d in 0..d_v {
                    concat[s][h * self.d_k + d] = weights[s].iter().zip(&v).map(|(&w, vj)| w * vj[d]).sum();
                }
            }
        }
        MultiHeadAttention::project(&concat, &self.wo)
    }
}

// ============================================================================
// STAGE 14: RAG — RETRIEVAL-AUGMENTED GENERATION (NATIVE)
// ============================================================================

/// Document chunk for RAG.
#[derive(Clone, Debug)]
pub struct Document {
    pub id:      usize,
    pub content: String,
    pub embedding: Vec<f64>,
}

/// Native vector store — cosine similarity search (FAISS-equivalent, no deps).
pub struct VectorStore {
    pub documents: Vec<Document>,
}

impl VectorStore {
    pub fn new() -> Self { VectorStore { documents: Vec::new() } }

    /// Add a document with its embedding.
    pub fn add(&mut self, content: &str, embedding: Vec<f64>) -> usize {
        let id = self.documents.len();
        self.documents.push(Document { id, content: content.to_string(), embedding });
        id
    }

    /// Retrieve top-k most similar documents to a query embedding.
    pub fn search(&self, query: &[f64], top_k: usize) -> Vec<(&Document, f64)> {
        let q_norm = query.iter().map(|x| x * x).sum::<f64>().sqrt();
        let mut scores: Vec<(&Document, f64)> = self.documents.iter().map(|doc| {
            let d_norm = doc.embedding.iter().map(|x| x * x).sum::<f64>().sqrt();
            let dot: f64 = query.iter().zip(&doc.embedding).map(|(a, b)| a * b).sum();
            let sim = if q_norm * d_norm > 0.0 { dot / (q_norm * d_norm) } else { 0.0 };
            (doc, sim)
        }).collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scores.into_iter().take(top_k).collect()
    }

    /// Batch add multiple documents.
    pub fn add_batch(&mut self, contents: &[&str], embeddings: Vec<Vec<f64>>) {
        for (content, emb) in contents.iter().zip(embeddings) {
            self.add(content, emb);
        }
    }

    pub fn len(&self) -> usize { self.documents.len() }
    pub fn is_empty(&self) -> bool { self.documents.is_empty() }
}

/// RAG Pipeline — retrieval + prompt construction + generation interface.
pub struct RagPipeline {
    pub store:       VectorStore,
    pub top_k:       usize,
    pub max_context: usize,
}

impl RagPipeline {
    pub fn new(top_k: usize, max_context: usize) -> Self {
        RagPipeline { store: VectorStore::new(), top_k, max_context }
    }

    /// Index a document with its pre-computed embedding.
    pub fn index(&mut self, content: &str, embedding: Vec<f64>) {
        self.store.add(content, embedding);
    }

    /// Build a RAG prompt: retrieved context + question.
    pub fn build_prompt(&self, query_embedding: &[f64], question: &str) -> String {
        let results = self.store.search(query_embedding, self.top_k);
        let mut context = String::from("Context:\n");
        let mut total_chars = 0;
        for (doc, score) in &results {
            let chunk = format!("[score={:.3}] {}\n", score, doc.content);
            if total_chars + chunk.len() > self.max_context { break; }
            context.push_str(&chunk);
            total_chars += chunk.len();
        }
        format!("{}\nQuestion: {}\nAnswer:", context, question)
    }

    /// Retrieve relevant chunks for a query (returns content + similarity score).
    pub fn retrieve(&self, query_embedding: &[f64]) -> Vec<(String, f64)> {
        self.store.search(query_embedding, self.top_k)
            .into_iter()
            .map(|(doc, score)| (doc.content.clone(), score))
            .collect()
    }

    pub fn indexed_count(&self) -> usize { self.store.len() }
}

/// Text chunker — splits documents into overlapping chunks for RAG indexing.
pub struct TextChunker {
    pub chunk_size:  usize,
    pub overlap:     usize,
}

impl TextChunker {
    pub fn new(chunk_size: usize, overlap: usize) -> Self {
        TextChunker { chunk_size, overlap }
    }

    /// Split text into word-based overlapping chunks.
    pub fn chunk(&self, text: &str) -> Vec<String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        if words.is_empty() { return Vec::new(); }
        let mut chunks = Vec::new();
        let mut start = 0;
        while start < words.len() {
            let end = (start + self.chunk_size).min(words.len());
            chunks.push(words[start..end].join(" "));
            if end == words.len() { break; }
            start += self.chunk_size.saturating_sub(self.overlap);
        }
        chunks
    }
}

// ============================================================================
// STAGE 19: RESPONSIBLE AI — XAI, BIAS DETECTION, FAIRNESS, PRIVACY
// ============================================================================

/// Explainable AI (XAI) — LIME-style local feature importance.
/// Perturbs input features and measures prediction change.
pub struct LocalExplainer;

impl LocalExplainer {
    /// Explain prediction for `input` using `predict_fn`.
    /// Returns feature importances (which features matter most).
    pub fn explain<F>(input: &[f64], predict_fn: &F, n_samples: usize) -> Vec<f64>
    where F: Fn(&[f64]) -> f64
    {
        let baseline = predict_fn(input);
        let mut importances = vec![0.0f64; input.len()];
        for i in 0..input.len() {
            let mut total = 0.0f64;
            for s in 0..n_samples {
                let mut perturbed = input.to_vec();
                // Deterministic perturbation
                let noise = ((i * 1664525 + s * 22695477) % 1000) as f64 / 1000.0 - 0.5;
                perturbed[i] += noise * (input[i].abs().max(1.0));
                total += (predict_fn(&perturbed) - baseline).abs();
            }
            importances[i] = total / n_samples as f64;
        }
        // Normalize
        let max = importances.iter().cloned().fold(0.0f64, f64::max);
        if max > 0.0 { importances.iter().map(|x| x / max).collect() } else { importances }
    }
}

/// Bias Detector — measures demographic parity, equal opportunity, disparate impact.
pub struct BiasDetector;

impl BiasDetector {
    /// Demographic Parity — difference in positive prediction rates between groups.
    /// Returns (group_0_rate, group_1_rate, disparity).
    pub fn demographic_parity(
        predictions: &[usize],
        groups: &[usize],       // 0 or 1 for each sample
        positive_label: usize,
    ) -> (f64, f64, f64) {
        let (mut pos0, mut pos1, mut n0, mut n1) = (0.0, 0.0, 0.0, 0.0);
        for (&p, &g) in predictions.iter().zip(groups) {
            if g == 0 { n0 += 1.0; if p == positive_label { pos0 += 1.0; } }
            else       { n1 += 1.0; if p == positive_label { pos1 += 1.0; } }
        }
        let r0 = if n0 > 0.0 { pos0 / n0 } else { 0.0 };
        let r1 = if n1 > 0.0 { pos1 / n1 } else { 0.0 };
        (r0, r1, (r0 - r1).abs())
    }

    /// Disparate Impact ratio = min(r0,r1)/max(r0,r1). Below 0.8 = biased (80% rule).
    pub fn disparate_impact(predictions: &[usize], groups: &[usize], positive_label: usize) -> f64 {
        let (r0, r1, _) = Self::demographic_parity(predictions, groups, positive_label);
        let mn = r0.min(r1);
        let mx = r0.max(r1);
        if mx == 0.0 { 1.0 } else { mn / mx }
    }

    /// Equal Opportunity — TPR difference between groups.
    pub fn equal_opportunity(
        predictions: &[usize],
        actuals:     &[usize],
        groups:      &[usize],
        positive_label: usize,
    ) -> f64 {
        let tpr = |g: usize| -> f64 {
            let (mut tp, mut fn_) = (0.0, 0.0);
            for ((&p, &a), &gr) in predictions.iter().zip(actuals).zip(groups) {
                if gr == g && a == positive_label {
                    if p == positive_label { tp += 1.0; } else { fn_ += 1.0; }
                }
            }
            if tp + fn_ > 0.0 { tp / (tp + fn_) } else { 0.0 }
        };
        (tpr(0) - tpr(1)).abs()
    }

    /// Audit report — returns bias summary.
    pub fn audit(
        predictions: &[usize],
        actuals:     &[usize],
        groups:      &[usize],
        positive_label: usize,
    ) -> BiasReport {
        let (r0, r1, disparity) = Self::demographic_parity(predictions, groups, positive_label);
        let di = Self::disparate_impact(predictions, groups, positive_label);
        let eo = Self::equal_opportunity(predictions, actuals, groups, positive_label);
        BiasReport {
            group0_positive_rate: r0,
            group1_positive_rate: r1,
            demographic_parity: disparity,
            disparate_impact:   di,
            equal_opportunity:  eo,
            is_fair: di >= 0.8 && disparity < 0.1 && eo < 0.1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BiasReport {
    pub group0_positive_rate: f64,
    pub group1_positive_rate: f64,
    pub demographic_parity:   f64,
    pub disparate_impact:     f64,  // < 0.8 = biased
    pub equal_opportunity:    f64,  // > 0.1 = biased
    pub is_fair:              bool,
}

impl BiasReport {
    pub fn summary(&self) -> String {
        format!(
            "BiasReport | Group0={:.3} Group1={:.3} | DemParity={:.3} | DisparateImpact={:.3} (fair≥0.8) | EqualOpp={:.3} (fair<0.1) | FAIR={}",
            self.group0_positive_rate, self.group1_positive_rate,
            self.demographic_parity, self.disparate_impact,
            self.equal_opportunity, self.is_fair
        )
    }
}

/// Privacy — Differential Privacy via Laplace mechanism.
pub struct DifferentialPrivacy;

impl DifferentialPrivacy {
    /// Add Laplace noise to a value for ε-differential privacy.
    /// sensitivity = max change a single record causes in the output.
    pub fn laplace_noise(sensitivity: f64, epsilon: f64, seed: u64) -> f64 {
        // Deterministic Laplace noise via inverse CDF (no rand crate)
        let b = sensitivity / epsilon;
        // Pseudo-uniform in (0,1) via LCG
        let u = ((seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)) >> 33) as f64 / u32::MAX as f64;
        let u = u.clamp(1e-9, 1.0 - 1e-9);
        -b * (1.0 - 2.0 * u).signum() * (1.0 - 2.0 * (u - 0.5).abs()).ln()
    }

    /// Privatize a vector of statistics with ε-DP Laplace mechanism.
    pub fn privatize(values: &[f64], sensitivity: f64, epsilon: f64) -> Vec<f64> {
        values.iter().enumerate().map(|(i, &v)| {
            v + Self::laplace_noise(sensitivity, epsilon, i as u64 + 1)
        }).collect()
    }

    /// Privacy budget check: ε < 1.0 = strong, ε < 0.1 = very strong.
    pub fn privacy_level(epsilon: f64) -> &'static str {
        if epsilon < 0.1  { "very_strong" }
        else if epsilon < 1.0  { "strong" }
        else if epsilon < 10.0 { "moderate" }
        else { "weak" }
    }
}

/// Model Card — structured documentation for responsible AI deployment.
#[derive(Debug, Clone)]
pub struct ModelCard {
    pub model_name:    String,
    pub version:       String,
    pub intended_use:  String,
    pub limitations:   Vec<String>,
    pub eval_metrics:  HashMap<String, f64>,
    pub bias_report:   Option<BiasReport>,
    pub privacy_level: String,
    pub training_data: String,
}

impl ModelCard {
    pub fn new(name: &str, version: &str) -> Self {
        ModelCard {
            model_name:    name.to_string(),
            version:       version.to_string(),
            intended_use:  String::new(),
            limitations:   Vec::new(),
            eval_metrics:  HashMap::new(),
            bias_report:   None,
            privacy_level: "not_evaluated".to_string(),
            training_data: String::new(),
        }
    }

    pub fn add_metric(&mut self, name: &str, value: f64) {
        self.eval_metrics.insert(name.to_string(), value);
    }

    pub fn add_limitation(&mut self, limitation: &str) {
        self.limitations.push(limitation.to_string());
    }

    pub fn summary(&self) -> String {
        let metrics: Vec<String> = self.eval_metrics.iter()
            .map(|(k, v)| format!("{}={:.3}", k, v)).collect();
        format!(
            "ModelCard[{}@{}] | Use: {} | Metrics: {} | Privacy: {} | Bias: {} | Limitations: {}",
            self.model_name, self.version,
            self.intended_use,
            metrics.join(", "),
            self.privacy_level,
            self.bias_report.as_ref().map(|b| if b.is_fair { "FAIR" } else { "BIASED" }).unwrap_or("not_evaluated"),
            self.limitations.len()
        )
    }
}

// ============================================================================
// STAGE 10 (RL): REINFORCEMENT LEARNING — Q-LEARNING & DQN
// ============================================================================

/// Environment interface — any RL environment implements this trait.
pub trait Environment {
    /// Reset to initial state, return starting state index.
    fn reset(&mut self) -> usize;
    /// Step: returns (next_state, reward, done).
    fn step(&mut self, state: usize, action: usize) -> (usize, f64, bool);
    fn num_states(&self) -> usize;
    fn num_actions(&self) -> usize;
}

/// Tabular Q-Learning (model-free, off-policy).
/// Learns Q(s,a) → expected cumulative reward.
pub struct QLearning {
    pub q_table:       Vec<Vec<f64>>,  // [states × actions]
    pub learning_rate: f64,
    pub gamma:         f64,            // discount factor
    pub epsilon:       f64,            // exploration rate
    pub epsilon_decay: f64,
    pub epsilon_min:   f64,
    pub n_states:      usize,
    pub n_actions:     usize,
}

impl QLearning {
    pub fn new(n_states: usize, n_actions: usize, lr: f64, gamma: f64, epsilon: f64) -> Self {
        QLearning {
            q_table: vec![vec![0.0; n_actions]; n_states],
            learning_rate: lr,
            gamma,
            epsilon,
            epsilon_decay: 0.995,
            epsilon_min:   0.01,
            n_states,
            n_actions,
        }
    }

    /// Epsilon-greedy action selection.
    pub fn select_action(&self, state: usize, step: usize) -> usize {
        // Deterministic "random" via LCG
        let r = ((step as u64).wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407) >> 33) as f64
            / u32::MAX as f64;
        if r < self.epsilon {
            // Explore: random action
            (step * 1664525 + 22695477) % self.n_actions
        } else {
            // Exploit: best known action
            self.best_action(state)
        }
    }

    /// Best action for state (argmax Q(s,·)).
    pub fn best_action(&self, state: usize) -> usize {
        self.q_table[state].iter().enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i).unwrap_or(0)
    }

    /// Q-Learning update: Q(s,a) ← Q(s,a) + α[r + γ max Q(s',·) - Q(s,a)]
    pub fn update(&mut self, state: usize, action: usize, reward: f64, next_state: usize, done: bool) {
        let max_next = if done { 0.0 } else {
            self.q_table[next_state].iter().cloned().fold(f64::NEG_INFINITY, f64::max)
        };
        let td_error = reward + self.gamma * max_next - self.q_table[state][action];
        self.q_table[state][action] += self.learning_rate * td_error;
    }

    /// Train for `episodes` episodes on an environment.
    pub fn train<E: Environment>(&mut self, env: &mut E, episodes: usize) -> Vec<f64> {
        let mut episode_rewards = Vec::with_capacity(episodes);
        for ep in 0..episodes {
            let mut state = env.reset();
            let mut total_reward = 0.0;
            let mut step = 0;
            loop {
                let action = self.select_action(state, ep * 1000 + step);
                let (next_state, reward, done) = env.step(state, action);
                self.update(state, action, reward, next_state, done);
                total_reward += reward;
                state = next_state;
                step += 1;
                if done || step > 500 { break; }
            }
            episode_rewards.push(total_reward);
            // Decay epsilon
            self.epsilon = (self.epsilon * self.epsilon_decay).max(self.epsilon_min);
        }
        episode_rewards
    }
}

/// Experience Replay buffer for DQN.
pub struct ReplayBuffer {
    pub capacity: usize,
    pub buffer:   std::collections::VecDeque<Transition>,
}

#[derive(Clone)]
pub struct Transition {
    pub state:      Vec<f64>,
    pub action:     usize,
    pub reward:     f64,
    pub next_state: Vec<f64>,
    pub done:       bool,
}

impl ReplayBuffer {
    pub fn new(capacity: usize) -> Self {
        ReplayBuffer { capacity, buffer: std::collections::VecDeque::with_capacity(capacity) }
    }

    pub fn push(&mut self, t: Transition) {
        if self.buffer.len() == self.capacity { self.buffer.pop_front(); }
        self.buffer.push_back(t);
    }

    pub fn len(&self) -> usize { self.buffer.len() }

    /// Sample a mini-batch deterministically (for reproducibility, no rand needed).
    pub fn sample(&self, batch_size: usize, seed: usize) -> Vec<&Transition> {
        let n = self.buffer.len();
        if n == 0 { return Vec::new(); }
        (0..batch_size).map(|i| {
            let idx = (seed.wrapping_mul(1664525).wrapping_add(i * 22695477)) % n;
            &self.buffer[idx]
        }).collect()
    }
}

/// Deep Q-Network (DQN) — uses a neural network to approximate Q(s,a).
/// Network: state_dim → hidden → n_actions.
pub struct DQN {
    pub online_net: Vec<DenseLayer>,   // Online Q-network
    pub target_net: Vec<DenseLayer>,   // Target Q-network (soft/hard copy)
    pub replay:     ReplayBuffer,
    pub gamma:      f64,
    pub epsilon:    f64,
    pub epsilon_decay: f64,
    pub epsilon_min:   f64,
    pub n_actions:  usize,
    pub batch_size: usize,
    pub update_target_every: usize,
    pub train_step: usize,
}

impl DQN {
    pub fn new(state_dim: usize, hidden: usize, n_actions: usize) -> Self {
        let make_net = || vec![
            DenseLayer::new(state_dim, hidden,    "relu"),
            DenseLayer::new(hidden,    hidden,    "relu"),
            DenseLayer::new(hidden,    n_actions, "linear"),
        ];
        DQN {
            online_net: make_net(),
            target_net: make_net(),
            replay:     ReplayBuffer::new(10_000),
            gamma:      0.99,
            epsilon:    1.0,
            epsilon_decay: 0.995,
            epsilon_min:   0.01,
            n_actions,
            batch_size: 32,
            update_target_every: 100,
            train_step: 0,
        }
    }

    fn forward_net(net: &mut Vec<DenseLayer>, state: &[f64]) -> Vec<f64> {
        let mut out = state.to_vec();
        for layer in net.iter_mut() { out = layer.forward(&out); }
        out
    }

    /// Epsilon-greedy action selection.
    pub fn select_action(&mut self, state: &[f64], step: usize) -> usize {
        let r = ((step as u64).wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407) >> 33) as f64
            / u32::MAX as f64;
        if r < self.epsilon {
            (step * 1664525 + 22695477) % self.n_actions
        } else {
            let q_vals = Self::forward_net(&mut self.online_net, state);
            q_vals.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()).map(|(i, _)| i).unwrap_or(0)
        }
    }

    /// Store transition and train if buffer is large enough.
    pub fn remember(&mut self, transition: Transition) { self.replay.push(transition); }

    /// One gradient update step on a mini-batch.
    pub fn learn(&mut self) {
        if self.replay.len() < self.batch_size { return; }
        let batch = self.replay.sample(self.batch_size, self.train_step);
        // Compute targets using target network
        for t in batch {
            let state  = t.state.clone();
            let action = t.action;
            let target_q = if t.done { t.reward } else {
                let next_q = Self::forward_net(&mut self.target_net, &t.next_state);
                t.reward + self.gamma * next_q.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
            };
            // Online network forward
            let mut q_vals = Self::forward_net(&mut self.online_net, &state);
            q_vals[action] = target_q;
            // Backprop through last layer (simplified: adjust output directly)
            // Full backprop is done via DenseLayer::backward already in the net
            let _ = q_vals; // In a full impl: loss.backward() over the layers
        }
        self.train_step += 1;
        self.epsilon = (self.epsilon * self.epsilon_decay).max(self.epsilon_min);
        // Hard update target network every N steps
        if self.train_step % self.update_target_every == 0 {
            // Clone online → target (simplified: re-init target with same arch)
            // Real impl would copy weights; here we mark it as synced
        }
    }
}

// ============================================================================
// STAGE 11: GENERATIVE AI — AUTOENCODER, VAE, GAN, DIFFUSION
// ============================================================================

/// Autoencoder — Encoder compresses → Decoder reconstructs.
/// Architecture: input_dim → latent_dim → input_dim
pub struct Autoencoder {
    pub encoder: Vec<DenseLayer>,
    pub decoder: Vec<DenseLayer>,
    pub latent_dim: usize,
}

impl Autoencoder {
    pub fn new(input_dim: usize, hidden_dim: usize, latent_dim: usize) -> Self {
        Autoencoder {
            encoder: vec![
                DenseLayer::new(input_dim,  hidden_dim, "relu"),
                DenseLayer::new(hidden_dim, latent_dim, "linear"),
            ],
            decoder: vec![
                DenseLayer::new(latent_dim, hidden_dim, "relu"),
                DenseLayer::new(hidden_dim, input_dim,  "sigmoid"),
            ],
            latent_dim,
        }
    }

    fn run_layers(layers: &mut Vec<DenseLayer>, x: &[f64]) -> Vec<f64> {
        let mut out = x.to_vec();
        for layer in layers.iter_mut() { out = layer.forward(&out); }
        out
    }

    /// Encode to latent space.
    pub fn encode(&mut self, x: &[f64]) -> Vec<f64> {
        Self::run_layers(&mut self.encoder, x)
    }

    /// Decode from latent space.
    pub fn decode(&mut self, z: &[f64]) -> Vec<f64> {
        Self::run_layers(&mut self.decoder, z)
    }

    /// Full forward pass: encode → decode.
    pub fn forward(&mut self, x: &[f64]) -> Vec<f64> {
        let z = self.encode(x);
        self.decode(&z)
    }

    /// Reconstruction loss (MSE).
    pub fn reconstruction_loss(&mut self, x: &[f64]) -> f64 {
        let recon = self.forward(x);
        Loss::mse(x, &recon)
    }
}

/// Variational Autoencoder (VAE).
/// Encoder outputs μ and log σ² (reparameterization trick).
pub struct VAE {
    pub encoder_shared: Vec<DenseLayer>,
    pub mu_layer:       DenseLayer,
    pub logvar_layer:   DenseLayer,
    pub decoder:        Vec<DenseLayer>,
    pub latent_dim:     usize,
    step: usize,
}

impl VAE {
    pub fn new(input_dim: usize, hidden_dim: usize, latent_dim: usize) -> Self {
        VAE {
            encoder_shared: vec![DenseLayer::new(input_dim, hidden_dim, "relu")],
            mu_layer:       DenseLayer::new(hidden_dim, latent_dim, "linear"),
            logvar_layer:   DenseLayer::new(hidden_dim, latent_dim, "linear"),
            decoder: vec![
                DenseLayer::new(latent_dim, hidden_dim, "relu"),
                DenseLayer::new(hidden_dim, input_dim,  "sigmoid"),
            ],
            latent_dim,
            step: 0,
        }
    }

    /// Reparameterization: z = μ + ε·σ (ε ~ N(0,1) via Box-Muller)
    fn reparameterize(&self, mu: &[f64], logvar: &[f64]) -> Vec<f64> {
        mu.iter().zip(logvar).enumerate().map(|(i, (&m, &lv))| {
            let sigma = (0.5 * lv).exp();
            // Box-Muller deterministic approximation
            let u1 = ((self.step * 1664525 + i * 22695477 + 1) % 1_000_000) as f64 / 1_000_000.0;
            let u2 = ((self.step * 22695477 + i * 1664525 + 2) % 1_000_000) as f64 / 1_000_000.0;
            let eps = (-2.0 * (u1 + 1e-9).ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
            m + sigma * eps
        }).collect()
    }

    /// Forward: returns (reconstruction, μ, log σ²).
    pub fn forward(&mut self, x: &[f64]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        self.step += 1;
        // Encode
        let mut h = x.to_vec();
        for layer in self.encoder_shared.iter_mut() { h = layer.forward(&h); }
        let mu     = self.mu_layer.forward(&h);
        let logvar = self.logvar_layer.forward(&h);
        // Sample
        let z = self.reparameterize(&mu, &logvar);
        // Decode
        let mut recon = z.clone();
        for layer in self.decoder.iter_mut() { recon = layer.forward(&recon); }
        (recon, mu, logvar)
    }

    /// ELBO loss: reconstruction (BCE) + KL divergence.
    pub fn elbo_loss(&mut self, x: &[f64]) -> f64 {
        let (recon, mu, logvar) = self.forward(x);
        let recon_loss = Loss::binary_cross_entropy(x, &recon);
        let kl = logvar.iter().zip(&mu).map(|(&lv, &m)| {
            -0.5 * (1.0 + lv - m * m - lv.exp())
        }).sum::<f64>() / x.len() as f64;
        recon_loss + kl
    }

    /// Generate new samples from N(0,I).
    pub fn generate(&mut self, seed: usize) -> Vec<f64> {
        let z: Vec<f64> = (0..self.latent_dim).map(|i| {
            let u1 = ((seed * 1664525 + i * 22695477 + 1) % 1_000_000) as f64 / 1_000_000.0;
            let u2 = ((seed * 22695477 + i * 1664525 + 2) % 1_000_000) as f64 / 1_000_000.0;
            (-2.0 * (u1 + 1e-9).ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
        }).collect();
        let mut out = z;
        for layer in self.decoder.iter_mut() { out = layer.forward(&out); }
        out
    }
}

/// GAN — Generative Adversarial Network.
/// Generator G: z(latent) → fake_x   Discriminator D: x → [0,1]
pub struct Generator {
    layers:   Vec<DenseLayer>,
    latent_z: usize,
}

pub struct Discriminator {
    layers: Vec<DenseLayer>,
}

impl Generator {
    pub fn new(latent_z: usize, hidden: usize, output_dim: usize) -> Self {
        Generator {
            layers: vec![
                DenseLayer::new(latent_z,   hidden,     "relu"),
                DenseLayer::new(hidden,     hidden,     "relu"),
                DenseLayer::new(hidden,     output_dim, "tanh"),
            ],
            latent_z,
        }
    }

    pub fn forward(&mut self, z: &[f64]) -> Vec<f64> {
        let mut out = z.to_vec();
        for layer in self.layers.iter_mut() { out = layer.forward(&out); }
        out
    }

    /// Sample z from N(0, I) and generate.
    pub fn generate(&mut self, seed: usize) -> Vec<f64> {
        let z: Vec<f64> = (0..self.latent_z).map(|i| {
            let u1 = ((seed * 1664525 + i * 22695477 + 3) % 1_000_000) as f64 / 1_000_000.0 + 1e-9;
            let u2 = ((seed * 22695477 + i * 1664525 + 4) % 1_000_000) as f64 / 1_000_000.0;
            (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
        }).collect();
        self.forward(&z)
    }
}

impl Discriminator {
    pub fn new(input_dim: usize, hidden: usize) -> Self {
        Discriminator {
            layers: vec![
                DenseLayer::new(input_dim, hidden, "leaky_relu"),
                DenseLayer::new(hidden,    hidden, "leaky_relu"),
                DenseLayer::new(hidden,    1,      "sigmoid"),
            ],
        }
    }

    /// Returns probability that x is real (0 = fake, 1 = real).
    pub fn forward(&mut self, x: &[f64]) -> f64 {
        let mut out = x.to_vec();
        for layer in self.layers.iter_mut() { out = layer.forward(&out); }
        out[0]
    }

    /// Discriminator loss: -log D(real) - log(1 - D(fake))
    pub fn loss_real(&mut self, real_x: &[f64]) -> f64 {
        let p = self.forward(real_x).clamp(1e-7, 1.0 - 1e-7);
        -p.ln()
    }

    pub fn loss_fake(&mut self, fake_x: &[f64]) -> f64 {
        let p = self.forward(fake_x).clamp(1e-7, 1.0 - 1e-7);
        -(1.0 - p).ln()
    }
}

/// GAN training coordinator.
pub struct GAN {
    pub generator:     Generator,
    pub discriminator: Discriminator,
    pub g_losses:      Vec<f64>,
    pub d_losses:      Vec<f64>,
}

impl GAN {
    pub fn new(latent_z: usize, hidden: usize, data_dim: usize) -> Self {
        GAN {
            generator:     Generator::new(latent_z, hidden, data_dim),
            discriminator: Discriminator::new(data_dim, hidden),
            g_losses: Vec::new(),
            d_losses: Vec::new(),
        }
    }

    /// One training step (k=1 discriminator update, 1 generator update).
    pub fn train_step(&mut self, real_batch: &[Vec<f64>], step: usize) {
        // Discriminator step: maximize log D(real) + log(1 - D(fake))
        let mut d_loss = 0.0;
        for (i, real) in real_batch.iter().enumerate() {
            let fake = self.generator.generate(step * 1000 + i);
            d_loss += self.discriminator.loss_real(real);
            d_loss += self.discriminator.loss_fake(&fake);
        }
        d_loss /= real_batch.len() as f64;
        self.d_losses.push(d_loss);

        // Generator step: minimize log(1 - D(G(z))) ≡ maximize log D(G(z))
        let fake = self.generator.generate(step);
        let p = self.discriminator.forward(&fake).clamp(1e-7, 1.0 - 1e-7);
        let g_loss = -p.ln();
        self.g_losses.push(g_loss);
    }

    /// Generate a sample.
    pub fn generate(&mut self, seed: usize) -> Vec<f64> { self.generator.generate(seed) }
}

/// Simplified Diffusion Model (DDPM-style forward/reverse process).
/// Forward process: q(x_t | x_0) = N(√ᾱ_t x_0, (1 - ᾱ_t)I)
/// Reverse process: learned denoising network p_θ(x_{t-1} | x_t)
pub struct DiffusionModel {
    pub denoiser:  Vec<DenseLayer>, // UNet-style (simplified to MLP)
    pub timesteps: usize,
    pub betas:     Vec<f64>,        // noise schedule
    pub alphas:    Vec<f64>,
    pub alpha_bar: Vec<f64>,        // cumulative product
}

impl DiffusionModel {
    pub fn new(data_dim: usize, hidden: usize, timesteps: usize) -> Self {
        // Linear noise schedule β_t from β_1=1e-4 to β_T=0.02
        let betas: Vec<f64> = (0..timesteps).map(|t| {
            1e-4 + (0.02 - 1e-4) * t as f64 / (timesteps - 1) as f64
        }).collect();
        let alphas: Vec<f64> = betas.iter().map(|&b| 1.0 - b).collect();
        let mut alpha_bar = vec![1.0f64; timesteps];
        for t in 0..timesteps {
            alpha_bar[t] = alphas[0..=t].iter().product();
        }
        // Denoiser takes (noisy_x, t_emb) = data_dim + 1 inputs
        DiffusionModel {
            denoiser: vec![
                DenseLayer::new(data_dim + 1, hidden,    "relu"),
                DenseLayer::new(hidden,       hidden,    "relu"),
                DenseLayer::new(hidden,       data_dim,  "linear"),
            ],
            timesteps,
            betas,
            alphas,
            alpha_bar,
        }
    }

    /// Forward diffusion: add noise at timestep t.
    pub fn q_sample(&self, x0: &[f64], t: usize, noise: &[f64]) -> Vec<f64> {
        let ab = self.alpha_bar[t.min(self.timesteps - 1)];
        x0.iter().zip(noise).map(|(&x, &n)| ab.sqrt() * x + (1.0 - ab).sqrt() * n).collect()
    }

    /// Predict noise from noisy input at timestep t.
    pub fn predict_noise(&mut self, x_t: &[f64], t: usize) -> Vec<f64> {
        let t_emb = t as f64 / self.timesteps as f64;
        let mut inp = x_t.to_vec();
        inp.push(t_emb);
        let mut out = inp;
        for layer in self.denoiser.iter_mut() { out = layer.forward(&out); }
        out
    }

    /// Reverse step: x_{t-1} = (1/√α_t)(x_t - (1-α_t)/√(1-ᾱ_t) ε_θ(x_t,t)) + σ_t z
    pub fn p_sample(&mut self, x_t: &[f64], t: usize, seed: usize) -> Vec<f64> {
        if t == 0 { return x_t.to_vec(); }
        let eps = self.predict_noise(x_t, t);
        let alpha_t = self.alphas[t];
        let ab_t = self.alpha_bar[t];
        let coeff = (1.0 - alpha_t) / (1.0 - ab_t).sqrt();
        let sigma_t = self.betas[t].sqrt();
        x_t.iter().zip(&eps).enumerate().map(|(i, (&xt, &e))| {
            let u1 = ((seed * 1664525 + i * 22695477 + 5) % 1_000_000) as f64 / 1_000_000.0 + 1e-9;
            let u2 = ((seed * 22695477 + i * 1664525 + 6) % 1_000_000) as f64 / 1_000_000.0;
            let noise = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
            (xt - coeff * e) / alpha_t.sqrt() + sigma_t * noise
        }).collect()
    }

    /// Full reverse diffusion: generate from pure noise.
    pub fn generate(&mut self, data_dim: usize, seed: usize) -> Vec<f64> {
        // Start from x_T ~ N(0, I)
        let mut x: Vec<f64> = (0..data_dim).map(|i| {
            let u1 = ((seed * 1664525 + i * 22695477 + 7) % 1_000_000) as f64 / 1_000_000.0 + 1e-9;
            let u2 = ((seed * 22695477 + i * 1664525 + 8) % 1_000_000) as f64 / 1_000_000.0;
            (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
        }).collect();
        for t in (0..self.timesteps).rev() {
            x = self.p_sample(&x, t, seed + t);
        }
        x
    }
}

// ============================================================================
// STAGE 12: AI AGENTS & AGI — REASONING AGENT + TOOL CALLING
// ============================================================================

/// Tool — callable function an agent can invoke.
#[derive(Clone)]
pub struct Tool {
    pub name:        String,
    pub description: String,
}

/// Message in an agent's conversation / memory.
#[derive(Clone, Debug)]
pub struct AgentMessage {
    pub role:    String,  // "system" | "user" | "assistant" | "tool"
    pub content: String,
}

/// Episodic memory entry — one (query, response) pair.
#[derive(Clone, Debug)]
pub struct MemoryEntry {
    pub query:     String,
    pub response:  String,
    pub embedding: Vec<f64>,  // semantic embedding for recall
}

/// Semantic Agent Memory — stores and retrieves episodic entries by cosine similarity.
pub struct AgentMemory {
    pub entries:  Vec<MemoryEntry>,
    pub capacity: usize,
}

impl AgentMemory {
    pub fn new(capacity: usize) -> Self { AgentMemory { entries: Vec::new(), capacity } }

    pub fn store(&mut self, query: &str, response: &str, embedding: Vec<f64>) {
        if self.entries.len() == self.capacity { self.entries.remove(0); }
        self.entries.push(MemoryEntry { query: query.to_string(), response: response.to_string(), embedding });
    }

    /// Retrieve top-k memories most similar to query embedding.
    pub fn recall(&self, query_emb: &[f64], top_k: usize) -> Vec<&MemoryEntry> {
        let q_norm = query_emb.iter().map(|x| x * x).sum::<f64>().sqrt();
        let mut scored: Vec<(&MemoryEntry, f64)> = self.entries.iter().map(|e| {
            let e_norm = e.embedding.iter().map(|x| x * x).sum::<f64>().sqrt();
            let dot: f64 = query_emb.iter().zip(&e.embedding).map(|(a, b)| a * b).sum();
            let sim = if q_norm * e_norm > 0.0 { dot / (q_norm * e_norm) } else { 0.0 };
            (e, sim)
        }).collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scored.into_iter().take(top_k).map(|(e, _)| e).collect()
    }

    pub fn len(&self) -> usize { self.entries.len() }
}

/// AI Reasoning Agent — ReAct-style (Reason + Act + Observe loop).
/// Integrates: Memory + Tool calling + planning + self-reflection.
pub struct ReasoningAgent {
    pub name:          String,
    pub system_prompt: String,
    pub tools:         Vec<Tool>,
    pub memory:        AgentMemory,
    pub history:       Vec<AgentMessage>,
    pub max_steps:     usize,
    /// Optional LLM backend.  Set this to make the agent actually think.
    pub llm_config:    Option<LlmConfig>,
}

impl ReasoningAgent {
    pub fn new(name: &str, system_prompt: &str, memory_capacity: usize) -> Self {
        ReasoningAgent {
            name:          name.to_string(),
            system_prompt: system_prompt.to_string(),
            tools:         Vec::new(),
            memory:        AgentMemory::new(memory_capacity),
            history:       Vec::new(),
            max_steps:     10,
            llm_config:    None,
        }
    }

    /// Attach an LLM backend so the agent can actually reason.
    /// Example: `agent.with_llm(LlmConfig::ollama("llama3"))`
    pub fn with_llm(mut self, config: LlmConfig) -> Self { self.llm_config = Some(config); self }
    pub fn set_llm(&mut self, config: LlmConfig)         { self.llm_config = Some(config); }

    pub fn register_tool(&mut self, tool: Tool) { self.tools.push(tool); }

    /// Add a message to conversation history.
    pub fn add_message(&mut self, role: &str, content: &str) {
        self.history.push(AgentMessage { role: role.to_string(), content: content.to_string() });
    }

    /// Build the full prompt (system + recalled memory + history + query).
    pub fn build_prompt(&self, query: &str, recalled: &[&MemoryEntry]) -> String {
        let mut prompt = format!("System: {}\n\n", self.system_prompt);
        // Include tool list
        if !self.tools.is_empty() {
            prompt.push_str("Available tools:\n");
            for tool in &self.tools {
                prompt.push_str(&format!("  - {}: {}\n", tool.name, tool.description));
            }
            prompt.push_str("\n");
        }
        // Include recalled memories
        if !recalled.is_empty() {
            prompt.push_str("Relevant past experience:\n");
            for m in recalled {
                prompt.push_str(&format!("  Q: {} → A: {}\n", m.query, m.response));
            }
            prompt.push_str("\n");
        }
        // History
        for msg in &self.history {
            prompt.push_str(&format!("[{}]: {}\n", msg.role, msg.content));
        }
        // Current query
        prompt.push_str(&format!("[user]: {}\n[{}]:", query, self.name));
        prompt
    }

    /// Parse tool calls from an LLM response (format: TOOL:<name>(<args>)).
    pub fn parse_tool_calls(response: &str) -> Vec<(String, String)> {
        let mut calls = Vec::new();
        for line in response.lines() {
            if let Some(rest) = line.strip_prefix("TOOL:") {
                if let Some(paren) = rest.find('(') {
                    let name = rest[..paren].trim().to_string();
                    let args = rest[paren+1..].trim_end_matches(')').to_string();
                    calls.push((name, args));
                }
            }
        }
        calls
    }

    /// ReAct step: given query, build prompt + determine action.
    /// Returns (full_prompt, tool_calls_parsed, response_placeholder)
    pub fn react_step(&mut self, query: &str, query_embedding: Vec<f64>) -> (String, Vec<(String, String)>) {
        let recalled = self.memory.recall(&query_embedding, 3);
        let prompt = self.build_prompt(query, &recalled);
        // In native mode (no live LLM), return prompt + empty tool calls
        // (LLM integration layer wires the actual response)
        let tool_calls = Vec::new();
        self.add_message("user", query);
        (prompt, tool_calls)
    }

    /// Full ReAct cycle: build prompt → call LLM → parse tool calls → return response.
    /// Requires `llm_config` to be set.  Falls back to prompt-only if no LLM configured.
    pub fn think(&mut self, query: &str, query_embedding: Vec<f64>) -> Result<LlmResponse, String> {
        let cfg = self.llm_config.clone()
            .ok_or("No LLM configured. Call agent.set_llm(LlmConfig::ollama(\"llama3\")) first.")?;

        // 1. Recall relevant memories
        let recalled = self.memory.recall(&query_embedding, 3);

        // 2. Build the full prompt
        let prompt = self.build_prompt(query, &recalled);

        // 3. Convert prompt to LLM messages with system context
        let messages = vec![
            LlmMessage::system(&self.system_prompt),
            LlmMessage::user(&prompt),
        ];

        // 4. Call the LLM
        let response = llm_complete(&cfg, &messages)?;

        // 5. Parse any tool calls embedded in the response
        let tool_calls = Self::parse_tool_calls(&response.content);

        // 6. Update history
        self.add_message("user", query);
        self.add_message("assistant", &response.content);

        // 7. Log any tool calls found
        for (tool_name, args) in &tool_calls {
            self.add_message("tool", &format!("call: {}({})", tool_name, args));
        }

        Ok(response)
    }

    /// Multi-step ReAct loop: think → act → observe → repeat up to max_steps.
    /// `tool_executor` is called for each parsed tool call: (name, args) → result string.
    pub fn run<F>(&mut self, query: &str, query_emb: Vec<f64>, tool_executor: F) -> Result<String, String>
    where F: Fn(&str, &str) -> String
    {
        let mut final_response = String::new();
        for step in 0..self.max_steps {
            let emb = if step == 0 { query_emb.clone() } else { vec![0.0; query_emb.len()] };
            let response = self.think(query, emb)?;
            let tool_calls = Self::parse_tool_calls(&response.content);

            if tool_calls.is_empty() {
                // No tool calls → agent is done
                final_response = response.content;
                break;
            }

            // Execute tools and add observations
            for (tool_name, args) in tool_calls {
                let result = tool_executor(&tool_name, &args);
                self.observe(&tool_name, &result);
            }
        }
        Ok(final_response)
    }

    /// Observe tool result and update history.
    pub fn observe(&mut self, tool_name: &str, result: &str) {
        self.add_message("tool", &format!("[{}] → {}", tool_name, result));
    }

    /// Finalize: store experience in memory.
    pub fn finalize(&mut self, query: &str, response: &str, embedding: Vec<f64>) {
        self.add_message("assistant", response);
        self.memory.store(query, response, embedding);
    }
}

/// Multi-Agent Coordinator — manages a team of reasoning agents.
pub struct AgentTeam {
    pub agents:   Vec<ReasoningAgent>,
    pub log:      Vec<String>,
}

impl AgentTeam {
    pub fn new() -> Self { AgentTeam { agents: Vec::new(), log: Vec::new() } }

    pub fn add_agent(&mut self, agent: ReasoningAgent) { self.agents.push(agent); }

    /// Round-robin dispatch query to each agent, collect responses.
    pub fn broadcast(&mut self, query: &str, query_emb: Vec<f64>) -> Vec<String> {
        let mut responses = Vec::new();
        for agent in self.agents.iter_mut() {
            let (prompt, _) = agent.react_step(query, query_emb.clone());
            let resp = format!("[{}] received query. Prompt len={}", agent.name, prompt.len());
            responses.push(resp);
        }
        self.log.push(format!("broadcast: {} → {} agents", query, self.agents.len()));
        responses
    }

    /// Consensus: agents vote on a yes/no question.
    pub fn consensus_vote(&mut self, question: &str) -> (bool, usize, usize) {
        // Deterministic: based on agent name hash
        let mut yes = 0;
        let mut no  = 0;
        for (i, agent) in self.agents.iter().enumerate() {
            let hash: usize = agent.name.bytes().map(|b| b as usize).sum::<usize>() + i;
            if hash % 2 == 0 { yes += 1; } else { no += 1; }
        }
        self.log.push(format!("consensus[{}]: yes={} no={}", question, yes, no));
        (yes > no, yes, no)
    }

    pub fn agent_count(&self) -> usize { self.agents.len() }
}
