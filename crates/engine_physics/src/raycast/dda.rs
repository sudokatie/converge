//! Digital Differential Analyzer for voxel ray traversal.

use engine_core::coords::WorldPos;
use glam::{IVec3, Vec3};

/// Result of a voxel raycast hit.
#[derive(Clone, Copy, Debug)]
pub struct VoxelHit {
    /// World position of the hit block.
    pub block_pos: WorldPos,
    /// Normal of the face that was hit (-1, 0, or 1 per axis).
    pub face_normal: IVec3,
    /// Distance from ray origin to hit point.
    pub distance: f32,
}

/// Trait for querying if a voxel is solid.
pub trait VoxelWorld {
    /// Check if block at position is solid (should stop ray).
    fn is_solid(&self, pos: WorldPos) -> bool;
}

/// Cast a ray through the voxel world using DDA algorithm.
///
/// # Arguments
/// * `origin` - Ray start position
/// * `direction` - Normalized ray direction
/// * `max_distance` - Maximum ray travel distance
/// * `world` - Voxel world to query
///
/// # Returns
/// The first solid voxel hit, or None if no hit within max_distance.
pub fn dda_raycast(
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
    world: &impl VoxelWorld,
) -> Option<VoxelHit> {
    // Current voxel position
    let mut pos = IVec3::new(
        origin.x.floor() as i32,
        origin.y.floor() as i32,
        origin.z.floor() as i32,
    );

    // Direction signs
    let step = IVec3::new(
        if direction.x >= 0.0 { 1 } else { -1 },
        if direction.y >= 0.0 { 1 } else { -1 },
        if direction.z >= 0.0 { 1 } else { -1 },
    );

    // Distance to next voxel boundary on each axis (t_max)
    let mut t_max = Vec3::new(
        if direction.x != 0.0 {
            let next_x = if step.x > 0 {
                (pos.x + 1) as f32
            } else {
                pos.x as f32
            };
            (next_x - origin.x) / direction.x
        } else {
            f32::INFINITY
        },
        if direction.y != 0.0 {
            let next_y = if step.y > 0 {
                (pos.y + 1) as f32
            } else {
                pos.y as f32
            };
            (next_y - origin.y) / direction.y
        } else {
            f32::INFINITY
        },
        if direction.z != 0.0 {
            let next_z = if step.z > 0 {
                (pos.z + 1) as f32
            } else {
                pos.z as f32
            };
            (next_z - origin.z) / direction.z
        } else {
            f32::INFINITY
        },
    );

    // Distance to traverse one full voxel on each axis (t_delta)
    let t_delta = Vec3::new(
        if direction.x != 0.0 {
            (1.0 / direction.x).abs()
        } else {
            f32::INFINITY
        },
        if direction.y != 0.0 {
            (1.0 / direction.y).abs()
        } else {
            f32::INFINITY
        },
        if direction.z != 0.0 {
            (1.0 / direction.z).abs()
        } else {
            f32::INFINITY
        },
    );

    // Track which face was crossed
    let mut face_normal = IVec3::ZERO;
    let mut distance = 0.0;

    // Maximum iterations to prevent infinite loop
    let max_steps = (max_distance * 2.0) as i32 + 10;

    for _ in 0..max_steps {
        // Check current voxel
        let world_pos = WorldPos::new(pos.x, pos.y, pos.z);
        if world.is_solid(world_pos) {
            return Some(VoxelHit {
                block_pos: world_pos,
                face_normal,
                distance,
            });
        }

        // Step to next voxel on axis with smallest t_max
        if t_max.x < t_max.y && t_max.x < t_max.z {
            distance = t_max.x;
            if distance > max_distance {
                return None;
            }
            pos.x += step.x;
            t_max.x += t_delta.x;
            face_normal = IVec3::new(-step.x, 0, 0);
        } else if t_max.y < t_max.z {
            distance = t_max.y;
            if distance > max_distance {
                return None;
            }
            pos.y += step.y;
            t_max.y += t_delta.y;
            face_normal = IVec3::new(0, -step.y, 0);
        } else {
            distance = t_max.z;
            if distance > max_distance {
                return None;
            }
            pos.z += step.z;
            t_max.z += t_delta.z;
            face_normal = IVec3::new(0, 0, -step.z);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test world with ground at y=0.
    struct GroundWorld;
    impl VoxelWorld for GroundWorld {
        fn is_solid(&self, pos: WorldPos) -> bool {
            pos.y() < 0
        }
    }

    /// Test world with a single block.
    struct SingleBlockWorld {
        block: IVec3,
    }
    impl VoxelWorld for SingleBlockWorld {
        fn is_solid(&self, pos: WorldPos) -> bool {
            pos.x() == self.block.x && pos.y() == self.block.y && pos.z() == self.block.z
        }
    }

    #[test]
    fn test_raycast_down_hits_ground() {
        let world = GroundWorld;
        let origin = Vec3::new(0.5, 5.0, 0.5);
        let direction = Vec3::new(0.0, -1.0, 0.0);

        let hit = dda_raycast(origin, direction, 100.0, &world);
        assert!(hit.is_some(), "Should hit ground");

        let hit = hit.unwrap();
        assert_eq!(hit.block_pos.y(), -1, "Should hit block at y=-1");
        assert_eq!(hit.face_normal, IVec3::new(0, 1, 0), "Normal should be +Y");
        // From y=5 to the top of block y=-1 (which is y=0) = 5 units
        assert!(hit.distance >= 4.9 && hit.distance <= 5.1, "Distance should be ~5, got {}", hit.distance);
    }

    #[test]
    fn test_raycast_misses_when_pointing_away() {
        let world = GroundWorld;
        let origin = Vec3::new(0.5, 5.0, 0.5);
        let direction = Vec3::new(0.0, 1.0, 0.0); // Up, away from ground

        let hit = dda_raycast(origin, direction, 100.0, &world);
        assert!(hit.is_none(), "Should not hit anything");
    }

    #[test]
    fn test_raycast_hits_single_block() {
        let world = SingleBlockWorld {
            block: IVec3::new(10, 5, 0),
        };
        let origin = Vec3::new(0.5, 5.5, 0.5);
        let direction = Vec3::new(1.0, 0.0, 0.0).normalize();

        let hit = dda_raycast(origin, direction, 100.0, &world);
        assert!(hit.is_some(), "Should hit block");

        let hit = hit.unwrap();
        assert_eq!(hit.block_pos.x(), 10);
        assert_eq!(hit.block_pos.y(), 5);
        assert_eq!(hit.block_pos.z(), 0);
        assert_eq!(hit.face_normal, IVec3::new(-1, 0, 0), "Hit from -X side");
    }

    #[test]
    fn test_raycast_diagonal() {
        let world = SingleBlockWorld {
            block: IVec3::new(5, 5, 5),
        };
        let origin = Vec3::new(0.5, 0.5, 0.5);
        let direction = Vec3::new(1.0, 1.0, 1.0).normalize();

        let hit = dda_raycast(origin, direction, 100.0, &world);
        assert!(hit.is_some(), "Should hit block diagonally");

        let hit = hit.unwrap();
        assert_eq!(hit.block_pos.x(), 5);
        assert_eq!(hit.block_pos.y(), 5);
        assert_eq!(hit.block_pos.z(), 5);
    }

    #[test]
    fn test_raycast_respects_max_distance() {
        let world = SingleBlockWorld {
            block: IVec3::new(100, 0, 0),
        };
        let origin = Vec3::new(0.5, 0.5, 0.5);
        let direction = Vec3::new(1.0, 0.0, 0.0);

        let hit = dda_raycast(origin, direction, 10.0, &world);
        assert!(hit.is_none(), "Should not hit block beyond max distance");
    }

    #[test]
    fn test_face_normal_correctness() {
        let world = SingleBlockWorld {
            block: IVec3::new(0, 0, 5),
        };

        // Hit from -Z
        let hit = dda_raycast(
            Vec3::new(0.5, 0.5, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            100.0,
            &world,
        );
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().face_normal, IVec3::new(0, 0, -1));

        // Hit from +Z
        let hit = dda_raycast(
            Vec3::new(0.5, 0.5, 10.0),
            Vec3::new(0.0, 0.0, -1.0),
            100.0,
            &world,
        );
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().face_normal, IVec3::new(0, 0, 1));
    }
}
