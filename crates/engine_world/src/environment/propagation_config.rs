//! Hazard propagation configuration and rules.

use serde::{Deserialize, Serialize};

use super::HazardKind;

/// Configuration for how a hazard spreads to neighbors.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpreadConfig {
    /// Base spread rate (cells per second at intensity 1.0).
    pub rate: f32,

    /// Intensity transferred to neighbor (fraction of source).
    pub transfer_fraction: f32,

    /// Minimum source intensity required to spread.
    pub min_intensity: f32,

    /// Whether spread is enabled.
    pub enabled: bool,

    /// Weight for face neighbors (directly adjacent).
    pub face_weight: f32,

    /// Weight for edge neighbors (share an edge).
    pub edge_weight: f32,

    /// Weight for corner neighbors (share a vertex only).
    pub corner_weight: f32,

    /// Downward spread multiplier for gravity-affected hazards.
    pub gravity_multiplier: f32,
}

impl SpreadConfig {
    /// No spread.
    pub const NONE: Self = Self {
        rate: 0.0,
        transfer_fraction: 0.0,
        min_intensity: 1.0,
        enabled: false,
        face_weight: 0.0,
        edge_weight: 0.0,
        corner_weight: 0.0,
        gravity_multiplier: 1.0,
    };

    /// Slow spread (corruption, frost).
    pub const SLOW: Self = Self {
        rate: 0.5,
        transfer_fraction: 0.4,
        min_intensity: 0.1,
        enabled: true,
        face_weight: 1.0,
        edge_weight: 0.3,
        corner_weight: 0.0,
        gravity_multiplier: 1.0,
    };

    /// Medium spread (infection).
    pub const MEDIUM: Self = Self {
        rate: 1.0,
        transfer_fraction: 0.5,
        min_intensity: 0.2,
        enabled: true,
        face_weight: 1.0,
        edge_weight: 0.5,
        corner_weight: 0.2,
        gravity_multiplier: 1.0,
    };

    /// Fast spread (fire, flood).
    pub const FAST: Self = Self {
        rate: 2.0,
        transfer_fraction: 0.6,
        min_intensity: 0.1,
        enabled: true,
        face_weight: 1.0,
        edge_weight: 0.7,
        corner_weight: 0.4,
        gravity_multiplier: 1.5,
    };

    /// Check if spread is active.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.enabled && self.rate > 0.0
    }

    /// Calculate spread interval (seconds between spread attempts).
    #[must_use]
    pub fn spread_interval(&self) -> f32 {
        if self.rate > 0.0 {
            1.0 / self.rate
        } else {
            f32::INFINITY
        }
    }
}

impl Default for SpreadConfig {
    fn default() -> Self {
        Self::MEDIUM
    }
}

/// Configuration for hazard intensity decay.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecayConfig {
    /// Base decay rate (intensity per second).
    pub rate: f32,

    /// Whether decay is enabled.
    pub enabled: bool,

    /// Minimum time before decay starts (seconds).
    pub grace_period: f32,

    /// Intensity threshold below which cell deactivates.
    pub extinction_threshold: f32,
}

impl DecayConfig {
    /// No decay.
    pub const NONE: Self = Self {
        rate: 0.0,
        enabled: false,
        grace_period: 0.0,
        extinction_threshold: 0.01,
    };

    /// Slow decay (corruption).
    pub const SLOW: Self = Self {
        rate: 0.05,
        enabled: true,
        grace_period: 5.0,
        extinction_threshold: 0.01,
    };

    /// Medium decay (frost, infection).
    pub const MEDIUM: Self = Self {
        rate: 0.15,
        enabled: true,
        grace_period: 2.0,
        extinction_threshold: 0.02,
    };

    /// Fast decay (fire without fuel).
    pub const FAST: Self = Self {
        rate: 0.4,
        enabled: true,
        grace_period: 0.5,
        extinction_threshold: 0.05,
    };

    /// Check if decay is active.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.enabled && self.rate > 0.0
    }
}

impl Default for DecayConfig {
    fn default() -> Self {
        Self::MEDIUM
    }
}

/// Resistance value for blocking or slowing hazard spread.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Resistance {
    /// Resistance factor (0.0 = no resistance, 1.0 = full block).
    factor: f32,
}

impl Resistance {
    /// No resistance (hazard spreads freely).
    pub const NONE: Self = Self { factor: 0.0 };

    /// Low resistance.
    pub const LOW: Self = Self { factor: 0.25 };

    /// Medium resistance.
    pub const MEDIUM: Self = Self { factor: 0.5 };

    /// High resistance.
    pub const HIGH: Self = Self { factor: 0.75 };

    /// Full block (hazard cannot pass).
    pub const FULL: Self = Self { factor: 1.0 };

    /// Create a resistance with a specific factor.
    #[must_use]
    pub fn new(factor: f32) -> Self {
        Self {
            factor: factor.clamp(0.0, 1.0),
        }
    }

    /// Get the resistance factor.
    #[must_use]
    pub const fn factor(&self) -> f32 {
        self.factor
    }

    /// Check if this fully blocks spread.
    #[must_use]
    pub fn blocks(&self) -> bool {
        self.factor >= 1.0
    }

    /// Apply resistance to an intensity value.
    #[must_use]
    pub fn apply(&self, intensity: f32) -> f32 {
        intensity * (1.0 - self.factor)
    }
}

impl Default for Resistance {
    fn default() -> Self {
        Self::NONE
    }
}

/// Complete propagation configuration for a hazard kind.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PropagationConfig {
    /// The hazard kind this config applies to.
    pub kind: HazardKind,

    /// Spread configuration.
    pub spread: SpreadConfig,

    /// Decay configuration.
    pub decay: DecayConfig,

    /// Maximum intensity cap.
    pub max_intensity: f32,

    /// Whether hazard persists at zero intensity (presence-based like vacuum).
    pub persist_at_zero: bool,
}

impl PropagationConfig {
    /// Create a new propagation config for a hazard kind.
    #[must_use]
    pub fn new(kind: HazardKind) -> Self {
        let (spread, decay, persist_at_zero) = match kind {
            HazardKind::Fire => (SpreadConfig::FAST, DecayConfig::FAST, false),
            HazardKind::Infection => (SpreadConfig::MEDIUM, DecayConfig::MEDIUM, false),
            HazardKind::Frost => (SpreadConfig::SLOW, DecayConfig::MEDIUM, false),
            HazardKind::Vacuum => (SpreadConfig::FAST, DecayConfig::NONE, true),
            HazardKind::Flood => (SpreadConfig::FAST, DecayConfig::SLOW, false),
            HazardKind::Corruption => (SpreadConfig::SLOW, DecayConfig::SLOW, false),
        };

        Self {
            kind,
            spread,
            decay,
            max_intensity: 1.0,
            persist_at_zero,
        }
    }

    /// Create configs for all hazard kinds with default settings.
    ///
    /// # Panics
    ///
    /// This function will not panic as it only iterates over valid hazard indices.
    #[must_use]
    pub fn all_defaults() -> [Self; HazardKind::COUNT] {
        std::array::from_fn(|i| {
            let kind = HazardKind::from_index(i).expect("valid index");
            Self::new(kind)
        })
    }

    /// Check if any propagation is active.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.spread.is_active() || self.decay.is_active()
    }
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    reason = "tests check exact constructor return values"
)]
mod tests {
    use super::*;

    #[test]
    fn spread_config_none() {
        let config = SpreadConfig::NONE;
        assert!(!config.is_active());
        assert_eq!(config.spread_interval(), f32::INFINITY);
    }

    #[test]
    fn spread_config_interval() {
        let config = SpreadConfig::FAST;
        assert!(config.is_active());
        assert!((config.spread_interval() - 0.5).abs() < 0.001);
    }

    #[test]
    fn decay_config_none() {
        let config = DecayConfig::NONE;
        assert!(!config.is_active());
    }

    #[test]
    fn decay_config_fast() {
        let config = DecayConfig::FAST;
        assert!(config.is_active());
        assert!(config.rate > DecayConfig::MEDIUM.rate);
    }

    #[test]
    fn resistance_clamps() {
        let low = Resistance::new(-0.5);
        assert_eq!(low.factor(), 0.0);

        let high = Resistance::new(1.5);
        assert_eq!(high.factor(), 1.0);
    }

    #[test]
    fn resistance_apply() {
        let half = Resistance::new(0.5);
        assert!((half.apply(1.0) - 0.5).abs() < 0.001);
        assert!((half.apply(0.8) - 0.4).abs() < 0.001);
    }

    #[test]
    fn resistance_blocks() {
        assert!(!Resistance::NONE.blocks());
        assert!(!Resistance::HIGH.blocks());
        assert!(Resistance::FULL.blocks());
    }

    #[test]
    fn propagation_config_defaults() {
        let configs = PropagationConfig::all_defaults();
        assert_eq!(configs.len(), HazardKind::COUNT);

        let fire = &configs[HazardKind::Fire.as_index()];
        assert!(fire.spread.is_active());
        assert!(fire.decay.is_active());

        let vacuum = &configs[HazardKind::Vacuum.as_index()];
        assert!(vacuum.persist_at_zero);
        assert!(!vacuum.decay.is_active());
    }

    #[test]
    fn propagation_config_is_active() {
        let config = PropagationConfig::new(HazardKind::Fire);
        assert!(config.is_active());

        let mut inactive = PropagationConfig::new(HazardKind::Fire);
        inactive.spread = SpreadConfig::NONE;
        inactive.decay = DecayConfig::NONE;
        assert!(!inactive.is_active());
    }

    #[test]
    fn serde_round_trip() {
        let config = PropagationConfig::new(HazardKind::Infection);
        let json = serde_json::to_string(&config).unwrap();
        let recovered: PropagationConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.kind, config.kind);
    }
}
