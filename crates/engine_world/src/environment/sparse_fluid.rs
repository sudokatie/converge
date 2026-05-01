//! Sparse voxel fluid simulation with pressure equalization and large-volume flows.
//!
//! This module provides a CPU-side, data-only sparse representation of fluid regions
//! that supports cross-chunk flow links, pressure equalization planning, and
//! deterministic simulation with stable ordering and checksums.

use std::collections::BTreeMap;

use engine_core::coords::{ChunkPos, LocalPos};
use serde::{Deserialize, Serialize};

use crate::replay::{ChecksumBuilder, StepChecksum};

use super::{FluidCell, FluidKind};

/// A sparse fluid entry representing fluid at a specific world position.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SparseFluidEntry {
    /// Chunk position.
    pub chunk: ChunkPos,
    /// Local position within chunk.
    pub local: LocalPos,
    /// Fluid kind.
    pub kind: FluidKind,
    /// Volume (0.0 to 1.0).
    pub volume: f32,
    /// Pressure in atmospheres.
    pub pressure: f32,
    /// Temperature in Celsius.
    pub temperature: f32,
}

impl SparseFluidEntry {
    /// Create a new sparse fluid entry.
    #[must_use]
    pub fn new(
        chunk: ChunkPos,
        local: LocalPos,
        kind: FluidKind,
        volume: f32,
        pressure: f32,
        temperature: f32,
    ) -> Self {
        Self {
            chunk,
            local,
            kind,
            volume: volume.clamp(0.0, 1.0),
            pressure: pressure.clamp(0.0, 100.0),
            temperature: temperature.clamp(-273.0, 2000.0),
        }
    }

    /// Create from a chunk position, local position, and fluid cell.
    #[must_use]
    pub fn from_cell(chunk: ChunkPos, local: LocalPos, cell: FluidCell) -> Self {
        Self {
            chunk,
            local,
            kind: cell.kind(),
            volume: cell.volume(),
            pressure: cell.pressure(),
            temperature: cell.temperature(),
        }
    }

    /// Convert to a fluid cell.
    #[must_use]
    pub fn to_cell(&self) -> FluidCell {
        FluidCell::with_state(self.kind, self.volume, self.pressure, self.temperature)
    }

    /// Check if this entry has significant volume.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.volume >= 0.001
    }

    /// Compute a deterministic sort key for stable ordering.
    #[must_use]
    pub fn sort_key(&self) -> (i32, i32, i32, usize) {
        (
            self.chunk.x(),
            self.chunk.y(),
            self.chunk.z(),
            self.local.to_index(),
        )
    }
}

/// Unique identifier for a position across chunks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SparseFluidPos {
    /// Chunk position.
    pub chunk: ChunkPos,
    /// Local position within chunk.
    pub local: LocalPos,
}

impl PartialOrd for SparseFluidPos {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SparseFluidPos {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

impl SparseFluidPos {
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
}

impl SparseFluidPos {
    /// Create a new position.
    #[must_use]
    pub const fn new(chunk: ChunkPos, local: LocalPos) -> Self {
        Self { chunk, local }
    }

    /// Get 6-face neighbors, returning chunk-local pairs.
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

            let (chunk_offset, local_x) = if nx < 0 {
                (-1, 15u32)
            } else if nx >= 16 {
                (1, 0u32)
            } else {
                (0, nx as u32)
            };

            let (chunk_y_offset, local_y) = if ny < 0 {
                (-1, 15u32)
            } else if ny >= 16 {
                (1, 0u32)
            } else {
                (0, ny as u32)
            };

            let (chunk_z_offset, local_z) = if nz < 0 {
                (-1, 15u32)
            } else if nz >= 16 {
                (1, 0u32)
            } else {
                (0, nz as u32)
            };

            neighbors[i] = Self {
                chunk: ChunkPos::new(
                    self.chunk.x() + chunk_offset,
                    self.chunk.y() + chunk_y_offset,
                    self.chunk.z() + chunk_z_offset,
                ),
                local: LocalPos::new(local_x, local_y, local_z),
            };
        }

        neighbors
    }
}

/// A large-volume flow link between two positions across chunks.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlowLink {
    /// Source position.
    pub source: SparseFluidPos,
    /// Destination position.
    pub dest: SparseFluidPos,
    /// Fluid kind being transferred.
    pub kind: FluidKind,
    /// Volume flow rate (volume per second).
    pub flow_rate: f32,
    /// Pressure at source.
    pub source_pressure: f32,
    /// Pressure at destination.
    pub dest_pressure: f32,
    /// Temperature of flowing fluid.
    pub temperature: f32,
}

impl FlowLink {
    /// Create a new flow link.
    #[must_use]
    pub fn new(
        source: SparseFluidPos,
        dest: SparseFluidPos,
        kind: FluidKind,
        flow_rate: f32,
        source_pressure: f32,
        dest_pressure: f32,
        temperature: f32,
    ) -> Self {
        Self {
            source,
            dest,
            kind,
            flow_rate,
            source_pressure,
            dest_pressure,
            temperature,
        }
    }

    /// Pressure differential (source - dest).
    #[must_use]
    pub fn pressure_diff(&self) -> f32 {
        self.source_pressure - self.dest_pressure
    }

    /// Check if this link crosses chunk boundaries.
    #[must_use]
    pub fn is_cross_chunk(&self) -> bool {
        self.source.chunk != self.dest.chunk
    }

    /// Compute volume to transfer over dt seconds.
    #[must_use]
    pub fn volume_for_dt(&self, dt: f32) -> f32 {
        self.flow_rate * dt
    }

    /// Deterministic sort key.
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

/// A planned pressure equalization step.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PressureEqualizationStep {
    /// First position.
    pub pos_a: SparseFluidPos,
    /// Second position.
    pub pos_b: SparseFluidPos,
    /// Pressure at position A.
    pub pressure_a: f32,
    /// Pressure at position B.
    pub pressure_b: f32,
    /// Target equilibrium pressure.
    pub target_pressure: f32,
    /// Equalization rate (0.0 to 1.0).
    pub rate: f32,
}

impl PressureEqualizationStep {
    /// Create a new equalization step.
    #[must_use]
    pub fn new(
        pos_a: SparseFluidPos,
        pos_b: SparseFluidPos,
        pressure_a: f32,
        pressure_b: f32,
        rate: f32,
    ) -> Self {
        Self {
            pos_a,
            pos_b,
            pressure_a,
            pressure_b,
            target_pressure: f32::midpoint(pressure_a, pressure_b),
            rate: rate.clamp(0.0, 1.0),
        }
    }

    /// Compute new pressures after applying this step for dt seconds.
    #[must_use]
    pub fn apply(&self, dt: f32) -> (f32, f32) {
        let factor = (self.rate * dt).clamp(0.0, 1.0);
        let new_a = self.pressure_a + (self.target_pressure - self.pressure_a) * factor;
        let new_b = self.pressure_b + (self.target_pressure - self.pressure_b) * factor;
        (new_a, new_b)
    }

    /// Pressure differential magnitude.
    #[must_use]
    pub fn pressure_diff(&self) -> f32 {
        (self.pressure_a - self.pressure_b).abs()
    }

    /// Deterministic sort key.
    #[must_use]
    pub fn sort_key(&self) -> (i32, i32, i32, usize, i32, i32, i32, usize) {
        let a_key = (
            self.pos_a.chunk.x(),
            self.pos_a.chunk.y(),
            self.pos_a.chunk.z(),
            self.pos_a.local.to_index(),
        );
        let b_key = (
            self.pos_b.chunk.x(),
            self.pos_b.chunk.y(),
            self.pos_b.chunk.z(),
            self.pos_b.local.to_index(),
        );
        if a_key <= b_key {
            (
                a_key.0, a_key.1, a_key.2, a_key.3, b_key.0, b_key.1, b_key.2, b_key.3,
            )
        } else {
            (
                b_key.0, b_key.1, b_key.2, b_key.3, a_key.0, a_key.1, a_key.2, a_key.3,
            )
        }
    }
}

/// Pressure equalization plan containing all steps for a region.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PressureEqualizationPlan {
    /// Ordered equalization steps.
    pub steps: Vec<PressureEqualizationStep>,
    /// Total pressure differential to resolve.
    pub total_pressure_diff: f32,
    /// Number of cross-chunk equalizations.
    pub cross_chunk_count: u32,
}

impl PressureEqualizationPlan {
    /// Create an empty plan.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a step to the plan.
    pub fn add_step(&mut self, step: PressureEqualizationStep) {
        self.total_pressure_diff += step.pressure_diff();
        if step.pos_a.chunk != step.pos_b.chunk {
            self.cross_chunk_count += 1;
        }
        self.steps.push(step);
    }

    /// Sort steps for deterministic ordering.
    pub fn sort(&mut self) {
        self.steps.sort_by_key(PressureEqualizationStep::sort_key);
    }

    /// Number of steps in the plan.
    #[must_use]
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Check if plan is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Compute checksum for determinism verification.
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "step count fits in u32")]
    pub fn checksum(&self) -> StepChecksum {
        let mut builder = ChecksumBuilder::new();
        builder.feed_u32(self.steps.len() as u32);
        for step in &self.steps {
            builder.feed_i32(step.pos_a.chunk.x());
            builder.feed_i32(step.pos_a.chunk.y());
            builder.feed_i32(step.pos_a.chunk.z());
            builder.feed_u32(step.pos_a.local.to_index() as u32);
            builder.feed_i32(step.pos_b.chunk.x());
            builder.feed_i32(step.pos_b.chunk.y());
            builder.feed_i32(step.pos_b.chunk.z());
            builder.feed_u32(step.pos_b.local.to_index() as u32);
            builder.feed_f32(step.pressure_a);
            builder.feed_f32(step.pressure_b);
            builder.feed_f32(step.rate);
        }
        builder.build()
    }
}

/// Configuration for sparse fluid simulation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SparseFluidConfig {
    /// Minimum volume threshold for active entries.
    pub min_volume: f32,
    /// Minimum pressure differential for equalization.
    pub min_pressure_diff: f32,
    /// Base pressure equalization rate.
    pub equalization_rate: f32,
    /// Maximum flow rate per link.
    pub max_flow_rate: f32,
    /// Gravity factor for vertical flow bias.
    pub gravity_factor: f32,
    /// Enable cross-chunk flows.
    pub cross_chunk_enabled: bool,
}

impl SparseFluidConfig {
    /// Default configuration.
    pub const DEFAULT: Self = Self {
        min_volume: 0.001,
        min_pressure_diff: 0.01,
        equalization_rate: 0.5,
        max_flow_rate: 1.0,
        gravity_factor: 2.0,
        cross_chunk_enabled: true,
    };

    /// High-pressure system configuration (faster equalization).
    pub const HIGH_PRESSURE: Self = Self {
        min_volume: 0.001,
        min_pressure_diff: 0.001,
        equalization_rate: 0.9,
        max_flow_rate: 2.0,
        gravity_factor: 2.0,
        cross_chunk_enabled: true,
    };

    /// Viscous fluid configuration (slower flow).
    pub const VISCOUS: Self = Self {
        min_volume: 0.01,
        min_pressure_diff: 0.1,
        equalization_rate: 0.1,
        max_flow_rate: 0.2,
        gravity_factor: 3.0,
        cross_chunk_enabled: true,
    };
}

impl Default for SparseFluidConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// A sparse fluid region containing entries indexed by position.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SparseFluidRegion {
    /// Entries indexed by position for O(1) lookup.
    #[serde(
        serialize_with = "serialize_entries",
        deserialize_with = "deserialize_entries"
    )]
    entries: BTreeMap<SparseFluidPos, SparseFluidEntry>,
    /// Fluid kind for this region (all entries share the same kind).
    kind: FluidKind,
    /// Total volume across all entries.
    total_volume: f32,
    /// Average pressure across entries.
    avg_pressure: f32,
}

fn serialize_entries<S>(
    entries: &BTreeMap<SparseFluidPos, SparseFluidEntry>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeSeq;
    let mut seq = serializer.serialize_seq(Some(entries.len()))?;
    for entry in entries.values() {
        seq.serialize_element(entry)?;
    }
    seq.end()
}

fn deserialize_entries<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<SparseFluidPos, SparseFluidEntry>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let entries: Vec<SparseFluidEntry> = Deserialize::deserialize(deserializer)?;
    let mut map = BTreeMap::new();
    for entry in entries {
        let pos = SparseFluidPos::new(entry.chunk, entry.local);
        map.insert(pos, entry);
    }
    Ok(map)
}

impl Default for SparseFluidRegion {
    fn default() -> Self {
        Self::new(FluidKind::Water)
    }
}

impl SparseFluidRegion {
    /// Create a new empty region for a fluid kind.
    #[must_use]
    pub fn new(kind: FluidKind) -> Self {
        Self {
            entries: BTreeMap::new(),
            kind,
            total_volume: 0.0,
            avg_pressure: 1.0,
        }
    }

    /// Get the fluid kind for this region.
    #[must_use]
    pub const fn kind(&self) -> FluidKind {
        self.kind
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if region is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Total volume across all entries.
    #[must_use]
    pub fn total_volume(&self) -> f32 {
        self.total_volume
    }

    /// Average pressure across entries.
    #[must_use]
    pub fn avg_pressure(&self) -> f32 {
        self.avg_pressure
    }

    /// Get an entry by position.
    #[must_use]
    pub fn get(&self, pos: SparseFluidPos) -> Option<&SparseFluidEntry> {
        self.entries.get(&pos)
    }

    /// Insert or update an entry.
    pub fn insert(&mut self, entry: SparseFluidEntry) {
        let pos = SparseFluidPos::new(entry.chunk, entry.local);
        if let Some(old) = self.entries.get(&pos) {
            self.total_volume -= old.volume;
        }
        self.total_volume += entry.volume;
        self.entries.insert(pos, entry);
        self.recompute_avg_pressure();
    }

    /// Remove an entry by position.
    pub fn remove(&mut self, pos: SparseFluidPos) -> Option<SparseFluidEntry> {
        let removed = self.entries.remove(&pos);
        if let Some(ref entry) = removed {
            self.total_volume -= entry.volume;
            self.recompute_avg_pressure();
        }
        removed
    }

    /// Iterate over all entries in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = (&SparseFluidPos, &SparseFluidEntry)> {
        self.entries.iter()
    }

    /// Iterate over entries mutably.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&SparseFluidPos, &mut SparseFluidEntry)> {
        self.entries.iter_mut()
    }

    /// Get all positions.
    pub fn positions(&self) -> impl Iterator<Item = &SparseFluidPos> {
        self.entries.keys()
    }

    /// Get all entries.
    pub fn entries(&self) -> impl Iterator<Item = &SparseFluidEntry> {
        self.entries.values()
    }

    /// Filter entries by predicate.
    pub fn filter<F>(&self, predicate: F) -> impl Iterator<Item = &SparseFluidEntry>
    where
        F: Fn(&SparseFluidEntry) -> bool,
    {
        self.entries.values().filter(move |e| predicate(e))
    }

    /// Prune entries below minimum volume.
    pub fn prune(&mut self, min_volume: f32) {
        self.entries.retain(|_, e| e.volume >= min_volume);
        self.recompute_stats();
    }

    /// Query entries in a chunk.
    pub fn entries_in_chunk(&self, chunk: ChunkPos) -> impl Iterator<Item = &SparseFluidEntry> {
        self.entries
            .range(
                SparseFluidPos::new(chunk, LocalPos::new(0, 0, 0))
                    ..=SparseFluidPos::new(chunk, LocalPos::new(15, 15, 15)),
            )
            .map(|(_, e)| e)
    }

    /// Count entries in a chunk.
    #[must_use]
    pub fn count_in_chunk(&self, chunk: ChunkPos) -> usize {
        self.entries_in_chunk(chunk).count()
    }

    /// Get chunks that have entries.
    #[must_use]
    pub fn active_chunks(&self) -> Vec<ChunkPos> {
        let mut chunks: Vec<ChunkPos> = self.entries.keys().map(|p| p.chunk).collect();
        chunks.dedup();
        chunks
    }

    /// Compute a summary of the region.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "entry count fits precisely in f32 for reasonable region sizes"
    )]
    pub fn summary(&self) -> SparseFluidSummary {
        let mut min_pressure = f32::MAX;
        let mut max_pressure = f32::MIN;
        let mut min_temperature = f32::MAX;
        let mut max_temperature = f32::MIN;
        let mut total_pressure = 0.0f32;
        let mut total_temperature = 0.0f32;

        for entry in self.entries.values() {
            min_pressure = min_pressure.min(entry.pressure);
            max_pressure = max_pressure.max(entry.pressure);
            min_temperature = min_temperature.min(entry.temperature);
            max_temperature = max_temperature.max(entry.temperature);
            total_pressure += entry.pressure;
            total_temperature += entry.temperature;
        }

        let count = self.entries.len();
        let avg_pressure = if count > 0 {
            total_pressure / count as f32
        } else {
            1.0
        };
        let avg_temperature = if count > 0 {
            total_temperature / count as f32
        } else {
            20.0
        };

        SparseFluidSummary {
            kind: self.kind,
            entry_count: count,
            total_volume: self.total_volume,
            min_pressure: if count > 0 { min_pressure } else { 1.0 },
            max_pressure: if count > 0 { max_pressure } else { 1.0 },
            avg_pressure,
            min_temperature: if count > 0 { min_temperature } else { 20.0 },
            max_temperature: if count > 0 { max_temperature } else { 20.0 },
            avg_temperature,
            chunk_count: self.active_chunks().len(),
        }
    }

    /// Validate region state.
    #[must_use]
    pub fn validate(&self) -> SparseFluidValidation {
        let mut issues = Vec::new();

        for (pos, entry) in &self.entries {
            if entry.kind != self.kind {
                issues.push(format!(
                    "Entry at {:?} has kind {:?}, expected {:?}",
                    pos, entry.kind, self.kind
                ));
            }
            if entry.volume < 0.0 || entry.volume > 1.0 {
                issues.push(format!(
                    "Entry at {:?} has invalid volume {}",
                    pos, entry.volume
                ));
            }
            if entry.pressure < 0.0 || entry.pressure > 100.0 {
                issues.push(format!(
                    "Entry at {:?} has invalid pressure {}",
                    pos, entry.pressure
                ));
            }
            if entry.temperature < -273.0 || entry.temperature > 2000.0 {
                issues.push(format!(
                    "Entry at {:?} has invalid temperature {}",
                    pos, entry.temperature
                ));
            }
        }

        let recomputed_volume: f32 = self.entries.values().map(|e| e.volume).sum();
        if (recomputed_volume - self.total_volume).abs() > 0.001 {
            issues.push(format!(
                "Total volume mismatch: stored {} vs computed {}",
                self.total_volume, recomputed_volume
            ));
        }

        SparseFluidValidation {
            is_valid: issues.is_empty(),
            issues,
        }
    }

    /// Compute checksum for determinism verification.
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "entry count fits in u32")]
    pub fn checksum(&self) -> StepChecksum {
        let mut builder = ChecksumBuilder::new();
        builder.feed_u32(self.kind as u32);
        builder.feed_u32(self.entries.len() as u32);
        for (pos, entry) in &self.entries {
            builder.feed_i32(pos.chunk.x());
            builder.feed_i32(pos.chunk.y());
            builder.feed_i32(pos.chunk.z());
            builder.feed_u32(pos.local.to_index() as u32);
            builder.feed_f32(entry.volume);
            builder.feed_f32(entry.pressure);
            builder.feed_f32(entry.temperature);
        }
        builder.build()
    }

    /// Compute fingerprint bytes for the region.
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "entry count fits in u32")]
    pub fn fingerprint(&self) -> SparseFluidFingerprint {
        let checksum = self.checksum();
        SparseFluidFingerprint {
            kind: self.kind,
            entry_count: self.entries.len() as u32,
            total_volume_bits: self.total_volume.to_bits(),
            checksum: checksum.value(),
        }
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "entry count fits in f32 for reasonable region sizes"
    )]
    fn recompute_avg_pressure(&mut self) {
        if self.entries.is_empty() {
            self.avg_pressure = 1.0;
        } else {
            let sum: f32 = self.entries.values().map(|e| e.pressure).sum();
            self.avg_pressure = sum / self.entries.len() as f32;
        }
    }

    fn recompute_stats(&mut self) {
        self.total_volume = self.entries.values().map(|e| e.volume).sum();
        self.recompute_avg_pressure();
    }
}

/// Summary statistics for a sparse fluid region.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SparseFluidSummary {
    /// Fluid kind.
    pub kind: FluidKind,
    /// Number of entries.
    pub entry_count: usize,
    /// Total volume.
    pub total_volume: f32,
    /// Minimum pressure.
    pub min_pressure: f32,
    /// Maximum pressure.
    pub max_pressure: f32,
    /// Average pressure.
    pub avg_pressure: f32,
    /// Minimum temperature.
    pub min_temperature: f32,
    /// Maximum temperature.
    pub max_temperature: f32,
    /// Average temperature.
    pub avg_temperature: f32,
    /// Number of unique chunks.
    pub chunk_count: usize,
}

/// Validation result for a sparse fluid region.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SparseFluidValidation {
    /// Whether the region is valid.
    pub is_valid: bool,
    /// List of validation issues.
    pub issues: Vec<String>,
}

/// Compact fingerprint for a sparse fluid region.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SparseFluidFingerprint {
    /// Fluid kind.
    pub kind: FluidKind,
    /// Entry count.
    pub entry_count: u32,
    /// Total volume as bits.
    pub total_volume_bits: u32,
    /// Checksum value.
    pub checksum: u32,
}

/// Result of a sparse fluid simulation step.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SparseFluidResult {
    /// Updated entries.
    pub updated_entries: Vec<SparseFluidEntry>,
    /// Flow links that were applied.
    pub applied_flows: Vec<FlowLink>,
    /// Pressure equalization plan that was applied.
    pub equalization_plan: PressureEqualizationPlan,
    /// Volume transferred.
    pub volume_transferred: f32,
    /// Pressure equalized (sum of differentials resolved).
    pub pressure_equalized: f32,
    /// Number of cross-chunk flows.
    pub cross_chunk_flows: u32,
}

impl SparseFluidResult {
    /// Create an empty result.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any changes occurred.
    #[must_use]
    pub fn has_changes(&self) -> bool {
        !self.updated_entries.is_empty()
            || !self.applied_flows.is_empty()
            || !self.equalization_plan.is_empty()
    }

    /// Compute checksum for determinism verification.
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "counts fit in u32")]
    pub fn checksum(&self) -> StepChecksum {
        let mut builder = ChecksumBuilder::new();
        builder.feed_u32(self.updated_entries.len() as u32);
        for entry in &self.updated_entries {
            builder.feed_i32(entry.chunk.x());
            builder.feed_i32(entry.chunk.y());
            builder.feed_i32(entry.chunk.z());
            builder.feed_u32(entry.local.to_index() as u32);
            builder.feed_f32(entry.volume);
            builder.feed_f32(entry.pressure);
        }
        builder.feed_u32(self.applied_flows.len() as u32);
        builder.feed_f32(self.volume_transferred);
        builder.feed_f32(self.pressure_equalized);
        let plan_checksum = self.equalization_plan.checksum();
        builder.feed_u32(plan_checksum.value());
        builder.build()
    }
}

/// Compute flow links for a sparse fluid region.
#[must_use]
pub fn compute_flow_links(region: &SparseFluidRegion, config: &SparseFluidConfig) -> Vec<FlowLink> {
    let mut links = Vec::new();

    for (pos, entry) in region.iter() {
        if entry.volume < config.min_volume {
            continue;
        }

        let neighbors = pos.face_neighbors();
        for (i, neighbor_pos) in neighbors.iter().enumerate() {
            if !config.cross_chunk_enabled && pos.chunk != neighbor_pos.chunk {
                continue;
            }

            let neighbor_entry = region.get(*neighbor_pos);
            let neighbor_volume = neighbor_entry.map_or(0.0, |e| e.volume);
            let neighbor_pressure = neighbor_entry.map_or(1.0, |e| e.pressure);

            let volume_diff = entry.volume - neighbor_volume;
            let pressure_diff = entry.pressure - neighbor_pressure;

            let dy = match i {
                2 => -1,
                3 => 1,
                _ => 0,
            };
            let gravity_bonus = if region.kind().rises() {
                match dy.cmp(&0) {
                    std::cmp::Ordering::Greater => config.gravity_factor,
                    std::cmp::Ordering::Less => -config.gravity_factor * 0.5,
                    std::cmp::Ordering::Equal => 0.0,
                }
            } else {
                match dy.cmp(&0) {
                    std::cmp::Ordering::Less => config.gravity_factor,
                    std::cmp::Ordering::Greater => -config.gravity_factor * 0.5,
                    std::cmp::Ordering::Equal => 0.0,
                }
            };

            let effective_diff = volume_diff + pressure_diff * 0.1 + gravity_bonus * 0.1;
            if effective_diff > 0.0 {
                let flow_rate = (effective_diff * config.max_flow_rate).min(config.max_flow_rate);
                if flow_rate > 0.001 {
                    links.push(FlowLink::new(
                        *pos,
                        *neighbor_pos,
                        entry.kind,
                        flow_rate,
                        entry.pressure,
                        neighbor_pressure,
                        entry.temperature,
                    ));
                }
            }
        }
    }

    links.sort_by_key(FlowLink::sort_key);
    links
}

/// Plan pressure equalization for a sparse fluid region.
#[must_use]
pub fn plan_pressure_equalization(
    region: &SparseFluidRegion,
    config: &SparseFluidConfig,
) -> PressureEqualizationPlan {
    let mut plan = PressureEqualizationPlan::new();

    let mut processed = std::collections::HashSet::new();

    for (pos, entry) in region.iter() {
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

            if let Some(neighbor_entry) = region.get(*neighbor_pos) {
                let pressure_diff = (entry.pressure - neighbor_entry.pressure).abs();
                if pressure_diff >= config.min_pressure_diff {
                    plan.add_step(PressureEqualizationStep::new(
                        *pos,
                        *neighbor_pos,
                        entry.pressure,
                        neighbor_entry.pressure,
                        config.equalization_rate,
                    ));
                }
            }
        }
    }

    plan.sort();
    plan
}

/// Apply flow links to a sparse fluid region.
pub fn apply_flows(region: &mut SparseFluidRegion, flows: &[FlowLink], dt: f32) -> (f32, u32) {
    let mut total_transferred = 0.0f32;
    let mut cross_chunk_count = 0u32;

    for flow in flows {
        let volume = flow.volume_for_dt(dt);
        if volume <= 0.0 {
            continue;
        }

        let source_volume = region.get(flow.source).map_or(0.0, |e| e.volume);
        let actual_volume = volume.min(source_volume);
        if actual_volume <= 0.001 {
            continue;
        }

        if let Some(source_entry) = region.get(flow.source) {
            let mut updated = *source_entry;
            updated.volume = (updated.volume - actual_volume).max(0.0);
            region.insert(updated);
        }

        let dest_entry = region.get(flow.dest);
        let dest_volume = dest_entry.map_or(0.0, |e| e.volume);
        let dest_pressure = dest_entry.map_or(1.0, |e| e.pressure);
        let dest_temp = dest_entry.map_or(flow.temperature, |e| e.temperature);
        let new_dest_volume = (dest_volume + actual_volume).min(1.0);
        let new_dest_temp = if dest_volume > 0.001 {
            (dest_temp * dest_volume + flow.temperature * actual_volume) / new_dest_volume
        } else {
            flow.temperature
        };

        region.insert(SparseFluidEntry::new(
            flow.dest.chunk,
            flow.dest.local,
            flow.kind,
            new_dest_volume,
            dest_pressure,
            new_dest_temp,
        ));

        total_transferred += actual_volume;
        if flow.is_cross_chunk() {
            cross_chunk_count += 1;
        }
    }

    (total_transferred, cross_chunk_count)
}

/// Apply pressure equalization plan to a sparse fluid region.
pub fn apply_equalization(
    region: &mut SparseFluidRegion,
    plan: &PressureEqualizationPlan,
    dt: f32,
) -> f32 {
    let mut total_equalized = 0.0f32;

    for step in &plan.steps {
        let (new_a, new_b) = step.apply(dt);

        if let Some(entry_a) = region.get(step.pos_a) {
            let mut updated = *entry_a;
            updated.pressure = new_a;
            region.insert(updated);
        }

        if let Some(entry_b) = region.get(step.pos_b) {
            let mut updated = *entry_b;
            updated.pressure = new_b;
            region.insert(updated);
        }

        total_equalized += step.pressure_diff();
    }

    total_equalized
}

/// Execute a full sparse fluid simulation step.
#[must_use]
pub fn sparse_fluid_step(
    region: &mut SparseFluidRegion,
    config: &SparseFluidConfig,
    dt: f32,
) -> SparseFluidResult {
    let flows = compute_flow_links(region, config);
    let plan = plan_pressure_equalization(region, config);

    let (volume_transferred, cross_chunk_flows) = apply_flows(region, &flows, dt);
    let pressure_equalized = apply_equalization(region, &plan, dt);

    region.prune(config.min_volume);

    let updated_entries: Vec<SparseFluidEntry> = region.entries().copied().collect();

    SparseFluidResult {
        updated_entries,
        applied_flows: flows,
        equalization_plan: plan,
        volume_transferred,
        pressure_equalized,
        cross_chunk_flows,
    }
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    clippy::similar_names,
    reason = "tests check exact values; similar names are intentional for a/b pairs"
)]
mod tests {
    use super::*;

    fn make_pos(cx: i32, cy: i32, cz: i32, lx: u32, ly: u32, lz: u32) -> SparseFluidPos {
        SparseFluidPos::new(ChunkPos::new(cx, cy, cz), LocalPos::new(lx, ly, lz))
    }

    #[allow(clippy::too_many_arguments)]
    fn make_entry(
        cx: i32,
        cy: i32,
        cz: i32,
        lx: u32,
        ly: u32,
        lz: u32,
        volume: f32,
        pressure: f32,
    ) -> SparseFluidEntry {
        SparseFluidEntry::new(
            ChunkPos::new(cx, cy, cz),
            LocalPos::new(lx, ly, lz),
            FluidKind::Water,
            volume,
            pressure,
            20.0,
        )
    }

    #[test]
    fn entry_creation() {
        let entry = SparseFluidEntry::new(
            ChunkPos::new(1, 2, 3),
            LocalPos::new(4, 5, 6),
            FluidKind::Water,
            0.5,
            2.0,
            25.0,
        );
        assert_eq!(entry.chunk, ChunkPos::new(1, 2, 3));
        assert_eq!(entry.local, LocalPos::new(4, 5, 6));
        assert_eq!(entry.kind, FluidKind::Water);
        assert!((entry.volume - 0.5).abs() < 0.001);
        assert!((entry.pressure - 2.0).abs() < 0.001);
        assert!((entry.temperature - 25.0).abs() < 0.001);
    }

    #[test]
    fn entry_clamping() {
        let entry = SparseFluidEntry::new(
            ChunkPos::new(0, 0, 0),
            LocalPos::new(0, 0, 0),
            FluidKind::Water,
            2.0,
            200.0,
            5000.0,
        );
        assert!((entry.volume - 1.0).abs() < 0.001);
        assert!((entry.pressure - 100.0).abs() < 0.001);
        assert!((entry.temperature - 2000.0).abs() < 0.001);
    }

    #[test]
    fn entry_from_cell() {
        let cell = FluidCell::with_state(FluidKind::Lava, 0.7, 3.0, 1100.0);
        let entry =
            SparseFluidEntry::from_cell(ChunkPos::new(1, 1, 1), LocalPos::new(8, 8, 8), cell);
        assert_eq!(entry.kind, FluidKind::Lava);
        assert!((entry.volume - 0.7).abs() < 0.001);
        assert!((entry.pressure - 3.0).abs() < 0.001);
        assert!((entry.temperature - 1100.0).abs() < 0.001);
    }

    #[test]
    fn entry_to_cell() {
        let entry = SparseFluidEntry::new(
            ChunkPos::new(0, 0, 0),
            LocalPos::new(0, 0, 0),
            FluidKind::Gas,
            0.3,
            1.5,
            30.0,
        );
        let cell = entry.to_cell();
        assert_eq!(cell.kind(), FluidKind::Gas);
        assert!((cell.volume() - 0.3).abs() < 0.001);
        assert!((cell.pressure() - 1.5).abs() < 0.001);
        assert!((cell.temperature() - 30.0).abs() < 0.001);
    }

    #[test]
    fn pos_face_neighbors_inside_chunk() {
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
        assert_eq!(neighbors[1], make_pos(0, 0, 0, 1, 0, 0));
        assert_eq!(neighbors[2], make_pos(0, -1, 0, 0, 15, 0));
        assert_eq!(neighbors[3], make_pos(0, 0, 0, 0, 1, 0));
        assert_eq!(neighbors[4], make_pos(0, 0, -1, 0, 0, 15));
        assert_eq!(neighbors[5], make_pos(0, 0, 0, 0, 0, 1));
    }

    #[test]
    fn pos_face_neighbors_upper_bound() {
        let pos = make_pos(0, 0, 0, 15, 15, 15);
        let neighbors = pos.face_neighbors();

        assert_eq!(neighbors[0], make_pos(0, 0, 0, 14, 15, 15));
        assert_eq!(neighbors[1], make_pos(1, 0, 0, 0, 15, 15));
        assert_eq!(neighbors[2], make_pos(0, 0, 0, 15, 14, 15));
        assert_eq!(neighbors[3], make_pos(0, 1, 0, 15, 0, 15));
        assert_eq!(neighbors[4], make_pos(0, 0, 0, 15, 15, 14));
        assert_eq!(neighbors[5], make_pos(0, 0, 1, 15, 15, 0));
    }

    #[test]
    fn flow_link_basics() {
        let link = FlowLink::new(
            make_pos(0, 0, 0, 8, 8, 8),
            make_pos(0, 0, 0, 9, 8, 8),
            FluidKind::Water,
            0.5,
            2.0,
            1.0,
            20.0,
        );
        assert!((link.pressure_diff() - 1.0).abs() < 0.001);
        assert!(!link.is_cross_chunk());
        assert!((link.volume_for_dt(0.1) - 0.05).abs() < 0.001);
    }

    #[test]
    fn flow_link_cross_chunk() {
        let link = FlowLink::new(
            make_pos(0, 0, 0, 0, 0, 0),
            make_pos(-1, 0, 0, 15, 0, 0),
            FluidKind::Water,
            0.5,
            2.0,
            1.0,
            20.0,
        );
        assert!(link.is_cross_chunk());
    }

    #[test]
    fn pressure_equalization_step() {
        let step = PressureEqualizationStep::new(
            make_pos(0, 0, 0, 8, 8, 8),
            make_pos(0, 0, 0, 9, 8, 8),
            5.0,
            1.0,
            0.5,
        );
        assert!((step.target_pressure - 3.0).abs() < 0.001);
        assert!((step.pressure_diff() - 4.0).abs() < 0.001);

        let (new_a, new_b) = step.apply(1.0);
        assert!((new_a - 4.0).abs() < 0.001);
        assert!((new_b - 2.0).abs() < 0.001);
    }

    #[test]
    fn region_insert_and_get() {
        let mut region = SparseFluidRegion::new(FluidKind::Water);
        let entry = make_entry(0, 0, 0, 8, 8, 8, 0.5, 2.0);
        let pos = SparseFluidPos::new(entry.chunk, entry.local);

        region.insert(entry);
        assert_eq!(region.len(), 1);
        assert!(!region.is_empty());

        let retrieved = region.get(pos).unwrap();
        assert!((retrieved.volume - 0.5).abs() < 0.001);
    }

    #[test]
    fn region_total_volume() {
        let mut region = SparseFluidRegion::new(FluidKind::Water);
        region.insert(make_entry(0, 0, 0, 0, 0, 0, 0.3, 1.0));
        region.insert(make_entry(0, 0, 0, 1, 0, 0, 0.5, 1.0));
        assert!((region.total_volume() - 0.8).abs() < 0.001);

        region.insert(make_entry(0, 0, 0, 0, 0, 0, 0.2, 1.0));
        assert!((region.total_volume() - 0.7).abs() < 0.001);
    }

    #[test]
    fn region_remove() {
        let mut region = SparseFluidRegion::new(FluidKind::Water);
        let entry = make_entry(0, 0, 0, 8, 8, 8, 0.5, 2.0);
        let pos = SparseFluidPos::new(entry.chunk, entry.local);

        region.insert(entry);
        let removed = region.remove(pos);
        assert!(removed.is_some());
        assert!(region.is_empty());
        assert!((region.total_volume() - 0.0).abs() < 0.001);
    }

    #[test]
    fn region_prune() {
        let mut region = SparseFluidRegion::new(FluidKind::Water);
        region.insert(make_entry(0, 0, 0, 0, 0, 0, 0.5, 1.0));
        region.insert(make_entry(0, 0, 0, 1, 0, 0, 0.0005, 1.0));
        assert_eq!(region.len(), 2);

        region.prune(0.001);
        assert_eq!(region.len(), 1);
    }

    #[test]
    fn region_entries_in_chunk() {
        let mut region = SparseFluidRegion::new(FluidKind::Water);
        region.insert(make_entry(0, 0, 0, 0, 0, 0, 0.5, 1.0));
        region.insert(make_entry(0, 0, 0, 1, 0, 0, 0.5, 1.0));
        region.insert(make_entry(1, 0, 0, 0, 0, 0, 0.5, 1.0));

        let count = region.count_in_chunk(ChunkPos::new(0, 0, 0));
        assert_eq!(count, 2);
    }

    #[test]
    fn region_active_chunks() {
        let mut region = SparseFluidRegion::new(FluidKind::Water);
        region.insert(make_entry(0, 0, 0, 0, 0, 0, 0.5, 1.0));
        region.insert(make_entry(1, 2, 3, 0, 0, 0, 0.5, 1.0));
        region.insert(make_entry(-1, 0, 0, 0, 0, 0, 0.5, 1.0));

        let chunks = region.active_chunks();
        assert_eq!(chunks.len(), 3);
    }

    #[test]
    fn region_summary() {
        let mut region = SparseFluidRegion::new(FluidKind::Water);
        region.insert(make_entry(0, 0, 0, 0, 0, 0, 0.3, 1.0));
        region.insert(make_entry(0, 0, 0, 1, 0, 0, 0.5, 3.0));

        let summary = region.summary();
        assert_eq!(summary.kind, FluidKind::Water);
        assert_eq!(summary.entry_count, 2);
        assert!((summary.total_volume - 0.8).abs() < 0.001);
        assert!((summary.min_pressure - 1.0).abs() < 0.001);
        assert!((summary.max_pressure - 3.0).abs() < 0.001);
        assert!((summary.avg_pressure - 2.0).abs() < 0.001);
    }

    #[test]
    fn region_validation_valid() {
        let mut region = SparseFluidRegion::new(FluidKind::Water);
        region.insert(make_entry(0, 0, 0, 0, 0, 0, 0.5, 1.0));
        let validation = region.validate();
        assert!(validation.is_valid);
        assert!(validation.issues.is_empty());
    }

    #[test]
    fn region_checksum_deterministic() {
        let mut region1 = SparseFluidRegion::new(FluidKind::Water);
        region1.insert(make_entry(0, 0, 0, 0, 0, 0, 0.5, 1.0));
        region1.insert(make_entry(0, 0, 0, 1, 0, 0, 0.3, 2.0));

        let mut region2 = SparseFluidRegion::new(FluidKind::Water);
        region2.insert(make_entry(0, 0, 0, 1, 0, 0, 0.3, 2.0));
        region2.insert(make_entry(0, 0, 0, 0, 0, 0, 0.5, 1.0));

        assert_eq!(region1.checksum(), region2.checksum());
    }

    #[test]
    fn region_checksum_differs() {
        let mut region1 = SparseFluidRegion::new(FluidKind::Water);
        region1.insert(make_entry(0, 0, 0, 0, 0, 0, 0.5, 1.0));

        let mut region2 = SparseFluidRegion::new(FluidKind::Water);
        region2.insert(make_entry(0, 0, 0, 0, 0, 0, 0.6, 1.0));

        assert_ne!(region1.checksum(), region2.checksum());
    }

    #[test]
    fn compute_flow_links_basic() {
        let mut region = SparseFluidRegion::new(FluidKind::Water);
        region.insert(make_entry(0, 0, 0, 8, 8, 8, 0.8, 2.0));
        region.insert(make_entry(0, 0, 0, 9, 8, 8, 0.2, 1.0));

        let config = SparseFluidConfig::DEFAULT;
        let links = compute_flow_links(&region, &config);

        assert!(!links.is_empty());
        let high_to_low = links
            .iter()
            .find(|l| l.source.local == LocalPos::new(8, 8, 8));
        assert!(high_to_low.is_some());
    }

    #[test]
    fn compute_flow_links_deterministic() {
        let mut region = SparseFluidRegion::new(FluidKind::Water);
        region.insert(make_entry(0, 0, 0, 5, 5, 5, 0.8, 2.0));
        region.insert(make_entry(0, 0, 0, 6, 5, 5, 0.2, 1.0));
        region.insert(make_entry(0, 0, 0, 5, 6, 5, 0.3, 1.5));

        let config = SparseFluidConfig::DEFAULT;
        let links1 = compute_flow_links(&region, &config);
        let links2 = compute_flow_links(&region, &config);

        assert_eq!(links1.len(), links2.len());
        for (l1, l2) in links1.iter().zip(links2.iter()) {
            assert_eq!(l1.source, l2.source);
            assert_eq!(l1.dest, l2.dest);
            assert!((l1.flow_rate - l2.flow_rate).abs() < 0.001);
        }
    }

    #[test]
    fn plan_pressure_equalization_basic() {
        let mut region = SparseFluidRegion::new(FluidKind::Water);
        region.insert(make_entry(0, 0, 0, 8, 8, 8, 0.5, 5.0));
        region.insert(make_entry(0, 0, 0, 9, 8, 8, 0.5, 1.0));

        let config = SparseFluidConfig::DEFAULT;
        let plan = plan_pressure_equalization(&region, &config);

        assert!(!plan.is_empty());
        assert!(plan.total_pressure_diff > 0.0);
    }

    #[test]
    fn plan_pressure_equalization_deterministic() {
        let mut region = SparseFluidRegion::new(FluidKind::Water);
        region.insert(make_entry(0, 0, 0, 8, 8, 8, 0.5, 5.0));
        region.insert(make_entry(0, 0, 0, 9, 8, 8, 0.5, 1.0));
        region.insert(make_entry(0, 0, 0, 8, 9, 8, 0.5, 3.0));

        let config = SparseFluidConfig::DEFAULT;
        let plan1 = plan_pressure_equalization(&region, &config);
        let plan2 = plan_pressure_equalization(&region, &config);

        assert_eq!(plan1.checksum(), plan2.checksum());
    }

    #[test]
    fn apply_flows_basic() {
        let mut region = SparseFluidRegion::new(FluidKind::Water);
        region.insert(make_entry(0, 0, 0, 8, 8, 8, 0.8, 1.0));

        let flows = vec![FlowLink::new(
            make_pos(0, 0, 0, 8, 8, 8),
            make_pos(0, 0, 0, 9, 8, 8),
            FluidKind::Water,
            0.5,
            1.0,
            1.0,
            20.0,
        )];

        let (transferred, _) = apply_flows(&mut region, &flows, 0.2);
        assert!(transferred > 0.0);

        let dest = region.get(make_pos(0, 0, 0, 9, 8, 8));
        assert!(dest.is_some());
        assert!(dest.unwrap().volume > 0.0);
    }

    #[test]
    fn apply_equalization_basic() {
        let mut region = SparseFluidRegion::new(FluidKind::Water);
        region.insert(make_entry(0, 0, 0, 8, 8, 8, 0.5, 5.0));
        region.insert(make_entry(0, 0, 0, 9, 8, 8, 0.5, 1.0));

        let mut plan = PressureEqualizationPlan::new();
        plan.add_step(PressureEqualizationStep::new(
            make_pos(0, 0, 0, 8, 8, 8),
            make_pos(0, 0, 0, 9, 8, 8),
            5.0,
            1.0,
            0.5,
        ));

        let equalized = apply_equalization(&mut region, &plan, 1.0);
        assert!(equalized > 0.0);

        let a = region.get(make_pos(0, 0, 0, 8, 8, 8)).unwrap();
        let b = region.get(make_pos(0, 0, 0, 9, 8, 8)).unwrap();
        assert!(a.pressure < 5.0);
        assert!(b.pressure > 1.0);
    }

    #[test]
    fn sparse_fluid_step_integration() {
        let mut region = SparseFluidRegion::new(FluidKind::Water);
        region.insert(make_entry(0, 0, 0, 8, 8, 8, 0.8, 3.0));
        region.insert(make_entry(0, 0, 0, 9, 8, 8, 0.2, 1.0));

        let config = SparseFluidConfig::DEFAULT;
        let result = sparse_fluid_step(&mut region, &config, 0.1);

        assert!(result.has_changes());
        assert!(result.volume_transferred > 0.0 || result.pressure_equalized > 0.0);
    }

    #[test]
    fn sparse_fluid_step_deterministic() {
        let make_region = || {
            let mut region = SparseFluidRegion::new(FluidKind::Water);
            region.insert(make_entry(0, 0, 0, 8, 8, 8, 0.8, 3.0));
            region.insert(make_entry(0, 0, 0, 9, 8, 8, 0.2, 1.0));
            region.insert(make_entry(0, 0, 0, 8, 7, 8, 0.5, 2.0));
            region
        };

        let config = SparseFluidConfig::DEFAULT;

        let mut region1 = make_region();
        let result1 = sparse_fluid_step(&mut region1, &config, 0.1);

        let mut region2 = make_region();
        let result2 = sparse_fluid_step(&mut region2, &config, 0.1);

        assert_eq!(region1.checksum(), region2.checksum());
        assert_eq!(result1.checksum(), result2.checksum());
    }

    #[test]
    fn config_presets() {
        const {
            assert!(SparseFluidConfig::DEFAULT.equalization_rate > 0.0);
        };
        const {
            assert!(
                SparseFluidConfig::HIGH_PRESSURE.equalization_rate
                    > SparseFluidConfig::DEFAULT.equalization_rate
            );
        };
        const {
            assert!(
                SparseFluidConfig::VISCOUS.max_flow_rate < SparseFluidConfig::DEFAULT.max_flow_rate
            );
        };
    }

    #[test]
    fn serde_entry_round_trip() {
        let entry = make_entry(1, 2, 3, 4, 5, 6, 0.7, 2.5);
        let json = serde_json::to_string(&entry).unwrap();
        let recovered: SparseFluidEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.chunk, entry.chunk);
        assert_eq!(recovered.local, entry.local);
        assert!((recovered.volume - entry.volume).abs() < 0.001);
    }

    #[test]
    fn serde_region_round_trip() {
        let mut region = SparseFluidRegion::new(FluidKind::Water);
        region.insert(make_entry(0, 0, 0, 0, 0, 0, 0.5, 1.0));
        region.insert(make_entry(0, 0, 0, 1, 0, 0, 0.3, 2.0));

        let json = serde_json::to_string(&region).unwrap();
        let recovered: SparseFluidRegion = serde_json::from_str(&json).unwrap();

        assert_eq!(recovered.len(), region.len());
        assert_eq!(recovered.kind(), region.kind());
        assert!((recovered.total_volume() - region.total_volume()).abs() < 0.001);
    }

    #[test]
    fn serde_config_round_trip() {
        let config = SparseFluidConfig::HIGH_PRESSURE;
        let json = serde_json::to_string(&config).unwrap();
        let recovered: SparseFluidConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, config);
    }

    #[test]
    fn serde_plan_round_trip() {
        let mut plan = PressureEqualizationPlan::new();
        plan.add_step(PressureEqualizationStep::new(
            make_pos(0, 0, 0, 8, 8, 8),
            make_pos(0, 0, 0, 9, 8, 8),
            5.0,
            1.0,
            0.5,
        ));

        let json = serde_json::to_string(&plan).unwrap();
        let recovered: PressureEqualizationPlan = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.len(), plan.len());
    }

    #[test]
    fn serde_result_round_trip() {
        let result = SparseFluidResult {
            updated_entries: vec![make_entry(0, 0, 0, 0, 0, 0, 0.5, 1.0)],
            applied_flows: vec![],
            equalization_plan: PressureEqualizationPlan::new(),
            volume_transferred: 0.1,
            pressure_equalized: 0.5,
            cross_chunk_flows: 0,
        };

        let json = serde_json::to_string(&result).unwrap();
        let recovered: SparseFluidResult = serde_json::from_str(&json).unwrap();
        assert_eq!(
            recovered.updated_entries.len(),
            result.updated_entries.len()
        );
        assert!((recovered.volume_transferred - result.volume_transferred).abs() < 0.001);
    }

    #[test]
    fn fingerprint_equality() {
        let mut region1 = SparseFluidRegion::new(FluidKind::Water);
        region1.insert(make_entry(0, 0, 0, 0, 0, 0, 0.5, 1.0));

        let mut region2 = SparseFluidRegion::new(FluidKind::Water);
        region2.insert(make_entry(0, 0, 0, 0, 0, 0, 0.5, 1.0));

        assert_eq!(region1.fingerprint(), region2.fingerprint());
    }

    #[test]
    fn fingerprint_differs_on_change() {
        let mut region = SparseFluidRegion::new(FluidKind::Water);
        region.insert(make_entry(0, 0, 0, 0, 0, 0, 0.5, 1.0));
        let fp1 = region.fingerprint();

        region.insert(make_entry(0, 0, 0, 0, 0, 0, 0.6, 1.0));
        let fp2 = region.fingerprint();

        assert_ne!(fp1, fp2);
    }

    #[test]
    fn cross_chunk_flow_disabled() {
        let mut region = SparseFluidRegion::new(FluidKind::Water);
        region.insert(make_entry(0, 0, 0, 0, 0, 0, 0.8, 2.0));

        let mut config = SparseFluidConfig::DEFAULT;
        config.cross_chunk_enabled = false;

        let links = compute_flow_links(&region, &config);
        for link in &links {
            assert!(!link.is_cross_chunk());
        }
    }

    #[test]
    fn gas_rises() {
        let mut region = SparseFluidRegion::new(FluidKind::Gas);
        let entry = SparseFluidEntry::new(
            ChunkPos::new(0, 0, 0),
            LocalPos::new(8, 8, 8),
            FluidKind::Gas,
            0.8,
            1.0,
            20.0,
        );
        region.insert(entry);

        let config = SparseFluidConfig::DEFAULT;
        let links = compute_flow_links(&region, &config);

        let up_link = links
            .iter()
            .find(|l| l.dest.local == LocalPos::new(8, 9, 8));
        let down_link = links
            .iter()
            .find(|l| l.dest.local == LocalPos::new(8, 7, 8));

        assert!(up_link.is_some());
        if let (Some(up), Some(down)) = (up_link, down_link) {
            assert!(up.flow_rate > down.flow_rate);
        }
    }

    #[test]
    fn water_falls() {
        let mut region = SparseFluidRegion::new(FluidKind::Water);
        region.insert(make_entry(0, 0, 0, 8, 8, 8, 0.8, 1.0));

        let config = SparseFluidConfig::DEFAULT;
        let links = compute_flow_links(&region, &config);

        let up_link = links
            .iter()
            .find(|l| l.dest.local == LocalPos::new(8, 9, 8));
        let down_link = links
            .iter()
            .find(|l| l.dest.local == LocalPos::new(8, 7, 8));

        assert!(down_link.is_some());
        if let (Some(up), Some(down)) = (up_link, down_link) {
            assert!(down.flow_rate > up.flow_rate);
        }
    }

    #[test]
    fn empty_region_operations() {
        let region = SparseFluidRegion::new(FluidKind::Water);
        assert!(region.is_empty());
        assert_eq!(region.len(), 0);
        assert!((region.total_volume() - 0.0).abs() < 0.001);
        assert!(region.active_chunks().is_empty());

        let summary = region.summary();
        assert_eq!(summary.entry_count, 0);

        let validation = region.validate();
        assert!(validation.is_valid);
    }

    #[test]
    fn plan_cross_chunk_count() {
        let mut region = SparseFluidRegion::new(FluidKind::Water);
        region.insert(make_entry(0, 0, 0, 0, 0, 0, 0.5, 5.0));
        region.insert(make_entry(-1, 0, 0, 15, 0, 0, 0.5, 1.0));

        let config = SparseFluidConfig::DEFAULT;
        let plan = plan_pressure_equalization(&region, &config);

        assert!(plan.cross_chunk_count > 0);
    }
}
