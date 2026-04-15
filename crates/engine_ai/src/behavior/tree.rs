//! Core behavior tree implementation

use std::fmt::Debug;

use super::blackboard::Blackboard;

/// Status returned by behavior tree nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeStatus {
    /// Node succeeded
    Success,
    /// Node failed
    Failure,
    /// Node is still running
    Running,
}

/// Trait for behavior tree nodes
pub trait BehaviorNode: Debug + Send + Sync {
    /// Execute this node
    fn tick(&mut self, blackboard: &mut Blackboard) -> NodeStatus;

    /// Reset this node to its initial state
    fn reset(&mut self);

    /// Get the name of this node (for debugging)
    fn name(&self) -> &str;
}

/// A complete behavior tree
#[derive(Debug)]
pub struct BehaviorTree {
    /// Root node of the tree
    root: Box<dyn BehaviorNode>,
    /// Shared blackboard for all nodes
    blackboard: Blackboard,
    /// Current status of the tree
    status: NodeStatus,
}

impl BehaviorTree {
    /// Create a new behavior tree with the given root node
    pub fn new(root: Box<dyn BehaviorNode>) -> Self {
        Self {
            root,
            blackboard: Blackboard::new(),
            status: NodeStatus::Success,
        }
    }

    /// Create a new behavior tree with an existing blackboard
    pub fn with_blackboard(root: Box<dyn BehaviorNode>, blackboard: Blackboard) -> Self {
        Self {
            root,
            blackboard,
            status: NodeStatus::Success,
        }
    }

    /// Execute one tick of the behavior tree
    pub fn tick(&mut self) -> NodeStatus {
        self.status = self.root.tick(&mut self.blackboard);
        self.status
    }

    /// Reset the entire tree
    pub fn reset(&mut self) {
        self.root.reset();
        self.status = NodeStatus::Success;
    }

    /// Get the current status
    pub fn status(&self) -> NodeStatus {
        self.status
    }

    /// Get a reference to the blackboard
    pub fn blackboard(&self) -> &Blackboard {
        &self.blackboard
    }

    /// Get a mutable reference to the blackboard
    pub fn blackboard_mut(&mut self) -> &mut Blackboard {
        &mut self.blackboard
    }
}

/// Action node - executes a single action
pub struct ActionNode<F>
where
    F: FnMut(&mut Blackboard) -> NodeStatus + Send + Sync,
{
    name: String,
    action: F,
}

impl<F> std::fmt::Debug for ActionNode<F>
where
    F: FnMut(&mut Blackboard) -> NodeStatus + Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActionNode")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl<F> ActionNode<F>
where
    F: FnMut(&mut Blackboard) -> NodeStatus + Send + Sync,
{
    pub fn new(name: impl Into<String>, action: F) -> Self {
        Self {
            name: name.into(),
            action,
        }
    }
}

impl<F> BehaviorNode for ActionNode<F>
where
    F: FnMut(&mut Blackboard) -> NodeStatus + Send + Sync,
{
    fn tick(&mut self, blackboard: &mut Blackboard) -> NodeStatus {
        (self.action)(blackboard)
    }

    fn reset(&mut self) {
        // Actions typically don't need reset
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Condition node - checks a condition
pub struct ConditionNode<F>
where
    F: Fn(&Blackboard) -> bool + Send + Sync,
{
    name: String,
    condition: F,
}

impl<F> std::fmt::Debug for ConditionNode<F>
where
    F: Fn(&Blackboard) -> bool + Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConditionNode")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl<F> ConditionNode<F>
where
    F: Fn(&Blackboard) -> bool + Send + Sync,
{
    pub fn new(name: impl Into<String>, condition: F) -> Self {
        Self {
            name: name.into(),
            condition,
        }
    }
}

impl<F> BehaviorNode for ConditionNode<F>
where
    F: Fn(&Blackboard) -> bool + Send + Sync,
{
    fn tick(&mut self, blackboard: &mut Blackboard) -> NodeStatus {
        if (self.condition)(blackboard) {
            NodeStatus::Success
        } else {
            NodeStatus::Failure
        }
    }

    fn reset(&mut self) {}

    fn name(&self) -> &str {
        &self.name
    }
}

/// Inverter decorator - inverts the result of its child
#[derive(Debug)]
pub struct Inverter {
    name: String,
    child: Box<dyn BehaviorNode>,
}

impl Inverter {
    pub fn new(name: impl Into<String>, child: Box<dyn BehaviorNode>) -> Self {
        Self {
            name: name.into(),
            child,
        }
    }
}

impl BehaviorNode for Inverter {
    fn tick(&mut self, blackboard: &mut Blackboard) -> NodeStatus {
        match self.child.tick(blackboard) {
            NodeStatus::Success => NodeStatus::Failure,
            NodeStatus::Failure => NodeStatus::Success,
            NodeStatus::Running => NodeStatus::Running,
        }
    }

    fn reset(&mut self) {
        self.child.reset();
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Repeater decorator - repeats its child a number of times
#[derive(Debug)]
pub struct Repeater {
    name: String,
    child: Box<dyn BehaviorNode>,
    max_count: Option<u32>,
    current_count: u32,
}

impl Repeater {
    /// Create a repeater that runs forever
    pub fn forever(name: impl Into<String>, child: Box<dyn BehaviorNode>) -> Self {
        Self {
            name: name.into(),
            child,
            max_count: None,
            current_count: 0,
        }
    }

    /// Create a repeater that runs a fixed number of times
    pub fn times(name: impl Into<String>, count: u32, child: Box<dyn BehaviorNode>) -> Self {
        Self {
            name: name.into(),
            child,
            max_count: Some(count),
            current_count: 0,
        }
    }
}

impl BehaviorNode for Repeater {
    fn tick(&mut self, blackboard: &mut Blackboard) -> NodeStatus {
        if let Some(max) = self.max_count {
            if self.current_count >= max {
                return NodeStatus::Success;
            }
        }

        match self.child.tick(blackboard) {
            NodeStatus::Success => {
                self.current_count += 1;
                self.child.reset();
                NodeStatus::Running
            }
            NodeStatus::Failure => NodeStatus::Failure,
            NodeStatus::Running => NodeStatus::Running,
        }
    }

    fn reset(&mut self) {
        self.current_count = 0;
        self.child.reset();
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Succeeder decorator - always returns success
#[derive(Debug)]
pub struct Succeeder {
    name: String,
    child: Box<dyn BehaviorNode>,
}

impl Succeeder {
    pub fn new(name: impl Into<String>, child: Box<dyn BehaviorNode>) -> Self {
        Self {
            name: name.into(),
            child,
        }
    }
}

impl BehaviorNode for Succeeder {
    fn tick(&mut self, blackboard: &mut Blackboard) -> NodeStatus {
        match self.child.tick(blackboard) {
            NodeStatus::Running => NodeStatus::Running,
            _ => NodeStatus::Success,
        }
    }

    fn reset(&mut self) {
        self.child.reset();
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_node() {
        let mut action = ActionNode::new("test", |_| NodeStatus::Success);
        let mut bb = Blackboard::new();
        assert_eq!(action.tick(&mut bb), NodeStatus::Success);
    }

    #[test]
    fn test_condition_node() {
        let mut cond = ConditionNode::new("test", |bb| {
            bb.get::<bool>("flag").copied().unwrap_or(false)
        });
        let mut bb = Blackboard::new();
        
        assert_eq!(cond.tick(&mut bb), NodeStatus::Failure);
        
        bb.set("flag", true);
        assert_eq!(cond.tick(&mut bb), NodeStatus::Success);
    }

    #[test]
    fn test_inverter() {
        let action = ActionNode::new("success", |_| NodeStatus::Success);
        let mut inverter = Inverter::new("invert", Box::new(action));
        let mut bb = Blackboard::new();
        
        assert_eq!(inverter.tick(&mut bb), NodeStatus::Failure);
    }
}
