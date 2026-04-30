//! Annotations for resources, hazards, and missions on regions.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Resource annotation for a region.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceAnnotation {
    /// Resource type identifier.
    pub resource_type: String,
    /// Estimated quantity (abstract units).
    pub quantity: u32,
    /// Quality tier (0 = common, higher = rarer).
    pub quality: u8,
    /// Whether the resource is renewable.
    pub renewable: bool,
    /// Custom properties.
    pub properties: BTreeMap<String, String>,
}

impl ResourceAnnotation {
    /// Create a new resource annotation.
    #[must_use]
    pub fn new(resource_type: impl Into<String>, quantity: u32) -> Self {
        Self {
            resource_type: resource_type.into(),
            quantity,
            quality: 0,
            renewable: false,
            properties: BTreeMap::new(),
        }
    }

    /// Set quality tier.
    #[must_use]
    pub fn with_quality(mut self, quality: u8) -> Self {
        self.quality = quality;
        self
    }

    /// Mark as renewable.
    #[must_use]
    pub fn renewable(mut self) -> Self {
        self.renewable = true;
        self
    }

    /// Add a custom property.
    #[must_use]
    pub fn with_property(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.properties.insert(key.into(), value.into());
        self
    }
}

impl Default for ResourceAnnotation {
    fn default() -> Self {
        Self::new("generic", 0)
    }
}

/// Hazard annotation for a region.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HazardAnnotation {
    /// Hazard type identifier.
    pub hazard_type: String,
    /// Severity level (0-10).
    pub severity: u8,
    /// Whether the hazard is active.
    pub active: bool,
    /// Whether the hazard can be disabled.
    pub disableable: bool,
    /// Custom properties.
    pub properties: BTreeMap<String, String>,
}

impl HazardAnnotation {
    /// Create a new hazard annotation.
    #[must_use]
    pub fn new(hazard_type: impl Into<String>, severity: u8) -> Self {
        Self {
            hazard_type: hazard_type.into(),
            severity: severity.min(10),
            active: true,
            disableable: false,
            properties: BTreeMap::new(),
        }
    }

    /// Mark as inactive.
    #[must_use]
    pub fn inactive(mut self) -> Self {
        self.active = false;
        self
    }

    /// Mark as disableable.
    #[must_use]
    pub fn disableable(mut self) -> Self {
        self.disableable = true;
        self
    }

    /// Add a custom property.
    #[must_use]
    pub fn with_property(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.properties.insert(key.into(), value.into());
        self
    }

    /// Check if this hazard is dangerous (severity >= 5).
    #[must_use]
    pub const fn is_dangerous(&self) -> bool {
        self.severity >= 5
    }

    /// Check if this hazard is lethal (severity >= 8).
    #[must_use]
    pub const fn is_lethal(&self) -> bool {
        self.severity >= 8
    }
}

impl Default for HazardAnnotation {
    fn default() -> Self {
        Self::new("generic", 1)
    }
}

/// Mission annotation for a region.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissionAnnotation {
    /// Mission identifier.
    pub mission_id: String,
    /// Objective index within the mission.
    pub objective_index: u32,
    /// Role this region plays in the mission.
    pub role: MissionRole,
    /// Whether this is the primary location for the objective.
    pub primary: bool,
    /// Custom properties.
    pub properties: BTreeMap<String, String>,
}

/// Role a region plays in a mission.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum MissionRole {
    /// Start point for the mission.
    #[default]
    Start = 0,
    /// End point or goal.
    End = 1,
    /// Objective location.
    Objective = 2,
    /// Waypoint along the route.
    Waypoint = 3,
    /// Pickup location.
    Pickup = 4,
    /// Delivery location.
    Delivery = 5,
    /// Defend location.
    Defend = 6,
    /// Explore target.
    Explore = 7,
}

impl MissionRole {
    /// Get the name of this role.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::End => "end",
            Self::Objective => "objective",
            Self::Waypoint => "waypoint",
            Self::Pickup => "pickup",
            Self::Delivery => "delivery",
            Self::Defend => "defend",
            Self::Explore => "explore",
        }
    }
}

impl MissionAnnotation {
    /// Create a new mission annotation.
    #[must_use]
    pub fn new(mission_id: impl Into<String>, objective_index: u32, role: MissionRole) -> Self {
        Self {
            mission_id: mission_id.into(),
            objective_index,
            role,
            primary: true,
            properties: BTreeMap::new(),
        }
    }

    /// Create a start annotation.
    #[must_use]
    pub fn start(mission_id: impl Into<String>) -> Self {
        Self::new(mission_id, 0, MissionRole::Start)
    }

    /// Create an end annotation.
    #[must_use]
    pub fn end(mission_id: impl Into<String>) -> Self {
        Self::new(mission_id, 0, MissionRole::End)
    }

    /// Create an objective annotation.
    #[must_use]
    pub fn objective(mission_id: impl Into<String>, index: u32) -> Self {
        Self::new(mission_id, index, MissionRole::Objective)
    }

    /// Mark as secondary location.
    #[must_use]
    pub fn secondary(mut self) -> Self {
        self.primary = false;
        self
    }

    /// Add a custom property.
    #[must_use]
    pub fn with_property(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.properties.insert(key.into(), value.into());
        self
    }
}

impl Default for MissionAnnotation {
    fn default() -> Self {
        Self::start("default")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_annotation() {
        let res = ResourceAnnotation::new("iron", 100)
            .with_quality(2)
            .renewable();

        assert_eq!(res.resource_type, "iron");
        assert_eq!(res.quantity, 100);
        assert_eq!(res.quality, 2);
        assert!(res.renewable);
    }

    #[test]
    fn hazard_annotation() {
        let haz = HazardAnnotation::new("radiation", 7).disableable();

        assert!(haz.is_dangerous());
        assert!(!haz.is_lethal());
        assert!(haz.disableable);

        let lethal = HazardAnnotation::new("void", 10);
        assert!(lethal.is_lethal());
    }

    #[test]
    fn mission_annotation() {
        let ann = MissionAnnotation::objective("supply_run", 2).secondary();

        assert_eq!(ann.mission_id, "supply_run");
        assert_eq!(ann.objective_index, 2);
        assert_eq!(ann.role, MissionRole::Objective);
        assert!(!ann.primary);
    }

    #[test]
    fn serde_roundtrip() {
        let res = ResourceAnnotation::new("gold", 50).with_quality(3);
        let json = serde_json::to_string(&res).unwrap();
        let recovered: ResourceAnnotation = serde_json::from_str(&json).unwrap();
        assert_eq!(res, recovered);

        let haz = HazardAnnotation::new("fire", 6);
        let json = serde_json::to_string(&haz).unwrap();
        let recovered: HazardAnnotation = serde_json::from_str(&json).unwrap();
        assert_eq!(haz, recovered);

        let miss = MissionAnnotation::start("rescue");
        let json = serde_json::to_string(&miss).unwrap();
        let recovered: MissionAnnotation = serde_json::from_str(&json).unwrap();
        assert_eq!(miss, recovered);
    }
}
