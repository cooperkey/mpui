use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, FocusedPanel, InputMode};
use mpui_audio::player::PlayerCommand;

/// Process a single key event and mutate app state accordingly.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    match app.input_mode {
        InputMode::Normal => handle_normal_mode(app, key),
        InputMode::Command => handle_command_mode(app, key),
        InputMode::Search => handle_search_mode(app, key),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    MoveDown,
    MoveUp,
    JumpTop,
    JumpBottom,
    CollapseOrSeekBack,
    ExpandOrSeekForward,
    Confirm,
    TogglePause,
    Search,
    SearchNext,
    SearchPrev,
    Quit,
    CommandMode,
    FocusNext,
    FocusPrev,
    FocusLibrary,
    FocusQueue,
    FocusVisualizer,
    FocusEqualizer,
    FocusAlbumArt,
    FocusMetadata,
    FocusLyrics,
    NextTrack,
    PrevTrack,
    VolumeUp,
    VolumeDown,
    MuteToggle,
    ShuffleToggle,
    RepeatCycle,
    QueueAdd,
    QueueRemove,
    OpenDirectory,
    TrackInfo,
    ToggleVisualizer,
    ToggleEqualizer,
    ToggleAlbumArt,
    ToggleLyrics,
    CenterCurrent,
    JumpToCurrent,
    ScrollHalfDown,
    ScrollHalfUp,
    ToggleHelp,
    SeekStart,
    SeekEnd,
}

fn resolve_action(key_str: &str, config: &mpui_config::Config) -> Option<KeyAction> {
    if let Some(action_str) = config.keybindings.get(key_str) {
        let action = match action_str.as_str() {
            "move_down" => Some(KeyAction::MoveDown),
            "move_up" => Some(KeyAction::MoveUp),
            "jump_top" => Some(KeyAction::JumpTop),
            "jump_bottom" => Some(KeyAction::JumpBottom),
            "collapse_or_seek_back" => Some(KeyAction::CollapseOrSeekBack),
            "expand_or_seek_forward" => Some(KeyAction::ExpandOrSeekForward),
            "seek_start" | "seek_beginning" | "seek_first" => Some(KeyAction::SeekStart),
            "seek_end" | "seek_last" => Some(KeyAction::SeekEnd),
            "confirm" => Some(KeyAction::Confirm),
            "toggle_pause" => Some(KeyAction::TogglePause),
            "search" => Some(KeyAction::Search),
            "search_next" => Some(KeyAction::SearchNext),
            "search_prev" => Some(KeyAction::SearchPrev),
            "quit" => Some(KeyAction::Quit),
            "command_mode" => Some(KeyAction::CommandMode),
            "focus_next" => Some(KeyAction::FocusNext),
            "focus_prev" => Some(KeyAction::FocusPrev),
            "focus_library" => Some(KeyAction::FocusLibrary),
            "focus_queue" => Some(KeyAction::FocusQueue),
            "focus_visualizer" => Some(KeyAction::FocusVisualizer),
            "focus_equalizer" => Some(KeyAction::FocusEqualizer),
            "focus_album_art" => Some(KeyAction::FocusAlbumArt),
            "focus_metadata" | "focus_info" | "focus_track_info" => Some(KeyAction::FocusMetadata),
            "focus_lyrics" => Some(KeyAction::FocusLyrics),
            "next_track" => Some(KeyAction::NextTrack),
            "prev_track" => Some(KeyAction::PrevTrack),
            "volume_up" => Some(KeyAction::VolumeUp),
            "volume_down" => Some(KeyAction::VolumeDown),
            "mute_toggle" => Some(KeyAction::MuteToggle),
            "shuffle_toggle" => Some(KeyAction::ShuffleToggle),
            "repeat_cycle" => Some(KeyAction::RepeatCycle),
            "queue_add" => Some(KeyAction::QueueAdd),
            "queue_remove" => Some(KeyAction::QueueRemove),
            "open_directory" => Some(KeyAction::OpenDirectory),
            "track_info" => Some(KeyAction::TrackInfo),
            "toggle_visualizer" => Some(KeyAction::ToggleVisualizer),
            "toggle_equalizer" => Some(KeyAction::ToggleEqualizer),
            "toggle_album_art" => Some(KeyAction::ToggleAlbumArt),
            "toggle_lyrics" => Some(KeyAction::ToggleLyrics),
            "center_current" => Some(KeyAction::CenterCurrent),
            "jump_to_current" | "center_library" => Some(KeyAction::JumpToCurrent),
            "scroll_half_down" => Some(KeyAction::ScrollHalfDown),
            "scroll_half_up" => Some(KeyAction::ScrollHalfUp),
            "toggle_help" => Some(KeyAction::ToggleHelp),
            _ => None,
        };
        if action.is_some() {
            return action;
        }
    }

    // Standard intuitive key fallbacks
    match key_str {
        "down" => Some(KeyAction::MoveDown),
        "up" => Some(KeyAction::MoveUp),
        "left" => Some(KeyAction::CollapseOrSeekBack),
        "right" => Some(KeyAction::ExpandOrSeekForward),
        "0" => Some(KeyAction::SeekStart),
        "$" => Some(KeyAction::SeekEnd),
        "pagedown" | "page_down" => Some(KeyAction::ScrollHalfDown),
        "pageup" | "page_up" => Some(KeyAction::ScrollHalfUp),
        "home" => Some(KeyAction::JumpTop),
        "end" => Some(KeyAction::JumpBottom),
        "enter" => Some(KeyAction::Confirm),
        "space" => Some(KeyAction::TogglePause),
        "media_play" | "media_pause" | "media_play_pause" => Some(KeyAction::TogglePause),
        "media_next" => Some(KeyAction::NextTrack),
        "media_prev" => Some(KeyAction::PrevTrack),
        "media_vol_up" => Some(KeyAction::VolumeUp),
        "media_vol_down" => Some(KeyAction::VolumeDown),
        "media_mute" => Some(KeyAction::MuteToggle),
        "v" => Some(KeyAction::ToggleVisualizer),
        "e" => Some(KeyAction::ToggleEqualizer),
        "c" => Some(KeyAction::ToggleAlbumArt),
        "i" => Some(KeyAction::TrackInfo),
        "y" => Some(KeyAction::ToggleLyrics),
        "[" => Some(KeyAction::FocusPrev),
        "]" => Some(KeyAction::FocusNext),
        "1" => Some(KeyAction::FocusLibrary),
        "2" => Some(KeyAction::FocusQueue),
        "3" => Some(KeyAction::FocusVisualizer),
        "4" => Some(KeyAction::FocusEqualizer),
        "5" => Some(KeyAction::FocusAlbumArt),
        "6" => Some(KeyAction::FocusMetadata),
        "7" => Some(KeyAction::FocusLyrics),
        "." => Some(KeyAction::JumpToCurrent),
        "?" | "f1" => Some(KeyAction::ToggleHelp),
        _ => None,
    }
}

fn key_event_to_string(key: &KeyEvent) -> String {
    let mut s = String::new();
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        s.push_str("c-");
    }
    if key.modifiers.contains(KeyModifiers::ALT) {
        s.push_str("m-");
    }
    match key.code {
        KeyCode::Char(c) => {
            if c == ' ' {
                s.push_str("space");
            } else {
                s.push(c);
            }
        }
        KeyCode::Tab => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                s.push_str("s-tab");
            } else {
                s.push_str("tab");
            }
        }
        KeyCode::BackTab => {
            s.push_str("s-tab");
        }
        KeyCode::Enter => s.push_str("enter"),
        KeyCode::Esc => s.push_str("esc"),
        KeyCode::Up => s.push_str("up"),
        KeyCode::Down => s.push_str("down"),
        KeyCode::Left => s.push_str("left"),
        KeyCode::Right => s.push_str("right"),
        KeyCode::PageDown => s.push_str("pagedown"),
        KeyCode::PageUp => s.push_str("pageup"),
        KeyCode::Home => s.push_str("home"),
        KeyCode::End => s.push_str("end"),
        KeyCode::Backspace => s.push_str("backspace"),
        KeyCode::F(n) => s.push_str(&format!("f{n}")),
        KeyCode::Media(m) => match m {
            crossterm::event::MediaKeyCode::Play => s.push_str("media_play"),
            crossterm::event::MediaKeyCode::Pause => s.push_str("media_pause"),
            crossterm::event::MediaKeyCode::PlayPause => s.push_str("media_play_pause"),
            crossterm::event::MediaKeyCode::Stop => s.push_str("media_stop"),
            crossterm::event::MediaKeyCode::TrackNext => s.push_str("media_next"),
            crossterm::event::MediaKeyCode::TrackPrevious => s.push_str("media_prev"),
            crossterm::event::MediaKeyCode::LowerVolume => s.push_str("media_vol_down"),
            crossterm::event::MediaKeyCode::RaiseVolume => s.push_str("media_vol_up"),
            crossterm::event::MediaKeyCode::MuteVolume => s.push_str("media_mute"),
            _ => {}
        },
        _ => {}
    }
    s
}

fn handle_normal_mode(app: &mut App, key: KeyEvent) {
    if app.show_help {
        match key.code {
            KeyCode::Esc
            | KeyCode::Char('q')
            | KeyCode::Char('?')
            | KeyCode::Enter
            | KeyCode::F(1) => {
                app.show_help = false;
                app.help_scroll = 0;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.help_scroll = app.help_scroll.saturating_add(1);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.help_scroll = app.help_scroll.saturating_sub(1);
            }
            KeyCode::PageDown => {
                app.help_scroll = app.help_scroll.saturating_add(5);
            }
            KeyCode::PageUp => {
                app.help_scroll = app.help_scroll.saturating_sub(5);
            }
            KeyCode::Home => {
                app.help_scroll = 0;
            }
            _ => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    match key.code {
                        KeyCode::Char('d') => {
                            app.help_scroll = app.help_scroll.saturating_add(5);
                        }
                        KeyCode::Char('u') => {
                            app.help_scroll = app.help_scroll.saturating_sub(5);
                        }
                        _ => {}
                    }
                }
            }
        }
        return;
    }

    let key_str = key_event_to_string(&key);
    if key_str.is_empty() {
        return;
    }

    if let Some((pending_str, time)) = app.pending_key.clone() {
        if time.elapsed() < std::time::Duration::from_millis(500) {
            let combined = format!("{} {}", pending_str, key_str);
            if let Some(action) = resolve_action(&combined, &app.config) {
                app.pending_key = None;
                execute_action(app, action);
                return;
            }
        }
        app.pending_key = None;
    }

    let is_prefix = app.config.keybindings.keys().any(|k| k.starts_with(&format!("{} ", key_str)));
    if is_prefix {
        app.pending_key = Some((key_str, std::time::Instant::now()));
        return;
    }

    if let Some(action) = resolve_action(&key_str, &app.config) {
        execute_action(app, action);
    }
}

fn execute_action(app: &mut App, action: KeyAction) {
    match action {
        KeyAction::MoveDown => move_cursor_down(app),
        KeyAction::MoveUp => move_cursor_up(app),
        KeyAction::JumpTop => {
            app.library_cursor = 0;
            app.queue_cursor = 0;
            app.metadata_scroll = 0;
        }
        KeyAction::JumpBottom => move_cursor_bottom(app),
        KeyAction::CollapseOrSeekBack => {
            if app.focused_panel == FocusedPanel::Equalizer {
                app.eq_cursor = app.eq_cursor.saturating_sub(1);
            } else {
                let mut state = app.playback.lock();
                if state.playing {
                    let new_pos = state.position.saturating_sub(std::time::Duration::from_secs(5));
                    state.position = new_pos;
                    let _ = app.command_tx.send(PlayerCommand::Seek(new_pos));
                }
            }
        }
        KeyAction::ExpandOrSeekForward => {
            if app.focused_panel == FocusedPanel::Equalizer {
                app.eq_cursor = (app.eq_cursor + 1).min(9);
            } else {
                let mut state = app.playback.lock();
                if state.playing {
                    let dur = state.duration;
                    let new_pos = if dur > std::time::Duration::ZERO {
                        state.position.saturating_add(std::time::Duration::from_secs(5)).min(dur)
                    } else {
                        state.position.saturating_add(std::time::Duration::from_secs(5))
                    };
                    state.position = new_pos;
                    let _ = app.command_tx.send(PlayerCommand::Seek(new_pos));
                }
            }
        }
        KeyAction::SeekStart => {
            let mut state = app.playback.lock();
            if state.playing {
                state.position = std::time::Duration::ZERO;
                let _ = app.command_tx.send(PlayerCommand::Seek(std::time::Duration::ZERO));
            }
        }
        KeyAction::SeekEnd => {
            let mut state = app.playback.lock();
            if state.playing {
                let dur = state.duration;
                if dur > std::time::Duration::ZERO {
                    let end_pos = dur.saturating_sub(std::time::Duration::from_millis(500));
                    state.position = end_pos;
                    let _ = app.command_tx.send(PlayerCommand::Seek(end_pos));
                }
            }
        }
        KeyAction::Confirm => handle_confirm(app),
        KeyAction::TogglePause => app.toggle_pause(),
        KeyAction::Search => {
            app.input_mode = InputMode::Search;
            app.search_vim_mode = crate::app::SearchVimMode::Normal;
            app.search_query.clear();
            app.search_results.clear();
            app.search_cursor = 0;
            app.pending_key = None;
        }
        KeyAction::CommandMode => {
            app.input_mode = InputMode::Command;
            app.command_buffer.clear();
        }
        KeyAction::Quit => app.should_quit = true,
        KeyAction::FocusNext => cycle_focus(app, true),
        KeyAction::FocusPrev => cycle_focus(app, false),
        KeyAction::FocusLibrary => focus_panel(app, FocusedPanel::Library),
        KeyAction::FocusQueue => focus_panel(app, FocusedPanel::Queue),
        KeyAction::FocusVisualizer => focus_panel(app, FocusedPanel::Visualizer),
        KeyAction::FocusEqualizer => focus_panel(app, FocusedPanel::Equalizer),
        KeyAction::FocusAlbumArt => focus_panel(app, FocusedPanel::AlbumArt),
        KeyAction::FocusMetadata => focus_panel(app, FocusedPanel::Metadata),
        KeyAction::FocusLyrics => focus_panel(app, FocusedPanel::Lyrics),
        KeyAction::QueueAdd => {
            let ids = app.library.track_ids();
            if let Some(&id) = ids.get(app.library_cursor) {
                app.queue.push(id);
            }
        }
        KeyAction::QueueRemove => {
            if app.focused_panel == FocusedPanel::Queue {
                if app.queue_cursor < app.queue.len() {
                    app.queue.remove(app.queue_cursor);
                    if app.queue.is_empty() {
                        app.queue_cursor = 0;
                    } else if app.queue_cursor >= app.queue.len() {
                        app.queue_cursor = app.queue.len() - 1;
                    }
                }
            }
        }
        KeyAction::NextTrack => app.next_track(),
        KeyAction::PrevTrack => app.previous_track(),
        KeyAction::ToggleVisualizer => app.show_visualizer = !app.show_visualizer,
        KeyAction::ToggleEqualizer => app.show_equalizer = !app.show_equalizer,
        KeyAction::ToggleAlbumArt => app.show_album_art = !app.show_album_art,
        KeyAction::TrackInfo => app.show_metadata = !app.show_metadata,
        KeyAction::ToggleLyrics => app.show_lyrics = !app.show_lyrics,
        KeyAction::VolumeUp => app.volume_up(),
        KeyAction::VolumeDown => app.volume_down(),
        KeyAction::MuteToggle => app.toggle_mute(),
        KeyAction::ShuffleToggle => app.handle_shuffle_key(),
        KeyAction::RepeatCycle => app.cycle_repeat(),
        KeyAction::JumpToCurrent => {
            let cur_id = app.current_playing_track.or_else(|| app.queue.current_track());
            if let Some(id) = cur_id {
                if let Some(idx) = app.library.track_ids().iter().position(|&t| t == id) {
                    app.library_cursor = idx;
                    app.focused_panel = FocusedPanel::Library;
                    app.metadata_scroll = 0;
                }
            }
        }
        KeyAction::CenterCurrent => match app.focused_panel {
            FocusedPanel::Queue => {
                if let Some(idx) = app.queue.current_index() {
                    app.queue_cursor = idx;
                }
            }
            FocusedPanel::Library => {
                let cur_id = app.current_playing_track.or_else(|| app.queue.current_track());
                if let Some(id) = cur_id {
                    if let Some(idx) = app.library.track_ids().iter().position(|&t| t == id) {
                        app.library_cursor = idx;
                    }
                }
            }
            _ => {}
        },
        KeyAction::ScrollHalfDown => {
            for _ in 0..10 {
                move_cursor_down(app);
            }
        }
        KeyAction::ScrollHalfUp => {
            for _ in 0..10 {
                move_cursor_up(app);
            }
        }
        KeyAction::ToggleHelp => app.show_help = !app.show_help,
        _ => {}
    }
}

fn handle_command_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.command_buffer.clear();
        }
        KeyCode::Enter => {
            let cmd = app.command_buffer.clone();
            execute_command(app, &cmd);
            app.input_mode = InputMode::Normal;
            app.command_buffer.clear();
        }
        KeyCode::Tab => {
            let suggestion = crate::command::get_suggestion(&app.command_buffer);
            if let Some(completion) = suggestion.completion {
                app.command_buffer = completion;
            }
        }
        KeyCode::Backspace => {
            app.command_buffer.pop();
            if app.command_buffer.is_empty() {
                app.input_mode = InputMode::Normal;
            }
        }
        KeyCode::Char(c) => {
            app.command_buffer.push(c);
        }
        _ => {}
    }
}

fn handle_search_mode(app: &mut App, key: KeyEvent) {
    match app.search_vim_mode {
        crate::app::SearchVimMode::Normal => handle_search_normal_mode(app, key),
        crate::app::SearchVimMode::Insert => handle_search_insert_mode(app, key),
    }
}

fn handle_search_insert_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            // Esc exits insert mode and switches to normal mode (does NOT exit search window)
            app.search_vim_mode = crate::app::SearchVimMode::Normal;
            app.pending_key = None;
        }
        KeyCode::Enter => {
            // Enter commits text input and switches to normal mode
            app.search_vim_mode = crate::app::SearchVimMode::Normal;
            app.pending_key = None;
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            update_search(app);
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            update_search(app);
        }
        KeyCode::Down => {
            let max = app.search_results.len().saturating_sub(1);
            app.search_cursor = (app.search_cursor + 1).min(max);
        }
        KeyCode::Up => {
            app.search_cursor = app.search_cursor.saturating_sub(1);
        }
        _ => {}
    }
}

fn handle_search_normal_mode(app: &mut App, key: KeyEvent) {
    if let Some((ref pending_str, time)) = app.pending_key.clone() {
        if time.elapsed() < std::time::Duration::from_millis(500) {
            if pending_str == "g" && key.code == KeyCode::Char('g') {
                app.search_cursor = 0;
                app.pending_key = None;
                return;
            }
        }
        app.pending_key = None;
    }

    match key.code {
        KeyCode::Esc => {
            // Only exits search window when escape is pressed on normal mode
            app.input_mode = InputMode::Normal;
            app.search_vim_mode = crate::app::SearchVimMode::Normal;
            app.search_query.clear();
            app.search_results.clear();
            app.search_cursor = 0;
            app.pending_key = None;
        }
        KeyCode::Char('i') | KeyCode::Char('/') => {
            app.search_vim_mode = crate::app::SearchVimMode::Insert;
            app.pending_key = None;
        }
        KeyCode::Char('A') => {
            // Add all tracks matching the searched query to queue/playlist without exiting
            let ids = app.library.track_ids();
            for &(orig_index, _) in &app.search_results {
                if let Some(&id) = ids.get(orig_index) {
                    app.queue.push(id);
                }
            }
        }
        KeyCode::Char('a') => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                let ids = app.library.track_ids();
                for &(orig_index, _) in &app.search_results {
                    if let Some(&id) = ids.get(orig_index) {
                        app.queue.push(id);
                    }
                }
            } else if !app.search_results.is_empty() && app.search_cursor < app.search_results.len() {
                // Add the selected track to queue/playlist without exiting
                let orig_index = app.search_results[app.search_cursor].0;
                if let Some(&id) = app.library.track_ids().get(orig_index) {
                    app.queue.push(id);
                }
            }
        }
        KeyCode::Char('j') | KeyCode::Down | KeyCode::Char('n') => {
            let max = app.search_results.len().saturating_sub(1);
            app.search_cursor = (app.search_cursor + 1).min(max);
        }
        KeyCode::Char('k') | KeyCode::Up | KeyCode::Char('N') => {
            app.search_cursor = app.search_cursor.saturating_sub(1);
        }
        KeyCode::Char('g') => {
            app.pending_key = Some(("g".to_string(), std::time::Instant::now()));
        }
        KeyCode::Char('G') | KeyCode::End => {
            let max = app.search_results.len().saturating_sub(1);
            app.search_cursor = max;
        }
        KeyCode::Home => {
            app.search_cursor = 0;
        }
        KeyCode::PageDown => {
            let max = app.search_results.len().saturating_sub(1);
            app.search_cursor = (app.search_cursor + 10).min(max);
        }
        KeyCode::PageUp => {
            app.search_cursor = app.search_cursor.saturating_sub(10);
        }
        KeyCode::Enter => {
            if !app.search_results.is_empty() && app.search_cursor < app.search_results.len() {
                let orig_index = app.search_results[app.search_cursor].0;
                if let Some(&id) = app.library.track_ids().get(orig_index) {
                    app.play_track_at_index(id);
                }
            }
        }
        KeyCode::Char(' ') => {
            app.toggle_pause();
        }
        KeyCode::Char('+') | KeyCode::Char('=') => {
            app.volume_up();
        }
        KeyCode::Char('-') => {
            app.volume_down();
        }
        KeyCode::Char('m') => {
            app.toggle_mute();
        }
        KeyCode::Char('s') => {
            app.handle_shuffle_key();
        }
        KeyCode::Char('r') => {
            app.cycle_repeat();
        }
        KeyCode::Char('>') => {
            app.next_track();
        }
        KeyCode::Char('<') => {
            app.previous_track();
        }
        _ => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match key.code {
                    KeyCode::Char('d') => {
                        let max = app.search_results.len().saturating_sub(1);
                        app.search_cursor = (app.search_cursor + 10).min(max);
                    }
                    KeyCode::Char('u') => {
                        app.search_cursor = app.search_cursor.saturating_sub(10);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn update_search(app: &mut App) {
    let ids = app.library.track_ids();
    let query = app.search_query.trim();

    let (scope, actual_query) = if let Some(stripped) = query.strip_prefix("t:") {
        ("title", stripped.trim())
    } else if let Some(stripped) = query.strip_prefix("a:") {
        ("artist", stripped.trim())
    } else if let Some(stripped) = query.strip_prefix("b:") {
        ("album", stripped.trim())
    } else if let Some(stripped) = query.strip_prefix("g:") {
        ("genre", stripped.trim())
    } else {
        ("all", query)
    };

    let strings: Vec<String> = ids.iter().map(|&id| {
        if let Some(track) = app.library.get(id) {
            match scope {
                "title" => track.display_title().to_string(),
                "artist" => track.display_artist().to_string(),
                "album" => track.display_album().to_string(),
                "genre" => track.genre.clone().unwrap_or_default(),
                _ => format!("{} - {} {}", track.display_artist(), track.display_title(), track.display_album()),
            }
        } else {
            String::new()
        }
    }).collect();

    let haystack = strings.iter().map(|s| s.as_str());
    let results = app.search_engine.search(actual_query, haystack);
    app.search_results = results.into_iter().map(|r| (r.index, r.score)).collect();
    app.search_cursor = 0;
}

fn execute_command(app: &mut App, cmd: &str) {
    let parts: Vec<&str> = cmd.trim().splitn(2, ' ').collect();
    match parts.first().copied() {
        Some("q" | "quit") => app.should_quit = true,
        Some("vol") => {
            if let Some(val) = parts.get(1).and_then(|v| v.parse::<u8>().ok()) {
                app.volume = mpui_config::types::Volume::new(val);
                let _ = app
                    .command_tx
                    .send(PlayerCommand::SetVolume(app.volume.as_f32()));
            }
        }
        Some("seek") => {
            if let Some(arg) = parts.get(1) {
                if *arg == "$" || *arg == "end" {
                    let mut state = app.playback.lock();
                    let dur = state.duration;
                    if dur > std::time::Duration::ZERO {
                        let end_pos = dur.saturating_sub(std::time::Duration::from_millis(500));
                        state.position = end_pos;
                        let _ = app.command_tx.send(PlayerCommand::Seek(end_pos));
                    }
                } else if let Some(dur) = parse_duration_str(arg) {
                    let _ = app.command_tx.send(PlayerCommand::Seek(dur));
                }
            }
        }
        Some("eq") => {
            if let Some(preset_name) = parts.get(1) {
                if let Some(preset) = app.config.equalizer.presets.get(*preset_name) {
                    app.eq_gains = preset.bands;
                    let _ = app.command_tx.send(PlayerCommand::SetEqPreset(preset.bands));
                }
            }
        }
        Some("sort") => {
            if let Some(field_str) = parts.get(1) {
                let field = match field_str.to_lowercase().as_str() {
                    "artist" => Some(mpui_core::library::SortField::Artist),
                    "album" => Some(mpui_core::library::SortField::Album),
                    "title" => Some(mpui_core::library::SortField::Title),
                    "duration" | "time" => Some(mpui_core::library::SortField::Duration),
                    "year" | "date" => Some(mpui_core::library::SortField::Year),
                    _ => None,
                };
                if let Some(f) = field {
                    app.library.sort_by(f);
                }
            }
        }
        Some("toggle") => {
            if let Some(panel_str) = parts.get(1) {
                match panel_str.to_lowercase().as_str() {
                    "viz" | "visualizer" => app.show_visualizer = !app.show_visualizer,
                    "eq" | "equalizer" => app.show_equalizer = !app.show_equalizer,
                    "art" | "album_art" => app.show_album_art = !app.show_album_art,
                    "meta" | "metadata" | "info" => app.show_metadata = !app.show_metadata,
                    "lyrics" | "lrc" => app.show_lyrics = !app.show_lyrics,
                    "queue" => app.show_queue = !app.show_queue,
                    _ => {}
                }
            }
        }
        Some("layout") => {
            if let Some(layout_name) = parts.get(1) {
                match layout_name.to_lowercase().as_str() {
                    "minimal" => {
                        app.show_visualizer = false;
                        app.show_equalizer = false;
                        app.show_album_art = false;
                        app.show_metadata = false;
                        app.show_lyrics = false;
                        app.show_queue = true;
                    }
                    "full" => {
                        app.show_visualizer = true;
                        app.show_equalizer = true;
                        app.show_album_art = true;
                        app.show_metadata = true;
                        app.show_lyrics = true;
                        app.show_queue = true;
                    }
                    "default" => {
                        app.show_visualizer = true;
                        app.show_equalizer = true;
                        app.show_album_art = true;
                        app.show_metadata = true;
                        app.show_lyrics = false;
                        app.show_queue = true;
                    }
                    _ => {}
                }
            }
        }
        Some("w") => {
            let filename = parts.get(1).map(|s| s.to_string()).unwrap_or_else(|| "playlist.m3u".to_string());
            let filename = if !filename.ends_with(".m3u") && !filename.ends_with(".m3u8") {
                format!("{filename}.m3u")
            } else {
                filename
            };
            let save_dir = &app.config.playlist.save_path;
            let _ = std::fs::create_dir_all(save_dir);
            let target = save_dir.join(filename);
            let mut content = String::from("#EXTM3U\n");
            for &id in app.queue.items() {
                if let Some(track) = app.library.get(id) {
                    content.push_str(&format!("#EXTINF:{},{}\n", track.duration.map(|d| d.as_secs()).unwrap_or(0), track.display_title()));
                    content.push_str(&format!("{}\n", track.path.display()));
                }
            }
            let _ = std::fs::write(target, content);
        }
        Some("e") => {
            if let Some(arg) = parts.get(1) {
                let path = std::path::PathBuf::from(arg);
                let path = mpui_config::expand_tilde(&path);
                if path.exists() {
                    if path.extension().and_then(|ext| ext.to_str()).map(|ext| ext.eq_ignore_ascii_case("m3u") || ext.eq_ignore_ascii_case("m3u8")).unwrap_or(false) {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            for line in content.lines() {
                                let line = line.trim();
                                if line.starts_with('#') || line.is_empty() {
                                    continue;
                                }
                                let track_path = std::path::PathBuf::from(line);
                                if track_path.exists() {
                                    if let Ok(track) = mpui_core::metadata::read_metadata(&track_path) {
                                        let id = track.id;
                                        app.library.insert(track);
                                        app.queue.push(id);
                                    }
                                }
                            }
                        }
                    } else if let Ok(track) = mpui_core::metadata::read_metadata(&path) {
                        let id = track.id;
                        app.library.insert(track.clone());
                        app.queue.push(id);
                        let _ = app.command_tx.send(PlayerCommand::Play(track.path.clone()));
                        app.current_art = mpui_core::metadata::extract_cover_art(&track.path).map(|art| (id, art));
                        app.current_lyrics = mpui_core::metadata::extract_lyrics(&track.path);
                    }
                }
            }
        }
        Some("rescan") => app.scan_library(),
        Some("help" | "keys") => app.show_help = true,
        _ => {} // Unknown command — silently ignore for now.
    }
}

fn parse_duration_str(s: &str) -> Option<std::time::Duration> {
    let s = s.trim();
    if s.contains(':') {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() == 2 {
            let mins: u64 = parts[0].parse().ok()?;
            let secs: u64 = parts[1].parse().ok()?;
            return Some(std::time::Duration::from_secs(mins * 60 + secs));
        } else if parts.len() == 3 {
            let hours: u64 = parts[0].parse().ok()?;
            let mins: u64 = parts[1].parse().ok()?;
            let secs: u64 = parts[2].parse().ok()?;
            return Some(std::time::Duration::from_secs(hours * 3600 + mins * 60 + secs));
        }
        None
    } else {
        let secs: u64 = s.parse().ok()?;
        Some(std::time::Duration::from_secs(secs))
    }
}

fn handle_confirm(app: &mut App) {
    match app.focused_panel {
        FocusedPanel::Library => {
            let ids = app.library.track_ids();
            if let Some(&id) = ids.get(app.library_cursor) {
                app.play_track_at_index(id);
            }
        }
        FocusedPanel::Queue => {
            let items = app.queue.items();
            if let Some(&id) = items.get(app.queue_cursor) {
                app.queue.set_current(app.queue_cursor);
                app.play_track_at_index(id);
            }
        }
        _ => {}
    }
}

fn move_cursor_down(app: &mut App) {
    match app.focused_panel {
        FocusedPanel::Library => {
            let max = app.library.len().saturating_sub(1);
            if app.library_cursor < max {
                app.library_cursor += 1;
                app.metadata_scroll = 0;
            }
        }
        FocusedPanel::Queue => {
            let max = app.queue.len().saturating_sub(1);
            if app.queue_cursor < max {
                app.queue_cursor += 1;
                app.metadata_scroll = 0;
            }
        }
        FocusedPanel::Equalizer => {
            app.eq_gains[app.eq_cursor] = (app.eq_gains[app.eq_cursor] - 1.0).max(-12.0);
            let _ = app.command_tx.send(PlayerCommand::SetEqBand {
                band: app.eq_cursor,
                gain: app.eq_gains[app.eq_cursor],
            });
        }
        FocusedPanel::Metadata => {
            app.metadata_scroll = app.metadata_scroll.saturating_add(1);
        }
        _ => {}
    }
}

fn move_cursor_up(app: &mut App) {
    match app.focused_panel {
        FocusedPanel::Library => {
            if app.library_cursor > 0 {
                app.library_cursor -= 1;
                app.metadata_scroll = 0;
            }
        }
        FocusedPanel::Queue => {
            if app.queue_cursor > 0 {
                app.queue_cursor -= 1;
                app.metadata_scroll = 0;
            }
        }
        FocusedPanel::Equalizer => {
            app.eq_gains[app.eq_cursor] = (app.eq_gains[app.eq_cursor] + 1.0).min(12.0);
            let _ = app.command_tx.send(PlayerCommand::SetEqBand {
                band: app.eq_cursor,
                gain: app.eq_gains[app.eq_cursor],
            });
        }
        FocusedPanel::Metadata => {
            app.metadata_scroll = app.metadata_scroll.saturating_sub(1);
        }
        _ => {}
    }
}

fn move_cursor_bottom(app: &mut App) {
    match app.focused_panel {
        FocusedPanel::Library => {
            app.library_cursor = app.library.len().saturating_sub(1);
            app.metadata_scroll = 0;
        }
        FocusedPanel::Queue => {
            app.queue_cursor = app.queue.len().saturating_sub(1);
            app.metadata_scroll = 0;
        }
        _ => {}
    }
}

fn cycle_focus(app: &mut App, forward: bool) {
    use FocusedPanel::*;
    let mut panels = vec![Library];
    if app.show_queue {
        panels.push(Queue);
    }
    if app.show_visualizer {
        panels.push(Visualizer);
    }
    if app.show_equalizer {
        panels.push(Equalizer);
    }
    if app.show_album_art {
        panels.push(AlbumArt);
    }
    if app.show_metadata {
        panels.push(Metadata);
    }
    if app.show_lyrics {
        panels.push(Lyrics);
    }

    let current = panels.iter().position(|&p| p == app.focused_panel).unwrap_or(0);
    let next = if forward {
        (current + 1) % panels.len()
    } else {
        (current + panels.len() - 1) % panels.len()
    };
    app.focused_panel = panels[next];
}

fn focus_panel(app: &mut App, panel: FocusedPanel) {
    match panel {
        FocusedPanel::Library => {
            app.focused_panel = FocusedPanel::Library;
        }
        FocusedPanel::Queue => {
            app.show_queue = true;
            app.focused_panel = FocusedPanel::Queue;
        }
        FocusedPanel::Visualizer => {
            app.show_visualizer = true;
            app.focused_panel = FocusedPanel::Visualizer;
        }
        FocusedPanel::Equalizer => {
            app.show_equalizer = true;
            app.focused_panel = FocusedPanel::Equalizer;
        }
        FocusedPanel::AlbumArt => {
            app.show_album_art = true;
            app.focused_panel = FocusedPanel::AlbumArt;
        }
        FocusedPanel::Metadata => {
            app.show_metadata = true;
            app.focused_panel = FocusedPanel::Metadata;
        }
        FocusedPanel::Lyrics => {
            app.show_lyrics = true;
            app.focused_panel = FocusedPanel::Lyrics;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use mpui_audio::player::PlaybackState;
    use mpui_config::Config;
    use parking_lot::Mutex;
    use std::sync::Arc;

    fn create_test_app() -> App {
        let config = Config::default();
        let playback = Arc::new(Mutex::new(PlaybackState::default()));
        let (cmd_tx, _cmd_rx) = crossbeam_channel::unbounded();
        let spectrum = Arc::new(Mutex::new(Vec::new()));
        let (_event_tx, event_rx) = crossbeam_channel::unbounded();
        App::new(config, playback, cmd_tx, spectrum, event_rx)
    }

    fn key_press(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn test_number_shortcuts_switch_panes() {
        let mut app = create_test_app();
        assert_eq!(app.focused_panel, FocusedPanel::Library);

        handle_key(&mut app, key_press(KeyCode::Char('2')));
        assert_eq!(app.focused_panel, FocusedPanel::Queue);

        handle_key(&mut app, key_press(KeyCode::Char('3')));
        assert_eq!(app.focused_panel, FocusedPanel::Visualizer);

        handle_key(&mut app, key_press(KeyCode::Char('4')));
        assert_eq!(app.focused_panel, FocusedPanel::Equalizer);

        handle_key(&mut app, key_press(KeyCode::Char('5')));
        assert_eq!(app.focused_panel, FocusedPanel::AlbumArt);

        handle_key(&mut app, key_press(KeyCode::Char('6')));
        assert_eq!(app.focused_panel, FocusedPanel::Metadata);

        handle_key(&mut app, key_press(KeyCode::Char('7')));
        assert_eq!(app.focused_panel, FocusedPanel::Lyrics);

        handle_key(&mut app, key_press(KeyCode::Char('1')));
        assert_eq!(app.focused_panel, FocusedPanel::Library);
    }

    #[test]
    fn test_bracket_shortcuts_cycle_panes() {
        let mut app = create_test_app();
        assert_eq!(app.focused_panel, FocusedPanel::Library);

        handle_key(&mut app, key_press(KeyCode::Char(']')));
        assert_eq!(app.focused_panel, FocusedPanel::Queue);

        handle_key(&mut app, key_press(KeyCode::Char('[')));
        assert_eq!(app.focused_panel, FocusedPanel::Library);
    }

    fn create_dummy_track(id: u64, path: &str, title: &str) -> mpui_core::track::Track {
        mpui_core::track::Track {
            id: mpui_core::track::TrackId(id),
            path: std::path::PathBuf::from(path),
            title: Some(title.to_string()),
            artist: None,
            album: None,
            album_artist: None,
            genre: None,
            year: None,
            track_number: None,
            disc_number: None,
            duration: None,
            sample_rate: None,
            channels: None,
            bitrate: None,
            format: "mp3".to_string(),
            replay_gain_track: None,
            replay_gain_album: None,
        }
    }

    #[test]
    fn test_dot_shortcut_jumps_to_current_playing_track_in_library() {
        let mut app = create_test_app();
        let track1 = create_dummy_track(10, "/music/song1.mp3", "Song 1");
        let track2 = create_dummy_track(20, "/music/song2.mp3", "Song 2");

        app.library.insert(track1);
        app.library.insert(track2);
        app.current_playing_track = Some(mpui_core::track::TrackId(20));

        // Switch to Equalizer panel
        app.focused_panel = FocusedPanel::Equalizer;
        assert_eq!(app.library_cursor, 0);

        // Press "."
        handle_key(&mut app, key_press(KeyCode::Char('.')));
        assert_eq!(app.focused_panel, FocusedPanel::Library);
        assert_eq!(app.library_cursor, 1);
    }

    #[test]
    fn test_command_mode_tab_completion() {
        let mut app = create_test_app();
        
        // Enter command mode
        handle_key(&mut app, key_press(KeyCode::Char(':')));
        assert_eq!(app.input_mode, InputMode::Command);

        // Type 'v', 'o'
        handle_key(&mut app, key_press(KeyCode::Char('v')));
        handle_key(&mut app, key_press(KeyCode::Char('o')));
        assert_eq!(app.command_buffer, "vo");

        // Tab completes to "vol "
        handle_key(&mut app, key_press(KeyCode::Tab));
        assert_eq!(app.command_buffer, "vol ");

        // Reset and test argument completion
        app.command_buffer = "sort ar".to_string();
        handle_key(&mut app, key_press(KeyCode::Tab));
        assert_eq!(app.command_buffer, "sort artist");
    }

    #[test]
    fn test_help_modal_scrolling() {
        let mut app = create_test_app();
        app.show_help = true;
        assert_eq!(app.help_scroll, 0);

        handle_key(&mut app, key_press(KeyCode::Char('j')));
        assert_eq!(app.help_scroll, 1);

        handle_key(&mut app, key_press(KeyCode::Down));
        assert_eq!(app.help_scroll, 2);

        handle_key(&mut app, key_press(KeyCode::Char('k')));
        assert_eq!(app.help_scroll, 1);

        handle_key(&mut app, key_press(KeyCode::Up));
        assert_eq!(app.help_scroll, 0);

        handle_key(&mut app, key_press(KeyCode::Esc));
        assert!(!app.show_help);
    }

    #[test]
    fn test_search_mode_vim_layer_modal_transitions() {
        use crate::app::SearchVimMode;

        let mut app = create_test_app();
        assert_eq!(app.input_mode, InputMode::Normal);

        // Pressing '/' opens search in Normal mode
        handle_key(&mut app, key_press(KeyCode::Char('/')));
        assert_eq!(app.input_mode, InputMode::Search);
        assert_eq!(app.search_vim_mode, SearchVimMode::Normal);

        // Pressing 'i' enters insert mode
        handle_key(&mut app, key_press(KeyCode::Char('i')));
        assert_eq!(app.input_mode, InputMode::Search);
        assert_eq!(app.search_vim_mode, SearchVimMode::Insert);

        // Pressing Esc in insert mode returns to normal mode without exiting search
        handle_key(&mut app, key_press(KeyCode::Esc));
        assert_eq!(app.input_mode, InputMode::Search);
        assert_eq!(app.search_vim_mode, SearchVimMode::Normal);

        // Pressing '/' in normal mode enters insert mode
        handle_key(&mut app, key_press(KeyCode::Char('/')));
        assert_eq!(app.input_mode, InputMode::Search);
        assert_eq!(app.search_vim_mode, SearchVimMode::Insert);

        // Pressing Enter in insert mode returns to normal mode without exiting search
        handle_key(&mut app, key_press(KeyCode::Enter));
        assert_eq!(app.input_mode, InputMode::Search);
        assert_eq!(app.search_vim_mode, SearchVimMode::Normal);

        // Pressing other keys like 'q' does NOT exit search
        handle_key(&mut app, key_press(KeyCode::Char('q')));
        assert_eq!(app.input_mode, InputMode::Search);

        // ONLY pressing Esc in normal mode exits search
        handle_key(&mut app, key_press(KeyCode::Esc));
        assert_eq!(app.input_mode, InputMode::Normal);
    }

    #[test]
    fn test_search_insert_mode_typing_and_backspace() {
        use crate::app::SearchVimMode;

        let mut app = create_test_app();
        handle_key(&mut app, key_press(KeyCode::Char('/')));
        handle_key(&mut app, key_press(KeyCode::Char('i')));
        assert_eq!(app.search_vim_mode, SearchVimMode::Insert);

        // Type characters
        handle_key(&mut app, key_press(KeyCode::Char('a')));
        handle_key(&mut app, key_press(KeyCode::Char('b')));
        handle_key(&mut app, key_press(KeyCode::Char('c')));
        assert_eq!(app.search_query, "abc");

        // Backspace characters
        handle_key(&mut app, key_press(KeyCode::Backspace));
        assert_eq!(app.search_query, "ab");

        handle_key(&mut app, key_press(KeyCode::Backspace));
        handle_key(&mut app, key_press(KeyCode::Backspace));
        assert_eq!(app.search_query, "");

        // Backspace on empty query must NOT exit search
        handle_key(&mut app, key_press(KeyCode::Backspace));
        assert_eq!(app.input_mode, InputMode::Search);
        assert_eq!(app.search_vim_mode, SearchVimMode::Insert);
    }

    #[test]
    fn test_search_normal_mode_add_to_playlist_without_exiting() {
        use crate::app::SearchVimMode;

        let mut app = create_test_app();
        let track1 = create_dummy_track(101, "/music/song1.mp3", "Alpha Song");
        let track2 = create_dummy_track(102, "/music/song2.mp3", "Beta Song");
        let track3 = create_dummy_track(103, "/music/song3.mp3", "Gamma Song");

        app.library.insert(track1);
        app.library.insert(track2);
        app.library.insert(track3);

        // Open search (starts in Normal mode)
        handle_key(&mut app, key_press(KeyCode::Char('/')));
        assert_eq!(app.input_mode, InputMode::Search);
        assert_eq!(app.search_vim_mode, SearchVimMode::Normal);

        // Enter insert mode and type query
        handle_key(&mut app, key_press(KeyCode::Char('i')));
        for c in "Song".chars() {
            handle_key(&mut app, key_press(KeyCode::Char(c)));
        }
        assert_eq!(app.search_results.len(), 3);

        // Press Esc to enter normal mode
        handle_key(&mut app, key_press(KeyCode::Esc));
        assert_eq!(app.search_vim_mode, SearchVimMode::Normal);
        assert_eq!(app.search_cursor, 0);

        // Press 'a' in normal mode: adds currently selected track (track 1) to queue
        assert_eq!(app.queue.len(), 0);
        handle_key(&mut app, key_press(KeyCode::Char('a')));
        assert_eq!(app.queue.len(), 1);
        assert_eq!(app.input_mode, InputMode::Search); // STILL in search!

        // Navigate down with 'j' and add track 2 with 'a'
        handle_key(&mut app, key_press(KeyCode::Char('j')));
        assert_eq!(app.search_cursor, 1);
        handle_key(&mut app, key_press(KeyCode::Char('a')));
        assert_eq!(app.queue.len(), 2);
        assert_eq!(app.input_mode, InputMode::Search); // STILL in search!

        // Shift+A adds all results of search query to queue
        handle_key(&mut app, key_press(KeyCode::Char('A')));
        assert_eq!(app.queue.len(), 5); // 2 + 3
        assert_eq!(app.input_mode, InputMode::Search); // STILL in search!

        // Esc in normal mode finally exits
        handle_key(&mut app, key_press(KeyCode::Esc));
        assert_eq!(app.input_mode, InputMode::Normal);
    }

    #[test]
    fn test_search_normal_mode_navigation_and_enter_play() {
        use crate::app::SearchVimMode;

        let mut app = create_test_app();
        let track1 = create_dummy_track(201, "/music/s1.mp3", "Rock 1");
        let track2 = create_dummy_track(202, "/music/s2.mp3", "Rock 2");
        let track3 = create_dummy_track(203, "/music/s3.mp3", "Rock 3");

        app.library.insert(track1);
        app.library.insert(track2);
        app.library.insert(track3);

        handle_key(&mut app, key_press(KeyCode::Char('/')));
        handle_key(&mut app, key_press(KeyCode::Char('i')));
        for c in "Rock".chars() {
            handle_key(&mut app, key_press(KeyCode::Char(c)));
        }
        handle_key(&mut app, key_press(KeyCode::Esc));
        assert_eq!(app.search_vim_mode, SearchVimMode::Normal);

        // Navigation
        handle_key(&mut app, key_press(KeyCode::Char('G')));
        assert_eq!(app.search_cursor, 2);

        // 'g g' jump to top
        handle_key(&mut app, key_press(KeyCode::Char('g')));
        handle_key(&mut app, key_press(KeyCode::Char('g')));
        assert_eq!(app.search_cursor, 0);

        // Press Enter plays selected track without exiting search
        handle_key(&mut app, key_press(KeyCode::Enter));
        assert_eq!(app.input_mode, InputMode::Search);
        assert_eq!(app.current_playing_track, Some(mpui_core::track::TrackId(201)));
    }
}

