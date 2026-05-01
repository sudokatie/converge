//! Content validation library for Lattice game data files.
//!
//! Validates items, recipes, blocks, creatures, and biomes defined in RON files.
//! Also provides asset manifest generation and versioning for content packs.

pub mod manifest;

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemCategory {
    Block,
    Tool,
    Weapon,
    Food,
    Material,
    Misc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolType {
    Pickaxe,
    Axe,
    Shovel,
    Hoe,
    Sword,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDef {
    pub id: u16,
    pub name: String,
    #[serde(default = "default_stack_size")]
    pub stack_size: u32,
    pub category: ItemCategory,
    #[serde(default)]
    pub tool_type: Option<ToolType>,
    #[serde(default)]
    pub durability: Option<u32>,
    #[serde(default)]
    pub block_id: Option<u16>,
    #[serde(default)]
    pub damage: f32,
    #[serde(default = "default_mining_speed")]
    pub mining_speed: f32,
    #[serde(default)]
    pub food_value: f32,
    #[serde(default)]
    pub saturation_value: f32,
}

fn default_stack_size() -> u32 {
    64
}

fn default_mining_speed() -> f32 {
    1.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CraftingStation {
    CraftingTable,
    Furnace,
    Anvil,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ingredient {
    pub item: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeOutput {
    pub item: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeDef {
    pub id: String,
    pub inputs: Vec<Ingredient>,
    pub output: RecipeOutput,
    #[serde(default)]
    pub station: Option<CraftingStation>,
    #[serde(default)]
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDef {
    pub name: String,
    pub solid: bool,
    pub transparent: bool,
    pub light_emission: u8,
    pub hardness: f32,
    pub texture_indices: [u16; 6],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CreatureKind {
    Pig,
    Cow,
    Sheep,
    Chicken,
    Zombie,
    Skeleton,
    Spider,
    Creeper,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatureDef {
    pub id: String,
    pub kind: CreatureKind,
    pub max_health: f32,
    pub move_speed: f32,
    #[serde(default)]
    pub attack_damage: f32,
    #[serde(default)]
    pub hostile: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BiomeType {
    Plains,
    Forest,
    Desert,
    Mountains,
    Ocean,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeDef {
    pub id: String,
    pub biome_type: BiomeType,
    pub surface_block: u16,
    pub subsurface_block: u16,
    #[serde(default)]
    pub tree_density: f32,
    #[serde(default)]
    pub height_modifier: f64,
}

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error in {file}: {message}")]
    Parse { file: String, message: String },
    #[error("Duplicate item ID {id} for items '{first}' and '{second}'")]
    DuplicateItemId {
        id: u16,
        first: String,
        second: String,
    },
    #[error("Duplicate item name '{name}' with IDs {first} and {second}")]
    DuplicateItemName {
        name: String,
        first: u16,
        second: u16,
    },
    #[error("Duplicate recipe ID '{id}'")]
    DuplicateRecipeId { id: String },
    #[error("Invalid stack size {size} for item '{name}' (must be 1-64)")]
    InvalidStackSize { name: String, size: u32 },
    #[error("Tool '{name}' missing durability")]
    ToolMissingDurability { name: String },
    #[error("Tool '{name}' missing tool_type")]
    ToolMissingType { name: String },
    #[error("Non-tool item '{name}' has tool_type but wrong category")]
    ToolTypeCategoryMismatch { name: String },
    #[error("Block item '{name}' references unknown block ID {block_id}")]
    UnknownBlockReference { name: String, block_id: u16 },
    #[error("Recipe '{recipe_id}' references unknown item '{item_name}'")]
    UnknownRecipeItem {
        recipe_id: String,
        item_name: String,
    },
    #[error("Recipe '{recipe_id}' has non-positive count {count} for item '{item_name}'")]
    NonPositiveCount {
        recipe_id: String,
        item_name: String,
        count: u32,
    },
    #[error("Recipe '{recipe_id}' has empty inputs")]
    EmptyRecipeInputs { recipe_id: String },
    #[error("Duplicate block ID {id}")]
    DuplicateBlockId { id: u16 },
    #[error("Duplicate creature ID '{id}'")]
    DuplicateCreatureId { id: String },
    #[error("Creature '{id}' has non-positive health {health}")]
    InvalidCreatureHealth { id: String, health: f32 },
    #[error("Duplicate biome ID '{id}'")]
    DuplicateBiomeId { id: String },
    #[error("Biome '{id}' references unknown block {block_id}")]
    UnknownBiomeBlock { id: String, block_id: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug)]
pub struct ValidationIssue {
    pub severity: Severity,
    pub message: String,
}

impl ValidationIssue {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
        }
    }
}

#[derive(Debug, Default)]
pub struct ValidationReport {
    pub issues: Vec<ValidationIssue>,
    pub items_validated: usize,
    pub recipes_validated: usize,
    pub blocks_validated: usize,
    pub creatures_validated: usize,
    pub biomes_validated: usize,
}

impl ValidationReport {
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.severity == Severity::Error)
    }

    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .count()
    }

    fn add_error(&mut self, err: &ValidationError) {
        self.issues.push(ValidationIssue::error(err.to_string()));
    }

    fn add_warning(&mut self, msg: impl Into<String>) {
        self.issues.push(ValidationIssue::warning(msg));
    }
}

impl std::fmt::Display for ValidationReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Content Validation Report")?;
        writeln!(f, "=========================")?;
        writeln!(f)?;

        writeln!(f, "Validated:")?;
        writeln!(f, "  Items:     {}", self.items_validated)?;
        writeln!(f, "  Recipes:   {}", self.recipes_validated)?;
        writeln!(f, "  Blocks:    {}", self.blocks_validated)?;
        writeln!(f, "  Creatures: {}", self.creatures_validated)?;
        writeln!(f, "  Biomes:    {}", self.biomes_validated)?;
        writeln!(f)?;

        if self.issues.is_empty() {
            writeln!(f, "No issues found.")?;
        } else {
            writeln!(
                f,
                "Issues: {} error(s), {} warning(s)",
                self.error_count(),
                self.warning_count()
            )?;
            writeln!(f)?;

            for issue in &self.issues {
                let prefix = match issue.severity {
                    Severity::Error => "ERROR",
                    Severity::Warning => "WARN ",
                };
                writeln!(f, "  [{prefix}] {}", issue.message)?;
            }
        }

        Ok(())
    }
}

const KNOWN_BLOCK_IDS: &[u16] = &[
    0,  // Air
    1,  // Stone
    2,  // Dirt
    3,  // Grass
    4,  // Sand
    5,  // Gravel (Water in engine, but items.ron says Gravel)
    6,  // Cobblestone
    7,  // Oak Log
    8,  // Oak Planks
    9,  // Oak Leaves
    10, // Glass
    11, // Coal Ore
    12, // Iron Ore
];

pub struct ContentValidator {
    content_root: std::path::PathBuf,
}

#[allow(clippy::unused_self)]
impl ContentValidator {
    pub fn new(content_root: impl AsRef<Path>) -> Self {
        Self {
            content_root: content_root.as_ref().to_path_buf(),
        }
    }

    pub fn validate(&self) -> ValidationReport {
        let mut report = ValidationReport::default();

        let items_path = self.content_root.join("items.ron");
        let recipes_path = self.content_root.join("recipes.ron");
        let blocks_path = self.content_root.join("blocks.ron");
        let creatures_path = self.content_root.join("creatures.ron");
        let biomes_path = self.content_root.join("biomes.ron");

        let items = self.validate_items(&items_path, &mut report);
        let item_names: HashSet<String> = items.iter().map(|i| i.name.to_lowercase()).collect();

        self.validate_recipes(&recipes_path, &item_names, &mut report);

        let block_ids: HashSet<u16> = if blocks_path.exists() {
            self.validate_blocks(&blocks_path, &mut report)
        } else {
            report.add_warning(format!(
                "Optional file {} not found, using built-in block IDs",
                blocks_path.display()
            ));
            KNOWN_BLOCK_IDS.iter().copied().collect()
        };

        self.validate_block_references(&items, &block_ids, &mut report);

        if creatures_path.exists() {
            self.validate_creatures(&creatures_path, &mut report);
        } else {
            report.add_warning(format!(
                "Optional file {} not found, skipping creature validation",
                creatures_path.display()
            ));
        }

        if biomes_path.exists() {
            self.validate_biomes(&biomes_path, &block_ids, &mut report);
        } else {
            report.add_warning(format!(
                "Optional file {} not found, skipping biome validation",
                biomes_path.display()
            ));
        }

        report
    }

    fn validate_items(&self, path: &Path, report: &mut ValidationReport) -> Vec<ItemDef> {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                report.add_error(&ValidationError::Parse {
                    file: path.display().to_string(),
                    message: e.to_string(),
                });
                return Vec::new();
            }
        };

        let items: Vec<ItemDef> = match ron::from_str(&content) {
            Ok(i) => i,
            Err(e) => {
                report.add_error(&ValidationError::Parse {
                    file: path.display().to_string(),
                    message: e.to_string(),
                });
                return Vec::new();
            }
        };

        let mut seen_ids: HashMap<u16, String> = HashMap::new();
        let mut seen_names: HashMap<String, u16> = HashMap::new();

        for item in &items {
            if let Some(existing) = seen_ids.get(&item.id) {
                report.add_error(&ValidationError::DuplicateItemId {
                    id: item.id,
                    first: existing.clone(),
                    second: item.name.clone(),
                });
            } else {
                seen_ids.insert(item.id, item.name.clone());
            }

            let lower_name = item.name.to_lowercase();
            if let Some(&existing_id) = seen_names.get(&lower_name) {
                report.add_error(&ValidationError::DuplicateItemName {
                    name: item.name.clone(),
                    first: existing_id,
                    second: item.id,
                });
            } else {
                seen_names.insert(lower_name, item.id);
            }

            if item.stack_size == 0 || item.stack_size > 64 {
                report.add_error(&ValidationError::InvalidStackSize {
                    name: item.name.clone(),
                    size: item.stack_size,
                });
            }

            match item.category {
                ItemCategory::Tool | ItemCategory::Weapon => {
                    if item.tool_type.is_none() {
                        report.add_error(&ValidationError::ToolMissingType {
                            name: item.name.clone(),
                        });
                    }
                    if item.durability.is_none() {
                        report.add_error(&ValidationError::ToolMissingDurability {
                            name: item.name.clone(),
                        });
                    }
                }
                _ => {
                    if item.tool_type.is_some() {
                        report.add_error(&ValidationError::ToolTypeCategoryMismatch {
                            name: item.name.clone(),
                        });
                    }
                }
            }
        }

        report.items_validated = items.len();
        items
    }

    fn validate_recipes(
        &self,
        path: &Path,
        item_names: &HashSet<String>,
        report: &mut ValidationReport,
    ) {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                report.add_error(&ValidationError::Parse {
                    file: path.display().to_string(),
                    message: e.to_string(),
                });
                return;
            }
        };

        let recipes: Vec<RecipeDef> = match ron::from_str(&content) {
            Ok(r) => r,
            Err(e) => {
                report.add_error(&ValidationError::Parse {
                    file: path.display().to_string(),
                    message: e.to_string(),
                });
                return;
            }
        };

        let mut seen_ids: HashSet<String> = HashSet::new();

        for recipe in &recipes {
            if seen_ids.contains(&recipe.id) {
                report.add_error(&ValidationError::DuplicateRecipeId {
                    id: recipe.id.clone(),
                });
            } else {
                seen_ids.insert(recipe.id.clone());
            }

            if recipe.inputs.is_empty() {
                report.add_error(&ValidationError::EmptyRecipeInputs {
                    recipe_id: recipe.id.clone(),
                });
            }

            for input in &recipe.inputs {
                if !item_names.contains(&input.item.to_lowercase()) {
                    report.add_error(&ValidationError::UnknownRecipeItem {
                        recipe_id: recipe.id.clone(),
                        item_name: input.item.clone(),
                    });
                }

                if input.count == 0 {
                    report.add_error(&ValidationError::NonPositiveCount {
                        recipe_id: recipe.id.clone(),
                        item_name: input.item.clone(),
                        count: input.count,
                    });
                }
            }

            if !item_names.contains(&recipe.output.item.to_lowercase()) {
                report.add_error(&ValidationError::UnknownRecipeItem {
                    recipe_id: recipe.id.clone(),
                    item_name: recipe.output.item.clone(),
                });
            }

            if recipe.output.count == 0 {
                report.add_error(&ValidationError::NonPositiveCount {
                    recipe_id: recipe.id.clone(),
                    item_name: recipe.output.item.clone(),
                    count: recipe.output.count,
                });
            }
        }

        report.recipes_validated = recipes.len();
    }

    fn validate_blocks(&self, path: &Path, report: &mut ValidationReport) -> HashSet<u16> {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                report.add_error(&ValidationError::Parse {
                    file: path.display().to_string(),
                    message: e.to_string(),
                });
                return KNOWN_BLOCK_IDS.iter().copied().collect();
            }
        };

        let blocks: HashMap<u16, BlockDef> = match ron::from_str(&content) {
            Ok(b) => b,
            Err(e) => {
                report.add_error(&ValidationError::Parse {
                    file: path.display().to_string(),
                    message: e.to_string(),
                });
                return KNOWN_BLOCK_IDS.iter().copied().collect();
            }
        };

        report.blocks_validated = blocks.len();
        blocks.keys().copied().collect()
    }

    fn validate_block_references(
        &self,
        items: &[ItemDef],
        block_ids: &HashSet<u16>,
        report: &mut ValidationReport,
    ) {
        for item in items {
            if let Some(block_id) = item.block_id
                && !block_ids.contains(&block_id)
            {
                report.add_error(&ValidationError::UnknownBlockReference {
                    name: item.name.clone(),
                    block_id,
                });
            }
        }
    }

    fn validate_creatures(&self, path: &Path, report: &mut ValidationReport) {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                report.add_error(&ValidationError::Parse {
                    file: path.display().to_string(),
                    message: e.to_string(),
                });
                return;
            }
        };

        let creatures: Vec<CreatureDef> = match ron::from_str(&content) {
            Ok(c) => c,
            Err(e) => {
                report.add_error(&ValidationError::Parse {
                    file: path.display().to_string(),
                    message: e.to_string(),
                });
                return;
            }
        };

        let mut seen_ids: HashSet<String> = HashSet::new();

        for creature in &creatures {
            if seen_ids.contains(&creature.id) {
                report.add_error(&ValidationError::DuplicateCreatureId {
                    id: creature.id.clone(),
                });
            } else {
                seen_ids.insert(creature.id.clone());
            }

            if creature.max_health <= 0.0 {
                report.add_error(&ValidationError::InvalidCreatureHealth {
                    id: creature.id.clone(),
                    health: creature.max_health,
                });
            }
        }

        report.creatures_validated = creatures.len();
    }

    fn validate_biomes(
        &self,
        path: &Path,
        block_ids: &HashSet<u16>,
        report: &mut ValidationReport,
    ) {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                report.add_error(&ValidationError::Parse {
                    file: path.display().to_string(),
                    message: e.to_string(),
                });
                return;
            }
        };

        let biomes: Vec<BiomeDef> = match ron::from_str(&content) {
            Ok(b) => b,
            Err(e) => {
                report.add_error(&ValidationError::Parse {
                    file: path.display().to_string(),
                    message: e.to_string(),
                });
                return;
            }
        };

        let mut seen_ids: HashSet<String> = HashSet::new();

        for biome in &biomes {
            if seen_ids.contains(&biome.id) {
                report.add_error(&ValidationError::DuplicateBiomeId {
                    id: biome.id.clone(),
                });
            } else {
                seen_ids.insert(biome.id.clone());
            }

            if !block_ids.contains(&biome.surface_block) {
                report.add_error(&ValidationError::UnknownBiomeBlock {
                    id: biome.id.clone(),
                    block_id: biome.surface_block,
                });
            }

            if !block_ids.contains(&biome.subsurface_block) {
                report.add_error(&ValidationError::UnknownBiomeBlock {
                    id: biome.id.clone(),
                    block_id: biome.subsurface_block,
                });
            }
        }

        report.biomes_validated = biomes.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_dir() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    fn write_file(dir: &Path, name: &str, content: &str) {
        fs::write(dir.join(name), content).unwrap();
    }

    #[test]
    fn test_valid_content_pack() {
        let dir = create_test_dir();
        let path = dir.path();

        write_file(
            path,
            "items.ron",
            r#"[
                (id: 1, name: "Stone", stack_size: 64, category: Block, block_id: Some(1)),
                (id: 2, name: "Stick", stack_size: 64, category: Material),
            ]"#,
        );

        write_file(
            path,
            "recipes.ron",
            r#"[
                (id: "sticks", inputs: [(item: "Stone", count: 1)], output: (item: "Stick", count: 2)),
            ]"#,
        );

        let validator = ContentValidator::new(path);
        let report = validator.validate();

        assert!(!report.has_errors());
        assert_eq!(report.items_validated, 2);
        assert_eq!(report.recipes_validated, 1);
    }

    #[test]
    fn test_duplicate_item_id() {
        let dir = create_test_dir();
        let path = dir.path();

        write_file(
            path,
            "items.ron",
            r#"[
                (id: 1, name: "Stone", stack_size: 64, category: Block),
                (id: 1, name: "Dirt", stack_size: 64, category: Block),
            ]"#,
        );

        write_file(path, "recipes.ron", "[]");

        let validator = ContentValidator::new(path);
        let report = validator.validate();

        assert!(report.has_errors());
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.message.contains("Duplicate item ID"))
        );
    }

    #[test]
    fn test_duplicate_item_name() {
        let dir = create_test_dir();
        let path = dir.path();

        write_file(
            path,
            "items.ron",
            r#"[
                (id: 1, name: "Stone", stack_size: 64, category: Block),
                (id: 2, name: "stone", stack_size: 64, category: Block),
            ]"#,
        );

        write_file(path, "recipes.ron", "[]");

        let validator = ContentValidator::new(path);
        let report = validator.validate();

        assert!(report.has_errors());
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.message.contains("Duplicate item name"))
        );
    }

    #[test]
    fn test_unknown_recipe_item() {
        let dir = create_test_dir();
        let path = dir.path();

        write_file(
            path,
            "items.ron",
            r#"[
                (id: 1, name: "Stone", stack_size: 64, category: Block),
            ]"#,
        );

        write_file(
            path,
            "recipes.ron",
            r#"[
                (id: "test", inputs: [(item: "Unknown", count: 1)], output: (item: "Stone", count: 1)),
            ]"#,
        );

        let validator = ContentValidator::new(path);
        let report = validator.validate();

        assert!(report.has_errors());
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.message.contains("unknown item") && i.message.contains("Recipe"))
        );
    }

    #[test]
    fn test_invalid_block_reference() {
        let dir = create_test_dir();
        let path = dir.path();

        write_file(
            path,
            "items.ron",
            r#"[
                (id: 1, name: "Stone", stack_size: 64, category: Block, block_id: Some(999)),
            ]"#,
        );

        write_file(path, "recipes.ron", "[]");

        let validator = ContentValidator::new(path);
        let report = validator.validate();

        assert!(report.has_errors());
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.message.contains("unknown block ID"))
        );
    }

    #[test]
    fn test_tool_missing_durability() {
        let dir = create_test_dir();
        let path = dir.path();

        write_file(
            path,
            "items.ron",
            r#"[
                (id: 1, name: "Pickaxe", stack_size: 1, category: Tool, tool_type: Some(Pickaxe)),
            ]"#,
        );

        write_file(path, "recipes.ron", "[]");

        let validator = ContentValidator::new(path);
        let report = validator.validate();

        assert!(report.has_errors());
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.message.contains("missing durability"))
        );
    }

    #[test]
    fn test_optional_files_warning() {
        let dir = create_test_dir();
        let path = dir.path();

        write_file(
            path,
            "items.ron",
            r#"[(id: 1, name: "Stone", stack_size: 64, category: Block)]"#,
        );

        write_file(path, "recipes.ron", "[]");

        let validator = ContentValidator::new(path);
        let report = validator.validate();

        assert!(report.warning_count() > 0);
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.severity == Severity::Warning && i.message.contains("creatures.ron"))
        );
    }

    #[test]
    fn test_duplicate_recipe_id() {
        let dir = create_test_dir();
        let path = dir.path();

        write_file(
            path,
            "items.ron",
            r#"[(id: 1, name: "Stone", stack_size: 64, category: Block)]"#,
        );

        write_file(
            path,
            "recipes.ron",
            r#"[
                (id: "same", inputs: [(item: "Stone", count: 1)], output: (item: "Stone", count: 1)),
                (id: "same", inputs: [(item: "Stone", count: 2)], output: (item: "Stone", count: 2)),
            ]"#,
        );

        let validator = ContentValidator::new(path);
        let report = validator.validate();

        assert!(report.has_errors());
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.message.contains("Duplicate recipe ID"))
        );
    }

    #[test]
    fn test_non_positive_count() {
        let dir = create_test_dir();
        let path = dir.path();

        write_file(
            path,
            "items.ron",
            r#"[(id: 1, name: "Stone", stack_size: 64, category: Block)]"#,
        );

        write_file(
            path,
            "recipes.ron",
            r#"[
                (id: "bad", inputs: [(item: "Stone", count: 0)], output: (item: "Stone", count: 1)),
            ]"#,
        );

        let validator = ContentValidator::new(path);
        let report = validator.validate();

        assert!(report.has_errors());
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.message.contains("non-positive count"))
        );
    }

    #[test]
    fn test_invalid_stack_size() {
        let dir = create_test_dir();
        let path = dir.path();

        write_file(
            path,
            "items.ron",
            r#"[(id: 1, name: "Stone", stack_size: 100, category: Block)]"#,
        );

        write_file(path, "recipes.ron", "[]");

        let validator = ContentValidator::new(path);
        let report = validator.validate();

        assert!(report.has_errors());
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.message.contains("Invalid stack size"))
        );
    }

    #[test]
    fn test_parse_error() {
        let dir = create_test_dir();
        let path = dir.path();

        write_file(path, "items.ron", "this is not valid ron {{{");
        write_file(path, "recipes.ron", "[]");

        let validator = ContentValidator::new(path);
        let report = validator.validate();

        assert!(report.has_errors());
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.message.contains("Parse error"))
        );
    }
}
