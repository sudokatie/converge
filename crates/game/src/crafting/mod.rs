//! Crafting system with recipes and execution.

mod executor;
mod registry;

pub use executor::{check_craft, execute_craft, execute_craft_by_id, CraftError, CraftRequirements};
pub use registry::{CraftingStation, Ingredient, Recipe, RecipeRegistry};
