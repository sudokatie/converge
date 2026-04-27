//! Pluggable gravity model API.
//!
//! Provides configurable gravity models for different world scenarios:
//! constant directional, inverted, zero-G, planetary (radial toward center),
//! inverted planetary (radial away), local field sampling, and moving frame.

use std::borrow::Cow;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::{ChunkVectorFields, VectorFieldChannel};

/// Standard Earth gravity magnitude in m/s^2.
pub const STANDARD_GRAVITY: f32 = 9.81;

/// Maximum allowed gravity magnitude for clamping.
pub const MAX_GRAVITY_MAGNITUDE: f32 = 100.0;

/// Minimum gravity magnitude below which gravity is treated as zero.
pub const MIN_GRAVITY_MAGNITUDE: f32 = 0.001;

/// A pluggable gravity model that defines how gravity behaves at any world position.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum GravityModel {
    /// Constant directional gravity (e.g., standard Earth gravity).
    Constant {
        /// Gravity direction (normalized internally).
        direction: Vec3,
        /// Gravity magnitude in m/s^2.
        magnitude: f32,
    },

    /// Inverted gravity (opposite of a constant direction).
    Inverted {
        /// Original gravity direction (will be negated).
        direction: Vec3,
        /// Gravity magnitude in m/s^2.
        magnitude: f32,
    },

    /// Zero gravity.
    ZeroG,

    /// Planetary gravity: radial toward a center point.
    Planetary {
        /// Center of gravitational attraction.
        center: Vec3,
        /// Surface gravity magnitude (at `surface_radius` distance).
        surface_gravity: f32,
        /// Radius at which `surface_gravity` applies.
        surface_radius: f32,
    },

    /// Inverted planetary: radial away from a center point.
    InvertedPlanetary {
        /// Center of gravitational repulsion.
        center: Vec3,
        /// Surface repulsion magnitude.
        surface_gravity: f32,
        /// Radius at which `surface_gravity` applies.
        surface_radius: f32,
    },

    /// Sample gravity from local vector field override.
    LocalField {
        /// Fallback model when field is not set or position is outside chunk.
        fallback: Box<GravityModel>,
    },

    /// Moving reference frame: adds frame acceleration to a base model.
    MovingFrame {
        /// Base gravity model.
        base: Box<GravityModel>,
        /// Frame velocity (for Coriolis-like effects).
        frame_velocity: Vec3,
        /// Frame acceleration offset added to gravity.
        frame_acceleration: Vec3,
    },
}

impl GravityModel {
    /// Standard Earth gravity (downward Y-).
    pub const EARTH: Self = Self::Constant {
        direction: Vec3::new(0.0, -1.0, 0.0),
        magnitude: STANDARD_GRAVITY,
    };

    /// Standard Earth gravity inverted (upward Y+).
    pub const EARTH_INVERTED: Self = Self::Inverted {
        direction: Vec3::new(0.0, -1.0, 0.0),
        magnitude: STANDARD_GRAVITY,
    };

    /// Zero gravity environment.
    pub const ZERO_G: Self = Self::ZeroG;

    /// Lunar gravity (about 1/6 Earth).
    pub const LUNAR: Self = Self::Constant {
        direction: Vec3::new(0.0, -1.0, 0.0),
        magnitude: 1.62,
    };

    /// Mars gravity (about 0.38 Earth).
    pub const MARS: Self = Self::Constant {
        direction: Vec3::new(0.0, -1.0, 0.0),
        magnitude: 3.72,
    };

    /// Create a constant gravity model.
    #[must_use]
    pub fn constant(direction: Vec3, magnitude: f32) -> Self {
        Self::Constant {
            direction: safe_normalize(direction),
            magnitude: magnitude.clamp(0.0, MAX_GRAVITY_MAGNITUDE),
        }
    }

    /// Create an inverted gravity model.
    #[must_use]
    pub fn inverted(direction: Vec3, magnitude: f32) -> Self {
        Self::Inverted {
            direction: safe_normalize(direction),
            magnitude: magnitude.clamp(0.0, MAX_GRAVITY_MAGNITUDE),
        }
    }

    /// Create a planetary gravity model.
    #[must_use]
    pub fn planetary(center: Vec3, surface_gravity: f32, surface_radius: f32) -> Self {
        Self::Planetary {
            center,
            surface_gravity: surface_gravity.clamp(0.0, MAX_GRAVITY_MAGNITUDE),
            surface_radius: surface_radius.max(1.0),
        }
    }

    /// Create an inverted planetary gravity model.
    #[must_use]
    pub fn inverted_planetary(center: Vec3, surface_gravity: f32, surface_radius: f32) -> Self {
        Self::InvertedPlanetary {
            center,
            surface_gravity: surface_gravity.clamp(0.0, MAX_GRAVITY_MAGNITUDE),
            surface_radius: surface_radius.max(1.0),
        }
    }

    /// Create a local field gravity model with a fallback.
    #[must_use]
    pub fn local_field(fallback: GravityModel) -> Self {
        Self::LocalField {
            fallback: Box::new(fallback),
        }
    }

    /// Create a moving frame gravity model.
    #[must_use]
    pub fn moving_frame(
        base: GravityModel,
        frame_velocity: Vec3,
        frame_acceleration: Vec3,
    ) -> Self {
        Self::MovingFrame {
            base: Box::new(base),
            frame_velocity,
            frame_acceleration: clamp_magnitude(frame_acceleration, MAX_GRAVITY_MAGNITUDE),
        }
    }

    /// Sample gravity at a world position without local field data.
    #[must_use]
    pub fn sample(&self, world_pos: Vec3) -> Vec3 {
        self.sample_with_field(world_pos, None, 0.0, 0.0, 0.0)
    }

    /// Sample gravity at a world position with optional local field data.
    ///
    /// # Arguments
    /// * `world_pos` - Position in world coordinates
    /// * `fields` - Optional chunk vector fields for `LocalField` sampling
    /// * `local_x`, `local_y`, `local_z` - Position within chunk [0, 16) for field sampling
    #[must_use]
    pub fn sample_with_field(
        &self,
        world_pos: Vec3,
        fields: Option<&ChunkVectorFields>,
        local_x: f32,
        local_y: f32,
        local_z: f32,
    ) -> Vec3 {
        let raw = self.sample_raw(world_pos, fields, local_x, local_y, local_z);
        clamp_magnitude(raw, MAX_GRAVITY_MAGNITUDE)
    }

    fn sample_raw(
        &self,
        world_pos: Vec3,
        fields: Option<&ChunkVectorFields>,
        local_x: f32,
        local_y: f32,
        local_z: f32,
    ) -> Vec3 {
        match self {
            Self::Constant {
                direction,
                magnitude,
            } => safe_normalize(*direction) * *magnitude,

            Self::Inverted {
                direction,
                magnitude,
            } => -safe_normalize(*direction) * *magnitude,

            Self::ZeroG => Vec3::ZERO,

            Self::Planetary {
                center,
                surface_gravity,
                surface_radius,
            } => {
                let to_center = *center - world_pos;
                let distance = to_center.length();
                if distance < MIN_GRAVITY_MAGNITUDE {
                    return Vec3::ZERO;
                }
                let direction = to_center / distance;
                let ratio = *surface_radius / distance;
                let gravity_mag = *surface_gravity * ratio * ratio;
                direction * gravity_mag.min(MAX_GRAVITY_MAGNITUDE)
            }

            Self::InvertedPlanetary {
                center,
                surface_gravity,
                surface_radius,
            } => {
                let from_center = world_pos - *center;
                let distance = from_center.length();
                if distance < MIN_GRAVITY_MAGNITUDE {
                    return Vec3::ZERO;
                }
                let direction = from_center / distance;
                let ratio = *surface_radius / distance;
                let gravity_mag = *surface_gravity * ratio * ratio;
                direction * gravity_mag.min(MAX_GRAVITY_MAGNITUDE)
            }

            Self::LocalField { fallback } => {
                if let Some(f) = fields {
                    let sampled = f.sample(
                        VectorFieldChannel::GravityOverride,
                        local_x,
                        local_y,
                        local_z,
                    );
                    let default = VectorFieldChannel::GravityOverride.default_value();
                    if (sampled - default).length_squared()
                        > MIN_GRAVITY_MAGNITUDE * MIN_GRAVITY_MAGNITUDE
                    {
                        return sampled;
                    }
                }
                fallback.sample_raw(world_pos, fields, local_x, local_y, local_z)
            }

            Self::MovingFrame {
                base,
                frame_velocity: _,
                frame_acceleration,
            } => {
                let base_gravity = base.sample_raw(world_pos, fields, local_x, local_y, local_z);
                base_gravity + *frame_acceleration
            }
        }
    }

    /// Get the gravity direction at a world position (normalized).
    #[must_use]
    pub fn direction(&self, world_pos: Vec3) -> Vec3 {
        safe_normalize(self.sample(world_pos))
    }

    /// Get the gravity magnitude at a world position.
    #[must_use]
    pub fn magnitude(&self, world_pos: Vec3) -> f32 {
        self.sample(world_pos).length()
    }

    /// Check if this model produces zero gravity everywhere.
    #[must_use]
    pub fn is_zero_g(&self) -> bool {
        matches!(self, Self::ZeroG)
    }

    /// Check if this model uses local field sampling.
    #[must_use]
    pub fn uses_local_field(&self) -> bool {
        match self {
            Self::LocalField { .. } => true,
            Self::MovingFrame { base, .. } => base.uses_local_field(),
            _ => false,
        }
    }

    /// Get the frame velocity if this is a moving frame model.
    #[must_use]
    pub fn frame_velocity(&self) -> Option<Vec3> {
        match self {
            Self::MovingFrame { frame_velocity, .. } => Some(*frame_velocity),
            _ => None,
        }
    }

    /// Get the frame acceleration if this is a moving frame model.
    #[must_use]
    pub fn frame_acceleration(&self) -> Option<Vec3> {
        match self {
            Self::MovingFrame {
                frame_acceleration, ..
            } => Some(*frame_acceleration),
            _ => None,
        }
    }
}

impl Default for GravityModel {
    fn default() -> Self {
        Self::EARTH
    }
}

/// Preset gravity profiles for common scenarios.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GravityProfile {
    /// Display name for this profile.
    pub name: Cow<'static, str>,
    /// The gravity model.
    pub model: GravityModel,
}

impl GravityProfile {
    /// Standard Earth gravity profile.
    pub const EARTH: Self = Self {
        name: Cow::Borrowed("Earth"),
        model: GravityModel::EARTH,
    };

    /// Lunar gravity profile.
    pub const LUNAR: Self = Self {
        name: Cow::Borrowed("Lunar"),
        model: GravityModel::LUNAR,
    };

    /// Mars gravity profile.
    pub const MARS: Self = Self {
        name: Cow::Borrowed("Mars"),
        model: GravityModel::MARS,
    };

    /// Zero-G profile.
    pub const ZERO_G: Self = Self {
        name: Cow::Borrowed("Zero-G"),
        model: GravityModel::ZERO_G,
    };

    /// Inverted Earth gravity profile.
    pub const INVERTED: Self = Self {
        name: Cow::Borrowed("Inverted"),
        model: GravityModel::EARTH_INVERTED,
    };

    /// Create a planetary profile with custom parameters.
    #[must_use]
    pub fn planetary(
        name: impl Into<Cow<'static, str>>,
        center: Vec3,
        surface_gravity: f32,
        radius: f32,
    ) -> Self {
        Self {
            name: name.into(),
            model: GravityModel::planetary(center, surface_gravity, radius),
        }
    }

    /// Create a local field profile with a fallback.
    #[must_use]
    pub fn local_field(name: impl Into<Cow<'static, str>>, fallback: GravityModel) -> Self {
        Self {
            name: name.into(),
            model: GravityModel::local_field(fallback),
        }
    }
}

impl Default for GravityProfile {
    fn default() -> Self {
        Self::EARTH
    }
}

/// Safely normalize a vector, returning Y- down if zero-length.
fn safe_normalize(v: Vec3) -> Vec3 {
    let len = v.length();
    if len < MIN_GRAVITY_MAGNITUDE {
        Vec3::new(0.0, -1.0, 0.0)
    } else {
        v / len
    }
}

/// Clamp a vector's magnitude.
fn clamp_magnitude(v: Vec3, max: f32) -> Vec3 {
    let len = v.length();
    if len > max { v * (max / len) } else { v }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use engine_core::coords::LocalPos;

    const EPSILON: f32 = 1e-5;

    #[test]
    fn constant_gravity() {
        let model = GravityModel::EARTH;
        let gravity = model.sample(Vec3::ZERO);
        assert_relative_eq!(gravity.x, 0.0, epsilon = EPSILON);
        assert_relative_eq!(gravity.y, -STANDARD_GRAVITY, epsilon = EPSILON);
        assert_relative_eq!(gravity.z, 0.0, epsilon = EPSILON);
    }

    #[test]
    fn inverted_gravity() {
        let model = GravityModel::EARTH_INVERTED;
        let gravity = model.sample(Vec3::ZERO);
        assert_relative_eq!(gravity.x, 0.0, epsilon = EPSILON);
        assert_relative_eq!(gravity.y, STANDARD_GRAVITY, epsilon = EPSILON);
        assert_relative_eq!(gravity.z, 0.0, epsilon = EPSILON);
    }

    #[test]
    fn zero_g() {
        let model = GravityModel::ZERO_G;
        let gravity = model.sample(Vec3::new(100.0, 200.0, 300.0));
        assert_eq!(gravity, Vec3::ZERO);
        assert!(model.is_zero_g());
    }

    #[test]
    fn planetary_gravity_at_surface() {
        let model = GravityModel::planetary(Vec3::ZERO, STANDARD_GRAVITY, 100.0);
        let gravity = model.sample(Vec3::new(100.0, 0.0, 0.0));
        assert_relative_eq!(gravity.length(), STANDARD_GRAVITY, epsilon = EPSILON);
        let dir = gravity.normalize();
        assert_relative_eq!(dir.x, -1.0, epsilon = EPSILON);
    }

    #[test]
    fn planetary_gravity_inverse_square() {
        let model = GravityModel::planetary(Vec3::ZERO, STANDARD_GRAVITY, 100.0);
        let at_surface = model.sample(Vec3::new(100.0, 0.0, 0.0)).length();
        let at_double = model.sample(Vec3::new(200.0, 0.0, 0.0)).length();
        assert_relative_eq!(at_double, at_surface / 4.0, epsilon = EPSILON);
    }

    #[test]
    fn planetary_gravity_at_center() {
        let model = GravityModel::planetary(Vec3::ZERO, STANDARD_GRAVITY, 100.0);
        let gravity = model.sample(Vec3::ZERO);
        assert_eq!(gravity, Vec3::ZERO);
    }

    #[test]
    fn inverted_planetary_gravity() {
        let model = GravityModel::inverted_planetary(Vec3::ZERO, STANDARD_GRAVITY, 100.0);
        let gravity = model.sample(Vec3::new(100.0, 0.0, 0.0));
        assert_relative_eq!(gravity.length(), STANDARD_GRAVITY, epsilon = EPSILON);
        let dir = gravity.normalize();
        assert_relative_eq!(dir.x, 1.0, epsilon = EPSILON);
    }

    #[test]
    fn local_field_fallback() {
        let model = GravityModel::local_field(GravityModel::LUNAR);
        let gravity = model.sample(Vec3::ZERO);
        assert_relative_eq!(gravity.y, -1.62, epsilon = EPSILON);
    }

    #[test]
    fn local_field_with_override() {
        let model = GravityModel::local_field(GravityModel::EARTH);
        let mut fields = ChunkVectorFields::new();
        fields.set(
            VectorFieldChannel::GravityOverride,
            LocalPos::new(8, 8, 8),
            Vec3::new(0.0, 5.0, 0.0),
        );

        let gravity = model.sample_with_field(Vec3::ZERO, Some(&fields), 8.0, 8.0, 8.0);
        assert_relative_eq!(gravity.y, 5.0, epsilon = EPSILON);
    }

    #[test]
    fn moving_frame_adds_acceleration() {
        let model = GravityModel::moving_frame(
            GravityModel::EARTH,
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
        );
        let gravity = model.sample(Vec3::ZERO);
        assert_relative_eq!(gravity.x, 2.0, epsilon = EPSILON);
        assert_relative_eq!(gravity.y, -STANDARD_GRAVITY, epsilon = EPSILON);
    }

    #[test]
    fn moving_frame_velocity_accessor() {
        let model =
            GravityModel::moving_frame(GravityModel::EARTH, Vec3::new(10.0, 0.0, 0.0), Vec3::ZERO);
        assert_eq!(model.frame_velocity(), Some(Vec3::new(10.0, 0.0, 0.0)));
        assert_eq!(GravityModel::EARTH.frame_velocity(), None);
    }

    #[test]
    fn direction_and_magnitude() {
        let model = GravityModel::EARTH;
        let dir = model.direction(Vec3::ZERO);
        let mag = model.magnitude(Vec3::ZERO);
        assert_relative_eq!(dir.y, -1.0, epsilon = EPSILON);
        assert_relative_eq!(mag, STANDARD_GRAVITY, epsilon = EPSILON);
    }

    #[test]
    fn uses_local_field() {
        assert!(!GravityModel::EARTH.uses_local_field());
        assert!(GravityModel::local_field(GravityModel::EARTH).uses_local_field());
        let nested = GravityModel::moving_frame(
            GravityModel::local_field(GravityModel::EARTH),
            Vec3::ZERO,
            Vec3::ZERO,
        );
        assert!(nested.uses_local_field());
    }

    #[test]
    fn magnitude_clamping() {
        let model = GravityModel::constant(Vec3::NEG_Y, 500.0);
        let gravity = model.sample(Vec3::ZERO);
        assert!(gravity.length() <= MAX_GRAVITY_MAGNITUDE + EPSILON);
    }

    #[test]
    fn factory_methods_clamp() {
        let model = GravityModel::constant(Vec3::NEG_Y, 1000.0);
        if let GravityModel::Constant { magnitude, .. } = model {
            assert!(magnitude <= MAX_GRAVITY_MAGNITUDE);
        }
    }

    #[test]
    fn default_is_earth() {
        assert_eq!(GravityModel::default(), GravityModel::EARTH);
        assert_eq!(GravityProfile::default(), GravityProfile::EARTH);
    }

    #[test]
    fn profiles() {
        assert_eq!(GravityProfile::EARTH.name, "Earth");
        assert_eq!(GravityProfile::LUNAR.name, "Lunar");
        assert_eq!(GravityProfile::ZERO_G.name, "Zero-G");
    }

    #[test]
    fn serde_constant() {
        let model = GravityModel::EARTH;
        let json = serde_json::to_string(&model).unwrap();
        let recovered: GravityModel = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, model);
    }

    #[test]
    fn serde_planetary() {
        let model = GravityModel::planetary(Vec3::new(100.0, 0.0, 0.0), 10.0, 50.0);
        let json = serde_json::to_string(&model).unwrap();
        let recovered: GravityModel = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, model);
    }

    #[test]
    fn serde_local_field() {
        let model = GravityModel::local_field(GravityModel::LUNAR);
        let json = serde_json::to_string(&model).unwrap();
        let recovered: GravityModel = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, model);
    }

    #[test]
    fn serde_moving_frame() {
        let model = GravityModel::moving_frame(
            GravityModel::EARTH,
            Vec3::new(5.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
        );
        let json = serde_json::to_string(&model).unwrap();
        let recovered: GravityModel = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, model);
    }

    #[test]
    fn serde_profile() {
        let profile = GravityProfile::MARS;
        let json = serde_json::to_string(&profile).unwrap();
        let recovered: GravityProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, profile);
    }

    #[test]
    fn safe_normalize_zero_vector() {
        let result = safe_normalize(Vec3::ZERO);
        assert_relative_eq!(result.y, -1.0, epsilon = EPSILON);
    }

    #[test]
    fn clamp_magnitude_within_bounds() {
        let v = Vec3::new(3.0, 4.0, 0.0);
        let clamped = clamp_magnitude(v, 10.0);
        assert_eq!(clamped, v);
    }

    #[test]
    fn clamp_magnitude_exceeds_bounds() {
        let v = Vec3::new(30.0, 40.0, 0.0);
        let clamped = clamp_magnitude(v, 10.0);
        assert_relative_eq!(clamped.length(), 10.0, epsilon = EPSILON);
    }
}
