//! Durability system for tools and equipment.
//!
//! Items with durability degrade on use and break when exhausted.

use serde::{Deserialize, Serialize};

/// Durability component for items that wear out.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Durability {
    /// Current durability remaining.
    current: u32,
    /// Maximum durability.
    max: u32,
}

impl Durability {
    /// Create new durability at full.
    #[must_use]
    pub fn new(max: u32) -> Self {
        Self { current: max, max }
    }

    /// Create durability with specific current value.
    #[must_use]
    pub fn with_current(current: u32, max: u32) -> Self {
        Self {
            current: current.min(max),
            max,
        }
    }

    /// Get current durability.
    #[must_use]
    pub fn current(&self) -> u32 {
        self.current
    }

    /// Get maximum durability.
    #[must_use]
    pub fn max(&self) -> u32 {
        self.max
    }

    /// Get durability as a fraction (0.0 to 1.0).
    #[must_use]
    pub fn fraction(&self) -> f32 {
        if self.max == 0 {
            return 0.0;
        }
        self.current as f32 / self.max as f32
    }

    /// Check if item is broken (durability exhausted).
    #[must_use]
    pub fn is_broken(&self) -> bool {
        self.current == 0
    }

    /// Check if item is at full durability.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.current >= self.max
    }

    /// Check if item is damaged (not full but not broken).
    #[must_use]
    pub fn is_damaged(&self) -> bool {
        self.current > 0 && self.current < self.max
    }

    /// Decrease durability by amount.
    ///
    /// Returns true if the item broke from this damage.
    pub fn damage(&mut self, amount: u32) -> bool {
        let was_intact = self.current > 0;
        self.current = self.current.saturating_sub(amount);
        was_intact && self.current == 0
    }

    /// Decrease durability by 1.
    ///
    /// Returns true if the item broke.
    pub fn use_once(&mut self) -> bool {
        self.damage(1)
    }

    /// Repair durability by amount.
    ///
    /// Returns actual amount repaired.
    pub fn repair(&mut self, amount: u32) -> u32 {
        let old = self.current;
        self.current = (self.current + amount).min(self.max);
        self.current - old
    }

    /// Fully repair the item.
    pub fn repair_full(&mut self) {
        self.current = self.max;
    }

    /// Set max durability (for enchantments, etc.).
    pub fn set_max(&mut self, max: u32) {
        self.max = max;
        if self.current > self.max {
            self.current = self.max;
        }
    }
}

/// Standard durability values for tool tiers.
#[derive(Clone, Copy, Debug)]
pub struct ToolDurability;

impl ToolDurability {
    /// Wooden tool durability.
    pub const WOOD: u32 = 59;
    /// Stone tool durability.
    pub const STONE: u32 = 131;
    /// Iron tool durability.
    pub const IRON: u32 = 250;
    /// Gold tool durability (low but fast).
    pub const GOLD: u32 = 32;
    /// Diamond tool durability.
    pub const DIAMOND: u32 = 1561;

    /// Get durability for a tier level.
    #[must_use]
    pub fn for_tier(tier: u8) -> u32 {
        match tier {
            0 => 0,              // Hand (no durability)
            1 => Self::WOOD,
            2 => Self::STONE,
            3 => Self::IRON,
            4 => Self::DIAMOND,
            _ => Self::DIAMOND,
        }
    }
}

/// An item stack with optional durability tracking.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DurableItem {
    /// Item ID.
    pub item_id: u16,
    /// Durability (None for non-durable items).
    pub durability: Option<Durability>,
}

impl DurableItem {
    /// Create a non-durable item.
    #[must_use]
    pub fn simple(item_id: u16) -> Self {
        Self {
            item_id,
            durability: None,
        }
    }

    /// Create a durable item at full durability.
    #[must_use]
    pub fn durable(item_id: u16, max_durability: u32) -> Self {
        Self {
            item_id,
            durability: Some(Durability::new(max_durability)),
        }
    }

    /// Create a durable item with specific durability.
    #[must_use]
    pub fn durable_with(item_id: u16, current: u32, max: u32) -> Self {
        Self {
            item_id,
            durability: Some(Durability::with_current(current, max)),
        }
    }

    /// Check if this item has durability.
    #[must_use]
    pub fn has_durability(&self) -> bool {
        self.durability.is_some()
    }

    /// Check if item is broken.
    #[must_use]
    pub fn is_broken(&self) -> bool {
        self.durability.as_ref().map_or(false, Durability::is_broken)
    }

    /// Use the item once (decrease durability by 1).
    ///
    /// Returns true if the item broke.
    pub fn use_once(&mut self) -> bool {
        self.durability.as_mut().map_or(false, Durability::use_once)
    }

    /// Damage the item by amount.
    ///
    /// Returns true if the item broke.
    pub fn damage(&mut self, amount: u32) -> bool {
        self.durability.as_mut().map_or(false, |d| d.damage(amount))
    }

    /// Repair the item.
    ///
    /// Returns amount actually repaired.
    pub fn repair(&mut self, amount: u32) -> u32 {
        self.durability.as_mut().map_or(0, |d| d.repair(amount))
    }

    /// Get durability fraction (1.0 for non-durable items).
    #[must_use]
    pub fn durability_fraction(&self) -> f32 {
        self.durability.as_ref().map_or(1.0, Durability::fraction)
    }
}

/// Event emitted when a tool breaks.
#[derive(Clone, Debug)]
pub struct ToolBrokeEvent {
    /// The item ID that broke.
    pub item_id: u16,
    /// The item name (if known).
    pub name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_durability() {
        let dur = Durability::new(100);
        assert_eq!(dur.current(), 100);
        assert_eq!(dur.max(), 100);
        assert!(dur.is_full());
        assert!(!dur.is_broken());
    }

    #[test]
    fn test_durability_fraction() {
        let mut dur = Durability::new(100);
        assert!((dur.fraction() - 1.0).abs() < f32::EPSILON);

        dur.damage(50);
        assert!((dur.fraction() - 0.5).abs() < f32::EPSILON);

        dur.damage(50);
        assert!((dur.fraction() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_use_once() {
        let mut dur = Durability::new(3);

        assert!(!dur.use_once()); // 3 -> 2
        assert!(!dur.use_once()); // 2 -> 1
        assert!(dur.use_once());  // 1 -> 0, breaks!
        assert!(dur.is_broken());
    }

    #[test]
    fn test_damage_returns_broke() {
        let mut dur = Durability::new(10);

        assert!(!dur.damage(5)); // 10 -> 5, not broken
        assert!(!dur.damage(4)); // 5 -> 1, not broken
        assert!(dur.damage(5));  // 1 -> 0, broke! (overkill still breaks)
    }

    #[test]
    fn test_repair() {
        let mut dur = Durability::new(100);
        dur.damage(60);
        assert_eq!(dur.current(), 40);

        let repaired = dur.repair(30);
        assert_eq!(repaired, 30);
        assert_eq!(dur.current(), 70);

        // Repair caps at max
        let repaired = dur.repair(100);
        assert_eq!(repaired, 30);
        assert!(dur.is_full());
    }

    #[test]
    fn test_repair_full() {
        let mut dur = Durability::new(100);
        dur.damage(80);
        dur.repair_full();
        assert!(dur.is_full());
    }

    #[test]
    fn test_tool_durability_tiers() {
        assert_eq!(ToolDurability::for_tier(1), ToolDurability::WOOD);
        assert_eq!(ToolDurability::for_tier(2), ToolDurability::STONE);
        assert_eq!(ToolDurability::for_tier(3), ToolDurability::IRON);
        assert_eq!(ToolDurability::for_tier(4), ToolDurability::DIAMOND);
    }

    #[test]
    fn test_durable_item_simple() {
        let item = DurableItem::simple(1);
        assert!(!item.has_durability());
        assert!(!item.is_broken());
        assert_eq!(item.durability_fraction(), 1.0);
    }

    #[test]
    fn test_durable_item_with_durability() {
        let mut item = DurableItem::durable(100, 60);
        assert!(item.has_durability());
        assert!(!item.is_broken());

        // Use until broken
        for _ in 0..59 {
            assert!(!item.use_once());
        }
        assert!(item.use_once()); // 60th use breaks it
        assert!(item.is_broken());
    }

    #[test]
    fn test_durable_item_repair() {
        let mut item = DurableItem::durable(100, 100);
        item.damage(50);
        assert!((item.durability_fraction() - 0.5).abs() < 0.01);

        let repaired = item.repair(25);
        assert_eq!(repaired, 25);
        assert!((item.durability_fraction() - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_is_damaged() {
        let mut dur = Durability::new(100);
        assert!(!dur.is_damaged()); // Full, not damaged

        dur.damage(50);
        assert!(dur.is_damaged()); // Partial, damaged

        dur.damage(50);
        assert!(!dur.is_damaged()); // Broken, not "damaged"
    }

    #[test]
    fn test_set_max() {
        let mut dur = Durability::new(100);
        dur.damage(30);
        assert_eq!(dur.current(), 70);

        // Increase max
        dur.set_max(150);
        assert_eq!(dur.max(), 150);
        assert_eq!(dur.current(), 70); // Current unchanged

        // Decrease max below current
        dur.set_max(50);
        assert_eq!(dur.max(), 50);
        assert_eq!(dur.current(), 50); // Current clamped
    }
}
