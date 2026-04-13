//! World metadata and persistence.
//!
//! Manages world saves including metadata and chunk storage via region files.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use engine_core::coords::{ChunkPos, WorldPos};
use glam::IVec2;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::region::{chunk_to_local, chunk_to_region, region_filename, Region, RegionError};
use crate::chunk::Chunk;

/// World metadata stored in world.json.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldMeta {
    /// World generation seed.
    pub seed: u64,
    /// Player spawn position.
    pub spawn: WorldPos,
    /// In-game time (day/night cycle).
    pub game_time: f64,
    /// World display name.
    pub name: String,
}

impl WorldMeta {
    /// Create new world metadata.
    #[must_use]
    pub fn new(seed: u64, name: &str) -> Self {
        Self {
            seed,
            spawn: WorldPos::new(0, 64, 0),
            game_time: 0.0,
            name: name.to_string(),
        }
    }
}

/// Error type for world persistence operations.
#[derive(Debug, Error)]
pub enum WorldError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Region error: {0}")]
    Region(#[from] RegionError),

    #[error("World not found at: {0}")]
    NotFound(PathBuf),

    #[error("World already exists at: {0}")]
    AlreadyExists(PathBuf),
}

/// World persistence manager.
///
/// Handles loading and saving world data including metadata and chunks.
pub struct WorldPersistence {
    /// Root directory for this world.
    root: PathBuf,
    /// World metadata.
    meta: WorldMeta,
    /// Open region files (cached for performance).
    regions: HashMap<IVec2, Region>,
    /// Directory for region files.
    regions_dir: PathBuf,
    /// Whether metadata needs to be saved.
    meta_dirty: bool,
}

impl WorldPersistence {
    /// Create a new world at the given path.
    ///
    /// # Errors
    /// Returns an error if the world already exists or cannot be created.
    pub fn create(path: &Path, seed: u64, name: &str) -> Result<Self, WorldError> {
        if path.exists() {
            return Err(WorldError::AlreadyExists(path.to_path_buf()));
        }

        // Create directory structure
        fs::create_dir_all(path)?;
        let regions_dir = path.join("regions");
        fs::create_dir_all(&regions_dir)?;

        // Create and save metadata
        let meta = WorldMeta::new(seed, name);
        let meta_path = path.join("world.json");
        let meta_json = serde_json::to_string_pretty(&meta)?;
        fs::write(&meta_path, meta_json)?;

        Ok(Self {
            root: path.to_path_buf(),
            meta,
            regions: HashMap::new(),
            regions_dir,
            meta_dirty: false,
        })
    }

    /// Open an existing world at the given path.
    ///
    /// # Errors
    /// Returns an error if the world does not exist or cannot be read.
    pub fn open(path: &Path) -> Result<Self, WorldError> {
        if !path.exists() {
            return Err(WorldError::NotFound(path.to_path_buf()));
        }

        let meta_path = path.join("world.json");
        if !meta_path.exists() {
            return Err(WorldError::NotFound(meta_path));
        }

        let meta_json = fs::read_to_string(&meta_path)?;
        let meta: WorldMeta = serde_json::from_str(&meta_json)?;

        let regions_dir = path.join("regions");
        if !regions_dir.exists() {
            fs::create_dir_all(&regions_dir)?;
        }

        Ok(Self {
            root: path.to_path_buf(),
            meta,
            regions: HashMap::new(),
            regions_dir,
            meta_dirty: false,
        })
    }

    /// Get world metadata.
    #[must_use]
    pub fn meta(&self) -> &WorldMeta {
        &self.meta
    }

    /// Get mutable world metadata.
    pub fn meta_mut(&mut self) -> &mut WorldMeta {
        self.meta_dirty = true;
        &mut self.meta
    }

    /// Get world seed.
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.meta.seed
    }

    /// Get world name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.meta.name
    }

    /// Get world root directory.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Save a chunk to disk.
    ///
    /// # Errors
    /// Returns an error if the chunk cannot be saved.
    pub fn save_chunk(&mut self, pos: ChunkPos, chunk: &Chunk) -> Result<(), WorldError> {
        let chunk_2d = IVec2::new(pos.x(), pos.z());
        let region_pos = chunk_to_region(chunk_2d);
        let local_pos = chunk_to_local(chunk_2d);

        let region = self.get_or_open_region(region_pos)?;
        region.save_chunk(local_pos, chunk)?;

        Ok(())
    }

    /// Load a chunk from disk.
    ///
    /// Returns `None` if the chunk has not been saved.
    ///
    /// # Errors
    /// Returns an error if the chunk cannot be loaded.
    pub fn load_chunk(&mut self, pos: ChunkPos) -> Result<Option<Chunk>, WorldError> {
        let chunk_2d = IVec2::new(pos.x(), pos.z());
        let region_pos = chunk_to_region(chunk_2d);
        let local_pos = chunk_to_local(chunk_2d);

        // Check if region file exists
        let region_path = self.regions_dir.join(region_filename(region_pos));
        if !region_path.exists() {
            return Ok(None);
        }

        let region = self.get_or_open_region(region_pos)?;
        let chunk = region.load_chunk(local_pos)?;

        Ok(chunk)
    }

    /// Check if a chunk exists on disk.
    #[must_use]
    pub fn has_chunk(&self, pos: ChunkPos) -> bool {
        let chunk_2d = IVec2::new(pos.x(), pos.z());
        let region_pos = chunk_to_region(chunk_2d);

        // Check if region is already open
        if let Some(region) = self.regions.get(&region_pos) {
            let local_pos = chunk_to_local(chunk_2d);
            return region.has_chunk(local_pos);
        }

        // Check if region file exists
        let region_path = self.regions_dir.join(region_filename(region_pos));
        region_path.exists()
    }

    /// Flush all pending changes to disk.
    ///
    /// # Errors
    /// Returns an error if data cannot be written.
    pub fn flush(&mut self) -> Result<(), WorldError> {
        // Save metadata if dirty
        if self.meta_dirty {
            let meta_path = self.root.join("world.json");
            let meta_json = serde_json::to_string_pretty(&self.meta)?;
            fs::write(&meta_path, meta_json)?;
            self.meta_dirty = false;
        }

        // Flush all regions
        for region in self.regions.values_mut() {
            region.flush()?;
        }

        Ok(())
    }

    /// Close and remove a region from the cache.
    pub fn close_region(&mut self, region_pos: IVec2) {
        if let Some(mut region) = self.regions.remove(&region_pos) {
            let _ = region.flush();
        }
    }

    /// Get the number of open regions.
    #[must_use]
    pub fn open_region_count(&self) -> usize {
        self.regions.len()
    }

    /// Get or open a region file.
    fn get_or_open_region(&mut self, region_pos: IVec2) -> Result<&mut Region, WorldError> {
        if !self.regions.contains_key(&region_pos) {
            let region_path = self.regions_dir.join(region_filename(region_pos));
            let region = Region::open(&region_path)?;
            self.regions.insert(region_pos, region);
        }

        Ok(self.regions.get_mut(&region_pos).unwrap())
    }
}

impl Drop for WorldPersistence {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{BlockId, STONE};
    use engine_core::coords::LocalPos;
    use tempfile::TempDir;

    fn test_chunk() -> Chunk {
        let mut chunk = Chunk::new();
        chunk.set(LocalPos::new(0, 0, 0), STONE);
        chunk.set(LocalPos::new(8, 8, 8), BlockId(10));
        chunk
    }

    #[test]
    fn test_create_world() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test_world");

        let world = WorldPersistence::create(&path, 12345, "Test World").unwrap();

        assert_eq!(world.seed(), 12345);
        assert_eq!(world.name(), "Test World");
        assert!(path.exists());
        assert!(path.join("world.json").exists());
        assert!(path.join("regions").exists());
    }

    #[test]
    fn test_open_world() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test_world");

        // Create world
        {
            let _world = WorldPersistence::create(&path, 12345, "Test World").unwrap();
        }

        // Reopen
        {
            let world = WorldPersistence::open(&path).unwrap();
            assert_eq!(world.seed(), 12345);
            assert_eq!(world.name(), "Test World");
        }
    }

    #[test]
    fn test_world_not_found() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent");

        let result = WorldPersistence::open(&path);
        assert!(matches!(result, Err(WorldError::NotFound(_))));
    }

    #[test]
    fn test_world_already_exists() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test_world");

        WorldPersistence::create(&path, 12345, "Test").unwrap();

        let result = WorldPersistence::create(&path, 54321, "Another");
        assert!(matches!(result, Err(WorldError::AlreadyExists(_))));
    }

    #[test]
    fn test_save_load_chunk() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test_world");

        let chunk = test_chunk();
        let pos = ChunkPos::new(10, 0, 20);

        // Save chunk
        {
            let mut world = WorldPersistence::create(&path, 12345, "Test").unwrap();
            world.save_chunk(pos, &chunk).unwrap();
            world.flush().unwrap();
        }

        // Load chunk
        {
            let mut world = WorldPersistence::open(&path).unwrap();
            let loaded = world.load_chunk(pos).unwrap().unwrap();
            assert_eq!(loaded.get(LocalPos::new(0, 0, 0)), STONE);
            assert_eq!(loaded.get(LocalPos::new(8, 8, 8)), BlockId(10));
        }
    }

    #[test]
    fn test_chunk_not_saved() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test_world");

        let mut world = WorldPersistence::create(&path, 12345, "Test").unwrap();
        let pos = ChunkPos::new(999, 0, 999);

        let result = world.load_chunk(pos).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_meta_persistence() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test_world");

        // Create and modify
        {
            let mut world = WorldPersistence::create(&path, 12345, "Test").unwrap();
            world.meta_mut().game_time = 1000.0;
            world.meta_mut().spawn = WorldPos::new(100, 50, 200);
            world.flush().unwrap();
        }

        // Reopen and verify
        {
            let world = WorldPersistence::open(&path).unwrap();
            assert_eq!(world.meta().game_time, 1000.0);
            assert_eq!(world.meta().spawn.x(), 100);
            assert_eq!(world.meta().spawn.y(), 50);
            assert_eq!(world.meta().spawn.z(), 200);
        }
    }

    #[test]
    fn test_multiple_regions() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test_world");

        let mut world = WorldPersistence::create(&path, 12345, "Test").unwrap();

        // Save chunks in different regions
        let chunk = test_chunk();
        world.save_chunk(ChunkPos::new(0, 0, 0), &chunk).unwrap();
        world.save_chunk(ChunkPos::new(35, 0, 0), &chunk).unwrap(); // Different region (x)
        world.save_chunk(ChunkPos::new(0, 0, 40), &chunk).unwrap(); // Different region (z)

        world.flush().unwrap();

        // Should have 3 regions open
        assert_eq!(world.open_region_count(), 3);
    }
}
