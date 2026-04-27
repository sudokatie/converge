//! Active effect tracking for world events.

use engine_core::coords::ChunkPos;
use serde::{Deserialize, Serialize};

use super::{Season, WorldEvent, WorldEventKind};

/// An active effect derived from a world event at a specific time.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiveEffect {
    /// Source event ID.
    event_id: u64,
    /// Type of effect.
    kind: WorldEventKind,
    /// Current intensity at the queried position (0.0 to 1.0).
    intensity: f32,
    /// Progress through the event (0.0 to 1.0).
    progress: f32,
    /// Ticks remaining until event ends.
    ticks_remaining: u64,
    /// Target season for season shifts.
    target_season: Option<Season>,
}

impl ActiveEffect {
    /// Create an active effect from an event at a position and time.
    #[must_use]
    pub fn from_event(event: &WorldEvent, pos: ChunkPos, tick: u64) -> Option<Self> {
        if !event.is_active_at(tick) || !event.affects_chunk(pos) {
            return None;
        }

        let intensity = event.intensity_at(pos);
        if intensity <= 0.0 {
            return None;
        }

        let progress = event.progress_at(tick);
        let ticks_remaining = event.end_tick().saturating_sub(tick);

        Some(Self {
            event_id: event.id(),
            kind: event.kind(),
            intensity,
            progress,
            ticks_remaining,
            target_season: event.target_season(),
        })
    }

    /// Get the source event ID.
    #[must_use]
    pub fn event_id(&self) -> u64 {
        self.event_id
    }

    /// Get the effect kind.
    #[must_use]
    pub fn kind(&self) -> WorldEventKind {
        self.kind
    }

    /// Get the intensity at this position.
    #[must_use]
    pub fn intensity(&self) -> f32 {
        self.intensity
    }

    /// Get the progress through the event.
    #[must_use]
    pub fn progress(&self) -> f32 {
        self.progress
    }

    /// Get the ticks remaining.
    #[must_use]
    pub fn ticks_remaining(&self) -> u64 {
        self.ticks_remaining
    }

    /// Get the target season (for season shifts).
    #[must_use]
    pub fn target_season(&self) -> Option<Season> {
        self.target_season
    }
}

/// Collection of active effects at a position.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ActiveEffects {
    effects: Vec<ActiveEffect>,
}

impl ActiveEffects {
    /// Create an empty effects collection.
    #[must_use]
    pub fn new() -> Self {
        Self {
            effects: Vec::new(),
        }
    }

    /// Create with pre-allocated capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            effects: Vec::with_capacity(capacity),
        }
    }

    /// Add an effect.
    pub fn push(&mut self, effect: ActiveEffect) {
        self.effects.push(effect);
    }

    /// Get the number of active effects.
    #[must_use]
    pub fn len(&self) -> usize {
        self.effects.len()
    }

    /// Check if there are no active effects.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    /// Iterate over active effects.
    pub fn iter(&self) -> impl Iterator<Item = &ActiveEffect> {
        self.effects.iter()
    }

    /// Check if any effect of the given kind is active.
    #[must_use]
    pub fn has_kind(&self, kind: WorldEventKind) -> bool {
        self.effects.iter().any(|e| e.kind == kind)
    }

    /// Get the strongest effect of a given kind.
    #[must_use]
    pub fn strongest(&self, kind: WorldEventKind) -> Option<&ActiveEffect> {
        self.effects
            .iter()
            .filter(|e| e.kind == kind)
            .max_by(|a, b| a.intensity.total_cmp(&b.intensity))
    }

    /// Get total intensity for a kind (sum of all matching effects).
    #[must_use]
    pub fn total_intensity(&self, kind: WorldEventKind) -> f32 {
        self.effects
            .iter()
            .filter(|e| e.kind == kind)
            .map(|e| e.intensity)
            .sum()
    }

    /// Get effects affecting lighting.
    pub fn lighting_effects(&self) -> impl Iterator<Item = &ActiveEffect> {
        self.effects.iter().filter(|e| e.kind.affects_lighting())
    }

    /// Get effects affecting temperature.
    pub fn temperature_effects(&self) -> impl Iterator<Item = &ActiveEffect> {
        self.effects.iter().filter(|e| e.kind.affects_temperature())
    }

    /// Get effects affecting structural stability.
    pub fn structural_effects(&self) -> impl Iterator<Item = &ActiveEffect> {
        self.effects.iter().filter(|e| e.kind.affects_structure())
    }

    /// Get effects affecting hazards.
    pub fn hazard_effects(&self) -> impl Iterator<Item = &ActiveEffect> {
        self.effects.iter().filter(|e| e.kind.affects_hazards())
    }

    /// Get effects affecting entities.
    pub fn entity_effects(&self) -> impl Iterator<Item = &ActiveEffect> {
        self.effects.iter().filter(|e| e.kind.affects_entities())
    }
}

impl IntoIterator for ActiveEffects {
    type Item = ActiveEffect;
    type IntoIter = std::vec::IntoIter<ActiveEffect>;

    fn into_iter(self) -> Self::IntoIter {
        self.effects.into_iter()
    }
}

impl<'a> IntoIterator for &'a ActiveEffects {
    type Item = &'a ActiveEffect;
    type IntoIter = std::slice::Iter<'a, ActiveEffect>;

    fn into_iter(self) -> Self::IntoIter {
        self.effects.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_event() -> WorldEvent {
        WorldEvent::global(1, WorldEventKind::Eclipse, 100, 200)
    }

    #[test]
    fn from_event_active() {
        let event = make_test_event();
        let pos = ChunkPos::new(0, 0, 0);

        let effect = ActiveEffect::from_event(&event, pos, 150);
        assert!(effect.is_some());

        let effect = effect.unwrap();
        assert_eq!(effect.event_id(), 1);
        assert_eq!(effect.kind(), WorldEventKind::Eclipse);
        assert!((effect.intensity() - 1.0).abs() < 0.001);
        assert!((effect.progress() - 0.25).abs() < 0.001);
        assert_eq!(effect.ticks_remaining(), 150);
    }

    #[test]
    fn from_event_not_active() {
        let event = make_test_event();
        let pos = ChunkPos::new(0, 0, 0);

        assert!(ActiveEffect::from_event(&event, pos, 50).is_none());
        assert!(ActiveEffect::from_event(&event, pos, 300).is_none());
    }

    #[test]
    fn from_event_regional_outside() {
        let event = WorldEvent::regional(
            1,
            WorldEventKind::Collapse,
            100,
            200,
            ChunkPos::new(10, 10, 10),
            5,
        );
        let pos = ChunkPos::new(0, 0, 0);

        assert!(ActiveEffect::from_event(&event, pos, 150).is_none());
    }

    #[test]
    fn active_effects_empty() {
        let effects = ActiveEffects::new();
        assert!(effects.is_empty());
        assert_eq!(effects.len(), 0);
    }

    #[test]
    fn active_effects_push() {
        let mut effects = ActiveEffects::new();
        let event = make_test_event();
        let effect = ActiveEffect::from_event(&event, ChunkPos::new(0, 0, 0), 150).unwrap();
        effects.push(effect);

        assert!(!effects.is_empty());
        assert_eq!(effects.len(), 1);
    }

    #[test]
    fn has_kind() {
        let mut effects = ActiveEffects::new();
        let event = make_test_event();
        effects.push(ActiveEffect::from_event(&event, ChunkPos::new(0, 0, 0), 150).unwrap());

        assert!(effects.has_kind(WorldEventKind::Eclipse));
        assert!(!effects.has_kind(WorldEventKind::Collapse));
    }

    #[test]
    fn strongest_effect() {
        let mut effects = ActiveEffects::new();

        let mut event1 = WorldEvent::global(1, WorldEventKind::Eclipse, 0, 100);
        event1.set_intensity(0.5);
        let mut event2 = WorldEvent::global(2, WorldEventKind::Eclipse, 0, 100);
        event2.set_intensity(0.8);

        let pos = ChunkPos::new(0, 0, 0);
        effects.push(ActiveEffect::from_event(&event1, pos, 50).unwrap());
        effects.push(ActiveEffect::from_event(&event2, pos, 50).unwrap());

        let strongest = effects.strongest(WorldEventKind::Eclipse).unwrap();
        assert_eq!(strongest.event_id(), 2);
    }

    #[test]
    fn total_intensity() {
        let mut effects = ActiveEffects::new();

        let mut event1 = WorldEvent::global(1, WorldEventKind::Eclipse, 0, 100);
        event1.set_intensity(0.3);
        let mut event2 = WorldEvent::global(2, WorldEventKind::Eclipse, 0, 100);
        event2.set_intensity(0.5);

        let pos = ChunkPos::new(0, 0, 0);
        effects.push(ActiveEffect::from_event(&event1, pos, 50).unwrap());
        effects.push(ActiveEffect::from_event(&event2, pos, 50).unwrap());

        let total = effects.total_intensity(WorldEventKind::Eclipse);
        assert!((total - 0.8).abs() < 0.001);
    }

    #[test]
    fn effect_filters() {
        let mut effects = ActiveEffects::new();
        let pos = ChunkPos::new(0, 0, 0);

        let eclipse = WorldEvent::global(1, WorldEventKind::Eclipse, 0, 100);
        let collapse = WorldEvent::regional(2, WorldEventKind::Collapse, 0, 100, pos, 10);

        effects.push(ActiveEffect::from_event(&eclipse, pos, 50).unwrap());
        effects.push(ActiveEffect::from_event(&collapse, pos, 50).unwrap());

        assert_eq!(effects.lighting_effects().count(), 1);
        assert_eq!(effects.structural_effects().count(), 1);
    }

    #[test]
    fn into_iter() {
        let mut effects = ActiveEffects::new();
        let event = make_test_event();
        effects.push(ActiveEffect::from_event(&event, ChunkPos::new(0, 0, 0), 150).unwrap());

        let collected: Vec<_> = effects.into_iter().collect();
        assert_eq!(collected.len(), 1);
    }

    #[test]
    fn serde_round_trip() {
        let mut effects = ActiveEffects::new();
        let event = make_test_event();
        effects.push(ActiveEffect::from_event(&event, ChunkPos::new(0, 0, 0), 150).unwrap());

        let json = serde_json::to_string(&effects).unwrap();
        let recovered: ActiveEffects = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.len(), 1);
    }
}
