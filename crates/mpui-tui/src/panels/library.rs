use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

use crate::app::{App, FocusedPanel};

/// Render the library panel — a scrollable list of tracks.
pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let is_focused = app.focused_panel == FocusedPanel::Library;
    let border_color = if is_focused {
        app.theme.border_focused
    } else {
        app.theme.border
    };

    let title = if is_focused {
        " [library] "
    } else {
        " library "
    };

    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(app.theme.title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let items: Vec<ListItem> = app
        .library
        .tracks()
        .map(|track| {
            let line = Line::from(vec![
                Span::styled(track.display_artist(), Style::default().fg(app.theme.muted_fg)),
                Span::raw(" - "),
                Span::styled(track.display_title(), Style::default().fg(app.theme.fg)),
                Span::raw("  "),
                Span::styled(track.display_duration(), Style::default().fg(app.theme.muted_fg)),
                Span::raw("  "),
                Span::styled(
                    format!("[{}]", track.path.display()),
                    Style::default().fg(app.theme.muted_fg),
                ),
            ]);

            ListItem::new(line)
        })
        .collect();

    app.library_state.select(Some(app.library_cursor));

    let highlight_style = if is_focused {
        if app.theme.selection_bg == ratatui::style::Color::Reset {
            Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
        } else {
            Style::default()
                .bg(app.theme.selection_bg)
                .fg(app.theme.selection_fg)
                .add_modifier(Modifier::BOLD)
        }
    } else {
        if app.theme.selection_bg == ratatui::style::Color::Reset {
            Style::default().add_modifier(Modifier::DIM)
        } else {
            Style::default()
                .bg(app.theme.selection_bg)
                .fg(app.theme.muted_fg)
        }
    };

    let list = List::new(items)
        .block(block)
        .highlight_style(highlight_style)
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut app.library_state);
}
