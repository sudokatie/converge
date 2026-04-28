//! Diplomacy and faction membership systems.

use super::{FactionId, ReputationSet, ReputationTier};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Unique identifier for an actor (same as reputation module).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ActorId(pub u64);

impl ActorId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Diplomatic stance between factions.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum Stance {
    /// At war, will attack on sight.
    War,
    /// Hostile but not openly fighting.
    Hostile,
    /// Distrustful, restricted interactions.
    Unfriendly,
    /// Default state, no special relationship.
    #[default]
    Neutral,
    /// Positive relations, some cooperation.
    Friendly,
    /// Close allies, full cooperation.
    Allied,
    /// Same faction or fully integrated.
    Unified,
}

impl Stance {
    /// Check if hostile (War or Hostile).
    #[must_use]
    pub fn is_hostile(self) -> bool {
        matches!(self, Self::War | Self::Hostile)
    }

    /// Check if at war.
    #[must_use]
    pub fn is_at_war(self) -> bool {
        self == Self::War
    }

    /// Check if friendly (Friendly, Allied, or Unified).
    #[must_use]
    pub fn is_friendly(self) -> bool {
        matches!(self, Self::Friendly | Self::Allied | Self::Unified)
    }

    /// Check if allied (Allied or Unified).
    #[must_use]
    pub fn is_allied(self) -> bool {
        matches!(self, Self::Allied | Self::Unified)
    }

    /// Check if allows trading.
    #[must_use]
    pub fn allows_trade(self) -> bool {
        matches!(
            self,
            Self::Neutral | Self::Friendly | Self::Allied | Self::Unified
        )
    }

    /// Check if allows territory access.
    #[must_use]
    pub fn allows_access(self) -> bool {
        matches!(self, Self::Friendly | Self::Allied | Self::Unified)
    }

    /// Check if allows resource sharing.
    #[must_use]
    pub fn allows_resource_sharing(self) -> bool {
        matches!(self, Self::Allied | Self::Unified)
    }

    /// Get the corresponding reputation tier for this stance.
    #[must_use]
    pub fn to_reputation_tier(self) -> ReputationTier {
        match self {
            Self::War | Self::Hostile => ReputationTier::Hostile,
            Self::Unfriendly => ReputationTier::Wary,
            Self::Neutral => ReputationTier::Neutral,
            Self::Friendly => ReputationTier::Friendly,
            Self::Allied => ReputationTier::Ally,
            Self::Unified => ReputationTier::Revered,
        }
    }

    /// Create from a reputation tier.
    #[must_use]
    pub fn from_reputation_tier(tier: ReputationTier) -> Self {
        match tier {
            ReputationTier::Hostile => Self::Hostile,
            ReputationTier::Wary => Self::Unfriendly,
            ReputationTier::Neutral => Self::Neutral,
            ReputationTier::Friendly => Self::Friendly,
            ReputationTier::Ally => Self::Allied,
            ReputationTier::Revered => Self::Unified,
        }
    }

    /// Upgrade stance one level (if possible).
    #[must_use]
    pub fn upgrade(self) -> Self {
        match self {
            Self::War => Self::Hostile,
            Self::Hostile => Self::Unfriendly,
            Self::Unfriendly => Self::Neutral,
            Self::Neutral => Self::Friendly,
            Self::Friendly => Self::Allied,
            Self::Allied | Self::Unified => Self::Unified,
        }
    }

    /// Downgrade stance one level (if possible).
    #[must_use]
    pub fn downgrade(self) -> Self {
        match self {
            Self::Unified => Self::Allied,
            Self::Allied => Self::Friendly,
            Self::Friendly => Self::Neutral,
            Self::Neutral => Self::Unfriendly,
            Self::Unfriendly => Self::Hostile,
            Self::Hostile | Self::War => Self::War,
        }
    }
}

/// Entry for stance serialization.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct StanceEntry {
    faction_a: FactionId,
    faction_b: FactionId,
    stance: Stance,
}

/// Table of stances between faction pairs.
#[derive(Clone, Debug, Default)]
pub struct StanceTable {
    stances: BTreeMap<(FactionId, FactionId), Stance>,
    default_stance: Stance,
}

impl serde::Serialize for StanceTable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let entries: Vec<StanceEntry> = self
            .stances
            .iter()
            .map(|((a, b), &s)| StanceEntry {
                faction_a: a.clone(),
                faction_b: b.clone(),
                stance: s,
            })
            .collect();

        let mut state = serializer.serialize_struct("StanceTable", 2)?;
        state.serialize_field("stances", &entries)?;
        state.serialize_field("default_stance", &self.default_stance)?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for StanceTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct StanceTableData {
            stances: Vec<StanceEntry>,
            default_stance: Stance,
        }

        let data = StanceTableData::deserialize(deserializer)?;
        let mut stances = BTreeMap::new();
        for entry in data.stances {
            let key = Self::normalize_key(&entry.faction_a, &entry.faction_b);
            stances.insert(key, entry.stance);
        }

        Ok(Self {
            stances,
            default_stance: data.default_stance,
        })
    }
}

impl StanceTable {
    /// Create a new stance table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with a specific default stance.
    #[must_use]
    pub fn with_default(default: Stance) -> Self {
        Self {
            stances: BTreeMap::new(),
            default_stance: default,
        }
    }

    fn normalize_key(a: &FactionId, b: &FactionId) -> (FactionId, FactionId) {
        if a <= b {
            (a.clone(), b.clone())
        } else {
            (b.clone(), a.clone())
        }
    }

    /// Set stance between two factions.
    pub fn set(&mut self, a: &FactionId, b: &FactionId, stance: Stance) {
        if a == b {
            return;
        }
        let key = Self::normalize_key(a, b);
        self.stances.insert(key, stance);
    }

    /// Get stance between two factions.
    #[must_use]
    pub fn get(&self, a: &FactionId, b: &FactionId) -> Stance {
        if a == b {
            return Stance::Unified;
        }
        let key = Self::normalize_key(a, b);
        self.stances
            .get(&key)
            .copied()
            .unwrap_or(self.default_stance)
    }

    /// Remove stance entry (revert to default).
    pub fn remove(&mut self, a: &FactionId, b: &FactionId) {
        let key = Self::normalize_key(a, b);
        self.stances.remove(&key);
    }

    /// Check if at war.
    #[must_use]
    pub fn is_at_war(&self, a: &FactionId, b: &FactionId) -> bool {
        self.get(a, b).is_at_war()
    }

    /// Check if hostile.
    #[must_use]
    pub fn is_hostile(&self, a: &FactionId, b: &FactionId) -> bool {
        self.get(a, b).is_hostile()
    }

    /// Check if allied.
    #[must_use]
    pub fn is_allied(&self, a: &FactionId, b: &FactionId) -> bool {
        self.get(a, b).is_allied()
    }

    /// Check if trade is allowed.
    #[must_use]
    pub fn allows_trade(&self, a: &FactionId, b: &FactionId) -> bool {
        self.get(a, b).allows_trade()
    }

    /// Get all factions with a specific stance toward a faction.
    pub fn with_stance_toward(
        &self,
        faction: &FactionId,
        stance: Stance,
    ) -> impl Iterator<Item = &FactionId> {
        self.stances
            .iter()
            .filter(move |(key, s)| **s == stance && (key.0 == *faction || key.1 == *faction))
            .map(
                move |(key, _)| {
                    if key.0 == *faction { &key.1 } else { &key.0 }
                },
            )
    }

    /// Get factions at war with a faction.
    pub fn at_war_with(&self, faction: &FactionId) -> impl Iterator<Item = &FactionId> {
        self.with_stance_toward(faction, Stance::War)
    }

    /// Get factions allied with a faction.
    pub fn allied_with(&self, faction: &FactionId) -> impl Iterator<Item = &FactionId> {
        self.stances
            .iter()
            .filter(move |(key, s)| s.is_allied() && (key.0 == *faction || key.1 == *faction))
            .map(
                move |(key, _)| {
                    if key.0 == *faction { &key.1 } else { &key.0 }
                },
            )
    }

    /// Get number of stance entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.stances.len()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.stances.is_empty()
    }

    /// Iterate over all stance entries.
    pub fn iter(&self) -> impl Iterator<Item = (&FactionId, &FactionId, Stance)> {
        self.stances.iter().map(|((a, b), &s)| (a, b, s))
    }

    /// Declare war between two factions.
    pub fn declare_war(&mut self, a: &FactionId, b: &FactionId) {
        self.set(a, b, Stance::War);
    }

    /// Make peace between two factions.
    pub fn make_peace(&mut self, a: &FactionId, b: &FactionId) {
        let current = self.get(a, b);
        if current.is_at_war() {
            self.set(a, b, Stance::Hostile);
        }
    }

    /// Form alliance between two factions.
    pub fn form_alliance(&mut self, a: &FactionId, b: &FactionId) {
        self.set(a, b, Stance::Allied);
    }

    /// Break alliance between two factions.
    pub fn break_alliance(&mut self, a: &FactionId, b: &FactionId) {
        let current = self.get(a, b);
        if current.is_allied() {
            self.set(a, b, Stance::Friendly);
        }
    }
}

/// Kind of membership an actor has with a faction.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum MembershipKind {
    /// Citizen/subject of the faction.
    #[default]
    Citizen,
    /// Member with full rights.
    Member,
    /// Leader/ruling position.
    Leader,
    /// Guest/temporary access.
    Guest,
    /// Exile/banned from faction.
    Exile,
}

impl MembershipKind {
    /// Check if this is a positive membership (not exile).
    #[must_use]
    pub fn is_positive(self) -> bool {
        !matches!(self, Self::Exile)
    }

    /// Check if has full member rights.
    #[must_use]
    pub fn is_full_member(self) -> bool {
        matches!(self, Self::Member | Self::Leader)
    }

    /// Check if can represent faction.
    #[must_use]
    pub fn can_represent(self) -> bool {
        self == Self::Leader
    }

    /// Check if has territory access.
    #[must_use]
    pub fn has_territory_access(self) -> bool {
        matches!(
            self,
            Self::Citizen | Self::Member | Self::Leader | Self::Guest
        )
    }
}

/// Membership record for an actor in a faction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FactionMembership {
    /// Actor identifier.
    pub actor: ActorId,
    /// Faction identifier.
    pub faction: FactionId,
    /// Kind of membership.
    pub kind: MembershipKind,
    /// Tick when membership started.
    pub joined_tick: u64,
    /// Optional rank/title within faction.
    pub rank: Option<String>,
    /// Local reputation override (if any).
    local_reputation_override: Option<i32>,
}

impl FactionMembership {
    /// Create a new membership.
    #[must_use]
    pub fn new(actor: ActorId, faction: FactionId, kind: MembershipKind, tick: u64) -> Self {
        Self {
            actor,
            faction,
            kind,
            joined_tick: tick,
            rank: None,
            local_reputation_override: None,
        }
    }

    /// Set rank.
    #[must_use]
    pub fn with_rank(mut self, rank: impl Into<String>) -> Self {
        self.rank = Some(rank.into());
        self
    }

    /// Set local reputation override.
    pub fn set_reputation_override(&mut self, value: Option<i32>) {
        self.local_reputation_override = value;
    }

    /// Get local reputation override.
    #[must_use]
    pub fn reputation_override(&self) -> Option<i32> {
        self.local_reputation_override
    }

    /// Check if membership is positive.
    #[must_use]
    pub fn is_positive(&self) -> bool {
        self.kind.is_positive()
    }

    /// Check if is exile.
    #[must_use]
    pub fn is_exile(&self) -> bool {
        self.kind == MembershipKind::Exile
    }
}

/// Table of diplomacy relations including stances and actor-faction state.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DiplomacyTable {
    /// Stances between factions.
    pub stances: StanceTable,
    /// Actor memberships.
    memberships: BTreeMap<ActorId, Vec<FactionMembership>>,
    /// Actor-specific reputation overrides (actor, faction) -> standing.
    actor_reputation_overrides: BTreeMap<(ActorId, FactionId), i32>,
}

impl DiplomacyTable {
    /// Create a new diplomacy table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add membership for an actor.
    pub fn add_membership(&mut self, membership: FactionMembership) {
        let memberships = self
            .memberships
            .entry(membership.actor.clone())
            .or_default();
        memberships.retain(|m| m.faction != membership.faction);
        memberships.push(membership);
        memberships.sort_by(|a, b| a.faction.cmp(&b.faction));
    }

    /// Remove membership.
    pub fn remove_membership(&mut self, actor: &ActorId, faction: &FactionId) {
        if let Some(memberships) = self.memberships.get_mut(actor) {
            memberships.retain(|m| &m.faction != faction);
        }
    }

    /// Get membership for an actor in a faction.
    #[must_use]
    pub fn get_membership(
        &self,
        actor: &ActorId,
        faction: &FactionId,
    ) -> Option<&FactionMembership> {
        self.memberships
            .get(actor)?
            .iter()
            .find(|m| &m.faction == faction)
    }

    /// Get all memberships for an actor.
    pub fn memberships_of(&self, actor: &ActorId) -> impl Iterator<Item = &FactionMembership> {
        self.memberships
            .get(actor)
            .into_iter()
            .flat_map(|v| v.iter())
    }

    /// Get actors in a faction.
    pub fn members_of(&self, faction: &FactionId) -> impl Iterator<Item = &FactionMembership> {
        self.memberships
            .values()
            .flat_map(|v| v.iter())
            .filter(move |m| &m.faction == faction && m.is_positive())
    }

    /// Check if actor is member of faction.
    #[must_use]
    pub fn is_member(&self, actor: &ActorId, faction: &FactionId) -> bool {
        self.get_membership(actor, faction)
            .is_some_and(FactionMembership::is_positive)
    }

    /// Check if actor is exiled from faction.
    #[must_use]
    pub fn is_exiled(&self, actor: &ActorId, faction: &FactionId) -> bool {
        self.get_membership(actor, faction)
            .is_some_and(FactionMembership::is_exile)
    }

    /// Get primary faction for an actor (first positive membership).
    #[must_use]
    pub fn primary_faction(&self, actor: &ActorId) -> Option<&FactionId> {
        self.memberships_of(actor)
            .filter(|m| m.is_positive())
            .map(|m| &m.faction)
            .next()
    }

    /// Set actor-specific reputation override.
    pub fn set_actor_reputation_override(
        &mut self,
        actor: &ActorId,
        faction: &FactionId,
        value: i32,
    ) {
        self.actor_reputation_overrides
            .insert((actor.clone(), faction.clone()), value);
    }

    /// Remove actor-specific reputation override.
    pub fn remove_actor_reputation_override(&mut self, actor: &ActorId, faction: &FactionId) {
        self.actor_reputation_overrides
            .remove(&(actor.clone(), faction.clone()));
    }

    /// Get actor-specific reputation override.
    #[must_use]
    pub fn actor_reputation_override(&self, actor: &ActorId, faction: &FactionId) -> Option<i32> {
        self.actor_reputation_overrides
            .get(&(actor.clone(), faction.clone()))
            .copied()
    }

    /// Get effective reputation tier for actor-faction relationship.
    #[must_use]
    pub fn effective_tier(
        &self,
        actor: &ActorId,
        faction: &FactionId,
        reputation_set: &ReputationSet,
    ) -> ReputationTier {
        if let Some(override_value) = self.actor_reputation_override(actor, faction) {
            return ReputationTier::classify(override_value);
        }

        if let Some(membership) = self.get_membership(actor, faction) {
            if let Some(override_value) = membership.reputation_override() {
                return ReputationTier::classify(override_value);
            }
            if membership.is_positive() {
                return ReputationTier::Ally;
            }
            if membership.is_exile() {
                return ReputationTier::Hostile;
            }
        }

        reputation_set.tier(faction)
    }

    /// Check if actor can enter faction territory.
    #[must_use]
    pub fn can_enter_territory(
        &self,
        actor: &ActorId,
        faction: &FactionId,
        reputation_set: &ReputationSet,
    ) -> bool {
        if self.is_member(actor, faction) {
            return true;
        }

        if self.is_exiled(actor, faction) {
            return false;
        }

        let tier = self.effective_tier(actor, faction, reputation_set);
        tier.allows_access()
    }

    /// Check if actor can build in faction territory.
    #[must_use]
    pub fn can_build_in_territory(
        &self,
        actor: &ActorId,
        faction: &FactionId,
        reputation_set: &ReputationSet,
    ) -> bool {
        if let Some(membership) = self.get_membership(actor, faction) {
            return membership.kind.is_full_member();
        }

        let tier = self.effective_tier(actor, faction, reputation_set);
        tier.allows_building()
    }

    /// Check if actor is considered a threat by faction.
    #[must_use]
    pub fn is_threat(
        &self,
        actor: &ActorId,
        faction: &FactionId,
        reputation_set: &ReputationSet,
    ) -> bool {
        if self.is_exiled(actor, faction) {
            return true;
        }

        if self.is_member(actor, faction) {
            return false;
        }

        let tier = self.effective_tier(actor, faction, reputation_set);
        tier.is_threat()
    }

    /// Get number of actors with memberships.
    #[must_use]
    pub fn actor_count(&self) -> usize {
        self.memberships.len()
    }

    /// Clear all memberships for an actor.
    pub fn clear_actor(&mut self, actor: &ActorId) {
        self.memberships.remove(actor);
        self.actor_reputation_overrides
            .retain(|(a, _), _| a != actor);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stance_properties() {
        assert!(Stance::War.is_hostile());
        assert!(Stance::War.is_at_war());
        assert!(!Stance::War.allows_trade());

        assert!(Stance::Allied.is_friendly());
        assert!(Stance::Allied.is_allied());
        assert!(Stance::Allied.allows_trade());
        assert!(Stance::Allied.allows_access());
        assert!(Stance::Allied.allows_resource_sharing());

        assert!(!Stance::Neutral.is_hostile());
        assert!(!Stance::Neutral.is_friendly());
        assert!(Stance::Neutral.allows_trade());
    }

    #[test]
    fn test_stance_upgrade_downgrade() {
        assert_eq!(Stance::Neutral.upgrade(), Stance::Friendly);
        assert_eq!(Stance::Neutral.downgrade(), Stance::Unfriendly);
        assert_eq!(Stance::War.downgrade(), Stance::War);
        assert_eq!(Stance::Unified.upgrade(), Stance::Unified);
    }

    #[test]
    fn test_stance_reputation_conversion() {
        assert_eq!(Stance::War.to_reputation_tier(), ReputationTier::Hostile);
        assert_eq!(Stance::Allied.to_reputation_tier(), ReputationTier::Ally);
        assert_eq!(
            Stance::from_reputation_tier(ReputationTier::Friendly),
            Stance::Friendly
        );
    }

    #[test]
    fn test_stance_table_basic() {
        let mut table = StanceTable::new();
        let a = FactionId::new("a");
        let b = FactionId::new("b");

        table.set(&a, &b, Stance::Allied);
        assert_eq!(table.get(&a, &b), Stance::Allied);
        assert_eq!(table.get(&b, &a), Stance::Allied);
    }

    #[test]
    fn test_stance_table_default() {
        let table = StanceTable::new();
        let a = FactionId::new("a");
        let b = FactionId::new("b");

        assert_eq!(table.get(&a, &b), Stance::Neutral);
    }

    #[test]
    fn test_stance_table_same_faction() {
        let table = StanceTable::new();
        let a = FactionId::new("a");

        assert_eq!(table.get(&a, &a), Stance::Unified);
    }

    #[test]
    fn test_stance_table_war_and_peace() {
        let mut table = StanceTable::new();
        let a = FactionId::new("a");
        let b = FactionId::new("b");

        table.declare_war(&a, &b);
        assert!(table.is_at_war(&a, &b));

        table.make_peace(&a, &b);
        assert!(!table.is_at_war(&a, &b));
        assert!(table.is_hostile(&a, &b));
    }

    #[test]
    fn test_stance_table_alliance() {
        let mut table = StanceTable::new();
        let a = FactionId::new("a");
        let b = FactionId::new("b");

        table.form_alliance(&a, &b);
        assert!(table.is_allied(&a, &b));

        table.break_alliance(&a, &b);
        assert!(!table.is_allied(&a, &b));
        assert_eq!(table.get(&a, &b), Stance::Friendly);
    }

    #[test]
    fn test_stance_table_queries() {
        let mut table = StanceTable::new();
        let a = FactionId::new("a");
        let b = FactionId::new("b");
        let c = FactionId::new("c");

        table.declare_war(&a, &b);
        table.form_alliance(&a, &c);

        let at_war: Vec<_> = table.at_war_with(&a).collect();
        assert_eq!(at_war.len(), 1);
        assert_eq!(at_war[0], &b);

        let allied: Vec<_> = table.allied_with(&a).collect();
        assert_eq!(allied.len(), 1);
        assert_eq!(allied[0], &c);
    }

    #[test]
    fn test_membership_kind() {
        assert!(MembershipKind::Member.is_positive());
        assert!(!MembershipKind::Exile.is_positive());
        assert!(MembershipKind::Leader.can_represent());
        assert!(!MembershipKind::Member.can_represent());
    }

    #[test]
    fn test_faction_membership() {
        let actor = ActorId::new(1);
        let faction = FactionId::new("miners");

        let membership =
            FactionMembership::new(actor.clone(), faction.clone(), MembershipKind::Member, 100)
                .with_rank("Foreman");

        assert!(membership.is_positive());
        assert_eq!(membership.rank.as_deref(), Some("Foreman"));
    }

    #[test]
    fn test_membership_reputation_override() {
        let actor = ActorId::new(1);
        let faction = FactionId::new("miners");

        let mut membership = FactionMembership::new(actor, faction, MembershipKind::Member, 100);
        membership.set_reputation_override(Some(500));

        assert_eq!(membership.reputation_override(), Some(500));
    }

    #[test]
    fn test_diplomacy_table_membership() {
        let mut table = DiplomacyTable::new();
        let actor = ActorId::new(1);
        let faction = FactionId::new("miners");

        table.add_membership(FactionMembership::new(
            actor.clone(),
            faction.clone(),
            MembershipKind::Member,
            100,
        ));

        assert!(table.is_member(&actor, &faction));
        assert!(!table.is_exiled(&actor, &faction));
        assert_eq!(table.primary_faction(&actor), Some(&faction));
    }

    #[test]
    fn test_diplomacy_table_exile() {
        let mut table = DiplomacyTable::new();
        let actor = ActorId::new(1);
        let faction = FactionId::new("miners");

        table.add_membership(FactionMembership::new(
            actor.clone(),
            faction.clone(),
            MembershipKind::Exile,
            100,
        ));

        assert!(!table.is_member(&actor, &faction));
        assert!(table.is_exiled(&actor, &faction));
    }

    #[test]
    fn test_diplomacy_table_effective_tier() {
        let mut table = DiplomacyTable::new();
        let actor = ActorId::new(1);
        let faction = FactionId::new("miners");
        let reputation = ReputationSet::new();

        table.add_membership(FactionMembership::new(
            actor.clone(),
            faction.clone(),
            MembershipKind::Member,
            100,
        ));

        assert_eq!(
            table.effective_tier(&actor, &faction, &reputation),
            ReputationTier::Ally
        );
    }

    #[test]
    fn test_diplomacy_table_reputation_override() {
        let mut table = DiplomacyTable::new();
        let actor = ActorId::new(1);
        let faction = FactionId::new("miners");
        let reputation = ReputationSet::new();

        table.set_actor_reputation_override(&actor, &faction, -600);

        assert_eq!(
            table.effective_tier(&actor, &faction, &reputation),
            ReputationTier::Hostile
        );
    }

    #[test]
    fn test_diplomacy_table_territory_access() {
        let mut table = DiplomacyTable::new();
        let actor = ActorId::new(1);
        let faction = FactionId::new("miners");
        let reputation = ReputationSet::new();

        assert!(table.can_enter_territory(&actor, &faction, &reputation));

        table.add_membership(FactionMembership::new(
            actor.clone(),
            faction.clone(),
            MembershipKind::Exile,
            100,
        ));

        assert!(!table.can_enter_territory(&actor, &faction, &reputation));
    }

    #[test]
    fn test_diplomacy_table_threat() {
        let mut table = DiplomacyTable::new();
        let actor = ActorId::new(1);
        let faction = FactionId::new("miners");
        let reputation = ReputationSet::new();

        assert!(!table.is_threat(&actor, &faction, &reputation));

        table.add_membership(FactionMembership::new(
            actor.clone(),
            faction.clone(),
            MembershipKind::Exile,
            100,
        ));

        assert!(table.is_threat(&actor, &faction, &reputation));
    }

    #[test]
    fn test_diplomacy_table_members_of() {
        let mut table = DiplomacyTable::new();
        let faction = FactionId::new("miners");

        table.add_membership(FactionMembership::new(
            ActorId::new(1),
            faction.clone(),
            MembershipKind::Member,
            100,
        ));
        table.add_membership(FactionMembership::new(
            ActorId::new(2),
            faction.clone(),
            MembershipKind::Leader,
            100,
        ));
        table.add_membership(FactionMembership::new(
            ActorId::new(3),
            faction.clone(),
            MembershipKind::Exile,
            100,
        ));

        let members: Vec<_> = table.members_of(&faction).collect();
        assert_eq!(members.len(), 2);
    }

    #[test]
    fn test_stance_table_serde() {
        let mut table = StanceTable::new();
        table.set(&FactionId::new("a"), &FactionId::new("b"), Stance::Allied);
        table.declare_war(&FactionId::new("a"), &FactionId::new("c"));

        let json = serde_json::to_string(&table).unwrap();
        let restored: StanceTable = serde_json::from_str(&json).unwrap();

        assert_eq!(
            restored.get(&FactionId::new("a"), &FactionId::new("b")),
            Stance::Allied
        );
        assert!(restored.is_at_war(&FactionId::new("a"), &FactionId::new("c")));
    }

    #[test]
    fn test_diplomacy_table_serde() {
        let mut table = DiplomacyTable::new();
        table
            .stances
            .form_alliance(&FactionId::new("a"), &FactionId::new("b"));
        table.add_membership(FactionMembership::new(
            ActorId::new(1),
            FactionId::new("a"),
            MembershipKind::Member,
            100,
        ));

        let json = serde_json::to_string(&table).unwrap();
        let restored: DiplomacyTable = serde_json::from_str(&json).unwrap();

        assert!(
            restored
                .stances
                .is_allied(&FactionId::new("a"), &FactionId::new("b"))
        );
        assert!(restored.is_member(&ActorId::new(1), &FactionId::new("a")));
    }

    #[test]
    fn test_diplomacy_table_clear_actor() {
        let mut table = DiplomacyTable::new();
        let actor = ActorId::new(1);

        table.add_membership(FactionMembership::new(
            actor.clone(),
            FactionId::new("a"),
            MembershipKind::Member,
            100,
        ));
        table.set_actor_reputation_override(&actor, &FactionId::new("b"), 500);

        table.clear_actor(&actor);

        assert!(!table.is_member(&actor, &FactionId::new("a")));
        assert!(
            table
                .actor_reputation_override(&actor, &FactionId::new("b"))
                .is_none()
        );
    }
}
