// ================================================================
// SIGNAL PROCESSING SOLVER - Phase 21.3
// Fourier analysis, filtering, wavelets, time-frequency analysis
// Ported from: solver_signal_processing.killer
// ================================================================

use std::f64;

pub type Signal = Vec<f64>;

/// Signal Processing Solver
pub struct SignalProcessingSolver;

impl SignalProcessingSolver {
    // ================================================================
    // FOURIER ANALYSIS (1-20)
    // ================================================================

    /// Problem 1: Discrete Fourier Transform (DFT) - Naive O(n²)
    pub fn dft(signal: &Signal) -> Vec<Complex> {
        let n = signal.len();
        let mut result = Vec::new();
        
        for k in 0..n {
            let mut real = 0.0;
            let mut imag = 0.0;
            
            for n_idx in 0..n {
                let angle = -2.0 * f64::consts::PI * k as f64 * n_idx as f64 / n as f64;
                real += signal[n_idx] * angle.cos();
                imag += signal[n_idx] * angle.sin();
            }
            
            result.push(Complex { real, imag });
        }
        result
    }

    /// Problem 2: Inverse DFT
    pub fn idft(spectrum: &[Complex]) -> Signal {
        let n = spectrum.len();
        let mut result = Vec::new();
        
        for k in 0..n {
            let mut real = 0.0;
            
            for n_idx in 0..n {
                let angle = 2.0 * f64::consts::PI * k as f64 * n_idx as f64 / n as f64;
                real += spectrum[n_idx].magnitude() * angle.cos();
            }
            
            result.push(real / n as f64);
        }
        result
    }

    /// Problem 3: Power Spectrum
    pub fn power_spectrum(spectrum: &[Complex]) -> Signal {
        spectrum.iter()
            .map(|c| c.magnitude().powi(2))
            .collect()
    }

    /// Problem 4: Magnitude Spectrum
    pub fn magnitude_spectrum(spectrum: &[Complex]) -> Signal {
        spectrum.iter()
            .map(|c| c.magnitude())
            .collect()
    }

    /// Problem 5: Phase Spectrum
    pub fn phase_spectrum(spectrum: &[Complex]) -> Signal {
        spectrum.iter()
            .map(|c| c.phase())
            .collect()
    }

    /// Problem 6: Parseval's Theorem
    pub fn parseval_energy(signal: &Signal, spectrum: &[Complex]) -> (f64, f64) {
        let time_energy: f64 = signal.iter().map(|x| x * x).sum();
        let freq_energy: f64 = spectrum.iter().map(|c| c.magnitude().powi(2)).sum();
        
        (time_energy, freq_energy / signal.len() as f64)
    }

    // ================================================================
    // FILTERING (7-25)
    // ================================================================

    /// Problem 7: Moving Average Filter
    pub fn moving_average(signal: &Signal, window_size: usize) -> Signal {
        if window_size < 1 || signal.is_empty() { return signal.to_vec(); }
        
        let mut result = Vec::new();
        
        for i in 0..signal.len() {
            let start = if i >= window_size { i - window_size + 1 } else { 0 };
            let end = i + 1;
            let avg: f64 = signal[start..end].iter().sum::<f64>() / (end - start) as f64;
            result.push(avg);
        }
        result
    }

    /// Problem 8: Exponential Moving Average
    pub fn exponential_moving_avg(signal: &Signal, alpha: f64) -> Signal {
        if signal.is_empty() { return vec![]; }
        
        let mut result = vec![signal[0]];
        
        for i in 1..signal.len() {
            let ema = alpha * signal[i] + (1.0 - alpha) * result[i - 1];
            result.push(ema);
        }
        result
    }

    /// Problem 9: Low-Pass Filter (Simple RC filter)
    pub fn low_pass_filter(signal: &Signal, cutoff_freq: f64, sample_rate: f64) -> Signal {
        let rc = 1.0 / (2.0 * f64::consts::PI * cutoff_freq);
        let dt = 1.0 / sample_rate;
        let alpha = dt / (rc + dt);
        
        Self::exponential_moving_avg(signal, alpha)
    }

    /// Problem 10: High-Pass Filter
    pub fn high_pass_filter(signal: &Signal, cutoff_freq: f64, sample_rate: f64) -> Signal {
        let low = Self::low_pass_filter(signal, cutoff_freq, sample_rate);
        signal.iter().zip(low.iter())
            .map(|(s, l)| s - l)
            .collect()
    }

    /// Problem 11: Butterworth Filter Order
    pub fn butterworth_filter_order(cutoff_db: f64, rolloff_db_oct: f64) -> usize {
        let order = cutoff_db.abs() / rolloff_db_oct;
        (order.ceil() as usize).max(1)
    }

    // ================================================================
    // WINDOWING (12-20)
    // ================================================================

    /// Problem 12: Hann Window
    pub fn hann_window(n: usize) -> Signal {
        (0..n)
            .map(|i| 0.5 * (1.0 - ((2.0 * f64::consts::PI * i as f64) / (n as f64 - 1.0)).cos()))
            .collect()
    }

    /// Problem 13: Hamming Window
    pub fn hamming_window(n: usize) -> Signal {
        (0..n)
            .map(|i| 0.54 - 0.46 * ((2.0 * f64::consts::PI * i as f64) / (n as f64 - 1.0)).cos())
            .collect()
    }

    /// Problem 14: Blackman Window
    pub fn blackman_window(n: usize) -> Signal {
        (0..n)
            .map(|i| {
                let angle = 2.0 * f64::consts::PI * i as f64 / (n as f64 - 1.0);
                0.42 - 0.5 * angle.cos() + 0.08 * (2.0 * angle).cos()
            })
            .collect()
    }

    /// Problem 15: Apply Window to Signal
    pub fn window_signal(signal: &Signal, window: &Signal) -> Signal {
        signal.iter().zip(window.iter())
            .map(|(s, w)| s * w)
            .collect()
    }

    // ================================================================
    // SPECTRAL ANALYSIS (20-35)
    // ================================================================

    /// Problem 16: Periodogram (Power Spectral Density estimate)
    pub fn periodogram(signal: &Signal, sample_rate: f64) -> (Signal, Signal) {
        let windowed = Self::window_signal(signal, &Self::hann_window(signal.len()));
        let spectrum = Self::dft(&windowed);
        let psd = Self::power_spectrum(&spectrum);
        
        let freq: Signal = (0..psd.len())
            .map(|k| k as f64 * sample_rate / signal.len() as f64)
            .collect();
        
        (freq, psd)
    }

    /// Problem 17: Spectral Centroid
    pub fn spectral_centroid(psd: &Signal, freqs: &Signal) -> f64 {
        if psd.is_empty() { return 0.0; }
        
        let numerator: f64 = psd.iter().zip(freqs.iter())
            .map(|(p, f)| p * f)
            .sum();
        
        let denominator: f64 = psd.iter().sum();
        
        if denominator.abs() < 1e-14 { return 0.0; }
        numerator / denominator
    }

    /// Problem 18: Spectral Rolloff
    pub fn spectral_rolloff(psd: &Signal, freqs: &Signal, threshold: f64) -> f64 {
        let total_power: f64 = psd.iter().sum();
        let target = total_power * threshold;
        
        let mut cumulative = 0.0;
        for (p, f) in psd.iter().zip(freqs.iter()) {
            cumulative += p;
            if cumulative >= target {
                return *f;
            }
        }
        freqs.last().cloned().unwrap_or(0.0)
    }

    /// Problem 19: Spectral Flux (change rate)
    pub fn spectral_flux(psd1: &Signal, psd2: &Signal) -> f64 {
        psd1.iter().zip(psd2.iter())
            .map(|(p1, p2)| (p2 - p1).powi(2))
            .sum::<f64>()
            .sqrt()
    }

    // ================================================================
    // TIME-FREQUENCY ANALYSIS (25-40)
    // ================================================================

    /// Problem 20: Short-Time Fourier Transform (STFT)
    pub fn stft(signal: &Signal, frame_size: usize, overlap: usize, sample_rate: f64) -> Vec<Signal> {
        let mut result = Vec::new();
        let hop_size = frame_size - overlap;
        
        for start in (0..signal.len()).step_by(hop_size) {
            let end = (start + frame_size).min(signal.len());
            if end - start < frame_size { break; }
            
            let frame = signal[start..end].to_vec();
            let spectrum = Self::dft(&frame);
            let psd = Self::power_spectrum(&spectrum);
            result.push(psd);
        }
        result
    }

    /// Problem 21: Spectral Contrast
    pub fn spectral_contrast(psd: &Signal, freq_bands: usize) -> Signal {
        if psd.is_empty() { return vec![]; }
        
        let band_size = psd.len() / freq_bands;
        let mut contrasts = Vec::new();
        
        for b in 0..freq_bands {
            let start = b * band_size;
            let end = ((b + 1) * band_size).min(psd.len());
            
            let band = &psd[start..end];
            let peak = band.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let valley = band.iter().cloned().fold(f64::INFINITY, f64::min);
            
            if peak > 0.0 {
                contrasts.push((peak - valley).log10());
            }
        }
        contrasts
    }

    // ================================================================
    // FEATURE EXTRACTION (30-45)
    // ================================================================

    /// Problem 22: Zero Crossing Rate
    pub fn zero_crossing_rate(signal: &Signal) -> f64 {
        if signal.is_empty() { return 0.0; }
        
        let mut crossings = 0;
        for i in 1..signal.len() {
            if signal[i - 1] * signal[i] < 0.0 {
                crossings += 1;
            }
        }
        crossings as f64 / signal.len() as f64
    }

    /// Problem 23: RMS Energy
    pub fn rms_energy(signal: &Signal) -> f64 {
        if signal.is_empty() { return 0.0; }
        (signal.iter().map(|x| x * x).sum::<f64>() / signal.len() as f64).sqrt()
    }

    /// Problem 24: Crest Factor (Peak-to-RMS ratio)
    pub fn crest_factor(signal: &Signal) -> f64 {
        let peak = signal.iter().map(|x| x.abs()).fold(f64::NEG_INFINITY, f64::max);
        let rms = Self::rms_energy(signal);
        
        if rms.abs() < 1e-14 { return 0.0; }
        peak / rms
    }

    // ================================================================
    // CONVOLUTION & CORRELATION (35-50)
    // ================================================================

    /// Problem 25: Cross-correlation
    pub fn cross_correlation(signal1: &Signal, signal2: &Signal) -> Signal {
        if signal1.is_empty() || signal2.is_empty() { return vec![]; }
        
        let n1 = signal1.len();
        let n2 = signal2.len();
        let mut result = vec![0.0; n1 + n2 - 1];
        
        for i in 0..n1 {
            for j in 0..n2 {
                result[i + j] += signal1[i] * signal2[j];
            }
        }
        result
    }
}

/// Complex Number representation
#[derive(Clone, Copy, Debug)]
pub struct Complex {
    pub real: f64,
    pub imag: f64,
}

impl Complex {
    pub fn new(real: f64, imag: f64) -> Self {
        Complex { real, imag }
    }

    pub fn magnitude(&self) -> f64 {
        (self.real * self.real + self.imag * self.imag).sqrt()
    }

    pub fn phase(&self) -> f64 {
        self.imag.atan2(self.real)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_moving_average() {
        let signal = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let smoothed = SignalProcessingSolver::moving_average(&signal, 2);
        assert_eq!(smoothed.len(), 5);
        assert!((smoothed[1] - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_zero_crossing_rate() {
        let signal = vec![1.0, -1.0, 1.0, -1.0];
        let zcr = SignalProcessingSolver::zero_crossing_rate(&signal);
        assert!((zcr - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_rms_energy() {
        let signal = vec![3.0, 4.0];
        let rms = SignalProcessingSolver::rms_energy(&signal);
        assert!((rms - 5.0 / f64::sqrt(2.0)).abs() < 1e-10);
    }

    #[test]
    fn test_hann_window() {
        let window = SignalProcessingSolver::hann_window(5);
        assert_eq!(window.len(), 5);
        assert!(window[0] < 1e-10);  // Should be near 0 at boundaries
    }
}
