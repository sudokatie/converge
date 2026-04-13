//! Crafting screen UI.
//!
//! Displays available recipes and allows crafting items.

use egui::{Color32, RichText, ScrollArea, Vec2};

/// Actions returned by the crafting screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CraftingAction {
    /// Craft a recipe by ID.
    Craft(String),
    /// Close the crafting screen.
    Close,
}

/// Recipe display data for the UI.
#[derive(Clone, Debug)]
pub struct RecipeDisplay {
    /// Recipe ID.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Category for filtering.
    pub category: Option<String>,
    /// Input items (name, count).
    pub inputs: Vec<(String, u32)>,
    /// Output (name, count).
    pub output: (String, u32),
    /// Whether player has materials.
    pub can_craft: bool,
}

impl RecipeDisplay {
    /// Create a new recipe display entry.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        output_count: u32,
        can_craft: bool,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            category: None,
            inputs: Vec::new(),
            output: (String::new(), output_count),
            can_craft,
        }
    }

    /// Add an input requirement.
    pub fn with_input(mut self, name: impl Into<String>, count: u32) -> Self {
        self.inputs.push((name.into(), count));
        self
    }

    /// Set the category.
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Set the output name.
    pub fn with_output_name(mut self, name: impl Into<String>) -> Self {
        self.output.0 = name.into();
        self
    }
}

/// Crafting screen state.
#[derive(Clone, Debug, Default)]
pub struct CraftingScreen {
    /// Currently selected recipe ID.
    selected_recipe: Option<String>,
    /// Current category filter.
    category_filter: Option<String>,
    /// Search query.
    search_query: String,
    /// Whether the screen is open.
    is_open: bool,
}

impl CraftingScreen {
    /// Create a new crafting screen.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if screen is open.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    /// Open the crafting screen.
    pub fn open(&mut self) {
        self.is_open = true;
    }

    /// Close the crafting screen.
    pub fn close(&mut self) {
        self.is_open = false;
        self.selected_recipe = None;
    }

    /// Toggle the crafting screen.
    pub fn toggle(&mut self) {
        if self.is_open {
            self.close();
        } else {
            self.open();
        }
    }

    /// Get the currently selected recipe.
    #[must_use]
    pub fn selected(&self) -> Option<&str> {
        self.selected_recipe.as_deref()
    }

    /// Set category filter.
    pub fn set_category_filter(&mut self, category: Option<String>) {
        self.category_filter = category;
    }

    /// Draw the crafting screen.
    ///
    /// Returns an action if the user interacted with the UI.
    pub fn draw(
        &mut self,
        ctx: &egui::Context,
        recipes: &[RecipeDisplay],
    ) -> Option<CraftingAction> {
        if !self.is_open {
            return None;
        }

        let mut action = None;

        egui::Window::new("Crafting")
            .collapsible(false)
            .resizable(true)
            .default_size(Vec2::new(400.0, 500.0))
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                // Close button
                ui.horizontal(|ui| {
                    if ui.button("X Close").clicked() {
                        action = Some(CraftingAction::Close);
                    }

                    ui.add_space(20.0);

                    // Search
                    ui.label("Search:");
                    ui.text_edit_singleline(&mut self.search_query);
                });

                ui.separator();

                // Category tabs
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(self.category_filter.is_none(), "All")
                        .clicked()
                    {
                        self.category_filter = None;
                    }

                    for category in get_categories(recipes) {
                        if ui
                            .selectable_label(
                                self.category_filter.as_deref() == Some(category.as_str()),
                                &category,
                            )
                            .clicked()
                        {
                            self.category_filter = Some(category);
                        }
                    }
                });

                ui.separator();

                // Recipe list
                ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        for recipe in self.filter_recipes(recipes) {
                            let is_selected = self.selected_recipe.as_ref() == Some(&recipe.id);

                            let text = if recipe.can_craft {
                                RichText::new(&recipe.name).color(Color32::WHITE)
                            } else {
                                RichText::new(&recipe.name).color(Color32::GRAY)
                            };

                            let response = ui.selectable_label(is_selected, text);

                            if response.clicked() {
                                self.selected_recipe = Some(recipe.id.clone());
                            }

                            // Show details on hover
                            response.on_hover_ui(|ui| {
                                ui.label(format!("Output: {} x{}", recipe.output.0, recipe.output.1));
                                ui.label("Requires:");
                                for (name, count) in &recipe.inputs {
                                    ui.label(format!("  - {} x{}", name, count));
                                }
                            });
                        }
                    });

                ui.separator();

                // Craft button
                ui.horizontal(|ui| {
                    let can_craft = self
                        .selected_recipe
                        .as_ref()
                        .and_then(|id| recipes.iter().find(|r| &r.id == id))
                        .map(|r| r.can_craft)
                        .unwrap_or(false);

                    let button = ui.add_enabled(
                        can_craft,
                        egui::Button::new("Craft"),
                    );

                    if button.clicked() {
                        if let Some(id) = &self.selected_recipe {
                            action = Some(CraftingAction::Craft(id.clone()));
                        }
                    }

                    if !can_craft {
                        ui.label(RichText::new("Missing materials").color(Color32::RED));
                    }
                });
            });

        // Handle close action
        if matches!(action, Some(CraftingAction::Close)) {
            self.close();
        }

        action
    }

    /// Filter recipes based on current filters.
    fn filter_recipes<'a>(&self, recipes: &'a [RecipeDisplay]) -> Vec<&'a RecipeDisplay> {
        recipes
            .iter()
            .filter(|r| {
                // Category filter
                if let Some(cat) = &self.category_filter {
                    if r.category.as_ref() != Some(cat) {
                        return false;
                    }
                }

                // Search filter
                if !self.search_query.is_empty() {
                    let query = self.search_query.to_lowercase();
                    if !r.name.to_lowercase().contains(&query) {
                        return false;
                    }
                }

                true
            })
            .collect()
    }
}

/// Extract unique categories from recipes.
fn get_categories(recipes: &[RecipeDisplay]) -> Vec<String> {
    let mut categories: Vec<_> = recipes
        .iter()
        .filter_map(|r| r.category.clone())
        .collect();
    categories.sort();
    categories.dedup();
    categories
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_recipes() -> Vec<RecipeDisplay> {
        vec![
            RecipeDisplay::new("planks", "Oak Planks", 4, true)
                .with_category("building")
                .with_input("Oak Log", 1)
                .with_output_name("Oak Planks"),
            RecipeDisplay::new("sticks", "Sticks", 4, true)
                .with_category("materials")
                .with_input("Oak Planks", 2)
                .with_output_name("Stick"),
            RecipeDisplay::new("pickaxe", "Wooden Pickaxe", 1, false)
                .with_category("tools")
                .with_input("Oak Planks", 3)
                .with_input("Stick", 2)
                .with_output_name("Wooden Pickaxe"),
        ]
    }

    #[test]
    fn test_crafting_screen_new() {
        let screen = CraftingScreen::new();
        assert!(!screen.is_open());
        assert!(screen.selected().is_none());
    }

    #[test]
    fn test_open_close() {
        let mut screen = CraftingScreen::new();

        screen.open();
        assert!(screen.is_open());

        screen.close();
        assert!(!screen.is_open());
    }

    #[test]
    fn test_toggle() {
        let mut screen = CraftingScreen::new();

        screen.toggle();
        assert!(screen.is_open());

        screen.toggle();
        assert!(!screen.is_open());
    }

    #[test]
    fn test_filter_by_category() {
        let mut screen = CraftingScreen::new();
        let recipes = sample_recipes();

        // No filter - all recipes
        let filtered = screen.filter_recipes(&recipes);
        assert_eq!(filtered.len(), 3);

        // Filter by category
        screen.set_category_filter(Some("tools".to_string()));
        let filtered = screen.filter_recipes(&recipes);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "pickaxe");
    }

    #[test]
    fn test_filter_by_search() {
        let mut screen = CraftingScreen::new();
        let recipes = sample_recipes();

        screen.search_query = "plank".to_string();
        let filtered = screen.filter_recipes(&recipes);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "planks");
    }

    #[test]
    fn test_recipe_display_builder() {
        let recipe = RecipeDisplay::new("test", "Test Recipe", 2, true)
            .with_category("misc")
            .with_input("Item A", 3)
            .with_input("Item B", 1)
            .with_output_name("Test Output");

        assert_eq!(recipe.id, "test");
        assert_eq!(recipe.name, "Test Recipe");
        assert_eq!(recipe.category, Some("misc".to_string()));
        assert_eq!(recipe.inputs.len(), 2);
        assert_eq!(recipe.output.0, "Test Output");
        assert_eq!(recipe.output.1, 2);
    }

    #[test]
    fn test_get_categories() {
        let recipes = sample_recipes();
        let cats = get_categories(&recipes);

        assert_eq!(cats.len(), 3);
        assert!(cats.contains(&"building".to_string()));
        assert!(cats.contains(&"materials".to_string()));
        assert!(cats.contains(&"tools".to_string()));
    }
}
