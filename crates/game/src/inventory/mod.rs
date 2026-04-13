//! Inventory and item management.

mod container;
mod durability;
mod registry;
mod tools;

pub use container::{Inventory, ItemId, ItemStack, HOTBAR_SIZE, INVENTORY_SIZE, MAX_STACK_SIZE};
pub use durability::{DurableItem, Durability, ToolBrokeEvent, ToolDurability};
pub use registry::{ItemCategory, ItemDef, ItemRegistry, ToolType};
pub use tools::{
    calculate_break_time, calculate_mining_speed, default_block_properties, will_drop_items,
    BlockHardness, BlockToolProperties, ToolTier,
};
