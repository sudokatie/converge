//! Local physics volumes with custom laws per region.
//!
//! Provides spatial volumes that override physics parameters for entities within
//! their bounds. Supports gravity, drag, damping, buoyancy, terminal velocity,
//! friction, time scale, and collision/material modifiers.
//!
//! # Architecture
//!
//! - [`VolumeId`]: Unique identifier for physics volumes
//! - [`VolumeShape`]: Spatial bounds (AABB, Sphere)
//! - [`PhysicsLaws`]: Custom physics parameters per volume
//! - [`VolumeConfig`]: Priority, blending, and overlap resolution
//! - [`PhysicsVolume`]: Complete volume definition
//! - [`VolumeRegistry`]: Manages volumes and samples entities
//! - [`VolumeEvent`]: Entry/exit events for game logic
//!
//! # Example
//!
//! ```ignore
//! use engine_physics::volume::*;
//! use engine_core::math::Aabb;
//! use glam::Vec3;
//!
//! // Create a low-gravity zone
//! let laws = PhysicsLaws::default()
//!     .with_gravity(Vec3::new(0.0, -2.0, 0.0))
//!     .with_drag(0.1);
//!
//! let volume = PhysicsVolume::new(
//!     VolumeId::new(1),
//!     VolumeShape::aabb(Aabb::from_center_half_extents(
//!         Vec3::new(0.0, 10.0, 0.0),
//!         Vec3::splat(5.0),
//!     )),
//!     laws,
//! );
//!
//! // Create underwater zone with buoyancy
//! let water_laws = PhysicsLaws::default()
//!     .with_gravity(Vec3::new(0.0, -4.0, 0.0))
//!     .with_drag(2.0)
//!     .with_buoyancy(0.8)
//!     .with_time_scale(0.9);
//!
//! let water = PhysicsVolume::new(
//!     VolumeId::new(2),
//!     VolumeShape::aabb(Aabb::new(Vec3::ZERO, Vec3::new(100.0, 0.0, 100.0))),
//!     water_laws,
//! ).with_config(VolumeConfig::default().with_priority(10));
//! ```

mod config;
mod definition;
mod event;
mod fingerprint;
mod laws;
mod modifier;
mod registry;
mod sample;
mod shape;

pub use config::{BlendMode, OverlapResolution, VolumeConfig};
pub use definition::PhysicsVolume;
pub use event::{EntryEvent, ExitEvent, VolumeEvent, VolumeEvents};
pub use fingerprint::{VolumeFingerprint, registry_checksum, registry_checksum_sorted};
pub use laws::PhysicsLaws;
pub use modifier::{CollisionModifier, MaterialModifier};
pub use registry::{SampleResult, VolumeRegistry};
pub use sample::{EntitySample, SampledLaws};
pub use shape::VolumeShape;

use serde::{Deserialize, Serialize};

/// Unique identifier for a physics volume.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VolumeId(u64);

impl VolumeId {
    /// Creates a new volume identifier.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Returns the raw identifier value.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

impl From<u64> for VolumeId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl From<VolumeId> for u64 {
    fn from(id: VolumeId) -> Self {
        id.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_id_roundtrip() {
        let id = VolumeId::new(42);
        assert_eq!(id.raw(), 42);
        assert_eq!(u64::from(id), 42);
        assert_eq!(VolumeId::from(42u64), id);
    }

    #[test]
    fn volume_id_serialization() {
        let id = VolumeId::new(123);
        let json = serde_json::to_string(&id).unwrap();
        let recovered: VolumeId = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, id);
    }

    #[test]
    fn volume_id_bincode() {
        let id = VolumeId::new(456);
        let bytes = bincode::serialize(&id).unwrap();
        let recovered: VolumeId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(recovered, id);
    }
}
