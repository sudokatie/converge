//! Game pack plugin layer for registering custom game content.
//!
//! Provides a deterministic registry for game packs that can contribute:
//! - Custom block types
//! - System hooks (declarative)
//! - Hazard definitions
//! - Shader references
//! - World rule profiles
//!
//! Game packs declare dependencies and capabilities, enabling conflict detection
//! and activation planning without dynamic loading.

mod activation;
mod descriptor;
mod error;
mod fingerprint;
mod id;
mod manifest;
mod registry;

pub use activation::{ActivationPlan, ActivationStatus, DependencyResolver};
pub use descriptor::{
    BlockDescriptor, HazardDescriptor, RuleProfileDescriptor, ShaderDescriptor, SystemDescriptor,
    SystemPhase,
};
pub use error::{GamePackError, GamePackResult};
pub use fingerprint::PackFingerprint;
pub use id::{BlockId, HazardId, PackId, RuleProfileId, ShaderId, SystemId};
pub use manifest::{Capability, PackDependency, PackManifest, PackVersion};
pub use registry::{CompatibilityReport, GamePackRegistry, PackQuery, RegisteredPack};
