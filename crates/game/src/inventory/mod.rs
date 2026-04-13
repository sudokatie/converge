//! Inventory and item management.

mod container;
mod registry;

pub use container::{Inventory, ItemId, ItemStack, HOTBAR_SIZE, INVENTORY_SIZE, MAX_STACK_SIZE};
pub use registry::{ItemCategory, ItemDef, ItemRegistry, ToolType};
