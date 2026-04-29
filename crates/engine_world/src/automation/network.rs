//! Automation network management and simulation.

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};

use super::device::{AutomationDevice, DeviceId};
use super::link::{AutomationLink, LinkId, PendingSignal};
use super::revision::{Revision, RevisionTracker, StateChange};
use super::signal::{PortId, SignalValue};
use super::snapshot::{AutomationDeltaBatch, AutomationSnapshot, SpatialFilter};

/// Configuration for automation network simulation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AutomationConfig {
    /// Maximum devices per network.
    pub max_devices: usize,
    /// Maximum links per network.
    pub max_links: usize,
    /// Maximum pending signals.
    pub max_pending_signals: usize,
    /// Maximum iterations per tick (cycle detection).
    pub max_iterations_per_tick: u32,
    /// Default signal propagation delay (0 = immediate).
    pub default_delay: u8,
}

impl Default for AutomationConfig {
    fn default() -> Self {
        Self {
            max_devices: 4096,
            max_links: 8192,
            max_pending_signals: 1024,
            max_iterations_per_tick: 64,
            default_delay: 0,
        }
    }
}

/// Result of a network simulation tick.
#[derive(Clone, Debug, Default)]
pub struct TickResult {
    /// Number of devices processed.
    pub devices_processed: usize,
    /// Number of signals propagated.
    pub signals_propagated: usize,
    /// Number of iterations needed.
    pub iterations: u32,
    /// Whether a cycle was detected (hit max iterations).
    pub cycle_detected: bool,
    /// Devices with changed outputs.
    pub changed_devices: Vec<DeviceId>,
}

/// An automation network managing devices and links.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AutomationNetwork {
    /// Current simulation tick.
    tick: u64,
    /// All devices.
    devices: BTreeMap<DeviceId, AutomationDevice>,
    /// All links.
    links: BTreeMap<LinkId, AutomationLink>,
    /// Index: source device -> outgoing links.
    outgoing_links: BTreeMap<DeviceId, Vec<LinkId>>,
    /// Index: target device -> incoming links.
    incoming_links: BTreeMap<DeviceId, Vec<LinkId>>,
    /// Pending delayed signals.
    pending_signals: Vec<PendingSignal>,
    /// Next device ID.
    next_device_id: u64,
    /// Next link ID.
    next_link_id: u64,
    /// Revision tracker for change history.
    #[serde(skip)]
    revisions: RevisionTracker,
}

impl AutomationNetwork {
    /// Create a new empty network.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a network with revision tracking.
    #[must_use]
    pub fn with_revision_tracking(max_history: usize) -> Self {
        Self {
            revisions: RevisionTracker::with_max_history(max_history),
            ..Self::default()
        }
    }

    /// Get the current tick.
    #[must_use]
    pub const fn current_tick(&self) -> u64 {
        self.tick
    }

    /// Get the current revision.
    #[must_use]
    pub fn revision(&self) -> Revision {
        self.revisions.current()
    }

    /// Number of devices.
    #[must_use]
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    /// Number of links.
    #[must_use]
    pub fn link_count(&self) -> usize {
        self.links.len()
    }

    /// Allocate a new device ID.
    pub fn alloc_device_id(&mut self) -> DeviceId {
        let id = DeviceId::new(self.next_device_id);
        self.next_device_id += 1;
        id
    }

    /// Allocate a new link ID.
    pub fn alloc_link_id(&mut self) -> LinkId {
        let id = LinkId::new(self.next_link_id);
        self.next_link_id += 1;
        id
    }

    /// Add a device to the network.
    pub fn add_device(&mut self, device: AutomationDevice) -> bool {
        if self.devices.contains_key(&device.id) {
            return false;
        }

        let change = StateChange::device_added(Revision::ZERO, self.tick, device.clone());
        self.revisions.record_and_advance(change);

        self.devices.insert(device.id, device);
        true
    }

    /// Remove a device and all connected links.
    pub fn remove_device(&mut self, id: DeviceId) -> Option<AutomationDevice> {
        let device = self.devices.remove(&id)?;

        let change = StateChange::device_removed(Revision::ZERO, self.tick, device.clone());
        self.revisions.record_and_advance(change);

        let links_to_remove: Vec<_> = self
            .links
            .values()
            .filter(|l| l.source_device == id || l.target_device == id)
            .map(|l| l.id)
            .collect();

        for link_id in links_to_remove {
            self.remove_link(link_id);
        }

        self.outgoing_links.remove(&id);
        self.incoming_links.remove(&id);

        Some(device)
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

    /// Add a link to the network.
    pub fn add_link(&mut self, link: AutomationLink) -> bool {
        if !link.is_valid() {
            return false;
        }

        if !self.devices.contains_key(&link.source_device)
            || !self.devices.contains_key(&link.target_device)
        {
            return false;
        }

        if self.links.contains_key(&link.id) {
            return false;
        }

        let change = StateChange::link_added(Revision::ZERO, self.tick, link);
        self.revisions.record_and_advance(change);

        self.outgoing_links
            .entry(link.source_device)
            .or_default()
            .push(link.id);
        self.incoming_links
            .entry(link.target_device)
            .or_default()
            .push(link.id);

        self.links.insert(link.id, link);
        true
    }

    /// Remove a link.
    pub fn remove_link(&mut self, id: LinkId) -> Option<AutomationLink> {
        let link = self.links.remove(&id)?;

        let change = StateChange::link_removed(Revision::ZERO, self.tick, link);
        self.revisions.record_and_advance(change);

        if let Some(links) = self.outgoing_links.get_mut(&link.source_device) {
            links.retain(|&l| l != id);
        }
        if let Some(links) = self.incoming_links.get_mut(&link.target_device) {
            links.retain(|&l| l != id);
        }

        self.pending_signals.retain(|s| s.link_id != id);

        Some(link)
    }

    /// Get a link by ID.
    #[must_use]
    pub fn link(&self, id: LinkId) -> Option<&AutomationLink> {
        self.links.get(&id)
    }

    /// Get outgoing links from a device.
    #[must_use]
    pub fn outgoing_from(&self, device_id: DeviceId) -> &[LinkId] {
        self.outgoing_links
            .get(&device_id)
            .map_or(&[], |v| v.as_slice())
    }

    /// Get incoming links to a device.
    #[must_use]
    pub fn incoming_to(&self, device_id: DeviceId) -> &[LinkId] {
        self.incoming_links
            .get(&device_id)
            .map_or(&[], |v| v.as_slice())
    }

    /// Set a device's output and propagate signals.
    pub fn set_output(&mut self, device_id: DeviceId, port: PortId, value: SignalValue) {
        let Some(device) = self.devices.get_mut(&device_id) else {
            return;
        };

        let old_value = device.port(port);
        if old_value == value {
            return;
        }

        let change = StateChange::device_port_change(
            Revision::ZERO,
            self.tick,
            device_id,
            port,
            old_value,
            value,
        );
        self.revisions.record_and_advance(change);

        device.set_output(port, value, self.tick);
    }

    /// Run one simulation step (advances the tick counter).
    pub fn step(&mut self, config: &AutomationConfig) -> TickResult {
        self.tick += 1;
        let current_tick = self.tick;
        let mut result = TickResult::default();

        let arriving: Vec<_> = self
            .pending_signals
            .iter()
            .filter(|s| s.arrives_at(current_tick))
            .copied()
            .collect();

        self.pending_signals.retain(|s| !s.arrives_at(current_tick));

        for signal in arriving {
            if let Some(link) = self.links.get(&signal.link_id)
                && link.enabled
                && let Some(device) = self.devices.get_mut(&link.target_device)
            {
                device.set_input(link.target_port, signal.value, current_tick);
            }
            result.signals_propagated += 1;
        }

        let mut dirty: HashSet<DeviceId> = self.devices.keys().copied().collect();
        let mut changed_outputs: HashSet<DeviceId> = HashSet::new();

        while !dirty.is_empty() && result.iterations < config.max_iterations_per_tick {
            result.iterations += 1;

            let processing: Vec<_> = dirty.drain().collect();

            let mut pending_propagations: Vec<(DeviceId, PortId, SignalValue)> = Vec::new();

            for device_id in processing {
                let Some(device) = self.devices.get_mut(&device_id) else {
                    continue;
                };

                let old_output = device.output();
                let first_tick = device.last_tick == 0;
                device.recompute(current_tick);
                let new_output = device.output();

                result.devices_processed += 1;

                let output_changed =
                    old_output != new_output || (first_tick && new_output != SignalValue::None);
                if output_changed {
                    changed_outputs.insert(device_id);

                    let outgoing: Vec<LinkId> = self
                        .outgoing_links
                        .get(&device_id)
                        .map_or_else(Vec::new, Clone::clone);

                    for link_id in outgoing {
                        let Some(link) = self.links.get(&link_id) else {
                            continue;
                        };

                        if !link.enabled {
                            continue;
                        }

                        let source_port = link.source_port;
                        let target_device = link.target_device;
                        let target_port = link.target_port;
                        let delay = link.delay;

                        let port_value = self
                            .devices
                            .get(&device_id)
                            .map_or(SignalValue::None, |d| d.port(source_port));

                        if delay > 0 {
                            if self.pending_signals.len() < config.max_pending_signals {
                                self.pending_signals.push(PendingSignal::new(
                                    link_id,
                                    current_tick + u64::from(delay),
                                    port_value,
                                ));
                            }
                        } else {
                            pending_propagations.push((target_device, target_port, port_value));
                        }
                    }
                }
            }

            for (target_device, target_port, port_value) in pending_propagations {
                if let Some(target) = self.devices.get_mut(&target_device) {
                    target.set_input(target_port, port_value, current_tick);
                    dirty.insert(target_device);
                }
            }
        }

        result.cycle_detected = result.iterations >= config.max_iterations_per_tick;
        result.changed_devices = changed_outputs.into_iter().collect();
        result.changed_devices.sort();

        self.pending_signals.sort();

        result
    }

    /// Create a full snapshot of current state.
    #[must_use]
    pub fn snapshot(&self) -> AutomationSnapshot {
        let mut snapshot = AutomationSnapshot::at(self.revisions.current(), self.tick);

        for device in self.devices.values() {
            snapshot.add_device(device.clone());
        }

        for link in self.links.values() {
            snapshot.add_link(*link);
        }

        snapshot.pending_signals.clone_from(&self.pending_signals);
        snapshot.update_checksum();

        snapshot
    }

    /// Create a filtered snapshot for a spatial region.
    #[must_use]
    pub fn snapshot_filtered(&self, filter: &SpatialFilter) -> AutomationSnapshot {
        filter.filter_snapshot(&self.snapshot())
    }

    /// Create a delta batch since a revision.
    #[must_use]
    pub fn delta_since(&self, since_revision: Revision) -> AutomationDeltaBatch {
        let changes: Vec<_> = self
            .revisions
            .changes_since(since_revision)
            .cloned()
            .collect();

        let from_tick = changes.first().map_or(self.tick, |c| c.tick);
        let to_tick = self.tick;

        let mut batch = AutomationDeltaBatch::from_changes(&changes, from_tick, to_tick);
        batch.checksum = self.snapshot().compute_checksum();

        batch
    }

    /// Apply a snapshot to reset state.
    pub fn apply_snapshot(&mut self, snapshot: AutomationSnapshot) {
        self.devices = snapshot.devices;
        self.links = snapshot.links;
        self.pending_signals = snapshot.pending_signals;
        self.tick = snapshot.tick;

        self.rebuild_indices();

        self.next_device_id = self.devices.keys().map(|id| id.0).max().unwrap_or(0) + 1;
        self.next_link_id = self.links.keys().map(|id| id.0).max().unwrap_or(0) + 1;
    }

    /// Apply a delta batch to update state.
    pub fn apply_delta(&mut self, batch: &AutomationDeltaBatch) {
        for id in &batch.devices_removed {
            self.devices.remove(id);
        }

        for id in &batch.links_removed {
            self.links.remove(id);
        }

        for device in &batch.devices_added {
            self.devices.insert(device.id, device.clone());
        }

        for link in &batch.links_added {
            self.links.insert(link.id, *link);
        }

        for delta in &batch.device_deltas {
            if let Some(device) = self.devices.get_mut(&delta.device_id) {
                delta.apply(device, batch.to_tick);
            }
        }

        self.pending_signals
            .extend(batch.signals_added.iter().copied());
        self.pending_signals.sort();

        self.tick = batch.to_tick;
        self.rebuild_indices();
    }

    /// Rebuild link indices.
    fn rebuild_indices(&mut self) {
        self.outgoing_links.clear();
        self.incoming_links.clear();

        for link in self.links.values() {
            self.outgoing_links
                .entry(link.source_device)
                .or_default()
                .push(link.id);
            self.incoming_links
                .entry(link.target_device)
                .or_default()
                .push(link.id);
        }
    }

    /// Compute checksum of current state.
    #[must_use]
    pub fn checksum(&self) -> u32 {
        self.snapshot().compute_checksum()
    }

    /// Get changes since a revision.
    pub fn changes_since(&self, since: Revision) -> impl Iterator<Item = &StateChange> {
        self.revisions.changes_since(since)
    }

    /// Clear revision history.
    pub fn clear_history(&mut self) {
        self.revisions.clear_history();
    }

    /// Iterate over all devices.
    pub fn devices(&self) -> impl Iterator<Item = &AutomationDevice> {
        self.devices.values()
    }

    /// Iterate over all links.
    pub fn links(&self) -> impl Iterator<Item = &AutomationLink> {
        self.links.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automation::device::{DeviceConfig, DeviceKind};
    use engine_core::coords::WorldPos;

    fn make_relay(network: &mut AutomationNetwork, x: i32) -> DeviceId {
        let id = network.alloc_device_id();
        let device = AutomationDevice::new(id, DeviceKind::Relay, WorldPos::new(x, 0, 0));
        network.add_device(device);
        id
    }

    fn make_source(network: &mut AutomationNetwork, x: i32, value: SignalValue) -> DeviceId {
        let id = network.alloc_device_id();
        let mut device = AutomationDevice::new(id, DeviceKind::Source, WorldPos::new(x, 0, 0));
        device.set_output(PortId::OUTPUT_0, value, 0);
        network.add_device(device);
        id
    }

    #[test]
    fn network_creation() {
        let network = AutomationNetwork::new();
        assert_eq!(network.device_count(), 0);
        assert_eq!(network.link_count(), 0);
        assert_eq!(network.current_tick(), 0);
    }

    #[test]
    fn add_remove_device() {
        let mut network = AutomationNetwork::new();

        let id = make_relay(&mut network, 0);
        assert_eq!(network.device_count(), 1);
        assert!(network.device(id).is_some());

        let removed = network.remove_device(id);
        assert!(removed.is_some());
        assert_eq!(network.device_count(), 0);
    }

    #[test]
    fn add_remove_link() {
        let mut network = AutomationNetwork::new();

        let d1 = make_relay(&mut network, 0);
        let d2 = make_relay(&mut network, 16);

        let link_id = network.alloc_link_id();
        let link = AutomationLink::simple(link_id, d1, d2);

        assert!(network.add_link(link));
        assert_eq!(network.link_count(), 1);
        assert_eq!(network.outgoing_from(d1).len(), 1);
        assert_eq!(network.incoming_to(d2).len(), 1);

        let removed = network.remove_link(link_id);
        assert!(removed.is_some());
        assert_eq!(network.link_count(), 0);
    }

    #[test]
    fn signal_propagation() {
        let mut network = AutomationNetwork::new();
        let config = AutomationConfig::default();

        let source = make_source(&mut network, 0, SignalValue::Boolean(true));
        let relay = make_relay(&mut network, 16);

        let link_id = network.alloc_link_id();
        network.add_link(AutomationLink::simple(link_id, source, relay));

        let result = network.step(&config);

        assert!(result.devices_processed > 0);
        assert_eq!(
            network.device(relay).unwrap().output(),
            SignalValue::Boolean(true)
        );
    }

    #[test]
    fn delayed_signal() {
        let mut network = AutomationNetwork::new();
        let config = AutomationConfig::default();

        let source = make_source(&mut network, 0, SignalValue::Boolean(true));
        let relay = make_relay(&mut network, 16);

        let link_id = network.alloc_link_id();
        let link = AutomationLink::simple(link_id, source, relay).with_delay(2);
        network.add_link(link);

        network.step(&config);
        assert_eq!(network.device(relay).unwrap().output(), SignalValue::None);

        network.step(&config);
        assert_eq!(network.device(relay).unwrap().output(), SignalValue::None);

        network.step(&config);
        assert_eq!(
            network.device(relay).unwrap().output(),
            SignalValue::Boolean(true)
        );
    }

    #[test]
    fn snapshot_roundtrip() {
        let mut network = AutomationNetwork::new();

        let d1 = make_relay(&mut network, 0);
        let d2 = make_relay(&mut network, 16);

        let link_id = network.alloc_link_id();
        network.add_link(AutomationLink::simple(link_id, d1, d2));

        let snapshot = network.snapshot();
        assert_eq!(snapshot.device_count(), 2);
        assert_eq!(snapshot.link_count(), 1);

        let mut network2 = AutomationNetwork::new();
        network2.apply_snapshot(snapshot);

        assert_eq!(network2.device_count(), 2);
        assert_eq!(network2.link_count(), 1);
    }

    #[test]
    fn delta_application() {
        let mut network = AutomationNetwork::with_revision_tracking(100);
        let config = AutomationConfig::default();

        let source = make_source(&mut network, 0, SignalValue::None);
        let relay = make_relay(&mut network, 16);

        let link_id = network.alloc_link_id();
        network.add_link(AutomationLink::simple(link_id, source, relay));

        let rev_before = network.revision();
        network.step(&config);

        network.set_output(source, PortId::OUTPUT_0, SignalValue::Boolean(true));
        network.step(&config);

        let delta = network.delta_since(rev_before);
        assert!(!delta.is_empty());
    }

    #[test]
    fn checksum_deterministic() {
        let mut n1 = AutomationNetwork::new();
        let mut n2 = AutomationNetwork::new();

        for n in [&mut n1, &mut n2] {
            let d1 = make_relay(n, 0);
            let d2 = make_relay(n, 16);
            let link_id = n.alloc_link_id();
            n.add_link(AutomationLink::simple(link_id, d1, d2));
        }

        assert_eq!(n1.checksum(), n2.checksum());
    }

    #[test]
    fn cycle_detection() {
        let mut network = AutomationNetwork::new();
        let config = AutomationConfig {
            max_iterations_per_tick: 5,
            ..Default::default()
        };

        let d1 = network.alloc_device_id();
        let d2 = network.alloc_device_id();

        let mut dev1 = AutomationDevice::new(d1, DeviceKind::Gate, WorldPos::new(0, 0, 0))
            .with_config(DeviceConfig::not_gate());
        dev1.set_output(PortId::OUTPUT_0, SignalValue::Boolean(true), 0);
        network.add_device(dev1);

        let mut dev2 = AutomationDevice::new(d2, DeviceKind::Gate, WorldPos::new(16, 0, 0))
            .with_config(DeviceConfig::not_gate());
        dev2.set_output(PortId::OUTPUT_0, SignalValue::Boolean(false), 0);
        network.add_device(dev2);

        let l1 = network.alloc_link_id();
        let l2 = network.alloc_link_id();
        network.add_link(AutomationLink::simple(l1, d1, d2));
        network.add_link(AutomationLink::simple(l2, d2, d1));

        let result = network.step(&config);
        assert!(result.cycle_detected);
    }

    #[test]
    fn remove_device_cascades_links() {
        let mut network = AutomationNetwork::new();

        let d1 = make_relay(&mut network, 0);
        let d2 = make_relay(&mut network, 16);
        let d3 = make_relay(&mut network, 32);

        let l1 = network.alloc_link_id();
        let l2 = network.alloc_link_id();
        network.add_link(AutomationLink::simple(l1, d1, d2));
        network.add_link(AutomationLink::simple(l2, d2, d3));

        assert_eq!(network.link_count(), 2);

        network.remove_device(d2);

        assert_eq!(network.link_count(), 0);
    }

    #[test]
    fn serde_roundtrip() {
        let mut network = AutomationNetwork::new();

        let d1 = make_relay(&mut network, 0);
        let d2 = make_relay(&mut network, 16);
        let link_id = network.alloc_link_id();
        network.add_link(AutomationLink::simple(link_id, d1, d2));

        let json = serde_json::to_string(&network).unwrap();
        let recovered: AutomationNetwork = serde_json::from_str(&json).unwrap();

        assert_eq!(network.device_count(), recovered.device_count());
        assert_eq!(network.link_count(), recovered.link_count());
    }
}
