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
        "v" => Some(KeyAction::ToggleVisualizer),
        "e" => Some(KeyAction::ToggleEqualizer),
        "c" => Some(KeyAction::ToggleAlbumArt),
        "y" => Some(KeyAction::ToggleLyrics),
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
                return;
            }
            _ => {}
        }
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
        }
        KeyAction::JumpBottom => move_cursor_bottom(app),
        KeyAction::CollapseOrSeekBack => {
            if app.focused_panel == FocusedPanel::Equalizer {
                app.eq_cursor = app.eq_cursor.saturating_sub(1);
            } else {
                let mut state = app.playback.lock();
                let new_pos = state.position.saturating_sub(std::time::Duration::from_secs(5));
                state.position = new_pos;
                let _ = app.command_tx.send(PlayerCommand::Seek(new_pos));
            }
        }
        KeyAction::ExpandOrSeekForward => {
            if app.focused_panel == FocusedPanel::Equalizer {
                app.eq_cursor = (app.eq_cursor + 1).min(9);
            } else {
                let mut state = app.playback.lock();
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
        KeyAction::SeekStart => {
            let mut state = app.playback.lock();
            state.position = std::time::Duration::ZERO;
            let _ = app.command_tx.send(PlayerCommand::Seek(std::time::Duration::ZERO));
        }
        KeyAction::SeekEnd => {
            let mut state = app.playback.lock();
            let dur = state.duration;
            if dur > std::time::Duration::ZERO {
                let end_pos = dur.saturating_sub(std::time::Duration::from_millis(500));
                state.position = end_pos;
                let _ = app.command_tx.send(PlayerCommand::Seek(end_pos));
            }
        }
        KeyAction::Confirm => handle_confirm(app),
        KeyAction::TogglePause => app.toggle_pause(),
        KeyAction::Search => {
            app.input_mode = InputMode::Search;
            app.search_query.clear();
            app.search_results.clear();
            app.search_cursor = 0;
        }
        KeyAction::CommandMode => {
            app.input_mode = InputMode::Command;
            app.command_buffer.clear();
        }
        KeyAction::Quit => app.should_quit = true,
        KeyAction::FocusNext => cycle_focus(app, true),
        KeyAction::FocusPrev => cycle_focus(app, false),
        KeyAction::QueueAdd => {
            let ids = app.library.track_ids();
            if let Some(&id) = ids.get(app.library_cursor) {
                app.queue.push(id);
            }
        }
        KeyAction::QueueRemove => {
            if app.focused_panel == FocusedPanel::Queue {
                let ids = app.queue.items();
                if app.queue_cursor < ids.len() {
                    app.queue.remove(app.queue_cursor);
                    app.queue_cursor = app.queue_cursor.saturating_sub(1);
                }
            }
        }
        KeyAction::NextTrack => {
            if let Some(id) = app.queue.next() {
                app.play_track_at_index(id);
            }
        }
        KeyAction::PrevTrack => {
            if let Some(id) = app.queue.prev() {
                app.play_track_at_index(id);
            }
        }
        KeyAction::ToggleVisualizer => app.show_visualizer = !app.show_visualizer,
        KeyAction::ToggleEqualizer => app.show_equalizer = !app.show_equalizer,
        KeyAction::ToggleAlbumArt => app.show_album_art = !app.show_album_art,
        KeyAction::ToggleLyrics => app.show_lyrics = !app.show_lyrics,
        KeyAction::VolumeUp => app.volume_up(),
        KeyAction::VolumeDown => app.volume_down(),
        KeyAction::MuteToggle => app.toggle_mute(),
        KeyAction::ShuffleToggle => app.toggle_shuffle(),
        KeyAction::RepeatCycle => app.cycle_repeat(),
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
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.search_query.clear();
            app.search_results.clear();
        }
        KeyCode::Enter => {
            if !app.search_results.is_empty() && app.search_cursor < app.search_results.len() {
                let target_index = app.search_results[app.search_cursor].0;
                app.library_cursor = target_index;
                app.focused_panel = FocusedPanel::Library;
            }
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            if app.search_query.is_empty() {
                app.input_mode = InputMode::Normal;
                app.search_results.clear();
            } else {
                update_search(app);
            }
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
                        app.show_lyrics = false;
                        app.show_queue = true;
                    }
                    "full" => {
                        app.show_visualizer = true;
                        app.show_equalizer = true;
                        app.show_album_art = true;
                        app.show_lyrics = true;
                        app.show_queue = true;
                    }
                    "default" => {
                        app.show_visualizer = true;
                        app.show_equalizer = true;
                        app.show_album_art = true;
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
                                        app.queue.push(id);
                                    }
                                }
                            }
                        }
                    } else if let Ok(track) = mpui_core::metadata::read_metadata(&path) {
                        let id = track.id;
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
                app.queue.push(id);
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
            app.library_cursor = (app.library_cursor + 1).min(max);
        }
        FocusedPanel::Queue => {
            let max = app.queue.len().saturating_sub(1);
            app.queue_cursor = (app.queue_cursor + 1).min(max);
        }
        FocusedPanel::Equalizer => {
            app.eq_gains[app.eq_cursor] = (app.eq_gains[app.eq_cursor] - 1.0).max(-12.0);
            let _ = app.command_tx.send(PlayerCommand::SetEqBand {
                band: app.eq_cursor,
                gain: app.eq_gains[app.eq_cursor],
            });
        }
        _ => {}
    }
}

fn move_cursor_up(app: &mut App) {
    match app.focused_panel {
        FocusedPanel::Library => {
            app.library_cursor = app.library_cursor.saturating_sub(1);
        }
        FocusedPanel::Queue => {
            app.queue_cursor = app.queue_cursor.saturating_sub(1);
        }
        FocusedPanel::Equalizer => {
            app.eq_gains[app.eq_cursor] = (app.eq_gains[app.eq_cursor] + 1.0).min(12.0);
            let _ = app.command_tx.send(PlayerCommand::SetEqBand {
                band: app.eq_cursor,
                gain: app.eq_gains[app.eq_cursor],
            });
        }
        _ => {}
    }
}

fn move_cursor_bottom(app: &mut App) {
    match app.focused_panel {
        FocusedPanel::Library => {
            app.library_cursor = app.library.len().saturating_sub(1);
        }
        FocusedPanel::Queue => {
            app.queue_cursor = app.queue.len().saturating_sub(1);
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
