use mpui_config::ThemeConfig;
use ratatui::style::Color;

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
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_config(&ThemeConfig::default())
    }
}

/// Parse hex colors (#rrggbb) or standard terminal color names.
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
    match s.to_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" => Color::Gray,
        "darkgray" | "darkgrey" | "dark_gray" => Color::DarkGray,
        "white" => Color::White,
        _ => Color::Reset,
    }
}
