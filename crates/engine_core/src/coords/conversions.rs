//! Coordinate conversion utilities between [`Vec3`], [`WorldPos`], and [`ChunkPos`].
//!
//! These functions convert between floating-point render coordinates and
//! integer voxel/chunk coordinates. Chunk size is [`CHUNK_SIZE`] voxels per axis.

use glam::Vec3;

use super::{CHUNK_SIZE, ChunkPos, WorldPos};

/// Convert a floating-point position to the nearest world position.
#[must_use]
#[allow(dead_code)]
pub fn vec3_to_world_pos(v: Vec3) -> WorldPos {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "voxel coords are intentionally i32; overflow beyond i32::MAX is acceptable"
    )]
    fn floor_to_i32(x: f32) -> i32 {
        x.floor() as i32
    }
    WorldPos::new(floor_to_i32(v.x), floor_to_i32(v.y), floor_to_i32(v.z))
}

/// Convert a world position to its center as a floating-point position.
#[must_use]
#[allow(dead_code)]
pub fn world_pos_to_vec3(pos: WorldPos) -> Vec3 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "voxel coordinates are small; precision loss beyond 2^24 is acceptable"
    )]
    fn i32_to_f32(x: i32) -> f32 {
        x as f32
    }
    Vec3::new(
        i32_to_f32(pos.x()) + 0.5,
        i32_to_f32(pos.y()) + 0.5,
        i32_to_f32(pos.z()) + 0.5,
    )
}

/// Get the world-space origin of a chunk (minimum corner).
#[must_use]
#[allow(dead_code)]
pub fn chunk_origin(chunk: ChunkPos) -> Vec3 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "chunk coordinates are small; precision loss beyond 2^24 chunks is acceptable"
    )]
    fn i32_to_f32(x: i32) -> f32 {
        x as f32
    }
    Vec3::new(
        i32_to_f32(chunk.x() * CHUNK_SIZE),
        i32_to_f32(chunk.y() * CHUNK_SIZE),
        i32_to_f32(chunk.z() * CHUNK_SIZE),
    )
}

/// Get the world-space center of a chunk.
#[must_use]
#[allow(dead_code)]
pub fn chunk_center(chunk: ChunkPos) -> Vec3 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "chunk coordinates are small; precision loss beyond 2^24 chunks is acceptable"
    )]
    fn i32_to_f32(x: i32) -> f32 {
        x as f32
    }
    let half = i32_to_f32(CHUNK_SIZE) * 0.5;
    Vec3::new(
        i32_to_f32(chunk.x() * CHUNK_SIZE) + half,
        i32_to_f32(chunk.y() * CHUNK_SIZE) + half,
        i32_to_f32(chunk.z() * CHUNK_SIZE) + half,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_vec3_to_world_pos_positive() {
        let v = Vec3::new(1.5, 2.7, 3.9);
        let pos = vec3_to_world_pos(v);
        assert_eq!(pos, WorldPos::new(1, 2, 3));
    }

    #[test]
    fn test_vec3_to_world_pos_negative() {
        let v = Vec3::new(-0.5, -1.5, -2.5);
        let pos = vec3_to_world_pos(v);
        assert_eq!(pos, WorldPos::new(-1, -2, -3));
    }

    #[test]
    fn test_world_pos_to_vec3() {
        let pos = WorldPos::new(5, 10, 15);
        let v = world_pos_to_vec3(pos);
        assert_relative_eq!(v.x, 5.5);
        assert_relative_eq!(v.y, 10.5);
        assert_relative_eq!(v.z, 15.5);
    }

    #[test]
    fn test_chunk_origin() {
        let chunk = ChunkPos::new(1, 2, 3);
        let origin = chunk_origin(chunk);
        assert_relative_eq!(origin.x, 16.0);
        assert_relative_eq!(origin.y, 32.0);
        assert_relative_eq!(origin.z, 48.0);
    }

    #[test]
    fn test_chunk_center() {
        let chunk = ChunkPos::new(0, 0, 0);
        let center = chunk_center(chunk);
        assert_relative_eq!(center.x, 8.0);
        assert_relative_eq!(center.y, 8.0);
        assert_relative_eq!(center.z, 8.0);
    }
}
