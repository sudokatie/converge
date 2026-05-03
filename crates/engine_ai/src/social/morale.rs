//! Morale system for tracking group and individual morale states.

use crate::social::ids::{SocialAgentId, SocialFactionId, SocialGroupId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Morale level (0.0 = broken, 1.0 = excellent).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MoraleLevel(f32);

impl MoraleLevel {
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
    pub fn is_broken(self) -> bool {
        self.0 < 0.1
    }

    #[must_use]
    pub fn is_critical(self) -> bool {
        self.0 < 0.25
    }

    #[must_use]
    pub fn is_low(self) -> bool {
        self.0 < 0.4
    }

    #[must_use]
    pub fn is_stable(self) -> bool {
        self.0 >= 0.4 && self.0 < 0.7
    }

    #[must_use]
    pub fn is_high(self) -> bool {
        self.0 >= 0.7
    }

    #[must_use]
    pub fn is_excellent(self) -> bool {
        self.0 >= 0.9
    }

    pub fn modify(&mut self, delta: f32) {
        self.0 = (self.0 + delta).clamp(Self::MIN, Self::MAX);
    }

    #[must_use]
    pub fn with_modifier(self, delta: f32) -> Self {
        Self::new(self.0 + delta)
    }
}

impl Default for MoraleLevel {
    fn default() -> Self {
        Self(0.5)
    }
}

impl Eq for MoraleLevel {}

impl std::hash::Hash for MoraleLevel {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PartialOrd for MoraleLevel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MoraleLevel {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// Factors that affect morale.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MoraleFactors {
    pub leadership_bonus: f32,
    pub recent_victory_bonus: f32,
    pub recent_loss_penalty: f32,
    pub resource_scarcity_penalty: f32,
    pub fear_penalty: f32,
    pub fatigue_penalty: f32,
    pub home_territory_bonus: f32,
    pub ally_presence_bonus: f32,
    pub enemy_superiority_penalty: f32,
    pub casualties_penalty: f32,
}

impl MoraleFactors {
    #[must_use]
    pub fn net_modifier(&self) -> f32 {
        self.leadership_bonus
            + self.recent_victory_bonus
            + self.home_territory_bonus
            + self.ally_presence_bonus
            - self.recent_loss_penalty
            - self.resource_scarcity_penalty
            - self.fear_penalty
            - self.fatigue_penalty
            - self.enemy_superiority_penalty
            - self.casualties_penalty
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

impl Eq for MoraleFactors {}

impl std::hash::Hash for MoraleFactors {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.leadership_bonus.to_bits().hash(state);
        self.recent_victory_bonus.to_bits().hash(state);
        self.recent_loss_penalty.to_bits().hash(state);
        self.resource_scarcity_penalty.to_bits().hash(state);
        self.fear_penalty.to_bits().hash(state);
        self.fatigue_penalty.to_bits().hash(state);
        self.home_territory_bonus.to_bits().hash(state);
        self.ally_presence_bonus.to_bits().hash(state);
        self.enemy_superiority_penalty.to_bits().hash(state);
        self.casualties_penalty.to_bits().hash(state);
    }
}

/// Morale state for an individual agent.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentMorale {
    pub agent_id: SocialAgentId,
    pub base_morale: MoraleLevel,
    pub current_morale: MoraleLevel,
    pub factors: MoraleFactors,
    pub resilience: f32,
    pub last_update_tick: u64,
}

impl AgentMorale {
    #[must_use]
    pub fn new(agent_id: SocialAgentId, tick: u64) -> Self {
        Self {
            agent_id,
            base_morale: MoraleLevel::default(),
            current_morale: MoraleLevel::default(),
            factors: MoraleFactors::default(),
            resilience: 0.5,
            last_update_tick: tick,
        }
    }

    #[must_use]
    pub fn with_base(mut self, morale: f32) -> Self {
        self.base_morale = MoraleLevel::new(morale);
        self.current_morale = self.base_morale;
        self
    }

    #[must_use]
    pub fn with_resilience(mut self, resilience: f32) -> Self {
        self.resilience = resilience.clamp(0.0, 1.0);
        self
    }

    pub fn apply_factors(&mut self) {
        let modifier = self.factors.net_modifier();
        self.current_morale = self.base_morale.with_modifier(modifier);
    }

    pub fn recover(&mut self, rate: f32, tick: u64) {
        let elapsed = tick.saturating_sub(self.last_update_tick);
        #[expect(clippy::cast_precision_loss, reason = "elapsed bounded")]
        let recovery = (elapsed as f32) * rate * self.resilience * 0.001;
        self.base_morale.modify(recovery);
        self.apply_factors();
        self.last_update_tick = tick;
    }

    #[must_use]
    pub fn effective_morale(&self) -> MoraleLevel {
        self.current_morale
    }

    #[must_use]
    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.agent_id.raw().to_le_bytes());
        hasher.update(&self.base_morale.raw().to_le_bytes());
        hasher.update(&self.current_morale.raw().to_le_bytes());
        hasher.update(&self.resilience.to_le_bytes());
        hasher.finalize()
    }
}

/// Morale state for a group.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroupMorale {
    pub group_id: SocialGroupId,
    pub collective_morale: MoraleLevel,
    pub cohesion: f32,
    pub member_count: u32,
    pub last_update_tick: u64,
}

impl GroupMorale {
    #[must_use]
    pub fn new(group_id: SocialGroupId, tick: u64) -> Self {
        Self {
            group_id,
            collective_morale: MoraleLevel::default(),
            cohesion: 0.5,
            member_count: 0,
            last_update_tick: tick,
        }
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "count bounded")]
    pub fn compute_from_members(&self, member_morales: &[MoraleLevel]) -> MoraleLevel {
        if member_morales.is_empty() {
            return MoraleLevel::default();
        }

        let sum: f32 = member_morales.iter().map(|m| m.raw()).sum();
        let avg = sum / member_morales.len() as f32;
        let cohesion_bonus = (self.cohesion - 0.5) * 0.1;
        MoraleLevel::new(avg + cohesion_bonus)
    }

    pub fn update_from_members(&mut self, member_morales: &[MoraleLevel], tick: u64) {
        self.collective_morale = self.compute_from_members(member_morales);
        #[expect(clippy::cast_possible_truncation, reason = "member count bounded")]
        {
            self.member_count = member_morales.len() as u32;
        }
        self.last_update_tick = tick;
    }

    #[must_use]
    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.group_id.raw().to_le_bytes());
        hasher.update(&self.collective_morale.raw().to_le_bytes());
        hasher.update(&self.cohesion.to_le_bytes());
        hasher.update(&self.member_count.to_le_bytes());
        hasher.finalize()
    }
}

/// Morale event for tracking.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MoraleEvent {
    pub tick: u64,
    pub kind: MoraleEventKind,
}

/// Kind of morale event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MoraleEventKind {
    MoraleDropped {
        agent: SocialAgentId,
        old_level: MoraleLevel,
        new_level: MoraleLevel,
    },
    MoraleBoosted {
        agent: SocialAgentId,
        old_level: MoraleLevel,
        new_level: MoraleLevel,
    },
    MoraleBroken {
        agent: SocialAgentId,
    },
    MoraleRecovered {
        agent: SocialAgentId,
    },
    GroupMoraleChanged {
        group: SocialGroupId,
        old_level: MoraleLevel,
        new_level: MoraleLevel,
    },
    CohesionChanged {
        group: SocialGroupId,
        old_cohesion: f32,
        new_cohesion: f32,
    },
}

/// Tracker for morale state across agents and groups.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MoraleTracker {
    agent_morale: BTreeMap<SocialAgentId, AgentMorale>,
    group_morale: BTreeMap<SocialGroupId, GroupMorale>,
    faction_base_morale: BTreeMap<SocialFactionId, MoraleLevel>,
    recovery_rate: f32,
}

impl MoraleTracker {
    #[must_use]
    pub fn new() -> Self {
        Self {
            recovery_rate: 0.1,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn with_recovery_rate(mut self, rate: f32) -> Self {
        self.recovery_rate = rate.clamp(0.0, 1.0);
        self
    }

    pub fn register_agent(&mut self, agent_id: SocialAgentId, tick: u64) -> &mut AgentMorale {
        self.agent_morale
            .entry(agent_id)
            .or_insert_with(|| AgentMorale::new(agent_id, tick))
    }

    pub fn register_group(&mut self, group_id: SocialGroupId, tick: u64) -> &mut GroupMorale {
        self.group_morale
            .entry(group_id)
            .or_insert_with(|| GroupMorale::new(group_id, tick))
    }

    #[must_use]
    pub fn get_agent_morale(&self, agent_id: SocialAgentId) -> Option<&AgentMorale> {
        self.agent_morale.get(&agent_id)
    }

    pub fn get_agent_morale_mut(&mut self, agent_id: SocialAgentId) -> Option<&mut AgentMorale> {
        self.agent_morale.get_mut(&agent_id)
    }

    #[must_use]
    pub fn get_group_morale(&self, group_id: SocialGroupId) -> Option<&GroupMorale> {
        self.group_morale.get(&group_id)
    }

    pub fn get_group_morale_mut(&mut self, group_id: SocialGroupId) -> Option<&mut GroupMorale> {
        self.group_morale.get_mut(&group_id)
    }

    pub fn set_faction_base_morale(&mut self, faction: SocialFactionId, morale: MoraleLevel) {
        self.faction_base_morale.insert(faction, morale);
    }

    #[must_use]
    pub fn get_faction_base_morale(&self, faction: &SocialFactionId) -> MoraleLevel {
        self.faction_base_morale
            .get(faction)
            .copied()
            .unwrap_or_default()
    }

    pub fn apply_morale_shock(&mut self, agent_id: SocialAgentId, shock: f32) {
        if let Some(morale) = self.agent_morale.get_mut(&agent_id) {
            morale.base_morale.modify(-shock.abs());
            morale.apply_factors();
        }
    }

    pub fn apply_morale_boost(&mut self, agent_id: SocialAgentId, boost: f32) {
        if let Some(morale) = self.agent_morale.get_mut(&agent_id) {
            morale.base_morale.modify(boost.abs());
            morale.apply_factors();
        }
    }

    pub fn tick_recovery(&mut self, tick: u64) {
        for morale in self.agent_morale.values_mut() {
            morale.recover(self.recovery_rate, tick);
        }
    }

    pub fn agents_with_broken_morale(&self) -> impl Iterator<Item = SocialAgentId> + '_ {
        self.agent_morale
            .iter()
            .filter(|(_, m)| m.effective_morale().is_broken())
            .map(|(id, _)| *id)
    }

    pub fn agents_with_low_morale(&self) -> impl Iterator<Item = SocialAgentId> + '_ {
        self.agent_morale
            .iter()
            .filter(|(_, m)| m.effective_morale().is_low())
            .map(|(id, _)| *id)
    }

    #[must_use]
    pub fn agent_count(&self) -> usize {
        self.agent_morale.len()
    }

    #[must_use]
    pub fn group_count(&self) -> usize {
        self.group_morale.len()
    }

    #[must_use]
    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&(self.agent_morale.len() as u64).to_le_bytes());
        for morale in self.agent_morale.values() {
            hasher.update(&morale.checksum().to_le_bytes());
        }
        hasher.update(&(self.group_morale.len() as u64).to_le_bytes());
        for morale in self.group_morale.values() {
            hasher.update(&morale.checksum().to_le_bytes());
        }
        hasher.finalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_morale_level_thresholds() {
        assert!(MoraleLevel::new(0.05).is_broken());
        assert!(MoraleLevel::new(0.2).is_critical());
        assert!(MoraleLevel::new(0.35).is_low());
        assert!(MoraleLevel::new(0.5).is_stable());
        assert!(MoraleLevel::new(0.75).is_high());
        assert!(MoraleLevel::new(0.95).is_excellent());
    }

    #[test]
    fn test_morale_level_modify() {
        let mut morale = MoraleLevel::new(0.5);
        morale.modify(0.2);
        assert!((morale.raw() - 0.7).abs() < f32::EPSILON);

        morale.modify(-0.9);
        assert!(morale.raw() >= MoraleLevel::MIN);
    }

    #[test]
    fn test_morale_factors() {
        let factors = MoraleFactors {
            leadership_bonus: 0.1,
            recent_victory_bonus: 0.05,
            recent_loss_penalty: 0.02,
            ..Default::default()
        };

        let net = factors.net_modifier();
        assert!((net - 0.13).abs() < f32::EPSILON);
    }

    #[test]
    fn test_agent_morale() {
        let mut morale = AgentMorale::new(SocialAgentId::new(1), 0)
            .with_base(0.6)
            .with_resilience(0.7);

        assert!((morale.base_morale.raw() - 0.6).abs() < f32::EPSILON);
        assert!((morale.resilience - 0.7).abs() < f32::EPSILON);

        morale.factors.leadership_bonus = 0.1;
        morale.apply_factors();
        assert!((morale.effective_morale().raw() - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn test_group_morale() {
        let mut group = GroupMorale::new(SocialGroupId::new(1), 0);
        group.cohesion = 0.6;

        let members = vec![
            MoraleLevel::new(0.5),
            MoraleLevel::new(0.6),
            MoraleLevel::new(0.7),
        ];
        group.update_from_members(&members, 100);

        assert!(group.collective_morale.raw() > 0.5);
        assert_eq!(group.member_count, 3);
    }

    #[test]
    fn test_morale_tracker() {
        let mut tracker = MoraleTracker::new();
        let agent = SocialAgentId::new(1);

        tracker.register_agent(agent, 0);
        assert_eq!(tracker.agent_count(), 1);

        tracker.apply_morale_shock(agent, 0.3);
        let morale = tracker.get_agent_morale(agent).unwrap();
        assert!(morale.effective_morale().raw() < 0.5);
    }

    #[test]
    fn test_checksum_determinism() {
        let mut tracker = MoraleTracker::new();
        tracker.register_agent(SocialAgentId::new(1), 0);

        let checksum1 = tracker.checksum();
        let checksum2 = tracker.checksum();
        assert_eq!(checksum1, checksum2);
    }

    #[test]
    fn test_serde_roundtrip() {
        let mut tracker = MoraleTracker::new();
        let agent = tracker.register_agent(SocialAgentId::new(1), 0);
        agent.base_morale = MoraleLevel::new(0.7);

        let json = serde_json::to_string(&tracker).unwrap();
        let restored: MoraleTracker = serde_json::from_str(&json).unwrap();
        assert_eq!(tracker.checksum(), restored.checksum());
    }
}
