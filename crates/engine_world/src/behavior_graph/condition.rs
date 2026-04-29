//! Conditions for gating block behavior execution.

use serde::{Deserialize, Serialize};

use crate::chunk::BlockId;
use crate::environment::{FluidKind, HazardKind};

/// Comparison operators for condition evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompareOp {
    /// Equal to
    Eq,
    /// Not equal to
    Ne,
    /// Less than
    Lt,
    /// Less than or equal
    Le,
    /// Greater than
    Gt,
    /// Greater than or equal
    Ge,
}

impl CompareOp {
    /// Apply this comparison to two i32 values.
    #[must_use]
    pub const fn compare_i32(self, a: i32, b: i32) -> bool {
        match self {
            Self::Eq => a == b,
            Self::Ne => a != b,
            Self::Lt => a < b,
            Self::Le => a <= b,
            Self::Gt => a > b,
            Self::Ge => a >= b,
        }
    }

    /// Apply this comparison to two f32 values.
    #[must_use]
    pub fn compare_f32(self, a: f32, b: f32) -> bool {
        match self {
            Self::Eq => (a - b).abs() < f32::EPSILON,
            Self::Ne => (a - b).abs() >= f32::EPSILON,
            Self::Lt => a < b,
            Self::Le => a <= b,
            Self::Gt => a > b,
            Self::Ge => a >= b,
        }
    }

    /// Get discriminant for checksums.
    #[must_use]
    pub const fn discriminant(self) -> u8 {
        match self {
            Self::Eq => 0,
            Self::Ne => 1,
            Self::Lt => 2,
            Self::Le => 3,
            Self::Gt => 4,
            Self::Ge => 5,
        }
    }
}

/// Predicates that must be satisfied for behavior to execute.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BehaviorCondition {
    /// Always true (no-op condition).
    Always,

    /// Never true (disables the node).
    Never,

    /// Logical AND of multiple conditions.
    And(Vec<BehaviorCondition>),

    /// Logical OR of multiple conditions.
    Or(Vec<BehaviorCondition>),

    /// Logical NOT of a condition.
    Not(Box<BehaviorCondition>),

    /// Check a neighbor block.
    NeighborIs {
        /// Offset from current block (-1, 0, 1 for each axis).
        offset: (i8, i8, i8),
        /// Required block type.
        block: BlockId,
    },

    /// Check if neighbor is solid.
    NeighborSolid { offset: (i8, i8, i8), solid: bool },

    /// Check light level at block.
    LightLevel { op: CompareOp, value: u8 },

    /// Check hazard presence/intensity.
    HazardLevel {
        kind: HazardKind,
        op: CompareOp,
        value: f32,
    },

    /// Check fluid presence/level.
    FluidLevel {
        kind: FluidKind,
        op: CompareOp,
        value: f32,
    },

    /// Check signal strength.
    SignalStrength { op: CompareOp, value: i32 },

    /// Check random value (deterministic with seed).
    RandomChance { probability: f32 },

    /// Check block metadata value.
    MetadataValue {
        key: String,
        op: CompareOp,
        value: i32,
    },

    /// Check time of day (0-23999 ticks).
    TimeOfDay { min_tick: u32, max_tick: u32 },

    /// Check if block has support below.
    HasSupportBelow,

    /// Check block age/ticks since placement.
    BlockAge { op: CompareOp, ticks: u64 },
}

impl BehaviorCondition {
    /// Evaluate this condition against the provided context.
    #[must_use]
    pub fn evaluate(&self, ctx: &ConditionContext) -> bool {
        match self {
            Self::Always => true,
            Self::Never => false,

            Self::And(conditions) => conditions.iter().all(|c| c.evaluate(ctx)),

            Self::Or(conditions) => conditions.iter().any(|c| c.evaluate(ctx)),

            Self::Not(condition) => !condition.evaluate(ctx),

            Self::NeighborIs { offset, block } => {
                ctx.get_neighbor(*offset).is_some_and(|b| b == *block)
            }

            Self::NeighborSolid { offset, solid } => {
                ctx.is_neighbor_solid(*offset).is_some_and(|s| s == *solid)
            }

            Self::LightLevel { op, value } => {
                op.compare_i32(i32::from(ctx.light_level), i32::from(*value))
            }

            Self::HazardLevel { kind, op, value } => {
                let level = ctx.get_hazard_level(*kind);
                op.compare_f32(level, *value)
            }

            Self::FluidLevel { kind, op, value } => {
                let level = ctx.get_fluid_level(*kind);
                op.compare_f32(level, *value)
            }

            Self::SignalStrength { op, value } => op.compare_i32(ctx.signal_strength, *value),

            Self::RandomChance { probability } => ctx.random_value < *probability,

            Self::MetadataValue { key, op, value } => {
                let meta = ctx.get_metadata(key);
                op.compare_i32(meta, *value)
            }

            Self::TimeOfDay { min_tick, max_tick } => {
                let time = ctx.time_of_day;
                if min_tick <= max_tick {
                    time >= *min_tick && time <= *max_tick
                } else {
                    time >= *min_tick || time <= *max_tick
                }
            }

            Self::HasSupportBelow => ctx.has_support_below,

            Self::BlockAge { op, ticks } => op.compare_i32(
                ctx.block_age.try_into().unwrap_or(i32::MAX),
                (*ticks).try_into().unwrap_or(i32::MAX),
            ),
        }
    }

    /// Get the discriminant for deterministic ordering.
    #[must_use]
    pub const fn discriminant(&self) -> u8 {
        match self {
            Self::Always => 0,
            Self::Never => 1,
            Self::And(_) => 2,
            Self::Or(_) => 3,
            Self::Not(_) => 4,
            Self::NeighborIs { .. } => 5,
            Self::NeighborSolid { .. } => 6,
            Self::LightLevel { .. } => 7,
            Self::HazardLevel { .. } => 8,
            Self::FluidLevel { .. } => 9,
            Self::SignalStrength { .. } => 10,
            Self::RandomChance { .. } => 11,
            Self::MetadataValue { .. } => 12,
            Self::TimeOfDay { .. } => 13,
            Self::HasSupportBelow => 14,
            Self::BlockAge { .. } => 15,
        }
    }

    /// Feed condition data into a checksum builder.
    #[expect(clippy::cast_possible_truncation, reason = "enum indices fit in u32")]
    pub fn feed_checksum(&self, hasher: &mut crate::ChecksumBuilder) {
        hasher.feed_u32(u32::from(self.discriminant()));

        match self {
            Self::Always | Self::Never | Self::HasSupportBelow => {}

            Self::And(conditions) | Self::Or(conditions) => {
                hasher.feed_u32(conditions.len() as u32);
                for c in conditions {
                    c.feed_checksum(hasher);
                }
            }

            Self::Not(condition) => {
                condition.feed_checksum(hasher);
            }

            Self::NeighborIs { offset, block } => {
                hasher.feed_i32(i32::from(offset.0));
                hasher.feed_i32(i32::from(offset.1));
                hasher.feed_i32(i32::from(offset.2));
                hasher.feed_u32(u32::from(block.raw()));
            }

            Self::NeighborSolid { offset, solid } => {
                hasher.feed_i32(i32::from(offset.0));
                hasher.feed_i32(i32::from(offset.1));
                hasher.feed_i32(i32::from(offset.2));
                hasher.feed_u32(u32::from(*solid));
            }

            Self::LightLevel { op, value } => {
                hasher.feed_u32(u32::from(op.discriminant()));
                hasher.feed_u32(u32::from(*value));
            }

            Self::HazardLevel { kind, op, value } => {
                hasher.feed_u32(kind.as_index() as u32);
                hasher.feed_u32(u32::from(op.discriminant()));
                hasher.feed_f32(*value);
            }

            Self::FluidLevel { kind, op, value } => {
                hasher.feed_u32(kind.as_index() as u32);
                hasher.feed_u32(u32::from(op.discriminant()));
                hasher.feed_f32(*value);
            }

            Self::SignalStrength { op, value } => {
                hasher.feed_u32(u32::from(op.discriminant()));
                hasher.feed_i32(*value);
            }

            Self::RandomChance { probability } => {
                hasher.feed_f32(*probability);
            }

            Self::MetadataValue { key, op, value } => {
                hasher.feed_str(key);
                hasher.feed_u32(u32::from(op.discriminant()));
                hasher.feed_i32(*value);
            }

            Self::TimeOfDay { min_tick, max_tick } => {
                hasher.feed_u32(*min_tick);
                hasher.feed_u32(*max_tick);
            }

            Self::BlockAge { op, ticks } => {
                hasher.feed_u32(u32::from(op.discriminant()));
                hasher.feed_u64(*ticks);
            }
        }
    }
}

/// Context data for condition evaluation.
#[derive(Clone, Debug)]
#[expect(clippy::type_complexity)]
pub struct ConditionContext {
    /// Current light level (0-15).
    pub light_level: u8,
    /// Current signal strength.
    pub signal_strength: i32,
    /// Deterministic random value for this evaluation (0.0-1.0).
    pub random_value: f32,
    /// Current time of day in ticks (0-23999).
    pub time_of_day: u32,
    /// Whether block has structural support below.
    pub has_support_below: bool,
    /// Ticks since block was placed.
    pub block_age: u64,
    /// Hazard levels by kind.
    hazard_levels: [f32; 6],
    /// Fluid levels by kind.
    fluid_levels: [f32; 4],
    /// Neighbor block lookup callback.
    neighbor_blocks: [(i8, i8, i8, Option<BlockId>, Option<bool>); 26],
    /// Block metadata.
    metadata: Vec<(String, i32)>,
}

impl ConditionContext {
    /// Create a new empty context.
    #[must_use]
    pub fn new() -> Self {
        Self {
            light_level: 15,
            signal_strength: 0,
            random_value: 0.0,
            time_of_day: 0,
            has_support_below: true,
            block_age: 0,
            hazard_levels: [0.0; 6],
            fluid_levels: [0.0; 4],
            neighbor_blocks: Self::empty_neighbors(),
            metadata: Vec::new(),
        }
    }

    #[expect(clippy::type_complexity)]
    fn empty_neighbors() -> [(i8, i8, i8, Option<BlockId>, Option<bool>); 26] {
        [
            (-1, -1, -1, None, None),
            (-1, -1, 0, None, None),
            (-1, -1, 1, None, None),
            (-1, 0, -1, None, None),
            (-1, 0, 0, None, None),
            (-1, 0, 1, None, None),
            (-1, 1, -1, None, None),
            (-1, 1, 0, None, None),
            (-1, 1, 1, None, None),
            (0, -1, -1, None, None),
            (0, -1, 0, None, None),
            (0, -1, 1, None, None),
            (0, 0, -1, None, None),
            (0, 0, 1, None, None),
            (0, 1, -1, None, None),
            (0, 1, 0, None, None),
            (0, 1, 1, None, None),
            (1, -1, -1, None, None),
            (1, -1, 0, None, None),
            (1, -1, 1, None, None),
            (1, 0, -1, None, None),
            (1, 0, 0, None, None),
            (1, 0, 1, None, None),
            (1, 1, -1, None, None),
            (1, 1, 0, None, None),
            (1, 1, 1, None, None),
        ]
    }

    /// Set hazard level for a kind.
    pub fn set_hazard_level(&mut self, kind: HazardKind, level: f32) {
        self.hazard_levels[kind.as_index()] = level;
    }

    /// Get hazard level for a kind.
    #[must_use]
    pub fn get_hazard_level(&self, kind: HazardKind) -> f32 {
        self.hazard_levels[kind.as_index()]
    }

    /// Set fluid level for a kind.
    pub fn set_fluid_level(&mut self, kind: FluidKind, level: f32) {
        self.fluid_levels[kind.as_index()] = level;
    }

    /// Get fluid level for a kind.
    #[must_use]
    pub fn get_fluid_level(&self, kind: FluidKind) -> f32 {
        self.fluid_levels[kind.as_index()]
    }

    /// Set neighbor block info.
    pub fn set_neighbor(&mut self, offset: (i8, i8, i8), block: BlockId, solid: bool) {
        for entry in &mut self.neighbor_blocks {
            if entry.0 == offset.0 && entry.1 == offset.1 && entry.2 == offset.2 {
                entry.3 = Some(block);
                entry.4 = Some(solid);
                return;
            }
        }
    }

    /// Get neighbor block ID.
    #[must_use]
    pub fn get_neighbor(&self, offset: (i8, i8, i8)) -> Option<BlockId> {
        for entry in &self.neighbor_blocks {
            if entry.0 == offset.0 && entry.1 == offset.1 && entry.2 == offset.2 {
                return entry.3;
            }
        }
        None
    }

    /// Check if neighbor is solid.
    #[must_use]
    pub fn is_neighbor_solid(&self, offset: (i8, i8, i8)) -> Option<bool> {
        for entry in &self.neighbor_blocks {
            if entry.0 == offset.0 && entry.1 == offset.1 && entry.2 == offset.2 {
                return entry.4;
            }
        }
        None
    }

    /// Set metadata value.
    pub fn set_metadata(&mut self, key: impl Into<String>, value: i32) {
        let key = key.into();
        for (k, v) in &mut self.metadata {
            if *k == key {
                *v = value;
                return;
            }
        }
        self.metadata.push((key, value));
    }

    /// Get metadata value.
    #[must_use]
    pub fn get_metadata(&self, key: &str) -> i32 {
        for (k, v) in &self.metadata {
            if k == key {
                return *v;
            }
        }
        0
    }
}

impl Default for ConditionContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn condition_always_never() {
        let ctx = ConditionContext::new();
        assert!(BehaviorCondition::Always.evaluate(&ctx));
        assert!(!BehaviorCondition::Never.evaluate(&ctx));
    }

    #[test]
    fn condition_and() {
        let ctx = ConditionContext::new();
        let cond =
            BehaviorCondition::And(vec![BehaviorCondition::Always, BehaviorCondition::Always]);
        assert!(cond.evaluate(&ctx));

        let cond =
            BehaviorCondition::And(vec![BehaviorCondition::Always, BehaviorCondition::Never]);
        assert!(!cond.evaluate(&ctx));
    }

    #[test]
    fn condition_or() {
        let ctx = ConditionContext::new();
        let cond = BehaviorCondition::Or(vec![BehaviorCondition::Never, BehaviorCondition::Always]);
        assert!(cond.evaluate(&ctx));

        let cond = BehaviorCondition::Or(vec![BehaviorCondition::Never, BehaviorCondition::Never]);
        assert!(!cond.evaluate(&ctx));
    }

    #[test]
    fn condition_not() {
        let ctx = ConditionContext::new();
        let cond = BehaviorCondition::Not(Box::new(BehaviorCondition::Never));
        assert!(cond.evaluate(&ctx));
    }

    #[test]
    fn condition_light_level() {
        let mut ctx = ConditionContext::new();
        ctx.light_level = 10;

        assert!(
            BehaviorCondition::LightLevel {
                op: CompareOp::Ge,
                value: 5,
            }
            .evaluate(&ctx)
        );

        assert!(
            !BehaviorCondition::LightLevel {
                op: CompareOp::Lt,
                value: 5,
            }
            .evaluate(&ctx)
        );
    }

    #[test]
    fn condition_hazard_level() {
        let mut ctx = ConditionContext::new();
        ctx.set_hazard_level(HazardKind::Fire, 0.7);

        assert!(
            BehaviorCondition::HazardLevel {
                kind: HazardKind::Fire,
                op: CompareOp::Gt,
                value: 0.5,
            }
            .evaluate(&ctx)
        );

        assert!(
            !BehaviorCondition::HazardLevel {
                kind: HazardKind::Frost,
                op: CompareOp::Gt,
                value: 0.5,
            }
            .evaluate(&ctx)
        );
    }

    #[test]
    fn condition_time_of_day_simple() {
        let mut ctx = ConditionContext::new();
        ctx.time_of_day = 6000;

        assert!(
            BehaviorCondition::TimeOfDay {
                min_tick: 5000,
                max_tick: 7000,
            }
            .evaluate(&ctx)
        );

        assert!(
            !BehaviorCondition::TimeOfDay {
                min_tick: 7000,
                max_tick: 9000,
            }
            .evaluate(&ctx)
        );
    }

    #[test]
    fn condition_time_of_day_wraparound() {
        let mut ctx = ConditionContext::new();
        ctx.time_of_day = 23000;

        assert!(
            BehaviorCondition::TimeOfDay {
                min_tick: 22000,
                max_tick: 2000,
            }
            .evaluate(&ctx)
        );

        ctx.time_of_day = 1000;
        assert!(
            BehaviorCondition::TimeOfDay {
                min_tick: 22000,
                max_tick: 2000,
            }
            .evaluate(&ctx)
        );

        ctx.time_of_day = 12000;
        assert!(
            !BehaviorCondition::TimeOfDay {
                min_tick: 22000,
                max_tick: 2000,
            }
            .evaluate(&ctx)
        );
    }

    #[test]
    fn condition_random_chance() {
        let mut ctx = ConditionContext::new();
        ctx.random_value = 0.3;

        assert!(BehaviorCondition::RandomChance { probability: 0.5 }.evaluate(&ctx));
        assert!(!BehaviorCondition::RandomChance { probability: 0.2 }.evaluate(&ctx));
    }

    #[test]
    fn condition_neighbor() {
        let mut ctx = ConditionContext::new();
        ctx.set_neighbor((0, -1, 0), BlockId(1), true);

        assert!(
            BehaviorCondition::NeighborIs {
                offset: (0, -1, 0),
                block: BlockId(1),
            }
            .evaluate(&ctx)
        );

        assert!(
            BehaviorCondition::NeighborSolid {
                offset: (0, -1, 0),
                solid: true,
            }
            .evaluate(&ctx)
        );
    }

    #[test]
    fn compare_op_i32() {
        assert!(CompareOp::Eq.compare_i32(5, 5));
        assert!(!CompareOp::Eq.compare_i32(5, 6));
        assert!(CompareOp::Ne.compare_i32(5, 6));
        assert!(CompareOp::Lt.compare_i32(5, 6));
        assert!(CompareOp::Le.compare_i32(5, 5));
        assert!(CompareOp::Gt.compare_i32(6, 5));
        assert!(CompareOp::Ge.compare_i32(5, 5));
    }

    #[test]
    fn serde_round_trip() {
        let conditions = [
            BehaviorCondition::Always,
            BehaviorCondition::Never,
            BehaviorCondition::And(vec![
                BehaviorCondition::Always,
                BehaviorCondition::LightLevel {
                    op: CompareOp::Ge,
                    value: 10,
                },
            ]),
            BehaviorCondition::Not(Box::new(BehaviorCondition::HasSupportBelow)),
            BehaviorCondition::RandomChance { probability: 0.25 },
        ];

        for cond in &conditions {
            let json = serde_json::to_string(cond).unwrap();
            let recovered: BehaviorCondition = serde_json::from_str(&json).unwrap();
            assert_eq!(*cond, recovered);
        }
    }
}
