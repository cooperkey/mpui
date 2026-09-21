use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;

/// Render the visualizer panel — spectrum bars from FFT data.
pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let is_focused = app.focused_panel == crate::app::FocusedPanel::Visualizer;
    let border_color = if is_focused {
        app.theme.border_focused
    } else {
        app.theme.border
    };

    let title = if is_focused {
        " [visualizer] "
    } else {
        " visualizer "
    };

    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(app.theme.title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let bar_chars = &app.config.visualizer.bar_chars;
    if bar_chars.is_empty() {
        return;
    }

    let bar_width = app.config.visualizer.bar_width.max(1) as usize;
    let max_bars = inner.width as usize / bar_width;
    let num_bars = if app.config.visualizer.bar_count > 0 {
        (app.config.visualizer.bar_count as usize).min(max_bars)
    } else {
        max_bars
    };

    if num_bars == 0 {
        return;
    }

    let is_active = {
        let state = app.playback.lock();
        state.playing && !state.paused
    };

    let spectrum = if is_active {
        app.spectrum.lock().clone()
    } else {
        Vec::new()
    };

    let sample_rate = {
        let rate = app.playback.lock().sample_rate;
        if rate > 0 {
            rate as f32
        } else {
            44100.0
        }
    };

    let targets = if !spectrum.is_empty() {
        bin_spectrum(&spectrum, num_bars, sample_rate)
    } else {
        vec![0.0; num_bars]
    };

    if app.visualizer_bars.len() != num_bars {
        app.visualizer_bars.resize(num_bars, 0.0);
    }

    let smoothing = (app.config.visualizer.smoothing as f32).clamp(0.0, 0.95);
    let gravity_drop = (app.config.visualizer.gravity as f32 * 0.025).max(0.005);

    for i in 0..num_bars {
        let target = targets[i];
        let prev = app.visualizer_bars[i];
        if target >= prev {
            // Instantaneous attack for punchy transients
            app.visualizer_bars[i] = target;
        } else {
            // Exponential smoothing combined with gravity falloff
            let smoothed = prev * smoothing + target * (1.0 - smoothing);
            let grav = prev - gravity_drop;
            let next_val = smoothed.min(grav).max(target).max(0.0);
            app.visualizer_bars[i] = if next_val < 0.005 { 0.0 } else { next_val };
        }
    }

    let height = inner.height as usize;
    let bar_color = app.theme.playing_fg;

    // Render each row from top to bottom.
    let lines: Vec<Line> = (0..height)
        .map(|row| {
            let cell_top = (height - row) as f32;
            let cell_bottom = cell_top - 1.0;
            let spans: Vec<Span> = app
                .visualizer_bars
                .iter()
                .flat_map(|&magnitude| {
                    let bar_h = magnitude * height as f32;
                    let ch = if bar_h >= cell_top {
                        bar_chars.last().map(|s| s.as_str()).unwrap_or("█")
                    } else if bar_h <= cell_bottom {
                        " "
                    } else {
                        let fraction = (bar_h - cell_bottom).clamp(0.0, 1.0);
                        let idx = ((fraction * bar_chars.len() as f32) as usize)
                            .min(bar_chars.len() - 1);
                        bar_chars[idx].as_str()
                    };
                    std::iter::repeat_n(
                        Span::styled(ch, Style::default().fg(bar_color)),
                        bar_width,
                    )
                })
                .collect();
            Line::from(spans)
        })
        .collect();

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

/// Compute logarithmic frequency bands with equal-loudness weighting and decibel scaling.
pub fn bin_spectrum(spectrum: &[f32], num_bars: usize, sample_rate: f32) -> Vec<f32> {
    if num_bars == 0 || spectrum.is_empty() {
        return Vec::new();
    }

    let spec_len = spectrum.len();
    let fft_size = (spec_len * 2) as f32;
    let nyquist = sample_rate / 2.0;

    let f_min: f32 = 30.0;
    let f_max: f32 = 16000.0f32.min(nyquist - 50.0);

    if f_max <= f_min {
        return vec![0.0; num_bars];
    }

    let log_ratio = (f_max / f_min).ln();

    (0..num_bars)
        .map(|i| {
            let frac_low = i as f32 / num_bars as f32;
            let frac_high = (i + 1) as f32 / num_bars as f32;

            let f_low = f_min * (frac_low * log_ratio).exp();
            let f_high = f_min * (frac_high * log_ratio).exp();
            let f_center = (f_low * f_high).sqrt();

            let k_start = ((f_low * fft_size / sample_rate).floor() as usize)
                .max(1)
                .min(spec_len - 1);
            let k_end = ((f_high * fft_size / sample_rate).ceil() as usize)
                .max(k_start + 1)
                .min(spec_len);

            let slice = &spectrum[k_start..k_end];
            let max_val = slice.iter().copied().fold(0.0f32, f32::max);
            let avg_val = slice.iter().copied().sum::<f32>() / slice.len() as f32;
            let raw_mag = 0.75 * max_val + 0.25 * avg_val;

            // Equal-loudness tilt weighting relative to 250 Hz
            let weight = (f_center / 250.0).powf(0.38).clamp(0.5, 5.0);
            let weighted_mag = raw_mag * weight;

            // Decibel scaling: reference 8.0 (~0 dBFS peak in Parseval normalization)
            let db = 20.0 * (weighted_mag / 8.0 + 1e-6).log10();
            let min_db = -55.0;
            let max_db = 0.0;

            ((db - min_db) / (max_db - min_db)).clamp(0.0, 1.0)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bin_spectrum_empty() {
        assert!(bin_spectrum(&[], 10, 44100.0).is_empty());
        assert!(bin_spectrum(&[1.0; 1024], 0, 44100.0).is_empty());
    }

    #[test]
    fn test_bin_spectrum_silence() {
        let spectrum = vec![0.0; 1024];
        let bins = bin_spectrum(&spectrum, 20, 44100.0);
        assert_eq!(bins.len(), 20);
        for val in bins {
            assert_eq!(val, 0.0);
        }
    }

    #[test]
    fn test_bin_spectrum_bass_response() {
        let mut spectrum = vec![0.0; 1024];
        // 40 Hz at 44100 Hz with N=2048: bin 1-2 (Bar 0)
        // 60 Hz at 44100 Hz with N=2048: bin 3 (Bar 1)
        spectrum[1] = 4.0;
        spectrum[3] = 4.0;

        let bins = bin_spectrum(&spectrum, 10, 44100.0);
        assert_eq!(bins.len(), 10);
        // The lowest bars must detect the bass energy
        assert!(bins[0] > 0.5, "Expected bar 0 to activate for 40Hz, got {}", bins[0]);
        assert!(bins[1] > 0.5, "Expected bar 1 to activate for 60Hz, got {}", bins[1]);
        // Treble bars must be silent
        assert_eq!(bins[9], 0.0, "Expected treble bar to be 0 for bass, got {}", bins[9]);
    }

    #[test]
    fn test_bin_spectrum_treble_response() {
        let mut spectrum = vec![0.0; 1024];
        // 10 kHz at 44100 Hz with N=2048: bin = round(10000 * 2048 / 44100) = 464
        spectrum[464] = 2.0;

        let bins = bin_spectrum(&spectrum, 10, 44100.0);
        assert_eq!(bins.len(), 10);
        // Treble bar must activate
        assert!(bins[8] > 0.5 || bins[9] > 0.5, "Expected high bar to activate for 10kHz");
        // Bass bar must be silent
        assert_eq!(bins[0], 0.0, "Expected bass bar to be 0 for 10kHz, got {}", bins[0]);
    }

    #[test]
    fn test_bin_spectrum_full_range_response() {
        let mut spectrum = vec![0.0; 1024];
        // 50 Hz (bin 2), 500 Hz (bin 23), 2500 Hz (bin 116), 8000 Hz (bin 372)
        spectrum[2] = 2.0;
        spectrum[23] = 1.5;
        spectrum[116] = 1.0;
        spectrum[372] = 0.8;

        let bins = bin_spectrum(&spectrum, 16, 44100.0);
        assert_eq!(bins.len(), 16);
        let active_count = bins.iter().filter(|&&v| v > 0.3).count();
        assert!(active_count >= 4, "Expected at least 4 active bars across the spectrum, got {}", active_count);
    }
}
