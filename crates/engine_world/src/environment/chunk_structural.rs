//! Per-chunk storage for structural integrity cells.

use engine_core::coords::LocalPos;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{StructuralCell, SupportKind};

/// Total cells per chunk (16x16x16).
const CHUNK_SIZE: usize = 16 * 16 * 16;

/// Storage for structural cells within a chunk.
#[derive(Clone)]
pub struct ChunkStructural {
    cells: Box<[StructuralCell; CHUNK_SIZE]>,
    active_count: u32,
    supported_count: u32,
    overstressed_count: u32,
}

impl Serialize for ChunkStructural {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeTuple;
        let mut tuple = serializer.serialize_tuple(4)?;
        tuple.serialize_element(self.cells.as_slice())?;
        tuple.serialize_element(&self.active_count)?;
        tuple.serialize_element(&self.supported_count)?;
        tuple.serialize_element(&self.overstressed_count)?;
        tuple.end()
    }
}

impl<'de> Deserialize<'de> for ChunkStructural {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{SeqAccess, Visitor};

        struct ChunkVisitor;

        impl<'de> Visitor<'de> for ChunkVisitor {
            type Value = ChunkStructural;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a structural chunk with cells and counts")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let cells_vec: Vec<StructuralCell> = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;

                let active_count: u32 = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;

                let supported_count: u32 = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(2, &self))?;

                let overstressed_count: u32 = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(3, &self))?;

                if cells_vec.len() != CHUNK_SIZE {
                    return Err(serde::de::Error::invalid_length(
                        cells_vec.len(),
                        &"4096 cells",
                    ));
                }

                #[expect(
                    clippy::large_stack_arrays,
                    reason = "temporary array immediately moved to heap via Box"
                )]
                let mut cells = Box::new([StructuralCell::EMPTY; CHUNK_SIZE]);
                cells.copy_from_slice(&cells_vec);

                Ok(ChunkStructural {
                    cells,
                    active_count,
                    supported_count,
                    overstressed_count,
                })
            }
        }

        deserializer.deserialize_tuple(4, ChunkVisitor)
    }
}

impl std::fmt::Debug for ChunkStructural {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChunkStructural")
            .field("active_count", &self.active_count)
            .field("supported_count", &self.supported_count)
            .field("overstressed_count", &self.overstressed_count)
            .finish_non_exhaustive()
    }
}

impl ChunkStructural {
    /// Create a new empty chunk (all cells are air/none).
    #[must_use]
    #[expect(
        clippy::large_stack_arrays,
        reason = "temporary array immediately moved to heap via Box"
    )]
    pub fn new() -> Self {
        Self {
            cells: Box::new([StructuralCell::EMPTY; CHUNK_SIZE]),
            active_count: 0,
            supported_count: 0,
            overstressed_count: 0,
        }
    }

    /// Get cell at position.
    #[must_use]
    pub fn get(&self, pos: LocalPos) -> StructuralCell {
        self.cells[pos.to_index()]
    }

    /// Get mutable reference to cell at position.
    pub fn get_mut(&mut self, pos: LocalPos) -> &mut StructuralCell {
        &mut self.cells[pos.to_index()]
    }

    /// Set cell at position.
    pub fn set(&mut self, pos: LocalPos, cell: StructuralCell) {
        self.cells[pos.to_index()] = cell;
    }

    /// Set support kind at position, creating appropriate cell.
    pub fn set_support(&mut self, pos: LocalPos, kind: SupportKind) {
        let cell = if kind.is_foundation() {
            StructuralCell::foundation()
        } else {
            StructuralCell::new(kind)
        };
        self.cells[pos.to_index()] = cell;
    }

    /// Get number of cells with structural support types.
    #[must_use]
    pub const fn active_count(&self) -> u32 {
        self.active_count
    }

    /// Get number of cells connected to foundation.
    #[must_use]
    pub const fn supported_count(&self) -> u32 {
        self.supported_count
    }

    /// Get number of overstressed cells.
    #[must_use]
    pub const fn overstressed_count(&self) -> u32 {
        self.overstressed_count
    }

    /// Recount active, supported, and overstressed cells.
    pub fn recount(&mut self) {
        self.active_count = 0;
        self.supported_count = 0;
        self.overstressed_count = 0;

        for cell in self.cells.iter() {
            if cell.support_kind().provides_support() {
                self.active_count += 1;
                if cell.is_supported() {
                    self.supported_count += 1;
                }
                if cell.is_overstressed() {
                    self.overstressed_count += 1;
                }
            }
        }
    }

    /// Check if chunk has any structural content.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.active_count == 0
    }

    /// Check if chunk has any unsupported structural cells.
    #[must_use]
    pub fn has_unsupported(&self) -> bool {
        self.active_count > self.supported_count
    }

    /// Check if chunk has any overstressed cells.
    #[must_use]
    pub fn has_overstressed(&self) -> bool {
        self.overstressed_count > 0
    }

    /// Iterate over all cells with positions.
    pub fn iter(&self) -> impl Iterator<Item = (LocalPos, StructuralCell)> + '_ {
        (0..CHUNK_SIZE).map(|i| {
            let pos = LocalPos::from_index(i);
            (pos, self.cells[i])
        })
    }

    /// Iterate over cells that provide structural support.
    pub fn iter_structural(&self) -> impl Iterator<Item = (LocalPos, StructuralCell)> + '_ {
        self.iter()
            .filter(|(_, cell)| cell.support_kind().provides_support())
    }

    /// Iterate over supported cells.
    pub fn iter_supported(&self) -> impl Iterator<Item = (LocalPos, StructuralCell)> + '_ {
        self.iter_structural()
            .filter(|(_, cell)| cell.is_supported())
    }

    /// Iterate over unsupported structural cells.
    pub fn iter_unsupported(&self) -> impl Iterator<Item = (LocalPos, StructuralCell)> + '_ {
        self.iter_structural()
            .filter(|(_, cell)| !cell.is_supported())
    }

    /// Iterate over overstressed cells.
    pub fn iter_overstressed(&self) -> impl Iterator<Item = (LocalPos, StructuralCell)> + '_ {
        self.iter_structural()
            .filter(|(_, cell)| cell.is_overstressed())
    }

    /// Iterate over foundation cells.
    pub fn iter_foundations(&self) -> impl Iterator<Item = (LocalPos, StructuralCell)> + '_ {
        self.iter()
            .filter(|(_, cell)| cell.support_kind().is_foundation())
    }

    /// Clear all structural data.
    pub fn clear(&mut self) {
        for cell in self.cells.iter_mut() {
            *cell = StructuralCell::EMPTY;
        }
        self.active_count = 0;
        self.supported_count = 0;
        self.overstressed_count = 0;
    }

    /// Sample structural state at fractional coordinates using nearest-neighbor.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamping to 0..15 ensures safe conversion"
    )]
    pub fn sample(&self, x: f32, y: f32, z: f32) -> StructuralCell {
        let ix = (x.round() as i32).clamp(0, 15) as u32;
        let iy = (y.round() as i32).clamp(0, 15) as u32;
        let iz = (z.round() as i32).clamp(0, 15) as u32;
        self.get(LocalPos::new(ix, iy, iz))
    }
}

impl Default for ChunkStructural {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for ChunkStructural {
    fn eq(&self, other: &Self) -> bool {
        self.active_count == other.active_count
            && self.supported_count == other.supported_count
            && self.overstressed_count == other.overstressed_count
            && self.cells.as_ref() == other.cells.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_chunk_empty() {
        let chunk = ChunkStructural::new();
        assert!(chunk.is_empty());
        assert_eq!(chunk.active_count(), 0);
        assert_eq!(chunk.supported_count(), 0);
        assert!(!chunk.has_unsupported());
    }

    #[test]
    fn set_and_get() {
        let mut chunk = ChunkStructural::new();
        let pos = LocalPos::new(5, 5, 5);
        let cell = StructuralCell::new(SupportKind::Column);

        chunk.set(pos, cell);
        assert_eq!(chunk.get(pos), cell);
    }

    #[test]
    fn set_support() {
        let mut chunk = ChunkStructural::new();
        let pos = LocalPos::new(0, 0, 0);

        chunk.set_support(pos, SupportKind::Foundation);
        let cell = chunk.get(pos);
        assert_eq!(cell.support_kind(), SupportKind::Foundation);
        assert!(cell.is_supported());
    }

    #[test]
    fn recount() {
        let mut chunk = ChunkStructural::new();

        chunk.set_support(LocalPos::new(0, 0, 0), SupportKind::Foundation);
        chunk.set_support(LocalPos::new(1, 0, 0), SupportKind::Column);
        chunk.set_support(LocalPos::new(2, 0, 0), SupportKind::Solid);

        chunk.get_mut(LocalPos::new(1, 0, 0)).mark_supported(1);

        chunk.recount();

        assert_eq!(chunk.active_count(), 3);
        assert_eq!(chunk.supported_count(), 2);
    }

    #[test]
    fn has_unsupported() {
        let mut chunk = ChunkStructural::new();

        chunk.set_support(LocalPos::new(0, 0, 0), SupportKind::Foundation);
        chunk.set_support(LocalPos::new(5, 5, 5), SupportKind::Column);
        chunk.recount();

        assert!(chunk.has_unsupported());
    }

    #[test]
    fn iter_structural() {
        let mut chunk = ChunkStructural::new();
        chunk.set_support(LocalPos::new(0, 0, 0), SupportKind::Foundation);
        chunk.set_support(LocalPos::new(1, 1, 1), SupportKind::Column);
        chunk.set(LocalPos::new(2, 2, 2), StructuralCell::EMPTY);

        let structural: Vec<_> = chunk.iter_structural().collect();
        assert_eq!(structural.len(), 2);
    }

    #[test]
    fn iter_foundations() {
        let mut chunk = ChunkStructural::new();
        chunk.set_support(LocalPos::new(0, 0, 0), SupportKind::Foundation);
        chunk.set_support(LocalPos::new(1, 0, 0), SupportKind::Foundation);
        chunk.set_support(LocalPos::new(2, 0, 0), SupportKind::Column);

        let foundations: Vec<_> = chunk.iter_foundations().collect();
        assert_eq!(foundations.len(), 2);
    }

    #[test]
    fn iter_overstressed() {
        let mut chunk = ChunkStructural::new();
        let pos = LocalPos::new(5, 5, 5);

        let mut cell = StructuralCell::new(SupportKind::Weak);
        cell.mark_supported(1);
        cell.add_load(0.5);
        chunk.set(pos, cell);
        chunk.recount();

        assert!(chunk.has_overstressed());
        let overstressed: Vec<_> = chunk.iter_overstressed().collect();
        assert_eq!(overstressed.len(), 1);
    }

    #[test]
    fn clear() {
        let mut chunk = ChunkStructural::new();
        chunk.set_support(LocalPos::new(0, 0, 0), SupportKind::Foundation);
        chunk.set_support(LocalPos::new(5, 5, 5), SupportKind::Column);
        chunk.recount();

        assert!(!chunk.is_empty());

        chunk.clear();
        assert!(chunk.is_empty());
        assert_eq!(chunk.active_count(), 0);
    }

    #[test]
    fn sample_nearest() {
        let mut chunk = ChunkStructural::new();
        let pos = LocalPos::new(5, 5, 5);
        chunk.set_support(pos, SupportKind::Beam);

        let sampled = chunk.sample(5.3, 5.4, 4.6);
        assert_eq!(sampled.support_kind(), SupportKind::Beam);
    }

    #[test]
    fn sample_clamped() {
        let mut chunk = ChunkStructural::new();
        chunk.set_support(LocalPos::new(15, 15, 15), SupportKind::Column);

        let sampled = chunk.sample(20.0, 20.0, 20.0);
        assert_eq!(sampled.support_kind(), SupportKind::Column);
    }

    #[test]
    fn default_is_new() {
        let default = ChunkStructural::default();
        let new = ChunkStructural::new();
        assert_eq!(default, new);
    }

    #[test]
    fn serde_round_trip() {
        let mut chunk = ChunkStructural::new();
        chunk.set_support(LocalPos::new(0, 0, 0), SupportKind::Foundation);
        chunk.set_support(LocalPos::new(5, 5, 5), SupportKind::Column);
        chunk.get_mut(LocalPos::new(5, 5, 5)).mark_supported(1);
        chunk.recount();

        let json = serde_json::to_string(&chunk).unwrap();
        let recovered: ChunkStructural = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, chunk);
    }

    #[test]
    fn iter_all_cells() {
        let chunk = ChunkStructural::new();
        let count = chunk.iter().count();
        assert_eq!(count, CHUNK_SIZE);
    }

    #[test]
    fn get_mut_modifies() {
        let mut chunk = ChunkStructural::new();
        let pos = LocalPos::new(8, 8, 8);
        chunk.set_support(pos, SupportKind::Column);

        chunk.get_mut(pos).add_load(0.3);
        assert!((chunk.get(pos).load() - 0.3).abs() < 0.001);
    }
}
