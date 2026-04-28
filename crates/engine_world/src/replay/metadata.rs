//! Replay session metadata.

use serde::{Deserialize, Serialize};

use crate::world_state::{Season, TimelineConfig};

/// Metadata for a replay session.
///
/// Captures session-level information needed to reproduce simulation state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReplayMetadata {
    /// Seed or rules identifier for deterministic generation.
    seed_identifier: String,
    /// Starting tick of the recording.
    start_tick: u64,
    /// Ending tick of the recording (inclusive).
    end_tick: u64,
    /// Timeline configuration at session start.
    timeline_config: Option<TimelineConfig>,
    /// Season at session start.
    start_season: Option<Season>,
    /// Observer positions at session start (serialized as tuples).
    observer_positions: Vec<(i32, i32, i32)>,
    /// Number of scheduled world events at session start.
    scheduled_event_count: usize,
    /// Custom tags for identifying replay sessions.
    tags: Vec<String>,
}

impl ReplayMetadata {
    /// Create new metadata with required fields.
    #[must_use]
    pub fn new(seed_identifier: impl Into<String>, start_tick: u64, end_tick: u64) -> Self {
        Self {
            seed_identifier: seed_identifier.into(),
            start_tick,
            end_tick,
            timeline_config: None,
            start_season: None,
            observer_positions: Vec::new(),
            scheduled_event_count: 0,
            tags: Vec::new(),
        }
    }

    /// Get the seed/rules identifier.
    #[must_use]
    pub fn seed_identifier(&self) -> &str {
        &self.seed_identifier
    }

    /// Get the starting tick.
    #[must_use]
    pub fn start_tick(&self) -> u64 {
        self.start_tick
    }

    /// Get the ending tick.
    #[must_use]
    pub fn end_tick(&self) -> u64 {
        self.end_tick
    }

    /// Get the tick range (end - start).
    #[must_use]
    pub fn tick_range(&self) -> u64 {
        self.end_tick.saturating_sub(self.start_tick)
    }

    /// Get the timeline configuration.
    #[must_use]
    pub fn timeline_config(&self) -> Option<&TimelineConfig> {
        self.timeline_config.as_ref()
    }

    /// Set the timeline configuration.
    pub fn set_timeline_config(&mut self, config: TimelineConfig) {
        self.timeline_config = Some(config);
    }

    /// Get the starting season.
    #[must_use]
    pub fn start_season(&self) -> Option<Season> {
        self.start_season
    }

    /// Set the starting season.
    pub fn set_start_season(&mut self, season: Season) {
        self.start_season = Some(season);
    }

    /// Get the observer positions.
    #[must_use]
    pub fn observer_positions(&self) -> &[(i32, i32, i32)] {
        &self.observer_positions
    }

    /// Set the observer positions.
    pub fn set_observer_positions(&mut self, positions: Vec<(i32, i32, i32)>) {
        self.observer_positions = positions;
    }

    /// Add an observer position.
    pub fn add_observer(&mut self, x: i32, y: i32, z: i32) {
        self.observer_positions.push((x, y, z));
    }

    /// Get the scheduled event count.
    #[must_use]
    pub fn scheduled_event_count(&self) -> usize {
        self.scheduled_event_count
    }

    /// Set the scheduled event count.
    pub fn set_scheduled_event_count(&mut self, count: usize) {
        self.scheduled_event_count = count;
    }

    /// Get the tags.
    #[must_use]
    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    /// Add a tag.
    pub fn add_tag(&mut self, tag: impl Into<String>) {
        self.tags.push(tag.into());
    }

    /// Set the ending tick (for updating after recording completes).
    pub fn set_end_tick(&mut self, tick: u64) {
        self.end_tick = tick;
    }

    /// Builder-style: set timeline config.
    #[must_use]
    pub fn with_timeline_config(mut self, config: TimelineConfig) -> Self {
        self.timeline_config = Some(config);
        self
    }

    /// Builder-style: set start season.
    #[must_use]
    pub fn with_start_season(mut self, season: Season) -> Self {
        self.start_season = Some(season);
        self
    }

    /// Builder-style: set observer positions.
    #[must_use]
    pub fn with_observers(mut self, positions: Vec<(i32, i32, i32)>) -> Self {
        self.observer_positions = positions;
        self
    }

    /// Builder-style: set scheduled event count.
    #[must_use]
    pub fn with_scheduled_event_count(mut self, count: usize) -> Self {
        self.scheduled_event_count = count;
        self
    }

    /// Builder-style: add tags.
    #[must_use]
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}

impl Default for ReplayMetadata {
    fn default() -> Self {
        Self::new("default", 0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_metadata() {
        let meta = ReplayMetadata::new("test_seed", 100, 500);
        assert_eq!(meta.seed_identifier(), "test_seed");
        assert_eq!(meta.start_tick(), 100);
        assert_eq!(meta.end_tick(), 500);
        assert_eq!(meta.tick_range(), 400);
    }

    #[test]
    fn builder_pattern() {
        let meta = ReplayMetadata::new("seed", 0, 1000)
            .with_start_season(Season::Winter)
            .with_observers(vec![(0, 0, 0), (10, 5, 10)])
            .with_scheduled_event_count(3)
            .with_tags(vec!["debug".to_string(), "physics".to_string()]);

        assert_eq!(meta.start_season(), Some(Season::Winter));
        assert_eq!(meta.observer_positions().len(), 2);
        assert_eq!(meta.scheduled_event_count(), 3);
        assert_eq!(meta.tags().len(), 2);
    }

    #[test]
    fn setters() {
        let mut meta = ReplayMetadata::default();
        meta.set_timeline_config(TimelineConfig::default());
        meta.set_start_season(Season::Autumn);
        meta.add_observer(5, 10, 15);
        meta.set_scheduled_event_count(7);
        meta.add_tag("test");
        meta.set_end_tick(999);

        assert!(meta.timeline_config().is_some());
        assert_eq!(meta.start_season(), Some(Season::Autumn));
        assert_eq!(meta.observer_positions(), &[(5, 10, 15)]);
        assert_eq!(meta.scheduled_event_count(), 7);
        assert_eq!(meta.tags(), &["test"]);
        assert_eq!(meta.end_tick(), 999);
    }

    #[test]
    fn serde_round_trip() {
        let meta = ReplayMetadata::new("complex_seed", 50, 2000)
            .with_timeline_config(TimelineConfig::default())
            .with_start_season(Season::Summer)
            .with_observers(vec![(1, 2, 3)])
            .with_scheduled_event_count(5)
            .with_tags(vec!["desync_debug".to_string()]);

        let json = serde_json::to_string(&meta).unwrap();
        let recovered: ReplayMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, meta);
    }

    #[test]
    fn tick_range_saturates() {
        let meta = ReplayMetadata::new("x", 100, 50);
        assert_eq!(meta.tick_range(), 0);
    }
}
