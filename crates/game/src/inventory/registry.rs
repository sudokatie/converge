//! Item definitions and registry.
//!
//! Items are defined in data files (items.ron) and loaded at startup.
//! Each item has properties like stack size, category, and optional tool info.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use engine_world::chunk::BlockId;

use super::ItemId;

/// Item category for filtering and UI organization.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemCategory {
    /// Block items that can be placed.
    Block,
    /// Tools (pickaxe, axe, shovel, etc.).
    Tool,
    /// Weapons for combat.
    Weapon,
    /// Food items that restore hunger.
    Food,
    /// Materials used in crafting.
    Material,
    /// Miscellaneous items.
    Misc,
}

/// Tool type for mining and harvesting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolType {
    /// Mines stone and ores.
    Pickaxe,
    /// Chops wood.
    Axe,
    /// Digs dirt, sand, gravel.
    Shovel,
    /// Tills soil, harvests crops.
    Hoe,
    /// Melee weapon.
    Sword,
}

/// Definition of an item type loaded from data files.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemDef {
    /// Unique item ID.
    pub id: u16,
    /// Display name.
    pub name: String,
    /// Maximum stack size (1 for tools/weapons, 64 for most items).
    #[serde(default = "default_stack_size")]
    pub stack_size: u32,
    /// Item category.
    pub category: ItemCategory,
    /// Tool type (if this is a tool).
    #[serde(default)]
    pub tool_type: Option<ToolType>,
    /// Maximum durability (if applicable).
    #[serde(default)]
    pub durability: Option<u32>,
    /// Block ID if this item places a block.
    #[serde(default)]
    pub block_id: Option<u16>,
    /// Damage dealt (for weapons/tools).
    #[serde(default)]
    pub damage: f32,
    /// Mining speed multiplier (for tools).
    #[serde(default = "default_mining_speed")]
    pub mining_speed: f32,
    /// Food restoration value (for food items).
    #[serde(default)]
    pub food_value: f32,
    /// Saturation restoration value (for food items).
    #[serde(default)]
    pub saturation_value: f32,
}

fn default_stack_size() -> u32 {
    64
}

fn default_mining_speed() -> f32 {
    1.0
}

/// Registry of all item definitions.
pub struct ItemRegistry {
    /// Items indexed by ID.
    items: HashMap<ItemId, ItemDef>,
    /// Items indexed by name (lowercase).
    by_name: HashMap<String, ItemId>,
}

impl ItemRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
            by_name: HashMap::new(),
        }
    }

    /// Load item definitions from a RON file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read items file: {}", path.display()))?;

        let defs: Vec<ItemDef> = ron::from_str(&content)
            .with_context(|| format!("Failed to parse items file: {}", path.display()))?;

        let mut registry = Self::new();

        for def in defs {
            let id = ItemId(def.id);
            let name = def.name.to_lowercase();

            registry.items.insert(id, def);
            registry.by_name.insert(name, id);
        }

        tracing::info!("Loaded {} item definitions", registry.items.len());

        Ok(registry)
    }

    /// Load item definitions from RON string (for testing).
    ///
    /// # Errors
    ///
    /// Returns an error if parsing fails.
    pub fn from_ron(content: &str) -> Result<Self> {
        let defs: Vec<ItemDef> = ron::from_str(content)
            .context("Failed to parse items RON")?;

        let mut registry = Self::new();

        for def in defs {
            let id = ItemId(def.id);
            let name = def.name.to_lowercase();

            registry.items.insert(id, def);
            registry.by_name.insert(name, id);
        }

        Ok(registry)
    }

    /// Get an item definition by ID.
    #[must_use]
    pub fn get(&self, id: ItemId) -> Option<&ItemDef> {
        self.items.get(&id)
    }

    /// Get an item ID by name (case-insensitive).
    #[must_use]
    pub fn by_name(&self, name: &str) -> Option<ItemId> {
        self.by_name.get(&name.to_lowercase()).copied()
    }

    /// Check if an item exists.
    #[must_use]
    pub fn contains(&self, id: ItemId) -> bool {
        self.items.contains_key(&id)
    }

    /// Get the stack size for an item.
    ///
    /// Returns 64 for unknown items.
    #[must_use]
    pub fn stack_size(&self, id: ItemId) -> u32 {
        self.items.get(&id).map_or(64, |def| def.stack_size)
    }

    /// Check if an item is a tool.
    #[must_use]
    pub fn is_tool(&self, id: ItemId) -> bool {
        self.items
            .get(&id)
            .map_or(false, |def| def.tool_type.is_some())
    }

    /// Get the tool type for an item.
    #[must_use]
    pub fn tool_type(&self, id: ItemId) -> Option<ToolType> {
        self.items.get(&id).and_then(|def| def.tool_type)
    }

    /// Get the block ID for a block item.
    #[must_use]
    pub fn block_id(&self, id: ItemId) -> Option<BlockId> {
        self.items
            .get(&id)
            .and_then(|def| def.block_id.map(BlockId))
    }

    /// Iterate over all items.
    pub fn iter(&self) -> impl Iterator<Item = (&ItemId, &ItemDef)> {
        self.items.iter()
    }

    /// Get items by category.
    pub fn by_category(&self, category: ItemCategory) -> impl Iterator<Item = (&ItemId, &ItemDef)> {
        self.items
            .iter()
            .filter(move |(_, def)| def.category == category)
    }

    /// Number of registered items.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Default for ItemRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_ITEMS: &str = r#"
[
    (
        id: 1,
        name: "Stone",
        stack_size: 64,
        category: Block,
        block_id: Some(1),
    ),
    (
        id: 2,
        name: "Dirt",
        stack_size: 64,
        category: Block,
        block_id: Some(2),
    ),
    (
        id: 100,
        name: "Wooden Pickaxe",
        stack_size: 1,
        category: Tool,
        tool_type: Some(Pickaxe),
        durability: Some(60),
        mining_speed: 2.0,
    ),
    (
        id: 101,
        name: "Stone Pickaxe",
        stack_size: 1,
        category: Tool,
        tool_type: Some(Pickaxe),
        durability: Some(132),
        mining_speed: 4.0,
    ),
    (
        id: 200,
        name: "Apple",
        stack_size: 64,
        category: Food,
        food_value: 4.0,
        saturation_value: 2.4,
    ),
]
"#;

    #[test]
    fn test_load_from_ron() {
        let registry = ItemRegistry::from_ron(TEST_ITEMS).unwrap();
        assert_eq!(registry.len(), 5);
    }

    #[test]
    fn test_get_by_id() {
        let registry = ItemRegistry::from_ron(TEST_ITEMS).unwrap();

        let stone = registry.get(ItemId(1)).unwrap();
        assert_eq!(stone.name, "Stone");
        assert_eq!(stone.category, ItemCategory::Block);
        assert_eq!(stone.block_id, Some(1));
    }

    #[test]
    fn test_get_by_name() {
        let registry = ItemRegistry::from_ron(TEST_ITEMS).unwrap();

        let id = registry.by_name("Stone").unwrap();
        assert_eq!(id, ItemId(1));

        // Case insensitive
        let id = registry.by_name("WOODEN PICKAXE").unwrap();
        assert_eq!(id, ItemId(100));
    }

    #[test]
    fn test_missing_item() {
        let registry = ItemRegistry::from_ron(TEST_ITEMS).unwrap();

        assert!(registry.get(ItemId(999)).is_none());
        assert!(registry.by_name("Nonexistent").is_none());
    }

    #[test]
    fn test_stack_size() {
        let registry = ItemRegistry::from_ron(TEST_ITEMS).unwrap();

        assert_eq!(registry.stack_size(ItemId(1)), 64);
        assert_eq!(registry.stack_size(ItemId(100)), 1);
        // Unknown item returns default
        assert_eq!(registry.stack_size(ItemId(999)), 64);
    }

    #[test]
    fn test_tool_type() {
        let registry = ItemRegistry::from_ron(TEST_ITEMS).unwrap();

        assert!(registry.is_tool(ItemId(100)));
        assert_eq!(registry.tool_type(ItemId(100)), Some(ToolType::Pickaxe));

        assert!(!registry.is_tool(ItemId(1)));
        assert_eq!(registry.tool_type(ItemId(1)), None);
    }

    #[test]
    fn test_block_id() {
        let registry = ItemRegistry::from_ron(TEST_ITEMS).unwrap();

        assert_eq!(registry.block_id(ItemId(1)), Some(BlockId(1)));
        assert_eq!(registry.block_id(ItemId(100)), None);
    }

    #[test]
    fn test_by_category() {
        let registry = ItemRegistry::from_ron(TEST_ITEMS).unwrap();

        let blocks: Vec<_> = registry.by_category(ItemCategory::Block).collect();
        assert_eq!(blocks.len(), 2);

        let tools: Vec<_> = registry.by_category(ItemCategory::Tool).collect();
        assert_eq!(tools.len(), 2);

        let food: Vec<_> = registry.by_category(ItemCategory::Food).collect();
        assert_eq!(food.len(), 1);
    }
}
