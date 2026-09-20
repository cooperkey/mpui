use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;

use crate::app::App;
use crate::panels;

/// Render the full UI layout.
///
/// Phase 1 uses a hardcoded default layout. The configurable layout DSL
/// parser will be implemented in phase 5.
pub fn render(frame: &mut Frame, app: &mut App) {
    let size = frame.area();

    // Top-level horizontal split: left panel (library) | right panel (rest).
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(size);

    // Left: library + queue vertical split.
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(main_chunks[0]);

    panels::library::render(frame, left_chunks[0], app);
    panels::queue::render(frame, left_chunks[1], app);

    // Right: now_playing (2 lines) + album_art (35%) + visualizer (30%) + equalizer (25%) + status (1 line)
    // We adjust percentages slightly since we have fixed lengths. We'll use Min for remainder.
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Percentage(35),
            Constraint::Percentage(30),
            Constraint::Percentage(25),
            Constraint::Length(1),
        ])
        .split(main_chunks[1]);

    panels::now_playing::render(frame, right_chunks[0], app);
    panels::album_art::render(frame, right_chunks[1], app);
    panels::visualizer::render(frame, right_chunks[2], app);
    panels::equalizer::render(frame, right_chunks[3], app);
    panels::status::render(frame, right_chunks[4], app);
    
    // Render search overlay on top of everything
    panels::search::render(frame, size, app);
}
