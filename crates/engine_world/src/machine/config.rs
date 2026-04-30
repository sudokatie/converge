//! Machine configuration and process definitions.

use serde::{Deserialize, Serialize};

use super::identity::{MachineCategory, MachineTier};

/// Unique identifier for a process/recipe definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProcessId(pub u32);

impl ProcessId {
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn raw(&self) -> u32 {
        self.0
    }
}

/// Direction of a fluid/resource port.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum PortDirection {
    #[default]
    Input = 0,
    Output = 1,
    Bidirectional = 2,
}

impl PortDirection {
    #[must_use]
    pub const fn accepts_input(&self) -> bool {
        matches!(self, Self::Input | Self::Bidirectional)
    }

    #[must_use]
    pub const fn provides_output(&self) -> bool {
        matches!(self, Self::Output | Self::Bidirectional)
    }
}

/// A fluid connection port on a machine.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FluidPort {
    /// Port index (0-based).
    pub index: u8,
    /// Port direction.
    pub direction: PortDirection,
    /// Fluid kind filter (None = any fluid).
    pub fluid_filter: Option<u16>,
    /// Maximum flow rate per tick.
    pub max_flow: f32,
    /// Tank capacity.
    pub capacity: f32,
}

impl FluidPort {
    #[must_use]
    pub fn input(index: u8, capacity: f32, max_flow: f32) -> Self {
        Self {
            index,
            direction: PortDirection::Input,
            fluid_filter: None,
            max_flow,
            capacity,
        }
    }

    #[must_use]
    pub fn output(index: u8, capacity: f32, max_flow: f32) -> Self {
        Self {
            index,
            direction: PortDirection::Output,
            fluid_filter: None,
            max_flow,
            capacity,
        }
    }

    #[must_use]
    pub fn with_filter(mut self, fluid_kind: u16) -> Self {
        self.fluid_filter = Some(fluid_kind);
        self
    }
}

/// Resource requirement for a process input.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResourceRequirement {
    /// Resource/item type identifier.
    pub resource_id: u32,
    /// Quantity required.
    pub quantity: u32,
    /// Whether this is consumed (vs catalyst).
    pub consumed: bool,
}

impl ResourceRequirement {
    #[must_use]
    pub const fn new(resource_id: u32, quantity: u32) -> Self {
        Self {
            resource_id,
            quantity,
            consumed: true,
        }
    }

    #[must_use]
    pub const fn catalyst(resource_id: u32, quantity: u32) -> Self {
        Self {
            resource_id,
            quantity,
            consumed: false,
        }
    }
}

/// Resource yield from a process output.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResourceYield {
    /// Resource/item type identifier.
    pub resource_id: u32,
    /// Base quantity produced.
    pub quantity: u32,
    /// Probability of producing this output (0.0-1.0).
    pub probability: f32,
    /// Bonus quantity from efficiency (added to base).
    pub efficiency_bonus: u32,
}

impl ResourceYield {
    #[must_use]
    pub fn new(resource_id: u32, quantity: u32) -> Self {
        Self {
            resource_id,
            quantity,
            probability: 1.0,
            efficiency_bonus: 0,
        }
    }

    #[must_use]
    pub fn with_probability(mut self, prob: f32) -> Self {
        self.probability = prob.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub const fn with_efficiency_bonus(mut self, bonus: u32) -> Self {
        self.efficiency_bonus = bonus;
        self
    }

    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    pub fn effective_quantity(&self, efficiency: f32) -> u32 {
        let bonus = (self.efficiency_bonus as f32 * (efficiency - 1.0)).max(0.0);
        self.quantity + bonus as u32
    }
}

/// Power configuration for a machine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PowerConfig {
    /// Power draw when idle (units per tick).
    pub idle_draw: f32,
    /// Power draw when active (units per tick).
    pub active_draw: f32,
    /// Power output when generating (units per tick).
    pub output: f32,
    /// Internal power buffer capacity.
    pub buffer_capacity: f32,
}

impl PowerConfig {
    #[must_use]
    pub fn consumer(idle: f32, active: f32) -> Self {
        Self {
            idle_draw: idle,
            active_draw: active,
            output: 0.0,
            buffer_capacity: active * 10.0,
        }
    }

    #[must_use]
    pub fn generator(output: f32) -> Self {
        Self {
            idle_draw: 0.0,
            active_draw: 0.0,
            output,
            buffer_capacity: output * 10.0,
        }
    }

    #[must_use]
    pub const fn with_buffer(mut self, capacity: f32) -> Self {
        self.buffer_capacity = capacity;
        self
    }
}

/// Heat configuration for a machine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HeatConfig {
    /// Heat produced when active (units per tick).
    pub output: f32,
    /// Maximum operating temperature.
    pub max_temp: f32,
    /// Temperature at which machine throttles.
    pub throttle_temp: f32,
    /// Passive heat dissipation rate.
    pub dissipation: f32,
}

impl HeatConfig {
    #[must_use]
    pub fn producer(output: f32, max_temp: f32) -> Self {
        Self {
            output,
            max_temp,
            throttle_temp: max_temp * 0.8,
            dissipation: output * 0.1,
        }
    }

    #[must_use]
    pub const fn with_throttle(mut self, temp: f32) -> Self {
        self.throttle_temp = temp;
        self
    }

    #[must_use]
    pub fn throttle_factor(&self, current_temp: f32) -> f32 {
        if current_temp < self.throttle_temp {
            1.0
        } else if current_temp >= self.max_temp {
            0.0
        } else {
            let range = self.max_temp - self.throttle_temp;
            if range <= 0.0 {
                0.0
            } else {
                1.0 - (current_temp - self.throttle_temp) / range
            }
        }
    }
}

/// Atmosphere effect produced by life support machines.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AtmosphereEffect {
    /// Oxygen production/consumption rate (positive = produce).
    pub oxygen_delta: f32,
    /// CO2 scrubbing rate (positive = remove CO2).
    pub co2_scrub: f32,
    /// Pressure adjustment rate.
    pub pressure_delta: f32,
    /// Temperature adjustment rate.
    pub temp_delta: f32,
    /// Humidity adjustment rate.
    pub humidity_delta: f32,
}

impl Default for AtmosphereEffect {
    fn default() -> Self {
        Self {
            oxygen_delta: 0.0,
            co2_scrub: 0.0,
            pressure_delta: 0.0,
            temp_delta: 0.0,
            humidity_delta: 0.0,
        }
    }
}

impl AtmosphereEffect {
    #[must_use]
    pub fn scrubber(co2_rate: f32) -> Self {
        Self {
            co2_scrub: co2_rate,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn oxygenator(oxygen_rate: f32) -> Self {
        Self {
            oxygen_delta: oxygen_rate,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn pressurizer(pressure_rate: f32) -> Self {
        Self {
            pressure_delta: pressure_rate,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn hvac(temp_rate: f32) -> Self {
        Self {
            temp_delta: temp_rate,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn scaled(&self, factor: f32) -> Self {
        Self {
            oxygen_delta: self.oxygen_delta * factor,
            co2_scrub: self.co2_scrub * factor,
            pressure_delta: self.pressure_delta * factor,
            temp_delta: self.temp_delta * factor,
            humidity_delta: self.humidity_delta * factor,
        }
    }
}

/// Definition of a process/recipe that a machine can execute.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProcessDefinition {
    /// Unique process identifier.
    pub id: ProcessId,
    /// Human-readable name.
    pub name: String,
    /// Duration in ticks at base speed.
    pub duration: u32,
    /// Input requirements.
    pub inputs: Vec<ResourceRequirement>,
    /// Output yields.
    pub outputs: Vec<ResourceYield>,
    /// Fluid input requirements: port index, fluid kind, amount.
    pub fluid_inputs: Vec<(u8, u16, f32)>,
    /// Fluid output yields: port index, fluid kind, amount.
    pub fluid_outputs: Vec<(u8, u16, f32)>,
    /// Power required per tick during process.
    pub power_per_tick: f32,
    /// Heat produced per tick during process.
    pub heat_per_tick: f32,
    /// Minimum tier required to execute.
    pub min_tier: MachineTier,
    /// Whether this process runs continuously (vs discrete).
    pub continuous: bool,
}

impl ProcessDefinition {
    #[must_use]
    pub fn new(id: ProcessId, name: impl Into<String>, duration: u32) -> Self {
        Self {
            id,
            name: name.into(),
            duration,
            inputs: Vec::new(),
            outputs: Vec::new(),
            fluid_inputs: Vec::new(),
            fluid_outputs: Vec::new(),
            power_per_tick: 0.0,
            heat_per_tick: 0.0,
            min_tier: MachineTier::Basic,
            continuous: false,
        }
    }

    #[must_use]
    pub fn continuous(id: ProcessId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            duration: 1,
            inputs: Vec::new(),
            outputs: Vec::new(),
            fluid_inputs: Vec::new(),
            fluid_outputs: Vec::new(),
            power_per_tick: 0.0,
            heat_per_tick: 0.0,
            min_tier: MachineTier::Basic,
            continuous: true,
        }
    }

    #[must_use]
    pub fn with_input(mut self, resource_id: u32, quantity: u32) -> Self {
        self.inputs
            .push(ResourceRequirement::new(resource_id, quantity));
        self
    }

    #[must_use]
    pub fn with_catalyst(mut self, resource_id: u32, quantity: u32) -> Self {
        self.inputs
            .push(ResourceRequirement::catalyst(resource_id, quantity));
        self
    }

    #[must_use]
    pub fn with_output(mut self, resource_id: u32, quantity: u32) -> Self {
        self.outputs.push(ResourceYield::new(resource_id, quantity));
        self
    }

    #[must_use]
    pub fn with_fluid_input(mut self, port: u8, fluid_kind: u16, amount: f32) -> Self {
        self.fluid_inputs.push((port, fluid_kind, amount));
        self
    }

    #[must_use]
    pub fn with_fluid_output(mut self, port: u8, fluid_kind: u16, amount: f32) -> Self {
        self.fluid_outputs.push((port, fluid_kind, amount));
        self
    }

    #[must_use]
    pub const fn with_power(mut self, power: f32) -> Self {
        self.power_per_tick = power;
        self
    }

    #[must_use]
    pub const fn with_heat(mut self, heat: f32) -> Self {
        self.heat_per_tick = heat;
        self
    }

    #[must_use]
    pub const fn with_min_tier(mut self, tier: MachineTier) -> Self {
        self.min_tier = tier;
        self
    }

    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    pub fn effective_duration(&self, tier: MachineTier) -> u32 {
        let speed = tier.speed_multiplier();
        let duration = (self.duration as f32 / speed).ceil() as u32;
        duration.max(1)
    }
}

/// Complete configuration for a machine type.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MachineConfig {
    /// Machine type name.
    pub name: String,
    /// Machine category.
    pub category: MachineCategory,
    /// Available processes/recipes.
    pub processes: Vec<ProcessDefinition>,
    /// Fluid ports.
    pub fluid_ports: Vec<FluidPort>,
    /// Power configuration.
    pub power: PowerConfig,
    /// Heat configuration.
    pub heat: HeatConfig,
    /// Atmosphere effects (for life support).
    pub atmosphere_effect: Option<AtmosphereEffect>,
    /// Input slot capacity.
    pub input_slots: u8,
    /// Output slot capacity.
    pub output_slots: u8,
    /// Process queue capacity (0 = no queue).
    pub queue_capacity: u8,
    /// Ticks between required maintenance.
    pub maintenance_interval: u32,
    /// Whether the machine auto-restarts after completion.
    pub auto_restart: bool,
}

impl MachineConfig {
    #[must_use]
    pub fn crafting(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            category: MachineCategory::Crafting,
            processes: Vec::new(),
            fluid_ports: Vec::new(),
            power: PowerConfig::default(),
            heat: HeatConfig::default(),
            atmosphere_effect: None,
            input_slots: 9,
            output_slots: 1,
            queue_capacity: 5,
            maintenance_interval: 0,
            auto_restart: false,
        }
    }

    #[must_use]
    pub fn processor(name: impl Into<String>, power_draw: f32) -> Self {
        Self {
            name: name.into(),
            category: MachineCategory::Processor,
            processes: Vec::new(),
            fluid_ports: Vec::new(),
            power: PowerConfig::consumer(power_draw * 0.1, power_draw),
            heat: HeatConfig::default(),
            atmosphere_effect: None,
            input_slots: 4,
            output_slots: 4,
            queue_capacity: 10,
            maintenance_interval: 10000,
            auto_restart: true,
        }
    }

    #[must_use]
    pub fn reactor(name: impl Into<String>, power_output: f32, heat_output: f32) -> Self {
        Self {
            name: name.into(),
            category: MachineCategory::Reactor,
            processes: Vec::new(),
            fluid_ports: Vec::new(),
            power: PowerConfig::generator(power_output),
            heat: HeatConfig::producer(heat_output, 1000.0),
            atmosphere_effect: None,
            input_slots: 1,
            output_slots: 1,
            queue_capacity: 0,
            maintenance_interval: 5000,
            auto_restart: true,
        }
    }

    #[must_use]
    pub fn incubator(name: impl Into<String>, power_draw: f32) -> Self {
        Self {
            name: name.into(),
            category: MachineCategory::Incubator,
            processes: Vec::new(),
            fluid_ports: Vec::new(),
            power: PowerConfig::consumer(power_draw * 0.2, power_draw),
            heat: HeatConfig::default(),
            atmosphere_effect: None,
            input_slots: 1,
            output_slots: 4,
            queue_capacity: 0,
            maintenance_interval: 20000,
            auto_restart: false,
        }
    }

    #[must_use]
    pub fn life_support(
        name: impl Into<String>,
        power_draw: f32,
        effect: AtmosphereEffect,
    ) -> Self {
        Self {
            name: name.into(),
            category: MachineCategory::LifeSupport,
            processes: Vec::new(),
            fluid_ports: Vec::new(),
            power: PowerConfig::consumer(power_draw * 0.5, power_draw),
            heat: HeatConfig::default(),
            atmosphere_effect: Some(effect),
            input_slots: 0,
            output_slots: 0,
            queue_capacity: 0,
            maintenance_interval: 15000,
            auto_restart: true,
        }
    }

    #[must_use]
    pub fn with_process(mut self, process: ProcessDefinition) -> Self {
        self.processes.push(process);
        self
    }

    #[must_use]
    pub fn with_fluid_port(mut self, port: FluidPort) -> Self {
        self.fluid_ports.push(port);
        self
    }

    #[must_use]
    pub const fn with_heat(mut self, heat: HeatConfig) -> Self {
        self.heat = heat;
        self
    }

    #[must_use]
    pub fn find_process(&self, id: ProcessId) -> Option<&ProcessDefinition> {
        self.processes.iter().find(|p| p.id == id)
    }

    #[must_use]
    pub fn process_count(&self) -> usize {
        self.processes.len()
    }

    pub fn fingerprint(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(self.name.as_bytes());
        hasher.update(&[self.category as u8]);
        #[expect(clippy::cast_possible_truncation)]
        let proc_count = self.processes.len() as u32;
        hasher.update(&proc_count.to_le_bytes());
        for proc in &self.processes {
            hasher.update(&proc.id.0.to_le_bytes());
            hasher.update(&proc.duration.to_le_bytes());
        }
        hasher.finalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_definition_builder() {
        let proc = ProcessDefinition::new(ProcessId::new(1), "Smelt Iron", 100)
            .with_input(10, 2)
            .with_output(20, 1)
            .with_power(5.0)
            .with_heat(2.0);

        assert_eq!(proc.inputs.len(), 1);
        assert_eq!(proc.outputs.len(), 1);
        assert!((proc.power_per_tick - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn process_effective_duration() {
        let proc = ProcessDefinition::new(ProcessId::new(1), "Test", 100);
        assert_eq!(proc.effective_duration(MachineTier::Basic), 100);
        assert_eq!(proc.effective_duration(MachineTier::Elite), 50);
    }

    #[test]
    fn resource_yield_efficiency() {
        let yield_ = ResourceYield::new(1, 10).with_efficiency_bonus(5);
        assert_eq!(yield_.effective_quantity(1.0), 10);
        assert_eq!(yield_.effective_quantity(1.5), 12);
        assert_eq!(yield_.effective_quantity(2.0), 15);
    }

    #[test]
    fn heat_throttle_factor() {
        let heat = HeatConfig::producer(10.0, 100.0);
        assert!((heat.throttle_factor(50.0) - 1.0).abs() < f32::EPSILON);
        assert!((heat.throttle_factor(90.0) - 0.5).abs() < f32::EPSILON);
        assert!((heat.throttle_factor(100.0) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn atmosphere_effect_scaled() {
        let effect = AtmosphereEffect::oxygenator(10.0);
        let scaled = effect.scaled(0.5);
        assert!((scaled.oxygen_delta - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn machine_config_crafting() {
        let config = MachineConfig::crafting("Workbench").with_process(ProcessDefinition::new(
            ProcessId::new(1),
            "Craft Plank",
            20,
        ));

        assert_eq!(config.category, MachineCategory::Crafting);
        assert_eq!(config.process_count(), 1);
    }

    #[test]
    fn machine_config_reactor() {
        let config = MachineConfig::reactor("Fusion Reactor", 1000.0, 500.0);
        assert_eq!(config.category, MachineCategory::Reactor);
        assert!((config.power.output - 1000.0).abs() < f32::EPSILON);
        assert!((config.heat.output - 500.0).abs() < f32::EPSILON);
    }

    #[test]
    fn config_fingerprint_deterministic() {
        let c1 = MachineConfig::processor("Smelter", 50.0);
        let c2 = MachineConfig::processor("Smelter", 50.0);
        assert_eq!(c1.fingerprint(), c2.fingerprint());
    }

    #[test]
    fn serde_process_definition() {
        let proc = ProcessDefinition::new(ProcessId::new(1), "Test", 100)
            .with_input(10, 5)
            .with_output(20, 2);
        let json = serde_json::to_string(&proc).unwrap();
        let recovered: ProcessDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(proc, recovered);
    }

    #[test]
    fn serde_machine_config() {
        let config =
            MachineConfig::life_support("O2 Generator", 25.0, AtmosphereEffect::oxygenator(5.0));
        let json = serde_json::to_string(&config).unwrap();
        let recovered: MachineConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, recovered);
    }

    #[test]
    fn fluid_port_directions() {
        let input = FluidPort::input(0, 100.0, 10.0);
        assert!(input.direction.accepts_input());
        assert!(!input.direction.provides_output());

        let output = FluidPort::output(1, 100.0, 10.0);
        assert!(!output.direction.accepts_input());
        assert!(output.direction.provides_output());
    }
}
