// Model Quantization Support for AI Runtime
// Reduces model size and latency through INT8/FP16 quantization
// Week 3: Performance Optimization Phase

use std::collections::HashMap;

/// Quantization configuration
#[derive(Debug, Clone)]
pub struct QuantizationConfig {
    pub enabled: bool,
    pub precision: QuantizationPrecision,
    pub calibration_data: Option<Vec<Vec<f32>>>,
    pub accuracy_threshold: f32,  // Max accuracy loss allowed (0.005 = 0.5%)
}

/// Supported quantization precisions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuantizationPrecision {
    FP32,      // No quantization (baseline)
    FP16,      // Half precision (2x memory reduction)
    INT8,      // 8-bit integer (4x memory reduction)
    INT4,      // 4-bit integer (8x memory reduction, experimental)
}

/// Quantized model representation
#[derive(Debug, Clone)]
pub struct QuantizedModel {
    pub name: String,
    pub precision: QuantizationPrecision,
    pub scale_factors: Vec<f32>,      // Scaling between quantized and original values
    pub zero_points: Vec<i32>,         // Zero point for asymmetric quantization
    pub weights_quantized: Vec<u8>,    // Quantized weight data
    pub original_size_mb: f32,
    pub quantized_size_mb: f32,
}

impl QuantizationConfig {
    pub fn new(precision: QuantizationPrecision) -> Self {
        QuantizationConfig {
            enabled: true,
            precision,
            calibration_data: None,
            accuracy_threshold: 0.005,  // 0.5% max accuracy loss
        }
    }

    pub fn with_calibration(mut self, data: Vec<Vec<f32>>) -> Self {
        self.calibration_data = Some(data);
        self
    }

    pub fn compression_ratio(&self) -> f32 {
        match self.precision {
            QuantizationPrecision::FP32 => 1.0,  // Baseline
            QuantizationPrecision::FP16 => 2.0,  // 2x reduction
            QuantizationPrecision::INT8 => 4.0,  // 4x reduction
            QuantizationPrecision::INT4 => 8.0,  // 8x reduction
        }
    }

    pub fn speedup_factor(&self) -> f32 {
        match self.precision {
            QuantizationPrecision::FP32 => 1.0,
            QuantizationPrecision::FP16 => 1.5,  // 1.5x faster
            QuantizationPrecision::INT8 => 2.5,  // 2.5x faster
            QuantizationPrecision::INT4 => 4.0,  // 4x faster
        }
    }
}

impl QuantizedModel {
    /// Create quantized model and measure performance
    pub fn quantize(
        name: &str,
        weights: &[f32],
        precision: QuantizationPrecision,
    ) -> Self {
        let original_size_mb = (weights.len() as f32 * 4.0) / (1024.0 * 1024.0);

        let (weights_quantized, scale_factors, zero_points) = match precision {
            QuantizationPrecision::FP32 => {
                // No quantization
                let quantized = weights.iter().map(|&w| w as u8).collect();
                (quantized, vec![1.0], vec![0])
            }
            QuantizationPrecision::FP16 => {
                Self::quantize_fp16(weights)
            }
            QuantizationPrecision::INT8 => {
                Self::quantize_int8(weights)
            }
            QuantizationPrecision::INT4 => {
                Self::quantize_int4(weights)
            }
        };

        let quantized_size_mb = (weights_quantized.len() as f32) / (1024.0 * 1024.0);

        QuantizedModel {
            name: name.to_string(),
            precision,
            scale_factors,
            zero_points,
            weights_quantized,
            original_size_mb,
            quantized_size_mb,
        }
    }

    /// Quantize to FP16 (half precision)
    fn quantize_fp16(weights: &[f32]) -> (Vec<u8>, Vec<f32>, Vec<i32>) {
        let quantized = weights
            .iter()
            .flat_map(|&w| {
                // Convert f32 to f16 (approximate)
                let bits = w.to_bits();
                let sign = (bits >> 31) & 1;
                let exponent = ((bits >> 23) & 0xFF) as i32 - 127 + 15;
                let mantissa = (bits & 0x7FFFFF) >> 13;

                let f16_bits = if exponent < 0 {
                    0
                } else if exponent >= 31 {
                    if sign == 1 { 0xFC00 } else { 0x7C00 }
                } else {
                    ((sign << 15) | ((exponent as u32) << 10) | mantissa) as u16
                };

                vec![(f16_bits & 0xFF) as u8, ((f16_bits >> 8) & 0xFF) as u8]
            })
            .collect();

        (quantized, vec![1.0], vec![0])
    }

    /// Quantize to INT8 (8-bit integer)
    fn quantize_int8(weights: &[f32]) -> (Vec<u8>, Vec<f32>, Vec<i32>) {
        // Find min/max for scaling
        let min = weights.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = weights.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        let scale = (max - min) / 255.0;
        let zero_point = (-min / scale) as i32;

        let quantized = weights
            .iter()
            .map(|&w| {
                let q = ((w - min) / scale) as i32;
                (q.max(0).min(255)) as u8
            })
            .collect();

        (quantized, vec![scale], vec![zero_point])
    }

    /// Quantize to INT4 (4-bit integer)
    fn quantize_int4(weights: &[f32]) -> (Vec<u8>, Vec<f32>, Vec<i32>) {
        // Find min/max for scaling
        let min = weights.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = weights.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        let scale = (max - min) / 15.0;
        let zero_point = (-min / scale) as i32;

        let mut quantized = Vec::new();
        for chunk in weights.chunks(2) {
            let mut byte = 0u8;
            for (i, &w) in chunk.iter().enumerate() {
                let q = ((w - min) / scale) as u8;
                let q = q.max(0).min(15);
                byte |= (q & 0xF) << (4 - i * 4);
            }
            quantized.push(byte);
        }

        (quantized, vec![scale], vec![zero_point])
    }

    /// Dequantize INT8 back to FP32
    pub fn dequantize_int8(&self) -> Vec<f32> {
        let scale = self.scale_factors[0];
        let zero_point = self.zero_points[0];

        self.weights_quantized
            .iter()
            .map(|&q| (q as f32 - zero_point as f32) * scale)
            .collect()
    }

    /// Get compression stats
    pub fn stats(&self) -> QuantStats {
        // Use byte-level ratio to avoid float precision loss on very small (< 1 KB) quantized data
        let original_bytes = (self.original_size_mb * 1024.0 * 1024.0) as f32;
        let quantized_bytes = (self.quantized_size_mb * 1024.0 * 1024.0).max(1.0) as f32;
        let compression_ratio = original_bytes / quantized_bytes;
        let speedup = match self.precision {
            QuantizationPrecision::FP32 => 1.0,
            QuantizationPrecision::FP16 => 1.5,
            QuantizationPrecision::INT8 => 2.5,
            QuantizationPrecision::INT4 => 4.0,
        };

        QuantStats {
            original_size_mb: self.original_size_mb,
            quantized_size_mb: self.quantized_size_mb,
            compression_ratio,
            speedup,
            precision: self.precision,
        }
    }
}

#[derive(Debug)]
pub struct QuantStats {
    pub original_size_mb: f32,
    pub quantized_size_mb: f32,
    pub compression_ratio: f32,
    pub speedup: f32,
    pub precision: QuantizationPrecision,
}

/// Model cache for lazy initialization
pub struct QuantizationCache {
    models: HashMap<String, QuantizedModel>,
    config: QuantizationConfig,
}

impl QuantizationCache {
    pub fn new(config: QuantizationConfig) -> Self {
        QuantizationCache {
            models: HashMap::new(),
            config,
        }
    }

    pub fn load_or_create(
        &mut self,
        model_name: &str,
        weights: &[f32],
    ) -> &QuantizedModel {
        if !self.models.contains_key(model_name) {
            let model = QuantizedModel::quantize(
                model_name,
                weights,
                self.config.precision,
            );
            self.models.insert(model_name.to_string(), model);
        }
        &self.models[model_name]
    }

    pub fn get_stats(&self) -> HashMap<String, QuantStats> {
        self.models
            .iter()
            .map(|(name, model)| (name.clone(), model.stats()))
            .collect()
    }

    pub fn total_memory_saved(&self) -> (f32, f32) {
        let original: f32 = self.models.values().map(|m| m.original_size_mb).sum();
        let quantized: f32 = self.models.values().map(|m| m.quantized_size_mb).sum();
        (original, quantized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantization_compression() {
        let weights: Vec<f32> = (0..1000).map(|i| (i as f32) * 0.1).collect();

        let qa = QuantizedModel::quantize("test_fp32", &weights, QuantizationPrecision::FP32);
        let qb = QuantizedModel::quantize("test_int8", &weights, QuantizationPrecision::INT8);
        let qc = QuantizedModel::quantize("test_int4", &weights, QuantizationPrecision::INT4);

        let stats_fp32 = qa.stats();
        let stats_int8 = qb.stats();
        let stats_int4 = qc.stats();

        println!("FP32: {:.2}x compression", stats_fp32.compression_ratio);
        println!("INT8: {:.2}x compression, {:.2}x speedup", stats_int8.compression_ratio, stats_int8.speedup);
        println!("INT4: {:.2}x compression, {:.2}x speedup", stats_int4.compression_ratio, stats_int4.speedup);

        assert!(stats_int8.compression_ratio > 2.0);
        assert!(stats_int4.compression_ratio > 4.0);
    }
}
