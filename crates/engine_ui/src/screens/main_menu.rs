//! Main menu screen UI.
//!
//! Entry point for the game with world selection and server connection.

use egui::{Color32, RichText, ScrollArea, TextEdit, Vec2};

/// Actions returned by the main menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MainMenuAction {
    /// Start singleplayer with selected world.
    PlayWorld(String),
    /// Create a new world.
    CreateWorld { name: String, seed: String },
    /// Delete a world.
    DeleteWorld(String),
    /// Connect to multiplayer server.
    ConnectServer { address: String },
    /// Open settings.
    OpenSettings,
    /// Quit the game.
    Quit,
}

/// Main menu screen state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MainMenuView {
    #[default]
    Main,
    Singleplayer,
    CreateWorld,
    Multiplayer,
    ConfirmDelete,
}

/// World save info for display.
#[derive(Clone, Debug)]
pub struct WorldInfo {
    /// World folder name (ID).
    pub id: String,
    /// Display name.
    pub name: String,
    /// Last played timestamp.
    pub last_played: Option<String>,
    /// World seed (optional display).
    pub seed: Option<String>,
}

impl WorldInfo {
    /// Create a new world info entry.
    #[must_use]
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            last_played: None,
            seed: None,
        }
    }

    /// Set last played timestamp.
    pub fn with_last_played(mut self, timestamp: impl Into<String>) -> Self {
        self.last_played = Some(timestamp.into());
        self
    }

    /// Set seed display.
    pub fn with_seed(mut self, seed: impl Into<String>) -> Self {
        self.seed = Some(seed.into());
        self
    }
}

/// Main menu screen.
#[derive(Clone, Debug, Default)]
pub struct MainMenuScreen {
    /// Current view.
    view: MainMenuView,
    /// Available worlds.
    worlds: Vec<WorldInfo>,
    /// Selected world index.
    selected_world: Option<usize>,
    /// New world name input.
    new_world_name: String,
    /// New world seed input.
    new_world_seed: String,
    /// Server address input.
    server_address: String,
    /// World pending deletion.
    pending_delete: Option<String>,
    /// Error message to display.
    error_message: Option<String>,
}

impl MainMenuScreen {
    /// Create a new main menu screen.
    #[must_use]
    pub fn new() -> Self {
        Self {
            server_address: "127.0.0.1:25565".to_string(),
            ..Default::default()
        }
    }

    /// Set available worlds.
    pub fn set_worlds(&mut self, worlds: Vec<WorldInfo>) {
        self.worlds = worlds;
        self.selected_world = if self.worlds.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    /// Get worlds.
    #[must_use]
    pub fn worlds(&self) -> &[WorldInfo] {
        &self.worlds
    }

    /// Get current view.
    #[must_use]
    pub fn view(&self) -> MainMenuView {
        self.view
    }

    /// Set error message.
    pub fn set_error(&mut self, error: impl Into<String>) {
        self.error_message = Some(error.into());
    }

    /// Clear error message.
    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    /// Go back to main view.
    pub fn back_to_main(&mut self) {
        self.view = MainMenuView::Main;
        self.error_message = None;
        self.pending_delete = None;
    }

    /// Draw the main menu and return any action.
    #[must_use]
    pub fn draw(&mut self, ctx: &egui::Context) -> Option<MainMenuAction> {
        let mut action = None;

        egui::CentralPanel::default().show(ctx, |ui| {
            // Title
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.label(
                    RichText::new("LATTICE")
                        .size(72.0)
                        .color(Color32::WHITE)
                        .strong(),
                );
                ui.label(
                    RichText::new("A Survival Adventure")
                        .size(18.0)
                        .color(Color32::GRAY),
                );
                ui.add_space(40.0);
            });

            // Error display
            if let Some(ref error) = self.error_message {
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(error).color(Color32::RED));
                    ui.add_space(8.0);
                });
            }

            // View content
            ui.vertical_centered(|ui| {
                match self.view {
                    MainMenuView::Main => {
                        action = self.draw_main_view(ui);
                    }
                    MainMenuView::Singleplayer => {
                        action = self.draw_singleplayer_view(ui);
                    }
                    MainMenuView::CreateWorld => {
                        action = self.draw_create_world_view(ui);
                    }
                    MainMenuView::Multiplayer => {
                        action = self.draw_multiplayer_view(ui);
                    }
                    MainMenuView::ConfirmDelete => {
                        action = self.draw_confirm_delete_view(ui);
                    }
                }
            });

            // Version footer
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.label(RichText::new("v0.1.0").color(Color32::DARK_GRAY));
            });
        });

        action
    }

    fn draw_main_view(&mut self, ui: &mut egui::Ui) -> Option<MainMenuAction> {
        let button_size = Vec2::new(200.0, 40.0);
        let mut action = None;

        if ui
            .add_sized(button_size, egui::Button::new("Singleplayer"))
            .clicked()
        {
            self.view = MainMenuView::Singleplayer;
        }

        ui.add_space(8.0);

        if ui
            .add_sized(button_size, egui::Button::new("Multiplayer"))
            .clicked()
        {
            self.view = MainMenuView::Multiplayer;
        }

        ui.add_space(8.0);

        if ui
            .add_sized(button_size, egui::Button::new("Settings"))
            .clicked()
        {
            action = Some(MainMenuAction::OpenSettings);
        }

        ui.add_space(8.0);

        if ui
            .add_sized(button_size, egui::Button::new("Quit"))
            .clicked()
        {
            action = Some(MainMenuAction::Quit);
        }

        action
    }

    fn draw_singleplayer_view(&mut self, ui: &mut egui::Ui) -> Option<MainMenuAction> {
        let mut action = None;

        ui.heading("Select World");
        ui.add_space(16.0);

        // World list
        ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                for (idx, world) in self.worlds.iter().enumerate() {
                    let is_selected = self.selected_world == Some(idx);
                    let response = ui.selectable_label(is_selected, &world.name);
                    if response.clicked() {
                        self.selected_world = Some(idx);
                    }
                    if let Some(ref last_played) = world.last_played {
                        ui.label(RichText::new(format!("  Last played: {last_played}")).size(12.0));
                    }
                }

                if self.worlds.is_empty() {
                    ui.label(RichText::new("No worlds found").color(Color32::GRAY));
                }
            });

        ui.add_space(16.0);

        // Buttons
        ui.horizontal(|ui| {
            if ui.button("Create New World").clicked() {
                self.view = MainMenuView::CreateWorld;
                self.new_world_name.clear();
                self.new_world_seed.clear();
            }

            if let Some(idx) = self.selected_world {
                if ui.button("Play Selected World").clicked() {
                    let world_id = self.worlds[idx].id.clone();
                    action = Some(MainMenuAction::PlayWorld(world_id));
                }

                if ui.button("Delete").clicked() {
                    self.pending_delete = Some(self.worlds[idx].id.clone());
                    self.view = MainMenuView::ConfirmDelete;
                }
            }
        });

        ui.add_space(16.0);

        if ui.button("Back").clicked() {
            self.view = MainMenuView::Main;
        }

        action
    }

    fn draw_create_world_view(&mut self, ui: &mut egui::Ui) -> Option<MainMenuAction> {
        let mut action = None;

        ui.heading("Create New World");
        ui.add_space(16.0);

        ui.horizontal(|ui| {
            ui.label("World Name:");
            ui.add(TextEdit::singleline(&mut self.new_world_name).desired_width(200.0));
        });

        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label("Seed (optional):");
            ui.add(
                TextEdit::singleline(&mut self.new_world_seed)
                    .desired_width(200.0)
                    .hint_text("Leave empty for random"),
            );
        });

        ui.add_space(16.0);

        ui.horizontal(|ui| {
            if ui.button("Create").clicked() {
                let name = self.new_world_name.trim().to_string();
                if name.is_empty() {
                    self.error_message = Some("World name cannot be empty".to_string());
                } else {
                    let seed = self.new_world_seed.trim().to_string();
                    action = Some(MainMenuAction::CreateWorld { name, seed });
                }
            }

            if ui.button("Cancel").clicked() {
                self.view = MainMenuView::Singleplayer;
            }
        });

        action
    }

    fn draw_multiplayer_view(&mut self, ui: &mut egui::Ui) -> Option<MainMenuAction> {
        let mut action = None;

        ui.heading("Join Server");
        ui.add_space(16.0);

        ui.horizontal(|ui| {
            ui.label("Server Address:");
            ui.add(
                TextEdit::singleline(&mut self.server_address)
                    .desired_width(200.0)
                    .hint_text("IP:Port"),
            );
        });

        ui.add_space(16.0);

        ui.horizontal(|ui| {
            if ui.button("Connect").clicked() {
                let address = self.server_address.trim().to_string();
                if address.is_empty() {
                    self.error_message = Some("Server address cannot be empty".to_string());
                } else {
                    action = Some(MainMenuAction::ConnectServer { address });
                }
            }

            if ui.button("Back").clicked() {
                self.view = MainMenuView::Main;
            }
        });

        action
    }

    fn draw_confirm_delete_view(&mut self, ui: &mut egui::Ui) -> Option<MainMenuAction> {
        let mut action = None;

        if let Some(ref world_id) = self.pending_delete.clone() {
            ui.heading("Delete World?");
            ui.add_space(16.0);

            ui.label(
                RichText::new(format!("Are you sure you want to delete '{world_id}'?"))
                    .color(Color32::YELLOW),
            );
            ui.label(RichText::new("This cannot be undone.").color(Color32::RED));

            ui.add_space(16.0);

            ui.horizontal(|ui| {
                if ui.button("Delete").clicked() {
                    action = Some(MainMenuAction::DeleteWorld(world_id.clone()));
                    self.pending_delete = None;
                    self.view = MainMenuView::Singleplayer;
                }

                if ui.button("Cancel").clicked() {
                    self.pending_delete = None;
                    self.view = MainMenuView::Singleplayer;
                }
            });
        }

        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main_menu_new() {
        let menu = MainMenuScreen::new();
        assert_eq!(menu.view(), MainMenuView::Main);
        assert!(menu.worlds().is_empty());
        assert_eq!(menu.server_address, "127.0.0.1:25565");
    }

    #[test]
    fn test_set_worlds() {
        let mut menu = MainMenuScreen::new();
        let worlds = vec![
            WorldInfo::new("world1", "My World"),
            WorldInfo::new("world2", "Test World"),
        ];
        menu.set_worlds(worlds);
        assert_eq!(menu.worlds().len(), 2);
        assert_eq!(menu.selected_world, Some(0));
    }

    #[test]
    fn test_set_worlds_empty() {
        let mut menu = MainMenuScreen::new();
        menu.set_worlds(vec![]);
        assert!(menu.worlds().is_empty());
        assert!(menu.selected_world.is_none());
    }

    #[test]
    fn test_world_info_builder() {
        let world = WorldInfo::new("test", "Test World")
            .with_last_played("2026-04-14")
            .with_seed("12345");

        assert_eq!(world.id, "test");
        assert_eq!(world.name, "Test World");
        assert_eq!(world.last_played, Some("2026-04-14".to_string()));
        assert_eq!(world.seed, Some("12345".to_string()));
    }

    #[test]
    fn test_error_handling() {
        let mut menu = MainMenuScreen::new();
        assert!(menu.error_message.is_none());

        menu.set_error("Test error");
        assert_eq!(menu.error_message, Some("Test error".to_string()));

        menu.clear_error();
        assert!(menu.error_message.is_none());
    }

    #[test]
    fn test_back_to_main() {
        let mut menu = MainMenuScreen::new();
        menu.view = MainMenuView::Singleplayer;
        menu.error_message = Some("Error".to_string());
        menu.pending_delete = Some("world1".to_string());

        menu.back_to_main();
        assert_eq!(menu.view(), MainMenuView::Main);
        assert!(menu.error_message.is_none());
        assert!(menu.pending_delete.is_none());
    }

    #[test]
    fn test_main_menu_view_default() {
        let view = MainMenuView::default();
        assert_eq!(view, MainMenuView::Main);
    }
}
