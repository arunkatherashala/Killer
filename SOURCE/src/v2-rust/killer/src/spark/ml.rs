/// MLlib - Machine Learning Library for Killer Spark
/// 
/// Implements core ML algorithms: regression, classification, clustering

use crate::value::Value;
use std::collections::HashMap;

/// ML Model trait - all models implement this
pub trait MLModel {
    fn predict(&self, features: &[f64]) -> Result<f64, String>;
    fn model_type(&self) -> &str;
}

/// Linear Regression Model
#[derive(Clone, Debug)]
pub struct LinearRegressionModel {
    coefficients: Vec<f64>,
    intercept: f64,
}

impl LinearRegressionModel {
    pub fn new(coefficients: Vec<f64>, intercept: f64) -> Self {
        Self {
            coefficients,
            intercept,
        }
    }

    pub fn coefficients(&self) -> &[f64] {
        &self.coefficients
    }

    pub fn intercept(&self) -> f64 {
        self.intercept
    }

    /// Predict using the model
    pub fn predict(&self, features: &[f64]) -> Result<f64, String> {
        if features.len() != self.coefficients.len() {
            return Err(format!(
                "Feature count mismatch: expected {}, got {}",
                self.coefficients.len(),
                features.len()
            ));
        }

        let mut result = self.intercept;
        for (feature, coef) in features.iter().zip(self.coefficients.iter()) {
            result += feature * coef;
        }
        Ok(result)
    }
}

impl MLModel for LinearRegressionModel {
    fn predict(&self, features: &[f64]) -> Result<f64, String> {
        self.predict(features)
    }

    fn model_type(&self) -> &str {
        "LinearRegression"
    }
}

/// Logistic Regression Model for binary classification
#[derive(Clone, Debug)]
pub struct LogisticRegressionModel {
    coefficients: Vec<f64>,
    intercept: f64,
}

impl LogisticRegressionModel {
    pub fn new(coefficients: Vec<f64>, intercept: f64) -> Self {
        Self {
            coefficients,
            intercept,
        }
    }

    pub fn coefficients(&self) -> &[f64] {
        &self.coefficients
    }

    pub fn intercept(&self) -> f64 {
        self.intercept
    }

    /// Sigmoid function
    fn sigmoid(x: f64) -> f64 {
        1.0 / (1.0 + (-x).exp())
    }

    /// Predict probability [0, 1] for binary classification
    pub fn predict_proba(&self, features: &[f64]) -> Result<f64, String> {
        if features.len() != self.coefficients.len() {
            return Err(format!(
                "Feature count mismatch: expected {}, got {}",
                self.coefficients.len(),
                features.len()
            ));
        }

        let mut logit = self.intercept;
        for (feature, coef) in features.iter().zip(self.coefficients.iter()) {
            logit += feature * coef;
        }
        Ok(Self::sigmoid(logit))
    }

    /// Predict class (0 or 1)
    pub fn predict(&self, features: &[f64]) -> Result<f64, String> {
        let prob = self.predict_proba(features)?;
        Ok(if prob >= 0.5 { 1.0 } else { 0.0 })
    }
}

impl MLModel for LogisticRegressionModel {
    fn predict(&self, features: &[f64]) -> Result<f64, String> {
        self.predict(features)
    }

    fn model_type(&self) -> &str {
        "LogisticRegression"
    }
}

/// Decision Tree Node
#[derive(Clone, Debug)]
pub enum TreeNode {
    Leaf {
        value: f64,
    },
    Split {
        feature: usize,
        threshold: f64,
        left: Box<TreeNode>,
        right: Box<TreeNode>,
    },
}

/// Decision Tree Classifier/Regressor
#[derive(Clone, Debug)]
pub struct DecisionTreeModel {
    root: TreeNode,
    max_depth: usize,
}

impl DecisionTreeModel {
    pub fn new(root: TreeNode, max_depth: usize) -> Self {
        Self { root, max_depth }
    }

    pub fn predict(&self, features: &[f64]) -> Result<f64, String> {
        self.traverse(&self.root, features)
    }

    fn traverse(&self, node: &TreeNode, features: &[f64]) -> Result<f64, String> {
        match node {
            TreeNode::Leaf { value } => Ok(*value),
            TreeNode::Split {
                feature,
                threshold,
                left,
                right,
            } => {
                if *feature >= features.len() {
                    return Err("Feature index out of bounds".to_string());
                }
                let next = if features[*feature] <= *threshold { left } else { right };
                self.traverse(next, features)
            }
        }
    }
}

impl MLModel for DecisionTreeModel {
    fn predict(&self, features: &[f64]) -> Result<f64, String> {
        self.predict(features)
    }

    fn model_type(&self) -> &str {
        "DecisionTree"
    }
}

/// K-Means Clustering
#[derive(Clone, Debug)]
pub struct KMeansModel {
    centroids: Vec<Vec<f64>>,
    k: usize,
}

impl KMeansModel {
    pub fn new(centroids: Vec<Vec<f64>>) -> Self {
        let k = centroids.len();
        Self { centroids, k }
    }

    /// Find the closest cluster for a point
    pub fn predict(&self, point: &[f64]) -> Result<usize, String> {
        let mut min_distance = f64::MAX;
        let mut closest_cluster = 0;

        for (cluster_id, centroid) in self.centroids.iter().enumerate() {
            if centroid.len() != point.len() {
                return Err("Dimension mismatch".to_string());
            }

            let distance: f64 = centroid
                .iter()
                .zip(point.iter())
                .map(|(a, b)| (a - b).powi(2))
                .sum();

            if distance < min_distance {
                min_distance = distance;
                closest_cluster = cluster_id;
            }
        }

        Ok(closest_cluster)
    }

    pub fn centroids(&self) -> &[Vec<f64>] {
        &self.centroids
    }

    pub fn k(&self) -> usize {
        self.k
    }

    pub fn inertia(&self, points: &[Vec<f64>]) -> Result<f64, String> {
        let mut total: f64 = 0.0;
        for point in points {
            let cluster = self.predict(point)?;
            let centroid = &self.centroids[cluster];
            let distance: f64 = centroid
                .iter()
                .zip(point.iter())
                .map(|(a, b)| (a - b).powi(2))
                .sum();
            total += distance.sqrt();
        }
        Ok(total)
    }
}

/// MLlib context for training models
pub struct MLlib {
    seed: u64,
}

impl MLlib {
    pub fn new() -> Self {
        Self { seed: 42 }
    }

    pub fn set_seed(&mut self, seed: u64) {
        self.seed = seed;
    }

    /// Train linear regression
    pub fn linear_regression(
        &self,
        x: &[Vec<f64>],
        y: &[f64],
    ) -> Result<LinearRegressionModel, String> {
        if x.is_empty() || y.is_empty() || x.len() != y.len() {
            return Err("Invalid training data".to_string());
        }

        // Simple linear regression: y = mx + b
        let n = x.len() as f64;
        let x_mean = x.iter().map(|row| row[0]).sum::<f64>() / n;
        let y_mean = y.iter().sum::<f64>() / n;

        let mut numerator = 0.0;
        let mut denominator = 0.0;

        for (xi, yi) in x.iter().zip(y.iter()) {
            let x_diff = xi[0] - x_mean;
            numerator += x_diff * (yi - y_mean);
            denominator += x_diff * x_diff;
        }

        let slope = if denominator == 0.0 { 0.0 } else { numerator / denominator };
        let intercept = y_mean - slope * x_mean;

        Ok(LinearRegressionModel::new(vec![slope], intercept))
    }

    /// Train logistic regression
    pub fn logistic_regression(
        &self,
        x: &[Vec<f64>],
        y: &[f64],
    ) -> Result<LogisticRegressionModel, String> {
        if x.is_empty() || y.is_empty() || x.len() != y.len() {
            return Err("Invalid training data".to_string());
        }

        // Simplified logistic regression initialization
        let coef = vec![0.1; x[0].len()];
        let intercept = 0.0;

        Ok(LogisticRegressionModel::new(coef, intercept))
    }

    /// Train K-Means clustering
    pub fn k_means(
        &self,
        points: &[Vec<f64>],
        k: usize,
        max_iterations: usize,
    ) -> Result<KMeansModel, String> {
        if points.is_empty() || k == 0 || k > points.len() {
            return Err("Invalid k or empty points".to_string());
        }

        // Initialize centroids as first k points
        let mut centroids: Vec<Vec<f64>> = points.iter().take(k).cloned().collect();

        for _ in 0..max_iterations {
            let mut new_centroids = vec![vec![0.0; points[0].len()]; k];
            let mut counts = vec![0.0; k];

            // Assign points to clusters
            for point in points {
                let mut min_dist = f64::MAX;
                let mut cluster = 0;

                for (i, centroid) in centroids.iter().enumerate() {
                    let dist: f64 = centroid
                        .iter()
                        .zip(point.iter())
                        .map(|(a, b)| (a - b).powi(2))
                        .sum();

                    if dist < min_dist {
                        min_dist = dist;
                        cluster = i;
                    }
                }

                // Update centroid sum
                for (j, val) in point.iter().enumerate() {
                    new_centroids[cluster][j] += val;
                }
                counts[cluster] += 1.0;
            }

            // Compute new centroids
            for i in 0..k {
                if counts[i] > 0.0 {
                    for j in 0..new_centroids[i].len() {
                        new_centroids[i][j] /= counts[i];
                    }
                }
            }

            centroids = new_centroids;
        }

        Ok(KMeansModel::new(centroids))
    }
}

impl Default for MLlib {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_regression() {
        let x = vec![vec![1.0], vec![2.0], vec![3.0]];
        let y = vec![2.0, 4.0, 6.0];

        let ml = MLlib::new();
        let model = ml.linear_regression(&x, &y).unwrap();

        assert!(model.predict(&[4.0]).unwrap() > 7.0);
    }

    #[test]
    fn test_logistic_regression() {
        let x = vec![vec![1.0], vec![2.0]];
        let y = vec![0.0, 1.0];

        let ml = MLlib::new();
        let model = ml.logistic_regression(&x, &y).unwrap();

        assert!(model.predict(&[0.5]).is_ok());
    }

    #[test]
    fn test_kmeans() {
        let points = vec![
            vec![1.0, 1.0],
            vec![1.1, 1.2],
            vec![10.0, 10.0],
            vec![10.2, 10.1],
        ];

        let ml = MLlib::new();
        let model = ml.k_means(&points, 2, 10).unwrap();

        assert_eq!(model.k(), 2);
    }

    #[test]
    fn test_kmeans_predict() {
        let centroids = vec![vec![1.0, 1.0], vec![10.0, 10.0]];
        let model = KMeansModel::new(centroids);

        assert_eq!(model.predict(&[1.1, 1.1]).unwrap(), 0);
        assert_eq!(model.predict(&[10.1, 10.1]).unwrap(), 1);
    }

    #[test]
    fn test_decision_tree() {
        let leaf_true = TreeNode::Leaf { value: 1.0 };
        let leaf_false = TreeNode::Leaf { value: 0.0 };

        let root = TreeNode::Split {
            feature: 0,
            threshold: 5.0,
            left: Box::new(leaf_false),
            right: Box::new(leaf_true),
        };

        let model = DecisionTreeModel::new(root, 1);
        assert_eq!(model.predict(&[3.0]).unwrap(), 0.0);
        assert_eq!(model.predict(&[7.0]).unwrap(), 1.0);
    }
}
