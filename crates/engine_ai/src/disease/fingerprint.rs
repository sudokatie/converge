//! Deterministic fingerprints for disease state verification.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::hash::{Hash, Hasher};

use super::tracker::DiseaseTracker;

/// Fingerprint for verifying deterministic disease state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DiseaseFingerprint(pub u32);

impl DiseaseFingerprint {
    #[must_use]
    pub fn raw(self) -> u32 {
        self.0
    }

    #[must_use]
    pub fn matches(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    #[must_use]
    pub fn from_tracker(tracker: &DiseaseTracker, tick: u64) -> Self {
        let mut hasher = StableHasher::new();

        tick.hash(&mut hasher);
        tracker.host_count().hash(&mut hasher);
        tracker.infected_count().hash(&mut hasher);
        tracker.zone_count().hash(&mut hasher);

        let registry_checksum = tracker.pathogen_registry().checksum();
        registry_checksum.hash(&mut hasher);

        let zone_checksum = tracker.contamination_registry().checksum();
        zone_checksum.hash(&mut hasher);

        for host_state in tracker.infected_hosts() {
            host_state.host_id.raw().hash(&mut hasher);
            host_state.checksum().hash(&mut hasher);
        }

        Self(hasher.finish_u32())
    }
}

impl fmt::Display for DiseaseFingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "disease:{:08x}", self.0)
    }
}

struct StableHasher {
    state: u64,
}

impl StableHasher {
    fn new() -> Self {
        Self {
            state: 0xcbf2_9ce4_8422_2325,
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "intentional truncation for u32 hash"
    )]
    fn finish_u32(&self) -> u32 {
        (self.state ^ (self.state >> 32)) as u32
    }
}

impl Hasher for StableHasher {
    fn finish(&self) -> u64 {
        self.state
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.state ^= u64::from(*byte);
            self.state = self.state.wrapping_mul(0x0100_0000_01b3);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::disease::pathogen::presets;
    use crate::disease::{DiseaseConfig, HostId, PathogenId, StrainId};

    fn make_test_tracker() -> DiseaseTracker {
        let config = DiseaseConfig::default();
        let mut tracker = DiseaseTracker::new(config);

        let registry = presets::create_preset_registry();
        for def in registry.iter() {
            tracker.register_pathogen(def.clone());
        }

        tracker
    }

    #[test]
    fn test_fingerprint_raw() {
        let fp = DiseaseFingerprint(0x1234_5678);
        assert_eq!(fp.raw(), 0x1234_5678);
    }

    #[test]
    fn test_fingerprint_matches() {
        let fp1 = DiseaseFingerprint(100);
        let fp2 = DiseaseFingerprint(100);
        let fp3 = DiseaseFingerprint(200);

        assert!(fp1.matches(&fp2));
        assert!(!fp1.matches(&fp3));
    }

    #[test]
    fn test_fingerprint_display() {
        let fp = DiseaseFingerprint(0xdead_beef);
        assert_eq!(format!("{fp}"), "disease:deadbeef");
    }

    #[test]
    fn test_fingerprint_from_empty_tracker() {
        let tracker = make_test_tracker();
        let fp1 = DiseaseFingerprint::from_tracker(&tracker, 0);
        let fp2 = DiseaseFingerprint::from_tracker(&tracker, 0);

        assert!(fp1.matches(&fp2));
    }

    #[test]
    fn test_fingerprint_determinism() {
        let mut tracker1 = make_test_tracker();
        let mut tracker2 = make_test_tracker();

        tracker1.register_host(HostId::new(1), "human");
        tracker2.register_host(HostId::new(1), "human");

        let plague_traits = tracker1
            .pathogen_registry()
            .get(&PathogenId::plague())
            .unwrap()
            .base_traits
            .clone();

        tracker1.expose_host(
            HostId::new(1),
            StrainId::base(PathogenId::plague()),
            plague_traits.clone(),
            0,
        );
        tracker2.expose_host(
            HostId::new(1),
            StrainId::base(PathogenId::plague()),
            plague_traits,
            0,
        );

        let fp1 = DiseaseFingerprint::from_tracker(&tracker1, 0);
        let fp2 = DiseaseFingerprint::from_tracker(&tracker2, 0);

        assert!(fp1.matches(&fp2));
    }

    #[test]
    fn test_fingerprint_changes_with_state() {
        let mut tracker = make_test_tracker();
        let fp_empty = DiseaseFingerprint::from_tracker(&tracker, 0);

        tracker.register_host(HostId::new(1), "human");
        let fp_with_host = DiseaseFingerprint::from_tracker(&tracker, 0);

        assert!(!fp_empty.matches(&fp_with_host));
    }

    #[test]
    fn test_fingerprint_changes_with_tick() {
        let tracker = make_test_tracker();

        let fp_tick_0 = DiseaseFingerprint::from_tracker(&tracker, 0);
        let fp_tick_1 = DiseaseFingerprint::from_tracker(&tracker, 1);

        assert!(!fp_tick_0.matches(&fp_tick_1));
    }

    #[test]
    fn test_fingerprint_serde() {
        let fp = DiseaseFingerprint(0xabcd_ef01);
        let json = serde_json::to_string(&fp).unwrap();
        let restored: DiseaseFingerprint = serde_json::from_str(&json).unwrap();
        assert!(fp.matches(&restored));
    }
}
