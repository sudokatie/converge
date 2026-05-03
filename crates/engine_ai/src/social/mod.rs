//! Social simulation for morale, diplomacy, betrayal, and panic systems.
//!
//! Provides deterministic social dynamics tracking for:
//! - Individual and group morale with recovery
//! - Diplomatic relations, treaties, and alliances
//! - Betrayal risk assessment and incident tracking
//! - Panic levels and cascade propagation
//! - Snapshots and projections for offline simulation
//! - Stable fingerprints for state verification

mod betrayal;
mod diplomacy;
mod fingerprint;
pub mod ids;
mod morale;
mod panic;
mod projection;
mod snapshot;

pub use betrayal::{
    BetrayalEvent, BetrayalEventKind, BetrayalFactors, BetrayalIncident, BetrayalKind,
    BetrayalProfile, BetrayalResolution, BetrayalRisk, BetrayalSeverity, BetrayalStatus,
    BetrayalTracker, GrievanceLevel, LoyaltyLevel, SuspicionLevel,
};
pub use diplomacy::{
    DiplomacyEvent, DiplomacyEventKind, DiplomacyTracker, DiplomaticRelation, DiplomaticStance,
    Treaty, TreatyId, TreatyKind, TreatyStatus, TrustLevel,
};
pub use fingerprint::{
    BetrayalFingerprint, DiplomacyFingerprint, MoraleFingerprint, PanicFingerprint,
    SocialFingerprint,
};
pub use ids::{BetrayalId, DiplomacyId, PanicId, SocialAgentId, SocialFactionId, SocialGroupId};
pub use morale::{
    AgentMorale, GroupMorale, MoraleEvent, MoraleEventKind, MoraleFactors, MoraleLevel,
    MoraleTracker,
};
pub use panic::{
    AgentPanic, PanicCascade, PanicEvent, PanicEventStatus, PanicLevel, PanicSource, PanicTracker,
    PanicTrackingEvent, PanicTrackingEventKind,
};
pub use projection::{MoraleProjection, PanicProjection, SocialProjection, SocialTrend};
pub use snapshot::{FactionSocialSummary, SocialSnapshot, SocialSummary, StanceCounts};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_full_social_simulation() {
        let mut morale_tracker = MoraleTracker::new();
        let mut panic_tracker = PanicTracker::new();
        let mut betrayal_tracker = BetrayalTracker::new();
        let mut diplomacy_tracker = DiplomacyTracker::new();

        let agent1 = SocialAgentId::new(1);
        let agent2 = SocialAgentId::new(2);
        let faction_a = SocialFactionId::new("faction_a");
        let faction_b = SocialFactionId::new("faction_b");

        morale_tracker.register_agent(agent1, 0);
        morale_tracker.register_agent(agent2, 0);
        panic_tracker.register_agent(agent1, 0);
        panic_tracker.register_agent(agent2, 0);
        betrayal_tracker.register_agent(agent1, faction_a.clone(), 0);
        betrayal_tracker.register_agent(agent2, faction_b.clone(), 0);

        diplomacy_tracker.set_stance(&faction_a, &faction_b, DiplomaticStance::Friendly, 0);

        let snapshot = SocialSnapshot::from_trackers(
            &morale_tracker,
            &panic_tracker,
            &betrayal_tracker,
            &diplomacy_tracker,
            0,
        );

        assert_eq!(snapshot.agent_count, 2);
        assert!(snapshot.stability_score() > 0.0);
    }

    #[test]
    fn test_morale_betrayal_interaction() {
        let mut morale = MoraleTracker::new();
        let mut betrayal = BetrayalTracker::new();

        let agent = SocialAgentId::new(1);
        let faction = SocialFactionId::new("empire");

        {
            let agent_morale = morale.register_agent(agent, 0);
            agent_morale.base_morale = MoraleLevel::new(0.2);
        }

        betrayal.register_agent(agent, faction.clone(), 0);

        let _profile = betrayal.get_profile(agent).unwrap();
        let factors = BetrayalFactors::new();
        let morale_state = morale.get_agent_morale(agent).unwrap();
        let risk = factors.compute_risk(morale_state.effective_morale(), 0.5, 0.3, 0.2, 0.1, 0.1);

        assert!(risk.raw() > 0.0);
    }

    #[test]
    fn test_panic_morale_impact() {
        let mut morale = MoraleTracker::new();
        let mut panic = PanicTracker::new();

        let agent = SocialAgentId::new(1);

        morale.register_agent(agent, 0);
        {
            let agent_panic = panic.register_agent(agent, 0);
            agent_panic.panic_level = PanicLevel::new(0.7);
        }

        let impact = panic.compute_morale_impact(agent);
        assert!(impact < 0.0);
    }

    #[test]
    fn test_diplomacy_treaty_flow() {
        let mut tracker = DiplomacyTracker::new();

        let faction_a = SocialFactionId::new("a");
        let faction_b = SocialFactionId::new("b");
        let faction_c = SocialFactionId::new("c");

        tracker.sign_treaty(
            TreatyKind::DefensiveAlliance,
            &[faction_a.clone(), faction_b.clone()],
            0,
        );

        tracker.declare_war(&faction_c, &faction_a, 100);

        assert!(tracker.are_at_war(&faction_c, &faction_a));
        assert!(!tracker.are_at_war(&faction_a, &faction_b));
    }

    #[test]
    fn test_fingerprint_consistency() {
        let morale = MoraleTracker::new();
        let panic = PanicTracker::new();
        let betrayal = BetrayalTracker::new();
        let diplomacy = DiplomacyTracker::new();

        let fp1 = SocialFingerprint::from_trackers(&morale, &panic, &betrayal, &diplomacy, 0);
        let fp2 = SocialFingerprint::from_trackers(&morale, &panic, &betrayal, &diplomacy, 0);

        assert!(fp1.matches(&fp2));
    }

    #[test]
    fn test_projection_generation() {
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
    fn test_serde_all_types() {
        let morale = MoraleTracker::new();
        let panic = PanicTracker::new();
        let betrayal = BetrayalTracker::new();
        let diplomacy = DiplomacyTracker::new();

        let json_morale = serde_json::to_string(&morale).unwrap();
        let json_panic = serde_json::to_string(&panic).unwrap();
        let json_betrayal = serde_json::to_string(&betrayal).unwrap();
        let json_diplomacy = serde_json::to_string(&diplomacy).unwrap();

        let _: MoraleTracker = serde_json::from_str(&json_morale).unwrap();
        let _: PanicTracker = serde_json::from_str(&json_panic).unwrap();
        let _: BetrayalTracker = serde_json::from_str(&json_betrayal).unwrap();
        let _: DiplomacyTracker = serde_json::from_str(&json_diplomacy).unwrap();
    }

    #[test]
    fn test_bincode_all_types() {
        let morale = MoraleTracker::new();
        let panic = PanicTracker::new();
        let betrayal = BetrayalTracker::new();
        let diplomacy = DiplomacyTracker::new();

        let bytes_morale = bincode::serialize(&morale).unwrap();
        let bytes_panic = bincode::serialize(&panic).unwrap();
        let bytes_betrayal = bincode::serialize(&betrayal).unwrap();
        let bytes_diplomacy = bincode::serialize(&diplomacy).unwrap();

        let restored_morale: MoraleTracker = bincode::deserialize(&bytes_morale).unwrap();
        let restored_panic: PanicTracker = bincode::deserialize(&bytes_panic).unwrap();
        let restored_betrayal: BetrayalTracker = bincode::deserialize(&bytes_betrayal).unwrap();
        let restored_diplomacy: DiplomacyTracker = bincode::deserialize(&bytes_diplomacy).unwrap();

        assert_eq!(morale.checksum(), restored_morale.checksum());
        assert_eq!(panic.checksum(), restored_panic.checksum());
        assert_eq!(betrayal.checksum(), restored_betrayal.checksum());
        assert_eq!(diplomacy.checksum(), restored_diplomacy.checksum());
    }

    #[test]
    fn test_bincode_with_data() {
        let mut morale = MoraleTracker::new();
        let mut betrayal = BetrayalTracker::new();

        let agent = SocialAgentId::new(42);
        let faction = SocialFactionId::new("test_faction");

        {
            let agent_morale = morale.register_agent(agent, 100);
            agent_morale.base_morale = MoraleLevel::new(0.75);
        }
        {
            let profile = betrayal.register_agent(agent, faction, 100);
            profile.risk = BetrayalRisk::new(0.3);
        }

        let bytes_morale = bincode::serialize(&morale).unwrap();
        let bytes_betrayal = bincode::serialize(&betrayal).unwrap();

        let restored_morale: MoraleTracker = bincode::deserialize(&bytes_morale).unwrap();
        let restored_betrayal: BetrayalTracker = bincode::deserialize(&bytes_betrayal).unwrap();

        assert_eq!(morale.checksum(), restored_morale.checksum());
        assert_eq!(betrayal.checksum(), restored_betrayal.checksum());

        let restored_agent_morale = restored_morale.get_agent_morale(agent).unwrap();
        assert!((restored_agent_morale.base_morale.raw() - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn test_snapshot_bincode() {
        let snapshot = SocialSnapshot {
            tick: 500,
            agent_count: 100,
            group_count: 10,
            faction_count: 3,
            average_morale: 0.65,
            average_panic: 0.15,
            broken_morale_count: 5,
            panicking_count: 8,
            active_betrayal_count: 2,
            active_treaty_count: 4,
            war_count: 1,
            alliance_count: 2,
            faction_stances: BTreeMap::default(),
        };

        let bytes = bincode::serialize(&snapshot).unwrap();
        let restored: SocialSnapshot = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.tick, 500);
        assert_eq!(restored.agent_count, 100);
        assert!((restored.average_morale - 0.65).abs() < f32::EPSILON);
    }

    #[test]
    fn test_fingerprint_bincode() {
        let fp = SocialFingerprint(0xdead_beef);

        let bytes = bincode::serialize(&fp).unwrap();
        let restored: SocialFingerprint = bincode::deserialize(&bytes).unwrap();

        assert!(fp.matches(&restored));
    }

    #[test]
    fn test_projection_bincode() {
        let morale = MoraleTracker::new();
        let panic = PanicTracker::new();
        let betrayal = BetrayalTracker::new();
        let diplomacy = DiplomacyTracker::new();

        let projection =
            SocialProjection::from_trackers(&morale, &panic, &betrayal, &diplomacy, 100, 500);

        let bytes = bincode::serialize(&projection).unwrap();
        let restored: SocialProjection = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.projected_tick, projection.projected_tick);
        assert!((restored.confidence - projection.confidence).abs() < f32::EPSILON);
    }

    #[test]
    fn test_ids_bincode() {
        let agent_id = SocialAgentId::new(12345);
        let faction_id = SocialFactionId::new("test_faction");
        let group_id = SocialGroupId::new(67890);
        let betrayal_id = BetrayalId::new(111);
        let panic_id = PanicId::new(222);

        let bytes_agent = bincode::serialize(&agent_id).unwrap();
        let bytes_faction = bincode::serialize(&faction_id).unwrap();
        let bytes_group = bincode::serialize(&group_id).unwrap();
        let bytes_betrayal = bincode::serialize(&betrayal_id).unwrap();
        let bytes_panic = bincode::serialize(&panic_id).unwrap();

        let restored_agent: SocialAgentId = bincode::deserialize(&bytes_agent).unwrap();
        let restored_faction: SocialFactionId = bincode::deserialize(&bytes_faction).unwrap();
        let restored_group: SocialGroupId = bincode::deserialize(&bytes_group).unwrap();
        let restored_betrayal: BetrayalId = bincode::deserialize(&bytes_betrayal).unwrap();
        let restored_panic: PanicId = bincode::deserialize(&bytes_panic).unwrap();

        assert_eq!(restored_agent.raw(), 12345);
        assert_eq!(restored_faction.as_str(), "test_faction");
        assert_eq!(restored_group.raw(), 67890);
        assert_eq!(restored_betrayal.raw(), 111);
        assert_eq!(restored_panic.raw(), 222);
    }
}
