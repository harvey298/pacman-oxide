use bevy::prelude::*;

#[derive(Debug, Clone, Eq, PartialEq, Hash, States)]
pub enum GhostState {
    Chase,
    Scatter,
    
    /// Disables the ghost's AI
    Frozen,
}

impl Default for GhostState {
    fn default() -> Self {
        Self::Frozen
    }
}