//! Chunk storage for voxel data.

use engine_core::coords::{LocalPos, CHUNK_SIZE};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::BlockId;
use super::AIR;

/// Number of blocks in a chunk (16^3).
pub const CHUNK_VOLUME: usize = (CHUNK_SIZE as usize).pow(3);

/// A chunk containing a 16x16x16 grid of blocks.
#[derive(Clone)]
pub struct Chunk {
    blocks: Box<[BlockId; CHUNK_VOLUME]>,
    non_air_count: u32,
}

impl Serialize for Chunk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize as slice + count
        use serde::ser::SerializeTuple;
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(self.blocks.as_slice())?;
        tuple.serialize_element(&self.non_air_count)?;
        tuple.end()
    }
}

impl<'de> Deserialize<'de> for Chunk {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{SeqAccess, Visitor};

        struct ChunkVisitor;

        impl<'de> Visitor<'de> for ChunkVisitor {
            type Value = Chunk;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a chunk with blocks and count")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let blocks_vec: Vec<BlockId> = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;

                let non_air_count: u32 = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;

                if blocks_vec.len() != CHUNK_VOLUME {
                    return Err(serde::de::Error::invalid_length(blocks_vec.len(), &"4096 blocks"));
                }

                let mut blocks = Box::new([AIR; CHUNK_VOLUME]);
                blocks.copy_from_slice(&blocks_vec);

                Ok(Chunk {
                    blocks,
                    non_air_count,
                })
            }
        }

        deserializer.deserialize_tuple(2, ChunkVisitor)
    }
}

impl std::fmt::Debug for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chunk")
            .field("non_air_count", &self.non_air_count)
            .finish_non_exhaustive()
    }
}

impl Chunk {
    /// Create a new chunk filled with air.
    #[must_use]
    pub fn new() -> Self {
        Self {
            blocks: Box::new([AIR; CHUNK_VOLUME]),
            non_air_count: 0,
        }
    }

    /// Create a chunk filled with a specific block.
    #[must_use]
    pub fn filled(block: BlockId) -> Self {
        let non_air = if block == AIR { 0 } else { CHUNK_VOLUME as u32 };
        Self {
            blocks: Box::new([block; CHUNK_VOLUME]),
            non_air_count: non_air,
        }
    }

    /// Get the block at a local position.
    #[must_use]
    pub fn get(&self, pos: LocalPos) -> BlockId {
        self.blocks[pos.to_index()]
    }

    /// Set the block at a local position.
    pub fn set(&mut self, pos: LocalPos, block: BlockId) {
        let index = pos.to_index();
        let old = self.blocks[index];

        if old != AIR && block == AIR {
            self.non_air_count -= 1;
        } else if old == AIR && block != AIR {
            self.non_air_count += 1;
        }

        self.blocks[index] = block;
    }

    /// Check if the chunk is empty (all air).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.non_air_count == 0
    }

    /// Get the number of non-air blocks.
    #[must_use]
    pub fn non_air_count(&self) -> u32 {
        self.non_air_count
    }

    /// Iterate over all blocks in the chunk.
    pub fn iter(&self) -> impl Iterator<Item = (LocalPos, BlockId)> + '_ {
        self.blocks.iter().enumerate().map(|(i, &block)| {
            (LocalPos::from_index(i), block)
        })
    }

    /// Iterate over non-air blocks only.
    pub fn iter_non_air(&self) -> impl Iterator<Item = (LocalPos, BlockId)> + '_ {
        self.iter().filter(|(_, block)| *block != AIR)
    }

    /// Get direct access to the block data.
    #[must_use]
    pub fn blocks(&self) -> &[BlockId; CHUNK_VOLUME] {
        &self.blocks
    }

    /// Get mutable access to the block data.
    ///
    /// Note: This bypasses non_air_count tracking. Call `recalculate_count()` after.
    pub fn blocks_mut(&mut self) -> &mut [BlockId; CHUNK_VOLUME] {
        &mut self.blocks
    }

    /// Recalculate the non-air block count.
    pub fn recalculate_count(&mut self) {
        self.non_air_count = self.blocks.iter().filter(|&&b| b != AIR).count() as u32;
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_chunk_empty() {
        let chunk = Chunk::new();
        assert!(chunk.is_empty());
        assert_eq!(chunk.non_air_count(), 0);
    }

    #[test]
    fn test_filled_chunk() {
        let chunk = Chunk::filled(super::super::STONE);
        assert!(!chunk.is_empty());
        assert_eq!(chunk.non_air_count(), CHUNK_VOLUME as u32);
    }

    #[test]
    fn test_get_set() {
        let mut chunk = Chunk::new();
        let pos = LocalPos::new(5, 5, 5);

        assert_eq!(chunk.get(pos), AIR);

        chunk.set(pos, super::super::STONE);
        assert_eq!(chunk.get(pos), super::super::STONE);
        assert_eq!(chunk.non_air_count(), 1);

        chunk.set(pos, AIR);
        assert_eq!(chunk.get(pos), AIR);
        assert_eq!(chunk.non_air_count(), 0);
    }

    #[test]
    fn test_iter() {
        let chunk = Chunk::new();
        let count = chunk.iter().count();
        assert_eq!(count, CHUNK_VOLUME);
    }

    #[test]
    fn test_iter_non_air() {
        let mut chunk = Chunk::new();
        chunk.set(LocalPos::new(0, 0, 0), super::super::STONE);
        chunk.set(LocalPos::new(1, 1, 1), super::super::DIRT);
        chunk.set(LocalPos::new(2, 2, 2), super::super::GRASS);

        let non_air: Vec<_> = chunk.iter_non_air().collect();
        assert_eq!(non_air.len(), 3);
    }

    #[test]
    fn test_recalculate_count() {
        let mut chunk = Chunk::new();

        // Directly modify blocks
        chunk.blocks_mut()[0] = super::super::STONE;
        chunk.blocks_mut()[1] = super::super::STONE;

        // Count should still be 0 (bypassed tracking)
        assert_eq!(chunk.non_air_count(), 0);

        // Recalculate
        chunk.recalculate_count();
        assert_eq!(chunk.non_air_count(), 2);
    }
}
