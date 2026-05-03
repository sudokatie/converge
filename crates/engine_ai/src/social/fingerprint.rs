//! Deterministic fingerprint for social state verification.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::hash::{Hash, Hasher};

use super::betrayal::BetrayalTracker;
use super::diplomacy::DiplomacyTracker;
use super::morale::MoraleTracker;
use super::panic::PanicTracker;

/// Fingerprint for verifying deterministic social state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SocialFingerprint(pub u32);

impl SocialFingerprint {
    #[must_use]
    pub fn raw(self) -> u32 {
        self.0
    }

    #[must_use]
    pub fn matches(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    #[must_use]
    pub fn from_trackers(
        morale: &MoraleTracker,
        panic: &PanicTracker,
        betrayal: &BetrayalTracker,
        diplomacy: &DiplomacyTracker,
        tick: u64,
    ) -> Self {
        let mut hasher = StableHasher::new();

        tick.hash(&mut hasher);
        morale.checksum().hash(&mut hasher);
        panic.checksum().hash(&mut hasher);
        betrayal.checksum().hash(&mut hasher);
        diplomacy.checksum().hash(&mut hasher);

        Self(hasher.finish_u32())
    }

    #[must_use]
    pub fn from_components(
        morale_checksum: u32,
        panic_checksum: u32,
        betrayal_checksum: u32,
        diplomacy_checksum: u32,
        tick: u64,
    ) -> Self {
        let mut hasher = StableHasher::new();

        tick.hash(&mut hasher);
        morale_checksum.hash(&mut hasher);
        panic_checksum.hash(&mut hasher);
        betrayal_checksum.hash(&mut hasher);
        diplomacy_checksum.hash(&mut hasher);

        Self(hasher.finish_u32())
    }

    #[must_use]
    pub fn combine(fingerprints: &[Self]) -> Self {
        let mut hasher = StableHasher::new();

        for fp in fingerprints {
            fp.0.hash(&mut hasher);
        }

        Self(hasher.finish_u32())
    }
}

impl fmt::Display for SocialFingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "social:{:08x}", self.0)
    }
}

/// Fingerprint for morale subsystem only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MoraleFingerprint(pub u32);

impl MoraleFingerprint {
    #[must_use]
    pub fn raw(self) -> u32 {
        self.0
    }

    #[must_use]
    pub fn matches(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    #[must_use]
    pub fn from_tracker(tracker: &MoraleTracker, tick: u64) -> Self {
        let mut hasher = StableHasher::new();

        tick.hash(&mut hasher);
        tracker.checksum().hash(&mut hasher);

        Self(hasher.finish_u32())
    }
}

impl fmt::Display for MoraleFingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "morale:{:08x}", self.0)
    }
}

/// Fingerprint for betrayal subsystem only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BetrayalFingerprint(pub u32);

impl BetrayalFingerprint {
    #[must_use]
    pub fn raw(self) -> u32 {
        self.0
    }

    #[must_use]
    pub fn matches(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    #[must_use]
    pub fn from_tracker(tracker: &BetrayalTracker, tick: u64) -> Self {
        let mut hasher = StableHasher::new();

        tick.hash(&mut hasher);
        tracker.checksum().hash(&mut hasher);

        Self(hasher.finish_u32())
    }
}

impl fmt::Display for BetrayalFingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "betrayal:{:08x}", self.0)
    }
}

/// Fingerprint for panic subsystem only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PanicFingerprint(pub u32);

impl PanicFingerprint {
    #[must_use]
    pub fn raw(self) -> u32 {
        self.0
    }

    #[must_use]
    pub fn matches(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    #[must_use]
    pub fn from_tracker(tracker: &PanicTracker, tick: u64) -> Self {
        let mut hasher = StableHasher::new();

        tick.hash(&mut hasher);
        tracker.checksum().hash(&mut hasher);

        Self(hasher.finish_u32())
    }
}

impl fmt::Display for PanicFingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "panic:{:08x}", self.0)
    }
}

/// Fingerprint for diplomacy subsystem only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DiplomacyFingerprint(pub u32);

impl DiplomacyFingerprint {
    #[must_use]
    pub fn raw(self) -> u32 {
        self.0
    }

    #[must_use]
    pub fn matches(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    #[must_use]
    pub fn from_tracker(tracker: &DiplomacyTracker, tick: u64) -> Self {
        let mut hasher = StableHasher::new();

        tick.hash(&mut hasher);
        tracker.checksum().hash(&mut hasher);

        Self(hasher.finish_u32())
    }
}

impl fmt::Display for DiplomacyFingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "diplomacy:{:08x}", self.0)
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

    #[test]
    fn test_fingerprint_raw() {
        let fp = SocialFingerprint(0x1234_5678);
        assert_eq!(fp.raw(), 0x1234_5678);
    }

    #[test]
    fn test_fingerprint_matches() {
        let fp1 = SocialFingerprint(100);
        let fp2 = SocialFingerprint(100);
        let fp3 = SocialFingerprint(200);

        assert!(fp1.matches(&fp2));
        assert!(!fp1.matches(&fp3));
    }

    #[test]
    fn test_fingerprint_display() {
        let fp = SocialFingerprint(0xdead_beef);
        assert_eq!(format!("{fp}"), "social:deadbeef");
    }

    #[test]
    fn test_fingerprint_from_empty_trackers() {
        let morale = MoraleTracker::new();
        let panic = PanicTracker::new();
        let betrayal = BetrayalTracker::new();
        let diplomacy = DiplomacyTracker::new();

        let fp1 = SocialFingerprint::from_trackers(&morale, &panic, &betrayal, &diplomacy, 0);
        let fp2 = SocialFingerprint::from_trackers(&morale, &panic, &betrayal, &diplomacy, 0);

        assert!(fp1.matches(&fp2));
    }

    #[test]
    fn test_fingerprint_changes_with_tick() {
        let morale = MoraleTracker::new();
        let panic = PanicTracker::new();
        let betrayal = BetrayalTracker::new();
        let diplomacy = DiplomacyTracker::new();

        let fp_tick_0 = SocialFingerprint::from_trackers(&morale, &panic, &betrayal, &diplomacy, 0);
        let fp_tick_1 = SocialFingerprint::from_trackers(&morale, &panic, &betrayal, &diplomacy, 1);

        assert!(!fp_tick_0.matches(&fp_tick_1));
    }

    #[test]
    fn test_fingerprint_from_components() {
        let fp1 = SocialFingerprint::from_components(100, 200, 300, 400, 0);
        let fp2 = SocialFingerprint::from_components(100, 200, 300, 400, 0);
        let fp3 = SocialFingerprint::from_components(100, 200, 300, 401, 0);

        assert!(fp1.matches(&fp2));
        assert!(!fp1.matches(&fp3));
    }

    #[test]
    fn test_fingerprint_combine() {
        let fps = vec![
            SocialFingerprint(100),
            SocialFingerprint(200),
            SocialFingerprint(300),
        ];

        let combined1 = SocialFingerprint::combine(&fps);
        let combined2 = SocialFingerprint::combine(&fps);

        assert!(combined1.matches(&combined2));
    }

    #[test]
    fn test_morale_fingerprint() {
        let tracker = MoraleTracker::new();

        let fp1 = MoraleFingerprint::from_tracker(&tracker, 0);
        let fp2 = MoraleFingerprint::from_tracker(&tracker, 0);

        assert!(fp1.matches(&fp2));
        assert!(format!("{fp1}").starts_with("morale:"));
    }

    #[test]
    fn test_betrayal_fingerprint() {
        let tracker = BetrayalTracker::new();

        let fp1 = BetrayalFingerprint::from_tracker(&tracker, 0);
        let fp2 = BetrayalFingerprint::from_tracker(&tracker, 0);

        assert!(fp1.matches(&fp2));
    }

    #[test]
    fn test_panic_fingerprint() {
        let tracker = PanicTracker::new();

        let fp1 = PanicFingerprint::from_tracker(&tracker, 0);
        let fp2 = PanicFingerprint::from_tracker(&tracker, 0);

        assert!(fp1.matches(&fp2));
    }

    #[test]
    fn test_diplomacy_fingerprint() {
        let tracker = DiplomacyTracker::new();

        let fp1 = DiplomacyFingerprint::from_tracker(&tracker, 0);
        let fp2 = DiplomacyFingerprint::from_tracker(&tracker, 0);

        assert!(fp1.matches(&fp2));
    }

    #[test]
    fn test_fingerprint_serde() {
        let fp = SocialFingerprint(0xabcd_ef01);
        let json = serde_json::to_string(&fp).unwrap();
        let restored: SocialFingerprint = serde_json::from_str(&json).unwrap();
        assert!(fp.matches(&restored));
    }
}
