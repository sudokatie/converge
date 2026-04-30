//! Machine tick simulation and events.

use serde::{Deserialize, Serialize};

use super::config::{AtmosphereEffect, MachineConfig, ProcessId};
use super::identity::MachineId;
use super::state::{FaultKind, MachineState};

/// Kind of event emitted during machine operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum MachineEventKind {
    /// Machine started a process.
    ProcessStarted = 0,
    /// Process completed successfully.
    ProcessCompleted = 1,
    /// Process was cancelled.
    ProcessCancelled = 2,
    /// Machine entered fault state.
    FaultOccurred = 3,
    /// Machine recovered from fault.
    FaultCleared = 4,
    /// Maintenance was performed.
    MaintenancePerformed = 5,
    /// Machine was enabled.
    Enabled = 6,
    /// Machine was disabled.
    Disabled = 7,
    /// Power was generated.
    PowerGenerated = 8,
    /// Heat was produced.
    HeatProduced = 9,
    /// Atmosphere was affected.
    AtmosphereAffected = 10,
    /// Resource was consumed.
    ResourceConsumed = 11,
    /// Resource was produced.
    ResourceProduced = 12,
}

impl MachineEventKind {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::ProcessStarted => "process_started",
            Self::ProcessCompleted => "process_completed",
            Self::ProcessCancelled => "process_cancelled",
            Self::FaultOccurred => "fault_occurred",
            Self::FaultCleared => "fault_cleared",
            Self::MaintenancePerformed => "maintenance_performed",
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
            Self::PowerGenerated => "power_generated",
            Self::HeatProduced => "heat_produced",
            Self::AtmosphereAffected => "atmosphere_affected",
            Self::ResourceConsumed => "resource_consumed",
            Self::ResourceProduced => "resource_produced",
        }
    }
}

/// An event emitted by a machine during a tick.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MachineEvent {
    /// Machine that emitted the event.
    pub machine_id: MachineId,
    /// Tick when event occurred.
    pub tick: u64,
    /// Event kind.
    pub kind: MachineEventKind,
    /// Associated process (if applicable).
    pub process_id: Option<ProcessId>,
    /// Associated fault (if applicable).
    pub fault: Option<FaultKind>,
    /// Numeric value (power, heat, resource amount).
    pub value: f32,
    /// Resource ID (for resource events).
    pub resource_id: Option<u32>,
}

impl MachineEvent {
    #[must_use]
    pub fn process_started(machine_id: MachineId, tick: u64, process_id: ProcessId) -> Self {
        Self {
            machine_id,
            tick,
            kind: MachineEventKind::ProcessStarted,
            process_id: Some(process_id),
            fault: None,
            value: 0.0,
            resource_id: None,
        }
    }

    #[must_use]
    pub fn process_completed(machine_id: MachineId, tick: u64, process_id: ProcessId) -> Self {
        Self {
            machine_id,
            tick,
            kind: MachineEventKind::ProcessCompleted,
            process_id: Some(process_id),
            fault: None,
            value: 0.0,
            resource_id: None,
        }
    }

    #[must_use]
    pub fn fault_occurred(machine_id: MachineId, tick: u64, fault: FaultKind) -> Self {
        Self {
            machine_id,
            tick,
            kind: MachineEventKind::FaultOccurred,
            process_id: None,
            fault: Some(fault),
            value: 0.0,
            resource_id: None,
        }
    }

    #[must_use]
    pub fn power_generated(machine_id: MachineId, tick: u64, amount: f32) -> Self {
        Self {
            machine_id,
            tick,
            kind: MachineEventKind::PowerGenerated,
            process_id: None,
            fault: None,
            value: amount,
            resource_id: None,
        }
    }

    #[must_use]
    pub fn heat_produced(machine_id: MachineId, tick: u64, amount: f32) -> Self {
        Self {
            machine_id,
            tick,
            kind: MachineEventKind::HeatProduced,
            process_id: None,
            fault: None,
            value: amount,
            resource_id: None,
        }
    }

    #[must_use]
    pub fn resource_produced(
        machine_id: MachineId,
        tick: u64,
        resource_id: u32,
        amount: f32,
    ) -> Self {
        Self {
            machine_id,
            tick,
            kind: MachineEventKind::ResourceProduced,
            process_id: None,
            fault: None,
            value: amount,
            resource_id: Some(resource_id),
        }
    }

    fn sort_key(&self) -> (u64, MachineId, MachineEventKind) {
        (self.tick, self.machine_id, self.kind)
    }
}

impl PartialOrd for MachineEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MachineEvent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

impl Eq for MachineEvent {}

/// Statistics from ticking machines.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MachineTickStats {
    /// Machines that were active.
    pub active_count: u32,
    /// Machines that were idle.
    pub idle_count: u32,
    /// Machines in fault state.
    pub faulted_count: u32,
    /// Total power consumed.
    pub power_consumed: f32,
    /// Total power generated.
    pub power_generated: f32,
    /// Total heat produced.
    pub heat_produced: f32,
    /// Processes completed this tick.
    pub processes_completed: u32,
}

impl MachineTickStats {
    pub fn add(&mut self, other: &MachineTickStats) {
        self.active_count += other.active_count;
        self.idle_count += other.idle_count;
        self.faulted_count += other.faulted_count;
        self.power_consumed += other.power_consumed;
        self.power_generated += other.power_generated;
        self.heat_produced += other.heat_produced;
        self.processes_completed += other.processes_completed;
    }
}

/// Result of ticking a single machine.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MachineTickResult {
    /// Events emitted this tick.
    pub events: Vec<MachineEvent>,
    /// Power consumed this tick.
    pub power_consumed: f32,
    /// Power generated this tick.
    pub power_generated: f32,
    /// Heat produced this tick.
    pub heat_produced: f32,
    /// Atmosphere effect (if life support).
    pub atmosphere_effect: Option<AtmosphereEffect>,
    /// Resource deltas as pairs of resource ID and quantity change.
    pub resource_deltas: Vec<(u32, i32)>,
    /// Whether process completed.
    pub process_completed: bool,
    /// Whether machine is now faulted.
    pub faulted: bool,
}

impl MachineTickResult {
    #[must_use]
    pub fn idle() -> Self {
        Self::default()
    }

    pub fn add_event(&mut self, event: MachineEvent) {
        self.events.push(event);
    }

    pub fn add_resource_delta(&mut self, resource_id: u32, delta: i32) {
        if let Some((_, existing)) = self
            .resource_deltas
            .iter_mut()
            .find(|(id, _)| *id == resource_id)
        {
            *existing += delta;
        } else {
            self.resource_deltas.push((resource_id, delta));
        }
    }

    pub fn sort_events(&mut self) {
        self.events.sort();
    }
}

/// Fingerprint for machine state comparison.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MachineFingerprint(pub u32);

impl MachineFingerprint {
    #[must_use]
    pub const fn value(&self) -> u32 {
        self.0
    }

    #[must_use]
    pub fn from_state(state: &MachineState) -> Self {
        Self(state.fingerprint())
    }

    #[must_use]
    pub fn matches(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

/// Tick a machine and produce results.
pub fn tick_machine(
    machine_id: MachineId,
    config: &MachineConfig,
    state: &mut MachineState,
    tick: u64,
    available_power: f32,
) -> MachineTickResult {
    if tick <= state.last_tick {
        return MachineTickResult::idle();
    }
    state.last_tick = tick;

    let mut result = MachineTickResult::default();

    if !state.enabled {
        return result;
    }

    if state.maintenance.is_overdue() && state.fault == FaultKind::None {
        state.set_fault(FaultKind::MaintenanceRequired);
        result.add_event(MachineEvent::fault_occurred(
            machine_id,
            tick,
            FaultKind::MaintenanceRequired,
        ));
        result.faulted = true;
        return result;
    }

    if state.fault.is_fault() {
        result.faulted = true;
        return result;
    }

    match config.category {
        super::identity::MachineCategory::Reactor => {
            tick_reactor(machine_id, config, state, tick, &mut result);
        }
        super::identity::MachineCategory::LifeSupport => {
            tick_life_support(
                machine_id,
                config,
                state,
                tick,
                available_power,
                &mut result,
            );
        }
        super::identity::MachineCategory::Crafting
        | super::identity::MachineCategory::Processor
        | super::identity::MachineCategory::Incubator => {
            tick_process_machine(
                machine_id,
                config,
                state,
                tick,
                available_power,
                &mut result,
            );
        }
    }

    if state.is_active() {
        state.maintenance.tick(true);
        state.total_active_ticks += 1;
    }

    result.sort_events();
    result
}

fn tick_reactor(
    machine_id: MachineId,
    config: &MachineConfig,
    state: &mut MachineState,
    tick: u64,
    result: &mut MachineTickResult,
) {
    let throttle = config.heat.throttle_factor(state.heat.current);

    if throttle <= 0.0 {
        state.set_fault(FaultKind::Overheat);
        result.add_event(MachineEvent::fault_occurred(
            machine_id,
            tick,
            FaultKind::Overheat,
        ));
        result.faulted = true;
        return;
    }

    let power_output = config.power.output * throttle * state.tier.efficiency_multiplier();
    let heat_output = config.heat.output * throttle;

    state.power.add(power_output);
    state.heat.add(heat_output);
    state.heat.remove(config.heat.dissipation);

    result.power_generated = power_output;
    result.heat_produced = heat_output;
    result.add_event(MachineEvent::power_generated(
        machine_id,
        tick,
        power_output,
    ));

    if heat_output > 0.0 {
        result.add_event(MachineEvent::heat_produced(machine_id, tick, heat_output));
    }
}

fn tick_life_support(
    machine_id: MachineId,
    config: &MachineConfig,
    state: &mut MachineState,
    tick: u64,
    available_power: f32,
    result: &mut MachineTickResult,
) {
    let power_needed = config.power.active_draw;
    let total_power = available_power + state.power.current;

    if total_power < power_needed {
        state.set_fault(FaultKind::NoPower);
        result.add_event(MachineEvent::fault_occurred(
            machine_id,
            tick,
            FaultKind::NoPower,
        ));
        result.faulted = true;
        return;
    }

    let from_buffer = state.power.remove(power_needed);
    let from_external = power_needed - from_buffer;
    result.power_consumed = from_external;

    if let Some(ref base_effect) = config.atmosphere_effect {
        let efficiency = state.tier.efficiency_multiplier();
        let scaled = base_effect.scaled(efficiency);
        result.atmosphere_effect = Some(scaled);
    }
}

fn tick_process_machine(
    machine_id: MachineId,
    config: &MachineConfig,
    state: &mut MachineState,
    tick: u64,
    available_power: f32,
    result: &mut MachineTickResult,
) {
    if state.process.is_idle()
        && !state.queue.is_empty()
        && let Some(queued) = state.queue.dequeue()
        && state.start_process(config, queued.process_id, queued.quantity)
    {
        result.add_event(MachineEvent::process_started(
            machine_id,
            tick,
            queued.process_id,
        ));
    }

    if state.process.is_idle() {
        return;
    }

    let process_id = state.process.process_id.unwrap();
    let Some(process) = config.find_process(process_id) else {
        state.process.reset();
        return;
    };

    let power_needed = process.power_per_tick + config.power.idle_draw;
    let total_power = available_power + state.power.current;

    if total_power < power_needed {
        state.set_fault(FaultKind::NoPower);
        result.add_event(MachineEvent::fault_occurred(
            machine_id,
            tick,
            FaultKind::NoPower,
        ));
        result.faulted = true;
        return;
    }

    let from_buffer = state.power.remove(power_needed);
    let from_external = power_needed - from_buffer;
    result.power_consumed = from_external;

    if process.heat_per_tick > 0.0 {
        let heat = process.heat_per_tick;
        state.heat.add(heat);
        state.heat.remove(config.heat.dissipation);
        result.heat_produced = heat;
    }

    state.process.advance(1);

    if state.process.is_complete() {
        let efficiency = state.tier.efficiency_multiplier();
        for output in &process.outputs {
            let qty = output.effective_quantity(efficiency);
            #[expect(clippy::cast_possible_wrap)]
            result.add_resource_delta(output.resource_id, qty as i32);
            #[expect(clippy::cast_precision_loss)]
            let qty_f32 = qty as f32;
            result.add_event(MachineEvent::resource_produced(
                machine_id,
                tick,
                output.resource_id,
                qty_f32,
            ));
        }

        state.total_completions += 1;
        result.process_completed = true;
        result.add_event(MachineEvent::process_completed(
            machine_id, tick, process_id,
        ));

        if state.process.remaining_quantity > 1 {
            state.process.complete_one();
        } else if config.auto_restart && !state.queue.is_empty() {
            state.process.reset();
        } else if config.auto_restart && process.continuous {
            state.process.progress = 0;
        } else {
            state.process.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine::config::{AtmosphereEffect, MachineConfig, ProcessDefinition};
    use crate::machine::identity::MachineTier;

    fn test_processor() -> MachineConfig {
        MachineConfig::processor("Test Processor", 10.0).with_process(
            ProcessDefinition::new(ProcessId::new(1), "Test Process", 10)
                .with_input(100, 1)
                .with_output(200, 2)
                .with_power(5.0),
        )
    }

    fn test_reactor() -> MachineConfig {
        MachineConfig::reactor("Test Reactor", 100.0, 50.0)
    }

    fn test_life_support() -> MachineConfig {
        MachineConfig::life_support("Test Scrubber", 20.0, AtmosphereEffect::scrubber(5.0))
    }

    #[test]
    fn tick_idle_machine() {
        let config = test_processor();
        let mut state = MachineState::new(&config, MachineTier::Basic);
        let result = tick_machine(MachineId::new(1), &config, &mut state, 1, 100.0);

        assert!(result.events.is_empty());
        assert!(!result.process_completed);
    }

    #[test]
    fn tick_active_process() {
        let config = test_processor();
        let mut state = MachineState::new(&config, MachineTier::Basic);
        state.start_process(&config, ProcessId::new(1), 1);

        for tick in 1..=10 {
            let _ = tick_machine(MachineId::new(1), &config, &mut state, tick, 100.0);
        }

        assert!(state.process.is_complete() || state.process.is_idle());
    }

    #[test]
    fn tick_process_completion() {
        let config = test_processor();
        let mut state = MachineState::new(&config, MachineTier::Basic);
        state.start_process(&config, ProcessId::new(1), 1);

        let mut completed = false;
        for tick in 1..=20 {
            let result = tick_machine(MachineId::new(1), &config, &mut state, tick, 100.0);
            if result.process_completed {
                completed = true;
                assert!(
                    result
                        .events
                        .iter()
                        .any(|e| e.kind == MachineEventKind::ProcessCompleted)
                );
                break;
            }
        }
        assert!(completed);
    }

    #[test]
    fn tick_insufficient_power() {
        let config = test_processor();
        let mut state = MachineState::new(&config, MachineTier::Basic);
        state.start_process(&config, ProcessId::new(1), 1);

        let result = tick_machine(MachineId::new(1), &config, &mut state, 1, 0.0);

        assert!(result.faulted);
        assert_eq!(state.fault, FaultKind::NoPower);
    }

    #[test]
    fn tick_reactor_power_generation() {
        let config = test_reactor();
        let mut state = MachineState::new(&config, MachineTier::Basic);

        let result = tick_machine(MachineId::new(1), &config, &mut state, 1, 0.0);

        assert!(result.power_generated > 0.0);
        assert!(result.heat_produced > 0.0);
    }

    #[test]
    fn tick_reactor_overheat() {
        let config = test_reactor();
        let mut state = MachineState::new(&config, MachineTier::Basic);
        state.heat.current = state.heat.max;

        let result = tick_machine(MachineId::new(1), &config, &mut state, 1, 0.0);

        assert!(result.faulted);
        assert_eq!(state.fault, FaultKind::Overheat);
    }

    #[test]
    fn tick_life_support_atmosphere() {
        let config = test_life_support();
        let mut state = MachineState::new(&config, MachineTier::Basic);

        let result = tick_machine(MachineId::new(1), &config, &mut state, 1, 100.0);

        assert!(result.atmosphere_effect.is_some());
        assert!(result.atmosphere_effect.unwrap().co2_scrub > 0.0);
    }

    #[test]
    fn tick_maintenance_required() {
        let mut config = test_processor();
        config.maintenance_interval = 5;
        let mut state = MachineState::new(&config, MachineTier::Basic);

        for _ in 1..=10 {
            state.maintenance.tick(true);
            if state.maintenance.is_overdue() {
                break;
            }
        }

        let result = tick_machine(MachineId::new(1), &config, &mut state, 100, 100.0);

        assert!(result.faulted);
        assert_eq!(state.fault, FaultKind::MaintenanceRequired);
    }

    #[test]
    fn tick_queue_processing() {
        let config = test_processor();
        let mut state = MachineState::new(&config, MachineTier::Basic);
        state.enqueue_process(ProcessId::new(1), 1);

        let result = tick_machine(MachineId::new(1), &config, &mut state, 1, 100.0);

        assert!(
            result
                .events
                .iter()
                .any(|e| e.kind == MachineEventKind::ProcessStarted)
        );
        assert!(state.process.is_active());
    }

    #[test]
    fn event_ordering() {
        let e1 = MachineEvent::process_started(MachineId::new(1), 1, ProcessId::new(1));
        let e2 = MachineEvent::process_started(MachineId::new(2), 1, ProcessId::new(1));
        let e3 = MachineEvent::process_started(MachineId::new(1), 2, ProcessId::new(1));

        assert!(e1 < e2);
        assert!(e1 < e3);
        assert!(e2 < e3);
    }

    #[test]
    fn fingerprint_consistency() {
        let config = test_processor();
        let state = MachineState::new(&config, MachineTier::Basic);

        let fp1 = MachineFingerprint::from_state(&state);
        let fp2 = MachineFingerprint::from_state(&state);
        assert!(fp1.matches(&fp2));
    }

    #[test]
    fn serde_machine_event() {
        let event = MachineEvent::process_completed(MachineId::new(1), 100, ProcessId::new(5));
        let json = serde_json::to_string(&event).unwrap();
        let recovered: MachineEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, recovered);
    }

    #[test]
    fn serde_tick_result() {
        let mut result = MachineTickResult {
            power_consumed: 25.0,
            ..Default::default()
        };
        result.add_event(MachineEvent::power_generated(MachineId::new(1), 1, 50.0));

        let json = serde_json::to_string(&result).unwrap();
        let recovered: MachineTickResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result, recovered);
    }
}
