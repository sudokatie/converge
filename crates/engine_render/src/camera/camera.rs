//! 3D camera with projection and view matrices.

use engine_core::math::Frustum;
use glam::{Mat4, Quat, Vec3};

/// Default field of view in radians (70 degrees).
const DEFAULT_FOV: f32 = 70.0 * std::f32::consts::PI / 180.0;

/// Default near plane distance.
const DEFAULT_NEAR: f32 = 0.1;

/// Default far plane distance.
const DEFAULT_FAR: f32 = 1000.0;

/// 3D camera for rendering.
#[derive(Debug, Clone)]
pub struct Camera {
    /// World position.
    pub position: Vec3,
    /// Rotation (orientation).
    pub rotation: Quat,
    /// Vertical field of view in radians.
    pub fov: f32,
    /// Near clip plane distance.
    pub near: f32,
    /// Far clip plane distance.
    pub far: f32,
}

impl Camera {
    /// Create a new camera at the origin looking down -Z.
    #[must_use]
    pub fn new() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            fov: DEFAULT_FOV,
            near: DEFAULT_NEAR,
            far: DEFAULT_FAR,
        }
    }

    /// Create a camera at a specific position looking at a target.
    #[must_use]
    pub fn look_at(position: Vec3, target: Vec3, up: Vec3) -> Self {
        let forward = (target - position).normalize();
        let right = forward.cross(up).normalize();
        let actual_up = right.cross(forward);

        // Build rotation from basis vectors
        let rotation_matrix = Mat4::from_cols(
            right.extend(0.0),
            actual_up.extend(0.0),
            (-forward).extend(0.0),
            Vec3::ZERO.extend(1.0),
        );

        Self {
            position,
            rotation: Quat::from_mat4(&rotation_matrix),
            fov: DEFAULT_FOV,
            near: DEFAULT_NEAR,
            far: DEFAULT_FAR,
        }
    }

    /// Get the view matrix (world to camera transform).
    #[must_use]
    pub fn view_matrix(&self) -> Mat4 {
        let rotation_matrix = Mat4::from_quat(self.rotation.conjugate());
        let translation_matrix = Mat4::from_translation(-self.position);
        rotation_matrix * translation_matrix
    }

    /// Get the projection matrix.
    #[must_use]
    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh(self.fov, aspect, self.near, self.far)
    }

    /// Get the combined view-projection matrix.
    #[must_use]
    pub fn view_projection(&self, aspect: f32) -> Mat4 {
        self.projection_matrix(aspect) * self.view_matrix()
    }

    /// Get the view frustum for culling.
    #[must_use]
    pub fn frustum(&self, aspect: f32) -> Frustum {
        Frustum::from_view_projection(self.view_projection(aspect))
    }

    /// Get the forward direction (where the camera is looking).
    #[must_use]
    pub fn forward(&self) -> Vec3 {
        self.rotation * Vec3::NEG_Z
    }

    /// Get the right direction.
    #[must_use]
    pub fn right(&self) -> Vec3 {
        self.rotation * Vec3::X
    }

    /// Get the up direction.
    #[must_use]
    pub fn up(&self) -> Vec3 {
        self.rotation * Vec3::Y
    }

    /// Rotate the camera by pitch (up/down) and yaw (left/right).
    pub fn rotate(&mut self, pitch: f32, yaw: f32) {
        // Apply yaw around world Y axis
        let yaw_rotation = Quat::from_rotation_y(yaw);
        // Apply pitch around local X axis
        let pitch_rotation = Quat::from_rotation_x(pitch);
        self.rotation = yaw_rotation * self.rotation * pitch_rotation;
    }

    /// Move the camera in a direction (relative to its orientation).
    pub fn translate(&mut self, direction: Vec3) {
        self.position += self.rotation * direction;
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_default_camera_at_origin() {
        let camera = Camera::new();
        assert_eq!(camera.position, Vec3::ZERO);
    }

    #[test]
    fn test_default_camera_looks_neg_z() {
        let camera = Camera::new();
        let forward = camera.forward();
        assert_relative_eq!(forward.x, 0.0, epsilon = 0.001);
        assert_relative_eq!(forward.y, 0.0, epsilon = 0.001);
        assert_relative_eq!(forward.z, -1.0, epsilon = 0.001);
    }

    #[test]
    fn test_view_matrix_inverse_of_transform() {
        let mut camera = Camera::new();
        camera.position = Vec3::new(1.0, 2.0, 3.0);

        let view = camera.view_matrix();
        let model = Mat4::from_rotation_translation(camera.rotation, camera.position);

        // View * Model should be identity for camera's own transform
        let product = view * model;
        assert_relative_eq!(product.x_axis.x, 1.0, epsilon = 0.001);
        assert_relative_eq!(product.y_axis.y, 1.0, epsilon = 0.001);
        assert_relative_eq!(product.z_axis.z, 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_look_at() {
        let camera = Camera::look_at(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::Y);
        let forward = camera.forward();
        assert_relative_eq!(forward.z, -1.0, epsilon = 0.001);
    }

    #[test]
    fn test_right_up_forward_orthogonal() {
        let camera = Camera::new();
        let f = camera.forward();
        let r = camera.right();
        let u = camera.up();

        // Check orthogonality
        assert_relative_eq!(f.dot(r), 0.0, epsilon = 0.001);
        assert_relative_eq!(f.dot(u), 0.0, epsilon = 0.001);
        assert_relative_eq!(r.dot(u), 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_frustum_extraction() {
        let camera = Camera::new();
        let frustum = camera.frustum(16.0 / 9.0);
        // Just check it doesn't panic and frustum exists
        assert!(frustum.contains_point(Vec3::new(0.0, 0.0, -1.0)));
    }
}
