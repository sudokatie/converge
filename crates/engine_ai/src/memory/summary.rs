//! Snapshots, fingerprints, and summaries for creature memory.

use super::record::{DangerCategory, FoodCategory, PlayerTraceKind, RegionScope};
use super::store::CreatureMemory;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Stable fingerprint for change detection.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryFingerprint {
    pub checksum: u32,
    pub danger_count: u32,
    pub food_count: u32,
    pub trace_count: u32,
    pub computed_at_tick: u64,
}

impl MemoryFingerprint {
    #[must_use]
    pub fn from_memory(memory: &CreatureMemory) -> Self {
        let mut hasher = crc32fast::Hasher::new();

        for danger in memory.iter_dangers() {
            hasher.update(&danger.id.0.to_le_bytes());
            hasher.update(&danger.position[0].to_le_bytes());
            hasher.update(&danger.position[1].to_le_bytes());
            hasher.update(&danger.position[2].to_le_bytes());
            hasher.update(&danger.strength.to_le_bytes());
        }

        for food in memory.iter_foods() {
            hasher.update(&food.id.0.to_le_bytes());
            hasher.update(&food.position[0].to_le_bytes());
            hasher.update(&food.position[1].to_le_bytes());
            hasher.update(&food.position[2].to_le_bytes());
            hasher.update(&food.strength.to_le_bytes());
        }

        for trace in memory.iter_traces() {
            hasher.update(&trace.id.0.to_le_bytes());
            hasher.update(&trace.position[0].to_le_bytes());
            hasher.update(&trace.position[1].to_le_bytes());
            hasher.update(&trace.position[2].to_le_bytes());
            hasher.update(&trace.strength.to_le_bytes());
        }

        Self {
            checksum: hasher.finalize(),
            danger_count: u32::try_from(memory.danger_count()).unwrap_or(u32::MAX),
            food_count: u32::try_from(memory.food_count()).unwrap_or(u32::MAX),
            trace_count: u32::try_from(memory.trace_count()).unwrap_or(u32::MAX),
            computed_at_tick: memory.current_tick(),
        }
    }

    #[must_use]
    pub fn from_snapshot(snapshot: &MemorySnapshot) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&snapshot.snapshot_tick.to_le_bytes());
        hasher.update(&snapshot.total_danger_strength.to_le_bytes());
        hasher.update(&snapshot.total_food_value.to_le_bytes());
        hasher.update(&snapshot.total_trace_threat.to_le_bytes());

        Self {
            checksum: hasher.finalize(),
            danger_count: u32::try_from(snapshot.danger_count).unwrap_or(u32::MAX),
            food_count: u32::try_from(snapshot.food_count).unwrap_or(u32::MAX),
            trace_count: u32::try_from(snapshot.trace_count).unwrap_or(u32::MAX),
            computed_at_tick: snapshot.snapshot_tick,
        }
    }

    #[must_use]
    pub fn matches(&self, other: &Self) -> bool {
        self.checksum == other.checksum
            && self.danger_count == other.danger_count
            && self.food_count == other.food_count
            && self.trace_count == other.trace_count
    }

    #[must_use]
    pub fn total_count(&self) -> u32 {
        self.danger_count + self.food_count + self.trace_count
    }
}

/// Lightweight summary of a creature's memory state.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MemorySummary {
    pub danger_count: usize,
    pub food_count: usize,
    pub trace_count: usize,
    pub total_danger_strength: f32,
    pub total_food_value: f32,
    pub total_trace_threat: f32,
    pub strongest_danger_priority: f32,
    pub strongest_food_value: f32,
    pub strongest_trace_threat: f32,
    pub average_danger_age: f32,
    pub average_food_age: f32,
    pub average_trace_age: f32,
    pub needs_attention: bool,
    pub computed_at_tick: u64,
}

impl MemorySummary {
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "count bounded")]
    pub fn from_memory(memory: &CreatureMemory) -> Self {
        let current_tick = memory.current_tick();

        let danger_count = memory.danger_count();
        let food_count = memory.food_count();
        let trace_count = memory.trace_count();

        let total_danger_strength: f32 = memory
            .iter_dangers()
            .map(super::record::DangerZoneMemory::priority)
            .sum();
        let total_food_value: f32 = memory
            .iter_foods()
            .map(super::record::FoodSourceMemory::value)
            .sum();
        let total_trace_threat: f32 = memory
            .iter_traces()
            .map(super::record::PlayerTraceMemory::threat_level)
            .sum();

        let strongest_danger_priority = memory
            .strongest_danger()
            .map_or(0.0, super::record::DangerZoneMemory::priority);
        let strongest_food_value = memory
            .strongest_food()
            .map_or(0.0, super::record::FoodSourceMemory::value);
        let strongest_trace_threat = memory
            .strongest_trace()
            .map_or(0.0, super::record::PlayerTraceMemory::threat_level);

        let average_danger_age = if danger_count > 0 {
            memory
                .iter_dangers()
                .map(|d| current_tick.saturating_sub(d.created_tick) as f32)
                .sum::<f32>()
                / danger_count as f32
        } else {
            0.0
        };

        let average_food_age = if food_count > 0 {
            memory
                .iter_foods()
                .map(|f| current_tick.saturating_sub(f.created_tick) as f32)
                .sum::<f32>()
                / food_count as f32
        } else {
            0.0
        };

        let average_trace_age = if trace_count > 0 {
            memory
                .iter_traces()
                .map(|t| current_tick.saturating_sub(t.created_tick) as f32)
                .sum::<f32>()
                / trace_count as f32
        } else {
            0.0
        };

        let needs_attention = strongest_danger_priority > 0.7 || strongest_trace_threat > 0.5;

        Self {
            danger_count,
            food_count,
            trace_count,
            total_danger_strength,
            total_food_value,
            total_trace_threat,
            strongest_danger_priority,
            strongest_food_value,
            strongest_trace_threat,
            average_danger_age,
            average_food_age,
            average_trace_age,
            needs_attention,
            computed_at_tick: current_tick,
        }
    }

    #[must_use]
    pub fn total_count(&self) -> usize {
        self.danger_count + self.food_count + self.trace_count
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.total_count() == 0
    }

    #[must_use]
    pub fn overall_threat(&self) -> f32 {
        self.strongest_danger_priority
            .max(self.strongest_trace_threat)
    }
}

/// Full snapshot for serialization/offline storage.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemorySnapshot {
    pub snapshot_tick: u64,
    pub danger_count: usize,
    pub food_count: usize,
    pub trace_count: usize,
    pub total_danger_strength: f32,
    pub total_food_value: f32,
    pub total_trace_threat: f32,
    pub danger_by_category: BTreeMap<DangerCategory, u32>,
    pub food_by_category: BTreeMap<FoodCategory, u32>,
    pub traces_by_kind: BTreeMap<PlayerTraceKind, u32>,
    pub regions_covered: Vec<String>,
    pub fingerprint: MemoryFingerprint,
}

impl MemorySnapshot {
    #[must_use]
    pub fn from_memory(memory: &CreatureMemory) -> Self {
        let mut danger_by_category: BTreeMap<DangerCategory, u32> = BTreeMap::new();
        let mut food_by_category: BTreeMap<FoodCategory, u32> = BTreeMap::new();
        let mut traces_by_kind: BTreeMap<PlayerTraceKind, u32> = BTreeMap::new();
        let mut regions: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

        let mut total_danger_strength = 0.0f32;
        for danger in memory.iter_dangers() {
            *danger_by_category.entry(danger.category).or_insert(0) += 1;
            total_danger_strength += danger.priority();
            if let Some(region) = &danger.region {
                regions.insert(region.region_id.clone());
            }
        }

        let mut total_food_value = 0.0f32;
        for food in memory.iter_foods() {
            *food_by_category.entry(food.category).or_insert(0) += 1;
            total_food_value += food.value();
            if let Some(region) = &food.region {
                regions.insert(region.region_id.clone());
            }
        }

        let mut total_trace_threat = 0.0f32;
        for trace in memory.iter_traces() {
            *traces_by_kind.entry(trace.kind).or_insert(0) += 1;
            total_trace_threat += trace.threat_level();
            if let Some(region) = &trace.region {
                regions.insert(region.region_id.clone());
            }
        }

        let fingerprint = MemoryFingerprint::from_memory(memory);

        Self {
            snapshot_tick: memory.current_tick(),
            danger_count: memory.danger_count(),
            food_count: memory.food_count(),
            trace_count: memory.trace_count(),
            total_danger_strength,
            total_food_value,
            total_trace_threat,
            danger_by_category,
            food_by_category,
            traces_by_kind,
            regions_covered: regions.into_iter().collect(),
            fingerprint,
        }
    }

    #[must_use]
    pub fn total_count(&self) -> usize {
        self.danger_count + self.food_count + self.trace_count
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.total_count() == 0
    }

    #[must_use]
    pub fn age(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.snapshot_tick)
    }

    #[must_use]
    pub fn is_stale(&self, current_tick: u64, max_age: u64) -> bool {
        self.age(current_tick) > max_age
    }

    #[must_use]
    pub fn needs_attention(&self) -> bool {
        self.total_danger_strength > 0.5 || self.total_trace_threat > 0.3
    }
}

/// Summary of memories scoped to a specific region.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegionMemorySummary {
    pub region: RegionScope,
    pub danger_count: u32,
    pub food_count: u32,
    pub trace_count: u32,
    pub total_danger_strength: f32,
    pub total_food_value: f32,
    pub total_trace_threat: f32,
    pub strongest_danger_priority: f32,
    pub strongest_food_value: f32,
    pub strongest_trace_threat: f32,
    pub last_danger_tick: Option<u64>,
    pub last_food_tick: Option<u64>,
    pub last_trace_tick: Option<u64>,
    pub computed_at_tick: u64,
}

impl RegionMemorySummary {
    #[must_use]
    pub fn from_memory(memory: &CreatureMemory, region: RegionScope) -> Self {
        let current_tick = memory.current_tick();

        let mut danger_count = 0u32;
        let mut food_count = 0u32;
        let mut trace_count = 0u32;

        let mut total_danger_strength = 0.0f32;
        let mut total_food_value = 0.0f32;
        let mut total_trace_threat = 0.0f32;

        let mut strongest_danger_priority = 0.0f32;
        let mut strongest_food_value = 0.0f32;
        let mut strongest_trace_threat = 0.0f32;

        let mut last_danger_tick: Option<u64> = None;
        let mut last_food_tick: Option<u64> = None;
        let mut last_trace_tick: Option<u64> = None;

        for danger in memory.iter_dangers() {
            if let Some(r) = &danger.region
                && region.matches(r)
            {
                danger_count += 1;
                let priority = danger.priority();
                total_danger_strength += priority;
                strongest_danger_priority = strongest_danger_priority.max(priority);
                last_danger_tick = Some(last_danger_tick.map_or(danger.last_refresh_tick, |t| {
                    t.max(danger.last_refresh_tick)
                }));
            }
        }

        for food in memory.iter_foods() {
            if let Some(r) = &food.region
                && region.matches(r)
            {
                food_count += 1;
                let value = food.value();
                total_food_value += value;
                strongest_food_value = strongest_food_value.max(value);
                last_food_tick = Some(
                    last_food_tick
                        .map_or(food.last_refresh_tick, |t| t.max(food.last_refresh_tick)),
                );
            }
        }

        for trace in memory.iter_traces() {
            if let Some(r) = &trace.region
                && region.matches(r)
            {
                trace_count += 1;
                let threat = trace.threat_level();
                total_trace_threat += threat;
                strongest_trace_threat = strongest_trace_threat.max(threat);
                last_trace_tick = Some(
                    last_trace_tick
                        .map_or(trace.last_refresh_tick, |t| t.max(trace.last_refresh_tick)),
                );
            }
        }

        Self {
            region,
            danger_count,
            food_count,
            trace_count,
            total_danger_strength,
            total_food_value,
            total_trace_threat,
            strongest_danger_priority,
            strongest_food_value,
            strongest_trace_threat,
            last_danger_tick,
            last_food_tick,
            last_trace_tick,
            computed_at_tick: current_tick,
        }
    }

    #[must_use]
    pub fn total_count(&self) -> u32 {
        self.danger_count + self.food_count + self.trace_count
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.total_count() == 0
    }

    #[must_use]
    pub fn overall_threat(&self) -> f32 {
        self.strongest_danger_priority
            .max(self.strongest_trace_threat)
    }

    #[must_use]
    pub fn needs_attention(&self) -> bool {
        self.strongest_danger_priority > 0.7 || self.strongest_trace_threat > 0.5
    }

    #[must_use]
    pub fn last_activity_tick(&self) -> Option<u64> {
        [
            self.last_danger_tick,
            self.last_food_tick,
            self.last_trace_tick,
        ]
        .into_iter()
        .flatten()
        .max()
    }

    #[must_use]
    pub fn staleness(&self, current_tick: u64) -> Option<u64> {
        self.last_activity_tick()
            .map(|t| current_tick.saturating_sub(t))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{
        DangerCategory, DangerZoneMemory, FoodCategory, FoodSourceMemory, MemoryId, MemorySource,
        PlayerTraceKind, PlayerTraceMemory,
    };

    fn make_danger(id: u64, region: Option<RegionScope>, strength: f32) -> DangerZoneMemory {
        let mut mem = DangerZoneMemory::new(
            MemoryId::new(id),
            [0.0, 0.0, 0.0],
            5.0,
            DangerCategory::Predator,
            strength,
            MemorySource::DirectObservation,
            0,
        );
        mem.region = region;
        mem
    }

    fn make_food(id: u64, region: Option<RegionScope>, strength: f32) -> FoodSourceMemory {
        let mut mem = FoodSourceMemory::new(
            MemoryId::new(id),
            [0.0, 0.0, 0.0],
            FoodCategory::Fruit,
            strength,
            MemorySource::DirectObservation,
            0,
        );
        mem.region = region;
        mem
    }

    fn make_trace(id: u64, region: Option<RegionScope>, strength: f32) -> PlayerTraceMemory {
        let mut mem = PlayerTraceMemory::new(
            MemoryId::new(id),
            [0.0, 0.0, 0.0],
            PlayerTraceKind::Scent,
            strength,
            MemorySource::Scent,
            0,
        );
        mem.region = region;
        mem
    }

    #[test]
    fn test_memory_fingerprint() {
        let mut mem = CreatureMemory::new();
        mem.remember_danger_zone(make_danger(1, None, 0.8));
        mem.remember_food_source(make_food(2, None, 0.9));

        let fp = MemoryFingerprint::from_memory(&mem);
        assert_eq!(fp.danger_count, 1);
        assert_eq!(fp.food_count, 1);
        assert_eq!(fp.trace_count, 0);
        assert_eq!(fp.total_count(), 2);
    }

    #[test]
    fn test_fingerprint_deterministic() {
        let mut mem = CreatureMemory::new();
        mem.remember_danger_zone(make_danger(1, None, 0.8));

        let fp1 = MemoryFingerprint::from_memory(&mem);
        let fp2 = MemoryFingerprint::from_memory(&mem);

        assert!(fp1.matches(&fp2));
        assert_eq!(fp1.checksum, fp2.checksum);
    }

    #[test]
    fn test_fingerprint_changes() {
        let mut mem = CreatureMemory::new();
        mem.remember_danger_zone(make_danger(1, None, 0.8));
        let fp1 = MemoryFingerprint::from_memory(&mem);

        mem.remember_danger_zone(make_danger(2, None, 0.5));
        let fp2 = MemoryFingerprint::from_memory(&mem);

        assert!(!fp1.matches(&fp2));
    }

    #[test]
    fn test_memory_summary() {
        let mut mem = CreatureMemory::new();
        mem.remember_danger_zone(make_danger(1, None, 0.8));
        mem.remember_food_source(make_food(2, None, 0.9));
        mem.remember_trace(make_trace(3, None, 0.7));

        let summary = MemorySummary::from_memory(&mem);
        assert_eq!(summary.danger_count, 1);
        assert_eq!(summary.food_count, 1);
        assert_eq!(summary.trace_count, 1);
        assert_eq!(summary.total_count(), 3);
    }

    #[test]
    fn test_memory_summary_attention() {
        let mut mem = CreatureMemory::new();
        mem.remember_danger_zone(make_danger(1, None, 0.9));

        let summary = MemorySummary::from_memory(&mem);
        assert!(summary.needs_attention);
    }

    #[test]
    fn test_memory_snapshot() {
        let mut mem = CreatureMemory::new();
        mem.remember_danger_zone(make_danger(1, Some(RegionScope::new("forest")), 0.8));
        mem.remember_danger_zone(make_danger(2, Some(RegionScope::new("forest")), 0.6));
        mem.remember_food_source(make_food(3, Some(RegionScope::new("cave")), 0.9));

        let snapshot = MemorySnapshot::from_memory(&mem);
        assert_eq!(snapshot.danger_count, 2);
        assert_eq!(snapshot.food_count, 1);
        assert_eq!(
            snapshot.danger_by_category.get(&DangerCategory::Predator),
            Some(&2)
        );
        assert!(snapshot.regions_covered.contains(&"forest".to_string()));
        assert!(snapshot.regions_covered.contains(&"cave".to_string()));
    }

    #[test]
    fn test_snapshot_staleness() {
        let mem = CreatureMemory::new();
        let snapshot = MemorySnapshot::from_memory(&mem);

        assert!(!snapshot.is_stale(100, 200));
        assert!(snapshot.is_stale(300, 200));
        assert_eq!(snapshot.age(100), 100);
    }

    #[test]
    fn test_region_memory_summary() {
        let mut mem = CreatureMemory::new();

        let forest = RegionScope::new("forest");
        let cave = RegionScope::new("cave");

        mem.remember_danger_zone(make_danger(1, Some(forest.clone()), 0.8));
        mem.remember_danger_zone(make_danger(2, Some(forest.clone()), 0.6));
        mem.remember_food_source(make_food(3, Some(cave.clone()), 0.9));

        let forest_summary = RegionMemorySummary::from_memory(&mem, forest);
        assert_eq!(forest_summary.danger_count, 2);
        assert_eq!(forest_summary.food_count, 0);

        let cave_summary = RegionMemorySummary::from_memory(&mem, cave);
        assert_eq!(cave_summary.danger_count, 0);
        assert_eq!(cave_summary.food_count, 1);
    }

    #[test]
    fn test_region_summary_threat() {
        let mut mem = CreatureMemory::new();
        let region = RegionScope::new("forest");

        mem.remember_danger_zone(make_danger(1, Some(region.clone()), 0.9));

        let summary = RegionMemorySummary::from_memory(&mem, region);
        assert!(summary.needs_attention());
        assert!(summary.overall_threat() > 0.7);
    }

    #[test]
    fn test_region_summary_activity() {
        let mut mem = CreatureMemory::new();
        let region = RegionScope::new("forest");

        mem.remember_danger_zone(make_danger(1, Some(region.clone()), 0.5));

        let summary = RegionMemorySummary::from_memory(&mem, region);
        assert!(summary.last_activity_tick().is_some());
        assert!(summary.staleness(100).is_some());
    }

    #[test]
    fn test_fingerprint_serde() {
        let mem = CreatureMemory::new();
        let fp = MemoryFingerprint::from_memory(&mem);

        let json = serde_json::to_string(&fp).unwrap();
        let restored: MemoryFingerprint = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.checksum, fp.checksum);
    }

    #[test]
    fn test_summary_serde() {
        let mut mem = CreatureMemory::new();
        mem.remember_danger_zone(make_danger(1, None, 0.8));

        let summary = MemorySummary::from_memory(&mem);
        let json = serde_json::to_string(&summary).unwrap();
        let restored: MemorySummary = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.danger_count, 1);
    }

    #[test]
    fn test_snapshot_serde() {
        let mut mem = CreatureMemory::new();
        mem.remember_danger_zone(make_danger(1, Some(RegionScope::new("forest")), 0.8));

        let snapshot = MemorySnapshot::from_memory(&mem);
        let json = serde_json::to_string(&snapshot).unwrap();
        let restored: MemorySnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.danger_count, 1);
        assert!(restored.regions_covered.contains(&"forest".to_string()));
    }

    #[test]
    fn test_region_summary_serde() {
        let mut mem = CreatureMemory::new();
        let region = RegionScope::new("forest");
        mem.remember_danger_zone(make_danger(1, Some(region.clone()), 0.8));

        let summary = RegionMemorySummary::from_memory(&mem, region);
        let json = serde_json::to_string(&summary).unwrap();
        let restored: RegionMemorySummary = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.danger_count, 1);
        assert_eq!(restored.region.region_id, "forest");
    }
}
