//! Region file format for chunk storage.
//!
//! Each region file stores a 32x32 grid of chunks (1024 chunks).
//! Format: header + offset table + chunk data.

use crate::chunk::Chunk;
use glam::IVec2;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Region dimensions (32x32 chunks).
pub const REGION_SIZE: i32 = 32;

/// Number of chunks per region.
const CHUNKS_PER_REGION: usize = (REGION_SIZE * REGION_SIZE) as usize;

/// Magic bytes for region file format.
const MAGIC: &[u8; 4] = b"LREG";

/// Current format version.
const VERSION: u32 = 1;

/// Header size in bytes.
const HEADER_SIZE: usize = 4 + 4 + (CHUNKS_PER_REGION * 8);

/// Error type for region operations.
#[derive(Debug, Error)]
pub enum RegionError {
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

/// Offset table entry.
#[derive(Clone, Copy, Debug, Default)]
struct ChunkOffset {
    /// Offset from start of file (0 = not present).
    offset: u32,
    /// Size of compressed data.
    size: u32,
}

/// Region file managing 32x32 chunks.
pub struct Region {
    path: PathBuf,
    file: File,
    offsets: [ChunkOffset; CHUNKS_PER_REGION],
    /// Next write position for new chunks.
    next_offset: u32,
    /// Dirty flag for offset table.
    dirty: bool,
}

impl Region {
    /// Open or create a region file.
    pub fn open(path: &Path) -> Result<Self, RegionError> {
        if path.exists() {
            Self::open_existing(path)
        } else {
            Self::create_new(path)
        }
    }

    /// Create a new region file.
    fn create_new(path: &Path) -> Result<Self, RegionError> {
        // Ensure parent directory exists
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

        // Write empty offset table
        let empty_offsets = [0u8; CHUNKS_PER_REGION * 8];
        file.write_all(&empty_offsets)?;

        file.flush()?;

        Ok(Self {
            path: path.to_path_buf(),
            file,
            offsets: [ChunkOffset::default(); CHUNKS_PER_REGION],
            next_offset: HEADER_SIZE as u32,
            dirty: false,
        })
    }

    /// Open an existing region file.
    fn open_existing(path: &Path) -> Result<Self, RegionError> {
        let mut file = OpenOptions::new().read(true).write(true).open(path)?;

        // Read and verify magic
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(RegionError::InvalidMagic);
        }

        // Read version
        let mut version_bytes = [0u8; 4];
        file.read_exact(&mut version_bytes)?;
        let version = u32::from_le_bytes(version_bytes);
        if version != VERSION {
            return Err(RegionError::UnsupportedVersion(version));
        }

        // Read offset table
        let mut offsets = [ChunkOffset::default(); CHUNKS_PER_REGION];
        for offset in &mut offsets {
            let mut buf = [0u8; 8];
            file.read_exact(&mut buf)?;
            offset.offset = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
            offset.size = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
        }

        // Calculate next offset (end of file)
        let next_offset = file.seek(SeekFrom::End(0))? as u32;

        Ok(Self {
            path: path.to_path_buf(),
            file,
            offsets,
            next_offset,
            dirty: false,
        })
    }

    /// Load a chunk from the region.
    ///
    /// `local` is the position within the region (0-31 for x and z).
    pub fn load_chunk(&mut self, local: IVec2) -> Result<Option<Chunk>, RegionError> {
        let index = self.local_to_index(local)?;
        let entry = self.offsets[index];

        if entry.offset == 0 {
            return Ok(None);
        }

        // Seek to chunk data
        self.file.seek(SeekFrom::Start(entry.offset as u64))?;

        // Read compressed data
        let mut compressed = vec![0u8; entry.size as usize];
        self.file.read_exact(&mut compressed)?;

        // Read CRC (last 4 bytes)
        if compressed.len() < 4 {
            return Err(RegionError::CorruptData);
        }
        let data_len = compressed.len() - 4;
        let stored_crc = u32::from_le_bytes([
            compressed[data_len],
            compressed[data_len + 1],
            compressed[data_len + 2],
            compressed[data_len + 3],
        ]);
        compressed.truncate(data_len);

        // Verify CRC
        let computed_crc = crc32fast::hash(&compressed);
        if computed_crc != stored_crc {
            return Err(RegionError::CorruptData);
        }

        // Decompress
        let decompressed = lz4_flex::decompress_size_prepended(&compressed)
            .map_err(|_| RegionError::Decompression)?;

        // Deserialize
        let chunk: Chunk = bincode::deserialize(&decompressed)?;

        Ok(Some(chunk))
    }

    /// Save a chunk to the region.
    ///
    /// `local` is the position within the region (0-31 for x and z).
    pub fn save_chunk(&mut self, local: IVec2, chunk: &Chunk) -> Result<(), RegionError> {
        let index = self.local_to_index(local)?;

        // Serialize chunk
        let serialized = bincode::serialize(chunk)?;

        // Compress with size prepended
        let compressed = lz4_flex::compress_prepend_size(&serialized);

        // Compute CRC
        let crc = crc32fast::hash(&compressed);

        // Total size including CRC
        let total_size = compressed.len() + 4;

        // Write at end of file
        self.file.seek(SeekFrom::Start(self.next_offset as u64))?;
        self.file.write_all(&compressed)?;
        self.file.write_all(&crc.to_le_bytes())?;

        // Update offset table
        self.offsets[index] = ChunkOffset {
            offset: self.next_offset,
            size: total_size as u32,
        };
        self.next_offset += total_size as u32;
        self.dirty = true;

        Ok(())
    }

    /// Flush changes to disk.
    pub fn flush(&mut self) -> Result<(), RegionError> {
        if self.dirty {
            // Write offset table
            self.file.seek(SeekFrom::Start(8))?; // After magic + version

            for offset in &self.offsets {
                self.file.write_all(&offset.offset.to_le_bytes())?;
                self.file.write_all(&offset.size.to_le_bytes())?;
            }

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
        if let Ok(index) = self.local_to_index(local) {
            self.offsets[index].offset != 0
        } else {
            false
        }
    }

    /// Get the number of stored chunks.
    #[must_use]
    pub fn chunk_count(&self) -> usize {
        self.offsets.iter().filter(|o| o.offset != 0).count()
    }

    /// Convert local position to index.
    fn local_to_index(&self, local: IVec2) -> Result<usize, RegionError> {
        if local.x < 0 || local.x >= REGION_SIZE || local.y < 0 || local.y >= REGION_SIZE {
            return Err(RegionError::OutOfBounds(local.x, local.y));
        }
        Ok((local.y * REGION_SIZE + local.x) as usize)
    }
}

impl Drop for Region {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

/// Convert chunk position to region position.
#[must_use]
pub fn chunk_to_region(chunk: IVec2) -> IVec2 {
    IVec2::new(
        chunk.x.div_euclid(REGION_SIZE),
        chunk.y.div_euclid(REGION_SIZE),
    )
}

/// Convert chunk position to local position within region.
#[must_use]
pub fn chunk_to_local(chunk: IVec2) -> IVec2 {
    IVec2::new(
        chunk.x.rem_euclid(REGION_SIZE),
        chunk.y.rem_euclid(REGION_SIZE),
    )
}

/// Generate region file name for a region position.
#[must_use]
pub fn region_filename(region: IVec2) -> String {
    format!("r.{}.{}.lreg", region.x, region.y)
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
    fn test_create_and_open() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lreg");

        // Create new region
        {
            let region = Region::open(&path).unwrap();
            assert_eq!(region.chunk_count(), 0);
        }

        // Reopen
        {
            let region = Region::open(&path).unwrap();
            assert_eq!(region.chunk_count(), 0);
        }
    }

    #[test]
    fn test_save_load_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lreg");

        let chunk = test_chunk();
        let local = IVec2::new(5, 10);

        // Save chunk
        {
            let mut region = Region::open(&path).unwrap();
            region.save_chunk(local, &chunk).unwrap();
            region.flush().unwrap();
        }

        // Load chunk
        {
            let mut region = Region::open(&path).unwrap();
            let loaded = region.load_chunk(local).unwrap().unwrap();
            assert_eq!(loaded.get(LocalPos::new(0, 0, 0)), STONE);
            assert_eq!(loaded.get(LocalPos::new(8, 8, 8)), BlockId(10));
        }
    }

    #[test]
    fn test_compression_works() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lreg");

        // Create a chunk with lots of the same block (compresses well)
        let mut chunk = Chunk::new();
        for x in 0..16 {
            for z in 0..16 {
                chunk.set(LocalPos::new(x, 0, z), STONE);
            }
        }

        let mut region = Region::open(&path).unwrap();
        region.save_chunk(IVec2::ZERO, &chunk).unwrap();
        region.flush().unwrap();

        // File should be smaller than uncompressed chunk data
        // Raw chunk = 4096 * 2 bytes = 8KB
        // Header = 4 + 4 + (1024 * 8) = 8200 bytes
        // With compression, the chunk data itself should be small
        let file_size = std::fs::metadata(&path).unwrap().len();
        // Header is ~8200 bytes, compressed chunk data should be minimal
        // Total should be well under 16KB
        assert!(file_size < 16384, "File size was {} bytes", file_size);
    }

    #[test]
    fn test_corrupt_data_detected() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lreg");

        let chunk = test_chunk();

        // Save chunk
        {
            let mut region = Region::open(&path).unwrap();
            region.save_chunk(IVec2::ZERO, &chunk).unwrap();
            region.flush().unwrap();
        }

        // Corrupt the file
        {
            let mut file = OpenOptions::new().write(true).open(&path).unwrap();
            file.seek(SeekFrom::Start(HEADER_SIZE as u64 + 10))
                .unwrap();
            file.write_all(&[0xFF, 0xFF, 0xFF, 0xFF]).unwrap();
        }

        // Try to load - should fail with CRC error
        {
            let mut region = Region::open(&path).unwrap();
            let result = region.load_chunk(IVec2::ZERO);
            assert!(matches!(
                result,
                Err(RegionError::CorruptData) | Err(RegionError::Decompression)
            ));
        }
    }

    #[test]
    fn test_chunk_to_region() {
        assert_eq!(chunk_to_region(IVec2::new(0, 0)), IVec2::new(0, 0));
        assert_eq!(chunk_to_region(IVec2::new(31, 31)), IVec2::new(0, 0));
        assert_eq!(chunk_to_region(IVec2::new(32, 0)), IVec2::new(1, 0));
        assert_eq!(chunk_to_region(IVec2::new(-1, 0)), IVec2::new(-1, 0));
        assert_eq!(chunk_to_region(IVec2::new(-32, 0)), IVec2::new(-1, 0));
        assert_eq!(chunk_to_region(IVec2::new(-33, 0)), IVec2::new(-2, 0));
    }

    #[test]
    fn test_chunk_to_local() {
        assert_eq!(chunk_to_local(IVec2::new(0, 0)), IVec2::new(0, 0));
        assert_eq!(chunk_to_local(IVec2::new(31, 31)), IVec2::new(31, 31));
        assert_eq!(chunk_to_local(IVec2::new(32, 0)), IVec2::new(0, 0));
        assert_eq!(chunk_to_local(IVec2::new(-1, 0)), IVec2::new(31, 0));
        assert_eq!(chunk_to_local(IVec2::new(-32, 0)), IVec2::new(0, 0));
    }
}
