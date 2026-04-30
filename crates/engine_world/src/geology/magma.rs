//! Magma pocket and flow simulation.

use serde::{Deserialize, Serialize};

use super::config::MagmaConfig;
use super::identity::FeatureId;

/// State of magma activity.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[repr(u8)]
pub enum MagmaState {
    /// Magma is cool and solidified.
    #[default]
    Dormant = 0,
    /// Magma is warming up.
    Warming = 1,
    /// Magma is active and flowing.
    Active = 2,
    /// Magma is under high pressure.
    Pressurized = 3,
    /// Magma is erupting.
    Erupting = 4,
    /// Magma is cooling after activity.
    Cooling = 5,
}

impl MagmaState {
    pub const ALL: [MagmaState; 6] = [
        Self::Dormant,
        Self::Warming,
        Self::Active,
        Self::Pressurized,
        Self::Erupting,
        Self::Cooling,
    ];

    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Dormant => "dormant",
            Self::Warming => "warming",
            Self::Active => "active",
            Self::Pressurized => "pressurized",
            Self::Erupting => "erupting",
            Self::Cooling => "cooling",
        }
    }

    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Active | Self::Pressurized | Self::Erupting)
    }

    #[must_use]
    pub const fn is_dangerous(&self) -> bool {
        matches!(self, Self::Pressurized | Self::Erupting)
    }

    #[must_use]
    pub const fn heat_output_multiplier(&self) -> f32 {
        match self {
            Self::Dormant => 0.0,
            Self::Warming => 0.3,
            Self::Active => 1.0,
            Self::Pressurized => 1.5,
            Self::Erupting => 2.0,
            Self::Cooling => 0.5,
        }
    }

    #[must_use]
    pub const fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Dormant),
            1 => Some(Self::Warming),
            2 => Some(Self::Active),
            3 => Some(Self::Pressurized),
            4 => Some(Self::Erupting),
            5 => Some(Self::Cooling),
            _ => None,
        }
    }
}

/// A pocket of magma beneath the surface.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MagmaPocket {
    /// Unique identifier.
    pub id: FeatureId,
    /// Center position (x, y, depth).
    pub position: (f32, f32, f32),
    /// Approximate radius.
    pub radius: f32,
    /// Volume of magma (cubic units).
    pub volume: f32,
    /// Current temperature.
    pub temperature: f32,
    /// Current pressure.
    pub pressure: f32,
    /// Current state.
    state: MagmaState,
    /// Viscosity (affects flow rate).
    pub viscosity: f32,
    /// Gas content (affects explosivity).
    pub gas_content: f32,
    /// Connected flow channels.
    connected_flows: Vec<FeatureId>,
    /// Last tick processed.
    last_tick: u64,
}

impl MagmaPocket {
    #[must_use]
    pub fn new(id: FeatureId, position: (f32, f32, f32), radius: f32) -> Self {
        let volume = (4.0 / 3.0) * std::f32::consts::PI * radius.powi(3);
        Self {
            id,
            position,
            radius: radius.max(1.0),
            volume,
            temperature: 1200.0,
            pressure: position.2 * 0.1,
            state: MagmaState::Active,
            viscosity: 0.5,
            gas_content: 0.1,
            connected_flows: Vec::new(),
            last_tick: 0,
        }
    }

    #[must_use]
    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = temp.clamp(700.0, 2000.0);
        self
    }

    #[must_use]
    pub fn with_pressure(mut self, pressure: f32) -> Self {
        self.pressure = pressure.max(0.0);
        self
    }

    #[must_use]
    pub fn with_viscosity(mut self, visc: f32) -> Self {
        self.viscosity = visc.clamp(0.1, 2.0);
        self
    }

    #[must_use]
    pub fn with_gas_content(mut self, gas: f32) -> Self {
        self.gas_content = gas.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn state(&self) -> MagmaState {
        self.state
    }

    #[must_use]
    pub fn connected_flows(&self) -> &[FeatureId] {
        &self.connected_flows
    }

    #[must_use]
    pub fn depth(&self) -> f32 {
        self.position.2
    }

    #[must_use]
    pub fn heat_output(&self) -> f32 {
        let base_heat = (self.temperature - 700.0).max(0.0) * 0.01;
        base_heat * self.state.heat_output_multiplier() * (self.volume / 1000.0).sqrt()
    }

    #[must_use]
    pub fn explosivity(&self) -> f32 {
        let gas_factor = self.gas_content * 2.0;
        let viscosity_factor = self.viscosity;
        let pressure_factor = (self.pressure / 100.0).min(1.0);
        (gas_factor * viscosity_factor * pressure_factor).clamp(0.0, 1.0)
    }

    pub fn connect_flow(&mut self, flow_id: FeatureId) {
        if !self.connected_flows.contains(&flow_id) {
            self.connected_flows.push(flow_id);
            self.connected_flows.sort();
        }
    }

    pub fn disconnect_flow(&mut self, flow_id: FeatureId) {
        self.connected_flows.retain(|&id| id != flow_id);
    }

    pub fn add_pressure(&mut self, amount: f32) {
        self.pressure = (self.pressure + amount).max(0.0);
    }

    pub fn release_pressure(&mut self, amount: f32) -> f32 {
        let released = amount.min(self.pressure);
        self.pressure -= released;
        released
    }

    pub fn add_volume(&mut self, amount: f32) {
        self.volume = (self.volume + amount).max(0.0);
    }

    pub fn remove_volume(&mut self, amount: f32) -> f32 {
        let removed = amount.min(self.volume);
        self.volume -= removed;
        removed
    }

    pub fn tick(&mut self, config: &MagmaConfig, current_tick: u64) -> Option<VolcanicEvent> {
        if current_tick <= self.last_tick {
            return None;
        }
        self.last_tick = current_tick;

        self.pressure += config.pressure_buildup_rate;

        let target_temp = if self.state.is_active() {
            config.base_temperature
        } else {
            700.0
        };
        let temp_delta = (target_temp - self.temperature) * config.cooling_rate;
        self.temperature = (self.temperature + temp_delta).clamp(700.0, 2000.0);

        let prev_state = self.state;
        self.update_state(config);

        if self.state == MagmaState::Erupting && prev_state != MagmaState::Erupting {
            return Some(VolcanicEvent {
                pocket_id: self.id,
                tick: current_tick,
                kind: VolcanicEventKind::EruptionStart,
                magnitude: self.explosivity(),
                position: self.position,
            });
        }

        if prev_state == MagmaState::Erupting && self.state != MagmaState::Erupting {
            return Some(VolcanicEvent {
                pocket_id: self.id,
                tick: current_tick,
                kind: VolcanicEventKind::EruptionEnd,
                magnitude: 0.0,
                position: self.position,
            });
        }

        None
    }

    fn update_state(&mut self, config: &MagmaConfig) {
        self.state = if self.pressure >= config.eruption_threshold {
            self.pressure *= 0.4;
            MagmaState::Erupting
        } else if self.pressure >= config.eruption_threshold * 0.8 {
            MagmaState::Pressurized
        } else if self.temperature >= config.base_temperature * 0.9 {
            MagmaState::Active
        } else if self.temperature >= 700.0 {
            if self.state == MagmaState::Active || self.state == MagmaState::Pressurized {
                MagmaState::Cooling
            } else {
                MagmaState::Warming
            }
        } else {
            MagmaState::Dormant
        };
    }

    #[must_use]
    pub fn fingerprint(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.id.raw().to_le_bytes());
        hasher.update(&self.temperature.to_le_bytes());
        hasher.update(&self.pressure.to_le_bytes());
        hasher.update(&self.volume.to_le_bytes());
        hasher.update(&[self.state as u8]);
        hasher.finalize()
    }
}

/// A channel through which magma flows.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MagmaFlow {
    /// Unique identifier.
    pub id: FeatureId,
    /// Start position.
    pub start: (f32, f32, f32),
    /// End position.
    pub end: (f32, f32, f32),
    /// Cross-sectional radius.
    pub radius: f32,
    /// Current flow rate (volume per tick).
    pub flow_rate: f32,
    /// Temperature of flowing magma.
    pub temperature: f32,
    /// Whether flow is currently active.
    active: bool,
    /// Source pocket.
    pub source_pocket: Option<FeatureId>,
    /// Resistance factor (higher = slower flow).
    pub resistance: f32,
    /// Last tick processed.
    last_tick: u64,
}

impl MagmaFlow {
    #[must_use]
    pub fn new(id: FeatureId, start: (f32, f32, f32), end: (f32, f32, f32), radius: f32) -> Self {
        Self {
            id,
            start,
            end,
            radius: radius.max(0.1),
            flow_rate: 0.0,
            temperature: 1000.0,
            active: false,
            source_pocket: None,
            resistance: 1.0,
            last_tick: 0,
        }
    }

    #[must_use]
    pub fn with_source(mut self, pocket_id: FeatureId) -> Self {
        self.source_pocket = Some(pocket_id);
        self
    }

    #[must_use]
    pub fn with_resistance(mut self, resistance: f32) -> Self {
        self.resistance = resistance.clamp(0.1, 10.0);
        self
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        self.active
    }

    #[must_use]
    pub fn length(&self) -> f32 {
        let dx = self.end.0 - self.start.0;
        let dy = self.end.1 - self.start.1;
        let dz = self.end.2 - self.start.2;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    #[must_use]
    pub fn direction(&self) -> (f32, f32, f32) {
        let len = self.length();
        if len < f32::EPSILON {
            return (0.0, 0.0, -1.0);
        }
        (
            (self.end.0 - self.start.0) / len,
            (self.end.1 - self.start.1) / len,
            (self.end.2 - self.start.2) / len,
        )
    }

    #[must_use]
    pub fn capacity(&self) -> f32 {
        std::f32::consts::PI * self.radius * self.radius * self.length()
    }

    #[must_use]
    pub fn max_flow_rate(&self, viscosity: f32) -> f32 {
        let cross_section = std::f32::consts::PI * self.radius * self.radius;
        cross_section / (viscosity * self.resistance)
    }

    pub fn activate(&mut self, source_temp: f32, flow_rate: f32) {
        self.active = true;
        self.temperature = source_temp;
        self.flow_rate = flow_rate.max(0.0);
    }

    pub fn deactivate(&mut self) {
        self.active = false;
        self.flow_rate = 0.0;
    }

    pub fn tick(&mut self, config: &MagmaConfig, current_tick: u64) {
        if current_tick <= self.last_tick {
            return;
        }
        self.last_tick = current_tick;

        if self.active {
            self.temperature -= config.cooling_rate * self.length() * 0.01;
            self.temperature = self.temperature.max(700.0);

            if self.temperature < 750.0 {
                self.flow_rate *= 0.9;
                if self.flow_rate < 0.01 {
                    self.deactivate();
                }
            }
        }
    }

    #[must_use]
    pub fn fingerprint(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.id.raw().to_le_bytes());
        hasher.update(&self.flow_rate.to_le_bytes());
        hasher.update(&self.temperature.to_le_bytes());
        hasher.update(&[u8::from(self.active)]);
        hasher.finalize()
    }
}

/// Kind of volcanic event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VolcanicEventKind {
    /// Eruption has started.
    EruptionStart,
    /// Eruption has ended.
    EruptionEnd,
    /// Pressure has exceeded warning threshold.
    PressureWarning,
    /// New flow channel has opened.
    FlowOpened,
    /// Flow channel has closed.
    FlowClosed,
    /// Magma pocket has been depleted.
    PocketDepleted,
}

impl VolcanicEventKind {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::EruptionStart => "eruption_start",
            Self::EruptionEnd => "eruption_end",
            Self::PressureWarning => "pressure_warning",
            Self::FlowOpened => "flow_opened",
            Self::FlowClosed => "flow_closed",
            Self::PocketDepleted => "pocket_depleted",
        }
    }

    #[must_use]
    pub const fn is_critical(&self) -> bool {
        matches!(self, Self::EruptionStart | Self::PressureWarning)
    }
}

/// A volcanic event during simulation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VolcanicEvent {
    /// Source magma pocket.
    pub pocket_id: FeatureId,
    /// Tick when event occurred.
    pub tick: u64,
    /// Kind of event.
    pub kind: VolcanicEventKind,
    /// Magnitude/intensity (0-1 for most events).
    pub magnitude: f32,
    /// Position where event occurred.
    pub position: (f32, f32, f32),
}

impl VolcanicEvent {
    fn sort_key(&self) -> (u64, FeatureId, &'static str) {
        (self.tick, self.pocket_id, self.kind.name())
    }
}

impl PartialOrd for VolcanicEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VolcanicEvent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

impl Eq for VolcanicEvent {}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_pocket() -> MagmaPocket {
        MagmaPocket::new(FeatureId::new(1), (0.0, 0.0, 100.0), 10.0)
    }

    fn test_flow() -> MagmaFlow {
        MagmaFlow::new(FeatureId::new(2), (0.0, 0.0, 100.0), (0.0, 0.0, 50.0), 2.0)
    }

    #[test]
    fn magma_state_properties() {
        assert!(MagmaState::Active.is_active());
        assert!(MagmaState::Erupting.is_dangerous());
        assert!(!MagmaState::Dormant.is_active());
        assert!(
            MagmaState::Erupting.heat_output_multiplier()
                > MagmaState::Active.heat_output_multiplier()
        );
    }

    #[test]
    fn magma_state_from_raw() {
        for (i, state) in MagmaState::ALL.iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            let idx = i as u8;
            assert_eq!(MagmaState::from_raw(idx), Some(*state));
        }
        assert_eq!(MagmaState::from_raw(10), None);
    }

    #[test]
    fn magma_pocket_creation() {
        let pocket = test_pocket();
        assert_eq!(pocket.id, FeatureId::new(1));
        assert!((pocket.depth() - 100.0).abs() < f32::EPSILON);
        assert!(pocket.volume > 0.0);
    }

    #[test]
    fn magma_pocket_heat_output() {
        let pocket = test_pocket().with_temperature(1200.0);
        let heat = pocket.heat_output();
        assert!(heat > 0.0);

        let dormant =
            MagmaPocket::new(FeatureId::new(2), (0.0, 0.0, 100.0), 10.0).with_temperature(700.0);
        let dormant_heat = dormant.heat_output();
        assert!(heat > dormant_heat);
    }

    #[test]
    fn magma_pocket_explosivity() {
        let low_gas = test_pocket().with_gas_content(0.1);
        let high_gas = test_pocket().with_gas_content(0.9);

        assert!(high_gas.explosivity() > low_gas.explosivity());
    }

    #[test]
    fn magma_pocket_flow_connections() {
        let mut pocket = test_pocket();
        pocket.connect_flow(FeatureId::new(10));
        pocket.connect_flow(FeatureId::new(20));

        assert_eq!(pocket.connected_flows().len(), 2);

        pocket.disconnect_flow(FeatureId::new(10));
        assert_eq!(pocket.connected_flows().len(), 1);
    }

    #[test]
    fn magma_pocket_pressure_operations() {
        let mut pocket = test_pocket();
        let initial = pocket.pressure;

        pocket.add_pressure(10.0);
        assert!((pocket.pressure - initial - 10.0).abs() < f32::EPSILON);

        let released = pocket.release_pressure(5.0);
        assert!((released - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn magma_flow_geometry() {
        let flow = test_flow();
        assert!((flow.length() - 50.0).abs() < f32::EPSILON);

        let dir = flow.direction();
        assert!((dir.2 - (-1.0)).abs() < f32::EPSILON);

        assert!(flow.capacity() > 0.0);
    }

    #[test]
    fn magma_flow_activation() {
        let mut flow = test_flow();
        assert!(!flow.is_active());

        flow.activate(1000.0, 5.0);
        assert!(flow.is_active());
        assert!((flow.temperature - 1000.0).abs() < f32::EPSILON);
        assert!((flow.flow_rate - 5.0).abs() < f32::EPSILON);

        flow.deactivate();
        assert!(!flow.is_active());
    }

    #[test]
    fn volcanic_event_ordering() {
        let e1 = VolcanicEvent {
            pocket_id: FeatureId::new(1),
            tick: 100,
            kind: VolcanicEventKind::EruptionStart,
            magnitude: 0.8,
            position: (0.0, 0.0, 0.0),
        };

        let e2 = VolcanicEvent {
            pocket_id: FeatureId::new(2),
            tick: 100,
            kind: VolcanicEventKind::EruptionStart,
            magnitude: 0.5,
            position: (0.0, 0.0, 0.0),
        };

        let e3 = VolcanicEvent {
            pocket_id: FeatureId::new(1),
            tick: 101,
            kind: VolcanicEventKind::EruptionEnd,
            magnitude: 0.0,
            position: (0.0, 0.0, 0.0),
        };

        assert!(e1 < e2);
        assert!(e2 < e3);
    }

    #[test]
    fn volcanic_event_kind_properties() {
        assert!(VolcanicEventKind::EruptionStart.is_critical());
        assert!(VolcanicEventKind::PressureWarning.is_critical());
        assert!(!VolcanicEventKind::FlowClosed.is_critical());
    }

    #[test]
    fn fingerprint_determinism() {
        let pocket1 = test_pocket();
        let pocket2 = test_pocket();
        assert_eq!(pocket1.fingerprint(), pocket2.fingerprint());

        let flow1 = test_flow();
        let flow2 = test_flow();
        assert_eq!(flow1.fingerprint(), flow2.fingerprint());
    }

    #[test]
    fn serde_magma_state() {
        let state = MagmaState::Pressurized;
        let json = serde_json::to_string(&state).unwrap();
        let recovered: MagmaState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, recovered);
    }

    #[test]
    fn serde_magma_pocket() {
        let pocket = test_pocket().with_temperature(1100.0).with_gas_content(0.3);
        let json = serde_json::to_string(&pocket).unwrap();
        let recovered: MagmaPocket = serde_json::from_str(&json).unwrap();
        assert_eq!(pocket.id, recovered.id);
        assert!((pocket.temperature - recovered.temperature).abs() < f32::EPSILON);
    }

    #[test]
    fn serde_magma_flow() {
        let flow = test_flow().with_source(FeatureId::new(5));
        let json = serde_json::to_string(&flow).unwrap();
        let recovered: MagmaFlow = serde_json::from_str(&json).unwrap();
        assert_eq!(flow.id, recovered.id);
        assert_eq!(flow.source_pocket, recovered.source_pocket);
    }

    #[test]
    fn serde_volcanic_event() {
        let event = VolcanicEvent {
            pocket_id: FeatureId::new(42),
            tick: 1000,
            kind: VolcanicEventKind::EruptionStart,
            magnitude: 0.75,
            position: (10.0, 20.0, 30.0),
        };
        let json = serde_json::to_string(&event).unwrap();
        let recovered: VolcanicEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, recovered);
    }
}
