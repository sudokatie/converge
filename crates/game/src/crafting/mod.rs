//! Crafting system with recipes and execution.

mod executor;
mod furnace;
mod registry;

pub use executor::{check_craft, execute_craft, execute_craft_by_id, CraftError, CraftRequirements};
pub use furnace::{
    Furnace, FurnaceState, FuelEntry, DEFAULT_SMELT_TIME, FUEL_CHARCOAL, FUEL_COAL,
    FUEL_LAVA_BUCKET, FUEL_STICK, FUEL_WOOD,
};
pub use registry::{CraftingStation, Ingredient, Recipe, RecipeRegistry};
