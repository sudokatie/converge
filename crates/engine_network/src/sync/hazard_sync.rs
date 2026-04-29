//! Hazard state synchronization for clients.
//!
//! Provides compact delta transmission and late-join synchronization for
//! hazard simulation state using server-authoritative deltas.

use std::collections::{BTreeMap, HashMap};

use engine_core::coords::ChunkPos;
use engine_world::{ChunkHazards, HazardDeltaRecord, HazardSnapshot};
use serde::{Deserialize, Serialize};

/// Client-side hazard synchronization state.
///
/// Tracks the last acknowledged tick and pending deltas to apply.
#[derive(Clone, Debug, Default)]
pub struct ClientHazardSync {
    /// Last tick fully acknowledged by client.
    last_ack_tick: u64,
    /// Pending deltas awaiting application.
    pending_deltas: Vec<HazardDeltaRecord>,
    /// Interest bounds for filtering (if set).
    interest_min: Option<ChunkPos>,
    interest_max: Option<ChunkPos>,
}

impl ClientHazardSync {
    /// Create a new client sync state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with specific acknowledged tick.
    #[must_use]
    pub fn at_tick(tick: u64) -> Self {
        Self {
            last_ack_tick: tick,
            ..Self::default()
        }
    }

    /// Get the last acknowledged tick.
    #[must_use]
    pub fn last_ack_tick(&self) -> u64 {
        self.last_ack_tick
    }

    /// Set the interest bounds for filtering.
    pub fn set_interest_bounds(&mut self, min: ChunkPos, max: ChunkPos) {
        self.interest_min = Some(min);
        self.interest_max = Some(max);
    }

    /// Clear interest bounds (receive all chunks).
    pub fn clear_interest_bounds(&mut self) {
        self.interest_min = None;
        self.interest_max = None;
    }

    /// Get the interest bounds if set.
    #[must_use]
    pub fn interest_bounds(&self) -> Option<(ChunkPos, ChunkPos)> {
        match (self.interest_min, self.interest_max) {
            (Some(min), Some(max)) => Some((min, max)),
            _ => None,
        }
    }

    /// Check if a chunk position is within interest bounds.
    #[must_use]
    pub fn is_in_interest(&self, pos: ChunkPos) -> bool {
        match (self.interest_min, self.interest_max) {
            (Some(min), Some(max)) => {
                pos.x() >= min.x()
                    && pos.x() <= max.x()
                    && pos.y() >= min.y()
                    && pos.y() <= max.y()
                    && pos.z() >= min.z()
                    && pos.z() <= max.z()
            }
            _ => true,
        }
    }

    /// Receive a delta batch from server.
    pub fn receive_deltas(&mut self, deltas: Vec<HazardDeltaRecord>) {
        for delta in deltas {
            if self.is_in_interest(delta.chunk_pos) {
                self.pending_deltas.push(delta);
            }
        }
        self.pending_deltas.sort();
    }

    /// Apply pending deltas to local hazard state.
    ///
    /// Returns the number of chunks modified.
    pub fn apply_pending(&mut self, chunks: &mut HashMap<ChunkPos, ChunkHazards>) -> usize {
        let deltas = std::mem::take(&mut self.pending_deltas);
        let mut modified_chunks = std::collections::HashSet::new();

        for record in deltas {
            let hazards = chunks.entry(record.chunk_pos).or_default();

            for (kind, cell_deltas) in record.delta.iter() {
                for cell_delta in cell_deltas {
                    match cell_delta.intensity {
                        Some(intensity) => {
                            hazards.activate(kind, cell_delta.local_pos(), intensity);
                        }
                        None => {
                            hazards.deactivate(kind, cell_delta.local_pos());
                        }
                    }
                }
            }

            modified_chunks.insert(record.chunk_pos);

            if record.tick > self.last_ack_tick {
                self.last_ack_tick = record.tick;
            }
        }

        modified_chunks.len()
    }

    /// Apply a full snapshot for late-join.
    pub fn apply_snapshot(
        &mut self,
        chunks: &mut HashMap<ChunkPos, ChunkHazards>,
        snapshot: &HazardSnapshot,
    ) {
        self.last_ack_tick = snapshot.base_tick;

        for (&pos, chunk_snapshot) in &snapshot.chunk_states {
            if !self.is_in_interest(pos) {
                continue;
            }

            let hazards = chunks.entry(pos).or_default();

            for (kind, cells) in chunk_snapshot.iter() {
                for &(index, intensity) in cells {
                    hazards.activate(kind, index.to_local_pos(), intensity);
                }
            }
        }
    }

    /// Get the number of pending deltas.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.pending_deltas.len()
    }

    /// Check if there are pending deltas.
    #[must_use]
    pub fn has_pending(&self) -> bool {
        !self.pending_deltas.is_empty()
    }

    /// Acknowledge receipt up to a tick.
    pub fn acknowledge(&mut self, tick: u64) {
        self.last_ack_tick = self.last_ack_tick.max(tick);
    }
}

/// Server-side per-client hazard sync state.
#[derive(Clone, Debug, Default)]
pub struct ServerClientHazardState {
    /// Last tick sent to this client.
    last_sent_tick: u64,
    /// Interest bounds for this client.
    interest_min: Option<ChunkPos>,
    interest_max: Option<ChunkPos>,
    /// Client-reported last acknowledged tick.
    last_ack_tick: u64,
}

impl ServerClientHazardState {
    /// Create new server-side client state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create at a specific tick.
    #[must_use]
    pub fn at_tick(tick: u64) -> Self {
        Self {
            last_sent_tick: tick,
            last_ack_tick: tick,
            ..Self::default()
        }
    }

    /// Get the last sent tick.
    #[must_use]
    pub fn last_sent_tick(&self) -> u64 {
        self.last_sent_tick
    }

    /// Get the last acknowledged tick.
    #[must_use]
    pub fn last_ack_tick(&self) -> u64 {
        self.last_ack_tick
    }

    /// Set the client's interest bounds.
    pub fn set_interest(&mut self, min: ChunkPos, max: ChunkPos) {
        self.interest_min = Some(min);
        self.interest_max = Some(max);
    }

    /// Clear interest bounds.
    pub fn clear_interest(&mut self) {
        self.interest_min = None;
        self.interest_max = None;
    }

    /// Get the interest bounds.
    #[must_use]
    pub fn interest_bounds(&self) -> Option<(ChunkPos, ChunkPos)> {
        match (self.interest_min, self.interest_max) {
            (Some(min), Some(max)) => Some((min, max)),
            _ => None,
        }
    }

    /// Record that we sent deltas up to a tick.
    pub fn mark_sent(&mut self, tick: u64) {
        self.last_sent_tick = tick;
    }

    /// Record client acknowledgment.
    pub fn receive_ack(&mut self, tick: u64) {
        self.last_ack_tick = self.last_ack_tick.max(tick);
    }

    /// Check if client needs deltas (ack tick < sent tick).
    #[must_use]
    pub fn needs_sync(&self, current_tick: u64) -> bool {
        self.last_ack_tick < current_tick
    }

    /// Get tick range for delta query.
    #[must_use]
    pub fn pending_tick_range(&self) -> (u64, u64) {
        (self.last_ack_tick, self.last_sent_tick)
    }
}

/// Server-side hazard synchronization manager.
///
/// Tracks per-client state and generates delta batches for transmission.
#[derive(Clone, Debug, Default)]
pub struct ServerHazardSync {
    /// Per-client sync state.
    clients: BTreeMap<u64, ServerClientHazardState>,
    /// Maximum deltas per batch.
    max_batch_size: usize,
}

impl ServerHazardSync {
    /// Create a new server sync manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            clients: BTreeMap::new(),
            max_batch_size: 64,
        }
    }

    /// Set maximum deltas per batch.
    pub fn set_max_batch_size(&mut self, size: usize) {
        self.max_batch_size = size;
    }

    /// Register a new client at a specific tick.
    pub fn register_client(&mut self, client_id: u64, at_tick: u64) {
        self.clients
            .insert(client_id, ServerClientHazardState::at_tick(at_tick));
    }

    /// Remove a client.
    pub fn remove_client(&mut self, client_id: u64) -> bool {
        self.clients.remove(&client_id).is_some()
    }

    /// Get mutable client state.
    pub fn client_mut(&mut self, client_id: u64) -> Option<&mut ServerClientHazardState> {
        self.clients.get_mut(&client_id)
    }

    /// Get client state.
    #[must_use]
    pub fn client(&self, client_id: u64) -> Option<&ServerClientHazardState> {
        self.clients.get(&client_id)
    }

    /// Set client interest bounds.
    pub fn set_client_interest(&mut self, client_id: u64, min: ChunkPos, max: ChunkPos) {
        if let Some(state) = self.clients.get_mut(&client_id) {
            state.set_interest(min, max);
        }
    }

    /// Record client acknowledgment.
    pub fn receive_ack(&mut self, client_id: u64, tick: u64) {
        if let Some(state) = self.clients.get_mut(&client_id) {
            state.receive_ack(tick);
        }
    }

    /// Collect deltas to send to a client.
    ///
    /// Returns records filtered by client interest and since-tick.
    pub fn collect_for_client<'a>(
        &mut self,
        client_id: u64,
        current_tick: u64,
        all_records: impl Iterator<Item = &'a HazardDeltaRecord>,
    ) -> Vec<HazardDeltaRecord> {
        let Some(state) = self.clients.get(&client_id) else {
            return Vec::new();
        };

        let since_tick = state.last_ack_tick;
        let bounds = state.interest_bounds();

        let filtered: Vec<_> = all_records
            .filter(|r| {
                if r.tick <= since_tick {
                    return false;
                }
                if let Some((min, max)) = bounds {
                    r.chunk_pos.x() >= min.x()
                        && r.chunk_pos.x() <= max.x()
                        && r.chunk_pos.y() >= min.y()
                        && r.chunk_pos.y() <= max.y()
                        && r.chunk_pos.z() >= min.z()
                        && r.chunk_pos.z() <= max.z()
                } else {
                    true
                }
            })
            .take(self.max_batch_size)
            .cloned()
            .collect();

        if let Some(state) = self.clients.get_mut(&client_id) {
            state.mark_sent(current_tick);
        }

        filtered
    }

    /// Get clients that need sync.
    pub fn clients_needing_sync(&self, current_tick: u64) -> Vec<u64> {
        self.clients
            .iter()
            .filter(|(_, state)| state.needs_sync(current_tick))
            .map(|(&id, _)| id)
            .collect()
    }

    /// Number of registered clients.
    #[must_use]
    pub fn client_count(&self) -> usize {
        self.clients.len()
    }
}

/// Compact hazard sync message for network transmission.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum HazardSyncMessage {
    /// Full snapshot for late-join.
    Snapshot(HazardSnapshot),
    /// Incremental deltas.
    Deltas {
        /// Tick range covered.
        from_tick: u64,
        to_tick: u64,
        /// Records in this batch.
        records: Vec<HazardDeltaRecord>,
    },
    /// Client acknowledgment of received tick.
    Ack { tick: u64 },
    /// Request snapshot from server.
    RequestSnapshot,
    /// Set interest bounds.
    SetInterest { min: ChunkPos, max: ChunkPos },
}

impl HazardSyncMessage {
    /// Create a deltas message.
    #[must_use]
    pub fn deltas(from_tick: u64, to_tick: u64, records: Vec<HazardDeltaRecord>) -> Self {
        Self::Deltas {
            from_tick,
            to_tick,
            records,
        }
    }

    /// Create an acknowledgment message.
    #[must_use]
    pub fn ack(tick: u64) -> Self {
        Self::Ack { tick }
    }

    /// Check if this is a snapshot message.
    #[must_use]
    pub fn is_snapshot(&self) -> bool {
        matches!(self, Self::Snapshot(_))
    }

    /// Check if this is a deltas message.
    #[must_use]
    pub fn is_deltas(&self) -> bool {
        matches!(self, Self::Deltas { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::coords::LocalPos;
    use engine_world::{ChunkHazardDelta, ChunkHazardSnapshot, HazardKind};

    fn make_delta_record(tick: u64, chunk_x: i32) -> HazardDeltaRecord {
        let mut delta = ChunkHazardDelta::new();
        delta.add_set(HazardKind::Fire, LocalPos::new(0, 0, 0), 1.0);
        HazardDeltaRecord::new(tick, 0, ChunkPos::new(chunk_x, 0, 0), delta)
    }

    #[test]
    fn client_sync_basic() {
        let sync = ClientHazardSync::at_tick(100);
        assert_eq!(sync.last_ack_tick(), 100);
        assert!(!sync.has_pending());
    }

    #[test]
    fn client_sync_receive_and_apply() {
        let mut sync = ClientHazardSync::at_tick(100);
        let mut chunks = HashMap::new();

        let records = vec![make_delta_record(101, 0), make_delta_record(102, 0)];

        sync.receive_deltas(records);
        assert!(sync.has_pending());
        assert_eq!(sync.pending_count(), 2);

        let modified = sync.apply_pending(&mut chunks);
        assert_eq!(modified, 1);
        assert_eq!(sync.last_ack_tick(), 102);
        assert!(!sync.has_pending());

        let hazards = chunks.get(&ChunkPos::new(0, 0, 0)).unwrap();
        assert!(
            hazards
                .get(HazardKind::Fire, LocalPos::new(0, 0, 0))
                .is_active()
        );
    }

    #[test]
    fn client_sync_interest_filtering() {
        let mut sync = ClientHazardSync::at_tick(100);
        sync.set_interest_bounds(ChunkPos::new(-1, 0, 0), ChunkPos::new(1, 0, 0));

        assert!(sync.is_in_interest(ChunkPos::new(0, 0, 0)));
        assert!(sync.is_in_interest(ChunkPos::new(-1, 0, 0)));
        assert!(!sync.is_in_interest(ChunkPos::new(5, 0, 0)));

        let records = vec![make_delta_record(101, 0), make_delta_record(101, 5)];

        sync.receive_deltas(records);
        assert_eq!(sync.pending_count(), 1);
    }

    #[test]
    fn client_sync_apply_snapshot() {
        let mut sync = ClientHazardSync::at_tick(100);
        let mut chunks = HashMap::new();

        let mut snapshot = HazardSnapshot::empty(150);
        let mut chunk_state = ChunkHazardSnapshot::new();
        chunk_state.add(HazardKind::Fire, LocalPos::new(5, 5, 5), 0.8);
        snapshot
            .chunk_states
            .insert(ChunkPos::new(0, 0, 0), chunk_state);

        sync.apply_snapshot(&mut chunks, &snapshot);

        assert_eq!(sync.last_ack_tick(), 150);
        let hazards = chunks.get(&ChunkPos::new(0, 0, 0)).unwrap();
        assert!(
            hazards
                .get(HazardKind::Fire, LocalPos::new(5, 5, 5))
                .is_active()
        );
    }

    #[test]
    fn server_client_state() {
        let mut state = ServerClientHazardState::at_tick(100);

        assert_eq!(state.last_sent_tick(), 100);
        assert_eq!(state.last_ack_tick(), 100);
        assert!(!state.needs_sync(100));
        assert!(state.needs_sync(101));

        state.mark_sent(105);
        state.receive_ack(102);

        assert_eq!(state.last_sent_tick(), 105);
        assert_eq!(state.last_ack_tick(), 102);
    }

    #[test]
    fn server_sync_register_clients() {
        let mut sync = ServerHazardSync::new();

        sync.register_client(1, 100);
        sync.register_client(2, 100);

        assert_eq!(sync.client_count(), 2);
        assert!(sync.client(1).is_some());
        assert!(sync.remove_client(1));
        assert_eq!(sync.client_count(), 1);
    }

    #[test]
    fn server_sync_collect_for_client() {
        let mut sync = ServerHazardSync::new();
        sync.register_client(1, 100);
        sync.set_client_interest(1, ChunkPos::new(-1, 0, 0), ChunkPos::new(1, 0, 0));

        let records = [
            make_delta_record(101, 0),
            make_delta_record(101, 5),
            make_delta_record(102, 0),
        ];

        let collected = sync.collect_for_client(1, 102, records.iter());

        assert_eq!(collected.len(), 2);
        assert!(collected.iter().all(|r| r.chunk_pos.x() <= 1));
    }

    #[test]
    fn server_sync_clients_needing_sync() {
        let mut sync = ServerHazardSync::new();
        sync.register_client(1, 100);
        sync.register_client(2, 100);

        sync.receive_ack(1, 100);

        let needing = sync.clients_needing_sync(101);
        assert_eq!(needing.len(), 2);
    }

    #[test]
    fn hazard_sync_message_variants() {
        let snapshot_msg = HazardSyncMessage::Snapshot(HazardSnapshot::empty(100));
        assert!(snapshot_msg.is_snapshot());
        assert!(!snapshot_msg.is_deltas());

        let deltas_msg = HazardSyncMessage::deltas(100, 105, Vec::new());
        assert!(deltas_msg.is_deltas());
        assert!(!deltas_msg.is_snapshot());

        let ack_msg = HazardSyncMessage::ack(100);
        assert!(!ack_msg.is_snapshot());
        assert!(!ack_msg.is_deltas());
    }

    #[test]
    fn serde_roundtrip_sync_message() {
        let mut snapshot = HazardSnapshot::empty(100);
        let mut chunk_state = ChunkHazardSnapshot::new();
        chunk_state.add(HazardKind::Fire, LocalPos::new(0, 0, 0), 1.0);
        snapshot
            .chunk_states
            .insert(ChunkPos::new(0, 0, 0), chunk_state);

        let msg = HazardSyncMessage::Snapshot(snapshot);
        let serialized = bincode::serialize(&msg).unwrap();
        let deserialized: HazardSyncMessage = bincode::deserialize(&serialized).unwrap();

        assert!(deserialized.is_snapshot());
    }

    #[test]
    fn serde_roundtrip_deltas_message() {
        let records = vec![make_delta_record(101, 0), make_delta_record(102, 1)];
        let msg = HazardSyncMessage::deltas(100, 102, records);

        let serialized = bincode::serialize(&msg).unwrap();
        let deserialized: HazardSyncMessage = bincode::deserialize(&serialized).unwrap();

        if let HazardSyncMessage::Deltas {
            from_tick,
            to_tick,
            records,
        } = deserialized
        {
            assert_eq!(from_tick, 100);
            assert_eq!(to_tick, 102);
            assert_eq!(records.len(), 2);
        } else {
            panic!("Expected Deltas variant");
        }
    }
}
