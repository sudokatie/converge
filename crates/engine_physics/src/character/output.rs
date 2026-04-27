//! Deterministic output from character controller updates.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::MovementMode;

/// Event generated during character movement.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MovementEvent {
    /// Character jumped.
    Jumped,
    /// Character landed on ground.
    Landed { impact_velocity: f32 },
    /// Character entered water.
    EnteredWater,
    /// Character exited water.
    ExitedWater,
    /// Character grabbed a climbing surface.
    GrabbedSurface { normal: Vec3 },
    /// Character released from climbing surface.
    ReleasedSurface,
    /// Character pushed off in zero-G.
    PushedOff { direction: Vec3 },
    /// Character attached tether.
    TetherAttached { anchor: Vec3 },
    /// Character detached tether.
    TetherDetached,
    /// Tether reached maximum length.
    TetherMaxLength,
    /// Movement mode changed.
    ModeChanged {
        from: MovementMode,
        to: MovementMode,
    },
    /// Character hit a wall.
    WallImpact { normal: Vec3, velocity: f32 },
    /// Character hit ceiling.
    CeilingImpact { velocity: f32 },
}

/// Deterministic output from a single tick of character movement.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MovementOutput {
    /// New position after movement.
    pub position: Vec3,
    /// New velocity after movement.
    pub velocity: Vec3,
    /// Current movement mode.
    pub mode: MovementMode,
    /// Events generated this tick.
    pub events: Vec<MovementEvent>,
    /// Whether the character is on the ground.
    pub on_ground: bool,
    /// Horizontal speed magnitude.
    pub horizontal_speed: f32,
    /// Vertical speed (positive = up).
    pub vertical_speed: f32,
}

impl MovementOutput {
    /// Create a new movement output.
    #[must_use]
    pub fn new(position: Vec3, velocity: Vec3, mode: MovementMode) -> Self {
        let horizontal = Vec3::new(velocity.x, 0.0, velocity.z);
        Self {
            position,
            velocity,
            mode,
            events: Vec::new(),
            on_ground: false,
            horizontal_speed: horizontal.length(),
            vertical_speed: velocity.y,
        }
    }

    /// Add an event to the output.
    pub fn push_event(&mut self, event: MovementEvent) {
        self.events.push(event);
    }

    /// Set the grounded state.
    #[must_use]
    pub fn with_on_ground(mut self, on_ground: bool) -> Self {
        self.on_ground = on_ground;
        self
    }

    /// Check if a specific event type occurred.
    #[must_use]
    pub fn has_event(&self, check: impl Fn(&MovementEvent) -> bool) -> bool {
        self.events.iter().any(check)
    }

    /// Check if character jumped this tick.
    #[must_use]
    pub fn jumped(&self) -> bool {
        self.has_event(|e| matches!(e, MovementEvent::Jumped))
    }

    /// Check if character landed this tick.
    #[must_use]
    pub fn landed(&self) -> bool {
        self.has_event(|e| matches!(e, MovementEvent::Landed { .. }))
    }

    /// Check if mode changed this tick.
    #[must_use]
    pub fn mode_changed(&self) -> bool {
        self.has_event(|e| matches!(e, MovementEvent::ModeChanged { .. }))
    }

    /// Get the total speed magnitude.
    #[must_use]
    pub fn speed(&self) -> f32 {
        self.velocity.length()
    }
}

impl Default for MovementOutput {
    fn default() -> Self {
        Self::new(Vec3::ZERO, Vec3::ZERO, MovementMode::Walking)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_creation() {
        let output = MovementOutput::new(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(5.0, 0.0, 0.0),
            MovementMode::Walking,
        );

        assert_eq!(output.position, Vec3::new(1.0, 2.0, 3.0));
        assert!((output.horizontal_speed - 5.0).abs() < f32::EPSILON);
        assert!(output.events.is_empty());
    }

    #[test]
    fn event_detection() {
        let mut output = MovementOutput::default();
        assert!(!output.jumped());

        output.push_event(MovementEvent::Jumped);
        assert!(output.jumped());
    }

    #[test]
    fn landed_event() {
        let mut output = MovementOutput::default();
        output.push_event(MovementEvent::Landed {
            impact_velocity: 10.0,
        });
        assert!(output.landed());
    }

    #[test]
    fn mode_change_event() {
        let mut output = MovementOutput::default();
        output.push_event(MovementEvent::ModeChanged {
            from: MovementMode::Walking,
            to: MovementMode::Swimming,
        });
        assert!(output.mode_changed());
    }

    #[test]
    fn speed_calculation() {
        let output =
            MovementOutput::new(Vec3::ZERO, Vec3::new(3.0, 4.0, 0.0), MovementMode::Walking);
        assert!((output.speed() - 5.0).abs() < 0.001);
    }
}
