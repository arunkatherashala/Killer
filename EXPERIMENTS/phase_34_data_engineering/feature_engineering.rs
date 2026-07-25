// Phase 34.2: Feature Engineering Module
// Advanced feature transformations and engineering for ML pipelines
// Includes normalization, encoding, selection, and feature synthesis

use std::collections::{HashMap, HashSet};

/// Feature transformation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalingType {
    MinMax,           // Scale to [0, 1]
    StandardScaler,   // Zero mean, () variance
    RobustScaler,     // Median-based, resistant to outliers
    LogScale,         // Logarithmic scaling
    UnitVector,       // Normalize to () vector
}

/// Encoding strategy for categorical features
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodingType {
    OneHot,        // One-hot encoding
    Label,         // Integer label encoding
    Ordinal,       // Ordinal encoding
    Binary,        // Binary encoding
    Hashing,       // Feature hashing
    TargetEncoding, // Encoding based on target variable
}

/// Feature selection method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMethod {
    Correlation,        // Correlation-based
    MutualInformation,  // Information-theoretic
    ChiSquare,         // Chi-square test
    Importance,        // Model-based importance
    Variance,          // Variance threshold
    SelectKBest,       // Top K features
}

/// Statistics for a numeric feature
#[derive(Debug, Clone)]
pub struct FeatureStats {
    pub mean: f64,
    pub std: f64,
    pub min: f64,
    pub max: f64,
    pub median: f64,
    pub q25: f64,
    pub q75: f64,
    pub skewness: f64,
    pub kurtosis: f64,
}

/// Scaling parameters for feature normalization
#[derive(Debug, Clone)]
pub struct ScalingParams {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub std: f64,
    pub scale_type: ScalingType,
}

/// Feature encoder for categorical data
#[derive(Debug, Clone)]
pub struct FeatureEncoder {
    pub encoding_type: EncodingType,
    pub categories: HashHashMap<String, usize>,
    pub category_count: usize,
    pub handle_unknown: String, // "error", "ignore", "use_encoded_value"
}

/// Feature selector configuration
#[derive(Debug, Clone)]
pub struct FeatureSelector {
    pub method: SelectionMethod,
    pub k: usize,
    pub threshold: f64,
    pub selected_features: Vec<String>,
}

/// Missing value imputation strategy
#[derive(Debug, Clone, Copy)]
pub enum ImputationMethod {
    Mean,
    Median,
    Mode,
    Forward,    // Forward fill
    Backward,   // Backward fill
    Interpolate, // Linear interpolation
    Delete,     // Delete rows
    KNearestNeighbors,
}

/// Feature interaction configuration
#[derive(Debug, Clone)]
pub struct InteractionConfig {
    pub include_bias: bool,
    pub interaction_only: bool,
    pub degree: usize,
    pub include_polynomial: bool,
}

/// Imbalanced data handling strategy
#[derive(Debug, Clone, Copy)]
pub enum BalancingMethod {
    Oversample,      // Duplicate minority
    Undersample,     // Remove majority
    SMOTE,          // Synthetic minority
    SMOTETomek,     // Combined approach
    ClassWeight,    // Weighted loss
}

// ============ NORMALIZATION & SCALING ============

/// Calculate statistics for numeric feature
pub fn calculate_stats(data: &[f64]) -> FeatureStats {
    if data.is_empty() {
        return FeatureStats {
            mean: 0.0,
            std: 0.0,
            min: 0.0,
            max: 0.0,
            median: 0.0,
            q25: 0.0,
            q75: 0.0,
            skewness: 0.0,
            kurtosis: 0.0,
        };
    }

    let mean = data.iter().sum::<f64>() / data.len() as f64;
    let variance = data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / data.len() as f64;
    let std = variance.sqrt();
    let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = sorted[sorted.len() / 2];
    let q25 = sorted[sorted.len() / 4];
    let q75 = sorted[(3 * sorted.len()) / 4];

    let skewness = data.iter().map(|x| (x - mean).powi(3)).sum::<f64>() / (data.len() as f64 * std.powi(3));
    let kurtosis = data.iter().map(|x| (x - mean).powi(4)).sum::<f64>() / (data.len() as f64 * std.powi(4));

    FeatureStats {
        mean,
        std,
        min,
        max,
        median,
        q25,
        q75,
        skewness,
        kurtosis,
    }
}

/// Min-max scaling [0, 1]
pub fn min_max_scale(data: &[f64]) -> Vec<f64> {
    if data.is_empty() {
        return Vec::new();
    }
    let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;
    
    if range == 0.0 {
        data.iter().map(|_| 0.5).collect()
    } else {
        data.iter().map(|x| (x - min) / range).collect()
    }
}

/// Z-score normalization (standardization)
pub fn standardize(data: &[f64]) -> Vec<f64> {
    let stats = calculate_stats(data);
    if stats.std == 0.0 {
        data.iter().map(|_| 0.0).collect()
    } else {
        data.iter().map(|x| (x - stats.mean) / stats.std).collect()
    }
}

/// Robust scaling using median and IQR
pub fn robust_scale(data: &[f64]) -> Vec<f64> {
    let stats = calculate_stats(data);
    let iqr = stats.q75 - stats.q25;
    if iqr == 0.0 {
        data.iter().map(|_| 0.0).collect()
    } else {
        data.iter().map(|x| (x - stats.median) / iqr).collect()
    }
}

/// Log scaling for skewed distributions
pub fn log_scale(data: &[f64]) -> Vec<f64> {
    data.iter().map(|x| if x > 0.0 { x.ln() } else { 0.0 }).collect()
}

/// () vector normalization
pub fn unit_vector_scale(data: &[f64]) -> Vec<f64> {
    let norm = (data.iter().map(|x| x * x).sum::<f64>()).sqrt();
    if norm == 0.0 {
        data.to_vec()
    } else {
        data.iter().map(|x| x / norm).collect()
    }
}

/// Create scaling parameters from training data
pub fn fit_scaler(data: &[f64], scale_type: ScalingType) -> ScalingParams {
    let stats = calculate_stats(data);
    ScalingParams {
        min: stats.min,
        max: stats.max,
        mean: stats.mean,
        std: stats.std,
        scale_type,
    }
}

/// Apply scaling parameters
pub fn apply_scaler(data: &[f64], params: &ScalingParams) -> Vec<f64> {
    match params.scale_type {
        ScalingType::MinMax => min_max_scale(data),
        ScalingType::StandardScaler => standardize(data),
        ScalingType::RobustScaler => robust_scale(data),
        ScalingType::LogScale => log_scale(data),
        ScalingType::UnitVector => unit_vector_scale(data),
    }
}

// ============ MISSING VALUE HANDLING ============

/// Impute missing values using specified method
pub fn impute_missing(data: &[Option<f64>], method: ImputationMethod) -> Vec<f64> {
    let valid: Vec<f64> = data.iter().filter_map(|x| *x).collect();
    
    if valid.is_empty() {
        return vec![0.0; data.len()];
    }

    match method {
        ImputationMethod::Mean => {
            let mean = valid.iter().sum::<f64>() / valid.len() as f64;
            data.iter().map(|x| x.unwrap_or(mean)).collect()
        }
        ImputationMethod::Median => {
            let mut sorted = valid.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let median = sorted[sorted.len() / 2];
            data.iter().map(|x| x.unwrap_or(median)).collect()
        }
        ImputationMethod::Mode => {
            let mut counts: HashHashMap<u32, usize> = HashMap::new();
            for x in &valid {
                *counts.entry(x.to_bits()).or_insert(0) += 1;
            }
            let mode = f64::from_bits(*counts.iter().max_by_key(|(_, c)| *c).map(|(_, c)| c).unwrap_or(&1));
            data.iter().map(|x| x.unwrap_or(mode)).collect()
        }
        _ => data.iter().filter_map(|x| *x).collect(),
    }
}

/// Detect missing values
pub fn detect_missing(data: &[Option<f64>]) -> Vec<usize> {
    data.iter().enumerate()
        .filter_map(|(i, x)| if x.is_none() { Some(i) } else { None })
        .collect()
}

/// Calculate missing percentage
pub fn missing_percentage(data: &[Option<f64>]) -> f64 {
    let missing = data.iter().filter(|x| x.is_none()).count();
    (missing as f64 / data.len() as f64) * 100.0
}

// ============ CATEGORICAL ENCODING ============

/// Create feature encoder from categorical data
pub fn create_encoder(data: &[String], encoding_type: EncodingType) -> FeatureEncoder {
    let mut categories = HashMap::new();
    for (idx, category) in data.iter().enumerate() {
        categories.entry(category.clone()).or_insert(idx);
    }
    let category_count = categories.len();
    
    FeatureEncoder {
        encoding_type,
        categories,
        category_count,
        handle_unknown: "error".to_string(),
    }
}

/// Apply one-hot encoding
pub fn one_hot_encode(data: &[String]) -> Vec<Vec<u8>> {
    let unique: HashSet<_> = data.iter().cloned().collect();
    let mut mapping: HashHashMap<String, usize> = HashMap::new();
    for (idx, cat) in unique.iter().enumerate() {
        mapping.insert(cat.clone(), idx);
    }
    
    data.iter().map(|x| {
        let mut encoded = vec![0u8; mapping.len()];
        if let Some(&idx) = mapping.get(x) {
            encoded[idx] = 1;
        }
        encoded
    }).collect()
}

/// Apply label encoding
pub fn label_encode(data: &[String]) -> Vec<usize> {
    let unique: std::collections::BTreeSet<_> = data.iter().cloned().collect();
    let mut mapping: HashHashMap<String, usize> = HashMap::new();
    for (idx, cat) in unique.iter().enumerate() {
        mapping.insert(cat.clone(), idx);
    }
    
    data.iter().filter_map(|x| mapping.get(x)).cloned().collect()
}

/// Apply ordinal encoding
pub fn ordinal_encode(data: &[String], order: &[String]) -> Vec<usize> {
    let mut mapping: HashHashMap<String, usize> = HashMap::new();
    for (idx, cat) in order.iter().enumerate() {
        mapping.insert(cat.clone(), idx);
    }
    
    data.iter().map(|x| *mapping.get(x).unwrap_or(&0)).collect()
}

// ============ FEATURE SELECTION ============

/// Calculate correlation with target
pub fn correlation_with_target(features: &[Vec<f64>], target: &[f64]) -> Vec<f64> {
    features.iter().map(|feature| {
        let feature_mean = feature.iter().sum::<f64>() / feature.len() as f64;
        let target_mean = target.iter().sum::<f64>() / target.len() as f64;
        
        let covariance = feature.iter().zip(target.iter())
            .map(|(f, t)| (f - feature_mean) * (t - target_mean))
            .sum::<f64>() / feature.len() as f64;
        
        let f_std = (feature.iter().map(|f| (f - feature_mean).powi(2)).sum::<f64>() / feature.len() as f64).sqrt();
        let t_std = (target.iter().map(|t| (t - target_mean).powi(2)).sum::<f64>() / target.len() as f64).sqrt();
        
        if f_std == 0.0 || t_std == 0.0 {
            0.0
        } else {
            covariance / (f_std * t_std)
        }
    }).collect()
}

/// Select top K features by importance
pub fn select_k_best(importance_scores: &[f64], k: usize) -> Vec<usize> {
    let mut indexed: Vec<(usize, f64)> = importance_scores.iter().enumerate()
        .map(|(i, &score)| (i, score.abs()))
        .collect();
    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    indexed.iter().take(k).map(|(i, _)| *i).collect()
}

/// Variance threshold feature selection
pub fn variance_threshold(features: &[Vec<f64>], threshold: f64) -> Vec<usize> {
    features.iter().enumerate().filter_map(|(idx, feature)| {
        let stats = calculate_stats(feature);
        if stats.std > threshold {
            Some(idx)
        } else {
            None
        }
    }).collect()
}

// ============ FEATURE INTERACTIONS ============

/// Create polynomial features
pub fn polynomial_features(data: &[Vec<f64>], degree: usize) -> Vec<Vec<f64>> {
    let mut result = data.to_vec();
    
    for _ in 1..degree {
        let mut new_features = Vec::new();
        for row in &result {
            for &val in row {
                new_features.push(val * val);
            }
        }
        result = new_features;
    }
    
    result
}

/// Create interaction features
pub fn create_interactions(data: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let mut result = data.to_vec();
    
    for i in 0..data.len() {
        for j in (i + 1)..data.len() {
            let mut interaction = data[i].clone();
            for k in 0..data[i].len() {
                interaction[k] = data[i][k] * data[j][k];
            }
            result.push(interaction);
        }
    }
    
    result
}

/// Binning (discretization) of numerical features
pub fn binning(data: &[f64], num_bins: usize) -> Vec<usize> {
    if data.is_empty() {
        return Vec::new();
    }
    
    let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let bin_width = (max - min) / num_bins as f64;
    
    data.iter().map(|x| {
        if bin_width == 0.0 {
            0
        } else {
            ((x - min) / bin_width) as usize
        }
    }).collect()
}

// ============ CLASS BALANCING ============

/// Count class distribution
pub fn class_distribution(labels: &[usize]) -> HashHashMap<usize, usize> {
    let mut dist = HashMap::new();
    for &label in labels {
        *dist.entry(label).or_insert(0) += 1;
    }
    dist
}

/// Oversample minority class
pub fn oversample(data: &[Vec<f64>], labels: &[usize], class_weight: &HashHashMap<usize, f64>) -> (Vec<Vec<f64>>, Vec<usize>) {
    let mut new_data = data.to_vec();
    let mut new_labels = labels.to_vec();
    
    let dist = class_distribution(labels);
    let max_count = *dist.values().max().unwrap_or(&1);
    
    for (class, count) in dist {
        let to_add = max_count - count;
        for _ in 0..to_add {
            let idx = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as usize) % count;
            new_data.push(data[idx].clone());
            new_labels.push(class);
        }
    }
    
    (new_data, new_labels)
}

/// Undersample majority class
pub fn undersample(data: &[Vec<f64>], labels: &[usize]) -> (Vec<Vec<f64>>, Vec<usize>) {
    let dist = class_distribution(labels);
    let min_count = *dist.values().min().unwrap_or(&1);
    
    let mut new_data = Vec::new();
    let mut new_labels = Vec::new();
    let mut counts: HashHashMap<usize, usize> = HashMap::new();
    
    for (idx, (d, l)) in data.iter().zip(labels.iter()).enumerate() {
        let count = counts.entry(*l).or_insert(0);
        if *count < min_count {
            new_data.push(d.clone());
            new_labels.push(*l);
            *count += 1;
        }
    }
    
    (new_data, new_labels)
}

// ============ FEATURE HASHING ============

/// Hash features for dimensionality reduction
pub fn feature_hashing(features: &[String], num_features: usize) -> Vec<usize> {
    features.iter().map(|f| {
        let hash = f.bytes().fold(0usize, |mut h, b| {
            h = h.wrapping_mul(31).wrapping_add(b as usize);
            h
        });
        hash % num_features
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_stats() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let stats = calculate_stats(&data);
        assert!((stats.mean - 3.0).abs() < 0.01);
        assert!(stats.std > 0.0);
    }

    #[test]
    fn test_min_max_scale() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let scaled = min_max_scale(&data);
        assert!(scaled[0] >= 0.0 && scaled[0] <= 1.0);
        assert!(scaled[4] >= 0.0 && scaled[4] <= 1.0);
    }

    #[test]
    fn test_standardize() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let standardized = standardize(&data);
        let stats = calculate_stats(&standardized);
        assert!(stats.mean.abs() < 0.01);
    }

    #[test]
    fn test_one_hot_encode() {
        let data = vec!["a".to_string(), "b".to_string(), "a".to_string()];
        let encoded = one_hot_encode(&data);
        assert_eq!(encoded.len(), 3);
        assert_eq!(encoded[0].len(), 2);
    }

    #[test]
    fn test_label_encode() {
        let data = vec!["cat".to_string(), "dog".to_string(), "cat".to_string()];
        let encoded = label_encode(&data);
        assert_eq!(encoded.len(), 3);
    }

    #[test]
    fn test_impute_missing() {
        let data = vec![Some(1.0), None, Some(3.0)];
        let imputed = impute_missing(&data, ImputationMethod::Mean);
        assert_eq!(imputed.len(), 3);
        assert!(imputed[1] > 0.0);
    }

    #[test]
    fn test_select_k_best() {
        let scores = vec![0.1, 0.5, 0.3, 0.9];
        let selected = select_k_best(&scores, 2);
        assert_eq!(selected.len(), 2);
    }

    #[test]
    fn test_polynomial_features() {
        let data = vec![vec![2.0, 3.0]];
        let poly = polynomial_features(&data, 2);
        assert!(!poly.is_empty());
    }

    #[test]
    fn test_binning() {
        let data = vec![1.5, 2.5, 3.5, 4.5];
        let binned = binning(&data, 2);
        assert_eq!(binned.len(), 4);
    }

    #[test]
    fn test_class_distribution() {
        let labels = vec![0, 1, 0, 1, 1];
        let dist = class_distribution(&labels);
        assert_eq!(dist.get(&0), Some(&2));
        assert_eq!(dist.get(&1), Some(&3));
    }

    #[test]
    fn test_feature_hashing() {
        let features = vec!["feature1".to_string(), "feature2".to_string()];
        let hashed = feature_hashing(&features, 10);
        assert_eq!(hashed.len(), 2);
        assert!(hashed[0] < 10);
    }

    #[test]
    fn test_correlation_with_target() {
        let features = vec![vec![1.0, 2.0, 3.0], vec![2.0, 4.0, 6.0]];
        let target = vec![1.0, 2.0, 3.0];
        let corr = correlation_with_target(&features, &target);
        assert_eq!(corr.len(), 2);
    }

    #[test]
    fn test_robust_scale() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 100.0];
        let scaled = robust_scale(&data);
        assert_eq!(scaled.len(), 5);
    }
}
