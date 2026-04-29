//! Screen-space distortion overlay configuration.
//!
//! Controls how distortion effects are composited onto the final image,
//! including quality settings, blend modes, and temporal behavior.

use std::f32::consts::TAU;

/// Quality/budget level for distortion rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum DistortionQuality {
    /// Minimal samples, fastest.
    Low = 0,
    /// Balanced quality and performance.
    #[default]
    Medium = 1,
    /// High sample count, best quality.
    High = 2,
    /// Maximum quality, expensive.
    Ultra = 3,
}

impl DistortionQuality {
    /// All quality levels.
    pub const ALL: [Self; 4] = [Self::Low, Self::Medium, Self::High, Self::Ultra];

    /// Get sample count for this quality level.
    #[must_use]
    pub fn sample_count(self) -> u32 {
        match self {
            Self::Low => 4,
            Self::Medium => 8,
            Self::High => 16,
            Self::Ultra => 32,
        }
    }

    /// Get resolution divisor (1 = full, 2 = half, etc.).
    #[must_use]
    pub fn resolution_divisor(self) -> u32 {
        match self {
            Self::Low => 4,
            Self::Medium => 2,
            Self::High | Self::Ultra => 1,
        }
    }
}

/// Flow direction for directional distortion patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum FlowDirection {
    /// No directional flow (radial or static).
    #[default]
    None = 0,
    /// Flow upward (heat shimmer).
    Up = 1,
    /// Flow downward (rain, waterfall).
    Down = 2,
    /// Flow to the right.
    Right = 3,
    /// Flow to the left.
    Left = 4,
    /// Radial outward from center.
    RadialOut = 5,
    /// Radial inward toward center.
    RadialIn = 6,
}

impl FlowDirection {
    /// All flow directions.
    pub const ALL: [Self; 7] = [
        Self::None,
        Self::Up,
        Self::Down,
        Self::Right,
        Self::Left,
        Self::RadialOut,
        Self::RadialIn,
    ];

    /// Convert to a 2D direction vector in screen space.
    #[must_use]
    pub fn to_vec2(self) -> (f32, f32) {
        match self {
            Self::Up => (0.0, 1.0),
            Self::Down => (0.0, -1.0),
            Self::Right => (1.0, 0.0),
            Self::Left => (-1.0, 0.0),
            Self::None | Self::RadialOut | Self::RadialIn => (0.0, 0.0),
        }
    }

    /// Whether this is a radial flow type.
    #[must_use]
    pub fn is_radial(self) -> bool {
        matches!(self, Self::RadialOut | Self::RadialIn)
    }
}

/// Blend mode for compositing distortion with the scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum BlendMode {
    /// Simple UV offset blending.
    #[default]
    Offset = 0,
    /// Additive blend (brightens).
    Additive = 1,
    /// Multiplicative blend (darkens).
    Multiply = 2,
    /// Chromatic aberration (RGB split).
    ChromaticAberration = 3,
    /// Heat haze with color shift.
    HeatHaze = 4,
}

impl BlendMode {
    /// All blend modes.
    pub const ALL: [Self; 5] = [
        Self::Offset,
        Self::Additive,
        Self::Multiply,
        Self::ChromaticAberration,
        Self::HeatHaze,
    ];
}

/// Screen-space distortion configuration.
#[derive(Debug, Clone, Copy)]
pub struct ScreenDistortion {
    /// Overall distortion strength (0.0 to 1.0).
    pub strength: f32,
    /// Spatial frequency of the distortion pattern.
    pub frequency: f32,
    /// Flow direction for animated distortion.
    pub flow_direction: FlowDirection,
    /// Flow speed (units per second).
    pub flow_speed: f32,
    /// Edge falloff (0.0 = no falloff, 1.0 = full screen falloff).
    pub edge_falloff: f32,
    /// Center position for radial effects (normalized 0-1).
    pub center: (f32, f32),
    /// Temporal phase offset (0.0 to TAU).
    pub phase: f32,
    /// Quality/budget setting.
    pub quality: DistortionQuality,
    /// Blend mode for compositing.
    pub blend_mode: BlendMode,
    /// Maximum displacement in pixels at full strength.
    pub max_displacement: f32,
    /// Whether this distortion is enabled.
    pub enabled: bool,
}

impl Default for ScreenDistortion {
    fn default() -> Self {
        Self {
            strength: 0.5,
            frequency: 4.0,
            flow_direction: FlowDirection::None,
            flow_speed: 1.0,
            edge_falloff: 0.2,
            center: (0.5, 0.5),
            phase: 0.0,
            quality: DistortionQuality::Medium,
            blend_mode: BlendMode::Offset,
            max_displacement: 16.0,
            enabled: true,
        }
    }
}

impl ScreenDistortion {
    /// Create a heat shimmer screen distortion.
    #[must_use]
    pub fn heat_shimmer() -> Self {
        Self {
            strength: 0.3,
            frequency: 8.0,
            flow_direction: FlowDirection::Up,
            flow_speed: 0.5,
            edge_falloff: 0.1,
            center: (0.5, 0.0),
            blend_mode: BlendMode::HeatHaze,
            max_displacement: 8.0,
            ..Default::default()
        }
    }

    /// Create a pressure wave screen distortion.
    #[must_use]
    pub fn pressure_wave() -> Self {
        Self {
            strength: 0.8,
            frequency: 2.0,
            flow_direction: FlowDirection::RadialOut,
            flow_speed: 5.0,
            edge_falloff: 0.5,
            center: (0.5, 0.5),
            blend_mode: BlendMode::Offset,
            max_displacement: 32.0,
            ..Default::default()
        }
    }

    /// Create a radiation warp screen distortion.
    #[must_use]
    pub fn radiation_warp() -> Self {
        Self {
            strength: 0.4,
            frequency: 3.0,
            flow_direction: FlowDirection::None,
            flow_speed: 0.2,
            edge_falloff: 0.3,
            center: (0.5, 0.5),
            blend_mode: BlendMode::ChromaticAberration,
            max_displacement: 12.0,
            ..Default::default()
        }
    }

    /// Create a fracture event screen distortion.
    #[must_use]
    pub fn fracture_event() -> Self {
        Self {
            strength: 1.0,
            frequency: 6.0,
            flow_direction: FlowDirection::RadialIn,
            flow_speed: 3.0,
            edge_falloff: 0.0,
            center: (0.5, 0.5),
            blend_mode: BlendMode::ChromaticAberration,
            max_displacement: 48.0,
            quality: DistortionQuality::High,
            ..Default::default()
        }
    }

    /// Set strength.
    #[must_use]
    pub fn with_strength(mut self, strength: f32) -> Self {
        self.strength = strength.clamp(0.0, 1.0);
        self
    }

    /// Set frequency.
    #[must_use]
    pub fn with_frequency(mut self, frequency: f32) -> Self {
        self.frequency = frequency.max(0.1);
        self
    }

    /// Set flow direction.
    #[must_use]
    pub fn with_flow_direction(mut self, direction: FlowDirection) -> Self {
        self.flow_direction = direction;
        self
    }

    /// Set flow speed.
    #[must_use]
    pub fn with_flow_speed(mut self, speed: f32) -> Self {
        self.flow_speed = speed.max(0.0);
        self
    }

    /// Set edge falloff.
    #[must_use]
    pub fn with_edge_falloff(mut self, falloff: f32) -> Self {
        self.edge_falloff = falloff.clamp(0.0, 1.0);
        self
    }

    /// Set center position.
    #[must_use]
    pub fn with_center(mut self, x: f32, y: f32) -> Self {
        self.center = (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0));
        self
    }

    /// Set phase offset.
    #[must_use]
    pub fn with_phase(mut self, phase: f32) -> Self {
        self.phase = phase % TAU;
        self
    }

    /// Set quality level.
    #[must_use]
    pub fn with_quality(mut self, quality: DistortionQuality) -> Self {
        self.quality = quality;
        self
    }

    /// Set blend mode.
    #[must_use]
    pub fn with_blend_mode(mut self, mode: BlendMode) -> Self {
        self.blend_mode = mode;
        self
    }

    /// Set maximum displacement.
    #[must_use]
    pub fn with_max_displacement(mut self, pixels: f32) -> Self {
        self.max_displacement = pixels.max(0.0);
        self
    }

    /// Enable or disable.
    #[must_use]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Compute UV offset at a given screen position and time.
    #[must_use]
    pub fn compute_offset(&self, uv: (f32, f32), time: f32) -> (f32, f32) {
        if !self.enabled || self.strength <= 0.0 {
            return (0.0, 0.0);
        }

        let (u, v) = uv;
        let (cx, cy) = self.center;

        let falloff = self.compute_falloff(uv);
        let (flow_x, flow_y) = self.flow_direction.to_vec2();
        let flow_offset = time * self.flow_speed;

        let sample_x = u * self.frequency + flow_x * flow_offset + self.phase;
        let sample_y = v * self.frequency + flow_y * flow_offset + self.phase;

        let noise_x = (sample_x * TAU).sin() * (sample_y * 1.3 * TAU).cos();
        let noise_y = (sample_y * TAU).cos() * (sample_x * 1.7 * TAU).sin();

        let radial_factor = if self.flow_direction.is_radial() {
            let dx = u - cx;
            let dy = v - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let radial_phase = dist * self.frequency - flow_offset;
            radial_phase.sin()
        } else {
            1.0
        };

        let scale = self.strength * falloff * radial_factor * self.max_displacement;
        (noise_x * scale, noise_y * scale)
    }

    /// Compute falloff factor at a screen position.
    #[must_use]
    pub fn compute_falloff(&self, uv: (f32, f32)) -> f32 {
        if self.edge_falloff <= 0.0 {
            return 1.0;
        }

        let (u, v) = uv;
        let edge_dist = u.min(1.0 - u).min(v).min(1.0 - v);
        let falloff_start = self.edge_falloff;

        if edge_dist >= falloff_start {
            1.0
        } else {
            edge_dist / falloff_start
        }
    }

    /// Clamp all values to valid ranges.
    #[must_use]
    pub fn clamped(mut self) -> Self {
        self.strength = self.strength.clamp(0.0, 1.0);
        self.frequency = self.frequency.clamp(0.1, 100.0);
        self.flow_speed = self.flow_speed.clamp(0.0, 100.0);
        self.edge_falloff = self.edge_falloff.clamp(0.0, 1.0);
        self.center.0 = self.center.0.clamp(0.0, 1.0);
        self.center.1 = self.center.1.clamp(0.0, 1.0);
        self.phase = self.phase.rem_euclid(TAU);
        self.max_displacement = self.max_displacement.clamp(0.0, 256.0);
        self
    }

    /// Check if values are valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.strength >= 0.0
            && self.strength <= 1.0
            && self.frequency >= 0.1
            && self.edge_falloff >= 0.0
            && self.edge_falloff <= 1.0
            && self.center.0 >= 0.0
            && self.center.0 <= 1.0
            && self.center.1 >= 0.0
            && self.center.1 <= 1.0
            && self.max_displacement >= 0.0
    }

    /// Interpolate between two configurations.
    #[must_use]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            strength: self.strength + (other.strength - self.strength) * t,
            frequency: self.frequency + (other.frequency - self.frequency) * t,
            flow_direction: if t < 0.5 {
                self.flow_direction
            } else {
                other.flow_direction
            },
            flow_speed: self.flow_speed + (other.flow_speed - self.flow_speed) * t,
            edge_falloff: self.edge_falloff + (other.edge_falloff - self.edge_falloff) * t,
            center: (
                self.center.0 + (other.center.0 - self.center.0) * t,
                self.center.1 + (other.center.1 - self.center.1) * t,
            ),
            phase: self.phase + (other.phase - self.phase) * t,
            quality: if t < 0.5 { self.quality } else { other.quality },
            blend_mode: if t < 0.5 {
                self.blend_mode
            } else {
                other.blend_mode
            },
            max_displacement: self.max_displacement
                + (other.max_displacement - self.max_displacement) * t,
            enabled: if t < 0.5 { self.enabled } else { other.enabled },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_quality_sample_counts() {
        assert!(DistortionQuality::Low.sample_count() < DistortionQuality::Ultra.sample_count());
        assert!(DistortionQuality::Medium.sample_count() >= 8);
    }

    #[test]
    fn test_quality_resolution() {
        assert!(
            DistortionQuality::Low.resolution_divisor()
                > DistortionQuality::High.resolution_divisor()
        );
    }

    #[test]
    fn test_flow_direction_vectors() {
        let (_x, y) = FlowDirection::Up.to_vec2();
        assert_relative_eq!(y, 1.0, epsilon = 0.001);

        let (x, _y) = FlowDirection::Right.to_vec2();
        assert_relative_eq!(x, 1.0, epsilon = 0.001);

        assert!(FlowDirection::RadialOut.is_radial());
        assert!(!FlowDirection::Up.is_radial());
    }

    #[test]
    fn test_heat_shimmer_preset() {
        let screen = ScreenDistortion::heat_shimmer();
        assert_eq!(screen.flow_direction, FlowDirection::Up);
        assert_eq!(screen.blend_mode, BlendMode::HeatHaze);
        assert!(screen.is_valid());
    }

    #[test]
    fn test_pressure_wave_preset() {
        let screen = ScreenDistortion::pressure_wave();
        assert!(screen.flow_direction.is_radial());
        assert!(screen.flow_speed > 1.0);
        assert!(screen.is_valid());
    }

    #[test]
    fn test_compute_offset_disabled() {
        let screen = ScreenDistortion::default().with_enabled(false);
        let (ox, oy) = screen.compute_offset((0.5, 0.5), 1.0);
        assert_relative_eq!(ox, 0.0, epsilon = 0.001);
        assert_relative_eq!(oy, 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_compute_offset_zero_strength() {
        let screen = ScreenDistortion::default().with_strength(0.0);
        let (ox, oy) = screen.compute_offset((0.5, 0.5), 1.0);
        assert_relative_eq!(ox, 0.0, epsilon = 0.001);
        assert_relative_eq!(oy, 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_compute_falloff_center() {
        let screen = ScreenDistortion::default().with_edge_falloff(0.1);
        let falloff = screen.compute_falloff((0.5, 0.5));
        assert_relative_eq!(falloff, 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_compute_falloff_edge() {
        let screen = ScreenDistortion::default().with_edge_falloff(0.2);
        let falloff = screen.compute_falloff((0.05, 0.5));
        assert!(falloff < 1.0);
        assert!(falloff > 0.0);
    }

    #[test]
    fn test_compute_falloff_no_falloff() {
        let screen = ScreenDistortion::default().with_edge_falloff(0.0);
        let falloff = screen.compute_falloff((0.01, 0.01));
        assert_relative_eq!(falloff, 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_lerp_endpoints() {
        let a = ScreenDistortion::heat_shimmer();
        let b = ScreenDistortion::pressure_wave();

        let at_a = a.lerp(b, 0.0);
        assert_relative_eq!(at_a.strength, a.strength, epsilon = 0.001);

        let at_b = a.lerp(b, 1.0);
        assert_relative_eq!(at_b.strength, b.strength, epsilon = 0.001);
    }

    #[test]
    fn test_lerp_midpoint() {
        let a = ScreenDistortion::default().with_strength(0.2);
        let b = ScreenDistortion::default().with_strength(0.8);

        let mid = a.lerp(b, 0.5);
        assert_relative_eq!(mid.strength, 0.5, epsilon = 0.001);
    }

    #[test]
    fn test_clamping() {
        let screen = ScreenDistortion {
            strength: 2.0,
            frequency: 0.01,
            edge_falloff: -0.5,
            center: (2.0, -1.0),
            max_displacement: 1000.0,
            ..Default::default()
        }
        .clamped();

        assert!(screen.is_valid());
        assert_relative_eq!(screen.strength, 1.0, epsilon = 0.001);
        assert_relative_eq!(screen.edge_falloff, 0.0, epsilon = 0.001);
        assert_relative_eq!(screen.center.0, 1.0, epsilon = 0.001);
        assert_relative_eq!(screen.center.1, 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_builder_chain() {
        let screen = ScreenDistortion::default()
            .with_strength(0.7)
            .with_frequency(5.0)
            .with_flow_direction(FlowDirection::Up)
            .with_quality(DistortionQuality::High)
            .with_blend_mode(BlendMode::HeatHaze);

        assert_relative_eq!(screen.strength, 0.7, epsilon = 0.001);
        assert_relative_eq!(screen.frequency, 5.0, epsilon = 0.001);
        assert_eq!(screen.flow_direction, FlowDirection::Up);
        assert_eq!(screen.quality, DistortionQuality::High);
        assert_eq!(screen.blend_mode, BlendMode::HeatHaze);
    }

    #[test]
    fn test_all_presets_valid() {
        for screen in [
            ScreenDistortion::heat_shimmer(),
            ScreenDistortion::pressure_wave(),
            ScreenDistortion::radiation_warp(),
            ScreenDistortion::fracture_event(),
        ] {
            assert!(screen.is_valid(), "preset should be valid");
            assert!(screen.enabled, "preset should be enabled");
        }
    }
}
