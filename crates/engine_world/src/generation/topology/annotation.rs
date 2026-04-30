//! Annotations for topology elements (hazards, resources, mission hooks).

use serde::{Deserialize, Serialize};

use super::node::NodeId;
use super::segment::SegmentId;

/// Type of hazard.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum HazardType {
    /// Structural instability.
    Unstable = 0,
    /// Toxic atmosphere.
    Toxic = 1,
    /// Extreme cold.
    Freezing = 2,
    /// Extreme heat.
    Burning = 3,
    /// Radiation.
    Radiation = 4,
    /// Low pressure / vacuum.
    Vacuum = 5,
    /// High pressure.
    Crushing = 6,
    /// Hostile creatures.
    Creatures = 7,
    /// Electrical hazard.
    Electrical = 8,
    /// Flooding / water.
    Flooding = 9,
}

impl HazardType {
    /// Get the name of this hazard type.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Unstable => "unstable",
            Self::Toxic => "toxic",
            Self::Freezing => "freezing",
            Self::Burning => "burning",
            Self::Radiation => "radiation",
            Self::Vacuum => "vacuum",
            Self::Crushing => "crushing",
            Self::Creatures => "creatures",
            Self::Electrical => "electrical",
            Self::Flooding => "flooding",
        }
    }

    /// Get danger level (1-5).
    #[must_use]
    pub const fn danger_level(&self) -> u8 {
        match self {
            Self::Creatures | Self::Electrical => 2,
            Self::Toxic | Self::Freezing | Self::Flooding => 3,
            Self::Burning | Self::Radiation => 4,
            Self::Vacuum | Self::Crushing | Self::Unstable => 5,
        }
    }

    /// Create from raw value.
    #[must_use]
    pub const fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Unstable),
            1 => Some(Self::Toxic),
            2 => Some(Self::Freezing),
            3 => Some(Self::Burning),
            4 => Some(Self::Radiation),
            5 => Some(Self::Vacuum),
            6 => Some(Self::Crushing),
            7 => Some(Self::Creatures),
            8 => Some(Self::Electrical),
            9 => Some(Self::Flooding),
            _ => None,
        }
    }

    /// Get raw value.
    #[must_use]
    pub const fn as_raw(self) -> u8 {
        self as u8
    }
}

/// Type of resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum ResourceType {
    /// Metal ore.
    Ore = 0,
    /// Energy crystals.
    Crystal = 1,
    /// Fuel deposits.
    Fuel = 2,
    /// Water/ice.
    Water = 3,
    /// Rare minerals.
    Mineral = 4,
    /// Salvage/scrap.
    Salvage = 5,
    /// Biological samples.
    Biological = 6,
    /// Ancient artifacts.
    Artifact = 7,
}

impl ResourceType {
    /// Get the name of this resource type.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Ore => "ore",
            Self::Crystal => "crystal",
            Self::Fuel => "fuel",
            Self::Water => "water",
            Self::Mineral => "mineral",
            Self::Salvage => "salvage",
            Self::Biological => "biological",
            Self::Artifact => "artifact",
        }
    }

    /// Get rarity level (1-5).
    #[must_use]
    pub const fn rarity(&self) -> u8 {
        match self {
            Self::Ore | Self::Water => 1,
            Self::Fuel | Self::Salvage => 2,
            Self::Crystal | Self::Mineral => 3,
            Self::Biological => 4,
            Self::Artifact => 5,
        }
    }

    /// Create from raw value.
    #[must_use]
    pub const fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Ore),
            1 => Some(Self::Crystal),
            2 => Some(Self::Fuel),
            3 => Some(Self::Water),
            4 => Some(Self::Mineral),
            5 => Some(Self::Salvage),
            6 => Some(Self::Biological),
            7 => Some(Self::Artifact),
            _ => None,
        }
    }

    /// Get raw value.
    #[must_use]
    pub const fn as_raw(self) -> u8 {
        self as u8
    }
}

/// Mission hook type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum MissionHook {
    /// Objective location.
    Objective = 0,
    /// Item pickup.
    Pickup = 1,
    /// NPC encounter.
    Encounter = 2,
    /// Data terminal.
    Terminal = 3,
    /// Rescue target.
    Rescue = 4,
    /// Activation target.
    Activation = 5,
    /// Boss location.
    Boss = 6,
    /// Secret area.
    Secret = 7,
}

impl MissionHook {
    /// Get the name of this hook.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Objective => "objective",
            Self::Pickup => "pickup",
            Self::Encounter => "encounter",
            Self::Terminal => "terminal",
            Self::Rescue => "rescue",
            Self::Activation => "activation",
            Self::Boss => "boss",
            Self::Secret => "secret",
        }
    }

    /// Get priority (1-5).
    #[must_use]
    pub const fn priority(&self) -> u8 {
        match self {
            Self::Pickup | Self::Terminal => 1,
            Self::Encounter | Self::Secret => 2,
            Self::Activation => 3,
            Self::Rescue | Self::Objective => 4,
            Self::Boss => 5,
        }
    }

    /// Create from raw value.
    #[must_use]
    pub const fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Objective),
            1 => Some(Self::Pickup),
            2 => Some(Self::Encounter),
            3 => Some(Self::Terminal),
            4 => Some(Self::Rescue),
            5 => Some(Self::Activation),
            6 => Some(Self::Boss),
            7 => Some(Self::Secret),
            _ => None,
        }
    }

    /// Get raw value.
    #[must_use]
    pub const fn as_raw(self) -> u8 {
        self as u8
    }
}

/// A topology annotation.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TopologyAnnotation {
    /// Hazard annotation on a node.
    NodeHazard {
        node: NodeId,
        hazard: HazardType,
        intensity: u8,
    },
    /// Hazard annotation on a segment.
    SegmentHazard {
        segment: SegmentId,
        hazard: HazardType,
        intensity: u8,
    },
    /// Resource annotation on a node.
    NodeResource {
        node: NodeId,
        resource: ResourceType,
        quantity: u32,
    },
    /// Mission hook on a node.
    NodeMission { node: NodeId, hook: MissionHook },
}

impl TopologyAnnotation {
    /// Create a node hazard annotation.
    #[must_use]
    pub fn node_hazard(node: NodeId, hazard: HazardType, intensity: u8) -> Self {
        Self::NodeHazard {
            node,
            hazard,
            intensity: intensity.min(10),
        }
    }

    /// Create a segment hazard annotation.
    #[must_use]
    pub fn segment_hazard(segment: SegmentId, hazard: HazardType, intensity: u8) -> Self {
        Self::SegmentHazard {
            segment,
            hazard,
            intensity: intensity.min(10),
        }
    }

    /// Create a node resource annotation.
    #[must_use]
    pub fn node_resource(node: NodeId, resource: ResourceType, quantity: u32) -> Self {
        Self::NodeResource {
            node,
            resource,
            quantity,
        }
    }

    /// Create a node mission hook.
    #[must_use]
    pub fn node_mission(node: NodeId, hook: MissionHook) -> Self {
        Self::NodeMission { node, hook }
    }

    /// Get the node ID if this annotation is on a node.
    #[must_use]
    pub fn node(&self) -> Option<NodeId> {
        match self {
            Self::NodeHazard { node, .. }
            | Self::NodeResource { node, .. }
            | Self::NodeMission { node, .. } => Some(*node),
            Self::SegmentHazard { .. } => None,
        }
    }

    /// Get the segment ID if this annotation is on a segment.
    #[must_use]
    pub fn segment(&self) -> Option<SegmentId> {
        match self {
            Self::SegmentHazard { segment, .. } => Some(*segment),
            _ => None,
        }
    }

    /// Check if this is a hazard annotation.
    #[must_use]
    pub const fn is_hazard(&self) -> bool {
        matches!(self, Self::NodeHazard { .. } | Self::SegmentHazard { .. })
    }

    /// Check if this is a resource annotation.
    #[must_use]
    pub const fn is_resource(&self) -> bool {
        matches!(self, Self::NodeResource { .. })
    }

    /// Check if this is a mission annotation.
    #[must_use]
    pub const fn is_mission(&self) -> bool {
        matches!(self, Self::NodeMission { .. })
    }
}

/// Collection of annotations for a topology.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyAnnotations {
    annotations: Vec<TopologyAnnotation>,
}

impl TopologyAnnotations {
    /// Create an empty annotation collection.
    #[must_use]
    pub fn new() -> Self {
        Self {
            annotations: Vec::new(),
        }
    }

    /// Add an annotation.
    pub fn add(&mut self, annotation: TopologyAnnotation) {
        self.annotations.push(annotation);
    }

    /// Get all annotations.
    #[must_use]
    pub fn all(&self) -> &[TopologyAnnotation] {
        &self.annotations
    }

    /// Get annotations for a specific node.
    pub fn for_node(&self, node: NodeId) -> impl Iterator<Item = &TopologyAnnotation> {
        self.annotations
            .iter()
            .filter(move |a| a.node() == Some(node))
    }

    /// Get annotations for a specific segment.
    pub fn for_segment(&self, segment: SegmentId) -> impl Iterator<Item = &TopologyAnnotation> {
        self.annotations
            .iter()
            .filter(move |a| a.segment() == Some(segment))
    }

    /// Get all hazard annotations.
    pub fn hazards(&self) -> impl Iterator<Item = &TopologyAnnotation> {
        self.annotations.iter().filter(|a| a.is_hazard())
    }

    /// Get all resource annotations.
    pub fn resources(&self) -> impl Iterator<Item = &TopologyAnnotation> {
        self.annotations.iter().filter(|a| a.is_resource())
    }

    /// Get all mission annotations.
    pub fn missions(&self) -> impl Iterator<Item = &TopologyAnnotation> {
        self.annotations.iter().filter(|a| a.is_mission())
    }

    /// Get the number of annotations.
    #[must_use]
    pub fn len(&self) -> usize {
        self.annotations.len()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.annotations.is_empty()
    }

    /// Clear all annotations.
    pub fn clear(&mut self) {
        self.annotations.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hazard_properties() {
        assert_eq!(HazardType::Vacuum.danger_level(), 5);
        assert_eq!(HazardType::Creatures.danger_level(), 2);
    }

    #[test]
    fn resource_properties() {
        assert_eq!(ResourceType::Ore.rarity(), 1);
        assert_eq!(ResourceType::Artifact.rarity(), 5);
    }

    #[test]
    fn mission_hook_properties() {
        assert_eq!(MissionHook::Boss.priority(), 5);
        assert_eq!(MissionHook::Pickup.priority(), 1);
    }

    #[test]
    fn annotation_creation() {
        let hazard = TopologyAnnotation::node_hazard(NodeId::new(1), HazardType::Toxic, 5);
        assert!(hazard.is_hazard());
        assert_eq!(hazard.node(), Some(NodeId::new(1)));

        let resource =
            TopologyAnnotation::node_resource(NodeId::new(2), ResourceType::Crystal, 100);
        assert!(resource.is_resource());

        let mission = TopologyAnnotation::node_mission(NodeId::new(3), MissionHook::Objective);
        assert!(mission.is_mission());
    }

    #[test]
    fn annotations_collection() {
        let mut annotations = TopologyAnnotations::new();
        assert!(annotations.is_empty());

        annotations.add(TopologyAnnotation::node_hazard(
            NodeId::new(1),
            HazardType::Toxic,
            5,
        ));
        annotations.add(TopologyAnnotation::node_resource(
            NodeId::new(1),
            ResourceType::Ore,
            50,
        ));
        annotations.add(TopologyAnnotation::node_hazard(
            NodeId::new(2),
            HazardType::Radiation,
            3,
        ));

        assert_eq!(annotations.len(), 3);
        assert_eq!(annotations.hazards().count(), 2);
        assert_eq!(annotations.resources().count(), 1);
        assert_eq!(annotations.for_node(NodeId::new(1)).count(), 2);
    }

    #[test]
    fn from_raw_roundtrip() {
        for i in 0..10 {
            if let Some(h) = HazardType::from_raw(i) {
                assert_eq!(h.as_raw(), i);
            }
        }
        for i in 0..8 {
            if let Some(r) = ResourceType::from_raw(i) {
                assert_eq!(r.as_raw(), i);
            }
        }
        for i in 0..8 {
            if let Some(m) = MissionHook::from_raw(i) {
                assert_eq!(m.as_raw(), i);
            }
        }
    }

    #[test]
    fn serde_roundtrip() {
        let mut annotations = TopologyAnnotations::new();
        annotations.add(TopologyAnnotation::node_hazard(
            NodeId::new(1),
            HazardType::Freezing,
            7,
        ));
        annotations.add(TopologyAnnotation::segment_hazard(
            SegmentId::new(5),
            HazardType::Unstable,
            4,
        ));

        let json = serde_json::to_string(&annotations).unwrap();
        let recovered: TopologyAnnotations = serde_json::from_str(&json).unwrap();
        assert_eq!(annotations, recovered);
    }
}
