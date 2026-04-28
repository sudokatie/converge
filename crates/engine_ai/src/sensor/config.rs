//! Sensor configuration, attenuation, and occlusion models.

use super::SensorKind;
use serde::{Deserialize, Serialize};

/// Curve model for signal attenuation over distance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AttenuationCurve {
    /// No attenuation (constant strength within range).
    None,
    /// Linear falloff from 1.0 at source to 0.0 at max range.
    Linear,
    /// Power law: intensity / (1 + distance^exponent).
    InversePower { exponent: f32 },
    /// Exponential decay: e^(-decay_rate * distance).
    Exponential { decay_rate: f32 },
    /// Step function: full intensity until threshold, then zero.
    Step { threshold: f32 },
}

impl AttenuationCurve {
    /// Calculate attenuation factor (0.0 to 1.0) for a given distance and max range.
    #[must_use]
    pub fn factor(&self, distance: f32, max_range: f32) -> f32 {
        if distance <= 0.0 {
            return 1.0;
        }
        if distance >= max_range {
            return 0.0;
        }

        match self {
            Self::None => 1.0,
            Self::Linear => 1.0 - (distance / max_range),
            Self::InversePower { exponent } => 1.0 / (1.0 + distance.powf(*exponent)),
            Self::Exponential { decay_rate } => (-decay_rate * distance).exp(),
            Self::Step { threshold } => {
                if distance < *threshold {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }

    /// Create an inverse-square curve (physical sound/light falloff).
    #[must_use]
    pub fn inverse_square() -> Self {
        Self::InversePower { exponent: 2.0 }
    }

    /// Create a sensor-appropriate default curve.
    #[must_use]
    pub fn default_for_kind(kind: SensorKind) -> Self {
        match kind {
            SensorKind::Sight | SensorKind::Pressure => Self::Linear,
            SensorKind::Sound | SensorKind::Heat => Self::inverse_square(),
            SensorKind::Vibration => Self::InversePower { exponent: 2.5 },
            SensorKind::Smell => Self::Exponential { decay_rate: 0.05 },
            SensorKind::ElectricalField => Self::InversePower { exponent: 3.0 },
        }
    }
}

impl Default for AttenuationCurve {
    fn default() -> Self {
        Self::inverse_square()
    }
}

/// How obstacles affect signal propagation.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum OcclusionModel {
    /// No occlusion checking (omnidirectional, penetrating).
    None,
    /// Binary line-of-sight: blocked or not.
    #[default]
    LineOfSight,
    /// Partial occlusion with multiplier (0.0 = fully blocked, 1.0 = no effect).
    Partial { multiplier: f32 },
    /// Multiple occlusion levels based on material penetration.
    Layered { penetration_per_layer: f32 },
}

impl OcclusionModel {
    /// Calculate occlusion factor (0.0 = fully blocked, 1.0 = clear).
    #[must_use]
    pub fn factor(&self, is_blocked: bool, blocker_count: u32) -> f32 {
        match self {
            Self::None => 1.0,
            Self::LineOfSight => {
                if is_blocked {
                    0.0
                } else {
                    1.0
                }
            }
            Self::Partial { multiplier } => {
                if is_blocked {
                    *multiplier
                } else {
                    1.0
                }
            }
            Self::Layered {
                penetration_per_layer,
            } => {
                if blocker_count == 0 {
                    1.0
                } else {
                    #[expect(
                        clippy::cast_possible_wrap,
                        reason = "blocker_count in practice is small, well under i32::MAX"
                    )]
                    let exponent = blocker_count as i32;
                    penetration_per_layer.powi(exponent).max(0.0)
                }
            }
        }
    }

    /// Create a default model for a sensor kind.
    #[must_use]
    pub fn default_for_kind(kind: SensorKind) -> Self {
        match kind {
            SensorKind::Sight => Self::LineOfSight,
            SensorKind::Sound => Self::Layered {
                penetration_per_layer: 0.5,
            },
            SensorKind::Vibration => Self::Layered {
                penetration_per_layer: 0.7,
            },
            SensorKind::Smell | SensorKind::Pressure => Self::None,
            SensorKind::Heat => Self::Partial { multiplier: 0.3 },
            SensorKind::ElectricalField => Self::Partial { multiplier: 0.1 },
        }
    }
}

/// Threshold configuration for sensor activation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SensorThreshold {
    /// Minimum intensity to register at all.
    pub minimum: f32,
    /// Threshold for "weak" detection.
    pub weak: f32,
    /// Threshold for "strong" detection.
    pub strong: f32,
}

impl SensorThreshold {
    #[must_use]
    pub fn new(minimum: f32, weak: f32, strong: f32) -> Self {
        Self {
            minimum,
            weak,
            strong,
        }
    }

    /// Classify an intensity value.
    #[must_use]
    pub fn classify(&self, intensity: f32) -> DetectionStrength {
        if intensity < self.minimum {
            DetectionStrength::None
        } else if intensity < self.weak {
            DetectionStrength::Faint
        } else if intensity < self.strong {
            DetectionStrength::Weak
        } else {
            DetectionStrength::Strong
        }
    }
}

impl Default for SensorThreshold {
    fn default() -> Self {
        Self {
            minimum: 0.1,
            weak: 1.0,
            strong: 10.0,
        }
    }
}

/// Strength classification for detected stimuli.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DetectionStrength {
    None,
    Faint,
    Weak,
    Strong,
}

impl DetectionStrength {
    /// Get a weight for priority calculations.
    #[must_use]
    pub fn weight(self) -> f32 {
        match self {
            Self::None => 0.0,
            Self::Faint => 0.25,
            Self::Weak => 0.5,
            Self::Strong => 1.0,
        }
    }
}

/// Configuration for a single sensor.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SensorConfig {
    /// Maximum detection range.
    pub range: f32,
    /// Sensitivity multiplier (applied to incoming intensity).
    pub sensitivity: f32,
    /// Attenuation model.
    pub attenuation: AttenuationCurve,
    /// Occlusion model.
    pub occlusion: OcclusionModel,
    /// Detection thresholds.
    pub thresholds: SensorThreshold,
    /// Field of view in radians (None = omnidirectional).
    pub field_of_view: Option<f32>,
    /// Whether this sensor is currently enabled.
    pub enabled: bool,
}

impl SensorConfig {
    /// Create a basic config for a sensor kind.
    #[must_use]
    pub fn basic(kind: SensorKind) -> Self {
        Self {
            range: kind.default_range(),
            sensitivity: 1.0,
            attenuation: AttenuationCurve::default_for_kind(kind),
            occlusion: OcclusionModel::default_for_kind(kind),
            thresholds: SensorThreshold::default(),
            field_of_view: if kind.is_directional() {
                Some(std::f32::consts::PI)
            } else {
                None
            },
            enabled: true,
        }
    }

    /// Builder: set range.
    #[must_use]
    pub fn with_range(mut self, range: f32) -> Self {
        self.range = range;
        self
    }

    /// Builder: set sensitivity.
    #[must_use]
    pub fn with_sensitivity(mut self, sensitivity: f32) -> Self {
        self.sensitivity = sensitivity;
        self
    }

    /// Builder: set attenuation curve.
    #[must_use]
    pub fn with_attenuation(mut self, attenuation: AttenuationCurve) -> Self {
        self.attenuation = attenuation;
        self
    }

    /// Builder: set occlusion model.
    #[must_use]
    pub fn with_occlusion(mut self, occlusion: OcclusionModel) -> Self {
        self.occlusion = occlusion;
        self
    }

    /// Builder: set thresholds.
    #[must_use]
    pub fn with_thresholds(mut self, thresholds: SensorThreshold) -> Self {
        self.thresholds = thresholds;
        self
    }

    /// Builder: set field of view.
    #[must_use]
    pub fn with_fov(mut self, fov: Option<f32>) -> Self {
        self.field_of_view = fov;
        self
    }

    /// Builder: set enabled state.
    #[must_use]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Calculate effective intensity after attenuation and occlusion.
    #[must_use]
    pub fn effective_intensity(
        &self,
        base_intensity: f32,
        distance: f32,
        is_blocked: bool,
        blocker_count: u32,
    ) -> f32 {
        if !self.enabled || distance > self.range {
            return 0.0;
        }

        let attenuation = self.attenuation.factor(distance, self.range);
        let occlusion = self.occlusion.factor(is_blocked, blocker_count);

        base_intensity * self.sensitivity * attenuation * occlusion
    }

    /// Check if an intensity would be detected.
    #[must_use]
    pub fn would_detect(&self, effective_intensity: f32) -> bool {
        effective_intensity >= self.thresholds.minimum
    }

    /// Get detection strength for an intensity.
    #[must_use]
    pub fn detection_strength(&self, effective_intensity: f32) -> DetectionStrength {
        self.thresholds.classify(effective_intensity)
    }
}

impl Default for SensorConfig {
    fn default() -> Self {
        Self::basic(SensorKind::Sight)
    }
}

/// Complete specification for a sensor including kind and config.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SensorSpec {
    /// The sensor kind.
    pub kind: SensorKind,
    /// Configuration for this sensor.
    pub config: SensorConfig,
    /// Priority weight for this sensor's observations.
    pub priority_weight: f32,
}

impl SensorSpec {
    /// Create a new spec with default config.
    #[must_use]
    pub fn new(kind: SensorKind) -> Self {
        Self {
            kind,
            config: SensorConfig::basic(kind),
            priority_weight: 1.0,
        }
    }

    /// Builder: set config.
    #[must_use]
    pub fn with_config(mut self, config: SensorConfig) -> Self {
        self.config = config;
        self
    }

    /// Builder: set priority weight.
    #[must_use]
    pub fn with_priority_weight(mut self, weight: f32) -> Self {
        self.priority_weight = weight;
        self
    }

    /// Create a vision sensor with typical humanoid parameters.
    #[must_use]
    pub fn humanoid_vision() -> Self {
        Self::new(SensorKind::Sight)
            .with_config(
                SensorConfig::basic(SensorKind::Sight)
                    .with_range(60.0)
                    .with_fov(Some(std::f32::consts::FRAC_PI_2 * 1.5)),
            )
            .with_priority_weight(2.0)
    }

    /// Create a hearing sensor with typical humanoid parameters.
    #[must_use]
    pub fn humanoid_hearing() -> Self {
        Self::new(SensorKind::Sound)
            .with_config(SensorConfig::basic(SensorKind::Sound).with_range(40.0))
            .with_priority_weight(1.5)
    }

    /// Create a keen smell sensor.
    #[must_use]
    pub fn keen_smell() -> Self {
        Self::new(SensorKind::Smell)
            .with_config(
                SensorConfig::basic(SensorKind::Smell)
                    .with_range(100.0)
                    .with_sensitivity(2.0),
            )
            .with_priority_weight(1.2)
    }

    /// Create a seismic/vibration sensor.
    #[must_use]
    pub fn seismic() -> Self {
        Self::new(SensorKind::Vibration).with_config(
            SensorConfig::basic(SensorKind::Vibration)
                .with_range(30.0)
                .with_sensitivity(1.5),
        )
    }

    /// Create a thermal sensor.
    #[must_use]
    pub fn thermal() -> Self {
        Self::new(SensorKind::Heat).with_config(
            SensorConfig::basic(SensorKind::Heat)
                .with_range(15.0)
                .with_sensitivity(1.0),
        )
    }

    /// Create an electroreceptor sensor.
    #[must_use]
    pub fn electroreception() -> Self {
        Self::new(SensorKind::ElectricalField).with_config(
            SensorConfig::basic(SensorKind::ElectricalField)
                .with_range(5.0)
                .with_sensitivity(3.0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attenuation_none() {
        let curve = AttenuationCurve::None;
        assert!((curve.factor(0.0, 100.0) - 1.0).abs() < f32::EPSILON);
        assert!((curve.factor(50.0, 100.0) - 1.0).abs() < f32::EPSILON);
        assert!((curve.factor(100.0, 100.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_attenuation_linear() {
        let curve = AttenuationCurve::Linear;
        assert!((curve.factor(0.0, 100.0) - 1.0).abs() < f32::EPSILON);
        assert!((curve.factor(50.0, 100.0) - 0.5).abs() < f32::EPSILON);
        assert!((curve.factor(100.0, 100.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_attenuation_inverse_square() {
        let curve = AttenuationCurve::inverse_square();
        assert!((curve.factor(0.0, 100.0) - 1.0).abs() < f32::EPSILON);
        assert!(curve.factor(1.0, 100.0) < 1.0);
        assert!(curve.factor(10.0, 100.0) < curve.factor(1.0, 100.0));
    }

    #[test]
    fn test_attenuation_exponential() {
        let curve = AttenuationCurve::Exponential { decay_rate: 0.1 };
        assert!((curve.factor(0.0, 100.0) - 1.0).abs() < f32::EPSILON);
        assert!(curve.factor(10.0, 100.0) < 1.0);
    }

    #[test]
    fn test_attenuation_step() {
        let curve = AttenuationCurve::Step { threshold: 50.0 };
        assert!((curve.factor(0.0, 100.0) - 1.0).abs() < f32::EPSILON);
        assert!((curve.factor(49.0, 100.0) - 1.0).abs() < f32::EPSILON);
        assert!((curve.factor(50.0, 100.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_occlusion_none() {
        let model = OcclusionModel::None;
        assert!((model.factor(true, 5) - 1.0).abs() < f32::EPSILON);
        assert!((model.factor(false, 0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_occlusion_line_of_sight() {
        let model = OcclusionModel::LineOfSight;
        assert!((model.factor(false, 0) - 1.0).abs() < f32::EPSILON);
        assert!((model.factor(true, 1)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_occlusion_partial() {
        let model = OcclusionModel::Partial { multiplier: 0.3 };
        assert!((model.factor(false, 0) - 1.0).abs() < f32::EPSILON);
        assert!((model.factor(true, 1) - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_occlusion_layered() {
        let model = OcclusionModel::Layered {
            penetration_per_layer: 0.5,
        };
        assert!((model.factor(false, 0) - 1.0).abs() < f32::EPSILON);
        assert!((model.factor(true, 1) - 0.5).abs() < f32::EPSILON);
        assert!((model.factor(true, 2) - 0.25).abs() < f32::EPSILON);
        assert!((model.factor(true, 3) - 0.125).abs() < f32::EPSILON);
    }

    #[test]
    fn test_sensor_threshold() {
        let threshold = SensorThreshold::new(0.1, 1.0, 10.0);
        assert_eq!(threshold.classify(0.05), DetectionStrength::None);
        assert_eq!(threshold.classify(0.5), DetectionStrength::Faint);
        assert_eq!(threshold.classify(5.0), DetectionStrength::Weak);
        assert_eq!(threshold.classify(15.0), DetectionStrength::Strong);
    }

    #[test]
    fn test_detection_strength_weight() {
        assert!((DetectionStrength::None.weight()).abs() < f32::EPSILON);
        assert!((DetectionStrength::Faint.weight() - 0.25).abs() < f32::EPSILON);
        assert!((DetectionStrength::Strong.weight() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_sensor_config_basic() {
        let config = SensorConfig::basic(SensorKind::Sound);
        assert!((config.range - SensorKind::Sound.default_range()).abs() < f32::EPSILON);
        assert!(config.enabled);
    }

    #[test]
    fn test_sensor_config_effective_intensity() {
        let config = SensorConfig::basic(SensorKind::Sound)
            .with_range(100.0)
            .with_sensitivity(1.0)
            .with_attenuation(AttenuationCurve::Linear)
            .with_occlusion(OcclusionModel::None);

        let intensity = config.effective_intensity(100.0, 50.0, false, 0);
        assert!((intensity - 50.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_sensor_config_disabled() {
        let config = SensorConfig::basic(SensorKind::Sound).with_enabled(false);
        let intensity = config.effective_intensity(100.0, 10.0, false, 0);
        assert!((intensity).abs() < f32::EPSILON);
    }

    #[test]
    fn test_sensor_config_out_of_range() {
        let config = SensorConfig::basic(SensorKind::Sound).with_range(50.0);
        let intensity = config.effective_intensity(100.0, 60.0, false, 0);
        assert!((intensity).abs() < f32::EPSILON);
    }

    #[test]
    fn test_sensor_config_with_occlusion() {
        let config = SensorConfig::basic(SensorKind::Sound)
            .with_range(100.0)
            .with_attenuation(AttenuationCurve::None)
            .with_occlusion(OcclusionModel::Partial { multiplier: 0.5 });

        let blocked = config.effective_intensity(100.0, 10.0, true, 1);
        let clear = config.effective_intensity(100.0, 10.0, false, 0);

        assert!((blocked - 50.0).abs() < f32::EPSILON);
        assert!((clear - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_sensor_spec_new() {
        let spec = SensorSpec::new(SensorKind::Sight);
        assert_eq!(spec.kind, SensorKind::Sight);
        assert!(spec.config.enabled);
    }

    #[test]
    fn test_sensor_spec_humanoid_vision() {
        let spec = SensorSpec::humanoid_vision();
        assert_eq!(spec.kind, SensorKind::Sight);
        assert!((spec.config.range - 60.0).abs() < f32::EPSILON);
        assert!(spec.config.field_of_view.is_some());
    }

    #[test]
    fn test_sensor_spec_keen_smell() {
        let spec = SensorSpec::keen_smell();
        assert_eq!(spec.kind, SensorKind::Smell);
        assert!((spec.config.range - 100.0).abs() < f32::EPSILON);
        assert!((spec.config.sensitivity - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_sensor_config_serde() {
        let config = SensorConfig::basic(SensorKind::Heat)
            .with_range(25.0)
            .with_sensitivity(1.5);

        let json = serde_json::to_string(&config).unwrap();
        let restored: SensorConfig = serde_json::from_str(&json).unwrap();

        assert!((restored.range - 25.0).abs() < f32::EPSILON);
        assert!((restored.sensitivity - 1.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_sensor_spec_serde() {
        let spec = SensorSpec::humanoid_hearing();

        let json = serde_json::to_string(&spec).unwrap();
        let restored: SensorSpec = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.kind, SensorKind::Sound);
    }

    #[test]
    fn test_all_sensor_kinds_have_defaults() {
        for kind in SensorKind::ALL {
            let config = SensorConfig::basic(*kind);
            assert!(config.range > 0.0);

            let attenuation = AttenuationCurve::default_for_kind(*kind);
            assert!(attenuation.factor(0.0, 100.0) > 0.0);

            let occlusion = OcclusionModel::default_for_kind(*kind);
            assert!(occlusion.factor(false, 0) > 0.0);
        }
    }
}
