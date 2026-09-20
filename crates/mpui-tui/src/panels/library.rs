use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::Frame;

use crate::app::App;

/// Render the library panel — a scrollable list of tracks.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" library ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let items: Vec<ListItem> = app
        .library
        .tracks()
        .enumerate()
        .map(|(i, track)| {
            let style = if i == app.library_cursor {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };

            let line = Line::from(vec![
                Span::styled(track.display_artist(), Style::default().fg(Color::DarkGray)),
                Span::raw(" - "),
                Span::styled(track.display_title(), style),
                Span::raw("  "),
                Span::styled(track.display_duration(), Style::default().fg(Color::DarkGray)),
            ]);

            ListItem::new(line)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.library_cursor));

    let list = List::new(items).block(block).highlight_symbol("> ");

    frame.render_stateful_widget(list, area, &mut state);
}
