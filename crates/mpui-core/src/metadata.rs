use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

use anyhow::{Context, Result};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::track::{Track, TrackId};

/// Extract metadata from an audio file using symphonia.
pub fn read_metadata(path: &Path) -> Result<Track> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("failed to open {}", path.display()))?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let mut probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;

    let mut title = None;
    let mut artist = None;
    let mut album = None;
    let mut album_artist = None;
    let mut genre = None;
    let mut year = None;
    let mut track_number = None;
    let mut disc_number = None;

    // Read metadata from the format reader's metadata store.
    if let Some(metadata) = probed.format.metadata().current() {
        for tag in metadata.tags() {
            if let Some(std_key) = tag.std_key {
                use symphonia::core::meta::StandardTagKey::*;
                match std_key {
                    TrackTitle => title = Some(tag.value.to_string()),
                    Artist => artist = Some(tag.value.to_string()),
                    Album => album = Some(tag.value.to_string()),
                    AlbumArtist => album_artist = Some(tag.value.to_string()),
                    Genre => genre = Some(tag.value.to_string()),
                    Date => {
                        year = tag.value.to_string().get(..4).and_then(|y| y.parse().ok());
                    }
                    TrackNumber => {
                        track_number = tag.value.to_string().parse().ok();
                    }
                    DiscNumber => {
                        disc_number = tag.value.to_string().parse().ok();
                    }
                    _ => {}
                }
            }
        }
    }

    // Extract codec info from the default track.
    let (duration, sample_rate, channels, bitrate, format) = match probed.format.default_track() {
        Some(track) => {
            let params = &track.codec_params;
            let sr = params.sample_rate;
            let ch = params.channels.map(|c| c.count() as u8);
            let dur = params.n_frames.and_then(|frames| {
                sr.map(|rate| std::time::Duration::from_secs_f64(frames as f64 / rate as f64))
            });
            let br = params.bits_per_coded_sample;
            let fmt = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown")
                .to_lowercase();
            (dur, sr, ch, br, fmt)
        }
        None => (None, None, None, None, "unknown".into()),
    };

    let id = {
        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        TrackId(hasher.finish())
    };

    Ok(Track {
        id,
        path: path.to_path_buf(),
        title,
        artist,
        album,
        album_artist,
        genre,
        year,
        track_number,
        disc_number,
        duration,
        sample_rate,
        channels,
        bitrate,
        format,
    })
}

/// Extract cover art from an audio file or its directory.
pub fn extract_cover_art(path: &Path) -> Option<Vec<u8>> {
    let file = std::fs::File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    if let Ok(mut probed) = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    ) {
        if let Some(metadata) = probed.format.metadata().current() {
            if let Some(visual) = metadata.visuals().first() {
                return Some(visual.data.to_vec());
            }
        }

        if let Some(metadata) = probed.metadata.get().as_ref().and_then(|m| m.current()) {
            if let Some(visual) = metadata.visuals().first() {
                return Some(visual.data.to_vec());
            }
        }
    }

    // Fallback to directory images
    if let Some(parent) = path.parent() {
        let candidates = ["cover.jpg", "cover.png", "folder.jpg", "folder.png"];
        for c in candidates {
            let p = parent.join(c);
            if p.exists() {
                if let Ok(data) = std::fs::read(&p) {
                    return Some(data);
                }
            }
        }
    }

    None
}

/// Extract synchronized or plain lyrics from sidecar .lrc files or embedded tags.
pub fn extract_lyrics(path: &Path) -> Option<Vec<(std::time::Duration, String)>> {
    // 1. Sidecar .lrc file check
    let lrc_path = path.with_extension("lrc");
    if lrc_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&lrc_path) {
            let parsed = parse_lrc(&content);
            if !parsed.is_empty() {
                return Some(parsed);
            }
        }
    }

    // 2. Embedded lyrics check
    if let Ok(file) = std::fs::File::open(path) {
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        if let Ok(mut probed) = symphonia::default::get_probe().format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        ) {
            let mut raw_lyrics = None;
            if let Some(metadata) = probed.format.metadata().current() {
                for tag in metadata.tags() {
                    let key = tag.key.to_uppercase();
                    if key.contains("LYRIC") || key.contains("USLT") {
                        raw_lyrics = Some(tag.value.to_string());
                        break;
                    }
                }
            }

            if let Some(text) = raw_lyrics {
                let parsed = parse_lrc(&text);
                if !parsed.is_empty() {
                    return Some(parsed);
                }
                // Plain unsynced text fallback
                let mut lines = Vec::new();
                for (idx, line) in text.lines().enumerate() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        lines.push((std::time::Duration::from_secs((idx * 3) as u64), trimmed.to_string()));
                    }
                }
                if !lines.is_empty() {
                    return Some(lines);
                }
            }
        }
    }

    None
}

fn parse_lrc(content: &str) -> Vec<(std::time::Duration, String)> {
    let mut entries = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            if let Some(end_bracket) = line.find(']') {
                let time_str = &line[1..end_bracket];
                let text = line[end_bracket + 1..].trim();

                // Format: mm:ss.xx or mm:ss
                if let Some(colon) = time_str.find(':') {
                    let mins_str = &time_str[..colon];
                    let secs_str = &time_str[colon + 1..];

                    if let (Ok(mins), Ok(secs)) = (mins_str.parse::<f64>(), secs_str.parse::<f64>()) {
                        let total_secs = mins * 60.0 + secs;
                        if total_secs >= 0.0 {
                            entries.push((std::time::Duration::from_secs_f64(total_secs), text.to_string()));
                        }
                    }
                }
            }
        }
    }
    entries.sort_by_key(|e| e.0);
    entries
}
