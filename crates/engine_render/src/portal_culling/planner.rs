//! CPU-side portal culling planner.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::frustum::{CullResult, Frustum, cull_aabb, cull_sphere};
use super::id::{CullRegionId, RenderPassId};
use super::region::{CullRegion, CullRegionSet, CullRegionState, CullStatistics};

use engine_world::portal::{Portal, PortalGraph, PortalSide, ZoneId};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CullPlannerConfig {
    pub max_portal_depth: u32,
    pub max_regions_per_pass: u32,
    pub max_render_passes: u32,
    pub cull_behind_portal: bool,
    pub use_portal_frustum_clipping: bool,
    pub occlusion_threshold: f32,
}

impl Default for CullPlannerConfig {
    fn default() -> Self {
        Self {
            max_portal_depth: 8,
            max_regions_per_pass: 4096,
            max_render_passes: 32,
            cull_behind_portal: true,
            use_portal_frustum_clipping: true,
            occlusion_threshold: 0.001,
        }
    }
}

impl CullPlannerConfig {
    #[must_use]
    pub fn with_max_depth(mut self, depth: u32) -> Self {
        self.max_portal_depth = depth;
        self
    }

    #[must_use]
    pub fn with_max_regions(mut self, count: u32) -> Self {
        self.max_regions_per_pass = count;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RenderPass {
    pub id: RenderPassId,
    pub zone_id: ZoneId,
    pub frustum: Frustum,
    pub portal_depth: u32,
    pub region_ids: Vec<CullRegionId>,
    pub source_portal: Option<(engine_world::portal::PortalId, PortalSide)>,
}

impl RenderPass {
    #[must_use]
    pub fn new(id: RenderPassId, zone_id: ZoneId, frustum: Frustum) -> Self {
        Self {
            id,
            zone_id,
            frustum,
            portal_depth: 0,
            region_ids: Vec::new(),
            source_portal: None,
        }
    }

    #[must_use]
    pub fn with_depth(mut self, depth: u32) -> Self {
        self.portal_depth = depth;
        self
    }

    #[must_use]
    pub fn with_source_portal(
        mut self,
        portal_id: engine_world::portal::PortalId,
        side: PortalSide,
    ) -> Self {
        self.source_portal = Some((portal_id, side));
        self
    }

    #[must_use]
    pub fn region_count(&self) -> usize {
        self.region_ids.len()
    }

    #[must_use]
    pub fn is_primary(&self) -> bool {
        self.portal_depth == 0
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CullPlan {
    pub passes: Vec<RenderPass>,
    pub visible_zones: BTreeSet<ZoneId>,
    pub statistics: CullStatistics,
}

impl CullPlan {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_pass(&mut self, pass: RenderPass) {
        self.visible_zones.insert(pass.zone_id);
        self.passes.push(pass);
    }

    #[must_use]
    pub fn pass_count(&self) -> usize {
        self.passes.len()
    }

    #[must_use]
    pub fn primary_pass(&self) -> Option<&RenderPass> {
        self.passes.iter().find(|p| p.is_primary())
    }

    #[must_use]
    pub fn portal_passes(&self) -> Vec<&RenderPass> {
        self.passes.iter().filter(|p| !p.is_primary()).collect()
    }

    #[must_use]
    pub fn total_region_count(&self) -> usize {
        self.passes.iter().map(RenderPass::region_count).sum()
    }

    #[must_use]
    pub fn max_depth(&self) -> u32 {
        self.passes
            .iter()
            .map(|p| p.portal_depth)
            .max()
            .unwrap_or(0)
    }

    pub fn sort_by_depth(&mut self) {
        self.passes.sort_by_key(|p| p.portal_depth);
    }
}

#[derive(Clone)]
struct QueueEntry {
    zone_id: ZoneId,
    frustum: Frustum,
    depth: u32,
    source_portal: Option<(engine_world::portal::PortalId, PortalSide)>,
}

pub struct CullPlanner {
    config: CullPlannerConfig,
    next_pass_id: u64,
}

impl CullPlanner {
    #[must_use]
    pub fn new(config: CullPlannerConfig) -> Self {
        Self {
            config,
            next_pass_id: 0,
        }
    }

    #[must_use]
    pub fn config(&self) -> &CullPlannerConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut CullPlannerConfig {
        &mut self.config
    }

    fn next_pass_id(&mut self) -> RenderPassId {
        let id = RenderPassId::from_raw(self.next_pass_id);
        self.next_pass_id += 1;
        id
    }

    #[expect(clippy::cast_possible_truncation, reason = "counts fit in u32")]
    #[expect(clippy::too_many_lines, reason = "plan() is a cohesive algorithm")]
    pub fn plan(
        &mut self,
        camera_zone: ZoneId,
        camera_frustum: Frustum,
        portal_graph: &PortalGraph,
        regions: &mut CullRegionSet,
    ) -> CullPlan {
        let mut plan = CullPlan::new();
        let mut visited_zones: BTreeMap<ZoneId, u32> = BTreeMap::new();

        regions.reset_states();

        let mut queue = VecDeque::new();
        queue.push_back(QueueEntry {
            zone_id: camera_zone,
            frustum: camera_frustum,
            depth: 0,
            source_portal: None,
        });

        while let Some(entry) = queue.pop_front() {
            if entry.depth > self.config.max_portal_depth {
                continue;
            }

            if plan.passes.len() >= self.config.max_render_passes as usize {
                break;
            }

            if visited_zones
                .get(&entry.zone_id)
                .is_some_and(|&prev_depth| prev_depth <= entry.depth)
            {
                continue;
            }
            visited_zones.insert(entry.zone_id, entry.depth);

            let pass_id = self.next_pass_id();
            let mut pass = RenderPass::new(pass_id, entry.zone_id, entry.frustum.clone());
            pass.portal_depth = entry.depth;
            pass.source_portal = entry.source_portal;

            let visible_in_zone: Vec<_> = {
                let zone_regions = regions.regions_in_zone(entry.zone_id);
                zone_regions
                    .into_iter()
                    .filter_map(|region| {
                        let result = cull_sphere(&entry.frustum, region.center, region.radius);
                        if result.is_visible() {
                            let aabb_result =
                                cull_aabb(&entry.frustum, region.bounds_min, region.bounds_max);
                            let state = match aabb_result {
                                CullResult::Inside => CullRegionState::FullyVisible,
                                CullResult::Intersecting => CullRegionState::Visible,
                                CullResult::Outside => CullRegionState::Hidden,
                            };
                            if state.needs_render() {
                                return Some((region.id, state));
                            }
                        }
                        None
                    })
                    .collect()
            };

            for (region_id, state) in visible_in_zone {
                regions.set_state(region_id, state);
                if let Some(r) = regions.get_mut(region_id) {
                    r.portal_depth = entry.depth;
                }
                pass.region_ids.push(region_id);
            }

            if pass.region_ids.len() > self.config.max_regions_per_pass as usize {
                pass.region_ids
                    .truncate(self.config.max_regions_per_pass as usize);
            }

            plan.add_pass(pass);

            if entry.depth < self.config.max_portal_depth {
                for portal in portal_graph.zone_portals(entry.zone_id) {
                    let Some(side) = portal_graph.portal_side_from_zone(portal.id, entry.zone_id)
                    else {
                        continue;
                    };

                    if !portal.can_traverse(side) {
                        continue;
                    }

                    let endpoint = portal.entry_endpoint(side);
                    if self.config.cull_behind_portal && !endpoint.is_in_front(entry.frustum.origin)
                    {
                        continue;
                    }

                    let portal_visible = self.is_portal_visible(&entry.frustum, portal, side);
                    if !portal_visible {
                        continue;
                    }

                    let exit_zone = portal.exit_endpoint(side).zone;

                    let clipped_frustum = if self.config.use_portal_frustum_clipping {
                        self.clip_frustum_to_portal(&entry.frustum, portal, side)
                    } else {
                        let transform = portal.transform(side);
                        entry.frustum.transform(transform.matrix())
                    };

                    queue.push_back(QueueEntry {
                        zone_id: exit_zone,
                        frustum: clipped_frustum,
                        depth: entry.depth + 1,
                        source_portal: Some((portal.id, side)),
                    });
                }
            }
        }

        plan.statistics.total_regions = regions.len() as u32;
        plan.statistics.visible_regions = regions.visible_regions().len() as u32;
        plan.statistics.hidden_regions =
            plan.statistics.total_regions - plan.statistics.visible_regions;
        plan.statistics.portal_traversals = plan.portal_passes().len() as u32;
        plan.statistics.max_portal_depth = plan.max_depth();

        plan.sort_by_depth();
        plan
    }

    #[expect(clippy::unused_self, reason = "method for future config access")]
    fn is_portal_visible(&self, frustum: &Frustum, portal: &Portal, side: PortalSide) -> bool {
        let endpoint = portal.entry_endpoint(side);
        let corners = endpoint.corners();

        let center = corners.iter().fold(Vec3::ZERO, |a, &b| a + b) / 4.0;
        let radius = corners
            .iter()
            .map(|&c| (c - center).length())
            .fold(0.0f32, f32::max);

        frustum.intersects_sphere(center, radius)
    }

    #[expect(clippy::unused_self, reason = "method for future config access")]
    fn clip_frustum_to_portal(
        &self,
        frustum: &Frustum,
        portal: &Portal,
        side: PortalSide,
    ) -> Frustum {
        let entry_endpoint = portal.entry_endpoint(side);
        let exit_endpoint = portal.exit_endpoint(side);
        let corners = entry_endpoint.corners();

        let clipped = frustum.clip_to_portal(&corners, entry_endpoint.forward);

        let transform = portal.transform(side);
        Frustum {
            planes: [
                clipped.planes[0].transform(transform.matrix()),
                clipped.planes[1].transform(transform.matrix()),
                clipped.planes[2].transform(transform.matrix()),
                clipped.planes[3].transform(transform.matrix()),
                clipped.planes[4].transform(transform.matrix()),
                clipped.planes[5].transform(transform.matrix()),
            ],
            origin: transform.transform_point(frustum.origin),
            forward: transform
                .transform_direction(exit_endpoint.forward)
                .normalize(),
        }
    }

    pub fn reset(&mut self) {
        self.next_pass_id = 0;
    }
}

impl Default for CullPlanner {
    fn default() -> Self {
        Self::new(CullPlannerConfig::default())
    }
}

pub fn sort_regions_by_distance(regions: &mut [&CullRegion], origin: Vec3) {
    regions.sort_by(|a, b| {
        let da = (a.center - origin).length_squared();
        let db = (b.center - origin).length_squared();
        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
    });
}

pub fn sort_regions_by_depth(regions: &mut [&CullRegion]) {
    regions.sort_by_key(|r| r.portal_depth);
}

pub fn group_regions_by_zone(regions: &[&CullRegion]) -> BTreeMap<ZoneId, Vec<CullRegionId>> {
    let mut groups: BTreeMap<ZoneId, Vec<CullRegionId>> = BTreeMap::new();
    for region in regions {
        groups.entry(region.zone_id).or_default().push(region.id);
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_world::portal::{PortalEndpoint, PortalId, ZoneMetadata};
    use glam::Mat4;

    fn test_frustum() -> Frustum {
        let view = Mat4::look_at_rh(Vec3::ZERO, -Vec3::Z, Vec3::Y);
        let proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0);
        Frustum::from_view_projection(proj * view, Vec3::ZERO, -Vec3::Z)
    }

    fn test_graph_and_regions() -> (PortalGraph, CullRegionSet) {
        let mut graph = PortalGraph::new();
        graph.add_zone(ZoneId::new(0, 0), ZoneMetadata::new().with_name("zone_a"));
        graph.add_zone(ZoneId::new(0, 1), ZoneMetadata::new().with_name("zone_b"));

        let portal = Portal::new(
            PortalId::new(0, 0),
            PortalEndpoint::rectangle(
                ZoneId::new(0, 0),
                Vec3::new(0.0, 0.0, -20.0),
                -Vec3::Z,
                Vec3::Y,
                5.0,
                5.0,
            ),
            PortalEndpoint::rectangle(
                ZoneId::new(0, 1),
                Vec3::new(100.0, 0.0, 0.0),
                Vec3::Z,
                Vec3::Y,
                5.0,
                5.0,
            ),
        );
        graph.add_portal(portal);

        let mut regions = CullRegionSet::new();
        regions.add(
            ZoneId::new(0, 0),
            Vec3::new(-5.0, -5.0, -15.0),
            Vec3::new(5.0, 5.0, -5.0),
        );
        regions.add(
            ZoneId::new(0, 1),
            Vec3::new(95.0, -5.0, -5.0),
            Vec3::new(105.0, 5.0, 5.0),
        );

        (graph, regions)
    }

    #[test]
    fn planner_creation() {
        let planner = CullPlanner::new(CullPlannerConfig::default());
        assert_eq!(planner.config().max_portal_depth, 8);
    }

    #[test]
    fn config_builder() {
        let config = CullPlannerConfig::default()
            .with_max_depth(4)
            .with_max_regions(1000);

        assert_eq!(config.max_portal_depth, 4);
        assert_eq!(config.max_regions_per_pass, 1000);
    }

    #[test]
    fn basic_plan() {
        let (graph, mut regions) = test_graph_and_regions();
        let mut planner = CullPlanner::default();
        let frustum = test_frustum();

        let plan = planner.plan(ZoneId::new(0, 0), frustum, &graph, &mut regions);

        assert!(plan.pass_count() >= 1);
        assert!(plan.primary_pass().is_some());
    }

    #[test]
    fn render_pass_properties() {
        let pass = RenderPass::new(RenderPassId::from_raw(0), ZoneId::new(0, 0), test_frustum())
            .with_depth(2)
            .with_source_portal(PortalId::new(0, 0), PortalSide::AtoB);

        assert_eq!(pass.portal_depth, 2);
        assert!(!pass.is_primary());
        assert!(pass.source_portal.is_some());
    }

    #[test]
    fn cull_plan_stats() {
        let (graph, mut regions) = test_graph_and_regions();
        let mut planner = CullPlanner::default();
        let frustum = test_frustum();

        let plan = planner.plan(ZoneId::new(0, 0), frustum, &graph, &mut regions);

        assert!(plan.statistics.total_regions > 0);
    }

    #[test]
    fn sort_by_distance() {
        let r1 = CullRegion::from_sphere(
            CullRegionId::from_raw(1),
            ZoneId::new(0, 0),
            Vec3::new(10.0, 0.0, 0.0),
            1.0,
        );
        let r2 = CullRegion::from_sphere(
            CullRegionId::from_raw(2),
            ZoneId::new(0, 0),
            Vec3::new(5.0, 0.0, 0.0),
            1.0,
        );

        let mut refs: Vec<&CullRegion> = vec![&r1, &r2];
        sort_regions_by_distance(&mut refs, Vec3::ZERO);

        assert_eq!(refs[0].id, CullRegionId::from_raw(2));
    }

    #[test]
    fn group_by_zone() {
        let r1 = CullRegion::new(
            CullRegionId::from_raw(1),
            ZoneId::new(0, 0),
            Vec3::ZERO,
            Vec3::ONE,
        );
        let r2 = CullRegion::new(
            CullRegionId::from_raw(2),
            ZoneId::new(0, 1),
            Vec3::ZERO,
            Vec3::ONE,
        );
        let r3 = CullRegion::new(
            CullRegionId::from_raw(3),
            ZoneId::new(0, 0),
            Vec3::ZERO,
            Vec3::ONE,
        );

        let refs: Vec<&CullRegion> = vec![&r1, &r2, &r3];
        let groups = group_regions_by_zone(&refs);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups.get(&ZoneId::new(0, 0)).unwrap().len(), 2);
        assert_eq!(groups.get(&ZoneId::new(0, 1)).unwrap().len(), 1);
    }

    #[test]
    fn serde_roundtrip() {
        let config = CullPlannerConfig::default().with_max_depth(5);
        let json = serde_json::to_string(&config).unwrap();
        let recovered: CullPlannerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.max_portal_depth, recovered.max_portal_depth);
    }
}
