use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use walkdir::WalkDir;

use crate::metadata;
use crate::track::{Track, TrackId};

/// Known audio file extensions.
const AUDIO_EXTENSIONS: &[&str] = &[
    "flac", "mp3", "ogg", "opus", "wav", "aac", "m4a", "wma",
];

/// In-memory library index.
#[derive(Debug, Default)]
pub struct Library {
    tracks: HashMap<TrackId, Track>,
    /// Tracks ordered by insertion (scan order).
    order: Vec<TrackId>,
}

impl Library {
    pub fn new() -> Self {
        Self::default()
    }

    /// Scan configured directories, extract metadata, populate index.
    pub fn scan(&mut self, directories: &[PathBuf], follow_symlinks: bool, scan_hidden: bool) {
        self.tracks.clear();
        self.order.clear();

        for dir in directories {
            if !dir.exists() {
                warn!("library directory does not exist: {}", dir.display());
                continue;
            }

            let walker = WalkDir::new(dir)
                .follow_links(follow_symlinks)
                .into_iter()
                .filter_entry(|e| {
                    if !scan_hidden {
                        e.file_name()
                            .to_str()
                            .map(|s| !s.starts_with('.'))
                            .unwrap_or(true)
                    } else {
                        true
                    }
                });

            for entry in walker.filter_map(|e| e.ok()) {
                if !entry.file_type().is_file() {
                    continue;
                }
                if !is_audio_file(entry.path()) {
                    continue;
                }
                match metadata::read_metadata(entry.path()) {
                    Ok(track) => {
                        let id = track.id;
                        self.tracks.insert(id, track);
                        self.order.push(id);
                    }
                    Err(e) => {
                        warn!("failed to read metadata for {}: {e}", entry.path().display());
                    }
                }
            }
        }

        info!("library scan complete: {} tracks indexed", self.tracks.len());
    }

    pub fn get(&self, id: TrackId) -> Option<&Track> {
        self.tracks.get(&id)
    }

    pub fn tracks(&self) -> impl Iterator<Item = &Track> {
        self.order.iter().filter_map(|id| self.tracks.get(id))
    }

    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    pub fn track_ids(&self) -> &[TrackId] {
        &self.order
    }
}

fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| AUDIO_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}
