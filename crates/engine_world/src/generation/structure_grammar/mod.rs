//! Template and grammar-based structure generation.
//!
//! This module provides a data-driven system for generating procedural structures
//! using templates and grammar rules. Key features:
//!
//! - Reusable structure templates with IDs, bounds, sockets, and metadata
//! - Grammar rules for deterministic expansion of symbols into template placements
//! - Weighted choices for varied generation outcomes
//! - Depth and step limits to prevent infinite recursion
//! - Query APIs for inspecting generated layouts
//! - Fingerprinting for verifying layout consistency
//!
//! # Example
//!
//! ```ignore
//! use engine_world::generation::structure_grammar::*;
//!
//! let grammar = GrammarBuilder::new()
//!     .root("Dungeon")
//!     .add_template("room", Bounds::from_size(10, 5, 10), |t| {
//!         t.with_kind(TemplateKind::Room)
//!             .with_socket(Socket::new("exit", [5, 0, 9], Direction::North))
//!     })
//!     .add_rule("Dungeon", RuleExpansion::template(TemplateId::new(0)))
//!     .build();
//!
//! let layout = generate_with_seed(&grammar, 42)?;
//! ```

mod context;
mod fingerprint;
mod generator;
mod grammar;
mod id;
mod layout;
mod query;
mod rule;
mod template;
mod validation;

pub use context::{GenerationConfig, GenerationContext};
pub use fingerprint::{LayoutChecksum, LayoutFingerprint, LayoutFingerprintBuilder};
pub use generator::{GenerationResult, StructureGenerator, generate, generate_with_seed};
pub use grammar::{GrammarBuilder, StructureGrammar};
pub use id::{PlacementId, RuleId, SymbolId, TemplateId};
pub use layout::{ConnectorSummary, GeneratedLayout, LayoutSummary, Placement};
pub use query::{LayoutQuery, LayoutQueryResult};
pub use rule::{ChildSymbol, GrammarRule, RuleExpansion, WeightedChoice};
pub use template::{
    Anchor, BlockPalette, BlockType, Bounds, Direction, PlacementRules, Socket, StructureTemplate,
    TemplateKind,
};
pub use validation::{ValidationError, ValidationErrors};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_exports_available() {
        let _ = TemplateId::new(1);
        let _ = RuleId::new(1);
        let _ = SymbolId::new("Test");
        let _ = PlacementId::new(1);
        let _ = Bounds::from_size(5, 5, 5);
        let _ = TemplateKind::Room;
        let _ = Direction::North;
        let _ = GenerationConfig::new(42);
    }

    #[test]
    fn basic_workflow() {
        let template = StructureTemplate::new(
            TemplateId::new(0),
            "test_room",
            Bounds::from_size(10, 5, 10),
        )
        .with_kind(TemplateKind::Room)
        .with_tag("start")
        .with_socket(Socket::new("exit", [5, 0, 9], Direction::North));

        let rule = GrammarRule::new(
            RuleId::new(0),
            "Start",
            RuleExpansion::template(TemplateId::new(0)),
        );

        let mut grammar = StructureGrammar::new();
        grammar.set_root("Start");
        grammar.add_template(template).unwrap();
        grammar.add_rule(rule).unwrap();

        let errors = grammar.validate();
        assert!(errors.is_empty());

        let layout = generate_with_seed(&grammar, 42).unwrap();
        assert_eq!(layout.placement_count(), 1);

        let fp = layout.fingerprint();
        assert_ne!(fp.value(), 0);
    }

    #[test]
    fn builder_workflow() {
        let grammar = GrammarBuilder::new()
            .root("Start")
            .add_template("room", Bounds::from_size(10, 5, 10), |t| {
                t.with_kind(TemplateKind::Room)
            })
            .add_rule("Start", RuleExpansion::template(TemplateId::new(0)))
            .build();

        let layout = generate_with_seed(&grammar, 42).unwrap();
        assert_eq!(layout.placement_count(), 1);
    }

    #[test]
    fn query_workflow() {
        let grammar = GrammarBuilder::new()
            .root("Start")
            .add_template("room", Bounds::from_size(10, 5, 10), |t| {
                t.with_kind(TemplateKind::Room).with_tag("main")
            })
            .add_rule("Start", RuleExpansion::template(TemplateId::new(0)))
            .build();

        let layout = generate_with_seed(&grammar, 42).unwrap();

        let query = layout.query();
        let by_kind = query.by_kind(TemplateKind::Room);
        assert_eq!(by_kind.len(), 1);

        let by_tag = query.by_tag("main");
        assert_eq!(by_tag.len(), 1);

        let summary = layout.summary();
        assert_eq!(summary.placement_count, 1);
    }

    #[test]
    fn serde_workflow() {
        let grammar = GrammarBuilder::new()
            .root("Start")
            .add_template("room", Bounds::from_size(10, 5, 10), |t| {
                t.with_kind(TemplateKind::Room)
            })
            .add_rule("Start", RuleExpansion::template(TemplateId::new(0)))
            .build();

        let json = serde_json::to_string(&grammar).unwrap();
        let recovered: StructureGrammar = serde_json::from_str(&json).unwrap();
        assert_eq!(grammar.template_count(), recovered.template_count());

        let layout = generate_with_seed(&grammar, 42).unwrap();
        let layout_json = serde_json::to_string(&layout).unwrap();
        let recovered_layout: GeneratedLayout = serde_json::from_str(&layout_json).unwrap();
        assert!(
            layout
                .fingerprint()
                .matches(&recovered_layout.fingerprint())
        );
    }

    #[test]
    fn determinism_verification() {
        let grammar = GrammarBuilder::new()
            .root("Start")
            .add_template("room", Bounds::from_size(10, 5, 10), |t| {
                t.with_kind(TemplateKind::Room).with_socket(Socket::new(
                    "exit",
                    [5, 0, 9],
                    Direction::North,
                ))
            })
            .add_rule(
                "Start",
                RuleExpansion::template_with_children(
                    TemplateId::new(0),
                    vec![ChildSymbol::new("Child", "exit")],
                ),
            )
            .add_rule("Child", RuleExpansion::empty())
            .build();

        let layout1 = generate_with_seed(&grammar, 99999).unwrap();
        let layout2 = generate_with_seed(&grammar, 99999).unwrap();

        assert!(layout1.fingerprint().matches(&layout2.fingerprint()));
        assert!(layout1.checksum().matches(&layout2.checksum()));
    }
}
