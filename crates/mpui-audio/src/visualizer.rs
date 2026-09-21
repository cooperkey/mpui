use std::collections::VecDeque;
use std::sync::Arc;

use parking_lot::Mutex;
use rustfft::{num_complex::Complex, FftPlanner};

/// FFT-based spectrum analyzer for the visualizer.
///
/// Receives a copy of audio samples (post-EQ), computes magnitude spectrum,
/// and exposes the result for the TUI to render.
pub struct SpectrumAnalyzer {
    fft_size: usize,
    planner: FftPlanner<f32>,
    window: Vec<f32>,
    history: VecDeque<f32>,
    /// Latest magnitude spectrum, shared with the UI thread.
    pub spectrum: Arc<Mutex<Vec<f32>>>,
}

impl SpectrumAnalyzer {
    pub fn new(fft_size: usize) -> Self {
        let window = hann_window(fft_size);
        let spectrum = Arc::new(Mutex::new(vec![0.0; fft_size / 2]));
        Self {
            fft_size,
            planner: FftPlanner::new(),
            window,
            history: VecDeque::with_capacity(fft_size),
            spectrum,
        }
    }

    /// Compute the magnitude spectrum from a buffer of mono f32 samples using a sliding window.
    pub fn analyze(&mut self, samples: &[f32]) {
        for &s in samples {
            if self.history.len() >= self.fft_size {
                self.history.pop_front();
            }
            self.history.push_back(s);
        }

        if self.history.len() < self.fft_size {
            return;
        }

        let fft = self.planner.plan_fft_forward(self.fft_size);

        let mut buffer: Vec<Complex<f32>> = self
            .history
            .iter()
            .enumerate()
            .map(|(i, &s)| Complex::new(s * self.window[i], 0.0))
            .collect();

        fft.process(&mut buffer);

        let half = self.fft_size / 2;
        // Parseval unitary normalization (1 / sqrt(N))
        let norm = 1.0 / (self.fft_size as f32).sqrt();
        let mut spec = self.spectrum.lock();
        spec.resize(half, 0.0);
        for i in 0..half {
            spec[i] = buffer[i].norm() * norm;
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
