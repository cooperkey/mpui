use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, FocusedPanel};

/// Render the metadata and technical specs of the currently selected media.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let active = app.focused_panel == FocusedPanel::Metadata;
    let border_color = if active {
        app.theme.border_focused
    } else {
        app.theme.border
    };

    let title = if active {
        " [metadata] "
    } else {
        " metadata "
    };

    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(app.theme.title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let Some(track) = app.selected_track() else {
        let pad = inner.height.saturating_sub(1) / 2;
        let padding = "\n".repeat(pad as usize);
        let text = format!("{}no media selected", padding);
        let widget = Paragraph::new(text)
            .block(block)
            .style(Style::default().fg(app.theme.muted_fg))
            .alignment(Alignment::Center);
        frame.render_widget(widget, area);
        return;
    };

    let mut lines = Vec::new();

    // 1. Title (highlighted bold)
    lines.push(Line::from(vec![
        Span::styled("Title   ", Style::default().fg(app.theme.muted_fg)),
        Span::styled(
            track.display_title(),
            Style::default()
                .fg(app.theme.title)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    // 2. Artist
    lines.push(Line::from(vec![
        Span::styled("Artist  ", Style::default().fg(app.theme.muted_fg)),
        Span::styled(track.display_artist(), Style::default().fg(app.theme.playing_fg)),
    ]));

    // 3. Album
    lines.push(Line::from(vec![
        Span::styled("Album   ", Style::default().fg(app.theme.muted_fg)),
        Span::styled(track.display_album(), Style::default().fg(app.theme.fg)),
    ]));

    // 4. Track # & Year
    let track_str = match (track.track_number, track.disc_number) {
        (Some(t), Some(d)) => format!("#{t} (d{d})"),
        (Some(t), None) => format!("#{t}"),
        _ => "---".into(),
    };
    let year_str = track
        .year
        .map(|y| y.to_string())
        .unwrap_or_else(|| "----".into());
    lines.push(Line::from(vec![
        Span::styled("No/Year ", Style::default().fg(app.theme.muted_fg)),
        Span::styled(format!("{track_str} • {year_str}"), Style::default().fg(app.theme.fg)),
    ]));

    // 5. Genre
    if let Some(ref genre) = track.genre {
        lines.push(Line::from(vec![
            Span::styled("Genre   ", Style::default().fg(app.theme.muted_fg)),
            Span::styled(genre.as_str(), Style::default().fg(app.theme.fg)),
        ]));
    }

    // 6. Length / Duration
    lines.push(Line::from(vec![
        Span::styled("Length  ", Style::default().fg(app.theme.muted_fg)),
        Span::styled(track.display_duration(), Style::default().fg(app.theme.fg)),
    ]));

    // 7. Format & Codec Specs
    let sr_str = track
        .sample_rate
        .map(|sr| format!("{:.1}kHz", sr as f64 / 1000.0))
        .unwrap_or_default();
    let ch_str = match track.channels {
        Some(1) => "Mono",
        Some(2) => "Stereo",
        Some(_) => "Multi",
        None => "",
    };
    let fmt_str = track.format.to_uppercase();
    let audio_spec = match (!sr_str.is_empty(), !ch_str.is_empty()) {
        (true, true) => format!("{fmt_str} • {sr_str} • {ch_str}"),
        (true, false) => format!("{fmt_str} • {sr_str}"),
        _ => fmt_str,
    };
    lines.push(Line::from(vec![
        Span::styled("Format  ", Style::default().fg(app.theme.muted_fg)),
        Span::styled(audio_spec, Style::default().fg(app.theme.fg)),
    ]));

    // 8. Bitrate
    if let Some(br) = track.bitrate {
        lines.push(Line::from(vec![
            Span::styled("Bitrate ", Style::default().fg(app.theme.muted_fg)),
            Span::styled(format!("{} kbps", br / 1000), Style::default().fg(app.theme.fg)),
        ]));
    }

    // 9. ReplayGain
    if let Some(rg) = track.replay_gain_track {
        lines.push(Line::from(vec![
            Span::styled("Gain    ", Style::default().fg(app.theme.muted_fg)),
            Span::styled(format!("{:+.2} dB", rg), Style::default().fg(app.theme.fg)),
        ]));
    }

    // 10. File name
    if let Some(file_name) = track.path.file_name().and_then(|f| f.to_str()) {
        lines.push(Line::from(vec![
            Span::styled("File    ", Style::default().fg(app.theme.muted_fg)),
            Span::styled(file_name, Style::default().fg(app.theme.muted_fg)),
        ]));
    }

    let widget = Paragraph::new(lines)
        .block(block)
        .scroll((app.metadata_scroll, 0))
        .wrap(Wrap { trim: true });

    frame.render_widget(widget, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::sync::Arc;
    use parking_lot::Mutex;
    use mpui_audio::player::PlaybackState;

    #[test]
    fn test_metadata_render_empty() {
        let backend = TestBackend::new(40, 15);
        let mut terminal = Terminal::new(backend).unwrap();

        let config = mpui_config::Config::default();
        let playback = Arc::new(Mutex::new(PlaybackState::default()));
        let (tx, _) = crossbeam_channel::unbounded();
        let (_, event_rx) = crossbeam_channel::unbounded();
        let spectrum = Arc::new(Mutex::new(Vec::new()));
        let app = App::new(config, playback, tx, spectrum, event_rx);

        terminal
            .draw(|f| {
                let area = f.area();
                render(f, area, &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        // Check for placeholder
        let content = buffer
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(content.contains("no media selected"));
    }

    #[test]
    fn test_metadata_render_with_track() {
        let backend = TestBackend::new(50, 18);
        let mut terminal = Terminal::new(backend).unwrap();

        let config = mpui_config::Config::default();
        let playback = Arc::new(Mutex::new(PlaybackState::default()));
        let (tx, _) = crossbeam_channel::unbounded();
        let (_, event_rx) = crossbeam_channel::unbounded();
        let spectrum = Arc::new(Mutex::new(Vec::new()));
        let mut app = App::new(config, playback, tx, spectrum, event_rx);

        let track = mpui_core::track::Track {
            id: mpui_core::track::TrackId(42),
            path: std::path::PathBuf::from("/music/test_song.flac"),
            title: Some("Stairway to Heaven".into()),
            artist: Some("Led Zeppelin".into()),
            album: Some("Led Zeppelin IV".into()),
            album_artist: None,
            genre: Some("Classic Rock".into()),
            year: Some(1971),
            track_number: Some(4),
            disc_number: Some(1),
            duration: Some(std::time::Duration::from_secs(482)),
            sample_rate: Some(96000),
            channels: Some(2),
            bitrate: Some(2840000),
            format: "flac".into(),
            replay_gain_track: Some(-4.5),
            replay_gain_album: Some(-3.2),
        };

        app.library.insert(track);
        app.library_cursor = 0;

        terminal
            .draw(|f| {
                let area = f.area();
                render(f, area, &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();

        assert!(content.contains("Stairway to Heaven"));
        assert!(content.contains("Led Zeppelin"));
        assert!(content.contains("FLAC"));
        assert!(content.contains("96.0kHz"));
        assert!(content.contains("Stereo"));
        assert!(content.contains("8:02"));
    }
}
