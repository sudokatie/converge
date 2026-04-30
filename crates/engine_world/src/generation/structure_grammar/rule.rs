//! Grammar rule definitions.

use serde::{Deserialize, Serialize};

use super::id::{RuleId, SymbolId, TemplateId};

/// A weighted choice for rule expansion.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WeightedChoice {
    /// Weight for selection (must be positive).
    pub weight: f32,
    /// Expansion result.
    pub expansion: RuleExpansion,
}

impl WeightedChoice {
    /// Create a new weighted choice.
    #[must_use]
    pub fn new(weight: f32, expansion: RuleExpansion) -> Self {
        Self { weight, expansion }
    }

    /// Create with weight 1.0.
    #[must_use]
    pub fn uniform(expansion: RuleExpansion) -> Self {
        Self {
            weight: 1.0,
            expansion,
        }
    }
}

/// Result of expanding a rule.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RuleExpansion {
    /// Place a template.
    Template {
        /// Template to place.
        template_id: TemplateId,
        /// Socket to use on template (if connecting).
        socket: Option<String>,
        /// Nested symbols to expand from this placement.
        children: Vec<ChildSymbol>,
    },
    /// Expand to another symbol.
    Symbol {
        /// Symbol to expand.
        symbol: SymbolId,
    },
    /// Sequence of expansions.
    Sequence {
        /// Ordered expansions.
        expansions: Vec<RuleExpansion>,
    },
    /// Empty/terminal expansion.
    Empty,
}

impl RuleExpansion {
    /// Create a template expansion.
    #[must_use]
    pub fn template(template_id: TemplateId) -> Self {
        Self::Template {
            template_id,
            socket: None,
            children: Vec::new(),
        }
    }

    /// Create a template expansion with socket.
    #[must_use]
    pub fn template_with_socket(template_id: TemplateId, socket: impl Into<String>) -> Self {
        Self::Template {
            template_id,
            socket: Some(socket.into()),
            children: Vec::new(),
        }
    }

    /// Create a template expansion with children.
    #[must_use]
    pub fn template_with_children(template_id: TemplateId, children: Vec<ChildSymbol>) -> Self {
        Self::Template {
            template_id,
            socket: None,
            children,
        }
    }

    /// Create a symbol expansion.
    #[must_use]
    pub fn symbol(symbol: impl Into<SymbolId>) -> Self {
        Self::Symbol {
            symbol: symbol.into(),
        }
    }

    /// Create a sequence expansion.
    #[must_use]
    pub fn sequence(expansions: Vec<RuleExpansion>) -> Self {
        Self::Sequence { expansions }
    }

    /// Create an empty expansion.
    #[must_use]
    pub const fn empty() -> Self {
        Self::Empty
    }

    /// Check if this is an empty expansion.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Check if this is a terminal expansion (template or empty).
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Template { .. } | Self::Empty)
    }
}

/// A child symbol to expand from a placement.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChildSymbol {
    /// Symbol to expand.
    pub symbol: SymbolId,
    /// Socket on parent to attach to.
    pub socket: String,
    /// Optional transform offset.
    pub offset: [i32; 3],
}

impl ChildSymbol {
    /// Create a new child symbol.
    #[must_use]
    pub fn new(symbol: impl Into<SymbolId>, socket: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            socket: socket.into(),
            offset: [0, 0, 0],
        }
    }

    /// Set offset.
    #[must_use]
    pub fn with_offset(mut self, offset: [i32; 3]) -> Self {
        self.offset = offset;
        self
    }
}

/// A grammar rule for expanding symbols.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GrammarRule {
    /// Rule identifier.
    pub id: RuleId,
    /// Symbol this rule expands.
    pub symbol: SymbolId,
    /// Weighted choices for expansion.
    pub choices: Vec<WeightedChoice>,
    /// Maximum depth this rule can be applied.
    pub max_depth: u32,
    /// Maximum times this rule can fire per generation.
    pub max_applications: u32,
    /// Priority for rule selection (higher = first).
    pub priority: i32,
    /// Condition tags required for this rule.
    pub condition_tags: Vec<String>,
}

impl GrammarRule {
    /// Create a new rule with a single expansion.
    #[must_use]
    pub fn new(id: RuleId, symbol: impl Into<SymbolId>, expansion: RuleExpansion) -> Self {
        Self {
            id,
            symbol: symbol.into(),
            choices: vec![WeightedChoice::uniform(expansion)],
            max_depth: u32::MAX,
            max_applications: u32::MAX,
            priority: 0,
            condition_tags: Vec::new(),
        }
    }

    /// Create a rule with weighted choices.
    #[must_use]
    pub fn with_choices(
        id: RuleId,
        symbol: impl Into<SymbolId>,
        choices: Vec<WeightedChoice>,
    ) -> Self {
        Self {
            id,
            symbol: symbol.into(),
            choices,
            max_depth: u32::MAX,
            max_applications: u32::MAX,
            priority: 0,
            condition_tags: Vec::new(),
        }
    }

    /// Set max depth.
    #[must_use]
    pub fn with_max_depth(mut self, depth: u32) -> Self {
        self.max_depth = depth;
        self
    }

    /// Set max applications.
    #[must_use]
    pub fn with_max_applications(mut self, count: u32) -> Self {
        self.max_applications = count;
        self
    }

    /// Set priority.
    #[must_use]
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Add condition tag.
    #[must_use]
    pub fn with_condition(mut self, tag: impl Into<String>) -> Self {
        self.condition_tags.push(tag.into());
        self
    }

    /// Get total weight of all choices.
    #[must_use]
    pub fn total_weight(&self) -> f32 {
        self.choices.iter().map(|c| c.weight).sum()
    }

    /// Check if rule is applicable at given depth.
    #[must_use]
    pub fn applicable_at_depth(&self, depth: u32) -> bool {
        depth <= self.max_depth
    }

    /// Check if rule has valid weights.
    #[must_use]
    pub fn has_valid_weights(&self) -> bool {
        !self.choices.is_empty()
            && self
                .choices
                .iter()
                .all(|c| c.weight > 0.0 && c.weight.is_finite())
    }

    /// Select a choice deterministically given a value in [0, 1).
    #[must_use]
    pub fn select_choice(&self, t: f32) -> Option<&RuleExpansion> {
        if self.choices.is_empty() {
            return None;
        }

        let total = self.total_weight();
        if total <= 0.0 {
            return None;
        }

        let target = t * total;
        let mut cumulative = 0.0;

        for choice in &self.choices {
            cumulative += choice.weight;
            if target < cumulative {
                return Some(&choice.expansion);
            }
        }

        self.choices.last().map(|c| &c.expansion)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_choice_creation() {
        let choice = WeightedChoice::new(2.0, RuleExpansion::template(TemplateId::new(1)));
        assert!((choice.weight - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn expansion_variants() {
        let template = RuleExpansion::template(TemplateId::new(1));
        assert!(template.is_terminal());

        let symbol = RuleExpansion::symbol("Room");
        assert!(!symbol.is_terminal());

        let empty = RuleExpansion::empty();
        assert!(empty.is_empty());
        assert!(empty.is_terminal());
    }

    #[test]
    fn child_symbol() {
        let child = ChildSymbol::new("Corridor", "north_exit").with_offset([0, 0, 5]);
        assert_eq!(child.symbol.name(), "Corridor");
        assert_eq!(child.socket, "north_exit");
        assert_eq!(child.offset, [0, 0, 5]);
    }

    #[test]
    fn rule_creation() {
        let rule = GrammarRule::new(
            RuleId::new(1),
            "Room",
            RuleExpansion::template(TemplateId::new(10)),
        )
        .with_max_depth(5)
        .with_priority(10);

        assert_eq!(rule.symbol.name(), "Room");
        assert_eq!(rule.max_depth, 5);
        assert_eq!(rule.priority, 10);
    }

    #[test]
    fn rule_weighted_selection() {
        let rule = GrammarRule::with_choices(
            RuleId::new(1),
            "Test",
            vec![
                WeightedChoice::new(1.0, RuleExpansion::template(TemplateId::new(1))),
                WeightedChoice::new(2.0, RuleExpansion::template(TemplateId::new(2))),
                WeightedChoice::new(1.0, RuleExpansion::template(TemplateId::new(3))),
            ],
        );

        assert!((rule.total_weight() - 4.0).abs() < f32::EPSILON);

        let choice0 = rule.select_choice(0.0).unwrap();
        if let RuleExpansion::Template { template_id, .. } = choice0 {
            assert_eq!(*template_id, TemplateId::new(1));
        }

        let choice_mid = rule.select_choice(0.5).unwrap();
        if let RuleExpansion::Template { template_id, .. } = choice_mid {
            assert_eq!(*template_id, TemplateId::new(2));
        }

        let choice_end = rule.select_choice(0.99).unwrap();
        if let RuleExpansion::Template { template_id, .. } = choice_end {
            assert_eq!(*template_id, TemplateId::new(3));
        }
    }

    #[test]
    fn rule_depth_check() {
        let rule =
            GrammarRule::new(RuleId::new(1), "Test", RuleExpansion::empty()).with_max_depth(3);

        assert!(rule.applicable_at_depth(0));
        assert!(rule.applicable_at_depth(3));
        assert!(!rule.applicable_at_depth(4));
    }

    #[test]
    fn rule_valid_weights() {
        let valid = GrammarRule::with_choices(
            RuleId::new(1),
            "Test",
            vec![WeightedChoice::new(1.0, RuleExpansion::empty())],
        );
        assert!(valid.has_valid_weights());

        let invalid_zero = GrammarRule::with_choices(
            RuleId::new(2),
            "Test",
            vec![WeightedChoice::new(0.0, RuleExpansion::empty())],
        );
        assert!(!invalid_zero.has_valid_weights());

        let empty = GrammarRule::with_choices(RuleId::new(3), "Test", vec![]);
        assert!(!empty.has_valid_weights());
    }

    #[test]
    fn serde_roundtrip() {
        let rule = GrammarRule::with_choices(
            RuleId::new(1),
            "Room",
            vec![
                WeightedChoice::new(1.0, RuleExpansion::template(TemplateId::new(1))),
                WeightedChoice::new(2.0, RuleExpansion::symbol("SubRoom")),
            ],
        )
        .with_max_depth(5);

        let json = serde_json::to_string(&rule).unwrap();
        let recovered: GrammarRule = serde_json::from_str(&json).unwrap();
        assert_eq!(rule, recovered);
    }
}
