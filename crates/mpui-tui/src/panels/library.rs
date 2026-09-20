use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

use crate::app::{App, FocusedPanel};

/// Render the library panel — a scrollable list of tracks.
pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let is_focused = app.focused_panel == FocusedPanel::Library;
    let border_color = if is_focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let title = if is_focused {
        " [library] "
    } else {
        " library "
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let items: Vec<ListItem> = app
        .library
        .tracks()
        .map(|track| {
            let line = Line::from(vec![
                Span::styled(track.display_artist(), Style::default().fg(Color::DarkGray)),
                Span::raw(" - "),
                Span::styled(track.display_title(), Style::default().fg(Color::White)),
                Span::raw("  "),
                Span::styled(track.display_duration(), Style::default().fg(Color::DarkGray)),
            ]);

            ListItem::new(line)
        })
        .collect();

    app.library_state.select(Some(app.library_cursor));

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
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut app.library_state);
}
