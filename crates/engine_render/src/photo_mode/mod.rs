//! Photo mode and cinematic camera system.
//!
//! Provides CPU-side primitives for photo mode, cinematic camera paths,
//! composition guides, and screenshot capture configuration.
//!
//! # Module Structure
//!
//! - [`settings`]: Core photo mode settings (exposure, aperture, focus, filters)
//! - [`easing`]: Easing functions for smooth camera interpolation
//! - [`keyframe`]: Camera keyframes for path animation
//! - [`path`]: Camera paths and playback control
//! - [`framing`]: Composition guides and shot framing utilities
//! - [`sampling`]: Deterministic sampling for DOF and previews
//! - [`uniform`]: GPU-friendly data structures

mod easing;
mod framing;
mod keyframe;
mod path;
mod sampling;
mod settings;
mod uniform;

pub use easing::{EasingFunction, easing_derivative, lerp_eased, sample_curve};
pub use framing::{
    CompositionGuide, ShotType, SubjectFraming, distance_for_shot_type, fov_for_shot_type,
};
pub use keyframe::{CameraKeyframe, InterpolatedCamera, compute_keyframe_fingerprint};
pub use path::{
    CameraPath, LoopMode, PathBuilder, PathPlayback, compute_path_fingerprint,
    create_dolly_zoom_path, create_orbit_path,
};
pub use sampling::{
    BokehShape, DofResult, PhotoSampler, PreviewConfig, calculate_dof, compute_preview_fingerprint,
    halton, position_hash,
};
pub use settings::{PhotoFilter, PhotoSettings, TimeControl};
pub use uniform::{
    CameraPathUniform, CompositionGuideUniform, DofUniform, FilterUniform, PhotoBatch,
    PhotoInstanceUniform, PhotoSettingsUniform, convert,
};

use std::hash::{Hash, Hasher};

/// Compute a stable fingerprint for a complete photo mode configuration.
#[must_use]
pub fn compute_fingerprint(
    settings: &PhotoSettings,
    path: Option<&CameraPath>,
    preview: &PreviewConfig,
) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hash_f32(settings.exposure, &mut hasher);
    hash_f32(settings.aperture, &mut hasher);
    hash_f32(settings.focus_distance, &mut hasher);
    hash_f32(settings.fov, &mut hasher);
    hash_f32(settings.roll, &mut hasher);
    settings.time_frozen.hash(&mut hasher);
    settings.filter.hash(&mut hasher);
    if let Some(p) = path {
        compute_path_fingerprint(p).hash(&mut hasher);
    }
    hash_f32(preview.resolution_scale, &mut hasher);
    preview.dof_samples.hash(&mut hasher);
    preview.bokeh_shape.hash(&mut hasher);
    hasher.finish()
}

/// Compute a stable fingerprint for photo settings only.
#[must_use]
pub fn compute_settings_fingerprint(settings: &PhotoSettings) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hash_f32(settings.exposure, &mut hasher);
    hash_f32(settings.aperture, &mut hasher);
    hash_f32(settings.focus_distance, &mut hasher);
    hash_f32(settings.fov, &mut hasher);
    hash_f32(settings.roll, &mut hasher);
    settings.time_frozen.hash(&mut hasher);
    settings.ui_visible.hash(&mut hasher);
    settings.filter.hash(&mut hasher);
    if let Some(pos) = settings.position {
        hash_vec3(&pos, &mut hasher);
    }
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

/// Validate a photo mode configuration.
#[must_use]
pub fn validate_config(settings: &PhotoSettings, path: Option<&CameraPath>) -> ValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if !settings.is_valid() {
        errors.push("Settings contain invalid values".to_string());
    }

    if let Some(p) = path {
        if p.is_empty() {
            warnings.push("Camera path has no keyframes".to_string());
        } else if !p.is_valid() {
            errors.push("Camera path contains invalid keyframes".to_string());
        }

        if p.duration() <= 0.0 && p.len() > 1 {
            warnings.push("Camera path has zero duration".to_string());
        }
    }

    if settings.aperture < 2.0 && settings.focus_distance < 1.0 {
        warnings.push("Very wide aperture with close focus may cause extreme blur".to_string());
    }

    ValidationResult { errors, warnings }
}

/// Result of configuration validation.
#[derive(Clone, Debug, Default)]
pub struct ValidationResult {
    /// Critical errors that prevent use.
    pub errors: Vec<String>,
    /// Non-critical warnings.
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// Whether the configuration is valid (no errors).
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Whether there are any warnings.
    #[must_use]
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

/// Serialize a photo mode configuration to bytes (bincode).
///
/// # Errors
///
/// Returns an error if bincode serialization fails.
pub fn serialize_config(
    settings: &PhotoSettings,
    path: Option<&CameraPath>,
) -> Result<Vec<u8>, bincode::Error> {
    let data = (settings, path);
    bincode::serialize(&data)
}

/// Deserialize a photo mode configuration from bytes (bincode).
///
/// # Errors
///
/// Returns an error if bincode deserialization fails.
pub fn deserialize_config(
    bytes: &[u8],
) -> Result<(PhotoSettings, Option<CameraPath>), bincode::Error> {
    bincode::deserialize(bytes)
}

/// Filter keyframes by time range.
#[must_use]
pub fn filter_keyframes_in_range(
    keyframes: &[CameraKeyframe],
    start: f32,
    end: f32,
) -> Vec<&CameraKeyframe> {
    keyframes
        .iter()
        .filter(|kf| kf.time >= start && kf.time <= end)
        .collect()
}

/// Sort camera paths by duration.
pub fn sort_paths_by_duration(paths: &mut [CameraPath]) {
    paths.sort_by(|a, b| {
        a.duration()
            .partial_cmp(&b.duration())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// Sort camera paths by keyframe count.
pub fn sort_paths_by_keyframe_count(paths: &mut [CameraPath]) {
    paths.sort_by_key(CameraPath::len);
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Quat, Vec3};

    #[test]
    fn test_fingerprint_determinism() {
        let settings = PhotoSettings::default();
        let preview = PreviewConfig::default();

        let fp1 = compute_fingerprint(&settings, None, &preview);
        let fp2 = compute_fingerprint(&settings, None, &preview);

        assert_eq!(fp1, fp2, "fingerprint must be deterministic");
    }

    #[test]
    fn test_fingerprint_sensitivity() {
        let settings1 = PhotoSettings::default();
        let settings2 = PhotoSettings::default().with_exposure(1.0);
        let preview = PreviewConfig::default();

        let fp1 = compute_fingerprint(&settings1, None, &preview);
        let fp2 = compute_fingerprint(&settings2, None, &preview);

        assert_ne!(fp1, fp2, "different settings should differ");
    }

    #[test]
    fn test_fingerprint_with_path() {
        let settings = PhotoSettings::default();
        let preview = PreviewConfig::default();
        let path = PathBuilder::new("test")
            .at(0.0, Vec3::ZERO)
            .at(1.0, Vec3::X)
            .build();

        let fp_without = compute_fingerprint(&settings, None, &preview);
        let fp_with = compute_fingerprint(&settings, Some(&path), &preview);

        assert_ne!(fp_without, fp_with, "path should affect fingerprint");
    }

    #[test]
    fn test_settings_fingerprint() {
        let s1 = PhotoSettings::default();
        let s2 = PhotoSettings::default().with_filter(PhotoFilter::Cinematic);

        let fp1 = compute_settings_fingerprint(&s1);
        let fp2 = compute_settings_fingerprint(&s2);

        assert_ne!(fp1, fp2, "different filters should differ");
    }

    #[test]
    fn test_validation_valid_config() {
        let settings = PhotoSettings::default();
        let result = validate_config(&settings, None);

        assert!(result.is_valid());
    }

    #[test]
    fn test_validation_invalid_settings() {
        let settings = PhotoSettings {
            exposure: 100.0,
            ..Default::default()
        };
        let result = validate_config(&settings, None);

        assert!(!result.is_valid());
    }

    #[test]
    fn test_validation_empty_path_warning() {
        let settings = PhotoSettings::default();
        let path = CameraPath::new("empty");
        let result = validate_config(&settings, Some(&path));

        assert!(result.is_valid());
        assert!(result.has_warnings());
    }

    #[test]
    fn test_validation_extreme_dof_warning() {
        let settings = PhotoSettings::default()
            .with_aperture(1.4)
            .with_focus_distance(0.5);
        let result = validate_config(&settings, None);

        assert!(result.is_valid());
        assert!(result.has_warnings());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let settings = PhotoSettings::default()
            .with_exposure(1.5)
            .with_filter(PhotoFilter::Sepia);

        let bytes = serialize_config(&settings, None).expect("serialize");
        let (restored, path) = deserialize_config(&bytes).expect("deserialize");

        assert!(path.is_none());
        assert!((settings.exposure - restored.exposure).abs() < 0.001);
        assert_eq!(settings.filter, restored.filter);
    }

    #[test]
    fn test_serialization_with_path() {
        let settings = PhotoSettings::default();
        let path = PathBuilder::new("test_path")
            .at(0.0, Vec3::ZERO)
            .at(2.0, Vec3::new(10.0, 5.0, 0.0))
            .build()
            .with_loop_mode(LoopMode::Loop);

        let bytes = serialize_config(&settings, Some(&path)).expect("serialize");
        let (_, restored_path) = deserialize_config(&bytes).expect("deserialize");

        let restored = restored_path.expect("path should exist");
        assert_eq!(path.name, restored.name);
        assert_eq!(path.loop_mode, restored.loop_mode);
        assert_eq!(path.len(), restored.len());
    }

    #[test]
    fn test_serialization_preserves_fingerprint() {
        let settings = PhotoSettings::default().with_aperture(2.8);
        let preview = PreviewConfig::default();

        let fp_before = compute_fingerprint(&settings, None, &preview);

        let bytes = serialize_config(&settings, None).expect("serialize");
        let (restored, _) = deserialize_config(&bytes).expect("deserialize");

        let fp_after = compute_fingerprint(&restored, None, &preview);

        assert_eq!(
            fp_before, fp_after,
            "fingerprint should survive serialization"
        );
    }

    #[test]
    fn test_filter_keyframes_in_range() {
        let keyframes = vec![
            CameraKeyframe::new(0.0, Vec3::ZERO, Quat::IDENTITY),
            CameraKeyframe::new(1.0, Vec3::X, Quat::IDENTITY),
            CameraKeyframe::new(2.0, Vec3::Y, Quat::IDENTITY),
            CameraKeyframe::new(3.0, Vec3::Z, Quat::IDENTITY),
        ];

        let filtered = filter_keyframes_in_range(&keyframes, 0.5, 2.5);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_sort_paths_by_duration() {
        let mut paths = vec![
            PathBuilder::new("long")
                .at(0.0, Vec3::ZERO)
                .at(10.0, Vec3::X)
                .build(),
            PathBuilder::new("short")
                .at(0.0, Vec3::ZERO)
                .at(2.0, Vec3::X)
                .build(),
            PathBuilder::new("medium")
                .at(0.0, Vec3::ZERO)
                .at(5.0, Vec3::X)
                .build(),
        ];

        sort_paths_by_duration(&mut paths);

        assert_eq!(paths[0].name, "short");
        assert_eq!(paths[1].name, "medium");
        assert_eq!(paths[2].name, "long");
    }

    #[test]
    fn test_sort_paths_by_keyframe_count() {
        let mut paths = vec![
            PathBuilder::new("many")
                .at(0.0, Vec3::ZERO)
                .at(1.0, Vec3::X)
                .at(2.0, Vec3::Y)
                .at(3.0, Vec3::Z)
                .build(),
            PathBuilder::new("few")
                .at(0.0, Vec3::ZERO)
                .at(1.0, Vec3::X)
                .build(),
        ];

        sort_paths_by_keyframe_count(&mut paths);

        assert_eq!(paths[0].name, "few");
        assert_eq!(paths[1].name, "many");
    }

    #[test]
    fn test_module_reexports() {
        let _ = PhotoSettings::default();
        let _ = PhotoFilter::Cinematic;
        let _ = TimeControl::frozen();
        let _ = EasingFunction::SmoothStep;
        let _ = CameraKeyframe::default();
        let _ = InterpolatedCamera::default();
        let _ = CameraPath::new("test");
        let _ = LoopMode::PingPong;
        let _ = PathPlayback::new();
        let _ = PathBuilder::new("test");
        let _ = CompositionGuide::RuleOfThirds;
        let _ = ShotType::Medium;
        let _ = SubjectFraming::default();
        let _ = BokehShape::Hexagon;
        let _ = PhotoSampler::new(0);
        let _ = PreviewConfig::default();
        let _ = PhotoSettingsUniform::default();
        let _ = DofUniform::default();
        let _ = FilterUniform::default();
        let _ = PhotoBatch::new(10);
    }

    #[test]
    fn test_all_filters_serialize() {
        for filter in PhotoFilter::ALL {
            let settings = PhotoSettings::default().with_filter(filter);
            let bytes = serialize_config(&settings, None);
            assert!(bytes.is_ok(), "{filter:?} should serialize");
        }
    }

    #[test]
    fn test_all_easings_work() {
        for easing in EasingFunction::ALL {
            let val = easing.evaluate(0.5);
            assert!(val.is_finite(), "{easing:?} should produce finite value");
        }
    }

    #[test]
    fn test_all_guides_have_lines() {
        for guide in CompositionGuide::ALL {
            let _ = guide.guide_lines();
            let _ = guide.power_points();
            assert!(!guide.name().is_empty());
        }
    }

    #[test]
    fn test_orbit_and_dolly_paths() {
        let orbit = create_orbit_path("orbit", Vec3::ZERO, 10.0, 5.0, 4.0, 8);
        assert!(orbit.is_valid());
        assert_eq!(orbit.loop_mode, LoopMode::Loop);

        let dolly = create_dolly_zoom_path("dolly", Vec3::ZERO, 10.0, 5.0, 30.0, 60.0, 2.0);
        assert!(dolly.is_valid());
    }

    #[test]
    fn test_sampler_determinism() {
        use approx::assert_relative_eq;
        let s1 = PhotoSampler::new(42);
        let s2 = PhotoSampler::new(42);

        assert_relative_eq!(s1.sample(0.5, 0.5), s2.sample(0.5, 0.5));
    }

    #[test]
    fn test_dof_calculation() {
        let settings = PhotoSettings::default()
            .with_focus_distance(5.0)
            .with_aperture(2.8);

        let result = calculate_dof(5.0, &settings, 24.0, 1080.0);
        assert!(result.blur_amount < 0.2);
    }

    #[test]
    fn test_uniform_conversion() {
        let settings = PhotoSettings::default().with_filter(PhotoFilter::Cinematic);

        let (settings_u, dof_u, filter_u) = convert(&settings, BokehShape::Circle);

        assert_eq!(filter_u.filter_type, PhotoFilter::Cinematic as u32);
        assert!(dof_u.focus_distance > 0.0);
        assert!(settings_u.exposure > 0.0);
    }

    #[test]
    fn test_settings_clamping() {
        let settings = PhotoSettings::default()
            .with_exposure(100.0)
            .with_aperture(0.5)
            .with_fov(200.0)
            .with_roll(500.0)
            .with_focus_distance(-10.0);

        assert!((settings.exposure - 5.0).abs() < 0.001);
        assert!((settings.aperture - 1.4).abs() < 0.001);
        assert!((settings.fov - 120.0).abs() < 0.001);
        assert!((settings.roll - 180.0).abs() < 0.001);
        assert!(settings.focus_distance >= 0.1);
    }

    #[test]
    fn test_path_sampling_interpolation() {
        let path = PathBuilder::new("test")
            .at(0.0, Vec3::ZERO)
            .at(1.0, Vec3::new(10.0, 0.0, 0.0))
            .build();

        let at_start = path.sample(0.0).expect("sample");
        let at_mid = path.sample(0.5).expect("sample");
        let at_end = path.sample(1.0).expect("sample");

        assert!(at_start.position.x < at_mid.position.x);
        assert!(at_mid.position.x < at_end.position.x);
    }

    #[test]
    fn test_path_looping() {
        let path = PathBuilder::new("loop")
            .at(0.0, Vec3::ZERO)
            .at(1.0, Vec3::X)
            .build()
            .with_loop_mode(LoopMode::Loop);

        let at_1_5 = path.sample(1.5).expect("sample");
        let at_0_5 = path.sample(0.5).expect("sample");

        assert!((at_1_5.position.x - at_0_5.position.x).abs() < 0.1);
    }

    #[test]
    fn test_framing_guides() {
        let thirds = CompositionGuide::RuleOfThirds;
        let lines = thirds.guide_lines();
        let points = thirds.power_points();

        assert_eq!(lines.len(), 4);
        assert_eq!(points.len(), 4);
    }

    #[test]
    fn test_keyframe_fingerprint_determinism() {
        let kf = CameraKeyframe::new(1.0, Vec3::new(1.0, 2.0, 3.0), Quat::IDENTITY);

        let fp1 = compute_keyframe_fingerprint(&kf);
        let fp2 = compute_keyframe_fingerprint(&kf);

        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_bincode_roundtrip_all_types() {
        let settings = PhotoSettings::default().with_filter(PhotoFilter::Noir);
        let bytes = bincode::serialize(&settings).expect("serialize");
        let restored: PhotoSettings = bincode::deserialize(&bytes).expect("deserialize");
        assert_eq!(settings.filter, restored.filter);

        for easing in EasingFunction::ALL {
            let bytes = bincode::serialize(&easing).expect("serialize");
            let restored: EasingFunction = bincode::deserialize(&bytes).expect("deserialize");
            assert_eq!(easing, restored);
        }

        let path = PathBuilder::new("test")
            .at(0.0, Vec3::ZERO)
            .at(1.0, Vec3::X)
            .build();
        let bytes = bincode::serialize(&path).expect("serialize");
        let restored: CameraPath = bincode::deserialize(&bytes).expect("deserialize");
        assert_eq!(path.len(), restored.len());

        let preview = PreviewConfig::default();
        let bytes = bincode::serialize(&preview).expect("serialize");
        let restored: PreviewConfig = bincode::deserialize(&bytes).expect("deserialize");
        assert_eq!(preview.dof_samples, restored.dof_samples);
    }
}
