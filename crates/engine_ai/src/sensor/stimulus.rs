//! Stimuli emitted into the environment for sensors to detect.

use super::SensorKind;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Unique identifier for a stimulus instance.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StimulusId(pub u64);

impl StimulusId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Identifies the source entity that emitted a stimulus.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StimulusSource(pub u64);

impl StimulusSource {
    #[must_use]
    pub fn new(entity_id: u64) -> Self {
        Self(entity_id)
    }

    /// Create an anonymous/environmental source.
    #[must_use]
    pub fn environmental() -> Self {
        Self(0)
    }

    /// Check if this is an environmental (non-entity) source.
    #[must_use]
    pub fn is_environmental(&self) -> bool {
        self.0 == 0
    }
}

/// A stimulus emitted into the environment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stimulus {
    /// Unique identifier for this stimulus instance.
    pub id: StimulusId,
    /// The sensor channel this stimulus targets.
    pub kind: SensorKind,
    /// Source entity that emitted this stimulus.
    pub source: StimulusSource,
    /// World position where stimulus originates (x, y, z).
    pub position: [f32; 3],
    /// Base intensity at the source (before attenuation).
    pub intensity: f32,
    /// Effective radius (stimuli beyond this are not considered).
    pub radius: f32,
    /// Tick when this stimulus was emitted.
    pub emitted_tick: u64,
    /// Duration in ticks before this stimulus expires (None = instant).
    pub duration_ticks: Option<u64>,
    /// Optional tag for categorizing stimuli (e.g., "footstep", "alarm").
    pub tag: Option<String>,
    /// Priority boost for this specific stimulus.
    pub priority_boost: f32,
}

impl Stimulus {
    /// Create a new stimulus with required fields.
    #[must_use]
    pub fn new(
        id: StimulusId,
        kind: SensorKind,
        source: StimulusSource,
        position: [f32; 3],
        intensity: f32,
    ) -> Self {
        Self {
            id,
            kind,
            source,
            position,
            intensity,
            radius: kind.default_range(),
            emitted_tick: 0,
            duration_ticks: None,
            tag: None,
            priority_boost: 0.0,
        }
    }

    /// Set the tick when emitted.
    #[must_use]
    pub fn at_tick(mut self, tick: u64) -> Self {
        self.emitted_tick = tick;
        self
    }

    /// Set the effective radius.
    #[must_use]
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// Set the duration in ticks.
    #[must_use]
    pub fn with_duration(mut self, ticks: u64) -> Self {
        self.duration_ticks = Some(ticks);
        self
    }

    /// Set a tag for categorization.
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    /// Set a priority boost.
    #[must_use]
    pub fn with_priority_boost(mut self, boost: f32) -> Self {
        self.priority_boost = boost;
        self
    }

    /// Check if this stimulus has expired.
    #[must_use]
    pub fn is_expired(&self, current_tick: u64) -> bool {
        if let Some(duration) = self.duration_ticks {
            current_tick > self.emitted_tick.saturating_add(duration)
        } else {
            current_tick > self.emitted_tick
        }
    }

    /// Calculate the age of this stimulus in ticks.
    #[must_use]
    pub fn age(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.emitted_tick)
    }

    /// Calculate distance to a point.
    #[must_use]
    pub fn distance_to(&self, point: [f32; 3]) -> f32 {
        let dx = self.position[0] - point[0];
        let dy = self.position[1] - point[1];
        let dz = self.position[2] - point[2];
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Check if a point is within the effective radius.
    #[must_use]
    pub fn in_range(&self, point: [f32; 3]) -> bool {
        self.distance_to(point) <= self.radius
    }
}

/// Ordering for stimuli to ensure deterministic processing.
impl Eq for Stimulus {}

impl PartialOrd for Stimulus {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Stimulus {
    fn cmp(&self, other: &Self) -> Ordering {
        self.kind
            .cmp(&other.kind)
            .then_with(|| {
                other
                    .intensity
                    .partial_cmp(&self.intensity)
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| self.emitted_tick.cmp(&other.emitted_tick))
            .then_with(|| self.id.cmp(&other.id))
    }
}

/// An entity that can emit stimuli into the environment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StimulusEmitter {
    /// Source entity ID.
    pub source: StimulusSource,
    /// Position of the emitter.
    pub position: [f32; 3],
    /// Active emissions by kind.
    emissions: Vec<EmissionConfig>,
}

/// Configuration for a continuous emission.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EmissionConfig {
    /// Which sensor channel this targets.
    pub kind: SensorKind,
    /// Base intensity.
    pub intensity: f32,
    /// Emission radius.
    pub radius: f32,
    /// Tag for emitted stimuli.
    pub tag: Option<String>,
    /// Whether this emission is currently active.
    pub active: bool,
}

impl EmissionConfig {
    #[must_use]
    pub fn new(kind: SensorKind, intensity: f32) -> Self {
        Self {
            kind,
            intensity,
            radius: kind.default_range(),
            tag: None,
            active: true,
        }
    }

    #[must_use]
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }
}

impl StimulusEmitter {
    #[must_use]
    pub fn new(source: StimulusSource, position: [f32; 3]) -> Self {
        Self {
            source,
            position,
            emissions: Vec::new(),
        }
    }

    /// Add an emission configuration.
    pub fn add_emission(&mut self, config: EmissionConfig) {
        self.emissions.push(config);
    }

    /// Builder pattern for adding emissions.
    #[must_use]
    pub fn with_emission(mut self, config: EmissionConfig) -> Self {
        self.add_emission(config);
        self
    }

    /// Update position.
    pub fn set_position(&mut self, position: [f32; 3]) {
        self.position = position;
    }

    /// Enable or disable an emission by kind.
    pub fn set_emission_active(&mut self, kind: SensorKind, active: bool) {
        for emission in &mut self.emissions {
            if emission.kind == kind {
                emission.active = active;
            }
        }
    }

    /// Generate stimuli for all active emissions.
    pub fn emit(&self, id_generator: &mut impl FnMut() -> u64, tick: u64) -> Vec<Stimulus> {
        self.emissions
            .iter()
            .filter(|e| e.active)
            .map(|e| {
                let mut stimulus = Stimulus::new(
                    StimulusId::new(id_generator()),
                    e.kind,
                    self.source.clone(),
                    self.position,
                    e.intensity,
                )
                .at_tick(tick)
                .with_radius(e.radius);

                if let Some(ref tag) = e.tag {
                    stimulus = stimulus.with_tag(tag.clone());
                }

                stimulus
            })
            .collect()
    }

    /// Get active emission kinds.
    pub fn active_kinds(&self) -> impl Iterator<Item = SensorKind> + '_ {
        self.emissions.iter().filter(|e| e.active).map(|e| e.kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stimulus_id() {
        let id = StimulusId::new(42);
        assert_eq!(id.0, 42);
    }

    #[test]
    fn test_stimulus_source() {
        let source = StimulusSource::new(100);
        assert_eq!(source.0, 100);
        assert!(!source.is_environmental());

        let env = StimulusSource::environmental();
        assert!(env.is_environmental());
    }

    #[test]
    fn test_stimulus_new() {
        let stimulus = Stimulus::new(
            StimulusId::new(1),
            SensorKind::Sound,
            StimulusSource::new(10),
            [1.0, 2.0, 3.0],
            50.0,
        );

        assert_eq!(stimulus.kind, SensorKind::Sound);
        assert!((stimulus.intensity - 50.0).abs() < f32::EPSILON);
        assert!((stimulus.radius - SensorKind::Sound.default_range()).abs() < f32::EPSILON);
    }

    #[test]
    fn test_stimulus_builders() {
        let stimulus = Stimulus::new(
            StimulusId::new(1),
            SensorKind::Smell,
            StimulusSource::new(1),
            [0.0, 0.0, 0.0],
            10.0,
        )
        .at_tick(100)
        .with_radius(15.0)
        .with_duration(50)
        .with_tag("smoke")
        .with_priority_boost(2.0);

        assert_eq!(stimulus.emitted_tick, 100);
        assert!((stimulus.radius - 15.0).abs() < f32::EPSILON);
        assert_eq!(stimulus.duration_ticks, Some(50));
        assert_eq!(stimulus.tag.as_deref(), Some("smoke"));
        assert!((stimulus.priority_boost - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_stimulus_expiry() {
        let instant = Stimulus::new(
            StimulusId::new(1),
            SensorKind::Sound,
            StimulusSource::new(1),
            [0.0, 0.0, 0.0],
            10.0,
        )
        .at_tick(100);

        assert!(!instant.is_expired(100));
        assert!(instant.is_expired(101));

        let duration = instant.clone().with_duration(10);
        assert!(!duration.is_expired(100));
        assert!(!duration.is_expired(105));
        assert!(!duration.is_expired(110));
        assert!(duration.is_expired(111));
    }

    #[test]
    fn test_stimulus_distance() {
        let stimulus = Stimulus::new(
            StimulusId::new(1),
            SensorKind::Sound,
            StimulusSource::new(1),
            [0.0, 0.0, 0.0],
            10.0,
        );

        assert!((stimulus.distance_to([3.0, 4.0, 0.0]) - 5.0).abs() < f32::EPSILON);
        assert!((stimulus.distance_to([0.0, 0.0, 0.0])).abs() < f32::EPSILON);
    }

    #[test]
    fn test_stimulus_in_range() {
        let stimulus = Stimulus::new(
            StimulusId::new(1),
            SensorKind::Sound,
            StimulusSource::new(1),
            [0.0, 0.0, 0.0],
            10.0,
        )
        .with_radius(10.0);

        assert!(stimulus.in_range([5.0, 0.0, 0.0]));
        assert!(stimulus.in_range([10.0, 0.0, 0.0]));
        assert!(!stimulus.in_range([11.0, 0.0, 0.0]));
    }

    #[test]
    fn test_stimulus_ordering_deterministic() {
        let s1 = Stimulus::new(
            StimulusId::new(1),
            SensorKind::Sound,
            StimulusSource::new(1),
            [0.0, 0.0, 0.0],
            10.0,
        )
        .at_tick(100);

        let s2 = Stimulus::new(
            StimulusId::new(2),
            SensorKind::Sound,
            StimulusSource::new(1),
            [0.0, 0.0, 0.0],
            10.0,
        )
        .at_tick(100);

        let cmp = s1.cmp(&s2);
        let cmp2 = s1.cmp(&s2);
        assert_eq!(cmp, cmp2);
    }

    #[test]
    fn test_stimulus_ordering_by_kind() {
        let sight = Stimulus::new(
            StimulusId::new(1),
            SensorKind::Sight,
            StimulusSource::new(1),
            [0.0, 0.0, 0.0],
            10.0,
        );

        let sound = Stimulus::new(
            StimulusId::new(2),
            SensorKind::Sound,
            StimulusSource::new(1),
            [0.0, 0.0, 0.0],
            10.0,
        );

        assert!(sight < sound);
    }

    #[test]
    fn test_stimulus_ordering_by_intensity() {
        let loud = Stimulus::new(
            StimulusId::new(1),
            SensorKind::Sound,
            StimulusSource::new(1),
            [0.0, 0.0, 0.0],
            100.0,
        );

        let quiet = Stimulus::new(
            StimulusId::new(2),
            SensorKind::Sound,
            StimulusSource::new(1),
            [0.0, 0.0, 0.0],
            10.0,
        );

        assert!(loud < quiet);
    }

    #[test]
    fn test_stimulus_emitter() {
        let emitter = StimulusEmitter::new(StimulusSource::new(1), [5.0, 10.0, 0.0])
            .with_emission(EmissionConfig::new(SensorKind::Heat, 20.0))
            .with_emission(EmissionConfig::new(SensorKind::Smell, 5.0).with_tag("body_odor"));

        let mut next_id = 100u64;
        let stimuli = emitter.emit(
            &mut || {
                let id = next_id;
                next_id += 1;
                id
            },
            50,
        );

        assert_eq!(stimuli.len(), 2);
        assert_eq!(stimuli[0].id, StimulusId::new(100));
        assert_eq!(stimuli[1].id, StimulusId::new(101));
    }

    #[test]
    fn test_stimulus_emitter_toggle() {
        let mut emitter = StimulusEmitter::new(StimulusSource::new(1), [0.0, 0.0, 0.0])
            .with_emission(EmissionConfig::new(SensorKind::Heat, 20.0))
            .with_emission(EmissionConfig::new(SensorKind::Sound, 10.0));

        let mut next_id = 1u64;
        let id_gen = &mut || {
            let id = next_id;
            next_id += 1;
            id
        };

        assert_eq!(emitter.emit(id_gen, 0).len(), 2);

        emitter.set_emission_active(SensorKind::Sound, false);
        assert_eq!(emitter.emit(id_gen, 0).len(), 1);

        emitter.set_emission_active(SensorKind::Sound, true);
        assert_eq!(emitter.emit(id_gen, 0).len(), 2);
    }

    #[test]
    fn test_stimulus_serde() {
        let stimulus = Stimulus::new(
            StimulusId::new(42),
            SensorKind::Vibration,
            StimulusSource::new(10),
            [1.0, 2.0, 3.0],
            75.0,
        )
        .with_tag("footstep");

        let json = serde_json::to_string(&stimulus).unwrap();
        let restored: Stimulus = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, StimulusId::new(42));
        assert_eq!(restored.kind, SensorKind::Vibration);
        assert_eq!(restored.tag.as_deref(), Some("footstep"));
    }

    #[test]
    fn test_emitter_serde() {
        let emitter = StimulusEmitter::new(StimulusSource::new(1), [0.0, 0.0, 0.0])
            .with_emission(EmissionConfig::new(SensorKind::Heat, 10.0));

        let json = serde_json::to_string(&emitter).unwrap();
        let restored: StimulusEmitter = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.source, StimulusSource::new(1));
    }
}
