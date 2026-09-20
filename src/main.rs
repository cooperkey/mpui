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

use mpui_audio::player::Player;
use mpui_audio::output::AudioOutput;
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
    let sample_rx = player.sample_receiver();
    
    // Default to common format. Dynamically re-opening CPAL streams per track can be tricky.
    let _audio_output = AudioOutput::new(None, None, sample_rx)?;
    
    std::thread::spawn(move || {
        player.run();
    });

    let mut app = App::new(config, playback_state, command_tx, spectrum);

    app.scan_library();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let tick_rate = Duration::from_millis(33);
    loop {
        terminal.draw(|frame| ui::render(frame, &app))?;

        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                input::handle_key(&mut app, key);
            }
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
