//! Narrative event triggers and predicates.

use std::ops::Not;

use engine_core::coords::ChunkPos;
use serde::{Deserialize, Serialize};

use super::NarrativeEventKind;

/// Result of evaluating a trigger condition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TriggerResult {
    /// Condition not met.
    Inactive,
    /// Condition met, event should fire.
    Triggered,
    /// Condition met with a specific intensity (0.0-1.0).
    TriggeredWithIntensity(u8),
}

impl TriggerResult {
    /// Create a triggered result with intensity (clamped to 0-100).
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn with_intensity(intensity: f32) -> Self {
        let clamped = (intensity.clamp(0.0, 1.0) * 100.0) as u8;
        Self::TriggeredWithIntensity(clamped)
    }

    /// Check if the trigger fired.
    #[must_use]
    pub const fn is_triggered(&self) -> bool {
        !matches!(self, TriggerResult::Inactive)
    }

    /// Get the intensity (1.0 for plain Triggered, 0.0 for Inactive).
    #[must_use]
    pub fn intensity(&self) -> f32 {
        match self {
            TriggerResult::Inactive => 0.0,
            TriggerResult::Triggered => 1.0,
            TriggerResult::TriggeredWithIntensity(i) => f32::from(*i) / 100.0,
        }
    }
}

/// Types of trigger conditions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TriggerKind {
    /// Fires after a specific world tick.
    AtTick(u64),

    /// Fires after elapsed ticks from start.
    TimeElapsed(u64),

    /// Fires when player enters a region.
    PlayerInRegion { center: ChunkPos, radius: u32 },

    /// Fires when player exits a region.
    PlayerExitsRegion { center: ChunkPos, radius: u32 },

    /// Fires when another narrative event starts.
    OnEventStart(String),

    /// Fires when another narrative event ends.
    OnEventEnd(String),

    /// Fires on a random chance per tick (0-100 percent chance per 1000 ticks).
    RandomChance(u8),

    /// Fires when a world event of specified kind is active.
    WorldEventActive(crate::world_state::WorldEventKind),

    /// Fires when a specific flag is set in the narrative context.
    FlagSet(String),

    /// Fires when a specific flag is not set.
    FlagUnset(String),

    /// Always triggers (used with predicates for complex logic).
    Always,

    /// Never triggers (used for disabled events).
    Never,
}

/// Composable predicate for complex trigger logic.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TriggerPredicate {
    /// Single trigger condition.
    Single(TriggerKind),

    /// All conditions must be true.
    All(Vec<TriggerPredicate>),

    /// Any condition must be true.
    Any(Vec<TriggerPredicate>),

    /// Condition must be false.
    Not(Box<TriggerPredicate>),

    /// At least N conditions must be true.
    AtLeast(usize, Vec<TriggerPredicate>),
}

impl TriggerPredicate {
    /// Create a single condition predicate.
    #[must_use]
    pub fn single(kind: TriggerKind) -> Self {
        Self::Single(kind)
    }

    /// Create an AND predicate.
    #[must_use]
    pub fn all(predicates: Vec<TriggerPredicate>) -> Self {
        Self::All(predicates)
    }

    /// Create an OR predicate.
    #[must_use]
    pub fn any(predicates: Vec<TriggerPredicate>) -> Self {
        Self::Any(predicates)
    }

    /// Create a NOT predicate.
    #[must_use]
    pub fn negated(predicate: TriggerPredicate) -> Self {
        Self::Not(Box::new(predicate))
    }

    /// Create an at-least-N predicate.
    #[must_use]
    pub fn at_least(n: usize, predicates: Vec<TriggerPredicate>) -> Self {
        Self::AtLeast(n, predicates)
    }
}

impl Not for TriggerPredicate {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self::Not(Box::new(self))
    }
}

/// A narrative trigger combining conditions with evaluation logic.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NarrativeTrigger {
    /// The predicate tree to evaluate.
    pub predicate: TriggerPredicate,

    /// Optional event kind filter (only fires for matching events).
    pub event_kind_filter: Option<NarrativeEventKind>,

    /// Minimum ticks between evaluations (debounce).
    pub debounce_ticks: u64,

    /// Whether to consume the trigger after firing (one-shot).
    pub consume_on_fire: bool,
}

impl NarrativeTrigger {
    /// Create a new trigger with a single condition.
    #[must_use]
    pub fn new(kind: TriggerKind) -> Self {
        Self {
            predicate: TriggerPredicate::single(kind),
            event_kind_filter: None,
            debounce_ticks: 0,
            consume_on_fire: false,
        }
    }

    /// Create a trigger that fires at a specific tick.
    #[must_use]
    pub fn at_tick(tick: u64) -> Self {
        Self::new(TriggerKind::AtTick(tick))
    }

    /// Create a trigger that fires after elapsed time.
    #[must_use]
    pub fn time_elapsed(ticks: u64) -> Self {
        Self::new(TriggerKind::TimeElapsed(ticks))
    }

    /// Create a trigger for player entering a region.
    #[must_use]
    pub fn player_in_region(center: ChunkPos, radius: u32) -> Self {
        Self::new(TriggerKind::PlayerInRegion { center, radius })
    }

    /// Create a trigger for random chance per tick window.
    #[must_use]
    pub fn random_chance(percent_per_1000_ticks: u8) -> Self {
        Self::new(TriggerKind::RandomChance(percent_per_1000_ticks))
    }

    /// Create a trigger for when a flag is set.
    #[must_use]
    pub fn flag_set(flag: impl Into<String>) -> Self {
        Self::new(TriggerKind::FlagSet(flag.into()))
    }

    /// Create a trigger that always fires.
    #[must_use]
    pub fn always() -> Self {
        Self::new(TriggerKind::Always)
    }

    /// Create a trigger that never fires.
    #[must_use]
    pub fn never() -> Self {
        Self::new(TriggerKind::Never)
    }

    /// Set the predicate tree.
    #[must_use]
    pub fn with_predicate(mut self, predicate: TriggerPredicate) -> Self {
        self.predicate = predicate;
        self
    }

    /// Set event kind filter.
    #[must_use]
    pub fn with_event_kind_filter(mut self, kind: NarrativeEventKind) -> Self {
        self.event_kind_filter = Some(kind);
        self
    }

    /// Set debounce ticks.
    #[must_use]
    pub fn with_debounce(mut self, ticks: u64) -> Self {
        self.debounce_ticks = ticks;
        self
    }

    /// Set consume-on-fire behavior.
    #[must_use]
    pub fn with_consume_on_fire(mut self, consume: bool) -> Self {
        self.consume_on_fire = consume;
        self
    }

    /// Evaluate the trigger against context data.
    #[must_use]
    pub fn evaluate(&self, ctx: &TriggerContext) -> TriggerResult {
        if let Some(filter) = self.event_kind_filter
            && ctx.event_kind != Some(filter)
        {
            return TriggerResult::Inactive;
        }
        Self::evaluate_predicate(&self.predicate, ctx)
    }

    fn evaluate_predicate(pred: &TriggerPredicate, ctx: &TriggerContext) -> TriggerResult {
        match pred {
            TriggerPredicate::Single(kind) => Self::evaluate_kind(kind, ctx),
            TriggerPredicate::All(preds) => {
                let mut max_intensity = 1.0f32;
                for p in preds {
                    let result = Self::evaluate_predicate(p, ctx);
                    if !result.is_triggered() {
                        return TriggerResult::Inactive;
                    }
                    max_intensity = max_intensity.min(result.intensity());
                }
                TriggerResult::with_intensity(max_intensity)
            }
            TriggerPredicate::Any(preds) => {
                let mut max_intensity = 0.0f32;
                for p in preds {
                    let result = Self::evaluate_predicate(p, ctx);
                    if result.is_triggered() {
                        max_intensity = max_intensity.max(result.intensity());
                    }
                }
                if max_intensity > 0.0 {
                    TriggerResult::with_intensity(max_intensity)
                } else {
                    TriggerResult::Inactive
                }
            }
            TriggerPredicate::Not(p) => {
                if Self::evaluate_predicate(p, ctx).is_triggered() {
                    TriggerResult::Inactive
                } else {
                    TriggerResult::Triggered
                }
            }
            TriggerPredicate::AtLeast(n, preds) => {
                let triggered_count = preds
                    .iter()
                    .filter(|p| Self::evaluate_predicate(p, ctx).is_triggered())
                    .count();
                if triggered_count >= *n {
                    TriggerResult::Triggered
                } else {
                    TriggerResult::Inactive
                }
            }
        }
    }

    #[allow(clippy::cast_precision_loss)]
    fn evaluate_kind(kind: &TriggerKind, ctx: &TriggerContext) -> TriggerResult {
        match kind {
            TriggerKind::AtTick(tick) => {
                if ctx.current_tick >= *tick {
                    TriggerResult::Triggered
                } else {
                    TriggerResult::Inactive
                }
            }
            TriggerKind::TimeElapsed(elapsed) => {
                if ctx.current_tick >= ctx.start_tick + *elapsed {
                    TriggerResult::Triggered
                } else {
                    TriggerResult::Inactive
                }
            }
            TriggerKind::PlayerInRegion { center, radius } => {
                if let Some(player_pos) = ctx.player_chunk {
                    let dx = (player_pos.x() - center.x()).unsigned_abs();
                    let dz = (player_pos.z() - center.z()).unsigned_abs();
                    let dist = dx.max(dz);
                    if dist <= *radius {
                        let intensity = 1.0 - (dist as f32 / (*radius as f32 + 1.0));
                        TriggerResult::with_intensity(intensity)
                    } else {
                        TriggerResult::Inactive
                    }
                } else {
                    TriggerResult::Inactive
                }
            }
            TriggerKind::PlayerExitsRegion { center, radius } => {
                if let Some(player_pos) = ctx.player_chunk {
                    let dx = (player_pos.x() - center.x()).unsigned_abs();
                    let dz = (player_pos.z() - center.z()).unsigned_abs();
                    let dist = dx.max(dz);
                    if dist > *radius {
                        TriggerResult::Triggered
                    } else {
                        TriggerResult::Inactive
                    }
                } else {
                    TriggerResult::Inactive
                }
            }
            TriggerKind::OnEventStart(event_id) => {
                if ctx.started_events.contains(event_id) {
                    TriggerResult::Triggered
                } else {
                    TriggerResult::Inactive
                }
            }
            TriggerKind::OnEventEnd(event_id) => {
                if ctx.ended_events.contains(event_id) {
                    TriggerResult::Triggered
                } else {
                    TriggerResult::Inactive
                }
            }
            TriggerKind::RandomChance(percent) => {
                let threshold = u64::from(*percent);
                let hash = ctx.deterministic_random(threshold);
                if hash < threshold {
                    TriggerResult::Triggered
                } else {
                    TriggerResult::Inactive
                }
            }
            TriggerKind::WorldEventActive(world_kind) => {
                if ctx.active_world_events.contains(world_kind) {
                    TriggerResult::Triggered
                } else {
                    TriggerResult::Inactive
                }
            }
            TriggerKind::FlagSet(flag) => {
                if ctx.flags.contains(flag) {
                    TriggerResult::Triggered
                } else {
                    TriggerResult::Inactive
                }
            }
            TriggerKind::FlagUnset(flag) => {
                if ctx.flags.contains(flag) {
                    TriggerResult::Inactive
                } else {
                    TriggerResult::Triggered
                }
            }
            TriggerKind::Always => TriggerResult::Triggered,
            TriggerKind::Never => TriggerResult::Inactive,
        }
    }
}

/// Context data for trigger evaluation.
#[derive(Clone, Debug, Default)]
pub struct TriggerContext {
    /// Current world tick.
    pub current_tick: u64,
    /// Start tick (for `TimeElapsed` calculations).
    pub start_tick: u64,
    /// Player chunk position if known.
    pub player_chunk: Option<ChunkPos>,
    /// Event kind being evaluated (for filters).
    pub event_kind: Option<NarrativeEventKind>,
    /// Events that started this tick.
    pub started_events: Vec<String>,
    /// Events that ended this tick.
    pub ended_events: Vec<String>,
    /// Active world event kinds.
    pub active_world_events: Vec<crate::world_state::WorldEventKind>,
    /// Set flags.
    pub flags: Vec<String>,
    /// Seed for deterministic random.
    pub random_seed: u64,
}

impl TriggerContext {
    /// Create a new context at a tick.
    #[must_use]
    pub fn new(current_tick: u64) -> Self {
        Self {
            current_tick,
            start_tick: 0,
            player_chunk: None,
            event_kind: None,
            started_events: Vec::new(),
            ended_events: Vec::new(),
            active_world_events: Vec::new(),
            flags: Vec::new(),
            random_seed: 0,
        }
    }

    /// Set start tick.
    #[must_use]
    pub fn with_start_tick(mut self, tick: u64) -> Self {
        self.start_tick = tick;
        self
    }

    /// Set player position.
    #[must_use]
    pub fn with_player_chunk(mut self, pos: ChunkPos) -> Self {
        self.player_chunk = Some(pos);
        self
    }

    /// Set event kind for filtering.
    #[must_use]
    pub fn with_event_kind(mut self, kind: NarrativeEventKind) -> Self {
        self.event_kind = Some(kind);
        self
    }

    /// Add a flag.
    #[must_use]
    pub fn with_flag(mut self, flag: impl Into<String>) -> Self {
        self.flags.push(flag.into());
        self
    }

    /// Set random seed.
    #[must_use]
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.random_seed = seed;
        self
    }

    /// Get deterministic random value in range [0, max).
    fn deterministic_random(&self, max: u64) -> u64 {
        if max == 0 {
            return 0;
        }
        let hash = self
            .random_seed
            .wrapping_mul(0x517c_c1b7_2722_0a95)
            .wrapping_add(self.current_tick.wrapping_mul(0x9e37_79b9_7f4a_7c15));
        hash % max
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_result_intensity() {
        assert!(TriggerResult::Inactive.intensity().abs() < f32::EPSILON);
        assert!((TriggerResult::Triggered.intensity() - 1.0).abs() < f32::EPSILON);
        assert!((TriggerResult::with_intensity(0.5).intensity() - 0.5).abs() < 0.02);
    }

    #[test]
    fn trigger_result_clamping() {
        let high = TriggerResult::with_intensity(1.5);
        assert!((high.intensity() - 1.0).abs() < f32::EPSILON);
        let low = TriggerResult::with_intensity(-0.5);
        assert!(low.intensity().abs() < f32::EPSILON);
    }

    #[test]
    fn trigger_at_tick() {
        let trigger = NarrativeTrigger::at_tick(100);
        let ctx_before = TriggerContext::new(50);
        let ctx_at = TriggerContext::new(100);
        let ctx_after = TriggerContext::new(150);

        assert!(!trigger.evaluate(&ctx_before).is_triggered());
        assert!(trigger.evaluate(&ctx_at).is_triggered());
        assert!(trigger.evaluate(&ctx_after).is_triggered());
    }

    #[test]
    fn trigger_time_elapsed() {
        let trigger = NarrativeTrigger::time_elapsed(50);
        let ctx = TriggerContext::new(100).with_start_tick(60);
        assert!(!trigger.evaluate(&ctx).is_triggered());

        let ctx2 = TriggerContext::new(110).with_start_tick(60);
        assert!(trigger.evaluate(&ctx2).is_triggered());
    }

    #[test]
    fn trigger_player_in_region() {
        let trigger = NarrativeTrigger::player_in_region(ChunkPos::new(10, 0, 10), 5);

        let ctx_in = TriggerContext::new(0).with_player_chunk(ChunkPos::new(12, 0, 12));
        assert!(trigger.evaluate(&ctx_in).is_triggered());

        let ctx_out = TriggerContext::new(0).with_player_chunk(ChunkPos::new(20, 0, 20));
        assert!(!trigger.evaluate(&ctx_out).is_triggered());
    }

    #[test]
    fn trigger_flag_set() {
        let trigger = NarrativeTrigger::flag_set("quest_active");

        let ctx_without = TriggerContext::new(0);
        assert!(!trigger.evaluate(&ctx_without).is_triggered());

        let ctx_with = TriggerContext::new(0).with_flag("quest_active");
        assert!(trigger.evaluate(&ctx_with).is_triggered());
    }

    #[test]
    fn trigger_always_never() {
        let always = NarrativeTrigger::always();
        let never = NarrativeTrigger::never();
        let ctx = TriggerContext::new(0);

        assert!(always.evaluate(&ctx).is_triggered());
        assert!(!never.evaluate(&ctx).is_triggered());
    }

    #[test]
    fn predicate_all() {
        let trigger =
            NarrativeTrigger::new(TriggerKind::Always).with_predicate(TriggerPredicate::all(vec![
                TriggerPredicate::single(TriggerKind::AtTick(10)),
                TriggerPredicate::single(TriggerKind::FlagSet("test".into())),
            ]));

        let ctx_partial = TriggerContext::new(15);
        assert!(!trigger.evaluate(&ctx_partial).is_triggered());

        let ctx_full = TriggerContext::new(15).with_flag("test");
        assert!(trigger.evaluate(&ctx_full).is_triggered());
    }

    #[test]
    fn predicate_any() {
        let trigger =
            NarrativeTrigger::new(TriggerKind::Always).with_predicate(TriggerPredicate::any(vec![
                TriggerPredicate::single(TriggerKind::AtTick(100)),
                TriggerPredicate::single(TriggerKind::FlagSet("test".into())),
            ]));

        let ctx_none = TriggerContext::new(50);
        assert!(!trigger.evaluate(&ctx_none).is_triggered());

        let ctx_tick = TriggerContext::new(100);
        assert!(trigger.evaluate(&ctx_tick).is_triggered());

        let ctx_flag = TriggerContext::new(50).with_flag("test");
        assert!(trigger.evaluate(&ctx_flag).is_triggered());
    }

    #[test]
    fn predicate_not() {
        let trigger =
            NarrativeTrigger::new(TriggerKind::Always).with_predicate(TriggerPredicate::negated(
                TriggerPredicate::single(TriggerKind::FlagSet("blocked".into())),
            ));

        let ctx_clear = TriggerContext::new(0);
        assert!(trigger.evaluate(&ctx_clear).is_triggered());

        let ctx_blocked = TriggerContext::new(0).with_flag("blocked");
        assert!(!trigger.evaluate(&ctx_blocked).is_triggered());
    }

    #[test]
    fn predicate_at_least() {
        let trigger =
            NarrativeTrigger::new(TriggerKind::Always).with_predicate(TriggerPredicate::at_least(
                2,
                vec![
                    TriggerPredicate::single(TriggerKind::FlagSet("a".into())),
                    TriggerPredicate::single(TriggerKind::FlagSet("b".into())),
                    TriggerPredicate::single(TriggerKind::FlagSet("c".into())),
                ],
            ));

        let ctx_one = TriggerContext::new(0).with_flag("a");
        assert!(!trigger.evaluate(&ctx_one).is_triggered());

        let mut ctx_two = TriggerContext::new(0);
        ctx_two.flags = vec!["a".into(), "b".into()];
        assert!(trigger.evaluate(&ctx_two).is_triggered());
    }

    #[test]
    fn event_kind_filter() {
        let trigger =
            NarrativeTrigger::always().with_event_kind_filter(NarrativeEventKind::Disaster);

        let ctx_wrong = TriggerContext::new(0).with_event_kind(NarrativeEventKind::Radio);
        assert!(!trigger.evaluate(&ctx_wrong).is_triggered());

        let ctx_right = TriggerContext::new(0).with_event_kind(NarrativeEventKind::Disaster);
        assert!(trigger.evaluate(&ctx_right).is_triggered());
    }

    #[test]
    fn serde_round_trip() {
        let trigger = NarrativeTrigger::player_in_region(ChunkPos::new(5, 0, 5), 10)
            .with_debounce(100)
            .with_consume_on_fire(true);

        let json = serde_json::to_string(&trigger).unwrap();
        let recovered: NarrativeTrigger = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, trigger);
    }
}
