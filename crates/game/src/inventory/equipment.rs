//! Equipment system for wearable items and stat modifiers.
//!
//! Implements spec 6.2.4 - equipment slots and stat modifiers.

use serde::{Deserialize, Serialize};

use super::{ItemId, ItemStack};

/// Equipment slot types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EquipmentSlot {
    /// Head armor (helmet).
    Head,
    /// Chest armor (chestplate).
    Chest,
    /// Leg armor (leggings).
    Legs,
    /// Foot armor (boots).
    Feet,
    /// Primary hand (weapons, tools).
    Mainhand,
    /// Secondary hand (shields, torches).
    Offhand,
}

impl EquipmentSlot {
    /// Get all armor slots.
    #[must_use]
    pub fn armor_slots() -> &'static [EquipmentSlot] {
        &[
            EquipmentSlot::Head,
            EquipmentSlot::Chest,
            EquipmentSlot::Legs,
            EquipmentSlot::Feet,
        ]
    }

    /// Get all hand slots.
    #[must_use]
    pub fn hand_slots() -> &'static [EquipmentSlot] {
        &[EquipmentSlot::Mainhand, EquipmentSlot::Offhand]
    }

    /// Get all slots in order.
    #[must_use]
    pub fn all() -> &'static [EquipmentSlot] {
        &[
            EquipmentSlot::Head,
            EquipmentSlot::Chest,
            EquipmentSlot::Legs,
            EquipmentSlot::Feet,
            EquipmentSlot::Mainhand,
            EquipmentSlot::Offhand,
        ]
    }

    /// Check if this is an armor slot.
    #[must_use]
    pub fn is_armor(&self) -> bool {
        matches!(
            self,
            EquipmentSlot::Head
                | EquipmentSlot::Chest
                | EquipmentSlot::Legs
                | EquipmentSlot::Feet
        )
    }

    /// Check if this is a hand slot.
    #[must_use]
    pub fn is_hand(&self) -> bool {
        matches!(self, EquipmentSlot::Mainhand | EquipmentSlot::Offhand)
    }

    /// Get the index of this slot (for array access).
    #[must_use]
    pub fn index(&self) -> usize {
        match self {
            EquipmentSlot::Head => 0,
            EquipmentSlot::Chest => 1,
            EquipmentSlot::Legs => 2,
            EquipmentSlot::Feet => 3,
            EquipmentSlot::Mainhand => 4,
            EquipmentSlot::Offhand => 5,
        }
    }

    /// Get slot from index.
    #[must_use]
    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(EquipmentSlot::Head),
            1 => Some(EquipmentSlot::Chest),
            2 => Some(EquipmentSlot::Legs),
            3 => Some(EquipmentSlot::Feet),
            4 => Some(EquipmentSlot::Mainhand),
            5 => Some(EquipmentSlot::Offhand),
            _ => None,
        }
    }
}

/// Stat modifiers from equipped items.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StatModifiers {
    /// Armor points (damage reduction).
    pub armor: f32,
    /// Armor toughness (reduces high damage).
    pub armor_toughness: f32,
    /// Attack damage bonus.
    pub attack_damage: f32,
    /// Attack speed modifier (multiplier).
    pub attack_speed: f32,
    /// Movement speed modifier (multiplier).
    pub movement_speed: f32,
    /// Mining speed modifier (multiplier).
    pub mining_speed: f32,
    /// Knockback resistance (0.0 to 1.0).
    pub knockback_resistance: f32,
    /// Max health bonus.
    pub max_health: f32,
}

impl StatModifiers {
    /// Create empty modifiers.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create modifiers with armor value.
    #[must_use]
    pub fn armor(value: f32) -> Self {
        Self {
            armor: value,
            ..Default::default()
        }
    }

    /// Create modifiers with attack damage.
    #[must_use]
    pub fn damage(value: f32) -> Self {
        Self {
            attack_damage: value,
            ..Default::default()
        }
    }

    /// Add another modifier set to this one.
    pub fn add(&mut self, other: &StatModifiers) {
        self.armor += other.armor;
        self.armor_toughness += other.armor_toughness;
        self.attack_damage += other.attack_damage;
        self.attack_speed *= other.attack_speed.max(0.01);
        self.movement_speed *= other.movement_speed.max(0.01);
        self.mining_speed *= other.mining_speed.max(0.01);
        self.knockback_resistance =
            (self.knockback_resistance + other.knockback_resistance).min(1.0);
        self.max_health += other.max_health;
    }

    /// Calculate damage reduction from armor.
    ///
    /// Uses simplified armor formula: reduction = armor / (armor + 10)
    #[must_use]
    pub fn damage_reduction(&self, damage: f32) -> f32 {
        let effective_armor = self.armor - (damage / (2.0 + self.armor_toughness / 4.0)).max(0.0);
        let effective_armor = effective_armor.max(0.0);
        effective_armor / (effective_armor + 10.0)
    }

    /// Apply armor reduction to damage.
    #[must_use]
    pub fn reduce_damage(&self, damage: f32) -> f32 {
        damage * (1.0 - self.damage_reduction(damage))
    }
}

/// Equipment definition for an item type.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EquipmentDef {
    /// Which slot this item goes in.
    pub slot: EquipmentSlot,
    /// Stat modifiers when equipped.
    pub modifiers: StatModifiers,
}

impl EquipmentDef {
    /// Create a new equipment definition.
    #[must_use]
    pub fn new(slot: EquipmentSlot, modifiers: StatModifiers) -> Self {
        Self { slot, modifiers }
    }

    /// Create helmet equipment.
    #[must_use]
    pub fn helmet(armor: f32) -> Self {
        Self::new(EquipmentSlot::Head, StatModifiers::armor(armor))
    }

    /// Create chestplate equipment.
    #[must_use]
    pub fn chestplate(armor: f32) -> Self {
        Self::new(EquipmentSlot::Chest, StatModifiers::armor(armor))
    }

    /// Create leggings equipment.
    #[must_use]
    pub fn leggings(armor: f32) -> Self {
        Self::new(EquipmentSlot::Legs, StatModifiers::armor(armor))
    }

    /// Create boots equipment.
    #[must_use]
    pub fn boots(armor: f32) -> Self {
        Self::new(EquipmentSlot::Feet, StatModifiers::armor(armor))
    }

    /// Create weapon equipment.
    #[must_use]
    pub fn weapon(damage: f32) -> Self {
        Self::new(EquipmentSlot::Mainhand, StatModifiers::damage(damage))
    }
}

/// Number of equipment slots.
pub const EQUIPMENT_SLOT_COUNT: usize = 6;

/// Player equipment container.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Equipment {
    /// Equipped items indexed by slot.
    slots: [Option<ItemStack>; EQUIPMENT_SLOT_COUNT],
}

impl Equipment {
    /// Create empty equipment.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the item in a slot.
    #[must_use]
    pub fn get(&self, slot: EquipmentSlot) -> Option<&ItemStack> {
        self.slots[slot.index()].as_ref()
    }

    /// Get mutable access to a slot.
    pub fn get_mut(&mut self, slot: EquipmentSlot) -> Option<&mut ItemStack> {
        self.slots[slot.index()].as_mut()
    }

    /// Equip an item to a slot, returning the previously equipped item.
    pub fn equip(&mut self, slot: EquipmentSlot, item: ItemStack) -> Option<ItemStack> {
        std::mem::replace(&mut self.slots[slot.index()], Some(item))
    }

    /// Unequip an item from a slot.
    pub fn unequip(&mut self, slot: EquipmentSlot) -> Option<ItemStack> {
        self.slots[slot.index()].take()
    }

    /// Check if a slot is empty.
    #[must_use]
    pub fn is_empty(&self, slot: EquipmentSlot) -> bool {
        self.slots[slot.index()].is_none()
    }

    /// Check if all slots are empty.
    #[must_use]
    pub fn is_all_empty(&self) -> bool {
        self.slots.iter().all(|s| s.is_none())
    }

    /// Get the mainhand item (primary weapon/tool).
    #[must_use]
    pub fn mainhand(&self) -> Option<&ItemStack> {
        self.get(EquipmentSlot::Mainhand)
    }

    /// Get the offhand item (shield, torch, etc.).
    #[must_use]
    pub fn offhand(&self) -> Option<&ItemStack> {
        self.get(EquipmentSlot::Offhand)
    }

    /// Get the mainhand item ID.
    #[must_use]
    pub fn mainhand_id(&self) -> Option<ItemId> {
        self.mainhand().map(|s| s.item_id)
    }

    /// Iterate over all equipped items.
    pub fn iter(&self) -> impl Iterator<Item = (EquipmentSlot, &ItemStack)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| {
                slot.as_ref()
                    .map(|item| (EquipmentSlot::from_index(i).unwrap(), item))
            })
    }

    /// Calculate total stat modifiers from all equipped items.
    ///
    /// Requires a function to look up equipment definitions for items.
    pub fn total_modifiers<F>(&self, get_def: F) -> StatModifiers
    where
        F: Fn(ItemId) -> Option<StatModifiers>,
    {
        let mut total = StatModifiers {
            attack_speed: 1.0,
            movement_speed: 1.0,
            mining_speed: 1.0,
            ..Default::default()
        };

        for slot in &self.slots {
            if let Some(item) = slot {
                if let Some(modifiers) = get_def(item.item_id) {
                    total.add(&modifiers);
                }
            }
        }

        total
    }

    /// Calculate total armor value from equipped armor.
    pub fn total_armor<F>(&self, get_def: F) -> f32
    where
        F: Fn(ItemId) -> Option<f32>,
    {
        let mut total = 0.0;
        for slot in EquipmentSlot::armor_slots() {
            if let Some(item) = self.get(*slot) {
                if let Some(armor) = get_def(item.item_id) {
                    total += armor;
                }
            }
        }
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equipment_slot_index() {
        assert_eq!(EquipmentSlot::Head.index(), 0);
        assert_eq!(EquipmentSlot::Mainhand.index(), 4);
        assert_eq!(EquipmentSlot::from_index(0), Some(EquipmentSlot::Head));
        assert_eq!(EquipmentSlot::from_index(6), None);
    }

    #[test]
    fn test_equipment_slot_categories() {
        assert!(EquipmentSlot::Head.is_armor());
        assert!(!EquipmentSlot::Head.is_hand());
        assert!(!EquipmentSlot::Mainhand.is_armor());
        assert!(EquipmentSlot::Mainhand.is_hand());
    }

    #[test]
    fn test_equipment_equip_unequip() {
        let mut equipment = Equipment::new();
        let item = ItemStack::new(ItemId(100), 1);

        assert!(equipment.is_empty(EquipmentSlot::Head));

        let prev = equipment.equip(EquipmentSlot::Head, item.clone());
        assert!(prev.is_none());
        assert!(!equipment.is_empty(EquipmentSlot::Head));

        let removed = equipment.unequip(EquipmentSlot::Head);
        assert!(removed.is_some());
        assert!(equipment.is_empty(EquipmentSlot::Head));
    }

    #[test]
    fn test_equipment_replace() {
        let mut equipment = Equipment::new();
        let item1 = ItemStack::new(ItemId(100), 1);
        let item2 = ItemStack::new(ItemId(101), 1);

        equipment.equip(EquipmentSlot::Head, item1.clone());
        let prev = equipment.equip(EquipmentSlot::Head, item2);

        assert!(prev.is_some());
        assert_eq!(prev.unwrap().item_id, ItemId(100));
    }

    #[test]
    fn test_stat_modifiers_add() {
        let mut base = StatModifiers {
            armor: 5.0,
            attack_speed: 1.0,
            movement_speed: 1.0,
            mining_speed: 1.0,
            ..Default::default()
        };

        let bonus = StatModifiers {
            armor: 3.0,
            attack_damage: 2.0,
            attack_speed: 1.0,
            movement_speed: 1.0,
            mining_speed: 1.0,
            ..Default::default()
        };

        base.add(&bonus);

        assert_eq!(base.armor, 8.0);
        assert_eq!(base.attack_damage, 2.0);
    }

    #[test]
    fn test_damage_reduction() {
        let mods = StatModifiers {
            armor: 10.0,
            ..Default::default()
        };

        let reduction = mods.damage_reduction(5.0);
        assert!(reduction > 0.0);
        assert!(reduction < 1.0);

        let reduced = mods.reduce_damage(10.0);
        assert!(reduced < 10.0);
    }

    #[test]
    fn test_total_modifiers() {
        let mut equipment = Equipment::new();
        equipment.equip(EquipmentSlot::Head, ItemStack::new(ItemId(1), 1));
        equipment.equip(EquipmentSlot::Chest, ItemStack::new(ItemId(2), 1));

        let total = equipment.total_modifiers(|id| {
            match id.0 {
                1 => Some(StatModifiers::armor(2.0)),
                2 => Some(StatModifiers::armor(6.0)),
                _ => None,
            }
        });

        assert_eq!(total.armor, 8.0);
    }

    #[test]
    fn test_equipment_iter() {
        let mut equipment = Equipment::new();
        equipment.equip(EquipmentSlot::Head, ItemStack::new(ItemId(1), 1));
        equipment.equip(EquipmentSlot::Mainhand, ItemStack::new(ItemId(2), 1));

        let items: Vec<_> = equipment.iter().collect();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_equipment_def_presets() {
        let helmet = EquipmentDef::helmet(3.0);
        assert_eq!(helmet.slot, EquipmentSlot::Head);
        assert_eq!(helmet.modifiers.armor, 3.0);

        let weapon = EquipmentDef::weapon(7.0);
        assert_eq!(weapon.slot, EquipmentSlot::Mainhand);
        assert_eq!(weapon.modifiers.attack_damage, 7.0);
    }
}
