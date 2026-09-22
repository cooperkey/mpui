use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;

/// Render the now-playing bar — current track info + progress.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let state = app.playback.lock();
    let track_info = app
        .queue
        .current_track()
        .or(app.current_playing_track)
        .and_then(|id| app.library.get(id));

    let title = track_info
        .map(|t| t.display_title())
        .unwrap_or("---");
    let artist = track_info
        .map(|t| t.display_artist())
        .unwrap_or("---");

    let position = format_duration(state.position);
    let duration = format_duration(state.duration);

    let status_char = if state.paused {
        "||"
    } else if state.playing {
        "|>"
    } else {
        "--"
    };

    let line1 = Line::from(vec![
        Span::styled(
            format!(" {status_char} "),
            Style::default().fg(app.theme.playing_fg),
        ),
        Span::styled(artist, Style::default().fg(app.theme.muted_fg)),
        Span::raw(" - "),
        Span::styled(title, Style::default().fg(app.theme.fg)),
        Span::raw("  "),
        Span::styled(
            format!("{position} / {duration}"),
            Style::default().fg(app.theme.muted_fg),
        ),
    ]);

    let bar_width = (area.width as usize).saturating_sub(2);
    let ratio = if state.duration.as_secs_f64() > 0.0 {
        (state.position.as_secs_f64() / state.duration.as_secs_f64()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let filled_count = ((ratio * bar_width as f64).round() as usize).min(bar_width);
    let empty_count = bar_width.saturating_sub(filled_count);

    let line2 = Line::from(vec![
        Span::raw(" "),
        Span::styled(
            "━".repeat(filled_count),
            Style::default().fg(app.theme.progress_filled),
        ),
        Span::styled(
            "─".repeat(empty_count),
            Style::default().fg(app.theme.progress_empty),
        ),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(app.theme.border));

    let paragraph = Paragraph::new(vec![line1, line2]).block(block);
    frame.render_widget(paragraph, area);
}

fn format_duration(d: std::time::Duration) -> String {
    let total = d.as_secs();
    let m = total / 60;
    let s = total % 60;
    format!("{m}:{s:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(std::time::Duration::from_secs(0)), "0:00");
        assert_eq!(format_duration(std::time::Duration::from_secs(59)), "0:59");
        assert_eq!(format_duration(std::time::Duration::from_secs(60)), "1:00");
        assert_eq!(format_duration(std::time::Duration::from_secs(215)), "3:35");
    }

    #[test]
    fn test_progress_bar_counts() {
        let bar_width = 20;
        let ratio = 0.5;
        let filled_count = ((ratio * bar_width as f64).round() as usize).min(bar_width);
        let empty_count = bar_width.saturating_sub(filled_count);
        assert_eq!(filled_count, 10);
        assert_eq!(empty_count, 10);
        assert_eq!(filled_count + empty_count, bar_width);
    }
}
