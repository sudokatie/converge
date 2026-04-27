//! Reference frame API for moving platforms and dynamic bodies.
//!
//! This module provides a coordinate system abstraction for large dynamic bodies
//! (platforms, vehicles, rotating structures) that characters and objects can
//! ride on.
//!
//! # Architecture
//!
//! - [`ReferenceFrame`]: Core type representing a moving/rotating coordinate system
//! - [`FrameAttachment`]: Tracks an entity's attachment to a frame with velocity inheritance
//! - [`FrameResolver`]: Manages frame hierarchies and computes effective transforms
//!
//! # Usage
//!
//! ```ignore
//! use engine_physics::reference_frame::{ReferenceFrame, FrameAttachment};
//! use glam::Vec3;
//!
//! // Create a moving platform frame
//! let mut platform = ReferenceFrame::with_velocity(
//!     Vec3::new(0.0, 10.0, 0.0),  // origin
//!     Vec3::new(5.0, 0.0, 0.0),   // velocity
//! );
//!
//! // Attach a character to the platform
//! let attachment = FrameAttachment::attach(
//!     &platform,
//!     Vec3::new(0.0, 11.0, 0.0),  // world position
//!     Vec3::new(0.0, 0.0, 0.0),   // world velocity
//! );
//!
//! // Get the character's world-space velocity (inherits platform motion)
//! let world_vel = attachment.world_velocity(&platform);
//! assert_eq!(world_vel.x, 5.0);  // inherited from platform
//!
//! // Detach and inherit full frame velocity
//! let result = attachment.detach(&platform);
//! // result.world_velocity includes platform velocity
//! ```
//!
//! # Frame Hierarchies
//!
//! Frames can have parent frames for nested motion (e.g., a turret on a tank):
//!
//! ```ignore
//! use engine_physics::reference_frame::{ReferenceFrame, FrameResolver};
//! use glam::Vec3;
//!
//! let mut resolver = FrameResolver::new();
//!
//! // Tank moving forward
//! let tank = ReferenceFrame::with_velocity(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
//! resolver.insert(1, tank);
//!
//! // Turret on the tank
//! let turret = ReferenceFrame::at_position(Vec3::new(0.0, 2.0, 0.0)).with_parent(1);
//! resolver.insert(2, turret);
//!
//! // Resolve turret's world-space state (includes tank motion)
//! let resolved = resolver.resolve(2).unwrap();
//! ```
//!
//! # Integration with Character Controller
//!
//! When a character is riding a frame:
//!
//! 1. Detect landing on a frame (via collision system)
//! 2. Create a [`FrameAttachment`] with current world position/velocity
//! 3. Each tick, run character physics in local frame coordinates
//! 4. Update attachment with local movement results
//! 5. Convert back to world space for rendering/collision
//! 6. On detach (jump, fall off), inherit frame velocity
//!
//! The [`FrameAttachment::update`] method handles smooth velocity blending
//! during attachment transitions to prevent jitter.

mod attachment;
mod frame;
#[cfg(test)]
mod integration_tests;
mod resolver;

pub use attachment::{AttachmentSet, DetachResult, FrameAttachment};
pub use frame::ReferenceFrame;
pub use resolver::FrameResolver;

/// Compute the relative velocity between two frames at a given world point.
///
/// Useful for determining if a character should attach to a new frame
/// (relative velocity below threshold) or bounce off.
#[must_use]
pub fn relative_velocity_at_point(
    from_frame: &ReferenceFrame,
    to_frame: &ReferenceFrame,
    world_point: Vec3,
) -> Vec3 {
    to_frame.velocity_at_point(world_point) - from_frame.velocity_at_point(world_point)
}

use glam::Vec3;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_velocity_same_frame() {
        let frame = ReferenceFrame::with_velocity(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
        let rel = relative_velocity_at_point(&frame, &frame, Vec3::ZERO);
        assert!((rel - Vec3::ZERO).length() < 1e-5);
    }

    #[test]
    fn relative_velocity_different_frames() {
        let frame_a = ReferenceFrame::with_velocity(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
        let frame_b = ReferenceFrame::with_velocity(Vec3::ZERO, Vec3::new(5.0, 0.0, 0.0));
        let rel = relative_velocity_at_point(&frame_a, &frame_b, Vec3::ZERO);
        assert!((rel - Vec3::new(-5.0, 0.0, 0.0)).length() < 1e-5);
    }
}
