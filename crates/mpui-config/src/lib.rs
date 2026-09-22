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
    pub sensitivity: f64,
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
            sensitivity: 1.0,
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
            fg: "default".into(),
            border: "default".into(),
            border_focused: "cyan".into(),
            title: "default".into(),
            selection_bg: "default".into(),
            selection_fg: "default".into(),
            playing_fg: "cyan".into(),
            progress_filled: "default".into(),
            progress_empty: "darkgray".into(),
            muted_fg: "darkgray".into(),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TermuxConfig {
    pub notifications: bool,
    pub background_playback: bool,
}

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
        ("?", "toggle_help"),
        ("f1", "toggle_help"),
    ];
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}


/// Expands a leading tilde `~` to the user's home directory.
pub fn expand_tilde(path: &Path) -> PathBuf {
    if let Ok(stripped) = path.strip_prefix("~") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }
    path.to_path_buf()
}

/// Returns the XDG config directory for mpui.
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().map(|h| h.join(".config")).unwrap_or_else(|| PathBuf::from("~/.config")))
        .join("mpui")
}

/// Returns the XDG cache directory for mpui.
pub fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| dirs::home_dir().map(|h| h.join(".cache")).unwrap_or_else(|| PathBuf::from("~/.cache")))
        .join("mpui")
}

/// Returns the XDG data directory for mpui.
pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| dirs::home_dir().map(|h| h.join(".local/share")).unwrap_or_else(|| PathBuf::from("~/.local/share")))
        .join("mpui")
}

/// Returns the resolved path to the config file.
pub fn get_config_path(path: Option<&Path>) -> PathBuf {
    path.map(expand_tilde)
        .unwrap_or_else(|| config_dir().join("config.toml"))
}

/// Load config from disk, merging user overrides onto compiled defaults.
pub fn load(path: Option<&Path>) -> Result<Config> {
    let config_path = get_config_path(path);

    if !config_path.exists() {
        tracing::info!("no config file found at {}, using defaults", config_path.display());
        return Ok(Config::default());
    }

    let contents = std::fs::read_to_string(&config_path)?;
    let mut config: Config = toml::from_str(&contents)?;
    
    for dir in &mut config.library.directories {
        *dir = expand_tilde(dir);
    }
    config.library.cache_path = expand_tilde(&config.library.cache_path);
    config.playlist.save_path = expand_tilde(&config.playlist.save_path);
    if let Some(ref fallback) = config.album_art.fallback_path {
        config.album_art.fallback_path = Some(expand_tilde(fallback));
    }

    tracing::info!("loaded config from {}", config_path.display());
    Ok(config)
}

/// Watch the configuration file for changes on disk and invoke `callback` on reload.
pub fn watch_config<F>(path: Option<&Path>, mut callback: F) -> Result<notify::RecommendedWatcher>
where
    F: FnMut(Config) + Send + 'static,
{
    use notify::{Event, RecursiveMode, Watcher};

    let target_file = get_config_path(path);
    let watch_dir = target_file
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| target_file.clone());

    let target_file_cloned = target_file.clone();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
        if let Ok(event) = res {
            let matches = event.paths.iter().any(|p| {
                p == &target_file_cloned
                    || p.file_name() == target_file_cloned.file_name()
            });
            if matches {
                std::thread::sleep(std::time::Duration::from_millis(60));
                if let Ok(new_cfg) = load(Some(&target_file_cloned)) {
                    tracing::info!("reloaded config from {}", target_file_cloned.display());
                    callback(new_cfg);
                }
            }
        }
    })?;

    if watch_dir.exists() {
        watcher.watch(&watch_dir, RecursiveMode::NonRecursive)?;
    }

    Ok(watcher)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_tilde() {
        let home = dirs::home_dir().expect("home dir should exist");
        assert_eq!(expand_tilde(Path::new("~/music")), home.join("music"));
        assert_eq!(expand_tilde(Path::new("~")), home);
        assert_eq!(expand_tilde(Path::new("/tmp/test")), PathBuf::from("/tmp/test"));
    }
}
