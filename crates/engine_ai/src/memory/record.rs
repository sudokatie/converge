//! Memory record types for danger zones, food sources, and player traces.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Unique identifier for a memory.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MemoryId(pub u64);

impl MemoryId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Source of a memory observation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MemorySource {
    /// Direct observation by this creature.
    DirectObservation,
    /// Sensed via smell.
    Scent,
    /// Sensed via hearing.
    Sound,
    /// Sensed via vibration.
    Vibration,
    /// Communicated by another creature.
    Social { from_creature_id: u64 },
    /// Innate knowledge.
    Instinct,
    /// External system injection.
    Injected,
}

impl MemorySource {
    #[must_use]
    pub fn reliability(&self) -> f32 {
        match self {
            Self::DirectObservation | Self::Injected => 1.0,
            Self::Instinct => 0.9,
            Self::Sound => 0.8,
            Self::Vibration => 0.7,
            Self::Scent => 0.6,
            Self::Social { .. } => 0.5,
        }
    }
}

/// Tag for categorizing memories.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MemoryTag(pub String);

impl MemoryTag {
    #[must_use]
    pub fn new(tag: impl Into<String>) -> Self {
        Self(tag.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn urgent() -> Self {
        Self::new("urgent")
    }

    #[must_use]
    pub fn verified() -> Self {
        Self::new("verified")
    }

    #[must_use]
    pub fn uncertain() -> Self {
        Self::new("uncertain")
    }

    #[must_use]
    pub fn temporary() -> Self {
        Self::new("temporary")
    }
}

impl<T: Into<String>> From<T> for MemoryTag {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

/// Region scope for a memory.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RegionScope {
    /// Region identifier (matches offline region IDs).
    pub region_id: String,
    /// Optional sub-region for finer granularity.
    pub sub_region: Option<String>,
}

impl RegionScope {
    #[must_use]
    pub fn new(region_id: impl Into<String>) -> Self {
        Self {
            region_id: region_id.into(),
            sub_region: None,
        }
    }

    #[must_use]
    pub fn with_sub_region(mut self, sub: impl Into<String>) -> Self {
        self.sub_region = Some(sub.into());
        self
    }

    #[must_use]
    pub fn global() -> Self {
        Self::new("global")
    }

    #[must_use]
    pub fn matches(&self, other: &Self) -> bool {
        if self.region_id != other.region_id {
            return false;
        }
        match (&self.sub_region, &other.sub_region) {
            (None, _) | (_, None) => true,
            (Some(a), Some(b)) => a == b,
        }
    }
}

/// Category of memory for filtering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MemoryCategory {
    DangerZone,
    FoodSource,
    PlayerTrace,
}

/// Common trait for memory records.
pub trait MemoryRecord {
    fn id(&self) -> &MemoryId;
    fn category(&self) -> MemoryCategory;
    fn position(&self) -> [f32; 3];
    fn strength(&self) -> f32;
    fn confidence(&self) -> f32;
    fn created_tick(&self) -> u64;
    fn last_refresh_tick(&self) -> u64;
    fn region(&self) -> Option<&RegionScope>;
    fn source(&self) -> &MemorySource;
    fn tags(&self) -> &[MemoryTag];

    fn effective_strength(&self) -> f32 {
        self.strength() * self.confidence()
    }

    fn age(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.created_tick())
    }

    fn staleness(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.last_refresh_tick())
    }
}

/// Category of danger.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DangerCategory {
    Predator,
    Trap,
    EnvironmentalHazard,
    PlayerActivity,
    TerritoryBoundary,
    UnknownThreat,
}

impl DangerCategory {
    #[must_use]
    pub fn base_priority(&self) -> f32 {
        match self {
            Self::Predator => 1.0,
            Self::PlayerActivity => 0.9,
            Self::Trap => 0.85,
            Self::EnvironmentalHazard => 0.7,
            Self::TerritoryBoundary => 0.5,
            Self::UnknownThreat => 0.6,
        }
    }
}

/// Memory of a danger zone.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DangerZoneMemory {
    pub id: MemoryId,
    pub position: [f32; 3],
    pub radius: f32,
    pub category: DangerCategory,
    pub strength: f32,
    pub confidence: f32,
    pub created_tick: u64,
    pub last_refresh_tick: u64,
    pub region: Option<RegionScope>,
    pub source: MemorySource,
    pub tags: Vec<MemoryTag>,
    pub threat_entity_id: Option<u64>,
    pub last_observed_direction: Option<[f32; 3]>,
}

impl DangerZoneMemory {
    #[must_use]
    pub fn new(
        id: MemoryId,
        position: [f32; 3],
        radius: f32,
        category: DangerCategory,
        strength: f32,
        source: MemorySource,
        tick: u64,
    ) -> Self {
        let confidence = source.reliability();
        Self {
            id,
            position,
            radius: radius.max(0.0),
            category,
            strength: strength.clamp(0.0, 1.0),
            confidence,
            created_tick: tick,
            last_refresh_tick: tick,
            region: None,
            source,
            tags: Vec::new(),
            threat_entity_id: None,
            last_observed_direction: None,
        }
    }

    #[must_use]
    pub fn with_region(mut self, region: RegionScope) -> Self {
        self.region = Some(region);
        self
    }

    #[must_use]
    pub fn with_threat_entity(mut self, entity_id: u64) -> Self {
        self.threat_entity_id = Some(entity_id);
        self
    }

    #[must_use]
    pub fn with_direction(mut self, direction: [f32; 3]) -> Self {
        self.last_observed_direction = Some(direction);
        self
    }

    pub fn add_tag(&mut self, tag: MemoryTag) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            self.tags.sort();
        }
    }

    pub fn refresh(&mut self, tick: u64, new_strength: f32, new_confidence: f32) {
        self.last_refresh_tick = tick;
        self.strength = f32::midpoint(self.strength, new_strength);
        self.confidence = self.confidence.max(new_confidence);
    }

    pub fn apply_decay(&mut self, decay_rate: f32) {
        self.strength *= decay_rate;
        self.confidence *= decay_rate.powf(0.5);
    }

    #[must_use]
    pub fn priority(&self) -> f32 {
        self.category.base_priority() * self.effective_strength()
    }

    #[must_use]
    pub fn effective_strength(&self) -> f32 {
        self.strength * self.confidence
    }

    #[must_use]
    pub fn contains_point(&self, point: [f32; 3]) -> bool {
        let dx = point[0] - self.position[0];
        let dy = point[1] - self.position[1];
        let dz = point[2] - self.position[2];
        let dist_sq = dx * dx + dy * dy + dz * dz;
        dist_sq <= self.radius * self.radius
    }

    #[must_use]
    pub fn distance_to(&self, point: [f32; 3]) -> f32 {
        let dx = point[0] - self.position[0];
        let dy = point[1] - self.position[1];
        let dz = point[2] - self.position[2];
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

impl MemoryRecord for DangerZoneMemory {
    fn id(&self) -> &MemoryId {
        &self.id
    }
    fn category(&self) -> MemoryCategory {
        MemoryCategory::DangerZone
    }
    fn position(&self) -> [f32; 3] {
        self.position
    }
    fn strength(&self) -> f32 {
        self.strength
    }
    fn confidence(&self) -> f32 {
        self.confidence
    }
    fn created_tick(&self) -> u64 {
        self.created_tick
    }
    fn last_refresh_tick(&self) -> u64 {
        self.last_refresh_tick
    }
    fn region(&self) -> Option<&RegionScope> {
        self.region.as_ref()
    }
    fn source(&self) -> &MemorySource {
        &self.source
    }
    fn tags(&self) -> &[MemoryTag] {
        &self.tags
    }
}

impl Eq for DangerZoneMemory {}

impl PartialOrd for DangerZoneMemory {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DangerZoneMemory {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .priority()
            .partial_cmp(&self.priority())
            .unwrap_or(Ordering::Equal)
            .then_with(|| self.category.cmp(&other.category))
            .then_with(|| self.created_tick.cmp(&other.created_tick))
            .then_with(|| self.id.cmp(&other.id))
    }
}

/// Category of food source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FoodCategory {
    Plant,
    Prey,
    Carrion,
    Fruit,
    Fungi,
    Water,
    Mineral,
}

impl FoodCategory {
    #[must_use]
    pub fn base_value(&self) -> f32 {
        match self {
            Self::Prey => 1.0,
            Self::Carrion => 0.7,
            Self::Fruit => 0.6,
            Self::Plant => 0.5,
            Self::Fungi => 0.4,
            Self::Water => 0.8,
            Self::Mineral => 0.3,
        }
    }
}

/// Memory of a food source.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FoodSourceMemory {
    pub id: MemoryId,
    pub position: [f32; 3],
    pub category: FoodCategory,
    pub strength: f32,
    pub confidence: f32,
    pub created_tick: u64,
    pub last_refresh_tick: u64,
    pub region: Option<RegionScope>,
    pub source: MemorySource,
    pub tags: Vec<MemoryTag>,
    pub estimated_quantity: f32,
    pub quality: f32,
    pub is_depleted: bool,
}

impl FoodSourceMemory {
    #[must_use]
    pub fn new(
        id: MemoryId,
        position: [f32; 3],
        category: FoodCategory,
        strength: f32,
        source: MemorySource,
        tick: u64,
    ) -> Self {
        let confidence = source.reliability();
        Self {
            id,
            position,
            category,
            strength: strength.clamp(0.0, 1.0),
            confidence,
            created_tick: tick,
            last_refresh_tick: tick,
            region: None,
            source,
            tags: Vec::new(),
            estimated_quantity: 1.0,
            quality: 1.0,
            is_depleted: false,
        }
    }

    #[must_use]
    pub fn with_region(mut self, region: RegionScope) -> Self {
        self.region = Some(region);
        self
    }

    #[must_use]
    pub fn with_quantity(mut self, quantity: f32) -> Self {
        self.estimated_quantity = quantity.max(0.0);
        self
    }

    #[must_use]
    pub fn with_quality(mut self, quality: f32) -> Self {
        self.quality = quality.clamp(0.0, 1.0);
        self
    }

    pub fn add_tag(&mut self, tag: MemoryTag) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            self.tags.sort();
        }
    }

    pub fn refresh(&mut self, tick: u64, new_strength: f32, new_quantity: f32) {
        self.last_refresh_tick = tick;
        self.strength = f32::midpoint(self.strength, new_strength);
        self.estimated_quantity = new_quantity;
        self.is_depleted = new_quantity <= 0.0;
    }

    pub fn apply_decay(&mut self, decay_rate: f32) {
        self.strength *= decay_rate;
        self.confidence *= decay_rate.powf(0.5);
    }

    pub fn mark_depleted(&mut self) {
        self.is_depleted = true;
        self.estimated_quantity = 0.0;
        self.strength *= 0.5;
    }

    #[must_use]
    pub fn value(&self) -> f32 {
        if self.is_depleted {
            return 0.0;
        }
        self.category.base_value()
            * self.effective_strength()
            * self.quality
            * self.estimated_quantity.min(1.0)
    }

    #[must_use]
    pub fn effective_strength(&self) -> f32 {
        self.strength * self.confidence
    }

    #[must_use]
    pub fn distance_to(&self, point: [f32; 3]) -> f32 {
        let dx = point[0] - self.position[0];
        let dy = point[1] - self.position[1];
        let dz = point[2] - self.position[2];
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

impl MemoryRecord for FoodSourceMemory {
    fn id(&self) -> &MemoryId {
        &self.id
    }
    fn category(&self) -> MemoryCategory {
        MemoryCategory::FoodSource
    }
    fn position(&self) -> [f32; 3] {
        self.position
    }
    fn strength(&self) -> f32 {
        self.strength
    }
    fn confidence(&self) -> f32 {
        self.confidence
    }
    fn created_tick(&self) -> u64 {
        self.created_tick
    }
    fn last_refresh_tick(&self) -> u64 {
        self.last_refresh_tick
    }
    fn region(&self) -> Option<&RegionScope> {
        self.region.as_ref()
    }
    fn source(&self) -> &MemorySource {
        &self.source
    }
    fn tags(&self) -> &[MemoryTag] {
        &self.tags
    }
}

impl Eq for FoodSourceMemory {}

impl PartialOrd for FoodSourceMemory {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FoodSourceMemory {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .value()
            .partial_cmp(&self.value())
            .unwrap_or(Ordering::Equal)
            .then_with(|| self.category.cmp(&other.category))
            .then_with(|| self.created_tick.cmp(&other.created_tick))
            .then_with(|| self.id.cmp(&other.id))
    }
}

/// Kind of player trace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PlayerTraceKind {
    Scent,
    Footstep,
    Noise,
    VisualSighting,
    DisturbedEnvironment,
    Equipment,
}

impl PlayerTraceKind {
    #[must_use]
    pub fn base_threat(&self) -> f32 {
        match self {
            Self::VisualSighting => 1.0,
            Self::Noise => 0.8,
            Self::Footstep => 0.6,
            Self::Scent => 0.5,
            Self::Equipment => 0.7,
            Self::DisturbedEnvironment => 0.3,
        }
    }

    #[must_use]
    pub fn typical_decay_rate(&self) -> f32 {
        match self {
            Self::VisualSighting => 0.9,
            Self::Noise => 0.85,
            Self::Footstep => 0.95,
            Self::Scent => 0.98,
            Self::Equipment => 0.99,
            Self::DisturbedEnvironment => 0.995,
        }
    }
}

/// Memory of a player trace (scent, noise, footsteps, etc.).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerTraceMemory {
    pub id: MemoryId,
    pub position: [f32; 3],
    pub kind: PlayerTraceKind,
    pub strength: f32,
    pub confidence: f32,
    pub created_tick: u64,
    pub last_refresh_tick: u64,
    pub region: Option<RegionScope>,
    pub source: MemorySource,
    pub tags: Vec<MemoryTag>,
    pub estimated_direction: Option<[f32; 3]>,
    pub intensity: f32,
    pub player_id: Option<u64>,
}

impl PlayerTraceMemory {
    #[must_use]
    pub fn new(
        id: MemoryId,
        position: [f32; 3],
        kind: PlayerTraceKind,
        strength: f32,
        source: MemorySource,
        tick: u64,
    ) -> Self {
        let confidence = source.reliability();
        Self {
            id,
            position,
            kind,
            strength: strength.clamp(0.0, 1.0),
            confidence,
            created_tick: tick,
            last_refresh_tick: tick,
            region: None,
            source,
            tags: Vec::new(),
            estimated_direction: None,
            intensity: 1.0,
            player_id: None,
        }
    }

    #[must_use]
    pub fn with_region(mut self, region: RegionScope) -> Self {
        self.region = Some(region);
        self
    }

    #[must_use]
    pub fn with_direction(mut self, direction: [f32; 3]) -> Self {
        self.estimated_direction = Some(direction);
        self
    }

    #[must_use]
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_player_id(mut self, player_id: u64) -> Self {
        self.player_id = Some(player_id);
        self
    }

    pub fn add_tag(&mut self, tag: MemoryTag) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            self.tags.sort();
        }
    }

    pub fn refresh(&mut self, tick: u64, new_strength: f32, new_direction: Option<[f32; 3]>) {
        self.last_refresh_tick = tick;
        self.strength = f32::midpoint(self.strength, new_strength);
        if new_direction.is_some() {
            self.estimated_direction = new_direction;
        }
    }

    pub fn apply_decay(&mut self, decay_rate: f32) {
        let effective_decay = decay_rate.min(self.kind.typical_decay_rate());
        self.strength *= effective_decay;
        self.confidence *= effective_decay.powf(0.5);
        self.intensity *= effective_decay;
    }

    #[must_use]
    pub fn threat_level(&self) -> f32 {
        self.kind.base_threat() * self.effective_strength() * self.intensity
    }

    #[must_use]
    pub fn effective_strength(&self) -> f32 {
        self.strength * self.confidence
    }

    #[must_use]
    pub fn distance_to(&self, point: [f32; 3]) -> f32 {
        let dx = point[0] - self.position[0];
        let dy = point[1] - self.position[1];
        let dz = point[2] - self.position[2];
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    #[must_use]
    pub fn is_recent(&self, current_tick: u64, threshold: u64) -> bool {
        self.staleness(current_tick) <= threshold
    }

    fn staleness(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.last_refresh_tick)
    }
}

impl MemoryRecord for PlayerTraceMemory {
    fn id(&self) -> &MemoryId {
        &self.id
    }
    fn category(&self) -> MemoryCategory {
        MemoryCategory::PlayerTrace
    }
    fn position(&self) -> [f32; 3] {
        self.position
    }
    fn strength(&self) -> f32 {
        self.strength
    }
    fn confidence(&self) -> f32 {
        self.confidence
    }
    fn created_tick(&self) -> u64 {
        self.created_tick
    }
    fn last_refresh_tick(&self) -> u64 {
        self.last_refresh_tick
    }
    fn region(&self) -> Option<&RegionScope> {
        self.region.as_ref()
    }
    fn source(&self) -> &MemorySource {
        &self.source
    }
    fn tags(&self) -> &[MemoryTag] {
        &self.tags
    }
}

impl Eq for PlayerTraceMemory {}

impl PartialOrd for PlayerTraceMemory {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PlayerTraceMemory {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .threat_level()
            .partial_cmp(&self.threat_level())
            .unwrap_or(Ordering::Equal)
            .then_with(|| self.kind.cmp(&other.kind))
            .then_with(|| self.created_tick.cmp(&other.created_tick))
            .then_with(|| self.id.cmp(&other.id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_id() {
        let id = MemoryId::new(42);
        assert_eq!(id.0, 42);
    }

    #[test]
    fn test_memory_source_reliability() {
        assert!((MemorySource::DirectObservation.reliability() - 1.0).abs() < f32::EPSILON);
        assert!(MemorySource::Scent.reliability() < MemorySource::Sound.reliability());
    }

    #[test]
    fn test_region_scope() {
        let global = RegionScope::global();
        assert_eq!(global.region_id, "global");
        assert!(global.sub_region.is_none());

        let sub = RegionScope::new("forest").with_sub_region("clearing");
        assert_eq!(sub.region_id, "forest");
        assert_eq!(sub.sub_region.as_deref(), Some("clearing"));
    }

    #[test]
    fn test_region_scope_matches() {
        let r1 = RegionScope::new("forest");
        let r2 = RegionScope::new("forest").with_sub_region("clearing");
        let r3 = RegionScope::new("desert");

        assert!(r1.matches(&r2));
        assert!(r2.matches(&r1));
        assert!(!r1.matches(&r3));
    }

    #[test]
    fn test_danger_zone_memory_new() {
        let mem = DangerZoneMemory::new(
            MemoryId::new(1),
            [10.0, 20.0, 0.0],
            5.0,
            DangerCategory::Predator,
            0.8,
            MemorySource::DirectObservation,
            100,
        );

        assert_eq!(mem.id.0, 1);
        assert!((mem.strength - 0.8).abs() < f32::EPSILON);
        assert!((mem.confidence - 1.0).abs() < f32::EPSILON);
        assert!((mem.radius - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_danger_zone_contains_point() {
        let mem = DangerZoneMemory::new(
            MemoryId::new(1),
            [0.0, 0.0, 0.0],
            10.0,
            DangerCategory::Trap,
            1.0,
            MemorySource::DirectObservation,
            0,
        );

        assert!(mem.contains_point([5.0, 0.0, 0.0]));
        assert!(!mem.contains_point([15.0, 0.0, 0.0]));
    }

    #[test]
    fn test_danger_zone_decay() {
        let mut mem = DangerZoneMemory::new(
            MemoryId::new(1),
            [0.0, 0.0, 0.0],
            5.0,
            DangerCategory::Predator,
            1.0,
            MemorySource::DirectObservation,
            0,
        );

        mem.apply_decay(0.9);
        assert!((mem.strength - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn test_danger_zone_refresh() {
        let mut mem = DangerZoneMemory::new(
            MemoryId::new(1),
            [0.0, 0.0, 0.0],
            5.0,
            DangerCategory::Predator,
            0.5,
            MemorySource::DirectObservation,
            0,
        );

        mem.refresh(100, 0.9, 1.0);
        assert_eq!(mem.last_refresh_tick, 100);
        assert!((mem.strength - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn test_danger_zone_ordering() {
        let high = DangerZoneMemory::new(
            MemoryId::new(1),
            [0.0, 0.0, 0.0],
            5.0,
            DangerCategory::Predator,
            1.0,
            MemorySource::DirectObservation,
            0,
        );
        let low = DangerZoneMemory::new(
            MemoryId::new(2),
            [0.0, 0.0, 0.0],
            5.0,
            DangerCategory::Trap,
            0.5,
            MemorySource::DirectObservation,
            0,
        );

        assert!(high < low);
    }

    #[test]
    fn test_food_source_memory_new() {
        let mem = FoodSourceMemory::new(
            MemoryId::new(1),
            [10.0, 20.0, 0.0],
            FoodCategory::Fruit,
            0.9,
            MemorySource::DirectObservation,
            100,
        );

        assert!((mem.strength - 0.9).abs() < f32::EPSILON);
        assert!(!mem.is_depleted);
    }

    #[test]
    fn test_food_source_depleted() {
        let mut mem = FoodSourceMemory::new(
            MemoryId::new(1),
            [0.0, 0.0, 0.0],
            FoodCategory::Plant,
            1.0,
            MemorySource::DirectObservation,
            0,
        );

        mem.mark_depleted();
        assert!(mem.is_depleted);
        assert!(mem.value().abs() < f32::EPSILON);
    }

    #[test]
    fn test_food_source_ordering() {
        let high = FoodSourceMemory::new(
            MemoryId::new(1),
            [0.0, 0.0, 0.0],
            FoodCategory::Prey,
            1.0,
            MemorySource::DirectObservation,
            0,
        );
        let low = FoodSourceMemory::new(
            MemoryId::new(2),
            [0.0, 0.0, 0.0],
            FoodCategory::Fungi,
            1.0,
            MemorySource::DirectObservation,
            0,
        );

        assert!(high < low);
    }

    #[test]
    fn test_player_trace_memory_new() {
        let mem = PlayerTraceMemory::new(
            MemoryId::new(1),
            [10.0, 20.0, 0.0],
            PlayerTraceKind::Scent,
            0.7,
            MemorySource::Scent,
            100,
        );

        assert!((mem.strength - 0.7).abs() < f32::EPSILON);
        assert!(mem.estimated_direction.is_none());
    }

    #[test]
    fn test_player_trace_with_direction() {
        let mem = PlayerTraceMemory::new(
            MemoryId::new(1),
            [0.0, 0.0, 0.0],
            PlayerTraceKind::Footstep,
            1.0,
            MemorySource::DirectObservation,
            0,
        )
        .with_direction([1.0, 0.0, 0.0]);

        assert!(mem.estimated_direction.is_some());
        let dir = mem.estimated_direction.unwrap();
        assert!((dir[0] - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_player_trace_threat_level() {
        let visual = PlayerTraceMemory::new(
            MemoryId::new(1),
            [0.0, 0.0, 0.0],
            PlayerTraceKind::VisualSighting,
            1.0,
            MemorySource::DirectObservation,
            0,
        );
        let scent = PlayerTraceMemory::new(
            MemoryId::new(2),
            [0.0, 0.0, 0.0],
            PlayerTraceKind::Scent,
            1.0,
            MemorySource::Scent,
            0,
        );

        assert!(visual.threat_level() > scent.threat_level());
    }

    #[test]
    fn test_player_trace_ordering() {
        let high = PlayerTraceMemory::new(
            MemoryId::new(1),
            [0.0, 0.0, 0.0],
            PlayerTraceKind::VisualSighting,
            1.0,
            MemorySource::DirectObservation,
            0,
        );
        let low = PlayerTraceMemory::new(
            MemoryId::new(2),
            [0.0, 0.0, 0.0],
            PlayerTraceKind::DisturbedEnvironment,
            1.0,
            MemorySource::DirectObservation,
            0,
        );

        assert!(high < low);
    }

    #[test]
    fn test_memory_tag() {
        let tag = MemoryTag::new("custom");
        assert_eq!(tag.as_str(), "custom");

        let urgent = MemoryTag::urgent();
        assert_eq!(urgent.as_str(), "urgent");
    }

    #[test]
    fn test_danger_zone_serde() {
        let mem = DangerZoneMemory::new(
            MemoryId::new(42),
            [1.0, 2.0, 3.0],
            10.0,
            DangerCategory::Predator,
            0.8,
            MemorySource::DirectObservation,
            100,
        )
        .with_region(RegionScope::new("forest"));

        let json = serde_json::to_string(&mem).unwrap();
        let restored: DangerZoneMemory = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, mem.id);
        assert!((restored.strength - mem.strength).abs() < f32::EPSILON);
        assert!(restored.region.is_some());
    }

    #[test]
    fn test_food_source_serde() {
        let mem = FoodSourceMemory::new(
            MemoryId::new(42),
            [1.0, 2.0, 3.0],
            FoodCategory::Fruit,
            0.9,
            MemorySource::DirectObservation,
            100,
        )
        .with_quantity(5.0)
        .with_quality(0.8);

        let json = serde_json::to_string(&mem).unwrap();
        let restored: FoodSourceMemory = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, mem.id);
        assert!((restored.estimated_quantity - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_player_trace_serde() {
        let mem = PlayerTraceMemory::new(
            MemoryId::new(42),
            [1.0, 2.0, 3.0],
            PlayerTraceKind::Footstep,
            0.7,
            MemorySource::Sound,
            100,
        )
        .with_direction([1.0, 0.0, 0.0])
        .with_player_id(999);

        let json = serde_json::to_string(&mem).unwrap();
        let restored: PlayerTraceMemory = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, mem.id);
        assert_eq!(restored.player_id, Some(999));
    }

    #[test]
    fn test_memory_record_trait() {
        let danger = DangerZoneMemory::new(
            MemoryId::new(1),
            [0.0, 0.0, 0.0],
            5.0,
            DangerCategory::Trap,
            0.8,
            MemorySource::DirectObservation,
            100,
        );

        assert_eq!(danger.category(), MemoryCategory::DangerZone);
        assert_eq!(danger.created_tick(), 100);
        assert!(danger.age(150) == 50);
    }
}
