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

    let inner = block.inner(area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let track_ids = app.library.track_ids();
    let total = track_ids.len();
    if total == 0 {
        let list = List::new(Vec::<ListItem>::new())
            .block(block)
            .highlight_style(app.theme.selection_style(is_focused))
            .highlight_symbol("▶ ");
        frame.render_stateful_widget(list, area, &mut app.library_state);
        return;
    }

    let height = inner.height as usize;
    let cursor = app.library_cursor.min(total - 1);
    let mut offset = app.library_state.offset();

    // Adjust scroll offset to keep cursor in view
    if cursor < offset {
        offset = cursor;
    } else if cursor >= offset + height {
        offset = cursor - height + 1;
    }
    let max_offset = total.saturating_sub(height);
    if offset > max_offset {
        offset = max_offset;
    }

    let end = (offset + height).min(total);
    let visible_ids = &track_ids[offset..end];

    let items: Vec<ListItem> = visible_ids
        .iter()
        .filter_map(|id| app.library.get(*id))
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

    // In the windowed sub-slice, the selected item is at cursor - offset
    let mut local_state = ratatui::widgets::ListState::default();
    local_state.select(Some(cursor.saturating_sub(offset)));
    *app.library_state.offset_mut() = offset;
    app.library_state.select(Some(cursor));

    let highlight_style = app.theme.selection_style(is_focused);

    let list = List::new(items)
        .block(block)
        .highlight_style(highlight_style)
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut local_state);
}
