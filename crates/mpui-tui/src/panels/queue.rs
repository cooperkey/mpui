use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::Frame;

use crate::app::App;

/// Render the queue panel — ordered list of queued tracks with current indicator.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" queue ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

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

            let prefix = if is_current { "|" } else { " " };
            let line = Line::from(vec![
                Span::styled(prefix, style),
                Span::raw(" "),
                Span::styled(title, style),
            ]);

            ListItem::new(line)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.queue_cursor));

    let list = List::new(items).block(block).highlight_symbol("> ");

    frame.render_stateful_widget(list, area, &mut state);
}
