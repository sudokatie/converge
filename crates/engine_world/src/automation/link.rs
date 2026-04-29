//! Links between automation devices for signal routing.

use serde::{Deserialize, Serialize};

use super::device::DeviceId;
use super::signal::PortId;

/// Unique identifier for a link between devices.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LinkId(pub u64);

impl LinkId {
    /// Create a link ID from raw value.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value.
    #[must_use]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// A directional link from one device port to another.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AutomationLink {
    /// Unique link identifier.
    pub id: LinkId,
    /// Source device.
    pub source_device: DeviceId,
    /// Source port (must be output).
    pub source_port: PortId,
    /// Target device.
    pub target_device: DeviceId,
    /// Target port (must be input).
    pub target_port: PortId,
    /// Signal delay in ticks (0 = immediate).
    pub delay: u8,
    /// Whether the link is currently active.
    pub enabled: bool,
}

impl AutomationLink {
    /// Create a new link.
    #[must_use]
    pub const fn new(
        id: LinkId,
        source_device: DeviceId,
        source_port: PortId,
        target_device: DeviceId,
        target_port: PortId,
    ) -> Self {
        Self {
            id,
            source_device,
            source_port,
            target_device,
            target_port,
            delay: 0,
            enabled: true,
        }
    }

    /// Create a simple link using default ports.
    #[must_use]
    pub const fn simple(id: LinkId, from: DeviceId, to: DeviceId) -> Self {
        Self::new(id, from, PortId::OUTPUT_0, to, PortId::INPUT_0)
    }

    /// Add delay to the link.
    #[must_use]
    pub const fn with_delay(mut self, delay: u8) -> Self {
        self.delay = delay;
        self
    }

    /// Check if link configuration is valid.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.source_port.is_output()
            && self.target_port.is_input()
            && self.source_device.0 != self.target_device.0
    }

    /// Ordering key for deterministic sorting.
    #[must_use]
    fn sort_key(&self) -> (u64, u8, u64, u8) {
        (
            self.source_device.0,
            self.source_port.0,
            self.target_device.0,
            self.target_port.0,
        )
    }
}

impl PartialOrd for AutomationLink {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AutomationLink {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

/// A pending signal transmission through a delayed link.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PendingSignal {
    /// Link this signal is traveling through.
    pub link_id: LinkId,
    /// Tick when the signal should arrive.
    pub arrival_tick: u64,
    /// The signal value.
    pub value: super::signal::SignalValue,
}

impl PendingSignal {
    /// Create a new pending signal.
    #[must_use]
    pub const fn new(
        link_id: LinkId,
        arrival_tick: u64,
        value: super::signal::SignalValue,
    ) -> Self {
        Self {
            link_id,
            arrival_tick,
            value,
        }
    }

    /// Check if this signal should arrive at the given tick.
    #[must_use]
    pub const fn arrives_at(&self, tick: u64) -> bool {
        self.arrival_tick == tick
    }

    /// Check if this signal has expired (arrival tick has passed).
    #[must_use]
    pub const fn is_expired(&self, current_tick: u64) -> bool {
        self.arrival_tick < current_tick
    }
}

impl PartialOrd for PendingSignal {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PendingSignal {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.arrival_tick
            .cmp(&other.arrival_tick)
            .then(self.link_id.0.cmp(&other.link_id.0))
    }
}

impl Eq for PendingSignal {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automation::signal::SignalValue;

    #[test]
    fn link_creation() {
        let link = AutomationLink::simple(LinkId::new(1), DeviceId::new(10), DeviceId::new(20));
        assert!(link.is_valid());
        assert!(link.enabled);
        assert_eq!(link.delay, 0);
    }

    #[test]
    fn link_with_delay() {
        let link = AutomationLink::simple(LinkId::new(1), DeviceId::new(10), DeviceId::new(20))
            .with_delay(5);
        assert_eq!(link.delay, 5);
    }

    #[test]
    fn link_validation() {
        let valid = AutomationLink::new(
            LinkId::new(1),
            DeviceId::new(10),
            PortId::OUTPUT_0,
            DeviceId::new(20),
            PortId::INPUT_0,
        );
        assert!(valid.is_valid());

        let invalid_ports = AutomationLink::new(
            LinkId::new(2),
            DeviceId::new(10),
            PortId::INPUT_0,
            DeviceId::new(20),
            PortId::OUTPUT_0,
        );
        assert!(!invalid_ports.is_valid());

        let self_link = AutomationLink::new(
            LinkId::new(3),
            DeviceId::new(10),
            PortId::OUTPUT_0,
            DeviceId::new(10),
            PortId::INPUT_0,
        );
        assert!(!self_link.is_valid());
    }

    #[test]
    fn link_ordering() {
        let l1 = AutomationLink::simple(LinkId::new(1), DeviceId::new(10), DeviceId::new(20));
        let l2 = AutomationLink::simple(LinkId::new(2), DeviceId::new(10), DeviceId::new(30));
        let l3 = AutomationLink::simple(LinkId::new(3), DeviceId::new(20), DeviceId::new(10));

        assert!(l1 < l2);
        assert!(l2 < l3);
    }

    #[test]
    fn pending_signal() {
        let signal = PendingSignal::new(LinkId::new(1), 100, SignalValue::Boolean(true));

        assert!(signal.arrives_at(100));
        assert!(!signal.arrives_at(99));
        assert!(!signal.is_expired(99));
        assert!(!signal.is_expired(100));
        assert!(signal.is_expired(101));
    }

    #[test]
    fn pending_signal_ordering() {
        let s1 = PendingSignal::new(LinkId::new(1), 100, SignalValue::Boolean(true));
        let s2 = PendingSignal::new(LinkId::new(2), 100, SignalValue::Boolean(true));
        let s3 = PendingSignal::new(LinkId::new(1), 101, SignalValue::Boolean(true));

        assert!(s1 < s2);
        assert!(s2 < s3);
    }

    #[test]
    fn serde_roundtrip() {
        let link = AutomationLink::simple(LinkId::new(42), DeviceId::new(1), DeviceId::new(2))
            .with_delay(3);

        let json = serde_json::to_string(&link).unwrap();
        let recovered: AutomationLink = serde_json::from_str(&json).unwrap();
        assert_eq!(link, recovered);
    }
}
