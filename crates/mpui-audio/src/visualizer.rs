use std::collections::VecDeque;
use std::sync::Arc;

use parking_lot::Mutex;
use rustfft::{num_complex::Complex, Fft, FftPlanner};

/// FFT-based spectrum analyzer for the visualizer.
///
/// Receives a copy of audio samples (post-EQ), computes magnitude spectrum,
/// and exposes the result for the TUI to render.
pub struct SpectrumAnalyzer {
    fft_size: usize,
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    history: VecDeque<f32>,
    buffer: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
    samples_since_last_fft: usize,
    fft_hop_size: usize,
    /// Latest magnitude spectrum, shared with the UI thread.
    pub spectrum: Arc<Mutex<Vec<f32>>>,
}

impl SpectrumAnalyzer {
    pub fn new(fft_size: usize) -> Self {
        let spectrum = Arc::new(Mutex::new(vec![0.0; fft_size / 2]));
        Self::with_spectrum(fft_size, spectrum)
    }

    pub fn with_spectrum(fft_size: usize, spectrum: Arc<Mutex<Vec<f32>>>) -> Self {
        let window = hann_window(fft_size);
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(fft_size);
        let scratch_len = fft.get_inplace_scratch_len();
        let buffer = vec![Complex::new(0.0, 0.0); fft_size];
        let scratch = vec![Complex::new(0.0, 0.0); scratch_len];
        let fft_hop_size = 1024;

        Self {
            fft_size,
            fft,
            window,
            history: VecDeque::with_capacity(fft_size),
            buffer,
            scratch,
            samples_since_last_fft: 1024,
            fft_hop_size,
            spectrum,
        }
    }

    /// Compute the magnitude spectrum from a buffer of mono f32 samples using a sliding window.
    pub fn analyze(&mut self, samples: &[f32]) {
        if samples.len() >= self.fft_size {
            self.history.clear();
            self.history.extend(&samples[samples.len() - self.fft_size..]);
        } else {
            let to_remove = (self.history.len() + samples.len()).saturating_sub(self.fft_size);
            if to_remove > 0 {
                self.history.drain(..to_remove.min(self.history.len()));
            }
            self.history.extend(samples);
        }
        self.samples_since_last_fft += samples.len();

        if self.history.len() < self.fft_size || self.samples_since_last_fft < self.fft_hop_size {
            return;
        }
        self.samples_since_last_fft = 0;

        for (i, &s) in self.history.iter().enumerate() {
            self.buffer[i] = Complex::new(s * self.window[i], 0.0);
        }

        self.fft.process_with_scratch(&mut self.buffer, &mut self.scratch);

        let half = self.fft_size / 2;
        // Parseval unitary normalization (1 / sqrt(N))
        let norm = 1.0 / (self.fft_size as f32).sqrt();
        let mut spec = self.spectrum.lock();
        if spec.len() != half {
            spec.resize(half, 0.0);
        }
        for i in 0..half {
            spec[i] = self.buffer[i].norm() * norm;
        }
    }

    /// Get a handle to the shared spectrum data for the UI thread.
    pub fn spectrum_handle(&self) -> Arc<Mutex<Vec<f32>>> {
        self.spectrum.clone()
    }
}

/// Generate a Hann window of the given size.
fn hann_window(size: usize) -> Vec<f32> {
    (0..size)
        .map(|i| {
            0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (size - 1) as f32).cos())
        })
        .collect()
}
