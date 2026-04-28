//! Region event journal for postmortem debugging and simulation recovery.
//!
//! Provides an append-only, deterministic journal of events scoped to regions/chunks.
//! Useful for debugging simulation issues, tracking state changes, and recovering
//! from crashes or synchronization failures.
//!
//! # Overview
//!
//! - [`EventRecord`] - Individual journal entries with tick, position, severity, payload
//! - [`EventCategory`] / [`EventKind`] - Typed event classification
//! - [`Severity`] - Event importance levels for filtering
//! - [`RegionJournal`] - Main journal with query, compaction, and checksum APIs
//! - [`JournalQuery`] - Flexible query builder for filtering records
//! - [`RecoverySummary`] - Aggregated stats for simulation recovery
//!
//! # Example
//!
//! ```
//! use engine_core::coords::ChunkPos;
//! use engine_world::region_journal::{
//!     EventKind, EventPayload, EventRecord, JournalQuery, RegionJournal, Severity,
//! };
//!
//! let mut journal = RegionJournal::new("world_1");
//!
//! // Append events
//! journal.append_simple(
//!     100,
//!     ChunkPos::new(0, 0, 0),
//!     EventKind::ChunkLoaded,
//!     Severity::Info,
//! );
//!
//! journal.append(
//!     EventRecord::new(150, 0, ChunkPos::new(0, 0, 0), EventKind::HazardSpawn)
//!         .with_severity(Severity::Warning)
//!         .with_tag("fire")
//!         .with_payload(EventPayload::with_primary(42)),
//! );
//!
//! // Query events
//! let warnings: Vec<_> = journal
//!     .query(&JournalQuery::new().with_min_severity(Severity::Warning))
//!     .collect();
//!
//! // Generate recovery summary
//! let summary = journal.recovery_summary(0, 200);
//! ```

mod event_category;
mod event_record;
mod journal;
mod recovery;
mod severity;

pub use event_category::{EventCategory, EventKind};
pub use event_record::{EventPayload, EventRecord};
pub use journal::{JournalQuery, RegionJournal};
pub use recovery::{CategoryStats, RecoverySummary, RegionSummary};
pub use severity::Severity;
