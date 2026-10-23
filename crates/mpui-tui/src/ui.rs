use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::Frame;

use crate::app::{App, FocusedPanel};
use crate::panels;

/// Render the UI layout, adapting dynamically to terminal dimensions and panel visibility.
pub fn render(frame: &mut Frame, app: &mut App) {
    let size = frame.area();

    if app.theme.bg != ratatui::style::Color::Reset {
        frame.render_widget(
            ratatui::widgets::Block::default().style(ratatui::style::Style::default().bg(app.theme.bg)),
            size,
        );
    }

    // Narrow terminal responsive reflow (e.g., Termux on mobile portrait, width < 80)
    if size.width < 80 {
        render_narrow(frame, size, app);
    } else {
        render_widescreen(frame, size, app);
    }

    // Floating overlays render atop base panels
    panels::help::render(frame, size, app);
}

fn render_narrow(frame: &mut Frame, size: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // now playing
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
        FocusedPanel::Metadata => panels::metadata::render(frame, chunks[1], app),
        FocusedPanel::Lyrics => panels::lyrics::render(frame, chunks[1], app),
        FocusedPanel::Library => panels::library::render(frame, chunks[1], app),
    }

    panels::status::render(frame, chunks[2], app);
}

fn render_widescreen(frame: &mut Frame, size: Rect, app: &mut App) {
    // Outer vertical layout: Full-width header (3) + Center workspace (Min 1) + Full-width footer (1)
    let outer_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // now playing with progress line
            Constraint::Min(1),    // center content
            Constraint::Length(1), // status
        ])
        .split(size);

    panels::now_playing::render(frame, outer_chunks[0], app);
    panels::status::render(frame, outer_chunks[2], app);

    // Center workspace split horizontally:
    // Left column (55%): Library + Queue (wide landscape rectangle)
    // Right column (45%): Top square panes (Album art + Metadata) + dynamic DSP panels
    let center_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(outer_chunks[1]);

    // Left Column: Library & Queue
    if app.show_queue {
        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(center_cols[0]);
        panels::library::render(frame, left_chunks[0], app);
        panels::queue::render(frame, left_chunks[1], app);
    } else {
        panels::library::render(frame, center_cols[0], app);
    }

    // Right Column: Album art & Metadata squares + dynamic DSP panels
    let right_area = center_cols[1];

    enum DspPanel {
        Lyrics,
        Visualizer,
        Equalizer,
    }

    let mut dsp_panels = Vec::new();
    if app.show_lyrics {
        dsp_panels.push(DspPanel::Lyrics);
    }
    if app.show_visualizer {
        dsp_panels.push(DspPanel::Visualizer);
    }
    if app.show_equalizer {
        dsp_panels.push(DspPanel::Equalizer);
    }

    let has_top = app.show_album_art || app.show_metadata;

    if has_top && !dsp_panels.is_empty() {
        let dsp_count = dsp_panels.len();
        let max_top_height = match dsp_count {
            1 => 14,
            2 => 11,
            _ => 8,
        };
        let target_sq_height = if app.show_album_art && app.show_metadata {
            (right_area.width / 4).max(7)
        } else {
            (right_area.width / 2).max(7)
        };
        let square_height = target_sq_height
            .min(max_top_height)
            .min(right_area.height.saturating_sub(4));

        let right_splits = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(square_height), // compact square panes
                Constraint::Min(1),                 // generous room for visualizer & equalizer
            ])
            .split(right_area);

        render_top_squares(frame, right_splits[0], app);

        let dsp_constraints: Vec<Constraint> = dsp_panels
            .iter()
            .map(|_| Constraint::Ratio(1, dsp_panels.len() as u32))
            .collect();

        let dsp_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(dsp_constraints)
            .split(right_splits[1]);

        for (panel, chunk) in dsp_panels.into_iter().zip(dsp_chunks.iter()) {
            match panel {
                DspPanel::Lyrics => panels::lyrics::render(frame, *chunk, app),
                DspPanel::Visualizer => panels::visualizer::render(frame, *chunk, app),
                DspPanel::Equalizer => panels::equalizer::render(frame, *chunk, app),
            }
        }
    } else if has_top {
        let target_sq_height = if app.show_album_art && app.show_metadata {
            (right_area.width / 4).max(7)
        } else {
            (right_area.width / 2).max(7)
        };
        let square_height = target_sq_height.min(right_area.height);

        let right_splits = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(square_height),
                Constraint::Min(0),
            ])
            .split(right_area);

        render_top_squares(frame, right_splits[0], app);
    } else if !dsp_panels.is_empty() {
        let dsp_constraints: Vec<Constraint> = dsp_panels
            .iter()
            .map(|_| Constraint::Ratio(1, dsp_panels.len() as u32))
            .collect();

        let dsp_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(dsp_constraints)
            .split(right_area);

        for (panel, chunk) in dsp_panels.into_iter().zip(dsp_chunks.iter()) {
            match panel {
                DspPanel::Lyrics => panels::lyrics::render(frame, *chunk, app),
                DspPanel::Visualizer => panels::visualizer::render(frame, *chunk, app),
                DspPanel::Equalizer => panels::equalizer::render(frame, *chunk, app),
            }
        }
    }
}

fn render_top_squares(frame: &mut Frame, area: Rect, app: &mut App) {
    if app.show_album_art && app.show_metadata {
        let splits = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        panels::album_art::render(frame, splits[0], app);
        panels::metadata::render(frame, splits[1], app);
    } else if app.show_album_art {
        panels::album_art::render(frame, area, app);
    } else if app.show_metadata {
        panels::metadata::render(frame, area, app);
    }
}
