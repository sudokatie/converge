//! Chunk lifecycle state machine.

/// State of a chunk in its lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChunkState {
    /// Not loaded - no data in memory.
    Unloaded,
    /// Currently generating terrain.
    Generating,
    /// Terrain generated, awaiting mesh.
    Generated,
    /// Currently building mesh.
    Meshing,
    /// Ready to render.
    Ready,
    /// Modified, needs remesh.
    Dirty,
}

impl ChunkState {
    /// Check if the chunk has block data available.
    #[must_use]
    pub fn has_data(self) -> bool {
        matches!(
            self,
            ChunkState::Generated
                | ChunkState::Meshing
                | ChunkState::Ready
                | ChunkState::Dirty
        )
    }

    /// Check if the chunk can be rendered.
    #[must_use]
    pub fn is_renderable(self) -> bool {
        matches!(self, ChunkState::Ready | ChunkState::Dirty)
    }

    /// Check if the chunk needs work (generation or meshing).
    #[must_use]
    pub fn needs_work(self) -> bool {
        matches!(
            self,
            ChunkState::Unloaded | ChunkState::Generated | ChunkState::Dirty
        )
    }

    /// Get the next state after generation completes.
    #[must_use]
    pub fn after_generation(self) -> Self {
        match self {
            ChunkState::Generating => ChunkState::Generated,
            _ => self,
        }
    }

    /// Get the next state after meshing completes.
    #[must_use]
    pub fn after_meshing(self) -> Self {
        match self {
            ChunkState::Meshing => ChunkState::Ready,
            _ => self,
        }
    }

    /// Mark the chunk as dirty (needs remesh).
    #[must_use]
    pub fn mark_dirty(self) -> Self {
        if self.has_data() {
            ChunkState::Dirty
        } else {
            self
        }
    }

    /// Start generation if unloaded.
    #[must_use]
    pub fn start_generation(self) -> Self {
        match self {
            ChunkState::Unloaded => ChunkState::Generating,
            _ => self,
        }
    }

    /// Start meshing if generated or dirty.
    #[must_use]
    pub fn start_meshing(self) -> Self {
        match self {
            ChunkState::Generated | ChunkState::Dirty => ChunkState::Meshing,
            _ => self,
        }
    }
}

impl Default for ChunkState {
    fn default() -> Self {
        ChunkState::Unloaded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unloaded_has_no_data() {
        assert!(!ChunkState::Unloaded.has_data());
        assert!(!ChunkState::Generating.has_data());
    }

    #[test]
    fn test_generated_has_data() {
        assert!(ChunkState::Generated.has_data());
        assert!(ChunkState::Ready.has_data());
        assert!(ChunkState::Dirty.has_data());
    }

    #[test]
    fn test_only_ready_and_dirty_renderable() {
        assert!(!ChunkState::Unloaded.is_renderable());
        assert!(!ChunkState::Generating.is_renderable());
        assert!(!ChunkState::Generated.is_renderable());
        assert!(!ChunkState::Meshing.is_renderable());
        assert!(ChunkState::Ready.is_renderable());
        assert!(ChunkState::Dirty.is_renderable());
    }

    #[test]
    fn test_state_transitions() {
        let state = ChunkState::Unloaded;

        // Unloaded -> Generating
        let state = state.start_generation();
        assert_eq!(state, ChunkState::Generating);

        // Generating -> Generated
        let state = state.after_generation();
        assert_eq!(state, ChunkState::Generated);

        // Generated -> Meshing
        let state = state.start_meshing();
        assert_eq!(state, ChunkState::Meshing);

        // Meshing -> Ready
        let state = state.after_meshing();
        assert_eq!(state, ChunkState::Ready);

        // Ready -> Dirty
        let state = state.mark_dirty();
        assert_eq!(state, ChunkState::Dirty);

        // Dirty -> Meshing
        let state = state.start_meshing();
        assert_eq!(state, ChunkState::Meshing);
    }

    #[test]
    fn test_needs_work() {
        assert!(ChunkState::Unloaded.needs_work());
        assert!(!ChunkState::Generating.needs_work());
        assert!(ChunkState::Generated.needs_work());
        assert!(!ChunkState::Meshing.needs_work());
        assert!(!ChunkState::Ready.needs_work());
        assert!(ChunkState::Dirty.needs_work());
    }
}
