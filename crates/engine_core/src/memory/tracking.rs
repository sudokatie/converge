//! Memory usage tracking.

use std::sync::atomic::{AtomicUsize, Ordering};

/// Global memory tracker for monitoring allocations.
///
/// Tracks allocations by category for debugging and profiling.
pub struct MemoryTracker {
    /// Bytes allocated for chunk data.
    pub chunks: AtomicUsize,
    /// Bytes allocated for mesh data.
    pub meshes: AtomicUsize,
    /// Bytes allocated for textures.
    pub textures: AtomicUsize,
    /// Bytes allocated for audio.
    pub audio: AtomicUsize,
    /// Other/miscellaneous allocations.
    pub other: AtomicUsize,
}

impl MemoryTracker {
    /// Create a new memory tracker with all counts at zero.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            chunks: AtomicUsize::new(0),
            meshes: AtomicUsize::new(0),
            textures: AtomicUsize::new(0),
            audio: AtomicUsize::new(0),
            other: AtomicUsize::new(0),
        }
    }

    /// Add bytes to a category.
    pub fn add(&self, category: MemoryCategory, bytes: usize) {
        self.counter(category).fetch_add(bytes, Ordering::Relaxed);
    }

    /// Subtract bytes from a category.
    pub fn sub(&self, category: MemoryCategory, bytes: usize) {
        self.counter(category).fetch_sub(bytes, Ordering::Relaxed);
    }

    /// Get the current count for a category.
    #[must_use]
    pub fn get(&self, category: MemoryCategory) -> usize {
        self.counter(category).load(Ordering::Relaxed)
    }

    /// Get the total bytes across all categories.
    #[must_use]
    pub fn total(&self) -> usize {
        self.chunks.load(Ordering::Relaxed)
            + self.meshes.load(Ordering::Relaxed)
            + self.textures.load(Ordering::Relaxed)
            + self.audio.load(Ordering::Relaxed)
            + self.other.load(Ordering::Relaxed)
    }

    /// Reset all counters to zero.
    pub fn reset(&self) {
        self.chunks.store(0, Ordering::Relaxed);
        self.meshes.store(0, Ordering::Relaxed);
        self.textures.store(0, Ordering::Relaxed);
        self.audio.store(0, Ordering::Relaxed);
        self.other.store(0, Ordering::Relaxed);
    }

    fn counter(&self, category: MemoryCategory) -> &AtomicUsize {
        match category {
            MemoryCategory::Chunks => &self.chunks,
            MemoryCategory::Meshes => &self.meshes,
            MemoryCategory::Textures => &self.textures,
            MemoryCategory::Audio => &self.audio,
            MemoryCategory::Other => &self.other,
        }
    }
}

impl Default for MemoryTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Categories for memory tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryCategory {
    /// Voxel chunk data.
    Chunks,
    /// Mesh vertex/index data.
    Meshes,
    /// Texture data.
    Textures,
    /// Audio samples.
    Audio,
    /// Everything else.
    Other,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_get() {
        let tracker = MemoryTracker::new();
        tracker.add(MemoryCategory::Chunks, 1024);
        assert_eq!(tracker.get(MemoryCategory::Chunks), 1024);
    }

    #[test]
    fn test_sub() {
        let tracker = MemoryTracker::new();
        tracker.add(MemoryCategory::Meshes, 1000);
        tracker.sub(MemoryCategory::Meshes, 400);
        assert_eq!(tracker.get(MemoryCategory::Meshes), 600);
    }

    #[test]
    fn test_total() {
        let tracker = MemoryTracker::new();
        tracker.add(MemoryCategory::Chunks, 100);
        tracker.add(MemoryCategory::Meshes, 200);
        tracker.add(MemoryCategory::Textures, 300);
        assert_eq!(tracker.total(), 600);
    }

    #[test]
    fn test_reset() {
        let tracker = MemoryTracker::new();
        tracker.add(MemoryCategory::Audio, 500);
        tracker.reset();
        assert_eq!(tracker.total(), 0);
    }
}
