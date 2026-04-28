//! Anchor and origin metadata for megastructures.

use engine_core::coords::ChunkPos;
use glam::IVec3;
use serde::{Deserialize, Serialize};

/// Anchor point defining the origin and orientation of a megastructure.
///
/// The anchor serves as the reference point for:
/// - Coordinate transformations (local to world space)
/// - Streaming decisions (distance from anchor to observers)
/// - Persistence (anchor chunk is always saved first)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StructureAnchor {
    /// World position of the anchor point (in blocks).
    origin: IVec3,
    /// Chunk containing the anchor point.
    chunk: ChunkPos,
    /// Rotation index (0-23 for axis-aligned rotations, 0 = identity).
    rotation: u8,
}

impl StructureAnchor {
    /// Create a new anchor at the given world position.
    #[must_use]
    pub fn new(origin: IVec3) -> Self {
        Self {
            origin,
            chunk: world_to_chunk(origin),
            rotation: 0,
        }
    }

    /// Create an anchor with rotation.
    #[must_use]
    pub fn with_rotation(origin: IVec3, rotation: u8) -> Self {
        Self {
            origin,
            chunk: world_to_chunk(origin),
            rotation: rotation % 24,
        }
    }

    /// Get the world origin position.
    #[must_use]
    pub const fn origin(&self) -> IVec3 {
        self.origin
    }

    /// Get the chunk containing the anchor.
    #[must_use]
    pub const fn chunk(&self) -> ChunkPos {
        self.chunk
    }

    /// Get the rotation index.
    #[must_use]
    pub const fn rotation(&self) -> u8 {
        self.rotation
    }

    /// Set a new origin, updating the chunk position.
    pub fn set_origin(&mut self, origin: IVec3) {
        self.origin = origin;
        self.chunk = world_to_chunk(origin);
    }

    /// Set rotation.
    pub fn set_rotation(&mut self, rotation: u8) {
        self.rotation = rotation % 24;
    }

    /// Translate the anchor by an offset.
    pub fn translate(&mut self, offset: IVec3) {
        self.set_origin(self.origin + offset);
    }

    /// Calculate chunk offset from anchor to a world position.
    #[must_use]
    pub fn chunk_offset(&self, world_pos: IVec3) -> IVec3 {
        let target_chunk = world_to_chunk(world_pos);
        IVec3::new(
            target_chunk.x() - self.chunk.x(),
            target_chunk.y() - self.chunk.y(),
            target_chunk.z() - self.chunk.z(),
        )
    }
}

impl Default for StructureAnchor {
    fn default() -> Self {
        Self::new(IVec3::ZERO)
    }
}

/// Convert world position to chunk position.
fn world_to_chunk(pos: IVec3) -> ChunkPos {
    use engine_core::coords::CHUNK_SIZE;
    ChunkPos::new(
        pos.x.div_euclid(CHUNK_SIZE),
        pos.y.div_euclid(CHUNK_SIZE),
        pos.z.div_euclid(CHUNK_SIZE),
    )
}

/// Metadata attached to a structure's anchor for persistence and streaming.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorMetadata {
    /// Human-readable name for the structure.
    pub name: Option<String>,
    /// Custom tags for filtering/querying.
    pub tags: Vec<String>,
    /// Faction or owner identifier.
    pub owner_id: Option<u64>,
    /// Creation timestamp (unix seconds).
    pub created_at: u64,
    /// Last modification timestamp.
    pub modified_at: u64,
}

impl AnchorMetadata {
    /// Create new metadata with a name.
    #[must_use]
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            ..Default::default()
        }
    }

    /// Add a tag.
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set owner.
    #[must_use]
    pub const fn with_owner(mut self, owner_id: u64) -> Self {
        self.owner_id = Some(owner_id);
        self
    }

    /// Check if a tag is present.
    #[must_use]
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anchor_new() {
        let anchor = StructureAnchor::new(IVec3::new(100, 200, 300));
        assert_eq!(anchor.origin(), IVec3::new(100, 200, 300));
        assert_eq!(anchor.rotation(), 0);
    }

    #[test]
    fn test_anchor_chunk_calculation() {
        let anchor = StructureAnchor::new(IVec3::new(32, 64, 48));
        assert_eq!(anchor.chunk(), ChunkPos::new(2, 4, 3));
    }

    #[test]
    fn test_anchor_negative_coords() {
        let anchor = StructureAnchor::new(IVec3::new(-1, -16, -17));
        assert_eq!(anchor.chunk(), ChunkPos::new(-1, -1, -2));
    }

    #[test]
    fn test_anchor_with_rotation() {
        let anchor = StructureAnchor::with_rotation(IVec3::ZERO, 5);
        assert_eq!(anchor.rotation(), 5);

        let anchor = StructureAnchor::with_rotation(IVec3::ZERO, 30);
        assert_eq!(anchor.rotation(), 6);
    }

    #[test]
    fn test_anchor_translate() {
        let mut anchor = StructureAnchor::new(IVec3::new(0, 0, 0));
        anchor.translate(IVec3::new(16, 0, 0));
        assert_eq!(anchor.origin(), IVec3::new(16, 0, 0));
        assert_eq!(anchor.chunk(), ChunkPos::new(1, 0, 0));
    }

    #[test]
    fn test_chunk_offset() {
        let anchor = StructureAnchor::new(IVec3::new(0, 0, 0));
        let offset = anchor.chunk_offset(IVec3::new(48, 32, 16));
        assert_eq!(offset, IVec3::new(3, 2, 1));
    }

    #[test]
    fn test_metadata_named() {
        let meta = AnchorMetadata::named("Test Station");
        assert_eq!(meta.name.as_deref(), Some("Test Station"));
    }

    #[test]
    fn test_metadata_tags() {
        let meta = AnchorMetadata::default()
            .with_tag("military")
            .with_tag("orbital");
        assert!(meta.has_tag("military"));
        assert!(meta.has_tag("orbital"));
        assert!(!meta.has_tag("civilian"));
    }

    #[test]
    fn test_serde_anchor() {
        let anchor = StructureAnchor::with_rotation(IVec3::new(100, 200, 300), 7);
        let serialized = bincode::serialize(&anchor).unwrap();
        let deserialized: StructureAnchor = bincode::deserialize(&serialized).unwrap();
        assert_eq!(anchor, deserialized);
    }

    #[test]
    fn test_serde_metadata() {
        let meta = AnchorMetadata::named("Station Alpha")
            .with_tag("hub")
            .with_owner(12345);
        let serialized = bincode::serialize(&meta).unwrap();
        let deserialized: AnchorMetadata = bincode::deserialize(&serialized).unwrap();
        assert_eq!(meta, deserialized);
    }
}
