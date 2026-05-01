//! Dynamic music layering system driven by environment pressure and threat.
//!
//! This module provides state-only, data-oriented music layer management.
//! Actual audio playback is handled elsewhere; this module computes which
//! layers should be active, their target volumes, and transition plans.
//!
//! # Architecture
//!
//! - [`MusicLayerKind`]: Typed categories of music layers.
//! - [`EnvironmentPressure`]: Input signals from the game world.
//! - [`LayerConfig`]: Per-layer activation thresholds and behavior.
//! - [`LayerProfile`]: Collection of layer configs forming a complete profile.
//! - [`LayerMix`]: Computed output with active layers and transition plan.

use serde::{Deserialize, Serialize};

/// Minimum weight threshold for a layer to be considered active.
pub const LAYER_ACTIVE_THRESHOLD: f32 = 0.001;

/// Default fade-in duration in seconds.
pub const DEFAULT_FADE_IN_SECS: f32 = 2.0;

/// Default fade-out duration in seconds.
pub const DEFAULT_FADE_OUT_SECS: f32 = 3.0;

/// Default hysteresis duration in seconds to prevent rapid toggling.
pub const DEFAULT_HYSTERESIS_SECS: f32 = 1.5;

/// Maximum number of concurrent active layers.
pub const MAX_ACTIVE_LAYERS: usize = 8;

/// Kind of music layer, defining its semantic role in the audio mix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum MusicLayerKind {
    /// Base ambient layer, always present at some level.
    BaseAmbient = 0,
    /// Calm exploration, low threat.
    Exploration = 1,
    /// Building tension, moderate threat detected.
    Tension = 2,
    /// Immediate danger, high threat.
    Danger = 3,
    /// Active combat engagement.
    Combat = 4,
    /// Storm or severe weather pressure.
    Storm = 5,
    /// Low oxygen or suffocation pressure.
    LowOxygen = 6,
    /// Radiation hazard pressure.
    Radiation = 7,
    /// Toxicity or poison pressure.
    Toxicity = 8,
    /// Corruption or eldritch pressure.
    Corruption = 9,
    /// Spore density or biological hazard.
    Spores = 10,
    /// Structural instability or collapse danger.
    StructuralDanger = 11,
    /// Deep underground or isolation pressure.
    Depth = 12,
    /// Void or space pressure.
    Void = 13,
    /// Player health critical.
    HealthCritical = 14,
    /// Player stress or sanity pressure.
    Stress = 15,
}

impl MusicLayerKind {
    /// All layer kinds in priority order (lower = higher priority for stacking).
    pub const ALL: [MusicLayerKind; 16] = [
        MusicLayerKind::BaseAmbient,
        MusicLayerKind::Exploration,
        MusicLayerKind::Tension,
        MusicLayerKind::Danger,
        MusicLayerKind::Combat,
        MusicLayerKind::Storm,
        MusicLayerKind::LowOxygen,
        MusicLayerKind::Radiation,
        MusicLayerKind::Toxicity,
        MusicLayerKind::Corruption,
        MusicLayerKind::Spores,
        MusicLayerKind::StructuralDanger,
        MusicLayerKind::Depth,
        MusicLayerKind::Void,
        MusicLayerKind::HealthCritical,
        MusicLayerKind::Stress,
    ];

    /// Returns the default priority for this layer kind.
    /// Lower values = higher priority (evaluated first, takes precedence).
    #[must_use]
    pub const fn default_priority(&self) -> i32 {
        match self {
            MusicLayerKind::Combat => 0,
            MusicLayerKind::HealthCritical => 5,
            MusicLayerKind::Danger => 10,
            MusicLayerKind::StructuralDanger => 15,
            MusicLayerKind::Void => 20,
            MusicLayerKind::Radiation => 25,
            MusicLayerKind::Toxicity => 26,
            MusicLayerKind::Corruption => 27,
            MusicLayerKind::LowOxygen => 28,
            MusicLayerKind::Spores => 29,
            MusicLayerKind::Storm => 30,
            MusicLayerKind::Tension => 40,
            MusicLayerKind::Stress => 45,
            MusicLayerKind::Depth => 50,
            MusicLayerKind::Exploration => 80,
            MusicLayerKind::BaseAmbient => 100,
        }
    }

    /// Returns whether this is an environmental pressure layer.
    #[must_use]
    pub const fn is_environmental(&self) -> bool {
        matches!(
            self,
            MusicLayerKind::Storm
                | MusicLayerKind::LowOxygen
                | MusicLayerKind::Radiation
                | MusicLayerKind::Toxicity
                | MusicLayerKind::Corruption
                | MusicLayerKind::Spores
                | MusicLayerKind::Depth
                | MusicLayerKind::Void
        )
    }

    /// Returns whether this is a threat/combat layer.
    #[must_use]
    pub const fn is_threat(&self) -> bool {
        matches!(
            self,
            MusicLayerKind::Tension
                | MusicLayerKind::Danger
                | MusicLayerKind::Combat
                | MusicLayerKind::StructuralDanger
        )
    }

    /// Returns whether this is a player condition layer.
    #[must_use]
    pub const fn is_player_condition(&self) -> bool {
        matches!(
            self,
            MusicLayerKind::HealthCritical | MusicLayerKind::Stress
        )
    }
}

/// Environmental pressure and threat signals from the game world.
///
/// All values are normalized to 0.0-1.0 range where applicable,
/// representing intensity or severity.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EnvironmentPressure {
    /// Weather severity (0.0 = clear, 1.0 = extreme storm).
    pub weather_severity: f32,
    /// Oxygen level (1.0 = normal, 0.0 = none).
    pub oxygen_level: f32,
    /// Radiation intensity (0.0 = none, 1.0 = lethal).
    pub radiation_intensity: f32,
    /// Toxicity level (0.0 = none, 1.0 = lethal).
    pub toxicity_level: f32,
    /// Corruption intensity (0.0 = none, 1.0 = overwhelming).
    pub corruption_intensity: f32,
    /// Spore density (0.0 = none, 1.0 = saturated).
    pub spore_density: f32,
    /// Structural danger (0.0 = stable, 1.0 = imminent collapse).
    pub structural_danger: f32,
    /// Depth/isolation factor (0.0 = surface, 1.0 = deep underground/void).
    pub depth_factor: f32,
    /// Whether in void/space environment.
    pub in_void: bool,
    /// Hostile threat level (0.0 = none, 1.0 = overwhelming).
    pub hostile_threat: f32,
    /// Whether actively in combat.
    pub in_combat: bool,
    /// Player health ratio (1.0 = full, 0.0 = dead).
    pub player_health: f32,
    /// Player stress/sanity (1.0 = calm, 0.0 = maximum stress).
    pub player_stress: f32,
}

impl Default for EnvironmentPressure {
    fn default() -> Self {
        Self::CALM
    }
}

impl EnvironmentPressure {
    /// Calm, safe environment with no pressures.
    pub const CALM: EnvironmentPressure = EnvironmentPressure {
        weather_severity: 0.0,
        oxygen_level: 1.0,
        radiation_intensity: 0.0,
        toxicity_level: 0.0,
        corruption_intensity: 0.0,
        spore_density: 0.0,
        structural_danger: 0.0,
        depth_factor: 0.0,
        in_void: false,
        hostile_threat: 0.0,
        in_combat: false,
        player_health: 1.0,
        player_stress: 1.0,
    };

    /// Creates a new environment pressure with all default (safe) values.
    #[must_use]
    pub const fn new() -> Self {
        Self::CALM
    }

    /// Builder: set weather severity.
    #[must_use]
    pub const fn with_weather(mut self, severity: f32) -> Self {
        self.weather_severity = severity;
        self
    }

    /// Builder: set oxygen level.
    #[must_use]
    pub const fn with_oxygen(mut self, level: f32) -> Self {
        self.oxygen_level = level;
        self
    }

    /// Builder: set radiation intensity.
    #[must_use]
    pub const fn with_radiation(mut self, intensity: f32) -> Self {
        self.radiation_intensity = intensity;
        self
    }

    /// Builder: set toxicity level.
    #[must_use]
    pub const fn with_toxicity(mut self, level: f32) -> Self {
        self.toxicity_level = level;
        self
    }

    /// Builder: set corruption intensity.
    #[must_use]
    pub const fn with_corruption(mut self, intensity: f32) -> Self {
        self.corruption_intensity = intensity;
        self
    }

    /// Builder: set spore density.
    #[must_use]
    pub const fn with_spores(mut self, density: f32) -> Self {
        self.spore_density = density;
        self
    }

    /// Builder: set structural danger.
    #[must_use]
    pub const fn with_structural_danger(mut self, danger: f32) -> Self {
        self.structural_danger = danger;
        self
    }

    /// Builder: set depth factor.
    #[must_use]
    pub const fn with_depth(mut self, factor: f32) -> Self {
        self.depth_factor = factor;
        self
    }

    /// Builder: set void environment.
    #[must_use]
    pub const fn with_void(mut self, in_void: bool) -> Self {
        self.in_void = in_void;
        self
    }

    /// Builder: set hostile threat level.
    #[must_use]
    pub const fn with_hostile_threat(mut self, threat: f32) -> Self {
        self.hostile_threat = threat;
        self
    }

    /// Builder: set combat state.
    #[must_use]
    pub const fn with_combat(mut self, in_combat: bool) -> Self {
        self.in_combat = in_combat;
        self
    }

    /// Builder: set player health.
    #[must_use]
    pub const fn with_player_health(mut self, health: f32) -> Self {
        self.player_health = health;
        self
    }

    /// Builder: set player stress level.
    #[must_use]
    pub const fn with_player_stress(mut self, stress: f32) -> Self {
        self.player_stress = stress;
        self
    }

    /// Returns a clamped copy with all values in valid ranges.
    #[must_use]
    pub fn clamped(&self) -> Self {
        Self {
            weather_severity: self.weather_severity.clamp(0.0, 1.0),
            oxygen_level: self.oxygen_level.clamp(0.0, 1.0),
            radiation_intensity: self.radiation_intensity.clamp(0.0, 1.0),
            toxicity_level: self.toxicity_level.clamp(0.0, 1.0),
            corruption_intensity: self.corruption_intensity.clamp(0.0, 1.0),
            spore_density: self.spore_density.clamp(0.0, 1.0),
            structural_danger: self.structural_danger.clamp(0.0, 1.0),
            depth_factor: self.depth_factor.clamp(0.0, 1.0),
            in_void: self.in_void,
            hostile_threat: self.hostile_threat.clamp(0.0, 1.0),
            in_combat: self.in_combat,
            player_health: self.player_health.clamp(0.0, 1.0),
            player_stress: self.player_stress.clamp(0.0, 1.0),
        }
    }

    /// Returns the overall threat level (0.0-1.0).
    #[must_use]
    pub fn threat_level(&self) -> f32 {
        if self.in_combat {
            1.0
        } else {
            self.hostile_threat
        }
    }

    /// Returns the overall environmental pressure (0.0-1.0).
    #[must_use]
    pub fn environmental_pressure(&self) -> f32 {
        let pressures = [
            self.weather_severity,
            1.0 - self.oxygen_level,
            self.radiation_intensity,
            self.toxicity_level,
            self.corruption_intensity,
            self.spore_density,
            self.depth_factor,
            if self.in_void { 1.0 } else { 0.0 },
        ];
        pressures.iter().copied().fold(0.0_f32, f32::max)
    }
}

/// Configuration for a single music layer.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LayerConfig {
    /// The kind of layer this configures.
    pub kind: MusicLayerKind,
    /// Activation threshold (signal must exceed this to activate).
    pub activation_threshold: f32,
    /// Deactivation threshold (signal must drop below this to deactivate).
    pub deactivation_threshold: f32,
    /// Maximum volume when fully active (0.0-1.0).
    pub max_volume: f32,
    /// Priority for stacking (lower = higher priority).
    pub priority: i32,
    /// Fade-in duration in seconds.
    pub fade_in_secs: f32,
    /// Fade-out duration in seconds.
    pub fade_out_secs: f32,
    /// Whether this layer is enabled.
    pub enabled: bool,
}

impl LayerConfig {
    /// Creates a new layer config with default values.
    #[must_use]
    pub const fn new(kind: MusicLayerKind) -> Self {
        Self {
            kind,
            activation_threshold: 0.3,
            deactivation_threshold: 0.2,
            max_volume: 1.0,
            priority: kind.default_priority(),
            fade_in_secs: DEFAULT_FADE_IN_SECS,
            fade_out_secs: DEFAULT_FADE_OUT_SECS,
            enabled: true,
        }
    }

    /// Builder: set activation threshold.
    #[must_use]
    pub const fn with_activation_threshold(mut self, threshold: f32) -> Self {
        self.activation_threshold = threshold;
        self
    }

    /// Builder: set deactivation threshold.
    #[must_use]
    pub const fn with_deactivation_threshold(mut self, threshold: f32) -> Self {
        self.deactivation_threshold = threshold;
        self
    }

    /// Builder: set both thresholds with automatic hysteresis.
    #[must_use]
    pub const fn with_threshold(mut self, threshold: f32) -> Self {
        self.activation_threshold = threshold;
        self.deactivation_threshold = threshold * 0.7;
        self
    }

    /// Builder: set maximum volume.
    #[must_use]
    pub const fn with_max_volume(mut self, volume: f32) -> Self {
        self.max_volume = volume;
        self
    }

    /// Builder: set priority.
    #[must_use]
    pub const fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Builder: set fade-in duration.
    #[must_use]
    pub const fn with_fade_in(mut self, secs: f32) -> Self {
        self.fade_in_secs = secs;
        self
    }

    /// Builder: set fade-out duration.
    #[must_use]
    pub const fn with_fade_out(mut self, secs: f32) -> Self {
        self.fade_out_secs = secs;
        self
    }

    /// Builder: set enabled state.
    #[must_use]
    pub const fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Returns a clamped copy with all values in valid ranges.
    #[must_use]
    pub fn clamped(&self) -> Self {
        Self {
            kind: self.kind,
            activation_threshold: self.activation_threshold.clamp(0.0, 1.0),
            deactivation_threshold: self
                .deactivation_threshold
                .clamp(0.0, self.activation_threshold),
            max_volume: self.max_volume.clamp(0.0, 1.0),
            priority: self.priority,
            fade_in_secs: self.fade_in_secs.max(0.0),
            fade_out_secs: self.fade_out_secs.max(0.0),
            enabled: self.enabled,
        }
    }

    /// Returns whether the config is valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.activation_threshold >= 0.0
            && self.activation_threshold <= 1.0
            && self.deactivation_threshold >= 0.0
            && self.deactivation_threshold <= self.activation_threshold
            && self.max_volume >= 0.0
            && self.max_volume <= 1.0
            && self.fade_in_secs >= 0.0
            && self.fade_out_secs >= 0.0
    }
}

/// State of a single active layer.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LayerState {
    /// The layer kind.
    pub kind: MusicLayerKind,
    /// Current weight (0.0-1.0, pre-volume).
    pub weight: f32,
    /// Target weight to transition to.
    pub target_weight: f32,
    /// Current volume (weight * `max_volume`).
    pub volume: f32,
    /// Whether currently fading in (vs out or stable).
    pub fading_in: bool,
    /// Remaining fade time in seconds.
    pub fade_remaining: f32,
    /// Time since activation in seconds.
    pub time_active: f32,
}

impl LayerState {
    /// Creates a new layer state.
    #[must_use]
    pub const fn new(kind: MusicLayerKind) -> Self {
        Self {
            kind,
            weight: 0.0,
            target_weight: 0.0,
            volume: 0.0,
            fading_in: false,
            fade_remaining: 0.0,
            time_active: 0.0,
        }
    }

    /// Returns whether this layer is considered active.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.weight > LAYER_ACTIVE_THRESHOLD || self.target_weight > LAYER_ACTIVE_THRESHOLD
    }

    /// Returns whether this layer is currently transitioning.
    #[must_use]
    pub fn is_transitioning(&self) -> bool {
        self.fade_remaining > 0.0
            && (self.weight - self.target_weight).abs() > LAYER_ACTIVE_THRESHOLD
    }
}

/// A layer in the computed mix output.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MixLayer {
    /// The layer kind.
    pub kind: MusicLayerKind,
    /// Target volume (0.0-1.0).
    pub target_volume: f32,
    /// Current volume (0.0-1.0).
    pub current_volume: f32,
    /// Fade duration to reach target.
    pub fade_duration: f32,
    /// Priority for this layer.
    pub priority: i32,
}

/// Computed layer mix with transition plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerMix {
    /// Active layers sorted by priority (lowest first = highest priority).
    pub layers: Vec<MixLayer>,
    /// Total combined volume of all layers.
    pub total_volume: f32,
    /// Dominant layer (highest weight among active).
    pub dominant_layer: Option<MusicLayerKind>,
    /// Whether any layers are transitioning.
    pub is_transitioning: bool,
    /// Fingerprint for sync/replay/debugging.
    pub fingerprint: u64,
}

impl LayerMix {
    /// Creates an empty mix.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            layers: Vec::new(),
            total_volume: 0.0,
            dominant_layer: None,
            is_transitioning: false,
            fingerprint: 0,
        }
    }

    /// Returns the number of active layers.
    #[must_use]
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// Returns whether the mix is silent.
    #[must_use]
    pub fn is_silent(&self) -> bool {
        self.total_volume < LAYER_ACTIVE_THRESHOLD
    }

    /// Returns the layer with highest volume.
    #[must_use]
    pub fn loudest_layer(&self) -> Option<&MixLayer> {
        self.layers
            .iter()
            .max_by(|a, b| a.target_volume.total_cmp(&b.target_volume))
    }
}

/// Profile of layer configurations for a specific context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerProfile {
    /// Name of this profile.
    pub name: String,
    /// Layer configurations.
    pub layers: Vec<LayerConfig>,
    /// Global volume multiplier.
    pub master_volume: f32,
    /// Hysteresis duration to prevent rapid toggling.
    pub hysteresis_secs: f32,
}

impl LayerProfile {
    /// Creates a new profile with default layers.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            layers: Vec::new(),
            master_volume: 1.0,
            hysteresis_secs: DEFAULT_HYSTERESIS_SECS,
        }
    }

    /// Builder: add a layer config.
    #[must_use]
    pub fn with_layer(mut self, config: LayerConfig) -> Self {
        self.layers.push(config);
        self
    }

    /// Builder: set master volume.
    #[must_use]
    pub fn with_master_volume(mut self, volume: f32) -> Self {
        self.master_volume = volume;
        self
    }

    /// Builder: set hysteresis duration.
    #[must_use]
    pub fn with_hysteresis(mut self, secs: f32) -> Self {
        self.hysteresis_secs = secs;
        self
    }

    /// Returns the config for a specific layer kind, if present.
    #[must_use]
    pub fn get_layer(&self, kind: MusicLayerKind) -> Option<&LayerConfig> {
        self.layers.iter().find(|l| l.kind == kind)
    }

    /// Calm overworld exploration profile.
    #[must_use]
    pub fn calm_overworld() -> Self {
        Self::new("calm_overworld")
            .with_layer(
                LayerConfig::new(MusicLayerKind::BaseAmbient)
                    .with_threshold(0.0)
                    .with_max_volume(0.6),
            )
            .with_layer(
                LayerConfig::new(MusicLayerKind::Exploration)
                    .with_threshold(0.0)
                    .with_max_volume(0.8),
            )
            .with_layer(
                LayerConfig::new(MusicLayerKind::Tension)
                    .with_threshold(0.3)
                    .with_max_volume(0.7),
            )
            .with_layer(
                LayerConfig::new(MusicLayerKind::Danger)
                    .with_threshold(0.6)
                    .with_max_volume(0.9),
            )
            .with_layer(
                LayerConfig::new(MusicLayerKind::Combat)
                    .with_threshold(0.8)
                    .with_max_volume(1.0)
                    .with_fade_in(0.5),
            )
            .with_layer(
                LayerConfig::new(MusicLayerKind::Storm)
                    .with_threshold(0.4)
                    .with_max_volume(0.7),
            )
            .with_layer(
                LayerConfig::new(MusicLayerKind::HealthCritical)
                    .with_activation_threshold(0.75)
                    .with_deactivation_threshold(0.6)
                    .with_max_volume(0.8),
            )
            .with_hysteresis(2.0)
    }

    /// Underground exploration profile with enhanced pressure sensitivity.
    #[must_use]
    pub fn underground_exploration() -> Self {
        Self::new("underground_exploration")
            .with_layer(
                LayerConfig::new(MusicLayerKind::BaseAmbient)
                    .with_threshold(0.0)
                    .with_max_volume(0.4),
            )
            .with_layer(
                LayerConfig::new(MusicLayerKind::Depth)
                    .with_threshold(0.2)
                    .with_max_volume(0.8),
            )
            .with_layer(
                LayerConfig::new(MusicLayerKind::Tension)
                    .with_threshold(0.2)
                    .with_max_volume(0.8),
            )
            .with_layer(
                LayerConfig::new(MusicLayerKind::Danger)
                    .with_threshold(0.5)
                    .with_max_volume(0.9),
            )
            .with_layer(
                LayerConfig::new(MusicLayerKind::Combat)
                    .with_threshold(0.7)
                    .with_max_volume(1.0)
                    .with_fade_in(0.3),
            )
            .with_layer(
                LayerConfig::new(MusicLayerKind::LowOxygen)
                    .with_threshold(0.3)
                    .with_max_volume(0.9),
            )
            .with_layer(
                LayerConfig::new(MusicLayerKind::StructuralDanger)
                    .with_threshold(0.4)
                    .with_max_volume(0.85),
            )
            .with_layer(
                LayerConfig::new(MusicLayerKind::Corruption)
                    .with_threshold(0.25)
                    .with_max_volume(0.85),
            )
            .with_hysteresis(1.5)
    }

    /// Storm survival profile with weather emphasis.
    #[must_use]
    pub fn storm_survival() -> Self {
        Self::new("storm_survival")
            .with_layer(
                LayerConfig::new(MusicLayerKind::BaseAmbient)
                    .with_threshold(0.0)
                    .with_max_volume(0.3),
            )
            .with_layer(
                LayerConfig::new(MusicLayerKind::Storm)
                    .with_threshold(0.1)
                    .with_max_volume(1.0)
                    .with_fade_in(1.0),
            )
            .with_layer(
                LayerConfig::new(MusicLayerKind::Tension)
                    .with_threshold(0.4)
                    .with_max_volume(0.6),
            )
            .with_layer(
                LayerConfig::new(MusicLayerKind::Danger)
                    .with_threshold(0.7)
                    .with_max_volume(0.8),
            )
            .with_layer(
                LayerConfig::new(MusicLayerKind::Combat)
                    .with_threshold(0.8)
                    .with_max_volume(0.9),
            )
            .with_layer(
                LayerConfig::new(MusicLayerKind::HealthCritical)
                    .with_activation_threshold(0.7)
                    .with_deactivation_threshold(0.5)
                    .with_max_volume(0.85),
            )
            .with_hysteresis(1.0)
    }

    /// Combat danger profile with rapid transitions.
    #[must_use]
    pub fn combat_danger() -> Self {
        Self::new("combat_danger")
            .with_layer(
                LayerConfig::new(MusicLayerKind::BaseAmbient)
                    .with_threshold(0.0)
                    .with_max_volume(0.2),
            )
            .with_layer(
                LayerConfig::new(MusicLayerKind::Tension)
                    .with_threshold(0.1)
                    .with_max_volume(0.7)
                    .with_fade_in(0.5),
            )
            .with_layer(
                LayerConfig::new(MusicLayerKind::Danger)
                    .with_threshold(0.3)
                    .with_max_volume(0.85)
                    .with_fade_in(0.3),
            )
            .with_layer(
                LayerConfig::new(MusicLayerKind::Combat)
                    .with_threshold(0.5)
                    .with_max_volume(1.0)
                    .with_fade_in(0.2)
                    .with_fade_out(1.5),
            )
            .with_layer(
                LayerConfig::new(MusicLayerKind::HealthCritical)
                    .with_activation_threshold(0.6)
                    .with_deactivation_threshold(0.4)
                    .with_max_volume(0.9)
                    .with_fade_in(0.2),
            )
            .with_hysteresis(0.5)
    }

    /// Void/space pressure profile with isolation emphasis.
    #[must_use]
    pub fn void_pressure() -> Self {
        Self::new("void_pressure")
            .with_layer(
                LayerConfig::new(MusicLayerKind::BaseAmbient)
                    .with_threshold(0.0)
                    .with_max_volume(0.2),
            )
            .with_layer(
                LayerConfig::new(MusicLayerKind::Void)
                    .with_threshold(0.0)
                    .with_max_volume(0.9),
            )
            .with_layer(
                LayerConfig::new(MusicLayerKind::LowOxygen)
                    .with_threshold(0.2)
                    .with_max_volume(0.95),
            )
            .with_layer(
                LayerConfig::new(MusicLayerKind::Radiation)
                    .with_threshold(0.3)
                    .with_max_volume(0.85),
            )
            .with_layer(
                LayerConfig::new(MusicLayerKind::Tension)
                    .with_threshold(0.3)
                    .with_max_volume(0.7),
            )
            .with_layer(
                LayerConfig::new(MusicLayerKind::Danger)
                    .with_threshold(0.6)
                    .with_max_volume(0.9),
            )
            .with_layer(
                LayerConfig::new(MusicLayerKind::HealthCritical)
                    .with_activation_threshold(0.5)
                    .with_deactivation_threshold(0.3)
                    .with_max_volume(1.0)
                    .with_fade_in(0.3),
            )
            .with_layer(
                LayerConfig::new(MusicLayerKind::Stress)
                    .with_threshold(0.4)
                    .with_max_volume(0.8),
            )
            .with_hysteresis(1.0)
    }
}

/// Controller for dynamic music layer evaluation and state management.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicMusicController {
    profile: LayerProfile,
    layer_states: Vec<LayerState>,
    hysteresis_timers: Vec<f32>,
    last_pressure: EnvironmentPressure,
    time_since_change: f32,
}

impl DynamicMusicController {
    /// Creates a new controller with the given profile.
    #[must_use]
    pub fn new(profile: LayerProfile) -> Self {
        let layer_count = profile.layers.len();
        Self {
            profile,
            layer_states: Vec::new(),
            hysteresis_timers: vec![0.0; layer_count],
            last_pressure: EnvironmentPressure::CALM,
            time_since_change: 0.0,
        }
    }

    /// Returns the current profile.
    #[must_use]
    pub fn profile(&self) -> &LayerProfile {
        &self.profile
    }

    /// Sets a new profile, resetting layer states.
    pub fn set_profile(&mut self, profile: LayerProfile) {
        let layer_count = profile.layers.len();
        self.profile = profile;
        self.layer_states.clear();
        self.hysteresis_timers = vec![0.0; layer_count];
    }

    /// Returns the signal value for a layer kind given current pressure.
    #[expect(
        clippy::unused_self,
        reason = "method for future extensibility with per-controller config"
    )]
    fn get_signal(&self, kind: MusicLayerKind, pressure: &EnvironmentPressure) -> f32 {
        match kind {
            MusicLayerKind::BaseAmbient => 1.0,
            MusicLayerKind::Exploration => {
                if pressure.hostile_threat < 0.1 && !pressure.in_combat {
                    1.0 - pressure.environmental_pressure()
                } else {
                    0.0
                }
            }
            MusicLayerKind::Tension => {
                if pressure.in_combat {
                    0.0
                } else {
                    pressure
                        .hostile_threat
                        .max(pressure.environmental_pressure() * 0.5)
                }
            }
            MusicLayerKind::Danger => {
                if pressure.in_combat {
                    0.0
                } else {
                    (pressure.hostile_threat - 0.3).max(0.0) * 1.5
                }
            }
            MusicLayerKind::Combat => {
                if pressure.in_combat {
                    1.0
                } else {
                    0.0
                }
            }
            MusicLayerKind::Storm => pressure.weather_severity,
            MusicLayerKind::LowOxygen => 1.0 - pressure.oxygen_level,
            MusicLayerKind::Radiation => pressure.radiation_intensity,
            MusicLayerKind::Toxicity => pressure.toxicity_level,
            MusicLayerKind::Corruption => pressure.corruption_intensity,
            MusicLayerKind::Spores => pressure.spore_density,
            MusicLayerKind::StructuralDanger => pressure.structural_danger,
            MusicLayerKind::Depth => pressure.depth_factor,
            MusicLayerKind::Void => {
                if pressure.in_void {
                    1.0
                } else {
                    0.0
                }
            }
            MusicLayerKind::HealthCritical => 1.0 - pressure.player_health,
            MusicLayerKind::Stress => 1.0 - pressure.player_stress,
        }
    }

    /// Evaluates the current pressure and returns the target layer mix.
    ///
    /// This is a pure computation that does not modify internal state.
    /// Use [`tick`] to advance state over time.
    #[must_use]
    pub fn evaluate(&self, pressure: &EnvironmentPressure) -> LayerMix {
        let pressure = pressure.clamped();
        let mut layers = Vec::new();

        for config in &self.profile.layers {
            if !config.enabled {
                continue;
            }

            let signal = self.get_signal(config.kind, &pressure);
            let current_state = self.layer_states.iter().find(|s| s.kind == config.kind);

            let (current_volume, target_volume, fade_duration) = if let Some(state) = current_state
            {
                let should_activate = signal >= config.activation_threshold;
                let should_deactivate = signal < config.deactivation_threshold;

                let target = if should_activate {
                    (signal * config.max_volume).min(config.max_volume)
                } else if should_deactivate {
                    0.0
                } else {
                    state.target_weight * config.max_volume
                };

                let fade = if target > state.volume {
                    config.fade_in_secs
                } else {
                    config.fade_out_secs
                };

                (state.volume, target, fade)
            } else {
                let target = if signal >= config.activation_threshold {
                    (signal * config.max_volume).min(config.max_volume)
                } else {
                    0.0
                };
                (0.0, target, config.fade_in_secs)
            };

            if target_volume > LAYER_ACTIVE_THRESHOLD || current_volume > LAYER_ACTIVE_THRESHOLD {
                layers.push(MixLayer {
                    kind: config.kind,
                    target_volume: target_volume * self.profile.master_volume,
                    current_volume: current_volume * self.profile.master_volume,
                    fade_duration,
                    priority: config.priority,
                });
            }
        }

        layers.sort_by_key(|l| l.priority);

        if layers.len() > MAX_ACTIVE_LAYERS {
            layers.truncate(MAX_ACTIVE_LAYERS);
        }

        let total_volume = layers.iter().map(|l| l.target_volume).sum();
        let dominant_layer = layers
            .iter()
            .max_by(|a, b| a.target_volume.total_cmp(&b.target_volume))
            .map(|l| l.kind);
        let is_transitioning = layers
            .iter()
            .any(|l| (l.target_volume - l.current_volume).abs() > LAYER_ACTIVE_THRESHOLD);

        let fingerprint = compute_mix_fingerprint(&layers, &pressure);

        LayerMix {
            layers,
            total_volume,
            dominant_layer,
            is_transitioning,
            fingerprint,
        }
    }

    /// Advances the controller state by the given delta time.
    ///
    /// Returns the new layer mix after state update.
    pub fn tick(&mut self, dt: f32, pressure: &EnvironmentPressure) -> LayerMix {
        let pressure = pressure.clamped();
        self.time_since_change += dt;

        for timer in &mut self.hysteresis_timers {
            *timer = (*timer - dt).max(0.0);
        }

        for (i, config) in self.profile.layers.iter().enumerate() {
            if !config.enabled {
                continue;
            }

            let signal = self.get_signal(config.kind, &pressure);
            let state = self.layer_states.iter_mut().find(|s| s.kind == config.kind);

            if let Some(state) = state {
                state.time_active += dt;

                let hysteresis_ok = self.hysteresis_timers.get(i).copied().unwrap_or(0.0) <= 0.0;
                let should_activate = signal >= config.activation_threshold && hysteresis_ok;
                let should_deactivate = signal < config.deactivation_threshold && hysteresis_ok;

                if should_activate && state.target_weight < LAYER_ACTIVE_THRESHOLD {
                    state.target_weight = signal.min(1.0);
                    state.fading_in = true;
                    state.fade_remaining = config.fade_in_secs;
                    if let Some(timer) = self.hysteresis_timers.get_mut(i) {
                        *timer = self.profile.hysteresis_secs;
                    }
                } else if should_deactivate && state.target_weight > LAYER_ACTIVE_THRESHOLD {
                    state.target_weight = 0.0;
                    state.fading_in = false;
                    state.fade_remaining = config.fade_out_secs;
                    if let Some(timer) = self.hysteresis_timers.get_mut(i) {
                        *timer = self.profile.hysteresis_secs;
                    }
                } else if state.is_active() {
                    state.target_weight = signal.min(1.0);
                }

                if state.fade_remaining > 0.0 {
                    let fade_progress = dt / state.fade_remaining.max(dt);
                    state.weight += (state.target_weight - state.weight) * fade_progress;
                    state.fade_remaining = (state.fade_remaining - dt).max(0.0);
                } else {
                    state.weight = state.target_weight;
                }

                state.volume = state.weight * config.max_volume;
            } else if signal >= config.activation_threshold {
                let mut new_state = LayerState::new(config.kind);
                new_state.target_weight = signal.min(1.0);
                new_state.fading_in = true;
                new_state.fade_remaining = config.fade_in_secs;
                self.layer_states.push(new_state);
                if let Some(timer) = self.hysteresis_timers.get_mut(i) {
                    *timer = self.profile.hysteresis_secs;
                }
            }
        }

        self.layer_states.retain(LayerState::is_active);
        self.last_pressure = pressure;

        self.evaluate(&self.last_pressure)
    }

    /// Returns the current layer states.
    #[must_use]
    pub fn layer_states(&self) -> &[LayerState] {
        &self.layer_states
    }

    /// Resets all layer states to inactive.
    pub fn reset(&mut self) {
        self.layer_states.clear();
        self.hysteresis_timers.fill(0.0);
        self.time_since_change = 0.0;
    }
}

/// Computes a deterministic fingerprint for a layer mix.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "layer count fits in u32 for fingerprinting"
)]
pub fn compute_mix_fingerprint(layers: &[MixLayer], pressure: &EnvironmentPressure) -> u64 {
    let mut hasher = crc32fast::Hasher::new();

    hasher.update(&(layers.len() as u32).to_le_bytes());

    for layer in layers {
        hasher.update(&(layer.kind as u8).to_le_bytes());
        hasher.update(&layer.target_volume.to_le_bytes());
        hasher.update(&layer.current_volume.to_le_bytes());
        hasher.update(&layer.priority.to_le_bytes());
    }

    hasher.update(&pressure.weather_severity.to_le_bytes());
    hasher.update(&pressure.oxygen_level.to_le_bytes());
    hasher.update(&pressure.radiation_intensity.to_le_bytes());
    hasher.update(&pressure.hostile_threat.to_le_bytes());
    hasher.update(&[u8::from(pressure.in_combat), u8::from(pressure.in_void)]);

    u64::from(hasher.finalize())
}

/// Computes a fingerprint for a layer profile.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "layer count fits in u32 for fingerprinting"
)]
pub fn compute_profile_fingerprint(profile: &LayerProfile) -> u64 {
    let mut hasher = crc32fast::Hasher::new();

    hasher.update(profile.name.as_bytes());
    hasher.update(&(profile.layers.len() as u32).to_le_bytes());

    for layer in &profile.layers {
        hasher.update(&(layer.kind as u8).to_le_bytes());
        hasher.update(&layer.activation_threshold.to_le_bytes());
        hasher.update(&layer.deactivation_threshold.to_le_bytes());
        hasher.update(&layer.max_volume.to_le_bytes());
        hasher.update(&layer.priority.to_le_bytes());
        hasher.update(&[u8::from(layer.enabled)]);
    }

    hasher.update(&profile.master_volume.to_le_bytes());
    hasher.update(&profile.hysteresis_secs.to_le_bytes());

    u64::from(hasher.finalize())
}

/// Serializes a layer profile to bytes.
///
/// # Errors
///
/// Returns an error if serialization fails.
pub fn serialize_profile(profile: &LayerProfile) -> Result<Vec<u8>, bincode::Error> {
    bincode::serialize(profile)
}

/// Deserializes a layer profile from bytes.
///
/// # Errors
///
/// Returns an error if deserialization fails.
pub fn deserialize_profile(data: &[u8]) -> Result<LayerProfile, bincode::Error> {
    bincode::deserialize(data)
}

/// Serializes a layer mix to bytes.
///
/// # Errors
///
/// Returns an error if serialization fails.
pub fn serialize_mix(mix: &LayerMix) -> Result<Vec<u8>, bincode::Error> {
    bincode::serialize(mix)
}

/// Deserializes a layer mix from bytes.
///
/// # Errors
///
/// Returns an error if deserialization fails.
pub fn deserialize_mix(data: &[u8]) -> Result<LayerMix, bincode::Error> {
    bincode::deserialize(data)
}

/// Serializes environment pressure to bytes.
///
/// # Errors
///
/// Returns an error if serialization fails.
pub fn serialize_pressure(pressure: &EnvironmentPressure) -> Result<Vec<u8>, bincode::Error> {
    bincode::serialize(pressure)
}

/// Deserializes environment pressure from bytes.
///
/// # Errors
///
/// Returns an error if deserialization fails.
pub fn deserialize_pressure(data: &[u8]) -> Result<EnvironmentPressure, bincode::Error> {
    bincode::deserialize(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_layer_kind_all_variants() {
        assert_eq!(MusicLayerKind::ALL.len(), 16);
        for (i, kind) in MusicLayerKind::ALL.iter().enumerate() {
            assert_eq!(*kind as usize, i);
        }
    }

    #[test]
    fn test_layer_kind_priorities() {
        assert!(
            MusicLayerKind::Combat.default_priority() < MusicLayerKind::Danger.default_priority()
        );
        assert!(
            MusicLayerKind::Danger.default_priority() < MusicLayerKind::Tension.default_priority()
        );
        assert!(
            MusicLayerKind::Tension.default_priority()
                < MusicLayerKind::Exploration.default_priority()
        );
        assert!(
            MusicLayerKind::Exploration.default_priority()
                < MusicLayerKind::BaseAmbient.default_priority()
        );
    }

    #[test]
    fn test_layer_kind_categories() {
        assert!(MusicLayerKind::Storm.is_environmental());
        assert!(MusicLayerKind::Radiation.is_environmental());
        assert!(MusicLayerKind::Void.is_environmental());
        assert!(!MusicLayerKind::Combat.is_environmental());

        assert!(MusicLayerKind::Combat.is_threat());
        assert!(MusicLayerKind::Danger.is_threat());
        assert!(!MusicLayerKind::Storm.is_threat());

        assert!(MusicLayerKind::HealthCritical.is_player_condition());
        assert!(MusicLayerKind::Stress.is_player_condition());
        assert!(!MusicLayerKind::Combat.is_player_condition());
    }

    #[test]
    fn test_environment_pressure_default() {
        let pressure = EnvironmentPressure::default();
        assert_relative_eq!(pressure.weather_severity, 0.0);
        assert_relative_eq!(pressure.oxygen_level, 1.0);
        assert_relative_eq!(pressure.hostile_threat, 0.0);
        assert!(!pressure.in_combat);
        assert!(!pressure.in_void);
    }

    #[test]
    fn test_environment_pressure_builders() {
        let pressure = EnvironmentPressure::new()
            .with_weather(0.8)
            .with_oxygen(0.3)
            .with_radiation(0.5)
            .with_combat(true);

        assert_relative_eq!(pressure.weather_severity, 0.8);
        assert_relative_eq!(pressure.oxygen_level, 0.3);
        assert_relative_eq!(pressure.radiation_intensity, 0.5);
        assert!(pressure.in_combat);
    }

    #[test]
    fn test_environment_pressure_clamping() {
        let pressure = EnvironmentPressure::new()
            .with_weather(1.5)
            .with_oxygen(-0.2)
            .with_hostile_threat(2.0);

        let clamped = pressure.clamped();
        assert_relative_eq!(clamped.weather_severity, 1.0);
        assert_relative_eq!(clamped.oxygen_level, 0.0);
        assert_relative_eq!(clamped.hostile_threat, 1.0);
    }

    #[test]
    fn test_environment_pressure_threat_level() {
        let calm = EnvironmentPressure::new();
        assert_relative_eq!(calm.threat_level(), 0.0);

        let hostile = EnvironmentPressure::new().with_hostile_threat(0.6);
        assert_relative_eq!(hostile.threat_level(), 0.6);

        let combat = EnvironmentPressure::new().with_combat(true);
        assert_relative_eq!(combat.threat_level(), 1.0);
    }

    #[test]
    fn test_environment_pressure_environmental() {
        let calm = EnvironmentPressure::new();
        assert_relative_eq!(calm.environmental_pressure(), 0.0);

        let storm = EnvironmentPressure::new().with_weather(0.8);
        assert_relative_eq!(storm.environmental_pressure(), 0.8);

        let multi = EnvironmentPressure::new()
            .with_weather(0.4)
            .with_radiation(0.7);
        assert_relative_eq!(multi.environmental_pressure(), 0.7);
    }

    #[test]
    fn test_layer_config_default() {
        let config = LayerConfig::new(MusicLayerKind::Combat);
        assert_eq!(config.kind, MusicLayerKind::Combat);
        assert_relative_eq!(config.activation_threshold, 0.3);
        assert_relative_eq!(config.max_volume, 1.0);
        assert!(config.enabled);
        assert!(config.is_valid());
    }

    #[test]
    fn test_layer_config_builders() {
        let config = LayerConfig::new(MusicLayerKind::Storm)
            .with_threshold(0.5)
            .with_max_volume(0.8)
            .with_priority(10)
            .with_fade_in(1.0)
            .with_fade_out(2.0);

        assert_relative_eq!(config.activation_threshold, 0.5);
        assert_relative_eq!(config.deactivation_threshold, 0.35);
        assert_relative_eq!(config.max_volume, 0.8);
        assert_eq!(config.priority, 10);
        assert_relative_eq!(config.fade_in_secs, 1.0);
        assert_relative_eq!(config.fade_out_secs, 2.0);
    }

    #[test]
    fn test_layer_config_clamping() {
        let config = LayerConfig::new(MusicLayerKind::Combat)
            .with_activation_threshold(1.5)
            .with_max_volume(-0.1);

        let clamped = config.clamped();
        assert_relative_eq!(clamped.activation_threshold, 1.0);
        assert_relative_eq!(clamped.max_volume, 0.0);
    }

    #[test]
    fn test_layer_state_active() {
        let mut state = LayerState::new(MusicLayerKind::Combat);
        assert!(!state.is_active());

        state.weight = 0.5;
        assert!(state.is_active());

        state.weight = 0.0;
        state.target_weight = 0.8;
        assert!(state.is_active());
    }

    #[test]
    fn test_layer_state_transitioning() {
        let mut state = LayerState::new(MusicLayerKind::Combat);
        state.weight = 0.0;
        state.target_weight = 1.0;
        state.fade_remaining = 2.0;
        assert!(state.is_transitioning());

        state.fade_remaining = 0.0;
        assert!(!state.is_transitioning());
    }

    #[test]
    fn test_layer_mix_empty() {
        let mix = LayerMix::empty();
        assert!(mix.is_silent());
        assert_eq!(mix.layer_count(), 0);
        assert!(mix.dominant_layer.is_none());
    }

    #[test]
    fn test_profile_preset_calm_overworld() {
        let profile = LayerProfile::calm_overworld();
        assert_eq!(profile.name, "calm_overworld");
        assert!(!profile.layers.is_empty());
        assert!(profile.get_layer(MusicLayerKind::BaseAmbient).is_some());
        assert!(profile.get_layer(MusicLayerKind::Combat).is_some());
    }

    #[test]
    fn test_profile_preset_underground() {
        let profile = LayerProfile::underground_exploration();
        assert_eq!(profile.name, "underground_exploration");
        assert!(profile.get_layer(MusicLayerKind::Depth).is_some());
        assert!(profile.get_layer(MusicLayerKind::LowOxygen).is_some());
    }

    #[test]
    fn test_profile_preset_storm() {
        let profile = LayerProfile::storm_survival();
        assert_eq!(profile.name, "storm_survival");
        let storm = profile.get_layer(MusicLayerKind::Storm).unwrap();
        assert_relative_eq!(storm.max_volume, 1.0);
    }

    #[test]
    fn test_profile_preset_combat() {
        let profile = LayerProfile::combat_danger();
        assert_eq!(profile.name, "combat_danger");
        let combat = profile.get_layer(MusicLayerKind::Combat).unwrap();
        assert!(combat.fade_in_secs < DEFAULT_FADE_IN_SECS);
    }

    #[test]
    fn test_profile_preset_void() {
        let profile = LayerProfile::void_pressure();
        assert_eq!(profile.name, "void_pressure");
        assert!(profile.get_layer(MusicLayerKind::Void).is_some());
        assert!(profile.get_layer(MusicLayerKind::LowOxygen).is_some());
    }

    #[test]
    fn test_controller_evaluate_calm() {
        let controller = DynamicMusicController::new(LayerProfile::calm_overworld());
        let pressure = EnvironmentPressure::CALM;
        let mix = controller.evaluate(&pressure);

        assert!(!mix.is_silent());
        assert!(
            mix.layers
                .iter()
                .any(|l| l.kind == MusicLayerKind::BaseAmbient)
        );
        assert!(
            mix.layers
                .iter()
                .any(|l| l.kind == MusicLayerKind::Exploration)
        );
    }

    #[test]
    fn test_controller_evaluate_combat() {
        let controller = DynamicMusicController::new(LayerProfile::calm_overworld());
        let pressure = EnvironmentPressure::new().with_combat(true);
        let mix = controller.evaluate(&pressure);

        assert!(mix.layers.iter().any(|l| l.kind == MusicLayerKind::Combat));
        assert_eq!(mix.dominant_layer, Some(MusicLayerKind::Combat));
    }

    #[test]
    fn test_controller_evaluate_storm() {
        let controller = DynamicMusicController::new(LayerProfile::storm_survival());
        let pressure = EnvironmentPressure::new().with_weather(0.9);
        let mix = controller.evaluate(&pressure);

        assert!(mix.layers.iter().any(|l| l.kind == MusicLayerKind::Storm));
    }

    #[test]
    fn test_controller_tick_fade_in() {
        let mut controller = DynamicMusicController::new(LayerProfile::combat_danger());
        let pressure = EnvironmentPressure::new().with_combat(true);

        let mix1 = controller.tick(0.0, &pressure);
        let mix2 = controller.tick(0.1, &pressure);

        let combat1 = mix1
            .layers
            .iter()
            .find(|l| l.kind == MusicLayerKind::Combat);
        let combat2 = mix2
            .layers
            .iter()
            .find(|l| l.kind == MusicLayerKind::Combat);

        assert!(combat1.is_some());
        assert!(combat2.is_some());
    }

    #[test]
    fn test_controller_priority_sorting() {
        let controller = DynamicMusicController::new(LayerProfile::calm_overworld());
        let pressure = EnvironmentPressure::new()
            .with_combat(true)
            .with_weather(0.8);
        let mix = controller.evaluate(&pressure);

        let priorities: Vec<i32> = mix.layers.iter().map(|l| l.priority).collect();
        let mut sorted = priorities.clone();
        sorted.sort_unstable();
        assert_eq!(priorities, sorted);
    }

    #[test]
    fn test_controller_reset() {
        let mut controller = DynamicMusicController::new(LayerProfile::calm_overworld());
        let pressure = EnvironmentPressure::new().with_combat(true);

        controller.tick(1.0, &pressure);
        assert!(!controller.layer_states().is_empty());

        controller.reset();
        assert!(controller.layer_states().is_empty());
    }

    #[test]
    fn test_fingerprint_deterministic() {
        let layers = vec![MixLayer {
            kind: MusicLayerKind::Combat,
            target_volume: 1.0,
            current_volume: 0.5,
            fade_duration: 0.5,
            priority: 0,
        }];
        let pressure = EnvironmentPressure::new().with_combat(true);

        let fp1 = compute_mix_fingerprint(&layers, &pressure);
        let fp2 = compute_mix_fingerprint(&layers, &pressure);

        assert_eq!(fp1, fp2);
        assert_ne!(fp1, 0);
    }

    #[test]
    fn test_fingerprint_sensitive_to_layers() {
        let pressure = EnvironmentPressure::new();

        let layers1 = vec![MixLayer {
            kind: MusicLayerKind::Combat,
            target_volume: 1.0,
            current_volume: 0.5,
            fade_duration: 0.5,
            priority: 0,
        }];

        let layers2 = vec![MixLayer {
            kind: MusicLayerKind::Storm,
            target_volume: 1.0,
            current_volume: 0.5,
            fade_duration: 0.5,
            priority: 0,
        }];

        let fp1 = compute_mix_fingerprint(&layers1, &pressure);
        let fp2 = compute_mix_fingerprint(&layers2, &pressure);

        assert_ne!(fp1, fp2);
    }

    #[test]
    fn test_fingerprint_sensitive_to_pressure() {
        let layers = vec![MixLayer {
            kind: MusicLayerKind::Combat,
            target_volume: 1.0,
            current_volume: 0.5,
            fade_duration: 0.5,
            priority: 0,
        }];

        let pressure1 = EnvironmentPressure::new().with_combat(true);
        let pressure2 = EnvironmentPressure::new().with_combat(false);

        let fp1 = compute_mix_fingerprint(&layers, &pressure1);
        let fp2 = compute_mix_fingerprint(&layers, &pressure2);

        assert_ne!(fp1, fp2);
    }

    #[test]
    fn test_profile_fingerprint_deterministic() {
        let profile = LayerProfile::calm_overworld();

        let fp1 = compute_profile_fingerprint(&profile);
        let fp2 = compute_profile_fingerprint(&profile);

        assert_eq!(fp1, fp2);
        assert_ne!(fp1, 0);
    }

    #[test]
    fn test_profile_fingerprint_sensitive() {
        let profile1 = LayerProfile::calm_overworld();
        let profile2 = LayerProfile::combat_danger();

        let fp1 = compute_profile_fingerprint(&profile1);
        let fp2 = compute_profile_fingerprint(&profile2);

        assert_ne!(fp1, fp2);
    }

    #[test]
    fn test_serialize_profile_roundtrip() {
        let profile = LayerProfile::underground_exploration();
        let bytes = serialize_profile(&profile).unwrap();
        let restored = deserialize_profile(&bytes).unwrap();

        assert_eq!(profile.name, restored.name);
        assert_eq!(profile.layers.len(), restored.layers.len());
        assert_relative_eq!(profile.master_volume, restored.master_volume);
    }

    #[test]
    fn test_serialize_mix_roundtrip() {
        let mix = LayerMix {
            layers: vec![
                MixLayer {
                    kind: MusicLayerKind::Combat,
                    target_volume: 1.0,
                    current_volume: 0.8,
                    fade_duration: 0.2,
                    priority: 0,
                },
                MixLayer {
                    kind: MusicLayerKind::Tension,
                    target_volume: 0.5,
                    current_volume: 0.5,
                    fade_duration: 0.0,
                    priority: 40,
                },
            ],
            total_volume: 1.5,
            dominant_layer: Some(MusicLayerKind::Combat),
            is_transitioning: true,
            fingerprint: 12345,
        };

        let bytes = serialize_mix(&mix).unwrap();
        let restored = deserialize_mix(&bytes).unwrap();

        assert_eq!(mix.layers.len(), restored.layers.len());
        assert_eq!(mix.dominant_layer, restored.dominant_layer);
        assert_eq!(mix.fingerprint, restored.fingerprint);
    }

    #[test]
    fn test_serialize_pressure_roundtrip() {
        let pressure = EnvironmentPressure::new()
            .with_weather(0.7)
            .with_oxygen(0.4)
            .with_radiation(0.3)
            .with_combat(true)
            .with_void(true);

        let bytes = serialize_pressure(&pressure).unwrap();
        let restored = deserialize_pressure(&bytes).unwrap();

        assert_relative_eq!(pressure.weather_severity, restored.weather_severity);
        assert_relative_eq!(pressure.oxygen_level, restored.oxygen_level);
        assert_eq!(pressure.in_combat, restored.in_combat);
        assert_eq!(pressure.in_void, restored.in_void);
    }

    #[test]
    fn test_max_active_layers_limit() {
        let mut profile = LayerProfile::new("many_layers");
        for kind in MusicLayerKind::ALL {
            profile = profile.with_layer(
                LayerConfig::new(kind)
                    .with_threshold(0.0)
                    .with_max_volume(1.0),
            );
        }

        let controller = DynamicMusicController::new(profile);
        let pressure = EnvironmentPressure::new()
            .with_weather(1.0)
            .with_radiation(1.0)
            .with_combat(true)
            .with_void(true)
            .with_player_health(0.1);

        let mix = controller.evaluate(&pressure);
        assert!(mix.layer_count() <= MAX_ACTIVE_LAYERS);
    }

    #[test]
    fn test_threshold_hysteresis() {
        let config = LayerConfig::new(MusicLayerKind::Storm)
            .with_activation_threshold(0.5)
            .with_deactivation_threshold(0.3);

        assert_relative_eq!(config.activation_threshold, 0.5);
        assert_relative_eq!(config.deactivation_threshold, 0.3);
        assert!(config.is_valid());
    }

    #[test]
    fn test_layer_mix_loudest() {
        let mix = LayerMix {
            layers: vec![
                MixLayer {
                    kind: MusicLayerKind::BaseAmbient,
                    target_volume: 0.3,
                    current_volume: 0.3,
                    fade_duration: 0.0,
                    priority: 100,
                },
                MixLayer {
                    kind: MusicLayerKind::Combat,
                    target_volume: 1.0,
                    current_volume: 1.0,
                    fade_duration: 0.0,
                    priority: 0,
                },
            ],
            total_volume: 1.3,
            dominant_layer: Some(MusicLayerKind::Combat),
            is_transitioning: false,
            fingerprint: 0,
        };

        let loudest = mix.loudest_layer().unwrap();
        assert_eq!(loudest.kind, MusicLayerKind::Combat);
    }
}
