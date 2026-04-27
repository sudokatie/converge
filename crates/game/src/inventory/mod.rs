//! Inventory and item management.

mod container;
mod durability;
mod registry;
mod tools;

pub use container::{HOTBAR_SIZE, INVENTORY_SIZE, Inventory, ItemId, ItemStack, MAX_STACK_SIZE};
pub use durability::{Durability, DurableItem, ToolBrokeEvent, ToolDurability};
pub use registry::{ItemCategory, ItemDef, ItemRegistry, ToolType};
pub use tools::{
    BlockHardness, BlockToolProperties, ToolTier, calculate_break_time, calculate_mining_speed,
    default_block_properties, will_drop_items,
};
