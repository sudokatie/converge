//! Resource verification for anti-cheat
//!
//! Validates inventory operations and crafting to detect item duplication and invalid items.

use std::collections::{HashMap, HashSet};

use super::ValidationResult;

/// Item stack representation
#[derive(Debug, Clone, PartialEq)]
pub struct ItemStack {
    pub item_id: u32,
    pub count: u32,
    pub max_stack: u32,
    pub metadata: Option<Vec<u8>>,
}

impl ItemStack {
    pub fn new(item_id: u32, count: u32, max_stack: u32) -> Self {
        Self {
            item_id,
            count,
            max_stack,
            metadata: None,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.count > 0 && self.count <= self.max_stack
    }
}

/// Crafting recipe for verification
#[derive(Debug, Clone)]
pub struct Recipe {
    pub id: u32,
    pub inputs: Vec<(u32, u32)>,  // (item_id, count)
    pub output: ItemStack,
}

/// Inventory snapshot for verification
#[derive(Debug, Default)]
pub struct InventorySnapshot {
    items: HashMap<u32, u32>,  // item_id -> total count
}

impl InventorySnapshot {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_item(&mut self, item_id: u32, count: u32) {
        *self.items.entry(item_id).or_insert(0) += count;
    }

    pub fn remove_item(&mut self, item_id: u32, count: u32) -> bool {
        if let Some(current) = self.items.get_mut(&item_id) {
            if *current >= count {
                *current -= count;
                if *current == 0 {
                    self.items.remove(&item_id);
                }
                return true;
            }
        }
        false
    }

    pub fn get_count(&self, item_id: u32) -> u32 {
        self.items.get(&item_id).copied().unwrap_or(0)
    }

    pub fn total_items(&self) -> u32 {
        self.items.values().sum()
    }
}

/// Resource verifier for inventory and crafting validation
pub struct ResourceVerifier {
    /// Valid item IDs
    valid_items: HashSet<u32>,
    /// Item stack limits
    stack_limits: HashMap<u32, u32>,
    /// Known recipes
    recipes: HashMap<u32, Recipe>,
    /// Player inventory snapshots
    player_inventories: HashMap<u64, InventorySnapshot>,
    /// Items that cannot be obtained in survival
    creative_only_items: HashSet<u32>,
}

impl ResourceVerifier {
    /// Create a new resource verifier
    pub fn new() -> Self {
        Self {
            valid_items: HashSet::new(),
            stack_limits: HashMap::new(),
            recipes: HashMap::new(),
            player_inventories: HashMap::new(),
            creative_only_items: HashSet::new(),
        }
    }

    /// Register a valid item
    pub fn register_item(&mut self, item_id: u32, max_stack: u32) {
        self.valid_items.insert(item_id);
        self.stack_limits.insert(item_id, max_stack);
    }

    /// Register a creative-only item
    pub fn register_creative_only(&mut self, item_id: u32) {
        self.creative_only_items.insert(item_id);
    }

    /// Register a crafting recipe
    pub fn register_recipe(&mut self, recipe: Recipe) {
        self.recipes.insert(recipe.id, recipe);
    }

    /// Register a player
    pub fn register_player(&mut self, player_id: u64) {
        self.player_inventories.insert(player_id, InventorySnapshot::new());
    }

    /// Remove a player
    pub fn remove_player(&mut self, player_id: u64) {
        self.player_inventories.remove(&player_id);
    }

    /// Validate an item stack
    pub fn validate_item(&self, item: &ItemStack) -> ValidationResult {
        // Check if item ID is valid
        if !self.valid_items.contains(&item.item_id) {
            return ValidationResult::Invalid {
                reason: format!("Invalid item ID: {}", item.item_id),
            };
        }

        // Check stack size
        let max_stack = self.stack_limits.get(&item.item_id).copied().unwrap_or(64);
        if item.count > max_stack {
            return ValidationResult::Invalid {
                reason: format!(
                    "Invalid stack size: {} (max: {})",
                    item.count, max_stack
                ),
            };
        }

        if item.count == 0 {
            return ValidationResult::Invalid {
                reason: "Empty item stack".to_string(),
            };
        }

        ValidationResult::Valid
    }

    /// Validate a crafting operation
    pub fn validate_crafting(
        &mut self,
        player_id: u64,
        recipe_id: u32,
        _is_creative: bool,
    ) -> ValidationResult {
        let recipe = match self.recipes.get(&recipe_id) {
            Some(r) => r.clone(),
            None => {
                return ValidationResult::Invalid {
                    reason: format!("Unknown recipe: {}", recipe_id),
                };
            }
        };

        let inventory = match self.player_inventories.get(&player_id) {
            Some(inv) => inv,
            None => {
                return ValidationResult::Suspicious {
                    reason: "Player inventory not tracked".to_string(),
                };
            }
        };

        // Check if player has required ingredients
        for (item_id, required_count) in &recipe.inputs {
            let available = inventory.get_count(*item_id);
            if available < *required_count {
                return ValidationResult::Invalid {
                    reason: format!(
                        "Missing ingredient: item {} (have {}, need {})",
                        item_id, available, required_count
                    ),
                };
            }
        }

        ValidationResult::Valid
    }

    /// Validate an inventory transaction
    pub fn validate_transaction(
        &self,
        player_id: u64,
        removed: &[(u32, u32)],
        added: &[(u32, u32)],
        is_creative: bool,
    ) -> ValidationResult {
        // In creative mode, allow anything
        if is_creative {
            // But still check for invalid items
            for (item_id, _) in added {
                if !self.valid_items.contains(item_id) {
                    return ValidationResult::Invalid {
                        reason: format!("Invalid item ID: {}", item_id),
                    };
                }
            }
            return ValidationResult::Valid;
        }

        let inventory = match self.player_inventories.get(&player_id) {
            Some(inv) => inv,
            None => {
                return ValidationResult::Suspicious {
                    reason: "Player inventory not tracked".to_string(),
                };
            }
        };

        // Check creative-only items
        for (item_id, _) in added {
            if self.creative_only_items.contains(item_id) {
                return ValidationResult::Kick {
                    reason: format!("Spawned creative-only item: {}", item_id),
                };
            }
        }

        // Check if removed items exist in inventory
        for (item_id, count) in removed {
            let available = inventory.get_count(*item_id);
            if available < *count {
                return ValidationResult::Invalid {
                    reason: format!(
                        "Cannot remove {} of item {} (only have {})",
                        count, item_id, available
                    ),
                };
            }
        }

        // Calculate net change to detect duplication
        let mut net_change: HashMap<u32, i64> = HashMap::new();
        for (item_id, count) in removed {
            *net_change.entry(*item_id).or_insert(0) -= *count as i64;
        }
        for (item_id, count) in added {
            *net_change.entry(*item_id).or_insert(0) += *count as i64;
        }

        // In survival, net gain should be zero (or from valid sources like crafting)
        let total_gain: i64 = net_change.values().filter(|&&v| v > 0).sum();
        if total_gain > 0 {
            return ValidationResult::Suspicious {
                reason: format!("Net item gain of {} in transaction", total_gain),
            };
        }

        ValidationResult::Valid
    }

    /// Update player inventory after validated transaction
    pub fn apply_transaction(
        &mut self,
        player_id: u64,
        removed: &[(u32, u32)],
        added: &[(u32, u32)],
    ) {
        if let Some(inventory) = self.player_inventories.get_mut(&player_id) {
            for (item_id, count) in removed {
                inventory.remove_item(*item_id, *count);
            }
            for (item_id, count) in added {
                inventory.add_item(*item_id, *count);
            }
        }
    }

    /// Get player inventory snapshot
    pub fn get_inventory(&self, player_id: u64) -> Option<&InventorySnapshot> {
        self.player_inventories.get(&player_id)
    }

    /// Set player inventory (for initialization)
    pub fn set_inventory(&mut self, player_id: u64, items: &[(u32, u32)]) {
        let inventory = self.player_inventories
            .entry(player_id)
            .or_insert_with(InventorySnapshot::new);
        
        inventory.items.clear();
        for (item_id, count) in items {
            inventory.add_item(*item_id, *count);
        }
    }
}

impl Default for ResourceVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_verifier() -> ResourceVerifier {
        let mut verifier = ResourceVerifier::new();
        verifier.register_item(1, 64);  // Stone
        verifier.register_item(2, 64);  // Dirt
        verifier.register_item(3, 16);  // Ender pearl (stacks to 16)
        verifier.register_item(100, 1); // Tool (doesn't stack)
        verifier.register_creative_only(200); // Barrier block
        verifier
    }

    #[test]
    fn test_valid_item() {
        let verifier = setup_verifier();
        let item = ItemStack::new(1, 32, 64);
        assert!(verifier.validate_item(&item).is_valid());
    }

    #[test]
    fn test_invalid_item_id() {
        let verifier = setup_verifier();
        let item = ItemStack::new(999, 1, 64);
        assert!(!verifier.validate_item(&item).is_valid());
    }

    #[test]
    fn test_invalid_stack_size() {
        let verifier = setup_verifier();
        let item = ItemStack::new(3, 32, 64); // Ender pearl only stacks to 16
        // Note: verifier checks against registered limit, not item's max_stack
        let result = verifier.validate_item(&item);
        // This should fail because we registered item 3 with max_stack 16
        assert!(!result.is_valid());
    }

    #[test]
    fn test_transaction_validation() {
        let mut verifier = setup_verifier();
        verifier.register_player(1);
        verifier.set_inventory(1, &[(1, 64), (2, 32)]);

        // Valid transaction: move items around
        let result = verifier.validate_transaction(
            1,
            &[(1, 10)],
            &[(1, 10)],
            false,
        );
        assert!(result.is_valid());
    }

    #[test]
    fn test_creative_only_item_detection() {
        let mut verifier = setup_verifier();
        verifier.register_player(1);
        verifier.set_inventory(1, &[]);

        // Try to add creative-only item in survival
        let result = verifier.validate_transaction(
            1,
            &[],
            &[(200, 1)],
            false,
        );
        assert!(result.is_kick());
    }
}
