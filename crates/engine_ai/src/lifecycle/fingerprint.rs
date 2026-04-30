//! Deterministic fingerprint for lifecycle state verification.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::hash::{Hash, Hasher};

use super::tracker::LifecycleTracker;

/// Fingerprint for verifying deterministic lifecycle state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LifecycleFingerprint(pub u32);

impl LifecycleFingerprint {
    #[must_use]
    pub fn raw(self) -> u32 {
        self.0
    }

    #[must_use]
    pub fn matches(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    #[must_use]
    pub fn from_tracker(tracker: &LifecycleTracker, tick: u64) -> Self {
        let mut hasher = StableHasher::new();

        tick.hash(&mut hasher);
        tracker.entity_count().hash(&mut hasher);

        let mut ids: Vec<_> = tracker.all_ids().collect();
        ids.sort();

        for id in ids {
            id.raw().hash(&mut hasher);
            if let Some(stage) = tracker.get_stage(id) {
                stage.stage_name().hash(&mut hasher);
                if let Some(health) = stage.health() {
                    #[expect(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        reason = "health scaled to integer for hashing"
                    )]
                    let health_int = (health * 1000.0) as u32;
                    health_int.hash(&mut hasher);
                }
            }
        }

        Self(hasher.finish_u32())
    }
}

impl fmt::Display for LifecycleFingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "lifecycle:{:08x}", self.0)
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
    use crate::lifecycle::{GrowthPhase, LifecycleConfig, LifecycleId};

    #[test]
    fn test_fingerprint_raw() {
        let fp = LifecycleFingerprint(0x1234_5678);
        assert_eq!(fp.raw(), 0x1234_5678);
    }

    #[test]
    fn test_fingerprint_matches() {
        let fp1 = LifecycleFingerprint(100);
        let fp2 = LifecycleFingerprint(100);
        let fp3 = LifecycleFingerprint(200);

        assert!(fp1.matches(&fp2));
        assert!(!fp1.matches(&fp3));
    }

    #[test]
    fn test_fingerprint_display() {
        let fp = LifecycleFingerprint(0xdead_beef);
        assert_eq!(format!("{fp}"), "lifecycle:deadbeef");
    }

    #[test]
    fn test_fingerprint_from_empty_tracker() {
        let tracker = LifecycleTracker::new(LifecycleConfig::standard());
        let fp1 = LifecycleFingerprint::from_tracker(&tracker, 0);
        let fp2 = LifecycleFingerprint::from_tracker(&tracker, 0);

        assert!(fp1.matches(&fp2));
    }

    #[test]
    fn test_fingerprint_determinism() {
        let config = LifecycleConfig::standard();
        let mut tracker1 = LifecycleTracker::new(config.clone());
        let mut tracker2 = LifecycleTracker::new(config);

        tracker1.spawn_egg(LifecycleId::new(1), 0);
        tracker2.spawn_egg(LifecycleId::new(1), 0);

        tracker1.spawn_living(LifecycleId::new(2), GrowthPhase::Adult, 0);
        tracker2.spawn_living(LifecycleId::new(2), GrowthPhase::Adult, 0);

        let fp1 = LifecycleFingerprint::from_tracker(&tracker1, 0);
        let fp2 = LifecycleFingerprint::from_tracker(&tracker2, 0);

        assert!(fp1.matches(&fp2));
    }

    #[test]
    fn test_fingerprint_changes_with_state() {
        let config = LifecycleConfig::standard();
        let mut tracker = LifecycleTracker::new(config);

        let fp_empty = LifecycleFingerprint::from_tracker(&tracker, 0);

        tracker.spawn_egg(LifecycleId::new(1), 0);
        let fp_with_egg = LifecycleFingerprint::from_tracker(&tracker, 0);

        assert!(!fp_empty.matches(&fp_with_egg));
    }

    #[test]
    fn test_fingerprint_changes_with_tick() {
        let config = LifecycleConfig::standard();
        let tracker = LifecycleTracker::new(config);

        let fp_tick_0 = LifecycleFingerprint::from_tracker(&tracker, 0);
        let fp_tick_1 = LifecycleFingerprint::from_tracker(&tracker, 1);

        assert!(!fp_tick_0.matches(&fp_tick_1));
    }

    #[test]
    fn test_fingerprint_serde() {
        let fp = LifecycleFingerprint(0xabcd_ef01);
        let json = serde_json::to_string(&fp).unwrap();
        let restored: LifecycleFingerprint = serde_json::from_str(&json).unwrap();
        assert!(fp.matches(&restored));
    }
}
