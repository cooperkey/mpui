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
