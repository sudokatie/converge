//! Cave generation using 3D noise carving.

use engine_core::coords::{ChunkPos, LocalPos, CHUNK_SIZE};

use crate::chunk::{Chunk, AIR};

use super::TerrainNoise;

/// Threshold for cave carving (values below this become air).
/// Higher values = more caves. Range is roughly -1 to 1.
const CAVE_THRESHOLD: f64 = 0.0;

/// Minimum Y level for caves (avoid bedrock layer).
const MIN_CAVE_Y: i32 = 5;

/// Distance below surface where caves can start.
const SURFACE_BUFFER: i32 = 3;

/// Cave carver using 3D noise.
pub struct CaveCarver {
    noise: TerrainNoise,
}

impl CaveCarver {
    /// Create a new cave carver with the given seed.
    ///
    /// Uses a different offset from terrain noise for variety.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        // Use a modified seed to get different noise from terrain
        Self {
            noise: TerrainNoise::new(seed.wrapping_add(12345)),
        }
    }

    /// Carve caves into a chunk.
    ///
    /// # Arguments
    /// * `chunk` - The chunk to modify
    /// * `chunk_pos` - Position of the chunk in the world
    /// * `surface_heights` - 16x16 array of surface heights for this chunk's columns
    pub fn carve(&self, chunk: &mut Chunk, chunk_pos: ChunkPos, surface_heights: &[[i32; 16]; 16]) {
        let chunk_base_y = chunk_pos.y() * CHUNK_SIZE;

        for lz in 0..CHUNK_SIZE as u32 {
            for lx in 0..CHUNK_SIZE as u32 {
                let world_x = chunk_pos.x() * CHUNK_SIZE + lx as i32;
                let world_z = chunk_pos.z() * CHUNK_SIZE + lz as i32;
                let surface_y = surface_heights[lz as usize][lx as usize];

                for ly in 0..CHUNK_SIZE as u32 {
                    let world_y = chunk_base_y + ly as i32;

                    // Only carve if:
                    // 1. Below surface buffer
                    // 2. Above minimum cave level
                    // 3. Current block is solid
                    if world_y >= surface_y - SURFACE_BUFFER {
                        continue;
                    }
                    if world_y < MIN_CAVE_Y {
                        continue;
                    }

                    let pos = LocalPos::new(lx, ly, lz);
                    let current_block = chunk.get(pos);

                    // Don't carve air
                    if current_block == AIR {
                        continue;
                    }

                    // Sample 3D noise at this position
                    let noise_value = self.noise.sample_3d(
                        world_x as f64,
                        world_y as f64,
                        world_z as f64,
                    );

                    // Carve if below threshold
                    if noise_value < CAVE_THRESHOLD {
                        chunk.set(pos, AIR);
                    }
                }
            }
        }
    }

    /// Check if a position should be a cave.
    #[must_use]
    pub fn is_cave(&self, x: i32, y: i32, z: i32) -> bool {
        if y < MIN_CAVE_Y {
            return false;
        }
        let noise_value = self.noise.sample_3d(x as f64, y as f64, z as f64);
        noise_value < CAVE_THRESHOLD
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::STONE;

    fn create_solid_chunk() -> Chunk {
        Chunk::filled(STONE)
    }

    fn flat_surface_heights(height: i32) -> [[i32; 16]; 16] {
        [[height; 16]; 16]
    }

    #[test]
    fn test_caves_exist_below_surface() {
        let carver = CaveCarver::new(42);

        // Create a fully solid chunk at Y=0..16
        let mut chunk = create_solid_chunk();
        let initial_count = chunk.non_air_count();

        // Surface at Y=100 (well above this chunk)
        let heights = flat_surface_heights(100);

        carver.carve(&mut chunk, ChunkPos::new(0, 0, 0), &heights);

        // Should have carved some blocks (fewer non-air)
        assert!(
            chunk.non_air_count() < initial_count,
            "Caves should carve some blocks: {} vs {}",
            chunk.non_air_count(),
            initial_count
        );
    }

    #[test]
    fn test_no_surface_caves() {
        let carver = CaveCarver::new(42);

        let mut chunk = create_solid_chunk();

        // Surface at Y=10 (within this chunk)
        let heights = flat_surface_heights(10);

        carver.carve(&mut chunk, ChunkPos::new(0, 0, 0), &heights);

        // Check that blocks near surface (Y=8-10) aren't carved much
        let mut near_surface_air = 0;
        for lx in 0..16_u32 {
            for lz in 0..16_u32 {
                for ly in 8..11_u32 {
                    if chunk.get(LocalPos::new(lx, ly, lz)) == AIR {
                        near_surface_air += 1;
                    }
                }
            }
        }

        // Surface buffer should prevent most caves near surface
        assert!(
            near_surface_air < 100,
            "Too many caves near surface: {}",
            near_surface_air
        );
    }

    #[test]
    fn test_same_seed_same_caves() {
        let carver1 = CaveCarver::new(12345);
        let carver2 = CaveCarver::new(12345);

        let mut chunk1 = create_solid_chunk();
        let mut chunk2 = create_solid_chunk();
        let heights = flat_surface_heights(100);

        carver1.carve(&mut chunk1, ChunkPos::new(5, 2, 5), &heights);
        carver2.carve(&mut chunk2, ChunkPos::new(5, 2, 5), &heights);

        assert_eq!(
            chunk1.non_air_count(),
            chunk2.non_air_count(),
            "Same seed should produce same caves"
        );
    }

    #[test]
    fn test_caves_dont_carve_air() {
        let carver = CaveCarver::new(42);

        // Empty chunk
        let mut chunk = Chunk::new();
        let heights = flat_surface_heights(100);

        carver.carve(&mut chunk, ChunkPos::new(0, 0, 0), &heights);

        // Should still be empty
        assert!(chunk.is_empty(), "Carving air should leave air");
    }

    #[test]
    fn test_min_cave_level() {
        let carver = CaveCarver::new(42);

        let mut chunk = create_solid_chunk();
        let heights = flat_surface_heights(100);

        carver.carve(&mut chunk, ChunkPos::new(0, 0, 0), &heights);

        // Check that Y=0-4 are not carved (bedrock layer protection)
        let mut low_level_air = 0;
        for lx in 0..16_u32 {
            for lz in 0..16_u32 {
                for ly in 0..MIN_CAVE_Y as u32 {
                    if chunk.get(LocalPos::new(lx, ly, lz)) == AIR {
                        low_level_air += 1;
                    }
                }
            }
        }

        assert_eq!(
            low_level_air, 0,
            "No caves should exist below MIN_CAVE_Y"
        );
    }
}
