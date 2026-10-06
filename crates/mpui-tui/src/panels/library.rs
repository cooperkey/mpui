use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

use crate::app::{App, FocusedPanel};

/// Render the library panel — a scrollable list of tracks.
pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let is_focused = app.focused_panel == FocusedPanel::Library;
    let title = app.panel_title("library", "focus_library", is_focused);

    let block = Block::default()
        .title(title)
        .title_style(app.theme.title_style(is_focused))
        .borders(Borders::ALL)
        .border_style(app.theme.border_style(is_focused));

    let items: Vec<ListItem> = app
        .library
        .tracks()
        .map(|track| {
            let line = Line::from(vec![
                Span::styled(track.display_artist(), app.theme.muted_style()),
                Span::raw(" - "),
                Span::styled(track.display_title(), app.theme.fg_style()),
                Span::raw("  "),
                Span::styled(track.display_duration(), app.theme.muted_style()),
                Span::raw("  "),
                Span::styled(
                    format!("[{}]", track.path.display()),
                    app.theme.muted_style(),
                ),
            ]);

            ListItem::new(line)
        })
        .collect();

    app.library_state.select(Some(app.library_cursor));

    let highlight_style = app.theme.selection_style(is_focused);

    let list = List::new(items)
        .block(block)
        .highlight_style(highlight_style)
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut app.library_state);
}
