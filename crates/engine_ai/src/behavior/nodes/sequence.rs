//! Sequence node - runs children in order until one fails

use crate::behavior::blackboard::Blackboard;
use crate::behavior::tree::{BehaviorNode, NodeStatus};

/// Sequence node - runs each child in order until one fails
///
/// - Returns Success if all children succeed
/// - Returns Failure if any child fails
/// - Returns Running if current child is running
#[derive(Debug)]
pub struct Sequence {
    name: String,
    children: Vec<Box<dyn BehaviorNode>>,
    current_index: usize,
}

impl Sequence {
    /// Create a new sequence node
    pub fn new(name: impl Into<String>, children: Vec<Box<dyn BehaviorNode>>) -> Self {
        Self {
            name: name.into(),
            children,
            current_index: 0,
        }
    }

    /// Add a child to the sequence
    pub fn add_child(&mut self, child: Box<dyn BehaviorNode>) {
        self.children.push(child);
    }
}

impl BehaviorNode for Sequence {
    fn tick(&mut self, blackboard: &mut Blackboard) -> NodeStatus {
        while self.current_index < self.children.len() {
            let status = self.children[self.current_index].tick(blackboard);
            
            match status {
                NodeStatus::Success => {
                    self.current_index += 1;
                }
                NodeStatus::Running => {
                    return NodeStatus::Running;
                }
                NodeStatus::Failure => {
                    self.current_index = 0;
                    return NodeStatus::Failure;
                }
            }
        }

        // All children succeeded
        self.current_index = 0;
        NodeStatus::Success
    }

    fn reset(&mut self) {
        self.current_index = 0;
        for child in &mut self.children {
            child.reset();
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Reactive sequence - re-evaluates completed children each tick
#[derive(Debug)]
pub struct ReactiveSequence {
    name: String,
    children: Vec<Box<dyn BehaviorNode>>,
}

impl ReactiveSequence {
    pub fn new(name: impl Into<String>, children: Vec<Box<dyn BehaviorNode>>) -> Self {
        Self {
            name: name.into(),
            children,
        }
    }
}

impl BehaviorNode for ReactiveSequence {
    fn tick(&mut self, blackboard: &mut Blackboard) -> NodeStatus {
        for child in &mut self.children {
            let status = child.tick(blackboard);
            
            match status {
                NodeStatus::Success => {
                    // Continue to next child
                }
                NodeStatus::Running => {
                    return NodeStatus::Running;
                }
                NodeStatus::Failure => {
                    return NodeStatus::Failure;
                }
            }
        }

        NodeStatus::Success
    }

    fn reset(&mut self) {
        for child in &mut self.children {
            child.reset();
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Parallel node - runs all children simultaneously
#[derive(Debug)]
pub struct Parallel {
    name: String,
    children: Vec<Box<dyn BehaviorNode>>,
    /// Number of children that must succeed for the node to succeed
    success_threshold: usize,
    /// Number of children that must fail for the node to fail
    failure_threshold: usize,
}

impl Parallel {
    /// Create a parallel node that succeeds when all children succeed
    pub fn all(name: impl Into<String>, children: Vec<Box<dyn BehaviorNode>>) -> Self {
        let len = children.len();
        Self {
            name: name.into(),
            children,
            success_threshold: len,
            failure_threshold: 1,
        }
    }

    /// Create a parallel node that succeeds when any child succeeds
    pub fn any(name: impl Into<String>, children: Vec<Box<dyn BehaviorNode>>) -> Self {
        let len = children.len();
        Self {
            name: name.into(),
            children,
            success_threshold: 1,
            failure_threshold: len,
        }
    }

    /// Create a parallel node with custom thresholds
    pub fn with_thresholds(
        name: impl Into<String>,
        children: Vec<Box<dyn BehaviorNode>>,
        success_threshold: usize,
        failure_threshold: usize,
    ) -> Self {
        Self {
            name: name.into(),
            children,
            success_threshold,
            failure_threshold,
        }
    }
}

impl BehaviorNode for Parallel {
    fn tick(&mut self, blackboard: &mut Blackboard) -> NodeStatus {
        let mut success_count = 0;
        let mut failure_count = 0;
        let mut running_count = 0;

        for child in &mut self.children {
            match child.tick(blackboard) {
                NodeStatus::Success => success_count += 1,
                NodeStatus::Failure => failure_count += 1,
                NodeStatus::Running => running_count += 1,
            }
        }

        if success_count >= self.success_threshold {
            NodeStatus::Success
        } else if failure_count >= self.failure_threshold {
            NodeStatus::Failure
        } else if running_count > 0 {
            NodeStatus::Running
        } else {
            NodeStatus::Failure
        }
    }

    fn reset(&mut self) {
        for child in &mut self.children {
            child.reset();
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Memory sequence - remembers last running child across ticks
#[derive(Debug)]
pub struct MemorySequence {
    name: String,
    children: Vec<Box<dyn BehaviorNode>>,
    current_index: usize,
}

impl MemorySequence {
    pub fn new(name: impl Into<String>, children: Vec<Box<dyn BehaviorNode>>) -> Self {
        Self {
            name: name.into(),
            children,
            current_index: 0,
        }
    }
}

impl BehaviorNode for MemorySequence {
    fn tick(&mut self, blackboard: &mut Blackboard) -> NodeStatus {
        // Start from where we left off
        while self.current_index < self.children.len() {
            let status = self.children[self.current_index].tick(blackboard);
            
            match status {
                NodeStatus::Success => {
                    self.current_index += 1;
                }
                NodeStatus::Running => {
                    return NodeStatus::Running;
                }
                NodeStatus::Failure => {
                    self.current_index = 0;
                    return NodeStatus::Failure;
                }
            }
        }

        self.current_index = 0;
        NodeStatus::Success
    }

    fn reset(&mut self) {
        self.current_index = 0;
        for child in &mut self.children {
            child.reset();
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behavior::tree::ActionNode;

    #[test]
    fn test_sequence_all_succeed() {
        let children: Vec<Box<dyn BehaviorNode>> = vec![
            Box::new(ActionNode::new("s1", |_| NodeStatus::Success)),
            Box::new(ActionNode::new("s2", |_| NodeStatus::Success)),
            Box::new(ActionNode::new("s3", |_| NodeStatus::Success)),
        ];
        let mut sequence = Sequence::new("test", children);
        let mut bb = Blackboard::new();
        
        assert_eq!(sequence.tick(&mut bb), NodeStatus::Success);
    }

    #[test]
    fn test_sequence_fails_on_failure() {
        let children: Vec<Box<dyn BehaviorNode>> = vec![
            Box::new(ActionNode::new("s1", |_| NodeStatus::Success)),
            Box::new(ActionNode::new("fail", |_| NodeStatus::Failure)),
            Box::new(ActionNode::new("never_run", |_| panic!("Should not run"))),
        ];
        let mut sequence = Sequence::new("test", children);
        let mut bb = Blackboard::new();
        
        assert_eq!(sequence.tick(&mut bb), NodeStatus::Failure);
    }

    #[test]
    fn test_parallel_all() {
        let children: Vec<Box<dyn BehaviorNode>> = vec![
            Box::new(ActionNode::new("s1", |_| NodeStatus::Success)),
            Box::new(ActionNode::new("s2", |_| NodeStatus::Success)),
        ];
        let mut parallel = Parallel::all("test", children);
        let mut bb = Blackboard::new();
        
        assert_eq!(parallel.tick(&mut bb), NodeStatus::Success);
    }

    #[test]
    fn test_parallel_any() {
        let children: Vec<Box<dyn BehaviorNode>> = vec![
            Box::new(ActionNode::new("fail", |_| NodeStatus::Failure)),
            Box::new(ActionNode::new("success", |_| NodeStatus::Success)),
        ];
        let mut parallel = Parallel::any("test", children);
        let mut bb = Blackboard::new();
        
        assert_eq!(parallel.tick(&mut bb), NodeStatus::Success);
    }
}
