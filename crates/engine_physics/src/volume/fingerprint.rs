//! Stable fingerprints and checksums for physics volumes.

use serde::{Deserialize, Serialize};

use super::{PhysicsLaws, PhysicsVolume, VolumeConfig, VolumeShape};

/// A stable fingerprint for a physics volume configuration.
///
/// Used for deterministic identification, change detection, and network sync.
/// The fingerprint is computed from the volume's configuration and does not
/// include mutable state like enabled/disabled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VolumeFingerprint {
    /// High bits of the fingerprint.
    high: u64,
    /// Low bits of the fingerprint.
    low: u64,
}

impl VolumeFingerprint {
    /// Creates a fingerprint from raw parts.
    #[must_use]
    pub const fn from_raw(high: u64, low: u64) -> Self {
        Self { high, low }
    }

    /// Creates a fingerprint from a physics volume.
    #[must_use]
    pub fn from_volume(volume: &PhysicsVolume) -> Self {
        let mut hasher = FnvHasher::new();

        hasher.write_u64(volume.id().raw());
        hash_shape(&mut hasher, volume.shape());
        hash_laws(&mut hasher, volume.laws());
        hash_config(&mut hasher, volume.config());
        hasher.write_u32(volume.tag());

        let high = hasher.finish();

        hasher.write_u64(0xDEAD_BEEF);
        let low = hasher.finish();

        Self { high, low }
    }

    /// Returns the high bits.
    #[must_use]
    pub const fn high(&self) -> u64 {
        self.high
    }

    /// Returns the low bits.
    #[must_use]
    pub const fn low(&self) -> u64 {
        self.low
    }

    /// Returns a compact 64-bit checksum (XOR of high and low).
    #[must_use]
    pub const fn checksum(&self) -> u64 {
        self.high ^ self.low
    }

    /// Returns the fingerprint as a 128-bit value.
    #[must_use]
    pub const fn as_u128(&self) -> u128 {
        ((self.high as u128) << 64) | (self.low as u128)
    }
}

fn hash_shape(hasher: &mut FnvHasher, shape: &VolumeShape) {
    match shape {
        VolumeShape::Aabb(aabb) => {
            hasher.write_u8(0);
            hash_f32(hasher, aabb.min.x);
            hash_f32(hasher, aabb.min.y);
            hash_f32(hasher, aabb.min.z);
            hash_f32(hasher, aabb.max.x);
            hash_f32(hasher, aabb.max.y);
            hash_f32(hasher, aabb.max.z);
        }
        VolumeShape::Sphere(sphere) => {
            hasher.write_u8(1);
            hash_f32(hasher, sphere.center.x);
            hash_f32(hasher, sphere.center.y);
            hash_f32(hasher, sphere.center.z);
            hash_f32(hasher, sphere.radius);
        }
    }
}

fn hash_laws(hasher: &mut FnvHasher, laws: &PhysicsLaws) {
    hash_opt_vec3(hasher, laws.gravity);
    hash_opt_f32(hasher, laws.drag);
    hash_opt_f32(hasher, laws.angular_damping);
    hash_opt_f32(hasher, laws.buoyancy);
    hash_opt_f32(hasher, laws.terminal_velocity);
    hash_opt_f32(hasher, laws.friction);
    hash_opt_f32(hasher, laws.time_scale);
    hash_opt_f32(hasher, laws.restitution);
}

fn hash_config(hasher: &mut FnvHasher, config: &VolumeConfig) {
    hasher.write_i32(config.priority);
    hasher.write_u8(config.blend_mode as u8);
    hasher.write_u8(config.overlap_resolution as u8);
    hasher.write_u32(config.layer_mask);
}

fn hash_f32(hasher: &mut FnvHasher, value: f32) {
    hasher.write_u32(value.to_bits());
}

fn hash_opt_f32(hasher: &mut FnvHasher, value: Option<f32>) {
    match value {
        Some(v) => {
            hasher.write_u8(1);
            hash_f32(hasher, v);
        }
        None => hasher.write_u8(0),
    }
}

fn hash_opt_vec3(hasher: &mut FnvHasher, value: Option<glam::Vec3>) {
    match value {
        Some(v) => {
            hasher.write_u8(1);
            hash_f32(hasher, v.x);
            hash_f32(hasher, v.y);
            hash_f32(hasher, v.z);
        }
        None => hasher.write_u8(0),
    }
}

/// FNV-1a hasher for deterministic fingerprinting.
struct FnvHasher {
    state: u64,
}

impl FnvHasher {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0100_0000_01b3;

    const fn new() -> Self {
        Self {
            state: Self::FNV_OFFSET,
        }
    }

    fn write_u8(&mut self, byte: u8) {
        self.state ^= u64::from(byte);
        self.state = self.state.wrapping_mul(Self::FNV_PRIME);
    }

    fn write_u32(&mut self, value: u32) {
        self.write_u8((value & 0xFF) as u8);
        self.write_u8(((value >> 8) & 0xFF) as u8);
        self.write_u8(((value >> 16) & 0xFF) as u8);
        self.write_u8(((value >> 24) & 0xFF) as u8);
    }

    fn write_i32(&mut self, value: i32) {
        self.write_u32(value.cast_unsigned());
    }

    fn write_u64(&mut self, value: u64) {
        self.write_u32((value & 0xFFFF_FFFF) as u32);
        self.write_u32((value >> 32) as u32);
    }

    const fn finish(&self) -> u64 {
        self.state
    }
}

/// Computes a registry checksum from multiple volume fingerprints.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn registry_checksum(volumes: &[VolumeFingerprint]) -> u64 {
    let mut combined: u64 = 0;
    for (i, fp) in volumes.iter().enumerate() {
        combined ^= fp.high.rotate_left((i % 64) as u32);
        combined ^= fp.low.rotate_right((i % 64) as u32);
    }
    combined
}

/// Computes a sorted registry checksum (order-independent).
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn registry_checksum_sorted(volumes: &[VolumeFingerprint]) -> u64 {
    let mut sorted: Vec<_> = volumes.iter().map(VolumeFingerprint::as_u128).collect();
    sorted.sort_unstable();

    let mut combined: u64 = 0;
    for (i, v) in sorted.iter().enumerate() {
        combined ^= (*v as u64).rotate_left((i % 64) as u32);
        combined ^= ((*v >> 64) as u64).rotate_right((i % 64) as u32);
    }
    combined
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::volume::VolumeId;
    use engine_core::math::Aabb;
    use glam::Vec3;

    fn test_volume(id: u64) -> PhysicsVolume {
        PhysicsVolume::new(
            VolumeId::new(id),
            VolumeShape::aabb(Aabb::from_center_half_extents(Vec3::ZERO, Vec3::ONE)),
            PhysicsLaws::low_gravity(),
        )
    }

    #[test]
    fn fingerprint_deterministic() {
        let volume = test_volume(1);
        let fp1 = VolumeFingerprint::from_volume(&volume);
        let fp2 = VolumeFingerprint::from_volume(&volume);
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn different_volumes_different_fingerprints() {
        let vol1 = test_volume(1);
        let vol2 = test_volume(2);
        let fp1 = VolumeFingerprint::from_volume(&vol1);
        let fp2 = VolumeFingerprint::from_volume(&vol2);
        assert_ne!(fp1, fp2);
    }

    #[test]
    fn fingerprint_changes_with_laws() {
        let vol1 = PhysicsVolume::new(
            VolumeId::new(1),
            VolumeShape::default(),
            PhysicsLaws::low_gravity(),
        );
        let vol2 = PhysicsVolume::new(
            VolumeId::new(1),
            VolumeShape::default(),
            PhysicsLaws::underwater(),
        );

        let fp1 = VolumeFingerprint::from_volume(&vol1);
        let fp2 = VolumeFingerprint::from_volume(&vol2);
        assert_ne!(fp1, fp2);
    }

    #[test]
    fn fingerprint_changes_with_shape() {
        let vol1 = PhysicsVolume::new(
            VolumeId::new(1),
            VolumeShape::sphere_centered(Vec3::ZERO, 1.0),
            PhysicsLaws::default(),
        );
        let vol2 = PhysicsVolume::new(
            VolumeId::new(1),
            VolumeShape::sphere_centered(Vec3::ZERO, 2.0),
            PhysicsLaws::default(),
        );

        let fp1 = VolumeFingerprint::from_volume(&vol1);
        let fp2 = VolumeFingerprint::from_volume(&vol2);
        assert_ne!(fp1, fp2);
    }

    #[test]
    fn checksum_xor() {
        let fp = VolumeFingerprint::from_raw(0xABCD, 0x1234);
        assert_eq!(fp.checksum(), 0xABCD ^ 0x1234);
    }

    #[test]
    fn as_u128() {
        let fp = VolumeFingerprint::from_raw(0x1234_5678_9ABC_DEF0, 0xFEDC_BA98_7654_3210);
        let combined = fp.as_u128();
        assert_eq!(combined >> 64, 0x1234_5678_9ABC_DEF0);
        assert_eq!(combined & 0xFFFF_FFFF_FFFF_FFFF, 0xFEDC_BA98_7654_3210);
    }

    #[test]
    fn registry_checksum_empty() {
        let checksum = registry_checksum(&[]);
        assert_eq!(checksum, 0);
    }

    #[test]
    fn registry_checksum_single() {
        let fp = VolumeFingerprint::from_volume(&test_volume(1));
        let checksum = registry_checksum(&[fp]);
        assert_ne!(checksum, 0);
    }

    #[test]
    fn registry_checksum_order_matters() {
        let fp1 = VolumeFingerprint::from_volume(&test_volume(1));
        let fp2 = VolumeFingerprint::from_volume(&test_volume(2));

        let checksum1 = registry_checksum(&[fp1, fp2]);
        let checksum2 = registry_checksum(&[fp2, fp1]);
        assert_ne!(checksum1, checksum2);
    }

    #[test]
    fn sorted_checksum_order_independent() {
        let fp1 = VolumeFingerprint::from_volume(&test_volume(1));
        let fp2 = VolumeFingerprint::from_volume(&test_volume(2));

        let checksum1 = registry_checksum_sorted(&[fp1, fp2]);
        let checksum2 = registry_checksum_sorted(&[fp2, fp1]);
        assert_eq!(checksum1, checksum2);
    }

    #[test]
    fn fingerprint_serialization() {
        let fp = VolumeFingerprint::from_volume(&test_volume(42));
        let json = serde_json::to_string(&fp).unwrap();
        let recovered: VolumeFingerprint = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, fp);
    }
}
