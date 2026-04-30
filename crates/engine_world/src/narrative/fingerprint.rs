//! Stable fingerprints and checksums for narrative state.

use serde::{Deserialize, Serialize};

/// Fingerprint of an event definition for change detection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventFingerprint(pub u64);

impl EventFingerprint {
    /// Create a new fingerprint from raw value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Compute fingerprint from event definition properties.
    #[must_use]
    pub fn from_definition(
        id: &str,
        kind: super::NarrativeEventKind,
        duration: u64,
        enabled: bool,
        trigger_count: usize,
    ) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(id.as_bytes());
        hasher.update(&[kind as u8]);
        hasher.update(&duration.to_le_bytes());
        hasher.update(&[u8::from(enabled)]);
        #[allow(clippy::cast_possible_truncation)]
        let count = trigger_count as u32;
        hasher.update(&count.to_le_bytes());
        Self(u64::from(hasher.finalize()))
    }

    /// Get raw fingerprint value.
    #[must_use]
    pub const fn value(&self) -> u64 {
        self.0
    }

    /// Combine with another fingerprint.
    #[must_use]
    pub fn combine(&self, other: &Self) -> Self {
        Self(self.0.wrapping_mul(31).wrapping_add(other.0))
    }
}

/// Checksum of narrative runtime state for synchronization.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StateChecksum {
    /// Checksum of active events.
    pub active_events: u32,

    /// Checksum of cooldown states.
    pub cooldowns: u32,

    /// Checksum of output queue.
    pub outputs: u32,

    /// Checksum of flags.
    pub flags: u32,

    /// Tick when checksum was computed.
    pub tick: u64,
}

impl StateChecksum {
    /// Create a new checksum.
    #[must_use]
    pub const fn new(tick: u64) -> Self {
        Self {
            active_events: 0,
            cooldowns: 0,
            outputs: 0,
            flags: 0,
            tick,
        }
    }

    /// Set active events checksum.
    #[must_use]
    pub const fn with_active_events(mut self, checksum: u32) -> Self {
        self.active_events = checksum;
        self
    }

    /// Set cooldowns checksum.
    #[must_use]
    pub const fn with_cooldowns(mut self, checksum: u32) -> Self {
        self.cooldowns = checksum;
        self
    }

    /// Set outputs checksum.
    #[must_use]
    pub const fn with_outputs(mut self, checksum: u32) -> Self {
        self.outputs = checksum;
        self
    }

    /// Set flags checksum.
    #[must_use]
    pub const fn with_flags(mut self, checksum: u32) -> Self {
        self.flags = checksum;
        self
    }

    /// Compute combined checksum.
    #[must_use]
    pub fn combined(&self) -> u64 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.tick.to_le_bytes());
        hasher.update(&self.active_events.to_le_bytes());
        hasher.update(&self.cooldowns.to_le_bytes());
        hasher.update(&self.outputs.to_le_bytes());
        hasher.update(&self.flags.to_le_bytes());
        u64::from(hasher.finalize())
    }

    /// Check if this checksum matches another.
    #[must_use]
    pub fn matches(&self, other: &Self) -> bool {
        self.active_events == other.active_events
            && self.cooldowns == other.cooldowns
            && self.outputs == other.outputs
            && self.flags == other.flags
    }

    /// Get mismatched components.
    #[must_use]
    pub fn mismatches(&self, other: &Self) -> Vec<&'static str> {
        let mut result = Vec::new();
        if self.active_events != other.active_events {
            result.push("active_events");
        }
        if self.cooldowns != other.cooldowns {
            result.push("cooldowns");
        }
        if self.outputs != other.outputs {
            result.push("outputs");
        }
        if self.flags != other.flags {
            result.push("flags");
        }
        result
    }
}

/// Builder for computing state checksums.
#[derive(Clone, Debug, Default)]
#[allow(clippy::struct_field_names)]
pub struct ChecksumBuilder {
    active_hasher: crc32fast::Hasher,
    cooldown_hasher: crc32fast::Hasher,
    flags_hasher: crc32fast::Hasher,
}

impl ChecksumBuilder {
    /// Create a new builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an active event to the checksum.
    pub fn add_active_event(&mut self, event_id: u64, def_id: &str, start_tick: u64) {
        self.active_hasher.update(&event_id.to_le_bytes());
        self.active_hasher.update(def_id.as_bytes());
        self.active_hasher.update(&start_tick.to_le_bytes());
    }

    /// Add a cooldown state to the checksum.
    pub fn add_cooldown(&mut self, def_id: &str, fire_count: u32, ready_at: u64) {
        self.cooldown_hasher.update(def_id.as_bytes());
        self.cooldown_hasher.update(&fire_count.to_le_bytes());
        self.cooldown_hasher.update(&ready_at.to_le_bytes());
    }

    /// Add a flag to the checksum.
    pub fn add_flag(&mut self, flag: &str) {
        self.flags_hasher.update(flag.as_bytes());
        self.flags_hasher.update(&[0u8]);
    }

    /// Build the checksum.
    #[must_use]
    pub fn build(self, tick: u64, output_checksum: u32) -> StateChecksum {
        StateChecksum {
            active_events: self.active_hasher.finalize(),
            cooldowns: self.cooldown_hasher.finalize(),
            outputs: output_checksum,
            flags: self.flags_hasher.finalize(),
            tick,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::narrative::NarrativeEventKind;

    #[test]
    fn fingerprint_deterministic() {
        let f1 = EventFingerprint::from_definition("test", NarrativeEventKind::Radio, 100, true, 2);
        let f2 = EventFingerprint::from_definition("test", NarrativeEventKind::Radio, 100, true, 2);
        assert_eq!(f1, f2);
    }

    #[test]
    fn fingerprint_differs_on_change() {
        let f1 = EventFingerprint::from_definition("test", NarrativeEventKind::Radio, 100, true, 2);
        let f2 =
            EventFingerprint::from_definition("test", NarrativeEventKind::Disaster, 100, true, 2);
        assert_ne!(f1, f2);
    }

    #[test]
    fn fingerprint_combine() {
        let f1 = EventFingerprint::new(100);
        let f2 = EventFingerprint::new(200);
        let combined = f1.combine(&f2);
        assert_ne!(combined, f1);
        assert_ne!(combined, f2);
    }

    #[test]
    fn checksum_matches() {
        let c1 = StateChecksum::new(100)
            .with_active_events(1)
            .with_cooldowns(2)
            .with_outputs(3)
            .with_flags(4);

        let c2 = StateChecksum::new(100)
            .with_active_events(1)
            .with_cooldowns(2)
            .with_outputs(3)
            .with_flags(4);

        assert!(c1.matches(&c2));
    }

    #[test]
    fn checksum_mismatches() {
        let c1 = StateChecksum::new(100)
            .with_active_events(1)
            .with_cooldowns(2);

        let c2 = StateChecksum::new(100)
            .with_active_events(1)
            .with_cooldowns(3);

        assert!(!c1.matches(&c2));
        assert_eq!(c1.mismatches(&c2), vec!["cooldowns"]);
    }

    #[test]
    fn checksum_combined_deterministic() {
        let c1 = StateChecksum::new(100).with_active_events(1);
        let c2 = StateChecksum::new(100).with_active_events(1);
        assert_eq!(c1.combined(), c2.combined());
    }

    #[test]
    fn builder_deterministic() {
        let mut b1 = ChecksumBuilder::new();
        b1.add_active_event(1, "test", 100);
        b1.add_cooldown("test", 2, 200);
        b1.add_flag("flag1");
        let c1 = b1.build(100, 0);

        let mut b2 = ChecksumBuilder::new();
        b2.add_active_event(1, "test", 100);
        b2.add_cooldown("test", 2, 200);
        b2.add_flag("flag1");
        let c2 = b2.build(100, 0);

        assert!(c1.matches(&c2));
    }

    #[test]
    fn serde_round_trip() {
        let fp = EventFingerprint::new(12345);
        let json = serde_json::to_string(&fp).unwrap();
        let recovered: EventFingerprint = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, fp);

        let cs = StateChecksum::new(100)
            .with_active_events(1)
            .with_cooldowns(2);
        let json = serde_json::to_string(&cs).unwrap();
        let recovered: StateChecksum = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, cs);
    }
}
