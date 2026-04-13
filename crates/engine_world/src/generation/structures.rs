//! Structure generation for world features like trees.

use engine_core::coords::{ChunkPos, LocalPos, WorldPos, CHUNK_SIZE};

use super::Biome;

/// Block IDs for structure generation.
pub mod blocks {
    pub const AIR: u16 = 0;
    pub const OAK_LOG: u16 = 6;
    pub const OAK_LEAVES: u16 = 7;
    pub const BIRCH_LOG: u16 = 8;
    pub const BIRCH_LEAVES: u16 = 9;
    pub const CACTUS: u16 = 10;
}

/// A block placement for structure generation.
#[derive(Clone, Copy, Debug)]
pub struct StructureBlock {
    /// Offset from structure origin.
    pub offset: (i32, i32, i32),
    /// Block ID to place.
    pub block: u16,
    /// Whether to replace existing non-air blocks.
    pub replace_solid: bool,
}

impl StructureBlock {
    /// Create a new structure block.
    #[must_use]
    pub fn new(dx: i32, dy: i32, dz: i32, block: u16) -> Self {
        Self {
            offset: (dx, dy, dz),
            block,
            replace_solid: false,
        }
    }

    /// Create a block that can replace solid blocks.
    #[must_use]
    pub fn replacing(dx: i32, dy: i32, dz: i32, block: u16) -> Self {
        Self {
            offset: (dx, dy, dz),
            block,
            replace_solid: true,
        }
    }
}

/// A structure template (e.g., tree, rock formation).
#[derive(Clone, Debug)]
pub struct Structure {
    /// Blocks that make up this structure.
    pub blocks: Vec<StructureBlock>,
    /// Bounding box size (width, height, depth).
    pub size: (i32, i32, i32),
}

impl Structure {
    /// Create an oak tree structure.
    #[must_use]
    pub fn oak_tree() -> Self {
        let mut blocks = Vec::new();

        // Trunk (5 blocks tall)
        for y in 0..5 {
            blocks.push(StructureBlock::new(0, y, 0, blocks::OAK_LOG));
        }

        // Leaves (3 layers)
        // Bottom layer (y=3): 5x5 minus corners
        for dx in -2_i32..=2 {
            for dz in -2_i32..=2 {
                if dx.abs() == 2 && dz.abs() == 2 {
                    continue; // Skip corners
                }
                if dx == 0 && dz == 0 {
                    continue; // Trunk position
                }
                blocks.push(StructureBlock::new(dx, 3, dz, blocks::OAK_LEAVES));
            }
        }

        // Middle layer (y=4): 5x5 minus corners
        for dx in -2_i32..=2 {
            for dz in -2_i32..=2 {
                if dx.abs() == 2 && dz.abs() == 2 {
                    continue;
                }
                if dx == 0 && dz == 0 {
                    continue;
                }
                blocks.push(StructureBlock::new(dx, 4, dz, blocks::OAK_LEAVES));
            }
        }

        // Top layer (y=5): 3x3
        for dx in -1_i32..=1 {
            for dz in -1_i32..=1 {
                blocks.push(StructureBlock::new(dx, 5, dz, blocks::OAK_LEAVES));
            }
        }

        // Very top (y=6): single block
        blocks.push(StructureBlock::new(0, 6, 0, blocks::OAK_LEAVES));

        Self {
            blocks,
            size: (5, 7, 5),
        }
    }

    /// Create a birch tree structure (taller, thinner).
    #[must_use]
    pub fn birch_tree() -> Self {
        let mut blocks = Vec::new();

        // Trunk (6 blocks tall)
        for y in 0..6 {
            blocks.push(StructureBlock::new(0, y, 0, blocks::BIRCH_LOG));
        }

        // Leaves (smaller canopy)
        // Layer at y=4: 3x3
        for dx in -1_i32..=1 {
            for dz in -1_i32..=1 {
                if dx == 0 && dz == 0 {
                    continue;
                }
                blocks.push(StructureBlock::new(dx, 4, dz, blocks::BIRCH_LEAVES));
            }
        }

        // Layer at y=5: 3x3
        for dx in -1_i32..=1 {
            for dz in -1_i32..=1 {
                if dx == 0 && dz == 0 {
                    continue;
                }
                blocks.push(StructureBlock::new(dx, 5, dz, blocks::BIRCH_LEAVES));
            }
        }

        // Top layers
        for dx in -1_i32..=1 {
            for dz in -1_i32..=1 {
                if dx.abs() == 1 && dz.abs() == 1 {
                    continue; // Skip corners
                }
                blocks.push(StructureBlock::new(dx, 6, dz, blocks::BIRCH_LEAVES));
            }
        }
        blocks.push(StructureBlock::new(0, 7, 0, blocks::BIRCH_LEAVES));

        Self {
            blocks,
            size: (3, 8, 3),
        }
    }

    /// Create a cactus structure (1-3 blocks tall).
    #[must_use]
    pub fn cactus(height: i32) -> Self {
        let h = height.clamp(1, 3);
        let mut blocks = Vec::new();

        for y in 0..h {
            blocks.push(StructureBlock::new(0, y, 0, blocks::CACTUS));
        }

        Self {
            blocks,
            size: (1, h, 1),
        }
    }

    /// Get the appropriate tree for a biome.
    #[must_use]
    pub fn tree_for_biome(biome: Biome) -> Option<Self> {
        match biome {
            Biome::Plains => Some(Self::oak_tree()),
            Biome::Forest => Some(Self::oak_tree()), // Could randomize oak/birch
            Biome::Mountains => Some(Self::birch_tree()),
            Biome::Desert => Some(Self::cactus(2)),
            Biome::Ocean => None,
        }
    }

    /// Check if structure fits within a single chunk at the given local position.
    #[must_use]
    pub fn fits_in_chunk(&self, local_x: i32, local_z: i32) -> bool {
        let half_w = self.size.0 / 2;
        let half_d = self.size.2 / 2;

        let min_x = local_x - half_w;
        let max_x = local_x + half_w;
        let min_z = local_z - half_d;
        let max_z = local_z + half_d;

        min_x >= 0 && max_x < CHUNK_SIZE && min_z >= 0 && max_z < CHUNK_SIZE
    }

    /// Get blocks that fall within a specific chunk.
    ///
    /// Returns (local_pos, block_id) pairs for blocks in the chunk.
    pub fn blocks_in_chunk(
        &self,
        origin: WorldPos,
        chunk_pos: ChunkPos,
    ) -> Vec<(LocalPos, u16)> {
        let chunk_min_x = chunk_pos.x() * CHUNK_SIZE;
        let chunk_min_y = chunk_pos.y() * CHUNK_SIZE;
        let chunk_min_z = chunk_pos.z() * CHUNK_SIZE;

        self.blocks
            .iter()
            .filter_map(|sb| {
                let wx = origin.x() + sb.offset.0;
                let wy = origin.y() + sb.offset.1;
                let wz = origin.z() + sb.offset.2;

                // Check if in this chunk
                let lx = wx - chunk_min_x;
                let ly = wy - chunk_min_y;
                let lz = wz - chunk_min_z;

                if lx >= 0
                    && lx < CHUNK_SIZE
                    && ly >= 0
                    && ly < CHUNK_SIZE
                    && lz >= 0
                    && lz < CHUNK_SIZE
                {
                    Some((
                        LocalPos::new(lx as u32, ly as u32, lz as u32),
                        sb.block,
                    ))
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Simple deterministic random for structure placement.
#[must_use]
pub fn structure_random(x: i32, z: i32, seed: u64) -> f32 {
    // Simple hash-based random
    let mut h = seed.wrapping_mul(31337);
    h = h.wrapping_add(x as u64 * 73856093);
    h = h.wrapping_add(z as u64 * 19349663);
    h ^= h >> 17;
    h = h.wrapping_mul(0xed5ad4bb);
    h ^= h >> 11;

    (h & 0xFFFF) as f32 / 65535.0
}

/// Check if a tree should spawn at this position.
#[must_use]
pub fn should_place_tree(x: i32, z: i32, seed: u64, density: f32) -> bool {
    structure_random(x, z, seed) < density
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oak_tree_structure() {
        let tree = Structure::oak_tree();

        // Should have trunk and leaves
        assert!(!tree.blocks.is_empty());

        // Check for trunk blocks
        let trunks: Vec<_> = tree
            .blocks
            .iter()
            .filter(|b| b.block == blocks::OAK_LOG)
            .collect();
        assert_eq!(trunks.len(), 5);

        // Check for leaves
        let leaves: Vec<_> = tree
            .blocks
            .iter()
            .filter(|b| b.block == blocks::OAK_LEAVES)
            .collect();
        assert!(!leaves.is_empty());
    }

    #[test]
    fn test_birch_tree_structure() {
        let tree = Structure::birch_tree();

        let trunks: Vec<_> = tree
            .blocks
            .iter()
            .filter(|b| b.block == blocks::BIRCH_LOG)
            .collect();
        assert_eq!(trunks.len(), 6);
    }

    #[test]
    fn test_cactus_height() {
        let c1 = Structure::cactus(1);
        let c2 = Structure::cactus(3);
        let c_clamped = Structure::cactus(10);

        assert_eq!(c1.blocks.len(), 1);
        assert_eq!(c2.blocks.len(), 3);
        assert_eq!(c_clamped.blocks.len(), 3); // Clamped to max 3
    }

    #[test]
    fn test_tree_for_biome() {
        assert!(Structure::tree_for_biome(Biome::Forest).is_some());
        assert!(Structure::tree_for_biome(Biome::Plains).is_some());
        assert!(Structure::tree_for_biome(Biome::Ocean).is_none());
    }

    #[test]
    fn test_fits_in_chunk() {
        let tree = Structure::oak_tree();

        // Center of chunk should fit
        assert!(tree.fits_in_chunk(8, 8));

        // Edge might not fit
        assert!(!tree.fits_in_chunk(0, 0));
        assert!(!tree.fits_in_chunk(15, 15));
    }

    #[test]
    fn test_structure_random_deterministic() {
        let r1 = structure_random(10, 20, 12345);
        let r2 = structure_random(10, 20, 12345);

        assert_eq!(r1, r2);
    }

    #[test]
    fn test_structure_random_distribution() {
        let mut sum = 0.0;
        let samples = 1000;

        for i in 0..samples {
            sum += structure_random(i, i * 7, 42);
        }

        let avg = sum / samples as f32;
        // Should be roughly centered around 0.5
        assert!(avg > 0.4 && avg < 0.6, "Average was {}", avg);
    }

    #[test]
    fn test_should_place_tree() {
        // With density 0, no trees
        assert!(!should_place_tree(10, 20, 42, 0.0));

        // With density 1, all trees
        assert!(should_place_tree(10, 20, 42, 1.0));
    }

    #[test]
    fn test_blocks_in_chunk() {
        let tree = Structure::oak_tree();
        let origin = WorldPos::new(8, 64, 8);
        let chunk = ChunkPos::new(0, 4, 0);

        let blocks = tree.blocks_in_chunk(origin, chunk);

        // Should have some blocks in this chunk
        assert!(!blocks.is_empty());

        // All local positions should be valid
        for (local, _) in &blocks {
            assert!(local.x() < CHUNK_SIZE as u32);
            assert!(local.y() < CHUNK_SIZE as u32);
            assert!(local.z() < CHUNK_SIZE as u32);
        }
    }
}
