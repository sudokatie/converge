//! Lifecycle projections for offline simulation.

use serde::{Deserialize, Serialize};

use super::tracker::LifecycleTracker;

/// Trend of lifecycle population change.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LifecycleTrend {
    /// Population is growing (more births than deaths).
    Growing,
    /// Population is stable.
    #[default]
    Stable,
    /// Population is declining (more deaths than births).
    Declining,
}

/// Projection of lifecycle state for unloaded regions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LifecycleProjection {
    pub projected_tick: u64,
    pub projected_egg_count: u32,
    pub projected_living_count: u32,
    pub projected_corpse_count: u32,
    pub expected_hatchings: u32,
    pub expected_deaths: u32,
    pub expected_decays: u32,
    pub confidence: f32,
    pub trend: LifecycleTrend,
}

impl LifecycleProjection {
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_possible_wrap,
        reason = "counts and ticks bounded in practice"
    )]
    pub fn from_tracker(tracker: &LifecycleTracker, current_tick: u64, ticks_ahead: u64) -> Self {
        let projected_tick = current_tick + ticks_ahead;
        let config = tracker.config();

        let egg_count = tracker.egg_count() as u32;
        let living_count = tracker.living_count() as u32;
        let corpse_count = tracker.corpse_count() as u32;

        let hatch_rate = if config.incubation.base_duration > 0 {
            ticks_ahead as f32 / config.incubation.base_duration as f32
        } else {
            0.0
        };
        let expected_hatchings =
            ((egg_count as f32 * hatch_rate * config.incubation.survival_chance) as u32)
                .min(egg_count);

        let lifespan = config.aging.max_lifespan.unwrap_or(10000);
        let death_rate = if lifespan > 0 {
            ticks_ahead as f32 / lifespan as f32
        } else {
            0.0
        };
        let expected_deaths = (living_count as f32 * death_rate * 0.5) as u32;

        let decay_rate = if config.decay.full_decay_duration > 0 {
            ticks_ahead as f32 / config.decay.full_decay_duration as f32
        } else {
            0.0
        };
        let expected_decays = ((corpse_count as f32 * decay_rate) as u32).min(corpse_count);

        let projected_egg_count = egg_count.saturating_sub(expected_hatchings);
        let projected_living_count = living_count + expected_hatchings - expected_deaths;
        let projected_corpse_count = corpse_count + expected_deaths - expected_decays;

        let confidence = (1.0 - ticks_ahead as f32 / 10000.0).clamp(0.1, 1.0);

        let net_change = expected_hatchings as i32 - expected_deaths as i32;
        let trend = match net_change.cmp(&0) {
            std::cmp::Ordering::Greater => LifecycleTrend::Growing,
            std::cmp::Ordering::Less => LifecycleTrend::Declining,
            std::cmp::Ordering::Equal => LifecycleTrend::Stable,
        };

        Self {
            projected_tick,
            projected_egg_count,
            projected_living_count,
            projected_corpse_count,
            expected_hatchings,
            expected_deaths,
            expected_decays,
            confidence,
            trend,
        }
    }

    #[must_use]
    pub fn total_projected(&self) -> u32 {
        self.projected_egg_count + self.projected_living_count + self.projected_corpse_count
    }

    #[must_use]
    pub fn is_high_confidence(&self) -> bool {
        self.confidence >= 0.7
    }

    #[must_use]
    pub fn is_growing(&self) -> bool {
        matches!(self.trend, LifecycleTrend::Growing)
    }

    #[must_use]
    pub fn is_declining(&self) -> bool {
        matches!(self.trend, LifecycleTrend::Declining)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle::{GrowthPhase, LifecycleConfig, LifecycleId};

    #[test]
    fn test_lifecycle_trend_default() {
        assert_eq!(LifecycleTrend::default(), LifecycleTrend::Stable);
    }

    #[test]
    fn test_projection_from_empty_tracker() {
        let tracker = LifecycleTracker::new(LifecycleConfig::standard());
        let projection = LifecycleProjection::from_tracker(&tracker, 0, 1000);

        assert_eq!(projection.projected_tick, 1000);
        assert_eq!(projection.projected_egg_count, 0);
        assert_eq!(projection.projected_living_count, 0);
        assert_eq!(projection.projected_corpse_count, 0);
        assert_eq!(projection.total_projected(), 0);
    }

    #[test]
    fn test_projection_with_eggs() {
        let config = LifecycleConfig::minimal();
        let mut tracker = LifecycleTracker::new(config);

        tracker.spawn_egg(LifecycleId::new(1), 0);
        tracker.spawn_egg(LifecycleId::new(2), 0);
        tracker.spawn_egg(LifecycleId::new(3), 0);

        let projection = LifecycleProjection::from_tracker(&tracker, 0, 500);

        assert!(projection.expected_hatchings > 0);
        assert!(projection.total_projected() > 0);
    }

    #[test]
    fn test_projection_confidence_degrades() {
        let tracker = LifecycleTracker::new(LifecycleConfig::standard());

        let short_projection = LifecycleProjection::from_tracker(&tracker, 0, 100);
        let long_projection = LifecycleProjection::from_tracker(&tracker, 0, 5000);

        assert!(short_projection.confidence > long_projection.confidence);
        assert!(short_projection.is_high_confidence());
    }

    #[test]
    fn test_projection_trend_growing() {
        let config = LifecycleConfig::minimal();
        let mut tracker = LifecycleTracker::new(config);

        for i in 1..=10 {
            tracker.spawn_egg(LifecycleId::new(i), 0);
        }

        let projection = LifecycleProjection::from_tracker(&tracker, 0, 200);
        assert!(projection.is_growing() || projection.trend == LifecycleTrend::Stable);
    }

    #[test]
    fn test_projection_trend_declining() {
        let config = LifecycleConfig::minimal();
        let mut tracker = LifecycleTracker::new(config.clone());

        for i in 1..=5 {
            tracker.spawn_living(LifecycleId::new(i), GrowthPhase::Elder, 0);
        }

        let projection = LifecycleProjection::from_tracker(
            &tracker,
            0,
            config.aging.max_lifespan.unwrap_or(2000),
        );
        assert!(projection.expected_deaths > 0);
    }

    #[test]
    fn test_projection_serde() {
        let tracker = LifecycleTracker::new(LifecycleConfig::standard());
        let projection = LifecycleProjection::from_tracker(&tracker, 100, 500);

        let json = serde_json::to_string(&projection).unwrap();
        let restored: LifecycleProjection = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.projected_tick, projection.projected_tick);
        assert!((restored.confidence - projection.confidence).abs() < f32::EPSILON);
    }

    #[test]
    fn test_lifecycle_trend_serde() {
        let trend = LifecycleTrend::Growing;
        let json = serde_json::to_string(&trend).unwrap();
        let restored: LifecycleTrend = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, trend);
    }
}
