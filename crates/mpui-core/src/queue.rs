use crate::track::TrackId;

/// Playback queue — ordered list of track IDs with a cursor.
#[derive(Debug, Default)]
pub struct Queue {
    items: Vec<TrackId>,
    current: Option<usize>,
}

impl Queue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a track to the end of the queue.
    pub fn push(&mut self, id: TrackId) {
        self.items.push(id);
        if self.current.is_none() {
            self.current = Some(0);
        }
    }

    /// Remove a track at the given index.
    pub fn remove(&mut self, index: usize) -> Option<TrackId> {
        if index >= self.items.len() {
            return None;
        }
        let removed = self.items.remove(index);
        if self.items.is_empty() {
            self.current = None;
        } else if let Some(cur) = self.current {
            if index < cur || cur >= self.items.len() {
                self.current = Some(cur.saturating_sub(1));
            }
        }
        Some(removed)
    }

    /// Move cursor to the next track. Returns the new current track ID.
    pub fn next(&mut self) -> Option<TrackId> {
        let cur = self.current?;
        if cur + 1 < self.items.len() {
            self.current = Some(cur + 1);
        }
        self.current_track()
    }

    /// Move cursor to the previous track. Returns the new current track ID.
    pub fn prev(&mut self) -> Option<TrackId> {
        let cur = self.current?;
        if cur > 0 {
            self.current = Some(cur - 1);
        }
        self.current_track()
    }

    /// Returns the currently selected track ID.
    pub fn current_track(&self) -> Option<TrackId> {
        self.current.and_then(|i| self.items.get(i).copied())
    }

    /// Returns the current cursor index.
    pub fn current_index(&self) -> Option<usize> {
        self.current
    }

    /// Set cursor to a specific index.
    pub fn set_current(&mut self, index: usize) {
        if index < self.items.len() {
            self.current = Some(index);
        }
    }

    pub fn items(&self) -> &[TrackId] {
        &self.items
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.current = None;
    }

    /// Shuffle the queue using fisher-yates. Keeps current track at index 0.
    pub fn shuffle(&mut self) {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};

        if self.items.len() <= 1 {
            return;
        }

        // Move current track to front if it exists.
        if let Some(cur) = self.current {
            self.items.swap(0, cur);
            self.current = Some(0);
        }

        let state = RandomState::new();
        let start = if self.current.is_some() { 1 } else { 0 };
        for i in (start + 1..self.items.len()).rev() {
            let mut hasher = state.build_hasher();
            hasher.write_usize(i);
            let j = start + (hasher.finish() as usize % (i - start + 1));
            self.items.swap(i, j);
        }
    }
}
