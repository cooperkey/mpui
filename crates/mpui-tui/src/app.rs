use std::sync::Arc;
use std::time::Instant;

use crossbeam_channel::Sender;
use parking_lot::Mutex;

use mpui_audio::player::{PlaybackState, PlayerCommand};
use mpui_core::library::Library;
use mpui_core::queue::Queue;
use mpui_config::Config;
use mpui_config::types::{RepeatMode, Volume};
use mpui_search::SearchEngine;

/// Active input mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Command,
    Search,
}

/// Which panel currently has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPanel {
    Library,
    Queue,
    Visualizer,
    Equalizer,
    AlbumArt,
    Lyrics,
}

/// Top-level application state shared across the TUI.
pub struct App {
    pub config: Config,
    pub theme: crate::theme::Theme,
    pub library: Library,
    pub queue: Queue,
    pub playback: Arc<Mutex<PlaybackState>>,
    pub command_tx: Sender<PlayerCommand>,
    pub spectrum: Arc<Mutex<Vec<f32>>>,

    pub input_mode: InputMode,
    pub focused_panel: FocusedPanel,
    pub command_buffer: String,
    pub search_query: String,
    pub volume: Volume,
    pub repeat_mode: RepeatMode,
    pub shuffle: bool,
    pub muted: bool,

    /// Cursor position within the library list.
    pub library_cursor: usize,
    /// Cursor position within the queue.
    pub queue_cursor: usize,

    pub should_quit: bool,

    pub pending_key: Option<(String, Instant)>,
    pub search_results: Vec<(usize, u32)>,
    pub search_cursor: usize,
    pub eq_gains: [f32; 10],
    pub eq_cursor: usize,
    pub search_engine: SearchEngine,
    pub current_art: Option<(mpui_core::track::TrackId, Vec<u8>)>,
    pub library_state: ratatui::widgets::ListState,
    pub queue_state: ratatui::widgets::ListState,

    pub show_visualizer: bool,
    pub show_equalizer: bool,
    pub show_album_art: bool,
    pub show_queue: bool,
    pub show_lyrics: bool,
    pub current_lyrics: Option<Vec<(std::time::Duration, String)>>,
    pub visualizer_bars: Vec<f32>,
    pub show_help: bool,
}

impl App {
    pub fn new(
        config: Config,
        playback: Arc<Mutex<PlaybackState>>,
        command_tx: Sender<PlayerCommand>,
        spectrum: Arc<Mutex<Vec<f32>>>,
    ) -> Self {
        let theme = crate::theme::Theme::from_config(&config.theme);
        let show_viz = config.visualizer.enabled;
        let show_eq = config.equalizer.enabled;
        let show_art = config.album_art.enabled;

        Self {
            config,
            theme,
            library: Library::new(),
            queue: Queue::new(),
            playback,
            command_tx,
            spectrum,
            input_mode: InputMode::Normal,
            focused_panel: FocusedPanel::Library,
            command_buffer: String::new(),
            search_query: String::new(),
            volume: Volume::default(),
            repeat_mode: RepeatMode::Off,
            shuffle: false,
            muted: false,
            library_cursor: 0,
            queue_cursor: 0,
            should_quit: false,
            pending_key: None,
            search_results: Vec::new(),
            search_cursor: 0,
            eq_gains: [0.0; 10],
            eq_cursor: 0,
            search_engine: SearchEngine::new(),
            current_art: None,
            library_state: ratatui::widgets::ListState::default(),
            queue_state: ratatui::widgets::ListState::default(),
            show_visualizer: show_viz,
            show_equalizer: show_eq,
            show_album_art: show_art,
            show_queue: true,
            show_lyrics: false,
            current_lyrics: None,
            visualizer_bars: Vec::new(),
            show_help: false,
        }
    }

    pub fn scan_library(&mut self) {
        self.library.scan_with_cache(
            &self.config.library.directories,
            self.config.library.follow_symlinks,
            self.config.library.scan_hidden,
            Some(&self.config.library.cache_path),
        );
    }

    pub fn volume_up(&mut self) {
        self.volume = self.volume.add(self.config.audio.volume_step);
        let _ = self
            .command_tx
            .send(PlayerCommand::SetVolume(self.volume.as_f32()));
    }

    pub fn volume_down(&mut self) {
        self.volume = self.volume.sub(self.config.audio.volume_step);
        let _ = self
            .command_tx
            .send(PlayerCommand::SetVolume(self.volume.as_f32()));
    }

    pub fn toggle_pause(&mut self) {
        let mut state = self.playback.lock();
        if state.paused {
            state.paused = false;
            let _ = self.command_tx.send(PlayerCommand::Resume);
        } else if state.playing {
            state.paused = true;
            let _ = self.command_tx.send(PlayerCommand::Pause);
        }
    }

    pub fn toggle_shuffle(&mut self) {
        self.shuffle = !self.shuffle;
        if self.shuffle {
            self.queue.shuffle();
        }
    }

    pub fn cycle_repeat(&mut self) {
        self.repeat_mode = self.repeat_mode.cycle();
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        let vol = if self.muted { 0.0 } else { self.volume.as_f32() };
        let _ = self.command_tx.send(PlayerCommand::SetVolume(vol));
    }

    pub fn reload_config(&mut self, config: Config) {
        self.theme = crate::theme::Theme::from_config(&config.theme);
        self.show_visualizer = config.visualizer.enabled;
        self.show_equalizer = config.equalizer.enabled;
        self.show_album_art = config.album_art.enabled;
        let _ = self
            .command_tx
            .send(PlayerCommand::SetReplayGainMode(config.audio.replaygain.clone()));
        self.config = config;
    }

    pub fn play_track_at_index(&mut self, id: mpui_core::track::TrackId) {
        if let Some(track) = self.library.get(id).cloned() {
            if self.config.termux.background_playback {
                mpui_core::termux::acquire_wake_lock();
            }
            if self.config.termux.notifications {
                mpui_core::termux::send_notification(track.display_title(), track.display_artist());
            }
            let _ = self.command_tx.send(PlayerCommand::Play(track.path.clone()));
            self.current_art = mpui_core::metadata::extract_cover_art(&track.path).map(|art| (id, art));
            self.current_lyrics = mpui_core::metadata::extract_lyrics(&track.path);
        }
    }
}
