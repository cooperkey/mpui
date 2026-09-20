use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

use crate::app::{App, FocusedPanel};

/// Render the queue panel — ordered list of queued tracks with current indicator.
pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let is_focused = app.focused_panel == FocusedPanel::Queue;
    let border_color = if is_focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let title = if is_focused {
        " [queue] "
    } else {
        " queue "
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let current_index = app.queue.current_index();

    let items: Vec<ListItem> = app
        .queue
        .items()
        .iter()
        .enumerate()
        .map(|(i, &id)| {
            let is_current = current_index == Some(i);
            let title = app
                .library
                .get(id)
                .map(|t| t.display_title())
                .unwrap_or("unknown");

            let style = if is_current {
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };

            let prefix = if is_current { "▶ " } else { "  " };
            let line = Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(title, style),
            ]);

            ListItem::new(line)
        })
        .collect();

    app.queue_state.select(Some(app.queue_cursor));

    let highlight_style = if is_focused {
        Style::default()
            .bg(Color::Rgb(35, 45, 75))
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .bg(Color::Rgb(25, 25, 35))
            .fg(Color::Gray)
    };

    let list = List::new(items)
        .block(block)
        .highlight_style(highlight_style)
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, &mut app.queue_state);
}
