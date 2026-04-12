//! Basic terrain generation.

use engine_core::coords::{ChunkPos, LocalPos, CHUNK_SIZE};

use crate::chunk::{BlockId, Chunk, AIR, DIRT, GRASS, STONE};

use super::TerrainNoise;

/// Default sea level (Y coordinate).
const SEA_LEVEL: i32 = 64;

/// Stone depth below surface.
const STONE_DEPTH: i32 = 4;

/// Terrain generator using noise-based heightmap.
pub struct TerrainGenerator {
    noise: TerrainNoise,
    sea_level: i32,
}

impl TerrainGenerator {
    /// Create a new terrain generator with the given seed.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            noise: TerrainNoise::new(seed),
            sea_level: SEA_LEVEL,
        }
    }

    /// Create a terrain generator with custom sea level.
    #[must_use]
    pub fn with_sea_level(seed: u64, sea_level: i32) -> Self {
        Self {
            noise: TerrainNoise::new(seed),
            sea_level,
        }
    }

    /// Get the sea level.
    #[must_use]
    pub fn sea_level(&self) -> i32 {
        self.sea_level
    }

    /// Generate terrain for a chunk.
    #[must_use]
    pub fn generate(&self, chunk_pos: ChunkPos) -> Chunk {
        let mut chunk = Chunk::new();

        // Calculate world Y range for this chunk
        let chunk_base_y = chunk_pos.y() * CHUNK_SIZE;

        // Generate each column
        for lz in 0..CHUNK_SIZE as u32 {
            for lx in 0..CHUNK_SIZE as u32 {
                // World X and Z for this column
                let world_x = chunk_pos.x() * CHUNK_SIZE + lx as i32;
                let world_z = chunk_pos.z() * CHUNK_SIZE + lz as i32;

                // Get terrain height at this column
                let surface_height = self.height_at(world_x, world_z);

                // Fill each Y in this column
                for ly in 0..CHUNK_SIZE as u32 {
                    let world_y = chunk_base_y + ly as i32;
                    let block = self.block_at_height(world_y, surface_height);

                    if block != AIR {
                        chunk.set(LocalPos::new(lx, ly, lz), block);
                    }
                }
            }
        }

        chunk
    }

    /// Get the terrain height at a world position.
    #[must_use]
    pub fn height_at(&self, x: i32, z: i32) -> i32 {
        self.noise.height_at(x as f64, z as f64).round() as i32
    }

    /// Get the block type at a given height relative to surface.
    fn block_at_height(&self, y: i32, surface_height: i32) -> BlockId {
        if y > surface_height {
            // Above surface - air
            AIR
        } else if y == surface_height {
            // Surface - grass
            GRASS
        } else if y > surface_height - STONE_DEPTH {
            // Near surface - dirt
            DIRT
        } else {
            // Deep underground - stone
            STONE
        }
    }

    /// Get surface heights for a chunk (for cave generation).
    #[must_use]
    pub fn surface_heights(&self, chunk_pos: ChunkPos) -> [[i32; 16]; 16] {
        let mut heights = [[0i32; 16]; 16];

        for lz in 0..16 {
            for lx in 0..16 {
                let world_x = chunk_pos.x() * CHUNK_SIZE + lx as i32;
                let world_z = chunk_pos.z() * CHUNK_SIZE + lz as i32;
                heights[lz][lx] = self.height_at(world_x, world_z);
            }
        }

        heights
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generated_chunks_have_blocks() {
        let generator = TerrainGenerator::new(42);

        // Generate a chunk at surface level
        let chunk = generator.generate(ChunkPos::new(0, 4, 0)); // Y=64 is chunk 4

        // Should have some non-air blocks
        assert!(
            !chunk.is_empty(),
            "Surface chunk should have blocks"
        );
    }

    #[test]
    fn test_underground_chunk_full() {
        let generator = TerrainGenerator::new(42);

        // Deep underground chunk (Y=0 to Y=16)
        let chunk = generator.generate(ChunkPos::new(0, 0, 0));

        // Should be mostly full (all stone)
        assert!(
            chunk.non_air_count() > 4000,
            "Underground chunk should be mostly full, got {}",
            chunk.non_air_count()
        );
    }

    #[test]
    fn test_sky_chunk_empty() {
        let generator = TerrainGenerator::new(42);

        // High sky chunk (Y=256 to Y=272)
        let chunk = generator.generate(ChunkPos::new(0, 16, 0));

        // Should be empty (all air)
        assert!(chunk.is_empty(), "Sky chunk should be empty");
    }

    #[test]
    fn test_same_seed_position_identical() {
        let gen1 = TerrainGenerator::new(12345);
        let gen2 = TerrainGenerator::new(12345);

        let chunk1 = gen1.generate(ChunkPos::new(5, 4, 5));
        let chunk2 = gen2.generate(ChunkPos::new(5, 4, 5));

        // Compare block by block
        for (pos, block) in chunk1.iter() {
            assert_eq!(
                block,
                chunk2.get(pos),
                "Blocks should be identical at {:?}",
                pos
            );
        }
    }

    #[test]
    fn test_surface_has_grass() {
        let generator = TerrainGenerator::new(42);
        let chunk = generator.generate(ChunkPos::new(0, 4, 0));

        // Find at least one grass block
        let has_grass = chunk.iter().any(|(_, b)| b == GRASS);
        assert!(has_grass, "Surface chunk should have grass");
    }

    #[test]
    fn test_no_floating_blocks() {
        let generator = TerrainGenerator::new(42);
        let chunk = generator.generate(ChunkPos::new(0, 4, 0));

        // In a simple layered terrain, solid blocks should have solid below
        // (except at chunk boundaries)
        for (pos, block) in chunk.iter() {
            if block != AIR && pos.y() > 0 {
                let below = LocalPos::new(pos.x(), pos.y() - 1, pos.z());
                let block_below = chunk.get(below);
                // Either the block below is solid, or we're at the bottom of a layer
                // For this simple test, just verify grass has dirt below
                if block == GRASS {
                    assert_eq!(
                        block_below, DIRT,
                        "Grass should have dirt below at {:?}",
                        pos
                    );
                }
            }
        }
    }

    #[test]
    fn test_height_varies() {
        let generator = TerrainGenerator::new(42);

        let h1 = generator.height_at(0, 0);
        let h2 = generator.height_at(100, 100);
        let h3 = generator.height_at(-50, 75);

        // Heights should vary (not all the same)
        assert!(h1 != h2 || h2 != h3, "Heights should vary across terrain");
    }
}
