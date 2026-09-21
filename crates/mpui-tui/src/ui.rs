use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::Frame;

use crate::app::{App, FocusedPanel};
use crate::panels;

/// Render the UI layout, adapting dynamically to terminal dimensions and panel visibility.
pub fn render(frame: &mut Frame, app: &mut App) {
    let size = frame.area();

    // Narrow terminal responsive reflow (e.g., Termux on mobile portrait, width < 80)
    if size.width < 80 {
        render_narrow(frame, size, app);
    } else {
        render_widescreen(frame, size, app);
    }

    // Floating overlays render atop base panels
    panels::search::render(frame, size, app);
}

fn render_narrow(frame: &mut Frame, size: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // now playing
            Constraint::Min(1),    // primary view based on focus
            Constraint::Length(1), // status
        ])
        .split(size);

    panels::now_playing::render(frame, chunks[0], app);

    match app.focused_panel {
        FocusedPanel::Queue => panels::queue::render(frame, chunks[1], app),
        FocusedPanel::Visualizer => panels::visualizer::render(frame, chunks[1], app),
        FocusedPanel::Equalizer => panels::equalizer::render(frame, chunks[1], app),
        FocusedPanel::AlbumArt => panels::album_art::render(frame, chunks[1], app),
        FocusedPanel::Lyrics => panels::lyrics::render(frame, chunks[1], app),
        FocusedPanel::Library => panels::library::render(frame, chunks[1], app),
    }

    panels::status::render(frame, chunks[2], app);
}

fn render_widescreen(frame: &mut Frame, size: Rect, app: &mut App) {
    // Top-level horizontal split: library/queue (left 38%) | player/dsp (right 62%)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(size);

    // Left column: Library + Queue
    if app.show_queue {
        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(main_chunks[0]);
        panels::library::render(frame, left_chunks[0], app);
        panels::queue::render(frame, left_chunks[1], app);
    } else {
        panels::library::render(frame, main_chunks[0], app);
    }

    // Right column: Fixed header (2) + dynamic middle panels + fixed footer (1)
    let right_outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // now playing
            Constraint::Min(1),    // dynamic content
            Constraint::Length(1), // status
        ])
        .split(main_chunks[1]);

    panels::now_playing::render(frame, right_outer[0], app);
    panels::status::render(frame, right_outer[2], app);

    // Collect visible panels on the right side
    enum RightPanel {
        Lyrics,
        AlbumArt,
        Visualizer,
        Equalizer,
    }

    let mut visible = Vec::new();
    if app.show_lyrics {
        visible.push(RightPanel::Lyrics);
    }
    if app.show_album_art {
        visible.push(RightPanel::AlbumArt);
    }
    if app.show_visualizer {
        visible.push(RightPanel::Visualizer);
    }
    if app.show_equalizer {
        visible.push(RightPanel::Equalizer);
    }

    if visible.is_empty() {
        return;
    }

    let constraints: Vec<Constraint> = visible
        .iter()
        .map(|_| Constraint::Ratio(1, visible.len() as u32))
        .collect();

    let content_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(right_outer[1]);

    for (panel, chunk) in visible.into_iter().zip(content_chunks.iter()) {
        match panel {
            RightPanel::Lyrics => panels::lyrics::render(frame, *chunk, app),
            RightPanel::AlbumArt => panels::album_art::render(frame, *chunk, app),
            RightPanel::Visualizer => panels::visualizer::render(frame, *chunk, app),
            RightPanel::Equalizer => panels::equalizer::render(frame, *chunk, app),
        }
    }
}
