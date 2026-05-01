//! Deterministic sampling and preview utilities for photo mode.
//!
//! Provides CPU-side sampling for reproducible previews
//! and DOF calculations.

use super::settings::PhotoSettings;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

/// Deterministic sampler for photo mode previews.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhotoSampler {
    seed: u64,
    time: f32,
    frame: u32,
}

impl Default for PhotoSampler {
    fn default() -> Self {
        Self::new(0)
    }
}

impl PhotoSampler {
    /// Create a new sampler with the given seed.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            time: 0.0,
            frame: 0,
        }
    }

    /// Set the current time.
    #[must_use]
    pub fn with_time(mut self, time: f32) -> Self {
        self.time = time;
        self
    }

    /// Set the current frame.
    #[must_use]
    pub fn with_frame(mut self, frame: u32) -> Self {
        self.frame = frame;
        self
    }

    /// Advance to next frame.
    pub fn next_frame(&mut self) {
        self.frame = self.frame.wrapping_add(1);
    }

    /// Sample a random value in [0, 1) for a given position.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "deterministic sampling: precision loss is acceptable for uniform [0,1) distribution"
    )]
    pub fn sample(&self, x: f32, y: f32) -> f32 {
        let hash = position_hash(x, y, self.seed, self.frame);
        (hash as f32) / (u64::MAX as f32)
    }

    /// Sample a 2D offset for jittering.
    #[must_use]
    pub fn sample_jitter(&self, x: f32, y: f32) -> (f32, f32) {
        let jx = self.sample(x, y);
        let jy = self.sample(x + 0.5, y + 0.5);
        (jx - 0.5, jy - 0.5)
    }

    /// Sample a point on a unit disk (for DOF bokeh).
    #[must_use]
    pub fn sample_disk(&self, px: f32, py: f32) -> (f32, f32) {
        let rand_u = self.sample(px, py);
        let rand_v = self.sample(px + 1.0, py + 1.0);

        let radius = rand_u.sqrt();
        let theta = rand_v * std::f32::consts::TAU;

        (radius * theta.cos(), radius * theta.sin())
    }

    /// Sample a Halton sequence point.
    #[must_use]
    pub fn sample_halton(&self, index: u32) -> (f32, f32) {
        (halton(index, 2), halton(index, 3))
    }

    /// Sample DOF blur offset for a given pixel.
    #[must_use]
    pub fn sample_dof_offset(
        &self,
        x: f32,
        y: f32,
        blur_radius: f32,
        bokeh_shape: BokehShape,
    ) -> (f32, f32) {
        let (dx, dy) = match bokeh_shape {
            BokehShape::Circle => self.sample_disk(x, y),
            BokehShape::Hexagon => sample_hexagon(self.sample(x, y), self.sample(x + 0.5, y)),
            BokehShape::Octagon => sample_octagon(self.sample(x, y), self.sample(x + 0.5, y)),
            BokehShape::Square => (
                self.sample(x, y) * 2.0 - 1.0,
                self.sample(x + 0.5, y) * 2.0 - 1.0,
            ),
        };

        (dx * blur_radius, dy * blur_radius)
    }

    /// Get deterministic noise value for film grain.
    #[must_use]
    pub fn film_grain(&self, x: f32, y: f32, intensity: f32) -> f32 {
        let noise = self.sample(x * 100.0, y * 100.0);
        (noise - 0.5) * intensity
    }
}

/// Compute a stable position hash.
#[must_use]
pub fn position_hash(x: f32, y: f32, seed: u64, frame: u32) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    x.to_bits().hash(&mut hasher);
    y.to_bits().hash(&mut hasher);
    seed.hash(&mut hasher);
    frame.hash(&mut hasher);
    hasher.finish()
}

/// Compute Halton sequence value.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn halton(index: u32, base: u32) -> f32 {
    let mut f = 1.0_f32;
    let mut r = 0.0_f32;
    let mut i = index;

    while i > 0 {
        f /= base as f32;
        r += f * (i % base) as f32;
        i /= base;
    }

    r
}

/// Bokeh shape for depth of field.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum BokehShape {
    /// Circular bokeh (default).
    #[default]
    Circle = 0,
    /// Hexagonal bokeh (6 blades).
    Hexagon = 1,
    /// Octagonal bokeh (8 blades).
    Octagon = 2,
    /// Square bokeh (4 blades).
    Square = 3,
}

impl BokehShape {
    /// All available bokeh shapes.
    pub const ALL: [BokehShape; 4] = [
        BokehShape::Circle,
        BokehShape::Hexagon,
        BokehShape::Octagon,
        BokehShape::Square,
    ];

    /// Get shape name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            BokehShape::Circle => "Circle",
            BokehShape::Hexagon => "Hexagon",
            BokehShape::Octagon => "Octagon",
            BokehShape::Square => "Square",
        }
    }

    /// Number of aperture blades.
    #[must_use]
    pub const fn blade_count(&self) -> u32 {
        match self {
            BokehShape::Circle => 0,
            BokehShape::Hexagon => 6,
            BokehShape::Octagon => 8,
            BokehShape::Square => 4,
        }
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "deterministic sampling: sector index [0,5] fits in i32, precision loss acceptable"
)]
fn sample_hexagon(u: f32, v: f32) -> (f32, f32) {
    let sector = (u * 6.0).floor() as i32;
    let t = u * 6.0 - sector as f32;

    let angle1 = (sector as f32) * std::f32::consts::FRAC_PI_3;
    let angle2 = angle1 + std::f32::consts::FRAC_PI_3;

    let x1 = angle1.cos();
    let y1 = angle1.sin();
    let x2 = angle2.cos();
    let y2 = angle2.sin();

    let px = x1 + t * (x2 - x1);
    let py = y1 + t * (y2 - y1);

    let r = v.sqrt();
    (px * r, py * r)
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "deterministic sampling: sector index [0,7] fits in i32, precision loss acceptable"
)]
fn sample_octagon(u: f32, v: f32) -> (f32, f32) {
    let sector = (u * 8.0).floor() as i32;
    let t = u * 8.0 - sector as f32;

    let angle1 = (sector as f32) * std::f32::consts::FRAC_PI_4;
    let angle2 = angle1 + std::f32::consts::FRAC_PI_4;

    let x1 = angle1.cos();
    let y1 = angle1.sin();
    let x2 = angle2.cos();
    let y2 = angle2.sin();

    let px = x1 + t * (x2 - x1);
    let py = y1 + t * (y2 - y1);

    let r = v.sqrt();
    (px * r, py * r)
}

/// DOF calculation result.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DofResult {
    /// Circle of confusion diameter in pixels.
    pub coc: f32,
    /// Whether this point is in focus.
    pub in_focus: bool,
    /// Blur amount (0 = sharp, 1 = maximum blur).
    pub blur_amount: f32,
    /// Whether in front of focal plane (foreground blur).
    pub is_foreground: bool,
}

/// Calculate depth of field parameters.
#[must_use]
pub fn calculate_dof(
    depth: f32,
    settings: &PhotoSettings,
    sensor_height_mm: f32,
    image_height_px: f32,
) -> DofResult {
    let focal_length_mm = sensor_height_mm / (2.0 * (settings.fov.to_radians() / 2.0).tan());

    let hyperfocal =
        (focal_length_mm * focal_length_mm) / (settings.aperture * 0.029) + focal_length_mm;

    let near_limit = (settings.focus_distance * (hyperfocal - focal_length_mm))
        / (hyperfocal + settings.focus_distance - 2.0 * focal_length_mm);

    let far_limit = (settings.focus_distance * (hyperfocal - focal_length_mm))
        / (hyperfocal - settings.focus_distance);

    let in_focus = depth >= near_limit.max(0.1) && depth <= far_limit.max(near_limit);

    let coc = if depth <= 0.0 {
        0.0
    } else {
        let magnification = focal_length_mm / (depth * 1000.0 - focal_length_mm);
        let subject_mag = focal_length_mm / (settings.focus_distance * 1000.0 - focal_length_mm);
        (magnification - subject_mag).abs() * focal_length_mm / settings.aperture
    };

    let coc_pixels = coc * image_height_px / sensor_height_mm;

    let max_coc = 30.0;
    let blur_amount = (coc_pixels / max_coc).clamp(0.0, 1.0);

    DofResult {
        coc: coc_pixels,
        in_focus,
        blur_amount,
        is_foreground: depth < settings.focus_distance,
    }
}

/// Preview configuration for photo mode.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PreviewConfig {
    /// Preview resolution scale (0.25 to 1.0).
    pub resolution_scale: f32,
    /// Number of DOF samples.
    pub dof_samples: u32,
    /// Whether to show composition guides.
    pub show_guides: bool,
    /// Whether to apply filters in preview.
    pub apply_filters: bool,
    /// Bokeh shape for DOF preview.
    pub bokeh_shape: BokehShape,
}

impl Default for PreviewConfig {
    fn default() -> Self {
        Self {
            resolution_scale: 0.5,
            dof_samples: 8,
            show_guides: true,
            apply_filters: true,
            bokeh_shape: BokehShape::Circle,
        }
    }
}

impl PreviewConfig {
    /// Create new preview config.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set resolution scale.
    #[must_use]
    pub fn with_resolution_scale(mut self, scale: f32) -> Self {
        self.resolution_scale = scale.clamp(0.25, 1.0);
        self
    }

    /// Set DOF sample count.
    #[must_use]
    pub fn with_dof_samples(mut self, samples: u32) -> Self {
        self.dof_samples = samples.clamp(1, 64);
        self
    }

    /// Calculate preview dimensions.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "preview sizing: inputs are valid u32, scale is [0.25,1.0], result fits in u32"
    )]
    pub fn preview_size(&self, full_width: u32, full_height: u32) -> (u32, u32) {
        let w = (full_width as f32 * self.resolution_scale).round() as u32;
        let h = (full_height as f32 * self.resolution_scale).round() as u32;
        (w.max(1), h.max(1))
    }
}

/// Compute a stable fingerprint for preview configuration.
#[must_use]
pub fn compute_preview_fingerprint(config: &PreviewConfig, settings: &PhotoSettings) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    config.resolution_scale.to_bits().hash(&mut hasher);
    config.dof_samples.hash(&mut hasher);
    config.show_guides.hash(&mut hasher);
    config.apply_filters.hash(&mut hasher);
    config.bokeh_shape.hash(&mut hasher);
    settings.exposure.to_bits().hash(&mut hasher);
    settings.aperture.to_bits().hash(&mut hasher);
    settings.fov.to_bits().hash(&mut hasher);
    settings.filter.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_sampler_determinism() {
        let s1 = PhotoSampler::new(42);
        let s2 = PhotoSampler::new(42);

        assert_relative_eq!(s1.sample(0.5, 0.5), s2.sample(0.5, 0.5));
    }

    #[test]
    fn test_sampler_sensitivity() {
        let s1 = PhotoSampler::new(42);
        let s2 = PhotoSampler::new(43);

        assert!((s1.sample(0.5, 0.5) - s2.sample(0.5, 0.5)).abs() > f32::EPSILON);
    }

    #[test]
    fn test_sample_range() {
        let sampler = PhotoSampler::new(0);
        for i in 0..100_i16 {
            let x = f32::from(i) * 0.1;
            let y = f32::from(i) * 0.05;
            let val = sampler.sample(x, y);
            assert!((0.0..1.0).contains(&val));
        }
    }

    #[test]
    fn test_sample_disk() {
        let sampler = PhotoSampler::new(0);
        for i in 0..100_i16 {
            let (dx, dy) = sampler.sample_disk(f32::from(i), 0.0);
            let r = (dx * dx + dy * dy).sqrt();
            assert!(r <= 1.0 + 0.001);
        }
    }

    #[test]
    fn test_halton_sequence() {
        let h0 = halton(0, 2);
        let h1 = halton(1, 2);
        let h2 = halton(2, 2);

        assert_relative_eq!(h0, 0.0);
        assert_relative_eq!(h1, 0.5);
        assert_relative_eq!(h2, 0.25);
    }

    #[test]
    fn test_bokeh_shapes() {
        for shape in BokehShape::ALL {
            assert!(!shape.name().is_empty());
            let sampler = PhotoSampler::new(0);
            let (dx, dy) = sampler.sample_dof_offset(0.5, 0.5, 1.0, shape);
            assert!(dx.is_finite());
            assert!(dy.is_finite());
        }
    }

    #[test]
    fn test_dof_calculation() {
        let settings = PhotoSettings::default()
            .with_focus_distance(5.0)
            .with_aperture(2.8)
            .with_fov(70.0);

        let at_focus = calculate_dof(5.0, &settings, 24.0, 1080.0);
        assert!(at_focus.blur_amount < 0.1);

        let far_away = calculate_dof(50.0, &settings, 24.0, 1080.0);
        assert!(far_away.blur_amount > at_focus.blur_amount);
        assert!(!far_away.is_foreground);

        let close = calculate_dof(1.0, &settings, 24.0, 1080.0);
        assert!(close.is_foreground);
    }

    #[test]
    fn test_preview_config() {
        let config = PreviewConfig::new().with_resolution_scale(0.5);

        let (w, h) = config.preview_size(1920, 1080);
        assert_eq!(w, 960);
        assert_eq!(h, 540);
    }

    #[test]
    fn test_preview_fingerprint_determinism() {
        let config = PreviewConfig::default();
        let settings = PhotoSettings::default();

        let fp1 = compute_preview_fingerprint(&config, &settings);
        let fp2 = compute_preview_fingerprint(&config, &settings);

        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_preview_fingerprint_sensitivity() {
        let config = PreviewConfig::default();
        let settings1 = PhotoSettings::default();
        let settings2 = PhotoSettings::default().with_exposure(1.0);

        assert_ne!(
            compute_preview_fingerprint(&config, &settings1),
            compute_preview_fingerprint(&config, &settings2)
        );
    }

    #[test]
    fn test_film_grain() {
        let sampler = PhotoSampler::new(0);

        for i in 0..100_i16 {
            let grain = sampler.film_grain(f32::from(i), 0.0, 0.1);
            assert!((-0.05..=0.05).contains(&grain));
        }
    }

    #[test]
    fn test_serde_roundtrip() {
        let config = PreviewConfig::new()
            .with_resolution_scale(0.75)
            .with_dof_samples(16);

        let bytes = bincode::serialize(&config).expect("serialize");
        let restored: PreviewConfig = bincode::deserialize(&bytes).expect("deserialize");

        assert_relative_eq!(config.resolution_scale, restored.resolution_scale);
        assert_eq!(config.dof_samples, restored.dof_samples);
    }
}
