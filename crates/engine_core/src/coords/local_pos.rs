//! Local position - position within a chunk.

use glam::UVec3;
use serde::{Deserialize, Serialize};

use super::CHUNK_SIZE_U;

/// Position within a chunk (0..CHUNK_SIZE for each axis).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LocalPos(pub UVec3);

impl LocalPos {
    /// Create a new local position.
    ///
    /// # Panics
    /// Panics in debug mode if any coordinate is >= CHUNK_SIZE.
    #[must_use]
    pub fn new(x: u32, y: u32, z: u32) -> Self {
        debug_assert!(x < CHUNK_SIZE_U, "x must be < CHUNK_SIZE");
        debug_assert!(y < CHUNK_SIZE_U, "y must be < CHUNK_SIZE");
        debug_assert!(z < CHUNK_SIZE_U, "z must be < CHUNK_SIZE");
        Self(UVec3::new(x, y, z))
    }

    /// Get the X coordinate.
    #[must_use]
    pub const fn x(&self) -> u32 {
        self.0.x
    }

    /// Get the Y coordinate.
    #[must_use]
    pub const fn y(&self) -> u32 {
        self.0.y
    }

    /// Get the Z coordinate.
    #[must_use]
    pub const fn z(&self) -> u32 {
        self.0.z
    }

    /// Convert to a linear index for array storage.
    ///
    /// Index = x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE
    #[must_use]
    pub fn to_index(&self) -> usize {
        (self.0.x + self.0.y * CHUNK_SIZE_U + self.0.z * CHUNK_SIZE_U * CHUNK_SIZE_U) as usize
    }

    /// Create from a linear index.
    ///
    /// # Panics
    /// Panics if index >= CHUNK_SIZE^3.
    #[must_use]
    pub fn from_index(index: usize) -> Self {
        let index = index as u32;
        let chunk_sq = CHUNK_SIZE_U * CHUNK_SIZE_U;
        let z = index / chunk_sq;
        let remainder = index % chunk_sq;
        let y = remainder / CHUNK_SIZE_U;
        let x = remainder % CHUNK_SIZE_U;
        Self::new(x, y, z)
    }

    /// Total number of voxels in a chunk (CHUNK_SIZE^3).
    pub const VOXELS_PER_CHUNK: usize = (CHUNK_SIZE_U * CHUNK_SIZE_U * CHUNK_SIZE_U) as usize;
}

impl From<UVec3> for LocalPos {
    fn from(v: UVec3) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

impl From<LocalPos> for UVec3 {
    fn from(pos: LocalPos) -> Self {
        pos.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_origin_index() {
        let pos = LocalPos::new(0, 0, 0);
        assert_eq!(pos.to_index(), 0);
    }

    #[test]
    fn test_index_round_trip() {
        for z in 0..16 {
            for y in 0..16 {
                for x in 0..16 {
                    let pos = LocalPos::new(x, y, z);
                    let index = pos.to_index();
                    let reconstructed = LocalPos::from_index(index);
                    assert_eq!(pos, reconstructed);
                }
            }
        }
    }

    #[test]
    fn test_voxels_per_chunk() {
        assert_eq!(LocalPos::VOXELS_PER_CHUNK, 16 * 16 * 16);
    }

    #[test]
    fn test_max_index() {
        let pos = LocalPos::new(15, 15, 15);
        let index = pos.to_index();
        assert_eq!(index, LocalPos::VOXELS_PER_CHUNK - 1);
    }
}
