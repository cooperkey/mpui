use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use image::imageops::FilterType;

use crate::app::{App, FocusedPanel};

pub fn render_image_halfblock(img_bytes: &[u8], width: u16, height: u16) -> Vec<Line<'static>> {
    let Ok(img) = image::load_from_memory(img_bytes) else {
        return vec![];
    };

    let target_width = width.saturating_sub(2) as u32; // account for borders
    let target_height = height.saturating_sub(2) as u32 * 2;
    
    if target_width == 0 || target_height == 0 {
        return vec![];
    }

    let resized = img.resize_exact(target_width, target_height, FilterType::Triangle).into_rgba8();

    let mut lines = Vec::new();
    for y in (0..target_height).step_by(2) {
        let mut spans = Vec::new();
        for x in 0..target_width {
            let top = resized.get_pixel(x, y);
            let bottom = if y + 1 < target_height {
                resized.get_pixel(x, y + 1)
            } else {
                top
            };

            let fg = Color::Rgb(top[0], top[1], top[2]);
            let bg = Color::Rgb(bottom[0], bottom[1], bottom[2]);
            
            spans.push(Span::styled("▀", Style::default().fg(fg).bg(bg)));
        }
        lines.push(Line::from(spans));
    }

    lines
}

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let active = app.focused_panel == FocusedPanel::AlbumArt;
    let border_color = if active {
        app.theme.border_focused
    } else {
        app.theme.border
    };

    let title = if active {
        " [album art] "
    } else {
        " album art "
    };

    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(app.theme.title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);

    if let Some((_, ref img_bytes)) = app.current_art {
        // Compute width to maintain square aspect ratio roughly if we want?
        // Since characters are approx 1:2 aspect ratio, half block makes them 1:1 "pixels".
        // Let's just fit it to the area.
        let lines = render_image_halfblock(img_bytes, area.width, area.height);
        
        let widget = Paragraph::new(lines)
            .block(block)
            .alignment(Alignment::Center);
        frame.render_widget(widget, area);
    } else {
        let placeholder = "no art";
        let height = inner.height;
        let pad = height.saturating_sub(1) / 2;
        let padding = "\n".repeat(pad as usize);
        let text = format!("{}{}", padding, placeholder);

        let widget = Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Center);
        frame.render_widget(widget, area);
    }
}
