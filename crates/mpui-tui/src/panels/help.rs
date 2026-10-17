use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::App;

/// Render an interactive multi-column keyboard shortcut manual with scroll support.
pub fn render(frame: &mut Frame, _area: Rect, app: &App) {
    if !app.show_help {
        return;
    }

    let size = frame.area();
    let area = centered_rect(82, 86, size);

    frame.render_widget(Clear, area);

    let scroll_hint = if app.help_scroll > 0 {
        format!(" [scroll: {}] ", app.help_scroll)
    } else {
        " [j/k: scroll] ".to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" [keyboard shortcuts manual]{scroll_hint}"))
        .title_style(app.theme.title_style(true))
        .border_style(app.theme.border_style(true));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let k_style = app.theme.playing_style();
    let desc_style = app.theme.fg_style();
    let header_style = app.theme.title_style(true).add_modifier(Modifier::UNDERLINED);
    let muted_style = app.theme.muted_style();

    let col1_sections: &[(&str, &[(&str, &str)])] = &[
        (
            "Playback",
            &[
                ("Space", "Play / Pause playback"),
                ("h / Left", "Seek backward 5 seconds"),
                ("l / Right", "Seek forward 5 seconds"),
                ("0", "Seek to start of track"),
                ("$", "Seek to end of track"),
                ("> / <", "Next / Previous track (or library)"),
                ("+ / -", "Volume up / down"),
                ("m", "Toggle audio mute"),
                ("s", "Toggle queue shuffle"),
                ("r", "Cycle repeat (off -> track -> queue)"),
            ],
        ),
        (
            "Navigation & Panes",
            &[
                ("j / Down", "Move cursor down / EQ gain -1 dB"),
                ("k / Up", "Move cursor up / EQ gain +1 dB"),
                ("g g / Home", "Jump to top of list"),
                ("G / End", "Jump to bottom of list"),
                ("c-d / PgDown", "Scroll down 10 items"),
                ("c-u / PgUp", "Scroll up 10 items"),
                ("Enter", "Play selected track / Confirm action"),
                (".", "Jump to playing track in library"),
                ("z z", "Center cursor on playing track"),
                ("Tab / [ / ]", "Cycle focus between panels"),
                ("1 - 7", "Jump direct to pane (Lib, Q, Viz...)"),
            ],
        ),
    ];

    let col2_sections: &[(&str, &[(&str, &str)])] = &[
        (
            "Queue & Search",
            &[
                ("a", "Add selected track to queue"),
                ("d d", "Remove selected track from queue"),
                ("/", "Fuzzy search (t: a: b: g: prefixes)"),
                ("Search: i / Esc", "Insert mode / Normal mode"),
                ("Search: a / A", "Add selected / all results to queue"),
                ("Search: Esc", "Exit search window (normal mode)"),
                ("n / N", "Next / previous search result"),
            ],
        ),
        (
            "Command Mode (:)",
            &[
                ("Tab", "Auto-complete command / arguments"),
                (":vol <0-100>", "Set volume level"),
                (":seek <pos|$>", "Seek mm:ss, $, or end"),
                (":sort <field>", "Sort (artist, album, title...)"),
                (":toggle <panel>", "Toggle (viz, eq, art, meta, lrc)"),
                (":layout <mode>", "minimal / full / default"),
                (":eq <preset>", "flat, rock, bass_boost..."),
                (":w / :e <path>", "Save / load playlist or track"),
                (":rescan / :quit", "Rescan library / Exit player"),
            ],
        ),
        (
            "Panels & General",
            &[
                ("c / i", "Toggle Album Art / Metadata"),
                ("v / e / y", "Toggle Visualizer / EQ / Lyrics"),
                ("? / F1", "Toggle this Keybinds Manual"),
                ("q / Esc", "Close manual / Quit"),
            ],
        ),
    ];

    if inner.width >= 70 {
        // Multi-column rendering
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(inner);

        let lines1 = build_section_lines(col1_sections, header_style, k_style, desc_style, 16);
        let mut lines2 = build_section_lines(col2_sections, header_style, k_style, desc_style, 18);
        lines2.push(Line::raw(""));
        lines2.push(Line::from(Span::styled("Press Esc, q, or ? to close (j/k: scroll)", muted_style)));

        let p1 = Paragraph::new(lines1).scroll((app.help_scroll, 0));
        let p2 = Paragraph::new(lines2).scroll((app.help_scroll, 0));

        frame.render_widget(p1, cols[0]);
        frame.render_widget(p2, cols[1]);
    } else {
        // Single column fallback for narrow terminals
        let mut all_sections = Vec::new();
        all_sections.extend_from_slice(col1_sections);
        all_sections.extend_from_slice(col2_sections);
        let mut lines = build_section_lines(&all_sections, header_style, k_style, desc_style, 16);
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled("Press Esc, q, or ? to close (j/k: scroll)", muted_style)));

        let p = Paragraph::new(lines).scroll((app.help_scroll, 0));
        frame.render_widget(p, inner);
    }
}

fn build_section_lines(
    sections: &[(&str, &[(&str, &str)])],
    header_style: Style,
    k_style: Style,
    desc_style: Style,
    key_pad: usize,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for (i, (header, binds)) in sections.iter().enumerate() {
        if i > 0 {
            lines.push(Line::raw(""));
        }
        lines.push(Line::from(Span::styled(format!("── {header} ──"), header_style)));
        for &(keys, desc) in *binds {
            lines.push(Line::from(vec![
                Span::raw(" "),
                Span::styled(format!("{:<width$}", keys, width = key_pad), k_style),
                Span::styled(desc.to_string(), desc_style),
            ]));
        }
    }
    lines
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
