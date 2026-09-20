use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::{App, InputMode};

pub fn render(frame: &mut Frame, _area: Rect, app: &App) {
    if app.input_mode != InputMode::Search {
        return;
    }

    let size = frame.area();

    // Create a floating box centered on screen
    let area = centered_rect(60, 50, size);

    // Clear the background
    frame.render_widget(Clear, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    let search_input = Paragraph::new(format!("/{}", app.search_query))
        .block(Block::default().borders(Borders::ALL).title("Search"));
    
    frame.render_widget(search_input, chunks[0]);

    let mut list_items = Vec::new();
    let ids = app.library.track_ids();

    for (i, &(orig_index, score)) in app.search_results.iter().enumerate() {
        let text = if let Some(track) = ids.get(orig_index).and_then(|&id| app.library.get(id)) {
            format!("{} - {} ({})", track.display_artist(), track.display_title(), score)
        } else {
            format!("Unknown ({})", score)
        };

        let mut item = ListItem::new(text);
        if i == app.search_cursor {
            item = item.style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));
        }
        list_items.push(item);
    }

    let results_list = List::new(list_items)
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(results_list, chunks[1]);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
