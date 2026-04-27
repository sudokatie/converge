//! World event scheduling and state.

use engine_core::coords::ChunkPos;
use serde::{Deserialize, Serialize};

use super::{Season, WorldEventKind};

/// A scheduled world event with timing and optional region.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldEvent {
    /// Unique event identifier.
    id: u64,
    /// Type of event.
    kind: WorldEventKind,
    /// World tick when event starts.
    start_tick: u64,
    /// Duration in world ticks (0 = instantaneous).
    duration: u64,
    /// Intensity of the event (0.0 to 1.0).
    intensity: f32,
    /// Center position for regional events (None for global).
    center: Option<ChunkPos>,
    /// Radius in chunks for regional events.
    radius: i32,
    /// For season shifts: target season.
    target_season: Option<Season>,
}

impl WorldEvent {
    /// Create a new global event.
    #[must_use]
    pub fn global(id: u64, kind: WorldEventKind, start_tick: u64, duration: u64) -> Self {
        Self {
            id,
            kind,
            start_tick,
            duration,
            intensity: 1.0,
            center: None,
            radius: 0,
            target_season: None,
        }
    }

    /// Create a new regional event.
    #[must_use]
    pub fn regional(
        id: u64,
        kind: WorldEventKind,
        start_tick: u64,
        duration: u64,
        center: ChunkPos,
        radius: i32,
    ) -> Self {
        Self {
            id,
            kind,
            start_tick,
            duration,
            intensity: 1.0,
            center: Some(center),
            radius: radius.max(0),
            target_season: None,
        }
    }

    /// Create a season shift event.
    #[must_use]
    pub fn season_shift(id: u64, start_tick: u64, target: Season) -> Self {
        Self {
            id,
            kind: WorldEventKind::SeasonShift,
            start_tick,
            duration: 0,
            intensity: 1.0,
            center: None,
            radius: 0,
            target_season: Some(target),
        }
    }

    /// Get the event ID.
    #[must_use]
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get the event kind.
    #[must_use]
    pub fn kind(&self) -> WorldEventKind {
        self.kind
    }

    /// Get the start tick.
    #[must_use]
    pub fn start_tick(&self) -> u64 {
        self.start_tick
    }

    /// Get the duration in ticks.
    #[must_use]
    pub fn duration(&self) -> u64 {
        self.duration
    }

    /// Get the end tick (exclusive).
    #[must_use]
    pub fn end_tick(&self) -> u64 {
        self.start_tick.saturating_add(self.duration)
    }

    /// Get the intensity.
    #[must_use]
    pub fn intensity(&self) -> f32 {
        self.intensity
    }

    /// Get the center position (for regional events).
    #[must_use]
    pub fn center(&self) -> Option<ChunkPos> {
        self.center
    }

    /// Get the radius in chunks.
    #[must_use]
    pub fn radius(&self) -> i32 {
        self.radius
    }

    /// Get the target season (for season shifts).
    #[must_use]
    pub fn target_season(&self) -> Option<Season> {
        self.target_season
    }

    /// Check if this is a global event.
    #[must_use]
    pub fn is_global(&self) -> bool {
        self.center.is_none()
    }

    /// Check if this event is active at the given tick.
    #[must_use]
    pub fn is_active_at(&self, tick: u64) -> bool {
        if self.duration == 0 {
            tick == self.start_tick
        } else {
            tick >= self.start_tick && tick < self.end_tick()
        }
    }

    /// Check if this event has completed by the given tick.
    #[must_use]
    pub fn is_complete_at(&self, tick: u64) -> bool {
        tick >= self.end_tick()
    }

    /// Check if a chunk position is affected by this event.
    #[must_use]
    pub fn affects_chunk(&self, pos: ChunkPos) -> bool {
        match self.center {
            None => true,
            Some(center) => {
                let dx = (pos.x() - center.x()).abs();
                let dy = (pos.y() - center.y()).abs();
                let dz = (pos.z() - center.z()).abs();
                dx.max(dy).max(dz) <= self.radius
            }
        }
    }

    /// Get intensity at a specific chunk position (falloff for regional).
    #[must_use]
    pub fn intensity_at(&self, pos: ChunkPos) -> f32 {
        match self.center {
            None => self.intensity,
            Some(center) => {
                let dx = (pos.x() - center.x()).abs();
                let dy = (pos.y() - center.y()).abs();
                let dz = (pos.z() - center.z()).abs();
                let dist = dx.max(dy).max(dz);
                if dist > self.radius {
                    return 0.0;
                }
                if self.radius == 0 {
                    return self.intensity;
                }
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "radius values are small, precision loss acceptable"
                )]
                let falloff = 1.0 - (dist as f32 / self.radius as f32);
                self.intensity * falloff
            }
        }
    }

    /// Get progress through the event (0.0 to 1.0).
    #[must_use]
    pub fn progress_at(&self, tick: u64) -> f32 {
        if self.duration == 0 {
            return if tick >= self.start_tick { 1.0 } else { 0.0 };
        }
        if tick < self.start_tick {
            return 0.0;
        }
        let elapsed = tick.saturating_sub(self.start_tick);
        #[expect(
            clippy::cast_precision_loss,
            reason = "tick values typically small enough for f32"
        )]
        let progress = elapsed as f32 / self.duration as f32;
        progress.clamp(0.0, 1.0)
    }

    /// Set the intensity (clamped to 0.0-1.0).
    pub fn set_intensity(&mut self, intensity: f32) {
        self.intensity = intensity.clamp(0.0, 1.0);
    }

    /// Set the radius (clamped to non-negative).
    pub fn set_radius(&mut self, radius: i32) {
        self.radius = radius.max(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_event_creation() {
        let event = WorldEvent::global(1, WorldEventKind::Eclipse, 100, 500);
        assert_eq!(event.id(), 1);
        assert_eq!(event.kind(), WorldEventKind::Eclipse);
        assert_eq!(event.start_tick(), 100);
        assert_eq!(event.duration(), 500);
        assert_eq!(event.end_tick(), 600);
        assert!(event.is_global());
        assert!(event.center().is_none());
    }

    #[test]
    fn regional_event_creation() {
        let center = ChunkPos::new(10, 5, 10);
        let event = WorldEvent::regional(2, WorldEventKind::Collapse, 200, 300, center, 8);
        assert_eq!(event.id(), 2);
        assert!(!event.is_global());
        assert_eq!(event.center(), Some(center));
        assert_eq!(event.radius(), 8);
    }

    #[test]
    fn season_shift_creation() {
        let event = WorldEvent::season_shift(3, 1000, Season::Winter);
        assert_eq!(event.kind(), WorldEventKind::SeasonShift);
        assert_eq!(event.target_season(), Some(Season::Winter));
        assert_eq!(event.duration(), 0);
    }

    #[test]
    fn is_active_at_duration() {
        let event = WorldEvent::global(1, WorldEventKind::Eclipse, 100, 200);
        assert!(!event.is_active_at(99));
        assert!(event.is_active_at(100));
        assert!(event.is_active_at(200));
        assert!(event.is_active_at(299));
        assert!(!event.is_active_at(300));
    }

    #[test]
    fn is_active_at_instantaneous() {
        let event = WorldEvent::season_shift(1, 500, Season::Summer);
        assert!(!event.is_active_at(499));
        assert!(event.is_active_at(500));
        assert!(!event.is_active_at(501));
    }

    #[test]
    fn is_complete_at() {
        let event = WorldEvent::global(1, WorldEventKind::MigrationWave, 100, 50);
        assert!(!event.is_complete_at(100));
        assert!(!event.is_complete_at(149));
        assert!(event.is_complete_at(150));
        assert!(event.is_complete_at(200));
    }

    #[test]
    fn affects_chunk_global() {
        let event = WorldEvent::global(1, WorldEventKind::Eclipse, 0, 100);
        assert!(event.affects_chunk(ChunkPos::new(0, 0, 0)));
        assert!(event.affects_chunk(ChunkPos::new(1000, -500, 2000)));
    }

    #[test]
    fn affects_chunk_regional() {
        let center = ChunkPos::new(10, 10, 10);
        let event = WorldEvent::regional(1, WorldEventKind::Collapse, 0, 100, center, 5);

        assert!(event.affects_chunk(center));
        assert!(event.affects_chunk(ChunkPos::new(10, 15, 10)));
        assert!(event.affects_chunk(ChunkPos::new(5, 10, 10)));
        assert!(!event.affects_chunk(ChunkPos::new(10, 16, 10)));
        assert!(!event.affects_chunk(ChunkPos::new(0, 10, 10)));
    }

    #[test]
    fn intensity_at_global() {
        let mut event = WorldEvent::global(1, WorldEventKind::Eclipse, 0, 100);
        event.set_intensity(0.8);
        assert!((event.intensity_at(ChunkPos::new(0, 0, 0)) - 0.8).abs() < 0.001);
        assert!((event.intensity_at(ChunkPos::new(1000, 1000, 1000)) - 0.8).abs() < 0.001);
    }

    #[test]
    fn intensity_at_regional_falloff() {
        let center = ChunkPos::new(10, 10, 10);
        let event = WorldEvent::regional(1, WorldEventKind::BiomeCorruption, 0, 100, center, 10);

        let at_center = event.intensity_at(center);
        let at_edge = event.intensity_at(ChunkPos::new(20, 10, 10));
        let outside = event.intensity_at(ChunkPos::new(21, 10, 10));

        assert!((at_center - 1.0).abs() < 0.001);
        assert!(at_edge.abs() < 0.001);
        assert!(outside.abs() < 0.001);
    }

    #[test]
    fn progress_at() {
        let event = WorldEvent::global(1, WorldEventKind::Eclipse, 100, 200);

        assert!(event.progress_at(50).abs() < 0.001);
        assert!(event.progress_at(100).abs() < 0.001);
        assert!((event.progress_at(200) - 0.5).abs() < 0.001);
        assert!((event.progress_at(300) - 1.0).abs() < 0.001);
        assert!((event.progress_at(400) - 1.0).abs() < 0.001);
    }

    #[test]
    fn set_intensity_clamps() {
        let mut event = WorldEvent::global(1, WorldEventKind::Eclipse, 0, 100);
        event.set_intensity(1.5);
        assert!((event.intensity() - 1.0).abs() < 0.001);
        event.set_intensity(-0.5);
        assert!(event.intensity().abs() < 0.001);
    }

    #[test]
    fn set_radius_clamps() {
        let center = ChunkPos::new(0, 0, 0);
        let mut event = WorldEvent::regional(1, WorldEventKind::Collapse, 0, 100, center, 5);
        event.set_radius(-10);
        assert_eq!(event.radius(), 0);
    }

    #[test]
    fn serde_round_trip() {
        let events = [
            WorldEvent::global(1, WorldEventKind::Eclipse, 100, 500),
            WorldEvent::regional(
                2,
                WorldEventKind::BiomeCorruption,
                0,
                1000,
                ChunkPos::new(5, 5, 5),
                15,
            ),
            WorldEvent::season_shift(3, 2000, Season::Autumn),
        ];

        for event in events {
            let json = serde_json::to_string(&event).unwrap();
            let recovered: WorldEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(recovered, event);
        }
    }
}
