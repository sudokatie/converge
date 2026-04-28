//! Spawn/despawn budgets and pacing intensity.

use super::species::SpeciesCapId;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;

/// Priority for spawn requests.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum SpawnPriority {
    /// Low priority, may be deferred.
    Low,
    /// Normal priority.
    #[default]
    Normal,
    /// High priority, spawns soon.
    High,
    /// Critical priority, spawns immediately if possible.
    Critical,
    /// Emergency, bypasses some caps.
    Emergency,
}

impl SpawnPriority {
    /// Get numeric weight for sorting.
    #[must_use]
    pub fn weight(self) -> u32 {
        match self {
            Self::Low => 1,
            Self::Normal => 2,
            Self::High => 3,
            Self::Critical => 4,
            Self::Emergency => 5,
        }
    }
}

/// A queued spawn event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpawnEvent {
    /// Species to spawn.
    pub species: SpeciesCapId,
    /// Number to spawn.
    pub count: u32,
    /// Spawn priority.
    pub priority: SpawnPriority,
    /// Tick when requested.
    pub requested_tick: u64,
    /// Optional deadline tick.
    pub deadline_tick: Option<u64>,
    /// Region identifier (optional).
    pub region_id: Option<String>,
}

impl SpawnEvent {
    /// Create a new spawn event.
    #[must_use]
    pub fn new(species: SpeciesCapId, count: u32) -> Self {
        Self {
            species,
            count,
            priority: SpawnPriority::Normal,
            requested_tick: 0,
            deadline_tick: None,
            region_id: None,
        }
    }

    #[must_use]
    pub fn with_priority(mut self, priority: SpawnPriority) -> Self {
        self.priority = priority;
        self
    }

    #[must_use]
    pub fn with_deadline(mut self, tick: u64) -> Self {
        self.deadline_tick = Some(tick);
        self
    }

    #[must_use]
    pub fn with_region(mut self, region_id: impl Into<String>) -> Self {
        self.region_id = Some(region_id.into());
        self
    }

    #[must_use]
    pub fn with_requested_tick(mut self, tick: u64) -> Self {
        self.requested_tick = tick;
        self
    }

    /// Check if deadline has passed.
    #[must_use]
    pub fn is_expired(&self, current_tick: u64) -> bool {
        self.deadline_tick.is_some_and(|d| current_tick > d)
    }
}

impl Eq for SpawnEvent {}

impl PartialOrd for SpawnEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SpawnEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .priority
            .cmp(&self.priority)
            .then_with(|| self.requested_tick.cmp(&other.requested_tick))
            .then_with(|| self.species.cmp(&other.species))
    }
}

/// Reason for despawning entities.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DespawnReason {
    /// Over population cap.
    OverCap,
    /// Migration to another region.
    Migration,
    /// Threat level too low for hostiles.
    ThreatTooLow,
    /// Too far from player activity.
    OutOfRange,
    /// Natural population decay.
    NaturalDecay,
    /// Explicit cleanup request.
    Cleanup,
    /// Custom reason.
    Custom(String),
}

/// Despawn budget for a region.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DespawnBudget {
    /// Maximum despawns per tick.
    pub max_per_tick: u32,
    /// Despawns remaining this tick.
    remaining: u32,
    /// Cooldown ticks between bulk despawns.
    pub cooldown_ticks: u64,
    /// Last bulk despawn tick.
    last_despawn_tick: u64,
    /// Protected species that cannot be despawned.
    protected: Vec<SpeciesCapId>,
}

impl DespawnBudget {
    /// Create a new despawn budget.
    #[must_use]
    pub fn new(max_per_tick: u32) -> Self {
        Self {
            max_per_tick,
            remaining: max_per_tick,
            cooldown_ticks: 0,
            last_despawn_tick: 0,
            protected: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_cooldown(mut self, ticks: u64) -> Self {
        self.cooldown_ticks = ticks;
        self
    }

    #[must_use]
    pub fn with_protected(mut self, species: SpeciesCapId) -> Self {
        if !self.protected.contains(&species) {
            self.protected.push(species);
            self.protected.sort();
        }
        self
    }

    /// Get remaining despawns this tick.
    #[must_use]
    pub fn remaining(&self) -> u32 {
        self.remaining
    }

    /// Check if species is protected.
    #[must_use]
    pub fn is_protected(&self, species: &SpeciesCapId) -> bool {
        self.protected.contains(species)
    }

    /// Check if on cooldown.
    #[must_use]
    pub fn is_on_cooldown(&self, current_tick: u64) -> bool {
        current_tick.saturating_sub(self.last_despawn_tick) < self.cooldown_ticks
    }

    /// Try to consume despawn budget.
    pub fn try_despawn(&mut self, count: u32, current_tick: u64) -> u32 {
        if self.is_on_cooldown(current_tick) {
            return 0;
        }

        let actual = count.min(self.remaining);
        self.remaining = self.remaining.saturating_sub(actual);

        if actual > 0 {
            self.last_despawn_tick = current_tick;
        }

        actual
    }

    /// Reset budget for new tick.
    pub fn reset(&mut self) {
        self.remaining = self.max_per_tick;
    }
}

/// Pacing intensity level.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum PacingIntensity {
    /// Very slow pacing, minimal spawns.
    VeryLow,
    /// Relaxed pacing.
    Low,
    /// Normal pacing.
    #[default]
    Normal,
    /// Elevated pacing.
    High,
    /// Intense pacing, frequent spawns.
    VeryHigh,
}

impl PacingIntensity {
    /// Get spawn rate multiplier.
    #[must_use]
    pub fn spawn_multiplier(self) -> f32 {
        match self {
            Self::VeryLow => 0.25,
            Self::Low => 0.5,
            Self::Normal => 1.0,
            Self::High => 1.5,
            Self::VeryHigh => 2.0,
        }
    }

    /// Get cooldown multiplier (lower = more frequent).
    #[must_use]
    pub fn cooldown_multiplier(self) -> f32 {
        match self {
            Self::VeryLow => 2.0,
            Self::Low => 1.5,
            Self::Normal => 1.0,
            Self::High => 0.75,
            Self::VeryHigh => 0.5,
        }
    }

    /// Create from a numeric value (0.0 = `VeryLow`, 1.0 = `VeryHigh`).
    #[must_use]
    pub fn from_normalized(value: f32) -> Self {
        let clamped = value.clamp(0.0, 1.0);
        if clamped < 0.2 {
            Self::VeryLow
        } else if clamped < 0.4 {
            Self::Low
        } else if clamped < 0.6 {
            Self::Normal
        } else if clamped < 0.8 {
            Self::High
        } else {
            Self::VeryHigh
        }
    }

    /// Convert to normalized value.
    #[must_use]
    pub fn to_normalized(self) -> f32 {
        match self {
            Self::VeryLow => 0.1,
            Self::Low => 0.3,
            Self::Normal => 0.5,
            Self::High => 0.7,
            Self::VeryHigh => 0.9,
        }
    }
}

/// Profile for pacing behavior over time.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PacingProfile {
    /// Current intensity.
    intensity: PacingIntensity,
    /// Target intensity (for ramping).
    target: PacingIntensity,
    /// Ticks to reach target.
    ramp_ticks: u64,
    /// Ticks elapsed in current ramp.
    ramp_elapsed: u64,
    /// Minimum intensity floor.
    pub floor: PacingIntensity,
    /// Maximum intensity ceiling.
    pub ceiling: PacingIntensity,
    /// Whether to automatically adjust based on activity.
    pub auto_adjust: bool,
}

impl PacingProfile {
    /// Create a new pacing profile.
    #[must_use]
    pub fn new(intensity: PacingIntensity) -> Self {
        Self {
            intensity,
            target: intensity,
            ramp_ticks: 0,
            ramp_elapsed: 0,
            floor: PacingIntensity::VeryLow,
            ceiling: PacingIntensity::VeryHigh,
            auto_adjust: false,
        }
    }

    #[must_use]
    pub fn with_bounds(mut self, floor: PacingIntensity, ceiling: PacingIntensity) -> Self {
        self.floor = floor;
        self.ceiling = ceiling;
        self
    }

    #[must_use]
    pub fn with_auto_adjust(mut self, enabled: bool) -> Self {
        self.auto_adjust = enabled;
        self
    }

    /// Get current intensity.
    #[must_use]
    pub fn intensity(&self) -> PacingIntensity {
        self.intensity
    }

    /// Get target intensity.
    #[must_use]
    pub fn target(&self) -> PacingIntensity {
        self.target
    }

    /// Check if currently ramping.
    #[must_use]
    pub fn is_ramping(&self) -> bool {
        self.intensity != self.target && self.ramp_ticks > 0
    }

    /// Set intensity immediately.
    pub fn set_intensity(&mut self, intensity: PacingIntensity) {
        self.intensity = self.clamp(intensity);
        self.target = self.intensity;
        self.ramp_ticks = 0;
        self.ramp_elapsed = 0;
    }

    /// Ramp to a target intensity over time.
    pub fn ramp_to(&mut self, target: PacingIntensity, ticks: u64) {
        self.target = self.clamp(target);
        self.ramp_ticks = ticks.max(1);
        self.ramp_elapsed = 0;
    }

    /// Clamp intensity to bounds.
    fn clamp(&self, intensity: PacingIntensity) -> PacingIntensity {
        if intensity < self.floor {
            self.floor
        } else if intensity > self.ceiling {
            self.ceiling
        } else {
            intensity
        }
    }

    /// Tick the pacing profile.
    #[expect(
        clippy::cast_precision_loss,
        reason = "ramp tick values are bounded and small"
    )]
    pub fn tick(&mut self) {
        if !self.is_ramping() {
            return;
        }

        self.ramp_elapsed += 1;

        if self.ramp_elapsed >= self.ramp_ticks {
            self.intensity = self.target;
            self.ramp_ticks = 0;
            self.ramp_elapsed = 0;
        } else {
            let progress = self.ramp_elapsed as f32 / self.ramp_ticks as f32;
            let current = self.intensity.to_normalized();
            let target = self.target.to_normalized();
            let lerped = current + (target - current) * progress;
            self.intensity = self.clamp(PacingIntensity::from_normalized(lerped));
        }
    }

    /// Get effective spawn multiplier.
    #[must_use]
    pub fn spawn_multiplier(&self) -> f32 {
        self.intensity.spawn_multiplier()
    }

    /// Get effective cooldown multiplier.
    #[must_use]
    pub fn cooldown_multiplier(&self) -> f32 {
        self.intensity.cooldown_multiplier()
    }
}

impl Default for PacingProfile {
    fn default() -> Self {
        Self::new(PacingIntensity::Normal)
    }
}

/// Spawn budget for a region.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpawnBudget {
    /// Base spawn budget per tick.
    pub base_budget: u32,
    /// Maximum spawn budget (hard cap).
    pub max_budget: u32,
    /// Current available budget.
    available: u32,
    /// Queued spawn events.
    queue: Vec<SpawnEvent>,
    /// Cooldown tracking per species.
    cooldowns: BTreeMap<SpeciesCapId, u64>,
    /// Base cooldown ticks between spawns of same species.
    pub species_cooldown: u64,
    /// Pacing profile.
    pacing: PacingProfile,
    /// Current tick.
    current_tick: u64,
}

impl SpawnBudget {
    /// Create a new spawn budget.
    #[must_use]
    pub fn new(base_budget: u32, max_budget: u32) -> Self {
        Self {
            base_budget,
            max_budget,
            available: base_budget,
            queue: Vec::new(),
            cooldowns: BTreeMap::new(),
            species_cooldown: 60,
            pacing: PacingProfile::default(),
            current_tick: 0,
        }
    }

    #[must_use]
    pub fn with_species_cooldown(mut self, ticks: u64) -> Self {
        self.species_cooldown = ticks;
        self
    }

    #[must_use]
    pub fn with_pacing(mut self, pacing: PacingProfile) -> Self {
        self.pacing = pacing;
        self
    }

    /// Get available budget.
    #[must_use]
    pub fn available(&self) -> u32 {
        self.available
    }

    /// Get pacing profile.
    #[must_use]
    pub fn pacing(&self) -> &PacingProfile {
        &self.pacing
    }

    /// Get mutable pacing profile.
    pub fn pacing_mut(&mut self) -> &mut PacingProfile {
        &mut self.pacing
    }

    /// Get current tick.
    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Queue a spawn event.
    pub fn queue_spawn(&mut self, mut event: SpawnEvent) {
        event.requested_tick = self.current_tick;
        self.queue.push(event);
        self.queue.sort();
    }

    /// Get queued events.
    pub fn queued_events(&self) -> &[SpawnEvent] {
        &self.queue
    }

    /// Check if species is on cooldown.
    #[must_use]
    pub fn is_species_on_cooldown(&self, species: &SpeciesCapId) -> bool {
        self.cooldowns
            .get(species)
            .is_some_and(|&cd| self.current_tick < cd)
    }

    /// Get effective cooldown for species.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "cooldown product is bounded"
    )]
    pub fn effective_cooldown(&self) -> u64 {
        (self.species_cooldown as f32 * self.pacing.cooldown_multiplier()) as u64
    }

    /// Try to consume spawn budget.
    pub fn try_spawn(&mut self, species: &SpeciesCapId, count: u32) -> u32 {
        if self.is_species_on_cooldown(species) {
            return 0;
        }

        let actual = count.min(self.available);
        self.available = self.available.saturating_sub(actual);

        if actual > 0 {
            let cooldown_end = self.current_tick + self.effective_cooldown();
            self.cooldowns.insert(species.clone(), cooldown_end);
        }

        actual
    }

    /// Process queued spawns, returning events that can be spawned.
    pub fn process_queue(&mut self) -> Vec<SpawnEvent> {
        let current_tick = self.current_tick;
        self.queue.retain(|e| !e.is_expired(current_tick));

        let mut spawnable = Vec::new();
        let mut remaining_budget = self.available;
        let mut remaining_queue = Vec::new();

        let effective_cooldown = self.effective_cooldown();
        let queue = std::mem::take(&mut self.queue);

        for event in queue {
            if remaining_budget == 0 {
                remaining_queue.push(event);
                continue;
            }

            let on_cooldown = self
                .cooldowns
                .get(&event.species)
                .is_some_and(|&cd| current_tick < cd);

            if on_cooldown && event.priority < SpawnPriority::Emergency {
                remaining_queue.push(event);
                continue;
            }

            let spawn_count = event.count.min(remaining_budget);
            if spawn_count > 0 {
                remaining_budget = remaining_budget.saturating_sub(spawn_count);

                let cooldown_end = current_tick + effective_cooldown;
                self.cooldowns.insert(event.species.clone(), cooldown_end);

                spawnable.push(SpawnEvent {
                    count: spawn_count,
                    ..event.clone()
                });

                if spawn_count < event.count {
                    remaining_queue.push(SpawnEvent {
                        count: event.count - spawn_count,
                        ..event
                    });
                }
            } else {
                remaining_queue.push(event);
            }
        }

        self.available = remaining_budget;
        self.queue = remaining_queue;
        self.queue.sort();

        spawnable
    }

    /// Tick the spawn budget.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "budget product is bounded"
    )]
    pub fn tick(&mut self) {
        self.current_tick += 1;
        self.pacing.tick();

        let effective_budget = (self.base_budget as f32 * self.pacing.spawn_multiplier()) as u32;
        self.available = effective_budget.min(self.max_budget);

        self.cooldowns.retain(|_, &mut cd| cd > self.current_tick);
    }

    /// Calculate effective spawn rate for a species.
    #[must_use]
    pub fn effective_spawn_rate(&self, base_rate: f32) -> f32 {
        base_rate * self.pacing.spawn_multiplier()
    }

    /// Clear all queued events.
    pub fn clear_queue(&mut self) {
        self.queue.clear();
    }

    /// Get queue length.
    #[must_use]
    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }
}

impl Default for SpawnBudget {
    fn default() -> Self {
        Self::new(5, 20)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_priority_ordering() {
        assert!(SpawnPriority::Low < SpawnPriority::Normal);
        assert!(SpawnPriority::Normal < SpawnPriority::High);
        assert!(SpawnPriority::High < SpawnPriority::Critical);
        assert!(SpawnPriority::Critical < SpawnPriority::Emergency);
    }

    #[test]
    fn test_spawn_event_new() {
        let event = SpawnEvent::new(SpeciesCapId::new("wolf"), 5);

        assert_eq!(event.species.as_str(), "wolf");
        assert_eq!(event.count, 5);
        assert_eq!(event.priority, SpawnPriority::Normal);
    }

    #[test]
    fn test_spawn_event_ordering() {
        let low = SpawnEvent::new(SpeciesCapId::new("deer"), 1).with_priority(SpawnPriority::Low);
        let high = SpawnEvent::new(SpeciesCapId::new("wolf"), 1).with_priority(SpawnPriority::High);

        assert!(high < low);
    }

    #[test]
    fn test_spawn_event_expiration() {
        let event = SpawnEvent::new(SpeciesCapId::new("wolf"), 1).with_deadline(100);

        assert!(!event.is_expired(50));
        assert!(!event.is_expired(100));
        assert!(event.is_expired(101));
    }

    #[test]
    fn test_despawn_budget_new() {
        let budget = DespawnBudget::new(10);

        assert_eq!(budget.max_per_tick, 10);
        assert_eq!(budget.remaining(), 10);
    }

    #[test]
    fn test_despawn_budget_try_despawn() {
        let mut budget = DespawnBudget::new(10);

        let despawned = budget.try_despawn(3, 100);
        assert_eq!(despawned, 3);
        assert_eq!(budget.remaining(), 7);

        let despawned = budget.try_despawn(20, 100);
        assert_eq!(despawned, 7);
        assert_eq!(budget.remaining(), 0);
    }

    #[test]
    fn test_despawn_budget_cooldown() {
        let mut budget = DespawnBudget::new(10).with_cooldown(50);

        budget.try_despawn(5, 100);

        assert!(budget.is_on_cooldown(110));
        assert!(!budget.is_on_cooldown(150));
    }

    #[test]
    fn test_despawn_budget_protected() {
        let budget = DespawnBudget::new(10).with_protected(SpeciesCapId::new("player"));

        assert!(budget.is_protected(&SpeciesCapId::new("player")));
        assert!(!budget.is_protected(&SpeciesCapId::new("wolf")));
    }

    #[test]
    fn test_pacing_intensity_multipliers() {
        assert!(PacingIntensity::VeryLow.spawn_multiplier() < 1.0);
        assert!((PacingIntensity::Normal.spawn_multiplier() - 1.0).abs() < f32::EPSILON);
        assert!(PacingIntensity::VeryHigh.spawn_multiplier() > 1.0);

        assert!(PacingIntensity::VeryLow.cooldown_multiplier() > 1.0);
        assert!((PacingIntensity::Normal.cooldown_multiplier() - 1.0).abs() < f32::EPSILON);
        assert!(PacingIntensity::VeryHigh.cooldown_multiplier() < 1.0);
    }

    #[test]
    fn test_pacing_intensity_from_normalized() {
        assert_eq!(
            PacingIntensity::from_normalized(0.0),
            PacingIntensity::VeryLow
        );
        assert_eq!(
            PacingIntensity::from_normalized(0.5),
            PacingIntensity::Normal
        );
        assert_eq!(
            PacingIntensity::from_normalized(1.0),
            PacingIntensity::VeryHigh
        );
    }

    #[test]
    fn test_pacing_profile_new() {
        let profile = PacingProfile::new(PacingIntensity::High);

        assert_eq!(profile.intensity(), PacingIntensity::High);
        assert_eq!(profile.target(), PacingIntensity::High);
        assert!(!profile.is_ramping());
    }

    #[test]
    fn test_pacing_profile_ramp() {
        let mut profile = PacingProfile::new(PacingIntensity::Low);
        profile.ramp_to(PacingIntensity::High, 100);

        assert!(profile.is_ramping());
        assert_eq!(profile.target(), PacingIntensity::High);

        for _ in 0..100 {
            profile.tick();
        }

        assert_eq!(profile.intensity(), PacingIntensity::High);
        assert!(!profile.is_ramping());
    }

    #[test]
    fn test_pacing_profile_bounds() {
        let mut profile = PacingProfile::new(PacingIntensity::Normal)
            .with_bounds(PacingIntensity::Low, PacingIntensity::High);

        profile.set_intensity(PacingIntensity::VeryLow);
        assert_eq!(profile.intensity(), PacingIntensity::Low);

        profile.set_intensity(PacingIntensity::VeryHigh);
        assert_eq!(profile.intensity(), PacingIntensity::High);
    }

    #[test]
    fn test_spawn_budget_new() {
        let budget = SpawnBudget::new(5, 20);

        assert_eq!(budget.base_budget, 5);
        assert_eq!(budget.max_budget, 20);
        assert_eq!(budget.available(), 5);
    }

    #[test]
    fn test_spawn_budget_try_spawn() {
        let mut budget = SpawnBudget::new(5, 20);
        let species = SpeciesCapId::new("wolf");

        let spawned = budget.try_spawn(&species, 3);
        assert_eq!(spawned, 3);
        assert_eq!(budget.available(), 2);
    }

    #[test]
    fn test_spawn_budget_species_cooldown() {
        let mut budget = SpawnBudget::new(10, 20).with_species_cooldown(50);
        let species = SpeciesCapId::new("wolf");

        budget.try_spawn(&species, 1);

        assert!(budget.is_species_on_cooldown(&species));
        assert_eq!(budget.try_spawn(&species, 1), 0);

        for _ in 0..60 {
            budget.tick();
        }

        assert!(!budget.is_species_on_cooldown(&species));
    }

    #[test]
    fn test_spawn_budget_queue() {
        let mut budget = SpawnBudget::new(5, 20);

        budget.queue_spawn(SpawnEvent::new(SpeciesCapId::new("wolf"), 3));
        budget.queue_spawn(SpawnEvent::new(SpeciesCapId::new("deer"), 2));

        assert_eq!(budget.queue_len(), 2);
    }

    #[test]
    fn test_spawn_budget_process_queue() {
        let mut budget = SpawnBudget::new(5, 20).with_species_cooldown(0);

        budget.queue_spawn(
            SpawnEvent::new(SpeciesCapId::new("wolf"), 3).with_priority(SpawnPriority::High),
        );
        budget.queue_spawn(
            SpawnEvent::new(SpeciesCapId::new("deer"), 4).with_priority(SpawnPriority::Normal),
        );

        let spawned = budget.process_queue();

        assert_eq!(spawned.len(), 2);
        assert_eq!(spawned[0].species.as_str(), "wolf");
        assert_eq!(spawned[0].count, 3);
        assert_eq!(spawned[1].species.as_str(), "deer");
        assert_eq!(spawned[1].count, 2);
    }

    #[test]
    fn test_spawn_budget_tick() {
        let mut budget = SpawnBudget::new(5, 20);
        budget.try_spawn(&SpeciesCapId::new("wolf"), 5);
        assert_eq!(budget.available(), 0);

        budget.tick();
        assert_eq!(budget.available(), 5);
    }

    #[test]
    fn test_spawn_budget_pacing_affects_budget() {
        let mut budget =
            SpawnBudget::new(10, 20).with_pacing(PacingProfile::new(PacingIntensity::VeryHigh));

        budget.tick();

        assert!(budget.available() > 10);
    }

    #[test]
    fn test_spawn_budget_deterministic_queue_order() {
        let mut budget = SpawnBudget::new(100, 100);

        budget.queue_spawn(
            SpawnEvent::new(SpeciesCapId::new("c"), 1).with_priority(SpawnPriority::Normal),
        );
        budget.queue_spawn(
            SpawnEvent::new(SpeciesCapId::new("a"), 1).with_priority(SpawnPriority::High),
        );
        budget.queue_spawn(
            SpawnEvent::new(SpeciesCapId::new("b"), 1).with_priority(SpawnPriority::Normal),
        );

        let events = budget.queued_events();
        assert_eq!(events[0].species.as_str(), "a");
    }

    #[test]
    fn test_serde_spawn_event() {
        let event = SpawnEvent::new(SpeciesCapId::new("wolf"), 5)
            .with_priority(SpawnPriority::High)
            .with_deadline(1000);

        let json = serde_json::to_string(&event).unwrap();
        let restored: SpawnEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.species.as_str(), "wolf");
        assert_eq!(restored.count, 5);
        assert_eq!(restored.priority, SpawnPriority::High);
    }

    #[test]
    fn test_serde_despawn_budget() {
        let budget = DespawnBudget::new(10).with_cooldown(50);

        let json = serde_json::to_string(&budget).unwrap();
        let restored: DespawnBudget = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.max_per_tick, 10);
        assert_eq!(restored.cooldown_ticks, 50);
    }

    #[test]
    fn test_serde_spawn_budget() {
        let mut budget = SpawnBudget::new(5, 20);
        budget.queue_spawn(SpawnEvent::new(SpeciesCapId::new("wolf"), 3));

        let json = serde_json::to_string(&budget).unwrap();
        let restored: SpawnBudget = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.base_budget, 5);
        assert_eq!(restored.queue_len(), 1);
    }

    #[test]
    fn test_serde_pacing_profile() {
        let profile = PacingProfile::new(PacingIntensity::High)
            .with_bounds(PacingIntensity::Low, PacingIntensity::VeryHigh);

        let json = serde_json::to_string(&profile).unwrap();
        let restored: PacingProfile = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.intensity(), PacingIntensity::High);
        assert_eq!(restored.floor, PacingIntensity::Low);
    }
}
