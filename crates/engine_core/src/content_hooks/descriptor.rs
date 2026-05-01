//! Descriptor types for scriptable content hooks.
//!
//! Descriptors are data-driven definitions for gameplay hooks, events, conditions,
//! and actions that can be declared by game packs without engine recompilation.

use serde::{Deserialize, Serialize};

use super::id::{ActionId, ConditionId, ContentHookId, EventId};

/// When an event can trigger.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventTrigger {
    #[default]
    Manual,
    OnBlockPlace,
    OnBlockBreak,
    OnEntitySpawn,
    OnEntityDeath,
    OnPlayerJoin,
    OnPlayerLeave,
    OnItemUse,
    OnTick,
    OnInterval,
    OnZoneEnter,
    OnZoneExit,
    Custom,
}

/// Descriptor for a triggerable gameplay event.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventDescriptor {
    pub id: EventId,
    pub name: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub trigger: EventTrigger,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub cancellable: bool,
    #[serde(default)]
    pub parameters: Vec<ParameterDef>,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl EventDescriptor {
    #[must_use]
    pub fn new(id: EventId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            display_name: None,
            trigger: EventTrigger::default(),
            priority: 0,
            cancellable: false,
            parameters: Vec::new(),
            tags: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_trigger(mut self, trigger: EventTrigger) -> Self {
        self.trigger = trigger;
        self
    }

    #[must_use]
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    #[must_use]
    pub fn cancellable(mut self) -> Self {
        self.cancellable = true;
        self
    }

    #[must_use]
    pub fn with_parameter(mut self, param: ParameterDef) -> Self {
        self.parameters.push(param);
        self
    }

    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

/// Type of a condition check.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConditionType {
    #[default]
    Always,
    Never,
    And,
    Or,
    Not,
    HasItem,
    InZone,
    HasPermission,
    TimeOfDay,
    Weather,
    RandomChance,
    CompareValue,
    HasTag,
    Custom,
}

/// Descriptor for a testable condition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConditionDescriptor {
    pub id: ConditionId,
    pub name: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub condition_type: ConditionType,
    #[serde(default)]
    pub parameters: Vec<ParameterDef>,
    #[serde(default)]
    pub inverted: bool,
    #[serde(default)]
    pub sub_conditions: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl ConditionDescriptor {
    #[must_use]
    pub fn new(id: ConditionId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            display_name: None,
            condition_type: ConditionType::default(),
            parameters: Vec::new(),
            inverted: false,
            sub_conditions: Vec::new(),
            tags: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_type(mut self, condition_type: ConditionType) -> Self {
        self.condition_type = condition_type;
        self
    }

    #[must_use]
    pub fn with_parameter(mut self, param: ParameterDef) -> Self {
        self.parameters.push(param);
        self
    }

    #[must_use]
    pub fn inverted(mut self) -> Self {
        self.inverted = true;
        self
    }

    #[must_use]
    pub fn with_sub_condition(mut self, condition_name: impl Into<String>) -> Self {
        self.sub_conditions.push(condition_name.into());
        self
    }

    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

/// Type of action to execute.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActionType {
    #[default]
    NoOp,
    Sequence,
    Parallel,
    Conditional,
    SpawnEntity,
    RemoveEntity,
    SetBlock,
    GiveItem,
    TakeItem,
    SendMessage,
    PlaySound,
    SpawnParticle,
    TeleportEntity,
    ApplyEffect,
    RemoveEffect,
    SetVariable,
    TriggerEvent,
    CancelEvent,
    Delay,
    Custom,
}

/// Descriptor for an executable action.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionDescriptor {
    pub id: ActionId,
    pub name: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub action_type: ActionType,
    #[serde(default)]
    pub parameters: Vec<ParameterDef>,
    #[serde(default)]
    pub sub_actions: Vec<String>,
    #[serde(default)]
    pub condition_ref: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl ActionDescriptor {
    #[must_use]
    pub fn new(id: ActionId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            display_name: None,
            action_type: ActionType::default(),
            parameters: Vec::new(),
            sub_actions: Vec::new(),
            condition_ref: None,
            tags: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_type(mut self, action_type: ActionType) -> Self {
        self.action_type = action_type;
        self
    }

    #[must_use]
    pub fn with_parameter(mut self, param: ParameterDef) -> Self {
        self.parameters.push(param);
        self
    }

    #[must_use]
    pub fn with_sub_action(mut self, action_name: impl Into<String>) -> Self {
        self.sub_actions.push(action_name.into());
        self
    }

    #[must_use]
    pub fn with_condition(mut self, condition_name: impl Into<String>) -> Self {
        self.condition_ref = Some(condition_name.into());
        self
    }

    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

/// Parameter definition for events, conditions, and actions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParameterDef {
    pub name: String,
    pub param_type: ParameterType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default_value: Option<ParameterValue>,
}

impl ParameterDef {
    #[must_use]
    pub fn new(name: impl Into<String>, param_type: ParameterType) -> Self {
        Self {
            name: name.into(),
            param_type,
            required: false,
            default_value: None,
        }
    }

    #[must_use]
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    #[must_use]
    pub fn with_default(mut self, value: ParameterValue) -> Self {
        self.default_value = Some(value);
        self
    }
}

/// Types of parameters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ParameterType {
    #[default]
    String,
    Int,
    Float,
    Bool,
    EntityRef,
    BlockPos,
    WorldPos,
    ItemStack,
    Duration,
}

/// Runtime parameter values.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ParameterValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    EntityRef(String),
    BlockPos { x: i32, y: i32, z: i32 },
    WorldPos { x: f64, y: f64, z: f64 },
    ItemStack { item: String, count: u32 },
    Duration(u64),
}

/// A complete content hook combining an event trigger with conditions and actions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentHookDescriptor {
    pub id: ContentHookId,
    pub name: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub priority: i32,
    pub event_ref: String,
    #[serde(default)]
    pub conditions: Vec<String>,
    pub actions: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl ContentHookDescriptor {
    #[must_use]
    pub fn new(id: ContentHookId, name: impl Into<String>, event_ref: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            display_name: None,
            enabled: true,
            priority: 0,
            event_ref: event_ref.into(),
            conditions: Vec::new(),
            actions: Vec::new(),
            tags: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    #[must_use]
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    #[must_use]
    pub fn with_condition(mut self, condition_name: impl Into<String>) -> Self {
        self.conditions.push(condition_name.into());
        self
    }

    #[must_use]
    pub fn with_action(mut self, action_name: impl Into<String>) -> Self {
        self.actions.push(action_name.into());
        self
    }

    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_descriptor_builder() {
        let event = EventDescriptor::new(EventId::new(1, 1), "on_block_break")
            .with_trigger(EventTrigger::OnBlockBreak)
            .cancellable()
            .with_parameter(ParameterDef::new("block_pos", ParameterType::BlockPos).required())
            .with_tag("world_event");

        assert_eq!(event.name, "on_block_break");
        assert_eq!(event.trigger, EventTrigger::OnBlockBreak);
        assert!(event.cancellable);
        assert_eq!(event.parameters.len(), 1);
        assert!(event.tags.contains(&"world_event".to_string()));
    }

    #[test]
    fn condition_descriptor_builder() {
        let condition = ConditionDescriptor::new(ConditionId::new(1, 1), "has_permission")
            .with_type(ConditionType::HasPermission)
            .with_parameter(ParameterDef::new("permission", ParameterType::String).required())
            .inverted();

        assert_eq!(condition.name, "has_permission");
        assert_eq!(condition.condition_type, ConditionType::HasPermission);
        assert!(condition.inverted);
    }

    #[test]
    fn action_descriptor_builder() {
        let action = ActionDescriptor::new(ActionId::new(1, 1), "give_reward")
            .with_type(ActionType::GiveItem)
            .with_parameter(
                ParameterDef::new("item", ParameterType::ItemStack).with_default(
                    ParameterValue::ItemStack {
                        item: "gold".to_string(),
                        count: 10,
                    },
                ),
            )
            .with_condition("is_eligible");

        assert_eq!(action.name, "give_reward");
        assert_eq!(action.action_type, ActionType::GiveItem);
        assert_eq!(action.condition_ref, Some("is_eligible".to_string()));
    }

    #[test]
    fn content_hook_descriptor_builder() {
        let hook = ContentHookDescriptor::new(
            ContentHookId::new(1, 1),
            "reward_on_kill",
            "on_entity_death",
        )
        .with_priority(10)
        .with_condition("is_boss")
        .with_action("give_reward")
        .with_action("play_fanfare");

        assert_eq!(hook.name, "reward_on_kill");
        assert_eq!(hook.event_ref, "on_entity_death");
        assert_eq!(hook.priority, 10);
        assert_eq!(hook.conditions, vec!["is_boss"]);
        assert_eq!(hook.actions, vec!["give_reward", "play_fanfare"]);
    }

    #[test]
    fn descriptor_serde_roundtrip() {
        let event = EventDescriptor::new(EventId::new(1, 2), "test_event")
            .with_trigger(EventTrigger::OnTick)
            .with_parameter(ParameterDef::new("delta", ParameterType::Float));

        let json = serde_json::to_string(&event).unwrap();
        let restored: EventDescriptor = serde_json::from_str(&json).unwrap();

        assert_eq!(event.id, restored.id);
        assert_eq!(event.name, restored.name);
        assert_eq!(event.trigger, restored.trigger);
    }

    #[test]
    fn descriptor_bincode_roundtrip() {
        let action = ActionDescriptor::new(ActionId::new(1, 3), "teleport")
            .with_type(ActionType::TeleportEntity)
            .with_parameter(ParameterDef::new("target", ParameterType::WorldPos).required());

        let bytes = bincode::serialize(&action).unwrap();
        let restored: ActionDescriptor = bincode::deserialize(&bytes).unwrap();

        assert_eq!(action.id, restored.id);
        assert_eq!(action.action_type, restored.action_type);
    }

    #[test]
    fn parameter_value_variants() {
        let values = vec![
            ParameterValue::String("hello".to_string()),
            ParameterValue::Int(42),
            ParameterValue::Float(3.5),
            ParameterValue::Bool(true),
            ParameterValue::EntityRef("player_1".to_string()),
            ParameterValue::BlockPos { x: 1, y: 2, z: 3 },
            ParameterValue::WorldPos {
                x: 1.5,
                y: 2.5,
                z: 3.5,
            },
            ParameterValue::ItemStack {
                item: "sword".to_string(),
                count: 1,
            },
            ParameterValue::Duration(1000),
        ];

        for value in values {
            let json = serde_json::to_string(&value).unwrap();
            let restored: ParameterValue = serde_json::from_str(&json).unwrap();
            assert_eq!(value, restored);
        }
    }
}
