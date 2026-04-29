//! Curvature body type definitions.
//!
//! Defines geometric bodies that induce curved-world rendering behavior:
//! planetary spheres, interior spheres (Dyson-like), cylinders/rings,
//! and large moving bodies (asteroids, stations).

use glam::{Mat3, Quat, Vec3};

/// Type of curved body geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CurvatureBodyKind {
    /// Standing on the outside of a sphere (planet surface).
    PlanetarySphere = 0,
    /// Standing on the inside of a sphere (Dyson sphere, enclosed world).
    InteriorSphere = 1,
    /// Standing on the inside of a cylinder (O'Neill cylinder, ring habitat).
    Cylinder = 2,
    /// Standing on the outside of a cylinder (asteroid, elongated station).
    ExteriorCylinder = 3,
    /// Large moving body with its own reference frame.
    MovingBody = 4,
}

impl CurvatureBodyKind {
    /// All curvature body kinds.
    pub const ALL: [Self; 5] = [
        Self::PlanetarySphere,
        Self::InteriorSphere,
        Self::Cylinder,
        Self::ExteriorCylinder,
        Self::MovingBody,
    ];

    /// Whether the observer is on the inside of the surface.
    #[must_use]
    pub fn is_interior(self) -> bool {
        matches!(self, Self::InteriorSphere | Self::Cylinder)
    }

    /// Whether the horizon curves upward (interior) or downward (exterior).
    #[must_use]
    pub fn horizon_curves_up(self) -> bool {
        self.is_interior()
    }

    /// Whether this body type has a cylindrical axis.
    #[must_use]
    pub fn has_axis(self) -> bool {
        matches!(self, Self::Cylinder | Self::ExteriorCylinder)
    }
}

/// Configuration for a curved body.
#[derive(Debug, Clone, Copy)]
pub struct CurvatureBody {
    /// Body type.
    pub kind: CurvatureBodyKind,
    /// Center of the body in world coordinates.
    pub center: Vec3,
    /// Radius of the body (for spheres) or cylinder radius.
    pub radius: f32,
    /// Cylinder half-length (only for cylinder types).
    pub half_length: f32,
    /// Axis direction (only for cylinders, normalized).
    pub axis: Vec3,
    /// Angular velocity for rotating bodies (radians/second around axis).
    pub angular_velocity: f32,
    /// Linear velocity for moving bodies (world units/second).
    pub velocity: Vec3,
    /// Surface gravity magnitude (m/s^2, for physics hints).
    pub surface_gravity: f32,
    /// Whether this body is currently active.
    pub active: bool,
}

impl Default for CurvatureBody {
    fn default() -> Self {
        Self {
            kind: CurvatureBodyKind::PlanetarySphere,
            center: Vec3::ZERO,
            radius: 6_371_000.0, // Earth-like radius in meters
            half_length: 0.0,
            axis: Vec3::Y,
            angular_velocity: 0.0,
            velocity: Vec3::ZERO,
            surface_gravity: 9.81,
            active: true,
        }
    }
}

impl CurvatureBody {
    /// Create a planetary sphere body.
    #[must_use]
    pub fn planetary_sphere(center: Vec3, radius: f32) -> Self {
        Self {
            kind: CurvatureBodyKind::PlanetarySphere,
            center,
            radius,
            ..Default::default()
        }
    }

    /// Create an interior sphere body (Dyson-like).
    #[must_use]
    pub fn interior_sphere(center: Vec3, radius: f32) -> Self {
        Self {
            kind: CurvatureBodyKind::InteriorSphere,
            center,
            radius,
            ..Default::default()
        }
    }

    /// Create a cylinder habitat (interior).
    #[must_use]
    pub fn cylinder(center: Vec3, radius: f32, half_length: f32, axis: Vec3) -> Self {
        Self {
            kind: CurvatureBodyKind::Cylinder,
            center,
            radius,
            half_length,
            axis: if axis.length_squared() > 0.0001 {
                axis.normalize()
            } else {
                Vec3::Y
            },
            ..Default::default()
        }
    }

    /// Create an exterior cylinder (asteroid).
    #[must_use]
    pub fn exterior_cylinder(center: Vec3, radius: f32, half_length: f32, axis: Vec3) -> Self {
        Self {
            kind: CurvatureBodyKind::ExteriorCylinder,
            center,
            radius,
            half_length,
            axis: if axis.length_squared() > 0.0001 {
                axis.normalize()
            } else {
                Vec3::Y
            },
            ..Default::default()
        }
    }

    /// Create a moving body with velocity.
    #[must_use]
    pub fn moving_body(center: Vec3, radius: f32, velocity: Vec3) -> Self {
        Self {
            kind: CurvatureBodyKind::MovingBody,
            center,
            radius,
            velocity,
            ..Default::default()
        }
    }

    /// Set angular velocity.
    #[must_use]
    pub fn with_angular_velocity(mut self, omega: f32) -> Self {
        self.angular_velocity = omega;
        self
    }

    /// Set linear velocity.
    #[must_use]
    pub fn with_velocity(mut self, velocity: Vec3) -> Self {
        self.velocity = velocity;
        self
    }

    /// Set surface gravity.
    #[must_use]
    pub fn with_surface_gravity(mut self, gravity: f32) -> Self {
        self.surface_gravity = gravity.max(0.0);
        self
    }

    /// Set active state.
    #[must_use]
    pub fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Compute the local "up" direction at a surface point.
    #[must_use]
    pub fn surface_up(&self, surface_point: Vec3) -> Vec3 {
        let local = surface_point - self.center;

        match self.kind {
            CurvatureBodyKind::PlanetarySphere | CurvatureBodyKind::MovingBody => {
                if local.length_squared() > 0.0001 {
                    local.normalize()
                } else {
                    Vec3::Y
                }
            }
            CurvatureBodyKind::InteriorSphere => {
                if local.length_squared() > 0.0001 {
                    -local.normalize()
                } else {
                    Vec3::NEG_Y
                }
            }
            CurvatureBodyKind::Cylinder => {
                let radial = local - self.axis * local.dot(self.axis);
                if radial.length_squared() > 0.0001 {
                    -radial.normalize()
                } else {
                    Vec3::NEG_Y
                }
            }
            CurvatureBodyKind::ExteriorCylinder => {
                let radial = local - self.axis * local.dot(self.axis);
                if radial.length_squared() > 0.0001 {
                    radial.normalize()
                } else {
                    Vec3::Y
                }
            }
        }
    }

    /// Compute tangent directions at a surface point.
    #[must_use]
    pub fn surface_tangent_frame(&self, surface_point: Vec3) -> (Vec3, Vec3, Vec3) {
        let up = self.surface_up(surface_point);
        let arbitrary = if up.dot(Vec3::X).abs() < 0.9 {
            Vec3::X
        } else {
            Vec3::Z
        };
        let tangent1 = up.cross(arbitrary).normalize();
        let tangent2 = up.cross(tangent1);
        (tangent1, tangent2, up)
    }

    /// Compute the surface point closest to a given point.
    #[must_use]
    pub fn closest_surface_point(&self, point: Vec3) -> Vec3 {
        let local = point - self.center;

        match self.kind {
            CurvatureBodyKind::PlanetarySphere
            | CurvatureBodyKind::InteriorSphere
            | CurvatureBodyKind::MovingBody => {
                if local.length_squared() > 0.0001 {
                    self.center + local.normalize() * self.radius
                } else {
                    self.center + Vec3::Y * self.radius
                }
            }
            CurvatureBodyKind::Cylinder | CurvatureBodyKind::ExteriorCylinder => {
                let along_axis = local.dot(self.axis);
                let clamped_along = along_axis.clamp(-self.half_length, self.half_length);
                let radial = local - self.axis * along_axis;
                let radial_dir = if radial.length_squared() > 0.0001 {
                    radial.normalize()
                } else {
                    let perp = if self.axis.dot(Vec3::X).abs() < 0.9 {
                        Vec3::X
                    } else {
                        Vec3::Z
                    };
                    self.axis.cross(perp).normalize()
                };
                self.center + self.axis * clamped_along + radial_dir * self.radius
            }
        }
    }

    /// Compute height above surface (negative if below).
    #[must_use]
    pub fn height_above_surface(&self, point: Vec3) -> f32 {
        let local = point - self.center;

        match self.kind {
            CurvatureBodyKind::PlanetarySphere | CurvatureBodyKind::MovingBody => {
                local.length() - self.radius
            }
            CurvatureBodyKind::InteriorSphere => self.radius - local.length(),
            CurvatureBodyKind::Cylinder => {
                let along_axis = local.dot(self.axis);
                let radial = local - self.axis * along_axis;
                self.radius - radial.length()
            }
            CurvatureBodyKind::ExteriorCylinder => {
                let along_axis = local.dot(self.axis);
                let radial = local - self.axis * along_axis;
                radial.length() - self.radius
            }
        }
    }

    /// Compute rotation matrix for body at given time.
    #[must_use]
    pub fn rotation_at_time(&self, time: f32) -> Mat3 {
        if self.angular_velocity.abs() < 0.0001 {
            return Mat3::IDENTITY;
        }
        let angle = time * self.angular_velocity;
        Mat3::from_quat(Quat::from_axis_angle(self.axis, angle))
    }

    /// Compute center position at given time (for moving bodies).
    #[must_use]
    pub fn center_at_time(&self, time: f32) -> Vec3 {
        self.center + self.velocity * time
    }

    /// Check if a point is within the body's solid volume (below surface).
    #[must_use]
    pub fn contains(&self, point: Vec3) -> bool {
        let local = point - self.center;

        match self.kind {
            CurvatureBodyKind::PlanetarySphere | CurvatureBodyKind::MovingBody => {
                local.length() <= self.radius
            }
            CurvatureBodyKind::InteriorSphere => local.length() >= self.radius,
            CurvatureBodyKind::Cylinder => {
                let along_axis = local.dot(self.axis);
                let radial = local - self.axis * along_axis;
                radial.length() >= self.radius && along_axis.abs() <= self.half_length
            }
            CurvatureBodyKind::ExteriorCylinder => {
                let along_axis = local.dot(self.axis);
                let radial = local - self.axis * along_axis;
                radial.length() <= self.radius && along_axis.abs() <= self.half_length
            }
        }
    }

    /// Clamp values to valid ranges.
    #[must_use]
    pub fn clamped(mut self) -> Self {
        self.radius = self.radius.max(0.1);
        self.half_length = self.half_length.max(0.0);
        self.surface_gravity = self.surface_gravity.max(0.0);
        if self.axis.length_squared() > 0.0001 {
            self.axis = self.axis.normalize();
        } else {
            self.axis = Vec3::Y;
        }
        self
    }

    /// Check if values are valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.radius > 0.0
            && self.half_length >= 0.0
            && self.surface_gravity >= 0.0
            && (self.axis.length() - 1.0).abs() < 0.01
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_planetary_sphere_surface_up() {
        let body = CurvatureBody::planetary_sphere(Vec3::ZERO, 1000.0);
        let up = body.surface_up(Vec3::new(1000.0, 0.0, 0.0));
        assert_relative_eq!(up.x, 1.0, epsilon = 0.001);
        assert_relative_eq!(up.y, 0.0, epsilon = 0.001);
        assert_relative_eq!(up.z, 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_interior_sphere_surface_up() {
        let body = CurvatureBody::interior_sphere(Vec3::ZERO, 1000.0);
        let up = body.surface_up(Vec3::new(1000.0, 0.0, 0.0));
        assert_relative_eq!(up.x, -1.0, epsilon = 0.001);
    }

    #[test]
    fn test_cylinder_surface_up() {
        let body = CurvatureBody::cylinder(Vec3::ZERO, 100.0, 500.0, Vec3::Y);
        let up = body.surface_up(Vec3::new(100.0, 50.0, 0.0));
        assert_relative_eq!(up.x, -1.0, epsilon = 0.001);
        assert_relative_eq!(up.y, 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_exterior_cylinder_surface_up() {
        let body = CurvatureBody::exterior_cylinder(Vec3::ZERO, 100.0, 500.0, Vec3::Y);
        let up = body.surface_up(Vec3::new(100.0, 50.0, 0.0));
        assert_relative_eq!(up.x, 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_height_above_surface_planetary() {
        let body = CurvatureBody::planetary_sphere(Vec3::ZERO, 1000.0);
        assert_relative_eq!(
            body.height_above_surface(Vec3::new(1100.0, 0.0, 0.0)),
            100.0,
            epsilon = 0.1
        );
        assert_relative_eq!(
            body.height_above_surface(Vec3::new(900.0, 0.0, 0.0)),
            -100.0,
            epsilon = 0.1
        );
    }

    #[test]
    fn test_height_above_surface_interior() {
        let body = CurvatureBody::interior_sphere(Vec3::ZERO, 1000.0);
        assert_relative_eq!(
            body.height_above_surface(Vec3::new(900.0, 0.0, 0.0)),
            100.0,
            epsilon = 0.1
        );
        assert_relative_eq!(
            body.height_above_surface(Vec3::new(1100.0, 0.0, 0.0)),
            -100.0,
            epsilon = 0.1
        );
    }

    #[test]
    fn test_closest_surface_point_sphere() {
        let body = CurvatureBody::planetary_sphere(Vec3::ZERO, 1000.0);
        let closest = body.closest_surface_point(Vec3::new(2000.0, 0.0, 0.0));
        assert_relative_eq!(closest.x, 1000.0, epsilon = 0.1);
        assert_relative_eq!(closest.y, 0.0, epsilon = 0.1);
    }

    #[test]
    fn test_closest_surface_point_cylinder() {
        let body = CurvatureBody::cylinder(Vec3::ZERO, 100.0, 500.0, Vec3::Y);
        let closest = body.closest_surface_point(Vec3::new(50.0, 200.0, 0.0));
        assert_relative_eq!(closest.x, 100.0, epsilon = 0.1);
        assert_relative_eq!(closest.y, 200.0, epsilon = 0.1);
    }

    #[test]
    fn test_surface_tangent_frame_orthonormal() {
        let body = CurvatureBody::planetary_sphere(Vec3::ZERO, 1000.0);
        let (t1, t2, up) = body.surface_tangent_frame(Vec3::new(1000.0, 0.0, 0.0));

        assert_relative_eq!(t1.length(), 1.0, epsilon = 0.001);
        assert_relative_eq!(t2.length(), 1.0, epsilon = 0.001);
        assert_relative_eq!(up.length(), 1.0, epsilon = 0.001);
        assert_relative_eq!(t1.dot(t2).abs(), 0.0, epsilon = 0.001);
        assert_relative_eq!(t1.dot(up).abs(), 0.0, epsilon = 0.001);
        assert_relative_eq!(t2.dot(up).abs(), 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_rotation_at_time() {
        let body = CurvatureBody::planetary_sphere(Vec3::ZERO, 1000.0)
            .with_angular_velocity(std::f32::consts::PI);
        let rot = body.rotation_at_time(1.0);
        let rotated = rot * Vec3::X;
        assert_relative_eq!(rotated.x, -1.0, epsilon = 0.001);
    }

    #[test]
    fn test_center_at_time() {
        let body = CurvatureBody::moving_body(Vec3::ZERO, 100.0, Vec3::new(10.0, 0.0, 0.0));
        let center = body.center_at_time(5.0);
        assert_relative_eq!(center.x, 50.0, epsilon = 0.001);
    }

    #[test]
    fn test_body_kind_properties() {
        assert!(CurvatureBodyKind::InteriorSphere.is_interior());
        assert!(CurvatureBodyKind::Cylinder.is_interior());
        assert!(!CurvatureBodyKind::PlanetarySphere.is_interior());

        assert!(CurvatureBodyKind::Cylinder.has_axis());
        assert!(CurvatureBodyKind::ExteriorCylinder.has_axis());
        assert!(!CurvatureBodyKind::PlanetarySphere.has_axis());

        assert!(CurvatureBodyKind::InteriorSphere.horizon_curves_up());
        assert!(!CurvatureBodyKind::PlanetarySphere.horizon_curves_up());
    }

    #[test]
    fn test_contains() {
        let planet = CurvatureBody::planetary_sphere(Vec3::ZERO, 1000.0);
        assert!(planet.contains(Vec3::new(500.0, 0.0, 0.0)));
        assert!(!planet.contains(Vec3::new(1500.0, 0.0, 0.0)));

        let interior = CurvatureBody::interior_sphere(Vec3::ZERO, 1000.0);
        assert!(interior.contains(Vec3::new(1500.0, 0.0, 0.0)));
        assert!(!interior.contains(Vec3::new(800.0, 0.0, 0.0)));
    }

    #[test]
    fn test_clamped() {
        let body = CurvatureBody {
            radius: -100.0,
            half_length: -50.0,
            surface_gravity: -10.0,
            axis: Vec3::ZERO,
            ..Default::default()
        }
        .clamped();

        assert!(body.is_valid());
        assert!(body.radius > 0.0);
        assert!(body.half_length >= 0.0);
        assert!(body.surface_gravity >= 0.0);
    }

    #[test]
    fn test_all_kinds_exist() {
        assert_eq!(CurvatureBodyKind::ALL.len(), 5);
        for kind in CurvatureBodyKind::ALL {
            let _ = kind.is_interior();
            let _ = kind.horizon_curves_up();
            let _ = kind.has_axis();
        }
    }
}
