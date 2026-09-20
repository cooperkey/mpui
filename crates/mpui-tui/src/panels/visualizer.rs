use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;

/// Render the visualizer panel — spectrum bars from FFT data.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" visualizer ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let bar_chars = &app.config.visualizer.bar_chars;
    if bar_chars.is_empty() {
        return;
    }

    let spectrum = app.spectrum.lock();
    if spectrum.is_empty() {
        return;
    }

    let bar_width = app.config.visualizer.bar_width.max(1) as usize;
    let num_bars = (inner.width as usize) / bar_width;
    let height = inner.height as usize;

    // Bin the spectrum into `num_bars` bands.
    let bins = bin_spectrum(&spectrum, num_bars);

    // Render each row from top to bottom.
    let lines: Vec<Line> = (0..height)
        .map(|row| {
            let threshold = height - row; // row 0 = top = highest threshold
            let spans: Vec<Span> = bins
                .iter()
                .flat_map(|&magnitude| {
                    let level = (magnitude * height as f32).ceil() as usize;
                    let ch = if level >= threshold {
                        // Select bar char based on fractional level within this cell.
                        let frac_idx =
                            ((magnitude * height as f32 - (threshold - 1) as f32) * (bar_chars.len() - 1) as f32)
                                .clamp(0.0, (bar_chars.len() - 1) as f32) as usize;
                        bar_chars[frac_idx].as_str()
                    } else {
                        " "
                    };
                    // Repeat for bar_width characters.
                    std::iter::repeat_n(
                        Span::styled(ch, Style::default().fg(Color::Blue)),
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

/// Average spectrum magnitudes into `num_bars` bins.
fn bin_spectrum(spectrum: &[f32], num_bars: usize) -> Vec<f32> {
    if num_bars == 0 || spectrum.is_empty() {
        return Vec::new();
    }
    let bin_size = spectrum.len() / num_bars;
    if bin_size == 0 {
        return spectrum.iter().copied().take(num_bars).collect();
    }
    (0..num_bars)
        .map(|i| {
            let start = i * bin_size;
            let end = (start + bin_size).min(spectrum.len());
            let sum: f32 = spectrum[start..end].iter().sum();
            (sum / bin_size as f32).clamp(0.0, 1.0)
        })
        .collect()
}
