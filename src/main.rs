use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use mpui_audio::player::{Player, PlayerCommand};
use mpui_config;
use mpui_tui::app::App;
use mpui_tui::input;
use mpui_tui::ui;

mod mpris;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive("mpui=info".parse()?),
        )
        .with_target(false)
        .init();

    let config = mpui_config::load(None)?;
    let player = Player::new();
    let playback_state = player.state.clone();
    let command_tx = player.command_sender();
    let spectrum = player.spectrum.clone();
    let event_rx = player.event_receiver();
    
    std::thread::spawn(move || {
        player.run();
    });

    let mut app = App::new(config, playback_state, command_tx, spectrum, event_rx);

    // Initial audio pipeline configuration
    let _ = app
        .command_tx
        .send(PlayerCommand::SetReplayGainMode(app.config.audio.replaygain.clone()));

    app.scan_library();

    // Start MPRIS D-Bus background service for global media commands
    let mpris = mpris::MprisServer::start();

    // Background config hot-reload watcher
    let (config_tx, config_rx) = crossbeam_channel::unbounded();
    let _config_watcher = if app.config.general.auto_reload_config {
        mpui_config::watch_config(None, move |new_cfg| {
            let _ = config_tx.send(new_cfg);
        })
        .ok()
    } else {
        None
    };

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let tick_rate = Duration::from_millis(33);
    let mut last_track_id = None;
    let mut last_status = (false, false);
    let mut last_vol = app.volume.as_f32();
    let mut last_repeat = app.repeat_mode;
    let mut last_shuffle = app.shuffle;
    let mut position_sync_tick = 0u32;

    loop {
        // Poll for reloaded config from file watcher
        while let Ok(new_cfg) = config_rx.try_recv() {
            app.reload_config(new_cfg);
        }

        app.handle_playback_events();

        // Handle incoming global media actions (MPRIS / OS / Bluetooth)
        if let Some(ref mpris) = mpris {
            while let Ok(action) = mpris.action_rx.try_recv() {
                match action {
                    mpris::MprisAction::PlayPause => app.toggle_pause(),
                    mpris::MprisAction::Play => {
                        let (playing, paused) = {
                            let s = app.playback.lock();
                            (s.playing, s.paused)
                        };
                        if paused || !playing {
                            app.toggle_pause();
                        }
                    }
                    mpris::MprisAction::Pause => {
                        let (playing, paused) = {
                            let s = app.playback.lock();
                            (s.playing, s.paused)
                        };
                        if playing && !paused {
                            app.toggle_pause();
                        }
                    }
                    mpris::MprisAction::Stop => {
                        let _ = app.command_tx.send(PlayerCommand::Stop);
                    }
                    mpris::MprisAction::Next => {
                        app.next_track();
                    }
                    mpris::MprisAction::Previous => {
                        app.previous_track();
                    }
                    mpris::MprisAction::SeekRelative(offset) => {
                        let mut state = app.playback.lock();
                        if state.playing {
                            let cur_micros = state.position.as_micros() as i64;
                            let new_micros = (cur_micros + offset).max(0) as u64;
                            let dur = state.duration;
                            let new_pos = Duration::from_micros(new_micros);
                            let final_pos = if dur > Duration::ZERO { new_pos.min(dur) } else { new_pos };
                            state.position = final_pos;
                            let _ = app.command_tx.send(PlayerCommand::Seek(final_pos));
                            let _ = mpris.update_tx.send(mpris::MprisUpdate::Seeked(final_pos));
                        }
                    }
                    mpris::MprisAction::SetPosition(pos) => {
                        let mut state = app.playback.lock();
                        if state.playing {
                            let dur = state.duration;
                            let final_pos = if dur > Duration::ZERO { pos.min(dur) } else { pos };
                            state.position = final_pos;
                            let _ = app.command_tx.send(PlayerCommand::Seek(final_pos));
                            let _ = mpris.update_tx.send(mpris::MprisUpdate::Seeked(final_pos));
                        }
                    }
                    mpris::MprisAction::SetVolume(vol) => {
                        app.volume = mpui_config::types::Volume::new((vol * 100.0).clamp(0.0, 100.0) as u8);
                        let _ = app.command_tx.send(PlayerCommand::SetVolume(app.volume.as_f32()));
                    }
                    mpris::MprisAction::SetLoopStatus(status) => {
                        app.repeat_mode = match status {
                            mpris_server::LoopStatus::None => mpui_config::types::RepeatMode::Off,
                            mpris_server::LoopStatus::Track => mpui_config::types::RepeatMode::Track,
                            mpris_server::LoopStatus::Playlist => mpui_config::types::RepeatMode::Queue,
                        };
                    }
                    mpris::MprisAction::SetShuffle(shuf) => {
                        if app.shuffle != shuf {
                            app.toggle_shuffle();
                        }
                    }
                }
            }

            // Sync state to MPRIS
            let cur_track_id = app.queue.current_track();
            if cur_track_id != last_track_id {
                last_track_id = cur_track_id;
                if let Some(id) = cur_track_id {
                    if let Some(track) = app.library.get(id) {
                        let dur = app.playback.lock().duration;
                        let _ = mpris.update_tx.send(mpris::MprisUpdate::Track {
                            id,
                            title: track.display_title().to_string(),
                            artist: track.display_artist().to_string(),
                            album: track.display_album().to_string(),
                            duration: dur,
                        });
                    }
                } else {
                    let _ = mpris.update_tx.send(mpris::MprisUpdate::Status(mpris_server::PlaybackStatus::Stopped));
                }
            }

            let (playing, paused) = {
                let s = app.playback.lock();
                (s.playing, s.paused)
            };
            if (playing, paused) != last_status {
                last_status = (playing, paused);
                let status = if !playing {
                    mpris_server::PlaybackStatus::Stopped
                } else if paused {
                    mpris_server::PlaybackStatus::Paused
                } else {
                    mpris_server::PlaybackStatus::Playing
                };
                let _ = mpris.update_tx.send(mpris::MprisUpdate::Status(status));
            }

            let vol = app.volume.as_f32();
            if (vol - last_vol).abs() > 0.005 {
                last_vol = vol;
                let _ = mpris.update_tx.send(mpris::MprisUpdate::Volume(vol as f64));
            }

            if app.repeat_mode != last_repeat {
                last_repeat = app.repeat_mode;
                let _ = mpris.update_tx.send(mpris::MprisUpdate::Repeat(app.repeat_mode));
            }

            if app.shuffle != last_shuffle {
                last_shuffle = app.shuffle;
                let _ = mpris.update_tx.send(mpris::MprisUpdate::Shuffle(app.shuffle));
            }

            position_sync_tick += 1;
            if position_sync_tick % 30 == 0 && playing && !paused {
                let pos = app.playback.lock().position;
                let _ = mpris.update_tx.send(mpris::MprisUpdate::Position(pos));
            }
        }

        terminal.draw(|frame| ui::render(frame, &mut app))?;

        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == crossterm::event::KeyEventKind::Press {
                    input::handle_key(&mut app, key);
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    if app.config.termux.background_playback {
        mpui_core::termux::release_wake_lock();
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
