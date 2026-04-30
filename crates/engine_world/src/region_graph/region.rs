//! Region node in the graph.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use super::annotation::{HazardAnnotation, MissionAnnotation, ResourceAnnotation};
use super::gate::{GateRequirement, ProgressionTier};
use super::region_id::RegionId;
use super::region_kind::{RegionKind, RegionTag};

/// A region node in the graph.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RegionNode {
    /// Unique identifier.
    pub id: RegionId,
    /// Kind of region.
    pub kind: RegionKind,
    /// Display name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Progression tier required to access.
    pub tier: ProgressionTier,
    /// Additional tags.
    pub tags: BTreeSet<RegionTag>,
    /// Gate requirement (if any).
    pub gate: Option<GateRequirement>,
    /// Resource annotations.
    pub resources: Vec<ResourceAnnotation>,
    /// Hazard annotations.
    pub hazards: Vec<HazardAnnotation>,
    /// Mission annotations.
    pub missions: Vec<MissionAnnotation>,
    /// Abstract position for layout (x, y).
    pub position: (i32, i32),
    /// Size/importance weight (1-10).
    pub weight: u8,
    /// Whether this region has been visited.
    pub visited: bool,
    /// Whether this region is currently visible.
    pub visible: bool,
}

impl RegionNode {
    /// Create a new region node.
    #[must_use]
    pub fn new(id: RegionId, kind: RegionKind) -> Self {
        Self {
            id,
            kind,
            name: String::new(),
            description: String::new(),
            tier: ProgressionTier::START,
            tags: BTreeSet::new(),
            gate: None,
            resources: Vec::new(),
            hazards: Vec::new(),
            missions: Vec::new(),
            position: (0, 0),
            weight: 5,
            visited: false,
            visible: true,
        }
    }

    /// Set the name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set the description.
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set the progression tier.
    #[must_use]
    pub fn with_tier(mut self, tier: u8) -> Self {
        self.tier = ProgressionTier::new(tier);
        self
    }

    /// Add a tag.
    #[must_use]
    pub fn with_tag(mut self, tag: RegionTag) -> Self {
        self.tags.insert(tag);
        self
    }

    /// Set the gate requirement.
    #[must_use]
    pub fn with_gate(mut self, gate: GateRequirement) -> Self {
        self.gate = Some(gate);
        self
    }

    /// Add a resource annotation.
    #[must_use]
    pub fn with_resource(mut self, resource: ResourceAnnotation) -> Self {
        self.resources.push(resource);
        self
    }

    /// Add a hazard annotation.
    #[must_use]
    pub fn with_hazard(mut self, hazard: HazardAnnotation) -> Self {
        self.hazards.push(hazard);
        self
    }

    /// Add a mission annotation.
    #[must_use]
    pub fn with_mission(mut self, mission: MissionAnnotation) -> Self {
        self.missions.push(mission);
        self
    }

    /// Set the position.
    #[must_use]
    pub fn with_position(mut self, x: i32, y: i32) -> Self {
        self.position = (x, y);
        self
    }

    /// Set the weight.
    #[must_use]
    pub fn with_weight(mut self, weight: u8) -> Self {
        self.weight = weight.clamp(1, 10);
        self
    }

    /// Mark as hidden.
    #[must_use]
    pub fn hidden(mut self) -> Self {
        self.visible = false;
        self
    }

    /// Check if this region has a specific tag.
    #[must_use]
    pub fn has_tag(&self, tag: RegionTag) -> bool {
        self.tags.contains(&tag)
    }

    /// Check if this region is a dead end.
    #[must_use]
    pub fn is_dead_end(&self) -> bool {
        self.has_tag(RegionTag::DeadEnd)
    }

    /// Check if this region is a branch point.
    #[must_use]
    pub fn is_branch(&self) -> bool {
        self.has_tag(RegionTag::Branch)
    }

    /// Check if this region is a chokepoint.
    #[must_use]
    pub fn is_chokepoint(&self) -> bool {
        self.has_tag(RegionTag::Chokepoint)
    }

    /// Check if this region is on the critical path.
    #[must_use]
    pub fn is_critical(&self) -> bool {
        self.has_tag(RegionTag::Critical)
    }

    /// Check if this region is accessible at the given tier.
    #[must_use]
    pub fn is_accessible(&self, player_tier: ProgressionTier) -> bool {
        if !self.tier.is_accessible_at(player_tier) {
            return false;
        }
        if let Some(gate) = &self.gate {
            return gate.is_accessible(player_tier);
        }
        true
    }

    /// Check if this region has any active hazards.
    #[must_use]
    pub fn has_active_hazards(&self) -> bool {
        self.hazards.iter().any(|h| h.active)
    }

    /// Check if this region has any dangerous hazards.
    #[must_use]
    pub fn has_dangerous_hazards(&self) -> bool {
        self.hazards.iter().any(|h| h.active && h.is_dangerous())
    }

    /// Get total resource quantity across all annotations.
    #[must_use]
    pub fn total_resources(&self) -> u32 {
        self.resources.iter().map(|r| r.quantity).sum()
    }

    /// Get the highest hazard severity.
    #[must_use]
    pub fn max_hazard_severity(&self) -> u8 {
        self.hazards
            .iter()
            .filter(|h| h.active)
            .map(|h| h.severity)
            .max()
            .unwrap_or(0)
    }

    /// Mark as visited.
    pub fn visit(&mut self) {
        self.visited = true;
    }

    /// Reveal this region.
    pub fn reveal(&mut self) {
        self.visible = true;
    }
}

impl Default for RegionNode {
    fn default() -> Self {
        Self::new(RegionId::default(), RegionKind::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_creation() {
        let region = RegionNode::new(RegionId::new(1, 1), RegionKind::Station)
            .with_name("Alpha Station")
            .with_tier(2)
            .with_tag(RegionTag::Critical);

        assert_eq!(region.name, "Alpha Station");
        assert_eq!(region.tier, ProgressionTier::new(2));
        assert!(region.has_tag(RegionTag::Critical));
    }

    #[test]
    fn region_accessibility() {
        let region = RegionNode::new(RegionId::new(1, 1), RegionKind::Cave).with_tier(3);

        assert!(!region.is_accessible(ProgressionTier::new(2)));
        assert!(region.is_accessible(ProgressionTier::new(3)));
        assert!(region.is_accessible(ProgressionTier::new(5)));
    }

    #[test]
    fn region_hazards() {
        let region = RegionNode::new(RegionId::new(1, 1), RegionKind::Hazard)
            .with_hazard(HazardAnnotation::new("radiation", 7))
            .with_hazard(HazardAnnotation::new("toxic", 3).inactive());

        assert!(region.has_active_hazards());
        assert!(region.has_dangerous_hazards());
        assert_eq!(region.max_hazard_severity(), 7);
    }

    #[test]
    fn region_resources() {
        let region = RegionNode::new(RegionId::new(1, 1), RegionKind::Resource)
            .with_resource(ResourceAnnotation::new("iron", 100))
            .with_resource(ResourceAnnotation::new("copper", 50));

        assert_eq!(region.total_resources(), 150);
    }

    #[test]
    fn region_tags() {
        let region = RegionNode::new(RegionId::new(1, 1), RegionKind::Hub)
            .with_tag(RegionTag::Branch)
            .with_tag(RegionTag::Safe);

        assert!(region.is_branch());
        assert!(!region.is_dead_end());
        assert!(!region.is_chokepoint());
    }

    #[test]
    fn serde_roundtrip() {
        let region = RegionNode::new(RegionId::new(42, 1), RegionKind::Station)
            .with_name("Test Station")
            .with_tier(2)
            .with_resource(ResourceAnnotation::new("iron", 100));

        let json = serde_json::to_string(&region).unwrap();
        let recovered: RegionNode = serde_json::from_str(&json).unwrap();
        assert_eq!(region, recovered);
    }
}
