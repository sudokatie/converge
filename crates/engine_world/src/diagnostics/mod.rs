//! In-world diagnostic overlay primitives for simulation fields and chunk-state debugging.
//!
//! Provides CPU-side data structures for sampling, querying, and visualizing environmental
//! fields, hazards, fluids, structural integrity, conduits, atmosphere, and scheduler state.
//! These primitives are renderer-agnostic and suitable for consumption by egui, gizmo
//! renderers, or custom visualization systems.

mod channel;
mod color;
mod filter;
mod fingerprint;
mod legend;
mod overlay;
mod sample;
mod summary;

pub use channel::{DiagnosticCategory, DiagnosticChannel};
pub use color::{ChannelPalette, DiagnosticColor};
pub use filter::{DiagnosticFilter, FilterMode};
pub use fingerprint::DiagnosticFingerprint;
pub use legend::{DiagnosticLegend, LegendEntry};
pub use overlay::{MarkerKind, OverlayMarker, OverlaySpec};
pub use sample::{SampleCell, SampleGrid, ScalarValue, VectorValue};
pub use summary::{CategoryCounts, ChannelStats, DiagnosticSummary};
