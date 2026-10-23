use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Unique identifier for a track within the library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TrackId(pub u64);

/// Core track metadata extracted from audio file tags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: TrackId,
    pub path: PathBuf,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub genre: Option<String>,
    pub year: Option<u32>,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub duration: Option<Duration>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u8>,
    pub bitrate: Option<u32>,
    pub format: String,
    pub replay_gain_track: Option<f32>,
    pub replay_gain_album: Option<f32>,
}

impl Track {
    /// Returns the display title, falling back to filename stem.
    pub fn display_title(&self) -> &str {
        self.title
            .as_deref()
            .unwrap_or_else(|| {
                self.path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
            })
    }

    /// Returns the display artist, falling back to "unknown".
    pub fn display_artist(&self) -> &str {
        self.artist.as_deref().unwrap_or("unknown")
    }

    /// Returns the display album, falling back to "unknown".
    pub fn display_album(&self) -> &str {
        self.album.as_deref().unwrap_or("unknown")
    }

    /// Formatted duration as mm:ss or hh:mm:ss.
    pub fn display_duration(&self) -> String {
        match self.duration {
            Some(d) => {
                let total_secs = d.as_secs();
                let hours = total_secs / 3600;
                let mins = (total_secs % 3600) / 60;
                let secs = total_secs % 60;
                if hours > 0 {
                    format!("{hours}:{mins:02}:{secs:02}")
                } else {
                    format!("{mins}:{secs:02}")
                }
            }
            None => "--:--".into(),
        }
    }
}
