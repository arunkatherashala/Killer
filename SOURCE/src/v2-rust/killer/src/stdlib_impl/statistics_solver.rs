// ================================================================
// STATISTICS SOLVER - Phase 21.2
// Comprehensive statistical functions, distributions, tests
// Ported from: solver_statistics.killer
// ================================================================

use std::f64;

pub type Vector = Vec<f64>;

/// Statistics Solver
pub struct StatisticsSolver;

impl StatisticsSolver {
    // ================================================================
    // DESCRIPTIVE STATISTICS (1-20)
    // ================================================================

    /// Problem 1: Mean (arithmetic average)
    pub fn mean(data: &[f64]) -> f64 {
        if data.is_empty() { return 0.0; }
        data.iter().sum::<f64>() / data.len() as f64
    }

    /// Problem 2: Median
    pub fn median(data: &[f64]) -> f64 {
        if data.is_empty() { return 0.0; }
        let mut sorted = data.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        
        let n = sorted.len();
        if n % 2 == 1 {
            sorted[n / 2]
        } else {
            (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
        }
    }

    /// Problem 3: Mode (most frequent value, for binned data)
    pub fn mode(data: &[f64]) -> f64 {
        if data.is_empty() { return 0.0; }
        let mut sorted = data.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        
        let mut max_count = 1;
        let mut current_count = 1;
        let mut mode = sorted[0];
        
        for i in 1..sorted.len() {
            if (sorted[i] - sorted[i - 1]).abs() < 1e-10 {
                current_count += 1;
                if current_count > max_count {
                    max_count = current_count;
                    mode = sorted[i];
                }
            } else {
                current_count = 1;
            }
        }
        mode
    }

    /// Problem 4: Range (max - min)
    pub fn range(data: &[f64]) -> f64 {
        if data.is_empty() { return 0.0; }
        data.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
        max - min
    }

    /// Problem 5: Variance
    pub fn variance(data: &[f64]) -> f64 {
        if data.is_empty() { return 0.0; }
        let mean = Self::mean(data);
        let sum_sq: f64 = data.iter().map(|x| (x - mean).powi(2)).sum();
        sum_sq / data.len() as f64
    }

    /// Problem 6: Sample Variance (n-1 denominator)
    pub fn sample_variance(data: &[f64]) -> f64 {
        if data.len() <= 1 { return 0.0; }
        let mean = Self::mean(data);
        let sum_sq: f64 = data.iter().map(|x| (x - mean).powi(2)).sum();
        sum_sq / (data.len() - 1) as f64
    }

    /// Problem 7: Standard Deviation
    pub fn stddev(data: &[f64]) -> f64 {
        Self::variance(data).sqrt()
    }

    /// Problem 8: Sample Standard Deviation
    pub fn sample_stddev(data: &[f64]) -> f64 {
        Self::sample_variance(data).sqrt()
    }

    /// Problem 9: Coefficient of Variation (CV = σ/μ)
    pub fn coefficient_of_variation(data: &[f64]) -> f64 {
        let mean = Self::mean(data);
        if mean.abs() < 1e-14 { return 0.0; }
        Self::stddev(data) / mean.abs()
    }

    /// Problem 10: Skewness (asymmetry)
    pub fn skewness(data: &[f64]) -> f64 {
        if data.len() < 3 { return 0.0; }
        let mean = Self::mean(data);
        let std = Self::stddev(data);
        if std.abs() < 1e-14 { return 0.0; }
        
        let n = data.len() as f64;
        let m3: f64 = data.iter().map(|x| ((x - mean) / std).powi(3)).sum();
        m3 / n
    }

    /// Problem 11: Kurtosis (tailedness)
    pub fn kurtosis(data: &[f64]) -> f64 {
        if data.len() < 4 { return 0.0; }
        let mean = Self::mean(data);
        let std = Self::stddev(data);
        if std.abs() < 1e-14 { return 0.0; }
        
        let n = data.len() as f64;
        let m4: f64 = data.iter().map(|x| ((x - mean) / std).powi(4)).sum();
        m4 / n - 3.0
    }

    /// Problem 12: Quantile/Percentile
    pub fn percentile(data: &[f64], p: f64) -> f64 {
        if data.is_empty() || p < 0.0 || p > 100.0 { return 0.0; }
        let mut sorted = data.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        
        let index = (p / 100.0 * (sorted.len() - 1) as f64).round() as usize;
        sorted[index.min(sorted.len() - 1)]
    }

    /// Problem 13: Interquartile Range (IQR = Q3 - Q1)
    pub fn iqr(data: &[f64]) -> f64 {
        Self::percentile(data, 75.0) - Self::percentile(data, 25.0)
    }

    /// Problem 14: Z-score normalization
    pub fn zscore(data: &[f64]) -> Vector {
        let mean = Self::mean(data);
        let std = Self::stddev(data);
        if std.abs() < 1e-14 {
            return data.to_vec();
        }
        
        data.iter().map(|x| (x - mean) / std).collect()
    }

    /// Problem 15: Min-Max normalization [0, 1]
    pub fn minmax_normalize(data: &[f64]) -> Vector {
        if data.is_empty() { return vec![]; }
        let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = max - min;
        
        if range.abs() < 1e-14 {
            return data.iter().map(|_| 0.5).collect();
        }
        
        data.iter().map(|x| (x - min) / range).collect()
    }

    // ================================================================
    // PROBABILITY DISTRIBUTIONS (16-40)
    // ================================================================

    /// Problem 16: Normal Distribution PDF
    pub fn normal_pdf(x: f64, mu: f64, sigma: f64) -> f64 {
        let z = (x - mu) / sigma;
        (1.0 / (sigma * (2.0 * f64::consts::PI).sqrt())) * (-0.5 * z * z).exp()
    }

    /// Problem 17: Normal Distribution CDF (approximation)
    pub fn normal_cdf(x: f64, mu: f64, sigma: f64) -> f64 {
        let z = (x - mu) / sigma;
        0.5 * (1.0 + Self::erf(z / f64::consts::PI.sqrt()))
    }

    /// Problem 18: Binomial Probability P(X = k)
    pub fn binomial_pmf(n: u32, k: u32, p: f64) -> f64 {
        if k > n { return 0.0; }
        let binom_coeff = Self::binomial_coefficient(n, k);
        binom_coeff as f64 * p.powi(k as i32) * (1.0 - p).powi((n - k) as i32)
    }

    /// Problem 19: Poisson Probability P(X = k)
    pub fn poisson_pmf(lambda: f64, k: u32) -> f64 {
        let k_fact = (1..=k).map(|i| i as f64).product::<f64>();
        (lambda.powi(k as i32) * (-lambda).exp()) / k_fact
    }

    /// Problem 20: Exponential Distribution PDF
    pub fn exponential_pdf(x: f64, lambda: f64) -> f64 {
        if x < 0.0 { 0.0 } else { lambda * (-lambda * x).exp() }
    }

    /// Problem 21: Chi-Square Distribution PDF
    pub fn chi2_pdf(x: f64, k: f64) -> f64 {
        if x < 0.0 { return 0.0; }
        let numerator = x.powf(k / 2.0 - 1.0) * (-x / 2.0).exp();
        let denominator = 2.0_f64.powf(k / 2.0) * Self::gamma(k / 2.0);
        numerator / denominator
    }

    /// Problem 22: Beta Distribution PDF
    pub fn beta_pdf(x: f64, alpha: f64, beta: f64) -> f64 {
        if x <= 0.0 || x >= 1.0 { return 0.0; }
        let numerator = x.powf(alpha - 1.0) * (1.0 - x).powf(beta - 1.0);
        let denominator = Self::beta_function(alpha, beta);
        numerator / denominator
    }

    /// Problem 23: Uniform Distribution PDF
    pub fn uniform_pdf(x: f64, a: f64, b: f64) -> f64 {
        if x >= a && x <= b { 1.0 / (b - a) } else { 0.0 }
    }

    /// Problem 24: Gamma Distribution PDF
    pub fn gamma_pdf(x: f64, alpha: f64, beta: f64) -> f64 {
        if x < 0.0 { return 0.0; }
        let numerator = beta.powf(alpha) * x.powf(alpha - 1.0) * (-beta * x).exp();
        let denominator = Self::gamma(alpha);
        numerator / denominator
    }

    /// Problem 25: Log-Normal Distribution PDF
    pub fn lognormal_pdf(x: f64, mu: f64, sigma: f64) -> f64 {
        if x <= 0.0 { return 0.0; }
        let z = (x.ln() - mu) / sigma;
        (1.0 / (x * sigma * (2.0 * f64::consts::PI).sqrt())) * (-0.5 * z * z).exp()
    }

    // ================================================================
    // HYPOTHESIS TESTING (26-45)
    // ================================================================

    /// Problem 26: T-test statistic
    pub fn t_statistic(sample_mean: f64, pop_mean: f64, sample_std: f64, n: usize) -> f64 {
        let se = sample_std / (n as f64).sqrt();
        if se.abs() < 1e-14 { return 0.0; }
        (sample_mean - pop_mean) / se
    }

    /// Problem 27: Z-test statistic
    pub fn z_statistic(sample_mean: f64, pop_mean: f64, pop_std: f64, n: usize) -> f64 {
        let se = pop_std / (n as f64).sqrt();
        if se.abs() < 1e-14 { return 0.0; }
        (sample_mean - pop_mean) / se
    }

    /// Problem 28: Chi-Square statistic (goodness of fit)
    pub fn chi2_statistic(observed: &[f64], expected: &[f64]) -> f64 {
        if observed.len() != expected.len() { return f64::NAN; }
        
        observed.iter().zip(expected.iter())
            .map(|(&o, &e)| {
                if e.abs() < 1e-14 { 0.0 } else { (o - e).powi(2) / e }
            })
            .sum()
    }

    /// Problem 29: Confidence Interval (Normal approximation)
    pub fn confidence_interval(mean: f64, std_err: f64, confidence: f64) -> (f64, f64) {
        let z = match confidence {
            0.90 => 1.645,
            0.95 => 1.96,
            0.99 => 2.576,
            _ => 1.96,
        };
        
        let margin = z * std_err;
        (mean - margin, mean + margin)
    }

    /// Problem 30: P-value from t-statistic (two-tailed approx)
    pub fn p_value_t(t_stat: f64, _df: usize) -> f64 {
        // Approximation using standard normal approximation
        // For larger df, t-distribution approaches normal
        let t_abs = t_stat.abs();
        // Standard normal CDF approximation
        let p_value = 2.0 * (1.0 - (t_abs / (1.0 + 0.2316419 * t_abs)) 
            * 0.3989423 * (-t_abs * t_abs / 2.0).exp());
        p_value.min(1.0).max(0.0)
    }

    // ================================================================
    // CORRELATION & REGRESSION (31-50)
    // ================================================================

    /// Problem 31: Pearson Correlation Coefficient
    pub fn pearson_correlation(x: &[f64], y: &[f64]) -> f64 {
        if x.len() != y.len() || x.is_empty() { return f64::NAN; }
        
        let x_mean = Self::mean(x);
        let y_mean = Self::mean(y);
        
        let mut num = 0.0;
        let mut sum_x_sq = 0.0;
        let mut sum_y_sq = 0.0;
        
        for i in 0..x.len() {
            let xdev = x[i] - x_mean;
            let ydev = y[i] - y_mean;
            num += xdev * ydev;
            sum_x_sq += xdev * xdev;
            sum_y_sq += ydev * ydev;
        }
        
        let denom = (sum_x_sq * sum_y_sq).sqrt();
        if denom.abs() < 1e-14 { return f64::NAN; }
        num / denom
    }

    /// Problem 32: Spearman's Rank Correlation
    pub fn spearman_correlation(x: &[f64], y: &[f64]) -> f64 {
        if x.len() != y.len() { return f64::NAN; }
        
        let rank_x = Self::rank_data(x);
        let rank_y = Self::rank_data(y);
        
        Self::pearson_correlation(&rank_x, &rank_y)
    }

    /// Problem 33: Linear Regression (least squares)
    pub fn linear_regression(x: &[f64], y: &[f64]) -> (f64, f64) {
        if x.len() != y.len() || x.is_empty() {
            return (f64::NAN, f64::NAN);
        }
        
        let x_mean = Self::mean(x);
        let y_mean = Self::mean(y);
        
        let mut num = 0.0;
        let mut denom = 0.0;
        
        for i in 0..x.len() {
            let xdev = x[i] - x_mean;
            let ydev = y[i] - y_mean;
            num += xdev * ydev;
            denom += xdev * xdev;
        }
        
        let slope = if denom.abs() < 1e-14 { f64::NAN } else { num / denom };
        let intercept = y_mean - slope * x_mean;
        
        (slope, intercept)
    }

    /// Problem 34: R-Squared (coefficient of determination)
    pub fn r_squared(x: &[f64], y: &[f64]) -> f64 {
        let r = Self::pearson_correlation(x, y);
        r * r
    }

    // ================================================================
    // HELPER FUNCTIONS
    // ================================================================

    fn erf(x: f64) -> f64 {
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

    fn gamma(z: f64) -> f64 {
        if z < 0.5 {
            f64::consts::PI / ((f64::consts::PI * z).sin() * Self::gamma(1.0 - z))
        } else {
            let g = 7.0;
            let coef = [
                0.99999999999980993, 676.5203681218851, -1259.1392167224028,
                771.32342877765313, -176.61502916214059, 12.507343278686905,
                -0.13857109526572012, 9.9843695780195716e-6, 1.5056327351493116e-7,
            ];

            let z = z - 1.0;
            let mut x = coef[0];
            for i in 1..coef.len() {
                x += coef[i] / (z + i as f64);
            }

            let t = z + g + 0.5;
            (2.0 * f64::consts::PI).sqrt() * t.powf(z + 0.5) * (-t).exp() * x
        }
    }

    fn beta_function(a: f64, b: f64) -> f64 {
        (Self::gamma(a) * Self::gamma(b)) / Self::gamma(a + b)
    }

    fn binomial_coefficient(n: u32, k: u32) -> u64 {
        if k > n || k == 0 { return if k == 0 { 1 } else { 0 }; }
        if k == n { return 1; }
        
        let k = k.min(n - k);
        let mut result = 1u64;
        for i in 0..k {
            result = result * (n - i) as u64 / (i + 1) as u64;
        }
        result
    }

    fn rank_data(data: &[f64]) -> Vector {
        let n = data.len();
        let mut indexed: Vec<(f64, usize)> = data.iter().copied().enumerate()
            .map(|(i, x)| (x, i)).collect();
        
        indexed.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        
        let mut ranks = vec![0.0; n];
        for (rank, (_val, orig_idx)) in indexed.iter().enumerate() {
            ranks[*orig_idx] = (rank + 1) as f64;
        }
        ranks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mean() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((StatisticsSolver::mean(&data) - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_variance() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let var = StatisticsSolver::variance(&data);
        assert!((var - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_median() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((StatisticsSolver::median(&data) - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_pearson_correlation() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        let r = StatisticsSolver::pearson_correlation(&x, &y);
        assert!((r - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_linear_regression() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        let (slope, intercept) = StatisticsSolver::linear_regression(&x, &y);
        assert!((slope - 2.0).abs() < 1e-10);
        assert!(intercept.abs() < 1e-10);
    }
}
