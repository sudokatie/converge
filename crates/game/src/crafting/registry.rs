//! Recipe definitions and registry.
//!
//! Recipes are defined in data files (recipes.ron) and loaded at startup.
//! The registry provides lookup by ID, output item, and available ingredients.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::inventory::{Inventory, ItemId, ItemRegistry};

/// Crafting station type (or hand crafting if None).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CraftingStation {
    /// Basic crafting table.
    CraftingTable,
    /// Furnace for smelting.
    Furnace,
    /// Anvil for repairing and combining.
    Anvil,
}

/// An ingredient in a recipe (item + count).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ingredient {
    /// Item name (resolved to ItemId at runtime).
    pub item: String,
    /// Required count.
    pub count: u32,
}

/// Output of a recipe.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecipeOutput {
    /// Item name (resolved to ItemId at runtime).
    pub item: String,
    /// Output count.
    pub count: u32,
}

/// A crafting recipe definition from data files.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecipeDef {
    /// Unique recipe ID (string for human readability).
    pub id: String,
    /// Required inputs.
    pub inputs: Vec<Ingredient>,
    /// Recipe output.
    pub output: RecipeOutput,
    /// Required crafting station (None = hand crafting).
    #[serde(default)]
    pub station: Option<CraftingStation>,
    /// Recipe category for UI filtering.
    #[serde(default)]
    pub category: Option<String>,
}

/// A resolved recipe with ItemIds.
#[derive(Clone, Debug)]
pub struct Recipe {
    /// Unique recipe ID.
    pub id: String,
    /// Required inputs (resolved to ItemIds).
    pub inputs: Vec<(ItemId, u32)>,
    /// Output item and count.
    pub output: (ItemId, u32),
    /// Required crafting station.
    pub station: Option<CraftingStation>,
    /// Recipe category.
    pub category: Option<String>,
}

/// Registry of all crafting recipes.
pub struct RecipeRegistry {
    /// Recipes indexed by ID.
    recipes: HashMap<String, Recipe>,
    /// Recipes indexed by output item.
    by_output: HashMap<ItemId, Vec<String>>,
    /// All recipe IDs for iteration.
    all_ids: Vec<String>,
}

impl RecipeRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            recipes: HashMap::new(),
            by_output: HashMap::new(),
            all_ids: Vec::new(),
        }
    }

    /// Load recipes from a RON file, resolving item names using the item registry.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, parsed, or item names are invalid.
    pub fn load(path: &Path, items: &ItemRegistry) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read recipes file: {}", path.display()))?;

        Self::from_ron(&content, items)
            .with_context(|| format!("Failed to parse recipes file: {}", path.display()))
    }

    /// Load recipes from a RON string, resolving item names.
    ///
    /// # Errors
    ///
    /// Returns an error if parsing fails or item names are invalid.
    pub fn from_ron(content: &str, items: &ItemRegistry) -> Result<Self> {
        let defs: Vec<RecipeDef> = ron::from_str(content)
            .context("Failed to parse recipes RON")?;

        let mut registry = Self::new();

        for def in defs {
            // Resolve input items
            let mut inputs = Vec::with_capacity(def.inputs.len());
            for ingredient in &def.inputs {
                let item_id = items
                    .by_name(&ingredient.item)
                    .with_context(|| {
                        format!("Unknown item '{}' in recipe '{}'", ingredient.item, def.id)
                    })?;
                inputs.push((item_id, ingredient.count));
            }

            // Resolve output item
            let output_id = items
                .by_name(&def.output.item)
                .with_context(|| {
                    format!("Unknown output item '{}' in recipe '{}'", def.output.item, def.id)
                })?;
            let output = (output_id, def.output.count);

            let recipe = Recipe {
                id: def.id.clone(),
                inputs,
                output,
                station: def.station,
                category: def.category,
            };

            // Index by output
            registry
                .by_output
                .entry(output_id)
                .or_default()
                .push(def.id.clone());

            registry.all_ids.push(def.id.clone());
            registry.recipes.insert(def.id, recipe);
        }

        tracing::info!("Loaded {} recipes", registry.recipes.len());

        Ok(registry)
    }

    /// Get a recipe by ID.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Recipe> {
        self.recipes.get(id)
    }

    /// Get all recipes that produce a specific item.
    pub fn by_output(&self, item: ItemId) -> impl Iterator<Item = &Recipe> {
        self.by_output
            .get(&item)
            .into_iter()
            .flatten()
            .filter_map(|id| self.recipes.get(id))
    }

    /// Get all recipes that can be crafted with current inventory contents.
    ///
    /// This is the main filtering function for the crafting UI.
    pub fn available<'a>(
        &'a self,
        inventory: &'a Inventory,
        station: Option<CraftingStation>,
    ) -> impl Iterator<Item = &'a Recipe> {
        self.recipes.values().filter(move |recipe| {
            // Check station requirement
            if recipe.station != station && recipe.station.is_some() {
                return false;
            }

            // Check all inputs are available
            recipe.inputs.iter().all(|(item_id, count)| {
                inventory.count_item(*item_id) >= *count
            })
        })
    }

    /// Get all recipes (for displaying in UI, even unavailable ones).
    pub fn all(&self) -> impl Iterator<Item = &Recipe> {
        self.recipes.values()
    }

    /// Get recipes that can be crafted at a specific station (or by hand).
    pub fn for_station(&self, station: Option<CraftingStation>) -> impl Iterator<Item = &Recipe> {
        self.recipes.values().filter(move |recipe| {
            // Hand recipes (station=None) are always available
            // Station recipes require matching station
            recipe.station.is_none() || recipe.station == station
        })
    }

    /// Check if a specific recipe can be crafted.
    #[must_use]
    pub fn can_craft(&self, id: &str, inventory: &Inventory) -> bool {
        self.get(id).map_or(false, |recipe| {
            recipe.inputs.iter().all(|(item_id, count)| {
                inventory.count_item(*item_id) >= *count
            })
        })
    }

    /// Number of registered recipes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.recipes.len()
    }

    /// Check if registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.recipes.is_empty()
    }
}

impl Default for RecipeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_ITEMS: &str = r#"
[
    (id: 1, name: "Oak Log", stack_size: 64, category: Block, block_id: Some(7)),
    (id: 2, name: "Oak Planks", stack_size: 64, category: Block, block_id: Some(8)),
    (id: 3, name: "Stick", stack_size: 64, category: Material),
    (id: 4, name: "Wooden Pickaxe", stack_size: 1, category: Tool, tool_type: Some(Pickaxe), durability: Some(60)),
    (id: 5, name: "Coal", stack_size: 64, category: Material),
    (id: 6, name: "Torch", stack_size: 64, category: Block, block_id: Some(20)),
]
"#;

    const TEST_RECIPES: &str = r#"
[
    (
        id: "oak_planks",
        inputs: [(item: "Oak Log", count: 1)],
        output: (item: "Oak Planks", count: 4),
    ),
    (
        id: "sticks",
        inputs: [(item: "Oak Planks", count: 2)],
        output: (item: "Stick", count: 4),
    ),
    (
        id: "wooden_pickaxe",
        inputs: [(item: "Oak Planks", count: 3), (item: "Stick", count: 2)],
        output: (item: "Wooden Pickaxe", count: 1),
        station: Some(CraftingTable),
    ),
    (
        id: "torch",
        inputs: [(item: "Coal", count: 1), (item: "Stick", count: 1)],
        output: (item: "Torch", count: 4),
    ),
]
"#;

    fn setup() -> (ItemRegistry, RecipeRegistry) {
        let items = ItemRegistry::from_ron(TEST_ITEMS).unwrap();
        let recipes = RecipeRegistry::from_ron(TEST_RECIPES, &items).unwrap();
        (items, recipes)
    }

    #[test]
    fn test_load_recipes() {
        let (_items, recipes) = setup();
        assert_eq!(recipes.len(), 4);
    }

    #[test]
    fn test_get_by_id() {
        let (items, recipes) = setup();
        
        let recipe = recipes.get("oak_planks").unwrap();
        assert_eq!(recipe.inputs.len(), 1);
        assert_eq!(recipe.inputs[0], (items.by_name("Oak Log").unwrap(), 1));
        assert_eq!(recipe.output.1, 4);
    }

    #[test]
    fn test_get_missing() {
        let (_items, recipes) = setup();
        assert!(recipes.get("nonexistent").is_none());
    }

    #[test]
    fn test_by_output() {
        let (items, recipes) = setup();

        let planks_id = items.by_name("Oak Planks").unwrap();
        let planks_recipes: Vec<_> = recipes.by_output(planks_id).collect();
        assert_eq!(planks_recipes.len(), 1);
        assert_eq!(planks_recipes[0].id, "oak_planks");
    }

    #[test]
    fn test_available_with_materials() {
        let (items, recipes) = setup();
        let mut inventory = Inventory::new();

        // Add enough materials for oak_planks
        inventory.add(crate::inventory::ItemStack::new(
            items.by_name("Oak Log").unwrap(),
            10,
        ));

        let available: Vec<_> = recipes.available(&inventory, None).collect();
        
        // Should be able to craft oak_planks (1 log -> 4 planks)
        assert!(available.iter().any(|r| r.id == "oak_planks"));
        
        // Should NOT be able to craft sticks (need planks)
        assert!(!available.iter().any(|r| r.id == "sticks"));
    }

    #[test]
    fn test_available_filters_station() {
        let (items, recipes) = setup();
        let mut inventory = Inventory::new();

        // Add enough for pickaxe
        inventory.add(crate::inventory::ItemStack::new(
            items.by_name("Oak Planks").unwrap(),
            10,
        ));
        inventory.add(crate::inventory::ItemStack::new(
            items.by_name("Stick").unwrap(),
            10,
        ));

        // Without crafting table, pickaxe shouldn't be available
        let hand_available: Vec<_> = recipes.available(&inventory, None).collect();
        assert!(!hand_available.iter().any(|r| r.id == "wooden_pickaxe"));

        // With crafting table, it should be
        let table_available: Vec<_> = recipes
            .available(&inventory, Some(CraftingStation::CraftingTable))
            .collect();
        assert!(table_available.iter().any(|r| r.id == "wooden_pickaxe"));
    }

    #[test]
    fn test_can_craft() {
        let (items, recipes) = setup();
        let mut inventory = Inventory::new();

        // Empty inventory can't craft
        assert!(!recipes.can_craft("oak_planks", &inventory));

        // Add logs
        inventory.add(crate::inventory::ItemStack::new(
            items.by_name("Oak Log").unwrap(),
            5,
        ));

        // Now can craft
        assert!(recipes.can_craft("oak_planks", &inventory));
    }

    #[test]
    fn test_for_station() {
        let (_items, recipes) = setup();

        // Hand craftable recipes
        let hand: Vec<_> = recipes.for_station(None).collect();
        assert!(hand.iter().any(|r| r.id == "oak_planks"));
        assert!(hand.iter().any(|r| r.id == "torch"));
        // Pickaxe requires table, but None returns it too (station-less always accessible)
        
        // Crafting table recipes (includes hand recipes too)
        let table: Vec<_> = recipes
            .for_station(Some(CraftingStation::CraftingTable))
            .collect();
        assert!(table.iter().any(|r| r.id == "wooden_pickaxe"));
        assert!(table.iter().any(|r| r.id == "oak_planks")); // Hand recipes available too
    }
}
