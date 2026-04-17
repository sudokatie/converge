//! Death consequences and respawn mechanics.
//!
//! Implements spec 6.5.1: on death, drop inventory at death location,
//! respawn at spawn point with full health and hunger.

use engine_core::coords::WorldPos;
use engine_world::chunk::BlockId;

use crate::inventory::{Inventory, ItemStack};

/// Result of a player death event.
#[derive(Debug, Clone)]
pub struct DeathResult {
    /// Position where the player died (items drop here).
    pub death_position: WorldPos,
    /// Items dropped from inventory.
    pub dropped_items: Vec<ItemStack>,
    /// Damage source that caused the death.
    pub cause: DeathCause,
}

/// Cause of player death.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeathCause {
    /// Fell from a height.
    Fall,
    /// Attacked by an entity.
    Combat,
    /// Drowned underwater.
    Drowning,
    /// Starvation (hunger depleted).
    Starvation,
    /// Burned by fire or lava.
    Fire,
    /// Unknown or generic damage.
    Other,
}

impl DeathCause {
    /// Get a human-readable death message.
    #[must_use]
    pub fn message(&self) -> &'static str {
        match self {
            DeathCause::Fall => "fell to their death",
            DeathCause::Combat => "was slain",
            DeathCause::Drowning => "drowned",
            DeathCause::Starvation => "starved to death",
            DeathCause::Fire => "burned to death",
            DeathCause::Other => "died",
        }
    }
}

/// Manages death state and consequences.
#[derive(Debug, Clone)]
pub struct DeathHandler {
    /// Whether the player is currently dead.
    is_dead: bool,
    /// Time since death in seconds (for death screen timer).
    time_since_death: f32,
    /// Position of last death.
    last_death_pos: Option<WorldPos>,
    /// Cause of last death.
    last_death_cause: Option<DeathCause>,
    /// Whether to keep inventory on death (creative mode, etc.).
    keep_inventory: bool,
}

impl Default for DeathHandler {
    fn default() -> Self {
        Self {
            is_dead: false,
            time_since_death: 0.0,
            last_death_pos: None,
            last_death_cause: None,
            keep_inventory: false,
        }
    }
}

impl DeathHandler {
    /// Create a new death handler.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Handle player death.
    ///
    /// Drops all inventory items (unless keep_inventory is set),
    /// records death position and cause.
    pub fn handle_death(
        &mut self,
        position: WorldPos,
        cause: DeathCause,
        inventory: &mut Inventory,
    ) -> DeathResult {
        self.is_dead = true;
        self.time_since_death = 0.0;
        self.last_death_pos = Some(position);
        self.last_death_cause = Some(cause);

        let dropped_items = if self.keep_inventory {
            Vec::new()
        } else {
            Self::drop_inventory(inventory)
        };

        DeathResult {
            death_position: position,
            dropped_items,
            cause,
        }
    }

    /// Drop all inventory items and clear the inventory.
    fn drop_inventory(inventory: &mut Inventory) -> Vec<ItemStack> {
        let mut dropped = Vec::new();

        // Drop all 36 slots
        for slot in 0..36 {
            if let Some(stack) = inventory.remove(slot, u32::MAX) {
                dropped.push(stack);
            }
        }

        dropped
    }

    /// Respawn the player at the given spawn point.
    ///
    /// Clears death state. The caller is responsible for
    /// resetting health and hunger to full.
    pub fn respawn(&mut self) -> Option<DeathCause> {
        let cause = self.last_death_cause;
        self.is_dead = false;
        self.time_since_death = 0.0;
        cause
    }

    /// Check if the player is currently dead.
    #[must_use]
    pub fn is_dead(&self) -> bool {
        self.is_dead
    }

    /// Get time since last death.
    #[must_use]
    pub fn time_since_death(&self) -> f32 {
        self.time_since_death
    }

    /// Get the position of the last death.
    #[must_use]
    pub fn last_death_pos(&self) -> Option<WorldPos> {
        self.last_death_pos
    }

    /// Get the cause of the last death.
    #[must_use]
    pub fn last_death_cause(&self) -> Option<DeathCause> {
        self.last_death_cause
    }

    /// Update death timer (call each frame while dead).
    pub fn tick(&mut self, dt: f32) {
        if self.is_dead {
            self.time_since_death += dt;
        }
    }

    /// Set keep inventory mode.
    pub fn set_keep_inventory(&mut self, keep: bool) {
        self.keep_inventory = keep;
    }

    /// Check if keep inventory is enabled.
    #[must_use]
    pub fn keeps_inventory(&self) -> bool {
        self.keep_inventory
    }
}

/// Dropped item entity in the world.
///
/// Items dropped on death float at the death position
/// and can be picked up by any player.
#[derive(Debug, Clone)]
pub struct DroppedItem {
    /// The item stack that was dropped.
    pub stack: ItemStack,
    /// World position where the item was dropped.
    pub position: WorldPos,
    /// Time the item has been on the ground (for despawn timer).
    pub age_secs: f32,
    /// Whether the item can be picked up yet (brief pickup delay).
    pub can_pickup: bool,
}

/// Despawn time for dropped items (5 minutes).
pub const ITEM_DESPAWN_TIME: f32 = 300.0;

/// Pickup delay after dropping (prevents instant re-pickup).
pub const PICKUP_DELAY_SECS: f32 = 0.5;

impl DroppedItem {
    /// Create a new dropped item.
    #[must_use]
    pub fn new(stack: ItemStack, position: WorldPos) -> Self {
        Self {
            stack,
            position,
            age_secs: 0.0,
            can_pickup: false,
        }
    }

    /// Update the dropped item each frame.
    pub fn tick(&mut self, dt: f32) {
        self.age_secs += dt;
        if self.age_secs >= PICKUP_DELAY_SECS {
            self.can_pickup = true;
        }
    }

    /// Check if this item should be despawned.
    #[must_use]
    pub fn should_despawn(&self) -> bool {
        self.age_secs >= ITEM_DESPAWN_TIME
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::ItemId;

    fn make_stack(id: u16, count: u32) -> ItemStack {
        ItemStack::new(ItemId(id), count)
    }

    fn add_to_inventory(inv: &mut Inventory, stack: ItemStack) {
        let leftover = inv.add(stack);
        assert!(leftover.is_none(), "inventory should have room");
    }

    #[test]
    fn test_death_drops_inventory() {
        let mut handler = DeathHandler::new();
        let mut inventory = Inventory::new();

        add_to_inventory(&mut inventory, make_stack(1, 64));
        add_to_inventory(&mut inventory, make_stack(2, 32));

        let result = handler.handle_death(
            WorldPos::new(10, 20, 30),
            DeathCause::Fall,
            &mut inventory,
        );

        assert!(handler.is_dead());
        assert_eq!(result.dropped_items.len(), 2);
        assert_eq!(result.death_position, WorldPos::new(10, 20, 30));
        assert_eq!(result.cause, DeathCause::Fall);
    }

    #[test]
    fn test_keep_inventory_no_drops() {
        let mut handler = DeathHandler::new();
        handler.set_keep_inventory(true);

        let mut inventory = Inventory::new();
        add_to_inventory(&mut inventory, make_stack(1, 64));

        let result = handler.handle_death(
            WorldPos::new(0, 0, 0),
            DeathCause::Combat,
            &mut inventory,
        );

        assert!(result.dropped_items.is_empty());
        assert!(handler.keeps_inventory());
    }

    #[test]
    fn test_respawn_clears_death() {
        let mut handler = DeathHandler::new();
        let mut inventory = Inventory::new();

        handler.handle_death(
            WorldPos::new(5, 10, 15),
            DeathCause::Drowning,
            &mut inventory,
        );

        assert!(handler.is_dead());

        let cause = handler.respawn();
        assert!(!handler.is_dead());
        assert_eq!(cause, Some(DeathCause::Drowning));
        assert_eq!(handler.time_since_death(), 0.0);
    }

    #[test]
    fn test_death_cause_messages() {
        assert_eq!(DeathCause::Fall.message(), "fell to their death");
        assert_eq!(DeathCause::Combat.message(), "was slain");
        assert_eq!(DeathCause::Drowning.message(), "drowned");
        assert_eq!(DeathCause::Starvation.message(), "starved to death");
        assert_eq!(DeathCause::Fire.message(), "burned to death");
        assert_eq!(DeathCause::Other.message(), "died");
    }

    #[test]
    fn test_death_timer() {
        let mut handler = DeathHandler::new();
        let mut inventory = Inventory::new();

        handler.handle_death(
            WorldPos::new(0, 0, 0),
            DeathCause::Other,
            &mut inventory,
        );

        handler.tick(1.0);
        assert!((handler.time_since_death() - 1.0).abs() < 0.001);

        handler.tick(0.5);
        assert!((handler.time_since_death() - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_timer_stops_on_respawn() {
        let mut handler = DeathHandler::new();
        let mut inventory = Inventory::new();

        handler.handle_death(
            WorldPos::new(0, 0, 0),
            DeathCause::Other,
            &mut inventory,
        );

        handler.tick(2.0);
        handler.respawn();
        handler.tick(1.0); // Should not increment after respawn

        assert_eq!(handler.time_since_death(), 0.0);
    }

    #[test]
    fn test_dropped_item_pickup_delay() {
        let item = DroppedItem::new(make_stack(1, 10), WorldPos::new(0, 0, 0));
        assert!(!item.can_pickup);
        assert!(!item.should_despawn());
    }

    #[test]
    fn test_dropped_item_becomes_pickupable() {
        let mut item = DroppedItem::new(make_stack(1, 10), WorldPos::new(0, 0, 0));
        item.tick(0.5);
        assert!(item.can_pickup);
    }

    #[test]
    fn test_dropped_item_despawn() {
        let mut item = DroppedItem::new(make_stack(1, 10), WorldPos::new(0, 0, 0));
        item.tick(300.0);
        assert!(item.should_despawn());
    }

    #[test]
    fn test_last_death_tracking() {
        let mut handler = DeathHandler::new();
        assert_eq!(handler.last_death_pos(), None);
        assert_eq!(handler.last_death_cause(), None);

        let mut inventory = Inventory::new();
        handler.handle_death(
            WorldPos::new(42, 64, 100),
            DeathCause::Fire,
            &mut inventory,
        );

        assert_eq!(handler.last_death_pos(), Some(WorldPos::new(42, 64, 100)));
        assert_eq!(handler.last_death_cause(), Some(DeathCause::Fire));
    }
}
