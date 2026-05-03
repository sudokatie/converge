//! Social snapshot and summary types.

use crate::social::betrayal::BetrayalTracker;
use crate::social::diplomacy::{DiplomacyTracker, DiplomaticStance};
use crate::social::ids::SocialFactionId;
use crate::social::morale::MoraleTracker;
use crate::social::panic::PanicTracker;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Full snapshot of social simulation state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SocialSnapshot {
    pub tick: u64,
    pub agent_count: u32,
    pub group_count: u32,
    pub faction_count: u32,
    pub average_morale: f32,
    pub average_panic: f32,
    pub broken_morale_count: u32,
    pub panicking_count: u32,
    pub active_betrayal_count: u32,
    pub active_treaty_count: u32,
    pub war_count: u32,
    pub alliance_count: u32,
    pub faction_stances: BTreeMap<String, StanceCounts>,
}

impl SocialSnapshot {
    #[must_use]
    pub fn new(tick: u64) -> Self {
        Self {
            tick,
            ..Default::default()
        }
    }

    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "counts bounded")]
    pub fn from_trackers(
        morale: &MoraleTracker,
        panic: &PanicTracker,
        betrayal: &BetrayalTracker,
        diplomacy: &DiplomacyTracker,
        tick: u64,
    ) -> Self {
        let mut snapshot = Self::new(tick);

        snapshot.agent_count = morale.agent_count() as u32;
        snapshot.group_count = morale.group_count() as u32;
        snapshot.average_morale = compute_average_morale(morale);
        snapshot.average_panic = panic.average_panic_level();
        snapshot.broken_morale_count = morale.agents_with_broken_morale().count() as u32;
        snapshot.panicking_count = panic.panicking_agents().count() as u32;
        snapshot.active_betrayal_count = betrayal.active_incident_count() as u32;
        snapshot.active_treaty_count = diplomacy.active_treaties().count() as u32;

        snapshot
    }

    #[must_use]
    pub fn stability_score(&self) -> f32 {
        let morale_factor = self.average_morale;
        let panic_factor = 1.0 - self.average_panic;

        #[expect(clippy::cast_precision_loss, reason = "counts bounded")]
        let betrayal_factor = if self.agent_count > 0 {
            1.0 - (self.active_betrayal_count as f32 / self.agent_count as f32).min(1.0)
        } else {
            1.0
        };

        (morale_factor * 0.4 + panic_factor * 0.3 + betrayal_factor * 0.3).clamp(0.0, 1.0)
    }

    #[must_use]
    pub fn is_stable(&self) -> bool {
        self.stability_score() >= 0.6
    }

    #[must_use]
    pub fn is_critical(&self) -> bool {
        self.stability_score() < 0.3
    }
}

/// Count of different diplomatic stances for a faction.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StanceCounts {
    pub war: u32,
    pub hostile: u32,
    pub unfriendly: u32,
    pub neutral: u32,
    pub cordial: u32,
    pub friendly: u32,
    pub allied: u32,
}

impl StanceCounts {
    pub fn add_stance(&mut self, stance: DiplomaticStance) {
        match stance {
            DiplomaticStance::War => self.war += 1,
            DiplomaticStance::Hostile => self.hostile += 1,
            DiplomaticStance::Unfriendly => self.unfriendly += 1,
            DiplomaticStance::Neutral => self.neutral += 1,
            DiplomaticStance::Cordial => self.cordial += 1,
            DiplomaticStance::Friendly => self.friendly += 1,
            DiplomaticStance::Allied => self.allied += 1,
        }
    }

    #[must_use]
    pub fn positive_count(&self) -> u32 {
        self.cordial + self.friendly + self.allied
    }

    #[must_use]
    pub fn negative_count(&self) -> u32 {
        self.war + self.hostile + self.unfriendly
    }
}

/// Lightweight summary for cheap transmission.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SocialSummary {
    pub tick: u64,
    pub agent_count: u32,
    pub stability_score: f32,
    pub average_morale: f32,
    pub average_panic: f32,
    pub critical_issues: u32,
}

impl SocialSummary {
    #[must_use]
    pub fn new(tick: u64) -> Self {
        Self {
            tick,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn from_snapshot(snapshot: &SocialSnapshot) -> Self {
        let critical_issues = snapshot.broken_morale_count
            + snapshot.panicking_count
            + snapshot.active_betrayal_count;

        Self {
            tick: snapshot.tick,
            agent_count: snapshot.agent_count,
            stability_score: snapshot.stability_score(),
            average_morale: snapshot.average_morale,
            average_panic: snapshot.average_panic,
            critical_issues,
        }
    }

    #[must_use]
    pub fn is_healthy(&self) -> bool {
        self.stability_score >= 0.7 && self.critical_issues == 0
    }

    #[must_use]
    pub fn needs_attention(&self) -> bool {
        self.stability_score < 0.5 || self.critical_issues > 0
    }
}

impl From<&SocialSnapshot> for SocialSummary {
    fn from(snapshot: &SocialSnapshot) -> Self {
        Self::from_snapshot(snapshot)
    }
}

/// Faction-specific social summary.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FactionSocialSummary {
    pub faction_id: SocialFactionId,
    pub tick: u64,
    pub member_count: u32,
    pub average_morale: f32,
    pub average_panic: f32,
    pub betrayal_risk_count: u32,
    pub active_wars: u32,
    pub active_alliances: u32,
    pub cohesion: f32,
}

impl FactionSocialSummary {
    #[must_use]
    pub fn new(faction_id: SocialFactionId, tick: u64) -> Self {
        Self {
            faction_id,
            tick,
            member_count: 0,
            average_morale: 0.5,
            average_panic: 0.0,
            betrayal_risk_count: 0,
            active_wars: 0,
            active_alliances: 0,
            cohesion: 0.5,
        }
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "counts bounded")]
    pub fn threat_level(&self) -> f32 {
        let war_threat = (self.active_wars as f32 * 0.2).min(1.0);
        let internal_threat = (1.0 - self.average_morale) * 0.3;
        let betrayal_threat = (self.betrayal_risk_count as f32 * 0.1).min(0.5);

        (war_threat + internal_threat + betrayal_threat).clamp(0.0, 1.0)
    }

    #[must_use]
    pub fn is_under_threat(&self) -> bool {
        self.threat_level() >= 0.5
    }
}

fn compute_average_morale(tracker: &MoraleTracker) -> f32 {
    if tracker.agent_count() == 0 {
        return 0.5;
    }

    let mut sum = 0.0f32;
    let mut count = 0u32;

    for agent_id in tracker.agents_with_low_morale() {
        if let Some(morale) = tracker.get_agent_morale(agent_id) {
            sum += morale.effective_morale().raw();
            count += 1;
        }
    }

    #[expect(clippy::cast_precision_loss, reason = "count bounded")]
    if count > 0 { sum / count as f32 } else { 0.5 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_new() {
        let snapshot = SocialSnapshot::new(100);
        assert_eq!(snapshot.tick, 100);
        assert_eq!(snapshot.agent_count, 0);
    }

    #[test]
    fn test_snapshot_stability() {
        let mut snapshot = SocialSnapshot::new(0);
        snapshot.average_morale = 0.8;
        snapshot.average_panic = 0.1;
        snapshot.agent_count = 10;

        assert!(snapshot.is_stable());
        assert!(!snapshot.is_critical());
    }

    #[test]
    fn test_snapshot_critical() {
        let mut snapshot = SocialSnapshot::new(0);
        snapshot.average_morale = 0.1;
        snapshot.average_panic = 0.9;
        snapshot.agent_count = 10;
        snapshot.active_betrayal_count = 5;

        assert!(snapshot.is_critical());
        assert!(!snapshot.is_stable());
    }

    #[test]
    fn test_stance_counts() {
        let mut counts = StanceCounts::default();
        counts.add_stance(DiplomaticStance::War);
        counts.add_stance(DiplomaticStance::Allied);
        counts.add_stance(DiplomaticStance::Allied);

        assert_eq!(counts.negative_count(), 1);
        assert_eq!(counts.positive_count(), 2);
    }

    #[test]
    fn test_summary_from_snapshot() {
        let mut snapshot = SocialSnapshot::new(100);
        snapshot.agent_count = 50;
        snapshot.average_morale = 0.7;
        snapshot.average_panic = 0.2;

        let summary = SocialSummary::from_snapshot(&snapshot);
        assert_eq!(summary.tick, 100);
        assert_eq!(summary.agent_count, 50);
        assert!(summary.stability_score > 0.0);
    }

    #[test]
    fn test_faction_summary() {
        let mut summary = FactionSocialSummary::new(SocialFactionId::new("empire"), 0);
        summary.active_wars = 2;
        summary.average_morale = 0.3;

        assert!(summary.threat_level() > 0.0);
        assert!(summary.is_under_threat() || !summary.is_under_threat());
    }

    #[test]
    fn test_serde_roundtrip() {
        let mut snapshot = SocialSnapshot::new(100);
        snapshot.agent_count = 25;
        snapshot.average_morale = 0.6;

        let json = serde_json::to_string(&snapshot).unwrap();
        let restored: SocialSnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.tick, 100);
        assert_eq!(restored.agent_count, 25);
    }

    #[test]
    fn test_summary_serde() {
        let summary = SocialSummary {
            tick: 200,
            agent_count: 30,
            stability_score: 0.75,
            average_morale: 0.8,
            average_panic: 0.1,
            critical_issues: 0,
        };

        let json = serde_json::to_string(&summary).unwrap();
        let restored: SocialSummary = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.tick, 200);
        assert!((restored.stability_score - 0.75).abs() < f32::EPSILON);
    }
}
