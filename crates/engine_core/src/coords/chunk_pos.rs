//! Chunk position - index of a chunk in the world.

use glam::IVec3;
use serde::{Deserialize, Serialize};

/// Chunk index in the world grid.
///
/// Each chunk contains CHUNK_SIZE^3 voxels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkPos(pub IVec3);

impl ChunkPos {
    /// Create a new chunk position.
    #[must_use]
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self(IVec3::new(x, y, z))
    }

    /// Get the X coordinate.
    #[must_use]
    pub const fn x(&self) -> i32 {
        self.0.x
    }

    /// Get the Y coordinate.
    #[must_use]
    pub const fn y(&self) -> i32 {
        self.0.y
    }

    /// Get the Z coordinate.
    #[must_use]
    pub const fn z(&self) -> i32 {
        self.0.z
    }

    /// Get the 6 face-adjacent neighbor positions.
    #[must_use]
    pub fn neighbors(&self) -> [ChunkPos; 6] {
        [
            Self::new(self.0.x - 1, self.0.y, self.0.z),
            Self::new(self.0.x + 1, self.0.y, self.0.z),
            Self::new(self.0.x, self.0.y - 1, self.0.z),
            Self::new(self.0.x, self.0.y + 1, self.0.z),
            Self::new(self.0.x, self.0.y, self.0.z - 1),
            Self::new(self.0.x, self.0.y, self.0.z + 1),
        ]
    }

    /// Manhattan distance to another chunk position.
    #[must_use]
    pub fn manhattan_distance(&self, other: ChunkPos) -> i32 {
        (self.0.x - other.0.x).abs()
            + (self.0.y - other.0.y).abs()
            + (self.0.z - other.0.z).abs()
    }

    /// Chebyshev (chessboard) distance to another chunk position.
    #[must_use]
    pub fn chebyshev_distance(&self, other: ChunkPos) -> i32 {
        (self.0.x - other.0.x)
            .abs()
            .max((self.0.y - other.0.y).abs())
            .max((self.0.z - other.0.z).abs())
    }
}

impl From<IVec3> for ChunkPos {
    fn from(v: IVec3) -> Self {
        Self(v)
    }
}

impl From<ChunkPos> for IVec3 {
    fn from(pos: ChunkPos) -> Self {
        pos.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neighbors() {
        let pos = ChunkPos::new(0, 0, 0);
        let neighbors = pos.neighbors();
        assert_eq!(neighbors.len(), 6);
        assert!(neighbors.contains(&ChunkPos::new(-1, 0, 0)));
        assert!(neighbors.contains(&ChunkPos::new(1, 0, 0)));
    }

    #[test]
    fn test_manhattan_distance() {
        let a = ChunkPos::new(0, 0, 0);
        let b = ChunkPos::new(3, 4, 5);
        assert_eq!(a.manhattan_distance(b), 12);
    }

    #[test]
    fn test_chebyshev_distance() {
        let a = ChunkPos::new(0, 0, 0);
        let b = ChunkPos::new(3, 4, 5);
        assert_eq!(a.chebyshev_distance(b), 5);
    }
}
