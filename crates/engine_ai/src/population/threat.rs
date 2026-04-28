//! Regional threat level and scaling.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Threat level classification.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum ThreatLevel {
    /// Safe zone, no threats.
    Safe,
    /// Low threat, occasional hostile encounters.
    Low,
    /// Moderate threat, regular hostile presence.
    #[default]
    Moderate,
    /// High threat, dangerous area.
    High,
    /// Extreme threat, very dangerous.
    Extreme,
}

impl ThreatLevel {
    /// Get the threat multiplier for spawn calculations.
    #[must_use]
    pub fn hostile_spawn_multiplier(self) -> f32 {
        match self {
            Self::Safe => 0.0,
            Self::Low => 0.5,
            Self::Moderate => 1.0,
            Self::High => 1.5,
            Self::Extreme => 2.0,
        }
    }

    /// Get the passive spawn multiplier (inverse of threat).
    #[must_use]
    pub fn passive_spawn_multiplier(self) -> f32 {
        match self {
            Self::Safe => 1.5,
            Self::Low => 1.2,
            Self::Moderate => 1.0,
            Self::High => 0.7,
            Self::Extreme => 0.3,
        }
    }

    /// Get threat score for comparison (0-100).
    #[must_use]
    pub fn score(self) -> u32 {
        match self {
            Self::Safe => 0,
            Self::Low => 25,
            Self::Moderate => 50,
            Self::High => 75,
            Self::Extreme => 100,
        }
    }

    /// Create from a numeric score (0-100).
    #[must_use]
    pub fn from_score(score: u32) -> Self {
        match score {
            0..=12 => Self::Safe,
            13..=37 => Self::Low,
            38..=62 => Self::Moderate,
            63..=87 => Self::High,
            _ => Self::Extreme,
        }
    }
}

/// Source of threat in a region.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ThreatSource {
    /// Base biome/terrain threat.
    Biome,
    /// Hostile creature presence.
    HostilePresence,
    /// Recent combat activity.
    RecentCombat,
    /// Environmental hazards.
    Environmental,
    /// Boss or elite presence.
    ElitePresence,
    /// Player-induced threat.
    PlayerActivity,
    /// Faction conflict.
    FactionConflict,
    /// Time-based (night, events).
    Temporal,
    /// Custom source.
    Custom(String),
}

impl ThreatSource {
    /// Get default weight for this source.
    #[must_use]
    pub fn default_weight(&self) -> f32 {
        match self {
            Self::Biome | Self::PlayerActivity | Self::Custom(_) => 1.0,
            Self::HostilePresence => 1.5,
            Self::RecentCombat => 1.2,
            Self::Environmental => 0.8,
            Self::ElitePresence => 2.0,
            Self::FactionConflict => 1.3,
            Self::Temporal => 0.7,
        }
    }
}

/// A modifier to regional threat.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThreatModifier {
    /// Source of the threat.
    pub source: ThreatSource,
    /// Magnitude of the modifier (-100 to 100).
    pub magnitude: i32,
    /// Optional expiration tick.
    pub expires_tick: Option<u64>,
    /// Weight multiplier for this modifier.
    pub weight: f32,
}

impl ThreatModifier {
    /// Create a new threat modifier.
    #[must_use]
    pub fn new(source: ThreatSource, magnitude: i32) -> Self {
        let weight = source.default_weight();
        Self {
            source,
            magnitude: magnitude.clamp(-100, 100),
            expires_tick: None,
            weight,
        }
    }

    #[must_use]
    pub fn with_expiration(mut self, tick: u64) -> Self {
        self.expires_tick = Some(tick);
        self
    }

    #[must_use]
    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight.max(0.0);
        self
    }

    /// Check if expired.
    #[must_use]
    pub fn is_expired(&self, current_tick: u64) -> bool {
        self.expires_tick.is_some_and(|t| current_tick >= t)
    }

    /// Get weighted magnitude.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "magnitude is bounded to -100..100"
    )]
    pub fn weighted_magnitude(&self) -> f32 {
        self.magnitude as f32 * self.weight
    }
}

impl Eq for ThreatModifier {}

impl PartialOrd for ThreatModifier {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ThreatModifier {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .magnitude
            .cmp(&self.magnitude)
            .then_with(|| self.source.cmp(&other.source))
    }
}

/// Configuration for threat calculation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThreatConfig {
    /// Base threat level for the region.
    pub base_level: ThreatLevel,
    /// How quickly threat decays toward base (0.0-1.0 per tick).
    pub decay_rate: f32,
    /// Minimum threat level (floor).
    pub minimum: ThreatLevel,
    /// Maximum threat level (ceiling).
    pub maximum: ThreatLevel,
    /// Whether threat can increase from player activity.
    pub player_threat_enabled: bool,
    /// Ticks between threat recalculation.
    pub recalc_interval: u64,
}

impl ThreatConfig {
    /// Create a new threat configuration.
    #[must_use]
    pub fn new(base_level: ThreatLevel) -> Self {
        Self {
            base_level,
            decay_rate: 0.01,
            minimum: ThreatLevel::Safe,
            maximum: ThreatLevel::Extreme,
            player_threat_enabled: true,
            recalc_interval: 60,
        }
    }

    #[must_use]
    pub fn with_decay_rate(mut self, rate: f32) -> Self {
        self.decay_rate = rate.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_bounds(mut self, minimum: ThreatLevel, maximum: ThreatLevel) -> Self {
        self.minimum = minimum;
        self.maximum = maximum;
        self
    }

    #[must_use]
    pub fn with_recalc_interval(mut self, ticks: u64) -> Self {
        self.recalc_interval = ticks.max(1);
        self
    }

    /// Clamp a threat score to configured bounds.
    #[must_use]
    pub fn clamp_score(&self, score: u32) -> u32 {
        score.clamp(self.minimum.score(), self.maximum.score())
    }

    /// Clamp a threat level to configured bounds.
    #[must_use]
    pub fn clamp_level(&self, level: ThreatLevel) -> ThreatLevel {
        if level < self.minimum {
            self.minimum
        } else if level > self.maximum {
            self.maximum
        } else {
            level
        }
    }
}

impl Default for ThreatConfig {
    fn default() -> Self {
        Self::new(ThreatLevel::Moderate)
    }
}

/// Regional threat state with modifiers and calculation.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RegionalThreat {
    /// Current calculated threat score (0-100).
    score: u32,
    /// Current threat level.
    level: ThreatLevel,
    /// Active modifiers.
    modifiers: Vec<ThreatModifier>,
    /// Configuration.
    config: ThreatConfig,
    /// Last recalculation tick.
    last_recalc_tick: u64,
}

impl RegionalThreat {
    /// Create a new regional threat tracker.
    #[must_use]
    pub fn new(config: ThreatConfig) -> Self {
        let level = config.base_level;
        let score = level.score();
        Self {
            score,
            level,
            modifiers: Vec::new(),
            config,
            last_recalc_tick: 0,
        }
    }

    /// Get current threat level.
    #[must_use]
    pub fn level(&self) -> ThreatLevel {
        self.level
    }

    /// Get current threat score.
    #[must_use]
    pub fn score(&self) -> u32 {
        self.score
    }

    /// Get the configuration.
    #[must_use]
    pub fn config(&self) -> &ThreatConfig {
        &self.config
    }

    /// Add a modifier.
    pub fn add_modifier(&mut self, modifier: ThreatModifier) {
        self.modifiers.push(modifier);
        self.modifiers.sort();
    }

    /// Remove modifiers from a specific source.
    pub fn remove_modifiers(&mut self, source: &ThreatSource) {
        self.modifiers.retain(|m| &m.source != source);
    }

    /// Get active modifiers.
    pub fn modifiers(&self) -> &[ThreatModifier] {
        &self.modifiers
    }

    /// Calculate effective threat score.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "score is clamped to valid u32 range"
    )]
    fn calculate_score(&self) -> u32 {
        let base = self.config.base_level.score() as f32;
        let modifier_sum: f32 = self
            .modifiers
            .iter()
            .map(ThreatModifier::weighted_magnitude)
            .sum();
        let raw = (base + modifier_sum).clamp(0.0, 100.0) as u32;
        self.config.clamp_score(raw)
    }

    /// Update threat state for a tick.
    pub fn tick(&mut self, current_tick: u64) {
        self.modifiers.retain(|m| !m.is_expired(current_tick));

        if current_tick.saturating_sub(self.last_recalc_tick) >= self.config.recalc_interval {
            self.recalculate();
            self.last_recalc_tick = current_tick;
        }
    }

    /// Force recalculation of threat level.
    pub fn recalculate(&mut self) {
        self.score = self.calculate_score();
        self.level = self.config.clamp_level(ThreatLevel::from_score(self.score));
    }

    /// Set threat level directly (overrides calculation until next tick).
    pub fn set_level(&mut self, level: ThreatLevel) {
        self.level = self.config.clamp_level(level);
        self.score = self.level.score();
    }

    /// Get hostile spawn multiplier.
    #[must_use]
    pub fn hostile_spawn_multiplier(&self) -> f32 {
        self.level.hostile_spawn_multiplier()
    }

    /// Get passive spawn multiplier.
    #[must_use]
    pub fn passive_spawn_multiplier(&self) -> f32 {
        self.level.passive_spawn_multiplier()
    }

    /// Check if this is a safe zone.
    #[must_use]
    pub fn is_safe(&self) -> bool {
        self.level == ThreatLevel::Safe
    }

    /// Check if this is a high threat zone.
    #[must_use]
    pub fn is_dangerous(&self) -> bool {
        self.level >= ThreatLevel::High
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threat_level_ordering() {
        assert!(ThreatLevel::Safe < ThreatLevel::Low);
        assert!(ThreatLevel::Low < ThreatLevel::Moderate);
        assert!(ThreatLevel::Moderate < ThreatLevel::High);
        assert!(ThreatLevel::High < ThreatLevel::Extreme);
    }

    #[test]
    fn test_threat_level_multipliers() {
        assert!((ThreatLevel::Safe.hostile_spawn_multiplier()).abs() < f32::EPSILON);
        assert!((ThreatLevel::Moderate.hostile_spawn_multiplier() - 1.0).abs() < f32::EPSILON);
        assert!(ThreatLevel::Extreme.hostile_spawn_multiplier() > 1.0);

        assert!(ThreatLevel::Safe.passive_spawn_multiplier() > 1.0);
        assert!(ThreatLevel::Extreme.passive_spawn_multiplier() < 1.0);
    }

    #[test]
    fn test_threat_level_score_roundtrip() {
        for level in [
            ThreatLevel::Safe,
            ThreatLevel::Low,
            ThreatLevel::Moderate,
            ThreatLevel::High,
            ThreatLevel::Extreme,
        ] {
            let score = level.score();
            let restored = ThreatLevel::from_score(score);
            assert_eq!(level, restored);
        }
    }

    #[test]
    fn test_threat_source_weight() {
        assert!(
            ThreatSource::ElitePresence.default_weight() > ThreatSource::Biome.default_weight()
        );
    }

    #[test]
    fn test_threat_modifier_new() {
        let modifier = ThreatModifier::new(ThreatSource::HostilePresence, 25);

        assert_eq!(modifier.magnitude, 25);
        assert!(modifier.expires_tick.is_none());
    }

    #[test]
    fn test_threat_modifier_expiration() {
        let modifier = ThreatModifier::new(ThreatSource::RecentCombat, 30).with_expiration(100);

        assert!(!modifier.is_expired(50));
        assert!(modifier.is_expired(100));
        assert!(modifier.is_expired(150));
    }

    #[test]
    fn test_threat_modifier_clamping() {
        let modifier = ThreatModifier::new(ThreatSource::Biome, 500);
        assert_eq!(modifier.magnitude, 100);

        let modifier = ThreatModifier::new(ThreatSource::Biome, -500);
        assert_eq!(modifier.magnitude, -100);
    }

    #[test]
    fn test_threat_modifier_weighted_magnitude() {
        let modifier = ThreatModifier::new(ThreatSource::Biome, 50).with_weight(2.0);
        assert!((modifier.weighted_magnitude() - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_threat_config_new() {
        let config = ThreatConfig::new(ThreatLevel::High);

        assert_eq!(config.base_level, ThreatLevel::High);
        assert_eq!(config.minimum, ThreatLevel::Safe);
        assert_eq!(config.maximum, ThreatLevel::Extreme);
    }

    #[test]
    fn test_threat_config_clamp() {
        let config = ThreatConfig::new(ThreatLevel::Moderate)
            .with_bounds(ThreatLevel::Low, ThreatLevel::High);

        assert_eq!(config.clamp_level(ThreatLevel::Safe), ThreatLevel::Low);
        assert_eq!(config.clamp_level(ThreatLevel::Extreme), ThreatLevel::High);
        assert_eq!(
            config.clamp_level(ThreatLevel::Moderate),
            ThreatLevel::Moderate
        );
    }

    #[test]
    fn test_regional_threat_new() {
        let config = ThreatConfig::new(ThreatLevel::Moderate);
        let threat = RegionalThreat::new(config);

        assert_eq!(threat.level(), ThreatLevel::Moderate);
        assert_eq!(threat.score(), ThreatLevel::Moderate.score());
    }

    #[test]
    fn test_regional_threat_modifiers() {
        let config = ThreatConfig::new(ThreatLevel::Moderate);
        let mut threat = RegionalThreat::new(config);

        threat.add_modifier(ThreatModifier::new(ThreatSource::HostilePresence, 30));
        threat.recalculate();

        assert!(threat.score() > ThreatLevel::Moderate.score());
    }

    #[test]
    fn test_regional_threat_negative_modifier() {
        let config = ThreatConfig::new(ThreatLevel::Moderate);
        let mut threat = RegionalThreat::new(config);

        threat.add_modifier(ThreatModifier::new(ThreatSource::PlayerActivity, -30));
        threat.recalculate();

        assert!(threat.score() < ThreatLevel::Moderate.score());
    }

    #[test]
    fn test_regional_threat_tick_expires_modifiers() {
        let config = ThreatConfig::new(ThreatLevel::Moderate).with_recalc_interval(1);
        let mut threat = RegionalThreat::new(config);

        threat
            .add_modifier(ThreatModifier::new(ThreatSource::RecentCombat, 50).with_expiration(100));
        assert_eq!(threat.modifiers().len(), 1);

        threat.tick(50);
        assert_eq!(threat.modifiers().len(), 1);

        threat.tick(100);
        assert!(threat.modifiers().is_empty());
    }

    #[test]
    fn test_regional_threat_remove_modifiers() {
        let config = ThreatConfig::new(ThreatLevel::Moderate);
        let mut threat = RegionalThreat::new(config);

        threat.add_modifier(ThreatModifier::new(ThreatSource::HostilePresence, 20));
        threat.add_modifier(ThreatModifier::new(ThreatSource::RecentCombat, 10));

        threat.remove_modifiers(&ThreatSource::HostilePresence);
        assert_eq!(threat.modifiers().len(), 1);
        assert_eq!(threat.modifiers()[0].source, ThreatSource::RecentCombat);
    }

    #[test]
    fn test_regional_threat_is_safe_dangerous() {
        let safe_config =
            ThreatConfig::new(ThreatLevel::Safe).with_bounds(ThreatLevel::Safe, ThreatLevel::Safe);
        let safe_threat = RegionalThreat::new(safe_config);

        assert!(safe_threat.is_safe());
        assert!(!safe_threat.is_dangerous());

        let dangerous_config = ThreatConfig::new(ThreatLevel::Extreme);
        let dangerous_threat = RegionalThreat::new(dangerous_config);

        assert!(!dangerous_threat.is_safe());
        assert!(dangerous_threat.is_dangerous());
    }

    #[test]
    fn test_regional_threat_set_level() {
        let config = ThreatConfig::new(ThreatLevel::Moderate);
        let mut threat = RegionalThreat::new(config);

        threat.set_level(ThreatLevel::High);

        assert_eq!(threat.level(), ThreatLevel::High);
        assert_eq!(threat.score(), ThreatLevel::High.score());
    }

    #[test]
    fn test_serde_threat_level() {
        let level = ThreatLevel::High;

        let json = serde_json::to_string(&level).unwrap();
        let restored: ThreatLevel = serde_json::from_str(&json).unwrap();

        assert_eq!(restored, level);
    }

    #[test]
    fn test_serde_threat_modifier() {
        let modifier = ThreatModifier::new(ThreatSource::HostilePresence, 40).with_expiration(500);

        let json = serde_json::to_string(&modifier).unwrap();
        let restored: ThreatModifier = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.magnitude, 40);
        assert_eq!(restored.expires_tick, Some(500));
    }

    #[test]
    fn test_serde_regional_threat() {
        let config = ThreatConfig::new(ThreatLevel::High);
        let mut threat = RegionalThreat::new(config);
        threat.add_modifier(ThreatModifier::new(ThreatSource::Biome, 10));

        let json = serde_json::to_string(&threat).unwrap();
        let restored: RegionalThreat = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.level(), threat.level());
        assert_eq!(restored.modifiers().len(), 1);
    }

    #[test]
    fn test_threat_config_serde() {
        let config = ThreatConfig::new(ThreatLevel::Low)
            .with_decay_rate(0.05)
            .with_bounds(ThreatLevel::Safe, ThreatLevel::High);

        let json = serde_json::to_string(&config).unwrap();
        let restored: ThreatConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.base_level, ThreatLevel::Low);
        assert!((restored.decay_rate - 0.05).abs() < f32::EPSILON);
    }
}
