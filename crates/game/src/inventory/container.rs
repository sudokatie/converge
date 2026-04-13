//! Inventory container for storing items.

use serde::{Deserialize, Serialize};

/// Total inventory size (36 slots).
pub const INVENTORY_SIZE: usize = 36;

/// Hotbar size (first 9 slots).
pub const HOTBAR_SIZE: usize = 9;

/// Maximum stack size for most items.
pub const MAX_STACK_SIZE: u32 = 64;

/// Unique item identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemId(pub u16);

impl ItemId {
    /// Get the raw ID value.
    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }
}

/// A stack of items.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStack {
    /// Item type.
    pub item_id: ItemId,
    /// Number of items in the stack.
    pub count: u32,
}

impl ItemStack {
    /// Create a new item stack.
    #[must_use]
    pub fn new(item_id: ItemId, count: u32) -> Self {
        Self { item_id, count }
    }

    /// Create a single item stack.
    #[must_use]
    pub fn single(item_id: ItemId) -> Self {
        Self::new(item_id, 1)
    }

    /// Check if this stack can merge with another.
    #[must_use]
    pub fn can_merge(&self, other: &ItemStack) -> bool {
        self.item_id == other.item_id && self.count < MAX_STACK_SIZE
    }

    /// Try to merge another stack into this one.
    ///
    /// Returns the remainder that couldn't be merged (if any).
    pub fn merge(&mut self, other: ItemStack) -> Option<ItemStack> {
        if self.item_id != other.item_id {
            return Some(other);
        }

        let space = MAX_STACK_SIZE.saturating_sub(self.count);
        let to_add = other.count.min(space);

        self.count += to_add;

        if to_add < other.count {
            Some(ItemStack::new(other.item_id, other.count - to_add))
        } else {
            None
        }
    }

    /// Split off a number of items from this stack.
    ///
    /// Returns the split-off stack, or None if not enough items.
    pub fn split(&mut self, count: u32) -> Option<ItemStack> {
        if count > self.count {
            return None;
        }

        self.count -= count;
        Some(ItemStack::new(self.item_id, count))
    }

    /// Check if the stack is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

/// Player inventory container.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Inventory {
    /// Inventory slots (36 total, 0-8 are hotbar).
    slots: Vec<Option<ItemStack>>,
    /// Currently selected hotbar slot (0-8).
    selected: usize,
}

impl Inventory {
    /// Create an empty inventory.
    #[must_use]
    pub fn new() -> Self {
        Self {
            slots: vec![None; INVENTORY_SIZE],
            selected: 0,
        }
    }

    /// Add an item stack to the inventory.
    ///
    /// First tries to merge with existing stacks, then uses empty slots.
    /// Returns any overflow that couldn't be added.
    pub fn add(&mut self, mut stack: ItemStack) -> Option<ItemStack> {
        // First, try to merge with existing stacks of the same item
        for slot in &mut self.slots {
            if let Some(existing) = slot {
                if existing.can_merge(&stack) {
                    stack = existing.merge(stack)?;
                }
            }
        }

        // If there's anything left, find an empty slot
        if !stack.is_empty() {
            for slot in &mut self.slots {
                if slot.is_none() {
                    *slot = Some(stack);
                    return None;
                }
            }
            // No room, return the remainder
            return Some(stack);
        }

        None
    }

    /// Remove items from a specific slot.
    ///
    /// Returns the removed items, or None if the slot is empty or doesn't have enough.
    pub fn remove(&mut self, slot: usize, count: u32) -> Option<ItemStack> {
        if slot >= INVENTORY_SIZE {
            return None;
        }

        let stack = self.slots[slot].as_mut()?;

        if count >= stack.count {
            // Remove entire stack
            self.slots[slot].take()
        } else {
            // Split the stack
            stack.split(count)
        }
    }

    /// Get the item stack in a slot.
    #[must_use]
    pub fn get(&self, slot: usize) -> Option<&ItemStack> {
        if slot >= INVENTORY_SIZE {
            return None;
        }
        self.slots[slot].as_ref()
    }

    /// Get mutable access to a slot.
    pub fn get_mut(&mut self, slot: usize) -> Option<&mut ItemStack> {
        if slot >= INVENTORY_SIZE {
            return None;
        }
        self.slots[slot].as_mut()
    }

    /// Get the currently selected hotbar slot index.
    #[must_use]
    pub fn selected_slot(&self) -> usize {
        self.selected
    }

    /// Set the selected hotbar slot.
    pub fn select_slot(&mut self, slot: usize) {
        if slot < HOTBAR_SIZE {
            self.selected = slot;
        }
    }

    /// Get the item in the selected hotbar slot.
    #[must_use]
    pub fn selected_item(&self) -> Option<&ItemStack> {
        self.get(self.selected)
    }

    /// Scroll hotbar selection.
    pub fn scroll(&mut self, delta: i32) {
        let new_slot = (self.selected as i32 + delta).rem_euclid(HOTBAR_SIZE as i32) as usize;
        self.selected = new_slot;
    }

    /// Swap two slots.
    pub fn swap(&mut self, a: usize, b: usize) {
        if a < INVENTORY_SIZE && b < INVENTORY_SIZE {
            self.slots.swap(a, b);
        }
    }

    /// Count total items of a specific type.
    #[must_use]
    pub fn count_item(&self, item_id: ItemId) -> u32 {
        self.slots
            .iter()
            .filter_map(|s| s.as_ref())
            .filter(|s| s.item_id == item_id)
            .map(|s| s.count)
            .sum()
    }

    /// Check if inventory is completely empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slots.iter().all(|s| s.is_none())
    }

    /// Get number of occupied slots.
    #[must_use]
    pub fn occupied_slots(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }
}

impl Default for Inventory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_to_empty_slot() {
        let mut inventory = Inventory::new();
        let stack = ItemStack::new(ItemId(1), 10);

        let overflow = inventory.add(stack);

        assert!(overflow.is_none(), "Should fit in empty inventory");
        assert_eq!(inventory.get(0).unwrap().count, 10);
    }

    #[test]
    fn test_stack_merging() {
        let mut inventory = Inventory::new();
        inventory.add(ItemStack::new(ItemId(1), 30));
        inventory.add(ItemStack::new(ItemId(1), 20));

        // Should merge into single stack
        assert_eq!(inventory.get(0).unwrap().count, 50);
        assert!(inventory.get(1).is_none());
    }

    #[test]
    fn test_stack_overflow() {
        let mut inventory = Inventory::new();
        inventory.add(ItemStack::new(ItemId(1), 60));
        inventory.add(ItemStack::new(ItemId(1), 20)); // 80 total, max 64

        // Should have 64 in first slot, 16 in second
        assert_eq!(inventory.get(0).unwrap().count, 64);
        assert_eq!(inventory.get(1).unwrap().count, 16);
    }

    #[test]
    fn test_remove_partial() {
        let mut inventory = Inventory::new();
        inventory.add(ItemStack::new(ItemId(1), 50));

        let removed = inventory.remove(0, 20);

        assert_eq!(removed.unwrap().count, 20);
        assert_eq!(inventory.get(0).unwrap().count, 30);
    }

    #[test]
    fn test_remove_all() {
        let mut inventory = Inventory::new();
        inventory.add(ItemStack::new(ItemId(1), 50));

        let removed = inventory.remove(0, 50);

        assert_eq!(removed.unwrap().count, 50);
        assert!(inventory.get(0).is_none());
    }

    #[test]
    fn test_hotbar_selection() {
        let mut inventory = Inventory::new();
        assert_eq!(inventory.selected_slot(), 0);

        inventory.select_slot(5);
        assert_eq!(inventory.selected_slot(), 5);

        inventory.select_slot(10); // Invalid, should be ignored
        assert_eq!(inventory.selected_slot(), 5);
    }

    #[test]
    fn test_scroll_wraps() {
        let mut inventory = Inventory::new();
        inventory.select_slot(0);

        inventory.scroll(-1);
        assert_eq!(inventory.selected_slot(), 8);

        inventory.scroll(2);
        assert_eq!(inventory.selected_slot(), 1);
    }

    #[test]
    fn test_count_item() {
        let mut inventory = Inventory::new();
        inventory.add(ItemStack::new(ItemId(1), 30));
        inventory.add(ItemStack::new(ItemId(2), 10));
        inventory.add(ItemStack::new(ItemId(1), 50)); // Will split due to 64 max

        assert_eq!(inventory.count_item(ItemId(1)), 80);
        assert_eq!(inventory.count_item(ItemId(2)), 10);
        assert_eq!(inventory.count_item(ItemId(99)), 0);
    }

    #[test]
    fn test_swap_slots() {
        let mut inventory = Inventory::new();
        inventory.add(ItemStack::new(ItemId(1), 10));
        inventory.add(ItemStack::new(ItemId(2), 20));

        inventory.swap(0, 1);

        assert_eq!(inventory.get(0).unwrap().item_id, ItemId(2));
        assert_eq!(inventory.get(1).unwrap().item_id, ItemId(1));
    }
}
