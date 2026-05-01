//! Sandbox capability definitions for mod permissions.
//!
//! Capabilities represent specific permissions that mods can request and
//! sandbox policies can grant or deny.

use serde::{Deserialize, Serialize};

/// Categories of sandbox capabilities.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CapabilityCategory {
    Filesystem,
    Network,
    NativeCode,
    WorldMutation,
    ContentRegistration,
    Scripting,
    SystemAccess,
}

/// Specific sandbox capabilities that mods can request.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SandboxCapability {
    // Filesystem capabilities
    ReadOwnData,
    WriteOwnData,
    ReadSharedData,
    WriteSharedData,
    ReadGameAssets,

    // Network capabilities
    NetworkLocalhost,
    NetworkLan,
    NetworkInternet,
    NetworkWebSocket,

    // Native code capabilities
    NativePlugins,
    NativeFFI,
    NativeThreads,

    // World mutation capabilities
    MutateBlocks,
    MutateEntities,
    MutateTerrain,
    MutateWeather,
    MutateTime,

    // Content registration capabilities
    RegisterBlocks,
    RegisterEntities,
    RegisterItems,
    RegisterRecipes,
    RegisterBiomes,
    RegisterDimensions,

    // Scripting capabilities
    ScriptExecution,
    ScriptTimers,
    ScriptEvents,
    ScriptHooks,

    // System access capabilities
    SystemInfo,
    SystemClipboard,
    SystemNotifications,

    // Custom capability with identifier
    Custom(String),
}

impl SandboxCapability {
    /// Get the category this capability belongs to.
    #[must_use]
    pub fn category(&self) -> CapabilityCategory {
        match self {
            Self::ReadOwnData
            | Self::WriteOwnData
            | Self::ReadSharedData
            | Self::WriteSharedData
            | Self::ReadGameAssets => CapabilityCategory::Filesystem,

            Self::NetworkLocalhost
            | Self::NetworkLan
            | Self::NetworkInternet
            | Self::NetworkWebSocket => CapabilityCategory::Network,

            Self::NativePlugins | Self::NativeFFI | Self::NativeThreads => {
                CapabilityCategory::NativeCode
            }

            Self::MutateBlocks
            | Self::MutateEntities
            | Self::MutateTerrain
            | Self::MutateWeather
            | Self::MutateTime => CapabilityCategory::WorldMutation,

            Self::RegisterBlocks
            | Self::RegisterEntities
            | Self::RegisterItems
            | Self::RegisterRecipes
            | Self::RegisterBiomes
            | Self::RegisterDimensions => CapabilityCategory::ContentRegistration,

            Self::ScriptExecution | Self::ScriptTimers | Self::ScriptEvents | Self::ScriptHooks => {
                CapabilityCategory::Scripting
            }

            Self::SystemInfo | Self::SystemClipboard | Self::SystemNotifications => {
                CapabilityCategory::SystemAccess
            }

            Self::Custom(_) => CapabilityCategory::SystemAccess,
        }
    }

    /// Check if this capability is considered high-risk.
    #[must_use]
    pub fn is_high_risk(&self) -> bool {
        matches!(
            self,
            Self::NetworkInternet
                | Self::NativePlugins
                | Self::NativeFFI
                | Self::NativeThreads
                | Self::WriteSharedData
                | Self::SystemClipboard
        )
    }

    /// Get a human-readable description of this capability.
    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            Self::ReadOwnData => "Read mod's own data directory",
            Self::WriteOwnData => "Write to mod's own data directory",
            Self::ReadSharedData => "Read shared game data",
            Self::WriteSharedData => "Write to shared game data",
            Self::ReadGameAssets => "Read game asset files",
            Self::NetworkLocalhost => "Network access to localhost only",
            Self::NetworkLan => "Network access within local network",
            Self::NetworkInternet => "Full internet network access",
            Self::NetworkWebSocket => "WebSocket connections",
            Self::NativePlugins => "Load native plugins",
            Self::NativeFFI => "Foreign function interface calls",
            Self::NativeThreads => "Create native threads",
            Self::MutateBlocks => "Modify world blocks",
            Self::MutateEntities => "Modify entities in world",
            Self::MutateTerrain => "Modify terrain generation",
            Self::MutateWeather => "Control weather systems",
            Self::MutateTime => "Control world time",
            Self::RegisterBlocks => "Register custom block types",
            Self::RegisterEntities => "Register custom entity types",
            Self::RegisterItems => "Register custom items",
            Self::RegisterRecipes => "Register crafting recipes",
            Self::RegisterBiomes => "Register custom biomes",
            Self::RegisterDimensions => "Register custom dimensions",
            Self::ScriptExecution => "Execute scripts",
            Self::ScriptTimers => "Create script timers",
            Self::ScriptEvents => "Listen to script events",
            Self::ScriptHooks => "Register script hooks",
            Self::SystemInfo => "Access system information",
            Self::SystemClipboard => "Access system clipboard",
            Self::SystemNotifications => "Show system notifications",
            Self::Custom(_) => "Custom capability",
        }
    }
}

/// A set of required capabilities with optional/required distinction.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityRequirements {
    #[serde(default)]
    pub required: Vec<SandboxCapability>,
    #[serde(default)]
    pub optional: Vec<SandboxCapability>,
}

impl CapabilityRequirements {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_required(mut self, cap: SandboxCapability) -> Self {
        self.required.push(cap);
        self
    }

    #[must_use]
    pub fn with_optional(mut self, cap: SandboxCapability) -> Self {
        self.optional.push(cap);
        self
    }

    /// Check if all requirements are empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.required.is_empty() && self.optional.is_empty()
    }

    /// Get all capabilities (required + optional).
    #[must_use]
    pub fn all(&self) -> Vec<&SandboxCapability> {
        self.required.iter().chain(self.optional.iter()).collect()
    }

    /// Check if any required capabilities are high-risk.
    #[must_use]
    pub fn has_high_risk_required(&self) -> bool {
        self.required.iter().any(SandboxCapability::is_high_risk)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_category() {
        assert_eq!(
            SandboxCapability::ReadOwnData.category(),
            CapabilityCategory::Filesystem
        );
        assert_eq!(
            SandboxCapability::NetworkInternet.category(),
            CapabilityCategory::Network
        );
        assert_eq!(
            SandboxCapability::NativeFFI.category(),
            CapabilityCategory::NativeCode
        );
        assert_eq!(
            SandboxCapability::MutateBlocks.category(),
            CapabilityCategory::WorldMutation
        );
        assert_eq!(
            SandboxCapability::RegisterItems.category(),
            CapabilityCategory::ContentRegistration
        );
        assert_eq!(
            SandboxCapability::ScriptExecution.category(),
            CapabilityCategory::Scripting
        );
        assert_eq!(
            SandboxCapability::SystemInfo.category(),
            CapabilityCategory::SystemAccess
        );
    }

    #[test]
    fn high_risk_capabilities() {
        assert!(SandboxCapability::NetworkInternet.is_high_risk());
        assert!(SandboxCapability::NativeFFI.is_high_risk());
        assert!(SandboxCapability::WriteSharedData.is_high_risk());
        assert!(!SandboxCapability::ReadOwnData.is_high_risk());
        assert!(!SandboxCapability::MutateBlocks.is_high_risk());
    }

    #[test]
    fn capability_requirements_builder() {
        let reqs = CapabilityRequirements::new()
            .with_required(SandboxCapability::ReadOwnData)
            .with_required(SandboxCapability::MutateBlocks)
            .with_optional(SandboxCapability::NetworkLocalhost);

        assert_eq!(reqs.required.len(), 2);
        assert_eq!(reqs.optional.len(), 1);
        assert!(!reqs.is_empty());
        assert_eq!(reqs.all().len(), 3);
    }

    #[test]
    fn capability_requirements_high_risk() {
        let safe = CapabilityRequirements::new()
            .with_required(SandboxCapability::ReadOwnData)
            .with_required(SandboxCapability::MutateBlocks);

        assert!(!safe.has_high_risk_required());

        let risky = CapabilityRequirements::new()
            .with_required(SandboxCapability::NetworkInternet)
            .with_required(SandboxCapability::ReadOwnData);

        assert!(risky.has_high_risk_required());
    }

    #[test]
    fn capability_serde_roundtrip() {
        let cap = SandboxCapability::Custom("my_cap".to_string());
        let json = serde_json::to_string(&cap).unwrap();
        let restored: SandboxCapability = serde_json::from_str(&json).unwrap();
        assert_eq!(cap, restored);
    }

    #[test]
    fn capability_bincode_roundtrip() {
        let reqs = CapabilityRequirements::new()
            .with_required(SandboxCapability::ReadOwnData)
            .with_optional(SandboxCapability::NetworkLocalhost);

        let bytes = bincode::serialize(&reqs).unwrap();
        let restored: CapabilityRequirements = bincode::deserialize(&bytes).unwrap();
        assert_eq!(reqs, restored);
    }
}
