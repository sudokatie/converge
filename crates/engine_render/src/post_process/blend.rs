//! Blend mode, weight, and priority handling for post-processing.
//!
//! Provides mechanisms for combining multiple overlapping post-processing
//! regions with proper weight accumulation and priority resolution.

use glam::Vec3;

/// How to blend overlapping post-processing regions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum PostBlendMode {
    /// Replace lower priority effects entirely.
    Replace = 0,
    /// Weighted average based on blend factors.
    #[default]
    Weighted = 1,
    /// Additive blending (stack effects).
    Additive = 2,
    /// Multiplicative blending.
    Multiply = 3,
    /// Take maximum intensity.
    Max = 4,
    /// Take minimum intensity.
    Min = 5,
}

impl PostBlendMode {
    /// All blend modes.
    pub const ALL: [Self; 6] = [
        Self::Replace,
        Self::Weighted,
        Self::Additive,
        Self::Multiply,
        Self::Max,
        Self::Min,
    ];

    /// Blend two values using this mode.
    #[must_use]
    pub fn blend(self, base: f32, incoming: f32, weight: f32) -> f32 {
        let w = weight.clamp(0.0, 1.0);
        match self {
            Self::Replace => {
                if w > 0.5 {
                    incoming
                } else {
                    base
                }
            }
            Self::Weighted => base * (1.0 - w) + incoming * w,
            Self::Additive => (base + incoming * w).min(1.0),
            Self::Multiply => base * (1.0 - w + incoming * w),
            Self::Max => base.max(incoming * w),
            Self::Min => {
                if w > 0.0 {
                    base.min(incoming)
                } else {
                    base
                }
            }
        }
    }

    /// Blend two colors using this mode.
    #[must_use]
    pub fn blend_color(self, base: Vec3, incoming: Vec3, weight: f32) -> Vec3 {
        Vec3::new(
            self.blend(base.x, incoming.x, weight),
            self.blend(base.y, incoming.y, weight),
            self.blend(base.z, incoming.z, weight),
        )
    }
}

/// Weight contribution from a single post-processing region.
#[derive(Debug, Clone, Copy)]
pub struct RegionWeight {
    /// Region index in the stack.
    pub region_index: u32,
    /// Environment identifier.
    pub environment_id: u32,
    /// Blend weight (0.0 to 1.0).
    pub weight: f32,
    /// Priority for ordering.
    pub priority: i32,
    /// Distance from region center (for tie-breaking).
    pub distance: f32,
}

impl RegionWeight {
    /// Create a new region weight.
    #[must_use]
    pub fn new(region_index: u32, environment_id: u32, weight: f32, priority: i32) -> Self {
        Self {
            region_index,
            environment_id,
            weight: weight.clamp(0.0, 1.0),
            priority,
            distance: 0.0,
        }
    }

    /// Set distance from region center.
    #[must_use]
    pub fn with_distance(mut self, distance: f32) -> Self {
        self.distance = distance.max(0.0);
        self
    }

    /// Check if this weight contributes meaningfully.
    #[must_use]
    pub fn is_significant(&self) -> bool {
        self.weight > 0.001
    }
}

/// Accumulated weights for blending multiple regions.
#[derive(Debug, Clone)]
pub struct BlendWeights {
    /// Individual region weights.
    weights: Vec<RegionWeight>,
    /// Total accumulated weight.
    total_weight: f32,
    /// Dominant region index (highest priority with non-zero weight).
    dominant_index: Option<usize>,
}

impl BlendWeights {
    /// Create empty blend weights.
    #[must_use]
    pub fn new() -> Self {
        Self {
            weights: Vec::new(),
            total_weight: 0.0,
            dominant_index: None,
        }
    }

    /// Create with capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            weights: Vec::with_capacity(capacity),
            total_weight: 0.0,
            dominant_index: None,
        }
    }

    /// Clear all weights.
    pub fn clear(&mut self) {
        self.weights.clear();
        self.total_weight = 0.0;
        self.dominant_index = None;
    }

    /// Add a region weight contribution.
    pub fn add(&mut self, weight: RegionWeight) {
        if !weight.is_significant() {
            return;
        }

        self.total_weight += weight.weight;

        let new_index = self.weights.len();
        self.weights.push(weight);

        match self.dominant_index {
            Some(idx) => {
                if weight.priority > self.weights[idx].priority {
                    self.dominant_index = Some(new_index);
                }
            }
            None => {
                self.dominant_index = Some(new_index);
            }
        }
    }

    /// Get normalized weights (sum to 1.0).
    #[must_use]
    pub fn normalized(&self) -> Vec<(u32, f32)> {
        if self.total_weight <= 0.0 {
            return Vec::new();
        }

        self.weights
            .iter()
            .map(|w| (w.region_index, w.weight / self.total_weight))
            .collect()
    }

    /// Get the dominant region (highest priority).
    #[must_use]
    pub fn dominant(&self) -> Option<&RegionWeight> {
        self.dominant_index.map(|idx| &self.weights[idx])
    }

    /// Get total accumulated weight.
    #[must_use]
    pub fn total(&self) -> f32 {
        self.total_weight
    }

    /// Number of contributing regions.
    #[must_use]
    pub fn count(&self) -> usize {
        self.weights.len()
    }

    /// Check if any regions contribute.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.weights.is_empty()
    }

    /// Get all weights sorted by priority (descending).
    #[must_use]
    pub fn sorted_by_priority(&self) -> Vec<&RegionWeight> {
        let mut sorted: Vec<_> = self.weights.iter().collect();
        sorted.sort_by(|a, b| {
            b.priority.cmp(&a.priority).then_with(|| {
                a.distance
                    .partial_cmp(&b.distance)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });
        sorted
    }

    /// Compute blended value from all contributing regions.
    #[must_use]
    pub fn blend_values(&self, values: &[f32], mode: PostBlendMode) -> f32 {
        if self.weights.is_empty() || values.is_empty() {
            return 0.0;
        }

        let sorted = self.sorted_by_priority();
        let mut result = 0.0;
        let mut first = true;

        for w in sorted {
            let idx = w.region_index as usize;
            if idx >= values.len() {
                continue;
            }

            let value = values[idx];
            let normalized_weight = if self.total_weight > 0.0 {
                w.weight / self.total_weight
            } else {
                0.0
            };

            if first {
                result = value * normalized_weight;
                first = false;
            } else {
                result = mode.blend(result, value, normalized_weight);
            }
        }

        result
    }
}

impl Default for BlendWeights {
    fn default() -> Self {
        Self::new()
    }
}

/// Priority range constants for common use cases.
pub mod priorities {
    /// Background/ambient effects (always lowest).
    pub const BACKGROUND: i32 = -1000;
    /// Default priority for most effects.
    pub const DEFAULT: i32 = 0;
    /// Environmental effects (caves, water, etc.).
    pub const ENVIRONMENT: i32 = 100;
    /// Local effects (explosions, magic, etc.).
    pub const LOCAL: i32 = 200;
    /// Player-centric effects.
    pub const PLAYER: i32 = 300;
    /// UI/overlay effects (always highest).
    pub const OVERLAY: i32 = 1000;
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_blend_mode_replace() {
        assert_relative_eq!(
            PostBlendMode::Replace.blend(0.5, 1.0, 0.0),
            0.5,
            epsilon = 0.001
        );
        assert_relative_eq!(
            PostBlendMode::Replace.blend(0.5, 1.0, 1.0),
            1.0,
            epsilon = 0.001
        );
    }

    #[test]
    fn test_blend_mode_weighted() {
        assert_relative_eq!(
            PostBlendMode::Weighted.blend(0.0, 1.0, 0.5),
            0.5,
            epsilon = 0.001
        );
        assert_relative_eq!(
            PostBlendMode::Weighted.blend(0.0, 1.0, 0.0),
            0.0,
            epsilon = 0.001
        );
        assert_relative_eq!(
            PostBlendMode::Weighted.blend(0.0, 1.0, 1.0),
            1.0,
            epsilon = 0.001
        );
    }

    #[test]
    fn test_blend_mode_additive() {
        assert_relative_eq!(
            PostBlendMode::Additive.blend(0.5, 0.3, 1.0),
            0.8,
            epsilon = 0.001
        );
        assert_relative_eq!(
            PostBlendMode::Additive.blend(0.8, 0.5, 1.0),
            1.0,
            epsilon = 0.001
        );
    }

    #[test]
    fn test_blend_mode_multiply() {
        assert_relative_eq!(
            PostBlendMode::Multiply.blend(1.0, 0.5, 1.0),
            0.5,
            epsilon = 0.001
        );
        assert_relative_eq!(
            PostBlendMode::Multiply.blend(1.0, 0.5, 0.0),
            1.0,
            epsilon = 0.001
        );
    }

    #[test]
    fn test_blend_mode_max() {
        assert_relative_eq!(
            PostBlendMode::Max.blend(0.3, 0.7, 1.0),
            0.7,
            epsilon = 0.001
        );
        assert_relative_eq!(
            PostBlendMode::Max.blend(0.9, 0.7, 1.0),
            0.9,
            epsilon = 0.001
        );
    }

    #[test]
    fn test_blend_mode_min() {
        assert_relative_eq!(
            PostBlendMode::Min.blend(0.3, 0.7, 1.0),
            0.3,
            epsilon = 0.001
        );
        assert_relative_eq!(
            PostBlendMode::Min.blend(0.9, 0.7, 1.0),
            0.7,
            epsilon = 0.001
        );
    }

    #[test]
    fn test_blend_color() {
        let base = Vec3::new(1.0, 0.0, 0.0);
        let incoming = Vec3::new(0.0, 1.0, 0.0);
        let result = PostBlendMode::Weighted.blend_color(base, incoming, 0.5);

        assert_relative_eq!(result.x, 0.5, epsilon = 0.001);
        assert_relative_eq!(result.y, 0.5, epsilon = 0.001);
        assert_relative_eq!(result.z, 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_region_weight_significance() {
        let significant = RegionWeight::new(0, 0, 0.5, 0);
        let insignificant = RegionWeight::new(0, 0, 0.0001, 0);

        assert!(significant.is_significant());
        assert!(!insignificant.is_significant());
    }

    #[test]
    fn test_blend_weights_empty() {
        let weights = BlendWeights::new();
        assert!(weights.is_empty());
        assert_eq!(weights.count(), 0);
        assert_relative_eq!(weights.total(), 0.0, epsilon = 0.001);
        assert!(weights.dominant().is_none());
    }

    #[test]
    fn test_blend_weights_add() {
        let mut weights = BlendWeights::new();
        weights.add(RegionWeight::new(0, 0, 0.5, 10));
        weights.add(RegionWeight::new(1, 0, 0.3, 5));

        assert_eq!(weights.count(), 2);
        assert_relative_eq!(weights.total(), 0.8, epsilon = 0.001);
    }

    #[test]
    fn test_blend_weights_dominant() {
        let mut weights = BlendWeights::new();
        weights.add(RegionWeight::new(0, 0, 0.5, 5));
        weights.add(RegionWeight::new(1, 0, 0.3, 10));
        weights.add(RegionWeight::new(2, 0, 0.2, 3));

        let dominant = weights.dominant().unwrap();
        assert_eq!(dominant.region_index, 1);
        assert_eq!(dominant.priority, 10);
    }

    #[test]
    fn test_blend_weights_normalized() {
        let mut weights = BlendWeights::new();
        weights.add(RegionWeight::new(0, 0, 0.6, 0));
        weights.add(RegionWeight::new(1, 0, 0.4, 0));

        let normalized = weights.normalized();
        let sum: f32 = normalized.iter().map(|(_, w)| w).sum();
        assert_relative_eq!(sum, 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_blend_weights_sorted_by_priority() {
        let mut weights = BlendWeights::new();
        weights.add(RegionWeight::new(0, 0, 0.5, 5));
        weights.add(RegionWeight::new(1, 0, 0.3, 15));
        weights.add(RegionWeight::new(2, 0, 0.2, 10));

        let sorted = weights.sorted_by_priority();
        assert_eq!(sorted[0].region_index, 1);
        assert_eq!(sorted[1].region_index, 2);
        assert_eq!(sorted[2].region_index, 0);
    }

    #[test]
    fn test_blend_weights_skip_insignificant() {
        let mut weights = BlendWeights::new();
        weights.add(RegionWeight::new(0, 0, 0.0001, 10));
        weights.add(RegionWeight::new(1, 0, 0.5, 5));

        assert_eq!(weights.count(), 1);
        assert_eq!(weights.weights[0].region_index, 1);
    }

    #[test]
    fn test_blend_weights_blend_values() {
        let mut weights = BlendWeights::new();
        weights.add(RegionWeight::new(0, 0, 0.6, 10));
        weights.add(RegionWeight::new(1, 0, 0.4, 5));

        let values = [0.8, 0.2];
        let result = weights.blend_values(&values, PostBlendMode::Weighted);

        assert!(result > 0.0 && result < 1.0);
    }

    #[test]
    fn test_blend_weights_clear() {
        let mut weights = BlendWeights::new();
        weights.add(RegionWeight::new(0, 0, 0.5, 10));
        assert!(!weights.is_empty());

        weights.clear();
        assert!(weights.is_empty());
        assert!(weights.dominant().is_none());
    }

    #[test]
    fn test_priority_constants() {
        const { assert!(priorities::BACKGROUND < priorities::DEFAULT) };
        const { assert!(priorities::DEFAULT < priorities::ENVIRONMENT) };
        const { assert!(priorities::ENVIRONMENT < priorities::LOCAL) };
        const { assert!(priorities::LOCAL < priorities::PLAYER) };
        const { assert!(priorities::PLAYER < priorities::OVERLAY) };
    }
}
