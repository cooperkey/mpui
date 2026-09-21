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
    
    std::thread::spawn(move || {
        player.run();
    });

    let mut app = App::new(config, playback_state, command_tx, spectrum);

    // Initial audio pipeline configuration
    let _ = app
        .command_tx
        .send(PlayerCommand::SetReplayGainMode(app.config.audio.replaygain.clone()));

    app.scan_library();

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
    loop {
        // Poll for reloaded config from file watcher
        while let Ok(new_cfg) = config_rx.try_recv() {
            app.reload_config(new_cfg);
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
