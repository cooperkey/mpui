use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use mpui_config::types::RepeatMode;

/// Render the status bar — volume, repeat, shuffle indicators.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let vol = if app.muted {
        "vol: muted".to_string()
    } else {
        format!("vol: {}%", app.volume.get())
    };

    let repeat = match app.repeat_mode {
        RepeatMode::Off => "rep: off",
        RepeatMode::Track => "rep: track",
        RepeatMode::Queue => "rep: queue",
    };

    let shuffle = if app.shuffle { "shuf: on" } else { "shuf: off" };

    let _mode_indicator = match app.input_mode {
        crate::app::InputMode::Normal => "",
        crate::app::InputMode::Command => ":",
        crate::app::InputMode::Search => "/",
    };

    let input_display = match app.input_mode {
        crate::app::InputMode::Command => format!(":{}", app.command_buffer),
        crate::app::InputMode::Search => format!("/{}", app.search_query),
        _ => String::new(),
    };

    let line = Line::from(vec![
        Span::styled(
            format!(" {vol}  {repeat}  {shuffle} "),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(input_display, Style::default().fg(Color::White)),
    ]);

    let paragraph = Paragraph::new(line);
    frame.render_widget(paragraph, area);
}
