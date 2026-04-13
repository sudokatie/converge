//! Crafting execution - actually perform crafts by consuming inputs and producing outputs.

use thiserror::Error;

use crate::inventory::{Inventory, ItemId, ItemRegistry, ItemStack};

use super::{Recipe, RecipeRegistry};

/// Errors that can occur during crafting.
#[derive(Debug, Clone, Error)]
pub enum CraftError {
    /// Recipe not found in registry.
    #[error("Invalid recipe: '{0}'")]
    InvalidRecipe(String),

    /// Missing required input materials.
    #[error("Missing inputs: need {needed} of item {item_id:?}, have {have}")]
    MissingInputs {
        item_id: ItemId,
        needed: u32,
        have: u32,
    },

    /// Not enough space in inventory for output.
    #[error("Inventory full - cannot fit output")]
    OutputOverflow,
}

/// Result of a craft check (what's missing if any).
#[derive(Debug, Clone)]
pub struct CraftRequirements {
    /// Whether the craft can proceed.
    pub can_craft: bool,
    /// Missing items (item_id, needed, have).
    pub missing: Vec<(ItemId, u32, u32)>,
}

/// Check if a craft can be performed and what's missing.
#[must_use]
pub fn check_craft(recipe: &Recipe, inventory: &Inventory) -> CraftRequirements {
    let mut missing = Vec::new();
    let mut can_craft = true;

    for (item_id, needed) in &recipe.inputs {
        let have = inventory.count_item(*item_id);
        if have < *needed {
            can_craft = false;
            missing.push((*item_id, *needed, have));
        }
    }

    CraftRequirements { can_craft, missing }
}

/// Execute a craft by recipe reference.
///
/// This is an atomic operation - either all inputs are consumed and output produced,
/// or nothing happens.
///
/// # Errors
///
/// Returns an error if:
/// - Inputs are insufficient
/// - Output can't fit in inventory
pub fn execute_craft(
    recipe: &Recipe,
    inventory: &mut Inventory,
    items: &ItemRegistry,
) -> Result<(), CraftError> {
    // First, verify all inputs are available
    for (item_id, needed) in &recipe.inputs {
        let have = inventory.count_item(*item_id);
        if have < *needed {
            return Err(CraftError::MissingInputs {
                item_id: *item_id,
                needed: *needed,
                have,
            });
        }
    }

    // Check if output can fit (we need to simulate adding it)
    let (output_id, output_count) = recipe.output;
    let stack_size = items.stack_size(output_id);

    // Simple check: can we fit output_count items?
    // For accurate check, we'd need to simulate the full add
    if !can_fit_items(inventory, output_id, output_count, stack_size) {
        return Err(CraftError::OutputOverflow);
    }

    // Now actually perform the craft
    // Remove inputs
    for (item_id, needed) in &recipe.inputs {
        remove_items(inventory, *item_id, *needed);
    }

    // Add output
    let output_stack = ItemStack::new(output_id, output_count);
    inventory.add(output_stack);

    Ok(())
}

/// Execute a craft by recipe ID.
///
/// # Errors
///
/// Returns an error if recipe not found or craft fails.
pub fn execute_craft_by_id(
    recipe_id: &str,
    recipes: &RecipeRegistry,
    inventory: &mut Inventory,
    items: &ItemRegistry,
) -> Result<(), CraftError> {
    let recipe = recipes
        .get(recipe_id)
        .ok_or_else(|| CraftError::InvalidRecipe(recipe_id.to_owned()))?;

    execute_craft(recipe, inventory, items)
}

/// Check if items can fit in inventory.
fn can_fit_items(inventory: &Inventory, item_id: ItemId, count: u32, stack_size: u32) -> bool {
    // Count existing items and available space
    let mut remaining = count;

    // First check existing stacks of same item
    for slot in 0..crate::inventory::INVENTORY_SIZE {
        if let Some(stack) = inventory.get(slot) {
            if stack.item_id == item_id {
                let space = stack_size.saturating_sub(stack.count);
                remaining = remaining.saturating_sub(space);
                if remaining == 0 {
                    return true;
                }
            }
        }
    }

    // Count empty slots
    let empty_slots = crate::inventory::INVENTORY_SIZE - inventory.occupied_slots();
    let can_fit_in_empty = empty_slots as u32 * stack_size;

    remaining <= can_fit_in_empty
}

/// Remove items from inventory across multiple slots.
fn remove_items(inventory: &mut Inventory, item_id: ItemId, mut count: u32) {
    for slot in 0..crate::inventory::INVENTORY_SIZE {
        if count == 0 {
            break;
        }

        if let Some(stack) = inventory.get(slot) {
            if stack.item_id == item_id {
                let take = stack.count.min(count);
                inventory.remove(slot, take);
                count -= take;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_ITEMS: &str = r#"
[
    (id: 1, name: "Oak Log", stack_size: 64, category: Block),
    (id: 2, name: "Oak Planks", stack_size: 64, category: Block),
    (id: 3, name: "Stick", stack_size: 64, category: Material),
    (id: 4, name: "Wooden Pickaxe", stack_size: 1, category: Tool),
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
    ),
]
"#;

    fn setup() -> (ItemRegistry, RecipeRegistry) {
        let items = ItemRegistry::from_ron(TEST_ITEMS).unwrap();
        let recipes = RecipeRegistry::from_ron(TEST_RECIPES, &items).unwrap();
        (items, recipes)
    }

    #[test]
    fn test_check_craft_success() {
        let (items, recipes) = setup();
        let mut inventory = Inventory::new();
        
        inventory.add(ItemStack::new(items.by_name("Oak Log").unwrap(), 5));

        let recipe = recipes.get("oak_planks").unwrap();
        let req = check_craft(recipe, &inventory);

        assert!(req.can_craft);
        assert!(req.missing.is_empty());
    }

    #[test]
    fn test_check_craft_missing() {
        let (items, recipes) = setup();
        let inventory = Inventory::new(); // Empty

        let recipe = recipes.get("oak_planks").unwrap();
        let req = check_craft(recipe, &inventory);

        assert!(!req.can_craft);
        assert_eq!(req.missing.len(), 1);
        assert_eq!(req.missing[0].0, items.by_name("Oak Log").unwrap());
        assert_eq!(req.missing[0].1, 1); // Needed
        assert_eq!(req.missing[0].2, 0); // Have
    }

    #[test]
    fn test_execute_craft_success() {
        let (items, recipes) = setup();
        let mut inventory = Inventory::new();
        
        let log_id = items.by_name("Oak Log").unwrap();
        let planks_id = items.by_name("Oak Planks").unwrap();
        
        inventory.add(ItemStack::new(log_id, 5));

        let recipe = recipes.get("oak_planks").unwrap();
        let result = execute_craft(recipe, &mut inventory, &items);

        assert!(result.is_ok());
        assert_eq!(inventory.count_item(log_id), 4); // 5 - 1 = 4
        assert_eq!(inventory.count_item(planks_id), 4); // 0 + 4 = 4
    }

    #[test]
    fn test_execute_craft_missing_inputs() {
        let (items, recipes) = setup();
        let mut inventory = Inventory::new(); // Empty

        let recipe = recipes.get("oak_planks").unwrap();
        let result = execute_craft(recipe, &mut inventory, &items);

        assert!(matches!(result, Err(CraftError::MissingInputs { .. })));
    }

    #[test]
    fn test_execute_craft_multiple_inputs() {
        let (items, recipes) = setup();
        let mut inventory = Inventory::new();
        
        let planks_id = items.by_name("Oak Planks").unwrap();
        let stick_id = items.by_name("Stick").unwrap();
        let pickaxe_id = items.by_name("Wooden Pickaxe").unwrap();
        
        inventory.add(ItemStack::new(planks_id, 10));
        inventory.add(ItemStack::new(stick_id, 10));

        let recipe = recipes.get("wooden_pickaxe").unwrap();
        let result = execute_craft(recipe, &mut inventory, &items);

        assert!(result.is_ok());
        assert_eq!(inventory.count_item(planks_id), 7); // 10 - 3 = 7
        assert_eq!(inventory.count_item(stick_id), 8); // 10 - 2 = 8
        assert_eq!(inventory.count_item(pickaxe_id), 1);
    }

    #[test]
    fn test_execute_craft_by_id() {
        let (items, recipes) = setup();
        let mut inventory = Inventory::new();
        
        let log_id = items.by_name("Oak Log").unwrap();
        let planks_id = items.by_name("Oak Planks").unwrap();
        
        inventory.add(ItemStack::new(log_id, 5));

        let result = execute_craft_by_id("oak_planks", &recipes, &mut inventory, &items);

        assert!(result.is_ok());
        assert_eq!(inventory.count_item(planks_id), 4);
    }

    #[test]
    fn test_execute_craft_invalid_recipe() {
        let (items, recipes) = setup();
        let mut inventory = Inventory::new();

        let result = execute_craft_by_id("nonexistent", &recipes, &mut inventory, &items);

        assert!(matches!(result, Err(CraftError::InvalidRecipe(_))));
    }

    #[test]
    fn test_craft_atomic_on_failure() {
        let (items, recipes) = setup();
        let mut inventory = Inventory::new();
        
        // Add only planks, no sticks - pickaxe craft should fail
        let planks_id = items.by_name("Oak Planks").unwrap();
        inventory.add(ItemStack::new(planks_id, 10));

        let recipe = recipes.get("wooden_pickaxe").unwrap();
        let result = execute_craft(recipe, &mut inventory, &items);

        // Should fail
        assert!(result.is_err());
        // Planks should be unchanged
        assert_eq!(inventory.count_item(planks_id), 10);
    }

    #[test]
    fn test_craft_multiple_times() {
        let (items, recipes) = setup();
        let mut inventory = Inventory::new();
        
        let log_id = items.by_name("Oak Log").unwrap();
        let planks_id = items.by_name("Oak Planks").unwrap();
        
        inventory.add(ItemStack::new(log_id, 10));

        let recipe = recipes.get("oak_planks").unwrap();

        // Craft 3 times
        for _ in 0..3 {
            let result = execute_craft(recipe, &mut inventory, &items);
            assert!(result.is_ok());
        }

        assert_eq!(inventory.count_item(log_id), 7); // 10 - 3 = 7
        assert_eq!(inventory.count_item(planks_id), 12); // 4 * 3 = 12
    }
}
