use ratatui::layout::Rect;
use ratatui::style::Style;
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

    let (input_spans, input_len) = match app.input_mode {
        crate::app::InputMode::Command => {
            let suggestion = crate::command::get_suggestion(&app.command_buffer);
            let cmd_text = format!(":{}", app.command_buffer);
            let ghost_text = suggestion.ghost;
            let hint_text = suggestion.hint;
            let total_len = cmd_text.len() + ghost_text.len() + hint_text.len();
            (
                vec![
                    Span::styled(cmd_text, Style::default().fg(app.theme.fg)),
                    Span::styled(ghost_text, Style::default().fg(app.theme.muted_fg)),
                    Span::styled(hint_text, Style::default().fg(app.theme.muted_fg)),
                ],
                total_len,
            )
        }
        crate::app::InputMode::Search => {
            let search_text = format!("/{}", app.search_query);
            let len = search_text.len();
            (
                vec![Span::styled(search_text, Style::default().fg(app.theme.fg))],
                len,
            )
        }
        _ => (vec![], 0),
    };

    let left_str = format!(" {vol}  {repeat}  {shuffle} ");
    let help_hint = "?: help ";
    let used_width = left_str.len() + input_len + help_hint.len();
    let pad_len = (area.width as usize).saturating_sub(used_width);
    let padding = " ".repeat(pad_len);

    let mut line_spans = vec![Span::styled(left_str, Style::default().fg(app.theme.muted_fg))];
    line_spans.extend(input_spans);
    line_spans.push(Span::raw(padding));
    line_spans.push(Span::styled(help_hint, Style::default().fg(app.theme.muted_fg)));

    let paragraph = Paragraph::new(Line::from(line_spans));
    frame.render_widget(paragraph, area);
}
