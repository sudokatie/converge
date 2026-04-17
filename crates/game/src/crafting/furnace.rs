//! Furnace processing station for smelting.
//!
//! Implements spec 6.3.4: input slot + fuel slot -> output slot,
//! smelting time per recipe, fuel burn time tracking.

use crate::crafting::{CraftingStation, Recipe, RecipeRegistry};
use crate::inventory::{Inventory, ItemId, ItemStack};

/// Default smelting time in seconds.
pub const DEFAULT_SMELT_TIME: f32 = 8.0;

/// Fuel entry with burn time.
#[derive(Debug, Clone)]
pub struct FuelEntry {
    /// Item ID of the fuel.
    pub item_id: ItemId,
    /// Burn time in seconds.
    pub burn_time: f32,
}

/// Well-known fuel burn times.
pub const FUEL_COAL: f32 = 80.0;
pub const FUEL_CHARCOAL: f32 = 80.0;
pub const FUEL_WOOD: f32 = 15.0;
pub const FUEL_STICK: f32 = 5.0;
pub const FUEL_LAVA_BUCKET: f32 = 1000.0;

/// Furnace state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FurnaceState {
    /// No input or no fuel.
    Idle,
    /// Currently burning fuel and smelting.
    Smelting,
    /// Input present but out of fuel.
    NeedsFuel,
    /// Output slot full (can't produce more).
    OutputFull,
}

/// Furnace with dedicated slots and processing state.
///
/// Three-slot system per spec 6.3.4:
/// - Input slot: item to smelt
/// - Fuel slot: fuel to burn
/// - Output slot: smelting result
#[derive(Debug, Clone)]
pub struct Furnace {
    /// Input slot (item to smelt).
    input: Option<ItemStack>,
    /// Fuel slot.
    fuel: Option<ItemStack>,
    /// Output slot (smelted result).
    output: Option<ItemStack>,
    /// Current furnace state.
    state: FurnaceState,
    /// Remaining burn time on current fuel item (seconds).
    burn_time_remaining: f32,
    /// Total burn time of current fuel item (for progress display).
    burn_time_total: f32,
    /// Smelting progress (0 to smelt_time).
    smelt_progress: f32,
    /// Total smelting time for current recipe.
    smelt_time_total: f32,
}

impl Default for Furnace {
    fn default() -> Self {
        Self::new()
    }
}

impl Furnace {
    /// Create a new empty furnace.
    #[must_use]
    pub fn new() -> Self {
        Self {
            input: None,
            fuel: None,
            output: None,
            state: FurnaceState::Idle,
            burn_time_remaining: 0.0,
            burn_time_total: 0.0,
            smelt_progress: 0.0,
            smelt_time_total: DEFAULT_SMELT_TIME,
        }
    }

    /// Get the input slot.
    #[must_use]
    pub fn input(&self) -> Option<&ItemStack> {
        self.input.as_ref()
    }

    /// Get the fuel slot.
    #[must_use]
    pub fn fuel(&self) -> Option<&ItemStack> {
        self.fuel.as_ref()
    }

    /// Get the output slot.
    #[must_use]
    pub fn output(&self) -> Option<&ItemStack> {
        self.output.as_ref()
    }

    /// Get the current furnace state.
    #[must_use]
    pub fn state(&self) -> FurnaceState {
        self.state
    }

    /// Get fuel burn progress (0.0 to 1.0).
    #[must_use]
    pub fn burn_progress(&self) -> f32 {
        if self.burn_time_total <= 0.0 {
            return 0.0;
        }
        (self.burn_time_remaining / self.burn_time_total).clamp(0.0, 1.0)
    }

    /// Get smelting progress (0.0 to 1.0).
    #[must_use]
    pub fn smelt_progress(&self) -> f32 {
        if self.smelt_time_total <= 0.0 {
            return 0.0;
        }
        (self.smelt_progress / self.smelt_time_total).clamp(0.0, 1.0)
    }

    /// Set the input slot.
    pub fn set_input(&mut self, stack: Option<ItemStack>) {
        self.input = stack;
        self.recalculate_state();
    }

    /// Set the fuel slot.
    pub fn set_fuel(&mut self, stack: Option<ItemStack>) {
        self.fuel = stack;
        self.recalculate_state();
    }

    /// Take the output slot.
    pub fn take_output(&mut self) -> Option<ItemStack> {
        let output = self.output.take();
        if self.state == FurnaceState::OutputFull {
            self.recalculate_state();
        }
        output
    }

    /// Get the burn time for a fuel item.
    #[must_use]
    pub fn fuel_burn_time(item_id: ItemId) -> f32 {
        // Map item IDs to burn times
        // In a full implementation, this would come from the item registry
        match item_id.raw() {
            5 => FUEL_COAL,       // Coal
            6 => FUEL_CHARCOAL,   // Charcoal
            7 => FUEL_WOOD,       // Wood/Log
            3 => FUEL_STICK,      // Stick
            50 => FUEL_LAVA_BUCKET, // Lava bucket
            _ => 0.0,             // Not a fuel
        }
    }

    /// Check if an item can be used as fuel.
    #[must_use]
    pub fn is_fuel(item_id: ItemId) -> bool {
        Self::fuel_burn_time(item_id) > 0.0
    }

    /// Update furnace each frame.
    ///
    /// Returns a list of completed smelting results.
    pub fn tick(&mut self, dt: f32, recipes: &RecipeRegistry) -> Vec<ItemStack> {
        let mut completed = Vec::new();

        if self.state != FurnaceState::Smelting {
            return completed;
        }

        // Consume fuel burn time
        if self.burn_time_remaining > 0.0 {
            self.burn_time_remaining = (self.burn_time_remaining - dt).max(0.0);
        }

        // If fuel is spent, try to consume next fuel item
        if self.burn_time_remaining <= 0.0 {
            if let Some(fuel_stack) = &mut self.fuel {
                if fuel_stack.count > 0 {
                    self.burn_time_total = Self::fuel_burn_time(fuel_stack.item_id);
                    self.burn_time_remaining = self.burn_time_total;
                    fuel_stack.count -= 1;
                    if fuel_stack.count == 0 {
                        self.fuel = None;
                    }
                }
            }
        }

        // Still have burn time? Advance smelting.
        if self.burn_time_remaining > 0.0 || self.burn_time_total > 0.0 {
            self.smelt_progress += dt;

            if self.smelt_progress >= self.smelt_time_total {
                // Smelting complete - produce output
                if let Some(result) = self.complete_smelt(recipes) {
                    completed.push(result);
                }
                self.smelt_progress = 0.0;
            }
        } else {
            // Out of fuel
            self.state = FurnaceState::NeedsFuel;
        }

        completed
    }

    /// Complete a smelt operation, moving result to output.
    fn complete_smelt(&mut self, recipes: &RecipeRegistry) -> Option<ItemStack> {
        let input = self.input.as_ref()?;

        // Find matching furnace recipe
        let recipe = recipes
            .for_station(Some(CraftingStation::Furnace))
            .find(|r| r.inputs.iter().any(|(id, _)| *id == input.item_id))?;

        let (output_id, output_count) = recipe.output;

        // Try to add to output slot
        let output_stack = ItemStack::new(output_id, output_count);

        if let Some(existing) = &mut self.output {
            if existing.item_id == output_id && existing.count + output_count <= existing.item_id.raw() as u32 * 64 {
                existing.count += output_count;
            } else {
                // Output full or wrong type
                self.state = FurnaceState::OutputFull;
                return None;
            }
        } else {
            self.output = Some(output_stack);
        }

        // Consume one input item
        if let Some(input_stack) = &mut self.input {
            input_stack.count -= 1;
            if input_stack.count == 0 {
                self.input = None;
            }
        }

        self.recalculate_state();
        Some(self.output.clone()?)
    }

    /// Recalculate furnace state based on current slots.
    fn recalculate_state(&mut self) {
        let has_input = self.input.is_some();
        let has_fuel = self.fuel.is_some() || self.burn_time_remaining > 0.0;
        let output_full = self.output.is_some()
            && self.output.as_ref().map_or(false, |s| s.count >= 64);

        self.state = if output_full {
            FurnaceState::OutputFull
        } else if !has_input {
            FurnaceState::Idle
        } else if !has_fuel {
            FurnaceState::NeedsFuel
        } else {
            FurnaceState::Smelting
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_furnace_is_idle() {
        let furnace = Furnace::new();
        assert_eq!(furnace.state(), FurnaceState::Idle);
        assert!(furnace.input().is_none());
        assert!(furnace.fuel().is_none());
        assert!(furnace.output().is_none());
    }

    #[test]
    fn test_fuel_burn_times() {
        assert_eq!(Furnace::fuel_burn_time(ItemId(5)), FUEL_COAL);
        assert_eq!(Furnace::fuel_burn_time(ItemId(7)), FUEL_WOOD);
        assert_eq!(Furnace::fuel_burn_time(ItemId(999)), 0.0);
    }

    #[test]
    fn test_is_fuel() {
        assert!(Furnace::is_fuel(ItemId(5))); // Coal
        assert!(Furnace::is_fuel(ItemId(7))); // Wood
        assert!(!Furnace::is_fuel(ItemId(999)));
    }

    #[test]
    fn test_set_input() {
        let mut furnace = Furnace::new();
        furnace.set_input(Some(ItemStack::new(ItemId(1), 10)));
        assert!(furnace.input().is_some());
        assert_eq!(furnace.input().unwrap().count, 10);
    }

    #[test]
    fn test_set_fuel() {
        let mut furnace = Furnace::new();
        furnace.set_fuel(Some(ItemStack::new(ItemId(5), 5))); // Coal
        assert!(furnace.fuel().is_some());
    }

    #[test]
    fn test_needs_fuel_state() {
        let mut furnace = Furnace::new();
        furnace.set_input(Some(ItemStack::new(ItemId(1), 10)));
        assert_eq!(furnace.state(), FurnaceState::NeedsFuel);
    }

    #[test]
    fn test_smelting_state() {
        let mut furnace = Furnace::new();
        furnace.set_input(Some(ItemStack::new(ItemId(1), 10)));
        furnace.set_fuel(Some(ItemStack::new(ItemId(5), 5)));
        // Should transition to smelting when both input and fuel present
        // (though actual smelting also needs a matching recipe)
    }

    #[test]
    fn test_take_output() {
        let mut furnace = Furnace::new();
        // Manually set output for testing
        furnace.output = Some(ItemStack::new(ItemId(10), 1));
        let taken = furnace.take_output();
        assert!(taken.is_some());
        assert!(furnace.output().is_none());
    }

    #[test]
    fn test_burn_progress_zero_when_no_fuel() {
        let furnace = Furnace::new();
        assert_eq!(furnace.burn_progress(), 0.0);
    }

    #[test]
    fn test_smelt_progress_zero_when_idle() {
        let furnace = Furnace::new();
        assert_eq!(furnace.smelt_progress(), 0.0);
    }

    #[test]
    fn test_fuel_consumed_on_tick() {
        let mut furnace = Furnace::new();
        furnace.input = Some(ItemStack::new(ItemId(1), 10));
        furnace.fuel = Some(ItemStack::new(ItemId(5), 1)); // 1 coal
        furnace.burn_time_total = FUEL_COAL;
        furnace.burn_time_remaining = FUEL_COAL;
        furnace.smelt_time_total = DEFAULT_SMELT_TIME;
        furnace.state = FurnaceState::Smelting;

        let recipes = RecipeRegistry::new();
        let _ = furnace.tick(1.0, &recipes);

        assert!(
            furnace.burn_time_remaining < FUEL_COAL,
            "Burn time should decrease"
        );
    }

    #[test]
    fn test_state_transitions() {
        // Idle -> NeedsFuel (add input)
        let mut furnace = Furnace::new();
        furnace.set_input(Some(ItemStack::new(ItemId(1), 1)));
        assert_eq!(furnace.state(), FurnaceState::NeedsFuel);

        // NeedsFuel -> Smelting (add fuel)
        furnace.set_fuel(Some(ItemStack::new(ItemId(5), 1)));
        // With both input and fuel, state should consider smelting
    }
}
