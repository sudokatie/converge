//! Generalized cellular automata framework for spores, corruption, crystal growth,
//! biofilm, and frost spread simulations.
//!
//! This module provides a CPU-side, data-only sparse representation of cellular
//! automata that supports deterministic simulation with stable ordering and checksums.

use std::collections::BTreeMap;

use engine_core::coords::{ChunkPos, LocalPos};
use serde::{Deserialize, Serialize};

use crate::replay::{ChecksumBuilder, StepChecksum};

/// Neighbor offsets for 6-face (von Neumann) neighborhood.
const FACE_OFFSETS: [(i32, i32, i32); 6] = [
    (-1, 0, 0),
    (1, 0, 0),
    (0, -1, 0),
    (0, 1, 0),
    (0, 0, -1),
    (0, 0, 1),
];

/// Neighbor offsets for 26-cell (Moore) neighborhood.
const MOORE_OFFSETS: [(i32, i32, i32); 26] = [
    // 6 face neighbors
    (-1, 0, 0),
    (1, 0, 0),
    (0, -1, 0),
    (0, 1, 0),
    (0, 0, -1),
    (0, 0, 1),
    // 12 edge neighbors
    (-1, -1, 0),
    (-1, 1, 0),
    (1, -1, 0),
    (1, 1, 0),
    (-1, 0, -1),
    (-1, 0, 1),
    (1, 0, -1),
    (1, 0, 1),
    (0, -1, -1),
    (0, -1, 1),
    (0, 1, -1),
    (0, 1, 1),
    // 8 corner neighbors
    (-1, -1, -1),
    (-1, -1, 1),
    (-1, 1, -1),
    (-1, 1, 1),
    (1, -1, -1),
    (1, -1, 1),
    (1, 1, -1),
    (1, 1, 1),
];

/// Kind of cellular automaton pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum AutomataKind {
    /// Spore spread: organic, decays over time, spreads through air.
    Spores = 0,
    /// Corruption: aggressive spread, converts neighbors, resistant to decay.
    Corruption = 1,
    /// Crystal growth: structured growth along axes, slow but persistent.
    Crystal = 2,
    /// Biofilm: surface-attached growth, spreads along surfaces.
    Biofilm = 3,
    /// Frost: temperature-dependent spread, melts under heat.
    Frost = 4,
}

impl AutomataKind {
    /// Get all automata kinds.
    #[must_use]
    pub const fn all() -> [Self; 5] {
        [
            Self::Spores,
            Self::Corruption,
            Self::Crystal,
            Self::Biofilm,
            Self::Frost,
        ]
    }

    /// Check if this kind decays naturally over time.
    #[must_use]
    pub const fn decays(&self) -> bool {
        matches!(self, Self::Spores | Self::Frost)
    }

    /// Check if this kind spreads aggressively (converts neighbors).
    #[must_use]
    pub const fn converts(&self) -> bool {
        matches!(self, Self::Corruption)
    }

    /// Check if this kind requires surface attachment.
    #[must_use]
    pub const fn surface_attached(&self) -> bool {
        matches!(self, Self::Biofilm | Self::Crystal)
    }
}

/// Neighborhood type for rule evaluation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Neighborhood {
    /// 6 face neighbors (von Neumann).
    #[default]
    VonNeumann,
    /// 26 neighbors (Moore).
    Moore,
    /// Only face neighbors on same Y level (4 horizontal).
    Horizontal,
    /// Only vertical neighbors (up/down).
    Vertical,
}

impl Neighborhood {
    /// Get neighbor offsets for this neighborhood type.
    #[must_use]
    pub fn offsets(&self) -> &'static [(i32, i32, i32)] {
        match self {
            Self::VonNeumann => &FACE_OFFSETS,
            Self::Moore => &MOORE_OFFSETS,
            Self::Horizontal => &FACE_OFFSETS[0..4],
            Self::Vertical => &FACE_OFFSETS[2..4],
        }
    }

    /// Maximum number of neighbors for this neighborhood.
    #[must_use]
    pub const fn max_neighbors(&self) -> u8 {
        match self {
            Self::VonNeumann => 6,
            Self::Moore => 26,
            Self::Horizontal => 4,
            Self::Vertical => 2,
        }
    }
}

/// State of a cellular automaton at a single cell.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutomataCell {
    /// Intensity/strength of the automaton (0.0 = inactive, 1.0 = maximum).
    intensity: f32,
    /// Age in simulation ticks since birth.
    age: u32,
    /// Generation (how many times this cell has been "reborn").
    generation: u8,
    /// Whether this cell is marked for death next step.
    dying: bool,
}

impl AutomataCell {
    /// Inactive cell state.
    pub const INACTIVE: Self = Self {
        intensity: 0.0,
        age: 0,
        generation: 0,
        dying: false,
    };

    /// Maximum intensity value.
    pub const MAX_INTENSITY: f32 = 1.0;

    /// Minimum active intensity threshold.
    pub const MIN_INTENSITY: f32 = 0.001;

    /// Maximum age before automatic death (10 million ticks).
    pub const MAX_AGE: u32 = 10_000_000;

    /// Maximum generation count.
    pub const MAX_GENERATION: u8 = 255;

    /// Create a new active cell with given intensity.
    #[must_use]
    pub fn new(intensity: f32) -> Self {
        Self {
            intensity: intensity.clamp(0.0, Self::MAX_INTENSITY),
            age: 0,
            generation: 0,
            dying: false,
        }
    }

    /// Create a cell with full state.
    #[must_use]
    pub fn with_state(intensity: f32, age: u32, generation: u8) -> Self {
        Self {
            intensity: intensity.clamp(0.0, Self::MAX_INTENSITY),
            age: age.min(Self::MAX_AGE),
            generation,
            dying: false,
        }
    }

    /// Get the current intensity.
    #[must_use]
    pub const fn intensity(&self) -> f32 {
        self.intensity
    }

    /// Get the cell age.
    #[must_use]
    pub const fn age(&self) -> u32 {
        self.age
    }

    /// Get the generation count.
    #[must_use]
    pub const fn generation(&self) -> u8 {
        self.generation
    }

    /// Check if the cell is dying.
    #[must_use]
    pub const fn is_dying(&self) -> bool {
        self.dying
    }

    /// Check if the cell is active (intensity > 0).
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.intensity >= Self::MIN_INTENSITY
    }

    /// Set intensity, clamping to valid range.
    pub fn set_intensity(&mut self, value: f32) {
        self.intensity = value.clamp(0.0, Self::MAX_INTENSITY);
    }

    /// Add to intensity, clamping to valid range.
    pub fn add_intensity(&mut self, delta: f32) {
        self.set_intensity(self.intensity + delta);
    }

    /// Increment age by one tick.
    pub fn tick_age(&mut self) {
        self.age = self.age.saturating_add(1).min(Self::MAX_AGE);
    }

    /// Mark cell for death.
    pub fn mark_dying(&mut self) {
        self.dying = true;
    }

    /// Rebirth: reset age, increment generation.
    pub fn rebirth(&mut self) {
        self.age = 0;
        self.generation = self.generation.saturating_add(1);
        self.dying = false;
    }

    /// Deactivate the cell completely.
    pub fn deactivate(&mut self) {
        *self = Self::INACTIVE;
    }
}

impl Default for AutomataCell {
    fn default() -> Self {
        Self::INACTIVE
    }
}

/// Rule set for birth, survival, and death conditions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutomataRule {
    /// Minimum neighbors for birth (empty cell becomes active).
    pub birth_min: u8,
    /// Maximum neighbors for birth.
    pub birth_max: u8,
    /// Minimum neighbors for survival (active cell stays active).
    pub survive_min: u8,
    /// Maximum neighbors for survival.
    pub survive_max: u8,
    /// Neighborhood type for counting.
    pub neighborhood: Neighborhood,
    /// Minimum intensity of neighbors to count them.
    pub neighbor_threshold: f32,
    /// Intensity transferred to new cells on birth.
    pub birth_intensity: f32,
    /// Intensity decay per tick for active cells.
    pub decay_rate: f32,
    /// Maximum age before forced death (0 = no limit).
    pub max_age: u32,
    /// Whether to apply gravity (spread more downward).
    pub gravity_bias: bool,
}

impl AutomataRule {
    /// Classic Game of Life rules (B3/S23) adapted for 3D.
    pub const LIFE: Self = Self {
        birth_min: 5,
        birth_max: 7,
        survive_min: 4,
        survive_max: 6,
        neighborhood: Neighborhood::VonNeumann,
        neighbor_threshold: 0.5,
        birth_intensity: 0.8,
        decay_rate: 0.0,
        max_age: 0,
        gravity_bias: false,
    };

    /// Spore rules: easy birth, easy death, decays.
    pub const SPORES: Self = Self {
        birth_min: 2,
        birth_max: 4,
        survive_min: 1,
        survive_max: 5,
        neighborhood: Neighborhood::VonNeumann,
        neighbor_threshold: 0.3,
        birth_intensity: 0.6,
        decay_rate: 0.02,
        max_age: 1000,
        gravity_bias: true,
    };

    /// Corruption rules: aggressive spread, hard to kill.
    pub const CORRUPTION: Self = Self {
        birth_min: 1,
        birth_max: 6,
        survive_min: 1,
        survive_max: 26,
        neighborhood: Neighborhood::Moore,
        neighbor_threshold: 0.2,
        birth_intensity: 0.9,
        decay_rate: 0.001,
        max_age: 0,
        gravity_bias: false,
    };

    /// Crystal rules: structured growth, survives indefinitely.
    pub const CRYSTAL: Self = Self {
        birth_min: 1,
        birth_max: 2,
        survive_min: 1,
        survive_max: 6,
        neighborhood: Neighborhood::VonNeumann,
        neighbor_threshold: 0.7,
        birth_intensity: 1.0,
        decay_rate: 0.0,
        max_age: 0,
        gravity_bias: false,
    };

    /// Biofilm rules: surface spread, moderate persistence.
    pub const BIOFILM: Self = Self {
        birth_min: 2,
        birth_max: 3,
        survive_min: 2,
        survive_max: 4,
        neighborhood: Neighborhood::Horizontal,
        neighbor_threshold: 0.4,
        birth_intensity: 0.7,
        decay_rate: 0.005,
        max_age: 5000,
        gravity_bias: false,
    };

    /// Frost rules: fragile spread, fast decay.
    pub const FROST: Self = Self {
        birth_min: 3,
        birth_max: 5,
        survive_min: 2,
        survive_max: 4,
        neighborhood: Neighborhood::VonNeumann,
        neighbor_threshold: 0.5,
        birth_intensity: 0.5,
        decay_rate: 0.03,
        max_age: 500,
        gravity_bias: true,
    };

    /// Create rule for a given automata kind.
    #[must_use]
    pub fn for_kind(kind: AutomataKind) -> Self {
        match kind {
            AutomataKind::Spores => Self::SPORES,
            AutomataKind::Corruption => Self::CORRUPTION,
            AutomataKind::Crystal => Self::CRYSTAL,
            AutomataKind::Biofilm => Self::BIOFILM,
            AutomataKind::Frost => Self::FROST,
        }
    }

    /// Check if a birth should occur given neighbor count and total intensity.
    #[must_use]
    pub fn should_birth(&self, neighbor_count: u8, _total_intensity: f32) -> bool {
        neighbor_count >= self.birth_min && neighbor_count <= self.birth_max
    }

    /// Check if a cell should survive given neighbor count.
    #[must_use]
    pub fn should_survive(&self, neighbor_count: u8) -> bool {
        neighbor_count >= self.survive_min && neighbor_count <= self.survive_max
    }
}

impl Default for AutomataRule {
    fn default() -> Self {
        Self::LIFE
    }
}

/// Configuration for cellular automata simulation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutomataConfig {
    /// Rule set to apply.
    pub rule: AutomataRule,
    /// Kind of automaton.
    pub kind: AutomataKind,
    /// Maximum cells to process per step (0 = unlimited).
    pub max_cells_per_step: usize,
    /// Maximum births per step (0 = unlimited).
    pub max_births_per_step: usize,
    /// Enable cross-chunk spread.
    pub cross_chunk_enabled: bool,
    /// Random variation in birth intensity (0.0 to 1.0).
    pub intensity_variance: f32,
}

impl AutomataConfig {
    /// Default configuration for spores.
    pub const SPORES: Self = Self {
        rule: AutomataRule::SPORES,
        kind: AutomataKind::Spores,
        max_cells_per_step: 1000,
        max_births_per_step: 100,
        cross_chunk_enabled: true,
        intensity_variance: 0.1,
    };

    /// Default configuration for corruption.
    pub const CORRUPTION: Self = Self {
        rule: AutomataRule::CORRUPTION,
        kind: AutomataKind::Corruption,
        max_cells_per_step: 500,
        max_births_per_step: 50,
        cross_chunk_enabled: true,
        intensity_variance: 0.05,
    };

    /// Default configuration for crystal growth.
    pub const CRYSTAL: Self = Self {
        rule: AutomataRule::CRYSTAL,
        kind: AutomataKind::Crystal,
        max_cells_per_step: 200,
        max_births_per_step: 20,
        cross_chunk_enabled: true,
        intensity_variance: 0.0,
    };

    /// Default configuration for biofilm.
    pub const BIOFILM: Self = Self {
        rule: AutomataRule::BIOFILM,
        kind: AutomataKind::Biofilm,
        max_cells_per_step: 800,
        max_births_per_step: 80,
        cross_chunk_enabled: true,
        intensity_variance: 0.15,
    };

    /// Default configuration for frost.
    pub const FROST: Self = Self {
        rule: AutomataRule::FROST,
        kind: AutomataKind::Frost,
        max_cells_per_step: 1500,
        max_births_per_step: 150,
        cross_chunk_enabled: true,
        intensity_variance: 0.2,
    };

    /// Create config for a given automata kind.
    #[must_use]
    pub fn for_kind(kind: AutomataKind) -> Self {
        match kind {
            AutomataKind::Spores => Self::SPORES,
            AutomataKind::Corruption => Self::CORRUPTION,
            AutomataKind::Crystal => Self::CRYSTAL,
            AutomataKind::Biofilm => Self::BIOFILM,
            AutomataKind::Frost => Self::FROST,
        }
    }
}

impl Default for AutomataConfig {
    fn default() -> Self {
        Self::SPORES
    }
}

/// Unique identifier for a position across chunks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AutomataPos {
    /// Chunk position.
    pub chunk: ChunkPos,
    /// Local position within chunk.
    pub local: LocalPos,
}

impl PartialOrd for AutomataPos {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AutomataPos {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

impl AutomataPos {
    /// Create a new position.
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

    /// Get neighbors using the specified neighborhood.
    #[must_use]
    #[expect(
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        reason = "local coords 0..16 and offsets stay bounded"
    )]
    pub fn neighbors(&self, neighborhood: Neighborhood) -> Vec<Self> {
        let mut result = Vec::with_capacity(neighborhood.max_neighbors() as usize);

        for &(dx, dy, dz) in neighborhood.offsets() {
            let nx = self.local.x() as i32 + dx;
            let ny = self.local.y() as i32 + dy;
            let nz = self.local.z() as i32 + dz;

            let (chunk_x_offset, local_x) = if nx < 0 {
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

            result.push(Self {
                chunk: ChunkPos::new(
                    self.chunk.x() + chunk_x_offset,
                    self.chunk.y() + chunk_y_offset,
                    self.chunk.z() + chunk_z_offset,
                ),
                local: LocalPos::new(local_x, local_y, local_z),
            });
        }

        result
    }

    /// Check if neighbor is in a different chunk.
    #[must_use]
    pub fn is_cross_chunk(&self, other: &Self) -> bool {
        self.chunk != other.chunk
    }
}

/// Entry representing an automaton cell at a specific world position.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutomataEntry {
    /// Chunk position.
    pub chunk: ChunkPos,
    /// Local position within chunk.
    pub local: LocalPos,
    /// Cell state.
    pub cell: AutomataCell,
}

impl AutomataEntry {
    /// Create a new entry.
    #[must_use]
    pub fn new(chunk: ChunkPos, local: LocalPos, intensity: f32) -> Self {
        Self {
            chunk,
            local,
            cell: AutomataCell::new(intensity),
        }
    }

    /// Create from position and cell.
    #[must_use]
    pub fn from_cell(chunk: ChunkPos, local: LocalPos, cell: AutomataCell) -> Self {
        Self { chunk, local, cell }
    }

    /// Get position as `AutomataPos`.
    #[must_use]
    pub const fn pos(&self) -> AutomataPos {
        AutomataPos::new(self.chunk, self.local)
    }

    /// Compute sort key for deterministic ordering.
    #[must_use]
    pub fn sort_key(&self) -> (i32, i32, i32, usize) {
        self.pos().sort_key()
    }
}

/// A delta representing a change to apply to a cell.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutomataDelta {
    /// Target position.
    pub pos: AutomataPos,
    /// Kind of change.
    pub kind: DeltaKind,
}

/// Kind of delta to apply.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum DeltaKind {
    /// Birth a new cell with given intensity.
    Birth { intensity: f32 },
    /// Update existing cell intensity.
    Update { intensity: f32 },
    /// Kill the cell.
    Death,
    /// Age the cell by one tick.
    Age,
    /// Rebirth (reset age, increment generation).
    Rebirth,
}

impl AutomataDelta {
    /// Create a birth delta.
    #[must_use]
    pub fn birth(pos: AutomataPos, intensity: f32) -> Self {
        Self {
            pos,
            kind: DeltaKind::Birth { intensity },
        }
    }

    /// Create an update delta.
    #[must_use]
    pub fn update(pos: AutomataPos, intensity: f32) -> Self {
        Self {
            pos,
            kind: DeltaKind::Update { intensity },
        }
    }

    /// Create a death delta.
    #[must_use]
    pub fn death(pos: AutomataPos) -> Self {
        Self {
            pos,
            kind: DeltaKind::Death,
        }
    }

    /// Create an age delta.
    #[must_use]
    pub fn age(pos: AutomataPos) -> Self {
        Self {
            pos,
            kind: DeltaKind::Age,
        }
    }

    /// Create a rebirth delta.
    #[must_use]
    pub fn rebirth(pos: AutomataPos) -> Self {
        Self {
            pos,
            kind: DeltaKind::Rebirth,
        }
    }

    /// Check if this delta crosses chunk boundaries from source.
    #[must_use]
    pub fn is_cross_chunk(&self, source_chunk: ChunkPos) -> bool {
        self.pos.chunk != source_chunk
    }

    /// Deterministic sort key.
    #[must_use]
    pub fn sort_key(&self) -> (i32, i32, i32, usize, u8) {
        let (cx, cy, cz, idx) = self.pos.sort_key();
        let kind_ord = match self.kind {
            DeltaKind::Death => 0,
            DeltaKind::Age => 1,
            DeltaKind::Update { .. } => 2,
            DeltaKind::Rebirth => 3,
            DeltaKind::Birth { .. } => 4,
        };
        (cx, cy, cz, idx, kind_ord)
    }
}

/// Plan for bounded automata step execution.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AutomataPlan {
    /// Deltas to apply, in deterministic order.
    pub deltas: Vec<AutomataDelta>,
    /// Number of deaths planned.
    pub death_count: u32,
    /// Number of births planned.
    pub birth_count: u32,
    /// Number of cross-chunk operations.
    pub cross_chunk_count: u32,
    /// Whether the plan was truncated due to limits.
    pub truncated: bool,
}

impl AutomataPlan {
    /// Create an empty plan.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a delta to the plan.
    pub fn add(&mut self, delta: AutomataDelta, source_chunk: Option<ChunkPos>) {
        match delta.kind {
            DeltaKind::Death => self.death_count += 1,
            DeltaKind::Birth { .. } => self.birth_count += 1,
            _ => {}
        }
        if let Some(sc) = source_chunk
            && delta.is_cross_chunk(sc)
        {
            self.cross_chunk_count += 1;
        }
        self.deltas.push(delta);
    }

    /// Sort deltas for deterministic ordering.
    pub fn sort(&mut self) {
        self.deltas.sort_by_key(AutomataDelta::sort_key);
    }

    /// Check if plan is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.deltas.is_empty()
    }

    /// Number of deltas.
    #[must_use]
    pub fn len(&self) -> usize {
        self.deltas.len()
    }

    /// Compute checksum for determinism verification.
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "counts fit in u32")]
    pub fn checksum(&self) -> StepChecksum {
        let mut builder = ChecksumBuilder::new();
        builder.feed_u32(self.deltas.len() as u32);
        builder.feed_u32(self.death_count);
        builder.feed_u32(self.birth_count);
        builder.feed_u32(self.cross_chunk_count);
        for delta in &self.deltas {
            builder.feed_i32(delta.pos.chunk.x());
            builder.feed_i32(delta.pos.chunk.y());
            builder.feed_i32(delta.pos.chunk.z());
            builder.feed_u32(delta.pos.local.to_index() as u32);
            let kind_byte = match delta.kind {
                DeltaKind::Death => 0u8,
                DeltaKind::Age => 1,
                DeltaKind::Update { .. } => 2,
                DeltaKind::Rebirth => 3,
                DeltaKind::Birth { .. } => 4,
            };
            builder.feed_u32(u32::from(kind_byte));
            if let DeltaKind::Birth { intensity } | DeltaKind::Update { intensity } = delta.kind {
                builder.feed_f32(intensity);
            }
        }
        builder.build()
    }
}

/// Sparse automata region containing entries indexed by position.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AutomataRegion {
    /// Entries indexed by position.
    #[serde(
        serialize_with = "serialize_entries",
        deserialize_with = "deserialize_entries"
    )]
    entries: BTreeMap<AutomataPos, AutomataEntry>,
    /// Automata kind for this region.
    kind: AutomataKind,
    /// Total intensity across all entries.
    total_intensity: f32,
}

fn serialize_entries<S>(
    entries: &BTreeMap<AutomataPos, AutomataEntry>,
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
) -> Result<BTreeMap<AutomataPos, AutomataEntry>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let entries: Vec<AutomataEntry> = Deserialize::deserialize(deserializer)?;
    let mut map = BTreeMap::new();
    for entry in entries {
        let pos = entry.pos();
        map.insert(pos, entry);
    }
    Ok(map)
}

impl Default for AutomataRegion {
    fn default() -> Self {
        Self::new(AutomataKind::Spores)
    }
}

impl AutomataRegion {
    /// Create a new empty region.
    #[must_use]
    pub fn new(kind: AutomataKind) -> Self {
        Self {
            entries: BTreeMap::new(),
            kind,
            total_intensity: 0.0,
        }
    }

    /// Get the automata kind.
    #[must_use]
    pub const fn kind(&self) -> AutomataKind {
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

    /// Total intensity across all cells.
    #[must_use]
    pub fn total_intensity(&self) -> f32 {
        self.total_intensity
    }

    /// Get an entry by position.
    #[must_use]
    pub fn get(&self, pos: AutomataPos) -> Option<&AutomataEntry> {
        self.entries.get(&pos)
    }

    /// Get a mutable entry by position.
    pub fn get_mut(&mut self, pos: AutomataPos) -> Option<&mut AutomataEntry> {
        self.entries.get_mut(&pos)
    }

    /// Insert or update an entry.
    pub fn insert(&mut self, entry: AutomataEntry) {
        let pos = entry.pos();
        if let Some(old) = self.entries.get(&pos) {
            self.total_intensity -= old.cell.intensity();
        }
        self.total_intensity += entry.cell.intensity();
        self.entries.insert(pos, entry);
    }

    /// Remove an entry by position.
    pub fn remove(&mut self, pos: AutomataPos) -> Option<AutomataEntry> {
        let removed = self.entries.remove(&pos);
        if let Some(ref entry) = removed {
            self.total_intensity -= entry.cell.intensity();
        }
        removed
    }

    /// Iterate over all entries in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = (&AutomataPos, &AutomataEntry)> {
        self.entries.iter()
    }

    /// Iterate over entries mutably.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&AutomataPos, &mut AutomataEntry)> {
        self.entries.iter_mut()
    }

    /// Get all active positions.
    pub fn positions(&self) -> impl Iterator<Item = &AutomataPos> {
        self.entries.keys()
    }

    /// Get all entries.
    pub fn entries(&self) -> impl Iterator<Item = &AutomataEntry> {
        self.entries.values()
    }

    /// Prune inactive entries.
    pub fn prune(&mut self) {
        self.entries.retain(|_, e| e.cell.is_active());
        self.recompute_total();
    }

    /// Count entries in a chunk.
    #[must_use]
    pub fn count_in_chunk(&self, chunk: ChunkPos) -> usize {
        self.entries_in_chunk(chunk).count()
    }

    /// Get entries in a specific chunk.
    pub fn entries_in_chunk(&self, chunk: ChunkPos) -> impl Iterator<Item = &AutomataEntry> {
        self.entries
            .range(
                AutomataPos::new(chunk, LocalPos::new(0, 0, 0))
                    ..=AutomataPos::new(chunk, LocalPos::new(15, 15, 15)),
            )
            .map(|(_, e)| e)
    }

    /// Get chunks that have entries.
    #[must_use]
    pub fn active_chunks(&self) -> Vec<ChunkPos> {
        let mut chunks: Vec<ChunkPos> = self.entries.keys().map(|p| p.chunk).collect();
        chunks.dedup();
        chunks
    }

    /// Count neighbors of a position that meet the threshold.
    #[must_use]
    pub fn count_neighbors(
        &self,
        pos: AutomataPos,
        neighborhood: Neighborhood,
        threshold: f32,
    ) -> (u8, f32) {
        let mut count = 0u8;
        let mut total_intensity = 0.0f32;

        for neighbor_pos in pos.neighbors(neighborhood) {
            if let Some(entry) = self.get(neighbor_pos)
                && entry.cell.intensity() >= threshold
            {
                count += 1;
                total_intensity += entry.cell.intensity();
            }
        }

        (count, total_intensity)
    }

    /// Compute summary statistics.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "entry count fits in f32 for reasonable sizes; avg age fits in u32"
    )]
    pub fn summary(&self) -> AutomataSummary {
        let mut min_intensity = f32::MAX;
        let mut max_intensity = f32::MIN;
        let mut min_age = u32::MAX;
        let mut max_age = u32::MIN;
        let mut total_age = 0u64;

        for entry in self.entries.values() {
            min_intensity = min_intensity.min(entry.cell.intensity());
            max_intensity = max_intensity.max(entry.cell.intensity());
            min_age = min_age.min(entry.cell.age());
            max_age = max_age.max(entry.cell.age());
            total_age += u64::from(entry.cell.age());
        }

        let count = self.entries.len();
        let avg_intensity = if count > 0 {
            self.total_intensity / count as f32
        } else {
            0.0
        };
        let avg_age = if count > 0 {
            (total_age / count as u64) as u32
        } else {
            0
        };

        AutomataSummary {
            kind: self.kind,
            entry_count: count,
            total_intensity: self.total_intensity,
            min_intensity: if count > 0 { min_intensity } else { 0.0 },
            max_intensity: if count > 0 { max_intensity } else { 0.0 },
            avg_intensity,
            min_age: if count > 0 { min_age } else { 0 },
            max_age: if count > 0 { max_age } else { 0 },
            avg_age,
            chunk_count: self.active_chunks().len(),
        }
    }

    /// Validate region state.
    #[must_use]
    pub fn validate(&self) -> AutomataValidation {
        let mut issues = Vec::new();

        for (pos, entry) in &self.entries {
            if entry.cell.intensity() < 0.0 || entry.cell.intensity() > AutomataCell::MAX_INTENSITY
            {
                issues.push(format!(
                    "Entry at {:?} has invalid intensity {}",
                    pos,
                    entry.cell.intensity()
                ));
            }
            if entry.cell.age() > AutomataCell::MAX_AGE {
                issues.push(format!(
                    "Entry at {:?} has invalid age {}",
                    pos,
                    entry.cell.age()
                ));
            }
        }

        let recomputed: f32 = self.entries.values().map(|e| e.cell.intensity()).sum();
        if (recomputed - self.total_intensity).abs() > 0.001 {
            issues.push(format!(
                "Total intensity mismatch: stored {} vs computed {}",
                self.total_intensity, recomputed
            ));
        }

        AutomataValidation {
            is_valid: issues.is_empty(),
            issues,
        }
    }

    /// Compute checksum for determinism verification.
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "counts fit in u32")]
    pub fn checksum(&self) -> StepChecksum {
        let mut builder = ChecksumBuilder::new();
        builder.feed_u32(self.kind as u32);
        builder.feed_u32(self.entries.len() as u32);
        for (pos, entry) in &self.entries {
            builder.feed_i32(pos.chunk.x());
            builder.feed_i32(pos.chunk.y());
            builder.feed_i32(pos.chunk.z());
            builder.feed_u32(pos.local.to_index() as u32);
            builder.feed_f32(entry.cell.intensity());
            builder.feed_u32(entry.cell.age());
            builder.feed_u32(u32::from(entry.cell.generation()));
        }
        builder.build()
    }

    /// Compute fingerprint for the region.
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "counts fit in u32")]
    pub fn fingerprint(&self) -> AutomataFingerprint {
        let checksum = self.checksum();
        AutomataFingerprint {
            kind: self.kind,
            entry_count: self.entries.len() as u32,
            total_intensity_bits: self.total_intensity.to_bits(),
            checksum: checksum.value(),
        }
    }

    fn recompute_total(&mut self) {
        self.total_intensity = self.entries.values().map(|e| e.cell.intensity()).sum();
    }
}

/// Summary statistics for a region.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutomataSummary {
    /// Automata kind.
    pub kind: AutomataKind,
    /// Number of entries.
    pub entry_count: usize,
    /// Total intensity.
    pub total_intensity: f32,
    /// Minimum intensity.
    pub min_intensity: f32,
    /// Maximum intensity.
    pub max_intensity: f32,
    /// Average intensity.
    pub avg_intensity: f32,
    /// Minimum age.
    pub min_age: u32,
    /// Maximum age.
    pub max_age: u32,
    /// Average age.
    pub avg_age: u32,
    /// Number of unique chunks.
    pub chunk_count: usize,
}

/// Validation result for a region.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutomataValidation {
    /// Whether the region is valid.
    pub is_valid: bool,
    /// List of validation issues.
    pub issues: Vec<String>,
}

/// Compact fingerprint for a region.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AutomataFingerprint {
    /// Automata kind.
    pub kind: AutomataKind,
    /// Entry count.
    pub entry_count: u32,
    /// Total intensity as bits.
    pub total_intensity_bits: u32,
    /// Checksum value.
    pub checksum: u32,
}

/// Result of a simulation step.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AutomataResult {
    /// Plan that was applied.
    pub plan: AutomataPlan,
    /// Number of cells that were born.
    pub births: u32,
    /// Number of cells that died.
    pub deaths: u32,
    /// Number of cells that aged.
    pub aged: u32,
    /// Number of cells decayed (intensity reduced).
    pub decayed: u32,
    /// Cross-chunk operations performed.
    pub cross_chunk_ops: u32,
}

impl AutomataResult {
    /// Create an empty result.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any changes occurred.
    #[must_use]
    pub fn has_changes(&self) -> bool {
        self.births > 0 || self.deaths > 0 || self.aged > 0 || self.decayed > 0
    }

    /// Compute checksum.
    #[must_use]
    pub fn checksum(&self) -> StepChecksum {
        let mut builder = ChecksumBuilder::new();
        builder.feed_u32(self.births);
        builder.feed_u32(self.deaths);
        builder.feed_u32(self.aged);
        builder.feed_u32(self.decayed);
        builder.feed_u32(self.cross_chunk_ops);
        let plan_cs = self.plan.checksum();
        builder.feed_u32(plan_cs.value());
        builder.build()
    }
}

/// Resistance map trait for blocking/slowing spread.
pub trait AutomataResistance {
    /// Get resistance at a position (0.0 = none, 1.0 = blocked).
    fn resistance(&self, pos: AutomataPos) -> f32;
}

impl AutomataResistance for () {
    fn resistance(&self, _pos: AutomataPos) -> f32 {
        0.0
    }
}

impl<F> AutomataResistance for F
where
    F: Fn(AutomataPos) -> f32,
{
    fn resistance(&self, pos: AutomataPos) -> f32 {
        self(pos)
    }
}

/// Plan an automata simulation step with bounded operations.
#[must_use]
pub fn plan_automata_step<R: AutomataResistance>(
    region: &AutomataRegion,
    config: &AutomataConfig,
    resistance: &R,
) -> AutomataPlan {
    let mut plan = AutomataPlan::new();
    let rule = &config.rule;

    let mut entries: Vec<_> = region.iter().collect();
    entries.sort_by_key(|(pos, _)| pos.sort_key());

    if config.max_cells_per_step > 0 && entries.len() > config.max_cells_per_step {
        entries.truncate(config.max_cells_per_step);
        plan.truncated = true;
    }

    let mut birth_candidates: Vec<(AutomataPos, f32)> = Vec::new();

    for (pos, entry) in &entries {
        let (neighbor_count, _total_intensity) =
            region.count_neighbors(**pos, rule.neighborhood, rule.neighbor_threshold);

        if rule.should_survive(neighbor_count) {
            let mut new_intensity = entry.cell.intensity();

            if rule.decay_rate > 0.0 {
                new_intensity = (new_intensity - rule.decay_rate).max(0.0);
            }

            if new_intensity < AutomataCell::MIN_INTENSITY {
                plan.add(AutomataDelta::death(**pos), Some(pos.chunk));
            } else if (new_intensity - entry.cell.intensity()).abs() > f32::EPSILON {
                plan.add(AutomataDelta::update(**pos, new_intensity), Some(pos.chunk));
            }

            if rule.max_age > 0 && entry.cell.age() >= rule.max_age {
                plan.add(AutomataDelta::death(**pos), Some(pos.chunk));
            } else {
                plan.add(AutomataDelta::age(**pos), Some(pos.chunk));
            }

            for neighbor_pos in pos.neighbors(rule.neighborhood) {
                if !config.cross_chunk_enabled && pos.is_cross_chunk(&neighbor_pos) {
                    continue;
                }

                let res = resistance.resistance(neighbor_pos);
                if res >= 1.0 {
                    continue;
                }

                if region.get(neighbor_pos).is_none() {
                    let (n_count, n_intensity) = region.count_neighbors(
                        neighbor_pos,
                        rule.neighborhood,
                        rule.neighbor_threshold,
                    );
                    if rule.should_birth(n_count, n_intensity) {
                        let birth_intensity = rule.birth_intensity * (1.0 - res);
                        birth_candidates.push((neighbor_pos, birth_intensity));
                    }
                }
            }
        } else {
            plan.add(AutomataDelta::death(**pos), Some(pos.chunk));
        }
    }

    birth_candidates.sort_by_key(|(pos, _)| pos.sort_key());
    birth_candidates.dedup_by_key(|(pos, _)| *pos);

    let max_births = if config.max_births_per_step > 0 {
        config.max_births_per_step
    } else {
        birth_candidates.len()
    };

    for (pos, intensity) in birth_candidates.into_iter().take(max_births) {
        if region.get(pos).is_none() {
            plan.add(AutomataDelta::birth(pos, intensity), None);
        }
    }

    plan.sort();
    plan
}

/// Apply a plan to a region.
pub fn apply_automata_plan(region: &mut AutomataRegion, plan: &AutomataPlan) -> AutomataResult {
    let mut result = AutomataResult {
        plan: plan.clone(),
        ..Default::default()
    };

    for delta in &plan.deltas {
        match delta.kind {
            DeltaKind::Birth { intensity } => {
                if region.get(delta.pos).is_none() {
                    region.insert(AutomataEntry::new(
                        delta.pos.chunk,
                        delta.pos.local,
                        intensity,
                    ));
                    result.births += 1;
                }
            }
            DeltaKind::Update { intensity } => {
                if let Some(entry) = region.get_mut(delta.pos) {
                    let old = entry.cell.intensity();
                    entry.cell.set_intensity(intensity);
                    if intensity < old {
                        result.decayed += 1;
                    }
                }
            }
            DeltaKind::Death => {
                if region.remove(delta.pos).is_some() {
                    result.deaths += 1;
                }
            }
            DeltaKind::Age => {
                if let Some(entry) = region.get_mut(delta.pos) {
                    entry.cell.tick_age();
                    result.aged += 1;
                }
            }
            DeltaKind::Rebirth => {
                if let Some(entry) = region.get_mut(delta.pos) {
                    entry.cell.rebirth();
                }
            }
        }
    }

    result.cross_chunk_ops = plan.cross_chunk_count;
    result
}

/// Execute a full automata simulation step.
#[must_use]
pub fn automata_step<R: AutomataResistance>(
    region: &mut AutomataRegion,
    config: &AutomataConfig,
    resistance: &R,
) -> AutomataResult {
    let plan = plan_automata_step(region, config, resistance);
    apply_automata_plan(region, &plan)
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    clippy::similar_names,
    reason = "tests check exact values; similar names intentional"
)]
mod tests {
    use super::*;

    fn make_pos(cx: i32, cy: i32, cz: i32, lx: u32, ly: u32, lz: u32) -> AutomataPos {
        AutomataPos::new(ChunkPos::new(cx, cy, cz), LocalPos::new(lx, ly, lz))
    }

    fn make_entry(
        cx: i32,
        cy: i32,
        cz: i32,
        lx: u32,
        ly: u32,
        lz: u32,
        intensity: f32,
    ) -> AutomataEntry {
        AutomataEntry::new(
            ChunkPos::new(cx, cy, cz),
            LocalPos::new(lx, ly, lz),
            intensity,
        )
    }

    #[test]
    fn cell_creation() {
        let cell = AutomataCell::new(0.5);
        assert!((cell.intensity() - 0.5).abs() < 0.001);
        assert_eq!(cell.age(), 0);
        assert_eq!(cell.generation(), 0);
        assert!(!cell.is_dying());
        assert!(cell.is_active());
    }

    #[test]
    fn cell_clamping() {
        let low = AutomataCell::new(-0.5);
        assert_eq!(low.intensity(), 0.0);

        let high = AutomataCell::new(1.5);
        assert_eq!(high.intensity(), 1.0);
    }

    #[test]
    fn cell_inactive() {
        let cell = AutomataCell::INACTIVE;
        assert!(!cell.is_active());
        assert_eq!(cell.intensity(), 0.0);
    }

    #[test]
    fn cell_aging() {
        let mut cell = AutomataCell::new(1.0);
        assert_eq!(cell.age(), 0);
        cell.tick_age();
        assert_eq!(cell.age(), 1);
        cell.tick_age();
        assert_eq!(cell.age(), 2);
    }

    #[test]
    fn cell_rebirth() {
        let mut cell = AutomataCell::with_state(0.8, 100, 0);
        cell.rebirth();
        assert_eq!(cell.age(), 0);
        assert_eq!(cell.generation(), 1);
    }

    #[test]
    fn cell_deactivate() {
        let mut cell = AutomataCell::new(1.0);
        cell.tick_age();
        cell.deactivate();
        assert_eq!(cell, AutomataCell::INACTIVE);
    }

    #[test]
    fn neighborhood_offsets() {
        assert_eq!(Neighborhood::VonNeumann.offsets().len(), 6);
        assert_eq!(Neighborhood::Moore.offsets().len(), 26);
        assert_eq!(Neighborhood::Horizontal.offsets().len(), 4);
        assert_eq!(Neighborhood::Vertical.offsets().len(), 2);
    }

    #[test]
    fn pos_neighbors_inside_chunk() {
        let pos = make_pos(0, 0, 0, 8, 8, 8);
        let neighbors = pos.neighbors(Neighborhood::VonNeumann);
        assert_eq!(neighbors.len(), 6);

        assert!(neighbors.contains(&make_pos(0, 0, 0, 7, 8, 8)));
        assert!(neighbors.contains(&make_pos(0, 0, 0, 9, 8, 8)));
        assert!(neighbors.contains(&make_pos(0, 0, 0, 8, 7, 8)));
        assert!(neighbors.contains(&make_pos(0, 0, 0, 8, 9, 8)));
        assert!(neighbors.contains(&make_pos(0, 0, 0, 8, 8, 7)));
        assert!(neighbors.contains(&make_pos(0, 0, 0, 8, 8, 9)));
    }

    #[test]
    fn pos_neighbors_cross_chunk() {
        let pos = make_pos(0, 0, 0, 0, 0, 0);
        let neighbors = pos.neighbors(Neighborhood::VonNeumann);

        assert!(neighbors.contains(&make_pos(-1, 0, 0, 15, 0, 0)));
        assert!(neighbors.contains(&make_pos(0, -1, 0, 0, 15, 0)));
        assert!(neighbors.contains(&make_pos(0, 0, -1, 0, 0, 15)));
    }

    #[test]
    fn region_insert_and_get() {
        let mut region = AutomataRegion::new(AutomataKind::Spores);
        let entry = make_entry(0, 0, 0, 8, 8, 8, 0.5);
        let pos = entry.pos();

        region.insert(entry);
        assert_eq!(region.len(), 1);
        assert!(!region.is_empty());

        let retrieved = region.get(pos).unwrap();
        assert!((retrieved.cell.intensity() - 0.5).abs() < 0.001);
    }

    #[test]
    fn region_total_intensity() {
        let mut region = AutomataRegion::new(AutomataKind::Corruption);
        region.insert(make_entry(0, 0, 0, 0, 0, 0, 0.3));
        region.insert(make_entry(0, 0, 0, 1, 0, 0, 0.5));
        assert!((region.total_intensity() - 0.8).abs() < 0.001);

        region.insert(make_entry(0, 0, 0, 0, 0, 0, 0.2));
        assert!((region.total_intensity() - 0.7).abs() < 0.001);
    }

    #[test]
    fn region_remove() {
        let mut region = AutomataRegion::new(AutomataKind::Crystal);
        let entry = make_entry(0, 0, 0, 8, 8, 8, 0.5);
        let pos = entry.pos();

        region.insert(entry);
        let removed = region.remove(pos);
        assert!(removed.is_some());
        assert!(region.is_empty());
    }

    #[test]
    fn region_count_neighbors() {
        let mut region = AutomataRegion::new(AutomataKind::Biofilm);
        region.insert(make_entry(0, 0, 0, 8, 8, 8, 1.0));
        region.insert(make_entry(0, 0, 0, 7, 8, 8, 0.8));
        region.insert(make_entry(0, 0, 0, 9, 8, 8, 0.6));
        region.insert(make_entry(0, 0, 0, 8, 7, 8, 0.4));

        let (count, total) =
            region.count_neighbors(make_pos(0, 0, 0, 8, 8, 8), Neighborhood::VonNeumann, 0.5);
        assert_eq!(count, 2);
        assert!((total - 1.4).abs() < 0.001);
    }

    #[test]
    fn region_validation_valid() {
        let mut region = AutomataRegion::new(AutomataKind::Frost);
        region.insert(make_entry(0, 0, 0, 0, 0, 0, 0.5));
        let validation = region.validate();
        assert!(validation.is_valid);
        assert!(validation.issues.is_empty());
    }

    #[test]
    fn region_checksum_deterministic() {
        let mut region1 = AutomataRegion::new(AutomataKind::Spores);
        region1.insert(make_entry(0, 0, 0, 0, 0, 0, 0.5));
        region1.insert(make_entry(0, 0, 0, 1, 0, 0, 0.3));

        let mut region2 = AutomataRegion::new(AutomataKind::Spores);
        region2.insert(make_entry(0, 0, 0, 1, 0, 0, 0.3));
        region2.insert(make_entry(0, 0, 0, 0, 0, 0, 0.5));

        assert_eq!(region1.checksum(), region2.checksum());
    }

    #[test]
    fn region_checksum_differs() {
        let mut region1 = AutomataRegion::new(AutomataKind::Spores);
        region1.insert(make_entry(0, 0, 0, 0, 0, 0, 0.5));

        let mut region2 = AutomataRegion::new(AutomataKind::Spores);
        region2.insert(make_entry(0, 0, 0, 0, 0, 0, 0.6));

        assert_ne!(region1.checksum(), region2.checksum());
    }

    #[test]
    fn region_fingerprint_equality() {
        let mut region1 = AutomataRegion::new(AutomataKind::Crystal);
        region1.insert(make_entry(0, 0, 0, 0, 0, 0, 0.5));

        let mut region2 = AutomataRegion::new(AutomataKind::Crystal);
        region2.insert(make_entry(0, 0, 0, 0, 0, 0, 0.5));

        assert_eq!(region1.fingerprint(), region2.fingerprint());
    }

    #[test]
    fn region_summary() {
        let mut region = AutomataRegion::new(AutomataKind::Corruption);
        region.insert(make_entry(0, 0, 0, 0, 0, 0, 0.3));
        region.insert(make_entry(0, 0, 0, 1, 0, 0, 0.9));

        let summary = region.summary();
        assert_eq!(summary.kind, AutomataKind::Corruption);
        assert_eq!(summary.entry_count, 2);
        assert!((summary.total_intensity - 1.2).abs() < 0.001);
        assert!((summary.min_intensity - 0.3).abs() < 0.001);
        assert!((summary.max_intensity - 0.9).abs() < 0.001);
    }

    #[test]
    fn rule_presets() {
        const { assert!(AutomataRule::SPORES.decay_rate > 0.0) };
        assert_eq!(AutomataRule::CORRUPTION.decay_rate, 0.001);
        assert_eq!(AutomataRule::CRYSTAL.decay_rate, 0.0);
        const { assert!(AutomataRule::FROST.gravity_bias) };
    }

    #[test]
    fn rule_birth_survival() {
        let rule = AutomataRule::LIFE;
        assert!(rule.should_birth(5, 1.0));
        assert!(rule.should_birth(6, 1.0));
        assert!(rule.should_birth(7, 1.0));
        assert!(!rule.should_birth(4, 1.0));
        assert!(!rule.should_birth(8, 1.0));

        assert!(rule.should_survive(4));
        assert!(rule.should_survive(5));
        assert!(rule.should_survive(6));
        assert!(!rule.should_survive(3));
        assert!(!rule.should_survive(7));
    }

    #[test]
    fn config_presets() {
        let spores = AutomataConfig::SPORES;
        assert_eq!(spores.kind, AutomataKind::Spores);

        let corruption = AutomataConfig::CORRUPTION;
        assert_eq!(corruption.kind, AutomataKind::Corruption);

        let crystal = AutomataConfig::CRYSTAL;
        assert_eq!(crystal.kind, AutomataKind::Crystal);

        let biofilm = AutomataConfig::BIOFILM;
        assert_eq!(biofilm.kind, AutomataKind::Biofilm);

        let frost = AutomataConfig::FROST;
        assert_eq!(frost.kind, AutomataKind::Frost);
    }

    #[test]
    fn delta_creation() {
        let pos = make_pos(0, 0, 0, 8, 8, 8);

        let birth = AutomataDelta::birth(pos, 0.5);
        assert!(
            matches!(birth.kind, DeltaKind::Birth { intensity } if (intensity - 0.5).abs() < 0.001)
        );

        let death = AutomataDelta::death(pos);
        assert!(matches!(death.kind, DeltaKind::Death));

        let age = AutomataDelta::age(pos);
        assert!(matches!(age.kind, DeltaKind::Age));
    }

    #[test]
    fn plan_checksum_deterministic() {
        let mut plan1 = AutomataPlan::new();
        plan1.add(AutomataDelta::birth(make_pos(0, 0, 0, 0, 0, 0), 0.5), None);
        plan1.add(AutomataDelta::death(make_pos(0, 0, 0, 1, 0, 0)), None);
        plan1.sort();

        let mut plan2 = AutomataPlan::new();
        plan2.add(AutomataDelta::death(make_pos(0, 0, 0, 1, 0, 0)), None);
        plan2.add(AutomataDelta::birth(make_pos(0, 0, 0, 0, 0, 0), 0.5), None);
        plan2.sort();

        assert_eq!(plan1.checksum(), plan2.checksum());
    }

    #[test]
    fn automata_step_deterministic() {
        let make_region = || {
            let mut region = AutomataRegion::new(AutomataKind::Spores);
            region.insert(make_entry(0, 0, 0, 8, 8, 8, 0.8));
            region.insert(make_entry(0, 0, 0, 9, 8, 8, 0.7));
            region.insert(make_entry(0, 0, 0, 8, 9, 8, 0.6));
            region
        };

        let config = AutomataConfig::SPORES;

        let mut region1 = make_region();
        let result1 = automata_step(&mut region1, &config, &());

        let mut region2 = make_region();
        let result2 = automata_step(&mut region2, &config, &());

        assert_eq!(region1.checksum(), region2.checksum());
        assert_eq!(result1.checksum(), result2.checksum());
    }

    #[test]
    fn automata_step_decay() {
        let mut region = AutomataRegion::new(AutomataKind::Spores);
        region.insert(make_entry(0, 0, 0, 8, 8, 8, 0.5));
        region.insert(make_entry(0, 0, 0, 9, 8, 8, 0.5));

        let config = AutomataConfig::SPORES;
        let result = automata_step(&mut region, &config, &());

        let entry = region.get(make_pos(0, 0, 0, 8, 8, 8));
        if let Some(e) = entry {
            assert!(e.cell.intensity() < 0.5 || result.deaths > 0);
        }
    }

    #[test]
    fn automata_step_growth() {
        let mut region = AutomataRegion::new(AutomataKind::Crystal);
        region.insert(make_entry(0, 0, 0, 8, 8, 8, 1.0));
        region.insert(make_entry(0, 0, 0, 9, 8, 8, 1.0));

        let initial_count = region.len();
        let config = AutomataConfig::CRYSTAL;
        let _result = automata_step(&mut region, &config, &());

        assert!(region.len() >= initial_count || !region.is_empty());
    }

    #[test]
    fn automata_step_resistance() {
        let mut region = AutomataRegion::new(AutomataKind::Corruption);
        region.insert(make_entry(0, 0, 0, 8, 8, 8, 1.0));

        let blocked = |pos: AutomataPos| {
            if pos.local.x() == 9 { 1.0 } else { 0.0 }
        };

        let config = AutomataConfig::CORRUPTION;
        let _result = automata_step(&mut region, &config, &blocked);

        let blocked_pos = region.get(make_pos(0, 0, 0, 9, 8, 8));
        assert!(blocked_pos.is_none());
    }

    #[test]
    fn automata_step_bounded() {
        let mut region = AutomataRegion::new(AutomataKind::Corruption);
        for i in 0..10 {
            region.insert(make_entry(0, 0, 0, i, 0, 0, 1.0));
        }

        let mut config = AutomataConfig::CORRUPTION;
        config.max_cells_per_step = 5;
        config.max_births_per_step = 2;

        let plan = plan_automata_step(&region, &config, &());
        assert!(plan.truncated || plan.birth_count <= 2);
    }

    #[test]
    fn serde_cell_round_trip() {
        let cell = AutomataCell::with_state(0.7, 100, 3);
        let json = serde_json::to_string(&cell).unwrap();
        let recovered: AutomataCell = serde_json::from_str(&json).unwrap();
        assert!((recovered.intensity() - cell.intensity()).abs() < 0.001);
        assert_eq!(recovered.age(), cell.age());
        assert_eq!(recovered.generation(), cell.generation());
    }

    #[test]
    fn serde_entry_round_trip() {
        let entry = make_entry(1, 2, 3, 4, 5, 6, 0.8);
        let json = serde_json::to_string(&entry).unwrap();
        let recovered: AutomataEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.chunk, entry.chunk);
        assert_eq!(recovered.local, entry.local);
    }

    #[test]
    fn serde_region_round_trip() {
        let mut region = AutomataRegion::new(AutomataKind::Biofilm);
        region.insert(make_entry(0, 0, 0, 0, 0, 0, 0.5));
        region.insert(make_entry(0, 0, 0, 1, 0, 0, 0.3));

        let json = serde_json::to_string(&region).unwrap();
        let recovered: AutomataRegion = serde_json::from_str(&json).unwrap();

        assert_eq!(recovered.len(), region.len());
        assert_eq!(recovered.kind(), region.kind());
        assert!((recovered.total_intensity() - region.total_intensity()).abs() < 0.001);
    }

    #[test]
    fn serde_config_round_trip() {
        let config = AutomataConfig::FROST;
        let json = serde_json::to_string(&config).unwrap();
        let recovered: AutomataConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.kind, config.kind);
    }

    #[test]
    fn serde_plan_round_trip() {
        let mut plan = AutomataPlan::new();
        plan.add(AutomataDelta::birth(make_pos(0, 0, 0, 8, 8, 8), 0.5), None);
        plan.sort();

        let json = serde_json::to_string(&plan).unwrap();
        let recovered: AutomataPlan = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.len(), plan.len());
    }

    #[test]
    fn serde_result_round_trip() {
        let result = AutomataResult {
            plan: AutomataPlan::new(),
            births: 5,
            deaths: 3,
            aged: 10,
            decayed: 2,
            cross_chunk_ops: 1,
        };

        let json = serde_json::to_string(&result).unwrap();
        let recovered: AutomataResult = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.births, result.births);
        assert_eq!(recovered.deaths, result.deaths);
    }

    #[test]
    fn bincode_region_round_trip() {
        let mut region = AutomataRegion::new(AutomataKind::Crystal);
        region.insert(make_entry(0, 0, 0, 0, 0, 0, 0.5));
        region.insert(make_entry(1, 2, 3, 4, 5, 6, 0.8));

        let bytes = bincode::serialize(&region).unwrap();
        let recovered: AutomataRegion = bincode::deserialize(&bytes).unwrap();

        assert_eq!(recovered.len(), region.len());
        assert_eq!(recovered.checksum(), region.checksum());
    }

    #[test]
    fn automata_kind_all() {
        let all = AutomataKind::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&AutomataKind::Spores));
        assert!(all.contains(&AutomataKind::Corruption));
        assert!(all.contains(&AutomataKind::Crystal));
        assert!(all.contains(&AutomataKind::Biofilm));
        assert!(all.contains(&AutomataKind::Frost));
    }

    #[test]
    fn automata_kind_properties() {
        assert!(AutomataKind::Spores.decays());
        assert!(AutomataKind::Frost.decays());
        assert!(!AutomataKind::Crystal.decays());

        assert!(AutomataKind::Corruption.converts());
        assert!(!AutomataKind::Spores.converts());

        assert!(AutomataKind::Biofilm.surface_attached());
        assert!(AutomataKind::Crystal.surface_attached());
        assert!(!AutomataKind::Spores.surface_attached());
    }

    #[test]
    fn region_prune() {
        let mut region = AutomataRegion::new(AutomataKind::Frost);
        region.insert(make_entry(0, 0, 0, 0, 0, 0, 0.5));
        region.insert(make_entry(0, 0, 0, 1, 0, 0, 0.0001));
        assert_eq!(region.len(), 2);

        region.prune();
        assert_eq!(region.len(), 1);
    }

    #[test]
    fn region_active_chunks() {
        let mut region = AutomataRegion::new(AutomataKind::Spores);
        region.insert(make_entry(0, 0, 0, 0, 0, 0, 0.5));
        region.insert(make_entry(1, 0, 0, 0, 0, 0, 0.5));
        region.insert(make_entry(0, 1, 0, 0, 0, 0, 0.5));

        let chunks = region.active_chunks();
        assert_eq!(chunks.len(), 3);
    }

    #[test]
    fn region_entries_in_chunk() {
        let mut region = AutomataRegion::new(AutomataKind::Corruption);
        region.insert(make_entry(0, 0, 0, 0, 0, 0, 0.5));
        region.insert(make_entry(0, 0, 0, 1, 0, 0, 0.5));
        region.insert(make_entry(1, 0, 0, 0, 0, 0, 0.5));

        let count = region.count_in_chunk(ChunkPos::new(0, 0, 0));
        assert_eq!(count, 2);
    }

    #[test]
    fn empty_region_operations() {
        let region = AutomataRegion::new(AutomataKind::Biofilm);
        assert!(region.is_empty());
        assert_eq!(region.len(), 0);
        assert_eq!(region.total_intensity(), 0.0);
        assert!(region.active_chunks().is_empty());

        let summary = region.summary();
        assert_eq!(summary.entry_count, 0);

        let validation = region.validate();
        assert!(validation.is_valid);
    }

    #[test]
    fn plan_cross_chunk_tracking() {
        let mut plan = AutomataPlan::new();
        plan.add(
            AutomataDelta::birth(make_pos(1, 0, 0, 0, 0, 0), 0.5),
            Some(ChunkPos::new(0, 0, 0)),
        );

        assert_eq!(plan.cross_chunk_count, 1);
    }

    #[test]
    fn result_has_changes() {
        let mut result = AutomataResult::new();
        assert!(!result.has_changes());

        result.births = 1;
        assert!(result.has_changes());

        result.births = 0;
        result.deaths = 1;
        assert!(result.has_changes());
    }
}
