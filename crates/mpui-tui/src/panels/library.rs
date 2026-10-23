use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::{App, FocusedPanel, InputMode, SearchVimMode};

/// Render the library panel — an lf-style file tree, with embedded in-panel search.
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

    // When in Search mode, render results list and inline search bar directly in the Library panel
    if app.input_mode == InputMode::Search {
        render_embedded_search(frame, area, block, inner, app);
        return;
    }

    // Ensure tree is built if library has tracks
    if app.library_tree.is_empty() && !app.library.is_empty() {
        app.library_tree.build(&app.library, &app.config.library.directories);
    }

    let total = app.library_tree.len();
    if total == 0 {
        let empty_items = if app.library.is_empty() {
            vec![ListItem::new(Line::from(vec![Span::styled(
                "  (empty library - check library directories)",
                app.theme.muted_style(),
            )]))]
        } else {
            Vec::new()
        };
        let list = List::new(empty_items)
            .block(block)
            .highlight_style(app.theme.selection_style(is_focused));
        frame.render_stateful_widget(list, area, &mut app.library_state);
        return;
    }

    let height = inner.height as usize;
    let cursor = app.library_cursor.min(total.saturating_sub(1));
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
    let visible_items = &app.library_tree.visible_items[offset..end];

    let items: Vec<ListItem> = visible_items
        .iter()
        .map(|item| {
            let indent = "  ".repeat(item.depth);
            if item.is_dir {
                // Directories display with + (collapsed) or - (expanded) and trailing slash (no emojis)
                let fold = if item.expanded { "- " } else { "+ " };
                let dir_str = format!("{indent}{fold}{}/", item.name);
                let line = Line::from(vec![Span::styled(
                    dir_str,
                    app.theme.title_style(is_focused),
                )]);
                ListItem::new(line)
            } else {
                // Tracks display with playing indicator (▶ ) or space
                let is_current = item.track_id == app.current_playing_track;
                let play_prefix = if is_current { "▶ " } else { "  " };
                let style = if is_current {
                    app.theme.playing_style()
                } else {
                    app.theme.fg_style()
                };

                let dur_str = if let Some(id) = item.track_id {
                    app.library
                        .get(id)
                        .map(|t| format!("  [{}]", t.display_duration()))
                        .unwrap_or_default()
                } else {
                    String::new()
                };

                let line = Line::from(vec![
                    Span::raw(indent),
                    Span::styled(play_prefix, style),
                    Span::styled(&item.name, style),
                    Span::styled(dur_str, app.theme.muted_style()),
                ]);
                ListItem::new(line)
            }
        })
        .collect();

    let mut local_state = ratatui::widgets::ListState::default();
    local_state.select(Some(cursor.saturating_sub(offset)));
    *app.library_state.offset_mut() = offset;
    app.library_state.select(Some(cursor));

    let highlight_style = app.theme.selection_style(is_focused);

    let list = List::new(items)
        .block(block)
        .highlight_style(highlight_style)
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, &mut local_state);
}

fn render_embedded_search(
    frame: &mut Frame,
    area: Rect,
    block: Block,
    _inner: Rect,
    app: &mut App,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(area);

    let mode_str = match app.search_vim_mode {
        SearchVimMode::Normal => "NORMAL",
        SearchVimMode::Insert => "INSERT",
    };

    let input_display = match app.search_vim_mode {
        SearchVimMode::Insert => format!("/{}█", app.search_query),
        SearchVimMode::Normal => format!("/{}", app.search_query),
    };

    let hint = match app.search_vim_mode {
        SearchVimMode::Normal => " [i: insert] [a: add] [A: add all] [enter: play] [esc: exit] ",
        SearchVimMode::Insert => " [esc / enter: normal mode] ",
    };

    let search_bar = Paragraph::new(input_display).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" [search - {mode_str}] "))
            .title_style(app.theme.title_style(true))
            .border_style(app.theme.border_style(true)),
    );

    let ids = app.library.track_ids();
    let mut list_items = Vec::new();

    if app.search_results.is_empty() && app.search_query.is_empty() {
        list_items.push(ListItem::new(Line::from(vec![Span::styled(
            "  (type query to search library tracks...)",
            app.theme.muted_style(),
        )])));
    } else {
        for (i, &(orig_index, score)) in app.search_results.iter().enumerate() {
            let is_selected = i == app.search_cursor;
            let (text, is_playing) = if let Some(&id) = ids.get(orig_index) {
                let playing = Some(id) == app.current_playing_track;
                let t_str = if let Some(track) = app.library.get(id) {
                    format!(
                        "{} - {} [{}] ({})",
                        track.display_artist(),
                        track.display_title(),
                        track.display_duration(),
                        score
                    )
                } else {
                    format!("Unknown ({})", score)
                };
                (t_str, playing)
            } else {
                (format!("Unknown ({})", score), false)
            };

            let prefix = if is_playing { "▶ " } else { "  " };
            let style = if is_selected {
                app.theme.selection_style(true)
            } else if is_playing {
                app.theme.playing_style()
            } else {
                app.theme.fg_style()
            };

            let line = Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(text, style),
            ]);
            list_items.push(ListItem::new(line));
        }
    }

    let results_block = block.title(format!(" library - search results {hint}"));
    let results_list = List::new(list_items)
        .block(results_block)
        .highlight_style(app.theme.selection_style(true))
        .highlight_symbol("> ");

    let mut state = ratatui::widgets::ListState::default();
    if !app.search_results.is_empty() {
        state.select(Some(app.search_cursor));
    }

    frame.render_stateful_widget(results_list, chunks[0], &mut state);
    frame.render_widget(search_bar, chunks[1]);
}
