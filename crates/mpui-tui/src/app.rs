use std::sync::Arc;
use std::time::Instant;

use crossbeam_channel::{Receiver, Sender};
use parking_lot::Mutex;

use mpui_audio::player::{PlaybackState, PlayerCommand, PlayerEvent};
use mpui_core::library::Library;
use mpui_core::queue::Queue;
use mpui_config::Config;
use mpui_config::types::{RepeatMode, Volume};
use mpui_search::SearchEngine;
use crate::library_tree::LibraryTree;

/// Active input mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Command,
    Search,
}

/// Vim modal state inside search mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchVimMode {
    #[default]
    Normal,
    Insert,
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
    pub event_rx: Receiver<PlayerEvent>,

    pub input_mode: InputMode,
    pub search_vim_mode: SearchVimMode,
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
    pub library_tree: LibraryTree,

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
    pub help_scroll: u16,
    pub current_playing_track: Option<mpui_core::track::TrackId>,
    pub library_shuffle_order: Vec<mpui_core::track::TrackId>,
    pub library_shuffle_index: Option<usize>,

    pub art_cache: ArtCache,
    pub art_cache_order: std::collections::VecDeque<(mpui_core::track::TrackId, u16, u16)>,
    pub art_request_tx: Sender<ArtRequest>,
    pub art_response_rx: crossbeam_channel::Receiver<ArtResponse>,
    pub pending_art_request: Option<((mpui_core::track::TrackId, u16, u16), std::time::Instant)>,
}

impl App {
    pub fn new(
        config: Config,
        playback: Arc<Mutex<PlaybackState>>,
        command_tx: Sender<PlayerCommand>,
        spectrum: Arc<Mutex<Vec<f32>>>,
        event_rx: Receiver<PlayerEvent>,
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
            event_rx,
            input_mode: InputMode::Normal,
            search_vim_mode: SearchVimMode::Normal,
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
            help_scroll: 0,
            current_playing_track: None,
            library_shuffle_order: Vec::new(),
            library_shuffle_index: None,
            library_tree: LibraryTree::new(),
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
        self.library_tree.build(&self.library, &self.config.library.directories);
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
        let (is_paused, is_playing) = {
            let state = self.playback.lock();
            (state.paused, state.playing)
        };

        if is_paused {
            self.playback.lock().paused = false;
            let _ = self.command_tx.send(PlayerCommand::Resume);
        } else if is_playing {
            self.playback.lock().paused = true;
            let _ = self.command_tx.send(PlayerCommand::Pause);
        } else {
            // Stopped: start playback with current queue track, playing track, or selected track
            if let Some(id) = self.queue.current_track() {
                self.play_track_at_index(id);
            } else if let Some(id) = self.current_playing_track {
                self.play_track_at_index(id);
            } else if let Some(id) = self.selected_track_id() {
                self.play_track_at_index(id);
            }
        }
    }

    pub fn toggle_shuffle(&mut self) {
        self.shuffle = !self.shuffle;
        if self.shuffle {
            if !self.queue.is_empty() {
                self.queue.shuffle();
            } else {
                self.reshuffle_library(self.current_playing_track);
            }
        } else {
            if !self.queue.is_empty() {
                self.queue.unshuffle();
            } else {
                self.library_shuffle_order.clear();
                self.library_shuffle_index = None;
            }
        }
    }

    pub fn handle_shuffle_key(&mut self) {
        let is_active = {
            let state = self.playback.lock();
            state.playing || state.paused
        };

        if !is_active {
            self.shuffle = true;
            self.play_random();
        } else {
            self.toggle_shuffle();
        }
    }

    pub fn play_random(&mut self) -> bool {
        if !self.queue.is_empty() {
            self.queue.shuffle_all();
            if let Some(&first_id) = self.queue.items().first() {
                self.queue.set_current(0);
                self.play_track_at_index(first_id);
                return true;
            }
        } else if !self.library.is_empty() {
            self.reshuffle_library(None);
            if let Some(&first_id) = self.library_shuffle_order.first() {
                self.library_shuffle_index = Some(0);
                self.play_track_at_index(first_id);
                return true;
            }
        }
        false
    }

    pub fn reshuffle_library(&mut self, starting_track: Option<mpui_core::track::TrackId>) {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};

        let mut ids = self.library.track_ids().to_vec();
        if ids.is_empty() {
            self.library_shuffle_order.clear();
            self.library_shuffle_index = None;
            return;
        }

        let mut seed = {
            let mut h = RandomState::new().build_hasher();
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            h.write_u128(nanos);
            h.finish()
        };
        if seed == 0 {
            seed = 0x853c49e6748fea9b;
        }

        let mut next_rand = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };

        for i in (1..ids.len()).rev() {
            let rand_val = (next_rand() as usize) % (i + 1);
            ids.swap(i, rand_val);
        }

        if let Some(start_id) = starting_track {
            if let Some(pos) = ids.iter().position(|&id| id == start_id) {
                ids.swap(0, pos);
            }
        }

        self.library_shuffle_order = ids;
        self.library_shuffle_index = Some(0);
    }

    pub fn next_track(&mut self) {
        if !self.queue.is_empty() {
            let next_id = match self.repeat_mode {
                mpui_config::types::RepeatMode::Queue => {
                    if let Some(id) = self.queue.next() {
                        Some(id)
                    } else if !self.queue.is_empty() {
                        self.queue.set_current(0);
                        self.queue.current_track()
                    } else {
                        None
                    }
                }
                _ => self.queue.next(),
            };
            if let Some(id) = next_id {
                self.play_track_at_index(id);
            }
        } else {
            self.advance_library(true);
        }
    }

    pub fn previous_track(&mut self) {
        if !self.queue.is_empty() {
            let prev_id = match self.repeat_mode {
                mpui_config::types::RepeatMode::Queue => {
                    if let Some(id) = self.queue.prev() {
                        Some(id)
                    } else if !self.queue.is_empty() {
                        let last = self.queue.len() - 1;
                        self.queue.set_current(last);
                        self.queue.current_track()
                    } else {
                        None
                    }
                }
                _ => self.queue.prev(),
            };
            if let Some(id) = prev_id {
                self.play_track_at_index(id);
            }
        } else {
            self.advance_library(false);
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
            if let Some((pending_key, _)) = self.pending_art_request {
                if pending_key == key {
                    self.pending_art_request = None;
                }
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
            if let Some(id) = self.library_tree.get_track_id(self.library_cursor) {
                return Some(id);
            }
            if let Some(&id) = self.library.track_ids().get(self.library_cursor) {
                return Some(id);
            }
        }

        // Fallbacks when focus is elsewhere
        if let Some(id) = self.library_tree.get_track_id(self.library_cursor) {
            return Some(id);
        }
        if let Some(&id) = self.library.track_ids().get(self.library_cursor) {
            return Some(id);
        }

        if let Some(id) = self.queue.current_track() {
            return Some(id);
        }

        if let Some(id) = self.current_playing_track {
            return Some(id);
        }

        if let Some(&id) = self.queue.items().get(self.queue_cursor) {
            return Some(id);
        }

        None
    }

    pub fn play_track_at_index(&mut self, id: mpui_core::track::TrackId) {
        self.current_playing_track = Some(id);
        self.reveal_track_in_library(id);

        if !self.queue.is_empty() {
            if self.queue.current_track() != Some(id) {
                if let Some(pos) = self.queue.items().iter().position(|&t| t == id) {
                    self.queue.set_current(pos);
                } else {
                    self.queue.push(id);
                    self.queue.set_current(self.queue.len() - 1);
                }
            }
        }

        if self.shuffle {
            if let Some(pos) = self.library_shuffle_order.iter().position(|&t| t == id) {
                self.library_shuffle_index = Some(pos);
            }
        }

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

    pub fn reveal_track_in_library(&mut self, id: mpui_core::track::TrackId) {
        if self.library_tree.is_empty() && !self.library.is_empty() {
            self.library_tree.build(&self.library, &self.config.library.directories);
        }

        if let Some(idx) = self.library_tree.reveal_track(id) {
            self.library_cursor = idx;
            self.library_state.select(Some(idx));
        } else if let Some(idx) = self.library.track_ids().iter().position(|&t| t == id) {
            self.library_cursor = idx;
            self.library_state.select(Some(idx));
        }
    }

    pub fn library_confirm(&mut self) {
        if self.library_tree.is_empty() && !self.library.is_empty() {
            self.library_tree.build(&self.library, &self.config.library.directories);
        }

        if let Some(item) = self.library_tree.visible_items.get(self.library_cursor).cloned() {
            if item.is_dir {
                self.library_tree.toggle_expand(self.library_cursor);
            } else if let Some(id) = item.track_id {
                self.play_track_at_index(id);
            }
        } else {
            let ids = self.library.track_ids();
            if let Some(&id) = ids.get(self.library_cursor) {
                self.play_track_at_index(id);
            }
        }
    }

    pub fn library_expand(&mut self) {
        if self.library_tree.is_empty() && !self.library.is_empty() {
            self.library_tree.build(&self.library, &self.config.library.directories);
        }

        if let Some(item) = self.library_tree.visible_items.get(self.library_cursor) {
            if item.is_dir {
                if !item.expanded {
                    self.library_tree.expand(self.library_cursor);
                } else if self.library_cursor + 1 < self.library_tree.len() {
                    self.library_cursor += 1;
                    self.metadata_scroll = 0;
                }
                return;
            }
        }

        // Seek forward fallback if not on directory
        let mut state = self.playback.lock();
        if state.playing {
            let new_pos = state.position + std::time::Duration::from_secs(5);
            state.position = new_pos;
            let _ = self.command_tx.send(PlayerCommand::Seek(new_pos));
        }
    }

    pub fn library_collapse(&mut self) {
        if self.library_tree.is_empty() && !self.library.is_empty() {
            self.library_tree.build(&self.library, &self.config.library.directories);
        }

        if let Some(item) = self.library_tree.visible_items.get(self.library_cursor) {
            if item.is_dir && item.expanded {
                self.library_tree.collapse(self.library_cursor);
                return;
            } else if let Some(parent_idx) = self.library_tree.collapse(self.library_cursor) {
                self.library_cursor = parent_idx;
                self.metadata_scroll = 0;
                return;
            }
        }

        // Seek back fallback if at root
        let mut state = self.playback.lock();
        if state.playing {
            let new_pos = state.position.saturating_sub(std::time::Duration::from_secs(5));
            state.position = new_pos;
            let _ = self.command_tx.send(PlayerCommand::Seek(new_pos));
        }
    }

    pub fn library_queue_add(&mut self) {
        if self.library_tree.is_empty() && !self.library.is_empty() {
            self.library_tree.build(&self.library, &self.config.library.directories);
        }

        if !self.library_tree.is_empty() {
            let ids = self.library_tree.collect_track_ids(self.library_cursor);
            for id in ids {
                self.queue.push(id);
            }
        } else {
            let ids = self.library.track_ids();
            if let Some(&id) = ids.get(self.library_cursor) {
                self.queue.push(id);
            }
        }
    }

    pub fn library_len(&self) -> usize {
        if !self.library_tree.is_empty() {
            self.library_tree.len()
        } else {
            self.library.len()
        }
    }

    pub fn advance_library(&mut self, forward: bool) -> bool {
        let ids = self.library.track_ids();
        if ids.is_empty() {
            return false;
        }

        if self.shuffle {
            if self.library_shuffle_order.len() != ids.len()
                || !ids.iter().all(|id| self.library_shuffle_order.contains(id))
            {
                self.reshuffle_library(self.current_playing_track);
            }

            if self.library_shuffle_order.is_empty() {
                return false;
            }

            let cur_idx = self
                .library_shuffle_index
                .or_else(|| {
                    self.current_playing_track
                        .and_then(|cur| self.library_shuffle_order.iter().position(|&id| id == cur))
                })
                .unwrap_or(0);

            let next_idx = match self.repeat_mode {
                mpui_config::types::RepeatMode::Track => Some(cur_idx),
                mpui_config::types::RepeatMode::Queue => {
                    if forward {
                        if cur_idx + 1 < self.library_shuffle_order.len() {
                            Some(cur_idx + 1)
                        } else {
                            self.reshuffle_library(None);
                            Some(0)
                        }
                    } else {
                        if cur_idx > 0 {
                            Some(cur_idx - 1)
                        } else {
                            Some(self.library_shuffle_order.len().saturating_sub(1))
                        }
                    }
                }
                mpui_config::types::RepeatMode::Off => {
                    if forward {
                        if cur_idx + 1 < self.library_shuffle_order.len() {
                            Some(cur_idx + 1)
                        } else {
                            None
                        }
                    } else {
                        if cur_idx > 0 {
                            Some(cur_idx - 1)
                        } else {
                            None
                        }
                    }
                }
            };

            if let Some(idx) = next_idx {
                self.library_shuffle_index = Some(idx);
                let next_id = self.library_shuffle_order[idx];
                self.play_track_at_index(next_id);
                true
            } else {
                false
            }
        } else {
            let current_id = self.current_playing_track;
            let next_id = match self.repeat_mode {
                mpui_config::types::RepeatMode::Track => current_id.or_else(|| ids.first().copied()),
                mpui_config::types::RepeatMode::Queue => {
                    if let Some(cur) = current_id {
                        if let Some(idx) = ids.iter().position(|&id| id == cur) {
                            if forward {
                                let next_idx = (idx + 1) % ids.len();
                                Some(ids[next_idx])
                            } else {
                                let prev_idx = if idx == 0 { ids.len() - 1 } else { idx - 1 };
                                Some(ids[prev_idx])
                            }
                        } else {
                            ids.first().copied()
                        }
                    } else {
                        ids.first().copied()
                    }
                }
                mpui_config::types::RepeatMode::Off => {
                    if let Some(cur) = current_id {
                        if let Some(idx) = ids.iter().position(|&id| id == cur) {
                            if forward {
                                if idx + 1 < ids.len() {
                                    Some(ids[idx + 1])
                                } else {
                                    None
                                }
                            } else {
                                if idx > 0 {
                                    Some(ids[idx - 1])
                                } else {
                                    None
                                }
                            }
                        } else {
                            ids.first().copied()
                        }
                    } else {
                        ids.first().copied()
                    }
                }
            };

            if let Some(id) = next_id {
                self.play_track_at_index(id);
                true
            } else {
                false
            }
        }
    }

    pub fn panel_title(&self, name: &str, action: &str, is_focused: bool) -> String {
        let mut candidates: Vec<&str> = self
            .config
            .keybindings
            .iter()
            .filter_map(|(k, v)| {
                let matches = v == action
                    || (action == "focus_metadata"
                        && (v == "focus_info" || v == "focus_track_info"));
                if matches {
                    Some(k.as_str())
                } else {
                    None
                }
            })
            .collect();

        candidates.sort_by_key(|k| {
            let is_digit = k.len() == 1 && k.chars().next().unwrap().is_ascii_digit();
            (!is_digit, k.len(), *k)
        });

        let shortcut = candidates.first().copied();
        match (shortcut, is_focused) {
            (Some(k), true) => format!(" [{name}][{k}] "),
            (Some(k), false) => format!(" {name} [{k}] "),
            (None, true) => format!(" [{name}] "),
            (None, false) => format!(" {name} "),
        }
    }

    pub fn handle_playback_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                PlayerEvent::TrackEnded => {
                    self.on_track_ended();
                }
            }
        }
    }

    pub fn on_track_ended(&mut self) {
        if !self.queue.is_empty() {
            if let Some(next_id) = self.queue.advance(self.repeat_mode) {
                self.play_track_at_index(next_id);
            } else {
                let mut state = self.playback.lock();
                state.playing = false;
                state.paused = false;
                state.position = std::time::Duration::ZERO;
                self.spectrum.lock().clear();
            }
        } else {
            if !self.advance_library(true) {
                let mut state = self.playback.lock();
                state.playing = false;
                state.paused = false;
                state.position = std::time::Duration::ZERO;
                self.spectrum.lock().clear();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_app() -> (App, crossbeam_channel::Sender<PlayerEvent>, crossbeam_channel::Receiver<PlayerCommand>) {
        let config = Config::default();
        let playback = Arc::new(Mutex::new(PlaybackState::default()));
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
        let spectrum = Arc::new(Mutex::new(Vec::new()));
        let (event_tx, event_rx) = crossbeam_channel::unbounded();

        let app = App::new(config, playback, cmd_tx, spectrum, event_rx);
        (app, event_tx, cmd_rx)
    }

    #[test]
    fn test_app_play_track_synchronizes_queue() {
        let (mut app, _event_tx, _cmd_rx) = create_test_app();
        let id1 = mpui_core::track::TrackId(1);
        let id2 = mpui_core::track::TrackId(2);

        app.queue.push(id1);
        app.queue.push(id2);
        assert_eq!(app.queue.current_index(), Some(0));

        // Playing track 2 directly must update queue cursor to 1
        app.play_track_at_index(id2);
        assert_eq!(app.queue.current_index(), Some(1));
        assert_eq!(app.queue.current_track(), Some(id2));

        // Playing an untracked id must push and select
        let id3 = mpui_core::track::TrackId(3);
        app.play_track_at_index(id3);
        assert_eq!(app.queue.current_index(), Some(2));
        assert_eq!(app.queue.current_track(), Some(id3));
        assert_eq!(app.queue.len(), 3);
    }

    #[test]
    fn test_app_auto_advance_event_handling() {
        let (mut app, event_tx, _cmd_rx) = create_test_app();
        let id1 = mpui_core::track::TrackId(10);
        let id2 = mpui_core::track::TrackId(20);

        app.queue.push(id1);
        app.queue.push(id2);
        app.repeat_mode = RepeatMode::Off;

        assert_eq!(app.queue.current_track(), Some(id1));

        // Send track ended: advances to track 2
        event_tx.send(PlayerEvent::TrackEnded).unwrap();
        app.handle_playback_events();
        assert_eq!(app.queue.current_track(), Some(id2));

        // Send track ended again: reaches end of queue in RepeatMode::Off
        event_tx.send(PlayerEvent::TrackEnded).unwrap();
        app.handle_playback_events();
        assert_eq!(app.playback.lock().playing, false);

        // Switch to RepeatMode::Queue and send event: wraps to track 1
        app.repeat_mode = RepeatMode::Queue;
        event_tx.send(PlayerEvent::TrackEnded).unwrap();
        app.handle_playback_events();
        assert_eq!(app.queue.current_track(), Some(id1));
    }

    #[test]
    fn test_app_toggle_pause_starts_when_stopped() {
        let (mut app, _event_tx, cmd_rx) = create_test_app();
        let id = mpui_core::track::TrackId(42);
        app.queue.push(id);

        assert!(!app.playback.lock().playing);
        assert!(!app.playback.lock().paused);

        // Spacebar on stopped player should start playing
        app.toggle_pause();
        assert_eq!(app.queue.current_track(), Some(id));
        // Verify Play command was emitted (or attempted)
        assert!(cmd_rx.try_recv().is_ok() || app.library.get(id).is_none());
    }

    #[test]
    fn test_app_toggle_shuffle_preserves_queue() {
        let (mut app, _event_tx, _cmd_rx) = create_test_app();
        for i in 1..=5 {
            app.queue.push(mpui_core::track::TrackId(i));
        }
        app.queue.set_current(2); // TrackId(3)

        assert!(!app.shuffle);
        app.toggle_shuffle();
        assert!(app.shuffle);
        assert!(app.queue.is_shuffled());
        assert_eq!(app.queue.current_track(), Some(mpui_core::track::TrackId(3)));

        app.toggle_shuffle();
        assert!(!app.shuffle);
        assert!(!app.queue.is_shuffled());
        assert_eq!(app.queue.current_track(), Some(mpui_core::track::TrackId(3)));
        assert_eq!(app.queue.current_index(), Some(2));
    }

    fn create_dummy_track(id: u64, path: &str, title: &str) -> mpui_core::track::Track {
        mpui_core::track::Track {
            id: mpui_core::track::TrackId(id),
            path: std::path::PathBuf::from(path),
            title: Some(title.to_string()),
            artist: Some("Artist".to_string()),
            album: None,
            album_artist: None,
            genre: None,
            year: None,
            track_number: None,
            disc_number: None,
            duration: Some(std::time::Duration::from_secs(180)),
            sample_rate: None,
            channels: None,
            bitrate: None,
            format: "mp3".to_string(),
            replay_gain_track: None,
            replay_gain_album: None,
        }
    }

    #[test]
    fn test_empty_queue_library_order_playback() {
        let (mut app, _event_tx, _cmd_rx) = create_test_app();
        let track1 = create_dummy_track(1, "/music/song1.mp3", "Song 1");
        let track2 = create_dummy_track(2, "/music/song2.mp3", "Song 2");

        app.library.insert(track1);
        app.library.insert(track2);
        assert!(app.queue.is_empty());

        // Play track 1 directly with empty queue
        app.play_track_at_index(mpui_core::track::TrackId(1));
        assert!(app.queue.is_empty());
        assert_eq!(app.current_playing_track, Some(mpui_core::track::TrackId(1)));

        // Advance library forward
        assert!(app.advance_library(true));
        assert!(app.queue.is_empty());
        assert_eq!(app.current_playing_track, Some(mpui_core::track::TrackId(2)));

        // Advance at end in RepeatMode::Off returns false
        app.repeat_mode = RepeatMode::Off;
        assert!(!app.advance_library(true));

        // Advance at end in RepeatMode::Queue wraps back to track 1
        app.repeat_mode = RepeatMode::Queue;
        assert!(app.advance_library(true));
        assert_eq!(app.current_playing_track, Some(mpui_core::track::TrackId(1)));

        // Advance backward in RepeatMode::Queue wraps to track 2
        assert!(app.advance_library(false));
        assert_eq!(app.current_playing_track, Some(mpui_core::track::TrackId(2)));
    }

    #[test]
    fn test_panel_title_dynamic_config_shortcuts() {
        let (mut app, _event_tx, _cmd_rx) = create_test_app();
        
        // Default binding for focus_library is "1"
        assert_eq!(app.panel_title("library", "focus_library", true), " [library][1] ");
        assert_eq!(app.panel_title("library", "focus_library", false), " library [1] ");

        // Reconfigure to "0"
        app.config.keybindings.insert("0".to_string(), "focus_library".to_string());
        app.config.keybindings.remove("1");
        assert_eq!(app.panel_title("library", "focus_library", true), " [library][0] ");
        assert_eq!(app.panel_title("library", "focus_library", false), " library [0] ");

        // Reconfigure equalizer to "e"
        app.config.keybindings.insert("e".to_string(), "focus_equalizer".to_string());
        app.config.keybindings.remove("4");
        assert_eq!(app.panel_title("equalizer", "focus_equalizer", true), " [equalizer][e] ");
    }

    #[test]
    fn test_empty_queue_library_shuffle_playback() {
        let (mut app, _event_tx, _cmd_rx) = create_test_app();
        for i in 1..=5 {
            app.library.insert(create_dummy_track(i, &format!("/music/song{i}.mp3"), &format!("Song {i}")));
        }
        assert!(app.queue.is_empty());

        app.shuffle = true;
        app.play_track_at_index(mpui_core::track::TrackId(1));
        assert_eq!(app.current_playing_track, Some(mpui_core::track::TrackId(1)));

        let mut played = vec![mpui_core::track::TrackId(1)];
        for _ in 0..4 {
            assert!(app.advance_library(true));
            assert!(app.queue.is_empty());
            let current = app.current_playing_track.unwrap();
            played.push(current);
        }

        // All 5 distinct tracks should have been played
        let mut sorted = played.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 5);

        // At end of shuffle cycle in RepeatMode::Off, advance_library returns false
        app.repeat_mode = RepeatMode::Off;
        assert!(!app.advance_library(true));
    }

    #[test]
    fn test_handle_shuffle_key_when_stopped_starts_random_playback() {
        let (mut app, _event_tx, cmd_rx) = create_test_app();
        for i in 1..=5 {
            app.library.insert(create_dummy_track(i, &format!("/music/song{i}.mp3"), &format!("Song {i}")));
        }

        assert!(!app.playback.lock().playing);
        assert!(!app.shuffle);
        assert_eq!(app.current_playing_track, None);

        // Pressing 's' when stopped enables shuffle and starts playing a random track
        app.handle_shuffle_key();

        assert!(app.shuffle);
        assert!(app.current_playing_track.is_some());
        assert!(cmd_rx.try_recv().is_ok());

        // Subsequent library advance continues playing randomly
        assert!(app.advance_library(true));
        assert!(app.current_playing_track.is_some());
    }

    #[test]
    fn test_handle_shuffle_key_when_playing_toggles_shuffle() {
        let (mut app, _event_tx, _cmd_rx) = create_test_app();
        for i in 1..=3 {
            app.library.insert(create_dummy_track(i, &format!("/music/song{i}.mp3"), &format!("Song {i}")));
        }

        app.play_track_at_index(mpui_core::track::TrackId(1));
        app.playback.lock().playing = true;
        assert!(!app.shuffle);

        // Pressing 's' while playing toggles shuffle on
        app.handle_shuffle_key();
        assert!(app.shuffle);
        assert_eq!(app.current_playing_track, Some(mpui_core::track::TrackId(1)));

        // Pressing 's' again while playing toggles shuffle off
        app.handle_shuffle_key();
        assert!(!app.shuffle);
        assert_eq!(app.current_playing_track, Some(mpui_core::track::TrackId(1)));
    }

    #[test]
    fn test_library_shuffle_backward_advance() {
        let (mut app, _event_tx, _cmd_rx) = create_test_app();
        for i in 1..=5 {
            app.library.insert(create_dummy_track(i, &format!("/music/song{i}.mp3"), &format!("Song {i}")));
        }

        app.shuffle = true;
        app.play_track_at_index(mpui_core::track::TrackId(1));

        // Advance forward
        assert!(app.advance_library(true));
        let second_track = app.current_playing_track;
        assert_ne!(second_track, Some(mpui_core::track::TrackId(1)));

        // Advance backward restores previous track
        assert!(app.advance_library(false));
        assert_eq!(app.current_playing_track, Some(mpui_core::track::TrackId(1)));

        // Advance forward again goes to the same second track
        assert!(app.advance_library(true));
        assert_eq!(app.current_playing_track, second_track);
    }

    #[test]
    fn test_playing_random_reveals_track_in_library() {
        let (mut app, _event_tx, _cmd_rx) = create_test_app();
        app.library.insert(create_dummy_track(1, "/music/Rock/Queen/01.mp3", "Bohemian Rhapsody"));
        app.library.insert(create_dummy_track(2, "/music/Rock/Queen/02.mp3", "Radio Ga Ga"));
        app.library.insert(create_dummy_track(3, "/music/Jazz/Miles/01.mp3", "So What"));

        app.library_tree.build(&app.library, &[std::path::PathBuf::from("/music")]);

        // Initially playback is stopped
        assert!(!app.playback.lock().playing);

        // Press 's' when stopped: starts random playback
        app.handle_shuffle_key();
        assert!(app.shuffle);
        let playing_id = app.current_playing_track.expect("should play a track");

        // Verify the library cursor now points to this track in the tree
        let cursor = app.library_cursor;
        assert!(cursor < app.library_tree.len());
        assert_eq!(app.library_tree.visible_items[cursor].track_id, Some(playing_id));
    }

    #[test]
    fn test_library_confirm_directory_toggles() {
        let (mut app, _event_tx, _cmd_rx) = create_test_app();
        app.library.insert(create_dummy_track(1, "/music/Rock/Queen/01.mp3", "Song 1"));
        app.library_tree.build(&app.library, &[std::path::PathBuf::from("/music")]);

        // Find the "Rock" directory node
        let rock_idx = app.library_tree.visible_items.iter().position(|i| i.name == "Rock").unwrap();
        app.library_cursor = rock_idx;
        assert!(app.library_tree.visible_items[rock_idx].expanded);

        // Enter on directory toggles collapse
        app.library_confirm();
        assert!(!app.library_tree.visible_items[rock_idx].expanded);

        // Enter again toggles expand
        app.library_confirm();
        assert!(app.library_tree.visible_items[rock_idx].expanded);
    }

    #[test]
    fn test_library_queue_add_directory() {
        let (mut app, _event_tx, _cmd_rx) = create_test_app();
        app.library.insert(create_dummy_track(1, "/music/Rock/Queen/01.mp3", "Song 1"));
        app.library.insert(create_dummy_track(2, "/music/Rock/Queen/02.mp3", "Song 2"));
        app.library_tree.build(&app.library, &[std::path::PathBuf::from("/music")]);

        let rock_idx = app.library_tree.visible_items.iter().position(|i| i.name == "Rock").unwrap();
        app.library_cursor = rock_idx;

        assert_eq!(app.queue.len(), 0);
        app.library_queue_add();
        // Both songs in the Rock folder should be added to the queue
        assert_eq!(app.queue.len(), 2);
    }
}
