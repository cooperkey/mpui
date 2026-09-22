use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, FocusedPanel};

/// Render synchronized lyrics or static text.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focused_panel == FocusedPanel::Lyrics;
    let border_color = if is_focused {
        app.theme.border_focused
    } else {
        app.theme.border
    };

    let title = app.panel_title("lyrics", "focus_lyrics", is_focused);

    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(app.theme.title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    if let Some(ref lyrics) = app.current_lyrics {
        if lyrics.is_empty() {
            render_placeholder(frame, area, block, "empty lyrics");
            return;
        }

        let pos = app.playback.lock().position;

        // Find active line index
        let mut active_idx = 0;
        for (i, (time, _)) in lyrics.iter().enumerate() {
            if *time <= pos {
                active_idx = i;
            } else {
                break;
            }
        }

        // Center active line in visible area
        let half_h = (inner.height / 2) as usize;
        let start_idx = active_idx.saturating_sub(half_h);
        let end_idx = (start_idx + inner.height as usize).min(lyrics.len());

        let mut lines = Vec::new();
        for (i, (_, text)) in lyrics[start_idx..end_idx].iter().enumerate() {
            let actual_i = start_idx + i;
            let style = if actual_i == active_idx {
                Style::default()
                    .fg(app.theme.playing_fg)
                    .add_modifier(Modifier::BOLD)
            } else if actual_i < active_idx {
                Style::default().fg(app.theme.fg)
            } else {
                Style::default().fg(app.theme.muted_fg)
            };

            lines.push(Line::from(Span::styled(text.clone(), style)));
        }

        let p = Paragraph::new(lines)
            .block(block)
            .alignment(Alignment::Center);

        frame.render_widget(p, area);
    } else {
        render_placeholder(frame, area, block, "no lyrics available");
    }
}

fn render_placeholder(frame: &mut Frame, area: Rect, block: Block, text: &str) {
    let inner = block.inner(area);
    let pad = inner.height.saturating_sub(1) / 2;
    let padding = "\n".repeat(pad as usize);
    let content = format!("{}{}", padding, text);

    let widget = Paragraph::new(content)
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(widget, area);
}
