//! Per-chunk fluid storage with lazy allocation.

use engine_core::coords::LocalPos;
use serde::de::{Deserializer, SeqAccess, Visitor};
use serde::ser::{SerializeTuple, Serializer};
use serde::{Deserialize, Serialize};

use crate::chunk::CHUNK_VOLUME;

use super::{FluidCell, FluidKind};

/// Per-kind fluid layer storage.
pub struct FluidLayer {
    cells: Box<[FluidCell; CHUNK_VOLUME]>,
    active_count: u32,
}

impl FluidLayer {
    /// Create a new empty layer.
    #[must_use]
    #[expect(
        clippy::large_stack_arrays,
        reason = "Copy type array initialization, optimized by compiler"
    )]
    pub fn new() -> Self {
        Self {
            cells: Box::new([FluidCell::EMPTY; CHUNK_VOLUME]),
            active_count: 0,
        }
    }

    /// Get cell at position.
    #[must_use]
    pub fn get(&self, pos: LocalPos) -> FluidCell {
        self.cells[pos.to_index()]
    }

    /// Get mutable reference to cell.
    pub fn get_mut(&mut self, pos: LocalPos) -> &mut FluidCell {
        &mut self.cells[pos.to_index()]
    }

    /// Set cell at position.
    pub fn set(&mut self, pos: LocalPos, cell: FluidCell) {
        let idx = pos.to_index();
        let was_active = !self.cells[idx].is_empty();
        let is_active = !cell.is_empty();

        self.cells[idx] = cell;

        match (was_active, is_active) {
            (false, true) => self.active_count += 1,
            (true, false) => self.active_count = self.active_count.saturating_sub(1),
            _ => {}
        }
    }

    /// Add volume to cell, returns overflow.
    pub fn add(&mut self, pos: LocalPos, volume: f32, kind: FluidKind) -> f32 {
        let idx = pos.to_index();
        let was_active = !self.cells[idx].is_empty();

        if self.cells[idx].is_empty() {
            self.cells[idx] = FluidCell::new(kind, 0.0);
        }
        let overflow = self.cells[idx].add_volume(volume);

        let is_active = !self.cells[idx].is_empty();
        match (was_active, is_active) {
            (false, true) => self.active_count += 1,
            (true, false) => self.active_count = self.active_count.saturating_sub(1),
            _ => {}
        }

        overflow
    }

    /// Remove volume from cell, returns amount removed.
    pub fn remove(&mut self, pos: LocalPos, amount: f32) -> f32 {
        let idx = pos.to_index();
        let was_active = !self.cells[idx].is_empty();
        let removed = self.cells[idx].remove_volume(amount);
        let is_active = !self.cells[idx].is_empty();

        if was_active && !is_active {
            self.active_count = self.active_count.saturating_sub(1);
        }

        removed
    }

    /// Fill all cells with the same state.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "CHUNK_VOLUME (4096) fits in u32"
    )]
    pub fn fill(&mut self, cell: FluidCell) {
        self.cells.fill(cell);
        self.active_count = if cell.is_empty() {
            0
        } else {
            CHUNK_VOLUME as u32
        };
    }

    /// Clear all cells to empty.
    pub fn clear(&mut self) {
        self.cells.fill(FluidCell::EMPTY);
        self.active_count = 0;
    }

    /// Number of non-empty cells.
    #[must_use]
    pub const fn active_count(&self) -> u32 {
        self.active_count
    }

    /// Check if layer has any fluid.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.active_count == 0
    }

    /// Iterate over active (non-empty) cells.
    pub fn iter_active(&self) -> impl Iterator<Item = (LocalPos, FluidCell)> + '_ {
        self.cells
            .iter()
            .enumerate()
            .filter(|(_, c)| !c.is_empty())
            .map(|(i, c)| (LocalPos::from_index(i), *c))
    }

    /// Direct access to cells slice.
    #[must_use]
    pub fn cells(&self) -> &[FluidCell; CHUNK_VOLUME] {
        &self.cells
    }

    /// Mutable access to cells slice.
    pub fn cells_mut(&mut self) -> &mut [FluidCell; CHUNK_VOLUME] {
        &mut self.cells
    }

    /// Trilinear interpolation sample.
    #[must_use]
    #[expect(
        clippy::similar_names,
        reason = "trilinear interpolation uses standard naming c000..c111 and c00..c11"
    )]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "coordinates clamped to [0, 15.999] guarantee safe u32 conversion"
    )]
    pub fn sample(&self, x: f32, y: f32, z: f32) -> FluidSample {
        let x = x.clamp(0.0, 15.999);
        let y = y.clamp(0.0, 15.999);
        let z = z.clamp(0.0, 15.999);

        let x0 = x.floor() as u32;
        let y0 = y.floor() as u32;
        let z0 = z.floor() as u32;

        let x1 = (x0 + 1).min(15);
        let y1 = (y0 + 1).min(15);
        let z1 = (z0 + 1).min(15);

        let fx = x.fract();
        let fy = y.fract();
        let fz = z.fract();

        let idx = |x: u32, y: u32, z: u32| -> usize { (x + y * 16 + z * 256) as usize };

        let c000 = &self.cells[idx(x0, y0, z0)];
        let c100 = &self.cells[idx(x1, y0, z0)];
        let c010 = &self.cells[idx(x0, y1, z0)];
        let c110 = &self.cells[idx(x1, y1, z0)];
        let c001 = &self.cells[idx(x0, y0, z1)];
        let c101 = &self.cells[idx(x1, y0, z1)];
        let c011 = &self.cells[idx(x0, y1, z1)];
        let c111 = &self.cells[idx(x1, y1, z1)];

        let interp = |f: fn(&FluidCell) -> f32| -> f32 {
            let v000 = f(c000);
            let v100 = f(c100);
            let v010 = f(c010);
            let v110 = f(c110);
            let v001 = f(c001);
            let v101 = f(c101);
            let v011 = f(c011);
            let v111 = f(c111);

            let c00 = v000 * (1.0 - fx) + v100 * fx;
            let c01 = v001 * (1.0 - fx) + v101 * fx;
            let c10 = v010 * (1.0 - fx) + v110 * fx;
            let c11 = v011 * (1.0 - fx) + v111 * fx;

            let c0 = c00 * (1.0 - fy) + c10 * fy;
            let c1 = c01 * (1.0 - fy) + c11 * fy;

            c0 * (1.0 - fz) + c1 * fz
        };

        let nearest_idx = idx(
            if fx < 0.5 { x0 } else { x1 },
            if fy < 0.5 { y0 } else { y1 },
            if fz < 0.5 { z0 } else { z1 },
        );

        FluidSample {
            kind: self.cells[nearest_idx].kind(),
            volume: interp(FluidCell::volume),
            pressure: interp(FluidCell::pressure),
            temperature: interp(FluidCell::temperature),
        }
    }

    /// Recount active cells (use after bulk operations).
    #[expect(
        clippy::cast_possible_truncation,
        reason = "CHUNK_VOLUME (4096) fits in u32"
    )]
    pub fn recount(&mut self) {
        self.active_count = self.cells.iter().filter(|c| !c.is_empty()).count() as u32;
    }
}

impl Default for FluidLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for FluidLayer {
    fn clone(&self) -> Self {
        Self {
            cells: self.cells.clone(),
            active_count: self.active_count,
        }
    }
}

impl std::fmt::Debug for FluidLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FluidLayer")
            .field("active_count", &self.active_count)
            .finish_non_exhaustive()
    }
}

impl Serialize for FluidLayer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(self.cells.as_slice())?;
        tuple.serialize_element(&self.active_count)?;
        tuple.end()
    }
}

impl<'de> Deserialize<'de> for FluidLayer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct LayerVisitor;

        impl<'de> Visitor<'de> for LayerVisitor {
            type Value = FluidLayer;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a tuple of [cells, active_count]")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let cells_vec: Vec<FluidCell> = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;

                let active_count: u32 = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;

                if cells_vec.len() != CHUNK_VOLUME {
                    return Err(serde::de::Error::invalid_length(
                        cells_vec.len(),
                        &"4096 FluidCell values",
                    ));
                }

                #[expect(
                    clippy::large_stack_arrays,
                    reason = "Copy type array initialization, optimized by compiler"
                )]
                let mut cells = Box::new([FluidCell::EMPTY; CHUNK_VOLUME]);
                cells.copy_from_slice(&cells_vec);

                Ok(FluidLayer {
                    cells,
                    active_count,
                })
            }
        }

        deserializer.deserialize_tuple(2, LayerVisitor)
    }
}

/// Interpolated fluid sample.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FluidSample {
    /// Nearest cell's fluid kind.
    pub kind: FluidKind,
    /// Interpolated volume.
    pub volume: f32,
    /// Interpolated pressure.
    pub pressure: f32,
    /// Interpolated temperature.
    pub temperature: f32,
}

impl FluidSample {
    /// Empty sample.
    pub const EMPTY: Self = Self {
        kind: FluidKind::Water,
        volume: 0.0,
        pressure: 1.0,
        temperature: 20.0,
    };
}

/// Per-chunk fluid storage with lazy per-kind layers.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ChunkFluids {
    layers: [Option<FluidLayer>; FluidKind::COUNT],
}

impl ChunkFluids {
    /// Create empty fluid storage.
    #[must_use]
    pub fn new() -> Self {
        Self {
            layers: std::array::from_fn(|_| None),
        }
    }

    /// Check if a layer is allocated.
    #[must_use]
    pub fn has_layer(&self, kind: FluidKind) -> bool {
        self.layers[kind.as_index()].is_some()
    }

    /// Get cell at position for a fluid kind.
    #[must_use]
    pub fn get(&self, kind: FluidKind, pos: LocalPos) -> FluidCell {
        match &self.layers[kind.as_index()] {
            Some(layer) => layer.get(pos),
            None => FluidCell::EMPTY,
        }
    }

    /// Set cell at position.
    pub fn set(&mut self, kind: FluidKind, pos: LocalPos, cell: FluidCell) {
        let layer = self.ensure_layer(kind);
        layer.set(pos, cell);
    }

    /// Add volume at position.
    pub fn add(&mut self, kind: FluidKind, pos: LocalPos, volume: f32) -> f32 {
        let layer = self.ensure_layer(kind);
        layer.add(pos, volume, kind)
    }

    /// Remove volume at position.
    pub fn remove(&mut self, kind: FluidKind, pos: LocalPos, amount: f32) -> f32 {
        match &mut self.layers[kind.as_index()] {
            Some(layer) => layer.remove(pos, amount),
            None => 0.0,
        }
    }

    /// Fill a layer with uniform state.
    pub fn fill(&mut self, kind: FluidKind, cell: FluidCell) {
        let layer = self.ensure_layer(kind);
        layer.fill(cell);
    }

    /// Clear a specific layer.
    pub fn clear_layer(&mut self, kind: FluidKind) {
        self.layers[kind.as_index()] = None;
    }

    /// Clear all layers.
    pub fn clear(&mut self) {
        for layer in &mut self.layers {
            *layer = None;
        }
    }

    /// Get layer reference.
    #[must_use]
    pub fn layer(&self, kind: FluidKind) -> Option<&FluidLayer> {
        self.layers[kind.as_index()].as_ref()
    }

    /// Get mutable layer reference.
    pub fn layer_mut(&mut self, kind: FluidKind) -> &mut FluidLayer {
        self.ensure_layer(kind)
    }

    /// Sample with trilinear interpolation.
    #[must_use]
    pub fn sample(&self, kind: FluidKind, x: f32, y: f32, z: f32) -> FluidSample {
        match &self.layers[kind.as_index()] {
            Some(layer) => layer.sample(x, y, z),
            None => FluidSample::EMPTY,
        }
    }

    /// Count of active cells across a layer.
    #[must_use]
    pub fn count(&self, kind: FluidKind) -> u32 {
        match &self.layers[kind.as_index()] {
            Some(layer) => layer.active_count(),
            None => 0,
        }
    }

    /// Total active cells across all layers.
    #[must_use]
    pub fn total_count(&self) -> u32 {
        self.layers
            .iter()
            .filter_map(|l| l.as_ref())
            .map(FluidLayer::active_count)
            .sum()
    }

    /// Number of allocated layers.
    #[must_use]
    pub fn allocated_count(&self) -> usize {
        self.layers.iter().filter(|l| Option::is_some(l)).count()
    }

    /// Check if all layers are empty/unallocated.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.layers.iter().all(Option::is_none)
    }

    fn ensure_layer(&mut self, kind: FluidKind) -> &mut FluidLayer {
        let idx = kind.as_index();
        if self.layers[idx].is_none() {
            self.layers[idx] = Some(FluidLayer::new());
        }
        self.layers[idx].as_mut().expect("just allocated")
    }
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    clippy::cast_possible_truncation,
    reason = "tests check exact values; CHUNK_VOLUME fits in u32"
)]
mod tests {
    use super::*;

    #[test]
    fn new_is_empty() {
        let fluids = ChunkFluids::new();
        assert!(fluids.is_empty());
        assert_eq!(fluids.allocated_count(), 0);
    }

    #[test]
    fn unallocated_returns_empty() {
        let fluids = ChunkFluids::new();
        let cell = fluids.get(FluidKind::Water, LocalPos::new(8, 8, 8));
        assert!(cell.is_empty());
    }

    #[test]
    fn set_allocates_layer() {
        let mut fluids = ChunkFluids::new();
        assert!(!fluids.has_layer(FluidKind::Water));

        fluids.set(
            FluidKind::Water,
            LocalPos::new(0, 0, 0),
            FluidCell::new(FluidKind::Water, 0.5),
        );

        assert!(fluids.has_layer(FluidKind::Water));
        assert_eq!(fluids.allocated_count(), 1);
    }

    #[test]
    fn set_and_get() {
        let mut fluids = ChunkFluids::new();
        let pos = LocalPos::new(5, 10, 3);
        let cell = FluidCell::new(FluidKind::Lava, 0.75);

        fluids.set(FluidKind::Lava, pos, cell);

        let retrieved = fluids.get(FluidKind::Lava, pos);
        assert_eq!(retrieved.kind(), FluidKind::Lava);
        assert!((retrieved.volume() - 0.75).abs() < 0.001);
    }

    #[test]
    fn add_volume() {
        let mut fluids = ChunkFluids::new();
        let pos = LocalPos::new(5, 5, 5);

        fluids.add(FluidKind::Water, pos, 0.3);
        assert!((fluids.get(FluidKind::Water, pos).volume() - 0.3).abs() < 0.001);

        fluids.add(FluidKind::Water, pos, 0.2);
        assert!((fluids.get(FluidKind::Water, pos).volume() - 0.5).abs() < 0.001);
    }

    #[test]
    fn add_volume_overflow() {
        let mut fluids = ChunkFluids::new();
        let pos = LocalPos::new(5, 5, 5);

        fluids.add(FluidKind::Water, pos, 0.8);
        let overflow = fluids.add(FluidKind::Water, pos, 0.5);

        assert!((overflow - 0.3).abs() < 0.001);
        assert!((fluids.get(FluidKind::Water, pos).volume() - 1.0).abs() < 0.001);
    }

    #[test]
    fn remove_volume() {
        let mut fluids = ChunkFluids::new();
        let pos = LocalPos::new(5, 5, 5);

        fluids.set(FluidKind::Water, pos, FluidCell::new(FluidKind::Water, 0.5));
        let removed = fluids.remove(FluidKind::Water, pos, 0.3);

        assert!((removed - 0.3).abs() < 0.001);
        assert!((fluids.get(FluidKind::Water, pos).volume() - 0.2).abs() < 0.001);
    }

    #[test]
    fn remove_from_unallocated() {
        let mut fluids = ChunkFluids::new();
        let removed = fluids.remove(FluidKind::Water, LocalPos::new(0, 0, 0), 1.0);
        assert_eq!(removed, 0.0);
    }

    #[test]
    fn fill_layer() {
        let mut fluids = ChunkFluids::new();
        fluids.fill(FluidKind::Gas, FluidCell::filled(FluidKind::Gas));

        assert!(fluids.has_layer(FluidKind::Gas));
        assert_eq!(fluids.count(FluidKind::Gas), CHUNK_VOLUME as u32);
    }

    #[test]
    fn clear_layer() {
        let mut fluids = ChunkFluids::new();
        fluids.fill(FluidKind::Water, FluidCell::filled(FluidKind::Water));
        assert!(fluids.has_layer(FluidKind::Water));

        fluids.clear_layer(FluidKind::Water);
        assert!(!fluids.has_layer(FluidKind::Water));
    }

    #[test]
    fn count_tracking() {
        let mut fluids = ChunkFluids::new();

        fluids.set(
            FluidKind::Water,
            LocalPos::new(0, 0, 0),
            FluidCell::new(FluidKind::Water, 0.5),
        );
        fluids.set(
            FluidKind::Water,
            LocalPos::new(1, 0, 0),
            FluidCell::new(FluidKind::Water, 0.5),
        );

        assert_eq!(fluids.count(FluidKind::Water), 2);
        assert_eq!(fluids.total_count(), 2);

        fluids.set(FluidKind::Water, LocalPos::new(0, 0, 0), FluidCell::EMPTY);
        assert_eq!(fluids.count(FluidKind::Water), 1);
    }

    #[test]
    fn sample_interpolation() {
        let mut fluids = ChunkFluids::new();

        fluids.set(
            FluidKind::Water,
            LocalPos::new(0, 0, 0),
            FluidCell::with_state(FluidKind::Water, 0.0, 1.0, 20.0),
        );
        fluids.set(
            FluidKind::Water,
            LocalPos::new(1, 0, 0),
            FluidCell::with_state(FluidKind::Water, 1.0, 1.0, 20.0),
        );

        let sample = fluids.sample(FluidKind::Water, 0.5, 0.0, 0.0);
        assert!((sample.volume - 0.5).abs() < 0.1);
    }

    #[test]
    fn sample_unallocated() {
        let fluids = ChunkFluids::new();
        let sample = fluids.sample(FluidKind::Water, 8.0, 8.0, 8.0);
        assert_eq!(sample.volume, 0.0);
    }

    #[test]
    fn layer_iter_active() {
        let mut fluids = ChunkFluids::new();

        fluids.set(
            FluidKind::Water,
            LocalPos::new(1, 2, 3),
            FluidCell::new(FluidKind::Water, 0.5),
        );
        fluids.set(
            FluidKind::Water,
            LocalPos::new(4, 5, 6),
            FluidCell::new(FluidKind::Water, 0.7),
        );

        let active: Vec<_> = fluids
            .layer(FluidKind::Water)
            .unwrap()
            .iter_active()
            .collect();
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn serde_round_trip_empty() {
        let fluids = ChunkFluids::new();
        let json = serde_json::to_string(&fluids).unwrap();
        let recovered: ChunkFluids = serde_json::from_str(&json).unwrap();
        assert!(recovered.is_empty());
    }

    #[test]
    fn serde_round_trip_with_data() {
        let mut fluids = ChunkFluids::new();
        fluids.set(
            FluidKind::Lava,
            LocalPos::new(5, 5, 5),
            FluidCell::with_state(FluidKind::Lava, 0.8, 2.0, 1100.0),
        );

        let json = serde_json::to_string(&fluids).unwrap();
        let recovered: ChunkFluids = serde_json::from_str(&json).unwrap();

        assert!(recovered.has_layer(FluidKind::Lava));
        let cell = recovered.get(FluidKind::Lava, LocalPos::new(5, 5, 5));
        assert!((cell.volume() - 0.8).abs() < 0.001);
        assert!((cell.temperature() - 1100.0).abs() < 0.001);
    }

    #[test]
    fn serde_invalid_length() {
        let bad_json = r#"[[{"volume":0.5,"pressure":1.0,"temperature":20.0,"kind":"Water"}],0]"#;
        let result: Result<FluidLayer, _> = serde_json::from_str(bad_json);
        assert!(result.is_err());
    }

    #[test]
    fn multiple_layers() {
        let mut fluids = ChunkFluids::new();

        fluids.set(
            FluidKind::Water,
            LocalPos::new(0, 0, 0),
            FluidCell::new(FluidKind::Water, 0.5),
        );
        fluids.set(
            FluidKind::Lava,
            LocalPos::new(1, 1, 1),
            FluidCell::new(FluidKind::Lava, 0.8),
        );

        assert_eq!(fluids.allocated_count(), 2);
        assert!(fluids.has_layer(FluidKind::Water));
        assert!(fluids.has_layer(FluidKind::Lava));
        assert!(!fluids.has_layer(FluidKind::Gas));
    }
}
