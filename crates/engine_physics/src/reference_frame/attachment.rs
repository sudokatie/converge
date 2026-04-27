//! Frame attachment tracking for characters and objects riding on reference frames.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::ReferenceFrame;

/// Attachment state for an entity riding on a reference frame.
///
/// Tracks the relationship between a character/object and the frame it's
/// attached to, handling velocity inheritance and smooth transitions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrameAttachment {
    /// Local position relative to the frame origin.
    pub local_position: Vec3,
    /// Velocity relative to the frame (local coordinates).
    pub local_velocity: Vec3,
    /// How long the attachment has been active (seconds).
    pub attached_time: f32,
    /// Velocity blending factor during attachment transition (0 = frame, 1 = world).
    blend_factor: f32,
}

impl Default for FrameAttachment {
    fn default() -> Self {
        Self {
            local_position: Vec3::ZERO,
            local_velocity: Vec3::ZERO,
            attached_time: 0.0,
            blend_factor: 1.0,
        }
    }
}

/// Duration over which to blend velocities during attachment transitions.
const BLEND_DURATION: f32 = 0.1;

impl FrameAttachment {
    /// Create a new attachment from world position and velocity.
    ///
    /// Computes the local position and velocity relative to the frame.
    #[must_use]
    pub fn attach(frame: &ReferenceFrame, world_position: Vec3, world_velocity: Vec3) -> Self {
        let local_position = frame.world_to_local_position(world_position);
        let local_velocity = frame.world_to_local_velocity(world_position, world_velocity);

        Self {
            local_position,
            local_velocity,
            attached_time: 0.0,
            blend_factor: 1.0,
        }
    }

    /// Update the attachment state for one time step.
    ///
    /// Call this each tick while attached. Updates blend factor and time.
    pub fn update(&mut self, dt: f32) {
        self.attached_time += dt;
        if self.blend_factor > 0.0 {
            self.blend_factor = (self.blend_factor - dt / BLEND_DURATION).max(0.0);
        }
    }

    /// Set the local position directly (for collision response).
    pub fn set_local_position(&mut self, local_pos: Vec3) {
        self.local_position = local_pos;
    }

    /// Set the local velocity directly.
    pub fn set_local_velocity(&mut self, local_vel: Vec3) {
        self.local_velocity = local_vel;
    }

    /// Get the current world position of the attached entity.
    #[must_use]
    pub fn world_position(&self, frame: &ReferenceFrame) -> Vec3 {
        frame.local_to_world_position(self.local_position)
    }

    /// Get the current world velocity of the attached entity.
    ///
    /// During the blend period, smoothly transitions from pure world velocity
    /// to frame-relative velocity to avoid jitter.
    #[must_use]
    pub fn world_velocity(&self, frame: &ReferenceFrame) -> Vec3 {
        frame.local_to_world_velocity(self.local_position, self.local_velocity)
    }

    /// Detach from the frame, returning world position and velocity.
    ///
    /// The returned velocity fully inherits the frame's motion at the detachment point.
    #[must_use]
    pub fn detach(self, frame: &ReferenceFrame) -> DetachResult {
        let world_position = self.world_position(frame);
        let world_velocity = self.world_velocity(frame);

        DetachResult {
            world_position,
            world_velocity,
        }
    }

    /// Check if the attachment is still in the blending transition period.
    #[must_use]
    pub fn is_blending(&self) -> bool {
        self.blend_factor > 0.0
    }

    /// Get the blend factor (0 = fully attached, 1 = just attached).
    #[must_use]
    pub fn blend_factor(&self) -> f32 {
        self.blend_factor
    }

    /// Apply movement in local frame coordinates.
    ///
    /// Used by the character controller to update position relative to the frame.
    pub fn apply_local_movement(&mut self, delta_position: Vec3, new_local_velocity: Vec3) {
        self.local_position += delta_position;
        self.local_velocity = new_local_velocity;
    }

    /// Compute what the world-space movement delta would be for the given local delta.
    #[must_use]
    pub fn local_delta_to_world(&self, frame: &ReferenceFrame, local_delta: Vec3) -> Vec3 {
        frame.local_to_world_direction(local_delta)
    }
}

/// Result of detaching from a reference frame.
#[derive(Clone, Debug, PartialEq)]
pub struct DetachResult {
    /// Final world position at detachment.
    pub world_position: Vec3,
    /// Final world velocity (inherits frame motion).
    pub world_velocity: Vec3,
}

/// Manager for tracking multiple entity attachments to frames.
///
/// Useful when many characters or objects can ride on the same platform.
#[derive(Clone, Debug, Default)]
pub struct AttachmentSet {
    attachments: Vec<(u64, FrameAttachment)>,
}

impl AttachmentSet {
    /// Create a new empty attachment set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach an entity to a frame.
    pub fn attach(
        &mut self,
        entity_id: u64,
        frame: &ReferenceFrame,
        world_position: Vec3,
        world_velocity: Vec3,
    ) {
        self.remove(entity_id);
        let attachment = FrameAttachment::attach(frame, world_position, world_velocity);
        self.attachments.push((entity_id, attachment));
    }

    /// Detach an entity, returning the detach result if it was attached.
    pub fn detach(&mut self, entity_id: u64, frame: &ReferenceFrame) -> Option<DetachResult> {
        if let Some(idx) = self.attachments.iter().position(|(id, _)| *id == entity_id) {
            let (_, attachment) = self.attachments.remove(idx);
            Some(attachment.detach(frame))
        } else {
            None
        }
    }

    /// Remove an entity without computing detach result.
    fn remove(&mut self, entity_id: u64) {
        self.attachments.retain(|(id, _)| *id != entity_id);
    }

    /// Get an attachment by entity ID.
    #[must_use]
    pub fn get(&self, entity_id: u64) -> Option<&FrameAttachment> {
        self.attachments
            .iter()
            .find(|(id, _)| *id == entity_id)
            .map(|(_, a)| a)
    }

    /// Get a mutable attachment by entity ID.
    #[must_use]
    pub fn get_mut(&mut self, entity_id: u64) -> Option<&mut FrameAttachment> {
        self.attachments
            .iter_mut()
            .find(|(id, _)| *id == entity_id)
            .map(|(_, a)| a)
    }

    /// Update all attachments for one time step.
    pub fn update_all(&mut self, dt: f32) {
        for (_, attachment) in &mut self.attachments {
            attachment.update(dt);
        }
    }

    /// Check if an entity is attached.
    #[must_use]
    pub fn is_attached(&self, entity_id: u64) -> bool {
        self.attachments.iter().any(|(id, _)| *id == entity_id)
    }

    /// Get the number of attached entities.
    #[must_use]
    pub fn len(&self) -> usize {
        self.attachments.len()
    }

    /// Check if there are no attachments.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.attachments.is_empty()
    }

    /// Iterate over all attachments.
    pub fn iter(&self) -> impl Iterator<Item = (u64, &FrameAttachment)> {
        self.attachments.iter().map(|(id, a)| (*id, a))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Quat;
    use std::f32::consts::PI;

    const EPSILON: f32 = 1e-5;

    fn approx_eq_vec3(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < EPSILON
    }

    #[test]
    fn attach_to_stationary_frame() {
        let frame = ReferenceFrame::at_position(Vec3::new(10.0, 0.0, 0.0));
        let world_pos = Vec3::new(12.0, 0.0, 0.0);
        let world_vel = Vec3::new(0.0, 5.0, 0.0);

        let attachment = FrameAttachment::attach(&frame, world_pos, world_vel);

        assert!(approx_eq_vec3(
            attachment.local_position,
            Vec3::new(2.0, 0.0, 0.0)
        ));
        assert!(approx_eq_vec3(
            attachment.local_velocity,
            Vec3::new(0.0, 5.0, 0.0)
        ));
    }

    #[test]
    fn attach_to_moving_frame() {
        let frame = ReferenceFrame::with_velocity(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
        let world_pos = Vec3::ZERO;
        let world_vel = Vec3::new(15.0, 0.0, 0.0);

        let attachment = FrameAttachment::attach(&frame, world_pos, world_vel);

        assert!(approx_eq_vec3(
            attachment.local_velocity,
            Vec3::new(5.0, 0.0, 0.0)
        ));
    }

    #[test]
    fn world_position_from_attachment() {
        let frame = ReferenceFrame::at_position(Vec3::new(10.0, 0.0, 0.0));
        let attachment = FrameAttachment {
            local_position: Vec3::new(2.0, 3.0, 0.0),
            ..Default::default()
        };

        let world = attachment.world_position(&frame);
        assert!(approx_eq_vec3(world, Vec3::new(12.0, 3.0, 0.0)));
    }

    #[test]
    fn world_velocity_from_attachment() {
        let frame = ReferenceFrame::with_velocity(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
        let attachment = FrameAttachment {
            local_position: Vec3::ZERO,
            local_velocity: Vec3::new(5.0, 0.0, 0.0),
            ..Default::default()
        };

        let world_vel = attachment.world_velocity(&frame);
        assert!(approx_eq_vec3(world_vel, Vec3::new(15.0, 0.0, 0.0)));
    }

    #[test]
    fn detach_inherits_frame_velocity() {
        let frame = ReferenceFrame::with_velocity(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
        let attachment = FrameAttachment {
            local_position: Vec3::ZERO,
            local_velocity: Vec3::new(5.0, 3.0, 0.0),
            attached_time: 1.0,
            blend_factor: 0.0,
        };

        let result = attachment.detach(&frame);

        assert!(approx_eq_vec3(
            result.world_velocity,
            Vec3::new(15.0, 3.0, 0.0)
        ));
    }

    #[test]
    fn blend_factor_decreases_over_time() {
        let frame = ReferenceFrame::IDENTITY;
        let mut attachment = FrameAttachment::attach(&frame, Vec3::ZERO, Vec3::ZERO);

        assert!(attachment.is_blending());

        for _ in 0..10 {
            attachment.update(BLEND_DURATION / 5.0);
        }

        assert!(!attachment.is_blending());
    }

    #[test]
    fn rotated_frame_attachment() {
        let frame = ReferenceFrame::at_position(Vec3::ZERO)
            .with_orientation(Quat::from_rotation_y(PI / 2.0));

        let world_pos = Vec3::new(0.0, 0.0, -5.0);
        let attachment = FrameAttachment::attach(&frame, world_pos, Vec3::ZERO);

        assert!(approx_eq_vec3(
            attachment.local_position,
            Vec3::new(5.0, 0.0, 0.0)
        ));
    }

    #[test]
    fn apply_local_movement() {
        let mut attachment = FrameAttachment::default();
        attachment.apply_local_movement(Vec3::new(1.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0));

        assert!(approx_eq_vec3(
            attachment.local_position,
            Vec3::new(1.0, 0.0, 0.0)
        ));
        assert!(approx_eq_vec3(
            attachment.local_velocity,
            Vec3::new(2.0, 0.0, 0.0)
        ));
    }

    #[test]
    fn attachment_set_basic_operations() {
        let frame = ReferenceFrame::at_position(Vec3::ZERO);
        let mut set = AttachmentSet::new();

        assert!(set.is_empty());

        set.attach(1, &frame, Vec3::X, Vec3::ZERO);
        set.attach(2, &frame, Vec3::Y, Vec3::ZERO);

        assert_eq!(set.len(), 2);
        assert!(set.is_attached(1));
        assert!(set.is_attached(2));
        assert!(!set.is_attached(3));

        let result = set.detach(1, &frame);
        assert!(result.is_some());
        assert!(!set.is_attached(1));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn attachment_set_update_all() {
        let frame = ReferenceFrame::at_position(Vec3::ZERO);
        let mut set = AttachmentSet::new();

        set.attach(1, &frame, Vec3::ZERO, Vec3::ZERO);
        set.attach(2, &frame, Vec3::ZERO, Vec3::ZERO);

        for (_, attachment) in set.iter() {
            assert!(attachment.is_blending());
        }

        for _ in 0..10 {
            set.update_all(BLEND_DURATION / 5.0);
        }

        for (_, attachment) in set.iter() {
            assert!(!attachment.is_blending());
        }
    }
}
