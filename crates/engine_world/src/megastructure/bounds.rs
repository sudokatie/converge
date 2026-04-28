//! Multi-chunk bounding volume for megastructures.

use std::collections::BTreeSet;

use engine_core::coords::ChunkPos;
use glam::IVec3;
use serde::{Deserialize, Serialize};

/// Axis-aligned bounding box in chunk coordinates.
///
/// Defines the rectangular volume containing all chunks of a megastructure.
/// Uses inclusive min/max bounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkBounds {
    /// Minimum corner (inclusive).
    min: IVec3,
    /// Maximum corner (inclusive).
    max: IVec3,
}

impl ChunkBounds {
    /// Create bounds from min/max corners.
    ///
    /// Automatically normalizes so min <= max on all axes.
    #[must_use]
    pub fn new(min: IVec3, max: IVec3) -> Self {
        Self {
            min: min.min(max),
            max: min.max(max),
        }
    }

    /// Create bounds from a single chunk.
    #[must_use]
    pub fn from_chunk(pos: ChunkPos) -> Self {
        let p = IVec3::from(pos);
        Self { min: p, max: p }
    }

    /// Create bounds containing all given chunks.
    ///
    /// Returns `None` if the iterator is empty.
    pub fn from_chunks(chunks: impl IntoIterator<Item = ChunkPos>) -> Option<Self> {
        let mut iter = chunks.into_iter();
        let first = iter.next()?;
        let first_vec = IVec3::from(first);

        let mut bounds = Self {
            min: first_vec,
            max: first_vec,
        };

        for chunk in iter {
            bounds = bounds.expanded_to(chunk);
        }

        Some(bounds)
    }

    /// Get the minimum corner.
    #[must_use]
    pub const fn min(&self) -> IVec3 {
        self.min
    }

    /// Get the maximum corner.
    #[must_use]
    pub const fn max(&self) -> IVec3 {
        self.max
    }

    /// Get the size in chunks along each axis.
    #[must_use]
    pub fn size(&self) -> IVec3 {
        self.max - self.min + IVec3::ONE
    }

    /// Get the total number of chunks in the bounding volume.
    #[must_use]
    #[expect(
        clippy::cast_sign_loss,
        reason = "size components are always positive after normalization"
    )]
    pub fn chunk_count(&self) -> usize {
        let s = self.size();
        (s.x * s.y * s.z) as usize
    }

    /// Check if a chunk position is within bounds.
    #[must_use]
    pub fn contains(&self, pos: ChunkPos) -> bool {
        let p = IVec3::from(pos);
        p.x >= self.min.x
            && p.x <= self.max.x
            && p.y >= self.min.y
            && p.y <= self.max.y
            && p.z >= self.min.z
            && p.z <= self.max.z
    }

    /// Return new bounds expanded to include the given chunk.
    #[must_use]
    pub fn expanded_to(self, pos: ChunkPos) -> Self {
        let p = IVec3::from(pos);
        Self {
            min: self.min.min(p),
            max: self.max.max(p),
        }
    }

    /// Return the union of two bounds.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// Return the intersection of two bounds, if they overlap.
    #[must_use]
    pub fn intersection(self, other: Self) -> Option<Self> {
        let min = self.min.max(other.min);
        let max = self.max.min(other.max);

        if min.x <= max.x && min.y <= max.y && min.z <= max.z {
            Some(Self { min, max })
        } else {
            None
        }
    }

    /// Check if two bounds overlap.
    #[must_use]
    pub fn intersects(&self, other: &Self) -> bool {
        self.intersection(*other).is_some()
    }

    /// Iterate over all chunk positions in the bounds.
    ///
    /// Iteration order is deterministic: X, then Z, then Y (layer by layer).
    pub fn iter_chunks(&self) -> impl Iterator<Item = ChunkPos> + '_ {
        let min = self.min;
        let max = self.max;

        (min.y..=max.y).flat_map(move |y| {
            (min.z..=max.z).flat_map(move |z| (min.x..=max.x).map(move |x| ChunkPos::new(x, y, z)))
        })
    }

    /// Get chunks on the boundary (surface) of the bounds.
    pub fn iter_boundary(&self) -> impl Iterator<Item = ChunkPos> + '_ {
        self.iter_chunks().filter(|&pos| {
            let p = IVec3::from(pos);
            p.x == self.min.x
                || p.x == self.max.x
                || p.y == self.min.y
                || p.y == self.max.y
                || p.z == self.min.z
                || p.z == self.max.z
        })
    }

    /// Translate bounds by an offset.
    #[must_use]
    pub fn translated(self, offset: IVec3) -> Self {
        Self {
            min: self.min + offset,
            max: self.max + offset,
        }
    }

    /// Expand bounds by a margin on all sides.
    #[must_use]
    pub fn padded(self, margin: i32) -> Self {
        Self {
            min: self.min - IVec3::splat(margin),
            max: self.max + IVec3::splat(margin),
        }
    }

    /// Get the center chunk position (rounded down).
    #[must_use]
    pub fn center(&self) -> ChunkPos {
        let c = (self.min + self.max) / 2;
        ChunkPos::new(c.x, c.y, c.z)
    }
}

impl Default for ChunkBounds {
    fn default() -> Self {
        Self::from_chunk(ChunkPos::new(0, 0, 0))
    }
}

/// Sparse set of chunk positions belonging to a megastructure.
///
/// Uses `BTreeSet` for deterministic iteration order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkMask {
    chunks: BTreeSet<(i32, i32, i32)>,
    bounds: Option<ChunkBounds>,
}

impl ChunkMask {
    /// Create a new empty mask.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a mask with initial capacity hint.
    #[must_use]
    pub fn with_capacity(_capacity: usize) -> Self {
        Self::new()
    }

    /// Add a chunk to the mask.
    ///
    /// Returns `true` if the chunk was newly added.
    pub fn insert(&mut self, pos: ChunkPos) -> bool {
        let key = (pos.x(), pos.y(), pos.z());
        let added = self.chunks.insert(key);
        if added {
            self.bounds = Some(match self.bounds {
                Some(b) => b.expanded_to(pos),
                None => ChunkBounds::from_chunk(pos),
            });
        }
        added
    }

    /// Remove a chunk from the mask.
    ///
    /// Returns `true` if the chunk was present.
    pub fn remove(&mut self, pos: ChunkPos) -> bool {
        let key = (pos.x(), pos.y(), pos.z());
        let removed = self.chunks.remove(&key);
        if removed && self.chunks.is_empty() {
            self.bounds = None;
        } else if removed {
            self.recalculate_bounds();
        }
        removed
    }

    /// Check if a chunk is in the mask.
    #[must_use]
    pub fn contains(&self, pos: ChunkPos) -> bool {
        self.chunks.contains(&(pos.x(), pos.y(), pos.z()))
    }

    /// Get the number of chunks in the mask.
    #[must_use]
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// Check if the mask is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Get the bounding box of all chunks.
    #[must_use]
    pub fn bounds(&self) -> Option<ChunkBounds> {
        self.bounds
    }

    /// Iterate over chunks in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = ChunkPos> + '_ {
        self.chunks.iter().map(|&(x, y, z)| ChunkPos::new(x, y, z))
    }

    /// Clear all chunks.
    pub fn clear(&mut self) {
        self.chunks.clear();
        self.bounds = None;
    }

    /// Recalculate bounds from scratch.
    fn recalculate_bounds(&mut self) {
        self.bounds = ChunkBounds::from_chunks(self.iter());
    }

    /// Create a filled mask from bounds.
    #[must_use]
    pub fn from_bounds(bounds: ChunkBounds) -> Self {
        let mut mask = Self::new();
        for chunk in bounds.iter_chunks() {
            mask.insert(chunk);
        }
        mask
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounds_new_normalizes() {
        let bounds = ChunkBounds::new(IVec3::new(5, 5, 5), IVec3::new(0, 0, 0));
        assert_eq!(bounds.min(), IVec3::new(0, 0, 0));
        assert_eq!(bounds.max(), IVec3::new(5, 5, 5));
    }

    #[test]
    fn test_bounds_size() {
        let bounds = ChunkBounds::new(IVec3::new(0, 0, 0), IVec3::new(2, 3, 4));
        assert_eq!(bounds.size(), IVec3::new(3, 4, 5));
    }

    #[test]
    fn test_bounds_chunk_count() {
        let bounds = ChunkBounds::new(IVec3::new(0, 0, 0), IVec3::new(1, 1, 1));
        assert_eq!(bounds.chunk_count(), 8);
    }

    #[test]
    fn test_bounds_contains() {
        let bounds = ChunkBounds::new(IVec3::new(0, 0, 0), IVec3::new(2, 2, 2));
        assert!(bounds.contains(ChunkPos::new(1, 1, 1)));
        assert!(bounds.contains(ChunkPos::new(0, 0, 0)));
        assert!(bounds.contains(ChunkPos::new(2, 2, 2)));
        assert!(!bounds.contains(ChunkPos::new(3, 1, 1)));
        assert!(!bounds.contains(ChunkPos::new(-1, 1, 1)));
    }

    #[test]
    fn test_bounds_from_chunks() {
        let chunks = [
            ChunkPos::new(1, 2, 3),
            ChunkPos::new(-1, 0, 5),
            ChunkPos::new(3, 1, 1),
        ];
        let bounds = ChunkBounds::from_chunks(chunks).unwrap();
        assert_eq!(bounds.min(), IVec3::new(-1, 0, 1));
        assert_eq!(bounds.max(), IVec3::new(3, 2, 5));
    }

    #[test]
    fn test_bounds_from_chunks_empty() {
        let bounds = ChunkBounds::from_chunks(std::iter::empty());
        assert!(bounds.is_none());
    }

    #[test]
    fn test_bounds_union() {
        let a = ChunkBounds::new(IVec3::new(0, 0, 0), IVec3::new(2, 2, 2));
        let b = ChunkBounds::new(IVec3::new(1, 1, 1), IVec3::new(4, 4, 4));
        let union = a.union(b);
        assert_eq!(union.min(), IVec3::new(0, 0, 0));
        assert_eq!(union.max(), IVec3::new(4, 4, 4));
    }

    #[test]
    fn test_bounds_intersection() {
        let a = ChunkBounds::new(IVec3::new(0, 0, 0), IVec3::new(3, 3, 3));
        let b = ChunkBounds::new(IVec3::new(2, 2, 2), IVec3::new(5, 5, 5));
        let intersection = a.intersection(b).unwrap();
        assert_eq!(intersection.min(), IVec3::new(2, 2, 2));
        assert_eq!(intersection.max(), IVec3::new(3, 3, 3));
    }

    #[test]
    fn test_bounds_no_intersection() {
        let a = ChunkBounds::new(IVec3::new(0, 0, 0), IVec3::new(1, 1, 1));
        let b = ChunkBounds::new(IVec3::new(5, 5, 5), IVec3::new(6, 6, 6));
        assert!(a.intersection(b).is_none());
    }

    #[test]
    fn test_bounds_iter_chunks() {
        let bounds = ChunkBounds::new(IVec3::new(0, 0, 0), IVec3::new(1, 1, 1));
        let chunks: Vec<_> = bounds.iter_chunks().collect();
        assert_eq!(chunks.len(), 8);
    }

    #[test]
    fn test_bounds_iter_chunks_deterministic() {
        let bounds = ChunkBounds::new(IVec3::new(0, 0, 0), IVec3::new(1, 1, 1));
        let chunks1: Vec<_> = bounds.iter_chunks().collect();
        let chunks2: Vec<_> = bounds.iter_chunks().collect();
        assert_eq!(chunks1, chunks2);
    }

    #[test]
    fn test_bounds_translated() {
        let bounds = ChunkBounds::new(IVec3::new(0, 0, 0), IVec3::new(2, 2, 2));
        let translated = bounds.translated(IVec3::new(5, 5, 5));
        assert_eq!(translated.min(), IVec3::new(5, 5, 5));
        assert_eq!(translated.max(), IVec3::new(7, 7, 7));
    }

    #[test]
    fn test_bounds_padded() {
        let bounds = ChunkBounds::new(IVec3::new(1, 1, 1), IVec3::new(3, 3, 3));
        let padded = bounds.padded(1);
        assert_eq!(padded.min(), IVec3::new(0, 0, 0));
        assert_eq!(padded.max(), IVec3::new(4, 4, 4));
    }

    #[test]
    fn test_bounds_center() {
        let bounds = ChunkBounds::new(IVec3::new(0, 0, 0), IVec3::new(4, 4, 4));
        assert_eq!(bounds.center(), ChunkPos::new(2, 2, 2));
    }

    #[test]
    fn test_mask_insert_remove() {
        let mut mask = ChunkMask::new();
        assert!(mask.insert(ChunkPos::new(1, 2, 3)));
        assert!(!mask.insert(ChunkPos::new(1, 2, 3)));
        assert!(mask.contains(ChunkPos::new(1, 2, 3)));
        assert_eq!(mask.len(), 1);

        assert!(mask.remove(ChunkPos::new(1, 2, 3)));
        assert!(!mask.remove(ChunkPos::new(1, 2, 3)));
        assert!(mask.is_empty());
    }

    #[test]
    fn test_mask_bounds_tracking() {
        let mut mask = ChunkMask::new();
        mask.insert(ChunkPos::new(0, 0, 0));
        mask.insert(ChunkPos::new(5, 5, 5));

        let bounds = mask.bounds().unwrap();
        assert_eq!(bounds.min(), IVec3::new(0, 0, 0));
        assert_eq!(bounds.max(), IVec3::new(5, 5, 5));
    }

    #[test]
    fn test_mask_iter_deterministic() {
        let mut mask = ChunkMask::new();
        mask.insert(ChunkPos::new(5, 0, 0));
        mask.insert(ChunkPos::new(1, 0, 0));
        mask.insert(ChunkPos::new(3, 0, 0));

        let chunks: Vec<_> = mask.iter().collect();
        assert_eq!(chunks[0], ChunkPos::new(1, 0, 0));
        assert_eq!(chunks[1], ChunkPos::new(3, 0, 0));
        assert_eq!(chunks[2], ChunkPos::new(5, 0, 0));
    }

    #[test]
    fn test_mask_from_bounds() {
        let bounds = ChunkBounds::new(IVec3::new(0, 0, 0), IVec3::new(1, 1, 1));
        let mask = ChunkMask::from_bounds(bounds);
        assert_eq!(mask.len(), 8);
    }

    #[test]
    fn test_serde_bounds() {
        let bounds = ChunkBounds::new(IVec3::new(-5, 0, 10), IVec3::new(5, 20, 30));
        let serialized = bincode::serialize(&bounds).unwrap();
        let deserialized: ChunkBounds = bincode::deserialize(&serialized).unwrap();
        assert_eq!(bounds, deserialized);
    }

    #[test]
    fn test_serde_mask() {
        let mut mask = ChunkMask::new();
        mask.insert(ChunkPos::new(1, 2, 3));
        mask.insert(ChunkPos::new(4, 5, 6));

        let serialized = bincode::serialize(&mask).unwrap();
        let deserialized: ChunkMask = bincode::deserialize(&serialized).unwrap();
        assert_eq!(mask, deserialized);
    }
}
