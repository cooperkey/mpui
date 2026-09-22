use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, FocusedPanel};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let active = app.focused_panel == FocusedPanel::Equalizer;
    let border_color = if active {
        app.theme.border_focused
    } else {
        app.theme.border
    };

    let title = app.panel_title("equalizer", "focus_equalizer", active);

    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(app.theme.title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let labels = ["32", "64", "128", "256", "512", "1k", "2k", "4k", "8k", "16k"];
    
    // Create 10 horizontal chunks
    let constraints = std::iter::repeat(Constraint::Ratio(1, 10)).take(10).collect::<Vec<_>>();
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(inner);

    let max_height = inner.height.saturating_sub(2); // Leave room for value and label

    for i in 0..10 {
        let gain = app.eq_gains[i];
        
        let mut bar_style = Style::default();
        if active && i == app.eq_cursor {
            bar_style = bar_style.fg(app.theme.playing_fg).add_modifier(Modifier::BOLD);
        } else {
            bar_style = bar_style.fg(app.theme.fg);
        }

        let chunk = chunks[i];
        
        let col_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(chunk);

        // Normalize gain for display (assume -12.0 to 12.0 db)
        let normalized = ((gain + 12.0) / 24.0).clamp(0.0, 1.0);
        let filled_lines = (normalized * max_height as f32).round() as u16;

        let empty_lines = max_height.saturating_sub(filled_lines);
        
        let mut bar_text = String::new();
        for _ in 0..empty_lines {
            bar_text.push_str(" \n");
        }
        for _ in 0..filled_lines {
            bar_text.push_str("█\n");
        }
        bar_text.pop(); // remove last newline

        let bar = Paragraph::new(bar_text).style(bar_style).alignment(Alignment::Center);
        frame.render_widget(bar, col_chunks[0]);

        let val_str = if gain > 0.0 {
            format!("+{}db", gain)
        } else {
            format!("{}db", gain)
        };
        let val = Paragraph::new(val_str).style(bar_style).alignment(Alignment::Center);
        frame.render_widget(val, col_chunks[1]);

        let label = Paragraph::new(labels[i]).style(bar_style).alignment(Alignment::Center);
        frame.render_widget(label, col_chunks[2]);
    }
}
