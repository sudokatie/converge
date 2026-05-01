//! Scriptable content hooks for high-level gameplay without engine recompiles.
//!
//! Provides a data-driven system for game packs to declare gameplay hooks that
//! combine events, conditions, and actions. This enables modular gameplay
//! scripting without requiring engine recompilation.
//!
//! # Architecture
//!
//! - **Events**: Triggerable gameplay moments (block break, entity spawn, etc.)
//! - **Conditions**: Testable predicates (has item, in zone, random chance, etc.)
//! - **Actions**: Executable behaviors (give item, spawn entity, play sound, etc.)
//! - **Content Hooks**: Combine an event trigger with conditions and actions
//!
//! # Features
//!
//! - Data-driven declarative definitions (no callbacks)
//! - Serde/bincode serialization for persistence
//! - Deterministic ordering and fingerprinting
//! - Reference validation (events, conditions, actions)
//! - Circular reference detection
//! - Hook activation planning and querying
//!
//! # Example
//!
//! ```
//! use engine_core::content_hooks::{
//!     ContentHookRegistry, EventDescriptor, ConditionDescriptor,
//!     ActionDescriptor, ContentHookDescriptor, EventTrigger, ConditionType, ActionType,
//! };
//!
//! let mut registry = ContentHookRegistry::new(42);
//!
//! // Define an event
//! let event = EventDescriptor::new(registry.generate_event_id(), "on_boss_kill")
//!     .with_trigger(EventTrigger::OnEntityDeath);
//! registry.register_event(event).unwrap();
//!
//! // Define a condition
//! let condition = ConditionDescriptor::new(registry.generate_condition_id(), "has_quest")
//!     .with_type(ConditionType::HasTag);
//! registry.register_condition(condition).unwrap();
//!
//! // Define an action
//! let action = ActionDescriptor::new(registry.generate_action_id(), "give_reward")
//!     .with_type(ActionType::GiveItem);
//! registry.register_action(action).unwrap();
//!
//! // Create a hook combining them
//! let hook = ContentHookDescriptor::new(
//!     registry.generate_hook_id(),
//!     "boss_reward_hook",
//!     "on_boss_kill"
//! )
//!     .with_condition("has_quest")
//!     .with_action("give_reward");
//! registry.register_hook(hook).unwrap();
//!
//! // Validate all references
//! let report = registry.validate();
//! assert!(report.is_valid());
//! ```

mod descriptor;
mod error;
mod fingerprint;
mod id;
mod registry;

pub use descriptor::{
    ActionDescriptor, ActionType, ConditionDescriptor, ConditionType, ContentHookDescriptor,
    EventDescriptor, EventTrigger, ParameterDef, ParameterType, ParameterValue,
};
pub use error::{ContentHookError, ContentHookResult};
pub use fingerprint::{ContentHookFingerprint, HookFingerprintBuilder};
pub use id::{ActionId, ConditionId, ContentHookId, EventId};
pub use registry::{
    ContentHookRegistry, HookActivationPlan, HookActivationStatus, HookIdGenerator, HookQuery,
    HookValidationReport,
};
