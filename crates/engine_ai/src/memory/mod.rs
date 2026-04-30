//! Creature memory system for danger zones, food sources, and player traces.
//!
//! Provides a framework for creatures to remember and recall information about
//! their environment with strength-based decay, region-scoped queries, and
//! deterministic ordering for replay/network sync.

mod config;
mod record;
mod store;
mod summary;

pub use config::{DecayConfig, MemoryStoreConfig};
pub use record::{
    DangerCategory, DangerZoneMemory, FoodCategory, FoodSourceMemory, MemoryCategory, MemoryId,
    MemoryRecord, MemorySource, MemoryTag, PlayerTraceKind, PlayerTraceMemory, RegionScope,
};
pub use store::{CreatureMemory, MemoryQuery, MemoryQueryBuilder, QueryResult};
pub use summary::{MemoryFingerprint, MemorySnapshot, MemorySummary, RegionMemorySummary};
