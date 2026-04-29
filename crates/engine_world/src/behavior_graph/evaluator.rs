//! Deterministic evaluation engine for behavior graphs.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::ChecksumBuilder;
use crate::chunk::BlockId;
use crate::environment::HazardKind;

use super::action::{BehaviorAction, BehaviorEffect, EffectData, EffectKind};
use super::condition::ConditionContext;
use super::graph::BehaviorGraph;
use super::node::NodeId;
use super::trigger::TriggerEventKind;

/// Trigger event with full context for evaluation.
#[derive(Clone, Debug, PartialEq)]
pub enum TriggerEvent {
    /// Player use interaction.
    Use,
    /// Mining event.
    Mine,
    /// Placement event.
    Place,
    /// Neighbor change event.
    NeighborChanged {
        direction: (i8, i8, i8),
        new_block: BlockId,
    },
    /// Tick event.
    Tick {
        current_tick: u64,
        ticks_since_last: u32,
    },
    /// Hazard exposure event.
    HazardExposure { kind: HazardKind, intensity: f32 },
    /// Fluid contact event.
    FluidContact { fluid_kind: u8, level: f32 },
    /// Signal received event.
    SignalReceived { strength: i32 },
    /// Entity collision event.
    EntityCollision { kind: u32 },
    /// Random tick event.
    RandomTick { activated: bool },
}

impl TriggerEvent {
    /// Convert to the internal event kind for trigger matching.
    #[must_use]
    pub fn to_event_kind(&self) -> TriggerEventKind {
        match self {
            Self::Use => TriggerEventKind::Use,
            Self::Mine => TriggerEventKind::Mine,
            Self::Place => TriggerEventKind::Place,
            Self::NeighborChanged {
                direction,
                new_block,
            } => TriggerEventKind::NeighborChanged {
                direction: *direction,
                new_block: *new_block,
            },
            Self::Tick {
                current_tick,
                ticks_since_last,
            } => TriggerEventKind::Tick {
                current_tick: *current_tick,
                ticks_since_last: *ticks_since_last,
            },
            Self::HazardExposure { kind, intensity } => TriggerEventKind::HazardExposure {
                kind: *kind,
                intensity: *intensity,
            },
            Self::FluidContact { fluid_kind, level } => TriggerEventKind::FluidContact {
                kind: *fluid_kind,
                level: *level,
            },
            Self::SignalReceived { strength } => TriggerEventKind::SignalReceived {
                strength: *strength,
            },
            Self::EntityCollision { kind } => TriggerEventKind::EntityCollision { kind: *kind },
            Self::RandomTick { activated } => TriggerEventKind::RandomTick {
                activated: *activated,
            },
        }
    }
}

/// Context provided to the evaluator for each evaluation.
#[derive(Clone, Debug, Default)]
pub struct TriggerContext {
    /// Condition context for evaluating node conditions.
    pub conditions: ConditionContext,
    /// Block position (for effects that need it).
    pub position: (i32, i32, i32),
    /// Current world tick.
    pub world_tick: u64,
    /// Deterministic seed for this evaluation.
    pub seed: u64,
}

impl TriggerContext {
    /// Create a new trigger context.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the position.
    #[must_use]
    pub fn with_position(mut self, x: i32, y: i32, z: i32) -> Self {
        self.position = (x, y, z);
        self
    }

    /// Set the world tick.
    #[must_use]
    pub fn with_world_tick(mut self, tick: u64) -> Self {
        self.world_tick = tick;
        self
    }

    /// Set the seed.
    #[must_use]
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }
}

/// Configuration for the graph evaluator.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvaluatorConfig {
    /// Maximum chain depth to prevent infinite loops.
    pub max_chain_depth: u32,
    /// Whether to collect stats during evaluation.
    pub collect_stats: bool,
    /// Maximum actions per evaluation.
    pub max_actions: usize,
}

impl Default for EvaluatorConfig {
    fn default() -> Self {
        Self {
            max_chain_depth: 16,
            collect_stats: false,
            max_actions: 256,
        }
    }
}

/// Statistics from graph evaluation.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EvaluatorStats {
    /// Number of nodes evaluated.
    pub nodes_evaluated: u32,
    /// Number of nodes that fired (conditions passed).
    pub nodes_fired: u32,
    /// Number of actions emitted.
    pub actions_emitted: u32,
    /// Maximum chain depth reached.
    pub max_chain_depth: u32,
    /// Nodes skipped due to cooldown.
    pub cooldown_skips: u32,
}

/// Result of evaluating a behavior graph.
#[derive(Clone, Debug)]
pub struct EvalResult {
    /// Actions to execute (in order).
    pub actions: Vec<BehaviorAction>,
    /// Effects produced (with metadata).
    pub effects: Vec<BehaviorEffect>,
    /// Node IDs that fired.
    pub fired_nodes: Vec<NodeId>,
    /// Whether evaluation was truncated due to limits.
    pub truncated: bool,
    /// Evaluation stats (if enabled).
    pub stats: Option<EvaluatorStats>,
}

impl EvalResult {
    fn new() -> Self {
        Self {
            actions: Vec::new(),
            effects: Vec::new(),
            fired_nodes: Vec::new(),
            truncated: false,
            stats: None,
        }
    }

    /// Feed result data into a checksum builder.
    #[expect(clippy::cast_possible_truncation, reason = "lengths fit in u32")]
    pub fn feed_checksum(&self, hasher: &mut ChecksumBuilder) {
        hasher.feed_u32(self.actions.len() as u32);
        for action in &self.actions {
            action.feed_checksum(hasher);
        }
        hasher.feed_u32(self.fired_nodes.len() as u32);
        for node in &self.fired_nodes {
            hasher.feed_u32(node.raw());
        }
        hasher.feed_u32(u32::from(self.truncated));
    }
}

/// Deterministic evaluation engine for behavior graphs.
pub struct GraphEvaluator<'a> {
    graph: &'a mut BehaviorGraph,
    config: EvaluatorConfig,
    cooldowns: HashMap<NodeId, u64>,
}

impl<'a> GraphEvaluator<'a> {
    /// Create a new evaluator for a graph.
    pub fn new(graph: &'a mut BehaviorGraph, config: EvaluatorConfig) -> Self {
        Self {
            graph,
            config,
            cooldowns: HashMap::new(),
        }
    }

    /// Evaluate the graph against a trigger event.
    pub fn evaluate(&mut self, event: &TriggerEvent) -> EvalResult {
        self.evaluate_with_context(event, &TriggerContext::default())
    }

    /// Evaluate with full context.
    pub fn evaluate_with_context(
        &mut self,
        event: &TriggerEvent,
        context: &TriggerContext,
    ) -> EvalResult {
        let mut result = EvalResult::new();
        let mut stats = if self.config.collect_stats {
            Some(EvaluatorStats::default())
        } else {
            None
        };
        let mut fired_this_eval: HashSet<NodeId> = HashSet::new();

        let event_kind = event.to_event_kind();
        let mut chain_queue: Vec<(NodeId, u32)> = Vec::new();

        for node in self.graph.nodes_ordered() {
            if !node.enabled {
                continue;
            }

            if let Some(stats) = &mut stats {
                stats.nodes_evaluated += 1;
            }

            if let Some(&last_fire) = self.cooldowns.get(&node.id)
                && context.world_tick < last_fire + u64::from(node.cooldown)
            {
                if let Some(stats) = &mut stats {
                    stats.cooldown_skips += 1;
                }
                continue;
            }

            if !node.trigger.matches(&event_kind) {
                continue;
            }

            if !node.conditions_pass(&context.conditions) {
                continue;
            }

            if fired_this_eval.contains(&node.id) {
                continue;
            }

            if let Some(stats) = &mut stats {
                stats.nodes_fired += 1;
            }

            fired_this_eval.insert(node.id);
            result.fired_nodes.push(node.id);

            if node.cooldown > 0 {
                self.cooldowns.insert(node.id, context.world_tick);
            }

            for action in &node.actions {
                if result.actions.len() >= self.config.max_actions {
                    result.truncated = true;
                    break;
                }

                if let BehaviorAction::ChainToNode { node_id } = action {
                    chain_queue.push((NodeId::new(*node_id), 1));
                } else {
                    result.actions.push(action.clone());
                    if let Some(effect) = action_to_effect(action, node.id.raw()) {
                        result.effects.push(effect);
                    }
                    if let Some(stats) = &mut stats {
                        stats.actions_emitted += 1;
                    }
                }
            }

            if result.truncated || node.exclusive {
                break;
            }
        }

        self.process_chain_queue(
            &mut chain_queue,
            context,
            &mut result,
            &mut stats,
            &mut fired_this_eval,
        );

        result.stats = stats;
        result
    }

    fn process_chain_queue(
        &mut self,
        queue: &mut Vec<(NodeId, u32)>,
        context: &TriggerContext,
        result: &mut EvalResult,
        stats: &mut Option<EvaluatorStats>,
        fired_this_eval: &mut HashSet<NodeId>,
    ) {
        while let Some((node_id, depth)) = queue.pop() {
            if depth > self.config.max_chain_depth {
                if let Some(stats) = stats {
                    stats.max_chain_depth = stats.max_chain_depth.max(depth);
                }
                continue;
            }

            if fired_this_eval.contains(&node_id) {
                continue;
            }

            let Some(node) = self.graph.get_node(node_id) else {
                continue;
            };

            if !node.enabled {
                continue;
            }

            if !node.conditions_pass(&context.conditions) {
                continue;
            }

            if let Some(stats) = stats {
                stats.nodes_fired += 1;
                stats.max_chain_depth = stats.max_chain_depth.max(depth);
            }

            fired_this_eval.insert(node_id);
            result.fired_nodes.push(node_id);

            let actions: Vec<_> = node.actions.clone();
            for action in &actions {
                if result.actions.len() >= self.config.max_actions {
                    result.truncated = true;
                    return;
                }

                if let BehaviorAction::ChainToNode {
                    node_id: chain_target,
                } = action
                {
                    queue.push((NodeId::new(*chain_target), depth + 1));
                } else {
                    result.actions.push(action.clone());
                    if let Some(effect) = action_to_effect(action, node_id.raw()) {
                        result.effects.push(effect);
                    }
                    if let Some(stats) = stats {
                        stats.actions_emitted += 1;
                    }
                }
            }
        }
    }

    /// Reset all cooldowns.
    pub fn reset_cooldowns(&mut self) {
        self.cooldowns.clear();
    }

    /// Get current cooldown state for a node.
    #[must_use]
    pub fn get_cooldown(&self, node_id: NodeId) -> Option<u64> {
        self.cooldowns.get(&node_id).copied()
    }
}

fn action_to_effect(action: &BehaviorAction, source_node: u32) -> Option<BehaviorEffect> {
    let (kind, data) = match action {
        BehaviorAction::TransformBlock { new_block } => {
            (EffectKind::BlockChanged, EffectData::Block(*new_block))
        }
        BehaviorAction::DestroyBlock => (EffectKind::BlockDestroyed, EffectData::None),
        BehaviorAction::DropItem { item_id, count } => (
            EffectKind::ItemDropped,
            EffectData::ItemDrop {
                item_id: *item_id,
                count: *count,
            },
        ),
        BehaviorAction::EmitParticle { particle_id, count } => (
            EffectKind::ParticleEmitted,
            EffectData::Particle {
                particle_id: *particle_id,
                count: *count,
            },
        ),
        BehaviorAction::PlaySound { sound_id } => {
            (EffectKind::SoundPlayed, EffectData::Sound(*sound_id))
        }
        BehaviorAction::EmitSignal { strength } => {
            (EffectKind::SignalEmitted, EffectData::Signal(*strength))
        }
        BehaviorAction::DamageEntities { radius, damage } => (
            EffectKind::EntitiesDamaged,
            EffectData::Damage {
                radius: *radius,
                damage: *damage,
            },
        ),
        BehaviorAction::SpawnHazard { kind, intensity } => (
            EffectKind::HazardSpawned,
            EffectData::Hazard {
                kind: *kind,
                intensity: *intensity,
            },
        ),
        BehaviorAction::SpawnFluid { kind, volume } => (
            EffectKind::FluidSpawned,
            EffectData::Fluid {
                kind: *kind,
                volume: *volume,
            },
        ),
        BehaviorAction::SetMetadata { key, value } => (
            EffectKind::MetadataChanged,
            EffectData::Metadata {
                key: key.clone(),
                value: *value,
            },
        ),
        BehaviorAction::IncrementMetadata { key, amount, .. } => (
            EffectKind::MetadataChanged,
            EffectData::Metadata {
                key: key.clone(),
                value: *amount,
            },
        ),
        BehaviorAction::NotifyNeighbors => (EffectKind::NeighborsNotified, EffectData::None),
        BehaviorAction::ScheduleTick { delay } => {
            (EffectKind::TickScheduled, EffectData::TickDelay(*delay))
        }
        BehaviorAction::ChainToNode { .. } => return None,
        BehaviorAction::ApplyEntityEffect {
            effect_id,
            duration,
        } => (
            EffectKind::EntityEffectApplied,
            EffectData::EntityEffect {
                effect_id: *effect_id,
                duration: *duration,
            },
        ),
        BehaviorAction::EmitEvent {
            event_kind,
            payload,
        } => (
            EffectKind::EventEmitted,
            EffectData::Event {
                event_kind: *event_kind,
                payload: payload.clone(),
            },
        ),
    };

    Some(BehaviorEffect {
        kind,
        source_node,
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FluidKind;
    use crate::behavior_graph::{BehaviorCondition, BehaviorNode, BehaviorTrigger};

    #[test]
    fn evaluator_basic_trigger() {
        let mut graph = BehaviorGraph::new(BlockId(100));
        graph.add_node(
            BehaviorNode::new(1)
                .with_trigger(BehaviorTrigger::Use)
                .with_action(BehaviorAction::PlaySound { sound_id: 42 }),
        );

        let config = EvaluatorConfig::default();
        let mut evaluator = GraphEvaluator::new(&mut graph, config);
        let result = evaluator.evaluate(&TriggerEvent::Use);

        assert_eq!(result.actions.len(), 1);
        assert!(matches!(
            result.actions[0],
            BehaviorAction::PlaySound { sound_id: 42 }
        ));
    }

    #[test]
    fn evaluator_no_match() {
        let mut graph = BehaviorGraph::new(BlockId(100));
        graph.add_node(
            BehaviorNode::new(1)
                .with_trigger(BehaviorTrigger::Use)
                .with_action(BehaviorAction::PlaySound { sound_id: 1 }),
        );

        let config = EvaluatorConfig::default();
        let mut evaluator = GraphEvaluator::new(&mut graph, config);
        let result = evaluator.evaluate(&TriggerEvent::Mine);

        assert!(result.actions.is_empty());
    }

    #[test]
    fn evaluator_condition_blocks() {
        let mut graph = BehaviorGraph::new(BlockId(100));
        graph.add_node(
            BehaviorNode::new(1)
                .with_trigger(BehaviorTrigger::Use)
                .with_condition(BehaviorCondition::Never)
                .with_action(BehaviorAction::PlaySound { sound_id: 1 }),
        );

        let config = EvaluatorConfig::default();
        let mut evaluator = GraphEvaluator::new(&mut graph, config);
        let result = evaluator.evaluate(&TriggerEvent::Use);

        assert!(result.actions.is_empty());
    }

    #[test]
    fn evaluator_fluid_contact() {
        let mut graph = BehaviorGraph::new(BlockId(100));
        graph.add_node(
            BehaviorNode::new(1)
                .with_trigger(BehaviorTrigger::FluidContact { fluid_kind: 0 })
                .with_action(BehaviorAction::TransformBlock {
                    new_block: BlockId(101),
                }),
        );

        let config = EvaluatorConfig::default();
        let mut evaluator = GraphEvaluator::new(&mut graph, config);
        let event = TriggerEvent::FluidContact {
            fluid_kind: 0,
            level: 0.5,
        };
        let result = evaluator.evaluate(&event);

        assert!(!result.actions.is_empty());
        assert!(matches!(
            result.actions[0],
            BehaviorAction::TransformBlock {
                new_block: BlockId(101)
            }
        ));
    }

    #[test]
    fn evaluator_exclusive_stops_others() {
        let mut graph = BehaviorGraph::new(BlockId(100));
        graph.add_node(
            BehaviorNode::new(1)
                .with_trigger(BehaviorTrigger::Use)
                .with_priority(10)
                .with_exclusive(true)
                .with_action(BehaviorAction::PlaySound { sound_id: 1 }),
        );
        graph.add_node(
            BehaviorNode::new(2)
                .with_trigger(BehaviorTrigger::Use)
                .with_priority(5)
                .with_action(BehaviorAction::PlaySound { sound_id: 2 }),
        );

        let config = EvaluatorConfig::default();
        let mut evaluator = GraphEvaluator::new(&mut graph, config);
        let result = evaluator.evaluate(&TriggerEvent::Use);

        assert_eq!(result.actions.len(), 1);
        assert!(matches!(
            result.actions[0],
            BehaviorAction::PlaySound { sound_id: 1 }
        ));
    }

    #[test]
    fn evaluator_chain_to_node() {
        let mut graph = BehaviorGraph::new(BlockId(100));
        graph.add_node(
            BehaviorNode::new(1)
                .with_trigger(BehaviorTrigger::Use)
                .with_action(BehaviorAction::PlaySound { sound_id: 1 })
                .with_action(BehaviorAction::ChainToNode { node_id: 2 }),
        );
        graph.add_node(
            BehaviorNode::new(2)
                .with_trigger(BehaviorTrigger::Use)
                .with_action(BehaviorAction::PlaySound { sound_id: 2 }),
        );

        let config = EvaluatorConfig::default();
        let mut evaluator = GraphEvaluator::new(&mut graph, config);
        let result = evaluator.evaluate(&TriggerEvent::Use);

        assert_eq!(result.actions.len(), 2);
    }

    #[test]
    fn evaluator_max_chain_depth() {
        let mut graph = BehaviorGraph::new(BlockId(100));
        for i in 1..=20 {
            graph.add_node(
                BehaviorNode::new(i)
                    .with_trigger(BehaviorTrigger::Use)
                    .with_action(BehaviorAction::ChainToNode { node_id: i + 1 }),
            );
        }
        graph.add_node(
            BehaviorNode::new(21)
                .with_trigger(BehaviorTrigger::Use)
                .with_action(BehaviorAction::PlaySound { sound_id: 99 }),
        );

        let config = EvaluatorConfig {
            max_chain_depth: 5,
            collect_stats: true,
            ..Default::default()
        };
        let mut evaluator = GraphEvaluator::new(&mut graph, config);
        let result = evaluator.evaluate(&TriggerEvent::Use);

        assert!(result.stats.is_some());
        assert!(result.stats.as_ref().unwrap().max_chain_depth <= 6);
    }

    #[test]
    fn evaluator_cooldown() {
        let mut graph = BehaviorGraph::new(BlockId(100));
        graph.add_node(
            BehaviorNode::new(1)
                .with_trigger(BehaviorTrigger::Use)
                .with_cooldown(10)
                .with_action(BehaviorAction::PlaySound { sound_id: 1 }),
        );

        let config = EvaluatorConfig::default();
        let mut evaluator = GraphEvaluator::new(&mut graph, config);

        let ctx1 = TriggerContext::new().with_world_tick(0);
        let result1 = evaluator.evaluate_with_context(&TriggerEvent::Use, &ctx1);
        assert_eq!(result1.actions.len(), 1);

        let ctx2 = TriggerContext::new().with_world_tick(5);
        let result2 = evaluator.evaluate_with_context(&TriggerEvent::Use, &ctx2);
        assert!(result2.actions.is_empty());

        let ctx3 = TriggerContext::new().with_world_tick(15);
        let result3 = evaluator.evaluate_with_context(&TriggerEvent::Use, &ctx3);
        assert_eq!(result3.actions.len(), 1);
    }

    #[test]
    fn evaluator_stats() {
        let mut graph = BehaviorGraph::new(BlockId(100));
        graph.add_node(
            BehaviorNode::new(1)
                .with_trigger(BehaviorTrigger::Use)
                .with_action(BehaviorAction::PlaySound { sound_id: 1 })
                .with_action(BehaviorAction::EmitSignal { strength: 5 }),
        );

        let config = EvaluatorConfig {
            collect_stats: true,
            ..Default::default()
        };
        let mut evaluator = GraphEvaluator::new(&mut graph, config);
        let result = evaluator.evaluate(&TriggerEvent::Use);

        let stats = result.stats.unwrap();
        assert_eq!(stats.nodes_evaluated, 1);
        assert_eq!(stats.nodes_fired, 1);
        assert_eq!(stats.actions_emitted, 2);
    }

    #[test]
    #[expect(clippy::cast_possible_truncation)]
    fn trigger_event_conversion() {
        let event = TriggerEvent::FluidContact {
            fluid_kind: FluidKind::Water.as_index() as u8,
            level: 0.75,
        };
        let kind = event.to_event_kind();

        assert!(matches!(kind, TriggerEventKind::FluidContact { .. }));
    }

    #[test]
    fn trigger_context_builder() {
        let ctx = TriggerContext::new()
            .with_position(10, 20, 30)
            .with_world_tick(1000)
            .with_seed(42);

        assert_eq!(ctx.position, (10, 20, 30));
        assert_eq!(ctx.world_tick, 1000);
        assert_eq!(ctx.seed, 42);
    }
}
