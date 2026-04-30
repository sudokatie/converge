//! Stable fingerprints and checksums for geological simulation state.

use serde::{Deserialize, Serialize};

/// Fingerprint of geological simulation state for change detection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GeologyFingerprint(pub u64);

impl GeologyFingerprint {
    /// Create a new fingerprint from raw value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Compute fingerprint from geology configuration.
    #[must_use]
    pub fn from_config(
        layer_count: usize,
        fault_count: usize,
        magma_count: usize,
        crystal_count: usize,
        max_depth: f32,
    ) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        #[allow(clippy::cast_possible_truncation)]
        let layers = layer_count as u32;
        hasher.update(&layers.to_le_bytes());
        #[allow(clippy::cast_possible_truncation)]
        let faults = fault_count as u32;
        hasher.update(&faults.to_le_bytes());
        #[allow(clippy::cast_possible_truncation)]
        let magmas = magma_count as u32;
        hasher.update(&magmas.to_le_bytes());
        #[allow(clippy::cast_possible_truncation)]
        let crystals = crystal_count as u32;
        hasher.update(&crystals.to_le_bytes());
        hasher.update(&max_depth.to_le_bytes());
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

/// Checksum of geological simulation state for synchronization.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GeologyChecksum {
    /// Checksum of layer state.
    pub layers: u32,

    /// Checksum of fault state.
    pub faults: u32,

    /// Checksum of magma state.
    pub magma: u32,

    /// Checksum of crystal state.
    pub crystals: u32,

    /// Checksum of field state.
    pub fields: u32,

    /// Tick when checksum was computed.
    pub tick: u64,
}

impl GeologyChecksum {
    /// Create a new checksum at the given tick.
    #[must_use]
    pub const fn new(tick: u64) -> Self {
        Self {
            layers: 0,
            faults: 0,
            magma: 0,
            crystals: 0,
            fields: 0,
            tick,
        }
    }

    /// Set layers checksum.
    #[must_use]
    pub const fn with_layers(mut self, checksum: u32) -> Self {
        self.layers = checksum;
        self
    }

    /// Set faults checksum.
    #[must_use]
    pub const fn with_faults(mut self, checksum: u32) -> Self {
        self.faults = checksum;
        self
    }

    /// Set magma checksum.
    #[must_use]
    pub const fn with_magma(mut self, checksum: u32) -> Self {
        self.magma = checksum;
        self
    }

    /// Set crystals checksum.
    #[must_use]
    pub const fn with_crystals(mut self, checksum: u32) -> Self {
        self.crystals = checksum;
        self
    }

    /// Set fields checksum.
    #[must_use]
    pub const fn with_fields(mut self, checksum: u32) -> Self {
        self.fields = checksum;
        self
    }

    /// Compute combined checksum value.
    #[must_use]
    pub fn combined(&self) -> u64 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.tick.to_le_bytes());
        hasher.update(&self.layers.to_le_bytes());
        hasher.update(&self.faults.to_le_bytes());
        hasher.update(&self.magma.to_le_bytes());
        hasher.update(&self.crystals.to_le_bytes());
        hasher.update(&self.fields.to_le_bytes());
        u64::from(hasher.finalize())
    }

    /// Check if this checksum matches another.
    #[must_use]
    pub fn matches(&self, other: &Self) -> bool {
        self.layers == other.layers
            && self.faults == other.faults
            && self.magma == other.magma
            && self.crystals == other.crystals
            && self.fields == other.fields
    }

    /// Get mismatched component names.
    #[must_use]
    pub fn mismatches(&self, other: &Self) -> Vec<&'static str> {
        let mut result = Vec::new();
        if self.layers != other.layers {
            result.push("layers");
        }
        if self.faults != other.faults {
            result.push("faults");
        }
        if self.magma != other.magma {
            result.push("magma");
        }
        if self.crystals != other.crystals {
            result.push("crystals");
        }
        if self.fields != other.fields {
            result.push("fields");
        }
        result
    }
}

/// Builder for computing geology state checksums.
#[derive(Clone, Debug, Default)]
pub struct FingerprintBuilder {
    layers: crc32fast::Hasher,
    faults: crc32fast::Hasher,
    magma: crc32fast::Hasher,
    crystals: crc32fast::Hasher,
    fields: crc32fast::Hasher,
}

impl FingerprintBuilder {
    /// Create a new builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a layer fingerprint.
    pub fn add_layer(&mut self, layer_id: u32, fingerprint: u32) -> &mut Self {
        self.layers.update(&layer_id.to_le_bytes());
        self.layers.update(&fingerprint.to_le_bytes());
        self
    }

    /// Add a fault fingerprint.
    pub fn add_fault(&mut self, fault_id: u64, fingerprint: u32) -> &mut Self {
        self.faults.update(&fault_id.to_le_bytes());
        self.faults.update(&fingerprint.to_le_bytes());
        self
    }

    /// Add a magma pocket fingerprint.
    pub fn add_magma_pocket(&mut self, pocket_id: u64, fingerprint: u32) -> &mut Self {
        self.magma.update(&pocket_id.to_le_bytes());
        self.magma.update(&fingerprint.to_le_bytes());
        self
    }

    /// Add a magma flow fingerprint.
    pub fn add_magma_flow(&mut self, flow_id: u64, fingerprint: u32) -> &mut Self {
        self.magma.update(&flow_id.to_le_bytes());
        self.magma.update(&fingerprint.to_le_bytes());
        self
    }

    /// Add a crystal seam fingerprint.
    pub fn add_crystal_seam(&mut self, seam_id: u64, fingerprint: u32) -> &mut Self {
        self.crystals.update(&seam_id.to_le_bytes());
        self.crystals.update(&fingerprint.to_le_bytes());
        self
    }

    /// Add a mineral deposit fingerprint.
    pub fn add_mineral_deposit(&mut self, deposit_id: u64, fingerprint: u32) -> &mut Self {
        self.crystals.update(&deposit_id.to_le_bytes());
        self.crystals.update(&fingerprint.to_le_bytes());
        self
    }

    /// Add field state fingerprint.
    pub fn add_field(&mut self, position: (f32, f32, f32), fingerprint: u32) -> &mut Self {
        self.fields.update(&position.0.to_le_bytes());
        self.fields.update(&position.1.to_le_bytes());
        self.fields.update(&position.2.to_le_bytes());
        self.fields.update(&fingerprint.to_le_bytes());
        self
    }

    /// Build the checksum.
    #[must_use]
    pub fn build(self, tick: u64) -> GeologyChecksum {
        GeologyChecksum {
            layers: self.layers.finalize(),
            faults: self.faults.finalize(),
            magma: self.magma.finalize(),
            crystals: self.crystals.finalize(),
            fields: self.fields.finalize(),
            tick,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_deterministic() {
        let f1 = GeologyFingerprint::from_config(3, 2, 4, 5, 500.0);
        let f2 = GeologyFingerprint::from_config(3, 2, 4, 5, 500.0);
        assert_eq!(f1, f2);
    }

    #[test]
    fn fingerprint_differs_on_change() {
        let f1 = GeologyFingerprint::from_config(3, 2, 4, 5, 500.0);
        let f2 = GeologyFingerprint::from_config(4, 2, 4, 5, 500.0);
        assert_ne!(f1, f2);

        let f3 = GeologyFingerprint::from_config(3, 2, 4, 5, 600.0);
        assert_ne!(f1, f3);
    }

    #[test]
    fn fingerprint_combine() {
        let f1 = GeologyFingerprint::new(100);
        let f2 = GeologyFingerprint::new(200);
        let combined = f1.combine(&f2);
        assert_ne!(combined, f1);
        assert_ne!(combined, f2);
    }

    #[test]
    fn checksum_matches() {
        let c1 = GeologyChecksum::new(100)
            .with_layers(1)
            .with_faults(2)
            .with_magma(3)
            .with_crystals(4)
            .with_fields(5);

        let c2 = GeologyChecksum::new(100)
            .with_layers(1)
            .with_faults(2)
            .with_magma(3)
            .with_crystals(4)
            .with_fields(5);

        assert!(c1.matches(&c2));
    }

    #[test]
    fn checksum_mismatches() {
        let c1 = GeologyChecksum::new(100).with_layers(1).with_faults(2);

        let c2 = GeologyChecksum::new(100).with_layers(1).with_faults(3);

        assert!(!c1.matches(&c2));
        assert_eq!(c1.mismatches(&c2), vec!["faults"]);
    }

    #[test]
    fn checksum_combined_deterministic() {
        let c1 = GeologyChecksum::new(100).with_layers(1);
        let c2 = GeologyChecksum::new(100).with_layers(1);
        assert_eq!(c1.combined(), c2.combined());
    }

    #[test]
    fn builder_deterministic() {
        let mut b1 = FingerprintBuilder::new();
        b1.add_layer(1, 100);
        b1.add_fault(2, 200);
        b1.add_magma_pocket(3, 300);
        let c1 = b1.build(100);

        let mut b2 = FingerprintBuilder::new();
        b2.add_layer(1, 100);
        b2.add_fault(2, 200);
        b2.add_magma_pocket(3, 300);
        let c2 = b2.build(100);

        assert!(c1.matches(&c2));
    }

    #[test]
    fn serde_fingerprint() {
        let fp = GeologyFingerprint::new(12345);
        let json = serde_json::to_string(&fp).unwrap();
        let recovered: GeologyFingerprint = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, fp);
    }

    #[test]
    fn serde_checksum() {
        let cs = GeologyChecksum::new(100).with_layers(1).with_faults(2);
        let json = serde_json::to_string(&cs).unwrap();
        let recovered: GeologyChecksum = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, cs);
    }
}
