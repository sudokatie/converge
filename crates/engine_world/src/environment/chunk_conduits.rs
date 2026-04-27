//! Per-chunk conduit storage.

use engine_core::coords::LocalPos;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::chunk::CHUNK_VOLUME;

use super::{ConduitCell, ConduitKind};

/// Storage layer for one conduit kind within a chunk.
pub struct ConduitLayer {
    cells: Box<[ConduitCell; CHUNK_VOLUME]>,
    active_count: u32,
}

impl ConduitLayer {
    /// Create an empty layer.
    #[must_use]
    #[expect(
        clippy::missing_panics_doc,
        reason = "panic is infallible: vec length matches array size"
    )]
    pub fn new() -> Self {
        Self {
            cells: vec![ConduitCell::EMPTY; CHUNK_VOLUME]
                .into_boxed_slice()
                .try_into()
                .expect("vec has correct length"),
            active_count: 0,
        }
    }

    /// Get cell at position.
    #[must_use]
    pub fn get(&self, pos: LocalPos) -> ConduitCell {
        self.cells[pos.to_index()]
    }

    /// Get mutable reference to cell at position.
    pub fn get_mut(&mut self, pos: LocalPos) -> &mut ConduitCell {
        &mut self.cells[pos.to_index()]
    }

    /// Set cell at position.
    pub fn set(&mut self, pos: LocalPos, cell: ConduitCell) {
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

    /// Clear cell at position.
    pub fn clear(&mut self, pos: LocalPos) {
        self.set(pos, ConduitCell::EMPTY);
    }

    /// Get number of active (non-empty) cells.
    #[must_use]
    pub const fn active_count(&self) -> u32 {
        self.active_count
    }

    /// Check if layer has any active cells.
    #[must_use]
    pub const fn has_active(&self) -> bool {
        self.active_count > 0
    }

    /// Iterate over active cells with their positions.
    pub fn iter_active(&self) -> impl Iterator<Item = (LocalPos, ConduitCell)> + '_ {
        self.cells
            .iter()
            .enumerate()
            .filter(|(_, c)| !c.is_empty())
            .map(|(i, c)| (LocalPos::from_index(i), *c))
    }

    /// Recount active cells (after bulk modifications).
    pub fn recount(&mut self) {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "chunk volume is 4096, fits in u32"
        )]
        {
            self.active_count = self.cells.iter().filter(|c| !c.is_empty()).count() as u32;
        }
    }

    /// Get raw cells slice.
    #[must_use]
    pub fn cells(&self) -> &[ConduitCell; CHUNK_VOLUME] {
        &self.cells
    }

    /// Get mutable raw cells slice.
    pub fn cells_mut(&mut self) -> &mut [ConduitCell; CHUNK_VOLUME] {
        &mut self.cells
    }
}

impl Default for ConduitLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ConduitLayer {
    fn clone(&self) -> Self {
        Self {
            cells: self.cells.clone(),
            active_count: self.active_count,
        }
    }
}

impl std::fmt::Debug for ConduitLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConduitLayer")
            .field("active_count", &self.active_count)
            .finish_non_exhaustive()
    }
}

impl Serialize for ConduitLayer {
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

impl<'de> Deserialize<'de> for ConduitLayer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{SeqAccess, Visitor};

        struct LayerVisitor;

        impl<'de> Visitor<'de> for LayerVisitor {
            type Value = ConduitLayer;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a conduit layer with cells and count")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let cells_vec: Vec<ConduitCell> = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(0, &"2 elements"))?;
                let active_count: u32 = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(1, &"2 elements"))?;

                if cells_vec.len() != CHUNK_VOLUME {
                    return Err(serde::de::Error::invalid_length(
                        cells_vec.len(),
                        &"4096 conduit cells",
                    ));
                }

                let cells: Box<[ConduitCell; CHUNK_VOLUME]> = cells_vec
                    .into_boxed_slice()
                    .try_into()
                    .map_err(|_| serde::de::Error::custom("failed to convert cells"))?;

                Ok(ConduitLayer {
                    cells,
                    active_count,
                })
            }
        }

        deserializer.deserialize_tuple(2, LayerVisitor)
    }
}

/// Per-chunk storage for all conduit kinds.
#[derive(Clone, Debug, Default)]
pub struct ChunkConduits {
    layers: [ConduitLayer; ConduitKind::COUNT],
}

impl ChunkConduits {
    /// Create empty chunk conduits.
    #[must_use]
    pub fn new() -> Self {
        Self {
            layers: std::array::from_fn(|_| ConduitLayer::new()),
        }
    }

    /// Get cell for a specific kind and position.
    #[must_use]
    pub fn get(&self, kind: ConduitKind, pos: LocalPos) -> ConduitCell {
        self.layers[kind.as_index()].get(pos)
    }

    /// Get mutable reference to cell.
    pub fn get_mut(&mut self, kind: ConduitKind, pos: LocalPos) -> &mut ConduitCell {
        self.layers[kind.as_index()].get_mut(pos)
    }

    /// Set cell for a specific kind and position.
    pub fn set(&mut self, kind: ConduitKind, pos: LocalPos, cell: ConduitCell) {
        self.layers[kind.as_index()].set(pos, cell);
    }

    /// Clear cell at position for all kinds.
    pub fn clear(&mut self, pos: LocalPos) {
        for layer in &mut self.layers {
            layer.clear(pos);
        }
    }

    /// Clear cell at position for specific kind.
    pub fn clear_kind(&mut self, kind: ConduitKind, pos: LocalPos) {
        self.layers[kind.as_index()].clear(pos);
    }

    /// Get layer for a specific kind.
    #[must_use]
    pub fn layer(&self, kind: ConduitKind) -> &ConduitLayer {
        &self.layers[kind.as_index()]
    }

    /// Get mutable layer for a specific kind.
    pub fn layer_mut(&mut self, kind: ConduitKind) -> &mut ConduitLayer {
        &mut self.layers[kind.as_index()]
    }

    /// Check if any kind has active cells at position.
    #[must_use]
    pub fn has_any(&self, pos: LocalPos) -> bool {
        self.layers.iter().any(|l| !l.get(pos).is_empty())
    }

    /// Get total active count across all kinds.
    #[must_use]
    pub fn total_active_count(&self) -> u32 {
        self.layers.iter().map(ConduitLayer::active_count).sum()
    }

    /// Check if chunk has any conduits.
    #[must_use]
    pub fn has_any_active(&self) -> bool {
        self.layers.iter().any(ConduitLayer::has_active)
    }
}

impl Serialize for ChunkConduits {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.layers.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ChunkConduits {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let layers = <[ConduitLayer; ConduitKind::COUNT]>::deserialize(deserializer)?;
        Ok(Self { layers })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_new_empty() {
        let layer = ConduitLayer::new();
        assert_eq!(layer.active_count(), 0);
        assert!(!layer.has_active());
    }

    #[test]
    fn layer_set_updates_count() {
        let mut layer = ConduitLayer::new();
        assert_eq!(layer.active_count(), 0);

        layer.set(LocalPos::new(5, 5, 5), ConduitCell::new(ConduitKind::Power));
        assert_eq!(layer.active_count(), 1);

        layer.set(LocalPos::new(6, 6, 6), ConduitCell::new(ConduitKind::Power));
        assert_eq!(layer.active_count(), 2);

        layer.clear(LocalPos::new(5, 5, 5));
        assert_eq!(layer.active_count(), 1);
    }

    #[test]
    fn layer_iter_active() {
        let mut layer = ConduitLayer::new();
        layer.set(LocalPos::new(1, 2, 3), ConduitCell::new(ConduitKind::Power));
        layer.set(LocalPos::new(4, 5, 6), ConduitCell::new(ConduitKind::Power));

        let active: Vec<_> = layer.iter_active().collect();
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn layer_recount() {
        let mut layer = ConduitLayer::new();
        layer.cells_mut()[100] = ConduitCell::new(ConduitKind::Power);
        layer.cells_mut()[200] = ConduitCell::new(ConduitKind::Power);

        assert_eq!(layer.active_count(), 0);
        layer.recount();
        assert_eq!(layer.active_count(), 2);
    }

    #[test]
    fn chunk_conduits_new() {
        let conduits = ChunkConduits::new();
        assert_eq!(conduits.total_active_count(), 0);
        assert!(!conduits.has_any_active());
    }

    #[test]
    fn chunk_conduits_set_get() {
        let mut conduits = ChunkConduits::new();
        let pos = LocalPos::new(8, 8, 8);

        conduits.set(
            ConduitKind::Power,
            pos,
            ConduitCell::new(ConduitKind::Power),
        );
        assert!(!conduits.get(ConduitKind::Power, pos).is_empty());
        assert!(conduits.get(ConduitKind::Heat, pos).is_empty());
    }

    #[test]
    fn chunk_conduits_has_any() {
        let mut conduits = ChunkConduits::new();
        let pos = LocalPos::new(3, 3, 3);

        assert!(!conduits.has_any(pos));
        conduits.set(
            ConduitKind::Fluid,
            pos,
            ConduitCell::new(ConduitKind::Fluid),
        );
        assert!(conduits.has_any(pos));
    }

    #[test]
    fn chunk_conduits_clear() {
        let mut conduits = ChunkConduits::new();
        let pos = LocalPos::new(7, 7, 7);

        conduits.set(
            ConduitKind::Power,
            pos,
            ConduitCell::new(ConduitKind::Power),
        );
        conduits.set(ConduitKind::Heat, pos, ConduitCell::new(ConduitKind::Heat));
        assert_eq!(conduits.total_active_count(), 2);

        conduits.clear(pos);
        assert_eq!(conduits.total_active_count(), 0);
    }

    #[test]
    fn chunk_conduits_clear_kind() {
        let mut conduits = ChunkConduits::new();
        let pos = LocalPos::new(2, 2, 2);

        conduits.set(
            ConduitKind::Power,
            pos,
            ConduitCell::new(ConduitKind::Power),
        );
        conduits.set(ConduitKind::Heat, pos, ConduitCell::new(ConduitKind::Heat));

        conduits.clear_kind(ConduitKind::Power, pos);
        assert!(conduits.get(ConduitKind::Power, pos).is_empty());
        assert!(!conduits.get(ConduitKind::Heat, pos).is_empty());
    }

    #[test]
    fn layer_serde_round_trip() {
        let mut layer = ConduitLayer::new();
        layer.set(LocalPos::new(1, 2, 3), ConduitCell::new(ConduitKind::Power));
        layer.set(
            LocalPos::new(10, 11, 12),
            ConduitCell::with_params(ConduitKind::Power, 50.0, 0.1),
        );

        let json = serde_json::to_string(&layer).unwrap();
        let recovered: ConduitLayer = serde_json::from_str(&json).unwrap();

        assert_eq!(recovered.active_count(), layer.active_count());
        assert_eq!(
            recovered.get(LocalPos::new(1, 2, 3)),
            layer.get(LocalPos::new(1, 2, 3))
        );
    }

    #[test]
    fn chunk_conduits_serde_round_trip() {
        let mut conduits = ChunkConduits::new();
        conduits.set(
            ConduitKind::Power,
            LocalPos::new(5, 5, 5),
            ConduitCell::new(ConduitKind::Power),
        );
        conduits.set(
            ConduitKind::Fluid,
            LocalPos::new(8, 8, 8),
            ConduitCell::new(ConduitKind::Fluid),
        );

        let json = serde_json::to_string(&conduits).unwrap();
        let recovered: ChunkConduits = serde_json::from_str(&json).unwrap();

        assert_eq!(
            recovered.total_active_count(),
            conduits.total_active_count()
        );
        assert_eq!(
            recovered.get(ConduitKind::Power, LocalPos::new(5, 5, 5)),
            conduits.get(ConduitKind::Power, LocalPos::new(5, 5, 5))
        );
    }
}
