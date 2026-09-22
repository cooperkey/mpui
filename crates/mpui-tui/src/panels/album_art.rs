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

    let max_w = width.saturating_sub(2) as u32; // account for borders
    let max_h = (height.saturating_sub(2) as u32) * 2; // 2 halfblock pixels per character row
    
    if max_w == 0 || max_h == 0 {
        return vec![];
    }

    // Maintain 1:1 square pixel aspect ratio so album art is never squashed or stretched
    let side = max_w.min(max_h);
    let target_width = side;
    let target_height = side;

    let resized = img.resize_exact(target_width, target_height, FilterType::Triangle).into_rgba8();

    let pad_left = ((max_w - target_width) / 2) as usize;
    let rows = (target_height + 1) / 2;
    let total_rows = height.saturating_sub(2) as usize;
    let pad_top = total_rows.saturating_sub(rows as usize) / 2;

    let mut lines = Vec::new();
    for _ in 0..pad_top {
        lines.push(Line::raw(""));
    }

    for y in (0..target_height).step_by(2) {
        let mut spans = Vec::new();
        if pad_left > 0 {
            spans.push(Span::raw(" ".repeat(pad_left)));
        }
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

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
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

    if let Some((track_id, ref img_bytes)) = app.current_art {
        let is_cached = match &app.cached_art {
            Some((id, w, h, _)) => *id == track_id && *w == area.width && *h == area.height,
            None => false,
        };

        if !is_cached {
            let rendered = render_image_halfblock(img_bytes, area.width, area.height);
            app.cached_art = Some((track_id, area.width, area.height, rendered));
        }

        if let Some((_, _, _, ref lines)) = app.cached_art {
            let widget = Paragraph::new(lines.clone())
                .block(block)
                .alignment(Alignment::Center);
            frame.render_widget(widget, area);
        }
    } else {
        app.cached_art = None;
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
