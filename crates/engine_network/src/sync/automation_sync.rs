//! Automation state synchronization for clients.
//!
//! Provides compact delta transmission and late-join synchronization for
//! automation networks using server-authoritative revisions.

use std::collections::BTreeMap;

use engine_core::coords::ChunkPos;
use engine_world::{AutomationDeltaBatch, AutomationSnapshot, Revision, SpatialFilter};
use serde::{Deserialize, Serialize};

/// Client-side automation synchronization state.
///
/// Tracks the last acknowledged revision and pending deltas to apply.
#[derive(Clone, Debug, Default)]
pub struct ClientAutomationSync {
    /// Last revision fully acknowledged by client.
    last_ack_revision: Revision,
    /// Last tick acknowledged.
    last_ack_tick: u64,
    /// Pending delta batches awaiting application.
    pending_deltas: Vec<AutomationDeltaBatch>,
    /// Interest bounds for filtering (if set).
    interest_filter: SpatialFilter,
    /// Current local checksum.
    local_checksum: u32,
}

impl ClientAutomationSync {
    /// Create a new client sync state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with specific acknowledged revision.
    #[must_use]
    pub fn at_revision(revision: Revision, tick: u64) -> Self {
        Self {
            last_ack_revision: revision,
            last_ack_tick: tick,
            ..Self::default()
        }
    }

    /// Get the last acknowledged revision.
    #[must_use]
    pub fn last_ack_revision(&self) -> Revision {
        self.last_ack_revision
    }

    /// Get the last acknowledged tick.
    #[must_use]
    pub fn last_ack_tick(&self) -> u64 {
        self.last_ack_tick
    }

    /// Set the interest bounds for filtering.
    pub fn set_interest_bounds(&mut self, min: ChunkPos, max: ChunkPos) {
        self.interest_filter = SpatialFilter::region(min, max);
    }

    /// Clear interest bounds (receive all).
    pub fn clear_interest_bounds(&mut self) {
        self.interest_filter = SpatialFilter::all();
    }

    /// Get the current interest filter.
    #[must_use]
    pub fn interest_filter(&self) -> &SpatialFilter {
        &self.interest_filter
    }

    /// Receive a delta batch from server.
    pub fn receive_delta(&mut self, delta: AutomationDeltaBatch) {
        if delta.from_revision >= self.last_ack_revision {
            self.pending_deltas.push(delta);
            self.pending_deltas
                .sort_by_key(|d| (d.from_revision, d.to_revision));
        }
    }

    /// Apply pending deltas to local state.
    ///
    /// Returns the number of batches applied.
    pub fn apply_pending(&mut self, snapshot: &mut AutomationSnapshot) -> usize {
        let mut applied = 0;

        while let Some(delta) = self.pending_deltas.first() {
            if delta.from_revision > self.last_ack_revision {
                break;
            }

            let delta = self.pending_deltas.remove(0);
            delta.apply(snapshot);

            self.last_ack_revision = delta.to_revision;
            self.last_ack_tick = delta.to_tick;
            self.local_checksum = delta.checksum;
            applied += 1;
        }

        applied
    }

    /// Apply a full snapshot for late-join.
    pub fn apply_snapshot(
        &mut self,
        snapshot: &mut AutomationSnapshot,
        incoming: &AutomationSnapshot,
    ) {
        let filtered = self.interest_filter.filter_snapshot(incoming);

        *snapshot = filtered;
        self.last_ack_revision = incoming.revision;
        self.last_ack_tick = incoming.tick;
        self.local_checksum = snapshot.compute_checksum();
        self.pending_deltas.clear();
    }

    /// Get the number of pending delta batches.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.pending_deltas.len()
    }

    /// Check if there are pending deltas.
    #[must_use]
    pub fn has_pending(&self) -> bool {
        !self.pending_deltas.is_empty()
    }

    /// Get local checksum.
    #[must_use]
    pub fn local_checksum(&self) -> u32 {
        self.local_checksum
    }

    /// Acknowledge receipt up to a revision.
    pub fn acknowledge(&mut self, revision: Revision, tick: u64) {
        if revision > self.last_ack_revision {
            self.last_ack_revision = revision;
            self.last_ack_tick = tick;
        }
    }
}

/// Server-side per-client automation sync state.
#[derive(Clone, Debug, Default)]
pub struct ServerClientAutomationState {
    /// Last revision sent to this client.
    last_sent_revision: Revision,
    /// Last tick sent.
    last_sent_tick: u64,
    /// Interest bounds for this client.
    interest_filter: SpatialFilter,
    /// Client-reported last acknowledged revision.
    last_ack_revision: Revision,
    /// Client-reported checksum for verification.
    last_client_checksum: u32,
}

impl ServerClientAutomationState {
    /// Create new server-side client state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create at a specific revision.
    #[must_use]
    pub fn at_revision(revision: Revision, tick: u64) -> Self {
        Self {
            last_sent_revision: revision,
            last_sent_tick: tick,
            last_ack_revision: revision,
            ..Self::default()
        }
    }

    /// Get the last sent revision.
    #[must_use]
    pub fn last_sent_revision(&self) -> Revision {
        self.last_sent_revision
    }

    /// Get the last acknowledged revision.
    #[must_use]
    pub fn last_ack_revision(&self) -> Revision {
        self.last_ack_revision
    }

    /// Set the client's interest bounds.
    pub fn set_interest(&mut self, min: ChunkPos, max: ChunkPos) {
        self.interest_filter = SpatialFilter::region(min, max);
    }

    /// Clear interest bounds.
    pub fn clear_interest(&mut self) {
        self.interest_filter = SpatialFilter::all();
    }

    /// Get the interest filter.
    #[must_use]
    pub fn interest_filter(&self) -> &SpatialFilter {
        &self.interest_filter
    }

    /// Record that we sent state up to a revision.
    pub fn mark_sent(&mut self, revision: Revision, tick: u64) {
        self.last_sent_revision = revision;
        self.last_sent_tick = tick;
    }

    /// Record client acknowledgment.
    pub fn receive_ack(&mut self, revision: Revision, checksum: u32) {
        if revision > self.last_ack_revision {
            self.last_ack_revision = revision;
            self.last_client_checksum = checksum;
        }
    }

    /// Check if client needs sync (ack revision < current).
    #[must_use]
    pub fn needs_sync(&self, current_revision: Revision) -> bool {
        self.last_ack_revision < current_revision
    }

    /// Get revision range for delta query.
    #[must_use]
    pub fn pending_revision_range(&self) -> (Revision, Revision) {
        (self.last_ack_revision, self.last_sent_revision)
    }

    /// Verify client checksum against expected.
    #[must_use]
    pub fn verify_checksum(&self, expected: u32) -> bool {
        self.last_client_checksum == expected
    }
}

/// Server-side automation synchronization manager.
///
/// Tracks per-client state and generates delta batches for transmission.
#[derive(Clone, Debug, Default)]
pub struct ServerAutomationSync {
    /// Per-client sync state.
    clients: BTreeMap<u64, ServerClientAutomationState>,
    /// Maximum deltas per batch.
    max_batch_size: usize,
}

impl ServerAutomationSync {
    /// Create a new server sync manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            clients: BTreeMap::new(),
            max_batch_size: 64,
        }
    }

    /// Set maximum changes per batch.
    pub fn set_max_batch_size(&mut self, size: usize) {
        self.max_batch_size = size;
    }

    /// Register a new client at a specific revision.
    pub fn register_client(&mut self, client_id: u64, at_revision: Revision, at_tick: u64) {
        self.clients.insert(
            client_id,
            ServerClientAutomationState::at_revision(at_revision, at_tick),
        );
    }

    /// Remove a client.
    pub fn remove_client(&mut self, client_id: u64) -> bool {
        self.clients.remove(&client_id).is_some()
    }

    /// Get mutable client state.
    pub fn client_mut(&mut self, client_id: u64) -> Option<&mut ServerClientAutomationState> {
        self.clients.get_mut(&client_id)
    }

    /// Get client state.
    #[must_use]
    pub fn client(&self, client_id: u64) -> Option<&ServerClientAutomationState> {
        self.clients.get(&client_id)
    }

    /// Set client interest bounds.
    pub fn set_client_interest(&mut self, client_id: u64, min: ChunkPos, max: ChunkPos) {
        if let Some(state) = self.clients.get_mut(&client_id) {
            state.set_interest(min, max);
        }
    }

    /// Record client acknowledgment.
    pub fn receive_ack(&mut self, client_id: u64, revision: Revision, checksum: u32) {
        if let Some(state) = self.clients.get_mut(&client_id) {
            state.receive_ack(revision, checksum);
        }
    }

    /// Prepare a delta batch for a client.
    ///
    /// Returns None if the client is up to date.
    pub fn prepare_delta_for_client(
        &mut self,
        client_id: u64,
        network: &engine_world::AutomationNetwork,
    ) -> Option<AutomationDeltaBatch> {
        let state = self.clients.get(&client_id)?;

        if !state.needs_sync(network.revision()) {
            return None;
        }

        let since = state.last_ack_revision;
        let delta = network.delta_since(since);

        if delta.is_empty() {
            return None;
        }

        if let Some(state) = self.clients.get_mut(&client_id) {
            state.mark_sent(delta.to_revision, delta.to_tick);
        }

        Some(delta)
    }

    /// Prepare a filtered snapshot for late-join.
    pub fn prepare_snapshot_for_client(
        &mut self,
        client_id: u64,
        network: &engine_world::AutomationNetwork,
    ) -> Option<AutomationSnapshot> {
        let state = self.clients.get(&client_id)?;

        let snapshot = network.snapshot_filtered(state.interest_filter());

        if let Some(state) = self.clients.get_mut(&client_id) {
            state.mark_sent(snapshot.revision, snapshot.tick);
        }

        Some(snapshot)
    }

    /// Get clients that need sync.
    pub fn clients_needing_sync(&self, current_revision: Revision) -> Vec<u64> {
        self.clients
            .iter()
            .filter(|(_, state)| state.needs_sync(current_revision))
            .map(|(&id, _)| id)
            .collect()
    }

    /// Number of registered clients.
    #[must_use]
    pub fn client_count(&self) -> usize {
        self.clients.len()
    }
}

/// Compact automation sync message for network transmission.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AutomationSyncMessage {
    /// Full snapshot for late-join.
    Snapshot(AutomationSnapshot),
    /// Incremental delta batch.
    Delta(AutomationDeltaBatch),
    /// Client acknowledgment of received revision.
    Ack { revision: Revision, checksum: u32 },
    /// Request snapshot from server.
    RequestSnapshot,
    /// Set interest bounds.
    SetInterest { min: ChunkPos, max: ChunkPos },
    /// Checksum mismatch detected - request resync.
    ChecksumMismatch {
        client_revision: Revision,
        client_checksum: u32,
        expected_checksum: u32,
    },
}

impl AutomationSyncMessage {
    /// Create a delta message.
    #[must_use]
    pub fn delta(batch: AutomationDeltaBatch) -> Self {
        Self::Delta(batch)
    }

    /// Create an acknowledgment message.
    #[must_use]
    pub fn ack(revision: Revision, checksum: u32) -> Self {
        Self::Ack { revision, checksum }
    }

    /// Check if this is a snapshot message.
    #[must_use]
    pub fn is_snapshot(&self) -> bool {
        matches!(self, Self::Snapshot(_))
    }

    /// Check if this is a delta message.
    #[must_use]
    pub fn is_delta(&self) -> bool {
        matches!(self, Self::Delta(_))
    }

    /// Check if this is an acknowledgment.
    #[must_use]
    pub fn is_ack(&self) -> bool {
        matches!(self, Self::Ack { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::coords::WorldPos;
    use engine_world::{
        AutomationDevice, AutomationLink, DeviceId, DeviceKind, LinkId, PortId, SignalValue,
    };

    fn make_test_network() -> engine_world::AutomationNetwork {
        let mut network = engine_world::AutomationNetwork::with_revision_tracking(100);

        let d1 = DeviceId::new(1);
        let d2 = DeviceId::new(2);

        network.add_device(AutomationDevice::new(
            d1,
            DeviceKind::Source,
            WorldPos::new(0, 0, 0),
        ));
        network.add_device(AutomationDevice::new(
            d2,
            DeviceKind::Sink,
            WorldPos::new(16, 0, 0),
        ));

        network.add_link(AutomationLink::simple(LinkId::new(1), d1, d2));

        network
    }

    #[test]
    fn client_sync_basic() {
        let sync = ClientAutomationSync::at_revision(Revision::new(100), 1000);
        assert_eq!(sync.last_ack_revision(), Revision::new(100));
        assert!(!sync.has_pending());
    }

    #[test]
    fn client_sync_receive_delta() {
        let mut sync = ClientAutomationSync::at_revision(Revision::new(1), 100);

        let batch = AutomationDeltaBatch::new(Revision::new(1), Revision::new(2));
        sync.receive_delta(batch);

        assert!(sync.has_pending());
        assert_eq!(sync.pending_count(), 1);
    }

    #[test]
    fn client_sync_apply_pending() {
        let mut sync = ClientAutomationSync::at_revision(Revision::new(1), 100);
        let mut snapshot = AutomationSnapshot::at(Revision::new(1), 100);

        let mut batch = AutomationDeltaBatch::new(Revision::new(1), Revision::new(2));
        batch.to_tick = 101;

        sync.receive_delta(batch);

        let applied = sync.apply_pending(&mut snapshot);
        assert_eq!(applied, 1);
        assert_eq!(sync.last_ack_revision(), Revision::new(2));
        assert!(!sync.has_pending());
    }

    #[test]
    fn server_client_state() {
        let mut state = ServerClientAutomationState::at_revision(Revision::new(100), 1000);

        assert_eq!(state.last_sent_revision(), Revision::new(100));
        assert_eq!(state.last_ack_revision(), Revision::new(100));
        assert!(!state.needs_sync(Revision::new(100)));
        assert!(state.needs_sync(Revision::new(101)));

        state.mark_sent(Revision::new(105), 1050);
        state.receive_ack(Revision::new(102), 0xABCD);

        assert_eq!(state.last_sent_revision(), Revision::new(105));
        assert_eq!(state.last_ack_revision(), Revision::new(102));
    }

    #[test]
    fn server_sync_register_clients() {
        let mut sync = ServerAutomationSync::new();

        sync.register_client(1, Revision::new(100), 1000);
        sync.register_client(2, Revision::new(100), 1000);

        assert_eq!(sync.client_count(), 2);
        assert!(sync.client(1).is_some());
        assert!(sync.remove_client(1));
        assert_eq!(sync.client_count(), 1);
    }

    #[test]
    fn server_sync_prepare_delta() {
        let mut sync = ServerAutomationSync::new();
        let mut network = make_test_network();

        let start_rev = network.revision();
        sync.register_client(1, start_rev, network.current_tick());

        network.set_output(
            DeviceId::new(1),
            PortId::OUTPUT_0,
            SignalValue::Boolean(true),
        );
        let config = engine_world::AutomationConfig::default();
        network.step(&config);

        let delta = sync.prepare_delta_for_client(1, &network);
        assert!(delta.is_some());
    }

    #[test]
    fn server_sync_clients_needing_sync() {
        let mut sync = ServerAutomationSync::new();

        sync.register_client(1, Revision::new(100), 1000);
        sync.register_client(2, Revision::new(100), 1000);

        sync.receive_ack(1, Revision::new(100), 0);

        let needing = sync.clients_needing_sync(Revision::new(101));
        assert_eq!(needing.len(), 2);
    }

    #[test]
    fn sync_message_variants() {
        let snapshot_msg = AutomationSyncMessage::Snapshot(AutomationSnapshot::empty());
        assert!(snapshot_msg.is_snapshot());
        assert!(!snapshot_msg.is_delta());

        let delta_msg = AutomationSyncMessage::delta(AutomationDeltaBatch::new(
            Revision::new(1),
            Revision::new(2),
        ));
        assert!(delta_msg.is_delta());
        assert!(!delta_msg.is_snapshot());

        let ack_msg = AutomationSyncMessage::ack(Revision::new(100), 0xDEAD);
        assert!(ack_msg.is_ack());
    }

    #[test]
    fn serde_roundtrip_sync_message() {
        let mut snapshot = AutomationSnapshot::at(Revision::new(1), 100);
        snapshot.add_device(AutomationDevice::new(
            DeviceId::new(1),
            DeviceKind::Relay,
            WorldPos::new(0, 0, 0),
        ));

        let msg = AutomationSyncMessage::Snapshot(snapshot);
        let serialized = bincode::serialize(&msg).unwrap();
        let deserialized: AutomationSyncMessage = bincode::deserialize(&serialized).unwrap();

        assert!(deserialized.is_snapshot());
    }

    #[test]
    fn serde_roundtrip_delta_message() {
        let batch = AutomationDeltaBatch::new(Revision::new(1), Revision::new(5));
        let msg = AutomationSyncMessage::delta(batch);

        let serialized = bincode::serialize(&msg).unwrap();
        let deserialized: AutomationSyncMessage = bincode::deserialize(&serialized).unwrap();

        assert!(deserialized.is_delta());
    }
}
