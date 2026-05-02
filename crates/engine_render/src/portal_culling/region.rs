//! Cull region management for portal-aware rendering.

use std::collections::BTreeMap;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::id::CullRegionId;
use engine_world::portal::ZoneId;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CullRegionState {
    #[default]
    Hidden,
    PotentiallyVisible,
    Visible,
    FullyVisible,
}

impl CullRegionState {
    #[must_use]
    pub const fn is_visible(&self) -> bool {
        !matches!(self, Self::Hidden)
    }

    #[must_use]
    pub const fn needs_render(&self) -> bool {
        matches!(self, Self::Visible | Self::FullyVisible)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CullRegion {
    pub id: CullRegionId,
    pub zone_id: ZoneId,
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
    pub center: Vec3,
    pub radius: f32,
    pub state: CullRegionState,
    pub portal_depth: u32,
}

impl CullRegion {
    #[must_use]
    pub fn new(id: CullRegionId, zone_id: ZoneId, bounds_min: Vec3, bounds_max: Vec3) -> Self {
        let center = (bounds_min + bounds_max) * 0.5;
        let half_extents = (bounds_max - bounds_min) * 0.5;
        let radius = half_extents.length();

        Self {
            id,
            zone_id,
            bounds_min,
            bounds_max,
            center,
            radius,
            state: CullRegionState::Hidden,
            portal_depth: 0,
        }
    }

    #[must_use]
    pub fn from_sphere(id: CullRegionId, zone_id: ZoneId, center: Vec3, radius: f32) -> Self {
        let half = Vec3::splat(radius);
        Self {
            id,
            zone_id,
            bounds_min: center - half,
            bounds_max: center + half,
            center,
            radius,
            state: CullRegionState::Hidden,
            portal_depth: 0,
        }
    }

    #[must_use]
    pub fn with_state(mut self, state: CullRegionState) -> Self {
        self.state = state;
        self
    }

    #[must_use]
    pub fn with_portal_depth(mut self, depth: u32) -> Self {
        self.portal_depth = depth;
        self
    }

    #[must_use]
    pub fn contains_point(&self, point: Vec3) -> bool {
        point.x >= self.bounds_min.x
            && point.x <= self.bounds_max.x
            && point.y >= self.bounds_min.y
            && point.y <= self.bounds_max.y
            && point.z >= self.bounds_min.z
            && point.z <= self.bounds_max.z
    }

    #[must_use]
    pub fn intersects(&self, other: &CullRegion) -> bool {
        self.bounds_min.x <= other.bounds_max.x
            && self.bounds_max.x >= other.bounds_min.x
            && self.bounds_min.y <= other.bounds_max.y
            && self.bounds_max.y >= other.bounds_min.y
            && self.bounds_min.z <= other.bounds_max.z
            && self.bounds_max.z >= other.bounds_min.z
    }

    #[must_use]
    pub fn distance_to(&self, point: Vec3) -> f32 {
        let clamped = point.clamp(self.bounds_min, self.bounds_max);
        (point - clamped).length()
    }

    #[must_use]
    pub fn volume(&self) -> f32 {
        let extents = self.bounds_max - self.bounds_min;
        extents.x * extents.y * extents.z
    }
}

impl Default for CullRegion {
    fn default() -> Self {
        Self::new(
            CullRegionId::from_raw(0),
            ZoneId::from_raw(0),
            Vec3::ZERO,
            Vec3::ONE,
        )
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CullRegionSet {
    regions: BTreeMap<CullRegionId, CullRegion>,
    zones: BTreeMap<ZoneId, Vec<CullRegionId>>,
    next_id: u64,
}

impl CullRegionSet {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, region: CullRegion) -> CullRegionId {
        let id = region.id;
        self.zones.entry(region.zone_id).or_default().push(id);
        self.regions.insert(id, region);
        id
    }

    pub fn add(&mut self, zone_id: ZoneId, bounds_min: Vec3, bounds_max: Vec3) -> CullRegionId {
        let id = CullRegionId::from_raw(self.next_id);
        self.next_id += 1;
        let region = CullRegion::new(id, zone_id, bounds_min, bounds_max);
        self.insert(region)
    }

    pub fn remove(&mut self, id: CullRegionId) -> Option<CullRegion> {
        let region = self.regions.remove(&id)?;
        if let Some(zone_regions) = self.zones.get_mut(&region.zone_id) {
            zone_regions.retain(|&r| r != id);
        }
        Some(region)
    }

    #[must_use]
    pub fn get(&self, id: CullRegionId) -> Option<&CullRegion> {
        self.regions.get(&id)
    }

    pub fn get_mut(&mut self, id: CullRegionId) -> Option<&mut CullRegion> {
        self.regions.get_mut(&id)
    }

    #[must_use]
    pub fn regions_in_zone(&self, zone_id: ZoneId) -> Vec<&CullRegion> {
        self.zones
            .get(&zone_id)
            .map(|ids| ids.iter().filter_map(|id| self.regions.get(id)).collect())
            .unwrap_or_default()
    }

    #[must_use]
    pub fn visible_regions(&self) -> Vec<&CullRegion> {
        self.regions
            .values()
            .filter(|r| r.state.is_visible())
            .collect()
    }

    #[must_use]
    pub fn renderable_regions(&self) -> Vec<&CullRegion> {
        self.regions
            .values()
            .filter(|r| r.state.needs_render())
            .collect()
    }

    pub fn set_state(&mut self, id: CullRegionId, state: CullRegionState) {
        if let Some(region) = self.regions.get_mut(&id) {
            region.state = state;
        }
    }

    pub fn reset_states(&mut self) {
        for region in self.regions.values_mut() {
            region.state = CullRegionState::Hidden;
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.regions.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }

    #[must_use]
    pub fn zone_count(&self) -> usize {
        self.zones.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &CullRegion> {
        self.regions.values()
    }

    pub fn clear(&mut self) {
        self.regions.clear();
        self.zones.clear();
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CullStatistics {
    pub total_regions: u32,
    pub visible_regions: u32,
    pub hidden_regions: u32,
    pub portal_traversals: u32,
    pub max_portal_depth: u32,
}

impl CullStatistics {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "region counts fit in f32 mantissa"
    )]
    pub fn visibility_ratio(&self) -> f32 {
        if self.total_regions == 0 {
            0.0
        } else {
            self.visible_regions as f32 / self.total_regions as f32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_creation() {
        let region = CullRegion::new(
            CullRegionId::from_raw(1),
            ZoneId::new(0, 0),
            Vec3::ZERO,
            Vec3::splat(10.0),
        );

        assert_eq!(region.center, Vec3::splat(5.0));
        assert!(region.radius > 0.0);
    }

    #[test]
    fn region_contains_point() {
        let region = CullRegion::new(
            CullRegionId::from_raw(1),
            ZoneId::new(0, 0),
            Vec3::ZERO,
            Vec3::splat(10.0),
        );

        assert!(region.contains_point(Vec3::splat(5.0)));
        assert!(!region.contains_point(Vec3::splat(15.0)));
    }

    #[test]
    fn region_intersects() {
        let r1 = CullRegion::new(
            CullRegionId::from_raw(1),
            ZoneId::new(0, 0),
            Vec3::ZERO,
            Vec3::splat(10.0),
        );
        let r2 = CullRegion::new(
            CullRegionId::from_raw(2),
            ZoneId::new(0, 0),
            Vec3::splat(5.0),
            Vec3::splat(15.0),
        );
        let r3 = CullRegion::new(
            CullRegionId::from_raw(3),
            ZoneId::new(0, 0),
            Vec3::splat(20.0),
            Vec3::splat(30.0),
        );

        assert!(r1.intersects(&r2));
        assert!(!r1.intersects(&r3));
    }

    #[test]
    fn region_set_operations() {
        let mut set = CullRegionSet::new();
        let zone = ZoneId::new(0, 0);

        let id1 = set.add(zone, Vec3::ZERO, Vec3::splat(10.0));
        let id2 = set.add(zone, Vec3::splat(10.0), Vec3::splat(20.0));

        assert_eq!(set.len(), 2);
        assert_eq!(set.regions_in_zone(zone).len(), 2);

        set.set_state(id1, CullRegionState::Visible);
        assert_eq!(set.visible_regions().len(), 1);

        set.remove(id2);
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn region_state_predicates() {
        assert!(!CullRegionState::Hidden.is_visible());
        assert!(CullRegionState::PotentiallyVisible.is_visible());
        assert!(CullRegionState::Visible.is_visible());
        assert!(CullRegionState::FullyVisible.is_visible());

        assert!(!CullRegionState::Hidden.needs_render());
        assert!(!CullRegionState::PotentiallyVisible.needs_render());
        assert!(CullRegionState::Visible.needs_render());
        assert!(CullRegionState::FullyVisible.needs_render());
    }

    #[test]
    fn statistics() {
        let mut stats = CullStatistics::new();
        stats.total_regions = 100;
        stats.visible_regions = 25;

        assert!((stats.visibility_ratio() - 0.25).abs() < 0.001);
    }

    #[test]
    fn reset_states() {
        let mut set = CullRegionSet::new();
        let zone = ZoneId::new(0, 0);
        let id = set.add(zone, Vec3::ZERO, Vec3::splat(10.0));
        set.set_state(id, CullRegionState::Visible);

        assert!(set.get(id).unwrap().state.is_visible());

        set.reset_states();
        assert!(!set.get(id).unwrap().state.is_visible());
    }

    #[test]
    fn serde_roundtrip() {
        let region = CullRegion::new(
            CullRegionId::from_raw(42),
            ZoneId::new(1, 2),
            Vec3::ZERO,
            Vec3::splat(5.0),
        );

        let json = serde_json::to_string(&region).unwrap();
        let recovered: CullRegion = serde_json::from_str(&json).unwrap();
        assert_eq!(region.id, recovered.id);
    }
}
