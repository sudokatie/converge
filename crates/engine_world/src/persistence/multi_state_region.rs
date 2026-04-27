//! Multi-state region file format for persisting chunks with multiple states.
//!
//! Extends the region file format to store alternate dimensions, time-loop
//! snapshots, and phased realities.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use glam::IVec2;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::multi_state_chunk::{MultiStateChunk, StateFallback};
use super::region::REGION_SIZE;
use super::state_id::StateId;
use crate::chunk::Chunk;

/// Magic bytes for multi-state region file format.
const MAGIC: &[u8; 4] = b"LMSF";

/// Current format version.
const VERSION: u32 = 1;

/// Number of chunks per region (32x32).
const CHUNKS_PER_REGION: usize = (REGION_SIZE * REGION_SIZE) as usize;

/// Header size: magic + version + state count + reserved (4 bytes each).
const HEADER_BASE_SIZE: usize = 16;

/// Per-chunk entry size: offset + size + state count + reserved.
const INDEX_ENTRY_SIZE: usize = 12;

/// Error type for multi-state region operations.
#[derive(Debug, Error)]
pub enum MultiStateRegionError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid magic bytes")]
    InvalidMagic,

    #[error("Unsupported version: {0}")]
    UnsupportedVersion(u32),

    #[error("Chunk position out of bounds: ({0}, {1})")]
    OutOfBounds(i32, i32),

    #[error("Corrupt chunk data: CRC mismatch")]
    CorruptData,

    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    #[error("Decompression error")]
    Decompression,
}

/// Index entry for a chunk in the region.
#[derive(Clone, Copy, Debug, Default)]
struct ChunkIndex {
    /// Offset from start of file (0 = not present).
    offset: u32,
    /// Size of compressed data.
    size: u32,
    /// Number of states stored.
    state_count: u16,
}

/// Serialized chunk data wrapper.
#[derive(Serialize, Deserialize)]
struct SerializedMultiStateChunk {
    active: StateId,
    fallback: StateFallback,
    states: Vec<(StateId, Chunk)>,
}

impl From<&MultiStateChunk> for SerializedMultiStateChunk {
    fn from(msc: &MultiStateChunk) -> Self {
        Self {
            active: msc.active_state(),
            fallback: msc.fallback(),
            states: msc.iter().map(|(id, chunk)| (id, chunk.clone())).collect(),
        }
    }
}

impl From<SerializedMultiStateChunk> for MultiStateChunk {
    fn from(serialized: SerializedMultiStateChunk) -> Self {
        let mut msc = MultiStateChunk::empty();
        for (id, chunk) in serialized.states {
            msc.insert(id, chunk);
        }
        msc.set_active_state(serialized.active);
        msc.set_fallback(serialized.fallback);
        msc
    }
}

/// Multi-state region file managing 32x32 chunks with multiple states each.
pub struct MultiStateRegion {
    path: PathBuf,
    file: File,
    index: [ChunkIndex; CHUNKS_PER_REGION],
    /// Next write position for new data.
    next_offset: u32,
    /// Dirty flag for index table.
    dirty: bool,
}

impl MultiStateRegion {
    /// Open or create a multi-state region file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened/created, has invalid format,
    /// or uses an unsupported version.
    pub fn open(path: &Path) -> Result<Self, MultiStateRegionError> {
        if path.exists() {
            Self::open_existing(path)
        } else {
            Self::create_new(path)
        }
    }

    /// Create a new multi-state region file.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "header size is a known constant fitting in u32"
    )]
    fn create_new(path: &Path) -> Result<Self, MultiStateRegionError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;

        // Write header
        file.write_all(MAGIC)?;
        file.write_all(&VERSION.to_le_bytes())?;
        file.write_all(&0u32.to_le_bytes())?; // total state count (placeholder)
        file.write_all(&0u32.to_le_bytes())?; // reserved

        // Write empty index
        let empty_index = [0u8; CHUNKS_PER_REGION * INDEX_ENTRY_SIZE];
        file.write_all(&empty_index)?;

        file.flush()?;

        let header_total = HEADER_BASE_SIZE + (CHUNKS_PER_REGION * INDEX_ENTRY_SIZE);

        Ok(Self {
            path: path.to_path_buf(),
            file,
            index: [ChunkIndex::default(); CHUNKS_PER_REGION],
            next_offset: header_total as u32,
            dirty: false,
        })
    }

    /// Open an existing multi-state region file.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "file offsets bounded by format, fit in u32"
    )]
    fn open_existing(path: &Path) -> Result<Self, MultiStateRegionError> {
        let mut file = OpenOptions::new().read(true).write(true).open(path)?;

        // Read and verify magic
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(MultiStateRegionError::InvalidMagic);
        }

        // Read version
        let mut version_bytes = [0u8; 4];
        file.read_exact(&mut version_bytes)?;
        let version = u32::from_le_bytes(version_bytes);
        if version != VERSION {
            return Err(MultiStateRegionError::UnsupportedVersion(version));
        }

        // Skip total state count and reserved
        file.seek(SeekFrom::Current(8))?;

        // Read index
        let mut index = [ChunkIndex::default(); CHUNKS_PER_REGION];
        for entry in &mut index {
            let mut buf = [0u8; INDEX_ENTRY_SIZE];
            file.read_exact(&mut buf)?;
            entry.offset = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
            entry.size = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
            entry.state_count = u16::from_le_bytes([buf[8], buf[9]]);
        }

        let next_offset = file.seek(SeekFrom::End(0))? as u32;

        Ok(Self {
            path: path.to_path_buf(),
            file,
            index,
            next_offset,
            dirty: false,
        })
    }

    /// Load a multi-state chunk from the region.
    ///
    /// `local` is the position within the region (0-31 for x and z).
    ///
    /// # Errors
    ///
    /// Returns an error if position is out of bounds, IO fails,
    /// data is corrupt, or deserialization fails.
    pub fn load_chunk(
        &mut self,
        local: IVec2,
    ) -> Result<Option<MultiStateChunk>, MultiStateRegionError> {
        let idx = Self::local_to_index(local)?;
        let entry = self.index[idx];

        if entry.offset == 0 {
            return Ok(None);
        }

        self.file.seek(SeekFrom::Start(u64::from(entry.offset)))?;

        let mut compressed = vec![0u8; entry.size as usize];
        self.file.read_exact(&mut compressed)?;

        // CRC is last 4 bytes
        if compressed.len() < 4 {
            return Err(MultiStateRegionError::CorruptData);
        }
        let data_len = compressed.len() - 4;
        let stored_crc = u32::from_le_bytes([
            compressed[data_len],
            compressed[data_len + 1],
            compressed[data_len + 2],
            compressed[data_len + 3],
        ]);
        compressed.truncate(data_len);

        let computed_crc = crc32fast::hash(&compressed);
        if computed_crc != stored_crc {
            return Err(MultiStateRegionError::CorruptData);
        }

        let decompressed = lz4_flex::decompress_size_prepended(&compressed)
            .map_err(|_| MultiStateRegionError::Decompression)?;

        let serialized: SerializedMultiStateChunk = bincode::deserialize(&decompressed)?;
        Ok(Some(serialized.into()))
    }

    /// Save a multi-state chunk to the region.
    ///
    /// `local` is the position within the region (0-31 for x and z).
    ///
    /// # Errors
    ///
    /// Returns an error if position is out of bounds, serialization fails,
    /// or IO fails.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "chunk data size bounded, fits in u32"
    )]
    pub fn save_chunk(
        &mut self,
        local: IVec2,
        chunk: &MultiStateChunk,
    ) -> Result<(), MultiStateRegionError> {
        let idx = Self::local_to_index(local)?;

        let serialized: SerializedMultiStateChunk = chunk.into();
        let bytes = bincode::serialize(&serialized)?;
        let compressed = lz4_flex::compress_prepend_size(&bytes);
        let crc = crc32fast::hash(&compressed);

        let total_size = compressed.len() + 4;

        self.file
            .seek(SeekFrom::Start(u64::from(self.next_offset)))?;
        self.file.write_all(&compressed)?;
        self.file.write_all(&crc.to_le_bytes())?;

        self.index[idx] = ChunkIndex {
            offset: self.next_offset,
            size: total_size as u32,
            state_count: chunk.state_count() as u16,
        };
        self.next_offset += total_size as u32;
        self.dirty = true;

        Ok(())
    }

    /// Load only a specific state from a chunk.
    ///
    /// More efficient than loading the full multi-state chunk when only
    /// one state is needed (though currently loads all and extracts).
    ///
    /// # Errors
    ///
    /// Returns an error if the chunk cannot be loaded due to IO or corruption.
    pub fn load_state(
        &mut self,
        local: IVec2,
        state: StateId,
    ) -> Result<Option<Chunk>, MultiStateRegionError> {
        let msc = self.load_chunk(local)?;
        Ok(msc.and_then(|m| m.get(state).cloned()))
    }

    /// Load a state with fallback to primary.
    ///
    /// Returns the requested state if it exists, otherwise the primary state.
    ///
    /// # Errors
    ///
    /// Returns an error if the chunk cannot be loaded due to IO or corruption.
    pub fn load_state_or_primary(
        &mut self,
        local: IVec2,
        state: StateId,
    ) -> Result<Option<Chunk>, MultiStateRegionError> {
        let msc = self.load_chunk(local)?;
        Ok(msc.and_then(|m| m.get(state).or_else(|| m.get_primary()).cloned()))
    }

    /// Flush changes to disk.
    ///
    /// # Errors
    ///
    /// Returns an error if writing fails.
    pub fn flush(&mut self) -> Result<(), MultiStateRegionError> {
        if self.dirty {
            // Write index
            self.file.seek(SeekFrom::Start(HEADER_BASE_SIZE as u64))?;

            for entry in &self.index {
                self.file.write_all(&entry.offset.to_le_bytes())?;
                self.file.write_all(&entry.size.to_le_bytes())?;
                self.file.write_all(&entry.state_count.to_le_bytes())?;
                self.file.write_all(&[0, 0])?; // reserved padding
            }

            // Update total state count in header
            let total_states: u32 = self.index.iter().map(|e| u32::from(e.state_count)).sum();
            self.file.seek(SeekFrom::Start(8))?;
            self.file.write_all(&total_states.to_le_bytes())?;

            self.file.flush()?;
            self.dirty = false;
        }

        Ok(())
    }

    /// Get the path to this region file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Check if a chunk exists in the region.
    #[must_use]
    pub fn has_chunk(&self, local: IVec2) -> bool {
        if let Ok(idx) = Self::local_to_index(local) {
            self.index[idx].offset != 0
        } else {
            false
        }
    }

    /// Get the number of stored chunks.
    #[must_use]
    pub fn chunk_count(&self) -> usize {
        self.index.iter().filter(|e| e.offset != 0).count()
    }

    /// Get the total number of states across all chunks.
    #[must_use]
    pub fn total_state_count(&self) -> usize {
        self.index.iter().map(|e| usize::from(e.state_count)).sum()
    }

    /// Get state count for a specific chunk.
    #[must_use]
    pub fn chunk_state_count(&self, local: IVec2) -> Option<u16> {
        Self::local_to_index(local).ok().and_then(|idx| {
            let entry = self.index[idx];
            if entry.offset != 0 {
                Some(entry.state_count)
            } else {
                None
            }
        })
    }

    /// Iterate over chunk positions that have data.
    pub fn iter_chunk_positions(&self) -> impl Iterator<Item = IVec2> + '_ {
        self.index.iter().enumerate().filter_map(|(idx, entry)| {
            if entry.offset != 0 {
                Some(Self::index_to_local(idx))
            } else {
                None
            }
        })
    }

    /// Convert local position to index.
    #[expect(
        clippy::cast_sign_loss,
        reason = "bounds check guarantees non-negative"
    )]
    fn local_to_index(local: IVec2) -> Result<usize, MultiStateRegionError> {
        if local.x < 0 || local.x >= REGION_SIZE || local.y < 0 || local.y >= REGION_SIZE {
            return Err(MultiStateRegionError::OutOfBounds(local.x, local.y));
        }
        Ok((local.y * REGION_SIZE + local.x) as usize)
    }

    /// Convert index to local position.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "index is bounded by CHUNKS_PER_REGION"
    )]
    fn index_to_local(idx: usize) -> IVec2 {
        let x = (idx % REGION_SIZE as usize) as i32;
        let y = (idx / REGION_SIZE as usize) as i32;
        IVec2::new(x, y)
    }
}

impl Drop for MultiStateRegion {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

/// Summary statistics for a multi-state region.
#[derive(Clone, Debug, Default)]
pub struct RegionStats {
    /// Number of chunks stored.
    pub chunk_count: usize,
    /// Total number of states across all chunks.
    pub total_states: usize,
    /// Map of state ID to count.
    pub states_by_id: HashMap<StateId, usize>,
}

impl MultiStateRegion {
    /// Compute detailed statistics about the region.
    ///
    /// This requires loading all chunks and is expensive.
    ///
    /// # Errors
    ///
    /// Returns an error if any chunk fails to load due to IO or corruption.
    pub fn compute_stats(&mut self) -> Result<RegionStats, MultiStateRegionError> {
        let mut stats = RegionStats::default();

        for pos in self.iter_chunk_positions().collect::<Vec<_>>() {
            if let Some(msc) = self.load_chunk(pos)? {
                stats.chunk_count += 1;
                stats.total_states += msc.state_count();

                for state_id in msc.state_ids() {
                    *stats.states_by_id.entry(state_id).or_insert(0) += 1;
                }
            }
        }

        Ok(stats)
    }
}

/// Generate multi-state region file name.
#[must_use]
pub fn multi_state_region_filename(region: IVec2) -> String {
    format!("ms.{}.{}.lmsf", region.x, region.y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{BlockId, STONE};
    use engine_core::coords::LocalPos;
    use tempfile::TempDir;

    fn test_chunk(marker: BlockId) -> Chunk {
        let mut chunk = Chunk::new();
        chunk.set(LocalPos::new(0, 0, 0), marker);
        chunk
    }

    fn test_multi_state_chunk() -> MultiStateChunk {
        let mut msc = MultiStateChunk::new(test_chunk(STONE));
        msc.insert(StateId::new(1), test_chunk(BlockId(100)));
        msc.insert(StateId::new(2), test_chunk(BlockId(200)));
        msc
    }

    #[test]
    fn test_create_and_open() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lmsf");

        {
            let region = MultiStateRegion::open(&path).unwrap();
            assert_eq!(region.chunk_count(), 0);
            assert_eq!(region.total_state_count(), 0);
        }

        {
            let region = MultiStateRegion::open(&path).unwrap();
            assert_eq!(region.chunk_count(), 0);
        }
    }

    #[test]
    fn test_save_load_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lmsf");

        let msc = test_multi_state_chunk();
        let local = IVec2::new(5, 10);

        {
            let mut region = MultiStateRegion::open(&path).unwrap();
            region.save_chunk(local, &msc).unwrap();
            region.flush().unwrap();
        }

        {
            let mut region = MultiStateRegion::open(&path).unwrap();
            let loaded = region.load_chunk(local).unwrap().unwrap();

            assert_eq!(loaded.state_count(), 3);
            assert!(loaded.has_primary());
            assert!(loaded.has_state(StateId::new(1)));
            assert!(loaded.has_state(StateId::new(2)));

            assert_eq!(
                loaded.get_primary().unwrap().get(LocalPos::new(0, 0, 0)),
                STONE
            );
            assert_eq!(
                loaded
                    .get(StateId::new(1))
                    .unwrap()
                    .get(LocalPos::new(0, 0, 0)),
                BlockId(100)
            );
        }
    }

    #[test]
    fn test_load_specific_state() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lmsf");

        let msc = test_multi_state_chunk();
        let local = IVec2::ZERO;

        {
            let mut region = MultiStateRegion::open(&path).unwrap();
            region.save_chunk(local, &msc).unwrap();
            region.flush().unwrap();
        }

        {
            let mut region = MultiStateRegion::open(&path).unwrap();

            let state1 = region.load_state(local, StateId::new(1)).unwrap().unwrap();
            assert_eq!(state1.get(LocalPos::new(0, 0, 0)), BlockId(100));

            let missing = region.load_state(local, StateId::new(999)).unwrap();
            assert!(missing.is_none());
        }
    }

    #[test]
    fn test_load_state_or_primary() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lmsf");

        let msc = test_multi_state_chunk();
        let local = IVec2::ZERO;

        {
            let mut region = MultiStateRegion::open(&path).unwrap();
            region.save_chunk(local, &msc).unwrap();
            region.flush().unwrap();
        }

        {
            let mut region = MultiStateRegion::open(&path).unwrap();

            let fallback = region
                .load_state_or_primary(local, StateId::new(999))
                .unwrap()
                .unwrap();
            assert_eq!(fallback.get(LocalPos::new(0, 0, 0)), STONE);
        }
    }

    #[test]
    fn test_active_state_preserved() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lmsf");

        let mut msc = test_multi_state_chunk();
        msc.set_active_state(StateId::new(2));
        msc.set_fallback(StateFallback::Primary);

        {
            let mut region = MultiStateRegion::open(&path).unwrap();
            region.save_chunk(IVec2::ZERO, &msc).unwrap();
            region.flush().unwrap();
        }

        {
            let mut region = MultiStateRegion::open(&path).unwrap();
            let loaded = region.load_chunk(IVec2::ZERO).unwrap().unwrap();

            assert_eq!(loaded.active_state(), StateId::new(2));
            assert_eq!(loaded.fallback(), StateFallback::Primary);
        }
    }

    #[test]
    fn test_chunk_state_count() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lmsf");

        let msc = test_multi_state_chunk();

        let mut region = MultiStateRegion::open(&path).unwrap();
        region.save_chunk(IVec2::ZERO, &msc).unwrap();
        region.flush().unwrap();

        assert_eq!(region.chunk_state_count(IVec2::ZERO), Some(3));
        assert_eq!(region.chunk_state_count(IVec2::new(1, 1)), None);
    }

    #[test]
    fn test_total_state_count() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lmsf");

        let mut region = MultiStateRegion::open(&path).unwrap();

        let msc1 = test_multi_state_chunk(); // 3 states
        let msc2 = MultiStateChunk::new(test_chunk(STONE)); // 1 state

        region.save_chunk(IVec2::new(0, 0), &msc1).unwrap();
        region.save_chunk(IVec2::new(1, 0), &msc2).unwrap();
        region.flush().unwrap();

        assert_eq!(region.chunk_count(), 2);
        assert_eq!(region.total_state_count(), 4);
    }

    #[test]
    fn test_iter_chunk_positions() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lmsf");

        let mut region = MultiStateRegion::open(&path).unwrap();

        let msc = MultiStateChunk::new(test_chunk(STONE));
        region.save_chunk(IVec2::new(5, 10), &msc).unwrap();
        region.save_chunk(IVec2::new(20, 25), &msc).unwrap();
        region.flush().unwrap();

        let positions: Vec<_> = region.iter_chunk_positions().collect();
        assert_eq!(positions.len(), 2);
        assert!(positions.contains(&IVec2::new(5, 10)));
        assert!(positions.contains(&IVec2::new(20, 25)));
    }

    #[test]
    fn test_corrupt_data_detected() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lmsf");

        let msc = test_multi_state_chunk();

        {
            let mut region = MultiStateRegion::open(&path).unwrap();
            region.save_chunk(IVec2::ZERO, &msc).unwrap();
            region.flush().unwrap();
        }

        // Corrupt the file
        {
            let header_size = HEADER_BASE_SIZE + (CHUNKS_PER_REGION * INDEX_ENTRY_SIZE);
            let mut file = OpenOptions::new().write(true).open(&path).unwrap();
            file.seek(SeekFrom::Start(header_size as u64 + 10)).unwrap();
            file.write_all(&[0xFF, 0xFF, 0xFF, 0xFF]).unwrap();
        }

        {
            let mut region = MultiStateRegion::open(&path).unwrap();
            let result = region.load_chunk(IVec2::ZERO);
            assert!(matches!(
                result,
                Err(MultiStateRegionError::CorruptData | MultiStateRegionError::Decompression)
            ));
        }
    }

    #[test]
    fn test_compression_efficiency() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lmsf");

        let mut msc = MultiStateChunk::empty();
        for i in 0..10 {
            let mut chunk = Chunk::new();
            for x in 0..16 {
                for z in 0..16 {
                    chunk.set(LocalPos::new(x, 0, z), STONE);
                }
            }
            msc.insert(StateId::new(i), chunk);
        }

        let mut region = MultiStateRegion::open(&path).unwrap();
        region.save_chunk(IVec2::ZERO, &msc).unwrap();
        region.flush().unwrap();

        let file_size = std::fs::metadata(&path).unwrap().len();
        // 10 states * ~8KB each uncompressed = ~80KB
        // With compression and header overhead, should be much smaller
        assert!(file_size < 100_000, "File was {file_size} bytes");
    }

    #[test]
    fn test_region_stats() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lmsf");

        let mut region = MultiStateRegion::open(&path).unwrap();

        let mut msc1 = MultiStateChunk::new(test_chunk(STONE));
        msc1.insert(StateId::new(1), test_chunk(BlockId(1)));
        msc1.insert(StateId::new(2), test_chunk(BlockId(2)));

        let mut msc2 = MultiStateChunk::new(test_chunk(STONE));
        msc2.insert(StateId::new(1), test_chunk(BlockId(1)));

        region.save_chunk(IVec2::new(0, 0), &msc1).unwrap();
        region.save_chunk(IVec2::new(1, 0), &msc2).unwrap();
        region.flush().unwrap();

        let stats = region.compute_stats().unwrap();
        assert_eq!(stats.chunk_count, 2);
        assert_eq!(stats.total_states, 5);
        assert_eq!(stats.states_by_id.get(&StateId::PRIMARY), Some(&2));
        assert_eq!(stats.states_by_id.get(&StateId::new(1)), Some(&2));
        assert_eq!(stats.states_by_id.get(&StateId::new(2)), Some(&1));
    }

    #[test]
    fn test_filename() {
        assert_eq!(multi_state_region_filename(IVec2::new(0, 0)), "ms.0.0.lmsf");
        assert_eq!(
            multi_state_region_filename(IVec2::new(-5, 3)),
            "ms.-5.3.lmsf"
        );
    }

    #[test]
    fn test_empty_chunk_persistence() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lmsf");

        let msc = MultiStateChunk::empty();

        {
            let mut region = MultiStateRegion::open(&path).unwrap();
            region.save_chunk(IVec2::ZERO, &msc).unwrap();
            region.flush().unwrap();
        }

        {
            let mut region = MultiStateRegion::open(&path).unwrap();
            let loaded = region.load_chunk(IVec2::ZERO).unwrap().unwrap();
            assert!(loaded.is_empty());
        }
    }
}
