//! Error types for mod sandbox operations.

use thiserror::Error;

use super::id::ModId;

/// Result type for mod sandbox operations.
pub type ModSandboxResult<T> = Result<T, ModSandboxError>;

/// Errors that can occur during mod sandbox operations.
#[derive(Debug, Error)]
pub enum ModSandboxError {
    #[error("duplicate mod ID: {0}")]
    DuplicateModId(ModId),

    #[error("duplicate mod name: {0}")]
    DuplicateModName(String),

    #[error("mod not found: {0}")]
    ModNotFound(ModId),

    #[error("mod not found by name: {0}")]
    ModNotFoundByName(String),

    #[error("missing dependency: mod {dependent} requires {dependency}")]
    MissingDependency {
        dependent: ModId,
        dependency: String,
    },

    #[error("dependency cycle detected involving mod: {0}")]
    DependencyCycle(ModId),

    #[error("version incompatible: {mod_name} requires {required}, found {found}")]
    VersionIncompatible {
        mod_name: String,
        required: String,
        found: String,
    },

    #[error("capability denied: {mod_name} requires {capability}")]
    CapabilityDenied {
        mod_name: String,
        capability: String,
    },

    #[error("budget exceeded: {mod_name} - {violation}")]
    BudgetExceeded { mod_name: String, violation: String },

    #[error("policy not found: {0}")]
    PolicyNotFound(String),

    #[error("conflict detected: {mod_a} conflicts with {mod_b}")]
    ConflictDetected { mod_a: String, mod_b: String },

    #[error("load order cycle: {0}")]
    LoadOrderCycle(String),

    #[error("serialization error: {0}")]
    Serialization(String),
}

impl From<bincode::Error> for ModSandboxError {
    fn from(err: bincode::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = ModSandboxError::DuplicateModId(ModId::new(1, 1));
        assert!(err.to_string().contains("duplicate mod ID"));

        let err = ModSandboxError::MissingDependency {
            dependent: ModId::new(1, 2),
            dependency: "base".to_string(),
        };
        assert!(err.to_string().contains("base"));

        let err = ModSandboxError::CapabilityDenied {
            mod_name: "test_mod".to_string(),
            capability: "NetworkInternet".to_string(),
        };
        assert!(err.to_string().contains("NetworkInternet"));
    }

    #[test]
    fn error_from_bincode() {
        let bincode_err: bincode::Error =
            bincode::deserialize::<String>(&[0xFF, 0xFF, 0xFF, 0xFF]).unwrap_err();
        let sandbox_err: ModSandboxError = bincode_err.into();
        assert!(matches!(sandbox_err, ModSandboxError::Serialization(_)));
    }
}
