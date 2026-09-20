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
    CenterCurrent,
    ScrollHalfDown,
    ScrollHalfUp,
}

fn resolve_action(key_str: &str, config: &mpui_config::Config) -> Option<KeyAction> {
    let action_str = config.keybindings.get(key_str)?;
    match action_str.as_str() {
        "move_down" => Some(KeyAction::MoveDown),
        "move_up" => Some(KeyAction::MoveUp),
        "jump_top" => Some(KeyAction::JumpTop),
        "jump_bottom" => Some(KeyAction::JumpBottom),
        "collapse_or_seek_back" => Some(KeyAction::CollapseOrSeekBack),
        "expand_or_seek_forward" => Some(KeyAction::ExpandOrSeekForward),
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
        "center_current" => Some(KeyAction::CenterCurrent),
        "scroll_half_down" => Some(KeyAction::ScrollHalfDown),
        "scroll_half_up" => Some(KeyAction::ScrollHalfUp),
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
                s.push(c.to_ascii_lowercase());
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
        KeyCode::Backspace => s.push_str("backspace"),
        _ => {}
    }
    s
}

fn handle_normal_mode(app: &mut App, key: KeyEvent) {
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
                let pos = app.playback.lock().position;
                let new_pos = pos.saturating_sub(std::time::Duration::from_secs(5));
                let _ = app.command_tx.send(PlayerCommand::Seek(new_pos));
            }
        }
        KeyAction::ExpandOrSeekForward => {
            if app.focused_panel == FocusedPanel::Equalizer {
                app.eq_cursor = (app.eq_cursor + 1).min(9);
            } else {
                let pos = app.playback.lock().position;
                let new_pos = pos.saturating_add(std::time::Duration::from_secs(5));
                let _ = app.command_tx.send(PlayerCommand::Seek(new_pos));
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
                if let Some(track) = app.library.get(id) {
                    let _ = app.command_tx.send(PlayerCommand::Play(track.path.clone()));
                    if let Some(art) = mpui_core::metadata::extract_cover_art(&track.path) {
                        app.current_art = Some((id, art));
                    } else {
                        app.current_art = None;
                    }
                }
            }
        }
        KeyAction::PrevTrack => {
            if let Some(id) = app.queue.prev() {
                if let Some(track) = app.library.get(id) {
                    let _ = app.command_tx.send(PlayerCommand::Play(track.path.clone()));
                    if let Some(art) = mpui_core::metadata::extract_cover_art(&track.path) {
                        app.current_art = Some((id, art));
                    } else {
                        app.current_art = None;
                    }
                }
            }
        }
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
    let titles: Vec<String> = ids.iter().map(|&id| {
        if let Some(track) = app.library.get(id) {
            format!("{} - {}", track.display_artist(), track.display_title())
        } else {
            String::new()
        }
    }).collect();
    
    let haystack = titles.iter().map(|s| s.as_str());
    let results = app.search_engine.search(&app.search_query, haystack);
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
        Some("rescan") => app.scan_library(),
        _ => {} // Unknown command — silently ignore for now.
    }
}

fn handle_confirm(app: &mut App) {
    match app.focused_panel {
        FocusedPanel::Library => {
            let ids = app.library.track_ids();
            if let Some(&id) = ids.get(app.library_cursor) {
                app.queue.push(id);
                if let Some(track) = app.library.get(id) {
                    let _ = app.command_tx.send(PlayerCommand::Play(track.path.clone()));
                    if let Some(art) = mpui_core::metadata::extract_cover_art(&track.path) {
                        app.current_art = Some((id, art));
                    } else {
                        app.current_art = None;
                    }
                }
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
    let panels = [Library, Queue, Visualizer, Equalizer, AlbumArt];
    let current = panels.iter().position(|&p| p == app.focused_panel).unwrap_or(0);
    let next = if forward {
        (current + 1) % panels.len()
    } else {
        (current + panels.len() - 1) % panels.len()
    };
    app.focused_panel = panels[next];
}
