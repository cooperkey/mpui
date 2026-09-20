use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, FocusedPanel};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let active = app.focused_panel == FocusedPanel::AlbumArt;
    let border_style = if active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let block = Block::default()
        .title("Album Art")
        .borders(Borders::ALL)
        .border_style(border_style);

    // Render ASCII placeholder
    // ▀ ▄ █
    let placeholder = "no art";
    
    let inner = block.inner(area);
    let height = inner.height;
    
    let pad = height.saturating_sub(1) / 2;
    let padding = "\n".repeat(pad as usize);
    let text = format!("{}{}", padding, placeholder);

    let widget = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(widget, area);
}
