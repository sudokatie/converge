//! Data-driven block behavior graph system.
//!
//! Provides declarative definition of block interactions and behaviors through
//! a directed graph of triggers, conditions, and actions. Supports deterministic
//! evaluation for multiplayer synchronization.
//!
//! # Overview
//!
//! - [`BehaviorTrigger`]: Events that can initiate behavior evaluation
//! - [`BehaviorCondition`]: Predicates that gate behavior execution
//! - [`BehaviorAction`]: Effects produced when behaviors activate
//! - [`BehaviorNode`]: Graph node combining trigger, conditions, and actions
//! - [`BehaviorGraph`]: Complete behavior definition for a block type
//! - [`GraphEvaluator`]: Deterministic evaluation engine
//!
//! # Determinism
//!
//! All graph evaluation is deterministic with stable ordering:
//! - Nodes ordered by ID
//! - Actions emitted in node order
//! - Checksums computed over ordered state
//!
//! # Example
//!
//! ```
//! use engine_world::behavior_graph::{
//!     BehaviorAction, BehaviorCondition, BehaviorGraph, BehaviorNode, BehaviorTrigger,
//!     GraphEvaluator, EvaluatorConfig, TriggerEvent,
//! };
//! use engine_world::chunk::BlockId;
//!
//! // Define a simple behavior: torch converts to off-torch when water contacts
//! let mut graph = BehaviorGraph::new(BlockId(100));
//! graph.add_node(BehaviorNode::new(1)
//!     .with_trigger(BehaviorTrigger::FluidContact { fluid_kind: 0 })
//!     .with_action(BehaviorAction::TransformBlock { new_block: BlockId(101) }));
//!
//! // Evaluate
//! let config = EvaluatorConfig::default();
//! let mut evaluator = GraphEvaluator::new(&mut graph, config);
//! let event = TriggerEvent::FluidContact { fluid_kind: 0, level: 0.5 };
//! let result = evaluator.evaluate(&event);
//! assert!(!result.actions.is_empty());
//! ```

mod action;
mod condition;
mod evaluator;
mod graph;
mod node;
mod trigger;

pub use action::{BehaviorAction, BehaviorEffect, EffectKind};
pub use condition::{BehaviorCondition, CompareOp};
pub use evaluator::{
    EvalResult, EvaluatorConfig, EvaluatorStats, GraphEvaluator, TriggerContext, TriggerEvent,
};
pub use graph::{BehaviorGraph, BlockFilter, GraphFingerprint};
pub use node::{BehaviorNode, NodeId};
pub use trigger::BehaviorTrigger;
