//! Configuration for character movement parameters.

use serde::{Deserialize, Serialize};

/// Configuration for walking mode physics.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WalkingConfig {
    /// Base movement speed (m/s).
    pub move_speed: f32,
    /// Sprint speed multiplier.
    pub sprint_multiplier: f32,
    /// Jump impulse velocity (m/s).
    pub jump_impulse: f32,
    /// Ground friction coefficient.
    pub ground_friction: f32,
    /// Air friction coefficient.
    pub air_friction: f32,
    /// Air control factor (0-1).
    pub air_control: f32,
    /// Gravity scale (1.0 = full gravity).
    pub gravity_scale: f32,
}

impl Default for WalkingConfig {
    fn default() -> Self {
        Self {
            move_speed: 5.0,
            sprint_multiplier: 1.5,
            jump_impulse: 8.0,
            ground_friction: 10.0,
            air_friction: 0.5,
            air_control: 0.3,
            gravity_scale: 1.0,
        }
    }
}

/// Configuration for swimming mode physics.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SwimmingConfig {
    /// Swim speed (m/s).
    pub swim_speed: f32,
    /// Sprint swim speed multiplier.
    pub sprint_multiplier: f32,
    /// Buoyancy force (positive = float, negative = sink).
    pub buoyancy: f32,
    /// Water drag coefficient.
    pub drag: f32,
    /// Vertical swim control factor.
    pub vertical_control: f32,
}

impl Default for SwimmingConfig {
    fn default() -> Self {
        Self {
            swim_speed: 3.0,
            sprint_multiplier: 1.3,
            buoyancy: 2.0,
            drag: 4.0,
            vertical_control: 0.8,
        }
    }
}

/// Configuration for climbing mode physics.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClimbingConfig {
    /// Climb speed (m/s).
    pub climb_speed: f32,
    /// Horizontal strafe speed while climbing.
    pub strafe_speed: f32,
    /// How quickly velocity responds to input.
    pub acceleration: f32,
    /// Friction when releasing input.
    pub friction: f32,
}

impl Default for ClimbingConfig {
    fn default() -> Self {
        Self {
            climb_speed: 2.5,
            strafe_speed: 2.0,
            acceleration: 8.0,
            friction: 12.0,
        }
    }
}

/// Configuration for zero-G mode physics.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ZeroGConfig {
    /// Thrust speed from push-off (m/s).
    pub push_speed: f32,
    /// Velocity damping coefficient (simulates stabilization thrusters).
    pub damping: f32,
    /// Angular damping for rotation control.
    pub angular_damping: f32,
    /// Maximum velocity magnitude.
    pub max_velocity: f32,
}

impl Default for ZeroGConfig {
    fn default() -> Self {
        Self {
            push_speed: 4.0,
            damping: 0.5,
            angular_damping: 2.0,
            max_velocity: 20.0,
        }
    }
}

/// Configuration for tethered mode physics.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TetheredConfig {
    /// Maximum tether length (m).
    pub max_length: f32,
    /// Minimum tether length (m).
    pub min_length: f32,
    /// Swing damping factor.
    pub swing_damping: f32,
    /// Reel speed for extending/retracting (m/s).
    pub reel_speed: f32,
    /// Gravity scale while tethered.
    pub gravity_scale: f32,
    /// Tether stiffness (spring constant).
    pub stiffness: f32,
}

impl Default for TetheredConfig {
    fn default() -> Self {
        Self {
            max_length: 50.0,
            min_length: 1.0,
            swing_damping: 0.1,
            reel_speed: 5.0,
            gravity_scale: 1.0,
            stiffness: 100.0,
        }
    }
}

/// Complete movement configuration for all modes.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MovementConfig {
    /// Walking mode configuration.
    pub walking: WalkingConfig,
    /// Swimming mode configuration.
    pub swimming: SwimmingConfig,
    /// Climbing mode configuration.
    pub climbing: ClimbingConfig,
    /// Zero-G mode configuration.
    pub zero_g: ZeroGConfig,
    /// Tethered mode configuration.
    pub tethered: TetheredConfig,
}

impl MovementConfig {
    /// Create a new configuration with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a configuration tuned for low gravity environments.
    #[must_use]
    pub fn low_gravity() -> Self {
        Self {
            walking: WalkingConfig {
                jump_impulse: 12.0,
                air_friction: 0.3,
                air_control: 0.5,
                gravity_scale: 0.4,
                ..Default::default()
            },
            swimming: SwimmingConfig {
                buoyancy: 4.0,
                drag: 2.0,
                ..Default::default()
            },
            tethered: TetheredConfig {
                gravity_scale: 0.4,
                swing_damping: 0.05,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Create a configuration for underwater/high-drag environments.
    #[must_use]
    pub fn high_drag() -> Self {
        Self {
            walking: WalkingConfig {
                move_speed: 3.0,
                sprint_multiplier: 1.2,
                ground_friction: 15.0,
                ..Default::default()
            },
            swimming: SwimmingConfig {
                swim_speed: 2.0,
                drag: 8.0,
                ..Default::default()
            },
            zero_g: ZeroGConfig {
                damping: 2.0,
                max_velocity: 10.0,
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_configs_reasonable() {
        let config = MovementConfig::default();

        assert!(config.walking.move_speed > 0.0);
        assert!(config.walking.sprint_multiplier >= 1.0);
        assert!(config.walking.jump_impulse > 0.0);

        assert!(config.swimming.swim_speed > 0.0);
        assert!(config.swimming.drag > 0.0);

        assert!(config.climbing.climb_speed > 0.0);

        assert!(config.zero_g.push_speed > 0.0);
        assert!(config.zero_g.max_velocity > config.zero_g.push_speed);

        assert!(config.tethered.max_length > config.tethered.min_length);
    }

    #[test]
    fn preset_configs() {
        let low_g = MovementConfig::low_gravity();
        assert!(low_g.walking.gravity_scale < 1.0);
        assert!(low_g.walking.jump_impulse > MovementConfig::default().walking.jump_impulse);

        let high_drag = MovementConfig::high_drag();
        assert!(high_drag.swimming.drag > MovementConfig::default().swimming.drag);
    }

    #[test]
    fn serde_roundtrip() {
        let config = MovementConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let recovered: MovementConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, config);
    }
}
