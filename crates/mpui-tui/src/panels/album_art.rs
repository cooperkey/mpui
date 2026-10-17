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
    let rows = target_height.div_ceil(2);
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

use ratatui::buffer::Buffer;
use ratatui::widgets::Widget;

pub struct ArtLinesWidget<'a>(pub &'a [Line<'static>]);

impl<'a> Widget for ArtLinesWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for (y, line) in self.0.iter().enumerate() {
            if y >= area.height as usize {
                break;
            }
            buf.set_line(area.left(), area.top() + y as u16, line, area.width);
        }
    }
}

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    app.update_art_cache();

    let active = app.focused_panel == FocusedPanel::AlbumArt;
    let title = app.panel_title("album art", "focus_album_art", active);

    let block = Block::default()
        .title(title)
        .title_style(app.theme.title_style(active))
        .borders(Borders::ALL)
        .border_style(app.theme.border_style(active));

    let inner = block.inner(area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let selected = app.selected_track().cloned();
    if let Some(track) = selected {
        let key = (track.id, area.width, area.height);
        let cached_exact = app.art_cache.get(&key).cloned();
        match cached_exact {
            Some(Some(ref lines)) => {
                frame.render_widget(block, area);
                frame.render_widget(ArtLinesWidget(lines.as_slice()), inner);
            }
            Some(None) => {
                render_placeholder(frame, area, block, inner.height, "no art", app.theme.muted_style());
            }
            None => {
                let should_request = match app.pending_art_request {
                    Some((pending_key, sent_at)) => pending_key != key || sent_at.elapsed() > std::time::Duration::from_millis(1500),
                    None => true,
                };
                if should_request {
                    let _ = app.art_request_tx.send(crate::app::ArtRequest {
                        track_id: track.id,
                        path: track.path.clone(),
                        width: area.width,
                        height: area.height,
                    });
                    app.pending_art_request = Some((key, std::time::Instant::now()));
                }

                // Fallback to cached render of the same track at different dimensions if available
                let fallback = app.art_cache.iter().find_map(|((id, _, _), opt)| {
                    if *id == track.id {
                        opt.as_ref().cloned()
                    } else {
                        None
                    }
                });

                if let Some(ref lines) = fallback {
                    frame.render_widget(block, area);
                    frame.render_widget(ArtLinesWidget(lines.as_slice()), inner);
                } else {
                    render_placeholder(frame, area, block, inner.height, "...", app.theme.muted_style());
                }
            }
        }
    } else {
        render_placeholder(frame, area, block, inner.height, "no art", app.theme.muted_style());
    }
}

fn render_placeholder(
    frame: &mut Frame,
    area: Rect,
    block: Block,
    height: u16,
    placeholder: &str,
    style: Style,
) {
    let pad = height.saturating_sub(1) / 2;
    let padding = "\n".repeat(pad as usize);
    let text = format!("{}{}", padding, placeholder);

    let widget = Paragraph::new(text)
        .block(block)
        .style(style)
        .alignment(Alignment::Center);
    frame.render_widget(widget, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::sync::Arc;
    use parking_lot::Mutex;
    use mpui_audio::player::PlaybackState;

    #[test]
    fn test_album_art_render_empty() {
        let backend = TestBackend::new(30, 15);
        let mut terminal = Terminal::new(backend).unwrap();

        let config = mpui_config::Config::default();
        let playback = Arc::new(Mutex::new(PlaybackState::default()));
        let (tx, _) = crossbeam_channel::unbounded();
        let (_, event_rx) = crossbeam_channel::unbounded();
        let spectrum = Arc::new(Mutex::new(Vec::new()));
        let mut app = App::new(config, playback, tx, spectrum, event_rx);

        terminal
            .draw(|f| {
                let area = f.area();
                render(f, area, &mut app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(content.contains("no art"));
    }

    #[test]
    fn test_album_art_render_cached() {
        let backend = TestBackend::new(30, 15);
        let mut terminal = Terminal::new(backend).unwrap();

        let config = mpui_config::Config::default();
        let playback = Arc::new(Mutex::new(PlaybackState::default()));
        let (tx, _) = crossbeam_channel::unbounded();
        let (_, event_rx) = crossbeam_channel::unbounded();
        let spectrum = Arc::new(Mutex::new(Vec::new()));
        let mut app = App::new(config, playback, tx, spectrum, event_rx);

        let track = mpui_core::track::Track {
            id: mpui_core::track::TrackId(101),
            path: std::path::PathBuf::from("/music/song.mp3"),
            title: Some("Song".into()),
            artist: Some("Artist".into()),
            album: Some("Album".into()),
            album_artist: None,
            genre: None,
            year: None,
            track_number: None,
            disc_number: None,
            duration: None,
            sample_rate: None,
            channels: None,
            bitrate: None,
            format: "mp3".into(),
            replay_gain_track: None,
            replay_gain_album: None,
        };

        app.library.insert(track);
        app.library_cursor = 0;

        let rendered_line = Line::from(vec![Span::raw("ART_CONTENT")]);
        let key = (mpui_core::track::TrackId(101), 30, 15);
        app.art_cache.insert(key, Some(Arc::new(vec![rendered_line])));

        terminal
            .draw(|f| {
                let area = f.area();
                render(f, area, &mut app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(content.contains("ART_CONTENT"));
    }
}
