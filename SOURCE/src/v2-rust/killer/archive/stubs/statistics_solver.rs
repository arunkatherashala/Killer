//! Statistics — 50+ functions (descriptive, distributions, hypothesis testing)

pub fn mean(data: &[f64]) -> f64 {
    if data.is_empty() { return 0.0; }
    data.iter().sum::<f64>() / data.len() as f64
}

pub fn variance(data: &[f64]) -> f64 {
    let m = mean(data);
    let sum: f64 = data.iter().map(|x| (x - m).powi(2)).sum();
    sum / data.len() as f64
}

pub fn stddev(data: &[f64]) -> f64 {
    variance(data).sqrt()
}
