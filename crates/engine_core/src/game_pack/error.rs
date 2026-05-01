//! Error types for game pack operations.

use thiserror::Error;

use super::id::PackId;

/// Result type for game pack operations.
pub type GamePackResult<T> = Result<T, GamePackError>;

/// Errors that can occur during game pack operations.
#[derive(Debug, Error)]
pub enum GamePackError {
    #[error("duplicate pack ID: {0}")]
    DuplicatePackId(PackId),

    #[error("duplicate pack name: {0}")]
    DuplicatePackName(String),

    #[error("duplicate block ID in pack {pack}: {block}")]
    DuplicateBlockId { pack: PackId, block: String },

    #[error("duplicate system ID in pack {pack}: {system}")]
    DuplicateSystemId { pack: PackId, system: String },

    #[error("duplicate hazard ID in pack {pack}: {hazard}")]
    DuplicateHazardId { pack: PackId, hazard: String },

    #[error("duplicate shader ID in pack {pack}: {shader}")]
    DuplicateShaderId { pack: PackId, shader: String },

    #[error("duplicate rule profile ID in pack {pack}: {profile}")]
    DuplicateRuleProfileId { pack: PackId, profile: String },

    #[error("pack not found: {0}")]
    PackNotFound(PackId),

    #[error("missing dependency: pack {dependent} requires {dependency}")]
    MissingDependency {
        dependent: PackId,
        dependency: String,
    },

    #[error("dependency cycle detected involving pack: {0}")]
    DependencyCycle(PackId),

    #[error("version incompatible: {pack} requires {required}, found {found}")]
    VersionIncompatible {
        pack: PackId,
        required: String,
        found: String,
    },

    #[error("capability conflict: {0}")]
    CapabilityConflict(String),

    #[error("serialization error: {0}")]
    Serialization(String),
}

impl From<bincode::Error> for GamePackError {
    fn from(err: bincode::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}
