/// 10-band equalizer applied as biquad filters in the audio pipeline.
///
/// Bands: 32hz, 64hz, 128hz, 256hz, 512hz, 1khz, 2khz, 4khz, 8khz, 16khz
/// Gain range: -12db to +12db per band.

pub const BAND_FREQUENCIES: [f32; 10] = [
    32.0, 64.0, 128.0, 256.0, 512.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

pub const GAIN_MIN: f32 = -12.0;
pub const GAIN_MAX: f32 = 12.0;

/// Coefficients for a second-order biquad filter (peaking EQ).
#[derive(Debug, Clone, Copy)]
struct BiquadCoeffs {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

/// Per-channel state for a single biquad filter.
#[derive(Debug, Clone, Copy, Default)]
struct BiquadState {
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl BiquadState {
    fn process(&mut self, coeffs: &BiquadCoeffs, input: f32) -> f32 {
        let output = coeffs.b0 * input + coeffs.b1 * self.x1 + coeffs.b2 * self.x2
            - coeffs.a1 * self.y1
            - coeffs.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;
        output
    }
}

/// Compute peaking EQ biquad coefficients.
fn peaking_eq(freq: f32, gain_db: f32, q: f32, sample_rate: f32) -> BiquadCoeffs {
    let a = 10.0_f32.powf(gain_db / 40.0);
    let w0 = 2.0 * std::f32::consts::PI * freq / sample_rate;
    let alpha = w0.sin() / (2.0 * q);

    let b0 = 1.0 + alpha * a;
    let b1 = -2.0 * w0.cos();
    let b2 = 1.0 - alpha * a;
    let a0 = 1.0 + alpha / a;
    let a1 = -2.0 * w0.cos();
    let a2 = 1.0 - alpha / a;

    BiquadCoeffs {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    }
}

/// 10-band parametric equalizer.
pub struct Equalizer {
    coeffs: [BiquadCoeffs; 10],
    /// Per-band, per-channel state. Indexed as [band][channel].
    state: Vec<[BiquadState; 10]>,
    gains: [f32; 10],
    enabled: bool,
    sample_rate: f32,
    channels: usize,
}

impl Equalizer {
    pub fn new(sample_rate: u32, channels: usize) -> Self {
        let sr = sample_rate as f32;
        let coeffs = std::array::from_fn(|i| peaking_eq(BAND_FREQUENCIES[i], 0.0, 1.0, sr));
        let state = vec![[BiquadState::default(); 10]; channels];

        Self {
            coeffs,
            state,
            gains: [0.0; 10],
            enabled: true,
            sample_rate: sr,
            channels,
        }
    }

    /// Set gain for a specific band (clamped to GAIN_MIN..GAIN_MAX).
    pub fn set_band(&mut self, band: usize, gain_db: f32) {
        if band >= 10 {
            return;
        }
        let gain = gain_db.clamp(GAIN_MIN, GAIN_MAX);
        self.gains[band] = gain;
        self.coeffs[band] = peaking_eq(BAND_FREQUENCIES[band], gain, 1.0, self.sample_rate);
    }

    /// Apply all 10 bands from a preset array.
    pub fn set_all_bands(&mut self, gains: &[f32; 10]) {
        for (i, &g) in gains.iter().enumerate() {
            self.set_band(i, g);
        }
    }

    pub fn gains(&self) -> &[f32; 10] {
        &self.gains
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Process interleaved audio samples in-place.
    pub fn process(&mut self, samples: &mut [f32]) {
        if !self.enabled {
            return;
        }

        for frame in samples.chunks_exact_mut(self.channels) {
            for (ch, sample) in frame.iter_mut().enumerate() {
                let ch_state = &mut self.state[ch];
                let mut val = *sample;
                for band in 0..10 {
                    val = ch_state[band].process(&self.coeffs[band], val);
                }
                *sample = val;
            }
        }
    }

    /// Reset filter state (call on track change to avoid transients).
    pub fn reset_state(&mut self) {
        for ch in &mut self.state {
            for s in ch.iter_mut() {
                *s = BiquadState::default();
            }
        }
    }
}
