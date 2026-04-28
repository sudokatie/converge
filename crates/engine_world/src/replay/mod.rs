//! Deterministic simulation replay hooks for debugging world events and desyncs.
//!
//! This module provides recording and playback APIs for simulation state,
//! enabling deterministic replay and mismatch detection for debugging purposes.
//!
//! # Overview
//!
//! - [`ReplayMetadata`]: Session-level information (seed, ticks, config)
//! - [`ReplayRecorder`]: Records simulation events and checksums
//! - [`ReplayVerifier`]: Compares recorded data against a replayed run
//! - [`Mismatch`]: Details about detected discrepancies
//!
//! # Example
//!
//! ```
//! use engine_world::replay::{ReplayMetadata, ReplayRecorder, ReplayVerifier};
//!
//! // Recording a session
//! let metadata = ReplayMetadata::new("world_seed_123", 0, 1000);
//! let mut recorder = ReplayRecorder::new(metadata);
//! recorder.record_world_event_start(100, 1, "Eclipse");
//! recorder.record_step_checksum(100, 0xDEAD_BEEF);
//!
//! // Verifying against replay
//! let mut verifier = ReplayVerifier::from_recorder(&recorder);
//! verifier.verify_world_event_start(100, 1, "Eclipse"); // Ok
//! verifier.verify_step_checksum(100, 0xDEAD_BEEF);       // Ok
//! assert!(verifier.mismatches().is_empty());
//! ```

mod checksum;
mod metadata;
mod playback;
mod record;

pub use checksum::{ChecksumBuilder, StepChecksum};
pub use metadata::ReplayMetadata;
pub use playback::{Mismatch, MismatchKind, ReplayVerifier};
pub use record::{ReplayEntry, ReplayEntryKind, ReplayRecorder};
