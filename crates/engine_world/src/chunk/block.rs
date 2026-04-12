//! Block types and registry.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Unique block identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockId(pub u16);

impl BlockId {
    /// Get the raw ID value.
    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }
}

// Built-in block IDs
/// Air (empty space).
pub const AIR: BlockId = BlockId(0);
/// Stone block.
pub const STONE: BlockId = BlockId(1);
/// Dirt block.
pub const DIRT: BlockId = BlockId(2);
/// Grass block (dirt with grass on top).
pub const GRASS: BlockId = BlockId(3);
/// Sand block.
pub const SAND: BlockId = BlockId(4);
/// Water block.
pub const WATER: BlockId = BlockId(5);

/// Properties of a block type.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockProperties {
    /// Display name.
    pub name: String,
    /// Whether the block is solid (can collide, can walk on).
    pub solid: bool,
    /// Whether the block is transparent (can see through).
    pub transparent: bool,
    /// Light emission level (0-15).
    pub light_emission: u8,
    /// Hardness (time to mine).
    pub hardness: f32,
    /// Texture indices for each face: [+X, -X, +Y, -Y, +Z, -Z].
    pub texture_indices: [u16; 6],
}

impl BlockProperties {
    /// Create properties for air.
    #[must_use]
    pub fn air() -> Self {
        Self {
            name: String::from("Air"),
            solid: false,
            transparent: true,
            light_emission: 0,
            hardness: 0.0,
            texture_indices: [0; 6],
        }
    }
}

impl Default for BlockProperties {
    fn default() -> Self {
        Self::air()
    }
}

/// Error type for block registry operations.
#[derive(Debug, Error)]
pub enum BlockRegistryError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(#[from] ron::de::SpannedError),
    #[error("Unknown block ID: {0}")]
    UnknownBlock(u16),
}

/// Registry of all block types.
#[derive(Debug)]
pub struct BlockRegistry {
    blocks: HashMap<BlockId, BlockProperties>,
}

impl BlockRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
        }
    }

    /// Create a registry with built-in blocks.
    #[must_use]
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();

        registry.register(
            AIR,
            BlockProperties {
                name: String::from("Air"),
                solid: false,
                transparent: true,
                light_emission: 0,
                hardness: 0.0,
                texture_indices: [0; 6],
            },
        );

        registry.register(
            STONE,
            BlockProperties {
                name: String::from("Stone"),
                solid: true,
                transparent: false,
                light_emission: 0,
                hardness: 1.5,
                texture_indices: [1; 6],
            },
        );

        registry.register(
            DIRT,
            BlockProperties {
                name: String::from("Dirt"),
                solid: true,
                transparent: false,
                light_emission: 0,
                hardness: 0.5,
                texture_indices: [2; 6],
            },
        );

        registry.register(
            GRASS,
            BlockProperties {
                name: String::from("Grass"),
                solid: true,
                transparent: false,
                light_emission: 0,
                hardness: 0.6,
                texture_indices: [3, 3, 4, 2, 3, 3], // Sides, top (grass), bottom (dirt)
            },
        );

        registry.register(
            SAND,
            BlockProperties {
                name: String::from("Sand"),
                solid: true,
                transparent: false,
                light_emission: 0,
                hardness: 0.5,
                texture_indices: [5; 6],
            },
        );

        registry.register(
            WATER,
            BlockProperties {
                name: String::from("Water"),
                solid: false,
                transparent: true,
                light_emission: 0,
                hardness: 100.0, // Can't break water
                texture_indices: [6; 6],
            },
        );

        registry
    }

    /// Load block definitions from a RON file.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed.
    pub fn load(path: &Path) -> Result<Self, BlockRegistryError> {
        let contents = fs::read_to_string(path)?;
        let blocks: HashMap<u16, BlockProperties> = ron::from_str(&contents)?;

        let registry = Self {
            blocks: blocks.into_iter().map(|(k, v)| (BlockId(k), v)).collect(),
        };

        Ok(registry)
    }

    /// Register a block type.
    pub fn register(&mut self, id: BlockId, properties: BlockProperties) {
        self.blocks.insert(id, properties);
    }

    /// Get properties for a block ID.
    #[must_use]
    pub fn get(&self, id: BlockId) -> Option<&BlockProperties> {
        self.blocks.get(&id)
    }

    /// Check if a block is solid.
    #[must_use]
    pub fn is_solid(&self, id: BlockId) -> bool {
        self.get(id).map_or(false, |p| p.solid)
    }

    /// Check if a block is transparent.
    #[must_use]
    pub fn is_transparent(&self, id: BlockId) -> bool {
        self.get(id).map_or(true, |p| p.transparent)
    }

    /// Check if a block emits light.
    #[must_use]
    pub fn light_emission(&self, id: BlockId) -> u8 {
        self.get(id).map_or(0, |p| p.light_emission)
    }

    /// Get the number of registered block types.
    #[must_use]
    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    /// Check if the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }
}

impl Default for BlockRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_air_not_solid() {
        let registry = BlockRegistry::with_defaults();
        assert!(!registry.is_solid(AIR));
        assert!(registry.is_transparent(AIR));
    }

    #[test]
    fn test_stone_solid_not_transparent() {
        let registry = BlockRegistry::with_defaults();
        assert!(registry.is_solid(STONE));
        assert!(!registry.is_transparent(STONE));
    }

    #[test]
    fn test_water_not_solid_transparent() {
        let registry = BlockRegistry::with_defaults();
        assert!(!registry.is_solid(WATER));
        assert!(registry.is_transparent(WATER));
    }

    #[test]
    fn test_load_from_ron() {
        let ron_content = r#"{
            0: BlockProperties(
                name: "Air",
                solid: false,
                transparent: true,
                light_emission: 0,
                hardness: 0.0,
                texture_indices: (0, 0, 0, 0, 0, 0),
            ),
            1: BlockProperties(
                name: "Stone",
                solid: true,
                transparent: false,
                light_emission: 0,
                hardness: 1.5,
                texture_indices: (1, 1, 1, 1, 1, 1),
            ),
        }"#;

        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(ron_content.as_bytes()).unwrap();

        let registry = BlockRegistry::load(temp.path()).unwrap();
        assert_eq!(registry.len(), 2);
        assert!(!registry.is_solid(AIR));
        assert!(registry.is_solid(STONE));
    }

    #[test]
    fn test_default_registry_has_blocks() {
        let registry = BlockRegistry::with_defaults();
        assert_eq!(registry.len(), 6); // Air, Stone, Dirt, Grass, Sand, Water
    }

    #[test]
    fn test_block_id_raw() {
        assert_eq!(STONE.raw(), 1);
        assert_eq!(WATER.raw(), 5);
    }
}
