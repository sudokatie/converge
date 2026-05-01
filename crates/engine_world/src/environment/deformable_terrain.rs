//! Deformable terrain simulation with stress accumulation and fracture propagation.
//!
//! This module provides deterministic terrain deformation simulation where
//! cells accumulate stress/strain and can fracture when thresholds are exceeded.
//!
//! # Physics Model
//!
//! - Stress accumulates from external forces and propagates to neighbors
//! - Strain develops based on material ductility
//! - Damage accumulates when stress exceeds yield threshold
//! - Fracture occurs when damage exceeds material fracture threshold
//! - Fractures propagate along links to neighboring cells

use std::collections::BTreeMap;

use engine_core::coords::{ChunkPos, LocalPos};
use serde::{Deserialize, Serialize};

use crate::replay::{ChecksumBuilder, StepChecksum};

/// Minimum valid hardness.
pub const MIN_HARDNESS: f32 = 0.01;

/// Maximum valid hardness.
pub const MAX_HARDNESS: f32 = 100.0;

/// Minimum valid ductility.
pub const MIN_DUCTILITY: f32 = 0.0;

/// Maximum valid ductility (perfectly plastic).
pub const MAX_DUCTILITY: f32 = 1.0;

/// Minimum valid fracture threshold.
pub const MIN_FRACTURE_THRESHOLD: f32 = 0.01;

/// Maximum valid fracture threshold.
pub const MAX_FRACTURE_THRESHOLD: f32 = 1000.0;

/// Maximum stress value.
pub const MAX_STRESS: f32 = 10000.0;

/// Maximum strain value.
pub const MAX_STRAIN: f32 = 10.0;

/// Maximum damage value (1.0 = fully damaged).
pub const MAX_DAMAGE: f32 = 1.0;

/// Maximum deformation offset.
pub const MAX_DEFORMATION: f32 = 1.0;

/// Default hardness for terrain cells.
pub const DEFAULT_HARDNESS: f32 = 1.0;

/// Default ductility for terrain cells.
pub const DEFAULT_DUCTILITY: f32 = 0.3;

/// Default fracture threshold.
pub const DEFAULT_FRACTURE_THRESHOLD: f32 = 10.0;

/// A terrain cell with material properties and deformation state.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerrainCell {
    /// Material hardness (resistance to stress).
    hardness: f32,
    /// Material ductility (0 = brittle, 1 = plastic).
    ductility: f32,
    /// Fracture threshold (damage level that causes fracture).
    fracture_threshold: f32,
    /// Accumulated stress.
    stress: f32,
    /// Accumulated strain.
    strain: f32,
    /// Accumulated damage (0 = pristine, 1 = fully damaged).
    damage: f32,
    /// Permanent deformation offset.
    deformation: f32,
}

impl TerrainCell {
    /// Create a new terrain cell with validation.
    #[must_use]
    pub fn new(
        hardness: f32,
        ductility: f32,
        fracture_threshold: f32,
        stress: f32,
        strain: f32,
        damage: f32,
        deformation: f32,
    ) -> Self {
        Self {
            hardness: hardness.clamp(MIN_HARDNESS, MAX_HARDNESS),
            ductility: ductility.clamp(MIN_DUCTILITY, MAX_DUCTILITY),
            fracture_threshold: fracture_threshold
                .clamp(MIN_FRACTURE_THRESHOLD, MAX_FRACTURE_THRESHOLD),
            stress: stress.clamp(0.0, MAX_STRESS),
            strain: strain.clamp(0.0, MAX_STRAIN),
            damage: damage.clamp(0.0, MAX_DAMAGE),
            deformation: deformation.clamp(0.0, MAX_DEFORMATION),
        }
    }

    /// Create a cell with default material properties and no damage.
    #[must_use]
    pub fn pristine() -> Self {
        Self::new(
            DEFAULT_HARDNESS,
            DEFAULT_DUCTILITY,
            DEFAULT_FRACTURE_THRESHOLD,
            0.0,
            0.0,
            0.0,
            0.0,
        )
    }

    /// Create a rock-like cell (hard, brittle).
    #[must_use]
    pub fn rock() -> Self {
        Self::new(5.0, 0.1, 15.0, 0.0, 0.0, 0.0, 0.0)
    }

    /// Create a soil-like cell (soft, ductile).
    #[must_use]
    pub fn soil() -> Self {
        Self::new(0.5, 0.7, 5.0, 0.0, 0.0, 0.0, 0.0)
    }

    /// Create a metal-like cell (hard, somewhat ductile).
    #[must_use]
    pub fn metal() -> Self {
        Self::new(10.0, 0.4, 50.0, 0.0, 0.0, 0.0, 0.0)
    }

    /// Get hardness.
    #[must_use]
    pub const fn hardness(&self) -> f32 {
        self.hardness
    }

    /// Get ductility.
    #[must_use]
    pub const fn ductility(&self) -> f32 {
        self.ductility
    }

    /// Get fracture threshold.
    #[must_use]
    pub const fn fracture_threshold(&self) -> f32 {
        self.fracture_threshold
    }

    /// Get current stress.
    #[must_use]
    pub const fn stress(&self) -> f32 {
        self.stress
    }

    /// Get current strain.
    #[must_use]
    pub const fn strain(&self) -> f32 {
        self.strain
    }

    /// Get current damage.
    #[must_use]
    pub const fn damage(&self) -> f32 {
        self.damage
    }

    /// Get deformation offset.
    #[must_use]
    pub const fn deformation(&self) -> f32 {
        self.deformation
    }

    /// Apply stress delta with clamping.
    pub fn apply_stress(&mut self, delta: f32) {
        self.stress = (self.stress + delta).clamp(0.0, MAX_STRESS);
    }

    /// Apply strain delta with clamping.
    pub fn apply_strain(&mut self, delta: f32) {
        self.strain = (self.strain + delta).clamp(0.0, MAX_STRAIN);
    }

    /// Apply damage delta with clamping.
    pub fn apply_damage(&mut self, delta: f32) {
        self.damage = (self.damage + delta).clamp(0.0, MAX_DAMAGE);
    }

    /// Apply deformation delta with clamping.
    pub fn apply_deformation(&mut self, delta: f32) {
        self.deformation = (self.deformation + delta).clamp(0.0, MAX_DEFORMATION);
    }

    /// Check if cell has exceeded fracture threshold.
    #[must_use]
    pub fn is_fractured(&self) -> bool {
        self.damage >= self.fracture_threshold / MAX_FRACTURE_THRESHOLD
    }

    /// Check if cell is under significant stress.
    #[must_use]
    pub fn is_stressed(&self, threshold: f32) -> bool {
        self.stress >= threshold
    }

    /// Check if cell has significant damage.
    #[must_use]
    pub fn is_damaged(&self, threshold: f32) -> bool {
        self.damage >= threshold
    }

    /// Compute stress transfer factor based on hardness difference.
    #[must_use]
    pub fn stress_transfer_factor(&self, other: &Self) -> f32 {
        let hardness_ratio = other.hardness / self.hardness;
        (1.0 - (-hardness_ratio).exp()).clamp(0.0, 1.0)
    }

    /// Compute strain from current stress based on ductility.
    #[must_use]
    pub fn compute_strain_from_stress(&self) -> f32 {
        if self.hardness <= 0.0 {
            return 0.0;
        }
        (self.stress / self.hardness) * self.ductility
    }

    /// Compute damage increment based on stress exceeding yield.
    #[must_use]
    pub fn compute_damage_increment(&self, yield_stress: f32) -> f32 {
        if self.stress <= yield_stress {
            return 0.0;
        }
        let excess = self.stress - yield_stress;
        (excess / self.fracture_threshold).clamp(0.0, 0.1)
    }
}

impl Default for TerrainCell {
    fn default() -> Self {
        Self::pristine()
    }
}

/// Unique position identifier for terrain cells across chunks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TerrainPos {
    /// Chunk position.
    pub chunk: ChunkPos,
    /// Local position within chunk.
    pub local: LocalPos,
}

impl PartialOrd for TerrainPos {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TerrainPos {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

impl TerrainPos {
    /// Create a new terrain position.
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

/// A terrain entry combining position and cell data.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerrainEntry {
    /// Chunk position.
    pub chunk: ChunkPos,
    /// Local position.
    pub local: LocalPos,
    /// Terrain cell data.
    pub cell: TerrainCell,
}

impl TerrainEntry {
    /// Create a new terrain entry.
    #[must_use]
    pub fn new(chunk: ChunkPos, local: LocalPos, cell: TerrainCell) -> Self {
        Self { chunk, local, cell }
    }

    /// Get position as [`TerrainPos`].
    #[must_use]
    pub const fn pos(&self) -> TerrainPos {
        TerrainPos::new(self.chunk, self.local)
    }
}

/// A link between two terrain cells that can fracture.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FractureLink {
    /// Source cell position.
    pub source: TerrainPos,
    /// Target cell position.
    pub target: TerrainPos,
    /// Link integrity (1.0 = intact, 0.0 = broken).
    pub integrity: f32,
    /// Stress transfer coefficient.
    pub transfer_coeff: f32,
}

impl FractureLink {
    /// Create a new fracture link.
    #[must_use]
    pub fn new(
        source: TerrainPos,
        target: TerrainPos,
        integrity: f32,
        transfer_coeff: f32,
    ) -> Self {
        Self {
            source,
            target,
            integrity: integrity.clamp(0.0, 1.0),
            transfer_coeff: transfer_coeff.clamp(0.0, 1.0),
        }
    }

    /// Create an intact link with default transfer.
    #[must_use]
    pub fn intact(source: TerrainPos, target: TerrainPos) -> Self {
        Self::new(source, target, 1.0, 0.5)
    }

    /// Whether this link crosses chunk boundaries.
    #[must_use]
    pub fn is_cross_chunk(&self) -> bool {
        self.source.chunk != self.target.chunk
    }

    /// Whether this link is broken.
    #[must_use]
    pub fn is_broken(&self) -> bool {
        self.integrity <= 0.0
    }

    /// Sort key for deterministic ordering.
    #[must_use]
    pub fn sort_key(&self) -> (i32, i32, i32, usize, i32, i32, i32, usize) {
        (
            self.source.chunk.x(),
            self.source.chunk.y(),
            self.source.chunk.z(),
            self.source.local.to_index(),
            self.target.chunk.x(),
            self.target.chunk.y(),
            self.target.chunk.z(),
            self.target.local.to_index(),
        )
    }
}

/// A fracture event recording when and where fracture occurred.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FractureEvent {
    /// Position where fracture occurred.
    pub pos: TerrainPos,
    /// Damage level at fracture.
    pub damage: f32,
    /// Stress at fracture.
    pub stress: f32,
    /// Whether fracture propagated from neighbor.
    pub propagated: bool,
}

impl FractureEvent {
    /// Create a new fracture event.
    #[must_use]
    pub fn new(pos: TerrainPos, damage: f32, stress: f32, propagated: bool) -> Self {
        Self {
            pos,
            damage,
            stress,
            propagated,
        }
    }

    /// Sort key for deterministic ordering.
    #[must_use]
    pub fn sort_key(&self) -> (i32, i32, i32, usize) {
        self.pos.sort_key()
    }
}

/// A planned stress propagation between cells.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StressPropagation {
    /// Source position.
    pub source: TerrainPos,
    /// Target position.
    pub target: TerrainPos,
    /// Stress transfer amount.
    pub stress_transfer: f32,
    /// Source stress at planning time.
    pub source_stress: f32,
    /// Target stress at planning time.
    pub target_stress: f32,
}

impl StressPropagation {
    /// Create a new stress propagation.
    #[must_use]
    pub fn new(
        source: TerrainPos,
        target: TerrainPos,
        stress_transfer: f32,
        source_stress: f32,
        target_stress: f32,
    ) -> Self {
        Self {
            source,
            target,
            stress_transfer: stress_transfer.max(0.0),
            source_stress,
            target_stress,
        }
    }

    /// Stress differential.
    #[must_use]
    pub fn stress_diff(&self) -> f32 {
        self.source_stress - self.target_stress
    }

    /// Whether this propagation crosses chunk boundaries.
    #[must_use]
    pub fn is_cross_chunk(&self) -> bool {
        self.source.chunk != self.target.chunk
    }

    /// Sort key for deterministic ordering.
    #[must_use]
    pub fn sort_key(&self) -> (i32, i32, i32, usize, i32, i32, i32, usize) {
        (
            self.source.chunk.x(),
            self.source.chunk.y(),
            self.source.chunk.z(),
            self.source.local.to_index(),
            self.target.chunk.x(),
            self.target.chunk.y(),
            self.target.chunk.z(),
            self.target.local.to_index(),
        )
    }
}

/// Configuration for deformable terrain simulation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeformableTerrainConfig {
    /// Minimum stress differential for propagation.
    pub min_stress_diff: f32,
    /// Global stress propagation scale.
    pub propagation_scale: f32,
    /// Yield stress threshold for damage accumulation.
    pub yield_stress: f32,
    /// Enable cross-chunk stress propagation.
    pub cross_chunk_enabled: bool,
    /// Maximum stress transfer per step.
    pub max_stress_transfer: f32,
    /// Damage decay rate per step.
    pub damage_decay_rate: f32,
    /// Stress relaxation rate per step.
    pub stress_relaxation_rate: f32,
    /// Fracture propagation factor.
    pub fracture_propagation_factor: f32,
}

impl DeformableTerrainConfig {
    /// Default configuration.
    pub const DEFAULT: Self = Self {
        min_stress_diff: 0.1,
        propagation_scale: 1.0,
        yield_stress: 5.0,
        cross_chunk_enabled: true,
        max_stress_transfer: 100.0,
        damage_decay_rate: 0.0,
        stress_relaxation_rate: 0.05,
        fracture_propagation_factor: 0.3,
    };

    /// Configuration for brittle materials (fast fracture propagation).
    pub const BRITTLE: Self = Self {
        min_stress_diff: 0.05,
        propagation_scale: 1.5,
        yield_stress: 2.0,
        cross_chunk_enabled: true,
        max_stress_transfer: 200.0,
        damage_decay_rate: 0.0,
        stress_relaxation_rate: 0.02,
        fracture_propagation_factor: 0.6,
    };

    /// Configuration for ductile materials (slow deformation).
    pub const DUCTILE: Self = Self {
        min_stress_diff: 0.2,
        propagation_scale: 0.5,
        yield_stress: 10.0,
        cross_chunk_enabled: true,
        max_stress_transfer: 50.0,
        damage_decay_rate: 0.01,
        stress_relaxation_rate: 0.1,
        fracture_propagation_factor: 0.1,
    };

    /// Validate configuration values.
    #[must_use]
    pub fn validate(&self) -> DeformableTerrainValidation {
        let mut issues = Vec::new();

        if self.min_stress_diff < 0.0 {
            issues.push("min_stress_diff must be non-negative".to_string());
        }
        if self.propagation_scale <= 0.0 {
            issues.push("propagation_scale must be positive".to_string());
        }
        if self.yield_stress < 0.0 {
            issues.push("yield_stress must be non-negative".to_string());
        }
        if self.max_stress_transfer <= 0.0 {
            issues.push("max_stress_transfer must be positive".to_string());
        }
        if self.damage_decay_rate < 0.0 || self.damage_decay_rate > 1.0 {
            issues.push("damage_decay_rate must be in [0, 1]".to_string());
        }
        if self.stress_relaxation_rate < 0.0 || self.stress_relaxation_rate > 1.0 {
            issues.push("stress_relaxation_rate must be in [0, 1]".to_string());
        }
        if self.fracture_propagation_factor < 0.0 || self.fracture_propagation_factor > 1.0 {
            issues.push("fracture_propagation_factor must be in [0, 1]".to_string());
        }

        DeformableTerrainValidation {
            is_valid: issues.is_empty(),
            issues,
        }
    }
}

impl Default for DeformableTerrainConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Validation result for configuration or region state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeformableTerrainValidation {
    /// Whether validation passed.
    pub is_valid: bool,
    /// List of validation issues.
    pub issues: Vec<String>,
}

/// A region of terrain cells for deformation simulation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeformableTerrainRegion {
    /// Cells indexed by position.
    #[serde(
        serialize_with = "serialize_cells",
        deserialize_with = "deserialize_cells"
    )]
    cells: BTreeMap<TerrainPos, TerrainCell>,
    /// Fracture links between cells.
    links: Vec<FractureLink>,
    /// Total accumulated stress.
    total_stress: f64,
    /// Average damage level.
    avg_damage: f32,
}

fn serialize_cells<S>(
    cells: &BTreeMap<TerrainPos, TerrainCell>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeSeq;
    let mut seq = serializer.serialize_seq(Some(cells.len()))?;
    for (pos, cell) in cells {
        seq.serialize_element(&TerrainEntry::new(pos.chunk, pos.local, *cell))?;
    }
    seq.end()
}

fn deserialize_cells<'de, D>(deserializer: D) -> Result<BTreeMap<TerrainPos, TerrainCell>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let entries: Vec<TerrainEntry> = Deserialize::deserialize(deserializer)?;
    let mut map = BTreeMap::new();
    for entry in entries {
        map.insert(entry.pos(), entry.cell);
    }
    Ok(map)
}

impl Default for DeformableTerrainRegion {
    fn default() -> Self {
        Self::new()
    }
}

impl DeformableTerrainRegion {
    /// Create an empty region.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cells: BTreeMap::new(),
            links: Vec::new(),
            total_stress: 0.0,
            avg_damage: 0.0,
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
    pub fn get(&self, pos: TerrainPos) -> Option<&TerrainCell> {
        self.cells.get(&pos)
    }

    /// Get a mutable cell by position.
    pub fn get_mut(&mut self, pos: TerrainPos) -> Option<&mut TerrainCell> {
        self.cells.get_mut(&pos)
    }

    /// Insert or update a cell.
    pub fn insert(&mut self, pos: TerrainPos, cell: TerrainCell) {
        self.cells.insert(pos, cell);
        self.recompute_stats();
    }

    /// Insert a terrain entry.
    pub fn insert_entry(&mut self, entry: TerrainEntry) {
        self.insert(entry.pos(), entry.cell);
    }

    /// Remove a cell.
    pub fn remove(&mut self, pos: TerrainPos) -> Option<TerrainCell> {
        let removed = self.cells.remove(&pos);
        if removed.is_some() {
            self.recompute_stats();
        }
        removed
    }

    /// Iterate over cells in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = (&TerrainPos, &TerrainCell)> {
        self.cells.iter()
    }

    /// Iterate mutably.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&TerrainPos, &mut TerrainCell)> {
        self.cells.iter_mut()
    }

    /// Get all positions.
    pub fn positions(&self) -> impl Iterator<Item = &TerrainPos> {
        self.cells.keys()
    }

    /// Get fracture links.
    #[must_use]
    pub fn links(&self) -> &[FractureLink] {
        &self.links
    }

    /// Add a fracture link.
    pub fn add_link(&mut self, link: FractureLink) {
        self.links.push(link);
        self.links.sort_by_key(FractureLink::sort_key);
    }

    /// Get total stress.
    #[must_use]
    pub fn total_stress(&self) -> f64 {
        self.total_stress
    }

    /// Get average damage.
    #[must_use]
    pub fn avg_damage(&self) -> f32 {
        self.avg_damage
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
                TerrainPos::new(chunk, LocalPos::new(0, 0, 0))
                    ..=TerrainPos::new(chunk, LocalPos::new(15, 15, 15)),
            )
            .count()
    }

    /// Compute a summary of the region.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "cell count fits in f32 for reasonable region sizes"
    )]
    pub fn summary(&self) -> DeformableTerrainSummary {
        const STRESSED_THRESHOLD: f32 = 1.0;
        const DAMAGED_THRESHOLD: f32 = 0.1;

        if self.cells.is_empty() {
            return DeformableTerrainSummary {
                cell_count: 0,
                chunk_count: 0,
                min_stress: 0.0,
                max_stress: 0.0,
                avg_stress: 0.0,
                min_damage: 0.0,
                max_damage: 0.0,
                avg_damage: 0.0,
                stressed_cells: 0,
                damaged_cells: 0,
                fractured_cells: 0,
                link_count: 0,
                broken_links: 0,
            };
        }

        let mut min_stress = f32::MAX;
        let mut max_stress = f32::MIN;
        let mut total_stress = 0.0f32;
        let mut min_damage = f32::MAX;
        let mut max_damage = f32::MIN;
        let mut total_damage = 0.0f32;
        let mut stressed_count = 0usize;
        let mut damaged_count = 0usize;
        let mut fractured_count = 0usize;

        for cell in self.cells.values() {
            min_stress = min_stress.min(cell.stress());
            max_stress = max_stress.max(cell.stress());
            total_stress += cell.stress();
            min_damage = min_damage.min(cell.damage());
            max_damage = max_damage.max(cell.damage());
            total_damage += cell.damage();

            if cell.is_stressed(STRESSED_THRESHOLD) {
                stressed_count += 1;
            }
            if cell.is_damaged(DAMAGED_THRESHOLD) {
                damaged_count += 1;
            }
            if cell.is_fractured() {
                fractured_count += 1;
            }
        }

        let count = self.cells.len();
        let broken_links = self.links.iter().filter(|l| l.is_broken()).count();

        DeformableTerrainSummary {
            cell_count: count,
            chunk_count: self.active_chunks().len(),
            min_stress,
            max_stress,
            avg_stress: total_stress / count as f32,
            min_damage,
            max_damage,
            avg_damage: total_damage / count as f32,
            stressed_cells: stressed_count,
            damaged_cells: damaged_count,
            fractured_cells: fractured_count,
            link_count: self.links.len(),
            broken_links,
        }
    }

    /// Validate region state.
    #[must_use]
    pub fn validate(&self) -> DeformableTerrainValidation {
        let mut issues = Vec::new();

        for (pos, cell) in &self.cells {
            if cell.hardness() < MIN_HARDNESS || cell.hardness() > MAX_HARDNESS {
                issues.push(format!(
                    "Cell at {:?} has invalid hardness {}",
                    pos,
                    cell.hardness()
                ));
            }
            if cell.ductility() < MIN_DUCTILITY || cell.ductility() > MAX_DUCTILITY {
                issues.push(format!(
                    "Cell at {:?} has invalid ductility {}",
                    pos,
                    cell.ductility()
                ));
            }
            if cell.damage() < 0.0 || cell.damage() > MAX_DAMAGE {
                issues.push(format!(
                    "Cell at {:?} has invalid damage {}",
                    pos,
                    cell.damage()
                ));
            }
        }

        for link in &self.links {
            if !self.cells.contains_key(&link.source) {
                issues.push(format!("Link source {:?} not in region", link.source));
            }
            if !self.cells.contains_key(&link.target) {
                issues.push(format!("Link target {:?} not in region", link.target));
            }
        }

        DeformableTerrainValidation {
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
            builder.feed_f32(cell.hardness());
            builder.feed_f32(cell.ductility());
            builder.feed_f32(cell.fracture_threshold());
            builder.feed_f32(cell.stress());
            builder.feed_f32(cell.strain());
            builder.feed_f32(cell.damage());
            builder.feed_f32(cell.deformation());
        }
        builder.feed_u32(self.links.len() as u32);
        for link in &self.links {
            builder.feed_i32(link.source.chunk.x());
            builder.feed_i32(link.source.chunk.y());
            builder.feed_i32(link.source.chunk.z());
            builder.feed_u32(link.source.local.to_index() as u32);
            builder.feed_i32(link.target.chunk.x());
            builder.feed_i32(link.target.chunk.y());
            builder.feed_i32(link.target.chunk.z());
            builder.feed_u32(link.target.local.to_index() as u32);
            builder.feed_f32(link.integrity);
        }
        builder.build()
    }

    /// Compute compact fingerprint.
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "cell count fits in u32")]
    pub fn fingerprint(&self) -> DeformableTerrainFingerprint {
        let checksum = self.checksum();
        DeformableTerrainFingerprint {
            cell_count: self.cells.len() as u32,
            avg_damage_bits: self.avg_damage.to_bits(),
            checksum: checksum.value(),
        }
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "cell count fits in f64 for reasonable region sizes"
    )]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "average values from f64 to f32 is acceptable precision"
    )]
    fn recompute_stats(&mut self) {
        if self.cells.is_empty() {
            self.total_stress = 0.0;
            self.avg_damage = 0.0;
            return;
        }

        let mut total_stress = 0.0f64;
        let mut total_damage = 0.0f64;

        for cell in self.cells.values() {
            total_stress += f64::from(cell.stress());
            total_damage += f64::from(cell.damage());
        }

        self.total_stress = total_stress;
        self.avg_damage = (total_damage / self.cells.len() as f64) as f32;
    }
}

/// Summary statistics for a deformable terrain region.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeformableTerrainSummary {
    /// Number of cells.
    pub cell_count: usize,
    /// Number of active chunks.
    pub chunk_count: usize,
    /// Minimum stress.
    pub min_stress: f32,
    /// Maximum stress.
    pub max_stress: f32,
    /// Average stress.
    pub avg_stress: f32,
    /// Minimum damage.
    pub min_damage: f32,
    /// Maximum damage.
    pub max_damage: f32,
    /// Average damage.
    pub avg_damage: f32,
    /// Count of stressed cells.
    pub stressed_cells: usize,
    /// Count of damaged cells.
    pub damaged_cells: usize,
    /// Count of fractured cells.
    pub fractured_cells: usize,
    /// Number of fracture links.
    pub link_count: usize,
    /// Number of broken links.
    pub broken_links: usize,
}

/// Compact fingerprint for a deformable terrain region.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeformableTerrainFingerprint {
    /// Cell count.
    pub cell_count: u32,
    /// Average damage as bits.
    pub avg_damage_bits: u32,
    /// Checksum value.
    pub checksum: u32,
}

/// Result of a deformable terrain simulation step.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DeformableTerrainResult {
    /// Planned stress propagations.
    pub propagations: Vec<StressPropagation>,
    /// Fracture events that occurred.
    pub fracture_events: Vec<FractureEvent>,
    /// Total stress transferred.
    pub stress_transferred: f32,
    /// Number of cells updated.
    pub cells_updated: u32,
    /// Number of cross-chunk propagations.
    pub cross_chunk_propagations: u32,
    /// Maximum stress change in any cell.
    pub max_stress_change: f32,
    /// Number of new fractures.
    pub new_fractures: u32,
}

impl DeformableTerrainResult {
    /// Create an empty result.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether any changes occurred.
    #[must_use]
    pub fn has_changes(&self) -> bool {
        self.cells_updated > 0 || !self.propagations.is_empty() || !self.fracture_events.is_empty()
    }

    /// Compute checksum for determinism verification.
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "counts fit in u32")]
    pub fn checksum(&self) -> StepChecksum {
        let mut builder = ChecksumBuilder::new();
        builder.feed_u32(self.propagations.len() as u32);
        for prop in &self.propagations {
            builder.feed_i32(prop.source.chunk.x());
            builder.feed_i32(prop.source.chunk.y());
            builder.feed_i32(prop.source.chunk.z());
            builder.feed_u32(prop.source.local.to_index() as u32);
            builder.feed_i32(prop.target.chunk.x());
            builder.feed_i32(prop.target.chunk.y());
            builder.feed_i32(prop.target.chunk.z());
            builder.feed_u32(prop.target.local.to_index() as u32);
            builder.feed_f32(prop.stress_transfer);
        }
        builder.feed_u32(self.fracture_events.len() as u32);
        for event in &self.fracture_events {
            builder.feed_i32(event.pos.chunk.x());
            builder.feed_i32(event.pos.chunk.y());
            builder.feed_i32(event.pos.chunk.z());
            builder.feed_u32(event.pos.local.to_index() as u32);
            builder.feed_f32(event.damage);
        }
        builder.feed_f32(self.stress_transferred);
        builder.feed_u32(self.cells_updated);
        builder.feed_f32(self.max_stress_change);
        builder.feed_u32(self.new_fractures);
        builder.build()
    }
}

/// Plan stress propagation for a region.
#[must_use]
pub fn plan_stress_propagation(
    region: &DeformableTerrainRegion,
    config: &DeformableTerrainConfig,
) -> Vec<StressPropagation> {
    let mut propagations = Vec::new();
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

            let stress_diff = (cell.stress() - neighbor_cell.stress()).abs();
            if stress_diff < config.min_stress_diff {
                continue;
            }

            let (source_pos, source_cell, target_pos, target_cell) =
                if cell.stress() > neighbor_cell.stress() {
                    (*pos, cell, *neighbor_pos, neighbor_cell)
                } else {
                    (*neighbor_pos, neighbor_cell, *pos, cell)
                };

            let transfer_factor = source_cell.stress_transfer_factor(target_cell);
            let stress_transfer = (stress_diff * transfer_factor * config.propagation_scale)
                .min(config.max_stress_transfer);

            if stress_transfer > 0.001 {
                propagations.push(StressPropagation::new(
                    source_pos,
                    target_pos,
                    stress_transfer,
                    source_cell.stress(),
                    target_cell.stress(),
                ));
            }
        }
    }

    propagations.sort_by_key(StressPropagation::sort_key);
    propagations
}

/// Apply stress propagations to a region.
pub fn apply_stress_propagation(
    region: &mut DeformableTerrainRegion,
    propagations: &[StressPropagation],
    config: &DeformableTerrainConfig,
) -> (f32, u32, u32, f32) {
    let mut total_stress = 0.0f32;
    let mut cells_updated = 0u32;
    let mut cross_chunk_count = 0u32;
    let mut max_change = 0.0f32;

    for prop in propagations {
        let stress_transfer = prop.stress_transfer;
        if stress_transfer <= 0.0 {
            continue;
        }

        let source_delta = -stress_transfer * 0.5;
        let target_delta = stress_transfer * 0.5;

        if let Some(cell) = region.get_mut(prop.source) {
            cell.apply_stress(source_delta);
            max_change = max_change.max(source_delta.abs());

            let strain_delta = cell.compute_strain_from_stress() * 0.1;
            cell.apply_strain(strain_delta);

            let damage_delta = cell.compute_damage_increment(config.yield_stress);
            cell.apply_damage(damage_delta);
        }

        if let Some(cell) = region.get_mut(prop.target) {
            cell.apply_stress(target_delta);
            max_change = max_change.max(target_delta.abs());

            let strain_delta = cell.compute_strain_from_stress() * 0.1;
            cell.apply_strain(strain_delta);

            let damage_delta = cell.compute_damage_increment(config.yield_stress);
            cell.apply_damage(damage_delta);
        }

        total_stress += stress_transfer;
        cells_updated += 2;

        if prop.is_cross_chunk() {
            cross_chunk_count += 1;
        }
    }

    region.recompute_stats();

    (total_stress, cells_updated, cross_chunk_count, max_change)
}

/// Check for fractures and generate fracture events.
#[must_use]
pub fn check_fractures(
    region: &DeformableTerrainRegion,
    _config: &DeformableTerrainConfig,
) -> Vec<FractureEvent> {
    let mut events = Vec::new();

    for (pos, cell) in region.iter() {
        if cell.is_fractured() {
            events.push(FractureEvent::new(
                *pos,
                cell.damage(),
                cell.stress(),
                false,
            ));
        }
    }

    events.sort_by_key(FractureEvent::sort_key);
    events
}

/// Propagate fractures to neighboring cells.
pub fn propagate_fractures(
    region: &mut DeformableTerrainRegion,
    fracture_events: &[FractureEvent],
    config: &DeformableTerrainConfig,
) -> Vec<FractureEvent> {
    let mut propagated = Vec::new();

    for event in fracture_events {
        let neighbors = event.pos.face_neighbors();

        for neighbor_pos in &neighbors {
            if !config.cross_chunk_enabled && event.pos.chunk != neighbor_pos.chunk {
                continue;
            }

            let Some(neighbor_cell) = region.get_mut(*neighbor_pos) else {
                continue;
            };

            if neighbor_cell.is_fractured() {
                continue;
            }

            let damage_transfer = event.damage * config.fracture_propagation_factor;
            neighbor_cell.apply_damage(damage_transfer);

            if neighbor_cell.is_fractured() {
                propagated.push(FractureEvent::new(
                    *neighbor_pos,
                    neighbor_cell.damage(),
                    neighbor_cell.stress(),
                    true,
                ));
            }
        }
    }

    propagated.sort_by_key(FractureEvent::sort_key);
    region.recompute_stats();

    propagated
}

/// Apply stress relaxation to the region.
pub fn apply_stress_relaxation(
    region: &mut DeformableTerrainRegion,
    config: &DeformableTerrainConfig,
) {
    if config.stress_relaxation_rate <= 0.0 {
        return;
    }

    for (_, cell) in region.iter_mut() {
        let relaxation = cell.stress() * config.stress_relaxation_rate;
        cell.apply_stress(-relaxation);
    }

    region.recompute_stats();
}

/// Apply deformation from accumulated damage.
pub fn apply_deformation_from_damage(region: &mut DeformableTerrainRegion) {
    for (_, cell) in region.iter_mut() {
        if cell.damage() > 0.0 {
            let deformation_delta = cell.damage() * cell.ductility() * 0.01;
            cell.apply_deformation(deformation_delta);
        }
    }

    region.recompute_stats();
}

/// Execute a complete deformable terrain simulation step.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "fracture count fits in u32 for reasonable region sizes"
)]
pub fn deformable_terrain_step(
    region: &mut DeformableTerrainRegion,
    config: &DeformableTerrainConfig,
) -> DeformableTerrainResult {
    let propagations = plan_stress_propagation(region, config);

    let (stress_transferred, cells_updated, cross_chunk_propagations, max_stress_change) =
        apply_stress_propagation(region, &propagations, config);

    let mut fracture_events = check_fractures(region, config);
    let initial_fractures = fracture_events.len() as u32;

    let propagated_fractures = propagate_fractures(region, &fracture_events, config);
    fracture_events.extend(propagated_fractures);

    apply_stress_relaxation(region, config);
    apply_deformation_from_damage(region);

    DeformableTerrainResult {
        propagations,
        fracture_events,
        stress_transferred,
        cells_updated,
        cross_chunk_propagations,
        max_stress_change,
        new_fractures: initial_fractures,
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

    fn make_pos(cx: i32, cy: i32, cz: i32, lx: u32, ly: u32, lz: u32) -> TerrainPos {
        TerrainPos::new(ChunkPos::new(cx, cy, cz), LocalPos::new(lx, ly, lz))
    }

    fn make_cell(stress: f32, damage: f32) -> TerrainCell {
        TerrainCell::new(
            DEFAULT_HARDNESS,
            DEFAULT_DUCTILITY,
            DEFAULT_FRACTURE_THRESHOLD,
            stress,
            0.0,
            damage,
            0.0,
        )
    }

    #[test]
    fn cell_creation() {
        let cell = TerrainCell::new(2.0, 0.5, 15.0, 5.0, 0.1, 0.05, 0.0);
        assert!((cell.hardness() - 2.0).abs() < 0.001);
        assert!((cell.ductility() - 0.5).abs() < 0.001);
        assert!((cell.fracture_threshold() - 15.0).abs() < 0.001);
        assert!((cell.stress() - 5.0).abs() < 0.001);
        assert!((cell.strain() - 0.1).abs() < 0.001);
        assert!((cell.damage() - 0.05).abs() < 0.001);
    }

    #[test]
    fn cell_clamping() {
        let cell = TerrainCell::new(-1.0, 2.0, -5.0, -10.0, -1.0, 2.0, 5.0);
        assert!((cell.hardness() - MIN_HARDNESS).abs() < 0.001);
        assert!((cell.ductility() - MAX_DUCTILITY).abs() < 0.001);
        assert!((cell.fracture_threshold() - MIN_FRACTURE_THRESHOLD).abs() < 0.001);
        assert!((cell.stress() - 0.0).abs() < 0.001);
        assert!((cell.strain() - 0.0).abs() < 0.001);
        assert!((cell.damage() - MAX_DAMAGE).abs() < 0.001);
        assert!((cell.deformation() - MAX_DEFORMATION).abs() < 0.001);
    }

    #[test]
    fn cell_presets() {
        let rock = TerrainCell::rock();
        assert!(rock.hardness() > 1.0);
        assert!(rock.ductility() < 0.2);

        let soil = TerrainCell::soil();
        assert!(soil.hardness() < 1.0);
        assert!(soil.ductility() > 0.5);

        let metal = TerrainCell::metal();
        assert!(metal.hardness() > rock.hardness());
        assert!(metal.fracture_threshold() > rock.fracture_threshold());
    }

    #[test]
    fn cell_apply_stress() {
        let mut cell = TerrainCell::pristine();
        cell.apply_stress(5.0);
        assert!((cell.stress() - 5.0).abs() < 0.001);

        cell.apply_stress(-3.0);
        assert!((cell.stress() - 2.0).abs() < 0.001);

        cell.apply_stress(-10.0);
        assert!((cell.stress() - 0.0).abs() < 0.001);
    }

    #[test]
    fn cell_apply_damage() {
        let mut cell = TerrainCell::pristine();
        cell.apply_damage(0.5);
        assert!((cell.damage() - 0.5).abs() < 0.001);

        cell.apply_damage(1.0);
        assert!((cell.damage() - MAX_DAMAGE).abs() < 0.001);
    }

    #[test]
    fn cell_is_fractured() {
        let pristine = TerrainCell::pristine();
        assert!(!pristine.is_fractured());

        let mut damaged = TerrainCell::pristine();
        damaged.apply_damage(DEFAULT_FRACTURE_THRESHOLD / MAX_FRACTURE_THRESHOLD + 0.01);
        assert!(damaged.is_fractured());
    }

    #[test]
    fn cell_stress_transfer_factor() {
        let hard = TerrainCell::rock();
        let soft = TerrainCell::soil();

        let factor_hard_to_soft = hard.stress_transfer_factor(&soft);
        let factor_soft_to_hard = soft.stress_transfer_factor(&hard);

        assert!(factor_hard_to_soft > 0.0);
        assert!(factor_soft_to_hard > 0.0);
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
    fn fracture_link_creation() {
        let link = FractureLink::new(
            make_pos(0, 0, 0, 0, 0, 0),
            make_pos(0, 0, 0, 1, 0, 0),
            1.0,
            0.5,
        );
        assert!(!link.is_broken());
        assert!(!link.is_cross_chunk());
    }

    #[test]
    fn fracture_link_cross_chunk() {
        let link = FractureLink::intact(make_pos(0, 0, 0, 0, 0, 0), make_pos(1, 0, 0, 0, 0, 0));
        assert!(link.is_cross_chunk());
    }

    #[test]
    fn stress_propagation_creation() {
        let prop = StressPropagation::new(
            make_pos(0, 0, 0, 0, 0, 0),
            make_pos(0, 0, 0, 1, 0, 0),
            5.0,
            10.0,
            2.0,
        );
        assert!((prop.stress_diff() - 8.0).abs() < 0.001);
        assert!(!prop.is_cross_chunk());
    }

    #[test]
    fn config_defaults() {
        let config = DeformableTerrainConfig::DEFAULT;
        assert!(config.min_stress_diff > 0.0);
        assert!(config.propagation_scale > 0.0);
        assert!(config.validate().is_valid);
    }

    #[test]
    fn config_presets() {
        assert!(DeformableTerrainConfig::BRITTLE.validate().is_valid);
        assert!(DeformableTerrainConfig::DUCTILE.validate().is_valid);
    }

    #[test]
    fn config_validation() {
        let mut config = DeformableTerrainConfig::DEFAULT;
        config.min_stress_diff = -1.0;
        let validation = config.validate();
        assert!(!validation.is_valid);
        assert!(!validation.issues.is_empty());
    }

    #[test]
    fn region_insert_get() {
        let mut region = DeformableTerrainRegion::new();
        let pos = make_pos(0, 0, 0, 8, 8, 8);
        let cell = make_cell(5.0, 0.1);

        region.insert(pos, cell);
        assert_eq!(region.len(), 1);
        assert!(!region.is_empty());

        let retrieved = region.get(pos).unwrap();
        assert!((retrieved.stress() - 5.0).abs() < 0.001);
    }

    #[test]
    fn region_remove() {
        let mut region = DeformableTerrainRegion::new();
        let pos = make_pos(0, 0, 0, 8, 8, 8);
        region.insert(pos, make_cell(5.0, 0.0));

        let removed = region.remove(pos);
        assert!(removed.is_some());
        assert!(region.is_empty());
    }

    #[test]
    fn region_stats() {
        let mut region = DeformableTerrainRegion::new();
        region.insert(make_pos(0, 0, 0, 0, 0, 0), make_cell(5.0, 0.1));
        region.insert(make_pos(0, 0, 0, 1, 0, 0), make_cell(10.0, 0.2));

        assert!(region.total_stress() > 0.0);
        assert!((region.avg_damage() - 0.15).abs() < 0.001);
    }

    #[test]
    fn region_summary() {
        let mut region = DeformableTerrainRegion::new();
        region.insert(make_pos(0, 0, 0, 0, 0, 0), make_cell(0.5, 0.05));
        region.insert(make_pos(0, 0, 0, 1, 0, 0), make_cell(8.0, 0.15));

        let summary = region.summary();
        assert_eq!(summary.cell_count, 2);
        assert!((summary.min_stress - 0.5).abs() < 0.001);
        assert!((summary.max_stress - 8.0).abs() < 0.001);
        assert!((summary.avg_stress - 4.25).abs() < 0.001);
        assert_eq!(summary.stressed_cells, 1);
        assert_eq!(summary.damaged_cells, 1);
    }

    #[test]
    fn region_validation() {
        let mut region = DeformableTerrainRegion::new();
        region.insert(make_pos(0, 0, 0, 0, 0, 0), make_cell(5.0, 0.0));
        let validation = region.validate();
        assert!(validation.is_valid);
    }

    #[test]
    fn region_checksum_deterministic() {
        let mut r1 = DeformableTerrainRegion::new();
        r1.insert(make_pos(0, 0, 0, 0, 0, 0), make_cell(5.0, 0.1));
        r1.insert(make_pos(0, 0, 0, 1, 0, 0), make_cell(10.0, 0.2));

        let mut r2 = DeformableTerrainRegion::new();
        r2.insert(make_pos(0, 0, 0, 1, 0, 0), make_cell(10.0, 0.2));
        r2.insert(make_pos(0, 0, 0, 0, 0, 0), make_cell(5.0, 0.1));

        assert_eq!(r1.checksum(), r2.checksum());
    }

    #[test]
    fn region_checksum_differs() {
        let mut r1 = DeformableTerrainRegion::new();
        r1.insert(make_pos(0, 0, 0, 0, 0, 0), make_cell(5.0, 0.0));

        let mut r2 = DeformableTerrainRegion::new();
        r2.insert(make_pos(0, 0, 0, 0, 0, 0), make_cell(5.1, 0.0));

        assert_ne!(r1.checksum(), r2.checksum());
    }

    #[test]
    fn region_fingerprint() {
        let mut region = DeformableTerrainRegion::new();
        region.insert(make_pos(0, 0, 0, 0, 0, 0), make_cell(5.0, 0.1));

        let fp = region.fingerprint();
        assert_eq!(fp.cell_count, 1);
        assert_ne!(fp.checksum, 0);
    }

    #[test]
    fn plan_stress_propagation_basic() {
        let mut region = DeformableTerrainRegion::new();
        region.insert(make_pos(0, 0, 0, 8, 8, 8), make_cell(10.0, 0.0));
        region.insert(make_pos(0, 0, 0, 9, 8, 8), make_cell(2.0, 0.0));

        let config = DeformableTerrainConfig::DEFAULT;
        let propagations = plan_stress_propagation(&region, &config);

        assert!(!propagations.is_empty());
        let prop = &propagations[0];
        assert!(prop.stress_transfer > 0.0);
        assert!(prop.source_stress > prop.target_stress);
    }

    #[test]
    fn plan_stress_propagation_deterministic() {
        let mut region = DeformableTerrainRegion::new();
        region.insert(make_pos(0, 0, 0, 5, 5, 5), make_cell(10.0, 0.0));
        region.insert(make_pos(0, 0, 0, 6, 5, 5), make_cell(2.0, 0.0));
        region.insert(make_pos(0, 0, 0, 5, 6, 5), make_cell(5.0, 0.0));

        let config = DeformableTerrainConfig::DEFAULT;
        let prop1 = plan_stress_propagation(&region, &config);
        let prop2 = plan_stress_propagation(&region, &config);

        assert_eq!(prop1.len(), prop2.len());
        for (p1, p2) in prop1.iter().zip(prop2.iter()) {
            assert_eq!(p1.source, p2.source);
            assert_eq!(p1.target, p2.target);
            assert!((p1.stress_transfer - p2.stress_transfer).abs() < 0.001);
        }
    }

    #[test]
    fn apply_stress_propagation_basic() {
        let mut region = DeformableTerrainRegion::new();
        region.insert(make_pos(0, 0, 0, 8, 8, 8), make_cell(10.0, 0.0));
        region.insert(make_pos(0, 0, 0, 9, 8, 8), make_cell(2.0, 0.0));

        let propagations = vec![StressPropagation::new(
            make_pos(0, 0, 0, 8, 8, 8),
            make_pos(0, 0, 0, 9, 8, 8),
            4.0,
            10.0,
            2.0,
        )];

        let config = DeformableTerrainConfig::DEFAULT;
        let (stress, cells, _, max_change) =
            apply_stress_propagation(&mut region, &propagations, &config);

        assert!(stress > 0.0);
        assert_eq!(cells, 2);
        assert!(max_change > 0.0);

        let high_after = region.get(make_pos(0, 0, 0, 8, 8, 8)).unwrap();
        let low_after = region.get(make_pos(0, 0, 0, 9, 8, 8)).unwrap();

        assert!(high_after.stress() < 10.0);
        assert!(low_after.stress() > 2.0);
    }

    #[test]
    fn check_fractures_threshold() {
        let mut region = DeformableTerrainRegion::new();

        let mut fractured_cell = TerrainCell::pristine();
        fractured_cell.apply_damage(DEFAULT_FRACTURE_THRESHOLD / MAX_FRACTURE_THRESHOLD + 0.01);
        region.insert(make_pos(0, 0, 0, 0, 0, 0), fractured_cell);
        region.insert(make_pos(0, 0, 0, 1, 0, 0), TerrainCell::pristine());

        let config = DeformableTerrainConfig::DEFAULT;
        let events = check_fractures(&region, &config);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].pos, make_pos(0, 0, 0, 0, 0, 0));
    }

    #[test]
    fn propagate_fractures_to_neighbors() {
        let mut region = DeformableTerrainRegion::new();

        let mut fractured_cell = TerrainCell::pristine();
        fractured_cell.apply_damage(0.5);
        region.insert(make_pos(0, 0, 0, 8, 8, 8), fractured_cell);
        region.insert(make_pos(0, 0, 0, 9, 8, 8), TerrainCell::pristine());

        let events = vec![FractureEvent::new(
            make_pos(0, 0, 0, 8, 8, 8),
            0.5,
            5.0,
            false,
        )];

        let config = DeformableTerrainConfig {
            fracture_propagation_factor: 0.5,
            ..DeformableTerrainConfig::DEFAULT
        };

        propagate_fractures(&mut region, &events, &config);

        let neighbor = region.get(make_pos(0, 0, 0, 9, 8, 8)).unwrap();
        assert!(neighbor.damage() > 0.0);
    }

    #[test]
    fn cross_chunk_disabled() {
        let mut region = DeformableTerrainRegion::new();
        region.insert(make_pos(0, 0, 0, 0, 0, 0), make_cell(10.0, 0.0));
        region.insert(make_pos(-1, 0, 0, 15, 0, 0), make_cell(2.0, 0.0));

        let mut config = DeformableTerrainConfig::DEFAULT;
        config.cross_chunk_enabled = false;

        let propagations = plan_stress_propagation(&region, &config);
        assert!(propagations.is_empty());
    }

    #[test]
    fn deformable_terrain_step_integration() {
        let mut region = DeformableTerrainRegion::new();
        region.insert(make_pos(0, 0, 0, 8, 8, 8), make_cell(10.0, 0.0));
        region.insert(make_pos(0, 0, 0, 9, 8, 8), make_cell(2.0, 0.0));

        let config = DeformableTerrainConfig::DEFAULT;
        let result = deformable_terrain_step(&mut region, &config);

        assert!(result.has_changes());
        assert!(result.stress_transferred > 0.0);
        assert!(result.cells_updated > 0);
    }

    #[test]
    fn deformable_terrain_step_deterministic() {
        let make_region = || {
            let mut region = DeformableTerrainRegion::new();
            region.insert(make_pos(0, 0, 0, 8, 8, 8), make_cell(10.0, 0.0));
            region.insert(make_pos(0, 0, 0, 9, 8, 8), make_cell(2.0, 0.0));
            region.insert(make_pos(0, 0, 0, 8, 9, 8), make_cell(5.0, 0.0));
            region
        };

        let config = DeformableTerrainConfig::DEFAULT;

        let mut r1 = make_region();
        let res1 = deformable_terrain_step(&mut r1, &config);

        let mut r2 = make_region();
        let res2 = deformable_terrain_step(&mut r2, &config);

        assert_eq!(r1.checksum(), r2.checksum());
        assert_eq!(res1.checksum(), res2.checksum());
    }

    #[test]
    fn min_stress_diff_threshold() {
        let mut region = DeformableTerrainRegion::new();
        region.insert(make_pos(0, 0, 0, 8, 8, 8), make_cell(5.0, 0.0));
        region.insert(make_pos(0, 0, 0, 9, 8, 8), make_cell(4.95, 0.0));

        let config = DeformableTerrainConfig::DEFAULT;
        let propagations = plan_stress_propagation(&region, &config);

        assert!(propagations.is_empty());
    }

    #[test]
    fn serde_cell_round_trip() {
        let cell = make_cell(7.5, 0.25);
        let json = serde_json::to_string(&cell).unwrap();
        let recovered: TerrainCell = serde_json::from_str(&json).unwrap();
        assert!((recovered.stress() - cell.stress()).abs() < 0.001);
        assert!((recovered.damage() - cell.damage()).abs() < 0.001);
    }

    #[test]
    fn serde_region_round_trip() {
        let mut region = DeformableTerrainRegion::new();
        region.insert(make_pos(0, 0, 0, 0, 0, 0), make_cell(5.0, 0.1));
        region.insert(make_pos(0, 0, 0, 1, 0, 0), make_cell(10.0, 0.2));

        let json = serde_json::to_string(&region).unwrap();
        let recovered: DeformableTerrainRegion = serde_json::from_str(&json).unwrap();

        assert_eq!(recovered.len(), region.len());
        assert_eq!(recovered.checksum(), region.checksum());
    }

    #[test]
    fn serde_config_round_trip() {
        let config = DeformableTerrainConfig::BRITTLE;
        let json = serde_json::to_string(&config).unwrap();
        let recovered: DeformableTerrainConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, config);
    }

    #[test]
    fn serde_result_round_trip() {
        let result = DeformableTerrainResult {
            propagations: vec![StressPropagation::new(
                make_pos(0, 0, 0, 0, 0, 0),
                make_pos(0, 0, 0, 1, 0, 0),
                4.0,
                10.0,
                2.0,
            )],
            fracture_events: vec![],
            stress_transferred: 4.0,
            cells_updated: 2,
            cross_chunk_propagations: 0,
            max_stress_change: 2.0,
            new_fractures: 0,
        };

        let json = serde_json::to_string(&result).unwrap();
        let recovered: DeformableTerrainResult = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.propagations.len(), result.propagations.len());
        assert!((recovered.stress_transferred - result.stress_transferred).abs() < 0.001);
    }

    #[test]
    fn empty_region_operations() {
        let region = DeformableTerrainRegion::new();
        assert!(region.is_empty());
        assert_eq!(region.len(), 0);
        assert!(region.active_chunks().is_empty());

        let summary = region.summary();
        assert_eq!(summary.cell_count, 0);

        let validation = region.validate();
        assert!(validation.is_valid);
    }

    #[test]
    fn result_checksum_deterministic() {
        let result1 = DeformableTerrainResult {
            propagations: vec![StressPropagation::new(
                make_pos(0, 0, 0, 0, 0, 0),
                make_pos(0, 0, 0, 1, 0, 0),
                4.0,
                10.0,
                2.0,
            )],
            fracture_events: vec![],
            stress_transferred: 4.0,
            cells_updated: 2,
            cross_chunk_propagations: 0,
            max_stress_change: 2.0,
            new_fractures: 0,
        };

        let result2 = DeformableTerrainResult {
            propagations: vec![StressPropagation::new(
                make_pos(0, 0, 0, 0, 0, 0),
                make_pos(0, 0, 0, 1, 0, 0),
                4.0,
                10.0,
                2.0,
            )],
            fracture_events: vec![],
            stress_transferred: 4.0,
            cells_updated: 2,
            cross_chunk_propagations: 0,
            max_stress_change: 2.0,
            new_fractures: 0,
        };

        assert_eq!(result1.checksum(), result2.checksum());
    }

    #[test]
    fn bincode_region_round_trip() {
        let mut region = DeformableTerrainRegion::new();
        region.insert(make_pos(0, 0, 0, 0, 0, 0), make_cell(5.0, 0.1));
        region.insert(make_pos(0, 0, 0, 1, 0, 0), make_cell(10.0, 0.2));

        let encoded = bincode::serialize(&region).unwrap();
        let decoded: DeformableTerrainRegion = bincode::deserialize(&encoded).unwrap();

        assert_eq!(decoded.len(), region.len());
        assert_eq!(decoded.checksum(), region.checksum());
    }

    #[test]
    fn stress_relaxation() {
        let mut region = DeformableTerrainRegion::new();
        region.insert(make_pos(0, 0, 0, 0, 0, 0), make_cell(10.0, 0.0));

        let config = DeformableTerrainConfig {
            stress_relaxation_rate: 0.1,
            ..DeformableTerrainConfig::DEFAULT
        };

        apply_stress_relaxation(&mut region, &config);

        let cell = region.get(make_pos(0, 0, 0, 0, 0, 0)).unwrap();
        assert!(cell.stress() < 10.0);
        assert!((cell.stress() - 9.0).abs() < 0.001);
    }

    #[test]
    fn damage_accumulation_above_yield() {
        let mut cell = TerrainCell::pristine();
        cell.apply_stress(10.0);

        let damage_increment = cell.compute_damage_increment(5.0);
        assert!(damage_increment > 0.0);

        let no_damage = cell.compute_damage_increment(15.0);
        assert!((no_damage - 0.0).abs() < 0.001);
    }

    #[test]
    fn cross_chunk_counting() {
        let mut region = DeformableTerrainRegion::new();
        region.insert(make_pos(0, 0, 0, 15, 8, 8), make_cell(10.0, 0.0));
        region.insert(make_pos(1, 0, 0, 0, 8, 8), make_cell(2.0, 0.0));

        let config = DeformableTerrainConfig::DEFAULT;
        let result = deformable_terrain_step(&mut region, &config);

        assert!(result.cross_chunk_propagations > 0);
    }

    #[test]
    fn count_in_chunk() {
        let mut region = DeformableTerrainRegion::new();
        region.insert(make_pos(0, 0, 0, 0, 0, 0), make_cell(5.0, 0.0));
        region.insert(make_pos(0, 0, 0, 1, 0, 0), make_cell(5.0, 0.0));
        region.insert(make_pos(1, 0, 0, 0, 0, 0), make_cell(5.0, 0.0));

        assert_eq!(region.count_in_chunk(ChunkPos::new(0, 0, 0)), 2);
        assert_eq!(region.count_in_chunk(ChunkPos::new(1, 0, 0)), 1);
        assert_eq!(region.count_in_chunk(ChunkPos::new(2, 0, 0)), 0);
    }

    #[test]
    fn link_operations() {
        let mut region = DeformableTerrainRegion::new();
        region.insert(make_pos(0, 0, 0, 0, 0, 0), make_cell(5.0, 0.0));
        region.insert(make_pos(0, 0, 0, 1, 0, 0), make_cell(5.0, 0.0));

        region.add_link(FractureLink::intact(
            make_pos(0, 0, 0, 0, 0, 0),
            make_pos(0, 0, 0, 1, 0, 0),
        ));

        assert_eq!(region.links().len(), 1);

        let validation = region.validate();
        assert!(validation.is_valid);
    }

    #[test]
    fn link_validation_missing_cells() {
        let mut region = DeformableTerrainRegion::new();
        region.insert(make_pos(0, 0, 0, 0, 0, 0), make_cell(5.0, 0.0));

        region.add_link(FractureLink::intact(
            make_pos(0, 0, 0, 0, 0, 0),
            make_pos(0, 0, 0, 5, 0, 0),
        ));

        let validation = region.validate();
        assert!(!validation.is_valid);
        assert!(validation.issues.iter().any(|s| s.contains("target")));
    }
}
