//! Snapshots and summaries for faction/territory state in unloaded regions.

use super::{
    DiplomacyTable, FactionId, FactionRegistry, OwnershipStatus, RegionId, Stance, StanceTable,
    TerritoryMap,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Summary of a faction's state for unloaded region simulation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FactionSummary {
    /// Faction identifier.
    pub faction_id: FactionId,
    /// Number of regions owned.
    pub regions_owned: u32,
    /// Total influence across all regions.
    pub total_influence: u32,
    /// Number of allied factions.
    pub ally_count: u32,
    /// Number of hostile factions.
    pub hostile_count: u32,
    /// Number of active members.
    pub member_count: u32,
    /// Average reputation tier with other factions.
    pub avg_reputation: f32,
    /// Is currently at war with anyone.
    pub at_war: bool,
    /// Has contested territories.
    pub has_contested: bool,
}

impl FactionSummary {
    /// Create a summary for a faction.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "count precision loss acceptable for summary"
    )]
    pub fn new(
        faction_id: FactionId,
        territory: &TerritoryMap,
        stances: &StanceTable,
        diplomacy: &DiplomacyTable,
        all_factions: &[&FactionId],
    ) -> Self {
        let regions_owned = territory.count_owned_by(&faction_id) as u32;
        let total_influence = territory.total_influence(&faction_id);

        let mut ally_count = 0u32;
        let mut hostile_count = 0u32;
        let mut at_war = false;
        let mut rep_sum = 0i32;

        for other in all_factions {
            if *other == &faction_id {
                continue;
            }

            let stance = stances.get(&faction_id, other);
            if stance.is_allied() {
                ally_count += 1;
            }
            if stance.is_hostile() {
                hostile_count += 1;
            }
            if stance.is_at_war() {
                at_war = true;
            }

            rep_sum += match stance {
                Stance::War => -2,
                Stance::Hostile => -1,
                Stance::Unfriendly => 0,
                Stance::Neutral => 1,
                Stance::Friendly => 2,
                Stance::Allied => 3,
                Stance::Unified => 4,
            };
        }

        let other_count = all_factions.len().saturating_sub(1);
        let avg_reputation = if other_count > 0 {
            rep_sum as f32 / other_count as f32
        } else {
            0.0
        };

        let member_count = diplomacy.members_of(&faction_id).count() as u32;

        let has_contested = territory.owned_by(&faction_id).any(|r| {
            matches!(
                r.status(),
                OwnershipStatus::Contested | OwnershipStatus::Disputed
            )
        });

        Self {
            faction_id,
            regions_owned,
            total_influence,
            ally_count,
            hostile_count,
            member_count,
            avg_reputation,
            at_war,
            has_contested,
        }
    }

    /// Check if faction is in a strong position.
    #[must_use]
    pub fn is_strong(&self) -> bool {
        self.regions_owned >= 5 && self.ally_count >= 2 && !self.at_war
    }

    /// Check if faction is in a weak position.
    #[must_use]
    pub fn is_weak(&self) -> bool {
        self.regions_owned == 0 || (self.at_war && self.ally_count == 0)
    }

    /// Check if faction needs attention.
    #[must_use]
    pub fn needs_attention(&self) -> bool {
        self.at_war || self.has_contested || self.is_weak()
    }

    /// Estimate threat level (0.0 = safe, 1.0 = critical).
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "count precision loss acceptable"
    )]
    pub fn threat_level(&self) -> f32 {
        let mut threat = 0.0f32;

        if self.at_war {
            threat += 0.4;
        }

        if self.has_contested {
            threat += 0.2;
        }

        if self.hostile_count > 0 {
            threat += (self.hostile_count as f32 * 0.1).min(0.3);
        }

        if self.ally_count == 0 {
            threat += 0.1;
        }

        threat.clamp(0.0, 1.0)
    }
}

/// Summary of territory state for a region.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerritorySnapshot {
    /// Region identifier.
    pub region_id: RegionId,
    /// Owner faction (if any).
    pub owner: Option<FactionId>,
    /// Ownership status.
    pub status: OwnershipStatus,
    /// Contestation level (0.0 to 1.0).
    pub contestation: f32,
    /// Dominant faction by influence.
    pub dominant: Option<FactionId>,
    /// Number of factions with influence.
    pub faction_count: u32,
    /// Total influence in region.
    pub total_influence: u32,
    /// Chunk count.
    pub chunk_count: u32,
    /// Tick when snapshot was taken.
    pub snapshot_tick: u64,
}

impl TerritorySnapshot {
    /// Create a snapshot from a territory map region.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "counts bounded by reasonable region sizes"
    )]
    pub fn from_territory(
        region_id: &RegionId,
        territory: &TerritoryMap,
        tick: u64,
    ) -> Option<Self> {
        let region = territory.get(region_id)?;

        let faction_count = region.factions_with_influence().count() as u32;
        let total_influence: u32 = region
            .factions_with_influence()
            .map(|f| region.get_influence(f))
            .sum();

        Some(Self {
            region_id: region_id.clone(),
            owner: region.owner().cloned(),
            status: region.status(),
            contestation: region.contestation_level(),
            dominant: region.dominant_faction().cloned(),
            faction_count,
            total_influence,
            chunk_count: region.chunk_count() as u32,
            snapshot_tick: tick,
        })
    }

    /// Check if region is stable (not contested).
    #[must_use]
    pub fn is_stable(&self) -> bool {
        matches!(
            self.status,
            OwnershipStatus::Owned | OwnershipStatus::Unclaimed
        )
    }

    /// Check if region needs attention.
    #[must_use]
    pub fn needs_attention(&self) -> bool {
        self.contestation > 0.3
            || matches!(
                self.status,
                OwnershipStatus::Contested | OwnershipStatus::Disputed
            )
    }

    /// Estimate ticks until status might change (simple heuristic).
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "estimation bounded"
    )]
    pub fn estimated_ticks_until_change(&self) -> Option<u64> {
        if self.is_stable() {
            return None;
        }

        let base = (100.0 / (1.0 + self.contestation * 10.0)) as u64;
        Some(base.max(10))
    }

    /// Check if snapshot is stale.
    #[must_use]
    pub fn is_stale(&self, current_tick: u64, max_staleness: u64) -> bool {
        current_tick.saturating_sub(self.snapshot_tick) > max_staleness
    }

    /// Get age of snapshot.
    #[must_use]
    pub fn age(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.snapshot_tick)
    }
}

/// Complete faction system snapshot for persistence or unloaded simulation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FactionSnapshot {
    /// Faction summaries.
    pub factions: BTreeMap<FactionId, FactionSummary>,
    /// Territory snapshots.
    pub territories: BTreeMap<RegionId, TerritorySnapshot>,
    /// Overall threat level.
    pub overall_threat: f32,
    /// Number of active wars.
    pub active_wars: u32,
    /// Number of contested regions.
    pub contested_regions: u32,
    /// Tick when snapshot was taken.
    pub snapshot_tick: u64,
}

impl FactionSnapshot {
    /// Create a complete snapshot.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "count precision loss acceptable"
    )]
    pub fn new(
        registry: &FactionRegistry,
        territory: &TerritoryMap,
        stances: &StanceTable,
        diplomacy: &DiplomacyTable,
        tick: u64,
    ) -> Self {
        let all_factions: Vec<_> = registry.ids().collect();

        let factions: BTreeMap<_, _> = registry
            .iter()
            .map(|f| {
                let summary =
                    FactionSummary::new(f.id.clone(), territory, stances, diplomacy, &all_factions);
                (f.id.clone(), summary)
            })
            .collect();

        let territories: BTreeMap<_, _> = territory
            .ids()
            .filter_map(|id| {
                TerritorySnapshot::from_territory(id, territory, tick).map(|s| (id.clone(), s))
            })
            .collect();

        let active_wars = factions.values().filter(|f| f.at_war).count() as u32 / 2;
        let contested_regions = territories.values().filter(|t| t.needs_attention()).count() as u32;

        let overall_threat = if factions.is_empty() {
            0.0
        } else {
            factions
                .values()
                .map(FactionSummary::threat_level)
                .sum::<f32>()
                / factions.len() as f32
        };

        Self {
            factions,
            territories,
            overall_threat,
            active_wars,
            contested_regions,
            snapshot_tick: tick,
        }
    }

    /// Get faction summary.
    #[must_use]
    pub fn get_faction(&self, id: &FactionId) -> Option<&FactionSummary> {
        self.factions.get(id)
    }

    /// Get territory snapshot.
    #[must_use]
    pub fn get_territory(&self, id: &RegionId) -> Option<&TerritorySnapshot> {
        self.territories.get(id)
    }

    /// Check if any faction needs attention.
    #[must_use]
    pub fn any_needs_attention(&self) -> bool {
        self.factions.values().any(FactionSummary::needs_attention)
            || self
                .territories
                .values()
                .any(TerritorySnapshot::needs_attention)
    }

    /// Get factions that need attention.
    pub fn factions_needing_attention(&self) -> impl Iterator<Item = &FactionSummary> {
        self.factions.values().filter(|f| f.needs_attention())
    }

    /// Get territories that need attention.
    pub fn territories_needing_attention(&self) -> impl Iterator<Item = &TerritorySnapshot> {
        self.territories.values().filter(|t| t.needs_attention())
    }

    /// Check if snapshot is stale.
    #[must_use]
    pub fn is_stale(&self, current_tick: u64, max_staleness: u64) -> bool {
        current_tick.saturating_sub(self.snapshot_tick) > max_staleness
    }

    /// Get age of snapshot.
    #[must_use]
    pub fn age(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.snapshot_tick)
    }

    /// Project threat level after elapsed ticks (simple decay model).
    #[must_use]
    pub fn projected_threat(&self, elapsed_ticks: u64) -> f32 {
        let decay_factor = 0.99_f32.powi(elapsed_ticks.min(1000) as i32);
        self.overall_threat * decay_factor
    }

    /// Estimate if intervention is needed.
    #[must_use]
    pub fn needs_intervention(&self) -> bool {
        self.overall_threat > 0.5 || self.active_wars > 0 || self.contested_regions > 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::faction::{Faction, Region};
    use engine_core::coords::ChunkPos;

    fn setup_test_state() -> (FactionRegistry, TerritoryMap, StanceTable, DiplomacyTable) {
        let mut registry = FactionRegistry::new();
        registry.register(Faction::new(FactionId::new("a"), "Faction A"));
        registry.register(Faction::new(FactionId::new("b"), "Faction B"));
        registry.register(Faction::new(FactionId::new("c"), "Faction C"));

        let mut territory = TerritoryMap::new();
        let mut r1 = Region::from_chunk(ChunkPos::new(0, 0, 0));
        r1.set_owner(Some(FactionId::new("a")), 0);
        r1.set_influence(&FactionId::new("a"), 100);
        territory.add_region(r1);

        let mut r2 = Region::from_chunk(ChunkPos::new(1, 0, 0));
        r2.set_owner(Some(FactionId::new("b")), 0);
        r2.set_influence(&FactionId::new("b"), 80);
        r2.set_influence(&FactionId::new("a"), 60);
        territory.add_region(r2);

        let mut stances = StanceTable::new();
        stances.form_alliance(&FactionId::new("a"), &FactionId::new("c"));
        stances.declare_war(&FactionId::new("a"), &FactionId::new("b"));

        let diplomacy = DiplomacyTable::new();

        (registry, territory, stances, diplomacy)
    }

    #[test]
    fn test_faction_summary_new() {
        let (registry, territory, stances, diplomacy) = setup_test_state();
        let all_factions: Vec<_> = registry.ids().collect();

        let summary = FactionSummary::new(
            FactionId::new("a"),
            &territory,
            &stances,
            &diplomacy,
            &all_factions,
        );

        assert_eq!(summary.faction_id.as_str(), "a");
        assert_eq!(summary.regions_owned, 1);
        assert_eq!(summary.ally_count, 1);
        assert!(summary.at_war);
    }

    #[test]
    fn test_faction_summary_threat_level() {
        let (registry, territory, stances, diplomacy) = setup_test_state();
        let all_factions: Vec<_> = registry.ids().collect();

        let summary = FactionSummary::new(
            FactionId::new("a"),
            &territory,
            &stances,
            &diplomacy,
            &all_factions,
        );

        assert!(summary.threat_level() > 0.0);
        assert!(summary.needs_attention());
    }

    #[test]
    fn test_territory_snapshot() {
        let (_, territory, _, _) = setup_test_state();

        let snapshot = TerritorySnapshot::from_territory(
            &RegionId::from_chunk(ChunkPos::new(0, 0, 0)),
            &territory,
            100,
        )
        .unwrap();

        assert_eq!(snapshot.owner, Some(FactionId::new("a")));
        assert!(snapshot.is_stable());
    }

    #[test]
    fn test_territory_snapshot_contested() {
        let (_, territory, _, _) = setup_test_state();

        let snapshot = TerritorySnapshot::from_territory(
            &RegionId::from_chunk(ChunkPos::new(1, 0, 0)),
            &territory,
            100,
        )
        .unwrap();

        assert!(snapshot.contestation > 0.0);
    }

    #[test]
    fn test_territory_snapshot_staleness() {
        let (_, territory, _, _) = setup_test_state();

        let snapshot = TerritorySnapshot::from_territory(
            &RegionId::from_chunk(ChunkPos::new(0, 0, 0)),
            &territory,
            100,
        )
        .unwrap();

        assert!(!snapshot.is_stale(150, 100));
        assert!(snapshot.is_stale(250, 100));
        assert_eq!(snapshot.age(150), 50);
    }

    #[test]
    fn test_faction_snapshot_new() {
        let (registry, territory, stances, diplomacy) = setup_test_state();

        let snapshot = FactionSnapshot::new(&registry, &territory, &stances, &diplomacy, 100);

        assert_eq!(snapshot.factions.len(), 3);
        assert_eq!(snapshot.territories.len(), 2);
        assert_eq!(snapshot.active_wars, 1);
        assert_eq!(snapshot.snapshot_tick, 100);
    }

    #[test]
    fn test_faction_snapshot_needs_attention() {
        let (registry, territory, stances, diplomacy) = setup_test_state();

        let snapshot = FactionSnapshot::new(&registry, &territory, &stances, &diplomacy, 100);

        assert!(snapshot.any_needs_attention());
        assert!(snapshot.factions_needing_attention().count() > 0);
    }

    #[test]
    fn test_faction_snapshot_projected_threat() {
        let (registry, territory, stances, diplomacy) = setup_test_state();

        let snapshot = FactionSnapshot::new(&registry, &territory, &stances, &diplomacy, 100);
        let initial = snapshot.overall_threat;
        let projected = snapshot.projected_threat(100);

        if initial > 0.0 {
            assert!(projected < initial);
        }
    }

    #[test]
    fn test_faction_summary_serde() {
        let (registry, territory, stances, diplomacy) = setup_test_state();
        let all_factions: Vec<_> = registry.ids().collect();

        let summary = FactionSummary::new(
            FactionId::new("a"),
            &territory,
            &stances,
            &diplomacy,
            &all_factions,
        );

        let json = serde_json::to_string(&summary).unwrap();
        let restored: FactionSummary = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.faction_id, summary.faction_id);
        assert_eq!(restored.regions_owned, summary.regions_owned);
        assert_eq!(restored.at_war, summary.at_war);
    }

    #[test]
    fn test_territory_snapshot_serde() {
        let (_, territory, _, _) = setup_test_state();

        let snapshot = TerritorySnapshot::from_territory(
            &RegionId::from_chunk(ChunkPos::new(0, 0, 0)),
            &territory,
            100,
        )
        .unwrap();

        let json = serde_json::to_string(&snapshot).unwrap();
        let restored: TerritorySnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.region_id, snapshot.region_id);
        assert_eq!(restored.owner, snapshot.owner);
    }

    #[test]
    fn test_faction_snapshot_serde() {
        let (registry, territory, stances, diplomacy) = setup_test_state();

        let snapshot = FactionSnapshot::new(&registry, &territory, &stances, &diplomacy, 100);

        let json = serde_json::to_string(&snapshot).unwrap();
        let restored: FactionSnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.factions.len(), snapshot.factions.len());
        assert_eq!(restored.territories.len(), snapshot.territories.len());
        assert_eq!(restored.active_wars, snapshot.active_wars);
    }

    #[test]
    fn test_faction_summary_strong_weak() {
        let summary = FactionSummary {
            faction_id: FactionId::new("test"),
            regions_owned: 10,
            total_influence: 500,
            ally_count: 3,
            hostile_count: 0,
            member_count: 50,
            avg_reputation: 2.0,
            at_war: false,
            has_contested: false,
        };

        assert!(summary.is_strong());
        assert!(!summary.is_weak());

        let weak_summary = FactionSummary {
            faction_id: FactionId::new("test"),
            regions_owned: 0,
            total_influence: 0,
            ally_count: 0,
            hostile_count: 2,
            member_count: 5,
            avg_reputation: -1.0,
            at_war: true,
            has_contested: false,
        };

        assert!(!weak_summary.is_strong());
        assert!(weak_summary.is_weak());
    }

    #[test]
    fn test_empty_snapshot() {
        let registry = FactionRegistry::new();
        let territory = TerritoryMap::new();
        let stances = StanceTable::new();
        let diplomacy = DiplomacyTable::new();

        let snapshot = FactionSnapshot::new(&registry, &territory, &stances, &diplomacy, 0);

        assert!(snapshot.factions.is_empty());
        assert!(snapshot.territories.is_empty());
        assert!((snapshot.overall_threat).abs() < f32::EPSILON);
    }
}
