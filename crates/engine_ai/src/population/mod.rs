//! Population director for spawn pressure, pacing, migration, and regional threat scaling.
//!
//! Provides deterministic population management across regions and chunks:
//!
//! - Population pressure and spawn/despawn budgets
//! - Pacing intensity and regional threat scaling
//! - Migration waves and routes between regions
//! - Species and group population caps
//! - Cooldowns for spawn/despawn events
//! - Safe/hostile zone biasing
//! - Cheap summaries for unloaded regions

mod budget;
mod director;
mod migration;
mod region;
mod species;
mod summary;
mod threat;

pub use budget::{
    DespawnBudget, DespawnReason, PacingIntensity, PacingProfile, SpawnBudget, SpawnEvent,
    SpawnPriority,
};
pub use director::{
    PopulationConfig, PopulationDirector, PopulationEvent, PopulationEventKind, TickResult,
};
pub use migration::{
    MigrationConfig, MigrationPhase, MigrationRoute, MigrationRouteId, MigrationStatus,
    MigrationWave, MigrationWaveId,
};
pub use region::{RegionalPopulation, ZoneBias};
pub use species::{GroupCap, GroupCapId, SpeciesCap, SpeciesCapId, SpeciesRegistry};
pub use summary::{PopulationSnapshot, PopulationSummary, RegionDensity, SpawnPressure};
pub use threat::{ThreatConfig, ThreatLevel, ThreatModifier, ThreatSource};
