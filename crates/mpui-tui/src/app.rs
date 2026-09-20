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
}

/// Top-level application state shared across the TUI.
pub struct App {
    pub config: Config,
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
}

impl App {
    pub fn new(
        config: Config,
        playback: Arc<Mutex<PlaybackState>>,
        command_tx: Sender<PlayerCommand>,
        spectrum: Arc<Mutex<Vec<f32>>>,
    ) -> Self {
        Self {
            config,
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
        }
    }

    pub fn scan_library(&mut self) {
        self.library.scan(
            &self.config.library.directories,
            self.config.library.follow_symlinks,
            self.config.library.scan_hidden,
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
        let state = self.playback.lock();
        if state.paused {
            let _ = self.command_tx.send(PlayerCommand::Resume);
        } else if state.playing {
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
}
