//! Disease system events.

use serde::{Deserialize, Serialize};

use super::ids::{ContaminationZoneId, DiseaseRegionId, HostId, PathogenId, StrainId};
use super::mutation::TraitChanges;

/// Event kind for disease system changes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DiseaseEventKind {
    /// Host was exposed to a pathogen.
    Exposed { host_id: HostId, strain: StrainId },
    /// Host progressed to incubation.
    IncubationStarted {
        host_id: HostId,
        pathogen_id: PathogenId,
    },
    /// Host became symptomatic.
    SymptomsAppeared {
        host_id: HostId,
        pathogen_id: PathogenId,
        severity: f32,
    },
    /// Host entered recovery.
    RecoveryStarted {
        host_id: HostId,
        pathogen_id: PathogenId,
    },
    /// Host fully recovered.
    Recovered {
        host_id: HostId,
        pathogen_id: PathogenId,
        immunity_duration: u64,
    },
    /// Host became a carrier.
    BecameCarrier {
        host_id: HostId,
        pathogen_id: PathogenId,
    },
    /// Latent infection activated.
    LatentReactivated {
        host_id: HostId,
        pathogen_id: PathogenId,
    },
    /// Host died from disease.
    Died {
        host_id: HostId,
        pathogen_id: PathogenId,
    },
    /// Immunity expired.
    ImmunityExpired {
        host_id: HostId,
        pathogen_id: PathogenId,
    },
    /// Pathogen mutated.
    Mutated {
        host_id: HostId,
        from_strain: StrainId,
        to_strain: StrainId,
        is_major_variant: bool,
        changes: TraitChanges,
    },
    /// Contamination zone created.
    ZoneCreated {
        zone_id: ContaminationZoneId,
        region_id: DiseaseRegionId,
        pathogen_id: PathogenId,
    },
    /// Contamination zone decayed.
    ZoneDecayed { zone_id: ContaminationZoneId },
    /// Disease spread to new region.
    SpreadToRegion {
        from_region: DiseaseRegionId,
        to_region: DiseaseRegionId,
        strain: StrainId,
    },
    /// Outbreak started in region.
    OutbreakStarted {
        region_id: DiseaseRegionId,
        pathogen_id: PathogenId,
        initial_cases: u32,
    },
    /// Outbreak contained/ended.
    OutbreakEnded {
        region_id: DiseaseRegionId,
        pathogen_id: PathogenId,
        total_cases: u32,
        total_deaths: u32,
    },
    /// Treatment applied to host.
    TreatmentApplied {
        host_id: HostId,
        pathogen_id: PathogenId,
    },
    /// Treatment cured infection.
    TreatmentSucceeded {
        host_id: HostId,
        pathogen_id: PathogenId,
    },
}

/// A disease system event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DiseaseEvent {
    /// Tick when event occurred.
    pub tick: u64,
    /// Event kind.
    pub kind: DiseaseEventKind,
}

impl DiseaseEvent {
    #[must_use]
    pub fn new(tick: u64, kind: DiseaseEventKind) -> Self {
        Self { tick, kind }
    }

    #[must_use]
    pub fn exposed(tick: u64, host_id: HostId, strain: StrainId) -> Self {
        Self::new(tick, DiseaseEventKind::Exposed { host_id, strain })
    }

    #[must_use]
    pub fn incubation_started(tick: u64, host_id: HostId, pathogen_id: PathogenId) -> Self {
        Self::new(
            tick,
            DiseaseEventKind::IncubationStarted {
                host_id,
                pathogen_id,
            },
        )
    }

    #[must_use]
    pub fn symptoms_appeared(
        tick: u64,
        host_id: HostId,
        pathogen_id: PathogenId,
        severity: f32,
    ) -> Self {
        Self::new(
            tick,
            DiseaseEventKind::SymptomsAppeared {
                host_id,
                pathogen_id,
                severity,
            },
        )
    }

    #[must_use]
    pub fn recovered(
        tick: u64,
        host_id: HostId,
        pathogen_id: PathogenId,
        immunity_duration: u64,
    ) -> Self {
        Self::new(
            tick,
            DiseaseEventKind::Recovered {
                host_id,
                pathogen_id,
                immunity_duration,
            },
        )
    }

    #[must_use]
    pub fn died(tick: u64, host_id: HostId, pathogen_id: PathogenId) -> Self {
        Self::new(
            tick,
            DiseaseEventKind::Died {
                host_id,
                pathogen_id,
            },
        )
    }

    #[must_use]
    pub fn mutated(
        tick: u64,
        host_id: HostId,
        from_strain: StrainId,
        to_strain: StrainId,
        is_major: bool,
        changes: TraitChanges,
    ) -> Self {
        Self::new(
            tick,
            DiseaseEventKind::Mutated {
                host_id,
                from_strain,
                to_strain,
                is_major_variant: is_major,
                changes,
            },
        )
    }

    #[must_use]
    pub fn zone_created(
        tick: u64,
        zone_id: ContaminationZoneId,
        region_id: DiseaseRegionId,
        pathogen_id: PathogenId,
    ) -> Self {
        Self::new(
            tick,
            DiseaseEventKind::ZoneCreated {
                zone_id,
                region_id,
                pathogen_id,
            },
        )
    }

    #[must_use]
    pub fn outbreak_started(
        tick: u64,
        region_id: DiseaseRegionId,
        pathogen_id: PathogenId,
        initial_cases: u32,
    ) -> Self {
        Self::new(
            tick,
            DiseaseEventKind::OutbreakStarted {
                region_id,
                pathogen_id,
                initial_cases,
            },
        )
    }

    /// Whether this is a critical event requiring immediate attention.
    #[must_use]
    pub fn is_critical(&self) -> bool {
        matches!(
            self.kind,
            DiseaseEventKind::Died { .. }
                | DiseaseEventKind::OutbreakStarted { .. }
                | DiseaseEventKind::SpreadToRegion { .. }
                | DiseaseEventKind::Mutated {
                    is_major_variant: true,
                    ..
                }
        )
    }

    /// Whether this event affects a specific host.
    #[must_use]
    pub fn affects_host(&self, host: HostId) -> bool {
        match &self.kind {
            DiseaseEventKind::Exposed { host_id, .. }
            | DiseaseEventKind::IncubationStarted { host_id, .. }
            | DiseaseEventKind::SymptomsAppeared { host_id, .. }
            | DiseaseEventKind::RecoveryStarted { host_id, .. }
            | DiseaseEventKind::Recovered { host_id, .. }
            | DiseaseEventKind::BecameCarrier { host_id, .. }
            | DiseaseEventKind::LatentReactivated { host_id, .. }
            | DiseaseEventKind::Died { host_id, .. }
            | DiseaseEventKind::ImmunityExpired { host_id, .. }
            | DiseaseEventKind::Mutated { host_id, .. }
            | DiseaseEventKind::TreatmentApplied { host_id, .. }
            | DiseaseEventKind::TreatmentSucceeded { host_id, .. } => *host_id == host,
            _ => false,
        }
    }

    /// Get the pathogen ID if this event is pathogen-related.
    #[must_use]
    pub fn pathogen_id(&self) -> Option<&PathogenId> {
        match &self.kind {
            DiseaseEventKind::Exposed { strain, .. }
            | DiseaseEventKind::SpreadToRegion { strain, .. } => Some(&strain.pathogen),
            DiseaseEventKind::IncubationStarted { pathogen_id, .. }
            | DiseaseEventKind::SymptomsAppeared { pathogen_id, .. }
            | DiseaseEventKind::RecoveryStarted { pathogen_id, .. }
            | DiseaseEventKind::Recovered { pathogen_id, .. }
            | DiseaseEventKind::BecameCarrier { pathogen_id, .. }
            | DiseaseEventKind::LatentReactivated { pathogen_id, .. }
            | DiseaseEventKind::Died { pathogen_id, .. }
            | DiseaseEventKind::ImmunityExpired { pathogen_id, .. }
            | DiseaseEventKind::TreatmentApplied { pathogen_id, .. }
            | DiseaseEventKind::TreatmentSucceeded { pathogen_id, .. }
            | DiseaseEventKind::ZoneCreated { pathogen_id, .. }
            | DiseaseEventKind::OutbreakStarted { pathogen_id, .. }
            | DiseaseEventKind::OutbreakEnded { pathogen_id, .. } => Some(pathogen_id),
            DiseaseEventKind::Mutated { from_strain, .. } => Some(&from_strain.pathogen),
            DiseaseEventKind::ZoneDecayed { .. } => None,
        }
    }
}

/// Batch of events from a tick.
#[derive(Clone, Debug, Default)]
pub struct DiseaseTickEvents {
    pub events: Vec<DiseaseEvent>,
    pub new_infections: u32,
    pub stage_transitions: u32,
    pub recoveries: u32,
    pub deaths: u32,
    pub mutations: u32,
}

impl DiseaseTickEvents {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, event: DiseaseEvent) {
        match &event.kind {
            DiseaseEventKind::Exposed { .. } => self.new_infections += 1,
            DiseaseEventKind::IncubationStarted { .. }
            | DiseaseEventKind::SymptomsAppeared { .. }
            | DiseaseEventKind::RecoveryStarted { .. }
            | DiseaseEventKind::BecameCarrier { .. }
            | DiseaseEventKind::LatentReactivated { .. } => self.stage_transitions += 1,
            DiseaseEventKind::Recovered { .. } => self.recoveries += 1,
            DiseaseEventKind::Died { .. } => self.deaths += 1,
            DiseaseEventKind::Mutated { .. } => self.mutations += 1,
            _ => {}
        }
        self.events.push(event);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &DiseaseEvent> {
        self.events.iter()
    }

    #[must_use]
    pub fn has_critical(&self) -> bool {
        self.events.iter().any(DiseaseEvent::is_critical)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disease_event_exposed() {
        let event =
            DiseaseEvent::exposed(100, HostId::new(1), StrainId::base(PathogenId::plague()));

        assert_eq!(event.tick, 100);
        assert!(matches!(event.kind, DiseaseEventKind::Exposed { .. }));
        assert!(event.affects_host(HostId::new(1)));
        assert!(!event.affects_host(HostId::new(2)));
    }

    #[test]
    fn test_disease_event_died() {
        let event = DiseaseEvent::died(200, HostId::new(5), PathogenId::plague());

        assert!(event.is_critical());
        assert_eq!(event.pathogen_id(), Some(&PathogenId::plague()));
    }

    #[test]
    fn test_disease_event_mutated() {
        let event = DiseaseEvent::mutated(
            100,
            HostId::new(1),
            StrainId::base(PathogenId::plague()),
            StrainId::new(PathogenId::plague(), 1),
            true,
            TraitChanges::default(),
        );

        assert!(event.is_critical());
    }

    #[test]
    fn test_disease_event_outbreak() {
        let event = DiseaseEvent::outbreak_started(
            0,
            DiseaseRegionId::new("region1"),
            PathogenId::plague(),
            5,
        );

        assert!(event.is_critical());
    }

    #[test]
    fn test_disease_tick_events() {
        let mut events = DiseaseTickEvents::new();

        events.push(DiseaseEvent::exposed(
            0,
            HostId::new(1),
            StrainId::base(PathogenId::plague()),
        ));
        events.push(DiseaseEvent::incubation_started(
            0,
            HostId::new(1),
            PathogenId::plague(),
        ));
        events.push(DiseaseEvent::recovered(
            0,
            HostId::new(2),
            PathogenId::fever(),
            1000,
        ));
        events.push(DiseaseEvent::died(0, HostId::new(3), PathogenId::rot()));

        assert_eq!(events.len(), 4);
        assert_eq!(events.new_infections, 1);
        assert_eq!(events.stage_transitions, 1);
        assert_eq!(events.recoveries, 1);
        assert_eq!(events.deaths, 1);
        assert!(events.has_critical());
    }

    #[test]
    fn test_serde_disease_event() {
        let event = DiseaseEvent::symptoms_appeared(100, HostId::new(1), PathogenId::plague(), 0.5);

        let json = serde_json::to_string(&event).unwrap();
        let restored: DiseaseEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(event.tick, restored.tick);
        if let (
            DiseaseEventKind::SymptomsAppeared { severity: s1, .. },
            DiseaseEventKind::SymptomsAppeared { severity: s2, .. },
        ) = (&event.kind, &restored.kind)
        {
            assert!((s1 - s2).abs() < f32::EPSILON);
        }
    }
}
