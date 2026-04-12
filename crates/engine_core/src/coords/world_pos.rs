//! World position - global voxel coordinates.

use glam::IVec3;
use serde::{Deserialize, Serialize};

use super::{ChunkPos, LocalPos, CHUNK_SIZE};

/// Global voxel position in the world.
///
/// Can be negative (world extends in all directions from origin).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorldPos(pub IVec3);

impl WorldPos {
    /// Create a new world position.
    #[must_use]
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self(IVec3::new(x, y, z))
    }

    /// Get the chunk this position is in.
    #[must_use]
    pub fn to_chunk_pos(&self) -> ChunkPos {
        ChunkPos(IVec3::new(
            self.0.x.div_euclid(CHUNK_SIZE),
            self.0.y.div_euclid(CHUNK_SIZE),
            self.0.z.div_euclid(CHUNK_SIZE),
        ))
    }

    /// Get the local position within the chunk.
    #[must_use]
    pub fn to_local_pos(&self) -> LocalPos {
        LocalPos::new(
            self.0.x.rem_euclid(CHUNK_SIZE) as u32,
            self.0.y.rem_euclid(CHUNK_SIZE) as u32,
            self.0.z.rem_euclid(CHUNK_SIZE) as u32,
        )
    }

    /// Create a world position from chunk and local coordinates.
    #[must_use]
    pub fn from_chunk_and_local(chunk: ChunkPos, local: LocalPos) -> Self {
        Self(IVec3::new(
            chunk.0.x * CHUNK_SIZE + local.0.x as i32,
            chunk.0.y * CHUNK_SIZE + local.0.y as i32,
            chunk.0.z * CHUNK_SIZE + local.0.z as i32,
        ))
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
}

impl From<IVec3> for WorldPos {
    fn from(v: IVec3) -> Self {
        Self(v)
    }
}

impl From<WorldPos> for IVec3 {
    fn from(pos: WorldPos) -> Self {
        pos.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_origin() {
        let pos = WorldPos::new(0, 0, 0);
        assert_eq!(pos.to_chunk_pos(), ChunkPos::new(0, 0, 0));
        assert_eq!(pos.to_local_pos(), LocalPos::new(0, 0, 0));
    }

    #[test]
    fn test_within_first_chunk() {
        let pos = WorldPos::new(15, 15, 15);
        assert_eq!(pos.to_chunk_pos(), ChunkPos::new(0, 0, 0));
        assert_eq!(pos.to_local_pos(), LocalPos::new(15, 15, 15));
    }

    #[test]
    fn test_second_chunk() {
        let pos = WorldPos::new(16, 0, 0);
        assert_eq!(pos.to_chunk_pos(), ChunkPos::new(1, 0, 0));
        assert_eq!(pos.to_local_pos(), LocalPos::new(0, 0, 0));
    }

    #[test]
    fn test_negative_position() {
        let pos = WorldPos::new(-1, 0, 0);
        assert_eq!(pos.to_chunk_pos(), ChunkPos::new(-1, 0, 0));
        assert_eq!(pos.to_local_pos(), LocalPos::new(15, 0, 0));
    }

    #[test]
    fn test_negative_full_chunk() {
        let pos = WorldPos::new(-16, -16, -16);
        assert_eq!(pos.to_chunk_pos(), ChunkPos::new(-1, -1, -1));
        assert_eq!(pos.to_local_pos(), LocalPos::new(0, 0, 0));
    }

    #[test]
    fn test_round_trip() {
        let original = WorldPos::new(37, -15, 128);
        let chunk = original.to_chunk_pos();
        let local = original.to_local_pos();
        let reconstructed = WorldPos::from_chunk_and_local(chunk, local);
        assert_eq!(original, reconstructed);
    }

    #[test]
    fn test_round_trip_negative() {
        let original = WorldPos::new(-37, -15, -128);
        let chunk = original.to_chunk_pos();
        let local = original.to_local_pos();
        let reconstructed = WorldPos::from_chunk_and_local(chunk, local);
        assert_eq!(original, reconstructed);
    }
}
