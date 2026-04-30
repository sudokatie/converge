//! Geological simulation tick processing and event generation.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::config::GeologyConfig;
use super::crystal::{CrystalSeam, MineralDeposit};
use super::fault::{FaultLine, QuakeEvent};
use super::fingerprint::{FingerprintBuilder, GeologyChecksum, GeologyFingerprint};
use super::identity::FeatureId;
use super::layer::LayerStack;
use super::magma::{MagmaFlow, MagmaPocket, VolcanicEvent};
use super::state::GeologyFields;

/// Kind of geological event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum GeologyEventKind {
    /// Earthquake occurred.
    Earthquake,
    /// Volcanic eruption started.
    EruptionStart,
    /// Volcanic eruption ended.
    EruptionEnd,
    /// Crystal seam depleted.
    SeamDepleted,
    /// Mineral deposit depleted.
    DepositDepleted,
    /// Stability failure detected.
    StabilityFailure,
    /// Pressure threshold exceeded.
    PressureExceeded,
    /// Temperature threshold exceeded.
    TemperatureExceeded,
}

impl GeologyEventKind {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Earthquake => "earthquake",
            Self::EruptionStart => "eruption_start",
            Self::EruptionEnd => "eruption_end",
            Self::SeamDepleted => "seam_depleted",
            Self::DepositDepleted => "deposit_depleted",
            Self::StabilityFailure => "stability_failure",
            Self::PressureExceeded => "pressure_exceeded",
            Self::TemperatureExceeded => "temperature_exceeded",
        }
    }

    #[must_use]
    pub const fn is_critical(&self) -> bool {
        matches!(
            self,
            Self::Earthquake | Self::EruptionStart | Self::StabilityFailure
        )
    }
}

/// A geological event generated during simulation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeologyEvent {
    /// Tick when event occurred.
    pub tick: u64,
    /// Kind of event.
    pub kind: GeologyEventKind,
    /// Associated feature ID (if applicable).
    pub feature_id: Option<FeatureId>,
    /// Event position.
    pub position: (f32, f32, f32),
    /// Event magnitude/intensity.
    pub magnitude: f32,
    /// Affected radius.
    pub radius: f32,
}

impl GeologyEvent {
    /// Create a new event.
    #[must_use]
    pub fn new(
        tick: u64,
        kind: GeologyEventKind,
        position: (f32, f32, f32),
        magnitude: f32,
    ) -> Self {
        Self {
            tick,
            kind,
            feature_id: None,
            position,
            magnitude,
            radius: magnitude * 10.0,
        }
    }

    /// Set the feature ID.
    #[must_use]
    pub fn with_feature_id(mut self, id: FeatureId) -> Self {
        self.feature_id = Some(id);
        self
    }

    /// Set the affected radius.
    #[must_use]
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// Create from quake event.
    #[must_use]
    pub fn from_quake(quake: &QuakeEvent) -> Self {
        Self {
            tick: quake.tick,
            kind: GeologyEventKind::Earthquake,
            feature_id: Some(quake.fault_id),
            position: quake.epicenter,
            magnitude: quake.magnitude,
            radius: quake.affected_radius(),
        }
    }

    /// Create from volcanic event.
    #[must_use]
    pub fn from_volcanic(volcanic: &VolcanicEvent) -> Self {
        use super::magma::VolcanicEventKind as VK;
        let kind = match volcanic.kind {
            VK::EruptionStart | VK::FlowOpened | VK::FlowClosed | VK::PocketDepleted => {
                GeologyEventKind::EruptionStart
            }
            VK::EruptionEnd => GeologyEventKind::EruptionEnd,
            VK::PressureWarning => GeologyEventKind::PressureExceeded,
        };

        Self {
            tick: volcanic.tick,
            kind,
            feature_id: Some(volcanic.pocket_id),
            position: volcanic.position,
            magnitude: volcanic.magnitude,
            radius: volcanic.magnitude * 50.0,
        }
    }

    fn sort_key(&self) -> (u64, &'static str, Option<FeatureId>) {
        (self.tick, self.kind.name(), self.feature_id)
    }
}

impl PartialOrd for GeologyEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GeologyEvent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

impl Eq for GeologyEvent {}

/// Statistics from a geology simulation tick.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GeologyTickStats {
    /// Number of faults processed.
    pub faults_processed: u32,
    /// Number of magma pockets processed.
    pub magma_pockets_processed: u32,
    /// Number of magma flows processed.
    pub magma_flows_processed: u32,
    /// Number of crystal seams processed.
    pub crystal_seams_processed: u32,
    /// Number of mineral deposits processed.
    pub mineral_deposits_processed: u32,
    /// Number of fields updated.
    pub fields_updated: u32,
    /// Number of events generated.
    pub events_generated: u32,
    /// Processing time in microseconds (if measured).
    pub elapsed_us: u64,
}

impl GeologyTickStats {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn total_features_processed(&self) -> u32 {
        self.faults_processed
            + self.magma_pockets_processed
            + self.magma_flows_processed
            + self.crystal_seams_processed
            + self.mineral_deposits_processed
    }
}

/// Result of a geology simulation tick.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GeologyTickResult {
    /// Current tick number.
    pub tick: u64,
    /// Events generated during this tick.
    pub events: Vec<GeologyEvent>,
    /// Tick statistics.
    pub stats: GeologyTickStats,
    /// State checksum after tick.
    pub checksum: GeologyChecksum,
}

impl GeologyTickResult {
    #[must_use]
    pub fn new(tick: u64) -> Self {
        Self {
            tick,
            events: Vec::new(),
            stats: GeologyTickStats::new(),
            checksum: GeologyChecksum::new(tick),
        }
    }

    /// Check if any critical events occurred.
    #[must_use]
    pub fn has_critical_events(&self) -> bool {
        self.events.iter().any(|e| e.kind.is_critical())
    }

    /// Get earthquake events only.
    pub fn earthquakes(&self) -> impl Iterator<Item = &GeologyEvent> {
        self.events
            .iter()
            .filter(|e| e.kind == GeologyEventKind::Earthquake)
    }

    /// Get eruption events only.
    pub fn eruptions(&self) -> impl Iterator<Item = &GeologyEvent> {
        self.events.iter().filter(|e| {
            e.kind == GeologyEventKind::EruptionStart || e.kind == GeologyEventKind::EruptionEnd
        })
    }
}

/// Summary of geological simulation state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GeologySummary {
    /// Total number of layers.
    pub layer_count: usize,
    /// Total number of faults.
    pub fault_count: usize,
    /// Number of active faults (accumulated stress).
    pub active_faults: usize,
    /// Total number of magma pockets.
    pub magma_pocket_count: usize,
    /// Number of active magma pockets.
    pub active_magma_pockets: usize,
    /// Total number of magma flows.
    pub magma_flow_count: usize,
    /// Number of active magma flows.
    pub active_magma_flows: usize,
    /// Total number of crystal seams.
    pub crystal_seam_count: usize,
    /// Number of active crystal seams.
    pub active_crystal_seams: usize,
    /// Total number of mineral deposits.
    pub mineral_deposit_count: usize,
    /// Number of discovered mineral deposits.
    pub discovered_deposits: usize,
    /// Maximum simulated depth.
    pub max_depth: f32,
    /// Total events generated.
    pub total_events: u64,
    /// Current fingerprint.
    pub fingerprint: GeologyFingerprint,
}

impl GeologySummary {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if simulation is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.layer_count == 0
            && self.fault_count == 0
            && self.magma_pocket_count == 0
            && self.crystal_seam_count == 0
            && self.mineral_deposit_count == 0
    }
}

/// Result of projecting geological state into the future.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectionResult {
    /// Number of ticks projected.
    pub ticks_ahead: u64,
    /// Projected fault stress levels (`fault_id` -> stress).
    pub fault_stress: BTreeMap<FeatureId, f32>,
    /// Projected magma pressure levels (`pocket_id` -> pressure).
    pub magma_pressure: BTreeMap<FeatureId, f32>,
    /// Estimated earthquake probability.
    pub earthquake_probability: f32,
    /// Estimated eruption probability.
    pub eruption_probability: f32,
    /// Expected number of events.
    pub expected_events: f32,
}

impl ProjectionResult {
    #[must_use]
    pub fn new(ticks_ahead: u64) -> Self {
        Self {
            ticks_ahead,
            fault_stress: BTreeMap::new(),
            magma_pressure: BTreeMap::new(),
            earthquake_probability: 0.0,
            eruption_probability: 0.0,
            expected_events: 0.0,
        }
    }

    #[must_use]
    pub fn total_hazard_probability(&self) -> f32 {
        (1.0 - (1.0 - self.earthquake_probability) * (1.0 - self.eruption_probability))
            .clamp(0.0, 1.0)
    }
}

impl Default for ProjectionResult {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Deterministic geological simulation engine.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GeologySimulator {
    /// Configuration.
    config: GeologyConfig,
    /// Layer stack.
    layers: LayerStack,
    /// Fault lines by ID.
    faults: BTreeMap<FeatureId, FaultLine>,
    /// Magma pockets by ID.
    magma_pockets: BTreeMap<FeatureId, MagmaPocket>,
    /// Magma flows by ID.
    magma_flows: BTreeMap<FeatureId, MagmaFlow>,
    /// Crystal seams by ID.
    crystal_seams: BTreeMap<FeatureId, CrystalSeam>,
    /// Mineral deposits by ID.
    mineral_deposits: BTreeMap<FeatureId, MineralDeposit>,
    /// Field state at sample points.
    fields: BTreeMap<(i32, i32, i32), GeologyFields>,
    /// Current tick.
    current_tick: u64,
    /// Total events generated.
    total_events: u64,
    /// Next feature ID.
    next_feature_id: u64,
}

impl GeologySimulator {
    /// Create a new simulator with the given configuration.
    #[must_use]
    pub fn new(config: GeologyConfig) -> Self {
        Self {
            config,
            layers: LayerStack::new(),
            faults: BTreeMap::new(),
            magma_pockets: BTreeMap::new(),
            magma_flows: BTreeMap::new(),
            crystal_seams: BTreeMap::new(),
            mineral_deposits: BTreeMap::new(),
            fields: BTreeMap::new(),
            current_tick: 0,
            total_events: 0,
            next_feature_id: 1,
        }
    }

    /// Get current configuration.
    #[must_use]
    pub fn config(&self) -> &GeologyConfig {
        &self.config
    }

    /// Get current tick.
    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Get layer stack.
    #[must_use]
    pub fn layers(&self) -> &LayerStack {
        &self.layers
    }

    /// Get mutable layer stack.
    pub fn layers_mut(&mut self) -> &mut LayerStack {
        &mut self.layers
    }

    /// Allocate a new feature ID.
    #[must_use]
    pub fn allocate_feature_id(&mut self) -> FeatureId {
        let id = FeatureId::new(self.next_feature_id);
        self.next_feature_id += 1;
        id
    }

    /// Add a fault line.
    pub fn add_fault(&mut self, fault: FaultLine) {
        self.faults.insert(fault.id, fault);
    }

    /// Get a fault by ID.
    #[must_use]
    pub fn get_fault(&self, id: FeatureId) -> Option<&FaultLine> {
        self.faults.get(&id)
    }

    /// Get mutable fault by ID.
    pub fn get_fault_mut(&mut self, id: FeatureId) -> Option<&mut FaultLine> {
        self.faults.get_mut(&id)
    }

    /// Iterate over faults.
    pub fn faults(&self) -> impl Iterator<Item = &FaultLine> {
        self.faults.values()
    }

    /// Add a magma pocket.
    pub fn add_magma_pocket(&mut self, pocket: MagmaPocket) {
        self.magma_pockets.insert(pocket.id, pocket);
    }

    /// Get a magma pocket by ID.
    #[must_use]
    pub fn get_magma_pocket(&self, id: FeatureId) -> Option<&MagmaPocket> {
        self.magma_pockets.get(&id)
    }

    /// Iterate over magma pockets.
    pub fn magma_pockets(&self) -> impl Iterator<Item = &MagmaPocket> {
        self.magma_pockets.values()
    }

    /// Add a magma flow.
    pub fn add_magma_flow(&mut self, flow: MagmaFlow) {
        self.magma_flows.insert(flow.id, flow);
    }

    /// Iterate over magma flows.
    pub fn magma_flows(&self) -> impl Iterator<Item = &MagmaFlow> {
        self.magma_flows.values()
    }

    /// Add a crystal seam.
    pub fn add_crystal_seam(&mut self, seam: CrystalSeam) {
        self.crystal_seams.insert(seam.id, seam);
    }

    /// Iterate over crystal seams.
    pub fn crystal_seams(&self) -> impl Iterator<Item = &CrystalSeam> {
        self.crystal_seams.values()
    }

    /// Add a mineral deposit.
    pub fn add_mineral_deposit(&mut self, deposit: MineralDeposit) {
        self.mineral_deposits.insert(deposit.id, deposit);
    }

    /// Iterate over mineral deposits.
    pub fn mineral_deposits(&self) -> impl Iterator<Item = &MineralDeposit> {
        self.mineral_deposits.values()
    }

    /// Set field state at a grid position.
    pub fn set_field(&mut self, pos: (i32, i32, i32), fields: GeologyFields) {
        self.fields.insert(pos, fields);
    }

    /// Get field state at a grid position.
    #[must_use]
    pub fn get_field(&self, pos: (i32, i32, i32)) -> Option<&GeologyFields> {
        self.fields.get(&pos)
    }

    /// Execute a simulation tick.
    pub fn tick(&mut self) -> GeologyTickResult {
        self.current_tick += 1;
        let tick = self.current_tick;
        let mut result = GeologyTickResult::new(tick);

        self.tick_faults(&mut result);
        self.tick_magma(&mut result);
        self.tick_crystals(&mut result);
        self.tick_fields(&mut result);

        result.events.sort();

        result.checksum = self.compute_checksum();
        self.total_events += result.events.len() as u64;

        result
    }

    fn tick_faults(&mut self, result: &mut GeologyTickResult) {
        let config = self.config.fault.clone();
        for fault in self.faults.values_mut() {
            if let Some(quake) = fault.tick(&config, self.current_tick) {
                result.events.push(GeologyEvent::from_quake(&quake));
            }
            result.stats.faults_processed += 1;
        }
    }

    fn tick_magma(&mut self, result: &mut GeologyTickResult) {
        let config = self.config.magma.clone();
        for pocket in self.magma_pockets.values_mut() {
            if let Some(volcanic) = pocket.tick(&config, self.current_tick) {
                result.events.push(GeologyEvent::from_volcanic(&volcanic));
            }
            result.stats.magma_pockets_processed += 1;
        }

        for flow in self.magma_flows.values_mut() {
            flow.tick(&config, self.current_tick);
            result.stats.magma_flows_processed += 1;
        }
    }

    fn tick_crystals(&mut self, result: &mut GeologyTickResult) {
        let config = self.config.crystal.clone();
        for seam in self.crystal_seams.values_mut() {
            let was_active = seam.is_active();
            let temp = self.config.thermal.temperature_at_depth(seam.depth());
            let pressure = self.config.pressure_at_depth(seam.depth());
            seam.tick(&config, temp, pressure, self.current_tick);

            if was_active && !seam.is_active() {
                result.events.push(
                    GeologyEvent::new(
                        self.current_tick,
                        GeologyEventKind::SeamDepleted,
                        seam.position,
                        0.0,
                    )
                    .with_feature_id(seam.id),
                );
            }
            result.stats.crystal_seams_processed += 1;
        }

        #[allow(clippy::cast_possible_truncation)]
        let deposit_count = self.mineral_deposits.len() as u32;
        result.stats.mineral_deposits_processed = deposit_count;
    }

    fn tick_fields(&mut self, result: &mut GeologyTickResult) {
        let magma_threshold = self.config.thermal.magma_threshold;
        for ((x, y, z), fields) in &mut self.fields {
            fields.tick(self.current_tick, 1.0);

            #[allow(clippy::cast_precision_loss)]
            let pos = (*x as f32, *y as f32, *z as f32);

            if fields.stability.is_failed() {
                result.events.push(GeologyEvent::new(
                    self.current_tick,
                    GeologyEventKind::StabilityFailure,
                    pos,
                    1.0,
                ));
            }

            if fields.temperature.is_molten(magma_threshold)
                && !fields.temperature.is_molten(magma_threshold * 0.95)
            {
                result.events.push(GeologyEvent::new(
                    self.current_tick,
                    GeologyEventKind::TemperatureExceeded,
                    pos,
                    fields.temperature.temperature(),
                ));
            }

            result.stats.fields_updated += 1;
        }

        #[allow(clippy::cast_possible_truncation)]
        let event_count = result.events.len() as u32;
        result.stats.events_generated = event_count;
    }

    /// Compute current state checksum.
    #[must_use]
    pub fn compute_checksum(&self) -> GeologyChecksum {
        let mut builder = FingerprintBuilder::new();

        builder.add_layer(0, self.layers.fingerprint());

        for (id, fault) in &self.faults {
            builder.add_fault(id.raw(), fault.fingerprint());
        }

        for (id, pocket) in &self.magma_pockets {
            builder.add_magma_pocket(id.raw(), pocket.fingerprint());
        }

        for (id, flow) in &self.magma_flows {
            builder.add_magma_flow(id.raw(), flow.fingerprint());
        }

        for (id, seam) in &self.crystal_seams {
            builder.add_crystal_seam(id.raw(), seam.fingerprint());
        }

        for (id, deposit) in &self.mineral_deposits {
            builder.add_mineral_deposit(id.raw(), deposit.fingerprint());
        }

        for (pos, fields) in &self.fields {
            #[allow(clippy::cast_precision_loss)]
            let pos_f32 = (pos.0 as f32, pos.1 as f32, pos.2 as f32);
            builder.add_field(pos_f32, fields.fingerprint());
        }

        builder.build(self.current_tick)
    }

    /// Compute current fingerprint.
    #[must_use]
    pub fn fingerprint(&self) -> GeologyFingerprint {
        GeologyFingerprint::from_config(
            self.layers.count(),
            self.faults.len(),
            self.magma_pockets.len(),
            self.crystal_seams.len(),
            self.config.max_depth,
        )
    }

    /// Get summary of current state.
    #[must_use]
    pub fn summary(&self) -> GeologySummary {
        GeologySummary {
            layer_count: self.layers.count(),
            fault_count: self.faults.len(),
            active_faults: self
                .faults
                .values()
                .filter(|f| f.stress().current > 10.0)
                .count(),
            magma_pocket_count: self.magma_pockets.len(),
            active_magma_pockets: self
                .magma_pockets
                .values()
                .filter(|p| p.state().is_active())
                .count(),
            magma_flow_count: self.magma_flows.len(),
            active_magma_flows: self.magma_flows.values().filter(|f| f.is_active()).count(),
            crystal_seam_count: self.crystal_seams.len(),
            active_crystal_seams: self
                .crystal_seams
                .values()
                .filter(|s| s.is_active())
                .count(),
            mineral_deposit_count: self.mineral_deposits.len(),
            discovered_deposits: self
                .mineral_deposits
                .values()
                .filter(|d| d.is_discovered())
                .count(),
            max_depth: self.layers.max_depth(),
            total_events: self.total_events,
            fingerprint: self.fingerprint(),
        }
    }

    /// Project geological state into the future.
    #[must_use]
    pub fn project(&self, ticks_ahead: u64) -> ProjectionResult {
        let mut result = ProjectionResult::new(ticks_ahead);

        for (id, fault) in &self.faults {
            let projected_stress = fault.projected_stress(ticks_ahead, &self.config.fault);
            result.fault_stress.insert(*id, projected_stress);

            if projected_stress >= self.config.fault.major_slip_threshold {
                result.earthquake_probability = (result.earthquake_probability + 0.5).min(1.0);
            } else if projected_stress >= self.config.fault.minor_slip_threshold {
                result.earthquake_probability = (result.earthquake_probability + 0.2).min(1.0);
            }
        }

        for (id, pocket) in &self.magma_pockets {
            #[allow(clippy::cast_precision_loss)]
            let projected_pressure =
                pocket.pressure + self.config.magma.pressure_buildup_rate * ticks_ahead as f32;
            result.magma_pressure.insert(*id, projected_pressure);

            if projected_pressure >= self.config.magma.eruption_threshold {
                result.eruption_probability = (result.eruption_probability + 0.5).min(1.0);
            } else if projected_pressure >= self.config.magma.eruption_threshold * 0.8 {
                result.eruption_probability = (result.eruption_probability + 0.2).min(1.0);
            }
        }

        result.expected_events = result.earthquake_probability + result.eruption_probability;

        result
    }

    /// Validate current state.
    ///
    /// # Errors
    ///
    /// Returns an error if any geological feature is invalid:
    /// - `ValidationError::Config` if configuration is invalid
    /// - `ValidationError::Fault` if a fault has non-positive depth extent
    /// - `ValidationError::MagmaPocket` if a pocket has non-positive radius or volume
    /// - `ValidationError::CrystalSeam` if a seam has non-positive extent
    /// - `ValidationError::MineralDeposit` if a deposit has non-positive extent
    pub fn validate(&self) -> Result<(), ValidationError> {
        self.config
            .validate()
            .map_err(|_| ValidationError::Config)?;

        for fault in self.faults.values() {
            if fault.depth_extent <= 0.0 {
                return Err(ValidationError::Fault(fault.id));
            }
        }

        for pocket in self.magma_pockets.values() {
            if pocket.radius <= 0.0 || pocket.volume <= 0.0 {
                return Err(ValidationError::MagmaPocket(pocket.id));
            }
        }

        for seam in self.crystal_seams.values() {
            if seam.extent <= 0.0 {
                return Err(ValidationError::CrystalSeam(seam.id));
            }
        }

        for deposit in self.mineral_deposits.values() {
            if deposit.extent <= 0.0 {
                return Err(ValidationError::MineralDeposit(deposit.id));
            }
        }

        Ok(())
    }
}

/// Validation error for geological simulation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidationError {
    /// Configuration is invalid.
    Config,
    /// Fault has invalid definition.
    Fault(FeatureId),
    /// Magma pocket is invalid.
    MagmaPocket(FeatureId),
    /// Crystal seam is invalid.
    CrystalSeam(FeatureId),
    /// Mineral deposit is invalid.
    MineralDeposit(FeatureId),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config => write!(f, "invalid geology configuration"),
            Self::Fault(id) => write!(f, "invalid fault: {id}"),
            Self::MagmaPocket(id) => write!(f, "invalid magma pocket: {id}"),
            Self::CrystalSeam(id) => write!(f, "invalid crystal seam: {id}"),
            Self::MineralDeposit(id) => write!(f, "invalid mineral deposit: {id}"),
        }
    }
}

impl std::error::Error for ValidationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geology::{CrystalType, FaultType, MineralType};

    fn test_simulator() -> GeologySimulator {
        GeologySimulator::new(GeologyConfig::default())
    }

    #[test]
    fn simulator_creation() {
        let sim = test_simulator();
        assert_eq!(sim.current_tick(), 0);
        assert!(sim.validate().is_ok());
    }

    #[test]
    fn simulator_tick() {
        let mut sim = test_simulator();
        let result = sim.tick();
        assert_eq!(result.tick, 1);
        assert_eq!(sim.current_tick(), 1);
    }

    #[test]
    fn simulator_add_fault() {
        let mut sim = test_simulator();
        let id = sim.allocate_feature_id();
        let fault = FaultLine::new(id, FaultType::Normal, (0.0, 0.0, 10.0), (100.0, 0.0, 10.0));
        sim.add_fault(fault);

        assert!(sim.get_fault(id).is_some());
        assert_eq!(sim.faults().count(), 1);
    }

    #[test]
    fn simulator_add_magma_pocket() {
        let mut sim = test_simulator();
        let id = sim.allocate_feature_id();
        let pocket = MagmaPocket::new(id, (0.0, 0.0, 100.0), 10.0);
        sim.add_magma_pocket(pocket);

        assert!(sim.get_magma_pocket(id).is_some());
        assert_eq!(sim.magma_pockets().count(), 1);
    }

    #[test]
    fn simulator_add_crystal_seam() {
        let mut sim = test_simulator();
        let id = sim.allocate_feature_id();
        let seam = CrystalSeam::new(id, CrystalType::Quartz, (0.0, 0.0, 50.0));
        sim.add_crystal_seam(seam);

        assert_eq!(sim.crystal_seams().count(), 1);
    }

    #[test]
    fn simulator_add_mineral_deposit() {
        let mut sim = test_simulator();
        let id = sim.allocate_feature_id();
        let deposit = MineralDeposit::new(id, MineralType::Iron, (0.0, 0.0, 100.0));
        sim.add_mineral_deposit(deposit);

        assert_eq!(sim.mineral_deposits().count(), 1);
    }

    #[test]
    fn simulator_summary() {
        let mut sim = test_simulator();

        let id1 = sim.allocate_feature_id();
        sim.add_fault(FaultLine::new(
            id1,
            FaultType::Normal,
            (0.0, 0.0, 10.0),
            (100.0, 0.0, 10.0),
        ));

        let id2 = sim.allocate_feature_id();
        sim.add_magma_pocket(MagmaPocket::new(id2, (0.0, 0.0, 100.0), 10.0));

        let summary = sim.summary();
        assert_eq!(summary.fault_count, 1);
        assert_eq!(summary.magma_pocket_count, 1);
        assert!(!summary.is_empty());
    }

    #[test]
    fn simulator_projection() {
        let mut sim = test_simulator();
        let id = sim.allocate_feature_id();
        let fault = FaultLine::new(id, FaultType::Normal, (0.0, 0.0, 10.0), (100.0, 0.0, 10.0))
            .with_initial_stress(1.0);
        sim.add_fault(fault);

        let projection = sim.project(1000);
        assert!(projection.fault_stress.contains_key(&id));
        assert!(projection.fault_stress[&id] > 1.0);
    }

    #[test]
    fn simulator_checksum_deterministic() {
        let mut sim1 = test_simulator();
        let mut sim2 = test_simulator();

        let id1 = sim1.allocate_feature_id();
        let id2 = sim2.allocate_feature_id();

        sim1.add_fault(FaultLine::new(
            id1,
            FaultType::Normal,
            (0.0, 0.0, 10.0),
            (100.0, 0.0, 10.0),
        ));
        sim2.add_fault(FaultLine::new(
            id2,
            FaultType::Normal,
            (0.0, 0.0, 10.0),
            (100.0, 0.0, 10.0),
        ));

        assert!(sim1.compute_checksum().matches(&sim2.compute_checksum()));
    }

    #[test]
    fn simulator_fingerprint_deterministic() {
        let mut sim1 = test_simulator();
        let mut sim2 = test_simulator();

        let id = sim1.allocate_feature_id();
        sim1.add_fault(FaultLine::new(
            id,
            FaultType::Normal,
            (0.0, 0.0, 10.0),
            (100.0, 0.0, 10.0),
        ));

        let id = sim2.allocate_feature_id();
        sim2.add_fault(FaultLine::new(
            id,
            FaultType::Normal,
            (0.0, 0.0, 10.0),
            (100.0, 0.0, 10.0),
        ));

        assert_eq!(sim1.fingerprint(), sim2.fingerprint());
    }

    #[test]
    fn tick_result_critical_events() {
        let mut result = GeologyTickResult::new(1);
        assert!(!result.has_critical_events());

        result.events.push(GeologyEvent::new(
            1,
            GeologyEventKind::Earthquake,
            (0.0, 0.0, 0.0),
            5.0,
        ));
        assert!(result.has_critical_events());
    }

    #[test]
    fn projection_total_hazard() {
        let mut proj = ProjectionResult::new(100);
        proj.earthquake_probability = 0.3;
        proj.eruption_probability = 0.2;

        let total = proj.total_hazard_probability();
        assert!(total > 0.3);
        assert!(total < 1.0);
    }

    #[test]
    fn event_ordering() {
        let e1 = GeologyEvent::new(100, GeologyEventKind::Earthquake, (0.0, 0.0, 0.0), 5.0);
        let e2 = GeologyEvent::new(101, GeologyEventKind::Earthquake, (0.0, 0.0, 0.0), 4.0);
        let e3 = GeologyEvent::new(100, GeologyEventKind::EruptionStart, (0.0, 0.0, 0.0), 3.0);

        assert!(e1 < e2);
        assert!(e1 < e3);
    }

    #[test]
    fn serde_geology_event() {
        let event = GeologyEvent::new(100, GeologyEventKind::Earthquake, (1.0, 2.0, 3.0), 5.5)
            .with_feature_id(FeatureId::new(42));
        let json = serde_json::to_string(&event).unwrap();
        let recovered: GeologyEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, recovered);
    }

    #[test]
    fn serde_tick_result() {
        let result = GeologyTickResult::new(100);
        let json = serde_json::to_string(&result).unwrap();
        let recovered: GeologyTickResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result.tick, recovered.tick);
    }

    #[test]
    fn serde_projection_result() {
        let mut proj = ProjectionResult::new(100);
        proj.earthquake_probability = 0.5;
        let json = serde_json::to_string(&proj).unwrap();
        let recovered: ProjectionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(proj.ticks_ahead, recovered.ticks_ahead);
    }

    #[test]
    fn serde_simulator() {
        let mut sim = test_simulator();
        let id = sim.allocate_feature_id();
        sim.add_fault(FaultLine::new(
            id,
            FaultType::Normal,
            (0.0, 0.0, 10.0),
            (100.0, 0.0, 10.0),
        ));

        let json = serde_json::to_string(&sim).unwrap();
        let recovered: GeologySimulator = serde_json::from_str(&json).unwrap();
        assert_eq!(sim.faults.len(), recovered.faults.len());
    }
}
