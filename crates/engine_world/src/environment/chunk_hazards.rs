//! Per-chunk hazard storage.

use engine_core::coords::LocalPos;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{HazardCell, HazardKind};
use crate::chunk::CHUNK_VOLUME;

/// Storage for a single hazard kind within a chunk.
#[derive(Clone)]
pub struct HazardLayer {
    cells: Box<[HazardCell; CHUNK_VOLUME]>,
    active_count: u32,
}

impl Serialize for HazardLayer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeTuple;
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(self.cells.as_slice())?;
        tuple.serialize_element(&self.active_count)?;
        tuple.end()
    }
}

impl<'de> Deserialize<'de> for HazardLayer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{SeqAccess, Visitor};

        struct LayerVisitor;

        impl<'de> Visitor<'de> for LayerVisitor {
            type Value = HazardLayer;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a hazard layer with cells and count")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let cells_vec: Vec<HazardCell> = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;

                let active_count: u32 = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;

                if cells_vec.len() != CHUNK_VOLUME {
                    return Err(serde::de::Error::invalid_length(
                        cells_vec.len(),
                        &"4096 cells",
                    ));
                }

                let mut cells = Box::new([HazardCell::INACTIVE; CHUNK_VOLUME]);
                cells.copy_from_slice(&cells_vec);

                Ok(HazardLayer {
                    cells,
                    active_count,
                })
            }
        }

        deserializer.deserialize_tuple(2, LayerVisitor)
    }
}

impl std::fmt::Debug for HazardLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HazardLayer")
            .field("active_count", &self.active_count)
            .finish_non_exhaustive()
    }
}

impl HazardLayer {
    /// Create a new empty hazard layer.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cells: Box::new([HazardCell::INACTIVE; CHUNK_VOLUME]),
            active_count: 0,
        }
    }

    /// Get the hazard cell at a position.
    #[must_use]
    pub fn get(&self, pos: LocalPos) -> HazardCell {
        self.cells[pos.to_index()]
    }

    /// Get mutable access to a cell.
    pub fn get_mut(&mut self, pos: LocalPos) -> &mut HazardCell {
        &mut self.cells[pos.to_index()]
    }

    /// Set a cell, updating active count.
    pub fn set(&mut self, pos: LocalPos, cell: HazardCell) {
        let index = pos.to_index();
        let was_active = self.cells[index].is_active();
        let is_active = cell.is_active();

        if was_active && !is_active {
            self.active_count = self.active_count.saturating_sub(1);
        } else if !was_active && is_active {
            self.active_count += 1;
        }

        self.cells[index] = cell;
    }

    /// Activate a cell with given intensity.
    pub fn activate(&mut self, pos: LocalPos, intensity: f32) {
        self.set(pos, HazardCell::new(intensity));
    }

    /// Deactivate a cell.
    pub fn deactivate(&mut self, pos: LocalPos) {
        self.set(pos, HazardCell::INACTIVE);
    }

    /// Number of active cells.
    #[must_use]
    pub const fn active_count(&self) -> u32 {
        self.active_count
    }

    /// Check if the layer has any active hazards.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.active_count == 0
    }

    /// Get direct access to cells.
    #[must_use]
    pub fn cells(&self) -> &[HazardCell; CHUNK_VOLUME] {
        &self.cells
    }

    /// Get mutable access to cells (bypasses active_count tracking).
    pub fn cells_mut(&mut self) -> &mut [HazardCell; CHUNK_VOLUME] {
        &mut self.cells
    }

    /// Recalculate active count after direct cell modification.
    pub fn recalculate_count(&mut self) {
        self.active_count = self.cells.iter().filter(|c| c.is_active()).count() as u32;
    }

    /// Iterate over active cells with positions.
    pub fn iter_active(&self) -> impl Iterator<Item = (LocalPos, HazardCell)> + '_ {
        self.cells.iter().enumerate().filter_map(|(i, &cell)| {
            if cell.is_active() {
                Some((LocalPos::from_index(i), cell))
            } else {
                None
            }
        })
    }

    /// Clear all cells.
    pub fn clear(&mut self) {
        self.cells.fill(HazardCell::INACTIVE);
        self.active_count = 0;
    }
}

impl Default for HazardLayer {
    fn default() -> Self {
        Self::new()
    }
}

/// Per-chunk storage for all hazard types.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ChunkHazards {
    layers: [Option<HazardLayer>; HazardKind::COUNT],
}

impl ChunkHazards {
    /// Create new chunk hazards with no allocated layers.
    #[must_use]
    pub fn new() -> Self {
        Self {
            layers: std::array::from_fn(|_| None),
        }
    }

    /// Check if a layer is allocated.
    #[must_use]
    pub fn has_layer(&self, kind: HazardKind) -> bool {
        self.layers[kind.as_index()].is_some()
    }

    /// Get a cell from a hazard layer.
    #[must_use]
    pub fn get(&self, kind: HazardKind, pos: LocalPos) -> HazardCell {
        match &self.layers[kind.as_index()] {
            Some(layer) => layer.get(pos),
            None => HazardCell::INACTIVE,
        }
    }

    /// Set a cell in a hazard layer, allocating if needed.
    pub fn set(&mut self, kind: HazardKind, pos: LocalPos, cell: HazardCell) {
        self.ensure_layer(kind).set(pos, cell);
    }

    /// Activate a hazard at a position.
    pub fn activate(&mut self, kind: HazardKind, pos: LocalPos, intensity: f32) {
        self.ensure_layer(kind).activate(pos, intensity);
    }

    /// Deactivate a hazard at a position.
    pub fn deactivate(&mut self, kind: HazardKind, pos: LocalPos) {
        if let Some(layer) = &mut self.layers[kind.as_index()] {
            layer.deactivate(pos);
        }
    }

    /// Get read-only access to a layer.
    #[must_use]
    pub fn layer(&self, kind: HazardKind) -> Option<&HazardLayer> {
        self.layers[kind.as_index()].as_ref()
    }

    /// Get mutable access to a layer, allocating if needed.
    pub fn layer_mut(&mut self, kind: HazardKind) -> &mut HazardLayer {
        self.ensure_layer(kind)
    }

    fn ensure_layer(&mut self, kind: HazardKind) -> &mut HazardLayer {
        let idx = kind.as_index();
        if self.layers[idx].is_none() {
            self.layers[idx] = Some(HazardLayer::new());
        }
        self.layers[idx].as_mut().expect("just allocated")
    }

    /// Clear a layer, deallocating storage.
    pub fn clear_layer(&mut self, kind: HazardKind) {
        self.layers[kind.as_index()] = None;
    }

    /// Count allocated layers.
    #[must_use]
    pub fn allocated_count(&self) -> usize {
        self.layers.iter().filter(|l| l.is_some()).count()
    }

    /// Check if all layers are unallocated.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.layers.iter().all(|l| l.is_none())
    }

    /// Total active hazard cells across all layers.
    #[must_use]
    pub fn total_active(&self) -> u32 {
        self.layers
            .iter()
            .filter_map(|l| l.as_ref())
            .map(|l| l.active_count())
            .sum()
    }

    /// Iterate over all active hazards with kind and position.
    pub fn iter_all_active(&self) -> impl Iterator<Item = (HazardKind, LocalPos, HazardCell)> + '_ {
        self.layers
            .iter()
            .enumerate()
            .flat_map(|(kind_idx, layer)| {
                let kind = HazardKind::from_index(kind_idx).expect("valid index");
                layer
                    .iter()
                    .flat_map(move |l| l.iter_active().map(move |(pos, cell)| (kind, pos, cell)))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_new_empty() {
        let layer = HazardLayer::new();
        assert!(layer.is_empty());
        assert_eq!(layer.active_count(), 0);
    }

    #[test]
    fn layer_activate_deactivate() {
        let mut layer = HazardLayer::new();
        let pos = LocalPos::new(5, 5, 5);

        layer.activate(pos, 0.8);
        assert_eq!(layer.active_count(), 1);
        assert!(layer.get(pos).is_active());

        layer.deactivate(pos);
        assert_eq!(layer.active_count(), 0);
        assert!(!layer.get(pos).is_active());
    }

    #[test]
    fn layer_iter_active() {
        let mut layer = HazardLayer::new();
        layer.activate(LocalPos::new(0, 0, 0), 1.0);
        layer.activate(LocalPos::new(1, 1, 1), 0.5);
        layer.activate(LocalPos::new(2, 2, 2), 0.3);

        let active: Vec<_> = layer.iter_active().collect();
        assert_eq!(active.len(), 3);
    }

    #[test]
    fn layer_recalculate_count() {
        let mut layer = HazardLayer::new();
        layer.cells_mut()[0] = HazardCell::new(1.0);
        layer.cells_mut()[1] = HazardCell::new(0.5);

        assert_eq!(layer.active_count(), 0);
        layer.recalculate_count();
        assert_eq!(layer.active_count(), 2);
    }

    #[test]
    fn chunk_hazards_lazy_allocation() {
        let mut hazards = ChunkHazards::new();
        assert!(hazards.is_empty());
        assert!(!hazards.has_layer(HazardKind::Fire));

        hazards.activate(HazardKind::Fire, LocalPos::new(0, 0, 0), 0.8);
        assert!(!hazards.is_empty());
        assert!(hazards.has_layer(HazardKind::Fire));
        assert_eq!(hazards.allocated_count(), 1);
    }

    #[test]
    fn chunk_hazards_get_unallocated() {
        let hazards = ChunkHazards::new();
        let cell = hazards.get(HazardKind::Fire, LocalPos::new(5, 5, 5));
        assert_eq!(cell, HazardCell::INACTIVE);
    }

    #[test]
    fn chunk_hazards_total_active() {
        let mut hazards = ChunkHazards::new();
        hazards.activate(HazardKind::Fire, LocalPos::new(0, 0, 0), 1.0);
        hazards.activate(HazardKind::Fire, LocalPos::new(1, 0, 0), 0.8);
        hazards.activate(HazardKind::Frost, LocalPos::new(5, 5, 5), 0.5);

        assert_eq!(hazards.total_active(), 3);
    }

    #[test]
    fn chunk_hazards_iter_all_active() {
        let mut hazards = ChunkHazards::new();
        hazards.activate(HazardKind::Fire, LocalPos::new(0, 0, 0), 1.0);
        hazards.activate(HazardKind::Infection, LocalPos::new(5, 5, 5), 0.5);

        let all: Vec<_> = hazards.iter_all_active().collect();
        assert_eq!(all.len(), 2);

        let kinds: Vec<_> = all.iter().map(|(k, _, _)| *k).collect();
        assert!(kinds.contains(&HazardKind::Fire));
        assert!(kinds.contains(&HazardKind::Infection));
    }

    #[test]
    fn chunk_hazards_clear_layer() {
        let mut hazards = ChunkHazards::new();
        hazards.activate(HazardKind::Fire, LocalPos::new(0, 0, 0), 1.0);
        assert!(hazards.has_layer(HazardKind::Fire));

        hazards.clear_layer(HazardKind::Fire);
        assert!(!hazards.has_layer(HazardKind::Fire));
    }

    #[test]
    fn layer_serde_round_trip() {
        let mut layer = HazardLayer::new();
        layer.activate(LocalPos::new(3, 4, 5), 0.75);
        layer.activate(LocalPos::new(10, 10, 10), 0.25);

        let json = serde_json::to_string(&layer).unwrap();
        let recovered: HazardLayer = serde_json::from_str(&json).unwrap();

        assert_eq!(recovered.active_count(), layer.active_count());
        assert_eq!(
            recovered.get(LocalPos::new(3, 4, 5)).intensity(),
            layer.get(LocalPos::new(3, 4, 5)).intensity()
        );
    }
}
