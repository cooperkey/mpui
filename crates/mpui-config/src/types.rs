//! Re-exported types used across crate boundaries.
//!
//! Kept in a separate module to avoid circular dependency on the full config struct
//! when only type definitions are needed.

use serde::{Deserialize, Serialize};

/// Playback repeat mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RepeatMode {
    #[default]
    Off,
    Track,
    Queue,
}

impl RepeatMode {
    pub fn cycle(self) -> Self {
        match self {
            Self::Off => Self::Track,
            Self::Track => Self::Queue,
            Self::Queue => Self::Off,
        }
    }
}

/// Volume as a clamped u8 (0..=100).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Volume(u8);

impl Volume {
    pub fn new(v: u8) -> Self {
        Self(v.min(100))
    }

    pub fn get(self) -> u8 {
        self.0
    }

    pub fn as_f32(self) -> f32 {
        self.0 as f32 / 100.0
    }

    #[allow(clippy::should_implement_trait)]
    pub fn add(self, step: u8) -> Self {
        Self::new(self.0.saturating_add(step))
    }

    #[allow(clippy::should_implement_trait)]
    pub fn sub(self, step: u8) -> Self {
        Self::new(self.0.saturating_sub(step))
    }
}

impl Default for Volume {
    fn default() -> Self {
        Self(80)
    }
}
