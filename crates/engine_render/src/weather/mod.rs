//! Weather and particle effect authoring primitives.
//!
//! Provides CPU-side primitives for authoring weather and particle effects:
//! rain, snow, underwater, dust, spores, ash, vacuum, and fog.
//! These types configure GPU particle rendering passes.
//!
//! # Module Structure
//!
//! - [`effect`]: Weather effect types and configurations
//! - [`emitter`]: Particle emitter configurations
//! - [`shape`]: Spawn shapes for particle emission
//! - [`curve`]: Over-time curves for animating properties
//! - [`sampling`]: Deterministic sampling for reproducible effects
//! - [`presets`]: Ready-to-use weather configurations
//! - [`uniform`]: GPU-friendly data structures
//! - [`summary`]: Authoring tools and previews

mod curve;
mod effect;
mod emitter;
mod presets;
mod sampling;
mod shape;
mod summary;
mod uniform;

pub use curve::{ColorOverTime, CurvePreset, Keyframe, OverTimeCurve};
pub use effect::{WeatherEffect, WeatherKind};
pub use emitter::{EmissionMode, EmitterConfig, SimulationSpace, ValueRange, VelocityMode};
pub use presets::{WeatherPreset, create_from_preset, create_layered};
pub use sampling::{
    ParticleSampler, SampledParticle, SpawnPlan, plan_spawns, position_hash, sample_turbulence,
};
pub use shape::{SpawnShape, SpawnShapeKind};
pub use summary::{CurvePreview, DistributionPreview, ValidationResult, WeatherSummary};
pub use uniform::{
    EmitterConfigUniform, ParticleBatch, ParticleInstance, SpawnShapeUniform, WeatherEffectUniform,
    convert,
};

use std::hash::{Hash, Hasher};

/// Compute a stable fingerprint for a weather configuration.
#[must_use]
pub fn compute_fingerprint(effect: &WeatherEffect, emitter: &EmitterConfig) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    effect.kind.hash(&mut hasher);
    hash_f32(effect.intensity, &mut hasher);
    hash_f32(effect.gravity, &mut hasher);
    hash_f32(effect.turbulence, &mut hasher);
    hash_f32(effect.particle_size, &mut hasher);
    hash_vec3(&effect.wind, &mut hasher);
    hash_vec3(&effect.color, &mut hasher);
    emitter.spawn_shape.kind.hash(&mut hasher);
    hash_f32(emitter.spawn_rate, &mut hasher);
    emitter.max_particles.hash(&mut hasher);
    hash_f32(emitter.lifetime.min, &mut hasher);
    hash_f32(emitter.lifetime.max, &mut hasher);
    emitter.seed.hash(&mut hasher);
    hasher.finish()
}

/// Compute a fingerprint for just the effect.
#[must_use]
pub fn compute_effect_fingerprint(effect: &WeatherEffect) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    effect.kind.hash(&mut hasher);
    hash_f32(effect.intensity, &mut hasher);
    hash_f32(effect.gravity, &mut hasher);
    hash_f32(effect.turbulence, &mut hasher);
    hash_f32(effect.turbulence_frequency, &mut hasher);
    hash_f32(effect.particle_size, &mut hasher);
    hash_f32(effect.size_variation, &mut hasher);
    hash_f32(effect.opacity, &mut hasher);
    hash_vec3(&effect.wind, &mut hasher);
    hash_vec3(&effect.color, &mut hasher);
    effect.active.hash(&mut hasher);
    hasher.finish()
}

/// Compute a fingerprint for just the emitter.
#[must_use]
pub fn compute_emitter_fingerprint(emitter: &EmitterConfig) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    emitter.spawn_shape.kind.hash(&mut hasher);
    hash_vec3(&emitter.spawn_shape.center, &mut hasher);
    hash_vec3(&emitter.spawn_shape.extents, &mut hasher);
    hash_f32(emitter.spawn_rate, &mut hasher);
    emitter.max_particles.hash(&mut hasher);
    emitter.emission_mode.hash(&mut hasher);
    hash_f32(emitter.lifetime.min, &mut hasher);
    hash_f32(emitter.lifetime.max, &mut hasher);
    emitter.velocity_mode.hash(&mut hasher);
    hash_f32(emitter.speed.min, &mut hasher);
    hash_f32(emitter.speed.max, &mut hasher);
    hash_f32(emitter.gravity_multiplier, &mut hasher);
    hash_f32(emitter.drag, &mut hasher);
    emitter.seed.hash(&mut hasher);
    emitter.enabled.hash(&mut hasher);
    hasher.finish()
}

fn hash_f32(value: f32, hasher: &mut impl Hasher) {
    value.to_bits().hash(hasher);
}

fn hash_vec3(value: &glam::Vec3, hasher: &mut impl Hasher) {
    value.x.to_bits().hash(hasher);
    value.y.to_bits().hash(hasher);
    value.z.to_bits().hash(hasher);
}

/// Sort weather effects by spawn rate (higher first), then by kind.
pub fn sort_by_spawn_rate(configs: &mut [(WeatherEffect, EmitterConfig)]) {
    configs.sort_by(|a, b| {
        b.1.spawn_rate
            .partial_cmp(&a.1.spawn_rate)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| (a.0.kind as u8).cmp(&(b.0.kind as u8)))
    });
}

/// Sort weather effects by kind, then by intensity.
pub fn sort_by_kind(configs: &mut [(WeatherEffect, EmitterConfig)]) {
    configs.sort_by(|a, b| {
        (a.0.kind as u8).cmp(&(b.0.kind as u8)).then_with(|| {
            b.0.intensity
                .partial_cmp(&a.0.intensity)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });
}

/// Filter to only active effects.
pub fn filter_active(
    configs: &[(WeatherEffect, EmitterConfig)],
) -> Vec<(WeatherEffect, EmitterConfig)> {
    configs
        .iter()
        .filter(|(effect, emitter)| effect.active && emitter.enabled)
        .cloned()
        .collect()
}

/// Serialize a weather configuration to bytes (bincode).
///
/// # Errors
///
/// Returns an error if bincode serialization fails.
pub fn serialize_config(
    effect: &WeatherEffect,
    emitter: &EmitterConfig,
) -> Result<Vec<u8>, bincode::Error> {
    let data = (effect, emitter);
    bincode::serialize(&data)
}

/// Deserialize a weather configuration from bytes (bincode).
///
/// # Errors
///
/// Returns an error if bincode deserialization fails.
pub fn deserialize_config(bytes: &[u8]) -> Result<(WeatherEffect, EmitterConfig), bincode::Error> {
    bincode::deserialize(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn test_fingerprint_determinism() {
        let effect = WeatherEffect::rain();
        let emitter = EmitterConfig::rain(Vec3::splat(50.0));

        let fp1 = compute_fingerprint(&effect, &emitter);
        let fp2 = compute_fingerprint(&effect, &emitter);

        assert_eq!(fp1, fp2, "fingerprint must be deterministic");
    }

    #[test]
    fn test_fingerprint_sensitivity() {
        let effect1 = WeatherEffect::rain();
        let effect2 = WeatherEffect::snow();
        let emitter = EmitterConfig::default();

        let fp1 = compute_fingerprint(&effect1, &emitter);
        let fp2 = compute_fingerprint(&effect2, &emitter);

        assert_ne!(
            fp1, fp2,
            "different effects should have different fingerprints"
        );
    }

    #[test]
    fn test_effect_fingerprint() {
        let e1 = WeatherEffect::rain();
        let e2 = WeatherEffect::rain().with_intensity(0.9);

        let fp1 = compute_effect_fingerprint(&e1);
        let fp2 = compute_effect_fingerprint(&e2);

        assert_ne!(fp1, fp2, "different intensities should differ");
    }

    #[test]
    fn test_emitter_fingerprint() {
        let e1 = EmitterConfig::default();
        let e2 = EmitterConfig::default().with_spawn_rate(200.0);

        let fp1 = compute_emitter_fingerprint(&e1);
        let fp2 = compute_emitter_fingerprint(&e2);

        assert_ne!(fp1, fp2, "different spawn rates should differ");
    }

    #[test]
    fn test_sort_by_spawn_rate() {
        use approx::assert_relative_eq;

        let low = (
            WeatherEffect::rain(),
            EmitterConfig::default().with_spawn_rate(100.0),
        );
        let high = (
            WeatherEffect::snow(),
            EmitterConfig::default().with_spawn_rate(500.0),
        );
        let mid = (
            WeatherEffect::dust(),
            EmitterConfig::default().with_spawn_rate(300.0),
        );

        let mut configs = vec![low, high.clone(), mid];
        sort_by_spawn_rate(&mut configs);

        assert_relative_eq!(configs[0].1.spawn_rate, 500.0, epsilon = 0.001);
        assert_relative_eq!(configs[1].1.spawn_rate, 300.0, epsilon = 0.001);
        assert_relative_eq!(configs[2].1.spawn_rate, 100.0, epsilon = 0.001);
    }

    #[test]
    fn test_sort_by_kind() {
        let rain = (WeatherEffect::rain(), EmitterConfig::default());
        let snow = (WeatherEffect::snow(), EmitterConfig::default());
        let dust = (WeatherEffect::dust(), EmitterConfig::default());

        let mut configs = vec![dust.clone(), rain.clone(), snow.clone()];
        sort_by_kind(&mut configs);

        assert_eq!(configs[0].0.kind, WeatherKind::Rain);
        assert_eq!(configs[1].0.kind, WeatherKind::Snow);
    }

    #[test]
    fn test_filter_active() {
        let active = (WeatherEffect::rain(), EmitterConfig::default());
        let inactive_effect = (
            WeatherEffect::snow().with_active(false),
            EmitterConfig::default(),
        );
        let inactive_emitter = (
            WeatherEffect::dust(),
            EmitterConfig::default().with_enabled(false),
        );

        let configs = vec![active.clone(), inactive_effect, inactive_emitter];
        let filtered = filter_active(&configs);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0.kind, WeatherKind::Rain);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let effect = WeatherEffect::rain().with_intensity(0.75);
        let emitter = EmitterConfig::rain(Vec3::splat(50.0));

        let bytes = serialize_config(&effect, &emitter).expect("serialize");
        let (e2, em2) = deserialize_config(&bytes).expect("deserialize");

        assert_eq!(effect.kind, e2.kind);
        assert!((effect.intensity - e2.intensity).abs() < 0.001);
        assert!((emitter.spawn_rate - em2.spawn_rate).abs() < 0.001);
    }

    #[test]
    fn test_serialization_preserves_fingerprint() {
        let effect = WeatherEffect::snow();
        let emitter = EmitterConfig::snow(Vec3::splat(100.0));

        let fp_before = compute_fingerprint(&effect, &emitter);

        let bytes = serialize_config(&effect, &emitter).expect("serialize");
        let (e2, em2) = deserialize_config(&bytes).expect("deserialize");

        let fp_after = compute_fingerprint(&e2, &em2);

        assert_eq!(
            fp_before, fp_after,
            "fingerprint should survive serialization"
        );
    }

    #[test]
    fn test_all_presets_serialize() {
        let bounds = Vec3::splat(50.0);
        for preset in WeatherPreset::ALL {
            let (effect, emitter) = create_from_preset(preset, bounds);
            let bytes = serialize_config(&effect, &emitter);
            assert!(bytes.is_ok(), "{preset:?} should serialize");
        }
    }

    #[test]
    fn test_create_layered() {
        let presets = [WeatherPreset::LightRain, WeatherPreset::LightFog];
        let layers = create_layered(&presets, Vec3::splat(50.0));

        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0].0.kind, WeatherKind::Rain);
        assert_eq!(layers[1].0.kind, WeatherKind::Fog);
    }

    #[test]
    fn test_module_reexports() {
        let _ = WeatherEffect::rain();
        let _ = WeatherKind::Snow;
        let _ = EmitterConfig::default();
        let _ = SpawnShape::sphere(Vec3::ZERO, 5.0);
        let _ = OverTimeCurve::linear(0.0, 1.0);
        let _ = ParticleSampler::new(0);
        let _ = WeatherPreset::Blizzard;
        let _ = WeatherSummary::from_preset(WeatherPreset::HeavyRain, Vec3::splat(50.0));
        let _ = ParticleBatch::new(100);
    }
}
