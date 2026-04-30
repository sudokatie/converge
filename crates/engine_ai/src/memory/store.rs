//! Memory store with decay, pruning, merge, and query APIs.

use super::config::MemoryStoreConfig;
use super::record::{
    DangerCategory, DangerZoneMemory, FoodCategory, FoodSourceMemory, MemoryCategory, MemoryId,
    MemoryRecord, MemorySource, PlayerTraceKind, PlayerTraceMemory, RegionScope,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Result of a memory query.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct QueryResult {
    pub danger_zones: Vec<DangerZoneMemory>,
    pub food_sources: Vec<FoodSourceMemory>,
    pub player_traces: Vec<PlayerTraceMemory>,
}

impl QueryResult {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn total_count(&self) -> usize {
        self.danger_zones.len() + self.food_sources.len() + self.player_traces.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.danger_zones.is_empty()
            && self.food_sources.is_empty()
            && self.player_traces.is_empty()
    }

    #[must_use]
    pub fn strongest_danger(&self) -> Option<&DangerZoneMemory> {
        self.danger_zones.first()
    }

    #[must_use]
    pub fn strongest_food(&self) -> Option<&FoodSourceMemory> {
        self.food_sources.first()
    }

    #[must_use]
    pub fn strongest_trace(&self) -> Option<&PlayerTraceMemory> {
        self.player_traces.first()
    }
}

/// Query builder for filtering memories.
#[derive(Clone, Debug, Default)]
pub struct MemoryQueryBuilder {
    categories: Option<Vec<MemoryCategory>>,
    region: Option<RegionScope>,
    min_strength: Option<f32>,
    max_age: Option<u64>,
    near_position: Option<([f32; 3], f32)>,
    danger_categories: Option<Vec<DangerCategory>>,
    food_categories: Option<Vec<FoodCategory>>,
    player_trace_kinds: Option<Vec<PlayerTraceKind>>,
    limit: Option<usize>,
    exclude_depleted: bool,
}

impl MemoryQueryBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn categories(mut self, cats: Vec<MemoryCategory>) -> Self {
        self.categories = Some(cats);
        self
    }

    #[must_use]
    pub fn danger_only(mut self) -> Self {
        self.categories = Some(vec![MemoryCategory::DangerZone]);
        self
    }

    #[must_use]
    pub fn food_only(mut self) -> Self {
        self.categories = Some(vec![MemoryCategory::FoodSource]);
        self
    }

    #[must_use]
    pub fn player_traces_only(mut self) -> Self {
        self.categories = Some(vec![MemoryCategory::PlayerTrace]);
        self
    }

    #[must_use]
    pub fn in_region(mut self, region: RegionScope) -> Self {
        self.region = Some(region);
        self
    }

    #[must_use]
    pub fn min_strength(mut self, strength: f32) -> Self {
        self.min_strength = Some(strength.clamp(0.0, 1.0));
        self
    }

    #[must_use]
    pub fn max_age(mut self, ticks: u64) -> Self {
        self.max_age = Some(ticks);
        self
    }

    #[must_use]
    pub fn near(mut self, position: [f32; 3], radius: f32) -> Self {
        self.near_position = Some((position, radius.max(0.0)));
        self
    }

    #[must_use]
    pub fn danger_categories(mut self, cats: Vec<DangerCategory>) -> Self {
        self.danger_categories = Some(cats);
        self
    }

    #[must_use]
    pub fn food_categories(mut self, cats: Vec<FoodCategory>) -> Self {
        self.food_categories = Some(cats);
        self
    }

    #[must_use]
    pub fn player_trace_kinds(mut self, kinds: Vec<PlayerTraceKind>) -> Self {
        self.player_trace_kinds = Some(kinds);
        self
    }

    #[must_use]
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    #[must_use]
    pub fn exclude_depleted(mut self) -> Self {
        self.exclude_depleted = true;
        self
    }

    fn should_include_category(&self, cat: MemoryCategory) -> bool {
        self.categories
            .as_ref()
            .is_none_or(|cats| cats.contains(&cat))
    }

    fn check_region<R: MemoryRecord>(&self, record: &R) -> bool {
        match (&self.region, record.region()) {
            (None, _) => true,
            (Some(query_region), Some(record_region)) => query_region.matches(record_region),
            (Some(_), None) => false,
        }
    }

    fn check_strength<R: MemoryRecord>(&self, record: &R) -> bool {
        self.min_strength
            .is_none_or(|min| record.effective_strength() >= min)
    }

    fn check_age<R: MemoryRecord>(&self, record: &R, current_tick: u64) -> bool {
        self.max_age
            .is_none_or(|max| record.age(current_tick) <= max)
    }

    fn check_position(&self, position: [f32; 3]) -> bool {
        self.near_position.is_none_or(|(center, radius)| {
            let dx = position[0] - center[0];
            let dy = position[1] - center[1];
            let dz = position[2] - center[2];
            let dist_sq = dx * dx + dy * dy + dz * dz;
            dist_sq <= radius * radius
        })
    }
}

/// Query specification built from a builder.
#[derive(Clone, Debug)]
pub struct MemoryQuery {
    builder: MemoryQueryBuilder,
}

impl MemoryQuery {
    #[must_use]
    pub fn all() -> Self {
        Self {
            builder: MemoryQueryBuilder::new(),
        }
    }

    #[must_use]
    pub fn danger_zones() -> MemoryQueryBuilder {
        MemoryQueryBuilder::new().danger_only()
    }

    #[must_use]
    pub fn food_sources() -> MemoryQueryBuilder {
        MemoryQueryBuilder::new().food_only()
    }

    #[must_use]
    pub fn player_traces() -> MemoryQueryBuilder {
        MemoryQueryBuilder::new().player_traces_only()
    }

    #[must_use]
    pub fn near(position: [f32; 3], radius: f32) -> MemoryQueryBuilder {
        MemoryQueryBuilder::new().near(position, radius)
    }

    #[must_use]
    pub fn in_region(region: RegionScope) -> MemoryQueryBuilder {
        MemoryQueryBuilder::new().in_region(region)
    }

    #[must_use]
    pub fn strongest(limit: usize) -> MemoryQueryBuilder {
        MemoryQueryBuilder::new().limit(limit)
    }
}

impl From<MemoryQueryBuilder> for MemoryQuery {
    fn from(builder: MemoryQueryBuilder) -> Self {
        Self { builder }
    }
}

/// Main creature memory store.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreatureMemory {
    config: MemoryStoreConfig,
    danger_zones: BTreeMap<MemoryId, DangerZoneMemory>,
    food_sources: BTreeMap<MemoryId, FoodSourceMemory>,
    player_traces: BTreeMap<MemoryId, PlayerTraceMemory>,
    current_tick: u64,
    next_id: u64,
    ticks_since_prune: u64,
}

impl CreatureMemory {
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(MemoryStoreConfig::default())
    }

    #[must_use]
    pub fn with_config(config: MemoryStoreConfig) -> Self {
        Self {
            config,
            danger_zones: BTreeMap::new(),
            food_sources: BTreeMap::new(),
            player_traces: BTreeMap::new(),
            current_tick: 0,
            next_id: 1,
            ticks_since_prune: 0,
        }
    }

    fn next_memory_id(&mut self) -> MemoryId {
        let id = MemoryId::new(self.next_id);
        self.next_id += 1;
        id
    }

    pub fn remember_danger(
        &mut self,
        position: [f32; 3],
        radius: f32,
        category: DangerCategory,
        strength: f32,
        source: MemorySource,
    ) -> MemoryId {
        let id = self.next_memory_id();
        let reliability = source.reliability();
        let memory = DangerZoneMemory::new(
            id.clone(),
            position,
            radius,
            category,
            strength,
            source,
            self.current_tick,
        );

        if self.config.enable_merge
            && let Some(existing_id) = self.find_mergeable_danger(&memory)
            && let Some(existing) = self.danger_zones.get_mut(&existing_id)
        {
            existing.refresh(self.current_tick, strength, reliability);
            return existing_id;
        }

        self.danger_zones.insert(id.clone(), memory);
        self.prune_danger_zones();
        id
    }

    pub fn remember_danger_zone(&mut self, mut memory: DangerZoneMemory) -> MemoryId {
        if memory.id.0 == 0 {
            memory.id = self.next_memory_id();
        }
        let id = memory.id.clone();

        if self.config.enable_merge
            && let Some(existing_id) = self.find_mergeable_danger(&memory)
            && let Some(existing) = self.danger_zones.get_mut(&existing_id)
        {
            existing.refresh(self.current_tick, memory.strength, memory.confidence);
            return existing_id;
        }

        self.danger_zones.insert(id.clone(), memory);
        self.prune_danger_zones();
        id
    }

    pub fn remember_food(
        &mut self,
        position: [f32; 3],
        category: FoodCategory,
        strength: f32,
        source: MemorySource,
    ) -> MemoryId {
        let id = self.next_memory_id();
        let memory = FoodSourceMemory::new(
            id.clone(),
            position,
            category,
            strength,
            source,
            self.current_tick,
        );

        if self.config.enable_merge
            && let Some(existing_id) = self.find_mergeable_food(&memory)
            && let Some(existing) = self.food_sources.get_mut(&existing_id)
        {
            existing.refresh(self.current_tick, strength, existing.estimated_quantity);
            return existing_id;
        }

        self.food_sources.insert(id.clone(), memory);
        self.prune_food_sources();
        id
    }

    pub fn remember_food_source(&mut self, mut memory: FoodSourceMemory) -> MemoryId {
        if memory.id.0 == 0 {
            memory.id = self.next_memory_id();
        }
        let id = memory.id.clone();

        if self.config.enable_merge
            && let Some(existing_id) = self.find_mergeable_food(&memory)
            && let Some(existing) = self.food_sources.get_mut(&existing_id)
        {
            existing.refresh(
                self.current_tick,
                memory.strength,
                memory.estimated_quantity,
            );
            return existing_id;
        }

        self.food_sources.insert(id.clone(), memory);
        self.prune_food_sources();
        id
    }

    pub fn remember_player_trace(
        &mut self,
        position: [f32; 3],
        kind: PlayerTraceKind,
        strength: f32,
        source: MemorySource,
    ) -> MemoryId {
        let id = self.next_memory_id();
        let memory = PlayerTraceMemory::new(
            id.clone(),
            position,
            kind,
            strength,
            source,
            self.current_tick,
        );

        if self.config.enable_merge
            && let Some(existing_id) = self.find_mergeable_trace(&memory)
            && let Some(existing) = self.player_traces.get_mut(&existing_id)
        {
            existing.refresh(self.current_tick, strength, memory.estimated_direction);
            return existing_id;
        }

        self.player_traces.insert(id.clone(), memory);
        self.prune_player_traces();
        id
    }

    pub fn remember_trace(&mut self, mut memory: PlayerTraceMemory) -> MemoryId {
        if memory.id.0 == 0 {
            memory.id = self.next_memory_id();
        }
        let id = memory.id.clone();

        if self.config.enable_merge
            && let Some(existing_id) = self.find_mergeable_trace(&memory)
            && let Some(existing) = self.player_traces.get_mut(&existing_id)
        {
            existing.refresh(
                self.current_tick,
                memory.strength,
                memory.estimated_direction,
            );
            return existing_id;
        }

        self.player_traces.insert(id.clone(), memory);
        self.prune_player_traces();
        id
    }

    fn find_mergeable_danger(&self, memory: &DangerZoneMemory) -> Option<MemoryId> {
        let merge_dist_sq = self.config.merge_distance * self.config.merge_distance;
        self.danger_zones
            .values()
            .find(|existing| {
                existing.category == memory.category && {
                    let dx = existing.position[0] - memory.position[0];
                    let dy = existing.position[1] - memory.position[1];
                    let dz = existing.position[2] - memory.position[2];
                    dx * dx + dy * dy + dz * dz <= merge_dist_sq
                }
            })
            .map(|m| m.id.clone())
    }

    fn find_mergeable_food(&self, memory: &FoodSourceMemory) -> Option<MemoryId> {
        let merge_dist_sq = self.config.merge_distance * self.config.merge_distance;
        self.food_sources
            .values()
            .find(|existing| {
                existing.category == memory.category && {
                    let dx = existing.position[0] - memory.position[0];
                    let dy = existing.position[1] - memory.position[1];
                    let dz = existing.position[2] - memory.position[2];
                    dx * dx + dy * dy + dz * dz <= merge_dist_sq
                }
            })
            .map(|m| m.id.clone())
    }

    fn find_mergeable_trace(&self, memory: &PlayerTraceMemory) -> Option<MemoryId> {
        let merge_dist_sq = self.config.merge_distance * self.config.merge_distance;
        self.player_traces
            .values()
            .find(|existing| {
                existing.kind == memory.kind && {
                    let dx = existing.position[0] - memory.position[0];
                    let dy = existing.position[1] - memory.position[1];
                    let dz = existing.position[2] - memory.position[2];
                    dx * dx + dy * dy + dz * dz <= merge_dist_sq
                }
            })
            .map(|m| m.id.clone())
    }

    pub fn forget_danger(&mut self, id: &MemoryId) -> Option<DangerZoneMemory> {
        self.danger_zones.remove(id)
    }

    pub fn forget_food(&mut self, id: &MemoryId) -> Option<FoodSourceMemory> {
        self.food_sources.remove(id)
    }

    pub fn forget_trace(&mut self, id: &MemoryId) -> Option<PlayerTraceMemory> {
        self.player_traces.remove(id)
    }

    pub fn clear(&mut self) {
        self.danger_zones.clear();
        self.food_sources.clear();
        self.player_traces.clear();
    }

    pub fn tick(&mut self) {
        self.current_tick += 1;
        self.ticks_since_prune += 1;

        self.apply_decay();

        if self.ticks_since_prune >= self.config.prune_interval {
            self.prune_forgotten();
            self.ticks_since_prune = 0;
        }
    }

    pub fn advance_to(&mut self, tick: u64) {
        while self.current_tick < tick {
            self.tick();
        }
    }

    fn apply_decay(&mut self) {
        for memory in self.danger_zones.values_mut() {
            let staleness = memory.staleness(self.current_tick);
            let decay = self.config.danger_decay.calculate_decay(staleness);
            memory.apply_decay(decay);
        }

        for memory in self.food_sources.values_mut() {
            let staleness = memory.staleness(self.current_tick);
            let decay = self.config.food_decay.calculate_decay(staleness);
            memory.apply_decay(decay);
        }

        for memory in self.player_traces.values_mut() {
            let staleness = memory.staleness(self.current_tick);
            let decay = self.config.player_decay.calculate_decay(staleness);
            memory.apply_decay(decay);
        }
    }

    fn prune_forgotten(&mut self) {
        let danger_threshold = self.config.danger_decay.forget_threshold;
        self.danger_zones
            .retain(|_, m| m.effective_strength() >= danger_threshold);

        let food_threshold = self.config.food_decay.forget_threshold;
        self.food_sources
            .retain(|_, m| m.effective_strength() >= food_threshold);

        let player_threshold = self.config.player_decay.forget_threshold;
        self.player_traces
            .retain(|_, m| m.effective_strength() >= player_threshold);
    }

    fn prune_danger_zones(&mut self) {
        if self.danger_zones.len() <= self.config.max_danger_zones {
            return;
        }
        self.prune_weakest_danger();
    }

    fn prune_weakest_danger(&mut self) {
        while self.danger_zones.len() > self.config.max_danger_zones {
            let weakest = self
                .danger_zones
                .values()
                .min_by(|a, b| a.priority().partial_cmp(&b.priority()).unwrap())
                .map(|m| m.id.clone());

            if let Some(id) = weakest {
                self.danger_zones.remove(&id);
            } else {
                break;
            }
        }
    }

    fn prune_food_sources(&mut self) {
        if self.food_sources.len() <= self.config.max_food_sources {
            return;
        }
        self.prune_weakest_food();
    }

    fn prune_weakest_food(&mut self) {
        while self.food_sources.len() > self.config.max_food_sources {
            let weakest = self
                .food_sources
                .values()
                .min_by(|a, b| a.value().partial_cmp(&b.value()).unwrap())
                .map(|m| m.id.clone());

            if let Some(id) = weakest {
                self.food_sources.remove(&id);
            } else {
                break;
            }
        }
    }

    fn prune_player_traces(&mut self) {
        if self.player_traces.len() <= self.config.max_player_traces {
            return;
        }
        self.prune_weakest_traces();
    }

    fn prune_weakest_traces(&mut self) {
        while self.player_traces.len() > self.config.max_player_traces {
            let weakest = self
                .player_traces
                .values()
                .min_by(|a, b| a.threat_level().partial_cmp(&b.threat_level()).unwrap())
                .map(|m| m.id.clone());

            if let Some(id) = weakest {
                self.player_traces.remove(&id);
            } else {
                break;
            }
        }
    }

    pub fn query(&self, query: impl Into<MemoryQuery>) -> QueryResult {
        let query: MemoryQuery = query.into();
        let builder = &query.builder;
        let mut result = QueryResult::new();

        if builder.should_include_category(MemoryCategory::DangerZone) {
            let mut dangers: Vec<_> = self
                .danger_zones
                .values()
                .filter(|m| {
                    builder.check_region(*m)
                        && builder.check_strength(*m)
                        && builder.check_age(*m, self.current_tick)
                        && builder.check_position(m.position)
                        && builder
                            .danger_categories
                            .as_ref()
                            .is_none_or(|cats| cats.contains(&m.category))
                })
                .cloned()
                .collect();
            dangers.sort();
            if let Some(limit) = builder.limit {
                dangers.truncate(limit);
            }
            result.danger_zones = dangers;
        }

        if builder.should_include_category(MemoryCategory::FoodSource) {
            let mut foods: Vec<_> = self
                .food_sources
                .values()
                .filter(|m| {
                    builder.check_region(*m)
                        && builder.check_strength(*m)
                        && builder.check_age(*m, self.current_tick)
                        && builder.check_position(m.position)
                        && (!builder.exclude_depleted || !m.is_depleted)
                        && builder
                            .food_categories
                            .as_ref()
                            .is_none_or(|cats| cats.contains(&m.category))
                })
                .cloned()
                .collect();
            foods.sort();
            if let Some(limit) = builder.limit {
                foods.truncate(limit);
            }
            result.food_sources = foods;
        }

        if builder.should_include_category(MemoryCategory::PlayerTrace) {
            let mut traces: Vec<_> = self
                .player_traces
                .values()
                .filter(|m| {
                    builder.check_region(*m)
                        && builder.check_strength(*m)
                        && builder.check_age(*m, self.current_tick)
                        && builder.check_position(m.position)
                        && builder
                            .player_trace_kinds
                            .as_ref()
                            .is_none_or(|kinds| kinds.contains(&m.kind))
                })
                .cloned()
                .collect();
            traces.sort();
            if let Some(limit) = builder.limit {
                traces.truncate(limit);
            }
            result.player_traces = traces;
        }

        result
    }

    #[must_use]
    pub fn danger_avoidance_candidates(
        &self,
        position: [f32; 3],
        radius: f32,
    ) -> Vec<&DangerZoneMemory> {
        let mut candidates: Vec<_> = self
            .danger_zones
            .values()
            .filter(|m| m.distance_to(position) <= radius + m.radius)
            .collect();
        candidates.sort();
        candidates
    }

    #[must_use]
    pub fn food_candidates(&self, position: [f32; 3], max_distance: f32) -> Vec<&FoodSourceMemory> {
        let mut candidates: Vec<_> = self
            .food_sources
            .values()
            .filter(|m| !m.is_depleted && m.distance_to(position) <= max_distance)
            .collect();
        candidates.sort();
        candidates
    }

    #[must_use]
    pub fn player_trace_hints(&self, recency_threshold: u64) -> Vec<&PlayerTraceMemory> {
        let mut hints: Vec<_> = self
            .player_traces
            .values()
            .filter(|m| m.is_recent(self.current_tick, recency_threshold))
            .collect();
        hints.sort();
        hints
    }

    #[must_use]
    pub fn strongest_danger(&self) -> Option<&DangerZoneMemory> {
        self.danger_zones.values().max_by(|a, b| {
            a.priority()
                .partial_cmp(&b.priority())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    #[must_use]
    pub fn strongest_food(&self) -> Option<&FoodSourceMemory> {
        self.food_sources.values().max_by(|a, b| {
            a.value()
                .partial_cmp(&b.value())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    #[must_use]
    pub fn strongest_trace(&self) -> Option<&PlayerTraceMemory> {
        self.player_traces.values().max_by(|a, b| {
            a.threat_level()
                .partial_cmp(&b.threat_level())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    #[must_use]
    pub fn get_danger(&self, id: &MemoryId) -> Option<&DangerZoneMemory> {
        self.danger_zones.get(id)
    }

    #[must_use]
    pub fn get_food(&self, id: &MemoryId) -> Option<&FoodSourceMemory> {
        self.food_sources.get(id)
    }

    #[must_use]
    pub fn get_trace(&self, id: &MemoryId) -> Option<&PlayerTraceMemory> {
        self.player_traces.get(id)
    }

    pub fn get_danger_mut(&mut self, id: &MemoryId) -> Option<&mut DangerZoneMemory> {
        self.danger_zones.get_mut(id)
    }

    pub fn get_food_mut(&mut self, id: &MemoryId) -> Option<&mut FoodSourceMemory> {
        self.food_sources.get_mut(id)
    }

    pub fn get_trace_mut(&mut self, id: &MemoryId) -> Option<&mut PlayerTraceMemory> {
        self.player_traces.get_mut(id)
    }

    #[must_use]
    pub fn danger_count(&self) -> usize {
        self.danger_zones.len()
    }

    #[must_use]
    pub fn food_count(&self) -> usize {
        self.food_sources.len()
    }

    #[must_use]
    pub fn trace_count(&self) -> usize {
        self.player_traces.len()
    }

    #[must_use]
    pub fn total_count(&self) -> usize {
        self.danger_zones.len() + self.food_sources.len() + self.player_traces.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.danger_zones.is_empty()
            && self.food_sources.is_empty()
            && self.player_traces.is_empty()
    }

    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    #[must_use]
    pub fn config(&self) -> &MemoryStoreConfig {
        &self.config
    }

    pub fn iter_dangers(&self) -> impl Iterator<Item = &DangerZoneMemory> {
        self.danger_zones.values()
    }

    pub fn iter_foods(&self) -> impl Iterator<Item = &FoodSourceMemory> {
        self.food_sources.values()
    }

    pub fn iter_traces(&self) -> impl Iterator<Item = &PlayerTraceMemory> {
        self.player_traces.values()
    }
}

impl Default for CreatureMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::cast_precision_loss)]
mod tests {
    use super::*;
    use crate::memory::config::DecayConfig;

    fn make_danger(id: u64, pos: [f32; 3], strength: f32) -> DangerZoneMemory {
        DangerZoneMemory::new(
            MemoryId::new(id),
            pos,
            5.0,
            DangerCategory::Predator,
            strength,
            MemorySource::DirectObservation,
            0,
        )
    }

    fn make_food(id: u64, pos: [f32; 3], strength: f32) -> FoodSourceMemory {
        FoodSourceMemory::new(
            MemoryId::new(id),
            pos,
            FoodCategory::Fruit,
            strength,
            MemorySource::DirectObservation,
            0,
        )
    }

    fn make_trace(id: u64, pos: [f32; 3], strength: f32) -> PlayerTraceMemory {
        PlayerTraceMemory::new(
            MemoryId::new(id),
            pos,
            PlayerTraceKind::Scent,
            strength,
            MemorySource::Scent,
            0,
        )
    }

    #[test]
    fn test_creature_memory_new() {
        let mem = CreatureMemory::new();
        assert!(mem.is_empty());
        assert_eq!(mem.current_tick(), 0);
    }

    #[test]
    fn test_remember_danger() {
        let mut mem = CreatureMemory::new();
        let id = mem.remember_danger(
            [0.0, 0.0, 0.0],
            5.0,
            DangerCategory::Predator,
            0.8,
            MemorySource::DirectObservation,
        );

        assert_eq!(mem.danger_count(), 1);
        assert!(mem.get_danger(&id).is_some());
    }

    #[test]
    fn test_remember_food() {
        let mut mem = CreatureMemory::new();
        let id = mem.remember_food(
            [10.0, 0.0, 0.0],
            FoodCategory::Fruit,
            0.9,
            MemorySource::DirectObservation,
        );

        assert_eq!(mem.food_count(), 1);
        assert!(mem.get_food(&id).is_some());
    }

    #[test]
    fn test_remember_trace() {
        let mut mem = CreatureMemory::new();
        let id = mem.remember_player_trace(
            [20.0, 0.0, 0.0],
            PlayerTraceKind::Scent,
            0.7,
            MemorySource::Scent,
        );

        assert_eq!(mem.trace_count(), 1);
        assert!(mem.get_trace(&id).is_some());
    }

    #[test]
    fn test_forget() {
        let mut mem = CreatureMemory::new();
        let id = mem.remember_danger(
            [0.0, 0.0, 0.0],
            5.0,
            DangerCategory::Trap,
            1.0,
            MemorySource::DirectObservation,
        );

        let removed = mem.forget_danger(&id);
        assert!(removed.is_some());
        assert!(mem.is_empty());
    }

    #[test]
    fn test_tick_decay() {
        let mut mem = CreatureMemory::with_config(
            MemoryStoreConfig::new().with_danger_decay(DecayConfig::new(0.9, 0.1)),
        );

        let id = mem.remember_danger(
            [0.0, 0.0, 0.0],
            5.0,
            DangerCategory::Predator,
            1.0,
            MemorySource::DirectObservation,
        );

        let initial = mem.get_danger(&id).unwrap().strength;
        mem.tick();
        let after = mem.get_danger(&id).unwrap().strength;

        assert!(after < initial);
    }

    #[test]
    fn test_prune_forgotten() {
        let mut mem = CreatureMemory::with_config(
            MemoryStoreConfig::new().with_danger_decay(DecayConfig::new(0.5, 0.3)),
        );

        mem.remember_danger(
            [0.0, 0.0, 0.0],
            5.0,
            DangerCategory::Predator,
            0.4,
            MemorySource::DirectObservation,
        );

        for _ in 0..100 {
            mem.tick();
        }

        assert!(mem.is_empty());
    }

    #[test]
    fn test_prune_max_count() {
        let mut mem =
            CreatureMemory::with_config(MemoryStoreConfig::new().with_max_danger_zones(3));

        for i in 0..5 {
            mem.remember_danger(
                [i as f32, 0.0, 0.0],
                5.0,
                DangerCategory::Predator,
                (i as f32 + 1.0) * 0.1,
                MemorySource::DirectObservation,
            );
        }

        assert_eq!(mem.danger_count(), 3);
    }

    #[test]
    fn test_merge_nearby() {
        let mut mem = CreatureMemory::with_config(MemoryStoreConfig::new().with_merge(true, 5.0));

        let id1 = mem.remember_danger(
            [0.0, 0.0, 0.0],
            5.0,
            DangerCategory::Predator,
            0.5,
            MemorySource::DirectObservation,
        );
        let id2 = mem.remember_danger(
            [2.0, 0.0, 0.0],
            5.0,
            DangerCategory::Predator,
            0.8,
            MemorySource::DirectObservation,
        );

        assert_eq!(id1, id2);
        assert_eq!(mem.danger_count(), 1);
    }

    #[test]
    fn test_no_merge_different_category() {
        let mut mem = CreatureMemory::with_config(MemoryStoreConfig::new().with_merge(true, 5.0));

        mem.remember_danger(
            [0.0, 0.0, 0.0],
            5.0,
            DangerCategory::Predator,
            0.5,
            MemorySource::DirectObservation,
        );
        mem.remember_danger(
            [2.0, 0.0, 0.0],
            5.0,
            DangerCategory::Trap,
            0.8,
            MemorySource::DirectObservation,
        );

        assert_eq!(mem.danger_count(), 2);
    }

    #[test]
    fn test_query_all() {
        let mut mem = CreatureMemory::new();
        mem.remember_danger_zone(make_danger(1, [0.0, 0.0, 0.0], 0.8));
        mem.remember_food_source(make_food(2, [10.0, 0.0, 0.0], 0.9));
        mem.remember_trace(make_trace(3, [20.0, 0.0, 0.0], 0.7));

        let result = mem.query(MemoryQuery::all());
        assert_eq!(result.total_count(), 3);
    }

    #[test]
    fn test_query_danger_only() {
        let mut mem = CreatureMemory::new();
        mem.remember_danger_zone(make_danger(1, [0.0, 0.0, 0.0], 0.8));
        mem.remember_food_source(make_food(2, [10.0, 0.0, 0.0], 0.9));

        let result = mem.query(MemoryQuery::danger_zones());
        assert_eq!(result.danger_zones.len(), 1);
        assert!(result.food_sources.is_empty());
    }

    #[test]
    fn test_query_near() {
        let mut mem = CreatureMemory::new();
        mem.remember_danger_zone(make_danger(1, [0.0, 0.0, 0.0], 0.8));
        mem.remember_danger_zone(make_danger(2, [100.0, 0.0, 0.0], 0.9));

        let result = mem.query(MemoryQuery::near([0.0, 0.0, 0.0], 10.0).danger_only());
        assert_eq!(result.danger_zones.len(), 1);
        assert_eq!(result.danger_zones[0].id.0, 1);
    }

    #[test]
    fn test_query_min_strength() {
        let mut mem = CreatureMemory::new();
        mem.remember_danger_zone(make_danger(1, [0.0, 0.0, 0.0], 0.3));
        mem.remember_danger_zone(make_danger(2, [10.0, 0.0, 0.0], 0.8));

        let result = mem.query(MemoryQuery::danger_zones().min_strength(0.5));
        assert_eq!(result.danger_zones.len(), 1);
        assert_eq!(result.danger_zones[0].id.0, 2);
    }

    #[test]
    fn test_query_limit() {
        let mut mem = CreatureMemory::new();
        for i in 0..10 {
            mem.remember_danger_zone(make_danger(i + 1, [i as f32, 0.0, 0.0], 0.5));
        }

        let result = mem.query(MemoryQuery::danger_zones().limit(3));
        assert_eq!(result.danger_zones.len(), 3);
    }

    #[test]
    fn test_query_exclude_depleted() {
        let mut mem = CreatureMemory::new();
        let id1 = mem.remember_food_source(make_food(1, [0.0, 0.0, 0.0], 0.8));
        mem.remember_food_source(make_food(2, [10.0, 0.0, 0.0], 0.9));

        mem.get_food_mut(&id1).unwrap().mark_depleted();

        let result = mem.query(MemoryQuery::food_sources().exclude_depleted());
        assert_eq!(result.food_sources.len(), 1);
        assert_eq!(result.food_sources[0].id.0, 2);
    }

    #[test]
    fn test_danger_avoidance_candidates() {
        let mut mem = CreatureMemory::new();
        mem.remember_danger_zone(make_danger(1, [0.0, 0.0, 0.0], 0.8));
        mem.remember_danger_zone(make_danger(2, [50.0, 0.0, 0.0], 0.9));

        let candidates = mem.danger_avoidance_candidates([0.0, 0.0, 0.0], 20.0);
        assert_eq!(candidates.len(), 1);
    }

    #[test]
    fn test_food_candidates() {
        let mut mem = CreatureMemory::new();
        let id1 = mem.remember_food_source(make_food(1, [0.0, 0.0, 0.0], 0.8));
        mem.remember_food_source(make_food(2, [10.0, 0.0, 0.0], 0.9));

        mem.get_food_mut(&id1).unwrap().mark_depleted();

        let candidates = mem.food_candidates([0.0, 0.0, 0.0], 20.0);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].id.0, 2);
    }

    #[test]
    fn test_player_trace_hints() {
        let mut mem = CreatureMemory::new();
        mem.remember_trace(make_trace(1, [0.0, 0.0, 0.0], 0.8));

        let hints = mem.player_trace_hints(100);
        assert_eq!(hints.len(), 1);
    }

    #[test]
    fn test_strongest_queries() {
        let mut mem = CreatureMemory::new();
        mem.remember_danger_zone(make_danger(1, [0.0, 0.0, 0.0], 0.3));
        mem.remember_danger_zone(make_danger(2, [10.0, 0.0, 0.0], 0.9));

        let strongest = mem.strongest_danger().unwrap();
        assert_eq!(strongest.id.0, 2);
    }

    #[test]
    fn test_deterministic_iteration() {
        let mut mem = CreatureMemory::new();
        mem.remember_danger_zone(make_danger(5, [0.0, 0.0, 0.0], 0.5));
        mem.remember_danger_zone(make_danger(1, [10.0, 0.0, 0.0], 0.5));
        mem.remember_danger_zone(make_danger(3, [20.0, 0.0, 0.0], 0.5));

        let ids1: Vec<_> = mem.iter_dangers().map(|m| m.id.0).collect();
        let ids2: Vec<_> = mem.iter_dangers().map(|m| m.id.0).collect();

        assert_eq!(ids1, ids2);
    }

    #[test]
    fn test_query_result() {
        let mut result = QueryResult::new();
        assert!(result.is_empty());

        result
            .danger_zones
            .push(make_danger(1, [0.0, 0.0, 0.0], 0.8));
        assert!(!result.is_empty());
        assert_eq!(result.total_count(), 1);
        assert!(result.strongest_danger().is_some());
    }

    #[test]
    fn test_creature_memory_serde() {
        let mut mem = CreatureMemory::new();
        mem.remember_danger(
            [1.0, 2.0, 3.0],
            5.0,
            DangerCategory::Predator,
            0.8,
            MemorySource::DirectObservation,
        );
        mem.remember_food(
            [4.0, 5.0, 6.0],
            FoodCategory::Fruit,
            0.9,
            MemorySource::DirectObservation,
        );

        let json = serde_json::to_string(&mem).unwrap();
        let restored: CreatureMemory = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.danger_count(), 1);
        assert_eq!(restored.food_count(), 1);
        assert_eq!(restored.current_tick(), mem.current_tick());
    }

    #[test]
    fn test_query_result_serde() {
        let mut result = QueryResult::new();
        result
            .danger_zones
            .push(make_danger(1, [0.0, 0.0, 0.0], 0.8));
        result
            .food_sources
            .push(make_food(2, [10.0, 0.0, 0.0], 0.9));

        let json = serde_json::to_string(&result).unwrap();
        let restored: QueryResult = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.total_count(), 2);
    }

    #[test]
    fn test_advance_to() {
        let mut mem = CreatureMemory::new();
        mem.advance_to(100);
        assert_eq!(mem.current_tick(), 100);
    }

    #[test]
    fn test_region_query() {
        let mut mem = CreatureMemory::new();

        let mut d1 = make_danger(1, [0.0, 0.0, 0.0], 0.8);
        d1.region = Some(RegionScope::new("forest"));
        mem.remember_danger_zone(d1);

        let mut d2 = make_danger(2, [10.0, 0.0, 0.0], 0.9);
        d2.region = Some(RegionScope::new("desert"));
        mem.remember_danger_zone(d2);

        let result = mem.query(MemoryQuery::in_region(RegionScope::new("forest")).danger_only());
        assert_eq!(result.danger_zones.len(), 1);
        assert_eq!(result.danger_zones[0].id.0, 1);
    }
}
