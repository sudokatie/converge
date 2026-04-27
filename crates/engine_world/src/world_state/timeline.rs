//! Deterministic world event timeline and scheduling.

use engine_core::coords::ChunkPos;
use serde::{Deserialize, Serialize};

use super::{ActiveEffect, ActiveEffects, Season, WorldEvent, WorldEventKind};

/// Configuration for timeline behavior.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimelineConfig {
    /// Ticks per season (0 disables automatic season cycling).
    pub ticks_per_season: u64,
    /// Maximum number of concurrent events.
    pub max_concurrent_events: usize,
    /// Whether to automatically remove completed events.
    pub auto_cleanup: bool,
}

impl Default for TimelineConfig {
    fn default() -> Self {
        Self {
            ticks_per_season: 86400,
            max_concurrent_events: 64,
            auto_cleanup: true,
        }
    }
}

impl TimelineConfig {
    /// Create a config with no automatic season cycling.
    #[must_use]
    pub fn no_seasons() -> Self {
        Self {
            ticks_per_season: 0,
            ..Default::default()
        }
    }

    /// Validate and clamp configuration values.
    pub fn validate(&mut self) {
        self.max_concurrent_events = self.max_concurrent_events.max(1);
    }
}

/// Deterministic world event timeline.
///
/// Manages scheduled events and provides queries for active effects.
/// All operations are deterministic given the same inputs.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldTimeline {
    /// Current world tick.
    current_tick: u64,
    /// Current season.
    current_season: Season,
    /// Tick when current season started.
    season_start_tick: u64,
    /// Next event ID to assign.
    next_event_id: u64,
    /// Scheduled events (sorted by start tick).
    events: Vec<WorldEvent>,
    /// Configuration.
    config: TimelineConfig,
}

impl WorldTimeline {
    /// Create a new timeline starting at tick 0.
    #[must_use]
    pub fn new(config: TimelineConfig) -> Self {
        let mut config = config;
        config.validate();
        Self {
            current_tick: 0,
            current_season: Season::default(),
            season_start_tick: 0,
            next_event_id: 1,
            events: Vec::new(),
            config,
        }
    }

    /// Get the current tick.
    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Get the current season.
    #[must_use]
    pub fn current_season(&self) -> Season {
        self.current_season
    }

    /// Get the configuration.
    #[must_use]
    pub fn config(&self) -> &TimelineConfig {
        &self.config
    }

    /// Get the number of scheduled events.
    #[must_use]
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Advance the timeline by one tick.
    ///
    /// Returns events that just started this tick.
    pub fn tick(&mut self) -> Vec<u64> {
        self.current_tick = self.current_tick.saturating_add(1);
        self.check_season_transition();

        let started: Vec<u64> = self
            .events
            .iter()
            .filter(|e| e.start_tick() == self.current_tick)
            .map(WorldEvent::id)
            .collect();

        if self.config.auto_cleanup {
            self.cleanup_completed();
        }

        started
    }

    /// Advance the timeline by multiple ticks.
    ///
    /// More efficient than calling `tick()` repeatedly for large advances.
    pub fn advance(&mut self, ticks: u64) {
        if ticks == 0 {
            return;
        }

        self.current_tick = self.current_tick.saturating_add(ticks);
        self.check_season_transition();

        if self.config.auto_cleanup {
            self.cleanup_completed();
        }
    }

    /// Set the current tick directly (for save/load).
    pub fn set_tick(&mut self, tick: u64) {
        self.current_tick = tick;
        self.check_season_transition();
    }

    /// Set the current season directly.
    pub fn set_season(&mut self, season: Season) {
        self.current_season = season;
        self.season_start_tick = self.current_tick;
    }

    /// Schedule a global event.
    ///
    /// Returns the assigned event ID.
    pub fn schedule_global(
        &mut self,
        kind: WorldEventKind,
        start_tick: u64,
        duration: u64,
    ) -> Option<u64> {
        if self.events.len() >= self.config.max_concurrent_events {
            return None;
        }

        let id = self.next_event_id;
        self.next_event_id = self.next_event_id.saturating_add(1);

        let event = WorldEvent::global(id, kind, start_tick, duration);
        self.insert_event(event);
        Some(id)
    }

    /// Schedule a regional event.
    ///
    /// Returns the assigned event ID.
    pub fn schedule_regional(
        &mut self,
        kind: WorldEventKind,
        start_tick: u64,
        duration: u64,
        center: ChunkPos,
        radius: i32,
    ) -> Option<u64> {
        if self.events.len() >= self.config.max_concurrent_events {
            return None;
        }

        let id = self.next_event_id;
        self.next_event_id = self.next_event_id.saturating_add(1);

        let event = WorldEvent::regional(id, kind, start_tick, duration, center, radius);
        self.insert_event(event);
        Some(id)
    }

    /// Schedule a season shift.
    ///
    /// Returns the assigned event ID.
    pub fn schedule_season_shift(&mut self, start_tick: u64, target: Season) -> Option<u64> {
        if self.events.len() >= self.config.max_concurrent_events {
            return None;
        }

        let id = self.next_event_id;
        self.next_event_id = self.next_event_id.saturating_add(1);

        let event = WorldEvent::season_shift(id, start_tick, target);
        self.insert_event(event);
        Some(id)
    }

    /// Schedule a pre-built event.
    ///
    /// The event's ID will be updated to ensure uniqueness.
    pub fn schedule(&mut self, event: &WorldEvent) -> Option<u64> {
        if self.events.len() >= self.config.max_concurrent_events {
            return None;
        }

        let id = self.next_event_id;
        self.next_event_id = self.next_event_id.saturating_add(1);

        let new_event = WorldEvent::global(id, event.kind(), event.start_tick(), event.duration());
        let mut new_event = if let Some(center) = event.center() {
            WorldEvent::regional(
                id,
                event.kind(),
                event.start_tick(),
                event.duration(),
                center,
                event.radius(),
            )
        } else {
            new_event
        };

        new_event.set_intensity(event.intensity());
        self.insert_event(new_event);
        Some(id)
    }

    /// Cancel an event by ID.
    ///
    /// Returns true if the event was found and removed.
    pub fn cancel(&mut self, event_id: u64) -> bool {
        if let Some(idx) = self.events.iter().position(|e| e.id() == event_id) {
            self.events.remove(idx);
            true
        } else {
            false
        }
    }

    /// Get an event by ID.
    #[must_use]
    pub fn get_event(&self, event_id: u64) -> Option<&WorldEvent> {
        self.events.iter().find(|e| e.id() == event_id)
    }

    /// Get a mutable reference to an event by ID.
    pub fn get_event_mut(&mut self, event_id: u64) -> Option<&mut WorldEvent> {
        self.events.iter_mut().find(|e| e.id() == event_id)
    }

    /// Query active effects at a position.
    #[must_use]
    pub fn query_effects(&self, pos: ChunkPos) -> ActiveEffects {
        self.query_effects_at(pos, self.current_tick)
    }

    /// Query active effects at a position and specific tick.
    #[must_use]
    pub fn query_effects_at(&self, pos: ChunkPos, tick: u64) -> ActiveEffects {
        let mut effects = ActiveEffects::with_capacity(4);

        for event in &self.events {
            if let Some(effect) = ActiveEffect::from_event(event, pos, tick) {
                effects.push(effect);
            }
        }

        effects
    }

    /// Check if any event of a kind is active at a position.
    #[must_use]
    pub fn is_active(&self, kind: WorldEventKind, pos: ChunkPos) -> bool {
        self.events
            .iter()
            .any(|e| e.kind() == kind && e.is_active_at(self.current_tick) && e.affects_chunk(pos))
    }

    /// Get all currently active events.
    pub fn active_events(&self) -> impl Iterator<Item = &WorldEvent> {
        self.events
            .iter()
            .filter(|e| e.is_active_at(self.current_tick))
    }

    /// Get all pending events (not yet started).
    pub fn pending_events(&self) -> impl Iterator<Item = &WorldEvent> {
        self.events
            .iter()
            .filter(|e| e.start_tick() > self.current_tick)
    }

    /// Get ticks until the next season change.
    #[must_use]
    pub fn ticks_until_season_change(&self) -> Option<u64> {
        if self.config.ticks_per_season == 0 {
            return None;
        }

        let elapsed = self.current_tick.saturating_sub(self.season_start_tick);
        Some(self.config.ticks_per_season.saturating_sub(elapsed))
    }

    /// Remove all completed events.
    pub fn cleanup_completed(&mut self) {
        self.events.retain(|e| !e.is_complete_at(self.current_tick));
    }

    /// Clear all events.
    pub fn clear(&mut self) {
        self.events.clear();
    }

    fn insert_event(&mut self, event: WorldEvent) {
        let start = event.start_tick();
        match self
            .events
            .binary_search_by_key(&start, WorldEvent::start_tick)
        {
            Ok(idx) | Err(idx) => self.events.insert(idx, event),
        }
    }

    fn check_season_transition(&mut self) {
        if self.config.ticks_per_season == 0 {
            return;
        }

        let elapsed = self.current_tick.saturating_sub(self.season_start_tick);
        if elapsed >= self.config.ticks_per_season {
            let seasons_passed = elapsed / self.config.ticks_per_season;
            for _ in 0..seasons_passed {
                self.current_season = self.current_season.next();
            }
            self.season_start_tick = self
                .current_tick
                .saturating_sub(elapsed % self.config.ticks_per_season);
        }

        for event in &self.events {
            if event.kind() == WorldEventKind::SeasonShift
                && event.start_tick() <= self.current_tick
                && let Some(target) = event.target_season()
            {
                self.current_season = target;
                self.season_start_tick = event.start_tick();
            }
        }
    }
}

impl Default for WorldTimeline {
    fn default() -> Self {
        Self::new(TimelineConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_timeline() {
        let timeline = WorldTimeline::default();
        assert_eq!(timeline.current_tick(), 0);
        assert_eq!(timeline.current_season(), Season::Spring);
        assert_eq!(timeline.event_count(), 0);
    }

    #[test]
    fn tick_advances() {
        let mut timeline = WorldTimeline::default();
        timeline.tick();
        assert_eq!(timeline.current_tick(), 1);
        timeline.tick();
        assert_eq!(timeline.current_tick(), 2);
    }

    #[test]
    fn advance_multiple() {
        let mut timeline = WorldTimeline::default();
        timeline.advance(100);
        assert_eq!(timeline.current_tick(), 100);
    }

    #[test]
    fn schedule_global_event() {
        let mut timeline = WorldTimeline::default();
        let id = timeline
            .schedule_global(WorldEventKind::Eclipse, 100, 200)
            .unwrap();

        assert_eq!(timeline.event_count(), 1);
        let event = timeline.get_event(id).unwrap();
        assert_eq!(event.kind(), WorldEventKind::Eclipse);
        assert_eq!(event.start_tick(), 100);
        assert_eq!(event.duration(), 200);
    }

    #[test]
    fn schedule_regional_event() {
        let mut timeline = WorldTimeline::default();
        let center = ChunkPos::new(10, 10, 10);
        let id = timeline
            .schedule_regional(WorldEventKind::Collapse, 50, 100, center, 5)
            .unwrap();

        let event = timeline.get_event(id).unwrap();
        assert_eq!(event.center(), Some(center));
        assert_eq!(event.radius(), 5);
    }

    #[test]
    fn schedule_season_shift() {
        let mut timeline = WorldTimeline::default();
        let id = timeline.schedule_season_shift(500, Season::Winter).unwrap();

        let event = timeline.get_event(id).unwrap();
        assert_eq!(event.kind(), WorldEventKind::SeasonShift);
        assert_eq!(event.target_season(), Some(Season::Winter));
    }

    #[test]
    fn cancel_event() {
        let mut timeline = WorldTimeline::default();
        let id = timeline
            .schedule_global(WorldEventKind::Eclipse, 100, 200)
            .unwrap();

        assert!(timeline.cancel(id));
        assert_eq!(timeline.event_count(), 0);
        assert!(!timeline.cancel(id));
    }

    #[test]
    fn query_effects() {
        let mut timeline = WorldTimeline::default();
        timeline.schedule_global(WorldEventKind::Eclipse, 100, 200);

        timeline.advance(50);
        let effects = timeline.query_effects(ChunkPos::new(0, 0, 0));
        assert!(effects.is_empty());

        timeline.advance(100);
        let effects = timeline.query_effects(ChunkPos::new(0, 0, 0));
        assert!(!effects.is_empty());
        assert!(effects.has_kind(WorldEventKind::Eclipse));
    }

    #[test]
    fn query_effects_regional() {
        let mut timeline = WorldTimeline::default();
        let center = ChunkPos::new(10, 10, 10);
        timeline.schedule_regional(WorldEventKind::Collapse, 0, 100, center, 5);

        timeline.tick();

        let in_range = timeline.query_effects(ChunkPos::new(10, 12, 10));
        assert!(!in_range.is_empty());

        let out_of_range = timeline.query_effects(ChunkPos::new(0, 0, 0));
        assert!(out_of_range.is_empty());
    }

    #[test]
    fn is_active() {
        let mut timeline = WorldTimeline::default();
        timeline.schedule_global(WorldEventKind::Eclipse, 100, 200);
        let pos = ChunkPos::new(0, 0, 0);

        timeline.advance(50);
        assert!(!timeline.is_active(WorldEventKind::Eclipse, pos));

        timeline.advance(100);
        assert!(timeline.is_active(WorldEventKind::Eclipse, pos));

        timeline.advance(200);
        assert!(!timeline.is_active(WorldEventKind::Eclipse, pos));
    }

    #[test]
    fn active_events_iter() {
        let mut timeline = WorldTimeline::default();
        timeline.schedule_global(WorldEventKind::Eclipse, 0, 100);
        timeline.schedule_global(WorldEventKind::MigrationWave, 50, 100);

        timeline.tick();
        assert_eq!(timeline.active_events().count(), 1);

        timeline.advance(50);
        assert_eq!(timeline.active_events().count(), 2);
    }

    #[test]
    fn auto_cleanup() {
        let config = TimelineConfig {
            auto_cleanup: true,
            ..Default::default()
        };
        let mut timeline = WorldTimeline::new(config);
        timeline.schedule_global(WorldEventKind::Eclipse, 0, 50);

        timeline.advance(100);
        assert_eq!(timeline.event_count(), 0);
    }

    #[test]
    fn no_auto_cleanup() {
        let config = TimelineConfig {
            auto_cleanup: false,
            ..Default::default()
        };
        let mut timeline = WorldTimeline::new(config);
        timeline.schedule_global(WorldEventKind::Eclipse, 0, 50);

        timeline.advance(100);
        assert_eq!(timeline.event_count(), 1);

        timeline.cleanup_completed();
        assert_eq!(timeline.event_count(), 0);
    }

    #[test]
    fn season_transition_automatic() {
        let config = TimelineConfig {
            ticks_per_season: 100,
            ..Default::default()
        };
        let mut timeline = WorldTimeline::new(config);

        assert_eq!(timeline.current_season(), Season::Spring);
        timeline.advance(100);
        assert_eq!(timeline.current_season(), Season::Summer);
        timeline.advance(100);
        assert_eq!(timeline.current_season(), Season::Autumn);
    }

    #[test]
    fn season_transition_forced() {
        let mut timeline = WorldTimeline::default();
        timeline.schedule_season_shift(50, Season::Winter);

        timeline.advance(51);
        assert_eq!(timeline.current_season(), Season::Winter);
    }

    #[test]
    fn ticks_until_season_change() {
        let config = TimelineConfig {
            ticks_per_season: 100,
            ..Default::default()
        };
        let mut timeline = WorldTimeline::new(config);

        assert_eq!(timeline.ticks_until_season_change(), Some(100));
        timeline.advance(30);
        assert_eq!(timeline.ticks_until_season_change(), Some(70));
    }

    #[test]
    fn ticks_until_season_change_disabled() {
        let config = TimelineConfig::no_seasons();
        let timeline = WorldTimeline::new(config);
        assert_eq!(timeline.ticks_until_season_change(), None);
    }

    #[test]
    fn max_concurrent_events() {
        let config = TimelineConfig {
            max_concurrent_events: 2,
            ..Default::default()
        };
        let mut timeline = WorldTimeline::new(config);

        assert!(
            timeline
                .schedule_global(WorldEventKind::Eclipse, 0, 100)
                .is_some()
        );
        assert!(
            timeline
                .schedule_global(WorldEventKind::Collapse, 0, 100)
                .is_some()
        );
        assert!(
            timeline
                .schedule_global(WorldEventKind::MigrationWave, 0, 100)
                .is_none()
        );
    }

    #[test]
    fn events_sorted_by_start() {
        let mut timeline = WorldTimeline::default();
        timeline.schedule_global(WorldEventKind::Eclipse, 200, 100);
        timeline.schedule_global(WorldEventKind::Collapse, 50, 100);
        timeline.schedule_global(WorldEventKind::MigrationWave, 100, 100);

        let starts: Vec<u64> = timeline
            .pending_events()
            .map(WorldEvent::start_tick)
            .collect();
        assert_eq!(starts, vec![50, 100, 200]);
    }

    #[test]
    fn tick_returns_started_events() {
        let mut timeline = WorldTimeline::default();
        let id1 = timeline
            .schedule_global(WorldEventKind::Eclipse, 5, 100)
            .unwrap();
        let id2 = timeline
            .schedule_global(WorldEventKind::Collapse, 5, 100)
            .unwrap();
        timeline.schedule_global(WorldEventKind::MigrationWave, 10, 100);

        for _ in 0..4 {
            let started = timeline.tick();
            assert!(started.is_empty());
        }

        let started = timeline.tick();
        assert_eq!(started.len(), 2);
        assert!(started.contains(&id1));
        assert!(started.contains(&id2));
    }

    #[test]
    fn serde_round_trip() {
        let mut timeline = WorldTimeline::default();
        timeline.schedule_global(WorldEventKind::Eclipse, 100, 200);
        timeline.schedule_regional(
            WorldEventKind::Collapse,
            50,
            150,
            ChunkPos::new(5, 5, 5),
            10,
        );
        timeline.advance(75);

        let json = serde_json::to_string(&timeline).unwrap();
        let recovered: WorldTimeline = serde_json::from_str(&json).unwrap();

        assert_eq!(recovered.current_tick(), timeline.current_tick());
        assert_eq!(recovered.current_season(), timeline.current_season());
        assert_eq!(recovered.event_count(), timeline.event_count());
    }
}
