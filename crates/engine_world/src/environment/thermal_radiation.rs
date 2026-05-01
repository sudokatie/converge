//! Thermal radiation simulation with blackbody-like heat exchange.
//!
//! This module provides deterministic thermal radiation simulation between
//! cells based on the Stefan-Boltzmann law for blackbody radiation.
//!
//! # Physics Model
//!
//! Heat exchange via radiation follows the Stefan-Boltzmann law:
//! - Power emitted: P = ε * σ * A * T^4
//! - Net exchange: Q = ε * σ * A * (`T_hot`^4 - `T_cold`^4)
//!
//! Where ε is emissivity (0-1), σ is the Stefan-Boltzmann constant,
//! A is surface area, and T is absolute temperature (Kelvin).

use std::collections::BTreeMap;

use engine_core::coords::{ChunkPos, LocalPos};
use serde::{Deserialize, Serialize};

use crate::replay::{ChecksumBuilder, StepChecksum};

/// Stefan-Boltzmann constant (W / m^2 / K^4).
pub const STEFAN_BOLTZMANN: f64 = 5.670_374e-8;

/// Absolute zero offset (Celsius to Kelvin).
pub const KELVIN_OFFSET: f32 = 273.15;

/// Minimum valid temperature in Celsius.
pub const MIN_TEMPERATURE: f32 = -273.0;

/// Maximum valid temperature in Celsius (plasma range).
pub const MAX_TEMPERATURE: f32 = 10000.0;

/// Minimum valid emissivity.
pub const MIN_EMISSIVITY: f32 = 0.0;

/// Maximum valid emissivity (perfect blackbody).
pub const MAX_EMISSIVITY: f32 = 1.0;

/// Default ambient temperature in Celsius.
pub const DEFAULT_AMBIENT: f32 = 20.0;

/// Default thermal mass (J/K equivalent units).
pub const DEFAULT_THERMAL_MASS: f32 = 1.0;

/// A thermal cell representing a region with temperature and emissivity.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThermalCell {
    /// Temperature in Celsius.
    temperature: f32,
    /// Emissivity factor (0.0 = reflective, 1.0 = perfect blackbody).
    emissivity: f32,
    /// Thermal mass / heat capacity factor.
    thermal_mass: f32,
    /// Surface area factor for radiation.
    surface_area: f32,
}

impl ThermalCell {
    /// Create a new thermal cell with validation.
    #[must_use]
    pub fn new(temperature: f32, emissivity: f32, thermal_mass: f32, surface_area: f32) -> Self {
        Self {
            temperature: temperature.clamp(MIN_TEMPERATURE, MAX_TEMPERATURE),
            emissivity: emissivity.clamp(MIN_EMISSIVITY, MAX_EMISSIVITY),
            thermal_mass: thermal_mass.max(0.001),
            surface_area: surface_area.max(0.0),
        }
    }

    /// Create a cell at ambient temperature with default properties.
    #[must_use]
    pub fn ambient() -> Self {
        Self::new(DEFAULT_AMBIENT, 0.9, DEFAULT_THERMAL_MASS, 1.0)
    }

    /// Create a hot cell (e.g., heat source).
    #[must_use]
    pub fn hot(temperature: f32) -> Self {
        Self::new(temperature, 0.95, DEFAULT_THERMAL_MASS, 1.0)
    }

    /// Create a cold cell (e.g., heat sink).
    #[must_use]
    pub fn cold(temperature: f32) -> Self {
        Self::new(temperature, 0.9, DEFAULT_THERMAL_MASS, 1.0)
    }

    /// Get temperature in Celsius.
    #[must_use]
    pub const fn temperature(&self) -> f32 {
        self.temperature
    }

    /// Get temperature in Kelvin.
    #[must_use]
    pub fn temperature_kelvin(&self) -> f32 {
        self.temperature + KELVIN_OFFSET
    }

    /// Get emissivity factor.
    #[must_use]
    pub const fn emissivity(&self) -> f32 {
        self.emissivity
    }

    /// Get thermal mass.
    #[must_use]
    pub const fn thermal_mass(&self) -> f32 {
        self.thermal_mass
    }

    /// Get surface area factor.
    #[must_use]
    pub const fn surface_area(&self) -> f32 {
        self.surface_area
    }

    /// Set temperature with clamping.
    pub fn set_temperature(&mut self, temp: f32) {
        self.temperature = temp.clamp(MIN_TEMPERATURE, MAX_TEMPERATURE);
    }

    /// Apply a temperature delta with clamping.
    pub fn apply_delta(&mut self, delta: f32) {
        self.set_temperature(self.temperature + delta);
    }

    /// Compute radiated power (W) using Stefan-Boltzmann law.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "f64 result cast to f32 for game units"
    )]
    pub fn radiated_power(&self) -> f32 {
        let t_kelvin = f64::from(self.temperature_kelvin().max(0.0));
        let power = f64::from(self.emissivity)
            * STEFAN_BOLTZMANN
            * f64::from(self.surface_area)
            * t_kelvin.powi(4);
        power as f32
    }

    /// Check if this cell is at or above a temperature threshold.
    #[must_use]
    pub fn is_hot(&self, threshold: f32) -> bool {
        self.temperature >= threshold
    }

    /// Check if this cell is at or below a temperature threshold.
    #[must_use]
    pub fn is_cold(&self, threshold: f32) -> bool {
        self.temperature <= threshold
    }
}

impl Default for ThermalCell {
    fn default() -> Self {
        Self::ambient()
    }
}

/// Unique position identifier for thermal cells across chunks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ThermalPos {
    /// Chunk position.
    pub chunk: ChunkPos,
    /// Local position within chunk.
    pub local: LocalPos,
}

impl PartialOrd for ThermalPos {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ThermalPos {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

impl ThermalPos {
    /// Create a new thermal position.
    #[must_use]
    pub const fn new(chunk: ChunkPos, local: LocalPos) -> Self {
        Self { chunk, local }
    }

    /// Compute sort key for deterministic ordering.
    #[must_use]
    pub fn sort_key(&self) -> (i32, i32, i32, usize) {
        (
            self.chunk.x(),
            self.chunk.y(),
            self.chunk.z(),
            self.local.to_index(),
        )
    }

    /// Get 6-face neighbors with chunk boundary handling.
    #[must_use]
    #[expect(
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        reason = "local coords 0..16 and offsets -1..1 stay bounded"
    )]
    pub fn face_neighbors(&self) -> [Self; 6] {
        const OFFSETS: [(i32, i32, i32); 6] = [
            (-1, 0, 0),
            (1, 0, 0),
            (0, -1, 0),
            (0, 1, 0),
            (0, 0, -1),
            (0, 0, 1),
        ];

        let mut neighbors = [*self; 6];

        for (i, &(dx, dy, dz)) in OFFSETS.iter().enumerate() {
            let nx = self.local.x() as i32 + dx;
            let ny = self.local.y() as i32 + dy;
            let nz = self.local.z() as i32 + dz;

            let (chunk_offset_x, local_x) = if nx < 0 {
                (-1, 15u32)
            } else if nx >= 16 {
                (1, 0u32)
            } else {
                (0, nx as u32)
            };

            let (chunk_offset_y, local_y) = if ny < 0 {
                (-1, 15u32)
            } else if ny >= 16 {
                (1, 0u32)
            } else {
                (0, ny as u32)
            };

            let (chunk_offset_z, local_z) = if nz < 0 {
                (-1, 15u32)
            } else if nz >= 16 {
                (1, 0u32)
            } else {
                (0, nz as u32)
            };

            neighbors[i] = Self {
                chunk: ChunkPos::new(
                    self.chunk.x() + chunk_offset_x,
                    self.chunk.y() + chunk_offset_y,
                    self.chunk.z() + chunk_offset_z,
                ),
                local: LocalPos::new(local_x, local_y, local_z),
            };
        }

        neighbors
    }
}

/// A thermal entry combining position and cell data.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThermalEntry {
    /// Chunk position.
    pub chunk: ChunkPos,
    /// Local position.
    pub local: LocalPos,
    /// Thermal cell data.
    pub cell: ThermalCell,
}

impl ThermalEntry {
    /// Create a new thermal entry.
    #[must_use]
    pub fn new(chunk: ChunkPos, local: LocalPos, cell: ThermalCell) -> Self {
        Self { chunk, local, cell }
    }

    /// Get position as [`ThermalPos`].
    #[must_use]
    pub const fn pos(&self) -> ThermalPos {
        ThermalPos::new(self.chunk, self.local)
    }
}

/// A planned radiation heat exchange between two cells.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RadiationExchange {
    /// Source (hotter) position.
    pub source: ThermalPos,
    /// Destination (cooler) position.
    pub dest: ThermalPos,
    /// Heat transfer rate (W).
    pub transfer_rate: f32,
    /// Source temperature at planning time.
    pub source_temp: f32,
    /// Destination temperature at planning time.
    pub dest_temp: f32,
    /// View factor between surfaces (0-1).
    pub view_factor: f32,
}

impl RadiationExchange {
    /// Create a new radiation exchange.
    #[must_use]
    pub fn new(
        source: ThermalPos,
        dest: ThermalPos,
        transfer_rate: f32,
        source_temp: f32,
        dest_temp: f32,
        view_factor: f32,
    ) -> Self {
        Self {
            source,
            dest,
            transfer_rate: transfer_rate.max(0.0),
            source_temp,
            dest_temp,
            view_factor: view_factor.clamp(0.0, 1.0),
        }
    }

    /// Temperature differential.
    #[must_use]
    pub fn temp_diff(&self) -> f32 {
        self.source_temp - self.dest_temp
    }

    /// Whether this exchange crosses chunk boundaries.
    #[must_use]
    pub fn is_cross_chunk(&self) -> bool {
        self.source.chunk != self.dest.chunk
    }

    /// Compute energy transferred over dt seconds (J).
    #[must_use]
    pub fn energy_for_dt(&self, dt: f32) -> f32 {
        self.transfer_rate * dt
    }

    /// Sort key for deterministic ordering.
    #[must_use]
    pub fn sort_key(&self) -> (i32, i32, i32, usize, i32, i32, i32, usize) {
        (
            self.source.chunk.x(),
            self.source.chunk.y(),
            self.source.chunk.z(),
            self.source.local.to_index(),
            self.dest.chunk.x(),
            self.dest.chunk.y(),
            self.dest.chunk.z(),
            self.dest.local.to_index(),
        )
    }
}

/// Configuration for thermal radiation simulation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThermalRadiationConfig {
    /// Minimum temperature differential for radiation exchange.
    pub min_temp_diff: f32,
    /// Global scaling factor for radiation rate.
    pub radiation_scale: f32,
    /// Default view factor between adjacent cells.
    pub default_view_factor: f32,
    /// Enable cross-chunk radiation.
    pub cross_chunk_enabled: bool,
    /// Maximum transfer rate per exchange (W).
    pub max_transfer_rate: f32,
    /// Ambient temperature for boundary conditions.
    pub ambient_temperature: f32,
    /// Rate of equilibration with ambient.
    pub ambient_exchange_rate: f32,
}

impl ThermalRadiationConfig {
    /// Default configuration.
    pub const DEFAULT: Self = Self {
        min_temp_diff: 1.0,
        radiation_scale: 1.0,
        default_view_factor: 0.5,
        cross_chunk_enabled: true,
        max_transfer_rate: 1000.0,
        ambient_temperature: DEFAULT_AMBIENT,
        ambient_exchange_rate: 0.01,
    };

    /// Configuration for high-temperature environments (lava, furnaces).
    pub const HIGH_TEMP: Self = Self {
        min_temp_diff: 10.0,
        radiation_scale: 2.0,
        default_view_factor: 0.6,
        cross_chunk_enabled: true,
        max_transfer_rate: 10000.0,
        ambient_temperature: 50.0,
        ambient_exchange_rate: 0.005,
    };

    /// Configuration for space/vacuum (no convection, pure radiation).
    pub const SPACE: Self = Self {
        min_temp_diff: 0.1,
        radiation_scale: 1.5,
        default_view_factor: 0.8,
        cross_chunk_enabled: true,
        max_transfer_rate: 5000.0,
        ambient_temperature: -270.0,
        ambient_exchange_rate: 0.0,
    };

    /// Validate configuration values.
    #[must_use]
    pub fn validate(&self) -> ThermalRadiationValidation {
        let mut issues = Vec::new();

        if self.min_temp_diff < 0.0 {
            issues.push("min_temp_diff must be non-negative".to_string());
        }
        if self.radiation_scale <= 0.0 {
            issues.push("radiation_scale must be positive".to_string());
        }
        if !(0.0..=1.0).contains(&self.default_view_factor) {
            issues.push("default_view_factor must be in [0, 1]".to_string());
        }
        if self.max_transfer_rate <= 0.0 {
            issues.push("max_transfer_rate must be positive".to_string());
        }
        if self.ambient_temperature < MIN_TEMPERATURE || self.ambient_temperature > MAX_TEMPERATURE
        {
            issues.push(format!(
                "ambient_temperature must be in [{MIN_TEMPERATURE}, {MAX_TEMPERATURE}]"
            ));
        }
        if self.ambient_exchange_rate < 0.0 || self.ambient_exchange_rate > 1.0 {
            issues.push("ambient_exchange_rate must be in [0, 1]".to_string());
        }

        ThermalRadiationValidation {
            is_valid: issues.is_empty(),
            issues,
        }
    }
}

impl Default for ThermalRadiationConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Validation result for configuration or region state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThermalRadiationValidation {
    /// Whether validation passed.
    pub is_valid: bool,
    /// List of validation issues.
    pub issues: Vec<String>,
}

/// A region of thermal cells for radiation simulation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThermalRadiationRegion {
    /// Cells indexed by position.
    #[serde(
        serialize_with = "serialize_cells",
        deserialize_with = "deserialize_cells"
    )]
    cells: BTreeMap<ThermalPos, ThermalCell>,
    /// Total thermal energy estimate.
    total_energy: f64,
    /// Average temperature.
    avg_temperature: f32,
}

fn serialize_cells<S>(
    cells: &BTreeMap<ThermalPos, ThermalCell>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeSeq;
    let mut seq = serializer.serialize_seq(Some(cells.len()))?;
    for (pos, cell) in cells {
        seq.serialize_element(&ThermalEntry::new(pos.chunk, pos.local, *cell))?;
    }
    seq.end()
}

fn deserialize_cells<'de, D>(deserializer: D) -> Result<BTreeMap<ThermalPos, ThermalCell>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let entries: Vec<ThermalEntry> = Deserialize::deserialize(deserializer)?;
    let mut map = BTreeMap::new();
    for entry in entries {
        map.insert(entry.pos(), entry.cell);
    }
    Ok(map)
}

impl Default for ThermalRadiationRegion {
    fn default() -> Self {
        Self::new()
    }
}

impl ThermalRadiationRegion {
    /// Create an empty region.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cells: BTreeMap::new(),
            total_energy: 0.0,
            avg_temperature: DEFAULT_AMBIENT,
        }
    }

    /// Number of cells.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Whether region is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Get a cell by position.
    #[must_use]
    pub fn get(&self, pos: ThermalPos) -> Option<&ThermalCell> {
        self.cells.get(&pos)
    }

    /// Get a mutable cell by position.
    pub fn get_mut(&mut self, pos: ThermalPos) -> Option<&mut ThermalCell> {
        self.cells.get_mut(&pos)
    }

    /// Insert or update a cell.
    pub fn insert(&mut self, pos: ThermalPos, cell: ThermalCell) {
        self.cells.insert(pos, cell);
        self.recompute_stats();
    }

    /// Insert a thermal entry.
    pub fn insert_entry(&mut self, entry: ThermalEntry) {
        self.insert(entry.pos(), entry.cell);
    }

    /// Remove a cell.
    pub fn remove(&mut self, pos: ThermalPos) -> Option<ThermalCell> {
        let removed = self.cells.remove(&pos);
        if removed.is_some() {
            self.recompute_stats();
        }
        removed
    }

    /// Iterate over cells in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = (&ThermalPos, &ThermalCell)> {
        self.cells.iter()
    }

    /// Iterate mutably.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&ThermalPos, &mut ThermalCell)> {
        self.cells.iter_mut()
    }

    /// Get all positions.
    pub fn positions(&self) -> impl Iterator<Item = &ThermalPos> {
        self.cells.keys()
    }

    /// Get average temperature.
    #[must_use]
    pub fn avg_temperature(&self) -> f32 {
        self.avg_temperature
    }

    /// Get total energy estimate.
    #[must_use]
    pub fn total_energy(&self) -> f64 {
        self.total_energy
    }

    /// Get active chunks.
    #[must_use]
    pub fn active_chunks(&self) -> Vec<ChunkPos> {
        let mut chunks: Vec<ChunkPos> = self.cells.keys().map(|p| p.chunk).collect();
        chunks.dedup();
        chunks
    }

    /// Count cells in a specific chunk.
    #[must_use]
    pub fn count_in_chunk(&self, chunk: ChunkPos) -> usize {
        self.cells
            .range(
                ThermalPos::new(chunk, LocalPos::new(0, 0, 0))
                    ..=ThermalPos::new(chunk, LocalPos::new(15, 15, 15)),
            )
            .count()
    }

    /// Compute a summary of the region.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "cell count fits in f32 for reasonable region sizes"
    )]
    pub fn summary(&self) -> ThermalRadiationSummary {
        const HOT_THRESHOLD: f32 = 100.0;
        const COLD_THRESHOLD: f32 = 0.0;

        if self.cells.is_empty() {
            return ThermalRadiationSummary {
                cell_count: 0,
                chunk_count: 0,
                min_temperature: DEFAULT_AMBIENT,
                max_temperature: DEFAULT_AMBIENT,
                avg_temperature: DEFAULT_AMBIENT,
                total_radiated_power: 0.0,
                hot_cells: 0,
                cold_cells: 0,
            };
        }

        let mut min_temp = f32::MAX;
        let mut max_temp = f32::MIN;
        let mut total_temp = 0.0f32;
        let mut total_power = 0.0f32;
        let mut hot_count = 0usize;
        let mut cold_count = 0usize;

        for cell in self.cells.values() {
            min_temp = min_temp.min(cell.temperature());
            max_temp = max_temp.max(cell.temperature());
            total_temp += cell.temperature();
            total_power += cell.radiated_power();

            if cell.is_hot(HOT_THRESHOLD) {
                hot_count += 1;
            }
            if cell.is_cold(COLD_THRESHOLD) {
                cold_count += 1;
            }
        }

        let count = self.cells.len();

        ThermalRadiationSummary {
            cell_count: count,
            chunk_count: self.active_chunks().len(),
            min_temperature: min_temp,
            max_temperature: max_temp,
            avg_temperature: total_temp / count as f32,
            total_radiated_power: total_power,
            hot_cells: hot_count,
            cold_cells: cold_count,
        }
    }

    /// Validate region state.
    #[must_use]
    pub fn validate(&self) -> ThermalRadiationValidation {
        let mut issues = Vec::new();

        for (pos, cell) in &self.cells {
            if cell.temperature() < MIN_TEMPERATURE || cell.temperature() > MAX_TEMPERATURE {
                issues.push(format!(
                    "Cell at {:?} has invalid temperature {}",
                    pos,
                    cell.temperature()
                ));
            }
            if cell.emissivity() < MIN_EMISSIVITY || cell.emissivity() > MAX_EMISSIVITY {
                issues.push(format!(
                    "Cell at {:?} has invalid emissivity {}",
                    pos,
                    cell.emissivity()
                ));
            }
            if cell.thermal_mass() <= 0.0 {
                issues.push(format!(
                    "Cell at {:?} has invalid thermal_mass {}",
                    pos,
                    cell.thermal_mass()
                ));
            }
        }

        ThermalRadiationValidation {
            is_valid: issues.is_empty(),
            issues,
        }
    }

    /// Compute checksum for determinism verification.
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "cell count fits in u32")]
    pub fn checksum(&self) -> StepChecksum {
        let mut builder = ChecksumBuilder::new();
        builder.feed_u32(self.cells.len() as u32);
        for (pos, cell) in &self.cells {
            builder.feed_i32(pos.chunk.x());
            builder.feed_i32(pos.chunk.y());
            builder.feed_i32(pos.chunk.z());
            builder.feed_u32(pos.local.to_index() as u32);
            builder.feed_f32(cell.temperature());
            builder.feed_f32(cell.emissivity());
            builder.feed_f32(cell.thermal_mass());
            builder.feed_f32(cell.surface_area());
        }
        builder.build()
    }

    /// Compute compact fingerprint.
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "cell count fits in u32")]
    pub fn fingerprint(&self) -> ThermalRadiationFingerprint {
        let checksum = self.checksum();
        ThermalRadiationFingerprint {
            cell_count: self.cells.len() as u32,
            avg_temp_bits: self.avg_temperature.to_bits(),
            checksum: checksum.value(),
        }
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "cell count fits in f64 for reasonable region sizes"
    )]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "average temperature from f64 to f32 is acceptable precision"
    )]
    fn recompute_stats(&mut self) {
        if self.cells.is_empty() {
            self.total_energy = 0.0;
            self.avg_temperature = DEFAULT_AMBIENT;
            return;
        }

        let mut total_temp = 0.0f64;
        let mut total_energy = 0.0f64;

        for cell in self.cells.values() {
            total_temp += f64::from(cell.temperature());
            total_energy += f64::from(cell.temperature_kelvin()) * f64::from(cell.thermal_mass());
        }

        self.avg_temperature = (total_temp / self.cells.len() as f64) as f32;
        self.total_energy = total_energy;
    }
}

/// Summary statistics for a thermal radiation region.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThermalRadiationSummary {
    /// Number of cells.
    pub cell_count: usize,
    /// Number of active chunks.
    pub chunk_count: usize,
    /// Minimum temperature.
    pub min_temperature: f32,
    /// Maximum temperature.
    pub max_temperature: f32,
    /// Average temperature.
    pub avg_temperature: f32,
    /// Total radiated power (W).
    pub total_radiated_power: f32,
    /// Count of cells above hot threshold.
    pub hot_cells: usize,
    /// Count of cells below cold threshold.
    pub cold_cells: usize,
}

/// Compact fingerprint for a thermal region.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ThermalRadiationFingerprint {
    /// Cell count.
    pub cell_count: u32,
    /// Average temperature as bits.
    pub avg_temp_bits: u32,
    /// Checksum value.
    pub checksum: u32,
}

/// Result of a thermal radiation simulation step.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ThermalRadiationResult {
    /// Planned radiation exchanges.
    pub exchanges: Vec<RadiationExchange>,
    /// Total energy transferred (J).
    pub energy_transferred: f32,
    /// Number of cells updated.
    pub cells_updated: u32,
    /// Number of cross-chunk exchanges.
    pub cross_chunk_exchanges: u32,
    /// Maximum temperature change in any cell.
    pub max_temp_change: f32,
}

impl ThermalRadiationResult {
    /// Create an empty result.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether any changes occurred.
    #[must_use]
    pub fn has_changes(&self) -> bool {
        self.cells_updated > 0 || !self.exchanges.is_empty()
    }

    /// Compute checksum for determinism verification.
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "counts fit in u32")]
    pub fn checksum(&self) -> StepChecksum {
        let mut builder = ChecksumBuilder::new();
        builder.feed_u32(self.exchanges.len() as u32);
        for ex in &self.exchanges {
            builder.feed_i32(ex.source.chunk.x());
            builder.feed_i32(ex.source.chunk.y());
            builder.feed_i32(ex.source.chunk.z());
            builder.feed_u32(ex.source.local.to_index() as u32);
            builder.feed_i32(ex.dest.chunk.x());
            builder.feed_i32(ex.dest.chunk.y());
            builder.feed_i32(ex.dest.chunk.z());
            builder.feed_u32(ex.dest.local.to_index() as u32);
            builder.feed_f32(ex.transfer_rate);
        }
        builder.feed_f32(self.energy_transferred);
        builder.feed_u32(self.cells_updated);
        builder.feed_f32(self.max_temp_change);
        builder.build()
    }
}

/// Compute net radiation heat transfer between two cells.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "f64 physics result cast to f32 for game units"
)]
pub fn compute_radiation_transfer(
    source: &ThermalCell,
    dest: &ThermalCell,
    view_factor: f32,
    scale: f32,
) -> f32 {
    let t_source = f64::from(source.temperature_kelvin().max(0.0));
    let t_dest = f64::from(dest.temperature_kelvin().max(0.0));

    let effective_emissivity = f64::from(source.emissivity() * dest.emissivity());
    let effective_area = f64::from(source.surface_area().min(dest.surface_area()));

    let power = effective_emissivity
        * STEFAN_BOLTZMANN
        * effective_area
        * f64::from(view_factor)
        * (t_source.powi(4) - t_dest.powi(4));

    (power * f64::from(scale)) as f32
}

/// Plan radiation exchanges for a region.
#[must_use]
pub fn plan_radiation_exchanges(
    region: &ThermalRadiationRegion,
    config: &ThermalRadiationConfig,
) -> Vec<RadiationExchange> {
    let mut exchanges = Vec::new();
    let mut processed = std::collections::HashSet::new();

    for (pos, cell) in region.iter() {
        let neighbors = pos.face_neighbors();

        for neighbor_pos in &neighbors {
            if !config.cross_chunk_enabled && pos.chunk != neighbor_pos.chunk {
                continue;
            }

            let pair_key = if *pos < *neighbor_pos {
                (*pos, *neighbor_pos)
            } else {
                (*neighbor_pos, *pos)
            };
            if processed.contains(&pair_key) {
                continue;
            }
            processed.insert(pair_key);

            let Some(neighbor_cell) = region.get(*neighbor_pos) else {
                continue;
            };

            let temp_diff = (cell.temperature() - neighbor_cell.temperature()).abs();
            if temp_diff < config.min_temp_diff {
                continue;
            }

            let (source_pos, source_cell, dest_pos, dest_cell) =
                if cell.temperature() > neighbor_cell.temperature() {
                    (*pos, cell, *neighbor_pos, neighbor_cell)
                } else {
                    (*neighbor_pos, neighbor_cell, *pos, cell)
                };

            let transfer_rate = compute_radiation_transfer(
                source_cell,
                dest_cell,
                config.default_view_factor,
                config.radiation_scale,
            )
            .min(config.max_transfer_rate);

            if transfer_rate > 0.001 {
                exchanges.push(RadiationExchange::new(
                    source_pos,
                    dest_pos,
                    transfer_rate,
                    source_cell.temperature(),
                    dest_cell.temperature(),
                    config.default_view_factor,
                ));
            }
        }
    }

    exchanges.sort_by_key(RadiationExchange::sort_key);
    exchanges
}

/// Apply radiation exchanges to a region.
pub fn apply_radiation_exchanges(
    region: &mut ThermalRadiationRegion,
    exchanges: &[RadiationExchange],
    dt: f32,
) -> (f32, u32, u32, f32) {
    let mut total_energy = 0.0f32;
    let mut cells_updated = 0u32;
    let mut cross_chunk_count = 0u32;
    let mut max_change = 0.0f32;

    for exchange in exchanges {
        let energy = exchange.energy_for_dt(dt);
        if energy <= 0.0 {
            continue;
        }

        let source_delta;
        let dest_delta;

        {
            let Some(source_cell) = region.get(exchange.source) else {
                continue;
            };
            let Some(dest_cell) = region.get(exchange.dest) else {
                continue;
            };

            source_delta = -energy / source_cell.thermal_mass();
            dest_delta = energy / dest_cell.thermal_mass();
        }

        if let Some(cell) = region.get_mut(exchange.source) {
            cell.apply_delta(source_delta);
            max_change = max_change.max(source_delta.abs());
        }

        if let Some(cell) = region.get_mut(exchange.dest) {
            cell.apply_delta(dest_delta);
            max_change = max_change.max(dest_delta.abs());
        }

        total_energy += energy;
        cells_updated += 2;

        if exchange.is_cross_chunk() {
            cross_chunk_count += 1;
        }
    }

    region.recompute_stats();

    (total_energy, cells_updated, cross_chunk_count, max_change)
}

/// Apply ambient temperature equilibration.
pub fn apply_ambient_exchange(
    region: &mut ThermalRadiationRegion,
    config: &ThermalRadiationConfig,
    dt: f32,
) {
    if config.ambient_exchange_rate <= 0.0 {
        return;
    }

    let rate = (config.ambient_exchange_rate * dt).clamp(0.0, 1.0);

    for (_, cell) in region.iter_mut() {
        let diff = config.ambient_temperature - cell.temperature();
        cell.apply_delta(diff * rate);
    }

    region.recompute_stats();
}

/// Execute a complete thermal radiation simulation step.
#[must_use]
pub fn thermal_radiation_step(
    region: &mut ThermalRadiationRegion,
    config: &ThermalRadiationConfig,
    dt: f32,
) -> ThermalRadiationResult {
    let exchanges = plan_radiation_exchanges(region, config);

    let (energy_transferred, cells_updated, cross_chunk_exchanges, max_temp_change) =
        apply_radiation_exchanges(region, &exchanges, dt);

    apply_ambient_exchange(region, config, dt);

    ThermalRadiationResult {
        exchanges,
        energy_transferred,
        cells_updated,
        cross_chunk_exchanges,
        max_temp_change,
    }
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    clippy::similar_names,
    reason = "tests check exact values; similar names are intentional for pairs"
)]
mod tests {
    use super::*;

    fn make_pos(cx: i32, cy: i32, cz: i32, lx: u32, ly: u32, lz: u32) -> ThermalPos {
        ThermalPos::new(ChunkPos::new(cx, cy, cz), LocalPos::new(lx, ly, lz))
    }

    fn make_cell(temp: f32, emissivity: f32) -> ThermalCell {
        ThermalCell::new(temp, emissivity, DEFAULT_THERMAL_MASS, 1.0)
    }

    #[test]
    fn cell_creation() {
        let cell = ThermalCell::new(100.0, 0.9, 1.0, 1.0);
        assert!((cell.temperature() - 100.0).abs() < 0.001);
        assert!((cell.emissivity() - 0.9).abs() < 0.001);
        assert!((cell.thermal_mass() - 1.0).abs() < 0.001);
        assert!((cell.surface_area() - 1.0).abs() < 0.001);
    }

    #[test]
    fn cell_clamping() {
        let cell = ThermalCell::new(-500.0, 2.0, -1.0, -5.0);
        assert!((cell.temperature() - MIN_TEMPERATURE).abs() < 0.001);
        assert!((cell.emissivity() - MAX_EMISSIVITY).abs() < 0.001);
        assert!(cell.thermal_mass() > 0.0);
        assert!((cell.surface_area() - 0.0).abs() < 0.001);
    }

    #[test]
    fn cell_temperature_kelvin() {
        let cell = ThermalCell::new(0.0, 0.9, 1.0, 1.0);
        assert!((cell.temperature_kelvin() - KELVIN_OFFSET).abs() < 0.001);

        let cold = ThermalCell::new(-273.0, 0.9, 1.0, 1.0);
        assert!(cold.temperature_kelvin() < 1.0);
    }

    #[test]
    fn cell_radiated_power() {
        let cell = ThermalCell::new(1000.0, 1.0, 1.0, 1.0);
        let power = cell.radiated_power();
        assert!(power > 0.0);

        let cold = ThermalCell::new(0.0, 1.0, 1.0, 1.0);
        let cold_power = cold.radiated_power();

        assert!(power > cold_power);
    }

    #[test]
    fn cell_apply_delta() {
        let mut cell = ThermalCell::new(50.0, 0.9, 1.0, 1.0);
        cell.apply_delta(10.0);
        assert!((cell.temperature() - 60.0).abs() < 0.001);

        cell.apply_delta(-100.0);
        assert!((cell.temperature() + 40.0).abs() < 0.001);
    }

    #[test]
    fn cell_apply_delta_clamped() {
        let mut cell = ThermalCell::new(9990.0, 0.9, 1.0, 1.0);
        cell.apply_delta(100.0);
        assert!((cell.temperature() - MAX_TEMPERATURE).abs() < 0.001);
    }

    #[test]
    fn cell_hot_cold_predicates() {
        let hot = ThermalCell::hot(500.0);
        let cold = ThermalCell::cold(-100.0);

        assert!(hot.is_hot(100.0));
        assert!(!cold.is_hot(100.0));
        assert!(cold.is_cold(0.0));
        assert!(!hot.is_cold(0.0));
    }

    #[test]
    fn pos_ordering() {
        let p1 = make_pos(0, 0, 0, 0, 0, 0);
        let p2 = make_pos(0, 0, 0, 1, 0, 0);
        let p3 = make_pos(1, 0, 0, 0, 0, 0);

        assert!(p1 < p2);
        assert!(p2 < p3);
    }

    #[test]
    fn pos_face_neighbors_inside() {
        let pos = make_pos(0, 0, 0, 8, 8, 8);
        let neighbors = pos.face_neighbors();

        assert_eq!(neighbors[0], make_pos(0, 0, 0, 7, 8, 8));
        assert_eq!(neighbors[1], make_pos(0, 0, 0, 9, 8, 8));
        assert_eq!(neighbors[2], make_pos(0, 0, 0, 8, 7, 8));
        assert_eq!(neighbors[3], make_pos(0, 0, 0, 8, 9, 8));
        assert_eq!(neighbors[4], make_pos(0, 0, 0, 8, 8, 7));
        assert_eq!(neighbors[5], make_pos(0, 0, 0, 8, 8, 9));
    }

    #[test]
    fn pos_face_neighbors_cross_chunk() {
        let pos = make_pos(0, 0, 0, 0, 0, 0);
        let neighbors = pos.face_neighbors();

        assert_eq!(neighbors[0], make_pos(-1, 0, 0, 15, 0, 0));
        assert_eq!(neighbors[2], make_pos(0, -1, 0, 0, 15, 0));
        assert_eq!(neighbors[4], make_pos(0, 0, -1, 0, 0, 15));
    }

    #[test]
    fn radiation_exchange_creation() {
        let ex = RadiationExchange::new(
            make_pos(0, 0, 0, 0, 0, 0),
            make_pos(0, 0, 0, 1, 0, 0),
            100.0,
            500.0,
            100.0,
            0.5,
        );
        assert!((ex.temp_diff() - 400.0).abs() < 0.001);
        assert!(!ex.is_cross_chunk());
        assert!((ex.energy_for_dt(0.1) - 10.0).abs() < 0.001);
    }

    #[test]
    fn radiation_exchange_cross_chunk() {
        let ex = RadiationExchange::new(
            make_pos(0, 0, 0, 0, 0, 0),
            make_pos(1, 0, 0, 0, 0, 0),
            100.0,
            500.0,
            100.0,
            0.5,
        );
        assert!(ex.is_cross_chunk());
    }

    #[test]
    fn config_defaults() {
        let config = ThermalRadiationConfig::DEFAULT;
        assert!(config.min_temp_diff > 0.0);
        assert!(config.radiation_scale > 0.0);
        assert!(config.validate().is_valid);
    }

    #[test]
    fn config_presets() {
        assert!(ThermalRadiationConfig::HIGH_TEMP.validate().is_valid);
        assert!(ThermalRadiationConfig::SPACE.validate().is_valid);
    }

    #[test]
    fn config_validation() {
        let mut config = ThermalRadiationConfig::DEFAULT;
        config.min_temp_diff = -1.0;
        let validation = config.validate();
        assert!(!validation.is_valid);
        assert!(!validation.issues.is_empty());
    }

    #[test]
    fn region_insert_get() {
        let mut region = ThermalRadiationRegion::new();
        let pos = make_pos(0, 0, 0, 8, 8, 8);
        let cell = make_cell(100.0, 0.9);

        region.insert(pos, cell);
        assert_eq!(region.len(), 1);
        assert!(!region.is_empty());

        let retrieved = region.get(pos).unwrap();
        assert!((retrieved.temperature() - 100.0).abs() < 0.001);
    }

    #[test]
    fn region_remove() {
        let mut region = ThermalRadiationRegion::new();
        let pos = make_pos(0, 0, 0, 8, 8, 8);
        region.insert(pos, make_cell(100.0, 0.9));

        let removed = region.remove(pos);
        assert!(removed.is_some());
        assert!(region.is_empty());
    }

    #[test]
    fn region_stats() {
        let mut region = ThermalRadiationRegion::new();
        region.insert(make_pos(0, 0, 0, 0, 0, 0), make_cell(100.0, 0.9));
        region.insert(make_pos(0, 0, 0, 1, 0, 0), make_cell(200.0, 0.9));

        let avg = region.avg_temperature();
        assert!((avg - 150.0).abs() < 0.001);
    }

    #[test]
    fn region_summary() {
        let mut region = ThermalRadiationRegion::new();
        region.insert(make_pos(0, 0, 0, 0, 0, 0), make_cell(50.0, 0.9));
        region.insert(make_pos(0, 0, 0, 1, 0, 0), make_cell(150.0, 0.9));

        let summary = region.summary();
        assert_eq!(summary.cell_count, 2);
        assert!((summary.min_temperature - 50.0).abs() < 0.001);
        assert!((summary.max_temperature - 150.0).abs() < 0.001);
        assert!((summary.avg_temperature - 100.0).abs() < 0.001);
        assert_eq!(summary.hot_cells, 1);
    }

    #[test]
    fn region_validation() {
        let mut region = ThermalRadiationRegion::new();
        region.insert(make_pos(0, 0, 0, 0, 0, 0), make_cell(100.0, 0.9));
        let validation = region.validate();
        assert!(validation.is_valid);
    }

    #[test]
    fn region_checksum_deterministic() {
        let mut r1 = ThermalRadiationRegion::new();
        r1.insert(make_pos(0, 0, 0, 0, 0, 0), make_cell(100.0, 0.9));
        r1.insert(make_pos(0, 0, 0, 1, 0, 0), make_cell(200.0, 0.8));

        let mut r2 = ThermalRadiationRegion::new();
        r2.insert(make_pos(0, 0, 0, 1, 0, 0), make_cell(200.0, 0.8));
        r2.insert(make_pos(0, 0, 0, 0, 0, 0), make_cell(100.0, 0.9));

        assert_eq!(r1.checksum(), r2.checksum());
    }

    #[test]
    fn region_checksum_differs() {
        let mut r1 = ThermalRadiationRegion::new();
        r1.insert(make_pos(0, 0, 0, 0, 0, 0), make_cell(100.0, 0.9));

        let mut r2 = ThermalRadiationRegion::new();
        r2.insert(make_pos(0, 0, 0, 0, 0, 0), make_cell(101.0, 0.9));

        assert_ne!(r1.checksum(), r2.checksum());
    }

    #[test]
    fn region_fingerprint() {
        let mut region = ThermalRadiationRegion::new();
        region.insert(make_pos(0, 0, 0, 0, 0, 0), make_cell(100.0, 0.9));

        let fp = region.fingerprint();
        assert_eq!(fp.cell_count, 1);
        assert_ne!(fp.checksum, 0);
    }

    #[test]
    fn compute_radiation_transfer_basic() {
        let hot = make_cell(1000.0, 0.9);
        let cold = make_cell(20.0, 0.9);

        let transfer = compute_radiation_transfer(&hot, &cold, 0.5, 1.0);
        assert!(transfer > 0.0);

        let reverse = compute_radiation_transfer(&cold, &hot, 0.5, 1.0);
        assert!(reverse < 0.0);
    }

    #[test]
    fn compute_radiation_transfer_symmetric() {
        let hot = make_cell(500.0, 0.9);
        let cold = make_cell(100.0, 0.9);

        let forward = compute_radiation_transfer(&hot, &cold, 0.5, 1.0);
        let backward = compute_radiation_transfer(&cold, &hot, 0.5, 1.0);

        assert!((forward + backward).abs() < 0.001);
    }

    #[test]
    fn plan_radiation_exchanges_basic() {
        let mut region = ThermalRadiationRegion::new();
        region.insert(make_pos(0, 0, 0, 8, 8, 8), make_cell(500.0, 0.9));
        region.insert(make_pos(0, 0, 0, 9, 8, 8), make_cell(100.0, 0.9));

        let config = ThermalRadiationConfig::DEFAULT;
        let exchanges = plan_radiation_exchanges(&region, &config);

        assert!(!exchanges.is_empty());
        let ex = &exchanges[0];
        assert!(ex.transfer_rate > 0.0);
        assert!(ex.source_temp > ex.dest_temp);
    }

    #[test]
    fn plan_radiation_exchanges_deterministic() {
        let mut region = ThermalRadiationRegion::new();
        region.insert(make_pos(0, 0, 0, 5, 5, 5), make_cell(500.0, 0.9));
        region.insert(make_pos(0, 0, 0, 6, 5, 5), make_cell(100.0, 0.9));
        region.insert(make_pos(0, 0, 0, 5, 6, 5), make_cell(300.0, 0.9));

        let config = ThermalRadiationConfig::DEFAULT;
        let ex1 = plan_radiation_exchanges(&region, &config);
        let ex2 = plan_radiation_exchanges(&region, &config);

        assert_eq!(ex1.len(), ex2.len());
        for (e1, e2) in ex1.iter().zip(ex2.iter()) {
            assert_eq!(e1.source, e2.source);
            assert_eq!(e1.dest, e2.dest);
            assert!((e1.transfer_rate - e2.transfer_rate).abs() < 0.001);
        }
    }

    #[test]
    fn apply_radiation_exchanges_basic() {
        let mut region = ThermalRadiationRegion::new();
        region.insert(make_pos(0, 0, 0, 8, 8, 8), make_cell(500.0, 0.9));
        region.insert(make_pos(0, 0, 0, 9, 8, 8), make_cell(100.0, 0.9));

        let exchanges = vec![RadiationExchange::new(
            make_pos(0, 0, 0, 8, 8, 8),
            make_pos(0, 0, 0, 9, 8, 8),
            100.0,
            500.0,
            100.0,
            0.5,
        )];

        let (energy, cells, _, max_change) =
            apply_radiation_exchanges(&mut region, &exchanges, 1.0);

        assert!(energy > 0.0);
        assert_eq!(cells, 2);
        assert!(max_change > 0.0);

        let hot_after = region.get(make_pos(0, 0, 0, 8, 8, 8)).unwrap();
        let cold_after = region.get(make_pos(0, 0, 0, 9, 8, 8)).unwrap();

        assert!(hot_after.temperature() < 500.0);
        assert!(cold_after.temperature() > 100.0);
    }

    #[test]
    fn apply_ambient_exchange() {
        let mut region = ThermalRadiationRegion::new();
        region.insert(make_pos(0, 0, 0, 0, 0, 0), make_cell(100.0, 0.9));

        let config = ThermalRadiationConfig {
            ambient_temperature: 20.0,
            ambient_exchange_rate: 0.1,
            ..ThermalRadiationConfig::DEFAULT
        };

        super::apply_ambient_exchange(&mut region, &config, 1.0);

        let cell = region.get(make_pos(0, 0, 0, 0, 0, 0)).unwrap();
        assert!(cell.temperature() < 100.0);
        assert!(cell.temperature() > 20.0);
    }

    #[test]
    fn thermal_radiation_step_integration() {
        let mut region = ThermalRadiationRegion::new();
        region.insert(make_pos(0, 0, 0, 8, 8, 8), make_cell(500.0, 0.9));
        region.insert(make_pos(0, 0, 0, 9, 8, 8), make_cell(100.0, 0.9));

        let config = ThermalRadiationConfig::DEFAULT;
        let result = thermal_radiation_step(&mut region, &config, 0.1);

        assert!(result.has_changes());
        assert!(result.energy_transferred > 0.0);
        assert!(result.cells_updated > 0);
    }

    #[test]
    fn thermal_radiation_step_deterministic() {
        let make_region = || {
            let mut region = ThermalRadiationRegion::new();
            region.insert(make_pos(0, 0, 0, 8, 8, 8), make_cell(500.0, 0.9));
            region.insert(make_pos(0, 0, 0, 9, 8, 8), make_cell(100.0, 0.9));
            region.insert(make_pos(0, 0, 0, 8, 9, 8), make_cell(300.0, 0.9));
            region
        };

        let config = ThermalRadiationConfig::DEFAULT;

        let mut r1 = make_region();
        let res1 = thermal_radiation_step(&mut r1, &config, 0.1);

        let mut r2 = make_region();
        let res2 = thermal_radiation_step(&mut r2, &config, 0.1);

        assert_eq!(r1.checksum(), r2.checksum());
        assert_eq!(res1.checksum(), res2.checksum());
    }

    #[test]
    fn cross_chunk_disabled() {
        let mut region = ThermalRadiationRegion::new();
        region.insert(make_pos(0, 0, 0, 0, 0, 0), make_cell(500.0, 0.9));
        region.insert(make_pos(-1, 0, 0, 15, 0, 0), make_cell(100.0, 0.9));

        let mut config = ThermalRadiationConfig::DEFAULT;
        config.cross_chunk_enabled = false;

        let exchanges = plan_radiation_exchanges(&region, &config);
        assert!(exchanges.is_empty());
    }

    #[test]
    fn serde_cell_round_trip() {
        let cell = make_cell(150.0, 0.85);
        let json = serde_json::to_string(&cell).unwrap();
        let recovered: ThermalCell = serde_json::from_str(&json).unwrap();
        assert!((recovered.temperature() - cell.temperature()).abs() < 0.001);
        assert!((recovered.emissivity() - cell.emissivity()).abs() < 0.001);
    }

    #[test]
    fn serde_region_round_trip() {
        let mut region = ThermalRadiationRegion::new();
        region.insert(make_pos(0, 0, 0, 0, 0, 0), make_cell(100.0, 0.9));
        region.insert(make_pos(0, 0, 0, 1, 0, 0), make_cell(200.0, 0.8));

        let json = serde_json::to_string(&region).unwrap();
        let recovered: ThermalRadiationRegion = serde_json::from_str(&json).unwrap();

        assert_eq!(recovered.len(), region.len());
        assert_eq!(recovered.checksum(), region.checksum());
    }

    #[test]
    fn serde_config_round_trip() {
        let config = ThermalRadiationConfig::HIGH_TEMP;
        let json = serde_json::to_string(&config).unwrap();
        let recovered: ThermalRadiationConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, config);
    }

    #[test]
    fn serde_result_round_trip() {
        let result = ThermalRadiationResult {
            exchanges: vec![RadiationExchange::new(
                make_pos(0, 0, 0, 0, 0, 0),
                make_pos(0, 0, 0, 1, 0, 0),
                100.0,
                500.0,
                100.0,
                0.5,
            )],
            energy_transferred: 50.0,
            cells_updated: 2,
            cross_chunk_exchanges: 0,
            max_temp_change: 10.0,
        };

        let json = serde_json::to_string(&result).unwrap();
        let recovered: ThermalRadiationResult = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.exchanges.len(), result.exchanges.len());
        assert!((recovered.energy_transferred - result.energy_transferred).abs() < 0.001);
    }

    #[test]
    fn empty_region_operations() {
        let region = ThermalRadiationRegion::new();
        assert!(region.is_empty());
        assert_eq!(region.len(), 0);
        assert!(region.active_chunks().is_empty());

        let summary = region.summary();
        assert_eq!(summary.cell_count, 0);

        let validation = region.validate();
        assert!(validation.is_valid);
    }

    #[test]
    fn min_temp_diff_threshold() {
        let mut region = ThermalRadiationRegion::new();
        region.insert(make_pos(0, 0, 0, 8, 8, 8), make_cell(100.0, 0.9));
        region.insert(make_pos(0, 0, 0, 9, 8, 8), make_cell(99.5, 0.9));

        let config = ThermalRadiationConfig::DEFAULT;
        let exchanges = plan_radiation_exchanges(&region, &config);

        assert!(exchanges.is_empty());
    }

    #[test]
    fn space_environment_simulation() {
        let mut region = ThermalRadiationRegion::new();
        region.insert(make_pos(0, 0, 0, 8, 8, 8), make_cell(300.0, 0.9));
        region.insert(make_pos(0, 0, 0, 9, 8, 8), make_cell(-100.0, 0.9));

        let config = ThermalRadiationConfig::SPACE;
        let result = thermal_radiation_step(&mut region, &config, 0.1);

        assert!(result.has_changes());

        let hot_after = region.get(make_pos(0, 0, 0, 8, 8, 8)).unwrap();
        assert!(hot_after.temperature() < 300.0);
    }

    #[test]
    fn result_checksum_deterministic() {
        let result1 = ThermalRadiationResult {
            exchanges: vec![RadiationExchange::new(
                make_pos(0, 0, 0, 0, 0, 0),
                make_pos(0, 0, 0, 1, 0, 0),
                100.0,
                500.0,
                100.0,
                0.5,
            )],
            energy_transferred: 50.0,
            cells_updated: 2,
            cross_chunk_exchanges: 0,
            max_temp_change: 10.0,
        };

        let result2 = ThermalRadiationResult {
            exchanges: vec![RadiationExchange::new(
                make_pos(0, 0, 0, 0, 0, 0),
                make_pos(0, 0, 0, 1, 0, 0),
                100.0,
                500.0,
                100.0,
                0.5,
            )],
            energy_transferred: 50.0,
            cells_updated: 2,
            cross_chunk_exchanges: 0,
            max_temp_change: 10.0,
        };

        assert_eq!(result1.checksum(), result2.checksum());
    }

    #[test]
    fn blackbody_t4_scaling() {
        let low = make_cell(300.0, 1.0);
        let high = make_cell(600.0, 1.0);

        let low_power = low.radiated_power();
        let high_power = high.radiated_power();

        let t_low_k = low.temperature_kelvin();
        let t_high_k = high.temperature_kelvin();
        let expected_ratio = (t_high_k / t_low_k).powi(4);
        let actual_ratio = high_power / low_power;

        assert!((actual_ratio - expected_ratio).abs() / expected_ratio < 0.01);
    }

    #[test]
    fn bincode_region_round_trip() {
        let mut region = ThermalRadiationRegion::new();
        region.insert(make_pos(0, 0, 0, 0, 0, 0), make_cell(100.0, 0.9));
        region.insert(make_pos(0, 0, 0, 1, 0, 0), make_cell(200.0, 0.8));

        let encoded = bincode::serialize(&region).unwrap();
        let decoded: ThermalRadiationRegion = bincode::deserialize(&encoded).unwrap();

        assert_eq!(decoded.len(), region.len());
        assert_eq!(decoded.checksum(), region.checksum());
    }
}
