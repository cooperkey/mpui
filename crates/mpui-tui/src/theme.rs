use mpui_config::ThemeConfig;
use ratatui::style::{Color, Modifier, Style};

/// Pre-parsed terminal colors derived from the user's ThemeConfig.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub border: Color,
    pub border_focused: Color,
    pub title: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,
    pub playing_fg: Color,
    pub progress_filled: Color,
    pub progress_empty: Color,
    pub muted_fg: Color,
}

impl Theme {
    pub fn from_config(c: &ThemeConfig) -> Self {
        Self {
            bg: parse_color(&c.bg),
            fg: parse_color(&c.fg),
            border: parse_color(&c.border),
            border_focused: parse_color(&c.border_focused),
            title: parse_color(&c.title),
            selection_bg: parse_color(&c.selection_bg),
            selection_fg: parse_color(&c.selection_fg),
            playing_fg: parse_color(&c.playing_fg),
            progress_filled: parse_color(&c.progress_filled),
            progress_empty: parse_color(&c.progress_empty),
            muted_fg: parse_color(&c.muted_fg),
        }
    }

    /// Border style: highlights active panel with bold when inheriting default terminal color.
    pub fn border_style(&self, focused: bool) -> Style {
        if focused {
            let mut style = Style::default().fg(self.border_focused);
            if self.border_focused == Color::Reset {
                style = style.add_modifier(Modifier::BOLD);
            }
            style
        } else {
            let mut style = Style::default().fg(self.border);
            if self.border == Color::Reset && self.border_focused == Color::Reset {
                style = style.add_modifier(Modifier::DIM);
            }
            style
        }
    }

    /// Title style: bold for active panel or default terminal font.
    pub fn title_style(&self, focused: bool) -> Style {
        let mut style = Style::default().fg(self.title);
        if focused || self.title == Color::Reset {
            style = style.add_modifier(Modifier::BOLD);
        }
        style
    }

    /// Base foreground text style.
    pub fn fg_style(&self) -> Style {
        Style::default().fg(self.fg)
    }

    /// Accent style for currently playing items or status badges.
    pub fn playing_style(&self) -> Style {
        let mut style = Style::default().fg(self.playing_fg);
        if self.playing_fg == Color::Reset {
            style = style.add_modifier(Modifier::BOLD);
        }
        style
    }

    /// Muted style for secondary labels, hints, and timestamps.
    /// Uses terminal DIM attribute when set to default terminal color.
    pub fn muted_style(&self) -> Style {
        let mut style = Style::default().fg(self.muted_fg);
        if self.muted_fg == Color::Reset {
            style = style.add_modifier(Modifier::DIM);
        }
        style
    }

    /// Filled progress bar style.
    pub fn progress_filled_style(&self) -> Style {
        let mut style = Style::default().fg(self.progress_filled);
        if self.progress_filled == Color::Reset {
            style = style.add_modifier(Modifier::BOLD);
        }
        style
    }

    /// Empty / track progress bar style.
    pub fn progress_empty_style(&self) -> Style {
        let mut style = Style::default().fg(self.progress_empty);
        if self.progress_empty == Color::Reset {
            style = style.add_modifier(Modifier::DIM);
        }
        style
    }

    /// List item selection style supporting terminal reverse/dim attributes.
    pub fn selection_style(&self, focused: bool) -> Style {
        if focused {
            if self.selection_bg == Color::Reset {
                Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
            } else {
                Style::default()
                    .bg(self.selection_bg)
                    .fg(self.selection_fg)
                    .add_modifier(Modifier::BOLD)
            }
        } else if self.selection_bg == Color::Reset {
            Style::default().add_modifier(Modifier::DIM)
        } else {
            Style::default()
                .bg(self.selection_bg)
                .fg(self.muted_fg)
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_config(&ThemeConfig::default())
    }
}

/// Parse hex colors (#rrggbb), standard ANSI names, or "default"/"reset".
pub fn parse_color(s: &str) -> Color {
    let s = s.trim();
    if s.eq_ignore_ascii_case("default") || s.eq_ignore_ascii_case("reset") {
        return Color::Reset;
    }
    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() == 6 {
            if let Ok(val) = u32::from_str_radix(hex, 16) {
                let r = ((val >> 16) & 0xff) as u8;
                let g = ((val >> 8) & 0xff) as u8;
                let b = (val & 0xff) as u8;
                return Color::Rgb(r, g, b);
            }
        }
    }
    s.parse::<Color>().unwrap_or_else(|_| match s.to_lowercase().as_str() {
        "gray" | "grey" => Color::Gray,
        "darkgray" | "darkgrey" | "dark_gray" => Color::DarkGray,
        _ => Color::Reset,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_color() {
        assert_eq!(parse_color("default"), Color::Reset);
        assert_eq!(parse_color("reset"), Color::Reset);
        assert_eq!(parse_color("#ff0000"), Color::Rgb(255, 0, 0));
        assert_eq!(parse_color("cyan"), Color::Cyan);
        assert_eq!(parse_color("darkgray"), Color::DarkGray);
        assert_eq!(parse_color("unknown_color"), Color::Reset);
    }

    #[test]
    fn test_default_theme_inherits_terminal_styles() {
        let theme = Theme::default();
        assert_eq!(theme.bg, Color::Reset);
        assert_eq!(theme.fg, Color::Reset);
        assert_eq!(theme.border, Color::Reset);
        assert_eq!(theme.border_focused, Color::Reset);
        assert_eq!(theme.playing_fg, Color::Reset);
        assert_eq!(theme.muted_fg, Color::Reset);

        // Focused border has bold modifier when inheriting default terminal color
        let focused_border = theme.border_style(true);
        assert_eq!(focused_border.fg, Some(Color::Reset));
        assert!(focused_border.add_modifier.contains(Modifier::BOLD));

        // Unfocused border has dim modifier when inheriting default terminal color
        let unfocused_border = theme.border_style(false);
        assert_eq!(unfocused_border.fg, Some(Color::Reset));
        assert!(unfocused_border.add_modifier.contains(Modifier::DIM));

        // Muted style has dim modifier
        let muted = theme.muted_style();
        assert_eq!(muted.fg, Some(Color::Reset));
        assert!(muted.add_modifier.contains(Modifier::DIM));

        // Playing style has bold modifier
        let playing = theme.playing_style();
        assert_eq!(playing.fg, Some(Color::Reset));
        assert!(playing.add_modifier.contains(Modifier::BOLD));

        // Selection has reversed + bold modifier when focused
        let sel_focused = theme.selection_style(true);
        assert!(sel_focused.add_modifier.contains(Modifier::REVERSED));
        assert!(sel_focused.add_modifier.contains(Modifier::BOLD));
    }
}
