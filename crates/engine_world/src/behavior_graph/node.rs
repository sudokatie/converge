//! Graph nodes combining triggers, conditions, and actions.

use serde::{Deserialize, Serialize};

use super::action::BehaviorAction;
use super::condition::BehaviorCondition;
use super::trigger::BehaviorTrigger;

/// Unique identifier for a node within a behavior graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NodeId(pub u32);

impl NodeId {
    /// Create a node ID from raw value.
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw ID value.
    #[must_use]
    pub const fn raw(&self) -> u32 {
        self.0
    }
}

/// A node in the behavior graph.
///
/// Each node specifies:
/// - A trigger that activates this node
/// - Optional conditions that must be satisfied
/// - Actions to execute when activated and conditions pass
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BehaviorNode {
    /// Unique identifier within the graph.
    pub id: NodeId,

    /// Human-readable name for debugging.
    pub name: Option<String>,

    /// The trigger that activates this node.
    pub trigger: BehaviorTrigger,

    /// Conditions that must all pass for actions to execute.
    pub conditions: Vec<BehaviorCondition>,

    /// Actions to execute when triggered and conditions pass.
    pub actions: Vec<BehaviorAction>,

    /// Priority for ordering when multiple nodes activate (higher = first).
    pub priority: i16,

    /// Whether this node is enabled.
    pub enabled: bool,

    /// Whether to stop processing other nodes after this one fires.
    pub exclusive: bool,

    /// Cooldown in ticks before this node can fire again.
    pub cooldown: u32,
}

impl BehaviorNode {
    /// Create a new behavior node with the given ID.
    #[must_use]
    pub fn new(id: u32) -> Self {
        Self {
            id: NodeId::new(id),
            name: None,
            trigger: BehaviorTrigger::Use,
            conditions: Vec::new(),
            actions: Vec::new(),
            priority: 0,
            enabled: true,
            exclusive: false,
            cooldown: 0,
        }
    }

    /// Set the node name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the trigger.
    #[must_use]
    pub fn with_trigger(mut self, trigger: BehaviorTrigger) -> Self {
        self.trigger = trigger;
        self
    }

    /// Add a condition.
    #[must_use]
    pub fn with_condition(mut self, condition: BehaviorCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Add multiple conditions.
    #[must_use]
    pub fn with_conditions(
        mut self,
        conditions: impl IntoIterator<Item = BehaviorCondition>,
    ) -> Self {
        self.conditions.extend(conditions);
        self
    }

    /// Add an action.
    #[must_use]
    pub fn with_action(mut self, action: BehaviorAction) -> Self {
        self.actions.push(action);
        self
    }

    /// Add multiple actions.
    #[must_use]
    pub fn with_actions(mut self, actions: impl IntoIterator<Item = BehaviorAction>) -> Self {
        self.actions.extend(actions);
        self
    }

    /// Set priority.
    #[must_use]
    pub fn with_priority(mut self, priority: i16) -> Self {
        self.priority = priority;
        self
    }

    /// Set exclusive flag.
    #[must_use]
    pub fn with_exclusive(mut self, exclusive: bool) -> Self {
        self.exclusive = exclusive;
        self
    }

    /// Set cooldown.
    #[must_use]
    pub fn with_cooldown(mut self, cooldown: u32) -> Self {
        self.cooldown = cooldown;
        self
    }

    /// Check if all conditions pass.
    #[must_use]
    pub fn conditions_pass(&self, ctx: &super::condition::ConditionContext) -> bool {
        self.conditions.iter().all(|c| c.evaluate(ctx))
    }

    /// Get chain target if this node chains to another.
    #[must_use]
    pub fn chain_targets(&self) -> Vec<u32> {
        self.actions
            .iter()
            .filter_map(|a| {
                if let BehaviorAction::ChainToNode { node_id } = a {
                    Some(*node_id)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Ordering key for deterministic sorting (priority descending, then ID ascending).
    #[must_use]
    fn sort_key(&self) -> (i16, u32) {
        (-self.priority, self.id.0)
    }

    /// Feed node data into a checksum builder.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "condition/action counts fit in u32"
    )]
    pub fn feed_checksum(&self, hasher: &mut crate::ChecksumBuilder) {
        hasher.feed_u32(self.id.0);

        if let Some(name) = &self.name {
            hasher.feed_u32(1);
            hasher.feed_str(name);
        } else {
            hasher.feed_u32(0);
        }

        self.trigger.feed_checksum(hasher);

        hasher.feed_u32(self.conditions.len() as u32);
        for cond in &self.conditions {
            cond.feed_checksum(hasher);
        }

        hasher.feed_u32(self.actions.len() as u32);
        for action in &self.actions {
            action.feed_checksum(hasher);
        }

        hasher.feed_i32(i32::from(self.priority));
        hasher.feed_u32(u32::from(self.enabled));
        hasher.feed_u32(u32::from(self.exclusive));
        hasher.feed_u32(self.cooldown);
    }
}

impl PartialOrd for BehaviorNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BehaviorNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

impl Eq for BehaviorNode {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behavior_graph::condition::ConditionContext;
    use crate::chunk::BlockId;

    #[test]
    fn node_creation() {
        let node = BehaviorNode::new(1)
            .with_name("test_node")
            .with_trigger(BehaviorTrigger::Use)
            .with_action(BehaviorAction::PlaySound { sound_id: 1 });

        assert_eq!(node.id, NodeId::new(1));
        assert_eq!(node.name, Some("test_node".into()));
        assert_eq!(node.actions.len(), 1);
    }

    #[test]
    fn node_conditions_pass() {
        let node = BehaviorNode::new(1)
            .with_condition(BehaviorCondition::Always)
            .with_condition(BehaviorCondition::Always);

        let ctx = ConditionContext::new();
        assert!(node.conditions_pass(&ctx));

        let node_fail = BehaviorNode::new(2)
            .with_condition(BehaviorCondition::Always)
            .with_condition(BehaviorCondition::Never);

        assert!(!node_fail.conditions_pass(&ctx));
    }

    #[test]
    fn node_chain_targets() {
        let node = BehaviorNode::new(1)
            .with_action(BehaviorAction::PlaySound { sound_id: 1 })
            .with_action(BehaviorAction::ChainToNode { node_id: 5 })
            .with_action(BehaviorAction::ChainToNode { node_id: 10 });

        let targets = node.chain_targets();
        assert_eq!(targets, vec![5, 10]);
    }

    #[test]
    fn node_ordering() {
        let high_priority = BehaviorNode::new(1).with_priority(10);
        let low_priority = BehaviorNode::new(2).with_priority(5);
        let same_priority_lower_id = BehaviorNode::new(1).with_priority(5);

        assert!(high_priority < low_priority);
        assert!(same_priority_lower_id < low_priority);
    }

    #[test]
    fn node_id_ordering() {
        assert!(NodeId::new(1) < NodeId::new(2));
        assert!(NodeId::new(100) > NodeId::new(50));
    }

    #[test]
    fn serde_round_trip() {
        let node = BehaviorNode::new(42)
            .with_name("water_extinguish")
            .with_trigger(BehaviorTrigger::FluidContact { fluid_kind: 0 })
            .with_condition(BehaviorCondition::LightLevel {
                op: super::super::condition::CompareOp::Lt,
                value: 10,
            })
            .with_action(BehaviorAction::TransformBlock {
                new_block: BlockId(0),
            })
            .with_priority(5)
            .with_cooldown(20);

        let json = serde_json::to_string(&node).unwrap();
        let recovered: BehaviorNode = serde_json::from_str(&json).unwrap();

        assert_eq!(node.id, recovered.id);
        assert_eq!(node.name, recovered.name);
        assert_eq!(node.priority, recovered.priority);
        assert_eq!(node.cooldown, recovered.cooldown);
    }
}
