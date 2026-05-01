//! Summary and preview generation for weather authoring tools.
//!
//! Provides CPU-side evaluation and statistics for weather effects
//! without requiring a full simulation.

use super::curve::OverTimeCurve;
use super::effect::WeatherEffect;
use super::emitter::EmitterConfig;
use super::presets::WeatherPreset;
use super::sampling::ParticleSampler;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Summary of a weather effect for authoring tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherSummary {
    /// Effect kind name.
    pub kind_name: String,
    /// Estimated particles per second.
    pub spawn_rate: f32,
    /// Estimated max active particles.
    pub max_active: u32,
    /// Average particle lifetime.
    pub avg_lifetime: f32,
    /// Memory estimate in bytes.
    pub memory_estimate: usize,
    /// Performance impact (0.0 = negligible, 1.0 = heavy).
    pub performance_impact: f32,
    /// Spawn bounds (AABB).
    pub spawn_bounds: (Vec3, Vec3),
    /// Warnings for potential issues.
    pub warnings: Vec<String>,
}

impl WeatherSummary {
    /// Generate a summary from a weather effect and emitter config.
    #[must_use]
    pub fn from_config(effect: &WeatherEffect, emitter: &EmitterConfig) -> Self {
        let avg_lifetime = emitter.lifetime.midpoint();
        let max_active = emitter.estimated_max_active();
        let spawn_bounds = emitter.spawn_shape.aabb();

        let particle_size = std::mem::size_of::<super::uniform::ParticleInstance>();
        let memory_estimate = max_active as usize * particle_size;

        let performance_impact =
            Self::estimate_performance(emitter.spawn_rate, max_active, avg_lifetime);

        let warnings = Self::collect_warnings(effect, emitter, max_active);

        Self {
            kind_name: format!("{:?}", effect.kind),
            spawn_rate: emitter.spawn_rate,
            max_active,
            avg_lifetime,
            memory_estimate,
            performance_impact,
            spawn_bounds,
            warnings,
        }
    }

    /// Generate a summary from a preset.
    #[must_use]
    pub fn from_preset(preset: WeatherPreset, spawn_bounds: Vec3) -> Self {
        let effect = preset.effect();
        let emitter = preset.emitter(spawn_bounds);
        Self::from_config(&effect, &emitter)
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "max_active is bounded, precision loss acceptable"
    )]
    fn estimate_performance(spawn_rate: f32, max_active: u32, avg_lifetime: f32) -> f32 {
        let spawn_factor = (spawn_rate / 500.0).min(1.0);
        let count_factor = (max_active as f32 / 5000.0).min(1.0);
        let lifetime_factor = (avg_lifetime / 10.0).min(1.0);

        (spawn_factor * 0.3 + count_factor * 0.5 + lifetime_factor * 0.2).clamp(0.0, 1.0)
    }

    fn collect_warnings(
        effect: &WeatherEffect,
        emitter: &EmitterConfig,
        max_active: u32,
    ) -> Vec<String> {
        let mut warnings = Vec::new();

        if max_active > 10000 {
            warnings.push(format!(
                "High particle count ({max_active}); may impact performance"
            ));
        }

        if emitter.spawn_rate > 1000.0 {
            warnings.push(String::from("High spawn rate may cause frame drops"));
        }

        if emitter.lifetime.max > 15.0 {
            warnings.push(String::from(
                "Long particle lifetime; particles may accumulate",
            ));
        }

        if effect.turbulence > 1.5 {
            warnings.push(String::from("High turbulence is computationally expensive"));
        }

        if emitter.size.max > 0.2 {
            warnings.push(String::from("Large particles may cause overdraw issues"));
        }

        let (min, max) = emitter.spawn_shape.aabb();
        let volume = (max - min).abs();
        if volume.x > 500.0 || volume.y > 500.0 || volume.z > 500.0 {
            warnings.push(String::from("Very large spawn area may reduce density"));
        }

        warnings
    }

    /// Format as human-readable text.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut lines = vec![
            format!("Weather Effect: {}", self.kind_name),
            format!("Spawn Rate: {:.1} particles/sec", self.spawn_rate),
            format!("Max Active: {} particles", self.max_active),
            format!("Avg Lifetime: {:.2} sec", self.avg_lifetime),
            format!("Memory: {} KB", self.memory_estimate / 1024),
            format!(
                "Performance Impact: {:.0}%",
                self.performance_impact * 100.0
            ),
        ];

        if !self.warnings.is_empty() {
            lines.push(String::from("Warnings:"));
            for warning in &self.warnings {
                lines.push(format!("  - {warning}"));
            }
        }

        lines.join("\n")
    }
}

/// Preview of particle distribution for visualization.
#[derive(Debug, Clone)]
pub struct DistributionPreview {
    /// Sample positions.
    pub positions: Vec<Vec3>,
    /// Sample velocities.
    pub velocities: Vec<Vec3>,
    /// Sample sizes.
    pub sizes: Vec<f32>,
}

impl DistributionPreview {
    /// Generate a preview by sampling the emitter configuration.
    #[must_use]
    pub fn sample(emitter: &EmitterConfig, count: u32, seed: u32) -> Self {
        let sampler = ParticleSampler::new(seed);
        let mut positions = Vec::with_capacity(count as usize);
        let mut velocities = Vec::with_capacity(count as usize);
        let mut sizes = Vec::with_capacity(count as usize);

        for i in 0..count {
            let props = sampler.sample_particle_properties(i, emitter);
            positions.push(props.position);
            velocities.push(props.velocity);
            sizes.push(props.size);
        }

        Self {
            positions,
            velocities,
            sizes,
        }
    }

    /// Get the bounding box of sampled positions.
    #[must_use]
    pub fn bounds(&self) -> (Vec3, Vec3) {
        if self.positions.is_empty() {
            return (Vec3::ZERO, Vec3::ZERO);
        }

        let mut min = self.positions[0];
        let mut max = self.positions[0];

        for pos in &self.positions {
            min = min.min(*pos);
            max = max.max(*pos);
        }

        (min, max)
    }

    /// Get velocity statistics.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "sample count is bounded")]
    pub fn velocity_stats(&self) -> (f32, f32, f32) {
        if self.velocities.is_empty() {
            return (0.0, 0.0, 0.0);
        }

        let speeds: Vec<f32> = self.velocities.iter().map(|v| v.length()).collect();
        let min = speeds.iter().copied().fold(f32::INFINITY, f32::min);
        let max = speeds.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let avg = speeds.iter().sum::<f32>() / speeds.len() as f32;

        (min, max, avg)
    }

    /// Get size statistics.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "sample count is bounded")]
    pub fn size_stats(&self) -> (f32, f32, f32) {
        if self.sizes.is_empty() {
            return (0.0, 0.0, 0.0);
        }

        let min = self.sizes.iter().copied().fold(f32::INFINITY, f32::min);
        let max = self.sizes.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let avg = self.sizes.iter().sum::<f32>() / self.sizes.len() as f32;

        (min, max, avg)
    }
}

/// Preview of curve behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurvePreview {
    /// Sampled values at regular intervals.
    pub samples: Vec<f32>,
    /// Minimum value.
    pub min: f32,
    /// Maximum value.
    pub max: f32,
    /// Value at t=0.
    pub start: f32,
    /// Value at t=1.
    pub end: f32,
}

impl CurvePreview {
    /// Generate a preview of a curve.
    #[must_use]
    pub fn from_curve(curve: &OverTimeCurve, sample_count: usize) -> Self {
        let samples = curve.sample(sample_count);
        let (min, max) = curve.bounds();
        let start = curve.evaluate(0.0);
        let end = curve.evaluate(1.0);

        Self {
            samples,
            min,
            max,
            start,
            end,
        }
    }

    /// Format as ASCII art graph.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "grid dimensions are small, normalized values are clamped to valid range"
    )]
    pub fn to_ascii(&self, width: usize, height: usize) -> String {
        if self.samples.is_empty() || width == 0 || height == 0 {
            return String::new();
        }

        let range = self.max - self.min;
        let range = if range.abs() < 0.0001 { 1.0 } else { range };

        let mut grid = vec![vec![' '; width]; height];

        for (i, &value) in self.samples.iter().enumerate() {
            let x = (i * width) / self.samples.len().max(1);
            let normalized = (value - self.min) / range;
            let y = ((1.0 - normalized) * (height - 1) as f32).round() as usize;
            let y = y.min(height - 1);
            let x = x.min(width - 1);
            grid[y][x] = '*';
        }

        grid.iter()
            .map(|row| row.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Validation result for weather configurations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the configuration is valid.
    pub is_valid: bool,
    /// List of errors (if any).
    pub errors: Vec<String>,
    /// List of warnings.
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// Validate a weather effect.
    #[must_use]
    pub fn validate_effect(effect: &WeatherEffect) -> Self {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        if !effect.is_valid() {
            errors.push(String::from("Effect has invalid parameter values"));
        }

        if effect.intensity < 0.1 {
            warnings.push(String::from(
                "Very low intensity; effect may not be visible",
            ));
        }

        if effect.opacity < 0.1 {
            warnings.push(String::from(
                "Very low opacity; particles may not be visible",
            ));
        }

        if effect.particle_size < 0.005 {
            warnings.push(String::from("Very small particles; may not render visibly"));
        }

        Self {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    /// Validate an emitter configuration.
    #[must_use]
    pub fn validate_emitter(emitter: &EmitterConfig) -> Self {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        if !emitter.is_valid() {
            errors.push(String::from("Emitter has invalid parameter values"));
        }

        if !emitter.spawn_shape.is_valid() {
            errors.push(String::from("Spawn shape has invalid parameters"));
        }

        if emitter.spawn_rate == 0.0 && emitter.burst_count == 0 {
            warnings.push(String::from("No particles will be spawned"));
        }

        if emitter.lifetime.max < emitter.lifetime.min {
            errors.push(String::from("Lifetime max is less than min"));
        }

        if emitter.size.max < emitter.size.min {
            errors.push(String::from("Size max is less than min"));
        }

        Self {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    /// Combine multiple validation results.
    #[must_use]
    pub fn combine(results: &[Self]) -> Self {
        let is_valid = results.iter().all(|r| r.is_valid);
        let errors = results.iter().flat_map(|r| r.errors.clone()).collect();
        let warnings = results.iter().flat_map(|r| r.warnings.clone()).collect();

        Self {
            is_valid,
            errors,
            warnings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summary_from_preset() {
        let summary = WeatherSummary::from_preset(WeatherPreset::LightRain, Vec3::splat(50.0));

        assert!(!summary.kind_name.is_empty());
        assert!(summary.spawn_rate > 0.0);
        assert!(summary.max_active > 0);
        assert!(summary.avg_lifetime > 0.0);
    }

    #[test]
    fn test_summary_warnings_high_count() {
        use super::super::emitter::ValueRange;
        let emitter = EmitterConfig {
            spawn_rate: 5000.0,
            max_particles: 50000,
            lifetime: ValueRange::constant(10.0),
            ..Default::default()
        };

        let effect = WeatherEffect::rain();
        let summary = WeatherSummary::from_config(&effect, &emitter);

        assert!(!summary.warnings.is_empty());
    }

    #[test]
    fn test_summary_to_text() {
        let summary = WeatherSummary::from_preset(WeatherPreset::HeavyRain, Vec3::splat(50.0));
        let text = summary.to_text();

        assert!(text.contains("Weather Effect"));
        assert!(text.contains("Spawn Rate"));
    }

    #[test]
    fn test_distribution_preview() {
        let emitter = EmitterConfig::rain(Vec3::splat(50.0));
        let preview = DistributionPreview::sample(&emitter, 100, 42);

        assert_eq!(preview.positions.len(), 100);
        assert_eq!(preview.velocities.len(), 100);
        assert_eq!(preview.sizes.len(), 100);
    }

    #[test]
    fn test_distribution_bounds() {
        let emitter = EmitterConfig::rain(Vec3::splat(50.0));
        let preview = DistributionPreview::sample(&emitter, 100, 42);
        let (min, max) = preview.bounds();

        assert!(min.x <= max.x);
        assert!(min.y <= max.y);
        assert!(min.z <= max.z);
    }

    #[test]
    fn test_velocity_stats() {
        let emitter = EmitterConfig::rain(Vec3::splat(50.0));
        let preview = DistributionPreview::sample(&emitter, 100, 42);
        let (min, max, avg) = preview.velocity_stats();

        assert!(min <= avg);
        assert!(avg <= max);
    }

    #[test]
    fn test_curve_preview() {
        let curve = OverTimeCurve::linear(1.0, 0.0);
        let preview = CurvePreview::from_curve(&curve, 10);

        assert_eq!(preview.samples.len(), 10);
        assert!(preview.start > preview.end);
    }

    #[test]
    fn test_curve_ascii() {
        let curve = OverTimeCurve::linear(1.0, 0.0);
        let preview = CurvePreview::from_curve(&curve, 20);
        let ascii = preview.to_ascii(20, 5);

        assert!(!ascii.is_empty());
        assert!(ascii.contains('*'));
    }

    #[test]
    fn test_validation_valid_effect() {
        let effect = WeatherEffect::rain();
        let result = ValidationResult::validate_effect(&effect);

        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validation_valid_emitter() {
        let emitter = EmitterConfig::default();
        let result = ValidationResult::validate_emitter(&emitter);

        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validation_low_intensity_warning() {
        let effect = WeatherEffect::rain().with_intensity(0.05);
        let result = ValidationResult::validate_effect(&effect);

        assert!(result.is_valid);
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn test_validation_combine() {
        let r1 = ValidationResult {
            is_valid: true,
            errors: Vec::new(),
            warnings: vec![String::from("warning1")],
        };
        let r2 = ValidationResult {
            is_valid: false,
            errors: vec![String::from("error1")],
            warnings: Vec::new(),
        };

        let combined = ValidationResult::combine(&[r1, r2]);

        assert!(!combined.is_valid);
        assert_eq!(combined.errors.len(), 1);
        assert_eq!(combined.warnings.len(), 1);
    }

    #[test]
    fn test_size_stats() {
        let emitter = EmitterConfig::default();
        let preview = DistributionPreview::sample(&emitter, 50, 0);
        let (min, max, avg) = preview.size_stats();

        assert!(min > 0.0);
        assert!(max >= min);
        assert!(avg >= min);
        assert!(avg <= max);
    }

    #[test]
    fn test_empty_distribution() {
        let emitter = EmitterConfig {
            spawn_rate: 0.0,
            ..Default::default()
        };

        let preview = DistributionPreview::sample(&emitter, 0, 0);
        let (min, max) = preview.bounds();

        assert_eq!(min, Vec3::ZERO);
        assert_eq!(max, Vec3::ZERO);
    }
}
