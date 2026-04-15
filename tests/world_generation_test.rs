//! Integration tests for world generation

use engine_world::generation::{WorldGenerator, GeneratorConfig, BiomeType};
use engine_world::chunk::{Chunk, ChunkPos};

#[test]
fn test_world_generation_creates_valid_chunks() {
    let config = GeneratorConfig::default();
    let generator = WorldGenerator::new(config, 12345);
    
    // Generate a chunk at origin
    let chunk = generator.generate_chunk(ChunkPos::new(0, 0, 0));
    
    // Chunk should have valid dimensions
    assert_eq!(chunk.size(), 16 * 16 * 16);
    
    // Chunk should not be entirely empty or entirely solid
    let air_count = chunk.count_block_type(0); // 0 = air
    assert!(air_count > 0, "Chunk should have some air blocks");
    assert!(air_count < chunk.size(), "Chunk should not be entirely air");
}

#[test]
fn test_world_generation_is_deterministic() {
    let config = GeneratorConfig::default();
    let generator1 = WorldGenerator::new(config.clone(), 42);
    let generator2 = WorldGenerator::new(config, 42);
    
    let chunk1 = generator1.generate_chunk(ChunkPos::new(5, 0, -3));
    let chunk2 = generator2.generate_chunk(ChunkPos::new(5, 0, -3));
    
    // Same seed should produce identical chunks
    assert_eq!(chunk1.data(), chunk2.data());
}

#[test]
fn test_different_seeds_produce_different_worlds() {
    let config = GeneratorConfig::default();
    let generator1 = WorldGenerator::new(config.clone(), 111);
    let generator2 = WorldGenerator::new(config, 222);
    
    let chunk1 = generator1.generate_chunk(ChunkPos::new(0, 0, 0));
    let chunk2 = generator2.generate_chunk(ChunkPos::new(0, 0, 0));
    
    // Different seeds should produce different chunks
    assert_ne!(chunk1.data(), chunk2.data());
}

#[test]
fn test_biome_generation() {
    let config = GeneratorConfig::default();
    let generator = WorldGenerator::new(config, 54321);
    
    // Sample multiple positions to find different biomes
    let mut biomes = std::collections::HashSet::new();
    for x in 0..10 {
        for z in 0..10 {
            let biome = generator.get_biome_at(x * 100, z * 100);
            biomes.insert(biome);
        }
    }
    
    // Should have at least 2 different biomes
    assert!(biomes.len() >= 2, "World should have biome variety");
}

#[test]
fn test_chunk_neighbor_continuity() {
    let config = GeneratorConfig::default();
    let generator = WorldGenerator::new(config, 99999);
    
    let chunk1 = generator.generate_chunk(ChunkPos::new(0, 0, 0));
    let chunk2 = generator.generate_chunk(ChunkPos::new(1, 0, 0));
    
    // Check that heightmap at boundary is continuous (within reason)
    for z in 0..16 {
        let h1 = chunk1.get_height(15, z);
        let h2 = chunk2.get_height(0, z);
        let diff = (h1 as i32 - h2 as i32).abs();
        assert!(diff <= 2, "Chunk boundary should be continuous, diff was {}", diff);
    }
}
