//! Betrayal system for risk assessment, detection, and resolution.

use crate::social::ids::{BetrayalId, SocialAgentId, SocialFactionId};
use crate::social::morale::MoraleLevel;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Risk level for betrayal.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BetrayalRisk(f32);

impl BetrayalRisk {
    pub const MIN: f32 = 0.0;
    pub const MAX: f32 = 1.0;

    pub fn new(value: f32) -> Self {
        Self(value.clamp(Self::MIN, Self::MAX))
    }

    pub fn raw(self) -> f32 {
        self.0
    }

    pub fn is_negligible(self) -> bool {
        self.0 < 0.1
    }

    pub fn is_low(self) -> bool {
        self.0 < 0.3
    }

    pub fn is_moderate(self) -> bool {
        self.0 >= 0.3 && self.0 < 0.6
    }

    pub fn is_high(self) -> bool {
        self.0 >= 0.6 && self.0 < 0.8
    }

    pub fn is_imminent(self) -> bool {
        self.0 >= 0.8
    }

    pub fn modify(&mut self, delta: f32) {
        self.0 = (self.0 + delta).clamp(Self::MIN, Self::MAX);
    }
}

impl Default for BetrayalRisk {
    fn default() -> Self {
        Self(0.0)
    }
}

impl Eq for BetrayalRisk {}

impl std::hash::Hash for BetrayalRisk {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PartialOrd for BetrayalRisk {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BetrayalRisk {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// Factors contributing to betrayal risk.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BetrayalFactors {
    pub low_morale_weight: f32,
    pub distrust_weight: f32,
    pub grievance_weight: f32,
    pub opportunity_weight: f32,
    pub outside_influence_weight: f32,
    pub fear_weight: f32,
}

impl BetrayalFactors {
    pub fn new() -> Self {
        Self {
            low_morale_weight: 0.3,
            distrust_weight: 0.25,
            grievance_weight: 0.2,
            opportunity_weight: 0.1,
            outside_influence_weight: 0.1,
            fear_weight: 0.05,
        }
    }

    pub fn compute_risk(
        &self,
        morale: MoraleLevel,
        trust: f32,
        grievance: f32,
        opportunity: f32,
        outside_influence: f32,
        fear: f32,
    ) -> BetrayalRisk {
        let morale_factor = 1.0 - morale.raw();
        let distrust_factor = (1.0 - trust).max(0.0);

        let risk = morale_factor * self.low_morale_weight
            + distrust_factor * self.distrust_weight
            + grievance.clamp(0.0, 1.0) * self.grievance_weight
            + opportunity.clamp(0.0, 1.0) * self.opportunity_weight
            + outside_influence.clamp(0.0, 1.0) * self.outside_influence_weight
            + fear.clamp(0.0, 1.0) * self.fear_weight;

        BetrayalRisk::new(risk)
    }
}

impl Eq for BetrayalFactors {}

impl std::hash::Hash for BetrayalFactors {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.low_morale_weight.to_bits().hash(state);
        self.distrust_weight.to_bits().hash(state);
        self.grievance_weight.to_bits().hash(state);
        self.opportunity_weight.to_bits().hash(state);
        self.outside_influence_weight.to_bits().hash(state);
        self.fear_weight.to_bits().hash(state);
    }
}

/// An agent's betrayal profile.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BetrayalProfile {
    pub agent_id: SocialAgentId,
    pub current_faction: SocialFactionId,
    pub risk: BetrayalRisk,
    pub grievance: GrievanceLevel,
    pub loyalty: LoyaltyLevel,
    pub suspicion_on_self: SuspicionLevel,
    pub detected_plots: Vec<BetrayalId>,
    pub last_assessment_tick: u64,
}

impl BetrayalProfile {
    pub fn new(agent_id: SocialAgentId, faction: SocialFactionId, tick: u64) -> Self {
        Self {
            agent_id,
            current_faction: faction,
            risk: BetrayalRisk::default(),
            grievance: GrievanceLevel::default(),
            loyalty: LoyaltyLevel::default(),
            suspicion_on_self: SuspicionLevel::default(),
            detected_plots: Vec::new(),
            last_assessment_tick: tick,
        }
    }

    #[must_use]
    pub fn with_risk(mut self, risk: f32) -> Self {
        self.risk = BetrayalRisk::new(risk);
        self
    }

    #[must_use]
    pub fn with_loyalty(mut self, loyalty: f32) -> Self {
        self.loyalty = LoyaltyLevel::new(loyalty);
        self
    }

    pub fn is_likely_to_betray(&self) -> bool {
        self.risk.is_high() && self.loyalty.is_low()
    }

    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.agent_id.raw().to_le_bytes());
        hasher.update(self.current_faction.as_str().as_bytes());
        hasher.update(&self.risk.raw().to_le_bytes());
        hasher.update(&self.grievance.raw().to_le_bytes());
        hasher.update(&self.loyalty.raw().to_le_bytes());
        hasher.update(&self.suspicion_on_self.raw().to_le_bytes());
        hasher.update(&(self.detected_plots.len() as u64).to_le_bytes());
        hasher.finalize()
    }
}

/// Grievance level toward faction.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct GrievanceLevel(f32);

impl GrievanceLevel {
    pub const MIN: f32 = 0.0;
    pub const MAX: f32 = 1.0;

    pub fn new(value: f32) -> Self {
        Self(value.clamp(Self::MIN, Self::MAX))
    }

    pub fn raw(self) -> f32 {
        self.0
    }

    pub fn is_satisfied(self) -> bool {
        self.0 < 0.2
    }

    pub fn is_resentful(self) -> bool {
        self.0 >= 0.5
    }

    pub fn is_vengeful(self) -> bool {
        self.0 >= 0.8
    }

    pub fn modify(&mut self, delta: f32) {
        self.0 = (self.0 + delta).clamp(Self::MIN, Self::MAX);
    }
}

impl Default for GrievanceLevel {
    fn default() -> Self {
        Self(0.0)
    }
}

impl Eq for GrievanceLevel {}

impl std::hash::Hash for GrievanceLevel {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PartialOrd for GrievanceLevel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GrievanceLevel {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// Loyalty level to current faction.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoyaltyLevel(f32);

impl LoyaltyLevel {
    pub const MIN: f32 = 0.0;
    pub const MAX: f32 = 1.0;

    pub fn new(value: f32) -> Self {
        Self(value.clamp(Self::MIN, Self::MAX))
    }

    pub fn raw(self) -> f32 {
        self.0
    }

    pub fn is_low(self) -> bool {
        self.0 < 0.3
    }

    pub fn is_moderate(self) -> bool {
        self.0 >= 0.3 && self.0 < 0.7
    }

    pub fn is_high(self) -> bool {
        self.0 >= 0.7
    }

    pub fn is_fanatical(self) -> bool {
        self.0 >= 0.95
    }

    pub fn modify(&mut self, delta: f32) {
        self.0 = (self.0 + delta).clamp(Self::MIN, Self::MAX);
    }
}

impl Default for LoyaltyLevel {
    fn default() -> Self {
        Self(0.5)
    }
}

impl Eq for LoyaltyLevel {}

impl std::hash::Hash for LoyaltyLevel {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PartialOrd for LoyaltyLevel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LoyaltyLevel {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// Suspicion level on an agent.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SuspicionLevel(f32);

impl SuspicionLevel {
    pub const MIN: f32 = 0.0;
    pub const MAX: f32 = 1.0;

    pub fn new(value: f32) -> Self {
        Self(value.clamp(Self::MIN, Self::MAX))
    }

    pub fn raw(self) -> f32 {
        self.0
    }

    pub fn is_unsuspected(self) -> bool {
        self.0 < 0.2
    }

    pub fn is_watched(self) -> bool {
        self.0 >= 0.4
    }

    pub fn is_suspected(self) -> bool {
        self.0 >= 0.6
    }

    pub fn is_known(self) -> bool {
        self.0 >= 0.9
    }

    pub fn modify(&mut self, delta: f32) {
        self.0 = (self.0 + delta).clamp(Self::MIN, Self::MAX);
    }
}

impl Default for SuspicionLevel {
    fn default() -> Self {
        Self(0.0)
    }
}

impl Eq for SuspicionLevel {}

impl std::hash::Hash for SuspicionLevel {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PartialOrd for SuspicionLevel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SuspicionLevel {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// A detected or active betrayal incident.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BetrayalIncident {
    pub id: BetrayalId,
    pub betrayer: SocialAgentId,
    pub betrayed_faction: SocialFactionId,
    pub target_faction: Option<SocialFactionId>,
    pub kind: BetrayalKind,
    pub status: BetrayalStatus,
    pub detected_tick: Option<u64>,
    pub resolved_tick: Option<u64>,
    pub severity: BetrayalSeverity,
}

impl BetrayalIncident {
    pub fn new(
        id: BetrayalId,
        betrayer: SocialAgentId,
        betrayed_faction: SocialFactionId,
        kind: BetrayalKind,
    ) -> Self {
        Self {
            id,
            betrayer,
            betrayed_faction,
            target_faction: None,
            kind,
            status: BetrayalStatus::Plotting,
            detected_tick: None,
            resolved_tick: None,
            severity: BetrayalSeverity::Minor,
        }
    }

    #[must_use]
    pub fn with_target(mut self, faction: SocialFactionId) -> Self {
        self.target_faction = Some(faction);
        self
    }

    #[must_use]
    pub fn with_severity(mut self, severity: BetrayalSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn detect(&mut self, tick: u64) {
        if self.status == BetrayalStatus::Plotting || self.status == BetrayalStatus::InProgress {
            self.status = BetrayalStatus::Detected;
            self.detected_tick = Some(tick);
        }
    }

    pub fn execute(&mut self) {
        if self.status == BetrayalStatus::Plotting {
            self.status = BetrayalStatus::InProgress;
        }
    }

    pub fn resolve(&mut self, resolution: BetrayalResolution, tick: u64) {
        self.status = BetrayalStatus::Resolved(resolution);
        self.resolved_tick = Some(tick);
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            BetrayalStatus::Plotting | BetrayalStatus::InProgress | BetrayalStatus::Detected
        )
    }

    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.id.raw().to_le_bytes());
        hasher.update(&self.betrayer.raw().to_le_bytes());
        hasher.update(self.betrayed_faction.as_str().as_bytes());
        hasher.update(&[self.kind.as_index()]);
        hasher.update(&[self.status.as_index()]);
        hasher.update(&[self.severity.as_index()]);
        hasher.finalize()
    }
}

/// Kind of betrayal.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BetrayalKind {
    Defection,
    Sabotage,
    InformationLeak,
    Assassination,
    Mutiny,
    Desertion,
    Coup,
    Espionage,
    ResourceTheft,
    Custom(String),
}

impl BetrayalKind {
    pub fn as_index(&self) -> u8 {
        match self {
            Self::Defection => 0,
            Self::Sabotage => 1,
            Self::InformationLeak => 2,
            Self::Assassination => 3,
            Self::Mutiny => 4,
            Self::Desertion => 5,
            Self::Coup => 6,
            Self::Espionage => 7,
            Self::ResourceTheft => 8,
            Self::Custom(_) => 9,
        }
    }
}

/// Status of a betrayal incident.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BetrayalStatus {
    Plotting,
    InProgress,
    Detected,
    Resolved(BetrayalResolution),
}

impl BetrayalStatus {
    pub fn as_index(&self) -> u8 {
        match self {
            Self::Plotting => 0,
            Self::InProgress => 1,
            Self::Detected => 2,
            Self::Resolved(_) => 3,
        }
    }
}

/// Resolution of a betrayal.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BetrayalResolution {
    Succeeded,
    Prevented,
    Pardoned,
    Punished,
    Escaped,
    Killed,
}

/// Severity of betrayal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BetrayalSeverity {
    Minor,
    Moderate,
    Major,
    Catastrophic,
}

impl BetrayalSeverity {
    pub fn as_index(self) -> u8 {
        match self {
            Self::Minor => 0,
            Self::Moderate => 1,
            Self::Major => 2,
            Self::Catastrophic => 3,
        }
    }

    pub fn morale_impact(self) -> f32 {
        match self {
            Self::Minor => -0.05,
            Self::Moderate => -0.15,
            Self::Major => -0.3,
            Self::Catastrophic => -0.5,
        }
    }

    pub fn trust_impact(self) -> f32 {
        match self {
            Self::Minor => -0.1,
            Self::Moderate => -0.25,
            Self::Major => -0.5,
            Self::Catastrophic => -0.8,
        }
    }
}

/// Betrayal event for tracking.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BetrayalEvent {
    pub tick: u64,
    pub kind: BetrayalEventKind,
}

/// Kind of betrayal event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BetrayalEventKind {
    PlotBegan {
        incident_id: BetrayalId,
        betrayer: SocialAgentId,
    },
    PlotDetected {
        incident_id: BetrayalId,
        detector: Option<SocialAgentId>,
    },
    BetrayalExecuted {
        incident_id: BetrayalId,
    },
    BetrayalPrevented {
        incident_id: BetrayalId,
    },
    BetrayalResolved {
        incident_id: BetrayalId,
        resolution: BetrayalResolution,
    },
    SuspicionRaised {
        target: SocialAgentId,
        new_level: SuspicionLevel,
    },
    LoyaltyChanged {
        agent: SocialAgentId,
        old_level: LoyaltyLevel,
        new_level: LoyaltyLevel,
    },
}

/// Tracker for all betrayal-related state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BetrayalTracker {
    profiles: BTreeMap<SocialAgentId, BetrayalProfile>,
    incidents: BTreeMap<BetrayalId, BetrayalIncident>,
    active_incidents_by_faction: BTreeMap<SocialFactionId, Vec<BetrayalId>>,
    next_incident_id: u64,
    factors: BetrayalFactors,
}

impl BetrayalTracker {
    pub fn new() -> Self {
        Self {
            factors: BetrayalFactors::new(),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn with_factors(mut self, factors: BetrayalFactors) -> Self {
        self.factors = factors;
        self
    }

    pub fn register_agent(
        &mut self,
        agent_id: SocialAgentId,
        faction: SocialFactionId,
        tick: u64,
    ) -> &mut BetrayalProfile {
        self.profiles
            .entry(agent_id)
            .or_insert_with(|| BetrayalProfile::new(agent_id, faction, tick))
    }

    pub fn get_profile(&self, agent_id: SocialAgentId) -> Option<&BetrayalProfile> {
        self.profiles.get(&agent_id)
    }

    pub fn get_profile_mut(&mut self, agent_id: SocialAgentId) -> Option<&mut BetrayalProfile> {
        self.profiles.get_mut(&agent_id)
    }

    pub fn create_incident(
        &mut self,
        betrayer: SocialAgentId,
        betrayed_faction: SocialFactionId,
        kind: BetrayalKind,
    ) -> BetrayalId {
        let id = BetrayalId::new(self.next_incident_id);
        self.next_incident_id += 1;

        let incident = BetrayalIncident::new(id, betrayer, betrayed_faction.clone(), kind);
        self.incidents.insert(id, incident);
        self.active_incidents_by_faction
            .entry(betrayed_faction)
            .or_default()
            .push(id);

        id
    }

    pub fn get_incident(&self, id: BetrayalId) -> Option<&BetrayalIncident> {
        self.incidents.get(&id)
    }

    pub fn get_incident_mut(&mut self, id: BetrayalId) -> Option<&mut BetrayalIncident> {
        self.incidents.get_mut(&id)
    }

    pub fn active_incidents(&self) -> impl Iterator<Item = &BetrayalIncident> {
        self.incidents.values().filter(|i| i.is_active())
    }

    pub fn active_incidents_for_faction(
        &self,
        faction: &SocialFactionId,
    ) -> impl Iterator<Item = &BetrayalIncident> {
        self.active_incidents_by_faction
            .get(faction)
            .into_iter()
            .flatten()
            .filter_map(|id| self.incidents.get(id))
            .filter(|i| i.is_active())
    }

    pub fn high_risk_agents(&self) -> impl Iterator<Item = SocialAgentId> + '_ {
        self.profiles
            .iter()
            .filter(|(_, p)| p.risk.is_high())
            .map(|(id, _)| *id)
    }

    pub fn agents_likely_to_betray(&self) -> impl Iterator<Item = SocialAgentId> + '_ {
        self.profiles
            .iter()
            .filter(|(_, p)| p.is_likely_to_betray())
            .map(|(id, _)| *id)
    }

    pub fn profile_count(&self) -> usize {
        self.profiles.len()
    }

    pub fn incident_count(&self) -> usize {
        self.incidents.len()
    }

    pub fn active_incident_count(&self) -> usize {
        self.incidents.values().filter(|i| i.is_active()).count()
    }

    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&(self.profiles.len() as u64).to_le_bytes());
        for profile in self.profiles.values() {
            hasher.update(&profile.checksum().to_le_bytes());
        }
        hasher.update(&(self.incidents.len() as u64).to_le_bytes());
        for incident in self.incidents.values() {
            hasher.update(&incident.checksum().to_le_bytes());
        }
        hasher.finalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_betrayal_risk_thresholds() {
        assert!(BetrayalRisk::new(0.05).is_negligible());
        assert!(BetrayalRisk::new(0.2).is_low());
        assert!(BetrayalRisk::new(0.45).is_moderate());
        assert!(BetrayalRisk::new(0.7).is_high());
        assert!(BetrayalRisk::new(0.9).is_imminent());
    }

    #[test]
    fn test_betrayal_factors() {
        let factors = BetrayalFactors::new();
        let risk = factors.compute_risk(MoraleLevel::new(0.3), 0.2, 0.5, 0.3, 0.1, 0.2);
        assert!(risk.raw() > 0.0);
        assert!(risk.raw() < 1.0);
    }

    #[test]
    fn test_loyalty_levels() {
        assert!(LoyaltyLevel::new(0.2).is_low());
        assert!(LoyaltyLevel::new(0.5).is_moderate());
        assert!(LoyaltyLevel::new(0.8).is_high());
        assert!(LoyaltyLevel::new(0.98).is_fanatical());
    }

    #[test]
    fn test_incident_lifecycle() {
        let mut incident = BetrayalIncident::new(
            BetrayalId::new(1),
            SocialAgentId::new(1),
            SocialFactionId::new("empire"),
            BetrayalKind::Defection,
        );

        assert!(incident.is_active());
        assert_eq!(incident.status, BetrayalStatus::Plotting);

        incident.execute();
        assert_eq!(incident.status, BetrayalStatus::InProgress);

        incident.detect(100);
        assert_eq!(incident.status, BetrayalStatus::Detected);
        assert_eq!(incident.detected_tick, Some(100));

        incident.resolve(BetrayalResolution::Prevented, 110);
        assert!(!incident.is_active());
    }

    #[test]
    fn test_betrayal_tracker() {
        let mut tracker = BetrayalTracker::new();
        let agent = SocialAgentId::new(1);
        let faction = SocialFactionId::new("empire");

        tracker.register_agent(agent, faction.clone(), 100);
        let incident_id = tracker.create_incident(agent, faction, BetrayalKind::Sabotage);

        assert_eq!(tracker.profile_count(), 1);
        assert_eq!(tracker.incident_count(), 1);
        assert!(tracker.get_incident(incident_id).is_some());
    }

    #[test]
    fn test_checksum_determinism() {
        let mut tracker = BetrayalTracker::new();
        tracker.register_agent(SocialAgentId::new(1), SocialFactionId::new("faction"), 100);

        let checksum1 = tracker.checksum();
        let checksum2 = tracker.checksum();
        assert_eq!(checksum1, checksum2);
    }

    #[test]
    fn test_serde_roundtrip() {
        let mut tracker = BetrayalTracker::new();
        let profile =
            tracker.register_agent(SocialAgentId::new(1), SocialFactionId::new("faction"), 100);
        profile.risk = BetrayalRisk::new(0.5);

        let json = serde_json::to_string(&tracker).unwrap();
        let restored: BetrayalTracker = serde_json::from_str(&json).unwrap();
        assert_eq!(tracker.checksum(), restored.checksum());
    }
}
