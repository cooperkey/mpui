pub mod types;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Complete application configuration, assembled from defaults + user overrides.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: GeneralConfig,
    pub library: LibraryConfig,
    pub audio: AudioConfig,
    pub visualizer: VisualizerConfig,
    pub equalizer: EqualizerConfig,
    pub album_art: AlbumArtConfig,
    pub layout: LayoutConfig,
    pub theme: ThemeConfig,
    pub keybindings: HashMap<String, String>,
    pub playlist: PlaylistConfig,

    #[cfg(target_os = "android")]
    pub termux: TermuxConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            library: LibraryConfig::default(),
            audio: AudioConfig::default(),
            visualizer: VisualizerConfig::default(),
            equalizer: EqualizerConfig::default(),
            album_art: AlbumArtConfig::default(),
            layout: LayoutConfig::default(),
            theme: ThemeConfig::default(),
            keybindings: default_keybindings(),
            playlist: PlaylistConfig::default(),

            #[cfg(target_os = "android")]
            termux: TermuxConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub auto_reload_config: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            auto_reload_config: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LibraryConfig {
    pub directories: Vec<PathBuf>,
    pub scan_hidden: bool,
    pub follow_symlinks: bool,
    pub cache_path: PathBuf,
}

impl Default for LibraryConfig {
    fn default() -> Self {
        Self {
            directories: vec![dirs::audio_dir().unwrap_or_else(|| PathBuf::from("~/music"))],
            scan_hidden: false,
            follow_symlinks: true,
            cache_path: cache_dir().join("library.db"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioConfig {
    pub backend: String,
    pub buffer_size: u32,
    pub crossfade_ms: u32,
    pub replaygain: String,
    pub volume_step: u8,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            backend: "auto".into(),
            buffer_size: 4096,
            crossfade_ms: 0,
            replaygain: "track".into(),
            volume_step: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VisualizerConfig {
    pub enabled: bool,
    pub style: String,
    pub bar_width: u16,
    pub bar_count: u16,
    pub gravity: f64,
    pub smoothing: f64,
    pub fps: u16,
    pub bar_chars: Vec<String>,
    pub color_gradient: Vec<String>,
}

impl Default for VisualizerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            style: "bars".into(),
            bar_width: 2,
            bar_count: 0,
            gravity: 1.8,
            smoothing: 0.7,
            fps: 30,
            bar_chars: vec![
                "\u{2581}".into(),
                "\u{2582}".into(),
                "\u{2583}".into(),
                "\u{2584}".into(),
                "\u{2585}".into(),
                "\u{2586}".into(),
                "\u{2587}".into(),
                "\u{2588}".into(),
            ],
            color_gradient: vec![
                "#1a1a2e".into(),
                "#16213e".into(),
                "#0f3460".into(),
                "#e94560".into(),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EqualizerConfig {
    pub enabled: bool,
    pub active_preset: String,
    pub presets: HashMap<String, EqualizerPreset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqualizerPreset {
    pub bands: [f32; 10],
}

impl Default for EqualizerConfig {
    fn default() -> Self {
        let mut presets = HashMap::new();
        presets.insert(
            "flat".into(),
            EqualizerPreset {
                bands: [0.0; 10],
            },
        );
        presets.insert(
            "bass_boost".into(),
            EqualizerPreset {
                bands: [6.0, 5.0, 4.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            },
        );
        Self {
            enabled: true,
            active_preset: "flat".into(),
            presets,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AlbumArtConfig {
    pub enabled: bool,
    pub render_mode: String,
    pub max_colors: u16,
    pub fallback_path: Option<PathBuf>,
    pub dither: bool,
}

impl Default for AlbumArtConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            render_mode: "halfblock".into(),
            max_colors: 256,
            fallback_path: None,
            dither: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutConfig {
    pub active: String,
    pub presets: HashMap<String, String>,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        let mut presets = HashMap::new();
        presets.insert(
            "default".into(),
            "horizontal [ vertical(35) [ library(60) queue(40) ] vertical(65) [ now_playing(10) album_art(45) visualizer(35) equalizer(10) ] ]".into(),
        );
        presets.insert(
            "minimal".into(),
            "horizontal [ library(50) vertical(50) [ now_playing(15) queue(85) ] ]".into(),
        );
        Self {
            active: "default".into(),
            presets,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    pub bg: String,
    pub fg: String,
    pub border: String,
    pub border_focused: String,
    pub title: String,
    pub selection_bg: String,
    pub selection_fg: String,
    pub playing_fg: String,
    pub progress_filled: String,
    pub progress_empty: String,
    pub muted_fg: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            bg: "default".into(),
            fg: "#c0c0c0".into(),
            border: "#333333".into(),
            border_focused: "#555555".into(),
            title: "#888888".into(),
            selection_bg: "#1a1a2e".into(),
            selection_fg: "#e0e0e0".into(),
            playing_fg: "#e94560".into(),
            progress_filled: "#0f3460".into(),
            progress_empty: "#1a1a1a".into(),
            muted_fg: "#555555".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PlaylistConfig {
    pub save_format: String,
    pub save_path: PathBuf,
    pub restore_on_start: bool,
}

impl Default for PlaylistConfig {
    fn default() -> Self {
        Self {
            save_format: "m3u".into(),
            save_path: data_dir().join("playlists"),
            restore_on_start: true,
        }
    }
}

#[cfg(target_os = "android")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TermuxConfig {
    pub notifications: bool,
    pub background_playback: bool,
}

#[cfg(target_os = "android")]
impl Default for TermuxConfig {
    fn default() -> Self {
        Self {
            notifications: true,
            background_playback: true,
        }
    }
}

fn default_keybindings() -> HashMap<String, String> {
    let pairs = [
        ("j", "move_down"),
        ("k", "move_up"),
        ("g g", "jump_top"),
        ("G", "jump_bottom"),
        ("h", "collapse_or_seek_back"),
        ("l", "expand_or_seek_forward"),
        ("enter", "confirm"),
        ("space", "toggle_pause"),
        ("/", "search"),
        ("n", "search_next"),
        ("N", "search_prev"),
        ("q", "quit"),
        (":", "command_mode"),
        ("tab", "focus_next"),
        ("s-tab", "focus_prev"),
        (">", "next_track"),
        ("<", "prev_track"),
        ("+", "volume_up"),
        ("-", "volume_down"),
        ("m", "mute_toggle"),
        ("s", "shuffle_toggle"),
        ("r", "repeat_cycle"),
        ("a", "queue_add"),
        ("d d", "queue_remove"),
        ("o", "open_directory"),
        ("i", "track_info"),
        ("v", "toggle_visualizer"),
        ("e", "toggle_equalizer"),
        ("c", "toggle_album_art"),
        ("z z", "center_current"),
        ("c-d", "scroll_half_down"),
        ("c-u", "scroll_half_up"),
    ];
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

/// Returns the XDG config directory for mpui.
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("mpui")
}

/// Returns the XDG cache directory for mpui.
pub fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("~/.cache"))
        .join("mpui")
}

/// Returns the XDG data directory for mpui.
pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("mpui")
}

/// Load config from disk, merging user overrides onto compiled defaults.
pub fn load(path: Option<&Path>) -> Result<Config> {
    let config_path = path
        .map(PathBuf::from)
        .unwrap_or_else(|| config_dir().join("config.toml"));

    if !config_path.exists() {
        tracing::info!("no config file found at {}, using defaults", config_path.display());
        return Ok(Config::default());
    }

    let contents = std::fs::read_to_string(&config_path)?;
    let config: Config = toml::from_str(&contents)?;
    tracing::info!("loaded config from {}", config_path.display());
    Ok(config)
}
