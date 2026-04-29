//! Snapshots and deltas for automation state replication.

use std::collections::BTreeMap;

use engine_core::coords::ChunkPos;
use serde::{Deserialize, Serialize};

use super::device::{AutomationDevice, DeviceId, PortState};
use super::link::{AutomationLink, LinkId, PendingSignal};
use super::revision::{Revision, StateChange};
use super::signal::{PortId, SignalValue};

/// A compact delta for a single device state change.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeviceDelta {
    /// Device ID.
    pub device_id: DeviceId,
    /// Port changes (`port_id` -> new value).
    pub port_changes: Vec<(PortId, SignalValue)>,
    /// Enabled state change (if any).
    pub enabled_change: Option<bool>,
    /// Timer counter change (if any).
    pub timer_counter: Option<u32>,
}

impl DeviceDelta {
    /// Create a new empty delta.
    #[must_use]
    pub const fn new(device_id: DeviceId) -> Self {
        Self {
            device_id,
            port_changes: Vec::new(),
            enabled_change: None,
            timer_counter: None,
        }
    }

    /// Add a port change.
    pub fn add_port_change(&mut self, port: PortId, value: SignalValue) {
        self.port_changes.push((port, value));
    }

    /// Set enabled change.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled_change = Some(enabled);
    }

    /// Check if this delta is empty (no changes).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.port_changes.is_empty()
            && self.enabled_change.is_none()
            && self.timer_counter.is_none()
    }

    /// Apply this delta to a device.
    pub fn apply(&self, device: &mut AutomationDevice, tick: u64) {
        for &(port, value) in &self.port_changes {
            device.ports[port.index()] = PortState::at_tick(value, tick);
        }
        if let Some(enabled) = self.enabled_change {
            device.enabled = enabled;
        }
        if let Some(counter) = self.timer_counter {
            device.timer_counter = counter;
        }
    }
}

/// A complete snapshot of automation network state.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AutomationSnapshot {
    /// Revision at snapshot time.
    pub revision: Revision,
    /// Tick at snapshot time.
    pub tick: u64,
    /// All devices keyed by ID.
    pub devices: BTreeMap<DeviceId, AutomationDevice>,
    /// All links keyed by ID.
    pub links: BTreeMap<LinkId, AutomationLink>,
    /// Pending signals in transit.
    pub pending_signals: Vec<PendingSignal>,
    /// Checksum of snapshot state.
    pub checksum: u32,
}

impl AutomationSnapshot {
    /// Create an empty snapshot at revision zero.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Create an empty snapshot at a specific revision and tick.
    #[must_use]
    pub fn at(revision: Revision, tick: u64) -> Self {
        Self {
            revision,
            tick,
            ..Self::default()
        }
    }

    /// Number of devices in snapshot.
    #[must_use]
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    /// Number of links in snapshot.
    #[must_use]
    pub fn link_count(&self) -> usize {
        self.links.len()
    }

    /// Add a device to the snapshot.
    pub fn add_device(&mut self, device: AutomationDevice) {
        self.devices.insert(device.id, device);
    }

    /// Add a link to the snapshot.
    pub fn add_link(&mut self, link: AutomationLink) {
        self.links.insert(link.id, link);
    }

    /// Remove a device from the snapshot.
    pub fn remove_device(&mut self, id: DeviceId) -> Option<AutomationDevice> {
        self.devices.remove(&id)
    }

    /// Remove a link from the snapshot.
    pub fn remove_link(&mut self, id: LinkId) -> Option<AutomationLink> {
        self.links.remove(&id)
    }

    /// Get a device by ID.
    #[must_use]
    pub fn device(&self, id: DeviceId) -> Option<&AutomationDevice> {
        self.devices.get(&id)
    }

    /// Get a mutable device by ID.
    pub fn device_mut(&mut self, id: DeviceId) -> Option<&mut AutomationDevice> {
        self.devices.get_mut(&id)
    }

    /// Get a link by ID.
    #[must_use]
    pub fn link(&self, id: LinkId) -> Option<&AutomationLink> {
        self.links.get(&id)
    }

    /// Compute the checksum of current state.
    #[must_use]
    pub fn compute_checksum(&self) -> u32 {
        let mut builder = crate::ChecksumBuilder::new();

        builder.feed_u64(self.revision.value());
        builder.feed_u64(self.tick);

        for (id, device) in &self.devices {
            builder.feed_u64(id.0);
            builder.feed_i32(device.position.x());
            builder.feed_i32(device.position.y());
            builder.feed_i32(device.position.z());
            builder.feed_u32(device.kind as u32);
            builder.feed_u32(u32::from(device.enabled));

            for port in &device.ports {
                port.value.feed_checksum(&mut builder);
            }
        }

        for (id, link) in &self.links {
            builder.feed_u64(id.0);
            builder.feed_u64(link.source_device.0);
            builder.feed_u32(u32::from(link.source_port.0));
            builder.feed_u64(link.target_device.0);
            builder.feed_u32(u32::from(link.target_port.0));
            builder.feed_u32(u32::from(link.enabled));
        }

        builder.build().value()
    }

    /// Update the stored checksum.
    pub fn update_checksum(&mut self) {
        self.checksum = self.compute_checksum();
    }

    /// Verify the stored checksum matches computed.
    #[must_use]
    pub fn verify_checksum(&self) -> bool {
        self.checksum == self.compute_checksum()
    }
}

/// A batch of deltas for incremental sync.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AutomationDeltaBatch {
    /// Starting revision (exclusive).
    pub from_revision: Revision,
    /// Ending revision (inclusive).
    pub to_revision: Revision,
    /// Tick range.
    pub from_tick: u64,
    pub to_tick: u64,
    /// Device deltas.
    pub device_deltas: Vec<DeviceDelta>,
    /// Devices added.
    pub devices_added: Vec<AutomationDevice>,
    /// Devices removed.
    pub devices_removed: Vec<DeviceId>,
    /// Links added.
    pub links_added: Vec<AutomationLink>,
    /// Links removed.
    pub links_removed: Vec<LinkId>,
    /// Pending signals added.
    pub signals_added: Vec<PendingSignal>,
    /// Checksum after applying deltas.
    pub checksum: u32,
}

impl AutomationDeltaBatch {
    /// Create an empty delta batch.
    #[must_use]
    pub fn new(from_revision: Revision, to_revision: Revision) -> Self {
        Self {
            from_revision,
            to_revision,
            ..Self::default()
        }
    }

    /// Check if this batch is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.device_deltas.is_empty()
            && self.devices_added.is_empty()
            && self.devices_removed.is_empty()
            && self.links_added.is_empty()
            && self.links_removed.is_empty()
            && self.signals_added.is_empty()
    }

    /// Apply this batch to a snapshot.
    pub fn apply(&self, snapshot: &mut AutomationSnapshot) {
        for id in &self.devices_removed {
            snapshot.devices.remove(id);
        }

        for id in &self.links_removed {
            snapshot.links.remove(id);
        }

        for device in &self.devices_added {
            snapshot.devices.insert(device.id, device.clone());
        }

        for link in &self.links_added {
            snapshot.links.insert(link.id, *link);
        }

        for delta in &self.device_deltas {
            if let Some(device) = snapshot.devices.get_mut(&delta.device_id) {
                delta.apply(device, self.to_tick);
            }
        }

        snapshot
            .pending_signals
            .extend(self.signals_added.iter().copied());
        snapshot.pending_signals.sort();

        snapshot.revision = self.to_revision;
        snapshot.tick = self.to_tick;
    }

    /// Build a delta batch from state changes.
    #[must_use]
    pub fn from_changes(changes: &[StateChange], from_tick: u64, to_tick: u64) -> Self {
        let mut batch = Self::default();

        if let Some(first) = changes.first() {
            batch.from_revision = Revision::new(first.revision.value().saturating_sub(1));
        }
        if let Some(last) = changes.last() {
            batch.to_revision = last.revision;
        }

        batch.from_tick = from_tick;
        batch.to_tick = to_tick;

        for change in changes {
            use super::revision::{
                ChangeKind, ChangePayload, DeviceChangePayload, LinkChangePayload,
            };

            match (&change.kind, &change.payload) {
                (ChangeKind::DeviceAdded, ChangePayload::Device(DeviceChangePayload::Full(d))) => {
                    batch.devices_added.push(d.as_ref().clone());
                }
                (ChangeKind::DeviceRemoved, _) => {
                    if let Some(id) = change.device_id {
                        batch.devices_removed.push(id);
                    }
                }
                (
                    ChangeKind::DeviceChanged,
                    ChangePayload::Device(DeviceChangePayload::PortChange {
                        port, new_value, ..
                    }),
                ) => {
                    if let Some(device_id) = change.device_id {
                        let delta = batch
                            .device_deltas
                            .iter_mut()
                            .find(|d| d.device_id == device_id);

                        if let Some(delta) = delta {
                            delta.add_port_change(*port, *new_value);
                        } else {
                            let mut delta = DeviceDelta::new(device_id);
                            delta.add_port_change(*port, *new_value);
                            batch.device_deltas.push(delta);
                        }
                    }
                }
                (ChangeKind::LinkAdded, ChangePayload::Link(LinkChangePayload::Full(l))) => {
                    batch.links_added.push(*l);
                }
                (ChangeKind::LinkRemoved, _) => {
                    if let Some(id) = change.link_id {
                        batch.links_removed.push(id);
                    }
                }
                _ => {}
            }
        }

        batch
    }
}

/// Spatial index for filtering snapshots by region.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SpatialFilter {
    /// Minimum chunk position (inclusive).
    pub min: Option<ChunkPos>,
    /// Maximum chunk position (inclusive).
    pub max: Option<ChunkPos>,
}

impl SpatialFilter {
    /// Create a filter that matches everything.
    #[must_use]
    pub const fn all() -> Self {
        Self {
            min: None,
            max: None,
        }
    }

    /// Create a filter for a specific region.
    #[must_use]
    pub const fn region(min: ChunkPos, max: ChunkPos) -> Self {
        Self {
            min: Some(min),
            max: Some(max),
        }
    }

    /// Check if a world position is within the filter.
    #[must_use]
    pub fn contains_world_pos(&self, pos: engine_core::coords::WorldPos) -> bool {
        let chunk = pos.to_chunk_pos();
        self.contains_chunk(chunk)
    }

    /// Check if a chunk position is within the filter.
    #[must_use]
    pub fn contains_chunk(&self, pos: ChunkPos) -> bool {
        match (self.min, self.max) {
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

    /// Filter a snapshot to only include devices/links in region.
    #[must_use]
    pub fn filter_snapshot(&self, snapshot: &AutomationSnapshot) -> AutomationSnapshot {
        let mut filtered = AutomationSnapshot::at(snapshot.revision, snapshot.tick);

        for device in snapshot.devices.values() {
            if self.contains_world_pos(device.position) {
                filtered.add_device(device.clone());
            }
        }

        let device_ids: std::collections::HashSet<_> = filtered.devices.keys().copied().collect();

        for link in snapshot.links.values() {
            if device_ids.contains(&link.source_device) && device_ids.contains(&link.target_device)
            {
                filtered.add_link(*link);
            }
        }

        filtered.pending_signals = snapshot
            .pending_signals
            .iter()
            .filter(|s| filtered.links.contains_key(&s.link_id))
            .copied()
            .collect();

        filtered.update_checksum();
        filtered
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automation::device::DeviceKind;
    use engine_core::coords::WorldPos;

    fn make_device(id: u64, x: i32) -> AutomationDevice {
        AutomationDevice::new(DeviceId::new(id), DeviceKind::Relay, WorldPos::new(x, 0, 0))
    }

    #[test]
    fn device_delta_apply() {
        let mut device = make_device(1, 0);
        let mut delta = DeviceDelta::new(DeviceId::new(1));
        delta.add_port_change(PortId::OUTPUT_0, SignalValue::Boolean(true));
        delta.set_enabled(false);

        delta.apply(&mut device, 100);

        assert_eq!(device.port(PortId::OUTPUT_0), SignalValue::Boolean(true));
        assert!(!device.enabled);
    }

    #[test]
    fn snapshot_operations() {
        let mut snapshot = AutomationSnapshot::at(Revision::new(1), 100);

        snapshot.add_device(make_device(1, 0));
        snapshot.add_device(make_device(2, 16));

        assert_eq!(snapshot.device_count(), 2);
        assert!(snapshot.device(DeviceId::new(1)).is_some());

        let removed = snapshot.remove_device(DeviceId::new(1));
        assert!(removed.is_some());
        assert_eq!(snapshot.device_count(), 1);
    }

    #[test]
    fn snapshot_checksum_deterministic() {
        let mut s1 = AutomationSnapshot::at(Revision::new(1), 100);
        let mut s2 = AutomationSnapshot::at(Revision::new(1), 100);

        s1.add_device(make_device(1, 0));
        s1.add_device(make_device(2, 16));

        s2.add_device(make_device(1, 0));
        s2.add_device(make_device(2, 16));

        assert_eq!(s1.compute_checksum(), s2.compute_checksum());
    }

    #[test]
    fn snapshot_checksum_differs() {
        let mut s1 = AutomationSnapshot::at(Revision::new(1), 100);
        let mut s2 = AutomationSnapshot::at(Revision::new(1), 100);

        s1.add_device(make_device(1, 0));
        s2.add_device(make_device(1, 16));

        assert_ne!(s1.compute_checksum(), s2.compute_checksum());
    }

    #[test]
    fn delta_batch_apply() {
        let mut snapshot = AutomationSnapshot::at(Revision::new(1), 100);
        snapshot.add_device(make_device(1, 0));

        let link = AutomationLink::simple(LinkId::new(1), DeviceId::new(1), DeviceId::new(2));

        let mut batch = AutomationDeltaBatch::new(Revision::new(1), Revision::new(3));
        batch.to_tick = 102;
        batch.devices_added.push(make_device(2, 16));
        batch.links_added.push(link);

        let mut delta = DeviceDelta::new(DeviceId::new(1));
        delta.set_enabled(false);
        batch.device_deltas.push(delta);

        batch.apply(&mut snapshot);

        assert_eq!(snapshot.device_count(), 2);
        assert_eq!(snapshot.link_count(), 1);
        assert!(!snapshot.device(DeviceId::new(1)).unwrap().enabled);
        assert_eq!(snapshot.revision, Revision::new(3));
    }

    #[test]
    fn spatial_filter() {
        let filter = SpatialFilter::region(ChunkPos::new(0, 0, 0), ChunkPos::new(1, 1, 1));

        assert!(filter.contains_chunk(ChunkPos::new(0, 0, 0)));
        assert!(filter.contains_chunk(ChunkPos::new(1, 1, 1)));
        assert!(!filter.contains_chunk(ChunkPos::new(2, 0, 0)));
        assert!(!filter.contains_chunk(ChunkPos::new(-1, 0, 0)));
    }

    #[test]
    fn spatial_filter_snapshot() {
        let mut snapshot = AutomationSnapshot::at(Revision::new(1), 100);
        snapshot.add_device(make_device(1, 0));
        snapshot.add_device(make_device(2, 100));

        let filter = SpatialFilter::region(ChunkPos::new(0, 0, 0), ChunkPos::new(0, 0, 0));
        let filtered = filter.filter_snapshot(&snapshot);

        assert_eq!(filtered.device_count(), 1);
        assert!(filtered.device(DeviceId::new(1)).is_some());
        assert!(filtered.device(DeviceId::new(2)).is_none());
    }

    #[test]
    fn serde_roundtrip_snapshot() {
        let mut snapshot = AutomationSnapshot::at(Revision::new(1), 100);
        snapshot.add_device(make_device(1, 0));
        snapshot.add_link(AutomationLink::simple(
            LinkId::new(1),
            DeviceId::new(1),
            DeviceId::new(2),
        ));
        snapshot.update_checksum();

        let json = serde_json::to_string(&snapshot).unwrap();
        let recovered: AutomationSnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(snapshot.revision, recovered.revision);
        assert_eq!(snapshot.checksum, recovered.checksum);
    }

    #[test]
    fn serde_roundtrip_batch() {
        let mut batch = AutomationDeltaBatch::new(Revision::new(1), Revision::new(2));
        batch.devices_added.push(make_device(1, 0));

        let json = serde_json::to_string(&batch).unwrap();
        let recovered: AutomationDeltaBatch = serde_json::from_str(&json).unwrap();

        assert_eq!(batch.from_revision, recovered.from_revision);
        assert_eq!(batch.devices_added.len(), recovered.devices_added.len());
    }
}
