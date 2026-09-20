use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
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

    let line = Line::from(vec![
        Span::styled(
            format!(" {status_char} "),
            Style::default().fg(Color::Magenta),
        ),
        Span::styled(artist, Style::default().fg(Color::DarkGray)),
        Span::raw(" - "),
        Span::styled(title, Style::default().fg(Color::White)),
        Span::raw("  "),
        Span::styled(
            format!("{position} / {duration}"),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(line).block(block);
    frame.render_widget(paragraph, area);
}

fn format_duration(d: std::time::Duration) -> String {
    let total = d.as_secs();
    let m = total / 60;
    let s = total % 60;
    format!("{m}:{s:02}")
}
