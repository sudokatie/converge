//! Portal transform primitives for non-euclidean traversal.

use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Transform applied when traversing through a portal.
///
/// Represents the spatial relationship between two portal endpoints.
/// When an entity crosses a portal, its position and orientation are
/// transformed by this matrix.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PortalTransform {
    /// The combined rotation/scale/translation matrix.
    matrix: Mat4,
    /// The inverse transform for reverse traversal.
    inverse: Mat4,
    /// Whether this transform includes non-uniform scaling.
    has_scale: bool,
    /// Whether this transform includes reflection (negative determinant).
    has_reflection: bool,
}

impl PortalTransform {
    /// Create an identity transform (no change when traversing).
    #[must_use]
    pub fn identity() -> Self {
        Self {
            matrix: Mat4::IDENTITY,
            inverse: Mat4::IDENTITY,
            has_scale: false,
            has_reflection: false,
        }
    }

    /// Create a transform from translation only.
    #[must_use]
    pub fn from_translation(translation: Vec3) -> Self {
        Self {
            matrix: Mat4::from_translation(translation),
            inverse: Mat4::from_translation(-translation),
            has_scale: false,
            has_reflection: false,
        }
    }

    /// Create a transform from rotation only.
    #[must_use]
    pub fn from_rotation(rotation: Quat) -> Self {
        let inv_rot = rotation.inverse();
        Self {
            matrix: Mat4::from_quat(rotation),
            inverse: Mat4::from_quat(inv_rot),
            has_scale: false,
            has_reflection: false,
        }
    }

    /// Create a transform from rotation and translation.
    #[must_use]
    pub fn from_rotation_translation(rotation: Quat, translation: Vec3) -> Self {
        let inv_rot = rotation.inverse();
        let inv_trans = inv_rot * -translation;
        Self {
            matrix: Mat4::from_rotation_translation(rotation, translation),
            inverse: Mat4::from_rotation_translation(inv_rot, inv_trans),
            has_scale: false,
            has_reflection: false,
        }
    }

    /// Create a transform from scale, rotation, and translation.
    #[must_use]
    pub fn from_scale_rotation_translation(scale: Vec3, rotation: Quat, translation: Vec3) -> Self {
        let has_scale = (scale - Vec3::ONE).length_squared() > 1e-6;
        let det = scale.x * scale.y * scale.z;
        let has_reflection = det < 0.0;

        let inv_scale = Vec3::new(1.0 / scale.x, 1.0 / scale.y, 1.0 / scale.z);
        let inv_rot = rotation.inverse();
        let inv_trans = inv_rot * (-translation * inv_scale);

        Self {
            matrix: Mat4::from_scale_rotation_translation(scale, rotation, translation),
            inverse: Mat4::from_scale_rotation_translation(inv_scale, inv_rot, inv_trans),
            has_scale,
            has_reflection,
        }
    }

    /// Create a transform from a raw matrix.
    ///
    /// Computes the inverse automatically. Returns None if the matrix is not invertible.
    #[must_use]
    pub fn from_matrix(matrix: Mat4) -> Option<Self> {
        let inverse = matrix.inverse();
        if !inverse.is_finite() {
            return None;
        }

        let det = matrix.determinant();
        let scale_factor = det.abs().cbrt();
        let has_scale = (scale_factor - 1.0).abs() > 1e-4;
        let has_reflection = det < 0.0;

        Some(Self {
            matrix,
            inverse,
            has_scale,
            has_reflection,
        })
    }

    /// Create a 180-degree rotation around the Y axis.
    ///
    /// Common for portals that face opposite directions.
    #[must_use]
    pub fn flip_facing() -> Self {
        Self::from_rotation(Quat::from_rotation_y(std::f32::consts::PI))
    }

    /// Create a portal transform between two positions and orientations.
    ///
    /// The transform maps from the first portal frame to the second,
    /// automatically including the 180-degree flip for face-to-face portals.
    #[must_use]
    pub fn between_frames(
        pos_a: Vec3,
        forward_a: Vec3,
        up_a: Vec3,
        pos_b: Vec3,
        forward_b: Vec3,
        up_b: Vec3,
    ) -> Self {
        let right_a = forward_a.cross(up_a).normalize();
        let right_b = forward_b.cross(up_b).normalize();

        let mat_a = Mat4::from_cols(
            right_a.extend(0.0),
            up_a.extend(0.0),
            forward_a.extend(0.0),
            pos_a.extend(1.0),
        );

        let mat_b = Mat4::from_cols(
            (-right_b).extend(0.0),
            up_b.extend(0.0),
            (-forward_b).extend(0.0),
            pos_b.extend(1.0),
        );

        let inverse_a = mat_a.inverse();
        let transform = mat_b * inverse_a;

        Self::from_matrix(transform).unwrap_or_else(Self::identity)
    }

    /// Get the forward transformation matrix.
    #[must_use]
    pub const fn matrix(&self) -> Mat4 {
        self.matrix
    }

    /// Get the inverse transformation matrix.
    #[must_use]
    pub const fn inverse(&self) -> Mat4 {
        self.inverse
    }

    /// Check if this transform includes scaling.
    #[must_use]
    pub const fn has_scale(&self) -> bool {
        self.has_scale
    }

    /// Check if this transform includes reflection.
    #[must_use]
    pub const fn has_reflection(&self) -> bool {
        self.has_reflection
    }

    /// Transform a position through the portal.
    #[must_use]
    pub fn transform_point(&self, point: Vec3) -> Vec3 {
        self.matrix.transform_point3(point)
    }

    /// Transform a direction vector through the portal.
    #[must_use]
    pub fn transform_direction(&self, direction: Vec3) -> Vec3 {
        self.matrix.transform_vector3(direction)
    }

    /// Transform a normal vector through the portal.
    ///
    /// Uses the inverse transpose for correct normal transformation.
    #[must_use]
    pub fn transform_normal(&self, normal: Vec3) -> Vec3 {
        self.inverse
            .transpose()
            .transform_vector3(normal)
            .normalize()
    }

    /// Apply the inverse transform to a position.
    #[must_use]
    pub fn inverse_transform_point(&self, point: Vec3) -> Vec3 {
        self.inverse.transform_point3(point)
    }

    /// Apply the inverse transform to a direction.
    #[must_use]
    pub fn inverse_transform_direction(&self, direction: Vec3) -> Vec3 {
        self.inverse.transform_vector3(direction)
    }

    /// Combine two transforms (self applied first, then other).
    #[must_use]
    pub fn then(&self, other: &Self) -> Self {
        let matrix = other.matrix * self.matrix;
        let inverse = self.inverse * other.inverse;
        Self {
            matrix,
            inverse,
            has_scale: self.has_scale || other.has_scale,
            has_reflection: self.has_reflection != other.has_reflection,
        }
    }

    /// Get the inverted transform.
    #[must_use]
    pub fn inverted(&self) -> Self {
        Self {
            matrix: self.inverse,
            inverse: self.matrix,
            has_scale: self.has_scale,
            has_reflection: self.has_reflection,
        }
    }

    /// Extract the translation component.
    #[must_use]
    pub fn translation(&self) -> Vec3 {
        Vec3::new(
            self.matrix.w_axis.x,
            self.matrix.w_axis.y,
            self.matrix.w_axis.z,
        )
    }

    /// Approximate equality check for deterministic verification.
    #[must_use]
    pub fn approx_eq(&self, other: &Self, epsilon: f32) -> bool {
        let diff = (self.matrix - other.matrix).abs();
        diff.x_axis.max_element() < epsilon
            && diff.y_axis.max_element() < epsilon
            && diff.z_axis.max_element() < epsilon
            && diff.w_axis.max_element() < epsilon
    }
}

impl Default for PortalTransform {
    fn default() -> Self {
        Self::identity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn identity_preserves_position() {
        let transform = PortalTransform::identity();
        let point = Vec3::new(1.0, 2.0, 3.0);
        let result = transform.transform_point(point);
        assert_relative_eq!(result.x, point.x, epsilon = 1e-5);
        assert_relative_eq!(result.y, point.y, epsilon = 1e-5);
        assert_relative_eq!(result.z, point.z, epsilon = 1e-5);
    }

    #[test]
    fn translation_moves_point() {
        let transform = PortalTransform::from_translation(Vec3::new(10.0, 0.0, 0.0));
        let point = Vec3::ZERO;
        let result = transform.transform_point(point);
        assert_relative_eq!(result.x, 10.0, epsilon = 1e-5);
    }

    #[test]
    fn inverse_roundtrip() {
        let transform = PortalTransform::from_rotation_translation(
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_4),
            Vec3::new(5.0, 10.0, 15.0),
        );
        let point = Vec3::new(1.0, 2.0, 3.0);
        let transformed = transform.transform_point(point);
        let recovered = transform.inverse_transform_point(transformed);
        assert_relative_eq!(recovered.x, point.x, epsilon = 1e-4);
        assert_relative_eq!(recovered.y, point.y, epsilon = 1e-4);
        assert_relative_eq!(recovered.z, point.z, epsilon = 1e-4);
    }

    #[test]
    fn flip_facing_rotates_180() {
        let transform = PortalTransform::flip_facing();
        let forward = Vec3::Z;
        let result = transform.transform_direction(forward);
        assert_relative_eq!(result.x, 0.0, epsilon = 1e-5);
        assert_relative_eq!(result.z, -1.0, epsilon = 1e-5);
    }

    #[test]
    fn chain_transforms() {
        let t1 = PortalTransform::from_translation(Vec3::new(10.0, 0.0, 0.0));
        let t2 = PortalTransform::from_translation(Vec3::new(0.0, 5.0, 0.0));
        let combined = t1.then(&t2);
        let point = Vec3::ZERO;
        let result = combined.transform_point(point);
        assert_relative_eq!(result.x, 10.0, epsilon = 1e-5);
        assert_relative_eq!(result.y, 5.0, epsilon = 1e-5);
    }

    #[test]
    fn between_frames_face_to_face() {
        let pos_a = Vec3::ZERO;
        let forward_a = Vec3::Z;
        let up_a = Vec3::Y;
        let pos_b = Vec3::new(100.0, 0.0, 0.0);
        let forward_b = -Vec3::Z;
        let up_b = Vec3::Y;

        let transform =
            PortalTransform::between_frames(pos_a, forward_a, up_a, pos_b, forward_b, up_b);

        let result = transform.transform_point(Vec3::ZERO);
        assert_relative_eq!(result.x, 100.0, epsilon = 1e-4);
    }

    #[test]
    fn scale_detection() {
        let uniform = PortalTransform::from_scale_rotation_translation(
            Vec3::splat(2.0),
            Quat::IDENTITY,
            Vec3::ZERO,
        );
        assert!(uniform.has_scale());

        let no_scale = PortalTransform::from_translation(Vec3::ONE);
        assert!(!no_scale.has_scale());
    }

    #[test]
    fn reflection_detection() {
        let reflected = PortalTransform::from_scale_rotation_translation(
            Vec3::new(-1.0, 1.0, 1.0),
            Quat::IDENTITY,
            Vec3::ZERO,
        );
        assert!(reflected.has_reflection());

        let normal = PortalTransform::identity();
        assert!(!normal.has_reflection());
    }

    #[test]
    fn serde_roundtrip() {
        let transform = PortalTransform::from_rotation_translation(
            Quat::from_rotation_y(0.5),
            Vec3::new(1.0, 2.0, 3.0),
        );
        let serialized = bincode::serialize(&transform).unwrap();
        let deserialized: PortalTransform = bincode::deserialize(&serialized).unwrap();
        assert!(transform.approx_eq(&deserialized, 1e-6));
    }

    #[test]
    fn approx_eq_works() {
        let t1 = PortalTransform::from_translation(Vec3::new(1.0, 2.0, 3.0));
        let t2 = PortalTransform::from_translation(Vec3::new(1.0001, 2.0, 3.0));
        assert!(t1.approx_eq(&t2, 0.001));
        assert!(!t1.approx_eq(&t2, 0.00001));
    }

    #[test]
    fn normal_transformation() {
        let transform = PortalTransform::from_scale_rotation_translation(
            Vec3::new(2.0, 1.0, 1.0),
            Quat::IDENTITY,
            Vec3::ZERO,
        );
        let normal = Vec3::X;
        let result = transform.transform_normal(normal);
        assert_relative_eq!(result.length(), 1.0, epsilon = 1e-5);
    }
}
