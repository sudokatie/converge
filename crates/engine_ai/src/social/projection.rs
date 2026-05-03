//! Social projections for offline simulation.

use crate::social::betrayal::BetrayalTracker;
use crate::social::diplomacy::DiplomacyTracker;
use crate::social::morale::MoraleTracker;
use crate::social::panic::PanicTracker;
use serde::{Deserialize, Serialize};

/// Trend of social stability.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SocialTrend {
    Improving,
    #[default]
    Stable,
    Deteriorating,
    Critical,
}

impl SocialTrend {
    #[must_use]
    pub fn is_positive(&self) -> bool {
        matches!(self, Self::Improving)
    }

    #[must_use]
    pub fn is_negative(&self) -> bool {
        matches!(self, Self::Deteriorating | Self::Critical)
    }

    #[must_use]
    pub fn as_index(&self) -> u8 {
        match self {
            Self::Improving => 0,
            Self::Stable => 1,
            Self::Deteriorating => 2,
            Self::Critical => 3,
        }
    }
}

/// Projection of social state for unloaded regions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SocialProjection {
    pub projected_tick: u64,
    pub projected_stability: f32,
    pub projected_morale: f32,
    pub projected_panic: f32,
    pub expected_betrayals: u32,
    pub expected_morale_breaks: u32,
    pub expected_panic_events: u32,
    pub expected_treaty_expirations: u32,
    pub confidence: f32,
    pub trend: SocialTrend,
}

impl SocialProjection {
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "counts and ticks bounded in practice"
    )]
    pub fn from_trackers(
        morale: &MoraleTracker,
        panic: &PanicTracker,
        betrayal: &BetrayalTracker,
        diplomacy: &DiplomacyTracker,
        current_tick: u64,
        ticks_ahead: u64,
    ) -> Self {
        let projected_tick = current_tick + ticks_ahead;

        let current_morale = compute_average_morale(morale);
        let current_panic = panic.average_panic_level();

        let morale_decay = (ticks_ahead as f32) * 0.0001;
        let panic_decay = (ticks_ahead as f32) * 0.0002;

        let projected_morale = (current_morale - morale_decay).clamp(0.0, 1.0);
        let projected_panic = (current_panic - panic_decay).clamp(0.0, 1.0);

        let low_morale_count = morale.agents_with_low_morale().count();
        let high_risk_count = betrayal.high_risk_agents().count();

        let expected_morale_breaks = ((low_morale_count as f32 * ticks_ahead as f32 * 0.001)
            as u32)
            .min(low_morale_count as u32);
        let expected_betrayals = ((high_risk_count as f32 * ticks_ahead as f32 * 0.0005) as u32)
            .min(high_risk_count as u32);
        let expected_panic_events = if projected_morale < 0.3 {
            (ticks_ahead / 500) as u32
        } else {
            0
        };

        let active_treaties = diplomacy.active_treaties().count();
        let expected_treaty_expirations =
            (active_treaties as f32 * ticks_ahead as f32 * 0.0001) as u32;

        let projected_stability = compute_projected_stability(
            projected_morale,
            projected_panic,
            expected_betrayals,
            morale.agent_count(),
        );

        let confidence = (1.0 - ticks_ahead as f32 / 10000.0).clamp(0.1, 1.0);

        let trend = compute_trend(
            current_morale,
            projected_morale,
            current_panic,
            projected_panic,
        );

        Self {
            projected_tick,
            projected_stability,
            projected_morale,
            projected_panic,
            expected_betrayals,
            expected_morale_breaks,
            expected_panic_events,
            expected_treaty_expirations,
            confidence,
            trend,
        }
    }

    #[must_use]
    pub fn is_high_confidence(&self) -> bool {
        self.confidence >= 0.7
    }

    #[must_use]
    pub fn requires_attention(&self) -> bool {
        self.projected_stability < 0.4 || self.trend.is_negative()
    }

    #[must_use]
    pub fn is_critical(&self) -> bool {
        self.projected_stability < 0.2 || matches!(self.trend, SocialTrend::Critical)
    }

    #[must_use]
    pub fn total_expected_incidents(&self) -> u32 {
        self.expected_betrayals + self.expected_morale_breaks + self.expected_panic_events
    }
}

fn compute_average_morale(tracker: &MoraleTracker) -> f32 {
    if tracker.agent_count() == 0 {
        return 0.5;
    }

    let low_count = tracker.agents_with_low_morale().count();
    let broken_count = tracker.agents_with_broken_morale().count();

    #[expect(clippy::cast_precision_loss, reason = "counts bounded")]
    let problem_ratio = (low_count + broken_count * 2) as f32 / tracker.agent_count() as f32;

    (1.0 - problem_ratio * 0.5).clamp(0.0, 1.0)
}

#[expect(clippy::cast_precision_loss, reason = "counts bounded")]
fn compute_projected_stability(
    morale: f32,
    panic: f32,
    expected_betrayals: u32,
    agent_count: usize,
) -> f32 {
    let morale_factor = morale;
    let panic_factor = 1.0 - panic;

    let betrayal_factor = if agent_count > 0 {
        1.0 - (expected_betrayals as f32 / agent_count as f32).min(1.0)
    } else {
        1.0
    };

    (morale_factor * 0.4 + panic_factor * 0.3 + betrayal_factor * 0.3).clamp(0.0, 1.0)
}

fn compute_trend(
    current_morale: f32,
    projected_morale: f32,
    current_panic: f32,
    projected_panic: f32,
) -> SocialTrend {
    let morale_delta = projected_morale - current_morale;
    let panic_delta = projected_panic - current_panic;

    let net_change = morale_delta - panic_delta;

    if projected_morale < 0.2 || projected_panic > 0.8 {
        SocialTrend::Critical
    } else if net_change > 0.05 {
        SocialTrend::Improving
    } else if net_change < -0.05 {
        SocialTrend::Deteriorating
    } else {
        SocialTrend::Stable
    }
}

/// Morale projection for a specific faction or group.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MoraleProjection {
    pub projected_tick: u64,
    pub current_morale: f32,
    pub projected_morale: f32,
    pub recovery_estimate_ticks: u64,
    pub trend: SocialTrend,
    pub confidence: f32,
}

impl MoraleProjection {
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "ticks bounded, recovery_needed always positive when used"
    )]
    pub fn new(
        current_morale: f32,
        recovery_rate: f32,
        current_tick: u64,
        ticks_ahead: u64,
    ) -> Self {
        let projected_tick = current_tick + ticks_ahead;
        let recovery = (ticks_ahead as f32) * recovery_rate * 0.001;
        let projected_morale = (current_morale + recovery).clamp(0.0, 1.0);

        let recovery_needed = 0.7 - current_morale;
        let recovery_estimate_ticks = if recovery_rate > 0.0 && recovery_needed > 0.0 {
            ((recovery_needed / (recovery_rate * 0.001)) as u64).max(1)
        } else {
            0
        };

        let trend = if projected_morale > current_morale + 0.05 {
            SocialTrend::Improving
        } else if projected_morale < current_morale - 0.05 {
            SocialTrend::Deteriorating
        } else {
            SocialTrend::Stable
        };

        let confidence = (1.0 - ticks_ahead as f32 / 5000.0).clamp(0.1, 1.0);

        Self {
            projected_tick,
            current_morale,
            projected_morale,
            recovery_estimate_ticks,
            trend,
            confidence,
        }
    }
}

/// Panic projection for cascade estimation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PanicProjection {
    pub projected_tick: u64,
    pub current_panic: f32,
    pub projected_panic: f32,
    pub cascade_risk: f32,
    pub expected_fleeing: u32,
    pub confidence: f32,
}

impl PanicProjection {
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "counts bounded, cascade_risk always positive"
    )]
    pub fn from_tracker(tracker: &PanicTracker, current_tick: u64, ticks_ahead: u64) -> Self {
        let projected_tick = current_tick + ticks_ahead;
        let current_panic = tracker.average_panic_level();

        let recovery = (ticks_ahead as f32) * 0.0005;
        let projected_panic = (current_panic - recovery).clamp(0.0, 1.0);

        let panicking_count = tracker.panicking_agents().count();
        let total_count = tracker.agent_count();

        let cascade_risk = if total_count > 0 {
            (panicking_count as f32 / total_count as f32 * 2.0).min(1.0)
        } else {
            0.0
        };

        let expected_fleeing = if cascade_risk > 0.5 {
            ((total_count as f32 * cascade_risk * 0.3) as u32).min(total_count as u32)
        } else {
            tracker.fleeing_agents().count() as u32
        };

        let confidence = (1.0 - ticks_ahead as f32 / 3000.0).clamp(0.1, 1.0);

        Self {
            projected_tick,
            current_panic,
            projected_panic,
            cascade_risk,
            expected_fleeing,
            confidence,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_social_trend_default() {
        assert_eq!(SocialTrend::default(), SocialTrend::Stable);
    }

    #[test]
    fn test_social_trend_classification() {
        assert!(SocialTrend::Improving.is_positive());
        assert!(SocialTrend::Deteriorating.is_negative());
        assert!(SocialTrend::Critical.is_negative());
        assert!(!SocialTrend::Stable.is_negative());
    }

    #[test]
    fn test_social_projection_from_empty() {
        let morale = MoraleTracker::new();
        let panic = PanicTracker::new();
        let betrayal = BetrayalTracker::new();
        let diplomacy = DiplomacyTracker::new();

        let projection =
            SocialProjection::from_trackers(&morale, &panic, &betrayal, &diplomacy, 0, 1000);

        assert_eq!(projection.projected_tick, 1000);
        assert!(projection.confidence > 0.0);
    }

    #[test]
    fn test_projection_confidence_degrades() {
        let morale = MoraleTracker::new();
        let panic = PanicTracker::new();
        let betrayal = BetrayalTracker::new();
        let diplomacy = DiplomacyTracker::new();

        let short = SocialProjection::from_trackers(&morale, &panic, &betrayal, &diplomacy, 0, 100);
        let long = SocialProjection::from_trackers(&morale, &panic, &betrayal, &diplomacy, 0, 5000);

        assert!(short.confidence > long.confidence);
        assert!(short.is_high_confidence());
    }

    #[test]
    fn test_morale_projection() {
        let projection = MoraleProjection::new(0.3, 0.1, 0, 1000);

        assert_eq!(projection.projected_tick, 1000);
        assert!(projection.projected_morale >= projection.current_morale);
        assert!(projection.recovery_estimate_ticks > 0);
    }

    #[test]
    fn test_panic_projection() {
        let tracker = PanicTracker::new();
        let projection = PanicProjection::from_tracker(&tracker, 0, 500);

        assert_eq!(projection.projected_tick, 500);
        assert!(projection.confidence > 0.0);
    }

    #[test]
    fn test_serde_roundtrip() {
        let morale = MoraleTracker::new();
        let panic = PanicTracker::new();
        let betrayal = BetrayalTracker::new();
        let diplomacy = DiplomacyTracker::new();

        let projection =
            SocialProjection::from_trackers(&morale, &panic, &betrayal, &diplomacy, 100, 500);

        let json = serde_json::to_string(&projection).unwrap();
        let restored: SocialProjection = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.projected_tick, projection.projected_tick);
        assert!((restored.confidence - projection.confidence).abs() < f32::EPSILON);
    }

    #[test]
    fn test_trend_serde() {
        let trend = SocialTrend::Improving;
        let json = serde_json::to_string(&trend).unwrap();
        let restored: SocialTrend = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, trend);
    }
}
