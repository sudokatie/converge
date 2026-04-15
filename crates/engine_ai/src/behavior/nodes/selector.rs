//! Selector node - tries children until one succeeds

use crate::behavior::blackboard::Blackboard;
use crate::behavior::tree::{BehaviorNode, NodeStatus};

/// Selector node - tries each child in order until one succeeds
/// 
/// - Returns Success if any child succeeds
/// - Returns Failure if all children fail
/// - Returns Running if current child is running
#[derive(Debug)]
pub struct Selector {
    name: String,
    children: Vec<Box<dyn BehaviorNode>>,
    current_index: usize,
}

impl Selector {
    /// Create a new selector node
    pub fn new(name: impl Into<String>, children: Vec<Box<dyn BehaviorNode>>) -> Self {
        Self {
            name: name.into(),
            children,
            current_index: 0,
        }
    }

    /// Add a child to the selector
    pub fn add_child(&mut self, child: Box<dyn BehaviorNode>) {
        self.children.push(child);
    }
}

impl BehaviorNode for Selector {
    fn tick(&mut self, blackboard: &mut Blackboard) -> NodeStatus {
        while self.current_index < self.children.len() {
            let status = self.children[self.current_index].tick(blackboard);
            
            match status {
                NodeStatus::Success => {
                    self.current_index = 0;
                    return NodeStatus::Success;
                }
                NodeStatus::Running => {
                    return NodeStatus::Running;
                }
                NodeStatus::Failure => {
                    self.current_index += 1;
                }
            }
        }

        // All children failed
        self.current_index = 0;
        NodeStatus::Failure
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

/// Reactive selector - re-evaluates higher priority children each tick
#[derive(Debug)]
pub struct ReactiveSelector {
    name: String,
    children: Vec<Box<dyn BehaviorNode>>,
    current_index: usize,
}

impl ReactiveSelector {
    pub fn new(name: impl Into<String>, children: Vec<Box<dyn BehaviorNode>>) -> Self {
        Self {
            name: name.into(),
            children,
            current_index: 0,
        }
    }
}

impl BehaviorNode for ReactiveSelector {
    fn tick(&mut self, blackboard: &mut Blackboard) -> NodeStatus {
        for (i, child) in self.children.iter_mut().enumerate() {
            let status = child.tick(blackboard);
            
            match status {
                NodeStatus::Success => {
                    // If a higher priority child succeeds, reset lower ones
                    if i < self.current_index {
                        for j in (i + 1)..=self.current_index {
                            if j < self.children.len() {
                                self.children[j].reset();
                            }
                        }
                    }
                    self.current_index = i;
                    return NodeStatus::Success;
                }
                NodeStatus::Running => {
                    // If a higher priority child is now running, reset lower ones
                    if i < self.current_index {
                        for j in (i + 1)..=self.current_index {
                            if j < self.children.len() {
                                self.children[j].reset();
                            }
                        }
                    }
                    self.current_index = i;
                    return NodeStatus::Running;
                }
                NodeStatus::Failure => {
                    // Continue to next child
                }
            }
        }

        self.current_index = 0;
        NodeStatus::Failure
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

/// Random selector - picks a random child to try first
#[derive(Debug)]
pub struct RandomSelector {
    name: String,
    children: Vec<Box<dyn BehaviorNode>>,
    order: Vec<usize>,
    current_index: usize,
    shuffled: bool,
}

impl RandomSelector {
    pub fn new(name: impl Into<String>, children: Vec<Box<dyn BehaviorNode>>) -> Self {
        let len = children.len();
        Self {
            name: name.into(),
            children,
            order: (0..len).collect(),
            current_index: 0,
            shuffled: false,
        }
    }

    fn shuffle(&mut self) {
        // Simple Fisher-Yates shuffle using a basic RNG
        let len = self.order.len();
        for i in (1..len).rev() {
            // Very simple pseudo-random swap
            let j = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos() as usize + i) % (i + 1);
            self.order.swap(i, j);
        }
        self.shuffled = true;
    }
}

impl BehaviorNode for RandomSelector {
    fn tick(&mut self, blackboard: &mut Blackboard) -> NodeStatus {
        if !self.shuffled {
            self.shuffle();
        }

        while self.current_index < self.order.len() {
            let child_index = self.order[self.current_index];
            let status = self.children[child_index].tick(blackboard);
            
            match status {
                NodeStatus::Success => {
                    self.current_index = 0;
                    self.shuffled = false;
                    return NodeStatus::Success;
                }
                NodeStatus::Running => {
                    return NodeStatus::Running;
                }
                NodeStatus::Failure => {
                    self.current_index += 1;
                }
            }
        }

        self.current_index = 0;
        self.shuffled = false;
        NodeStatus::Failure
    }

    fn reset(&mut self) {
        self.current_index = 0;
        self.shuffled = false;
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
    fn test_selector_success_on_first() {
        let children: Vec<Box<dyn BehaviorNode>> = vec![
            Box::new(ActionNode::new("success", |_| NodeStatus::Success)),
            Box::new(ActionNode::new("never_run", |_| panic!("Should not run"))),
        ];
        let mut selector = Selector::new("test", children);
        let mut bb = Blackboard::new();
        
        assert_eq!(selector.tick(&mut bb), NodeStatus::Success);
    }

    #[test]
    fn test_selector_tries_until_success() {
        let children: Vec<Box<dyn BehaviorNode>> = vec![
            Box::new(ActionNode::new("fail1", |_| NodeStatus::Failure)),
            Box::new(ActionNode::new("fail2", |_| NodeStatus::Failure)),
            Box::new(ActionNode::new("success", |_| NodeStatus::Success)),
        ];
        let mut selector = Selector::new("test", children);
        let mut bb = Blackboard::new();
        
        assert_eq!(selector.tick(&mut bb), NodeStatus::Success);
    }

    #[test]
    fn test_selector_all_fail() {
        let children: Vec<Box<dyn BehaviorNode>> = vec![
            Box::new(ActionNode::new("fail1", |_| NodeStatus::Failure)),
            Box::new(ActionNode::new("fail2", |_| NodeStatus::Failure)),
        ];
        let mut selector = Selector::new("test", children);
        let mut bb = Blackboard::new();
        
        assert_eq!(selector.tick(&mut bb), NodeStatus::Failure);
    }
}
