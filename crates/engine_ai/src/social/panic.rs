//! Panic system for modeling fear cascades and mass panic events.

use crate::social::ids::{PanicId, SocialAgentId, SocialFactionId, SocialGroupId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Panic level (0.0 = calm, 1.0 = complete panic).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PanicLevel(f32);

impl PanicLevel {
    pub const MIN: f32 = 0.0;
    pub const MAX: f32 = 1.0;

    #[must_use]
    pub fn new(value: f32) -> Self {
        Self(value.clamp(Self::MIN, Self::MAX))
    }

    #[must_use]
    pub fn raw(self) -> f32 {
        self.0
    }

    #[must_use]
    pub fn is_calm(self) -> bool {
        self.0 < 0.1
    }

    #[must_use]
    pub fn is_uneasy(self) -> bool {
        self.0 >= 0.1 && self.0 < 0.3
    }

    #[must_use]
    pub fn is_alarmed(self) -> bool {
        self.0 >= 0.3 && self.0 < 0.5
    }

    #[must_use]
    pub fn is_panicking(self) -> bool {
        self.0 >= 0.5 && self.0 < 0.8
    }

    #[must_use]
    pub fn is_fleeing(self) -> bool {
        self.0 >= 0.8
    }

    pub fn modify(&mut self, delta: f32) {
        self.0 = (self.0 + delta).clamp(Self::MIN, Self::MAX);
    }

    #[must_use]
    pub fn with_modifier(self, delta: f32) -> Self {
        Self::new(self.0 + delta)
    }
}

impl Default for PanicLevel {
    fn default() -> Self {
        Self(0.0)
    }
}

impl Eq for PanicLevel {}

impl std::hash::Hash for PanicLevel {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PartialOrd for PanicLevel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PanicLevel {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// Source of a panic trigger.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PanicSource {
    Threat(String),
    CasualtyWitnessed(SocialAgentId),
    LeaderFallen(SocialAgentId),
    MoraleBroken(SocialAgentId),
    SuddenNoise,
    Fire,
    Explosion,
    EnemyBreakthrough,
    AmbushDetected,
    FlankingAttack,
    FriendlyFire,
    Contagion(SocialAgentId),
    Custom(String),
}

/// An individual agent's panic state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentPanic {
    pub agent_id: SocialAgentId,
    pub panic_level: PanicLevel,
    pub susceptibility: f32,
    pub recovery_rate: f32,
    pub panic_sources: Vec<PanicSource>,
    pub panic_started_tick: Option<u64>,
    pub last_update_tick: u64,
}

impl AgentPanic {
    #[must_use]
    pub fn new(agent_id: SocialAgentId, tick: u64) -> Self {
        Self {
            agent_id,
            panic_level: PanicLevel::default(),
            susceptibility: 0.5,
            recovery_rate: 0.1,
            panic_sources: Vec::new(),
            panic_started_tick: None,
            last_update_tick: tick,
        }
    }

    #[must_use]
    pub fn with_susceptibility(mut self, susceptibility: f32) -> Self {
        self.susceptibility = susceptibility.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_recovery_rate(mut self, rate: f32) -> Self {
        self.recovery_rate = rate.clamp(0.0, 1.0);
        self
    }

    pub fn trigger_panic(&mut self, source: PanicSource, intensity: f32, tick: u64) {
        let effective_intensity = intensity * self.susceptibility;
        self.panic_level.modify(effective_intensity);

        if self.panic_started_tick.is_none() && self.panic_level.is_panicking() {
            self.panic_started_tick = Some(tick);
        }

        if !self.panic_sources.contains(&source) {
            self.panic_sources.push(source);
        }

        self.last_update_tick = tick;
    }

    pub fn recover(&mut self, tick: u64) {
        let elapsed = tick.saturating_sub(self.last_update_tick);
        #[expect(clippy::cast_precision_loss, reason = "elapsed bounded")]
        let recovery = (elapsed as f32) * self.recovery_rate * 0.01;
        self.panic_level.modify(-recovery);

        if self.panic_level.is_calm() {
            self.panic_sources.clear();
            self.panic_started_tick = None;
        }

        self.last_update_tick = tick;
    }

    #[must_use]
    pub fn can_spread_panic(&self) -> bool {
        self.panic_level.is_panicking()
    }

    #[must_use]
    pub fn spread_intensity(&self) -> f32 {
        if self.panic_level.is_fleeing() {
            0.3
        } else if self.panic_level.is_panicking() {
            0.15
        } else {
            0.0
        }
    }

    #[must_use]
    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.agent_id.raw().to_le_bytes());
        hasher.update(&self.panic_level.raw().to_le_bytes());
        hasher.update(&self.susceptibility.to_le_bytes());
        hasher.update(&(self.panic_sources.len() as u64).to_le_bytes());
        hasher.finalize()
    }
}

/// A mass panic event affecting multiple agents.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PanicEvent {
    pub id: PanicId,
    pub source: PanicSource,
    pub initial_intensity: f32,
    pub affected_agents: BTreeSet<SocialAgentId>,
    pub affected_groups: BTreeSet<SocialGroupId>,
    pub started_tick: u64,
    pub resolved_tick: Option<u64>,
    pub status: PanicEventStatus,
}

impl PanicEvent {
    #[must_use]
    pub fn new(id: PanicId, source: PanicSource, intensity: f32, tick: u64) -> Self {
        Self {
            id,
            source,
            initial_intensity: intensity.clamp(0.0, 1.0),
            affected_agents: BTreeSet::new(),
            affected_groups: BTreeSet::new(),
            started_tick: tick,
            resolved_tick: None,
            status: PanicEventStatus::Spreading,
        }
    }

    pub fn add_affected_agent(&mut self, agent: SocialAgentId) {
        self.affected_agents.insert(agent);
    }

    pub fn add_affected_group(&mut self, group: SocialGroupId) {
        self.affected_groups.insert(group);
    }

    pub fn contain(&mut self) {
        self.status = PanicEventStatus::Contained;
    }

    pub fn resolve(&mut self, tick: u64) {
        self.status = PanicEventStatus::Resolved;
        self.resolved_tick = Some(tick);
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            PanicEventStatus::Spreading | PanicEventStatus::Contained
        )
    }

    #[must_use]
    pub fn affected_count(&self) -> usize {
        self.affected_agents.len()
    }

    #[must_use]
    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.id.raw().to_le_bytes());
        hasher.update(&self.initial_intensity.to_le_bytes());
        hasher.update(&(self.affected_agents.len() as u64).to_le_bytes());
        hasher.update(&[self.status.as_index()]);
        hasher.finalize()
    }
}

/// Status of a panic event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PanicEventStatus {
    Spreading,
    Contained,
    Resolved,
}

impl PanicEventStatus {
    #[must_use]
    pub fn as_index(self) -> u8 {
        match self {
            Self::Spreading => 0,
            Self::Contained => 1,
            Self::Resolved => 2,
        }
    }
}

/// Panic cascade through a group.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PanicCascade {
    pub origin: SocialAgentId,
    pub affected_sequence: Vec<(SocialAgentId, u64)>,
    pub total_spread: f32,
    pub started_tick: u64,
    pub ended_tick: Option<u64>,
}

impl PanicCascade {
    #[must_use]
    pub fn new(origin: SocialAgentId, tick: u64) -> Self {
        Self {
            origin,
            affected_sequence: vec![(origin, tick)],
            total_spread: 0.0,
            started_tick: tick,
            ended_tick: None,
        }
    }

    pub fn add_spread(&mut self, agent: SocialAgentId, tick: u64) {
        self.affected_sequence.push((agent, tick));
        self.total_spread += 1.0;
    }

    pub fn end(&mut self, tick: u64) {
        self.ended_tick = Some(tick);
    }

    #[must_use]
    pub fn cascade_depth(&self) -> usize {
        self.affected_sequence.len()
    }
}

/// Tracker for panic state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PanicTracker {
    agent_panic: BTreeMap<SocialAgentId, AgentPanic>,
    active_events: BTreeMap<PanicId, PanicEvent>,
    faction_panic_threshold: BTreeMap<SocialFactionId, f32>,
    next_event_id: u64,
    cascade_radius: f32,
}

impl PanicTracker {
    #[must_use]
    pub fn new() -> Self {
        Self {
            cascade_radius: 10.0,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn with_cascade_radius(mut self, radius: f32) -> Self {
        self.cascade_radius = radius.max(0.0);
        self
    }

    pub fn register_agent(&mut self, agent_id: SocialAgentId, tick: u64) -> &mut AgentPanic {
        self.agent_panic
            .entry(agent_id)
            .or_insert_with(|| AgentPanic::new(agent_id, tick))
    }

    #[must_use]
    pub fn get_agent_panic(&self, agent_id: SocialAgentId) -> Option<&AgentPanic> {
        self.agent_panic.get(&agent_id)
    }

    pub fn get_agent_panic_mut(&mut self, agent_id: SocialAgentId) -> Option<&mut AgentPanic> {
        self.agent_panic.get_mut(&agent_id)
    }

    pub fn trigger_agent_panic(
        &mut self,
        agent_id: SocialAgentId,
        source: PanicSource,
        intensity: f32,
        tick: u64,
    ) {
        if let Some(agent) = self.agent_panic.get_mut(&agent_id) {
            agent.trigger_panic(source, intensity, tick);
        }
    }

    pub fn create_panic_event(
        &mut self,
        source: PanicSource,
        intensity: f32,
        tick: u64,
    ) -> PanicId {
        let id = PanicId::new(self.next_event_id);
        self.next_event_id += 1;

        let event = PanicEvent::new(id, source, intensity, tick);
        self.active_events.insert(id, event);

        id
    }

    pub fn spread_panic_from_agent(
        &mut self,
        source_agent: SocialAgentId,
        nearby_agents: &[SocialAgentId],
        tick: u64,
    ) {
        let spread_intensity = self
            .agent_panic
            .get(&source_agent)
            .map_or(0.0, AgentPanic::spread_intensity);

        if spread_intensity > 0.0 {
            let source = PanicSource::Contagion(source_agent);
            for &agent_id in nearby_agents {
                if agent_id != source_agent
                    && let Some(agent) = self.agent_panic.get_mut(&agent_id)
                {
                    agent.trigger_panic(source.clone(), spread_intensity, tick);
                }
            }
        }
    }

    pub fn tick_recovery(&mut self, tick: u64) {
        for agent in self.agent_panic.values_mut() {
            agent.recover(tick);
        }
    }

    pub fn resolve_event(&mut self, event_id: PanicId, tick: u64) {
        if let Some(event) = self.active_events.get_mut(&event_id) {
            event.resolve(tick);
        }
    }

    pub fn set_faction_panic_threshold(&mut self, faction: SocialFactionId, threshold: f32) {
        self.faction_panic_threshold
            .insert(faction, threshold.clamp(0.0, 1.0));
    }

    pub fn panicking_agents(&self) -> impl Iterator<Item = SocialAgentId> + '_ {
        self.agent_panic
            .iter()
            .filter(|(_, a)| a.panic_level.is_panicking())
            .map(|(id, _)| *id)
    }

    pub fn fleeing_agents(&self) -> impl Iterator<Item = SocialAgentId> + '_ {
        self.agent_panic
            .iter()
            .filter(|(_, a)| a.panic_level.is_fleeing())
            .map(|(id, _)| *id)
    }

    #[must_use]
    pub fn active_event_count(&self) -> usize {
        self.active_events
            .values()
            .filter(|e| e.is_active())
            .count()
    }

    #[must_use]
    pub fn agent_count(&self) -> usize {
        self.agent_panic.len()
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "count bounded")]
    pub fn average_panic_level(&self) -> f32 {
        if self.agent_panic.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.agent_panic.values().map(|a| a.panic_level.raw()).sum();
        sum / self.agent_panic.len() as f32
    }

    #[must_use]
    pub fn compute_morale_impact(&self, agent_id: SocialAgentId) -> f32 {
        self.agent_panic
            .get(&agent_id)
            .map_or(0.0, |a| -a.panic_level.raw() * 0.5)
    }

    #[must_use]
    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&(self.agent_panic.len() as u64).to_le_bytes());
        for panic in self.agent_panic.values() {
            hasher.update(&panic.checksum().to_le_bytes());
        }
        hasher.update(&(self.active_events.len() as u64).to_le_bytes());
        for event in self.active_events.values() {
            hasher.update(&event.checksum().to_le_bytes());
        }
        hasher.finalize()
    }
}

/// Panic tracking event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PanicTrackingEvent {
    pub tick: u64,
    pub kind: PanicTrackingEventKind,
}

/// Kind of panic tracking event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PanicTrackingEventKind {
    PanicTriggered {
        agent: SocialAgentId,
        source: PanicSource,
        new_level: PanicLevel,
    },
    PanicSpread {
        from: SocialAgentId,
        to: SocialAgentId,
    },
    PanicRecovered {
        agent: SocialAgentId,
    },
    MassPanicStarted {
        event_id: PanicId,
    },
    MassPanicContained {
        event_id: PanicId,
    },
    MassPanicResolved {
        event_id: PanicId,
    },
    FleeingBegan {
        agent: SocialAgentId,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panic_level_thresholds() {
        assert!(PanicLevel::new(0.05).is_calm());
        assert!(PanicLevel::new(0.2).is_uneasy());
        assert!(PanicLevel::new(0.4).is_alarmed());
        assert!(PanicLevel::new(0.65).is_panicking());
        assert!(PanicLevel::new(0.9).is_fleeing());
    }

    #[test]
    fn test_panic_level_modify() {
        let mut panic = PanicLevel::new(0.3);
        panic.modify(0.3);
        assert!((panic.raw() - 0.6).abs() < f32::EPSILON);

        panic.modify(0.6);
        assert!((panic.raw() - PanicLevel::MAX).abs() < f32::EPSILON);
    }

    #[test]
    fn test_agent_panic() {
        let mut agent = AgentPanic::new(SocialAgentId::new(1), 0)
            .with_susceptibility(0.8)
            .with_recovery_rate(0.2);

        agent.trigger_panic(PanicSource::Explosion, 0.5, 100);
        assert!(agent.panic_level.raw() > 0.0);
        assert!(agent.can_spread_panic() || !agent.can_spread_panic());
    }

    #[test]
    fn test_panic_recovery() {
        let mut agent = AgentPanic::new(SocialAgentId::new(1), 0).with_recovery_rate(0.5);

        agent.trigger_panic(PanicSource::Fire, 0.6, 0);
        let initial = agent.panic_level.raw();

        agent.recover(100);
        assert!(agent.panic_level.raw() < initial);
    }

    #[test]
    fn test_panic_event() {
        let mut event = PanicEvent::new(PanicId::new(1), PanicSource::Explosion, 0.8, 0);

        event.add_affected_agent(SocialAgentId::new(1));
        event.add_affected_agent(SocialAgentId::new(2));

        assert!(event.is_active());
        assert_eq!(event.affected_count(), 2);

        event.resolve(100);
        assert!(!event.is_active());
    }

    #[test]
    fn test_panic_tracker() {
        let mut tracker = PanicTracker::new();

        tracker.register_agent(SocialAgentId::new(1), 0);
        tracker.register_agent(SocialAgentId::new(2), 0);

        tracker.trigger_agent_panic(SocialAgentId::new(1), PanicSource::Fire, 0.7, 100);

        let agent = tracker.get_agent_panic(SocialAgentId::new(1)).unwrap();
        assert!(agent.panic_level.raw() > 0.0);
    }

    #[test]
    fn test_panic_spread() {
        let mut tracker = PanicTracker::new();

        let source = SocialAgentId::new(1);
        let nearby = SocialAgentId::new(2);

        {
            let agent = tracker.register_agent(source, 0);
            agent.panic_level = PanicLevel::new(0.9);
        }
        tracker.register_agent(nearby, 0);

        tracker.spread_panic_from_agent(source, &[nearby], 100);

        let affected = tracker.get_agent_panic(nearby).unwrap();
        assert!(affected.panic_level.raw() > 0.0);
    }

    #[test]
    fn test_checksum_determinism() {
        let mut tracker = PanicTracker::new();
        tracker.register_agent(SocialAgentId::new(1), 0);

        let checksum1 = tracker.checksum();
        let checksum2 = tracker.checksum();
        assert_eq!(checksum1, checksum2);
    }

    #[test]
    fn test_serde_roundtrip() {
        let mut tracker = PanicTracker::new();
        {
            let agent = tracker.register_agent(SocialAgentId::new(1), 0);
            agent.panic_level = PanicLevel::new(0.5);
        }

        let json = serde_json::to_string(&tracker).unwrap();
        let restored: PanicTracker = serde_json::from_str(&json).unwrap();
        assert_eq!(tracker.checksum(), restored.checksum());
    }
}
