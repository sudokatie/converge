//! Player movement physics simulation.

use engine_core::coords::WorldPos;
use glam::Vec3;

/// Gravity acceleration (m/s^2).
pub const GRAVITY: f32 = -20.0;

/// Jump impulse velocity (m/s).
pub const JUMP_IMPULSE: f32 = 8.0;

/// Ground friction coefficient.
pub const GROUND_FRICTION: f32 = 10.0;

/// Air friction coefficient.
pub const AIR_FRICTION: f32 = 0.5;

/// Air control factor (0-1).
pub const AIR_CONTROL: f32 = 0.3;

/// Base movement speed (m/s).
pub const MOVE_SPEED: f32 = 5.0;

/// Sprint speed multiplier.
pub const SPRINT_MULTIPLIER: f32 = 1.5;

/// Trait for querying voxel world for collision.
pub trait VoxelQuery {
    /// Check if a block at the given position is solid.
    fn is_solid(&self, pos: WorldPos) -> bool;
}

/// Player physics state and simulation.
#[derive(Clone, Debug, Default)]
pub struct PlayerPhysics {
    /// Current velocity.
    pub velocity: Vec3,
    /// Whether player is on the ground.
    pub on_ground: bool,
}

impl PlayerPhysics {
    /// Create new player physics state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Update player physics for one frame.
    ///
    /// # Arguments
    /// * `position` - Player position (modified in place)
    /// * `input` - Normalized movement input (x = right, z = forward)
    /// * `jump` - Whether jump is pressed
    /// * `sprint` - Whether sprint is pressed
    /// * `world` - Voxel world for collision queries
    /// * `dt` - Delta time in seconds
    pub fn update(
        &mut self,
        position: &mut Vec3,
        input: Vec3,
        jump: bool,
        sprint: bool,
        world: &impl VoxelQuery,
        dt: f32,
    ) {
        // Apply gravity
        self.velocity.y += GRAVITY * dt;

        // Calculate target horizontal velocity
        let speed = if sprint {
            MOVE_SPEED * SPRINT_MULTIPLIER
        } else {
            MOVE_SPEED
        };

        let target_velocity = Vec3::new(input.x * speed, 0.0, input.z * speed);

        // Apply movement with different control based on ground state
        let control = if self.on_ground {
            GROUND_FRICTION
        } else {
            AIR_FRICTION * AIR_CONTROL
        };

        // Lerp horizontal velocity toward target
        self.velocity.x = lerp(self.velocity.x, target_velocity.x, control * dt);
        self.velocity.z = lerp(self.velocity.z, target_velocity.z, control * dt);

        // Handle jump
        if jump && self.on_ground {
            self.velocity.y = JUMP_IMPULSE;
            self.on_ground = false;
        }

        // Apply velocity to position
        let new_position = *position + self.velocity * dt;

        // Simple collision detection - check ground
        let feet_pos = WorldPos::new(
            new_position.x.floor() as i32,
            (new_position.y - 0.1).floor() as i32,
            new_position.z.floor() as i32,
        );

        let head_pos = WorldPos::new(
            new_position.x.floor() as i32,
            (new_position.y + 1.7).floor() as i32,
            new_position.z.floor() as i32,
        );

        // Check for ground collision
        if self.velocity.y <= 0.0 && world.is_solid(feet_pos) {
            // Land on ground
            self.on_ground = true;
            self.velocity.y = 0.0;
            position.x = new_position.x;
            position.y = (feet_pos.y() + 1) as f32 + 0.001;
            position.z = new_position.z;
        } else if self.velocity.y > 0.0 && world.is_solid(head_pos) {
            // Hit ceiling
            self.velocity.y = 0.0;
            position.x = new_position.x;
            position.z = new_position.z;
        } else {
            // In air
            self.on_ground = false;
            *position = new_position;
        }

        // Simple horizontal collision detection
        self.check_horizontal_collision(position, world);
    }

    /// Check and resolve horizontal collisions.
    fn check_horizontal_collision(&mut self, position: &mut Vec3, world: &impl VoxelQuery) {
        let check_positions = [
            // Check at feet level
            WorldPos::new(
                (position.x + 0.3).floor() as i32,
                position.y.floor() as i32,
                position.z.floor() as i32,
            ),
            WorldPos::new(
                (position.x - 0.3).floor() as i32,
                position.y.floor() as i32,
                position.z.floor() as i32,
            ),
            WorldPos::new(
                position.x.floor() as i32,
                position.y.floor() as i32,
                (position.z + 0.3).floor() as i32,
            ),
            WorldPos::new(
                position.x.floor() as i32,
                position.y.floor() as i32,
                (position.z - 0.3).floor() as i32,
            ),
        ];

        // Push player out of solid blocks
        for check_pos in &check_positions {
            if world.is_solid(*check_pos) {
                let block_center = Vec3::new(
                    check_pos.x() as f32 + 0.5,
                    check_pos.y() as f32 + 0.5,
                    check_pos.z() as f32 + 0.5,
                );
                let diff = *position - block_center;

                // Push in the direction of least penetration
                if diff.x.abs() > diff.z.abs() {
                    if diff.x > 0.0 {
                        position.x = (check_pos.x() + 1) as f32 + 0.31;
                    } else {
                        position.x = check_pos.x() as f32 - 0.31;
                    }
                    self.velocity.x = 0.0;
                } else {
                    if diff.z > 0.0 {
                        position.z = (check_pos.z() + 1) as f32 + 0.31;
                    } else {
                        position.z = check_pos.z() as f32 - 0.31;
                    }
                    self.velocity.z = 0.0;
                }
            }
        }
    }

    /// Get current speed (horizontal only).
    #[must_use]
    pub fn speed(&self) -> f32 {
        Vec3::new(self.velocity.x, 0.0, self.velocity.z).length()
    }
}

/// Linear interpolation.
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test world that is solid below y=0.
    struct TestWorld {
        solid_below: i32,
    }

    impl VoxelQuery for TestWorld {
        fn is_solid(&self, pos: WorldPos) -> bool {
            pos.y() < self.solid_below
        }
    }

    #[test]
    fn test_player_falls_with_gravity() {
        let mut physics = PlayerPhysics::new();
        let mut position = Vec3::new(0.0, 10.0, 0.0);
        let world = TestWorld { solid_below: -100 }; // No ground

        let initial_y = position.y;
        physics.update(&mut position, Vec3::ZERO, false, false, &world, 0.1);

        assert!(position.y < initial_y, "Player should fall");
        assert!(physics.velocity.y < 0.0, "Velocity should be negative");
    }

    #[test]
    fn test_player_lands_on_ground() {
        let mut physics = PlayerPhysics::new();
        physics.velocity.y = -5.0;
        let mut position = Vec3::new(0.0, 0.5, 0.0);
        let world = TestWorld { solid_below: 0 };

        physics.update(&mut position, Vec3::ZERO, false, false, &world, 0.1);

        assert!(physics.on_ground, "Player should be on ground");
        assert!(physics.velocity.y >= 0.0, "Vertical velocity should stop");
    }

    #[test]
    fn test_jump_only_on_ground() {
        let mut physics = PlayerPhysics::new();
        let mut position = Vec3::new(0.0, 1.0, 0.0);
        let world = TestWorld { solid_below: 0 };

        // Set on ground
        physics.on_ground = true;

        // Jump
        physics.update(&mut position, Vec3::ZERO, true, false, &world, 0.016);
        assert!(physics.velocity.y > 0.0, "Should jump when on ground");

        let velocity_after_jump = physics.velocity.y;

        // Try to jump again while in air
        physics.on_ground = false;
        physics.update(&mut position, Vec3::ZERO, true, false, &world, 0.016);
        assert!(
            physics.velocity.y < velocity_after_jump,
            "Should not double jump"
        );
    }

    #[test]
    fn test_sprint_increases_speed() {
        let mut physics_normal = PlayerPhysics::new();
        let mut physics_sprint = PlayerPhysics::new();
        physics_normal.on_ground = true;
        physics_sprint.on_ground = true;

        let mut pos1 = Vec3::new(0.0, 1.0, 0.0);
        let mut pos2 = Vec3::new(0.0, 1.0, 0.0);
        let world = TestWorld { solid_below: 0 };

        let input = Vec3::new(0.0, 0.0, 1.0); // Forward

        // Simulate several frames
        for _ in 0..60 {
            physics_normal.update(&mut pos1, input, false, false, &world, 0.016);
            physics_sprint.update(&mut pos2, input, false, true, &world, 0.016);
        }

        assert!(
            physics_sprint.speed() > physics_normal.speed(),
            "Sprint should be faster"
        );
    }

    #[test]
    fn test_cannot_walk_through_blocks() {
        struct WallWorld;
        impl VoxelQuery for WallWorld {
            fn is_solid(&self, pos: WorldPos) -> bool {
                // Ground at y=0, wall at x=5
                pos.y() < 0 || pos.x() == 5
            }
        }

        let mut physics = PlayerPhysics::new();
        physics.on_ground = true;
        let mut position = Vec3::new(4.0, 1.0, 0.0);
        let world = WallWorld;

        // Try to walk into wall
        let input = Vec3::new(1.0, 0.0, 0.0); // Right, toward wall
        for _ in 0..120 {
            physics.update(&mut position, input, false, false, &world, 0.016);
        }

        assert!(position.x < 5.0, "Should not pass through wall at x=5");
    }
}
