use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::App;

/// Render an interactive keyboard shortcut manual overlay.
pub fn render(frame: &mut Frame, _area: Rect, app: &App) {
    if !app.show_help {
        return;
    }

    let size = frame.area();
    let area = centered_rect(76, 82, size);

    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" [keyboard shortcuts manual] ")
        .title_style(Style::default().fg(app.theme.title).add_modifier(Modifier::BOLD))
        .border_style(Style::default().fg(app.theme.border_focused));

    let k_style = Style::default().fg(app.theme.playing_fg).add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(app.theme.fg);
    let header_style = Style::default().fg(app.theme.title).add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
    let muted_style = Style::default().fg(app.theme.muted_fg);

    let sections: &[(&str, &[(&str, &str)])] = &[
        (
            "Playback",
            &[
                ("Space", "Play / Pause playback"),
                ("h / Left", "Seek backward 5 seconds"),
                ("l / Right", "Seek forward 5 seconds"),
                ("0", "Seek to start of track"),
                ("$", "Seek to end of track"),
                ("> / <", "Next / Previous track in queue"),
                ("+ / -", "Volume up / down (step configurable)"),
                ("m", "Toggle audio mute"),
                ("s", "Toggle queue shuffle"),
                ("r", "Cycle repeat mode (off -> track -> queue)"),
            ],
        ),
        (
            "Navigation & Selection",
            &[
                ("j / Down", "Move cursor down / EQ band -1 dB"),
                ("k / Up", "Move cursor up / EQ band +1 dB"),
                ("g g / Home", "Jump to top of list"),
                ("G / End", "Jump to bottom of list"),
                ("c-d / PgDown", "Scroll down 10 items"),
                ("c-u / PgUp", "Scroll up 10 items"),
                ("Enter", "Play selected track / Confirm action"),
                ("Tab / S-Tab", "Cycle focus between panels"),
            ],
        ),
        (
            "Queue & Search",
            &[
                ("a", "Add selected track to playback queue"),
                ("d d", "Remove selected track from queue"),
                ("/", "Fuzzy search (supports t: a: b: prefixes)"),
                (":", "Command mode (:q, :vol, :seek, :eq, :sort)"),
            ],
        ),
        (
            "Panel Toggles",
            &[
                ("c", "Toggle Album Art display"),
                ("v", "Toggle Spectrum Visualizer"),
                ("e", "Toggle 10-Band Equalizer"),
                ("y", "Toggle Synchronized Lyrics"),
            ],
        ),
        (
            "General",
            &[
                ("? / F1", "Toggle this Keybinds Manual"),
                ("q", "Quit mpui (or close modal)"),
            ],
        ),
    ];

    let mut lines = Vec::new();
    for (i, (header, binds)) in sections.iter().enumerate() {
        if i > 0 {
            lines.push(Line::raw(""));
        }
        lines.push(Line::from(Span::styled(format!("── {header} ──"), header_style)));
        for &(keys, desc) in *binds {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(format!("{:<18}", keys), k_style),
                Span::styled(desc, desc_style),
            ]));
        }
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled("  Press Esc, q, or ? to close this manual", muted_style)));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_centered_rect() {
        let parent = Rect::new(0, 0, 100, 50);
        let popup = centered_rect(60, 50, parent);
        assert_eq!(popup.width, 60);
        assert_eq!(popup.height, 25);
        assert_eq!(popup.x, 20);
        assert_eq!(popup.y, 13);
    }
}
