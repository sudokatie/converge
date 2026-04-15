//! Integration tests for save/load functionality

use engine_world::persistence::{WorldSave, SaveConfig, SaveFormat};
use engine_world::chunk::{Chunk, ChunkPos};
use engine_world::manager::ChunkManager;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn test_save_and_load_world() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let save_path = temp_dir.path().join("test_world");
    
    let config = SaveConfig {
        format: SaveFormat::Binary,
        compression: true,
        path: save_path.clone(),
    };
    
    // Create a world save
    let mut save = WorldSave::new(config.clone());
    
    // Add some chunks
    let mut chunk1 = Chunk::new(ChunkPos::new(0, 0, 0));
    chunk1.set_block(0, 0, 0, 1); // Set a stone block
    chunk1.set_block(1, 1, 1, 2); // Set a dirt block
    
    let mut chunk2 = Chunk::new(ChunkPos::new(1, 0, 0));
    chunk2.set_block(5, 10, 5, 3); // Set a grass block
    
    save.save_chunk(&chunk1).expect("Failed to save chunk 1");
    save.save_chunk(&chunk2).expect("Failed to save chunk 2");
    
    // Save world metadata
    save.save_metadata().expect("Failed to save metadata");
    
    // Load the world
    let loaded_save = WorldSave::load(save_path).expect("Failed to load world");
    
    // Verify chunks
    let loaded_chunk1 = loaded_save.load_chunk(ChunkPos::new(0, 0, 0))
        .expect("Failed to load chunk 1");
    let loaded_chunk2 = loaded_save.load_chunk(ChunkPos::new(1, 0, 0))
        .expect("Failed to load chunk 2");
    
    assert_eq!(loaded_chunk1.get_block(0, 0, 0), 1);
    assert_eq!(loaded_chunk1.get_block(1, 1, 1), 2);
    assert_eq!(loaded_chunk2.get_block(5, 10, 5), 3);
}

#[test]
fn test_save_compression() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    
    // Create chunk with repetitive data (compresses well)
    let mut chunk = Chunk::new(ChunkPos::new(0, 0, 0));
    for x in 0..16 {
        for z in 0..16 {
            chunk.set_block(x, 0, z, 1); // Fill bottom layer with stone
        }
    }
    
    // Save without compression
    let path_uncompressed = temp_dir.path().join("uncompressed");
    let config_uncompressed = SaveConfig {
        format: SaveFormat::Binary,
        compression: false,
        path: path_uncompressed.clone(),
    };
    let mut save_uncompressed = WorldSave::new(config_uncompressed);
    save_uncompressed.save_chunk(&chunk).expect("Failed to save");
    
    // Save with compression
    let path_compressed = temp_dir.path().join("compressed");
    let config_compressed = SaveConfig {
        format: SaveFormat::Binary,
        compression: true,
        path: path_compressed.clone(),
    };
    let mut save_compressed = WorldSave::new(config_compressed);
    save_compressed.save_chunk(&chunk).expect("Failed to save");
    
    // Compressed should be smaller
    let size_uncompressed = std::fs::metadata(path_uncompressed.join("chunks/0_0_0.dat"))
        .map(|m| m.len())
        .unwrap_or(0);
    let size_compressed = std::fs::metadata(path_compressed.join("chunks/0_0_0.dat.gz"))
        .map(|m| m.len())
        .unwrap_or(0);
    
    assert!(size_compressed < size_uncompressed, 
        "Compressed size {} should be less than uncompressed {}", 
        size_compressed, size_uncompressed);
}

#[test]
fn test_incremental_save() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let save_path = temp_dir.path().join("incremental_world");
    
    let config = SaveConfig {
        format: SaveFormat::Binary,
        compression: false,
        path: save_path.clone(),
    };
    
    let mut save = WorldSave::new(config);
    
    // Initial save
    let mut chunk = Chunk::new(ChunkPos::new(0, 0, 0));
    chunk.set_block(0, 0, 0, 1);
    save.save_chunk(&chunk).expect("Failed to save");
    
    // Modify and save again
    chunk.set_block(1, 1, 1, 2);
    chunk.mark_dirty();
    save.save_chunk(&chunk).expect("Failed to save modified chunk");
    
    // Load and verify
    let loaded = WorldSave::load(save_path).expect("Failed to load");
    let loaded_chunk = loaded.load_chunk(ChunkPos::new(0, 0, 0)).expect("Failed to load chunk");
    
    assert_eq!(loaded_chunk.get_block(0, 0, 0), 1);
    assert_eq!(loaded_chunk.get_block(1, 1, 1), 2);
}

#[test]
fn test_save_player_data() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let save_path = temp_dir.path().join("player_world");
    
    let config = SaveConfig {
        format: SaveFormat::Binary,
        compression: true,
        path: save_path.clone(),
    };
    
    let mut save = WorldSave::new(config);
    
    // Save player data
    let player_data = PlayerData {
        uuid: "test-player-uuid".to_string(),
        position: (10.5, 64.0, -20.3),
        rotation: (45.0, 30.0),
        health: 18,
        max_health: 20,
        inventory: vec![(1, 64), (2, 32), (3, 1)],
    };
    
    save.save_player(&player_data).expect("Failed to save player");
    
    // Load and verify
    let loaded = WorldSave::load(save_path).expect("Failed to load");
    let loaded_player = loaded.load_player("test-player-uuid").expect("Failed to load player");
    
    assert_eq!(loaded_player.position, player_data.position);
    assert_eq!(loaded_player.health, player_data.health);
    assert_eq!(loaded_player.inventory.len(), 3);
}

#[test]
fn test_save_handles_missing_chunks() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let save_path = temp_dir.path().join("sparse_world");
    
    let config = SaveConfig {
        format: SaveFormat::Binary,
        compression: false,
        path: save_path.clone(),
    };
    
    let mut save = WorldSave::new(config);
    
    // Only save one chunk
    let chunk = Chunk::new(ChunkPos::new(5, 0, 5));
    save.save_chunk(&chunk).expect("Failed to save");
    
    // Try to load a non-existent chunk
    let loaded = WorldSave::load(save_path).expect("Failed to load");
    let missing = loaded.load_chunk(ChunkPos::new(0, 0, 0));
    
    assert!(missing.is_none() || missing.is_err());
}

#[test]
fn test_world_metadata_persistence() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let save_path = temp_dir.path().join("metadata_world");
    
    let config = SaveConfig {
        format: SaveFormat::Binary,
        compression: true,
        path: save_path.clone(),
    };
    
    let mut save = WorldSave::new(config);
    
    // Set world properties
    save.set_world_name("Test World");
    save.set_seed(12345);
    save.set_spawn_point((100.0, 65.0, 100.0));
    save.set_time(6000); // Noon
    save.set_weather(Weather::Clear);
    
    save.save_metadata().expect("Failed to save metadata");
    
    // Load and verify
    let loaded = WorldSave::load(save_path).expect("Failed to load");
    
    assert_eq!(loaded.world_name(), "Test World");
    assert_eq!(loaded.seed(), 12345);
    assert_eq!(loaded.spawn_point(), (100.0, 65.0, 100.0));
    assert_eq!(loaded.time(), 6000);
    assert_eq!(loaded.weather(), Weather::Clear);
}

// Test helper types (would be in engine_world)
#[derive(Debug, Clone, PartialEq)]
struct PlayerData {
    uuid: String,
    position: (f64, f64, f64),
    rotation: (f32, f32),
    health: u32,
    max_health: u32,
    inventory: Vec<(u32, u32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Weather {
    Clear,
    Rain,
    Thunder,
}
