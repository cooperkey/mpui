use mpui_config::types::RepeatMode;

use crate::track::TrackId;

/// Playback queue — ordered list of track IDs with a cursor.
#[derive(Debug, Default, Clone)]
pub struct Queue {
    items: Vec<TrackId>,
    current: Option<usize>,
    original_items: Option<Vec<TrackId>>,
}

impl Queue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a track to the end of the queue.
    pub fn push(&mut self, id: TrackId) {
        if let Some(ref mut orig) = self.original_items {
            orig.push(id);
        }
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
        if let Some(ref mut orig) = self.original_items {
            if let Some(orig_pos) = orig.iter().position(|&id| id == removed) {
                orig.remove(orig_pos);
            }
        }
        if self.items.is_empty() {
            self.current = None;
            self.original_items = None;
        } else if let Some(cur) = self.current {
            if index < cur || cur >= self.items.len() {
                self.current = Some(cur.saturating_sub(1));
            }
        }
        Some(removed)
    }

    /// Move cursor to the next track. Returns None if already at the end of the queue.
    pub fn next(&mut self) -> Option<TrackId> {
        let cur = self.current?;
        if cur + 1 < self.items.len() {
            self.current = Some(cur + 1);
            self.current_track()
        } else {
            None
        }
    }

    /// Move cursor to the previous track. Returns None if already at the start of the queue.
    pub fn prev(&mut self) -> Option<TrackId> {
        let cur = self.current?;
        if cur > 0 {
            self.current = Some(cur - 1);
            self.current_track()
        } else {
            None
        }
    }

    /// Advance playback cursor based on repeat mode. Returns the next track to play.
    pub fn advance(&mut self, repeat_mode: RepeatMode) -> Option<TrackId> {
        if self.items.is_empty() {
            return None;
        }

        match repeat_mode {
            RepeatMode::Track => self.current_track(),
            RepeatMode::Queue => {
                let cur = self.current.unwrap_or(0);
                if cur + 1 < self.items.len() {
                    self.current = Some(cur + 1);
                } else {
                    self.current = Some(0);
                }
                self.current_track()
            }
            RepeatMode::Off => {
                let cur = self.current.unwrap_or(0);
                if cur + 1 < self.items.len() {
                    self.current = Some(cur + 1);
                    self.current_track()
                } else {
                    None
                }
            }
        }
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

    pub fn is_shuffled(&self) -> bool {
        self.original_items.is_some()
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.original_items = None;
        self.current = None;
    }

    /// Non-destructively shuffle queue. Preserves current track at index 0.
    pub fn shuffle(&mut self) {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};

        if self.items.len() <= 1 {
            return;
        }

        if self.original_items.is_none() {
            self.original_items = Some(self.items.clone());
        }

        // Keep current track at index 0
        if let Some(cur) = self.current {
            self.items.swap(0, cur);
            self.current = Some(0);
        }

        let mut seed = {
            let mut h = RandomState::new().build_hasher();
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            h.write_u128(nanos);
            h.finish()
        };
        if seed == 0 {
            seed = 0x853c49e6748fea9b;
        }

        let mut next_rand = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };

        let start = if self.current.is_some() { 1 } else { 0 };
        for i in (start + 1..self.items.len()).rev() {
            let rand_val = (next_rand() as usize) % (i - start + 1);
            let j = start + rand_val;
            self.items.swap(i, j);
        }
    }

    /// Restore queue to pre-shuffled canonical order.
    pub fn unshuffle(&mut self) {
        if let Some(orig) = self.original_items.take() {
            if let Some(cur_track) = self.current_track() {
                self.current = orig.iter().position(|&id| id == cur_track);
            }
            self.items = orig;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_push_and_cursor() {
        let mut q = Queue::new();
        assert_eq!(q.current_index(), None);
        assert_eq!(q.current_track(), None);

        q.push(TrackId(10));
        assert_eq!(q.current_index(), Some(0));
        assert_eq!(q.current_track(), Some(TrackId(10)));

        q.push(TrackId(20));
        assert_eq!(q.current_index(), Some(0));
        assert_eq!(q.current_track(), Some(TrackId(10)));
        assert_eq!(q.len(), 2);
    }

    #[test]
    fn test_queue_boundary_next_prev() {
        let mut q = Queue::new();
        q.push(TrackId(1));
        q.push(TrackId(2));

        assert_eq!(q.current_track(), Some(TrackId(1)));
        assert_eq!(q.prev(), None);
        assert_eq!(q.current_track(), Some(TrackId(1)));

        assert_eq!(q.next(), Some(TrackId(2)));
        assert_eq!(q.current_track(), Some(TrackId(2)));

        // Boundary check: next at end must return None and NOT loop track 2
        assert_eq!(q.next(), None);
        assert_eq!(q.current_track(), Some(TrackId(2)));

        assert_eq!(q.prev(), Some(TrackId(1)));
        assert_eq!(q.current_track(), Some(TrackId(1)));
    }

    #[test]
    fn test_queue_advance_repeat_modes() {
        let mut q = Queue::new();
        q.push(TrackId(1));
        q.push(TrackId(2));

        // Advance with RepeatMode::Off
        assert_eq!(q.advance(RepeatMode::Off), Some(TrackId(2)));
        assert_eq!(q.advance(RepeatMode::Off), None);
        assert_eq!(q.current_track(), Some(TrackId(2)));

        // Advance with RepeatMode::Queue (should wrap to 0)
        assert_eq!(q.advance(RepeatMode::Queue), Some(TrackId(1)));
        assert_eq!(q.advance(RepeatMode::Queue), Some(TrackId(2)));
        assert_eq!(q.advance(RepeatMode::Queue), Some(TrackId(1)));

        // Advance with RepeatMode::Track (should stay on current track)
        assert_eq!(q.advance(RepeatMode::Track), Some(TrackId(1)));
        assert_eq!(q.advance(RepeatMode::Track), Some(TrackId(1)));
    }

    #[test]
    fn test_queue_remove_and_cursor_adjustment() {
        let mut q = Queue::new();
        q.push(TrackId(1));
        q.push(TrackId(2));
        q.push(TrackId(3));

        q.set_current(2);
        assert_eq!(q.current_track(), Some(TrackId(3)));

        // Remove item before current: cursor adjusts to stay on TrackId(3)
        assert_eq!(q.remove(0), Some(TrackId(1)));
        assert_eq!(q.current_index(), Some(1));
        assert_eq!(q.current_track(), Some(TrackId(3)));

        // Remove current item: cursor clamps
        assert_eq!(q.remove(1), Some(TrackId(3)));
        assert_eq!(q.current_index(), Some(0));
        assert_eq!(q.current_track(), Some(TrackId(2)));

        // Remove last remaining
        assert_eq!(q.remove(0), Some(TrackId(2)));
        assert_eq!(q.current_index(), None);
        assert_eq!(q.current_track(), None);
        assert!(q.is_empty());
    }

    #[test]
    fn test_non_destructive_shuffle() {
        let mut q = Queue::new();
        for i in 1..=10 {
            q.push(TrackId(i));
        }
        q.set_current(3); // TrackId(4)

        assert!(!q.is_shuffled());
        q.shuffle();
        assert!(q.is_shuffled());
        assert_eq!(q.current_track(), Some(TrackId(4)));
        assert_eq!(q.current_index(), Some(0));
        assert_eq!(q.len(), 10);

        // Unshuffle restores exact original order and cursor
        q.unshuffle();
        assert!(!q.is_shuffled());
        assert_eq!(q.current_track(), Some(TrackId(4)));
        assert_eq!(q.current_index(), Some(3));
        for (idx, &item) in q.items().iter().enumerate() {
            assert_eq!(item, TrackId((idx + 1) as u64));
        }
    }
}
