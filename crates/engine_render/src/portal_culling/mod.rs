//! Portal-aware frustum culling for non-euclidean rendering.
//!
//! This module provides CPU-side planning for portal-based visibility:
//!
//! - **Frustum culling**: Standard and portal-clipped frustum tests
//! - **Cull regions**: Spatial bounds for visibility tracking
//! - **Cull planning**: Multi-pass render planning through portals
//!
//! # Architecture
//!
//! The culling system works with the `engine_world::portal` module to plan
//! multi-pass rendering through portal networks. Each render pass covers
//! a single zone, with the frustum clipped to the visible portal opening.
//!
//! # Example
//!
//! ```ignore
//! use engine_render::portal_culling::*;
//! use engine_world::portal::{PortalGraph, ZoneId};
//! use glam::{Mat4, Vec3};
//!
//! // Create planner and regions
//! let mut planner = CullPlanner::default();
//! let mut regions = CullRegionSet::new();
//!
//! // Add regions for each zone
//! regions.add(ZoneId::new(0, 0), Vec3::ZERO, Vec3::splat(10.0));
//!
//! // Build camera frustum
//! let view = Mat4::look_at_rh(Vec3::ZERO, -Vec3::Z, Vec3::Y);
//! let proj = Mat4::perspective_rh(1.57, 1.0, 0.1, 100.0);
//! let frustum = Frustum::from_view_projection(proj * view, Vec3::ZERO, -Vec3::Z);
//!
//! // Plan rendering
//! let portal_graph = PortalGraph::new();
//! let plan = planner.plan(ZoneId::new(0, 0), frustum, &portal_graph, &mut regions);
//!
//! for pass in &plan.passes {
//!     // Render regions for this pass
//!     for region_id in &pass.region_ids {
//!         // Submit draw calls...
//!     }
//! }
//! ```

mod frustum;
mod id;
mod planner;
mod region;

pub use frustum::{CullResult, Frustum, Plane, cull_aabb, cull_sphere};
pub use id::{CullRegionId, RenderPassId};
pub use planner::{
    CullPlan, CullPlanner, CullPlannerConfig, RenderPass, group_regions_by_zone,
    sort_regions_by_depth, sort_regions_by_distance,
};
pub use region::{CullRegion, CullRegionSet, CullRegionState, CullStatistics};

use std::hash::{Hash, Hasher};

#[must_use]
pub fn compute_fingerprint(plan: &CullPlan) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    plan.passes.len().hash(&mut hasher);
    for pass in &plan.passes {
        pass.zone_id.raw().hash(&mut hasher);
        pass.portal_depth.hash(&mut hasher);
        pass.region_ids.len().hash(&mut hasher);
    }
    plan.statistics.visible_regions.hash(&mut hasher);
    hasher.finish()
}

pub fn sort_passes_by_depth(passes: &mut [RenderPass]) {
    passes.sort_by_key(|p| p.portal_depth);
}

pub fn filter_visible_passes(passes: &[RenderPass]) -> Vec<&RenderPass> {
    passes.iter().filter(|p| !p.region_ids.is_empty()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Mat4, Vec3};

    fn test_frustum() -> Frustum {
        let view = Mat4::look_at_rh(Vec3::ZERO, -Vec3::Z, Vec3::Y);
        let proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0);
        Frustum::from_view_projection(proj * view, Vec3::ZERO, -Vec3::Z)
    }

    #[test]
    fn public_exports_available() {
        let _ = CullRegionId::from_raw(0);
        let _ = RenderPassId::from_raw(0);
        let _ = CullResult::Inside;
        let _ = CullRegionState::Visible;
        let _ = CullPlannerConfig::default();
        let _ = CullStatistics::new();
    }

    #[test]
    fn basic_frustum_cull() {
        let frustum = test_frustum();
        let inside = cull_sphere(&frustum, Vec3::new(0.0, 0.0, -10.0), 1.0);
        assert!(inside.is_visible());
    }

    #[test]
    fn region_set_workflow() {
        use engine_world::portal::ZoneId;

        let mut set = CullRegionSet::new();
        let zone = ZoneId::new(0, 0);

        let id = set.add(zone, Vec3::ZERO, Vec3::splat(10.0));
        assert_eq!(set.len(), 1);

        set.set_state(id, CullRegionState::Visible);
        assert_eq!(set.renderable_regions().len(), 1);
    }

    #[test]
    fn fingerprint_determinism() {
        let plan1 = CullPlan::new();
        let plan2 = CullPlan::new();

        assert_eq!(compute_fingerprint(&plan1), compute_fingerprint(&plan2));
    }

    #[test]
    fn filter_visible() {
        use engine_world::portal::ZoneId;

        let empty_pass =
            RenderPass::new(RenderPassId::from_raw(0), ZoneId::new(0, 0), test_frustum());
        let mut nonempty_pass =
            RenderPass::new(RenderPassId::from_raw(1), ZoneId::new(0, 0), test_frustum());
        nonempty_pass.region_ids.push(CullRegionId::from_raw(1));

        let passes = vec![empty_pass, nonempty_pass];
        let visible = filter_visible_passes(&passes);

        assert_eq!(visible.len(), 1);
    }
}
