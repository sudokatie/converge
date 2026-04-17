//! Combined steering behaviors - weighted combination of multiple behaviors.

use crate::behavior::blackboard::Vec3;
use super::{SteeringBehavior, SteeringOutput};

/// Weight configuration for a steering behavior
#[derive(Debug, Clone)]
pub struct SteeringWeight {
    /// Weight multiplier (higher = more influence)
    pub weight: f32,
    /// Priority (higher priority behaviors are processed first)
    pub priority: u32,
}

impl SteeringWeight {
    pub fn new(weight: f32) -> Self {
        Self { weight, priority: 0 }
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }
}

impl Default for SteeringWeight {
    fn default() -> Self {
        Self::new(1.0)
    }
}

/// Combines multiple steering behaviors with weights
pub struct SteeringCombiner {
    /// Maximum total force magnitude
    pub max_force: f32,
    /// Whether to use priority-based blending
    pub use_priorities: bool,
}

impl SteeringCombiner {
    pub fn new() -> Self {
        Self {
            max_force: 10.0,
            use_priorities: false,
        }
    }

    pub fn with_max_force(mut self, force: f32) -> Self {
        self.max_force = force;
        self
    }

    pub fn with_priorities(mut self, use_priorities: bool) -> Self {
        self.use_priorities = use_priorities;
        self
    }

    /// Combine steering outputs using weighted blending
    pub fn blend_weighted(&self, outputs: &[(SteeringOutput, SteeringWeight)]) -> SteeringOutput {
        if outputs.is_empty() {
            return SteeringOutput::zero();
        }

        let mut total_x = 0.0;
        let mut total_y = 0.0;
        let mut total_z = 0.0;
        let mut total_angular = 0.0;
        let mut total_weight = 0.0;

        for (output, weight) in outputs {
            total_x += output.linear.x * weight.weight;
            total_y += output.linear.y * weight.weight;
            total_z += output.linear.z * weight.weight;
            total_angular += output.angular * weight.weight;
            total_weight += weight.weight;
        }

        if total_weight > 0.0 {
            let result = SteeringOutput::new(
                Vec3::new(
                    total_x / total_weight,
                    total_y / total_weight,
                    total_z / total_weight,
                ),
                total_angular / total_weight,
            );
            result.clamp_linear(self.max_force)
        } else {
            SteeringOutput::zero()
        }
    }

    /// Combine steering outputs using priority-based blending
    /// Higher priority behaviors override lower ones
    pub fn blend_priority(&self, outputs: &mut [(SteeringOutput, SteeringWeight)]) -> SteeringOutput {
        if outputs.is_empty() {
            return SteeringOutput::zero();
        }

        // Sort by priority (descending)
        outputs.sort_by(|a, b| b.1.priority.cmp(&a.1.priority));

        let mut remaining_budget = self.max_force;
        let mut result = SteeringOutput::zero();

        for (output, weight) in outputs.iter() {
            if remaining_budget <= 0.0 {
                break;
            }

            let magnitude = (output.linear.x * output.linear.x
                + output.linear.y * output.linear.y
                + output.linear.z * output.linear.z)
                .sqrt();

            let weighted_magnitude = magnitude * weight.weight;
            let contribution = weighted_magnitude.min(remaining_budget);

            if magnitude > 0.0 {
                let scale = contribution / magnitude;
                result = result.add(&output.scale(scale));
                remaining_budget -= contribution;
            }
        }

        result.clamp_linear(self.max_force)
    }

    /// Combine behaviors - automatically chooses blending mode
    pub fn combine(&self, outputs: &mut [(SteeringOutput, SteeringWeight)]) -> SteeringOutput {
        if self.use_priorities {
            self.blend_priority(outputs)
        } else {
            self.blend_weighted(outputs)
        }
    }
}

impl Default for SteeringCombiner {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to build combined steering from multiple behaviors
pub struct SteeringBuilder {
    behaviors: Vec<(Box<dyn SteeringBehavior>, SteeringWeight)>,
    combiner: SteeringCombiner,
}

impl SteeringBuilder {
    pub fn new() -> Self {
        Self {
            behaviors: Vec::new(),
            combiner: SteeringCombiner::new(),
        }
    }

    pub fn with_combiner(mut self, combiner: SteeringCombiner) -> Self {
        self.combiner = combiner;
        self
    }

    pub fn add<B: SteeringBehavior + 'static>(mut self, behavior: B, weight: SteeringWeight) -> Self {
        self.behaviors.push((Box::new(behavior), weight));
        self
    }

    pub fn calculate(&self, position: &Vec3, velocity: &Vec3) -> SteeringOutput {
        let mut outputs: Vec<(SteeringOutput, SteeringWeight)> = self
            .behaviors
            .iter()
            .map(|(behavior, weight)| (behavior.calculate(position, velocity), weight.clone()))
            .collect();

        self.combiner.combine(&mut outputs)
    }
}

impl Default for SteeringBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::steering::{Seek, Flee};

    #[test]
    fn test_weighted_blend() {
        let combiner = SteeringCombiner::new();

        let outputs = vec![
            (SteeringOutput::new(Vec3::new(10.0, 0.0, 0.0), 0.0), SteeringWeight::new(1.0)),
            (SteeringOutput::new(Vec3::new(0.0, 0.0, 10.0), 0.0), SteeringWeight::new(1.0)),
        ];

        let result = combiner.blend_weighted(&outputs);

        // Average of two perpendicular forces
        assert!((result.linear.x - 5.0).abs() < 0.001);
        assert!((result.linear.z - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_weighted_blend_unequal() {
        let combiner = SteeringCombiner::new();

        let outputs = vec![
            (SteeringOutput::new(Vec3::new(10.0, 0.0, 0.0), 0.0), SteeringWeight::new(3.0)),
            (SteeringOutput::new(Vec3::new(0.0, 0.0, 10.0), 0.0), SteeringWeight::new(1.0)),
        ];

        let result = combiner.blend_weighted(&outputs);

        // 3:1 weighted average
        assert!((result.linear.x - 7.5).abs() < 0.001);
        assert!((result.linear.z - 2.5).abs() < 0.001);
    }

    #[test]
    fn test_max_force_clamp() {
        let combiner = SteeringCombiner::new().with_max_force(5.0);

        let outputs = vec![
            (SteeringOutput::new(Vec3::new(100.0, 0.0, 0.0), 0.0), SteeringWeight::new(1.0)),
        ];

        let result = combiner.blend_weighted(&outputs);

        // Should be clamped to max force
        let magnitude = (result.linear.x * result.linear.x + result.linear.z * result.linear.z).sqrt();
        assert!((magnitude - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_builder() {
        let seek = Seek::new(Vec3::new(10.0, 0.0, 0.0));
        let flee = Flee::new(Vec3::new(-10.0, 0.0, 0.0));

        let builder = SteeringBuilder::new()
            .add(seek, SteeringWeight::new(1.0))
            .add(flee, SteeringWeight::new(0.5));

        let result = builder.calculate(&Vec3::new(0.0, 0.0, 0.0), &Vec3::new(0.0, 0.0, 0.0));

        // Both behaviors push in +X direction
        assert!(result.linear.x > 0.0);
    }

    #[test]
    fn test_priority_blend() {
        let combiner = SteeringCombiner::new()
            .with_max_force(10.0)
            .with_priorities(true);

        let mut outputs = vec![
            (
                SteeringOutput::new(Vec3::new(5.0, 0.0, 0.0), 0.0),
                SteeringWeight::new(1.0).with_priority(10), // High priority
            ),
            (
                SteeringOutput::new(Vec3::new(0.0, 0.0, 100.0), 0.0),
                SteeringWeight::new(1.0).with_priority(1), // Low priority
            ),
        ];

        let result = combiner.blend_priority(&mut outputs);

        // High priority should dominate within budget
        assert!(result.linear.x > 0.0);
    }
}
