//! Volume configuration for priority and overlap handling.

use serde::{Deserialize, Serialize};

/// Configuration for volume behavior.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct VolumeConfig {
    /// Priority for overlap resolution (higher wins).
    pub priority: i32,
    /// How to blend with other overlapping volumes.
    pub blend_mode: BlendMode,
    /// How to resolve overlaps with same-priority volumes.
    pub overlap_resolution: OverlapResolution,
    /// Whether this volume is currently active.
    pub enabled: bool,
    /// Optional layer mask for selective application.
    pub layer_mask: u32,
}

impl Default for VolumeConfig {
    fn default() -> Self {
        Self {
            priority: 0,
            blend_mode: BlendMode::Replace,
            overlap_resolution: OverlapResolution::First,
            enabled: true,
            layer_mask: u32::MAX,
        }
    }
}

impl VolumeConfig {
    /// Creates a new volume config with default settings.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            priority: 0,
            blend_mode: BlendMode::Replace,
            overlap_resolution: OverlapResolution::First,
            enabled: true,
            layer_mask: u32::MAX,
        }
    }

    /// Builder: sets priority.
    #[must_use]
    pub const fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Builder: sets blend mode.
    #[must_use]
    pub const fn with_blend_mode(mut self, mode: BlendMode) -> Self {
        self.blend_mode = mode;
        self
    }

    /// Builder: sets overlap resolution.
    #[must_use]
    pub const fn with_overlap_resolution(mut self, resolution: OverlapResolution) -> Self {
        self.overlap_resolution = resolution;
        self
    }

    /// Builder: sets enabled state.
    #[must_use]
    pub const fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Builder: sets layer mask.
    #[must_use]
    pub const fn with_layer_mask(mut self, mask: u32) -> Self {
        self.layer_mask = mask;
        self
    }

    /// Returns whether this volume applies to the given layer.
    #[must_use]
    pub const fn applies_to_layer(&self, layer: u32) -> bool {
        self.layer_mask & (1 << layer) != 0
    }
}

/// How volume physics laws blend with overlapping volumes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlendMode {
    /// Replace lower-priority volume laws entirely.
    #[default]
    Replace,
    /// Blend laws by penetration depth ratio.
    Blend,
    /// Add laws on top of existing (cumulative).
    Additive,
    /// Multiply law values together.
    Multiply,
}

/// How to resolve volumes with equal priority.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OverlapResolution {
    /// First volume in registration order wins.
    #[default]
    First,
    /// Last volume in registration order wins.
    Last,
    /// Blend all equal-priority volumes.
    BlendAll,
    /// Volume with deepest penetration wins.
    Deepest,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = VolumeConfig::default();
        assert_eq!(config.priority, 0);
        assert!(config.enabled);
        assert_eq!(config.blend_mode, BlendMode::Replace);
    }

    #[test]
    fn builder_chain() {
        let config = VolumeConfig::new()
            .with_priority(10)
            .with_blend_mode(BlendMode::Blend)
            .with_overlap_resolution(OverlapResolution::Deepest)
            .with_enabled(false);

        assert_eq!(config.priority, 10);
        assert_eq!(config.blend_mode, BlendMode::Blend);
        assert_eq!(config.overlap_resolution, OverlapResolution::Deepest);
        assert!(!config.enabled);
    }

    #[test]
    fn layer_mask() {
        let config = VolumeConfig::default().with_layer_mask(0b1010);
        assert!(!config.applies_to_layer(0));
        assert!(config.applies_to_layer(1));
        assert!(!config.applies_to_layer(2));
        assert!(config.applies_to_layer(3));
    }

    #[test]
    fn serialization() {
        let config = VolumeConfig::new()
            .with_priority(5)
            .with_blend_mode(BlendMode::Additive);
        let json = serde_json::to_string(&config).unwrap();
        let recovered: VolumeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.priority, 5);
        assert_eq!(recovered.blend_mode, BlendMode::Additive);
    }
}
