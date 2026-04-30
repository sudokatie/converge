//! Generation context for structure grammar.

use serde::{Deserialize, Serialize};

use super::template::Bounds;

/// Configuration for structure generation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GenerationConfig {
    /// Random seed for deterministic generation.
    pub seed: u64,
    /// Maximum recursion depth.
    pub max_depth: u32,
    /// Maximum expansion steps.
    pub max_steps: u32,
    /// Optional bounds constraint.
    pub bounds: Option<Bounds>,
    /// Whether to allow overlapping placements.
    pub allow_overlap: bool,
    /// Tags to enable during generation.
    pub enabled_tags: Vec<String>,
}

impl GenerationConfig {
    /// Create a new config with seed.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            max_depth: 10,
            max_steps: 1000,
            bounds: None,
            allow_overlap: false,
            enabled_tags: Vec::new(),
        }
    }

    /// Set max depth.
    #[must_use]
    pub fn with_max_depth(mut self, depth: u32) -> Self {
        self.max_depth = depth;
        self
    }

    /// Set max steps.
    #[must_use]
    pub fn with_max_steps(mut self, steps: u32) -> Self {
        self.max_steps = steps;
        self
    }

    /// Set bounds constraint.
    #[must_use]
    pub fn with_bounds(mut self, bounds: Bounds) -> Self {
        self.bounds = Some(bounds);
        self
    }

    /// Allow overlapping placements.
    #[must_use]
    pub fn with_overlap(mut self, allow: bool) -> Self {
        self.allow_overlap = allow;
        self
    }

    /// Enable a tag.
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.enabled_tags.push(tag.into());
        self
    }
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Runtime context during generation.
#[derive(Clone, Debug)]
pub struct GenerationContext {
    /// Configuration.
    pub config: GenerationConfig,
    /// Current recursion depth.
    pub depth: u32,
    /// Total steps taken.
    pub steps: u32,
    /// RNG state for deterministic generation.
    rng_state: u64,
}

impl GenerationContext {
    /// Create a new context from config.
    #[must_use]
    pub fn new(config: GenerationConfig) -> Self {
        Self {
            rng_state: config.seed,
            config,
            depth: 0,
            steps: 0,
        }
    }

    /// Create with seed only.
    #[must_use]
    pub fn with_seed(seed: u64) -> Self {
        Self::new(GenerationConfig::new(seed))
    }

    /// Generate next random u64.
    pub fn next_u64(&mut self) -> u64 {
        self.rng_state = self
            .rng_state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        self.rng_state
    }

    /// Generate random float in [0, 1).
    #[expect(clippy::cast_precision_loss)]
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u64() as f32) / (u64::MAX as f32)
    }

    /// Generate random value in range [0, max).
    #[expect(clippy::cast_possible_truncation)]
    pub fn next_range(&mut self, max: u32) -> u32 {
        if max == 0 {
            return 0;
        }
        (self.next_u64() % u64::from(max)) as u32
    }

    /// Check if depth limit exceeded.
    #[must_use]
    pub fn depth_exceeded(&self) -> bool {
        self.depth > self.config.max_depth
    }

    /// Check if step limit exceeded.
    #[must_use]
    pub fn steps_exceeded(&self) -> bool {
        self.steps > self.config.max_steps
    }

    /// Increment depth.
    pub fn enter(&mut self) {
        self.depth += 1;
    }

    /// Decrement depth.
    pub fn exit(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    /// Increment step counter.
    pub fn step(&mut self) {
        self.steps += 1;
    }

    /// Check if a tag is enabled.
    #[must_use]
    pub fn has_tag(&self, tag: &str) -> bool {
        self.config.enabled_tags.iter().any(|t| t == tag)
    }

    /// Check if position is within bounds.
    #[must_use]
    pub fn in_bounds(&self, pos: [i32; 3]) -> bool {
        self.config.bounds.as_ref().is_none_or(|b| b.contains(pos))
    }

    /// Check if bounds fit within constraint.
    #[must_use]
    pub fn bounds_fit(&self, bounds: &Bounds) -> bool {
        self.config.bounds.as_ref().is_none_or(|constraint| {
            bounds.min[0] >= constraint.min[0]
                && bounds.min[1] >= constraint.min[1]
                && bounds.min[2] >= constraint.min[2]
                && bounds.max[0] <= constraint.max[0]
                && bounds.max[1] <= constraint.max[1]
                && bounds.max[2] <= constraint.max[2]
        })
    }

    /// Create a child context at increased depth.
    #[must_use]
    pub fn child(&self) -> Self {
        Self {
            config: self.config.clone(),
            depth: self.depth + 1,
            steps: self.steps,
            rng_state: self.rng_state,
        }
    }

    /// Get current RNG state (for fingerprinting).
    #[must_use]
    pub fn rng_state(&self) -> u64 {
        self.rng_state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_creation() {
        let config = GenerationConfig::new(42)
            .with_max_depth(5)
            .with_max_steps(100);

        assert_eq!(config.seed, 42);
        assert_eq!(config.max_depth, 5);
        assert_eq!(config.max_steps, 100);
    }

    #[test]
    fn context_deterministic_rng() {
        let mut ctx1 = GenerationContext::with_seed(12345);
        let mut ctx2 = GenerationContext::with_seed(12345);

        for _ in 0..10 {
            assert_eq!(ctx1.next_u64(), ctx2.next_u64());
        }
    }

    #[test]
    fn context_different_seeds() {
        let mut ctx1 = GenerationContext::with_seed(111);
        let mut ctx2 = GenerationContext::with_seed(222);

        let mut same = true;
        for _ in 0..10 {
            if ctx1.next_u64() != ctx2.next_u64() {
                same = false;
                break;
            }
        }
        assert!(!same);
    }

    #[test]
    fn context_depth_tracking() {
        let mut ctx = GenerationContext::new(GenerationConfig::new(0).with_max_depth(3));

        assert!(!ctx.depth_exceeded());
        ctx.enter();
        ctx.enter();
        ctx.enter();
        assert!(!ctx.depth_exceeded());
        ctx.enter();
        assert!(ctx.depth_exceeded());
        ctx.exit();
        assert!(!ctx.depth_exceeded());
    }

    #[test]
    fn context_step_tracking() {
        let mut ctx = GenerationContext::new(GenerationConfig::new(0).with_max_steps(5));

        for _ in 0..5 {
            assert!(!ctx.steps_exceeded());
            ctx.step();
        }
        ctx.step();
        assert!(ctx.steps_exceeded());
    }

    #[test]
    fn context_bounds_check() {
        let ctx = GenerationContext::new(
            GenerationConfig::new(0).with_bounds(Bounds::new([0, 0, 0], [100, 100, 100])),
        );

        assert!(ctx.in_bounds([50, 50, 50]));
        assert!(!ctx.in_bounds([150, 50, 50]));

        let inner = Bounds::new([10, 10, 10], [90, 90, 90]);
        assert!(ctx.bounds_fit(&inner));

        let outer = Bounds::new([-10, 0, 0], [110, 100, 100]);
        assert!(!ctx.bounds_fit(&outer));
    }

    #[test]
    fn context_tags() {
        let ctx = GenerationContext::new(
            GenerationConfig::new(0)
                .with_tag("interior")
                .with_tag("hazard"),
        );

        assert!(ctx.has_tag("interior"));
        assert!(ctx.has_tag("hazard"));
        assert!(!ctx.has_tag("exterior"));
    }

    #[test]
    fn context_child() {
        let parent = GenerationContext::new(GenerationConfig::new(42).with_max_depth(10));
        let child = parent.child();

        assert_eq!(child.depth, parent.depth + 1);
        assert_eq!(child.config.seed, parent.config.seed);
    }

    #[test]
    fn next_range() {
        let mut ctx = GenerationContext::with_seed(42);
        for _ in 0..100 {
            let val = ctx.next_range(10);
            assert!(val < 10);
        }
    }

    #[test]
    fn next_f32_range() {
        let mut ctx = GenerationContext::with_seed(42);
        for _ in 0..100 {
            let val = ctx.next_f32();
            assert!((0.0..1.0).contains(&val));
        }
    }
}
