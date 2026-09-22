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
    Metadata,
    Lyrics,
}

pub struct ArtRequest {
    pub track_id: mpui_core::track::TrackId,
    pub path: std::path::PathBuf,
    pub width: u16,
    pub height: u16,
}

pub struct ArtResponse {
    pub track_id: mpui_core::track::TrackId,
    pub width: u16,
    pub height: u16,
    pub lines: Option<Arc<Vec<ratatui::text::Line<'static>>>>,
}

pub type ArtCache =
    std::collections::HashMap<(mpui_core::track::TrackId, u16, u16), Option<Arc<Vec<ratatui::text::Line<'static>>>>>;

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
    pub cached_art: Option<(mpui_core::track::TrackId, u16, u16, Vec<ratatui::text::Line<'static>>)>,
    pub library_state: ratatui::widgets::ListState,
    pub queue_state: ratatui::widgets::ListState,

    pub show_visualizer: bool,
    pub show_equalizer: bool,
    pub show_album_art: bool,
    pub show_metadata: bool,
    pub metadata_scroll: u16,
    pub show_queue: bool,
    pub show_lyrics: bool,
    pub current_lyrics: Option<Vec<(std::time::Duration, String)>>,
    pub visualizer_bars: Vec<f32>,
    pub show_help: bool,

    pub art_cache: ArtCache,
    pub art_cache_order: std::collections::VecDeque<(mpui_core::track::TrackId, u16, u16)>,
    pub art_request_tx: Sender<ArtRequest>,
    pub art_response_rx: crossbeam_channel::Receiver<ArtResponse>,
    pub pending_art_request: Option<(mpui_core::track::TrackId, u16, u16)>,
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
        let show_meta = config.metadata.enabled;

        let (art_tx, art_rx) = crossbeam_channel::unbounded::<ArtRequest>();
        let (resp_tx, resp_rx) = crossbeam_channel::unbounded::<ArtResponse>();

        std::thread::Builder::new()
            .name("mpui-art-worker".into())
            .spawn(move || {
                while let Ok(mut req) = art_rx.recv() {
                    // Drain superseded requests during fast scrolling
                    while let Ok(newer) = art_rx.try_recv() {
                        req = newer;
                    }

                    let lines = if let Some(bytes) = mpui_core::metadata::extract_cover_art(&req.path) {
                        let rendered = crate::panels::album_art::render_image_halfblock(&bytes, req.width, req.height);
                        if rendered.is_empty() {
                            None
                        } else {
                            Some(Arc::new(rendered))
                        }
                    } else {
                        None
                    };

                    let _ = resp_tx.send(ArtResponse {
                        track_id: req.track_id,
                        width: req.width,
                        height: req.height,
                        lines,
                    });
                }
            })
            .expect("failed to spawn art worker thread");

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
            cached_art: None,
            library_state: ratatui::widgets::ListState::default(),
            queue_state: ratatui::widgets::ListState::default(),
            show_visualizer: show_viz,
            show_equalizer: show_eq,
            show_album_art: show_art,
            show_metadata: show_meta,
            metadata_scroll: 0,
            show_queue: true,
            show_lyrics: false,
            current_lyrics: None,
            visualizer_bars: Vec::new(),
            show_help: false,
            art_cache: std::collections::HashMap::with_capacity(128),
            art_cache_order: std::collections::VecDeque::with_capacity(128),
            art_request_tx: art_tx,
            art_response_rx: resp_rx,
            pending_art_request: None,
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
        self.show_metadata = config.metadata.enabled;
        let _ = self
            .command_tx
            .send(PlayerCommand::SetReplayGainMode(config.audio.replaygain.clone()));
        self.config = config;
    }

    pub fn update_art_cache(&mut self) {
        while let Ok(resp) = self.art_response_rx.try_recv() {
            let key = (resp.track_id, resp.width, resp.height);
            if self.pending_art_request == Some(key) {
                self.pending_art_request = None;
            }
            if self.art_cache.len() >= 128 {
                if let Some(oldest) = self.art_cache_order.pop_front() {
                    self.art_cache.remove(&oldest);
                }
            }
            self.art_cache.insert(key, resp.lines);
            self.art_cache_order.push_back(key);
        }
    }

    pub fn selected_track(&self) -> Option<&mpui_core::track::Track> {
        self.selected_track_id().and_then(|id| self.library.get(id))
    }

    pub fn selected_track_id(&self) -> Option<mpui_core::track::TrackId> {
        if self.input_mode == InputMode::Search {
            if let Some(&(idx, _)) = self.search_results.get(self.search_cursor) {
                if let Some(&id) = self.library.track_ids().get(idx) {
                    return Some(id);
                }
            }
        }

        if self.focused_panel == FocusedPanel::Queue {
            if let Some(&id) = self.queue.items().get(self.queue_cursor) {
                return Some(id);
            }
        }

        if self.focused_panel == FocusedPanel::Library {
            if let Some(&id) = self.library.track_ids().get(self.library_cursor) {
                return Some(id);
            }
        }

        // Fallbacks when focus is elsewhere
        if let Some(&id) = self.library.track_ids().get(self.library_cursor) {
            return Some(id);
        }

        if let Some(id) = self.queue.current_track() {
            return Some(id);
        }

        if let Some(&id) = self.queue.items().get(self.queue_cursor) {
            return Some(id);
        }

        None
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
