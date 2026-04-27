//! Movement mode definitions for the character controller.

use serde::{Deserialize, Serialize};

/// Movement mode defining how the character interacts with their environment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MovementMode {
    /// Standard ground-based movement with gravity, jumping, and friction.
    #[default]
    Walking,
    /// Fluid movement with buoyancy, drag, and vertical swim controls.
    Swimming,
    /// Surface-attached movement for walls and ladders.
    Climbing,
    /// Microgravity movement with inertial damping and push-off mechanics.
    ZeroG,
    /// Constrained movement on a tether with pendulum physics.
    Tethered,
}

impl MovementMode {
    /// Whether this mode responds to gravity.
    #[must_use]
    pub const fn uses_gravity(&self) -> bool {
        matches!(self, Self::Walking | Self::Tethered)
    }

    /// Whether this mode allows jumping.
    #[must_use]
    pub const fn can_jump(&self) -> bool {
        matches!(self, Self::Walking)
    }

    /// Whether this mode uses buoyancy physics.
    #[must_use]
    pub const fn uses_buoyancy(&self) -> bool {
        matches!(self, Self::Swimming)
    }

    /// Whether this mode is attached to a surface.
    #[must_use]
    pub const fn is_surface_attached(&self) -> bool {
        matches!(self, Self::Climbing)
    }

    /// Whether this mode has constrained movement range.
    #[must_use]
    pub const fn is_constrained(&self) -> bool {
        matches!(self, Self::Tethered)
    }

    /// Whether this mode operates in microgravity.
    #[must_use]
    pub const fn is_zero_g(&self) -> bool {
        matches!(self, Self::ZeroG)
    }

    /// Get a human-readable name for this mode.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Walking => "Walking",
            Self::Swimming => "Swimming",
            Self::Climbing => "Climbing",
            Self::ZeroG => "Zero-G",
            Self::Tethered => "Tethered",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_properties() {
        assert!(MovementMode::Walking.uses_gravity());
        assert!(MovementMode::Walking.can_jump());
        assert!(!MovementMode::Walking.uses_buoyancy());

        assert!(!MovementMode::Swimming.uses_gravity());
        assert!(!MovementMode::Swimming.can_jump());
        assert!(MovementMode::Swimming.uses_buoyancy());

        assert!(!MovementMode::Climbing.uses_gravity());
        assert!(MovementMode::Climbing.is_surface_attached());

        assert!(MovementMode::ZeroG.is_zero_g());
        assert!(!MovementMode::ZeroG.uses_gravity());

        assert!(MovementMode::Tethered.uses_gravity());
        assert!(MovementMode::Tethered.is_constrained());
    }

    #[test]
    fn default_is_walking() {
        assert_eq!(MovementMode::default(), MovementMode::Walking);
    }

    #[test]
    fn mode_names() {
        assert_eq!(MovementMode::Walking.name(), "Walking");
        assert_eq!(MovementMode::Swimming.name(), "Swimming");
        assert_eq!(MovementMode::Climbing.name(), "Climbing");
        assert_eq!(MovementMode::ZeroG.name(), "Zero-G");
        assert_eq!(MovementMode::Tethered.name(), "Tethered");
    }

    #[test]
    fn serde_roundtrip() {
        for mode in [
            MovementMode::Walking,
            MovementMode::Swimming,
            MovementMode::Climbing,
            MovementMode::ZeroG,
            MovementMode::Tethered,
        ] {
            let json = serde_json::to_string(&mode).unwrap();
            let recovered: MovementMode = serde_json::from_str(&json).unwrap();
            assert_eq!(recovered, mode);
        }
    }
}
