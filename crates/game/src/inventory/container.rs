//! Inventory container for storing items.

use std::collections::HashMap;

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

/// Custom item data for specialized items.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ItemData {
    /// Book with pages of text.
    Book {
        /// Title of the book.
        title: String,
        /// Author of the book.
        author: String,
        /// Pages of content.
        pages: Vec<String>,
    },
    /// Potion with effect data.
    Potion {
        /// Effect type identifier.
        effect: String,
        /// Effect duration in seconds.
        duration: f32,
        /// Effect amplifier/level.
        amplifier: u8,
    },
    /// Enchanted item data.
    Enchantments {
        /// Map of enchantment ID to level.
        enchants: HashMap<String, u8>,
    },
    /// Map with explored data.
    Map {
        /// Map scale (0 = 1:1, 1 = 1:2, etc.).
        scale: u8,
        /// Center X coordinate.
        center_x: i32,
        /// Center Z coordinate.
        center_z: i32,
    },
    /// Custom NBT-like data for modding.
    Custom {
        /// Arbitrary key-value data.
        data: HashMap<String, String>,
    },
}

/// A stack of items.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ItemStack {
    /// Item type.
    pub item_id: ItemId,
    /// Number of items in the stack.
    pub count: u32,
    /// Current durability (for tools/weapons/armor).
    #[serde(default)]
    pub durability: Option<u32>,
    /// Custom item data.
    #[serde(default)]
    pub data: Option<ItemData>,
}

impl ItemStack {
    /// Create a new item stack.
    #[must_use]
    pub fn new(item_id: ItemId, count: u32) -> Self {
        Self {
            item_id,
            count,
            durability: None,
            data: None,
        }
    }

    /// Create a single item stack.
    #[must_use]
    pub fn single(item_id: ItemId) -> Self {
        Self::new(item_id, 1)
    }

    /// Create an item stack with durability.
    #[must_use]
    pub fn with_durability(item_id: ItemId, count: u32, durability: u32) -> Self {
        Self {
            item_id,
            count,
            durability: Some(durability),
            data: None,
        }
    }

    /// Create an item stack with custom data.
    #[must_use]
    pub fn with_data(item_id: ItemId, count: u32, data: ItemData) -> Self {
        Self {
            item_id,
            count,
            durability: None,
            data: Some(data),
        }
    }

    /// Set durability on this stack.
    pub fn set_durability(&mut self, durability: u32) {
        self.durability = Some(durability);
    }

    /// Get current durability.
    #[must_use]
    pub fn get_durability(&self) -> Option<u32> {
        self.durability
    }

    /// Reduce durability by amount, returns true if item broke (durability reached 0).
    pub fn damage(&mut self, amount: u32) -> bool {
        if let Some(ref mut dur) = self.durability {
            *dur = dur.saturating_sub(amount);
            *dur == 0
        } else {
            false
        }
    }

    /// Set custom data on this stack.
    pub fn set_data(&mut self, data: ItemData) {
        self.data = Some(data);
    }

    /// Get custom data.
    #[must_use]
    pub fn get_data(&self) -> Option<&ItemData> {
        self.data.as_ref()
    }

    /// Check if this stack can merge with another.
    ///
    /// Stacks can only merge if they have the same item ID and neither has
    /// durability or custom data (since those make items unique).
    #[must_use]
    pub fn can_merge(&self, other: &ItemStack) -> bool {
        self.item_id == other.item_id
            && self.count < MAX_STACK_SIZE
            && self.durability.is_none()
            && other.durability.is_none()
            && self.data.is_none()
            && other.data.is_none()
    }

    /// Try to merge another stack into this one.
    ///
    /// Returns the remainder that couldn't be merged (if any).
    /// Items with durability or custom data cannot be merged.
    pub fn merge(&mut self, other: ItemStack) -> Option<ItemStack> {
        if !self.can_merge(&other) {
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
    /// Note: Split stacks from durable/data items share the same durability/data.
    pub fn split(&mut self, count: u32) -> Option<ItemStack> {
        if count > self.count {
            return None;
        }

        self.count -= count;
        Some(ItemStack {
            item_id: self.item_id,
            count,
            durability: self.durability,
            data: self.data.clone(),
        })
    }

    /// Check if the stack is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Check if this item has durability.
    #[must_use]
    pub fn has_durability(&self) -> bool {
        self.durability.is_some()
    }

    /// Check if this item has custom data.
    #[must_use]
    pub fn has_data(&self) -> bool {
        self.data.is_some()
    }
}

/// Errors that can occur during inventory operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InventoryError {
    /// Slot index is out of bounds.
    SlotOutOfBounds { slot: usize, max: usize },
    /// Not enough items in the slot.
    InsufficientItems { have: u32, requested: u32 },
    /// Slot is empty.
    EmptySlot { slot: usize },
    /// Item count is zero.
    ZeroCount,
    /// Invalid item ID.
    InvalidItem { item_id: ItemId },
    /// Inventory is full.
    InventoryFull,
}

impl std::fmt::Display for InventoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SlotOutOfBounds { slot, max } => {
                write!(f, "Slot {slot} is out of bounds (max {max})")
            }
            Self::InsufficientItems { have, requested } => {
                write!(f, "Insufficient items: have {have}, requested {requested}")
            }
            Self::EmptySlot { slot } => write!(f, "Slot {slot} is empty"),
            Self::ZeroCount => write!(f, "Cannot add zero items"),
            Self::InvalidItem { item_id } => write!(f, "Invalid item ID: {:?}", item_id),
            Self::InventoryFull => write!(f, "Inventory is full"),
        }
    }
}

impl std::error::Error for InventoryError {}

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
