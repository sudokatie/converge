//! Scoring types and evaluation for goal utility.

use super::consideration::ConsiderationId;
use super::definition::GoalId;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Score breakdown for a single consideration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConsiderationScore {
    /// Consideration identifier.
    pub id: ConsiderationId,
    /// Raw input value from context.
    pub input_value: f32,
    /// Output after curve transformation.
    pub curve_output: f32,
    /// Final weighted contribution.
    pub weighted_score: f32,
    /// Whether this consideration triggered a veto.
    pub vetoed: bool,
}

impl ConsiderationScore {
    /// Create a new consideration score.
    #[must_use]
    pub fn new(
        id: ConsiderationId,
        input_value: f32,
        curve_output: f32,
        weighted_score: f32,
        vetoed: bool,
    ) -> Self {
        Self {
            id,
            input_value,
            curve_output,
            weighted_score,
            vetoed,
        }
    }
}

/// Detailed scoring breakdown for a goal.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScoringBreakdown {
    /// Goal identifier.
    pub goal_id: GoalId,
    /// Individual consideration scores.
    pub considerations: Vec<ConsiderationScore>,
    /// Combined utility before priority weighting.
    pub raw_utility: f32,
    /// Base priority from goal definition.
    pub base_priority: f32,
    /// Inertia bonus if this is the current goal.
    pub inertia_bonus: f32,
    /// Hysteresis penalty if recently completed.
    pub hysteresis_penalty: f32,
    /// Final score after all modifiers.
    pub final_score: f32,
    /// Whether the goal was vetoed by any consideration.
    pub vetoed: bool,
    /// Whether the goal is on cooldown.
    pub on_cooldown: bool,
    /// Whether the goal met the minimum threshold.
    pub met_threshold: bool,
}

impl ScoringBreakdown {
    /// Create a new scoring breakdown.
    #[must_use]
    pub fn new(goal_id: GoalId) -> Self {
        Self {
            goal_id,
            considerations: Vec::new(),
            raw_utility: 0.0,
            base_priority: 1.0,
            inertia_bonus: 0.0,
            hysteresis_penalty: 0.0,
            final_score: 0.0,
            vetoed: false,
            on_cooldown: false,
            met_threshold: true,
        }
    }

    /// Add a consideration score.
    pub fn add_consideration(&mut self, score: ConsiderationScore) {
        if score.vetoed {
            self.vetoed = true;
        }
        self.considerations.push(score);
    }

    /// Compute the final score from considerations and modifiers.
    pub fn compute_final_score(&mut self, min_threshold: f32) {
        if self.vetoed || self.on_cooldown {
            self.final_score = 0.0;
            self.met_threshold = false;
            return;
        }

        if self.considerations.is_empty() {
            self.raw_utility = 0.0;
        } else {
            let product: f32 = self
                .considerations
                .iter()
                .map(|c| c.weighted_score)
                .product();

            #[expect(clippy::cast_precision_loss, reason = "consideration count is small")]
            let compensation_factor = 1.0 - (1.0 / self.considerations.len() as f32);
            self.raw_utility =
                product + (1.0 - product) * compensation_factor * self.average_score();
        }

        self.met_threshold = self.raw_utility >= min_threshold;

        if !self.met_threshold {
            self.final_score = 0.0;
            return;
        }

        let priority_weighted = self.raw_utility * self.base_priority;
        let with_inertia = priority_weighted + self.inertia_bonus;
        self.final_score = (with_inertia - self.hysteresis_penalty).max(0.0);
    }

    fn average_score(&self) -> f32 {
        if self.considerations.is_empty() {
            return 0.0;
        }
        #[expect(clippy::cast_precision_loss, reason = "consideration count is small")]
        {
            self.considerations
                .iter()
                .map(|c| c.weighted_score)
                .sum::<f32>()
                / self.considerations.len() as f32
        }
    }

    /// Get a human-readable explanation of why this goal scored as it did.
    #[must_use]
    pub fn explain(&self) -> String {
        let mut parts = Vec::new();

        if self.vetoed {
            let vetoing: Vec<_> = self
                .considerations
                .iter()
                .filter(|c| c.vetoed)
                .map(|c| c.id.as_str())
                .collect();
            parts.push(format!("Vetoed by: {}", vetoing.join(", ")));
        }

        if self.on_cooldown {
            parts.push("On cooldown".to_string());
        }

        if !self.vetoed && !self.on_cooldown {
            parts.push(format!("Raw utility: {:.3}", self.raw_utility));
            parts.push(format!("Base priority: {:.2}", self.base_priority));

            if self.inertia_bonus > 0.0 {
                parts.push(format!("Inertia bonus: +{:.3}", self.inertia_bonus));
            }

            if self.hysteresis_penalty > 0.0 {
                parts.push(format!(
                    "Hysteresis penalty: -{:.3}",
                    self.hysteresis_penalty
                ));
            }

            if !self.met_threshold {
                parts.push("Below minimum threshold".to_string());
            }
        }

        parts.push(format!("Final score: {:.3}", self.final_score));

        parts.join("; ")
    }

    /// Get the top contributing considerations.
    #[must_use]
    pub fn top_contributors(&self, count: usize) -> Vec<&ConsiderationScore> {
        let mut sorted: Vec<_> = self.considerations.iter().collect();
        sorted.sort_by(|a, b| {
            b.weighted_score
                .partial_cmp(&a.weighted_score)
                .unwrap_or(Ordering::Equal)
        });
        sorted.truncate(count);
        sorted
    }

    /// Check if this breakdown represents a viable goal selection.
    #[must_use]
    pub fn is_viable(&self) -> bool {
        !self.vetoed && !self.on_cooldown && self.met_threshold && self.final_score > 0.0
    }
}

/// Final score for a goal with deterministic ordering.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GoalScore {
    /// Goal identifier.
    pub id: GoalId,
    /// Final computed score.
    pub score: f32,
    /// Tick when scored.
    pub scored_tick: u64,
}

impl GoalScore {
    /// Create a new goal score.
    #[must_use]
    pub fn new(id: GoalId, score: f32, tick: u64) -> Self {
        Self {
            id,
            score,
            scored_tick: tick,
        }
    }
}

impl Eq for GoalScore {}

impl PartialOrd for GoalScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GoalScore {
    fn cmp(&self, other: &Self) -> Ordering {
        match other.score.partial_cmp(&self.score) {
            Some(Ordering::Equal) | None => self.id.cmp(&other.id),
            Some(ord) => ord,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consideration_score() {
        let score = ConsiderationScore::new("hunger".into(), 0.7, 0.49, 0.49, false);

        assert_eq!(score.id.as_str(), "hunger");
        assert!((score.input_value - 0.7).abs() < f32::EPSILON);
        assert!((score.curve_output - 0.49).abs() < f32::EPSILON);
        assert!(!score.vetoed);
    }

    #[test]
    fn test_scoring_breakdown_new() {
        let breakdown = ScoringBreakdown::new(GoalId::satisfy_hunger());

        assert_eq!(breakdown.goal_id, GoalId::satisfy_hunger());
        assert!(breakdown.considerations.is_empty());
        assert!(!breakdown.vetoed);
        assert!(!breakdown.on_cooldown);
    }

    #[test]
    fn test_scoring_breakdown_add_consideration() {
        let mut breakdown = ScoringBreakdown::new(GoalId::satisfy_hunger());

        breakdown.add_consideration(ConsiderationScore::new(
            "hunger".into(),
            0.7,
            0.7,
            0.7,
            false,
        ));

        assert_eq!(breakdown.considerations.len(), 1);
        assert!(!breakdown.vetoed);
    }

    #[test]
    fn test_scoring_breakdown_veto() {
        let mut breakdown = ScoringBreakdown::new(GoalId::satisfy_hunger());

        breakdown.add_consideration(ConsiderationScore::new(
            "safety".into(),
            0.9,
            0.0,
            0.0,
            true,
        ));

        assert!(breakdown.vetoed);
    }

    #[test]
    fn test_scoring_breakdown_compute_final() {
        let mut breakdown = ScoringBreakdown::new(GoalId::satisfy_hunger());
        breakdown.base_priority = 1.5;

        breakdown.add_consideration(ConsiderationScore::new(
            "hunger".into(),
            0.6,
            0.6,
            0.6,
            false,
        ));
        breakdown.add_consideration(ConsiderationScore::new(
            "safety".into(),
            0.8,
            0.8,
            0.8,
            false,
        ));

        breakdown.compute_final_score(0.0);

        assert!(breakdown.final_score > 0.0);
        assert!(breakdown.met_threshold);
    }

    #[test]
    fn test_scoring_breakdown_vetoed_zero_score() {
        let mut breakdown = ScoringBreakdown::new(GoalId::satisfy_hunger());
        breakdown.vetoed = true;

        breakdown.compute_final_score(0.0);

        assert!((breakdown.final_score).abs() < f32::EPSILON);
    }

    #[test]
    fn test_scoring_breakdown_cooldown_zero_score() {
        let mut breakdown = ScoringBreakdown::new(GoalId::satisfy_hunger());
        breakdown.on_cooldown = true;

        breakdown.add_consideration(ConsiderationScore::new(
            "hunger".into(),
            1.0,
            1.0,
            1.0,
            false,
        ));

        breakdown.compute_final_score(0.0);

        assert!((breakdown.final_score).abs() < f32::EPSILON);
    }

    #[test]
    fn test_scoring_breakdown_below_threshold() {
        let mut breakdown = ScoringBreakdown::new(GoalId::satisfy_hunger());

        breakdown.add_consideration(ConsiderationScore::new(
            "hunger".into(),
            0.1,
            0.1,
            0.1,
            false,
        ));

        breakdown.compute_final_score(0.5);

        assert!(!breakdown.met_threshold);
        assert!((breakdown.final_score).abs() < f32::EPSILON);
    }

    #[test]
    fn test_scoring_breakdown_inertia() {
        let mut breakdown = ScoringBreakdown::new(GoalId::satisfy_hunger());
        breakdown.inertia_bonus = 0.2;

        breakdown.add_consideration(ConsiderationScore::new(
            "hunger".into(),
            0.5,
            0.5,
            0.5,
            false,
        ));

        breakdown.compute_final_score(0.0);

        let without_inertia = breakdown.raw_utility * breakdown.base_priority;
        assert!(breakdown.final_score > without_inertia);
    }

    #[test]
    fn test_scoring_breakdown_hysteresis() {
        let mut breakdown = ScoringBreakdown::new(GoalId::satisfy_hunger());
        breakdown.hysteresis_penalty = 0.3;

        breakdown.add_consideration(ConsiderationScore::new(
            "hunger".into(),
            0.8,
            0.8,
            0.8,
            false,
        ));

        breakdown.compute_final_score(0.0);

        let without_penalty = breakdown.raw_utility * breakdown.base_priority;
        assert!(breakdown.final_score < without_penalty);
    }

    #[test]
    fn test_scoring_breakdown_explain() {
        let mut breakdown = ScoringBreakdown::new(GoalId::satisfy_hunger());
        breakdown.vetoed = true;

        breakdown.add_consideration(ConsiderationScore::new(
            "safety".into(),
            0.0,
            0.0,
            0.0,
            true,
        ));

        let explanation = breakdown.explain();
        assert!(explanation.contains("Vetoed"));
        assert!(explanation.contains("safety"));
    }

    #[test]
    fn test_scoring_breakdown_top_contributors() {
        let mut breakdown = ScoringBreakdown::new(GoalId::satisfy_hunger());

        breakdown.add_consideration(ConsiderationScore::new("a".into(), 0.3, 0.3, 0.3, false));
        breakdown.add_consideration(ConsiderationScore::new("b".into(), 0.8, 0.8, 0.8, false));
        breakdown.add_consideration(ConsiderationScore::new("c".into(), 0.5, 0.5, 0.5, false));

        let top = breakdown.top_contributors(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].id.as_str(), "b");
        assert_eq!(top[1].id.as_str(), "c");
    }

    #[test]
    fn test_scoring_breakdown_is_viable() {
        let mut breakdown = ScoringBreakdown::new(GoalId::satisfy_hunger());
        breakdown.add_consideration(ConsiderationScore::new(
            "hunger".into(),
            0.7,
            0.7,
            0.7,
            false,
        ));
        breakdown.compute_final_score(0.0);

        assert!(breakdown.is_viable());

        breakdown.vetoed = true;
        assert!(!breakdown.is_viable());
    }

    #[test]
    fn test_goal_score_ordering() {
        let s1 = GoalScore::new(GoalId::satisfy_hunger(), 0.8, 100);
        let s2 = GoalScore::new(GoalId::rest(), 0.6, 100);
        let s3 = GoalScore::new(GoalId::idle(), 0.8, 100);

        assert!(s1 < s2);
        assert!(s3 < s1);
    }

    #[test]
    fn test_goal_score_ordering_deterministic() {
        let s1 = GoalScore::new(GoalId::new("alpha"), 0.5, 100);
        let s2 = GoalScore::new(GoalId::new("beta"), 0.5, 100);

        assert!(s1 < s2);

        let mut scores = [s2.clone(), s1.clone()];
        scores.sort();

        assert_eq!(scores[0].id, GoalId::new("alpha"));
        assert_eq!(scores[1].id, GoalId::new("beta"));
    }

    #[test]
    fn test_scoring_breakdown_serde() {
        let mut breakdown = ScoringBreakdown::new(GoalId::flee_danger());
        breakdown.add_consideration(ConsiderationScore::new(
            "threat".into(),
            0.9,
            0.9,
            1.35,
            false,
        ));
        breakdown.base_priority = 2.0;
        breakdown.inertia_bonus = 0.1;
        breakdown.compute_final_score(0.0);

        let json = serde_json::to_string(&breakdown).unwrap();
        let restored: ScoringBreakdown = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.goal_id, breakdown.goal_id);
        assert!((restored.final_score - breakdown.final_score).abs() < f32::EPSILON);
    }

    #[test]
    fn test_goal_score_serde() {
        let score = GoalScore::new(GoalId::patrol(), 0.42, 500);

        let json = serde_json::to_string(&score).unwrap();
        let restored: GoalScore = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, GoalId::patrol());
        assert!((restored.score - 0.42).abs() < f32::EPSILON);
        assert_eq!(restored.scored_tick, 500);
    }
}
