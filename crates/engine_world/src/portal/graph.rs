//! Portal graph structure for zone connectivity.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::id::{PortalId, ZoneId};
use super::portal::{Portal, PortalSide};
use super::validation::{PortalValidationError, PortalValidationErrors};

/// Metadata for a zone in the portal graph.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ZoneMetadata {
    /// Optional name for debugging.
    pub name: Option<String>,
    /// Tags for zone categorization.
    pub tags: Vec<String>,
    /// Whether this zone is currently loaded.
    pub loaded: bool,
    /// Priority for loading (higher = load first).
    pub priority: i32,
}

impl ZoneMetadata {
    /// Create new zone metadata.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the zone name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Add a tag.
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set loaded state.
    #[must_use]
    pub fn with_loaded(mut self, loaded: bool) -> Self {
        self.loaded = loaded;
        self
    }

    /// Set priority.
    #[must_use]
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
}

/// A graph of zones connected by portals.
///
/// The portal graph tracks zone connectivity and enables efficient
/// traversal queries. Zones are vertices, portals are edges.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PortalGraph {
    /// All portals in the graph.
    portals: BTreeMap<PortalId, Portal>,
    /// Zone metadata.
    zones: BTreeMap<ZoneId, ZoneMetadata>,
    /// Portals indexed by zone (for each zone, which portals connect to it).
    zone_portals: BTreeMap<ZoneId, BTreeSet<PortalId>>,
    /// Generation counter for change tracking.
    generation: u64,
}

impl PortalGraph {
    /// Create an empty portal graph.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a zone to the graph.
    ///
    /// Returns false if the zone already exists.
    pub fn add_zone(&mut self, id: ZoneId, metadata: ZoneMetadata) -> bool {
        if self.zones.contains_key(&id) {
            return false;
        }
        self.zones.insert(id, metadata);
        self.zone_portals.entry(id).or_default();
        self.generation += 1;
        true
    }

    /// Add a portal to the graph.
    ///
    /// The zones referenced by the portal endpoints are created if they don't exist.
    /// Returns false if the portal ID already exists.
    pub fn add_portal(&mut self, portal: Portal) -> bool {
        if self.portals.contains_key(&portal.id) {
            return false;
        }

        let zone_a = portal.endpoint_a.zone;
        let zone_b = portal.endpoint_b.zone;

        self.zones.entry(zone_a).or_default();
        self.zones.entry(zone_b).or_default();

        self.zone_portals
            .entry(zone_a)
            .or_default()
            .insert(portal.id);
        self.zone_portals
            .entry(zone_b)
            .or_default()
            .insert(portal.id);

        self.portals.insert(portal.id, portal);
        self.generation += 1;
        true
    }

    /// Remove a portal from the graph.
    ///
    /// Returns the removed portal, or None if not found.
    pub fn remove_portal(&mut self, id: PortalId) -> Option<Portal> {
        let portal = self.portals.remove(&id)?;

        if let Some(set) = self.zone_portals.get_mut(&portal.endpoint_a.zone) {
            set.remove(&id);
        }
        if let Some(set) = self.zone_portals.get_mut(&portal.endpoint_b.zone) {
            set.remove(&id);
        }

        self.generation += 1;
        Some(portal)
    }

    /// Remove a zone and all its portals.
    ///
    /// Returns the zone metadata if found.
    pub fn remove_zone(&mut self, id: ZoneId) -> Option<ZoneMetadata> {
        let portal_ids: Vec<_> = self
            .zone_portals
            .get(&id)
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default();

        for portal_id in portal_ids {
            self.remove_portal(portal_id);
        }

        self.zone_portals.remove(&id);
        let metadata = self.zones.remove(&id);
        if metadata.is_some() {
            self.generation += 1;
        }
        metadata
    }

    /// Get a portal by ID.
    #[must_use]
    pub fn portal(&self, id: PortalId) -> Option<&Portal> {
        self.portals.get(&id)
    }

    /// Get a mutable portal by ID.
    pub fn portal_mut(&mut self, id: PortalId) -> Option<&mut Portal> {
        self.generation += 1;
        self.portals.get_mut(&id)
    }

    /// Get zone metadata.
    #[must_use]
    pub fn zone(&self, id: ZoneId) -> Option<&ZoneMetadata> {
        self.zones.get(&id)
    }

    /// Get mutable zone metadata.
    pub fn zone_mut(&mut self, id: ZoneId) -> Option<&mut ZoneMetadata> {
        self.generation += 1;
        self.zones.get_mut(&id)
    }

    /// Get all portal IDs.
    pub fn portal_ids(&self) -> impl Iterator<Item = PortalId> + '_ {
        self.portals.keys().copied()
    }

    /// Get all portals.
    pub fn portals(&self) -> impl Iterator<Item = &Portal> {
        self.portals.values()
    }

    /// Get all zone IDs.
    pub fn zone_ids(&self) -> impl Iterator<Item = ZoneId> + '_ {
        self.zones.keys().copied()
    }

    /// Get portals connected to a zone.
    pub fn zone_portal_ids(&self, zone: ZoneId) -> impl Iterator<Item = PortalId> + '_ {
        self.zone_portals
            .get(&zone)
            .into_iter()
            .flat_map(|s| s.iter().copied())
    }

    /// Get portals connected to a zone as references.
    #[must_use]
    pub fn zone_portals(&self, zone: ZoneId) -> Vec<&Portal> {
        self.zone_portal_ids(zone)
            .filter_map(|id| self.portals.get(&id))
            .collect()
    }

    /// Get zones connected to a given zone via active portals.
    #[must_use]
    pub fn connected_zones(&self, zone: ZoneId) -> Vec<ZoneId> {
        let mut result = Vec::new();
        for portal in self.zone_portals(zone) {
            if !portal.is_active() {
                continue;
            }
            if portal.endpoint_a.zone == zone {
                result.push(portal.endpoint_b.zone);
            }
            if portal.endpoint_b.zone == zone && portal.flags.bidirectional {
                result.push(portal.endpoint_a.zone);
            }
        }
        result.sort();
        result.dedup();
        result
    }

    /// Count total portals.
    #[must_use]
    pub fn portal_count(&self) -> usize {
        self.portals.len()
    }

    /// Count total zones.
    #[must_use]
    pub fn zone_count(&self) -> usize {
        self.zones.len()
    }

    /// Get the current generation (for change tracking).
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Check if a zone exists.
    #[must_use]
    pub fn has_zone(&self, id: ZoneId) -> bool {
        self.zones.contains_key(&id)
    }

    /// Check if a portal exists.
    #[must_use]
    pub fn has_portal(&self, id: PortalId) -> bool {
        self.portals.contains_key(&id)
    }

    /// Find all portals between two zones.
    #[must_use]
    pub fn portals_between(&self, zone_a: ZoneId, zone_b: ZoneId) -> Vec<&Portal> {
        self.zone_portals(zone_a)
            .into_iter()
            .filter(|p| {
                (p.endpoint_a.zone == zone_a && p.endpoint_b.zone == zone_b)
                    || (p.endpoint_a.zone == zone_b && p.endpoint_b.zone == zone_a)
            })
            .collect()
    }

    /// Determine the traversal side for a portal when entering from a zone.
    #[must_use]
    pub fn portal_side_from_zone(
        &self,
        portal_id: PortalId,
        from_zone: ZoneId,
    ) -> Option<PortalSide> {
        let portal = self.portals.get(&portal_id)?;
        if portal.endpoint_a.zone == from_zone {
            Some(PortalSide::AtoB)
        } else if portal.endpoint_b.zone == from_zone {
            Some(PortalSide::BtoA)
        } else {
            None
        }
    }

    /// Validate the portal graph for consistency.
    #[must_use]
    pub fn validate(&self) -> PortalValidationErrors {
        let mut errors = PortalValidationErrors::new();

        for portal in self.portals.values() {
            if !self.zones.contains_key(&portal.endpoint_a.zone) {
                errors.add(PortalValidationError::MissingZone {
                    portal_id: portal.id,
                    zone_id: portal.endpoint_a.zone,
                });
            }
            if !self.zones.contains_key(&portal.endpoint_b.zone) {
                errors.add(PortalValidationError::MissingZone {
                    portal_id: portal.id,
                    zone_id: portal.endpoint_b.zone,
                });
            }
            if portal.endpoint_a.zone == portal.endpoint_b.zone {
                errors.add(PortalValidationError::SameZoneEndpoints {
                    portal_id: portal.id,
                    zone_id: portal.endpoint_a.zone,
                });
            }
        }

        for (zone_id, portal_ids) in &self.zone_portals {
            for portal_id in portal_ids {
                if !self.portals.contains_key(portal_id) {
                    errors.add(PortalValidationError::MissingPortal {
                        zone_id: *zone_id,
                        portal_id: *portal_id,
                    });
                }
            }
        }

        errors
    }

    /// Clear all portals and zones.
    pub fn clear(&mut self) {
        self.portals.clear();
        self.zones.clear();
        self.zone_portals.clear();
        self.generation += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portal::endpoint::PortalEndpoint;
    use glam::Vec3;

    fn test_graph() -> PortalGraph {
        let mut graph = PortalGraph::new();

        graph.add_zone(ZoneId::new(0, 0), ZoneMetadata::new().with_name("zone_a"));
        graph.add_zone(ZoneId::new(0, 1), ZoneMetadata::new().with_name("zone_b"));
        graph.add_zone(ZoneId::new(0, 2), ZoneMetadata::new().with_name("zone_c"));

        let portal_ab = Portal::new(
            PortalId::new(0, 0),
            PortalEndpoint::rectangle(ZoneId::new(0, 0), Vec3::ZERO, Vec3::Z, Vec3::Y, 2.0, 3.0),
            PortalEndpoint::rectangle(
                ZoneId::new(0, 1),
                Vec3::new(10.0, 0.0, 0.0),
                -Vec3::Z,
                Vec3::Y,
                2.0,
                3.0,
            ),
        );
        let portal_bc = Portal::new(
            PortalId::new(0, 1),
            PortalEndpoint::rectangle(
                ZoneId::new(0, 1),
                Vec3::new(20.0, 0.0, 0.0),
                Vec3::Z,
                Vec3::Y,
                2.0,
                3.0,
            ),
            PortalEndpoint::rectangle(
                ZoneId::new(0, 2),
                Vec3::new(30.0, 0.0, 0.0),
                -Vec3::Z,
                Vec3::Y,
                2.0,
                3.0,
            ),
        );

        graph.add_portal(portal_ab);
        graph.add_portal(portal_bc);

        graph
    }

    #[test]
    fn add_and_get_zones() {
        let graph = test_graph();
        assert_eq!(graph.zone_count(), 3);
        assert!(graph.has_zone(ZoneId::new(0, 0)));
        assert!(graph.has_zone(ZoneId::new(0, 1)));
        assert!(graph.has_zone(ZoneId::new(0, 2)));
    }

    #[test]
    fn add_and_get_portals() {
        let graph = test_graph();
        assert_eq!(graph.portal_count(), 2);
        assert!(graph.has_portal(PortalId::new(0, 0)));
        assert!(graph.has_portal(PortalId::new(0, 1)));
    }

    #[test]
    fn zone_portals() {
        let graph = test_graph();
        let portals_a: Vec<_> = graph.zone_portal_ids(ZoneId::new(0, 0)).collect();
        let portals_b: Vec<_> = graph.zone_portal_ids(ZoneId::new(0, 1)).collect();

        assert_eq!(portals_a.len(), 1);
        assert_eq!(portals_b.len(), 2);
    }

    #[test]
    fn connected_zones() {
        let graph = test_graph();
        let connected = graph.connected_zones(ZoneId::new(0, 1));
        assert_eq!(connected.len(), 2);
        assert!(connected.contains(&ZoneId::new(0, 0)));
        assert!(connected.contains(&ZoneId::new(0, 2)));
    }

    #[test]
    fn portals_between() {
        let graph = test_graph();
        let between = graph.portals_between(ZoneId::new(0, 0), ZoneId::new(0, 1));
        assert_eq!(between.len(), 1);
    }

    #[test]
    fn remove_portal() {
        let mut graph = test_graph();
        let removed = graph.remove_portal(PortalId::new(0, 0));
        assert!(removed.is_some());
        assert_eq!(graph.portal_count(), 1);
        assert!(!graph.has_portal(PortalId::new(0, 0)));
    }

    #[test]
    fn remove_zone() {
        let mut graph = test_graph();
        let removed = graph.remove_zone(ZoneId::new(0, 1));
        assert!(removed.is_some());
        assert_eq!(graph.zone_count(), 2);
        assert_eq!(graph.portal_count(), 0);
    }

    #[test]
    fn portal_side_from_zone() {
        let graph = test_graph();
        let side_a = graph.portal_side_from_zone(PortalId::new(0, 0), ZoneId::new(0, 0));
        let side_b = graph.portal_side_from_zone(PortalId::new(0, 0), ZoneId::new(0, 1));

        assert_eq!(side_a, Some(PortalSide::AtoB));
        assert_eq!(side_b, Some(PortalSide::BtoA));
    }

    #[test]
    fn validate_clean_graph() {
        let graph = test_graph();
        let errors = graph.validate();
        assert!(errors.is_empty());
    }

    #[test]
    fn generation_tracking() {
        let mut graph = PortalGraph::new();
        let gen0 = graph.generation();

        graph.add_zone(ZoneId::new(0, 0), ZoneMetadata::new());
        let gen1 = graph.generation();
        assert!(gen1 > gen0);

        let endpoint =
            PortalEndpoint::rectangle(ZoneId::new(0, 0), Vec3::ZERO, Vec3::Z, Vec3::Y, 1.0, 1.0);
        graph.add_portal(Portal::new(PortalId::new(0, 0), endpoint.clone(), endpoint));
        let gen2 = graph.generation();
        assert!(gen2 > gen1);
    }

    #[test]
    fn duplicate_add_fails() {
        let mut graph = test_graph();
        let result = graph.add_zone(ZoneId::new(0, 0), ZoneMetadata::new());
        assert!(!result);

        let endpoint =
            PortalEndpoint::rectangle(ZoneId::new(0, 0), Vec3::ZERO, Vec3::Z, Vec3::Y, 1.0, 1.0);
        let result = graph.add_portal(Portal::new(PortalId::new(0, 0), endpoint.clone(), endpoint));
        assert!(!result);
    }

    #[test]
    fn serde_roundtrip() {
        let graph = test_graph();
        let serialized = bincode::serialize(&graph).unwrap();
        let deserialized: PortalGraph = bincode::deserialize(&serialized).unwrap();
        assert_eq!(graph.portal_count(), deserialized.portal_count());
        assert_eq!(graph.zone_count(), deserialized.zone_count());
    }

    #[test]
    fn clear() {
        let mut graph = test_graph();
        graph.clear();
        assert_eq!(graph.portal_count(), 0);
        assert_eq!(graph.zone_count(), 0);
    }
}
