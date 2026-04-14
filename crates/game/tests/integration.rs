//! Integration tests for the Lattice game.
//!
//! Tests core functionality across multiple systems.

use lattice_game::crafting::{CraftingStation, Recipe};
use lattice_game::inventory::{Inventory, ItemId, ItemStack};
use lattice_game::survival::{DamageSource, Health, Hunger};

/// Test inventory operations.
mod inventory_tests {
    use super::*;

    #[test]
    fn test_inventory_add_items() {
        let mut inventory = Inventory::new();
        
        // Add wood planks
        let wood_planks = ItemStack::new(ItemId(10), 8);
        inventory.add(wood_planks);
        
        // Verify added
        let total = inventory.count_item(ItemId(10));
        assert_eq!(total, 8);
    }

    #[test]
    fn test_inventory_stack_splitting() {
        let mut inventory = Inventory::new();
        
        // Add a stack
        let stack = ItemStack::new(ItemId(1), 64);
        inventory.add(stack);
        
        // Remove partial
        let removed = inventory.remove(0, 32);
        assert!(removed.is_some());
        let removed = removed.unwrap();
        assert_eq!(removed.count, 32);
        
        // Verify remaining
        let remaining = inventory.get(0);
        assert!(remaining.is_some());
        assert_eq!(remaining.unwrap().count, 32);
    }

    #[test]
    fn test_inventory_full_behavior() {
        let mut inventory = Inventory::new();
        
        // Fill all slots with different items
        for i in 0..36 {
            let stack = ItemStack::new(ItemId(i as u16), 1);
            let overflow = inventory.add(stack);
            assert!(overflow.is_none(), "Should not overflow on slot {i}");
        }
        
        // Adding another different item should overflow
        let extra = ItemStack::new(ItemId(100), 1);
        let overflow = inventory.add(extra);
        assert!(overflow.is_some(), "Should overflow when full");
    }

    #[test]
    fn test_inventory_stacking_same_item() {
        let mut inventory = Inventory::new();
        
        // Add same item multiple times
        for _ in 0..4 {
            inventory.add(ItemStack::new(ItemId(1), 16));
        }
        
        // Should all be in one slot (assuming max stack 64)
        let count = inventory.count_item(ItemId(1));
        assert_eq!(count, 64);
    }

    #[test]
    fn test_inventory_selected_slot() {
        let mut inventory = Inventory::new();
        
        assert_eq!(inventory.selected_slot(), 0);
        inventory.select_slot(5);
        assert_eq!(inventory.selected_slot(), 5);
    }
}

/// Test survival systems.
mod survival_tests {
    use super::*;

    #[test]
    fn test_health_damage_heal_cycle() {
        let mut health = Health::new(20.0);
        
        // Take damage
        let died = health.damage(5.0, DamageSource::Attack);
        assert!(!died);
        assert!((health.current() - 15.0).abs() < 0.01);
        
        // Heal
        health.heal(3.0);
        assert!((health.current() - 18.0).abs() < 0.01);
        
        // Over-heal should cap at max
        health.heal(10.0);
        assert!((health.current() - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_health_lethal_damage() {
        let mut health = Health::new(20.0);
        
        // Take lethal damage
        let died = health.damage(25.0, DamageSource::Void);
        assert!(died);
        assert!(health.is_dead());
    }

    #[test]
    fn test_health_invincibility() {
        let mut health = Health::new(20.0);
        
        // Take damage (sets invincibility)
        health.damage(5.0, DamageSource::Attack);
        
        // Should be invincible
        assert!(health.is_invincible());
        
        // Tick to clear invincibility
        health.tick(1.0);
        assert!(!health.is_invincible());
    }

    #[test]
    fn test_health_restore() {
        let mut health = Health::new(20.0);
        health.damage(10.0, DamageSource::Attack);
        
        assert!((health.current() - 10.0).abs() < 0.01);
        
        health.restore();
        assert!((health.current() - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_hunger_sprint_interaction() {
        let hunger = Hunger::new(20.0);
        
        // Full hunger can sprint
        assert!(hunger.can_sprint());
    }

    #[test]
    fn test_hunger_eat_restores() {
        let mut hunger = Hunger::new(20.0);
        
        // Drain some hunger by ticking
        for _ in 0..60 {
            hunger.tick(1.0, true); // Sprinting drains faster
        }
        
        let before = hunger.current();
        
        // Eat food
        hunger.eat(6.0, 6.0);
        
        // Should restore
        assert!(hunger.current() >= before);
    }

    #[test]
    fn test_hunger_saturation() {
        let mut hunger = Hunger::new(20.0);
        
        // Eat food with saturation
        hunger.eat(4.0, 8.0);
        
        // Saturation should be added
        assert!(hunger.saturation() > 0.0);
    }
}

/// Test item stack operations.
mod item_stack_tests {
    use super::*;

    #[test]
    fn test_item_stack_merge_same() {
        let mut stack1 = ItemStack::new(ItemId(1), 32);
        let stack2 = ItemStack::new(ItemId(1), 32);
        
        // Merge same items (max stack is 64)
        let overflow = stack1.merge(stack2);
        assert!(overflow.is_none());
        assert_eq!(stack1.count, 64);
    }

    #[test]
    fn test_item_stack_merge_overflow() {
        let mut stack1 = ItemStack::new(ItemId(1), 50);
        let stack2 = ItemStack::new(ItemId(1), 32);
        
        // Merge with overflow
        let overflow = stack1.merge(stack2);
        assert!(overflow.is_some());
        assert_eq!(stack1.count, 64);
        let overflow = overflow.unwrap();
        assert_eq!(overflow.count, 18);
    }

    #[test]
    fn test_item_stack_merge_different_items() {
        let mut stack1 = ItemStack::new(ItemId(1), 32);
        let stack2 = ItemStack::new(ItemId(2), 32);
        
        // Cannot merge different items - returns the other stack
        let overflow = stack1.merge(stack2);
        assert!(overflow.is_some());
        assert_eq!(overflow.unwrap().count, 32);
        assert_eq!(stack1.count, 32); // Unchanged
    }

    #[test]
    fn test_item_stack_split() {
        let mut stack = ItemStack::new(ItemId(1), 64);
        
        let split = stack.split(32);
        assert!(split.is_some());
        let split = split.unwrap();
        
        assert_eq!(stack.count, 32);
        assert_eq!(split.count, 32);
        assert_eq!(split.item_id, stack.item_id);
    }
}

/// Test recipe definitions.
mod recipe_tests {
    use super::*;

    #[test]
    fn test_recipe_creation() {
        let recipe = Recipe {
            id: "test_recipe".to_string(),
            inputs: vec![
                (ItemId(1), 2),
                (ItemId(2), 1),
            ],
            output: (ItemId(10), 1),
            station: None,
            category: None,
        };
        
        assert_eq!(recipe.inputs.len(), 2);
        assert_eq!(recipe.output.1, 1);
    }

    #[test]
    fn test_recipe_with_station() {
        let recipe = Recipe {
            id: "furnace_recipe".to_string(),
            inputs: vec![(ItemId(5), 1)],
            output: (ItemId(20), 1),
            station: Some(CraftingStation::Furnace),
            category: Some("smelting".to_string()),
        };
        
        assert_eq!(recipe.station, Some(CraftingStation::Furnace));
    }

    #[test]
    fn test_recipe_stations() {
        assert_ne!(CraftingStation::CraftingTable, CraftingStation::Furnace);
        assert_ne!(CraftingStation::Furnace, CraftingStation::Anvil);
    }
}

/// Test state persistence (serialization round-trip simulation).
mod persistence_tests {
    use super::*;

    #[test]
    fn test_inventory_read_state() {
        let mut inventory = Inventory::new();
        
        // Add various items
        inventory.add(ItemStack::new(ItemId(1), 64));
        inventory.add(ItemStack::new(ItemId(2), 32));
        inventory.add(ItemStack::new(ItemId(10), 1));
        
        // Verify we can read all slot data (for serialization)
        let slot0 = inventory.get(0);
        let slot1 = inventory.get(1);
        let slot2 = inventory.get(2);
        
        assert!(slot0.is_some());
        assert!(slot1.is_some());
        assert!(slot2.is_some());
        
        assert_eq!(slot0.unwrap().item_id, ItemId(1));
        assert_eq!(slot1.unwrap().item_id, ItemId(2));
        assert_eq!(slot2.unwrap().item_id, ItemId(10));
    }

    #[test]
    fn test_health_state_capture() {
        let mut health = Health::new(20.0);
        health.damage(7.5, DamageSource::Fall);
        
        // Capture current state
        let current = health.current();
        let max = health.max();
        
        // Verify we can read state
        assert!((current - 12.5).abs() < 0.01);
        assert!((max - 20.0).abs() < 0.01);
    }
    
    #[test]
    fn test_item_stack_serializable_fields() {
        let stack = ItemStack::new(ItemId(42), 16);
        
        // Verify public fields accessible for serialization
        assert_eq!(stack.item_id, ItemId(42));
        assert_eq!(stack.count, 16);
    }
}

/// Performance sanity checks.
mod performance_tests {
    use super::*;

    #[test]
    fn test_inventory_many_operations() {
        let mut inventory = Inventory::new();
        
        // Many add operations
        for i in 0..1000 {
            let item_id = ItemId((i % 100) as u16);
            inventory.add(ItemStack::new(item_id, 1));
        }
        
        // Count should work
        let count = inventory.count_item(ItemId(0));
        assert!(count >= 10);
    }

    #[test]
    fn test_health_many_updates() {
        let mut health = Health::new(20.0);
        
        // Many small updates
        for _ in 0..1000 {
            health.damage_absolute(0.01, DamageSource::Environment);
            health.heal(0.01);
        }
        
        // Should still be valid
        assert!(!health.is_dead());
        assert!(health.current() > 0.0);
    }

    #[test]
    fn test_hunger_sustained_activity() {
        let mut hunger = Hunger::new(20.0);
        
        // Simulate extended gameplay (10 minutes)
        for _ in 0..600 {
            hunger.tick(1.0, false);
        }
        
        // Hunger should have decreased but not depleted in 10 min of walking
        assert!(hunger.current() < 20.0);
    }
}
