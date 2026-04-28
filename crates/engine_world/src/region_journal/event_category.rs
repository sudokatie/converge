//! Event category definitions for the region journal.

use serde::{Deserialize, Serialize};

/// High-level category of journal events.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum EventCategory {
    /// World-level events (eclipses, season changes, global state).
    World = 0,

    /// Scheduler and job execution events.
    Scheduler = 1,

    /// Hazard and environmental events (fire, flood, atmosphere).
    Environment = 2,

    /// Chunk mutation and persistence events.
    ChunkMutation = 3,

    /// Entity and population events (spawns, deaths, migrations).
    Entity = 4,

    /// Network synchronization and recovery events.
    Network = 5,

    /// Custom or debug events from external systems.
    Custom = 6,
}

impl EventCategory {
    /// Total number of event categories.
    pub const COUNT: usize = 7;

    /// All categories in order.
    pub const ALL: [EventCategory; Self::COUNT] = [
        EventCategory::World,
        EventCategory::Scheduler,
        EventCategory::Environment,
        EventCategory::ChunkMutation,
        EventCategory::Entity,
        EventCategory::Network,
        EventCategory::Custom,
    ];

    /// Convert to array index.
    #[must_use]
    pub const fn as_index(self) -> usize {
        self as usize
    }

    /// Create from array index.
    #[must_use]
    pub const fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(EventCategory::World),
            1 => Some(EventCategory::Scheduler),
            2 => Some(EventCategory::Environment),
            3 => Some(EventCategory::ChunkMutation),
            4 => Some(EventCategory::Entity),
            5 => Some(EventCategory::Network),
            6 => Some(EventCategory::Custom),
            _ => None,
        }
    }

    /// Get the display name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            EventCategory::World => "World",
            EventCategory::Scheduler => "Scheduler",
            EventCategory::Environment => "Environment",
            EventCategory::ChunkMutation => "ChunkMutation",
            EventCategory::Entity => "Entity",
            EventCategory::Network => "Network",
            EventCategory::Custom => "Custom",
        }
    }
}

/// Specific event kind within a category.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum EventKind {
    // World events (0-19)
    /// World event started.
    WorldEventStart = 0,
    /// World event ended.
    WorldEventEnd = 1,
    /// Season transition.
    SeasonChange = 2,
    /// Timeline tick processed.
    TimelineTick = 3,

    // Scheduler events (20-39)
    /// Simulation job dispatched.
    JobDispatched = 20,
    /// Simulation job completed.
    JobCompleted = 21,
    /// Fidelity level changed.
    FidelityChange = 22,
    /// Region state changed.
    RegionStateChange = 23,

    // Environment events (40-59)
    /// Hazard spawned.
    HazardSpawn = 40,
    /// Hazard spread.
    HazardSpread = 41,
    /// Hazard extinguished.
    HazardExtinguish = 42,
    /// Fluid transport occurred.
    FluidTransport = 43,
    /// Structural change (collapse, support).
    StructuralChange = 44,
    /// Atmosphere change (pressure, temperature).
    AtmosphereChange = 45,
    /// Conduit network change.
    ConduitChange = 46,

    // Chunk mutation events (60-79)
    /// Chunk loaded from storage.
    ChunkLoaded = 60,
    /// Chunk unloaded/persisted.
    ChunkUnloaded = 61,
    /// Block(s) modified.
    BlockModified = 62,
    /// Chunk generated.
    ChunkGenerated = 63,
    /// Chunk delta applied.
    DeltaApplied = 64,
    /// Chunk state snapshot.
    StateSnapshot = 65,

    // Entity events (80-99)
    /// Entity spawned.
    EntitySpawn = 80,
    /// Entity despawned.
    EntityDespawn = 81,
    /// Entity migrated between regions.
    EntityMigration = 82,
    /// Population threshold reached.
    PopulationThreshold = 83,

    // Network events (100-119)
    /// Region recovery started.
    RecoveryStart = 100,
    /// Region recovery completed.
    RecoveryComplete = 101,
    /// Sync mismatch detected.
    SyncMismatch = 102,
    /// Checkpoint created.
    Checkpoint = 103,
    /// State rollback.
    Rollback = 104,

    // Custom events (120+)
    /// User-defined event.
    UserDefined = 120,
    /// Debug marker.
    DebugMarker = 121,
    /// Profiling event.
    ProfilingEvent = 122,
}

impl EventKind {
    /// Get the category this kind belongs to.
    #[must_use]
    pub const fn category(self) -> EventCategory {
        match self as u8 {
            0..=19 => EventCategory::World,
            20..=39 => EventCategory::Scheduler,
            40..=59 => EventCategory::Environment,
            60..=79 => EventCategory::ChunkMutation,
            80..=99 => EventCategory::Entity,
            100..=119 => EventCategory::Network,
            _ => EventCategory::Custom,
        }
    }

    /// Get the display name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            EventKind::WorldEventStart => "WorldEventStart",
            EventKind::WorldEventEnd => "WorldEventEnd",
            EventKind::SeasonChange => "SeasonChange",
            EventKind::TimelineTick => "TimelineTick",
            EventKind::JobDispatched => "JobDispatched",
            EventKind::JobCompleted => "JobCompleted",
            EventKind::FidelityChange => "FidelityChange",
            EventKind::RegionStateChange => "RegionStateChange",
            EventKind::HazardSpawn => "HazardSpawn",
            EventKind::HazardSpread => "HazardSpread",
            EventKind::HazardExtinguish => "HazardExtinguish",
            EventKind::FluidTransport => "FluidTransport",
            EventKind::StructuralChange => "StructuralChange",
            EventKind::AtmosphereChange => "AtmosphereChange",
            EventKind::ConduitChange => "ConduitChange",
            EventKind::ChunkLoaded => "ChunkLoaded",
            EventKind::ChunkUnloaded => "ChunkUnloaded",
            EventKind::BlockModified => "BlockModified",
            EventKind::ChunkGenerated => "ChunkGenerated",
            EventKind::DeltaApplied => "DeltaApplied",
            EventKind::StateSnapshot => "StateSnapshot",
            EventKind::EntitySpawn => "EntitySpawn",
            EventKind::EntityDespawn => "EntityDespawn",
            EventKind::EntityMigration => "EntityMigration",
            EventKind::PopulationThreshold => "PopulationThreshold",
            EventKind::RecoveryStart => "RecoveryStart",
            EventKind::RecoveryComplete => "RecoveryComplete",
            EventKind::SyncMismatch => "SyncMismatch",
            EventKind::Checkpoint => "Checkpoint",
            EventKind::Rollback => "Rollback",
            EventKind::UserDefined => "UserDefined",
            EventKind::DebugMarker => "DebugMarker",
            EventKind::ProfilingEvent => "ProfilingEvent",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_count_matches_all() {
        assert_eq!(EventCategory::ALL.len(), EventCategory::COUNT);
    }

    #[test]
    fn category_index_round_trip() {
        for cat in EventCategory::ALL {
            let index = cat.as_index();
            let recovered = EventCategory::from_index(index);
            assert_eq!(recovered, Some(cat));
        }
    }

    #[test]
    fn category_from_index_out_of_range() {
        assert_eq!(EventCategory::from_index(7), None);
        assert_eq!(EventCategory::from_index(255), None);
    }

    #[test]
    fn kind_category_mapping() {
        assert_eq!(EventKind::WorldEventStart.category(), EventCategory::World);
        assert_eq!(
            EventKind::JobDispatched.category(),
            EventCategory::Scheduler
        );
        assert_eq!(
            EventKind::HazardSpawn.category(),
            EventCategory::Environment
        );
        assert_eq!(
            EventKind::ChunkLoaded.category(),
            EventCategory::ChunkMutation
        );
        assert_eq!(EventKind::EntitySpawn.category(), EventCategory::Entity);
        assert_eq!(EventKind::RecoveryStart.category(), EventCategory::Network);
        assert_eq!(EventKind::UserDefined.category(), EventCategory::Custom);
    }

    #[test]
    fn kind_names_not_empty() {
        assert!(!EventKind::WorldEventStart.name().is_empty());
        assert!(!EventKind::DebugMarker.name().is_empty());
    }

    #[test]
    fn serde_round_trip_category() {
        for cat in EventCategory::ALL {
            let json = serde_json::to_string(&cat).unwrap();
            let recovered: EventCategory = serde_json::from_str(&json).unwrap();
            assert_eq!(recovered, cat);
        }
    }

    #[test]
    fn serde_round_trip_kind() {
        let kinds = [
            EventKind::WorldEventStart,
            EventKind::JobDispatched,
            EventKind::HazardSpawn,
            EventKind::ChunkLoaded,
            EventKind::EntitySpawn,
            EventKind::RecoveryStart,
            EventKind::UserDefined,
        ];
        for kind in kinds {
            let json = serde_json::to_string(&kind).unwrap();
            let recovered: EventKind = serde_json::from_str(&json).unwrap();
            assert_eq!(recovered, kind);
        }
    }
}
