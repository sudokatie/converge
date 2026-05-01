//! Mod sandboxing and compatibility reporting layer.
//!
//! Provides a data-driven system for modeling sandbox permissions, capabilities,
//! and resource budgets for mods and game packs. Includes deterministic compatibility
//! reporting for version checking, dependency resolution, conflict detection,
//! load ordering, and policy validation.
//!
//! # Features
//!
//! - **Capabilities**: Fine-grained permission model for filesystem, network, native code,
//!   world mutation, content registration, scripting, and system access.
//! - **Resource Budgets**: Limits on memory, CPU time, storage, and network usage.
//! - **Policies**: Configurable rules for allowing/denying capabilities with category defaults.
//! - **Mod Descriptors**: Complete metadata including version ranges, dependencies, conflicts,
//!   load order constraints, and content integration references.
//! - **Compatibility Checking**: Deterministic analysis producing load orders and fingerprints.
//!
//! # Example
//!
//! ```
//! use engine_core::mod_sandbox::{
//!     CapabilityRequirements, CapabilityRule, CompatibilityChecker, ModDescriptor,
//!     ModId, PolicyId, SandboxCapability, SandboxPolicy,
//! };
//! use engine_core::game_pack::PackVersion;
//!
//! // Create a mod descriptor
//! let my_mod = ModDescriptor::new(
//!     ModId::new(1, 1),
//!     "my_mod",
//!     PackVersion::new(1, 0, 0),
//! )
//! .with_capabilities(
//!     CapabilityRequirements::new()
//!         .with_required(SandboxCapability::ReadOwnData)
//!         .with_required(SandboxCapability::MutateBlocks)
//! );
//!
//! // Create a sandbox policy
//! let policy = SandboxPolicy::permissive(
//!     PolicyId::new(1, 1),
//!     "default",
//! )
//! .with_rule(CapabilityRule::allow(SandboxCapability::MutateBlocks));
//!
//! // Check compatibility
//! let mut checker = CompatibilityChecker::new()
//!     .with_policy(policy)
//!     .with_engine_version(PackVersion::new(0, 2, 0));
//! checker.add_mod(my_mod);
//!
//! let report = checker.check();
//! assert!(report.compatible);
//! ```

mod budget;
mod capability;
mod compatibility;
mod descriptor;
mod error;
mod fingerprint;
mod id;
mod policy;

pub use budget::{
    BudgetValidation, CpuBudget, MemoryBudget, NetworkBudget, ResourceBudget, StorageBudget,
};
pub use capability::{CapabilityCategory, CapabilityRequirements, SandboxCapability};
pub use compatibility::{
    CompatibilityChecker, CompatibilityIssue, IssueCategory, IssueSeverity, LoadOrderEntry,
    LoadStatus, ModCompatibilityReport,
};
pub use descriptor::{
    ApiVersionRange, ContentHookRef, GamePackRef, LoadOrderConstraint, ModConflict, ModDescriptor,
};
pub use error::{ModSandboxError, ModSandboxResult};
pub use fingerprint::{SandboxFingerprint, SandboxFingerprintBuilder};
pub use id::{ModId, PolicyId};
pub use policy::{CapabilityDecision, CapabilityRule, PolicyValidation, SandboxPolicy};
