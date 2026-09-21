use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use walkdir::WalkDir;

use crate::metadata;
use crate::track::{Track, TrackId};

use serde::{Deserialize, Serialize};

/// Known audio file extensions.
const AUDIO_EXTENSIONS: &[&str] = &[
    "flac", "mp3", "ogg", "opus", "wav", "aac", "m4a", "wma",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    Artist,
    Album,
    Title,
    Duration,
    Year,
}

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    track: Track,
    mtime: u64,
}

#[derive(Serialize, Deserialize, Default)]
struct LibraryCache {
    entries: HashMap<PathBuf, CacheEntry>,
}

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
        self.scan_with_cache(directories, follow_symlinks, scan_hidden, None);
    }

    /// Scan configured directories with incremental cache lookup.
    pub fn scan_with_cache(
        &mut self,
        directories: &[PathBuf],
        follow_symlinks: bool,
        scan_hidden: bool,
        cache_path: Option<&Path>,
    ) {
        self.tracks.clear();
        self.order.clear();

        // Load existing cache if available
        let mut cache: LibraryCache = cache_path
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|data| bincode::deserialize(&data).ok())
            .unwrap_or_default();

        let mut updated_cache = LibraryCache::default();

        for raw_dir in directories {
            let dir = mpui_config::expand_tilde(raw_dir);
            if !dir.exists() {
                warn!("library directory does not exist: {}", dir.display());
                continue;
            }

            let walker = WalkDir::new(&dir)
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

                let path = entry.path().to_path_buf();
                let mtime = entry
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                let track = if let Some(cached) = cache.entries.remove(&path) {
                    if cached.mtime == mtime && mtime != 0 {
                        cached.track
                    } else {
                        match metadata::read_metadata(&path) {
                            Ok(t) => t,
                            Err(e) => {
                                warn!("failed to read metadata for {}: {e}", path.display());
                                continue;
                            }
                        }
                    }
                } else {
                    match metadata::read_metadata(&path) {
                        Ok(t) => t,
                        Err(e) => {
                            warn!("failed to read metadata for {}: {e}", path.display());
                            continue;
                        }
                    }
                };

                let id = track.id;
                updated_cache.entries.insert(path, CacheEntry { track: track.clone(), mtime });
                self.tracks.insert(id, track);
                self.order.push(id);
            }
        }

        // Save updated cache back to disk
        if let Some(cp) = cache_path {
            if let Some(parent) = cp.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(data) = bincode::serialize(&updated_cache) {
                let _ = std::fs::write(cp, data);
            }
        }

        info!("library scan complete: {} tracks indexed", self.tracks.len());
    }

    /// Sort current library index by specified metadata field.
    pub fn sort_by(&mut self, field: SortField) {
        self.order.sort_by(|&a, &b| {
            let ta = self.tracks.get(&a);
            let tb = self.tracks.get(&b);
            match (ta, tb) {
                (Some(a), Some(b)) => match field {
                    SortField::Artist => a.display_artist().to_lowercase().cmp(&b.display_artist().to_lowercase()),
                    SortField::Album => a.display_album().to_lowercase().cmp(&b.display_album().to_lowercase()),
                    SortField::Title => a.display_title().to_lowercase().cmp(&b.display_title().to_lowercase()),
                    SortField::Duration => a.duration.cmp(&b.duration),
                    SortField::Year => a.year.cmp(&b.year),
                },
                _ => std::cmp::Ordering::Equal,
            }
        });
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_with_tilde() {
        let mut lib = Library::new();
        lib.scan(&[PathBuf::from("~/Downloads")], true, false);
        // Verify scanning with tilde expansion executes without panicking
    }
}
