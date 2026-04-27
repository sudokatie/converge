//! Frame hierarchy resolver for nested reference frames.

use std::collections::HashMap;

use glam::Vec3;

use super::ReferenceFrame;

/// Maximum depth for nested frame hierarchies to prevent infinite loops.
const MAX_HIERARCHY_DEPTH: usize = 8;

/// Resolves nested reference frame hierarchies to compute effective world-space transforms.
///
/// Frames can have parent frames, forming a hierarchy. This resolver computes
/// the effective (composed) frame that accounts for all parent motion.
#[derive(Clone, Debug, Default)]
pub struct FrameResolver {
    frames: HashMap<u64, ReferenceFrame>,
}

impl FrameResolver {
    /// Create a new empty frame resolver.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or update a frame.
    pub fn insert(&mut self, id: u64, frame: ReferenceFrame) {
        self.frames.insert(id, frame);
    }

    /// Remove a frame by ID.
    pub fn remove(&mut self, id: u64) -> Option<ReferenceFrame> {
        self.frames.remove(&id)
    }

    /// Get a frame by ID.
    #[must_use]
    pub fn get(&self, id: u64) -> Option<&ReferenceFrame> {
        self.frames.get(&id)
    }

    /// Get a mutable frame by ID.
    #[must_use]
    pub fn get_mut(&mut self, id: u64) -> Option<&mut ReferenceFrame> {
        self.frames.get_mut(&id)
    }

    /// Check if a frame exists.
    #[must_use]
    pub fn contains(&self, id: u64) -> bool {
        self.frames.contains_key(&id)
    }

    /// Get the number of frames.
    #[must_use]
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Resolve a frame's effective world-space state by composing with all ancestors.
    ///
    /// Returns `None` if the frame ID doesn't exist or there's a broken parent chain.
    #[must_use]
    pub fn resolve(&self, id: u64) -> Option<ReferenceFrame> {
        let frame = self.frames.get(&id)?;
        self.resolve_recursive(frame, 0)
    }

    fn resolve_recursive(&self, frame: &ReferenceFrame, depth: usize) -> Option<ReferenceFrame> {
        if depth >= MAX_HIERARCHY_DEPTH {
            return None;
        }

        let Some(parent_id) = frame.parent else {
            return Some(frame.clone());
        };

        let parent = self.frames.get(&parent_id)?;
        let resolved_parent = self.resolve_recursive(parent, depth + 1)?;

        Some(compose_frames(&resolved_parent, frame))
    }

    /// Transform a position from a frame's local space to world space,
    /// accounting for the full parent hierarchy.
    #[must_use]
    pub fn local_to_world_position(&self, frame_id: u64, local_pos: Vec3) -> Option<Vec3> {
        let resolved = self.resolve(frame_id)?;
        Some(resolved.local_to_world_position(local_pos))
    }

    /// Transform a position from world space to a frame's local space,
    /// accounting for the full parent hierarchy.
    #[must_use]
    pub fn world_to_local_position(&self, frame_id: u64, world_pos: Vec3) -> Option<Vec3> {
        let resolved = self.resolve(frame_id)?;
        Some(resolved.world_to_local_position(world_pos))
    }

    /// Get the effective velocity at a world point for a frame, accounting for hierarchy.
    #[must_use]
    pub fn velocity_at_point(&self, frame_id: u64, world_point: Vec3) -> Option<Vec3> {
        let resolved = self.resolve(frame_id)?;
        Some(resolved.velocity_at_point(world_point))
    }

    /// Update all frames by one time step.
    pub fn integrate_all(&mut self, dt: f32) {
        for frame in self.frames.values_mut() {
            frame.integrate(dt);
        }
    }

    /// Iterate over all frames.
    pub fn iter(&self) -> impl Iterator<Item = (u64, &ReferenceFrame)> {
        self.frames.iter().map(|(id, f)| (*id, f))
    }

    /// Clear all frames.
    pub fn clear(&mut self) {
        self.frames.clear();
    }
}

/// Compose two frames: child's motion expressed in parent's reference frame.
///
/// The result is a frame whose origin, velocity, etc. are in world coordinates,
/// representing the child frame's full world-space state when attached to the parent.
fn compose_frames(parent: &ReferenceFrame, child: &ReferenceFrame) -> ReferenceFrame {
    let origin = parent.local_to_world_position(child.origin);
    let orientation = parent.orientation * child.orientation;

    let child_origin_velocity = parent.velocity_at_point(origin);
    let child_linear_in_world = parent.orientation * child.linear_velocity;
    let linear_velocity = child_origin_velocity + child_linear_in_world;

    let angular_velocity = parent.angular_velocity + parent.orientation * child.angular_velocity;

    let child_accel_in_world = parent.orientation * child.linear_acceleration;
    let linear_acceleration = parent.linear_acceleration + child_accel_in_world;

    ReferenceFrame {
        origin,
        orientation,
        linear_velocity,
        angular_velocity,
        linear_acceleration,
        parent: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Quat;
    use std::f32::consts::PI;

    const EPSILON: f32 = 1e-4;

    fn approx_eq_vec3(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < EPSILON
    }

    #[test]
    fn resolve_single_frame() {
        let mut resolver = FrameResolver::new();
        let frame = ReferenceFrame::at_position(Vec3::new(10.0, 0.0, 0.0));
        resolver.insert(1, frame.clone());

        let result = resolver.resolve(1).unwrap();
        assert!(approx_eq_vec3(result.origin, frame.origin));
    }

    #[test]
    fn resolve_parent_child_translation() {
        let mut resolver = FrameResolver::new();

        let parent = ReferenceFrame::at_position(Vec3::new(10.0, 0.0, 0.0));
        resolver.insert(1, parent);

        let child = ReferenceFrame::at_position(Vec3::new(5.0, 0.0, 0.0)).with_parent(1);
        resolver.insert(2, child);

        let result = resolver.resolve(2).unwrap();
        assert!(approx_eq_vec3(result.origin, Vec3::new(15.0, 0.0, 0.0)));
    }

    #[test]
    fn resolve_parent_child_rotation() {
        let mut resolver = FrameResolver::new();

        let parent = ReferenceFrame::at_position(Vec3::ZERO)
            .with_orientation(Quat::from_rotation_y(PI / 2.0));
        resolver.insert(1, parent);

        let child = ReferenceFrame::at_position(Vec3::new(5.0, 0.0, 0.0)).with_parent(1);
        resolver.insert(2, child);

        let result = resolver.resolve(2).unwrap();
        assert!(approx_eq_vec3(result.origin, Vec3::new(0.0, 0.0, -5.0)));
    }

    #[test]
    fn resolve_velocity_composition() {
        let mut resolver = FrameResolver::new();

        let parent = ReferenceFrame::with_velocity(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
        resolver.insert(1, parent);

        let child =
            ReferenceFrame::with_velocity(Vec3::ZERO, Vec3::new(5.0, 0.0, 0.0)).with_parent(1);
        resolver.insert(2, child);

        let result = resolver.resolve(2).unwrap();
        assert!(approx_eq_vec3(
            result.linear_velocity,
            Vec3::new(15.0, 0.0, 0.0)
        ));
    }

    #[test]
    fn resolve_rotated_parent_velocity() {
        let mut resolver = FrameResolver::new();

        let parent = ReferenceFrame::at_position(Vec3::ZERO)
            .with_orientation(Quat::from_rotation_y(PI / 2.0));
        resolver.insert(1, parent);

        let child =
            ReferenceFrame::with_velocity(Vec3::ZERO, Vec3::new(5.0, 0.0, 0.0)).with_parent(1);
        resolver.insert(2, child);

        let result = resolver.resolve(2).unwrap();
        assert!(approx_eq_vec3(
            result.linear_velocity,
            Vec3::new(0.0, 0.0, -5.0)
        ));
    }

    #[test]
    fn resolve_missing_frame() {
        let resolver = FrameResolver::new();
        assert!(resolver.resolve(999).is_none());
    }

    #[test]
    fn resolve_broken_parent_chain() {
        let mut resolver = FrameResolver::new();

        let child = ReferenceFrame::at_position(Vec3::X).with_parent(999);
        resolver.insert(1, child);

        assert!(resolver.resolve(1).is_none());
    }

    #[test]
    fn resolve_deep_hierarchy() {
        let mut resolver = FrameResolver::new();

        resolver.insert(1, ReferenceFrame::at_position(Vec3::new(1.0, 0.0, 0.0)));

        for i in 2..=5u64 {
            let frame = ReferenceFrame::at_position(Vec3::new(1.0, 0.0, 0.0)).with_parent(i - 1);
            resolver.insert(i, frame);
        }

        let result = resolver.resolve(5).unwrap();
        assert!(approx_eq_vec3(result.origin, Vec3::new(5.0, 0.0, 0.0)));
    }

    #[test]
    fn local_to_world_through_resolver() {
        let mut resolver = FrameResolver::new();

        let parent = ReferenceFrame::at_position(Vec3::new(10.0, 0.0, 0.0));
        resolver.insert(1, parent);

        let child = ReferenceFrame::at_position(Vec3::new(5.0, 0.0, 0.0)).with_parent(1);
        resolver.insert(2, child);

        let world = resolver.local_to_world_position(2, Vec3::new(1.0, 0.0, 0.0));
        assert!(approx_eq_vec3(world.unwrap(), Vec3::new(16.0, 0.0, 0.0)));
    }

    #[test]
    fn velocity_at_point_through_resolver() {
        let mut resolver = FrameResolver::new();

        let frame = ReferenceFrame::with_velocity(Vec3::ZERO, Vec3::new(5.0, 0.0, 0.0));
        resolver.insert(1, frame);

        let vel = resolver.velocity_at_point(1, Vec3::new(10.0, 0.0, 0.0));
        assert!(approx_eq_vec3(vel.unwrap(), Vec3::new(5.0, 0.0, 0.0)));
    }

    #[test]
    fn integrate_all_frames() {
        let mut resolver = FrameResolver::new();

        resolver.insert(
            1,
            ReferenceFrame::with_velocity(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0)),
        );
        resolver.insert(
            2,
            ReferenceFrame::with_velocity(Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0)),
        );

        resolver.integrate_all(1.0);

        let f1 = resolver.get(1).unwrap();
        let f2 = resolver.get(2).unwrap();

        assert!(approx_eq_vec3(f1.origin, Vec3::new(1.0, 0.0, 0.0)));
        assert!(approx_eq_vec3(f2.origin, Vec3::new(0.0, 1.0, 0.0)));
    }

    #[test]
    fn resolver_basic_operations() {
        let mut resolver = FrameResolver::new();
        assert!(resolver.is_empty());

        resolver.insert(1, ReferenceFrame::IDENTITY);
        resolver.insert(2, ReferenceFrame::IDENTITY);

        assert_eq!(resolver.len(), 2);
        assert!(resolver.contains(1));
        assert!(!resolver.contains(3));

        resolver.remove(1);
        assert!(!resolver.contains(1));
        assert_eq!(resolver.len(), 1);

        resolver.clear();
        assert!(resolver.is_empty());
    }
}
