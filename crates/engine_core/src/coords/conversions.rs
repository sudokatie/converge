//! Coordinate conversion utilities.

use glam::Vec3;

use super::{ChunkPos, WorldPos, CHUNK_SIZE};

/// Convert a floating-point position to the nearest world position.
#[must_use]
pub fn vec3_to_world_pos(v: Vec3) -> WorldPos {
    WorldPos::new(v.x.floor() as i32, v.y.floor() as i32, v.z.floor() as i32)
}

/// Convert a world position to its center as a floating-point position.
#[must_use]
pub fn world_pos_to_vec3(pos: WorldPos) -> Vec3 {
    Vec3::new(
        pos.x() as f32 + 0.5,
        pos.y() as f32 + 0.5,
        pos.z() as f32 + 0.5,
    )
}

/// Get the world-space origin of a chunk (minimum corner).
#[must_use]
pub fn chunk_origin(chunk: ChunkPos) -> Vec3 {
    Vec3::new(
        (chunk.x() * CHUNK_SIZE) as f32,
        (chunk.y() * CHUNK_SIZE) as f32,
        (chunk.z() * CHUNK_SIZE) as f32,
    )
}

/// Get the world-space center of a chunk.
#[must_use]
pub fn chunk_center(chunk: ChunkPos) -> Vec3 {
    let half = CHUNK_SIZE as f32 * 0.5;
    Vec3::new(
        (chunk.x() * CHUNK_SIZE) as f32 + half,
        (chunk.y() * CHUNK_SIZE) as f32 + half,
        (chunk.z() * CHUNK_SIZE) as f32 + half,
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
