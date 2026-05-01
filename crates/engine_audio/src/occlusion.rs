//! Audio obstruction and occlusion primitives.
//!
//! Models sound propagation through materials with absorption, transmission,
//! reflection, and low-pass filtering properties.

use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

/// Acoustic material type for sound propagation modeling.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum AcousticMaterial {
    /// Open air, minimal absorption.
    #[default]
    Air = 0,
    /// Dense rock/concrete, high absorption.
    Stone = 1,
    /// Wooden surfaces, moderate absorption.
    Wood = 2,
    /// Metal surfaces, high reflection.
    Metal = 3,
    /// Glass, mixed transmission/reflection.
    Glass = 4,
    /// Water/liquids, frequency-dependent absorption.
    Liquid = 5,
    /// Organic materials (flesh, plants), high absorption.
    Organic = 6,
    /// Soil/dirt, very high absorption.
    Earth = 7,
    /// Fabric/cloth, extreme absorption.
    Fabric = 8,
}

impl AcousticMaterial {
    /// All material variants in order.
    pub const ALL: [AcousticMaterial; 9] = [
        AcousticMaterial::Air,
        AcousticMaterial::Stone,
        AcousticMaterial::Wood,
        AcousticMaterial::Metal,
        AcousticMaterial::Glass,
        AcousticMaterial::Liquid,
        AcousticMaterial::Organic,
        AcousticMaterial::Earth,
        AcousticMaterial::Fabric,
    ];

    /// Get the default acoustic profile for this material.
    #[must_use]
    pub fn profile(&self) -> MaterialProfile {
        match self {
            AcousticMaterial::Air => MaterialProfile::AIR,
            AcousticMaterial::Stone => MaterialProfile::STONE,
            AcousticMaterial::Wood => MaterialProfile::WOOD,
            AcousticMaterial::Metal => MaterialProfile::METAL,
            AcousticMaterial::Glass => MaterialProfile::GLASS,
            AcousticMaterial::Liquid => MaterialProfile::LIQUID,
            AcousticMaterial::Organic => MaterialProfile::ORGANIC,
            AcousticMaterial::Earth => MaterialProfile::EARTH,
            AcousticMaterial::Fabric => MaterialProfile::FABRIC,
        }
    }
}

/// Acoustic properties of a material.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterialProfile {
    /// Sound absorption coefficient (0.0 = none, 1.0 = full).
    pub absorption: f32,
    /// Sound transmission coefficient (0.0 = none, 1.0 = full).
    pub transmission: f32,
    /// Sound reflection coefficient (0.0 = none, 1.0 = full).
    pub reflection: f32,
    /// Low-pass filter cutoff factor (1.0 = no filtering, 0.0 = full).
    pub lowpass_factor: f32,
    /// Reverb coloration tint (influences reverb character).
    pub reverb_color: Vec3,
    /// Thickness in meters for attenuation calculation.
    pub thickness: f32,
}

impl MaterialProfile {
    /// Air: minimal effect on sound.
    pub const AIR: MaterialProfile = MaterialProfile {
        absorption: 0.001,
        transmission: 0.999,
        reflection: 0.0,
        lowpass_factor: 1.0,
        reverb_color: Vec3::new(1.0, 1.0, 1.0),
        thickness: 0.0,
    };

    /// Stone: heavy absorption, low transmission.
    pub const STONE: MaterialProfile = MaterialProfile {
        absorption: 0.85,
        transmission: 0.05,
        reflection: 0.10,
        lowpass_factor: 0.25,
        reverb_color: Vec3::new(0.8, 0.7, 0.6),
        thickness: 0.5,
    };

    /// Wood: moderate absorption, some transmission.
    pub const WOOD: MaterialProfile = MaterialProfile {
        absorption: 0.45,
        transmission: 0.35,
        reflection: 0.20,
        lowpass_factor: 0.55,
        reverb_color: Vec3::new(0.9, 0.75, 0.5),
        thickness: 0.15,
    };

    /// Metal: low absorption, high reflection.
    pub const METAL: MaterialProfile = MaterialProfile {
        absorption: 0.15,
        transmission: 0.15,
        reflection: 0.70,
        lowpass_factor: 0.70,
        reverb_color: Vec3::new(0.6, 0.7, 0.9),
        thickness: 0.02,
    };

    /// Glass: moderate transmission, some reflection.
    pub const GLASS: MaterialProfile = MaterialProfile {
        absorption: 0.25,
        transmission: 0.55,
        reflection: 0.20,
        lowpass_factor: 0.80,
        reverb_color: Vec3::new(0.9, 0.95, 1.0),
        thickness: 0.01,
    };

    /// Liquid: frequency-dependent absorption.
    pub const LIQUID: MaterialProfile = MaterialProfile {
        absorption: 0.70,
        transmission: 0.25,
        reflection: 0.05,
        lowpass_factor: 0.30,
        reverb_color: Vec3::new(0.5, 0.6, 0.9),
        thickness: 1.0,
    };

    /// Organic: high absorption (flesh, plants).
    pub const ORGANIC: MaterialProfile = MaterialProfile {
        absorption: 0.80,
        transmission: 0.15,
        reflection: 0.05,
        lowpass_factor: 0.35,
        reverb_color: Vec3::new(0.7, 0.6, 0.5),
        thickness: 0.3,
    };

    /// Earth: very high absorption.
    pub const EARTH: MaterialProfile = MaterialProfile {
        absorption: 0.95,
        transmission: 0.03,
        reflection: 0.02,
        lowpass_factor: 0.15,
        reverb_color: Vec3::new(0.6, 0.5, 0.4),
        thickness: 0.5,
    };

    /// Fabric: extreme absorption.
    pub const FABRIC: MaterialProfile = MaterialProfile {
        absorption: 0.90,
        transmission: 0.08,
        reflection: 0.02,
        lowpass_factor: 0.40,
        reverb_color: Vec3::new(0.8, 0.7, 0.7),
        thickness: 0.05,
    };

    /// Create a custom profile.
    #[must_use]
    pub const fn new(
        absorption: f32,
        transmission: f32,
        reflection: f32,
        lowpass_factor: f32,
        reverb_color: Vec3,
        thickness: f32,
    ) -> Self {
        Self {
            absorption,
            transmission,
            reflection,
            lowpass_factor,
            reverb_color,
            thickness,
        }
    }

    /// Validate that coefficients sum to approximately 1.0.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        let sum = self.absorption + self.transmission + self.reflection;
        (0.99..=1.01).contains(&sum)
            && (0.0..=1.0).contains(&self.lowpass_factor)
            && self.thickness >= 0.0
    }

    /// Normalize coefficients to sum to 1.0.
    #[must_use]
    pub fn normalized(&self) -> Self {
        let sum = self.absorption + self.transmission + self.reflection;
        if sum > 0.0 {
            Self {
                absorption: self.absorption / sum,
                transmission: self.transmission / sum,
                reflection: self.reflection / sum,
                lowpass_factor: self.lowpass_factor.clamp(0.0, 1.0),
                reverb_color: self.reverb_color,
                thickness: self.thickness.max(0.0),
            }
        } else {
            Self::AIR
        }
    }

    /// Blend two profiles by weight.
    #[must_use]
    pub fn blend(&self, other: &Self, weight: f32) -> Self {
        let w = weight.clamp(0.0, 1.0);
        let inv_w = 1.0 - w;
        Self {
            absorption: self.absorption * inv_w + other.absorption * w,
            transmission: self.transmission * inv_w + other.transmission * w,
            reflection: self.reflection * inv_w + other.reflection * w,
            lowpass_factor: self.lowpass_factor * inv_w + other.lowpass_factor * w,
            reverb_color: self.reverb_color * inv_w + other.reverb_color * w,
            thickness: self.thickness * inv_w + other.thickness * w,
        }
    }
}

impl Default for MaterialProfile {
    fn default() -> Self {
        Self::AIR
    }
}

/// A sample point along an occlusion path.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct OcclusionSample {
    /// Position of this sample.
    pub position: Vec3,
    /// Material at this sample point.
    pub material: AcousticMaterial,
    /// Distance traveled through this material (meters).
    pub distance: f32,
    /// Custom profile override (if any).
    pub profile_override: Option<MaterialProfile>,
}

impl OcclusionSample {
    /// Create a new sample.
    #[must_use]
    pub fn new(position: Vec3, material: AcousticMaterial, distance: f32) -> Self {
        Self {
            position,
            material,
            distance,
            profile_override: None,
        }
    }

    /// Create a sample with a custom profile.
    #[must_use]
    pub fn with_profile(mut self, profile: MaterialProfile) -> Self {
        self.profile_override = Some(profile);
        self
    }

    /// Get the effective profile for this sample.
    #[must_use]
    pub fn effective_profile(&self) -> MaterialProfile {
        self.profile_override
            .unwrap_or_else(|| self.material.profile())
    }
}

/// Path from sound source to listener with material samples.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ObstructionPath {
    /// Source position.
    pub source: Vec3,
    /// Listener position.
    pub listener: Vec3,
    /// Material samples along the path (ordered source to listener).
    pub samples: Vec<OcclusionSample>,
}

impl ObstructionPath {
    /// Create an empty path.
    #[must_use]
    pub fn new(source: Vec3, listener: Vec3) -> Self {
        Self {
            source,
            listener,
            samples: Vec::new(),
        }
    }

    /// Add a sample to the path.
    pub fn add_sample(&mut self, sample: OcclusionSample) {
        self.samples.push(sample);
    }

    /// Builder: add a sample.
    #[must_use]
    pub fn with_sample(mut self, sample: OcclusionSample) -> Self {
        self.samples.push(sample);
        self
    }

    /// Total direct distance from source to listener.
    #[must_use]
    pub fn total_distance(&self) -> f32 {
        self.source.distance(self.listener)
    }

    /// Total material distance (sum of all sample distances).
    #[must_use]
    pub fn material_distance(&self) -> f32 {
        self.samples.iter().map(|s| s.distance).sum()
    }

    /// Sort samples by distance from source.
    pub fn sort_by_distance_from_source(&mut self) {
        self.samples.sort_by(|a, b| {
            let da = a.position.distance(self.source);
            let db = b.position.distance(self.source);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// Check if path is clear (no solid materials).
    #[must_use]
    pub fn is_clear(&self) -> bool {
        self.samples
            .iter()
            .all(|s| s.material == AcousticMaterial::Air)
    }

    /// Get unique materials in this path.
    #[must_use]
    pub fn unique_materials(&self) -> Vec<AcousticMaterial> {
        let mut mats: Vec<_> = self.samples.iter().map(|s| s.material).collect();
        mats.sort_by_key(|m| *m as u8);
        mats.dedup();
        mats
    }
}

/// Result of occlusion computation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct OcclusionResult {
    /// Overall gain attenuation (0.0 = silent, 1.0 = full volume).
    pub gain: f32,
    /// Low-pass filter cutoff factor (1.0 = no filter, 0.0 = full).
    pub lowpass: f32,
    /// Transmission contribution (direct through materials).
    pub transmission: f32,
    /// Reflection contribution (diffracted around).
    pub reflection: f32,
    /// Weighted reverb color from materials.
    pub reverb_color: Vec3,
    /// Number of material layers penetrated.
    pub layer_count: u32,
    /// Fingerprint of this result.
    pub fingerprint: u64,
}

impl OcclusionResult {
    /// Unoccluded result (clear line of sight).
    pub const CLEAR: OcclusionResult = OcclusionResult {
        gain: 1.0,
        lowpass: 1.0,
        transmission: 1.0,
        reflection: 0.0,
        reverb_color: Vec3::ONE,
        layer_count: 0,
        fingerprint: 0,
    };

    /// Fully occluded (silent).
    pub const SILENT: OcclusionResult = OcclusionResult {
        gain: 0.0,
        lowpass: 0.0,
        transmission: 0.0,
        reflection: 0.0,
        reverb_color: Vec3::ZERO,
        layer_count: 0,
        fingerprint: 0,
    };

    /// Check if sound is effectively inaudible.
    #[must_use]
    pub fn is_inaudible(&self) -> bool {
        self.gain < 0.001
    }

    /// Check if heavily occluded (muffled).
    #[must_use]
    pub fn is_muffled(&self) -> bool {
        self.lowpass < 0.5
    }
}

impl Default for OcclusionResult {
    fn default() -> Self {
        Self::CLEAR
    }
}

/// Compute occlusion result for a path.
#[must_use]
pub fn compute_occlusion(path: &ObstructionPath) -> OcclusionResult {
    if path.samples.is_empty() {
        return OcclusionResult::CLEAR;
    }

    let mut gain = 1.0_f32;
    let mut lowpass = 1.0_f32;
    let mut transmission_sum = 0.0_f32;
    let mut reflection_sum = 0.0_f32;
    let mut reverb_color = Vec3::ZERO;
    let mut total_weight = 0.0_f32;
    let mut layer_count = 0_u32;

    for sample in &path.samples {
        if sample.material == AcousticMaterial::Air {
            continue;
        }

        layer_count += 1;
        let profile = sample.effective_profile();
        let thickness_factor = (sample.distance / profile.thickness.max(0.01)).min(10.0);

        let attenuation = (profile.absorption * thickness_factor).min(0.99);
        gain *= 1.0 - attenuation;
        lowpass *= profile.lowpass_factor.powf(thickness_factor.sqrt());

        transmission_sum += profile.transmission * (1.0 - attenuation);
        reflection_sum += profile.reflection * (1.0 - attenuation);

        let weight = sample.distance;
        reverb_color += profile.reverb_color * weight;
        total_weight += weight;
    }

    if total_weight > 0.0 {
        reverb_color /= total_weight;
    } else {
        reverb_color = Vec3::ONE;
    }

    let total_contribution = transmission_sum + reflection_sum;
    let (transmission, reflection) = if total_contribution > 0.0 {
        (
            transmission_sum / total_contribution,
            reflection_sum / total_contribution,
        )
    } else {
        (1.0, 0.0)
    };

    let fingerprint =
        compute_result_fingerprint(gain, lowpass, transmission, reflection, layer_count);

    OcclusionResult {
        gain: gain.max(0.0),
        lowpass: lowpass.clamp(0.0, 1.0),
        transmission,
        reflection,
        reverb_color,
        layer_count,
        fingerprint,
    }
}

/// Compute a deterministic fingerprint for an occlusion result.
fn compute_result_fingerprint(
    gain: f32,
    lowpass: f32,
    transmission: f32,
    reflection: f32,
    layer_count: u32,
) -> u64 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(&gain.to_le_bytes());
    hasher.update(&lowpass.to_le_bytes());
    hasher.update(&transmission.to_le_bytes());
    hasher.update(&reflection.to_le_bytes());
    hasher.update(&layer_count.to_le_bytes());
    u64::from(hasher.finalize())
}

/// Compute fingerprint for an obstruction path.
#[must_use]
pub fn compute_path_fingerprint(path: &ObstructionPath) -> u64 {
    let mut hasher = crc32fast::Hasher::new();
    hash_vec3(&path.source, &mut hasher);
    hash_vec3(&path.listener, &mut hasher);
    #[allow(clippy::cast_possible_truncation)]
    let sample_count = path.samples.len() as u32;
    hasher.update(&sample_count.to_le_bytes());
    for sample in &path.samples {
        hash_vec3(&sample.position, &mut hasher);
        hasher.update(&[sample.material as u8]);
        hasher.update(&sample.distance.to_le_bytes());
    }
    u64::from(hasher.finalize())
}

fn hash_vec3(v: &Vec3, hasher: &mut crc32fast::Hasher) {
    hasher.update(&v.x.to_le_bytes());
    hasher.update(&v.y.to_le_bytes());
    hasher.update(&v.z.to_le_bytes());
}

/// Material stack summary for debugging/display.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MaterialStackSummary {
    /// Materials and their total distances.
    pub layers: Vec<(AcousticMaterial, f32)>,
    /// Total solid material distance.
    pub total_solid_distance: f32,
    /// Dominant material (by distance).
    pub dominant: Option<AcousticMaterial>,
}

impl MaterialStackSummary {
    /// Build summary from a path.
    #[must_use]
    pub fn from_path(path: &ObstructionPath) -> Self {
        let mut material_distances: std::collections::HashMap<AcousticMaterial, f32> =
            std::collections::HashMap::new();

        for sample in &path.samples {
            *material_distances.entry(sample.material).or_default() += sample.distance;
        }

        let mut layers: Vec<_> = material_distances.into_iter().collect();
        layers.sort_by(|a, b| (a.0 as u8).cmp(&(b.0 as u8)));

        let total_solid_distance: f32 = layers
            .iter()
            .filter(|(m, _)| *m != AcousticMaterial::Air)
            .map(|(_, d)| d)
            .sum();

        let dominant = layers
            .iter()
            .filter(|(m, _)| *m != AcousticMaterial::Air)
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(m, _)| *m);

        Self {
            layers,
            total_solid_distance,
            dominant,
        }
    }
}

/// Serialize occlusion data to bincode.
///
/// # Errors
///
/// Returns error if serialization fails.
pub fn serialize_path(path: &ObstructionPath) -> Result<Vec<u8>, bincode::Error> {
    bincode::serialize(path)
}

/// Deserialize occlusion data from bincode.
///
/// # Errors
///
/// Returns error if deserialization fails.
pub fn deserialize_path(bytes: &[u8]) -> Result<ObstructionPath, bincode::Error> {
    bincode::deserialize(bytes)
}

/// Serialize occlusion result to bincode.
///
/// # Errors
///
/// Returns error if serialization fails.
pub fn serialize_result(result: &OcclusionResult) -> Result<Vec<u8>, bincode::Error> {
    bincode::serialize(result)
}

/// Deserialize occlusion result from bincode.
///
/// # Errors
///
/// Returns error if deserialization fails.
pub fn deserialize_result(bytes: &[u8]) -> Result<OcclusionResult, bincode::Error> {
    bincode::deserialize(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_material_all_variants() {
        const EXPECTED: [u8; 9] = [0, 1, 2, 3, 4, 5, 6, 7, 8];
        assert_eq!(AcousticMaterial::ALL.len(), 9);
        for (mat, expected) in AcousticMaterial::ALL.iter().zip(EXPECTED.iter()) {
            assert_eq!(*mat as u8, *expected);
        }
    }

    #[test]
    fn test_material_default_profiles() {
        for mat in AcousticMaterial::ALL {
            let profile = mat.profile();
            assert!(profile.is_valid(), "{mat:?} profile should be valid");
        }
    }

    #[test]
    fn test_profile_normalization() {
        let unbalanced = MaterialProfile::new(0.5, 0.3, 0.1, 0.8, Vec3::ONE, 0.1);
        assert!(!unbalanced.is_valid());

        let normalized = unbalanced.normalized();
        assert!(normalized.is_valid());
        assert_relative_eq!(
            normalized.absorption + normalized.transmission + normalized.reflection,
            1.0,
            epsilon = 0.01
        );
    }

    #[test]
    fn test_profile_blend() {
        let a = MaterialProfile::STONE;
        let b = MaterialProfile::WOOD;
        let blended = a.blend(&b, 0.5);

        assert_relative_eq!(
            blended.absorption,
            f32::midpoint(a.absorption, b.absorption),
            epsilon = 0.01
        );
        assert_relative_eq!(
            blended.transmission,
            f32::midpoint(a.transmission, b.transmission),
            epsilon = 0.01
        );
    }

    #[test]
    fn test_clear_path() {
        let path = ObstructionPath::new(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
        let result = compute_occlusion(&path);

        assert_relative_eq!(result.gain, 1.0, epsilon = 0.001);
        assert_relative_eq!(result.lowpass, 1.0, epsilon = 0.001);
        assert_eq!(result.layer_count, 0);
    }

    #[test]
    fn test_single_material_occlusion() {
        let path = ObstructionPath::new(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0)).with_sample(
            OcclusionSample::new(Vec3::new(5.0, 0.0, 0.0), AcousticMaterial::Stone, 0.5),
        );

        let result = compute_occlusion(&path);

        assert!(result.gain < 1.0, "gain should be attenuated");
        assert!(result.lowpass < 1.0, "should have low-pass filtering");
        assert_eq!(result.layer_count, 1);
    }

    #[test]
    fn test_multiple_materials() {
        let path = ObstructionPath::new(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0))
            .with_sample(OcclusionSample::new(
                Vec3::new(3.0, 0.0, 0.0),
                AcousticMaterial::Wood,
                0.2,
            ))
            .with_sample(OcclusionSample::new(
                Vec3::new(6.0, 0.0, 0.0),
                AcousticMaterial::Stone,
                0.5,
            ));

        let result = compute_occlusion(&path);

        assert!(result.gain < 0.5, "should be heavily attenuated");
        assert_eq!(result.layer_count, 2);
    }

    #[test]
    fn test_air_samples_ignored() {
        let path = ObstructionPath::new(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0)).with_sample(
            OcclusionSample::new(Vec3::new(5.0, 0.0, 0.0), AcousticMaterial::Air, 5.0),
        );

        let result = compute_occlusion(&path);

        assert_relative_eq!(result.gain, 1.0, epsilon = 0.001);
        assert_eq!(result.layer_count, 0);
    }

    #[test]
    fn test_path_fingerprint_deterministic() {
        let path = ObstructionPath::new(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0)).with_sample(
            OcclusionSample::new(Vec3::new(5.0, 0.0, 0.0), AcousticMaterial::Stone, 0.5),
        );

        let fp1 = compute_path_fingerprint(&path);
        let fp2 = compute_path_fingerprint(&path);

        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_path_fingerprint_sensitive() {
        let path1 = ObstructionPath::new(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0)).with_sample(
            OcclusionSample::new(Vec3::new(5.0, 0.0, 0.0), AcousticMaterial::Stone, 0.5),
        );

        let path2 = ObstructionPath::new(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0)).with_sample(
            OcclusionSample::new(Vec3::new(5.0, 0.0, 0.0), AcousticMaterial::Wood, 0.5),
        );

        assert_ne!(
            compute_path_fingerprint(&path1),
            compute_path_fingerprint(&path2)
        );
    }

    #[test]
    fn test_result_fingerprint_in_result() {
        let path = ObstructionPath::new(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0)).with_sample(
            OcclusionSample::new(Vec3::new(5.0, 0.0, 0.0), AcousticMaterial::Stone, 0.5),
        );

        let result = compute_occlusion(&path);

        assert_ne!(result.fingerprint, 0);
    }

    #[test]
    fn test_material_stack_summary() {
        let path = ObstructionPath::new(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0))
            .with_sample(OcclusionSample::new(
                Vec3::new(3.0, 0.0, 0.0),
                AcousticMaterial::Wood,
                0.2,
            ))
            .with_sample(OcclusionSample::new(
                Vec3::new(6.0, 0.0, 0.0),
                AcousticMaterial::Stone,
                0.5,
            ))
            .with_sample(OcclusionSample::new(
                Vec3::new(7.0, 0.0, 0.0),
                AcousticMaterial::Stone,
                0.3,
            ));

        let summary = MaterialStackSummary::from_path(&path);

        assert_eq!(summary.layers.len(), 2);
        assert_eq!(summary.dominant, Some(AcousticMaterial::Stone));
        assert_relative_eq!(summary.total_solid_distance, 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_path_sort_by_distance() {
        let mut path = ObstructionPath::new(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
        path.add_sample(OcclusionSample::new(
            Vec3::new(8.0, 0.0, 0.0),
            AcousticMaterial::Metal,
            0.1,
        ));
        path.add_sample(OcclusionSample::new(
            Vec3::new(2.0, 0.0, 0.0),
            AcousticMaterial::Wood,
            0.1,
        ));
        path.add_sample(OcclusionSample::new(
            Vec3::new(5.0, 0.0, 0.0),
            AcousticMaterial::Stone,
            0.1,
        ));

        path.sort_by_distance_from_source();

        assert_eq!(path.samples[0].material, AcousticMaterial::Wood);
        assert_eq!(path.samples[1].material, AcousticMaterial::Stone);
        assert_eq!(path.samples[2].material, AcousticMaterial::Metal);
    }

    #[test]
    fn test_path_unique_materials() {
        let path = ObstructionPath::new(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0))
            .with_sample(OcclusionSample::new(
                Vec3::new(3.0, 0.0, 0.0),
                AcousticMaterial::Stone,
                0.2,
            ))
            .with_sample(OcclusionSample::new(
                Vec3::new(6.0, 0.0, 0.0),
                AcousticMaterial::Wood,
                0.2,
            ))
            .with_sample(OcclusionSample::new(
                Vec3::new(8.0, 0.0, 0.0),
                AcousticMaterial::Stone,
                0.3,
            ));

        let unique = path.unique_materials();
        assert_eq!(unique.len(), 2);
        assert!(unique.contains(&AcousticMaterial::Stone));
        assert!(unique.contains(&AcousticMaterial::Wood));
    }

    #[test]
    fn test_bincode_path_roundtrip() {
        let path = ObstructionPath::new(Vec3::ZERO, Vec3::new(10.0, 5.0, 3.0)).with_sample(
            OcclusionSample::new(Vec3::new(5.0, 2.5, 1.5), AcousticMaterial::Stone, 0.5),
        );

        let bytes = serialize_path(&path).expect("serialize");
        let recovered = deserialize_path(&bytes).expect("deserialize");

        assert_eq!(recovered.source, path.source);
        assert_eq!(recovered.listener, path.listener);
        assert_eq!(recovered.samples.len(), path.samples.len());
        assert_eq!(recovered.samples[0].material, path.samples[0].material);
    }

    #[test]
    fn test_bincode_result_roundtrip() {
        let path = ObstructionPath::new(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0)).with_sample(
            OcclusionSample::new(Vec3::new(5.0, 0.0, 0.0), AcousticMaterial::Metal, 0.1),
        );

        let result = compute_occlusion(&path);
        let bytes = serialize_result(&result).expect("serialize");
        let recovered = deserialize_result(&bytes).expect("deserialize");

        assert_relative_eq!(recovered.gain, result.gain, epsilon = 0.0001);
        assert_relative_eq!(recovered.lowpass, result.lowpass, epsilon = 0.0001);
        assert_eq!(recovered.fingerprint, result.fingerprint);
    }

    #[test]
    fn test_serialization_preserves_fingerprint() {
        let path =
            ObstructionPath::new(Vec3::new(1.0, 2.0, 3.0), Vec3::new(10.0, 5.0, 0.0)).with_sample(
                OcclusionSample::new(Vec3::new(5.0, 3.0, 1.0), AcousticMaterial::Glass, 0.05),
            );

        let fp_before = compute_path_fingerprint(&path);
        let bytes = serialize_path(&path).expect("serialize");
        let recovered = deserialize_path(&bytes).expect("deserialize");
        let fp_after = compute_path_fingerprint(&recovered);

        assert_eq!(fp_before, fp_after);
    }

    #[test]
    fn test_occlusion_result_helpers() {
        let inaudible = OcclusionResult {
            gain: 0.0001,
            ..Default::default()
        };
        assert!(inaudible.is_inaudible());

        let muffled = OcclusionResult {
            lowpass: 0.3,
            ..Default::default()
        };
        assert!(muffled.is_muffled());

        let clear = OcclusionResult::CLEAR;
        assert!(!clear.is_inaudible());
        assert!(!clear.is_muffled());
    }

    #[test]
    fn test_custom_profile_override() {
        let custom = MaterialProfile::new(0.5, 0.3, 0.2, 0.6, Vec3::new(1.0, 0.5, 0.5), 0.25);
        let sample =
            OcclusionSample::new(Vec3::ZERO, AcousticMaterial::Stone, 0.5).with_profile(custom);

        let profile = sample.effective_profile();
        assert_relative_eq!(profile.absorption, 0.5, epsilon = 0.001);
    }

    #[test]
    fn test_thick_material_high_attenuation() {
        let thin = ObstructionPath::new(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0)).with_sample(
            OcclusionSample::new(Vec3::new(5.0, 0.0, 0.0), AcousticMaterial::Stone, 0.1),
        );

        let thick = ObstructionPath::new(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0)).with_sample(
            OcclusionSample::new(Vec3::new(5.0, 0.0, 0.0), AcousticMaterial::Stone, 2.0),
        );

        let thin_result = compute_occlusion(&thin);
        let thick_result = compute_occlusion(&thick);

        assert!(
            thick_result.gain < thin_result.gain,
            "thicker material should attenuate more"
        );
    }
}
