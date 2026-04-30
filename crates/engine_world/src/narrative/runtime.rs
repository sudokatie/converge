//! Narrative runtime state and event management.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use super::{
    ChecksumBuilder, CooldownState, EventDefinition, EventFingerprint, EventRegistry,
    NarrativeEventKind, NarrativeOutput, OutputQueue, RegistryError, StateChecksum,
};
use crate::world_state::WorldEventKind;
use engine_core::coords::ChunkPos;

/// Unique identifier for an active narrative event instance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(pub u64);

impl EventId {
    /// Create a new event ID.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

/// An active narrative event instance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiveEvent {
    /// Unique instance ID.
    pub id: EventId,

    /// Definition ID this event is based on.
    pub definition_id: String,

    /// Event kind.
    pub kind: NarrativeEventKind,

    /// Tick when the event started.
    pub start_tick: u64,

    /// Tick when the event will end (if timed).
    pub end_tick: Option<u64>,

    /// Intensity of the trigger that activated this event (0.0-1.0).
    pub intensity: f32,

    /// Whether the event is paused.
    pub paused: bool,

    /// Custom state data.
    pub state_data: HashMap<String, String>,
}

impl ActiveEvent {
    /// Create a new active event.
    #[must_use]
    pub fn new(
        id: EventId,
        definition_id: impl Into<String>,
        kind: NarrativeEventKind,
        start_tick: u64,
        duration: Option<u64>,
    ) -> Self {
        Self {
            id,
            definition_id: definition_id.into(),
            kind,
            start_tick,
            end_tick: duration.map(|d| start_tick + d),
            intensity: 1.0,
            paused: false,
            state_data: HashMap::new(),
        }
    }

    /// Set intensity.
    #[must_use]
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity.clamp(0.0, 1.0);
        self
    }

    /// Check if the event has expired.
    #[must_use]
    pub fn is_expired(&self, current_tick: u64) -> bool {
        if let Some(end) = self.end_tick {
            current_tick >= end
        } else {
            false
        }
    }

    /// Get remaining duration in ticks.
    #[must_use]
    pub fn remaining_ticks(&self, current_tick: u64) -> Option<u64> {
        self.end_tick.map(|end| end.saturating_sub(current_tick))
    }

    /// Get elapsed ticks since start.
    #[must_use]
    pub fn elapsed_ticks(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.start_tick)
    }

    /// Pause the event.
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Resume the event.
    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// Set custom state data.
    pub fn set_state(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.state_data.insert(key.into(), value.into());
    }

    /// Get custom state data.
    #[must_use]
    pub fn get_state(&self, key: &str) -> Option<&str> {
        self.state_data.get(key).map(String::as_str)
    }
}

/// Context for evaluating triggers and generating outputs.
#[derive(Clone, Debug, Default)]
pub struct NarrativeContext {
    /// Current world tick.
    pub current_tick: u64,

    /// Player chunk position if known.
    pub player_chunk: Option<ChunkPos>,

    /// Active world event kinds.
    pub world_events: Vec<WorldEventKind>,

    /// Deterministic random seed.
    pub random_seed: u64,

    /// Events that started this tick (for chaining).
    pub started_events: Vec<String>,

    /// Events that ended this tick (for chaining).
    pub ended_events: Vec<String>,
}

impl NarrativeContext {
    /// Create a new context at a tick.
    #[must_use]
    pub fn new(current_tick: u64) -> Self {
        Self {
            current_tick,
            player_chunk: None,
            world_events: Vec::new(),
            random_seed: current_tick,
            started_events: Vec::new(),
            ended_events: Vec::new(),
        }
    }

    /// Set player chunk position.
    #[must_use]
    pub fn with_player_chunk(mut self, pos: ChunkPos) -> Self {
        self.player_chunk = Some(pos);
        self
    }

    /// Add a world event.
    #[must_use]
    pub fn with_world_event(mut self, event: WorldEventKind) -> Self {
        self.world_events.push(event);
        self
    }

    /// Set random seed.
    #[must_use]
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.random_seed = seed;
        self
    }

    /// Build trigger context from this narrative context.
    fn to_trigger_context(
        &self,
        start_tick: u64,
        flags: &[String],
    ) -> super::trigger::TriggerContext {
        super::trigger::TriggerContext {
            current_tick: self.current_tick,
            start_tick,
            player_chunk: self.player_chunk,
            event_kind: None,
            started_events: self.started_events.clone(),
            ended_events: self.ended_events.clone(),
            active_world_events: self.world_events.clone(),
            flags: flags.to_vec(),
            random_seed: self.random_seed,
        }
    }
}

/// Result of a narrative tick.
#[derive(Clone, Debug, Default)]
pub struct TickResult {
    /// Events that were triggered this tick.
    pub triggered: Vec<EventId>,

    /// Events that ended this tick.
    pub ended: Vec<EventId>,

    /// New outputs generated this tick.
    pub outputs: Vec<NarrativeOutput>,

    /// Checksum of state after tick.
    pub checksum: StateChecksum,
}

impl TickResult {
    /// Check if any events were triggered.
    #[must_use]
    pub fn had_activity(&self) -> bool {
        !self.triggered.is_empty() || !self.ended.is_empty() || !self.outputs.is_empty()
    }
}

/// Runtime state for the narrative system.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NarrativeState {
    /// Active events.
    active_events: HashMap<EventId, ActiveEvent>,

    /// Cooldown states per definition.
    cooldowns: HashMap<String, CooldownState>,

    /// Output queue.
    output_queue: OutputQueue,

    /// Global flags.
    flags: HashSet<String>,

    /// Next event ID.
    next_event_id: u64,

    /// Start tick for elapsed time calculations.
    start_tick: u64,

    /// Events that have ever fired (for prerequisites).
    fired_events: HashSet<String>,

    /// Currently blocked event definitions.
    blocked_definitions: HashSet<String>,

    /// Registry fingerprint (for validation).
    registry_fingerprint: u32,
}

impl NarrativeState {
    /// Create a new narrative state.
    #[must_use]
    pub fn new(registry: &EventRegistry) -> Self {
        Self {
            active_events: HashMap::new(),
            cooldowns: HashMap::new(),
            output_queue: OutputQueue::new(),
            flags: HashSet::new(),
            next_event_id: 0,
            start_tick: 0,
            fired_events: HashSet::new(),
            blocked_definitions: HashSet::new(),
            registry_fingerprint: registry.fingerprint(),
        }
    }

    /// Create a new state starting at a specific tick.
    #[must_use]
    pub fn new_at_tick(registry: &EventRegistry, start_tick: u64) -> Self {
        let mut state = Self::new(registry);
        state.start_tick = start_tick;
        state
    }

    /// Validate that the registry fingerprint matches.
    ///
    /// # Errors
    ///
    /// Returns `RegistryError::Invalid` if the fingerprints do not match.
    pub fn validate_registry(&self, registry: &EventRegistry) -> Result<(), RegistryError> {
        let current = registry.fingerprint();
        if current == self.registry_fingerprint {
            Ok(())
        } else {
            Err(RegistryError::Invalid(format!(
                "registry fingerprint mismatch: expected {}, got {}",
                self.registry_fingerprint, current
            )))
        }
    }

    /// Update registry fingerprint.
    pub fn update_registry_fingerprint(&mut self, registry: &EventRegistry) {
        self.registry_fingerprint = registry.fingerprint();
    }

    /// Tick the narrative system.
    pub fn tick(&mut self, registry: &EventRegistry, ctx: &NarrativeContext) -> TickResult {
        let mut result = TickResult::default();
        let flags: Vec<String> = self.flags.iter().cloned().collect();

        self.update_blocked_definitions(registry);

        let ended = self.expire_events(ctx.current_tick);
        result.ended.clone_from(&ended);

        let mut context_with_ended = ctx.clone();
        for event in &ended {
            if let Some(active) = self.active_events.get(event) {
                context_with_ended
                    .ended_events
                    .push(active.definition_id.clone());
            }
        }

        for def in registry.enabled() {
            if self.blocked_definitions.contains(&def.id) {
                continue;
            }

            if !self.check_prerequisites(def) {
                continue;
            }

            let can_fire = {
                let cooldown = self.cooldowns.entry(def.id.clone()).or_default();
                cooldown.can_fire(&def.cooldown, ctx.current_tick)
            };
            if !can_fire {
                continue;
            }

            let trigger_ctx = context_with_ended.to_trigger_context(self.start_tick, &flags);

            for trigger in &def.triggers {
                let trigger_result = trigger.evaluate(&trigger_ctx);
                if trigger_result.is_triggered() {
                    let event_id =
                        self.fire_event(def, ctx.current_tick, trigger_result.intensity());
                    result.triggered.push(event_id);

                    let output = Self::create_output(def, event_id, ctx.current_tick);
                    self.output_queue.enqueue(output.clone());
                    result.outputs.push(output);

                    if let Some(cooldown) = self.cooldowns.get_mut(&def.id) {
                        cooldown.record_fire(&def.cooldown, ctx.current_tick, ctx.random_seed);
                    }
                    self.fired_events.insert(def.id.clone());

                    if trigger.consume_on_fire {
                        break;
                    }
                }
            }
        }

        for event_id in &result.triggered {
            if let Some(event) = self.active_events.get(event_id) {
                context_with_ended
                    .started_events
                    .push(event.definition_id.clone());
            }
        }

        self.output_queue.cleanup(ctx.current_tick);

        result.checksum = self.compute_checksum(ctx.current_tick);
        result
    }

    fn update_blocked_definitions(&mut self, registry: &EventRegistry) {
        self.blocked_definitions.clear();
        for event in self.active_events.values() {
            if let Some(def) = registry.get(&event.definition_id) {
                for blocked in &def.blocks {
                    self.blocked_definitions.insert(blocked.clone());
                }
            }
        }
    }

    fn check_prerequisites(&self, def: &EventDefinition) -> bool {
        def.prerequisites
            .iter()
            .all(|prereq| self.fired_events.contains(prereq))
    }

    fn expire_events(&mut self, current_tick: u64) -> Vec<EventId> {
        let expired: Vec<EventId> = self
            .active_events
            .iter()
            .filter(|(_, e)| e.is_expired(current_tick))
            .map(|(id, _)| *id)
            .collect();

        for id in &expired {
            self.active_events.remove(id);
        }

        expired
    }

    fn fire_event(&mut self, def: &EventDefinition, tick: u64, intensity: f32) -> EventId {
        let id = EventId::new(self.next_event_id);
        self.next_event_id += 1;

        let event = ActiveEvent::new(id, &def.id, def.kind, tick, Some(def.duration))
            .with_intensity(intensity);

        self.active_events.insert(id, event);
        id
    }

    fn create_output(def: &EventDefinition, event_id: EventId, tick: u64) -> NarrativeOutput {
        let mut output = NarrativeOutput::new(0, event_id, def.kind, tick)
            .with_priority(def.priority)
            .with_display_duration(def.duration.min(900));

        if let Some(text) = &def.text_template {
            output = output.with_text(text);
        }

        if let Some(audio) = &def.audio_cue {
            output = output.with_audio(audio);
        }

        output
    }

    /// Compute a checksum of the current state.
    #[must_use]
    pub fn compute_checksum(&self, tick: u64) -> StateChecksum {
        let mut builder = ChecksumBuilder::new();

        let mut event_ids: Vec<_> = self.active_events.keys().collect();
        event_ids.sort_by_key(|id| id.0);
        for id in event_ids {
            if let Some(event) = self.active_events.get(id) {
                builder.add_active_event(id.0, &event.definition_id, event.start_tick);
            }
        }

        let mut cooldown_ids: Vec<_> = self.cooldowns.keys().collect();
        cooldown_ids.sort();
        for id in cooldown_ids {
            if let Some(state) = self.cooldowns.get(id) {
                builder.add_cooldown(id, state.fire_count, state.ready_at_tick);
            }
        }

        let mut sorted_flags: Vec<_> = self.flags.iter().collect();
        sorted_flags.sort();
        for flag in sorted_flags {
            builder.add_flag(flag);
        }

        builder.build(tick, self.output_queue.checksum())
    }

    /// Get active events.
    #[must_use]
    pub fn active_events(&self) -> &HashMap<EventId, ActiveEvent> {
        &self.active_events
    }

    /// Get active events by kind.
    pub fn active_by_kind(&self, kind: NarrativeEventKind) -> impl Iterator<Item = &ActiveEvent> {
        self.active_events.values().filter(move |e| e.kind == kind)
    }

    /// Get an active event by ID.
    #[must_use]
    pub fn get_event(&self, id: EventId) -> Option<&ActiveEvent> {
        self.active_events.get(&id)
    }

    /// Get a mutable active event by ID.
    pub fn get_event_mut(&mut self, id: EventId) -> Option<&mut ActiveEvent> {
        self.active_events.get_mut(&id)
    }

    /// Manually end an event.
    pub fn end_event(&mut self, id: EventId) -> Option<ActiveEvent> {
        self.active_events.remove(&id)
    }

    /// Get the output queue.
    #[must_use]
    pub fn output_queue(&self) -> &OutputQueue {
        &self.output_queue
    }

    /// Get a mutable reference to the output queue.
    pub fn output_queue_mut(&mut self) -> &mut OutputQueue {
        &mut self.output_queue
    }

    /// Set a flag.
    pub fn set_flag(&mut self, flag: impl Into<String>) {
        self.flags.insert(flag.into());
    }

    /// Clear a flag.
    pub fn clear_flag(&mut self, flag: &str) {
        self.flags.remove(flag);
    }

    /// Check if a flag is set.
    #[must_use]
    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.contains(flag)
    }

    /// Get all flags.
    #[must_use]
    pub fn flags(&self) -> &HashSet<String> {
        &self.flags
    }

    /// Check if a definition has fired.
    #[must_use]
    pub fn has_fired(&self, definition_id: &str) -> bool {
        self.fired_events.contains(definition_id)
    }

    /// Get cooldown state for a definition.
    #[must_use]
    pub fn cooldown_for(&self, definition_id: &str) -> Option<&CooldownState> {
        self.cooldowns.get(definition_id)
    }

    /// Reset cooldown for a definition.
    pub fn reset_cooldown(&mut self, definition_id: &str) {
        if let Some(state) = self.cooldowns.get_mut(definition_id) {
            state.reset();
        }
    }

    /// Reset all state.
    pub fn reset(&mut self) {
        self.active_events.clear();
        self.cooldowns.clear();
        self.output_queue.clear();
        self.flags.clear();
        self.fired_events.clear();
        self.blocked_definitions.clear();
        self.next_event_id = 0;
    }

    /// Get number of active events.
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.active_events.len()
    }

    /// Get the registry fingerprint.
    #[must_use]
    pub fn registry_fingerprint(&self) -> u32 {
        self.registry_fingerprint
    }

    /// Compute an event fingerprint.
    #[must_use]
    pub fn event_fingerprint(&self, def: &EventDefinition) -> EventFingerprint {
        EventFingerprint::from_definition(
            &def.id,
            def.kind,
            def.duration,
            def.enabled,
            def.triggers.len(),
        )
    }
}

impl Default for NarrativeState {
    fn default() -> Self {
        Self {
            active_events: HashMap::new(),
            cooldowns: HashMap::new(),
            output_queue: OutputQueue::new(),
            flags: HashSet::new(),
            next_event_id: 0,
            start_tick: 0,
            fired_events: HashSet::new(),
            blocked_definitions: HashSet::new(),
            registry_fingerprint: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::narrative::{CooldownConfig, NarrativeTrigger, OutputPriority};

    fn make_registry_with_event(id: &str, kind: NarrativeEventKind) -> EventRegistry {
        let mut registry = EventRegistry::new();
        let def = EventDefinition::new(id, kind)
            .with_trigger(NarrativeTrigger::always())
            .with_duration(100);
        registry.register(def).unwrap();
        registry
    }

    fn make_timed_registry(id: &str, trigger_tick: u64) -> EventRegistry {
        let mut registry = EventRegistry::new();
        let def = EventDefinition::new(id, NarrativeEventKind::Radio)
            .with_trigger(NarrativeTrigger::at_tick(trigger_tick))
            .with_duration(100);
        registry.register(def).unwrap();
        registry
    }

    #[test]
    fn event_id_creation() {
        let id = EventId::new(42);
        assert_eq!(id.0, 42);
    }

    #[test]
    fn active_event_lifecycle() {
        let event = ActiveEvent::new(
            EventId::new(1),
            "test",
            NarrativeEventKind::Radio,
            100,
            Some(200),
        );

        assert!(!event.is_expired(100));
        assert!(!event.is_expired(299));
        assert!(event.is_expired(300));
        assert_eq!(event.remaining_ticks(100), Some(200));
        assert_eq!(event.remaining_ticks(200), Some(100));
        assert_eq!(event.elapsed_ticks(150), 50);
    }

    #[test]
    fn active_event_intensity() {
        let event = ActiveEvent::new(EventId::new(1), "test", NarrativeEventKind::Radio, 0, None)
            .with_intensity(0.75);
        assert!((event.intensity - 0.75).abs() < 0.01);
    }

    #[test]
    fn active_event_state_data() {
        let mut event =
            ActiveEvent::new(EventId::new(1), "test", NarrativeEventKind::Radio, 0, None);

        event.set_state("key", "value");
        assert_eq!(event.get_state("key"), Some("value"));
        assert_eq!(event.get_state("missing"), None);
    }

    #[test]
    fn active_event_pause_resume() {
        let mut event =
            ActiveEvent::new(EventId::new(1), "test", NarrativeEventKind::Radio, 0, None);

        assert!(!event.paused);
        event.pause();
        assert!(event.paused);
        event.resume();
        assert!(!event.paused);
    }

    #[test]
    fn narrative_context_creation() {
        let ctx = NarrativeContext::new(100)
            .with_player_chunk(ChunkPos::new(5, 0, 5))
            .with_seed(12345);

        assert_eq!(ctx.current_tick, 100);
        assert_eq!(ctx.player_chunk, Some(ChunkPos::new(5, 0, 5)));
        assert_eq!(ctx.random_seed, 12345);
    }

    #[test]
    fn state_new() {
        let registry = EventRegistry::new();
        let state = NarrativeState::new(&registry);

        assert_eq!(state.active_count(), 0);
        assert!(state.flags().is_empty());
    }

    #[test]
    fn state_tick_triggers_event() {
        let registry = make_registry_with_event("test_event", NarrativeEventKind::Radio);
        let mut state = NarrativeState::new(&registry);
        let ctx = NarrativeContext::new(100);

        let result = state.tick(&registry, &ctx);

        assert_eq!(result.triggered.len(), 1);
        assert_eq!(state.active_count(), 1);
        assert!(state.has_fired("test_event"));
    }

    #[test]
    fn state_tick_generates_output() {
        let registry = make_registry_with_event("test_event", NarrativeEventKind::Radio);
        let mut state = NarrativeState::new(&registry);
        let ctx = NarrativeContext::new(100);

        let result = state.tick(&registry, &ctx);

        assert_eq!(result.outputs.len(), 1);
        assert_eq!(result.outputs[0].kind, NarrativeEventKind::Radio);
    }

    #[test]
    fn state_tick_respects_cooldown() {
        let mut registry = EventRegistry::new();
        let def = EventDefinition::new("test", NarrativeEventKind::Radio)
            .with_trigger(NarrativeTrigger::always())
            .with_cooldown(CooldownConfig::repeating(100))
            .with_duration(50);
        registry.register(def).unwrap();

        let mut state = NarrativeState::new(&registry);

        let result1 = state.tick(&registry, &NarrativeContext::new(0));
        assert_eq!(result1.triggered.len(), 1);

        let result2 = state.tick(&registry, &NarrativeContext::new(50));
        assert_eq!(result2.triggered.len(), 0);

        let result3 = state.tick(&registry, &NarrativeContext::new(100));
        assert_eq!(result3.triggered.len(), 1);
    }

    #[test]
    fn state_tick_respects_once_cooldown() {
        let registry = make_registry_with_event("test", NarrativeEventKind::Radio);
        let mut state = NarrativeState::new(&registry);

        let result1 = state.tick(&registry, &NarrativeContext::new(0));
        assert_eq!(result1.triggered.len(), 1);

        let result2 = state.tick(&registry, &NarrativeContext::new(1000));
        assert_eq!(result2.triggered.len(), 0);
    }

    #[test]
    fn state_events_expire() {
        let registry = make_registry_with_event("test", NarrativeEventKind::Radio);
        let mut state = NarrativeState::new(&registry);

        state.tick(&registry, &NarrativeContext::new(0));
        assert_eq!(state.active_count(), 1);

        let result = state.tick(&registry, &NarrativeContext::new(100));
        assert_eq!(result.ended.len(), 1);
        assert_eq!(state.active_count(), 0);
    }

    #[test]
    fn state_timed_trigger() {
        let registry = make_timed_registry("timed_event", 500);
        let mut state = NarrativeState::new(&registry);

        let result1 = state.tick(&registry, &NarrativeContext::new(400));
        assert_eq!(result1.triggered.len(), 0);

        let result2 = state.tick(&registry, &NarrativeContext::new(500));
        assert_eq!(result2.triggered.len(), 1);
    }

    #[test]
    fn state_flags() {
        let registry = EventRegistry::new();
        let mut state = NarrativeState::new(&registry);

        assert!(!state.has_flag("test_flag"));

        state.set_flag("test_flag");
        assert!(state.has_flag("test_flag"));

        state.clear_flag("test_flag");
        assert!(!state.has_flag("test_flag"));
    }

    #[test]
    fn state_flag_trigger() {
        let mut registry = EventRegistry::new();
        let def = EventDefinition::new("flag_event", NarrativeEventKind::Radio)
            .with_trigger(NarrativeTrigger::flag_set("activate"))
            .with_duration(100);
        registry.register(def).unwrap();

        let mut state = NarrativeState::new(&registry);

        let result1 = state.tick(&registry, &NarrativeContext::new(0));
        assert_eq!(result1.triggered.len(), 0);

        state.set_flag("activate");
        let result2 = state.tick(&registry, &NarrativeContext::new(1));
        assert_eq!(result2.triggered.len(), 1);
    }

    #[test]
    fn state_prerequisites() {
        let mut registry = EventRegistry::new();
        registry
            .register(
                EventDefinition::new("first", NarrativeEventKind::Radio)
                    .with_trigger(NarrativeTrigger::at_tick(0))
                    .with_duration(10),
            )
            .unwrap();
        registry
            .register(
                EventDefinition::new("second", NarrativeEventKind::Radio)
                    .with_trigger(NarrativeTrigger::always())
                    .with_prerequisite("first")
                    .with_cooldown(CooldownConfig::repeating(0))
                    .with_duration(10),
            )
            .unwrap();

        let mut state = NarrativeState::new(&registry);

        let result1 = state.tick(&registry, &NarrativeContext::new(0));
        let triggered_ids: Vec<_> = result1
            .triggered
            .iter()
            .filter_map(|id| state.get_event(*id).map(|e| e.definition_id.clone()))
            .collect();
        assert!(triggered_ids.contains(&"first".to_string()));

        let result2 = state.tick(&registry, &NarrativeContext::new(1));
        let has_second = result2
            .triggered
            .iter()
            .filter_map(|id| state.get_event(*id))
            .any(|e| e.definition_id == "second");
        assert!(has_second);
    }

    #[test]
    fn state_blocking() {
        let mut registry = EventRegistry::new();
        registry
            .register(
                EventDefinition::new("blocker", NarrativeEventKind::Disaster)
                    .with_trigger(NarrativeTrigger::at_tick(0))
                    .with_blocks("blocked")
                    .with_duration(100),
            )
            .unwrap();
        registry
            .register(
                EventDefinition::new("blocked", NarrativeEventKind::Radio)
                    .with_trigger(NarrativeTrigger::always())
                    .with_cooldown(CooldownConfig::repeating(0))
                    .with_duration(50),
            )
            .unwrap();

        let mut state = NarrativeState::new(&registry);

        state.tick(&registry, &NarrativeContext::new(0));

        let result = state.tick(&registry, &NarrativeContext::new(50));
        let triggered_blocked = result
            .triggered
            .iter()
            .filter_map(|id| state.get_event(*id))
            .any(|e| e.definition_id == "blocked");
        assert!(!triggered_blocked);

        state.tick(&registry, &NarrativeContext::new(100));
        let result_unblocked = state.tick(&registry, &NarrativeContext::new(101));
        let triggered_after = result_unblocked
            .triggered
            .iter()
            .filter_map(|id| state.get_event(*id))
            .any(|e| e.definition_id == "blocked");
        assert!(triggered_after);
    }

    #[test]
    fn state_manual_end_event() {
        let registry = make_registry_with_event("test", NarrativeEventKind::Radio);
        let mut state = NarrativeState::new(&registry);

        let result = state.tick(&registry, &NarrativeContext::new(0));
        let event_id = result.triggered[0];

        assert!(state.get_event(event_id).is_some());

        let ended = state.end_event(event_id);
        assert!(ended.is_some());
        assert!(state.get_event(event_id).is_none());
    }

    #[test]
    fn state_reset() {
        let registry = make_registry_with_event("test", NarrativeEventKind::Radio);
        let mut state = NarrativeState::new(&registry);

        state.tick(&registry, &NarrativeContext::new(0));
        state.set_flag("test_flag");

        assert!(state.active_count() > 0);
        assert!(state.has_flag("test_flag"));

        state.reset();

        assert_eq!(state.active_count(), 0);
        assert!(!state.has_flag("test_flag"));
        assert!(!state.has_fired("test"));
    }

    #[test]
    fn state_checksum_deterministic() {
        let registry = make_registry_with_event("test", NarrativeEventKind::Radio);

        let mut state1 = NarrativeState::new(&registry);
        let mut state2 = NarrativeState::new(&registry);

        state1.tick(&registry, &NarrativeContext::new(0).with_seed(42));
        state2.tick(&registry, &NarrativeContext::new(0).with_seed(42));

        let cs1 = state1.compute_checksum(0);
        let cs2 = state2.compute_checksum(0);

        assert!(cs1.matches(&cs2));
    }

    #[test]
    fn state_registry_validation() {
        let mut registry = EventRegistry::new();
        registry
            .register(
                EventDefinition::new("test", NarrativeEventKind::Radio)
                    .with_trigger(NarrativeTrigger::always()),
            )
            .unwrap();

        let state = NarrativeState::new(&registry);

        assert!(state.validate_registry(&registry).is_ok());

        registry
            .register(
                EventDefinition::new("new_event", NarrativeEventKind::Radio)
                    .with_trigger(NarrativeTrigger::always()),
            )
            .unwrap();

        assert!(state.validate_registry(&registry).is_err());
    }

    #[test]
    fn tick_result_had_activity() {
        let empty = TickResult::default();
        assert!(!empty.had_activity());

        let with_triggered = TickResult {
            triggered: vec![EventId::new(1)],
            ..Default::default()
        };
        assert!(with_triggered.had_activity());

        let with_ended = TickResult {
            ended: vec![EventId::new(1)],
            ..Default::default()
        };
        assert!(with_ended.had_activity());
    }

    #[test]
    fn output_queue_ordering() {
        let registry = make_registry_with_event("test", NarrativeEventKind::Radio);
        let mut state = NarrativeState::new(&registry);

        state.tick(&registry, &NarrativeContext::new(0));

        assert!(!state.output_queue().is_empty());
    }

    #[test]
    fn serde_round_trip_active_event() {
        let event = ActiveEvent::new(
            EventId::new(1),
            "test_event",
            NarrativeEventKind::Disaster,
            100,
            Some(200),
        )
        .with_intensity(0.8);

        let json = serde_json::to_string(&event).unwrap();
        let recovered: ActiveEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, event);
    }

    #[test]
    fn serde_round_trip_state() {
        let registry = make_registry_with_event("test", NarrativeEventKind::Radio);
        let mut state = NarrativeState::new(&registry);

        state.tick(&registry, &NarrativeContext::new(0));
        state.set_flag("test_flag");

        let json = serde_json::to_string(&state).unwrap();
        let recovered: NarrativeState = serde_json::from_str(&json).unwrap();

        assert_eq!(recovered.active_count(), state.active_count());
        assert!(recovered.has_flag("test_flag"));
        assert!(recovered.has_fired("test"));
    }

    #[test]
    fn disaster_event_priority() {
        let mut registry = EventRegistry::new();
        registry
            .register(
                EventDefinition::new("disaster", NarrativeEventKind::Disaster)
                    .with_trigger(NarrativeTrigger::always())
                    .with_priority(OutputPriority::CRITICAL)
                    .with_duration(100),
            )
            .unwrap();
        registry
            .register(
                EventDefinition::new("radio", NarrativeEventKind::Radio)
                    .with_trigger(NarrativeTrigger::always())
                    .with_priority(OutputPriority::LOW)
                    .with_duration(100),
            )
            .unwrap();

        let mut state = NarrativeState::new(&registry);
        let result = state.tick(&registry, &NarrativeContext::new(0));

        assert_eq!(result.outputs.len(), 2);

        let disaster_output = result
            .outputs
            .iter()
            .find(|o| o.kind == NarrativeEventKind::Disaster);
        let radio_output = result
            .outputs
            .iter()
            .find(|o| o.kind == NarrativeEventKind::Radio);

        assert!(disaster_output.is_some());
        assert!(radio_output.is_some());
        assert!(disaster_output.unwrap().priority < radio_output.unwrap().priority);
    }

    #[test]
    fn anomaly_sighting_repeats() {
        let mut registry = EventRegistry::new();
        registry
            .register(
                EventDefinition::new("anomaly", NarrativeEventKind::Anomaly)
                    .with_trigger(NarrativeTrigger::always())
                    .with_cooldown(CooldownConfig::repeating(50))
                    .with_duration(20),
            )
            .unwrap();

        let mut state = NarrativeState::new(&registry);

        let result1 = state.tick(&registry, &NarrativeContext::new(0));
        assert_eq!(result1.triggered.len(), 1);

        state.tick(&registry, &NarrativeContext::new(20));

        let result2 = state.tick(&registry, &NarrativeContext::new(50));
        assert_eq!(result2.triggered.len(), 1);

        let result3 = state.tick(&registry, &NarrativeContext::new(100));
        assert_eq!(result3.triggered.len(), 1);
    }

    #[test]
    fn radio_chatter_with_jitter() {
        let mut registry = EventRegistry::new();
        registry
            .register(
                EventDefinition::new("chatter", NarrativeEventKind::Radio)
                    .with_trigger(NarrativeTrigger::always())
                    .with_cooldown(CooldownConfig::repeating(100).with_jitter(50))
                    .with_duration(10),
            )
            .unwrap();

        let mut state = NarrativeState::new(&registry);

        state.tick(&registry, &NarrativeContext::new(0).with_seed(42));

        let cooldown = state.cooldown_for("chatter").unwrap();
        assert!(cooldown.ready_at_tick >= 100);
        assert!(cooldown.ready_at_tick <= 150);
    }
}
