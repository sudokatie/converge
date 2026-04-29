//! Automation device types and state.

use engine_core::coords::WorldPos;
use serde::{Deserialize, Serialize};

use super::signal::{PortId, SignalValue};

/// Unique identifier for an automation device within a network.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DeviceId(pub u64);

impl DeviceId {
    /// Create a device ID from raw value.
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

/// The kind/type of automation device.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum DeviceKind {
    /// Constant signal source (e.g., lever, button, sensor).
    Source = 0,
    /// Signal sink/consumer (e.g., lamp, door, machine).
    Sink = 1,
    /// Signal relay/repeater.
    Relay = 2,
    /// Logic gate (AND, OR, NOT, XOR).
    Gate = 3,
    /// Timer/pulse generator.
    Timer = 4,
    /// Memory/latch.
    Memory = 5,
    /// Comparator/threshold detector.
    Comparator = 6,
    /// Arithmetic combinator.
    Combinator = 7,
    /// Custom/scripted device.
    Custom = 255,
}

impl DeviceKind {
    /// Get display name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Sink => "sink",
            Self::Relay => "relay",
            Self::Gate => "gate",
            Self::Timer => "timer",
            Self::Memory => "memory",
            Self::Comparator => "comparator",
            Self::Combinator => "combinator",
            Self::Custom => "custom",
        }
    }

    /// Check if this device can produce output signals.
    #[must_use]
    pub const fn can_output(&self) -> bool {
        !matches!(self, Self::Sink)
    }

    /// Check if this device can accept input signals.
    #[must_use]
    pub const fn can_input(&self) -> bool {
        !matches!(self, Self::Source)
    }
}

/// Configuration data for specific device behaviors.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DeviceConfig {
    /// Gate type for Gate devices (0=AND, 1=OR, 2=NOT, 3=XOR).
    pub gate_type: u8,
    /// Timer interval in ticks for Timer devices.
    pub timer_interval: u32,
    /// Threshold value for Comparator devices.
    pub threshold: SignalValue,
    /// Custom configuration data.
    pub custom_data: Vec<u8>,
}

impl DeviceConfig {
    /// Empty configuration constant.
    pub const EMPTY: Self = Self {
        gate_type: 0,
        timer_interval: 0,
        threshold: SignalValue::None,
        custom_data: Vec::new(),
    };

    /// Create an AND gate config.
    #[must_use]
    pub fn and_gate() -> Self {
        Self {
            gate_type: 0,
            ..Self::default()
        }
    }

    /// Create an OR gate config.
    #[must_use]
    pub fn or_gate() -> Self {
        Self {
            gate_type: 1,
            ..Self::default()
        }
    }

    /// Create a NOT gate config.
    #[must_use]
    pub fn not_gate() -> Self {
        Self {
            gate_type: 2,
            ..Self::default()
        }
    }

    /// Create a timer config.
    #[must_use]
    pub fn timer(interval: u32) -> Self {
        Self {
            timer_interval: interval,
            ..Self::default()
        }
    }
}

/// Runtime state of a device port.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PortState {
    /// Current signal value.
    pub value: SignalValue,
    /// Tick when last updated.
    pub last_update_tick: u64,
}

impl PortState {
    /// Create a new port state with a value.
    #[must_use]
    pub const fn new(value: SignalValue) -> Self {
        Self {
            value,
            last_update_tick: 0,
        }
    }

    /// Create a new port state at a specific tick.
    #[must_use]
    pub const fn at_tick(value: SignalValue, tick: u64) -> Self {
        Self {
            value,
            last_update_tick: tick,
        }
    }
}

/// Maximum number of ports per device.
pub const MAX_PORTS: usize = 8;

/// An automation device with its current state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutomationDevice {
    /// Unique device identifier.
    pub id: DeviceId,
    /// Device type.
    pub kind: DeviceKind,
    /// World position of the device.
    pub position: WorldPos,
    /// Configuration/behavior parameters.
    pub config: DeviceConfig,
    /// Port states (indexed by `PortId.0`).
    pub ports: [PortState; MAX_PORTS],
    /// Whether the device is enabled.
    pub enabled: bool,
    /// Internal timer state for Timer devices.
    pub timer_counter: u32,
    /// Last tick this device was processed.
    pub last_tick: u64,
}

impl AutomationDevice {
    /// Create a new device.
    #[must_use]
    pub fn new(id: DeviceId, kind: DeviceKind, position: WorldPos) -> Self {
        Self {
            id,
            kind,
            position,
            config: DeviceConfig::EMPTY,
            ports: [PortState::default(); MAX_PORTS],
            enabled: true,
            timer_counter: 0,
            last_tick: 0,
        }
    }

    /// Create a device with configuration.
    #[must_use]
    pub fn with_config(mut self, config: DeviceConfig) -> Self {
        self.config = config;
        self
    }

    /// Get the current output value (from primary output port).
    #[must_use]
    pub fn output(&self) -> SignalValue {
        self.ports[PortId::OUTPUT_0.index()].value
    }

    /// Get the current input value (from primary input port).
    #[must_use]
    pub fn input(&self) -> SignalValue {
        self.ports[PortId::INPUT_0.index()].value
    }

    /// Set an input port value.
    pub fn set_input(&mut self, port: PortId, value: SignalValue, tick: u64) {
        if port.is_input() {
            self.ports[port.index()] = PortState::at_tick(value, tick);
        }
    }

    /// Set an output port value.
    pub fn set_output(&mut self, port: PortId, value: SignalValue, tick: u64) {
        if port.is_output() {
            self.ports[port.index()] = PortState::at_tick(value, tick);
        }
    }

    /// Get a port value.
    #[must_use]
    pub fn port(&self, port: PortId) -> SignalValue {
        self.ports[port.index()].value
    }

    /// Get port state including tick info.
    #[must_use]
    pub fn port_state(&self, port: PortId) -> PortState {
        self.ports[port.index()]
    }

    /// Process one tick and compute outputs.
    pub fn tick(&mut self, tick: u64) {
        if !self.enabled || tick <= self.last_tick {
            return;
        }

        self.compute_output(tick);
        self.last_tick = tick;
    }

    /// Recompute outputs without the `last_tick` guard (for within-step iterations).
    pub fn recompute(&mut self, tick: u64) {
        if !self.enabled {
            return;
        }

        self.compute_output(tick);
        self.last_tick = tick;
    }

    fn compute_output(&mut self, tick: u64) {
        let output = match self.kind {
            DeviceKind::Source | DeviceKind::Custom => self.output(),
            DeviceKind::Sink => SignalValue::None,
            DeviceKind::Relay => self.input(),
            DeviceKind::Gate => self.compute_gate(),
            DeviceKind::Timer => self.compute_timer(),
            DeviceKind::Memory => self.ports[PortId::OUTPUT_0.index()].value,
            DeviceKind::Comparator => self.compute_comparator(),
            DeviceKind::Combinator => self.compute_combinator(),
        };

        self.set_output(PortId::OUTPUT_0, output, tick);
    }

    fn compute_gate(&self) -> SignalValue {
        let a = self.ports[PortId::INPUT_0.index()].value.is_truthy();
        let b = self.ports[PortId::INPUT_1.index()].value.is_truthy();

        let result = match self.config.gate_type {
            0 => a && b,
            1 => a || b,
            2 => !a,
            3 => a ^ b,
            _ => false,
        };

        SignalValue::Boolean(result)
    }

    fn compute_timer(&mut self) -> SignalValue {
        if self.config.timer_interval == 0 {
            return SignalValue::Boolean(false);
        }

        self.timer_counter += 1;
        if self.timer_counter >= self.config.timer_interval {
            self.timer_counter = 0;
            SignalValue::Boolean(true)
        } else {
            SignalValue::Boolean(false)
        }
    }

    fn compute_comparator(&self) -> SignalValue {
        let input = self.input().to_float();
        let threshold = self.config.threshold.to_float();
        SignalValue::Boolean(input >= threshold)
    }

    fn compute_combinator(&self) -> SignalValue {
        let a = self.ports[PortId::INPUT_0.index()].value.to_int();
        let b = self.ports[PortId::INPUT_1.index()].value.to_int();
        SignalValue::Integer(a.saturating_add(b))
    }

    /// Ordering key for deterministic sorting.
    #[must_use]
    fn sort_key(&self) -> (i32, i32, i32, u64) {
        (
            self.position.x(),
            self.position.y(),
            self.position.z(),
            self.id.0,
        )
    }
}

impl PartialOrd for AutomationDevice {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AutomationDevice {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

impl Eq for AutomationDevice {}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_device(kind: DeviceKind) -> AutomationDevice {
        AutomationDevice::new(DeviceId::new(1), kind, WorldPos::new(0, 0, 0))
    }

    #[test]
    fn device_creation() {
        let device = make_device(DeviceKind::Relay);
        assert_eq!(device.kind, DeviceKind::Relay);
        assert!(device.enabled);
        assert_eq!(device.output(), SignalValue::None);
    }

    #[test]
    fn device_relay() {
        let mut device = make_device(DeviceKind::Relay);
        device.set_input(PortId::INPUT_0, SignalValue::Integer(42), 1);
        device.tick(1);
        assert_eq!(device.output(), SignalValue::Integer(42));
    }

    #[test]
    fn device_and_gate() {
        let mut device = make_device(DeviceKind::Gate).with_config(DeviceConfig::and_gate());

        device.set_input(PortId::INPUT_0, SignalValue::Boolean(true), 1);
        device.set_input(PortId::INPUT_1, SignalValue::Boolean(false), 1);
        device.tick(1);
        assert_eq!(device.output(), SignalValue::Boolean(false));

        device.set_input(PortId::INPUT_1, SignalValue::Boolean(true), 2);
        device.tick(2);
        assert_eq!(device.output(), SignalValue::Boolean(true));
    }

    #[test]
    fn device_or_gate() {
        let mut device = make_device(DeviceKind::Gate).with_config(DeviceConfig::or_gate());

        device.set_input(PortId::INPUT_0, SignalValue::Boolean(false), 1);
        device.set_input(PortId::INPUT_1, SignalValue::Boolean(false), 1);
        device.tick(1);
        assert_eq!(device.output(), SignalValue::Boolean(false));

        device.set_input(PortId::INPUT_0, SignalValue::Boolean(true), 2);
        device.tick(2);
        assert_eq!(device.output(), SignalValue::Boolean(true));
    }

    #[test]
    fn device_timer() {
        let mut device = make_device(DeviceKind::Timer).with_config(DeviceConfig::timer(3));

        device.tick(1);
        assert_eq!(device.output(), SignalValue::Boolean(false));
        device.tick(2);
        assert_eq!(device.output(), SignalValue::Boolean(false));
        device.tick(3);
        assert_eq!(device.output(), SignalValue::Boolean(true));
        device.tick(4);
        assert_eq!(device.output(), SignalValue::Boolean(false));
    }

    #[test]
    fn device_comparator() {
        let mut device = make_device(DeviceKind::Comparator).with_config(DeviceConfig {
            threshold: SignalValue::Float(0.5),
            ..DeviceConfig::EMPTY
        });

        device.set_input(PortId::INPUT_0, SignalValue::Float(0.3), 1);
        device.tick(1);
        assert_eq!(device.output(), SignalValue::Boolean(false));

        device.set_input(PortId::INPUT_0, SignalValue::Float(0.7), 2);
        device.tick(2);
        assert_eq!(device.output(), SignalValue::Boolean(true));
    }

    #[test]
    fn device_ordering() {
        let d1 = AutomationDevice::new(DeviceId::new(1), DeviceKind::Relay, WorldPos::new(0, 0, 0));
        let d2 = AutomationDevice::new(DeviceId::new(2), DeviceKind::Relay, WorldPos::new(0, 0, 0));
        let d3 = AutomationDevice::new(DeviceId::new(1), DeviceKind::Relay, WorldPos::new(1, 0, 0));

        assert!(d1 < d2);
        assert!(d1 < d3);
        assert!(d2 < d3);
    }

    #[test]
    fn serde_roundtrip() {
        let device = make_device(DeviceKind::Gate).with_config(DeviceConfig::and_gate());

        let json = serde_json::to_string(&device).unwrap();
        let recovered: AutomationDevice = serde_json::from_str(&json).unwrap();
        assert_eq!(device, recovered);
    }
}
