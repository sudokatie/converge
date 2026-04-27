//! Crafting system with recipes and execution.

mod executor;
mod furnace;
mod registry;

pub use executor::{
    CraftError, CraftRequirements, check_craft, execute_craft, execute_craft_by_id,
};
pub use furnace::{
    DEFAULT_SMELT_TIME, FUEL_CHARCOAL, FUEL_COAL, FUEL_LAVA_BUCKET, FUEL_STICK, FUEL_WOOD,
    FuelEntry, Furnace, FurnaceState,
};
pub use registry::{CraftingStation, Ingredient, Recipe, RecipeRegistry};
