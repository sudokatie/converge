//! Deterministic structure generator.

use std::collections::BTreeMap;

use super::context::{GenerationConfig, GenerationContext};
use super::grammar::StructureGrammar;
use super::id::{PlacementId, SymbolId, TemplateId};
use super::layout::{ConnectorSummary, GeneratedLayout, Placement};
use super::rule::{ChildSymbol, RuleExpansion};
use super::template::{Socket, StructureTemplate};
use super::validation::{ValidationError, ValidationErrors};

/// Result of a generation operation.
pub type GenerationResult = Result<GeneratedLayout, ValidationErrors>;

/// Deterministic structure generator.
pub struct StructureGenerator<'a> {
    grammar: &'a StructureGrammar,
    config: GenerationConfig,
    context: GenerationContext,
    layout: GeneratedLayout,
    next_placement_id: u64,
    rule_applications: BTreeMap<super::id::RuleId, u32>,
}

impl<'a> StructureGenerator<'a> {
    /// Create a new generator.
    #[must_use]
    pub fn new(grammar: &'a StructureGrammar, config: GenerationConfig) -> Self {
        let seed = config.seed;
        Self {
            grammar,
            context: GenerationContext::new(config.clone()),
            config,
            layout: GeneratedLayout::new(seed),
            next_placement_id: 0,
            rule_applications: BTreeMap::new(),
        }
    }

    /// Generate a layout from the grammar.
    ///
    /// # Errors
    ///
    /// Returns validation errors if generation fails.
    pub fn generate(mut self) -> GenerationResult {
        let mut errors = self.grammar.validate();
        if !errors.is_empty() {
            return Err(errors);
        }

        let Some(root_symbol) = self.grammar.root_symbol() else {
            errors.add(ValidationError::NoRootSymbol);
            return Err(errors);
        };

        let result = self.expand_symbol(root_symbol.clone(), None, None, [0, 0, 0]);
        if let Err(e) = result {
            errors.add(e);
            return Err(errors);
        }

        self.layout.total_steps = self.context.steps;
        Ok(self.layout)
    }

    fn expand_symbol(
        &mut self,
        symbol: SymbolId,
        parent: Option<PlacementId>,
        parent_socket: Option<&Socket>,
        position: [i32; 3],
    ) -> Result<(), ValidationError> {
        if self.context.depth_exceeded() {
            return Err(ValidationError::MaxDepthExceeded {
                depth: self.context.depth,
                max_depth: self.config.max_depth,
            });
        }

        if self.context.steps_exceeded() {
            return Err(ValidationError::MaxStepsExceeded {
                steps: self.context.steps,
                max_steps: self.config.max_steps,
            });
        }

        self.context.step();

        if symbol.is_terminal() {
            return Ok(());
        }

        let Some((rule_id, max_applications, expansion)) = self.select_and_expand_rule(&symbol)
        else {
            return Err(ValidationError::MissingSymbol(symbol));
        };

        let count = self.rule_applications.entry(rule_id).or_insert(0);
        *count += 1;
        if *count > max_applications {
            return Ok(());
        }

        self.expand(expansion, parent, parent_socket, position)
    }

    fn select_and_expand_rule(
        &mut self,
        symbol: &SymbolId,
    ) -> Option<(super::id::RuleId, u32, RuleExpansion)> {
        let depth = self.context.depth;
        let mut rules: Vec<_> = self
            .grammar
            .rules_for_symbol(symbol)
            .into_iter()
            .filter(|r| r.applicable_at_depth(depth))
            .filter(|r| r.condition_tags.iter().all(|tag| self.context.has_tag(tag)))
            .collect();

        if rules.is_empty() {
            return None;
        }

        rules.sort_by(|a, b| b.priority.cmp(&a.priority));

        let rule = if rules.len() == 1 {
            rules[0]
        } else {
            let max_priority = rules[0].priority;
            let top_rules: Vec<_> = rules
                .iter()
                .filter(|r| r.priority == max_priority)
                .copied()
                .collect();

            if top_rules.len() == 1 {
                top_rules[0]
            } else {
                let total_weight: f32 = top_rules.iter().map(|r| r.total_weight()).sum();
                let target = self.context.next_f32() * total_weight;
                let mut cumulative = 0.0;
                let mut selected = top_rules.last().unwrap();

                for r in &top_rules {
                    cumulative += r.total_weight();
                    if target < cumulative {
                        selected = r;
                        break;
                    }
                }
                selected
            }
        };

        let rule_id = rule.id;
        let max_applications = rule.max_applications;

        let t = self.context.next_f32();
        let expansion = rule.select_choice(t).cloned();

        expansion.map(|e| (rule_id, max_applications, e))
    }

    fn expand(
        &mut self,
        expansion: RuleExpansion,
        parent: Option<PlacementId>,
        parent_socket: Option<&Socket>,
        position: [i32; 3],
    ) -> Result<(), ValidationError> {
        match expansion {
            RuleExpansion::Template {
                template_id,
                socket,
                children,
            } => self.place_template(
                template_id,
                socket.as_deref(),
                &children,
                parent,
                parent_socket,
                position,
            ),
            RuleExpansion::Symbol { symbol } => {
                self.context.enter();
                let result = self.expand_symbol(symbol, parent, parent_socket, position);
                self.context.exit();
                result
            }
            RuleExpansion::Sequence { expansions } => {
                let mut current_pos = position;
                for exp in expansions {
                    self.expand(exp, parent, parent_socket, current_pos)?;
                    current_pos[0] += 10;
                }
                Ok(())
            }
            RuleExpansion::Empty => Ok(()),
        }
    }

    fn place_template(
        &mut self,
        template_id: TemplateId,
        socket_name: Option<&str>,
        children: &[ChildSymbol],
        parent: Option<PlacementId>,
        parent_socket: Option<&Socket>,
        base_position: [i32; 3],
    ) -> Result<(), ValidationError> {
        let template = self
            .grammar
            .template(template_id)
            .ok_or(ValidationError::MissingTemplate(template_id))?;

        let position = if let (Some(socket_name), Some(parent_sock)) = (socket_name, parent_socket)
        {
            let our_socket =
                template
                    .socket(socket_name)
                    .ok_or_else(|| ValidationError::InvalidSocket {
                        template_id,
                        socket_name: socket_name.to_owned(),
                    })?;

            Self::calculate_connected_position(base_position, parent_sock, our_socket)
        } else {
            base_position
        };

        let world_bounds = template.bounds.translate(position);

        if !self.config.allow_overlap && self.layout.would_overlap(&world_bounds) {
            return Ok(());
        }

        if !self.context.bounds_fit(&world_bounds) {
            return Ok(());
        }

        let placement_id = PlacementId::new(self.next_placement_id);
        self.next_placement_id += 1;

        let placement = Placement::new(placement_id, template_id, position, world_bounds)
            .with_kind(template.kind)
            .with_depth(self.context.depth)
            .with_tags(template.tags.clone());

        let placement = if let Some(pid) = parent {
            placement.with_parent(pid)
        } else {
            placement
        };

        let placement = if let Some(s) = socket_name {
            placement.with_socket(s)
        } else {
            placement
        };

        self.add_connectors(placement_id, template, position);
        self.layout.add_placement(placement);

        if self.layout.max_depth < self.context.depth {
            self.layout.max_depth = self.context.depth;
        }

        self.context.enter();
        for child in children {
            let child_socket = template.socket(&child.socket);
            let child_pos = if let Some(sock) = child_socket {
                [
                    position[0] + sock.position[0] + child.offset[0],
                    position[1] + sock.position[1] + child.offset[1],
                    position[2] + sock.position[2] + child.offset[2],
                ]
            } else {
                [
                    position[0] + child.offset[0],
                    position[1] + child.offset[1],
                    position[2] + child.offset[2],
                ]
            };

            self.expand_symbol(
                child.symbol.clone(),
                Some(placement_id),
                child_socket,
                child_pos,
            )?;
        }
        self.context.exit();

        Ok(())
    }

    fn calculate_connected_position(
        parent_socket_world: [i32; 3],
        parent_socket: &Socket,
        our_socket: &Socket,
    ) -> [i32; 3] {
        let dir_vec = parent_socket.direction.vector();

        let offset_x = parent_socket_world[0] + dir_vec[0] - our_socket.position[0];
        let offset_y = parent_socket_world[1] + dir_vec[1] - our_socket.position[1];
        let offset_z = parent_socket_world[2] + dir_vec[2] - our_socket.position[2];

        [offset_x, offset_y, offset_z]
    }

    fn add_connectors(
        &mut self,
        placement_id: PlacementId,
        template: &StructureTemplate,
        position: [i32; 3],
    ) {
        for socket in &template.sockets {
            let world_pos = [
                position[0] + socket.position[0],
                position[1] + socket.position[1],
                position[2] + socket.position[2],
            ];

            self.layout.add_connector(ConnectorSummary {
                placement_id,
                socket_name: socket.name.clone(),
                socket_type: socket.socket_type.clone(),
                world_position: world_pos,
                connected: false,
                connected_to: None,
            });
        }
    }
}

/// Generate a layout from a grammar.
///
/// # Errors
///
/// Returns validation errors if generation fails.
pub fn generate(grammar: &StructureGrammar, config: GenerationConfig) -> GenerationResult {
    StructureGenerator::new(grammar, config).generate()
}

/// Generate a layout with a simple seed.
///
/// # Errors
///
/// Returns validation errors if generation fails.
pub fn generate_with_seed(grammar: &StructureGrammar, seed: u64) -> GenerationResult {
    generate(grammar, GenerationConfig::new(seed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generation::structure_grammar::grammar::GrammarBuilder;
    use crate::generation::structure_grammar::id::RuleId;
    use crate::generation::structure_grammar::rule::{ChildSymbol, GrammarRule, WeightedChoice};
    use crate::generation::structure_grammar::template::{Bounds, Direction, Socket, TemplateKind};

    fn simple_grammar() -> StructureGrammar {
        GrammarBuilder::new()
            .root("Start")
            .add_template("room", Bounds::from_size(10, 5, 10), |t| {
                t.with_kind(TemplateKind::Room)
                    .with_tag("start")
                    .with_socket(Socket::new("north", [5, 0, 9], Direction::North))
                    .with_socket(Socket::new("east", [9, 0, 5], Direction::East))
            })
            .add_rule("Start", RuleExpansion::template(TemplateId::new(0)))
            .build()
    }

    fn branching_grammar() -> StructureGrammar {
        GrammarBuilder::new()
            .root("Dungeon")
            .add_template("entry", Bounds::from_size(10, 5, 10), |t| {
                t.with_kind(TemplateKind::Room)
                    .with_tag("entry")
                    .with_socket(Socket::new("exit", [5, 0, 9], Direction::North))
            })
            .add_template("corridor", Bounds::from_size(5, 3, 10), |t| {
                t.with_kind(TemplateKind::Corridor)
                    .with_socket(Socket::new("start", [2, 0, 0], Direction::South))
                    .with_socket(Socket::new("end", [2, 0, 9], Direction::North))
            })
            .add_template("chamber", Bounds::from_size(15, 8, 15), |t| {
                t.with_kind(TemplateKind::Room).with_tag("chamber")
            })
            .add_rule(
                "Dungeon",
                RuleExpansion::template_with_children(
                    TemplateId::new(0),
                    vec![ChildSymbol::new("Branch", "exit")],
                ),
            )
            .add_rule(
                "Branch",
                RuleExpansion::template_with_children(
                    TemplateId::new(1),
                    vec![ChildSymbol::new("Room", "end")],
                ),
            )
            .add_rule("Room", RuleExpansion::template(TemplateId::new(2)))
            .build()
    }

    #[test]
    fn generate_simple() {
        let grammar = simple_grammar();
        let result = generate_with_seed(&grammar, 42);
        assert!(result.is_ok());

        let layout = result.unwrap();
        assert_eq!(layout.placement_count(), 1);
        assert!(!layout.placements_by_tag("start").is_empty());
    }

    #[test]
    fn generate_deterministic() {
        let grammar = simple_grammar();

        let layout1 = generate_with_seed(&grammar, 12345).unwrap();
        let layout2 = generate_with_seed(&grammar, 12345).unwrap();

        assert!(layout1.fingerprint().matches(&layout2.fingerprint()));
    }

    #[test]
    fn generate_different_seeds() {
        let grammar = simple_grammar();

        let layout1 = generate_with_seed(&grammar, 111).unwrap();
        let layout2 = generate_with_seed(&grammar, 222).unwrap();

        assert!(!layout1.fingerprint().matches(&layout2.fingerprint()));
    }

    #[test]
    fn generate_branching() {
        let grammar = branching_grammar();
        let result = generate_with_seed(&grammar, 42);
        assert!(result.is_ok());

        let layout = result.unwrap();
        assert!(layout.placement_count() >= 1);
    }

    #[test]
    fn generate_with_depth_limit() {
        let grammar = branching_grammar();
        let config = GenerationConfig::new(42).with_max_depth(2);
        let result = generate(&grammar, config);
        assert!(result.is_ok());

        let layout = result.unwrap();
        assert!(layout.max_depth <= 2);
    }

    #[test]
    fn generate_with_bounds() {
        let grammar = simple_grammar();
        let config =
            GenerationConfig::new(42).with_bounds(Bounds::new([-100, -100, -100], [100, 100, 100]));
        let result = generate(&grammar, config);
        assert!(result.is_ok());

        let layout = result.unwrap();
        if let Some(bounds) = layout.overall_bounds() {
            assert!(bounds.min[0] >= -100);
            assert!(bounds.max[0] <= 100);
        }
    }

    #[test]
    fn generate_respects_overlap() {
        let grammar = GrammarBuilder::new()
            .root("Test")
            .add_template("box", Bounds::from_size(5, 5, 5), |t| t)
            .add_rule(
                "Test",
                RuleExpansion::sequence(vec![
                    RuleExpansion::template(TemplateId::new(0)),
                    RuleExpansion::template(TemplateId::new(0)),
                ]),
            )
            .build();

        let config = GenerationConfig::new(42).with_overlap(false);
        let result = generate(&grammar, config);
        assert!(result.is_ok());
    }

    #[test]
    fn generate_weighted_choices() {
        let grammar = GrammarBuilder::new()
            .root("Test")
            .add_template("heavy", Bounds::from_size(5, 5, 5), |t| t.with_tag("heavy"))
            .add_template("light", Bounds::from_size(5, 5, 5), |t| t.with_tag("light"))
            .rule(GrammarRule::with_choices(
                RuleId::new(0),
                "Test",
                vec![
                    WeightedChoice::new(9.0, RuleExpansion::template(TemplateId::new(0))),
                    WeightedChoice::new(1.0, RuleExpansion::template(TemplateId::new(1))),
                ],
            ))
            .build();

        let mut heavy_count = 0;
        let mut light_count = 0;

        for seed in 0..100 {
            let layout = generate_with_seed(&grammar, seed).unwrap();
            if !layout.placements_by_tag("heavy").is_empty() {
                heavy_count += 1;
            }
            if !layout.placements_by_tag("light").is_empty() {
                light_count += 1;
            }
        }

        assert!(heavy_count > light_count);
    }

    #[test]
    fn generate_invalid_grammar() {
        let grammar = StructureGrammar::new();
        let result = generate_with_seed(&grammar, 42);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
    }

    #[test]
    fn generate_max_steps_exceeded() {
        let grammar = GrammarBuilder::new()
            .root("Infinite")
            .add_template("node", Bounds::from_size(1, 1, 1), |t| {
                t.with_socket(Socket::new("out", [0, 0, 0], Direction::North))
            })
            .rule(GrammarRule::with_choices(
                RuleId::new(0),
                "Infinite",
                vec![
                    WeightedChoice::new(
                        99.0,
                        RuleExpansion::template_with_children(
                            TemplateId::new(0),
                            vec![ChildSymbol::new("Infinite", "out").with_offset([2, 0, 0])],
                        ),
                    ),
                    WeightedChoice::new(0.01, RuleExpansion::template(TemplateId::new(0))),
                ],
            ))
            .build();

        let config = GenerationConfig::new(42)
            .with_max_depth(100)
            .with_max_steps(10);
        let result = generate(&grammar, config);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        let has_max_steps = errors
            .errors()
            .iter()
            .any(|e| matches!(e, ValidationError::MaxStepsExceeded { .. }));
        assert!(has_max_steps);
    }

    #[test]
    fn generate_connectors() {
        let grammar = simple_grammar();
        let layout = generate_with_seed(&grammar, 42).unwrap();

        assert!(layout.connector_count() > 0);
        for connector in layout.connectors() {
            assert!(!connector.socket_name.is_empty());
        }
    }

    #[test]
    fn generate_cells_occupied() {
        let grammar = simple_grammar();
        let layout = generate_with_seed(&grammar, 42).unwrap();

        assert!(layout.occupied_cell_count() > 0);
        assert!(layout.is_occupied([5, 2, 5]));
    }

    #[test]
    fn generate_parent_child_relations() {
        let grammar = branching_grammar();
        let layout = generate_with_seed(&grammar, 42).unwrap();

        let roots = layout.roots();
        assert!(!roots.is_empty());

        for root in roots {
            let children = layout.children_of(root.id);
            for child in children {
                assert_eq!(child.parent, Some(root.id));
            }
        }
    }
}
