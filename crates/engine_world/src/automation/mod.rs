//! Base automation state replication primitives.
//!
//! Provides deterministic, serde-covered foundation for multiplayer-safe
//! automation networks including:
//!
//! - [`AutomationDevice`] - Devices/nodes with typed ports and signals
//! - [`AutomationLink`] - Directional connections between device ports
//! - [`SignalValue`] - Type-safe signal values (bool, int, float, color)
//! - [`AutomationNetwork`] - Network management with tick simulation
//! - [`AutomationSnapshot`] / [`AutomationDeltaBatch`] - Compact replication
//! - [`RevisionTracker`] - Authoritative revision tracking with checksums
//!
//! # Multiplayer Synchronization
//!
//! The automation system supports both full snapshots for late-joiners and
//! incremental deltas for ongoing sync:
//!
//! ```ignore
//! // Server: capture state for new client
//! let snapshot = network.snapshot();
//! send_to_client(snapshot);
//!
//! // Server: send deltas to connected clients
//! let delta = network.delta_since(client_revision);
//! if !delta.is_empty() {
//!     send_to_client(delta);
//! }
//!
//! // Client: apply updates
//! network.apply_delta(&delta);
//! assert_eq!(network.checksum(), delta.checksum);
//! ```
//!
//! # Determinism
//!
//! All operations are deterministic with stable ordering:
//! - Devices ordered by (position, id)
//! - Links ordered by (source, port, target, port)
//! - Changes ordered by (tick, revision, kind, id)
//! - Checksums computed over ordered state

mod device;
mod link;
mod network;
mod revision;
mod signal;
mod snapshot;

pub use device::{AutomationDevice, DeviceConfig, DeviceId, DeviceKind, MAX_PORTS, PortState};
pub use link::{AutomationLink, LinkId, PendingSignal};
pub use network::{AutomationConfig, AutomationNetwork, TickResult};
pub use revision::{
    ChangeKind, ChangePayload, DeviceChangePayload, LinkChangePayload, Revision, RevisionTracker,
    StateChange,
};
pub use signal::{PortId, SignalValue};
pub use snapshot::{AutomationDeltaBatch, AutomationSnapshot, DeviceDelta, SpatialFilter};
