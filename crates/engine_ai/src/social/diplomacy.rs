//! Diplomacy system for faction relations, alliances, and treaties.

use crate::social::ids::SocialFactionId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Diplomatic stance between factions.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum DiplomaticStance {
    War,
    Hostile,
    Unfriendly,
    #[default]
    Neutral,
    Cordial,
    Friendly,
    Allied,
}

impl DiplomaticStance {
    #[must_use]
    pub fn allows_passage(&self) -> bool {
        matches!(
            self,
            Self::Neutral | Self::Cordial | Self::Friendly | Self::Allied
        )
    }

    #[must_use]
    pub fn allows_trade(&self) -> bool {
        matches!(self, Self::Cordial | Self::Friendly | Self::Allied)
    }

    #[must_use]
    pub fn allows_combat(&self) -> bool {
        matches!(self, Self::War | Self::Hostile)
    }

    #[must_use]
    pub fn is_positive(&self) -> bool {
        matches!(self, Self::Cordial | Self::Friendly | Self::Allied)
    }

    #[must_use]
    pub fn is_negative(&self) -> bool {
        matches!(self, Self::War | Self::Hostile | Self::Unfriendly)
    }

    #[must_use]
    pub fn as_value(&self) -> i32 {
        match self {
            Self::War => -3,
            Self::Hostile => -2,
            Self::Unfriendly => -1,
            Self::Neutral => 0,
            Self::Cordial => 1,
            Self::Friendly => 2,
            Self::Allied => 3,
        }
    }

    #[must_use]
    pub fn from_value(value: i32) -> Self {
        match value {
            ..=-3 => Self::War,
            -2 => Self::Hostile,
            -1 => Self::Unfriendly,
            0 => Self::Neutral,
            1 => Self::Cordial,
            2 => Self::Friendly,
            3.. => Self::Allied,
        }
    }
}

/// A relation between two factions.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DiplomaticRelation {
    pub faction_a: SocialFactionId,
    pub faction_b: SocialFactionId,
    pub stance: DiplomaticStance,
    pub trust: TrustLevel,
    pub accumulated_favor: i32,
    pub last_interaction_tick: u64,
    pub active_treaties: Vec<TreatyId>,
}

impl DiplomaticRelation {
    #[must_use]
    pub fn new(faction_a: SocialFactionId, faction_b: SocialFactionId, tick: u64) -> Self {
        Self {
            faction_a,
            faction_b,
            stance: DiplomaticStance::default(),
            trust: TrustLevel::default(),
            accumulated_favor: 0,
            last_interaction_tick: tick,
            active_treaties: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_stance(mut self, stance: DiplomaticStance) -> Self {
        self.stance = stance;
        self
    }

    pub fn modify_favor(&mut self, delta: i32, tick: u64) {
        self.accumulated_favor = self.accumulated_favor.saturating_add(delta);
        self.last_interaction_tick = tick;
        self.update_stance_from_favor();
    }

    fn update_stance_from_favor(&mut self) {
        let threshold_value = self.accumulated_favor / 100;
        let current_value = self.stance.as_value();
        if threshold_value > current_value + 1 {
            self.stance = DiplomaticStance::from_value(current_value + 1);
        } else if threshold_value < current_value - 1 {
            self.stance = DiplomaticStance::from_value(current_value - 1);
        }
    }

    pub fn add_treaty(&mut self, treaty_id: TreatyId) {
        if !self.active_treaties.contains(&treaty_id) {
            self.active_treaties.push(treaty_id);
        }
    }

    pub fn remove_treaty(&mut self, treaty_id: TreatyId) {
        self.active_treaties.retain(|t| *t != treaty_id);
    }

    #[must_use]
    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(self.faction_a.as_str().as_bytes());
        hasher.update(self.faction_b.as_str().as_bytes());
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "stance value is in range -3..=3, offset to 0..=6"
        )]
        let stance_byte = (self.stance.as_value() + 3) as u8;
        hasher.update(&[stance_byte]);
        hasher.update(&self.trust.raw().to_le_bytes());
        hasher.update(&self.accumulated_favor.to_le_bytes());
        hasher.finalize()
    }
}

/// Trust level between factions (0.0 = no trust, 1.0 = complete trust).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrustLevel(f32);

impl TrustLevel {
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
    pub fn is_distrustful(self) -> bool {
        self.0 < 0.3
    }

    #[must_use]
    pub fn is_neutral(self) -> bool {
        self.0 >= 0.3 && self.0 < 0.6
    }

    #[must_use]
    pub fn is_trusting(self) -> bool {
        self.0 >= 0.6
    }

    pub fn modify(&mut self, delta: f32) {
        self.0 = (self.0 + delta).clamp(Self::MIN, Self::MAX);
    }
}

impl Default for TrustLevel {
    fn default() -> Self {
        Self(0.5)
    }
}

impl Eq for TrustLevel {}

impl std::hash::Hash for TrustLevel {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PartialOrd for TrustLevel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TrustLevel {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// Unique identifier for a treaty.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TreatyId(pub u64);

impl TreatyId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for TreatyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "treaty:{}", self.0)
    }
}

/// A treaty between factions.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Treaty {
    pub id: TreatyId,
    pub kind: TreatyKind,
    pub parties: Vec<SocialFactionId>,
    pub signed_tick: u64,
    pub expires_tick: Option<u64>,
    pub status: TreatyStatus,
}

impl Treaty {
    #[must_use]
    pub fn new(id: TreatyId, kind: TreatyKind, parties: Vec<SocialFactionId>, tick: u64) -> Self {
        Self {
            id,
            kind,
            parties,
            signed_tick: tick,
            expires_tick: None,
            status: TreatyStatus::Active,
        }
    }

    #[must_use]
    pub fn with_duration(mut self, duration: u64) -> Self {
        self.expires_tick = Some(self.signed_tick + duration);
        self
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(self.status, TreatyStatus::Active)
    }

    #[must_use]
    pub fn is_expired(&self, current_tick: u64) -> bool {
        self.expires_tick.is_some_and(|exp| current_tick >= exp)
    }

    pub fn break_treaty(&mut self, tick: u64) {
        self.status = TreatyStatus::Broken(tick);
    }

    pub fn expire(&mut self, tick: u64) {
        self.status = TreatyStatus::Expired(tick);
    }

    #[must_use]
    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.id.raw().to_le_bytes());
        hasher.update(&[self.kind.as_index()]);
        hasher.update(&(self.parties.len() as u64).to_le_bytes());
        hasher.update(&self.signed_tick.to_le_bytes());
        hasher.finalize()
    }
}

/// Kind of treaty.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TreatyKind {
    NonAggression,
    TradeAgreement,
    DefensiveAlliance,
    MilitaryAlliance,
    Ceasefire,
    Vassalage,
    Protectorate,
    ResourceSharing,
    BorderAgreement,
    Custom(String),
}

impl TreatyKind {
    #[must_use]
    pub fn as_index(&self) -> u8 {
        match self {
            Self::NonAggression => 0,
            Self::TradeAgreement => 1,
            Self::DefensiveAlliance => 2,
            Self::MilitaryAlliance => 3,
            Self::Ceasefire => 4,
            Self::Vassalage => 5,
            Self::Protectorate => 6,
            Self::ResourceSharing => 7,
            Self::BorderAgreement => 8,
            Self::Custom(_) => 9,
        }
    }

    #[must_use]
    pub fn required_trust(&self) -> TrustLevel {
        match self {
            Self::Ceasefire => TrustLevel::new(0.2),
            Self::NonAggression | Self::BorderAgreement => TrustLevel::new(0.3),
            Self::TradeAgreement | Self::ResourceSharing => TrustLevel::new(0.4),
            Self::DefensiveAlliance | Self::Protectorate => TrustLevel::new(0.6),
            Self::MilitaryAlliance | Self::Vassalage => TrustLevel::new(0.7),
            Self::Custom(_) => TrustLevel::new(0.5),
        }
    }
}

/// Status of a treaty.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TreatyStatus {
    Active,
    Expired(u64),
    Broken(u64),
    Renegotiating,
}

/// Event for diplomatic changes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DiplomacyEvent {
    pub tick: u64,
    pub kind: DiplomacyEventKind,
}

/// Kind of diplomacy event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DiplomacyEventKind {
    StanceChanged {
        faction_a: SocialFactionId,
        faction_b: SocialFactionId,
        old_stance: DiplomaticStance,
        new_stance: DiplomaticStance,
    },
    TreatySigned {
        treaty_id: TreatyId,
        parties: Vec<SocialFactionId>,
        kind: TreatyKind,
    },
    TreatyBroken {
        treaty_id: TreatyId,
        breaker: SocialFactionId,
    },
    TreatyExpired {
        treaty_id: TreatyId,
    },
    WarDeclared {
        aggressor: SocialFactionId,
        defender: SocialFactionId,
    },
    PeaceOffered {
        offerer: SocialFactionId,
        target: SocialFactionId,
    },
    FavorChanged {
        faction_a: SocialFactionId,
        faction_b: SocialFactionId,
        delta: i32,
    },
}

/// Tracker for all diplomatic state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DiplomacyTracker {
    relations: BTreeMap<(SocialFactionId, SocialFactionId), DiplomaticRelation>,
    treaties: BTreeMap<TreatyId, Treaty>,
    next_treaty_id: u64,
}

impl DiplomacyTracker {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn ordered_key(a: &SocialFactionId, b: &SocialFactionId) -> (SocialFactionId, SocialFactionId) {
        if a <= b {
            (a.clone(), b.clone())
        } else {
            (b.clone(), a.clone())
        }
    }

    pub fn get_or_create_relation(
        &mut self,
        faction_a: &SocialFactionId,
        faction_b: &SocialFactionId,
        tick: u64,
    ) -> &mut DiplomaticRelation {
        let key = Self::ordered_key(faction_a, faction_b);
        self.relations
            .entry(key.clone())
            .or_insert_with(|| DiplomaticRelation::new(key.0.clone(), key.1.clone(), tick))
    }

    #[must_use]
    pub fn get_relation(
        &self,
        faction_a: &SocialFactionId,
        faction_b: &SocialFactionId,
    ) -> Option<&DiplomaticRelation> {
        let key = Self::ordered_key(faction_a, faction_b);
        self.relations.get(&key)
    }

    #[must_use]
    pub fn get_stance(
        &self,
        faction_a: &SocialFactionId,
        faction_b: &SocialFactionId,
    ) -> DiplomaticStance {
        self.get_relation(faction_a, faction_b)
            .map_or(DiplomaticStance::default(), |r| r.stance)
    }

    pub fn set_stance(
        &mut self,
        faction_a: &SocialFactionId,
        faction_b: &SocialFactionId,
        stance: DiplomaticStance,
        tick: u64,
    ) {
        let relation = self.get_or_create_relation(faction_a, faction_b, tick);
        relation.stance = stance;
        relation.last_interaction_tick = tick;
    }

    pub fn declare_war(
        &mut self,
        aggressor: &SocialFactionId,
        defender: &SocialFactionId,
        tick: u64,
    ) {
        self.set_stance(aggressor, defender, DiplomaticStance::War, tick);
    }

    pub fn sign_treaty(
        &mut self,
        kind: TreatyKind,
        parties: &[SocialFactionId],
        tick: u64,
    ) -> TreatyId {
        let id = TreatyId::new(self.next_treaty_id);
        self.next_treaty_id += 1;

        let treaty = Treaty::new(id, kind, parties.to_vec(), tick);
        self.treaties.insert(id, treaty);

        for i in 0..parties.len() {
            for j in (i + 1)..parties.len() {
                let relation = self.get_or_create_relation(&parties[i], &parties[j], tick);
                relation.add_treaty(id);
            }
        }

        id
    }

    pub fn break_treaty(&mut self, treaty_id: TreatyId, tick: u64) {
        if let Some(treaty) = self.treaties.get_mut(&treaty_id) {
            treaty.break_treaty(tick);

            for i in 0..treaty.parties.len() {
                for j in (i + 1)..treaty.parties.len() {
                    let key = Self::ordered_key(&treaty.parties[i], &treaty.parties[j]);
                    if let Some(relation) = self.relations.get_mut(&key) {
                        relation.remove_treaty(treaty_id);
                        relation.trust.modify(-0.2);
                    }
                }
            }
        }
    }

    pub fn tick_treaties(&mut self, tick: u64) {
        let expired: Vec<TreatyId> = self
            .treaties
            .values()
            .filter(|t| t.is_active() && t.is_expired(tick))
            .map(|t| t.id)
            .collect();

        for id in expired {
            if let Some(treaty) = self.treaties.get_mut(&id) {
                treaty.expire(tick);

                for i in 0..treaty.parties.len() {
                    for j in (i + 1)..treaty.parties.len() {
                        let key = Self::ordered_key(&treaty.parties[i], &treaty.parties[j]);
                        if let Some(relation) = self.relations.get_mut(&key) {
                            relation.remove_treaty(id);
                        }
                    }
                }
            }
        }
    }

    #[must_use]
    pub fn get_treaty(&self, id: TreatyId) -> Option<&Treaty> {
        self.treaties.get(&id)
    }

    pub fn active_treaties(&self) -> impl Iterator<Item = &Treaty> {
        self.treaties.values().filter(|t| t.is_active())
    }

    pub fn treaties_for_faction(&self, faction: &SocialFactionId) -> impl Iterator<Item = &Treaty> {
        self.treaties
            .values()
            .filter(|t| t.is_active() && t.parties.contains(faction))
    }

    #[must_use]
    pub fn are_at_war(&self, faction_a: &SocialFactionId, faction_b: &SocialFactionId) -> bool {
        self.get_stance(faction_a, faction_b) == DiplomaticStance::War
    }

    #[must_use]
    pub fn are_allied(&self, faction_a: &SocialFactionId, faction_b: &SocialFactionId) -> bool {
        self.get_stance(faction_a, faction_b) == DiplomaticStance::Allied
    }

    #[must_use]
    pub fn relation_count(&self) -> usize {
        self.relations.len()
    }

    #[must_use]
    pub fn treaty_count(&self) -> usize {
        self.treaties.len()
    }

    #[must_use]
    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&(self.relations.len() as u64).to_le_bytes());
        for relation in self.relations.values() {
            hasher.update(&relation.checksum().to_le_bytes());
        }
        hasher.update(&(self.treaties.len() as u64).to_le_bytes());
        for treaty in self.treaties.values() {
            hasher.update(&treaty.checksum().to_le_bytes());
        }
        hasher.finalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diplomatic_stance() {
        assert!(DiplomaticStance::War.allows_combat());
        assert!(!DiplomaticStance::War.allows_passage());
        assert!(DiplomaticStance::Neutral.allows_passage());
        assert!(DiplomaticStance::Allied.allows_trade());
        assert!(DiplomaticStance::Friendly.is_positive());
        assert!(DiplomaticStance::Hostile.is_negative());
    }

    #[test]
    fn test_stance_value_conversion() {
        for stance in [
            DiplomaticStance::War,
            DiplomaticStance::Hostile,
            DiplomaticStance::Unfriendly,
            DiplomaticStance::Neutral,
            DiplomaticStance::Cordial,
            DiplomaticStance::Friendly,
            DiplomaticStance::Allied,
        ] {
            let value = stance.as_value();
            let restored = DiplomaticStance::from_value(value);
            assert_eq!(stance, restored);
        }
    }

    #[test]
    fn test_trust_level() {
        assert!(TrustLevel::new(0.2).is_distrustful());
        assert!(TrustLevel::new(0.45).is_neutral());
        assert!(TrustLevel::new(0.7).is_trusting());
    }

    #[test]
    fn test_diplomatic_relation() {
        let mut relation = DiplomaticRelation::new(
            SocialFactionId::new("empire"),
            SocialFactionId::new("rebels"),
            0,
        );

        relation.modify_favor(150, 100);
        assert!(relation.accumulated_favor > 0);
    }

    #[test]
    fn test_treaty_lifecycle() {
        let treaty = Treaty::new(
            TreatyId::new(1),
            TreatyKind::NonAggression,
            vec![
                SocialFactionId::new("empire"),
                SocialFactionId::new("federation"),
            ],
            100,
        )
        .with_duration(500);

        assert!(treaty.is_active());
        assert!(!treaty.is_expired(200));
        assert!(treaty.is_expired(600));
    }

    #[test]
    fn test_diplomacy_tracker() {
        let mut tracker = DiplomacyTracker::new();

        let empire = SocialFactionId::new("empire");
        let rebels = SocialFactionId::new("rebels");

        tracker.set_stance(&empire, &rebels, DiplomaticStance::Hostile, 0);
        assert_eq!(
            tracker.get_stance(&empire, &rebels),
            DiplomaticStance::Hostile
        );

        tracker.declare_war(&empire, &rebels, 100);
        assert!(tracker.are_at_war(&empire, &rebels));
    }

    #[test]
    fn test_treaty_signing() {
        let mut tracker = DiplomacyTracker::new();

        let parties = [
            SocialFactionId::new("faction_a"),
            SocialFactionId::new("faction_b"),
        ];

        let treaty_id = tracker.sign_treaty(TreatyKind::DefensiveAlliance, &parties, 0);
        assert!(tracker.get_treaty(treaty_id).is_some());
        assert_eq!(tracker.active_treaties().count(), 1);
    }

    #[test]
    fn test_checksum_determinism() {
        let mut tracker = DiplomacyTracker::new();
        tracker.set_stance(
            &SocialFactionId::new("a"),
            &SocialFactionId::new("b"),
            DiplomaticStance::Friendly,
            0,
        );

        let checksum1 = tracker.checksum();
        let checksum2 = tracker.checksum();
        assert_eq!(checksum1, checksum2);
    }

    #[test]
    fn test_serde_roundtrip() {
        let mut tracker = DiplomacyTracker::new();
        tracker.sign_treaty(
            TreatyKind::TradeAgreement,
            &[SocialFactionId::new("a"), SocialFactionId::new("b")],
            100,
        );

        let bytes = bincode::serialize(&tracker).unwrap();
        let restored: DiplomacyTracker = bincode::deserialize(&bytes).unwrap();
        assert_eq!(tracker.checksum(), restored.checksum());
    }
}
