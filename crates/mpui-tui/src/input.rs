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

fn handle_normal_mode(app: &mut App, key: KeyEvent) {
    if let Some((pending_char, time)) = app.pending_key {
        if time.elapsed() < std::time::Duration::from_millis(500) {
            match (pending_char, key.code) {
                ('g', KeyCode::Char('g')) => {
                    app.library_cursor = 0;
                    app.queue_cursor = 0;
                }
                ('d', KeyCode::Char('d')) => {
                    if app.focused_panel == FocusedPanel::Queue {
                        let ids = app.queue.items();
                        if app.queue_cursor < ids.len() {
                            app.queue.remove(app.queue_cursor);
                            app.queue_cursor = app.queue_cursor.saturating_sub(1);
                        }
                    }
                }
                ('z', KeyCode::Char('z')) => {
                    // center_current (stub or visual only, currently app doesn't track list scroll offset)
                }
                _ => {} // discard
            }
            app.pending_key = None;
            return;
        }
        app.pending_key = None; // discard on timeout
    }

    match key.code {
        KeyCode::Char('g') | KeyCode::Char('d') | KeyCode::Char('z') => {
            if let KeyCode::Char(c) = key.code {
                app.pending_key = Some((c, std::time::Instant::now()));
            }
        }
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Char(':') => {
            app.input_mode = InputMode::Command;
            app.command_buffer.clear();
        }
        KeyCode::Char('/') => {
            app.input_mode = InputMode::Search;
            app.search_query.clear();
            app.search_results.clear();
            app.search_cursor = 0;
        }
        KeyCode::Char('j') => move_cursor_down(app),
        KeyCode::Char('k') => move_cursor_up(app),
        KeyCode::Char('h') => {
            if app.focused_panel == FocusedPanel::Equalizer {
                app.eq_cursor = app.eq_cursor.saturating_sub(1);
            } else {
                let pos = app.playback.lock().position;
                let new_pos = pos.saturating_sub(std::time::Duration::from_secs(5));
                let _ = app.command_tx.send(PlayerCommand::Seek(new_pos));
            }
        }
        KeyCode::Char('l') => {
            if app.focused_panel == FocusedPanel::Equalizer {
                app.eq_cursor = (app.eq_cursor + 1).min(9);
            } else {
                let pos = app.playback.lock().position;
                let new_pos = pos.saturating_add(std::time::Duration::from_secs(5));
                let _ = app.command_tx.send(PlayerCommand::Seek(new_pos));
            }
        }
        KeyCode::Char('a') => {
            let ids = app.library.track_ids();
            if let Some(&id) = ids.get(app.library_cursor) {
                app.queue.push(id);
            }
        }
        KeyCode::Char('G') => move_cursor_bottom(app),
        KeyCode::Char(' ') => app.toggle_pause(),
        KeyCode::Char('+') => app.volume_up(),
        KeyCode::Char('-') => app.volume_down(),
        KeyCode::Char('m') => app.toggle_mute(),
        KeyCode::Char('s') => app.toggle_shuffle(),
        KeyCode::Char('r') => app.cycle_repeat(),
        KeyCode::Char('>') => {
            if let Some(id) = app.queue.next() {
                if let Some(track) = app.library.get(id) {
                    let _ = app.command_tx.send(PlayerCommand::Play(track.path.clone()));
                }
            }
        }
        KeyCode::Char('<') => {
            if let Some(id) = app.queue.prev() {
                if let Some(track) = app.library.get(id) {
                    let _ = app.command_tx.send(PlayerCommand::Play(track.path.clone()));
                }
            }
        }
        KeyCode::Enter => handle_confirm(app),
        KeyCode::Tab => cycle_focus(app, true),
        KeyCode::BackTab => cycle_focus(app, false),
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // ctrl-d: scroll half page down
            for _ in 0..10 {
                move_cursor_down(app);
            }
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // ctrl-u: scroll half page up
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
