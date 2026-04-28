//! Territory model with ownership, influence, and claims.

use super::FactionId;
use engine_core::coords::ChunkPos;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

/// Unique identifier for a region (group of chunks).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RegionId(pub String);

impl RegionId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn from_chunk(pos: ChunkPos) -> Self {
        Self(format!("chunk_{}_{}", pos.x(), pos.z()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for RegionId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

/// Strength of a territorial claim.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum ClaimStrength {
    /// Weak claim, easily contested.
    Weak,
    /// Standard claim.
    #[default]
    Normal,
    /// Strong claim, hard to contest.
    Strong,
    /// Absolute claim, cannot be contested.
    Absolute,
}

impl ClaimStrength {
    /// Get the influence multiplier for this strength.
    #[must_use]
    pub fn influence_multiplier(self) -> f32 {
        match self {
            Self::Weak => 0.5,
            Self::Normal => 1.0,
            Self::Strong => 1.5,
            Self::Absolute => 2.0,
        }
    }

    /// Get the base influence value.
    #[must_use]
    pub fn base_influence(self) -> u32 {
        match self {
            Self::Weak => 25,
            Self::Normal => 50,
            Self::Strong => 75,
            Self::Absolute => 100,
        }
    }
}

/// Type of territorial claim.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ClaimKind {
    /// Claim through presence/settlement.
    Settlement,
    /// Claim through historical/ancestral right.
    Historical,
    /// Claim through conquest.
    Conquest,
    /// Claim through treaty/agreement.
    Treaty,
    /// Claim through resource exploitation.
    Resource,
    /// Claim through religious/cultural significance.
    Sacred,
}

/// A territorial claim by a faction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Claim {
    /// Faction making the claim.
    pub faction: FactionId,
    /// Strength of the claim.
    pub strength: ClaimStrength,
    /// Type of claim.
    pub kind: ClaimKind,
    /// Tick when claim was established.
    pub established_tick: u64,
    /// Optional expiration tick.
    pub expires_tick: Option<u64>,
}

impl Claim {
    #[must_use]
    pub fn new(faction: FactionId, kind: ClaimKind, tick: u64) -> Self {
        Self {
            faction,
            strength: ClaimStrength::Normal,
            kind,
            established_tick: tick,
            expires_tick: None,
        }
    }

    #[must_use]
    pub fn with_strength(mut self, strength: ClaimStrength) -> Self {
        self.strength = strength;
        self
    }

    #[must_use]
    pub fn with_expiration(mut self, tick: u64) -> Self {
        self.expires_tick = Some(tick);
        self
    }

    /// Check if claim has expired.
    #[must_use]
    pub fn is_expired(&self, current_tick: u64) -> bool {
        self.expires_tick.is_some_and(|t| current_tick >= t)
    }

    /// Get effective influence from this claim.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "influence values small and positive"
    )]
    pub fn influence(&self) -> u32 {
        ((self.strength.base_influence() as f32) * self.strength.influence_multiplier()) as u32
    }
}

impl Eq for Claim {}

impl PartialOrd for Claim {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Claim {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .strength
            .cmp(&self.strength)
            .then_with(|| self.established_tick.cmp(&other.established_tick))
            .then_with(|| self.faction.cmp(&other.faction))
    }
}

/// Ownership status of a region.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OwnershipStatus {
    /// No owner, unclaimed.
    Unclaimed,
    /// Owned by a single faction.
    Owned,
    /// Contested between multiple factions.
    Contested,
    /// Disputed with active conflict.
    Disputed,
}

/// Influence data for a faction in a region.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Influence {
    /// Base influence value.
    pub base: u32,
    /// Bonus from structures/presence.
    pub structure_bonus: u32,
    /// Bonus from population.
    pub population_bonus: u32,
    /// Temporary modifiers (decay over time).
    pub temporary: i32,
    /// Tick when last updated.
    pub last_update_tick: u64,
}

impl Influence {
    #[must_use]
    pub fn new(base: u32) -> Self {
        Self {
            base,
            structure_bonus: 0,
            population_bonus: 0,
            temporary: 0,
            last_update_tick: 0,
        }
    }

    /// Get total influence value.
    #[must_use]
    #[expect(
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        reason = "influence values bounded, sum clamped to non-negative"
    )]
    pub fn total(&self) -> u32 {
        let sum = (self.base as i32)
            + (self.structure_bonus as i32)
            + (self.population_bonus as i32)
            + self.temporary;
        sum.max(0) as u32
    }

    /// Add structure bonus.
    pub fn add_structure_bonus(&mut self, amount: u32) {
        self.structure_bonus = self.structure_bonus.saturating_add(amount);
    }

    /// Add population bonus.
    pub fn add_population_bonus(&mut self, amount: u32) {
        self.population_bonus = self.population_bonus.saturating_add(amount);
    }

    /// Apply temporary modifier.
    pub fn apply_temporary(&mut self, delta: i32, tick: u64) {
        self.temporary += delta;
        self.last_update_tick = tick;
    }

    /// Decay temporary influence toward zero.
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "temporary influence values bounded, precision loss acceptable"
    )]
    pub fn decay_temporary(&mut self, rate: f32) {
        if self.temporary == 0 {
            return;
        }

        let decay = ((self.temporary.abs() as f32) * rate).max(1.0) as i32;
        if self.temporary > 0 {
            self.temporary = (self.temporary - decay).max(0);
        } else {
            self.temporary = (self.temporary + decay).min(0);
        }
    }
}

impl Default for Influence {
    fn default() -> Self {
        Self::new(0)
    }
}

/// A region with ownership and influence tracking.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Region {
    /// Region identifier.
    pub id: RegionId,
    /// Chunks contained in this region.
    chunks: BTreeSet<(i32, i32, i32)>,
    /// Current owner (if any).
    owner: Option<FactionId>,
    /// Ownership status.
    status: OwnershipStatus,
    /// Claims on this region.
    claims: Vec<Claim>,
    /// Influence by faction.
    influence: BTreeMap<FactionId, Influence>,
    /// Custom metadata.
    metadata: BTreeMap<String, String>,
    /// Tick when ownership last changed.
    ownership_changed_tick: u64,
}

impl Region {
    /// Create a new region.
    #[must_use]
    pub fn new(id: RegionId) -> Self {
        Self {
            id,
            chunks: BTreeSet::new(),
            owner: None,
            status: OwnershipStatus::Unclaimed,
            claims: Vec::new(),
            influence: BTreeMap::new(),
            metadata: BTreeMap::new(),
            ownership_changed_tick: 0,
        }
    }

    /// Create from a single chunk.
    #[must_use]
    pub fn from_chunk(pos: ChunkPos) -> Self {
        let mut region = Self::new(RegionId::from_chunk(pos));
        region.add_chunk(pos);
        region
    }

    /// Add a chunk to this region.
    pub fn add_chunk(&mut self, pos: ChunkPos) {
        self.chunks.insert((pos.x(), pos.y(), pos.z()));
    }

    /// Remove a chunk from this region.
    pub fn remove_chunk(&mut self, pos: &ChunkPos) -> bool {
        self.chunks.remove(&(pos.x(), pos.y(), pos.z()))
    }

    /// Check if region contains a chunk.
    #[must_use]
    pub fn contains_chunk(&self, pos: &ChunkPos) -> bool {
        self.chunks.contains(&(pos.x(), pos.y(), pos.z()))
    }

    /// Get all chunks in this region.
    pub fn chunks(&self) -> impl Iterator<Item = ChunkPos> + '_ {
        self.chunks
            .iter()
            .map(|(x, y, z)| ChunkPos::new(*x, *y, *z))
    }

    /// Get chunk count.
    #[must_use]
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Get current owner.
    #[must_use]
    pub fn owner(&self) -> Option<&FactionId> {
        self.owner.as_ref()
    }

    /// Get ownership status.
    #[must_use]
    pub fn status(&self) -> OwnershipStatus {
        self.status
    }

    /// Check if owned by a specific faction.
    #[must_use]
    pub fn is_owned_by(&self, faction: &FactionId) -> bool {
        self.owner.as_ref() == Some(faction)
    }

    /// Set owner directly.
    pub fn set_owner(&mut self, faction: Option<FactionId>, tick: u64) {
        self.owner = faction;
        self.status = if self.owner.is_some() {
            OwnershipStatus::Owned
        } else {
            OwnershipStatus::Unclaimed
        };
        self.ownership_changed_tick = tick;
    }

    /// Add a claim.
    pub fn add_claim(&mut self, claim: Claim) {
        if !self.claims.iter().any(|c| c.faction == claim.faction) {
            self.claims.push(claim);
            self.claims.sort();
        }
    }

    /// Remove claims by a faction.
    pub fn remove_claims(&mut self, faction: &FactionId) {
        self.claims.retain(|c| &c.faction != faction);
    }

    /// Get claims.
    pub fn claims(&self) -> &[Claim] {
        &self.claims
    }

    /// Get the strongest claim.
    #[must_use]
    pub fn strongest_claim(&self) -> Option<&Claim> {
        self.claims.first()
    }

    /// Get influence for a faction.
    #[must_use]
    pub fn get_influence(&self, faction: &FactionId) -> u32 {
        self.influence.get(faction).map_or(0, Influence::total)
    }

    /// Get mutable influence for a faction.
    pub fn influence_mut(&mut self, faction: &FactionId) -> &mut Influence {
        self.influence.entry(faction.clone()).or_default()
    }

    /// Set base influence for a faction.
    pub fn set_influence(&mut self, faction: &FactionId, base: u32) {
        self.influence.entry(faction.clone()).or_default().base = base;
    }

    /// Get all factions with influence.
    pub fn factions_with_influence(&self) -> impl Iterator<Item = &FactionId> {
        self.influence
            .iter()
            .filter(|(_, i)| i.total() > 0)
            .map(|(f, _)| f)
    }

    /// Get dominant faction (highest influence).
    #[must_use]
    pub fn dominant_faction(&self) -> Option<&FactionId> {
        self.influence
            .iter()
            .max_by_key(|(_, i)| i.total())
            .filter(|(_, i)| i.total() > 0)
            .map(|(f, _)| f)
    }

    /// Calculate contestation level (0.0 = uncontested, 1.0 = heavily contested).
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "influence values bounded, precision loss acceptable for ratio"
    )]
    pub fn contestation_level(&self) -> f32 {
        let influences: Vec<u32> = self
            .influence
            .values()
            .map(Influence::total)
            .filter(|&i| i > 0)
            .collect();

        if influences.len() <= 1 {
            return 0.0;
        }

        let mut sorted = influences.clone();
        sorted.sort_by(|a, b| b.cmp(a));

        let highest = sorted[0] as f32;
        let second = sorted.get(1).copied().unwrap_or(0) as f32;

        if highest == 0.0 {
            return 0.0;
        }

        (second / highest).clamp(0.0, 1.0)
    }

    /// Update ownership status based on claims and influence.
    pub fn update_status(&mut self, tick: u64) {
        self.claims.retain(|c| !c.is_expired(tick));

        let contestation = self.contestation_level();
        let dominant = self.dominant_faction().cloned();

        if contestation > 0.8 {
            self.status = OwnershipStatus::Disputed;
        } else if contestation > 0.4 {
            self.status = OwnershipStatus::Contested;
        } else if let Some(faction) = dominant {
            if self.owner.as_ref() != Some(&faction) {
                self.owner = Some(faction);
                self.ownership_changed_tick = tick;
            }
            self.status = OwnershipStatus::Owned;
        } else {
            self.owner = None;
            self.status = OwnershipStatus::Unclaimed;
            self.ownership_changed_tick = tick;
        }
    }

    /// Decay temporary influence for all factions.
    pub fn decay_influence(&mut self, rate: f32) {
        for influence in self.influence.values_mut() {
            influence.decay_temporary(rate);
        }
    }

    /// Set metadata.
    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Get metadata.
    #[must_use]
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(String::as_str)
    }

    /// Get tick when ownership last changed.
    #[must_use]
    pub fn ownership_changed_tick(&self) -> u64 {
        self.ownership_changed_tick
    }
}

/// Map of all territories with query support.
#[derive(Clone, Debug, Default)]
pub struct TerritoryMap {
    regions: BTreeMap<RegionId, Region>,
    chunk_to_region: BTreeMap<(i32, i32, i32), RegionId>,
    current_tick: u64,
}

impl serde::Serialize for TerritoryMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("TerritoryMap", 2)?;
        state.serialize_field("regions", &self.regions)?;
        state.serialize_field("current_tick", &self.current_tick)?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for TerritoryMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct TerritoryMapData {
            regions: BTreeMap<RegionId, Region>,
            current_tick: u64,
        }

        let data = TerritoryMapData::deserialize(deserializer)?;
        let mut chunk_to_region = BTreeMap::new();
        for (id, region) in &data.regions {
            for chunk in region.chunks() {
                chunk_to_region.insert((chunk.x(), chunk.y(), chunk.z()), id.clone());
            }
        }

        Ok(Self {
            regions: data.regions,
            chunk_to_region,
            current_tick: data.current_tick,
        })
    }
}

impl TerritoryMap {
    /// Create an empty territory map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a region to the map.
    pub fn add_region(&mut self, region: Region) {
        for chunk in region.chunks() {
            self.chunk_to_region
                .insert((chunk.x(), chunk.y(), chunk.z()), region.id.clone());
        }
        self.regions.insert(region.id.clone(), region);
    }

    /// Remove a region from the map.
    pub fn remove_region(&mut self, id: &RegionId) -> Option<Region> {
        if let Some(region) = self.regions.remove(id) {
            for chunk in region.chunks() {
                self.chunk_to_region
                    .remove(&(chunk.x(), chunk.y(), chunk.z()));
            }
            Some(region)
        } else {
            None
        }
    }

    /// Get a region by ID.
    #[must_use]
    pub fn get(&self, id: &RegionId) -> Option<&Region> {
        self.regions.get(id)
    }

    /// Get mutable region.
    pub fn get_mut(&mut self, id: &RegionId) -> Option<&mut Region> {
        self.regions.get_mut(id)
    }

    /// Get region containing a chunk.
    #[must_use]
    pub fn region_at(&self, pos: &ChunkPos) -> Option<&Region> {
        self.chunk_to_region
            .get(&(pos.x(), pos.y(), pos.z()))
            .and_then(|id| self.regions.get(id))
    }

    /// Get mutable region containing a chunk.
    pub fn region_at_mut(&mut self, pos: &ChunkPos) -> Option<&mut Region> {
        let id = self
            .chunk_to_region
            .get(&(pos.x(), pos.y(), pos.z()))
            .cloned()?;
        self.regions.get_mut(&id)
    }

    /// Get owner of a chunk.
    #[must_use]
    pub fn owner_at(&self, pos: &ChunkPos) -> Option<&FactionId> {
        self.region_at(pos).and_then(Region::owner)
    }

    /// Check if a faction owns a chunk.
    #[must_use]
    pub fn is_owned_by(&self, pos: &ChunkPos, faction: &FactionId) -> bool {
        self.owner_at(pos) == Some(faction)
    }

    /// Check if chunk is contested.
    #[must_use]
    pub fn is_contested(&self, pos: &ChunkPos) -> bool {
        self.region_at(pos).is_some_and(|r| {
            matches!(
                r.status(),
                OwnershipStatus::Contested | OwnershipStatus::Disputed
            )
        })
    }

    /// Get number of regions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.regions.len()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }

    /// Iterate over all regions (deterministic order).
    pub fn iter(&self) -> impl Iterator<Item = &Region> {
        self.regions.values()
    }

    /// Iterate over regions mutably.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Region> {
        self.regions.values_mut()
    }

    /// Get regions owned by a faction.
    pub fn owned_by(&self, faction: &FactionId) -> impl Iterator<Item = &Region> {
        self.regions
            .values()
            .filter(move |r| r.is_owned_by(faction))
    }

    /// Count regions owned by a faction.
    #[must_use]
    pub fn count_owned_by(&self, faction: &FactionId) -> usize {
        self.owned_by(faction).count()
    }

    /// Get contested regions.
    pub fn contested(&self) -> impl Iterator<Item = &Region> {
        self.regions.values().filter(|r| {
            matches!(
                r.status(),
                OwnershipStatus::Contested | OwnershipStatus::Disputed
            )
        })
    }

    /// Get regions where a faction has influence.
    pub fn with_influence(&self, faction: &FactionId) -> impl Iterator<Item = &Region> {
        self.regions
            .values()
            .filter(move |r| r.get_influence(faction) > 0)
    }

    /// Get total influence of a faction across all regions.
    #[must_use]
    pub fn total_influence(&self, faction: &FactionId) -> u32 {
        self.regions
            .values()
            .map(|r| r.get_influence(faction))
            .sum()
    }

    /// Update all regions (claims, status).
    pub fn tick(&mut self) {
        self.current_tick += 1;
        let tick = self.current_tick;

        for region in self.regions.values_mut() {
            region.update_status(tick);
            region.decay_influence(0.01);
        }
    }

    /// Advance to a specific tick.
    pub fn advance_to(&mut self, tick: u64) {
        while self.current_tick < tick {
            self.tick();
        }
    }

    /// Get current tick.
    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Get all region IDs (deterministic order).
    pub fn ids(&self) -> impl Iterator<Item = &RegionId> {
        self.regions.keys()
    }

    /// Query: Can faction enter a chunk?
    #[must_use]
    pub fn can_enter(&self, pos: &ChunkPos, faction: &FactionId) -> bool {
        match self.region_at(pos) {
            Some(region) => {
                region.is_owned_by(faction)
                    || region.owner().is_none()
                    || region.get_influence(faction) > 0
            }
            None => true,
        }
    }

    /// Query: Can faction build at a chunk?
    #[must_use]
    pub fn can_build(&self, pos: &ChunkPos, faction: &FactionId) -> bool {
        match self.region_at(pos) {
            Some(region) => {
                region.is_owned_by(faction)
                    || (region.owner().is_none() && region.get_influence(faction) > 0)
            }
            None => true,
        }
    }

    /// Query: Can faction harvest at a chunk?
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "influence values bounded, precision loss acceptable for comparison"
    )]
    pub fn can_harvest(&self, pos: &ChunkPos, faction: &FactionId) -> bool {
        match self.region_at(pos) {
            Some(region) => {
                region.is_owned_by(faction)
                    || region.owner().is_none()
                    || (region.get_influence(faction) as f32
                        >= region.get_influence(region.owner().unwrap_or(faction)) as f32 * 0.5)
            }
            None => true,
        }
    }

    /// Create or get region for a chunk.
    ///
    /// # Panics
    ///
    /// Panics if the internal region map becomes inconsistent (should not happen).
    pub fn get_or_create_region(&mut self, pos: ChunkPos) -> &mut Region {
        let key = (pos.x(), pos.y(), pos.z());
        if !self.chunk_to_region.contains_key(&key) {
            let region = Region::from_chunk(pos);
            let id = region.id.clone();
            self.regions.insert(id.clone(), region);
            self.chunk_to_region.insert(key, id);
        }
        let id = self.chunk_to_region.get(&key).unwrap().clone();
        self.regions.get_mut(&id).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_id() {
        let id = RegionId::new("test_region");
        assert_eq!(id.as_str(), "test_region");

        let id = RegionId::from_chunk(ChunkPos::new(5, 0, -3));
        assert_eq!(id.as_str(), "chunk_5_-3");
    }

    #[test]
    fn test_claim_strength() {
        assert!(ClaimStrength::Absolute.base_influence() > ClaimStrength::Normal.base_influence());
        assert!(
            ClaimStrength::Strong.influence_multiplier()
                > ClaimStrength::Weak.influence_multiplier()
        );
    }

    #[test]
    fn test_claim_new() {
        let claim = Claim::new(FactionId::new("miners"), ClaimKind::Settlement, 100);
        assert_eq!(claim.faction.as_str(), "miners");
        assert_eq!(claim.strength, ClaimStrength::Normal);
        assert!(!claim.is_expired(100));
    }

    #[test]
    fn test_claim_expiration() {
        let claim =
            Claim::new(FactionId::new("test"), ClaimKind::Conquest, 100).with_expiration(200);

        assert!(!claim.is_expired(150));
        assert!(claim.is_expired(200));
        assert!(claim.is_expired(250));
    }

    #[test]
    fn test_claim_ordering() {
        let strong = Claim::new(FactionId::new("a"), ClaimKind::Settlement, 100)
            .with_strength(ClaimStrength::Strong);
        let weak = Claim::new(FactionId::new("b"), ClaimKind::Settlement, 100)
            .with_strength(ClaimStrength::Weak);

        assert!(strong < weak);
    }

    #[test]
    fn test_influence_new() {
        let inf = Influence::new(50);
        assert_eq!(inf.total(), 50);
    }

    #[test]
    fn test_influence_bonuses() {
        let mut inf = Influence::new(50);
        inf.add_structure_bonus(25);
        inf.add_population_bonus(15);

        assert_eq!(inf.total(), 90);
    }

    #[test]
    fn test_influence_temporary() {
        let mut inf = Influence::new(50);
        inf.apply_temporary(30, 1);
        assert_eq!(inf.total(), 80);

        inf.apply_temporary(-50, 2);
        assert_eq!(inf.total(), 30);
    }

    #[test]
    fn test_influence_decay() {
        let mut inf = Influence::new(50);
        inf.apply_temporary(100, 1);

        inf.decay_temporary(0.1);
        assert!(inf.temporary < 100);
    }

    #[test]
    fn test_region_new() {
        let region = Region::new(RegionId::new("test"));
        assert_eq!(region.id.as_str(), "test");
        assert!(region.owner().is_none());
        assert_eq!(region.status(), OwnershipStatus::Unclaimed);
    }

    #[test]
    fn test_region_chunks() {
        let mut region = Region::new(RegionId::new("test"));
        region.add_chunk(ChunkPos::new(0, 0, 0));
        region.add_chunk(ChunkPos::new(1, 0, 0));

        assert_eq!(region.chunk_count(), 2);
        assert!(region.contains_chunk(&ChunkPos::new(0, 0, 0)));
        assert!(!region.contains_chunk(&ChunkPos::new(2, 0, 0)));
    }

    #[test]
    fn test_region_ownership() {
        let mut region = Region::new(RegionId::new("test"));
        let faction = FactionId::new("miners");

        region.set_owner(Some(faction.clone()), 100);

        assert!(region.is_owned_by(&faction));
        assert_eq!(region.status(), OwnershipStatus::Owned);
    }

    #[test]
    fn test_region_claims() {
        let mut region = Region::new(RegionId::new("test"));

        let claim1 = Claim::new(FactionId::new("a"), ClaimKind::Settlement, 100)
            .with_strength(ClaimStrength::Normal);
        let claim2 = Claim::new(FactionId::new("b"), ClaimKind::Conquest, 100)
            .with_strength(ClaimStrength::Strong);

        region.add_claim(claim1);
        region.add_claim(claim2);

        assert_eq!(region.claims().len(), 2);
        assert_eq!(region.strongest_claim().unwrap().faction.as_str(), "b");
    }

    #[test]
    fn test_region_influence() {
        let mut region = Region::new(RegionId::new("test"));

        region.set_influence(&FactionId::new("a"), 100);
        region.set_influence(&FactionId::new("b"), 50);

        assert_eq!(region.get_influence(&FactionId::new("a")), 100);
        assert_eq!(region.get_influence(&FactionId::new("b")), 50);
        assert_eq!(region.get_influence(&FactionId::new("c")), 0);
    }

    #[test]
    fn test_region_dominant_faction() {
        let mut region = Region::new(RegionId::new("test"));

        region.set_influence(&FactionId::new("a"), 100);
        region.set_influence(&FactionId::new("b"), 50);

        assert_eq!(region.dominant_faction(), Some(&FactionId::new("a")));
    }

    #[test]
    fn test_region_contestation() {
        let mut region = Region::new(RegionId::new("test"));

        region.set_influence(&FactionId::new("a"), 100);
        assert!((region.contestation_level()).abs() < f32::EPSILON);

        region.set_influence(&FactionId::new("b"), 80);
        assert!(region.contestation_level() > 0.5);
    }

    #[test]
    fn test_region_update_status() {
        let mut region = Region::new(RegionId::new("test"));

        region.set_influence(&FactionId::new("a"), 100);
        region.set_influence(&FactionId::new("b"), 90);
        region.update_status(100);

        assert!(matches!(
            region.status(),
            OwnershipStatus::Contested | OwnershipStatus::Disputed
        ));
    }

    #[test]
    fn test_territory_map_basic() {
        let mut map = TerritoryMap::new();

        let region = Region::from_chunk(ChunkPos::new(0, 0, 0));
        map.add_region(region);

        assert_eq!(map.len(), 1);
        assert!(map.region_at(&ChunkPos::new(0, 0, 0)).is_some());
        assert!(map.region_at(&ChunkPos::new(1, 0, 0)).is_none());
    }

    #[test]
    fn test_territory_map_ownership() {
        let mut map = TerritoryMap::new();

        let mut region = Region::from_chunk(ChunkPos::new(0, 0, 0));
        region.set_owner(Some(FactionId::new("miners")), 0);
        map.add_region(region);

        assert_eq!(
            map.owner_at(&ChunkPos::new(0, 0, 0)),
            Some(&FactionId::new("miners"))
        );
        assert!(map.is_owned_by(&ChunkPos::new(0, 0, 0), &FactionId::new("miners")));
    }

    #[test]
    fn test_territory_map_queries() {
        let mut map = TerritoryMap::new();

        let mut region = Region::from_chunk(ChunkPos::new(0, 0, 0));
        region.set_owner(Some(FactionId::new("miners")), 0);
        map.add_region(region);

        let chunk = ChunkPos::new(0, 0, 0);

        assert!(map.can_enter(&chunk, &FactionId::new("miners")));
        assert!(map.can_build(&chunk, &FactionId::new("miners")));
        assert!(map.can_harvest(&chunk, &FactionId::new("miners")));

        assert!(!map.can_build(&chunk, &FactionId::new("bandits")));
    }

    #[test]
    fn test_territory_map_owned_by() {
        let mut map = TerritoryMap::new();

        let mut r1 = Region::from_chunk(ChunkPos::new(0, 0, 0));
        r1.set_owner(Some(FactionId::new("a")), 0);
        map.add_region(r1);

        let mut r2 = Region::from_chunk(ChunkPos::new(1, 0, 0));
        r2.set_owner(Some(FactionId::new("a")), 0);
        map.add_region(r2);

        let mut r3 = Region::from_chunk(ChunkPos::new(2, 0, 0));
        r3.set_owner(Some(FactionId::new("b")), 0);
        map.add_region(r3);

        assert_eq!(map.count_owned_by(&FactionId::new("a")), 2);
        assert_eq!(map.count_owned_by(&FactionId::new("b")), 1);
    }

    #[test]
    fn test_territory_map_deterministic_iteration() {
        let mut map = TerritoryMap::new();

        map.add_region(Region::new(RegionId::new("z")));
        map.add_region(Region::new(RegionId::new("a")));
        map.add_region(Region::new(RegionId::new("m")));

        let ids: Vec<_> = map.ids().map(RegionId::as_str).collect();
        assert_eq!(ids, vec!["a", "m", "z"]);
    }

    #[test]
    fn test_territory_map_tick() {
        let mut map = TerritoryMap::new();

        let mut region = Region::new(RegionId::new("test"));
        region.set_influence(&FactionId::new("a"), 100);
        region
            .influence_mut(&FactionId::new("a"))
            .apply_temporary(50, 0);
        map.add_region(region);

        let initial = map
            .get(&RegionId::new("test"))
            .unwrap()
            .get_influence(&FactionId::new("a"));

        for _ in 0..10 {
            map.tick();
        }

        let after = map
            .get(&RegionId::new("test"))
            .unwrap()
            .get_influence(&FactionId::new("a"));
        assert!(after <= initial);
    }

    #[test]
    fn test_territory_map_get_or_create() {
        let mut map = TerritoryMap::new();

        let region = map.get_or_create_region(ChunkPos::new(5, 0, 5));
        assert!(region.contains_chunk(&ChunkPos::new(5, 0, 5)));

        assert_eq!(map.len(), 1);

        let _ = map.get_or_create_region(ChunkPos::new(5, 0, 5));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_region_serde() {
        let mut region = Region::new(RegionId::new("test"));
        region.add_chunk(ChunkPos::new(0, 0, 0));
        region.set_owner(Some(FactionId::new("miners")), 100);
        region.add_claim(Claim::new(
            FactionId::new("miners"),
            ClaimKind::Settlement,
            100,
        ));
        region.set_influence(&FactionId::new("miners"), 75);

        let json = serde_json::to_string(&region).unwrap();
        let restored: Region = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, region.id);
        assert_eq!(restored.owner(), region.owner());
        assert_eq!(restored.chunk_count(), 1);
        assert_eq!(restored.claims().len(), 1);
    }

    #[test]
    fn test_territory_map_serde() {
        let mut map = TerritoryMap::new();

        let mut r1 = Region::from_chunk(ChunkPos::new(0, 0, 0));
        r1.set_owner(Some(FactionId::new("a")), 0);
        map.add_region(r1);

        let r2 = Region::from_chunk(ChunkPos::new(1, 0, 0));
        map.add_region(r2);

        let json = serde_json::to_string(&map).unwrap();
        let restored: TerritoryMap = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.len(), 2);
        assert!(restored.region_at(&ChunkPos::new(0, 0, 0)).is_some());
    }

    #[test]
    fn test_unowned_territory_access() {
        let mut map = TerritoryMap::new();

        let region = Region::from_chunk(ChunkPos::new(0, 0, 0));
        map.add_region(region);

        let chunk = ChunkPos::new(0, 0, 0);

        assert!(map.can_enter(&chunk, &FactionId::new("anyone")));
        assert!(!map.can_build(&chunk, &FactionId::new("anyone")));
    }

    #[test]
    fn test_influence_allows_entry() {
        let mut map = TerritoryMap::new();

        let mut region = Region::from_chunk(ChunkPos::new(0, 0, 0));
        region.set_owner(Some(FactionId::new("owner")), 0);
        region.set_influence(&FactionId::new("visitor"), 10);
        map.add_region(region);

        let chunk = ChunkPos::new(0, 0, 0);

        assert!(map.can_enter(&chunk, &FactionId::new("visitor")));
        assert!(!map.can_enter(&chunk, &FactionId::new("stranger")));
    }
}
