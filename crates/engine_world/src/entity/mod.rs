//! Entity system for the voxel engine

pub mod equipment;
pub mod status;

pub use equipment::{
    EquipmentFingerprint, EquipmentLoadout, EquipmentModule, FilterType, GrappleType, LoadoutError,
    LoadoutTickResult, MAX_MODULES, ModuleCategory, ModuleConfig, ModuleEffect, ModuleId,
    ModuleStatus, ModuleTickResult, ModuleTier, ResourceState, TankContent,
};
pub use status::{StatusEffect, StatusEffectManager, StatusEffectType, TickResult};
