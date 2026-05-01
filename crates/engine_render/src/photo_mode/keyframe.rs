//! Keyframe types for cinematic camera animation.
//!
//! Defines camera keyframes with position, rotation, and settings
//! that can be interpolated for smooth camera paths.

use super::easing::EasingFunction;
use super::settings::{PhotoFilter, PhotoSettings};
use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

/// A single camera keyframe in a cinematic path.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CameraKeyframe {
    /// Time in seconds from path start.
    pub time: f32,
    /// Camera position.
    pub position: Vec3,
    /// Camera rotation.
    pub rotation: Quat,
    /// Field of view in degrees.
    pub fov: f32,
    /// Camera roll in degrees.
    pub roll: f32,
    /// Focus distance.
    pub focus_distance: f32,
    /// Easing to next keyframe.
    pub easing: EasingFunction,
    /// Optional filter at this keyframe.
    pub filter: Option<PhotoFilter>,
    /// Optional look-at target.
    pub look_at: Option<Vec3>,
}

impl Default for CameraKeyframe {
    fn default() -> Self {
        Self {
            time: 0.0,
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            fov: 70.0,
            roll: 0.0,
            focus_distance: 10.0,
            easing: EasingFunction::SmoothStep,
            filter: None,
            look_at: None,
        }
    }
}

impl CameraKeyframe {
    /// Create a new keyframe at the specified time and position.
    #[must_use]
    pub fn new(time: f32, position: Vec3, rotation: Quat) -> Self {
        Self {
            time,
            position,
            rotation,
            ..Default::default()
        }
    }

    /// Create a keyframe that looks at a target.
    #[must_use]
    pub fn looking_at(time: f32, position: Vec3, target: Vec3, up: Vec3) -> Self {
        let direction = (target - position).normalize_or_zero();
        let rotation = if direction.length_squared() > 0.0 {
            Quat::from_rotation_arc(Vec3::NEG_Z, direction)
        } else {
            Quat::IDENTITY
        };

        Self {
            time,
            position,
            rotation,
            look_at: Some(target),
            ..Default::default()
        }
        .with_up_vector(up)
    }

    /// Set the rotation to align with an up vector.
    #[must_use]
    fn with_up_vector(mut self, up: Vec3) -> Self {
        let forward = self.rotation * Vec3::NEG_Z;
        let right = up.cross(forward).normalize_or_zero();
        if right.length_squared() > 0.0 {
            let corrected_up = forward.cross(right).normalize();
            self.rotation =
                Quat::from_mat3(&glam::Mat3::from_cols(right, corrected_up, -forward)).normalize();
        }
        self
    }

    /// Set the easing function.
    #[must_use]
    pub fn with_easing(mut self, easing: EasingFunction) -> Self {
        self.easing = easing;
        self
    }

    /// Set the field of view.
    #[must_use]
    pub fn with_fov(mut self, fov: f32) -> Self {
        self.fov = fov.clamp(10.0, 120.0);
        self
    }

    /// Set the camera roll.
    #[must_use]
    pub fn with_roll(mut self, roll: f32) -> Self {
        self.roll = roll.clamp(-180.0, 180.0);
        self
    }

    /// Set the focus distance.
    #[must_use]
    pub fn with_focus_distance(mut self, distance: f32) -> Self {
        self.focus_distance = distance.max(0.1);
        self
    }

    /// Set the filter.
    #[must_use]
    pub fn with_filter(mut self, filter: PhotoFilter) -> Self {
        self.filter = Some(filter);
        self
    }

    /// Set look-at target.
    #[must_use]
    pub fn with_look_at(mut self, target: Vec3) -> Self {
        self.look_at = Some(target);
        self
    }

    /// Validate the keyframe.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.time >= 0.0
            && self.position.is_finite()
            && self.rotation.is_normalized()
            && (10.0..=120.0).contains(&self.fov)
            && (-180.0..=180.0).contains(&self.roll)
            && self.focus_distance >= 0.1
    }

    /// Convert to photo settings.
    #[must_use]
    pub fn to_photo_settings(&self) -> PhotoSettings {
        PhotoSettings {
            fov: self.fov,
            roll: self.roll,
            focus_distance: self.focus_distance,
            filter: self.filter.unwrap_or(PhotoFilter::None),
            position: Some(self.position),
            rotation: Some(self.rotation),
            ..Default::default()
        }
    }
}

/// Result of interpolating between keyframes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InterpolatedCamera {
    /// Interpolated position.
    pub position: Vec3,
    /// Interpolated rotation.
    pub rotation: Quat,
    /// Interpolated field of view.
    pub fov: f32,
    /// Interpolated roll.
    pub roll: f32,
    /// Interpolated focus distance.
    pub focus_distance: f32,
    /// Active filter (from nearest keyframe).
    pub filter: PhotoFilter,
    /// Velocity at this point (for motion blur).
    pub velocity: Vec3,
    /// Angular velocity (for motion blur).
    pub angular_velocity: Vec3,
}

impl Default for InterpolatedCamera {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            fov: 70.0,
            roll: 0.0,
            focus_distance: 10.0,
            filter: PhotoFilter::None,
            velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
        }
    }
}

impl InterpolatedCamera {
    /// Create from a single keyframe.
    #[must_use]
    pub fn from_keyframe(kf: &CameraKeyframe) -> Self {
        Self {
            position: kf.position,
            rotation: kf.rotation,
            fov: kf.fov,
            roll: kf.roll,
            focus_distance: kf.focus_distance,
            filter: kf.filter.unwrap_or(PhotoFilter::None),
            velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
        }
    }

    /// Interpolate between two keyframes.
    #[must_use]
    pub fn interpolate(from: &CameraKeyframe, to: &CameraKeyframe, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let eased_t = from.easing.evaluate(t);

        let position = from.position.lerp(to.position, eased_t);
        let rotation = from.rotation.slerp(to.rotation, eased_t);
        let fov = lerp(from.fov, to.fov, eased_t);
        let roll = lerp_angle(from.roll, to.roll, eased_t);
        let focus_distance = lerp(from.focus_distance, to.focus_distance, eased_t);

        let filter = if t < 0.5 {
            from.filter.unwrap_or(PhotoFilter::None)
        } else {
            to.filter
                .unwrap_or(from.filter.unwrap_or(PhotoFilter::None))
        };

        let dt = (to.time - from.time).max(0.001);
        let velocity = (to.position - from.position) / dt;
        let angular_velocity = rotation_to_angular_velocity(from.rotation, to.rotation, dt);

        Self {
            position,
            rotation,
            fov,
            roll,
            focus_distance,
            filter,
            velocity,
            angular_velocity,
        }
    }

    /// Convert to photo settings.
    #[must_use]
    pub fn to_photo_settings(&self) -> PhotoSettings {
        PhotoSettings {
            fov: self.fov,
            roll: self.roll,
            focus_distance: self.focus_distance,
            filter: self.filter,
            position: Some(self.position),
            rotation: Some(self.rotation),
            ..Default::default()
        }
    }
}

/// Compute a stable fingerprint for a keyframe.
#[must_use]
pub fn compute_keyframe_fingerprint(kf: &CameraKeyframe) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hash_f32(kf.time, &mut hasher);
    hash_vec3(&kf.position, &mut hasher);
    hash_quat(&kf.rotation, &mut hasher);
    hash_f32(kf.fov, &mut hasher);
    hash_f32(kf.roll, &mut hasher);
    hash_f32(kf.focus_distance, &mut hasher);
    (kf.easing as u8).hash(&mut hasher);
    kf.filter.hash(&mut hasher);
    hasher.finish()
}

fn hash_f32(value: f32, hasher: &mut impl Hasher) {
    value.to_bits().hash(hasher);
}

fn hash_vec3(v: &Vec3, hasher: &mut impl Hasher) {
    hash_f32(v.x, hasher);
    hash_f32(v.y, hasher);
    hash_f32(v.z, hasher);
}

fn hash_quat(q: &Quat, hasher: &mut impl Hasher) {
    hash_f32(q.x, hasher);
    hash_f32(q.y, hasher);
    hash_f32(q.z, hasher);
    hash_f32(q.w, hasher);
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    let mut diff = b - a;
    while diff > 180.0 {
        diff -= 360.0;
    }
    while diff < -180.0 {
        diff += 360.0;
    }
    let result = a + diff * t;
    ((result % 360.0) + 360.0) % 360.0
}

fn rotation_to_angular_velocity(from: Quat, to: Quat, dt: f32) -> Vec3 {
    let delta = to * from.conjugate();
    let (axis, angle) = delta.to_axis_angle();
    if axis.is_finite() && angle.is_finite() {
        axis * (angle / dt)
    } else {
        Vec3::ZERO
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_default_keyframe() {
        let kf = CameraKeyframe::default();
        assert!(kf.is_valid());
        assert_relative_eq!(kf.time, 0.0);
        assert_eq!(kf.position, Vec3::ZERO);
    }

    #[test]
    fn test_keyframe_creation() {
        let kf = CameraKeyframe::new(1.0, Vec3::new(10.0, 5.0, 0.0), Quat::IDENTITY);
        assert_relative_eq!(kf.time, 1.0);
        assert_relative_eq!(kf.position.x, 10.0);
    }

    #[test]
    fn test_keyframe_builder() {
        let kf = CameraKeyframe::default()
            .with_fov(90.0)
            .with_roll(45.0)
            .with_easing(EasingFunction::CubicInOut)
            .with_filter(PhotoFilter::Cinematic);

        assert_relative_eq!(kf.fov, 90.0);
        assert_relative_eq!(kf.roll, 45.0);
        assert_eq!(kf.easing, EasingFunction::CubicInOut);
        assert_eq!(kf.filter, Some(PhotoFilter::Cinematic));
    }

    #[test]
    fn test_look_at_keyframe() {
        let kf = CameraKeyframe::looking_at(0.0, Vec3::new(0.0, 5.0, 10.0), Vec3::ZERO, Vec3::Y);

        assert!(kf.is_valid());
        assert!(kf.look_at.is_some());
    }

    #[test]
    fn test_interpolate_position() {
        let from = CameraKeyframe::new(0.0, Vec3::ZERO, Quat::IDENTITY);
        let to = CameraKeyframe::new(1.0, Vec3::new(10.0, 0.0, 0.0), Quat::IDENTITY);

        let mid = InterpolatedCamera::interpolate(&from, &to, 0.5);
        assert!(mid.position.x > 0.0 && mid.position.x < 10.0);
    }

    #[test]
    fn test_interpolate_at_endpoints() {
        let from = CameraKeyframe::new(0.0, Vec3::ZERO, Quat::IDENTITY).with_fov(60.0);
        let to = CameraKeyframe::new(1.0, Vec3::X * 10.0, Quat::IDENTITY).with_fov(90.0);

        let at_start = InterpolatedCamera::interpolate(&from, &to, 0.0);
        assert_relative_eq!(at_start.position.x, 0.0, epsilon = 0.01);

        let at_end = InterpolatedCamera::interpolate(&from, &to, 1.0);
        assert_relative_eq!(at_end.position.x, 10.0, epsilon = 0.01);
    }

    #[test]
    fn test_interpolate_rotation() {
        let from = CameraKeyframe::new(0.0, Vec3::ZERO, Quat::IDENTITY);
        let to = CameraKeyframe::new(1.0, Vec3::ZERO, Quat::from_rotation_y(std::f32::consts::PI));

        let mid = InterpolatedCamera::interpolate(&from, &to, 0.5);
        assert!(mid.rotation.is_normalized());
    }

    #[test]
    fn test_velocity_calculation() {
        let from = CameraKeyframe::new(0.0, Vec3::ZERO, Quat::IDENTITY);
        let to = CameraKeyframe::new(1.0, Vec3::new(10.0, 0.0, 0.0), Quat::IDENTITY);

        let result = InterpolatedCamera::interpolate(&from, &to, 0.5);
        assert_relative_eq!(result.velocity.x, 10.0, epsilon = 0.1);
    }

    #[test]
    fn test_keyframe_fingerprint_determinism() {
        let kf = CameraKeyframe::new(1.0, Vec3::new(1.0, 2.0, 3.0), Quat::IDENTITY);

        let fp1 = compute_keyframe_fingerprint(&kf);
        let fp2 = compute_keyframe_fingerprint(&kf);

        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_keyframe_fingerprint_sensitivity() {
        let kf1 = CameraKeyframe::new(1.0, Vec3::ZERO, Quat::IDENTITY);
        let kf2 = CameraKeyframe::new(2.0, Vec3::ZERO, Quat::IDENTITY);

        assert_ne!(
            compute_keyframe_fingerprint(&kf1),
            compute_keyframe_fingerprint(&kf2)
        );
    }

    #[test]
    fn test_to_photo_settings() {
        let kf = CameraKeyframe::default()
            .with_fov(85.0)
            .with_filter(PhotoFilter::Sepia);

        let settings = kf.to_photo_settings();
        assert_relative_eq!(settings.fov, 85.0);
        assert_eq!(settings.filter, PhotoFilter::Sepia);
    }

    #[test]
    fn test_lerp_angle_wrapping() {
        let result = lerp_angle(350.0, 10.0, 0.5);
        assert_relative_eq!(result, 0.0, epsilon = 0.1);

        let result = lerp_angle(-170.0, 170.0, 0.5);
        assert!(result.abs() > 170.0 || result.abs() < 10.0);
    }

    #[test]
    fn test_serde_roundtrip() {
        let kf = CameraKeyframe::new(1.5, Vec3::new(1.0, 2.0, 3.0), Quat::IDENTITY)
            .with_fov(90.0)
            .with_easing(EasingFunction::CubicOut);

        let bytes = bincode::serialize(&kf).expect("serialize");
        let restored: CameraKeyframe = bincode::deserialize(&bytes).expect("deserialize");

        assert_relative_eq!(kf.time, restored.time);
        assert_relative_eq!(kf.fov, restored.fov);
        assert_eq!(kf.easing, restored.easing);
    }
}
