use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

use crate::app::{App, FocusedPanel};

/// Render the queue panel — ordered list of queued tracks with current indicator.
pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let is_focused = app.focused_panel == FocusedPanel::Queue;
    let title = app.panel_title("queue", "focus_queue", is_focused);

    let block = Block::default()
        .title(title)
        .title_style(app.theme.title_style(is_focused))
        .borders(Borders::ALL)
        .border_style(app.theme.border_style(is_focused));

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
                app.theme.playing_style()
            } else {
                app.theme.fg_style()
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

    let highlight_style = app.theme.selection_style(is_focused);

    let list = List::new(items)
        .block(block)
        .highlight_style(highlight_style)
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, &mut app.queue_state);
}
