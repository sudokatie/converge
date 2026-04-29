//! Revision tracking for automation state changes.

use serde::{Deserialize, Serialize};

use super::device::DeviceId;
use super::link::LinkId;
use super::signal::SignalValue;

/// A monotonically increasing revision number.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct Revision(pub u64);

impl Revision {
    /// The initial revision.
    pub const ZERO: Self = Self(0);

    /// Create a revision from raw value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Increment and return the next revision.
    #[must_use]
    pub const fn next(&self) -> Self {
        Self(self.0 + 1)
    }

    /// Get the raw revision value.
    #[must_use]
    pub const fn value(&self) -> u64 {
        self.0
    }
}

/// The type of state change recorded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ChangeKind {
    /// Device added.
    DeviceAdded = 0,
    /// Device removed.
    DeviceRemoved = 1,
    /// Device state changed (port values, enabled, config).
    DeviceChanged = 2,
    /// Link added.
    LinkAdded = 3,
    /// Link removed.
    LinkRemoved = 4,
    /// Link state changed (enabled, delay).
    LinkChanged = 5,
}

impl ChangeKind {
    /// Check if this is a device-related change.
    #[must_use]
    pub const fn is_device(&self) -> bool {
        matches!(
            self,
            Self::DeviceAdded | Self::DeviceRemoved | Self::DeviceChanged
        )
    }

    /// Check if this is a link-related change.
    #[must_use]
    pub const fn is_link(&self) -> bool {
        matches!(
            self,
            Self::LinkAdded | Self::LinkRemoved | Self::LinkChanged
        )
    }

    /// Check if this is an addition.
    #[must_use]
    pub const fn is_add(&self) -> bool {
        matches!(self, Self::DeviceAdded | Self::LinkAdded)
    }

    /// Check if this is a removal.
    #[must_use]
    pub const fn is_remove(&self) -> bool {
        matches!(self, Self::DeviceRemoved | Self::LinkRemoved)
    }
}

/// Detailed change payload for device changes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DeviceChangePayload {
    /// Full device state for adds.
    Full(Box<super::device::AutomationDevice>),
    /// Port value change.
    PortChange {
        port: super::signal::PortId,
        old_value: SignalValue,
        new_value: SignalValue,
    },
    /// Enabled state change.
    EnabledChange { old: bool, new: bool },
    /// Config change.
    ConfigChange {
        old: super::device::DeviceConfig,
        new: super::device::DeviceConfig,
    },
    /// Device removed (stores previous state for rollback).
    Removed(Box<super::device::AutomationDevice>),
}

/// Detailed change payload for link changes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LinkChangePayload {
    /// Full link state for adds.
    Full(super::link::AutomationLink),
    /// Enabled state change.
    EnabledChange { old: bool, new: bool },
    /// Delay change.
    DelayChange { old: u8, new: u8 },
    /// Link removed (stores previous state for rollback).
    Removed(super::link::AutomationLink),
}

/// A single state change record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StateChange {
    /// Revision when this change occurred.
    pub revision: Revision,
    /// Simulation tick when this change occurred.
    pub tick: u64,
    /// Type of change.
    pub kind: ChangeKind,
    /// Device ID if this is a device change.
    pub device_id: Option<DeviceId>,
    /// Link ID if this is a link change.
    pub link_id: Option<LinkId>,
    /// Detailed payload.
    pub payload: ChangePayload,
}

/// Union of device and link change payloads.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ChangePayload {
    /// No payload (tombstone).
    None,
    /// Device change.
    Device(DeviceChangePayload),
    /// Link change.
    Link(LinkChangePayload),
}

impl StateChange {
    /// Create a device added change.
    #[must_use]
    pub fn device_added(
        revision: Revision,
        tick: u64,
        device: super::device::AutomationDevice,
    ) -> Self {
        Self {
            revision,
            tick,
            kind: ChangeKind::DeviceAdded,
            device_id: Some(device.id),
            link_id: None,
            payload: ChangePayload::Device(DeviceChangePayload::Full(Box::new(device))),
        }
    }

    /// Create a device removed change.
    #[must_use]
    pub fn device_removed(
        revision: Revision,
        tick: u64,
        device: super::device::AutomationDevice,
    ) -> Self {
        Self {
            revision,
            tick,
            kind: ChangeKind::DeviceRemoved,
            device_id: Some(device.id),
            link_id: None,
            payload: ChangePayload::Device(DeviceChangePayload::Removed(Box::new(device))),
        }
    }

    /// Create a device port change.
    #[must_use]
    pub fn device_port_change(
        revision: Revision,
        tick: u64,
        device_id: DeviceId,
        port: super::signal::PortId,
        old_value: SignalValue,
        new_value: SignalValue,
    ) -> Self {
        Self {
            revision,
            tick,
            kind: ChangeKind::DeviceChanged,
            device_id: Some(device_id),
            link_id: None,
            payload: ChangePayload::Device(DeviceChangePayload::PortChange {
                port,
                old_value,
                new_value,
            }),
        }
    }

    /// Create a link added change.
    #[must_use]
    pub fn link_added(revision: Revision, tick: u64, link: super::link::AutomationLink) -> Self {
        Self {
            revision,
            tick,
            kind: ChangeKind::LinkAdded,
            device_id: None,
            link_id: Some(link.id),
            payload: ChangePayload::Link(LinkChangePayload::Full(link)),
        }
    }

    /// Create a link removed change.
    #[must_use]
    pub fn link_removed(revision: Revision, tick: u64, link: super::link::AutomationLink) -> Self {
        Self {
            revision,
            tick,
            kind: ChangeKind::LinkRemoved,
            device_id: None,
            link_id: Some(link.id),
            payload: ChangePayload::Link(LinkChangePayload::Removed(link)),
        }
    }

    /// Ordering key for deterministic sorting.
    #[must_use]
    fn sort_key(&self) -> (u64, u64, u8, Option<u64>, Option<u64>) {
        (
            self.tick,
            self.revision.0,
            self.kind as u8,
            self.device_id.map(|d| d.0),
            self.link_id.map(|l| l.0),
        )
    }
}

impl PartialOrd for StateChange {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for StateChange {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

impl Eq for StateChange {}

/// Tracks revisions and provides change history.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RevisionTracker {
    /// Current revision number.
    current: Revision,
    /// History of changes.
    history: Vec<StateChange>,
    /// Maximum history size (0 = unlimited).
    max_history: usize,
}

impl RevisionTracker {
    /// Create a new tracker.
    #[must_use]
    pub fn new() -> Self {
        Self {
            current: Revision::ZERO,
            history: Vec::new(),
            max_history: 0,
        }
    }

    /// Create a tracker with limited history.
    #[must_use]
    pub fn with_max_history(max: usize) -> Self {
        Self {
            current: Revision::ZERO,
            history: Vec::new(),
            max_history: max,
        }
    }

    /// Get the current revision.
    #[must_use]
    pub const fn current(&self) -> Revision {
        self.current
    }

    /// Advance to the next revision and return it.
    pub fn advance(&mut self) -> Revision {
        self.current = self.current.next();
        self.current
    }

    /// Record a change.
    pub fn record(&mut self, change: StateChange) {
        self.history.push(change);
        if self.max_history > 0 && self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }

    /// Record a change and advance revision.
    pub fn record_and_advance(&mut self, mut change: StateChange) -> Revision {
        let rev = self.advance();
        change.revision = rev;
        self.record(change);
        rev
    }

    /// Get changes since a revision (exclusive).
    pub fn changes_since(&self, since: Revision) -> impl Iterator<Item = &StateChange> {
        self.history.iter().filter(move |c| c.revision > since)
    }

    /// Get changes in a revision range (inclusive).
    pub fn changes_in_range(
        &self,
        from: Revision,
        to: Revision,
    ) -> impl Iterator<Item = &StateChange> {
        self.history
            .iter()
            .filter(move |c| c.revision >= from && c.revision <= to)
    }

    /// Get changes for a specific device.
    pub fn changes_for_device(&self, device_id: DeviceId) -> impl Iterator<Item = &StateChange> {
        self.history
            .iter()
            .filter(move |c| c.device_id == Some(device_id))
    }

    /// Get changes for a specific link.
    pub fn changes_for_link(&self, link_id: LinkId) -> impl Iterator<Item = &StateChange> {
        self.history
            .iter()
            .filter(move |c| c.link_id == Some(link_id))
    }

    /// Get the full history.
    #[must_use]
    pub fn history(&self) -> &[StateChange] {
        &self.history
    }

    /// Clear history before a revision.
    pub fn truncate_before(&mut self, revision: Revision) {
        self.history.retain(|c| c.revision >= revision);
    }

    /// Clear all history.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Number of changes in history.
    #[must_use]
    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// Compute a checksum of changes since a revision.
    #[must_use]
    pub fn checksum_since(&self, since: Revision) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        for change in self.changes_since(since) {
            hasher.update(&change.revision.0.to_le_bytes());
            hasher.update(&change.tick.to_le_bytes());
            hasher.update(&[change.kind as u8]);
            if let Some(id) = change.device_id {
                hasher.update(&id.0.to_le_bytes());
            }
            if let Some(id) = change.link_id {
                hasher.update(&id.0.to_le_bytes());
            }
        }
        hasher.finalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automation::device::{AutomationDevice, DeviceKind};
    use crate::automation::link::AutomationLink;
    use crate::automation::signal::PortId;
    use engine_core::coords::WorldPos;

    #[test]
    fn revision_operations() {
        let r0 = Revision::ZERO;
        let r1 = r0.next();
        let r2 = r1.next();

        assert_eq!(r0.value(), 0);
        assert_eq!(r1.value(), 1);
        assert_eq!(r2.value(), 2);
        assert!(r0 < r1);
        assert!(r1 < r2);
    }

    #[test]
    fn tracker_advance() {
        let mut tracker = RevisionTracker::new();
        assert_eq!(tracker.current(), Revision::ZERO);

        let r1 = tracker.advance();
        assert_eq!(r1, Revision::new(1));
        assert_eq!(tracker.current(), r1);
    }

    #[test]
    fn tracker_record() {
        let mut tracker = RevisionTracker::new();
        let device =
            AutomationDevice::new(DeviceId::new(1), DeviceKind::Relay, WorldPos::new(0, 0, 0));

        let change = StateChange::device_added(Revision::new(1), 100, device);
        tracker.record(change);

        assert_eq!(tracker.history_len(), 1);
    }

    #[test]
    fn tracker_changes_since() {
        let mut tracker = RevisionTracker::new();
        let device =
            AutomationDevice::new(DeviceId::new(1), DeviceKind::Relay, WorldPos::new(0, 0, 0));

        let c1 = StateChange::device_added(Revision::new(1), 100, device.clone());
        let c2 = StateChange::device_port_change(
            Revision::new(2),
            101,
            DeviceId::new(1),
            PortId::OUTPUT_0,
            SignalValue::None,
            SignalValue::Boolean(true),
        );

        tracker.record(c1);
        tracker.record(c2);

        let since_0: Vec<_> = tracker.changes_since(Revision::ZERO).collect();
        assert_eq!(since_0.len(), 2);

        let since_1: Vec<_> = tracker.changes_since(Revision::new(1)).collect();
        assert_eq!(since_1.len(), 1);
    }

    #[test]
    fn tracker_max_history() {
        let mut tracker = RevisionTracker::with_max_history(2);
        let device =
            AutomationDevice::new(DeviceId::new(1), DeviceKind::Relay, WorldPos::new(0, 0, 0));

        for i in 1..=5 {
            let change = StateChange::device_added(Revision::new(i), i * 100, device.clone());
            tracker.record(change);
        }

        assert_eq!(tracker.history_len(), 2);
        assert_eq!(tracker.history()[0].revision, Revision::new(4));
    }

    #[test]
    fn change_kind_classification() {
        assert!(ChangeKind::DeviceAdded.is_device());
        assert!(ChangeKind::DeviceAdded.is_add());
        assert!(!ChangeKind::DeviceAdded.is_link());
        assert!(!ChangeKind::DeviceAdded.is_remove());

        assert!(ChangeKind::LinkRemoved.is_link());
        assert!(ChangeKind::LinkRemoved.is_remove());
    }

    #[test]
    fn state_change_ordering() {
        let device =
            AutomationDevice::new(DeviceId::new(1), DeviceKind::Relay, WorldPos::new(0, 0, 0));
        let c1 = StateChange::device_added(Revision::new(1), 100, device.clone());
        let c2 = StateChange::device_added(Revision::new(2), 100, device.clone());
        let c3 = StateChange::device_added(Revision::new(1), 101, device);

        assert!(c1 < c2);
        assert!(c1 < c3);
    }

    #[test]
    fn checksum_deterministic() {
        let mut tracker1 = RevisionTracker::new();
        let mut tracker2 = RevisionTracker::new();
        let device =
            AutomationDevice::new(DeviceId::new(1), DeviceKind::Relay, WorldPos::new(0, 0, 0));
        let link = AutomationLink::simple(LinkId::new(1), DeviceId::new(1), DeviceId::new(2));

        let c1 = StateChange::device_added(Revision::new(1), 100, device);
        let c2 = StateChange::link_added(Revision::new(2), 101, link);

        tracker1.record(c1.clone());
        tracker1.record(c2.clone());
        tracker2.record(c1);
        tracker2.record(c2);

        assert_eq!(
            tracker1.checksum_since(Revision::ZERO),
            tracker2.checksum_since(Revision::ZERO)
        );
    }

    #[test]
    fn serde_roundtrip() {
        let device =
            AutomationDevice::new(DeviceId::new(1), DeviceKind::Relay, WorldPos::new(0, 0, 0));
        let change = StateChange::device_added(Revision::new(1), 100, device);

        let json = serde_json::to_string(&change).unwrap();
        let recovered: StateChange = serde_json::from_str(&json).unwrap();
        assert_eq!(change, recovered);
    }
}
