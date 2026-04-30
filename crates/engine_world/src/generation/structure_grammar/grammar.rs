//! Structure grammar definition and validation.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::id::{RuleId, SymbolId, TemplateId};
use super::rule::{GrammarRule, RuleExpansion};
use super::template::StructureTemplate;
use super::validation::{ValidationError, ValidationErrors};

/// A structure grammar containing templates and expansion rules.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StructureGrammar {
    /// Templates indexed by ID.
    templates: BTreeMap<TemplateId, StructureTemplate>,
    /// Rules indexed by ID.
    rules: BTreeMap<RuleId, GrammarRule>,
    /// Rules indexed by symbol.
    rules_by_symbol: BTreeMap<SymbolId, Vec<RuleId>>,
    /// Root symbol for generation.
    root_symbol: Option<SymbolId>,
}

impl StructureGrammar {
    /// Create a new empty grammar.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the root symbol.
    pub fn set_root(&mut self, symbol: impl Into<SymbolId>) {
        self.root_symbol = Some(symbol.into());
    }

    /// Get the root symbol.
    #[must_use]
    pub fn root_symbol(&self) -> Option<&SymbolId> {
        self.root_symbol.as_ref()
    }

    /// Add a template.
    ///
    /// # Errors
    ///
    /// Returns error if template ID is already registered.
    pub fn add_template(&mut self, template: StructureTemplate) -> Result<(), ValidationError> {
        if self.templates.contains_key(&template.id) {
            return Err(ValidationError::DuplicateTemplateId(template.id));
        }
        self.templates.insert(template.id, template);
        Ok(())
    }

    /// Add a rule.
    ///
    /// # Errors
    ///
    /// Returns error if rule ID is already registered.
    pub fn add_rule(&mut self, rule: GrammarRule) -> Result<(), ValidationError> {
        if self.rules.contains_key(&rule.id) {
            return Err(ValidationError::DuplicateRuleId(rule.id));
        }
        let symbol = rule.symbol.clone();
        let rule_id = rule.id;
        self.rules.insert(rule_id, rule);
        self.rules_by_symbol
            .entry(symbol)
            .or_default()
            .push(rule_id);
        Ok(())
    }

    /// Get a template by ID.
    #[must_use]
    pub fn template(&self, id: TemplateId) -> Option<&StructureTemplate> {
        self.templates.get(&id)
    }

    /// Get a rule by ID.
    #[must_use]
    pub fn rule(&self, id: RuleId) -> Option<&GrammarRule> {
        self.rules.get(&id)
    }

    /// Get rules for a symbol.
    #[must_use]
    pub fn rules_for_symbol(&self, symbol: &SymbolId) -> Vec<&GrammarRule> {
        self.rules_by_symbol
            .get(symbol)
            .map(|ids| ids.iter().filter_map(|id| self.rules.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all templates.
    pub fn templates(&self) -> impl Iterator<Item = &StructureTemplate> {
        self.templates.values()
    }

    /// Get all rules.
    pub fn rules(&self) -> impl Iterator<Item = &GrammarRule> {
        self.rules.values()
    }

    /// Get template count.
    #[must_use]
    pub fn template_count(&self) -> usize {
        self.templates.len()
    }

    /// Get rule count.
    #[must_use]
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Get all defined symbols.
    #[must_use]
    pub fn symbols(&self) -> BTreeSet<SymbolId> {
        self.rules_by_symbol.keys().cloned().collect()
    }

    /// Check if a symbol is defined.
    #[must_use]
    pub fn has_symbol(&self, symbol: &SymbolId) -> bool {
        self.rules_by_symbol.contains_key(symbol)
    }

    /// Validate the grammar.
    pub fn validate(&self) -> ValidationErrors {
        let mut errors = ValidationErrors::new();

        if self.rules.is_empty() {
            errors.add(ValidationError::EmptyGrammar);
        }

        if let Some(ref root) = self.root_symbol {
            if !self.has_symbol(root) {
                errors.add(ValidationError::MissingSymbol(root.clone()));
            }
        } else {
            errors.add(ValidationError::NoRootSymbol);
        }

        self.validate_templates(&mut errors);
        self.validate_rules(&mut errors);
        self.validate_references(&mut errors);
        self.detect_recursion(&mut errors);

        errors
    }

    fn validate_templates(&self, errors: &mut ValidationErrors) {
        for template in self.templates.values() {
            let bounds = &template.bounds;
            if bounds.min[0] > bounds.max[0]
                || bounds.min[1] > bounds.max[1]
                || bounds.min[2] > bounds.max[2]
            {
                errors.add(ValidationError::InvalidBounds {
                    template_id: template.id,
                    message: "min > max".to_string(),
                });
            }

            for socket in &template.sockets {
                if !bounds.contains(socket.position) {
                    errors.add(ValidationError::InvalidBounds {
                        template_id: template.id,
                        message: format!("socket '{}' outside bounds", socket.name),
                    });
                }
            }

            for anchor in &template.anchors {
                if !bounds.contains(anchor.position) {
                    errors.add(ValidationError::InvalidBounds {
                        template_id: template.id,
                        message: format!("anchor '{}' outside bounds", anchor.name),
                    });
                }
            }

            if template.rules.weight <= 0.0 || !template.rules.weight.is_finite() {
                errors.add(ValidationError::InvalidWeight {
                    rule_id: RuleId::new(0),
                    message: format!(
                        "template {} has invalid weight: {}",
                        template.id, template.rules.weight
                    ),
                });
            }
        }
    }

    fn validate_rules(&self, errors: &mut ValidationErrors) {
        for rule in self.rules.values() {
            if !rule.has_valid_weights() {
                errors.add(ValidationError::InvalidWeight {
                    rule_id: rule.id,
                    message: "zero, negative, or non-finite weight".to_string(),
                });
            }

            for choice in &rule.choices {
                self.validate_expansion(&choice.expansion, errors);
            }
        }
    }

    fn validate_expansion(&self, expansion: &RuleExpansion, errors: &mut ValidationErrors) {
        match expansion {
            RuleExpansion::Template {
                template_id,
                socket,
                children,
            } => {
                if !self.templates.contains_key(template_id) {
                    errors.add(ValidationError::MissingTemplate(*template_id));
                } else if let Some(socket_name) = socket {
                    let template = &self.templates[template_id];
                    if template.socket(socket_name).is_none() {
                        errors.add(ValidationError::InvalidSocket {
                            template_id: *template_id,
                            socket_name: socket_name.clone(),
                        });
                    }
                }

                for child in children {
                    if let Some(template) = self.templates.get(template_id)
                        && template.socket(&child.socket).is_none()
                    {
                        errors.add(ValidationError::InvalidSocket {
                            template_id: *template_id,
                            socket_name: child.socket.clone(),
                        });
                    }
                }
            }
            RuleExpansion::Sequence { expansions } => {
                for exp in expansions {
                    self.validate_expansion(exp, errors);
                }
            }
            RuleExpansion::Symbol { .. } | RuleExpansion::Empty => {}
        }
    }

    fn validate_references(&self, errors: &mut ValidationErrors) {
        let mut referenced_symbols = BTreeSet::new();
        self.collect_referenced_symbols(&mut referenced_symbols);

        for symbol in &referenced_symbols {
            if !self.has_symbol(symbol) && symbol.is_non_terminal() {
                errors.add(ValidationError::MissingSymbol(symbol.clone()));
            }
        }
    }

    fn collect_referenced_symbols(&self, symbols: &mut BTreeSet<SymbolId>) {
        for rule in self.rules.values() {
            for choice in &rule.choices {
                Self::collect_symbols_from_expansion(&choice.expansion, symbols);
            }
        }
    }

    fn collect_symbols_from_expansion(expansion: &RuleExpansion, symbols: &mut BTreeSet<SymbolId>) {
        match expansion {
            RuleExpansion::Symbol { symbol } => {
                symbols.insert(symbol.clone());
            }
            RuleExpansion::Template { children, .. } => {
                for child in children {
                    symbols.insert(child.symbol.clone());
                }
            }
            RuleExpansion::Sequence { expansions } => {
                for exp in expansions {
                    Self::collect_symbols_from_expansion(exp, symbols);
                }
            }
            RuleExpansion::Empty => {}
        }
    }

    fn detect_recursion(&self, errors: &mut ValidationErrors) {
        for symbol in self.rules_by_symbol.keys() {
            let mut visited = BTreeSet::new();
            let mut path = Vec::new();
            if self.is_potentially_recursive(symbol, &mut visited, &mut path) {
                path.push(symbol.clone());
                path.reverse();
                errors.add(ValidationError::PotentialRecursion { symbols: path });
                break;
            }
        }
    }

    fn is_potentially_recursive(
        &self,
        symbol: &SymbolId,
        visited: &mut BTreeSet<SymbolId>,
        path: &mut Vec<SymbolId>,
    ) -> bool {
        if visited.contains(symbol) {
            return true;
        }

        visited.insert(symbol.clone());
        path.push(symbol.clone());

        let rules = self.rules_for_symbol(symbol);
        let all_recursive = !rules.is_empty()
            && rules.iter().all(|rule| {
                rule.choices.iter().all(|choice| {
                    self.expansion_leads_to_recursion(&choice.expansion, visited, path)
                })
            });

        path.pop();
        visited.remove(symbol);

        all_recursive
    }

    fn expansion_leads_to_recursion(
        &self,
        expansion: &RuleExpansion,
        visited: &mut BTreeSet<SymbolId>,
        path: &mut Vec<SymbolId>,
    ) -> bool {
        match expansion {
            RuleExpansion::Symbol { symbol } => {
                self.is_potentially_recursive(symbol, visited, path)
            }
            RuleExpansion::Template { children, .. } => {
                !children.is_empty()
                    && children
                        .iter()
                        .all(|child| self.is_potentially_recursive(&child.symbol, visited, path))
            }
            RuleExpansion::Sequence { expansions } => {
                !expansions.is_empty()
                    && expansions
                        .iter()
                        .all(|exp| self.expansion_leads_to_recursion(exp, visited, path))
            }
            RuleExpansion::Empty => false,
        }
    }
}

/// Builder for constructing grammars fluently.
#[derive(Clone, Debug, Default)]
pub struct GrammarBuilder {
    grammar: StructureGrammar,
    next_template_id: u64,
    next_rule_id: u64,
}

impl GrammarBuilder {
    /// Create a new builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set root symbol.
    #[must_use]
    pub fn root(mut self, symbol: impl Into<SymbolId>) -> Self {
        self.grammar.set_root(symbol);
        self
    }

    /// Add a template, auto-assigning ID.
    #[must_use]
    pub fn template(mut self, template: StructureTemplate) -> Self {
        let _ = self.grammar.add_template(template);
        self
    }

    /// Add a template with auto-generated ID.
    #[must_use]
    pub fn add_template(
        mut self,
        name: impl Into<String>,
        bounds: super::template::Bounds,
        build: impl FnOnce(StructureTemplate) -> StructureTemplate,
    ) -> Self {
        let id = TemplateId::new(self.next_template_id);
        self.next_template_id += 1;
        let template = build(StructureTemplate::new(id, name, bounds));
        let _ = self.grammar.add_template(template);
        self
    }

    /// Add a rule.
    #[must_use]
    pub fn rule(mut self, rule: GrammarRule) -> Self {
        let _ = self.grammar.add_rule(rule);
        self
    }

    /// Add a rule with auto-generated ID.
    #[must_use]
    pub fn add_rule(mut self, symbol: impl Into<SymbolId>, expansion: RuleExpansion) -> Self {
        let id = RuleId::new(self.next_rule_id);
        self.next_rule_id += 1;
        let rule = GrammarRule::new(id, symbol, expansion);
        let _ = self.grammar.add_rule(rule);
        self
    }

    /// Build the grammar.
    #[must_use]
    pub fn build(self) -> StructureGrammar {
        self.grammar
    }

    /// Build and validate.
    ///
    /// # Errors
    ///
    /// Returns validation errors if the grammar is invalid.
    pub fn build_validated(self) -> Result<StructureGrammar, ValidationErrors> {
        let grammar = self.build();
        grammar.validate().into_result()?;
        Ok(grammar)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generation::structure_grammar::template::Bounds;

    fn simple_template(id: u64, name: &str) -> StructureTemplate {
        StructureTemplate::new(TemplateId::new(id), name, Bounds::from_size(5, 5, 5))
    }

    #[test]
    fn grammar_creation() {
        let mut grammar = StructureGrammar::new();
        grammar.set_root("Start");

        grammar.add_template(simple_template(1, "room")).unwrap();
        grammar
            .add_rule(GrammarRule::new(
                RuleId::new(1),
                "Start",
                RuleExpansion::template(TemplateId::new(1)),
            ))
            .unwrap();

        assert_eq!(grammar.template_count(), 1);
        assert_eq!(grammar.rule_count(), 1);
        assert!(grammar.has_symbol(&SymbolId::new("Start")));
    }

    #[test]
    fn grammar_duplicate_ids() {
        let mut grammar = StructureGrammar::new();

        grammar.add_template(simple_template(1, "a")).unwrap();
        let err = grammar.add_template(simple_template(1, "b")).unwrap_err();
        assert!(matches!(err, ValidationError::DuplicateTemplateId(_)));

        grammar
            .add_rule(GrammarRule::new(
                RuleId::new(1),
                "A",
                RuleExpansion::empty(),
            ))
            .unwrap();
        let err = grammar
            .add_rule(GrammarRule::new(
                RuleId::new(1),
                "B",
                RuleExpansion::empty(),
            ))
            .unwrap_err();
        assert!(matches!(err, ValidationError::DuplicateRuleId(_)));
    }

    #[test]
    fn grammar_validation_empty() {
        let grammar = StructureGrammar::new();
        let errors = grammar.validate();

        assert!(!errors.is_empty());
        let has_empty = errors
            .errors()
            .iter()
            .any(|e| matches!(e, ValidationError::EmptyGrammar));
        let has_no_root = errors
            .errors()
            .iter()
            .any(|e| matches!(e, ValidationError::NoRootSymbol));
        assert!(has_empty);
        assert!(has_no_root);
    }

    #[test]
    fn grammar_validation_missing_template() {
        let mut grammar = StructureGrammar::new();
        grammar.set_root("Start");
        grammar
            .add_rule(GrammarRule::new(
                RuleId::new(1),
                "Start",
                RuleExpansion::template(TemplateId::new(999)),
            ))
            .unwrap();

        let errors = grammar.validate();
        let has_missing = errors.errors().iter().any(
            |e| matches!(e, ValidationError::MissingTemplate(id) if *id == TemplateId::new(999)),
        );
        assert!(has_missing);
    }

    #[test]
    fn grammar_validation_missing_symbol() {
        let mut grammar = StructureGrammar::new();
        grammar.set_root("Start");
        grammar.add_template(simple_template(1, "room")).unwrap();
        grammar
            .add_rule(GrammarRule::new(
                RuleId::new(1),
                "Start",
                RuleExpansion::symbol("UndefinedSymbol"),
            ))
            .unwrap();

        let errors = grammar.validate();
        let has_missing = errors.errors().iter().any(
            |e| matches!(e, ValidationError::MissingSymbol(s) if s.name() == "UndefinedSymbol"),
        );
        assert!(has_missing);
    }

    #[test]
    fn grammar_builder() {
        let grammar = GrammarBuilder::new()
            .root("Start")
            .add_template("room", Bounds::from_size(10, 5, 10), |t| {
                t.with_tag("interior")
            })
            .add_rule("Start", RuleExpansion::template(TemplateId::new(0)))
            .build();

        assert_eq!(grammar.template_count(), 1);
        assert_eq!(grammar.rule_count(), 1);
        assert!(
            grammar
                .template(TemplateId::new(0))
                .unwrap()
                .has_tag("interior")
        );
    }

    #[test]
    fn grammar_rules_for_symbol() {
        let mut grammar = StructureGrammar::new();
        grammar
            .add_rule(GrammarRule::new(
                RuleId::new(1),
                "Room",
                RuleExpansion::empty(),
            ))
            .unwrap();
        grammar
            .add_rule(GrammarRule::new(
                RuleId::new(2),
                "Room",
                RuleExpansion::empty(),
            ))
            .unwrap();
        grammar
            .add_rule(GrammarRule::new(
                RuleId::new(3),
                "Other",
                RuleExpansion::empty(),
            ))
            .unwrap();

        let room_rules = grammar.rules_for_symbol(&SymbolId::new("Room"));
        assert_eq!(room_rules.len(), 2);
    }

    #[test]
    fn serde_roundtrip() {
        let grammar = GrammarBuilder::new()
            .root("Start")
            .add_template("room", Bounds::from_size(5, 5, 5), |t| t)
            .add_rule("Start", RuleExpansion::template(TemplateId::new(0)))
            .build();

        let json = serde_json::to_string(&grammar).unwrap();
        let recovered: StructureGrammar = serde_json::from_str(&json).unwrap();

        assert_eq!(grammar.template_count(), recovered.template_count());
        assert_eq!(grammar.rule_count(), recovered.rule_count());
    }
}
