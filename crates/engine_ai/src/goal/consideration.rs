//! Considerations for goal utility scoring.

use super::curve::UtilityCurve;
use serde::{Deserialize, Serialize};

/// Identifier for a consideration within a goal.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ConsiderationId(pub String);

impl ConsiderationId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for ConsiderationId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

/// Binding that specifies where to read the input value from context.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum InputBinding {
    /// Read normalized need value (0.0 = empty, 1.0 = full).
    NeedValue(String),
    /// Read need deficit (1.0 - normalized value).
    NeedDeficit(String),
    /// Read need urgency score.
    NeedUrgency(String),
    /// Read specific context fact by name.
    Fact(String),
    /// Read threat level (0.0 = safe, 1.0 = extreme danger).
    ThreatLevel,
    /// Read danger proximity (0.0 = far, 1.0 = immediate).
    DangerProximity,
    /// Read stimulus intensity for a specific sensor kind.
    StimulusIntensity(String),
    /// Read number of nearby allies (normalized by threshold).
    AllyCount { threshold: u32 },
    /// Read number of nearby enemies (normalized by threshold).
    EnemyCount { threshold: u32 },
    /// Read territory ownership (0.0 = unowned, 1.0 = fully controlled).
    TerritoryOwnership,
    /// Read faction standing with another faction (normalized).
    FactionStanding(String),
    /// Read distance to leader (normalized, 1.0 = max distance).
    LeaderDistance { max_distance: f32 },
    /// Read time since last goal completion (normalized by threshold).
    TimeSinceGoal {
        goal_id: String,
        threshold_ticks: u64,
    },
    /// Constant value.
    Constant(f32),
}

impl InputBinding {
    /// Create a need value binding.
    #[must_use]
    pub fn need_value(need_id: impl Into<String>) -> Self {
        Self::NeedValue(need_id.into())
    }

    /// Create a need deficit binding.
    #[must_use]
    pub fn need_deficit(need_id: impl Into<String>) -> Self {
        Self::NeedDeficit(need_id.into())
    }

    /// Create a need urgency binding.
    #[must_use]
    pub fn need_urgency(need_id: impl Into<String>) -> Self {
        Self::NeedUrgency(need_id.into())
    }

    /// Create a fact binding.
    #[must_use]
    pub fn fact(name: impl Into<String>) -> Self {
        Self::Fact(name.into())
    }

    /// Create a stimulus intensity binding.
    #[must_use]
    pub fn stimulus_intensity(kind: impl Into<String>) -> Self {
        Self::StimulusIntensity(kind.into())
    }

    /// Create an ally count binding.
    #[must_use]
    pub fn ally_count(threshold: u32) -> Self {
        Self::AllyCount { threshold }
    }

    /// Create an enemy count binding.
    #[must_use]
    pub fn enemy_count(threshold: u32) -> Self {
        Self::EnemyCount { threshold }
    }

    /// Create a faction standing binding.
    #[must_use]
    pub fn faction_standing(faction_id: impl Into<String>) -> Self {
        Self::FactionStanding(faction_id.into())
    }

    /// Create a leader distance binding.
    #[must_use]
    pub fn leader_distance(max_distance: f32) -> Self {
        Self::LeaderDistance { max_distance }
    }

    /// Create a time since goal binding.
    #[must_use]
    pub fn time_since_goal(goal_id: impl Into<String>, threshold_ticks: u64) -> Self {
        Self::TimeSinceGoal {
            goal_id: goal_id.into(),
            threshold_ticks,
        }
    }

    /// Create a constant binding.
    #[must_use]
    pub fn constant(value: f32) -> Self {
        Self::Constant(value)
    }
}

/// A single consideration that contributes to goal utility scoring.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Consideration {
    /// Identifier for this consideration.
    pub id: ConsiderationId,
    /// Where to read the input value from.
    pub input: InputBinding,
    /// Curve to transform input to utility.
    pub curve: UtilityCurve,
    /// Weight multiplier for this consideration (default 1.0).
    pub weight: f32,
    /// Whether this consideration can veto the goal (output 0 blocks goal).
    pub is_veto: bool,
    /// Minimum output to not trigger veto (if `is_veto` is true).
    pub veto_threshold: f32,
}

impl Consideration {
    /// Create a new consideration with default values.
    #[must_use]
    pub fn new(id: impl Into<ConsiderationId>) -> Self {
        Self {
            id: id.into(),
            input: InputBinding::Constant(0.5),
            curve: UtilityCurve::linear(),
            weight: 1.0,
            is_veto: false,
            veto_threshold: 0.01,
        }
    }

    /// Set the input binding.
    #[must_use]
    pub fn with_input(mut self, input: InputBinding) -> Self {
        self.input = input;
        self
    }

    /// Set the utility curve.
    #[must_use]
    pub fn with_curve(mut self, curve: UtilityCurve) -> Self {
        self.curve = curve;
        self
    }

    /// Set the weight.
    #[must_use]
    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }

    /// Set as veto consideration.
    #[must_use]
    pub fn with_veto(mut self, threshold: f32) -> Self {
        self.is_veto = true;
        self.veto_threshold = threshold;
        self
    }

    /// Evaluate this consideration given an input value.
    #[must_use]
    pub fn evaluate(&self, input_value: f32) -> f32 {
        let utility = self.curve.evaluate(input_value);
        utility * self.weight
    }

    /// Check if this consideration vetoes at the given utility.
    #[must_use]
    pub fn is_vetoed(&self, utility: f32) -> bool {
        self.is_veto && utility < self.veto_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consideration_id() {
        let id = ConsiderationId::new("hunger_deficit");
        assert_eq!(id.as_str(), "hunger_deficit");

        let id2: ConsiderationId = "safety_factor".into();
        assert_eq!(id2.as_str(), "safety_factor");
    }

    #[test]
    fn test_input_binding_constructors() {
        let b1 = InputBinding::need_value("hunger");
        assert!(matches!(b1, InputBinding::NeedValue(s) if s == "hunger"));

        let b2 = InputBinding::need_deficit("thirst");
        assert!(matches!(b2, InputBinding::NeedDeficit(s) if s == "thirst"));

        let b3 = InputBinding::ally_count(5);
        assert!(matches!(b3, InputBinding::AllyCount { threshold: 5 }));

        let b4 = InputBinding::leader_distance(100.0);
        assert!(
            matches!(b4, InputBinding::LeaderDistance { max_distance } if (max_distance - 100.0).abs() < f32::EPSILON)
        );
    }

    #[test]
    fn test_consideration_new() {
        let c = Consideration::new("test");

        assert_eq!(c.id.as_str(), "test");
        assert!((c.weight - 1.0).abs() < f32::EPSILON);
        assert!(!c.is_veto);
    }

    #[test]
    fn test_consideration_builder() {
        let c = Consideration::new("hunger")
            .with_input(InputBinding::need_deficit("hunger"))
            .with_curve(UtilityCurve::quadratic())
            .with_weight(1.5);

        assert!(matches!(c.input, InputBinding::NeedDeficit(_)));
        assert!((c.weight - 1.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_consideration_veto() {
        let c = Consideration::new("safety")
            .with_input(InputBinding::ThreatLevel)
            .with_curve(UtilityCurve::inverse_linear())
            .with_veto(0.1);

        assert!(c.is_veto);
        assert!((c.veto_threshold - 0.1).abs() < f32::EPSILON);

        assert!(c.is_vetoed(0.05));
        assert!(!c.is_vetoed(0.5));
    }

    #[test]
    fn test_consideration_evaluate() {
        let c = Consideration::new("test")
            .with_input(InputBinding::Constant(0.5))
            .with_curve(UtilityCurve::linear())
            .with_weight(2.0);

        let result = c.evaluate(0.5);
        assert!((result - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_consideration_evaluate_with_curve() {
        let c = Consideration::new("test")
            .with_curve(UtilityCurve::quadratic())
            .with_weight(1.0);

        assert!((c.evaluate(0.5) - 0.25).abs() < f32::EPSILON);
        assert!((c.evaluate(1.0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_input_binding_serde() {
        let binding = InputBinding::time_since_goal("flee_danger", 300);

        let json = serde_json::to_string(&binding).unwrap();
        let restored: InputBinding = serde_json::from_str(&json).unwrap();

        assert!(
            matches!(restored, InputBinding::TimeSinceGoal { goal_id, threshold_ticks }
                if goal_id == "flee_danger" && threshold_ticks == 300)
        );
    }

    #[test]
    fn test_consideration_serde() {
        let c = Consideration::new("hunger")
            .with_input(InputBinding::need_deficit("hunger"))
            .with_curve(UtilityCurve::sigmoid())
            .with_weight(1.5)
            .with_veto(0.05);

        let json = serde_json::to_string(&c).unwrap();
        let restored: Consideration = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, c.id);
        assert!((restored.weight - 1.5).abs() < f32::EPSILON);
        assert!(restored.is_veto);
    }

    #[test]
    fn test_input_binding_threat_level() {
        let binding = InputBinding::ThreatLevel;
        assert!(matches!(binding, InputBinding::ThreatLevel));
    }

    #[test]
    fn test_input_binding_territory() {
        let binding = InputBinding::TerritoryOwnership;
        assert!(matches!(binding, InputBinding::TerritoryOwnership));
    }
}
