//! Pause menu screen.
//!
//! Overlay shown when the game is paused. Provides resume, settings,
//! and quit to main menu options.

use egui::{Color32, RichText, Vec2};

/// Action returned by the pause menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseAction {
    /// Close the pause menu and resume playing.
    Resume,
    /// Open the settings screen.
    Settings,
    /// Quit to the main menu.
    QuitToMenu,
    /// Quit the application entirely.
    QuitApp,
}

/// Pause menu state.
#[derive(Debug, Clone, Default)]
pub struct PauseMenu {
    /// Whether the "quit to menu" confirmation is shown.
    confirm_quit: bool,
}

impl PauseMenu {
    /// Create a new pause menu.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset state (e.g., when reopening).
    pub fn reset(&mut self) {
        self.confirm_quit = false;
    }

    /// Draw the pause menu and return any action.
    pub fn draw(&mut self, ctx: &egui::Context) -> Option<PauseAction> {
        let mut action = None;

        // Dark overlay behind the menu
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(Color32::from_black_alpha(180)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(80.0);

                    // Title
                    ui.label(
                        RichText::new("Game Paused")
                            .size(32.0)
                            .color(Color32::WHITE),
                    );
                    ui.add_space(40.0);

                    let button_size = Vec2::new(240.0, 40.0);

                    if self.confirm_quit {
                        action = self.draw_confirm_quit(ui, button_size);
                    } else {
                        action = self.draw_main_buttons(ui, button_size);
                    }
                });
            });

        action
    }

    fn draw_main_buttons(&mut self, ui: &mut egui::Ui, button_size: Vec2) -> Option<PauseAction> {
        // Resume button
        if ui
            .add_sized(button_size, egui::Button::new(RichText::new("Resume").size(18.0)))
            .clicked()
        {
            return Some(PauseAction::Resume);
        }

        ui.add_space(12.0);

        // Settings button
        if ui
            .add_sized(button_size, egui::Button::new(RichText::new("Settings").size(18.0)))
            .clicked()
        {
            return Some(PauseAction::Settings);
        }

        ui.add_space(12.0);

        // Quit to menu button
        if ui
            .add_sized(
                button_size,
                egui::Button::new(RichText::new("Quit to Menu").size(18.0)),
            )
            .clicked()
        {
            self.confirm_quit = true;
        }

        ui.add_space(12.0);

        // Quit app button
        if ui
            .add_sized(
                button_size,
                egui::Button::new(RichText::new("Quit Game").size(18.0)),
            )
            .clicked()
        {
            return Some(PauseAction::QuitApp);
        }

        None
    }

    fn draw_confirm_quit(&mut self, ui: &mut egui::Ui, button_size: Vec2) -> Option<PauseAction> {
        ui.label(
            RichText::new("Quit to main menu?")
                .size(20.0)
                .color(Color32::YELLOW),
        );
        ui.add_space(8.0);
        ui.label(
            RichText::new("Unsaved progress will be lost.")
                .size(14.0)
                .color(Color32::GRAY),
        );

        ui.add_space(20.0);

        let half_width = Vec2::new(button_size.x * 0.48, button_size.y);

        let mut result = None;
        ui.horizontal(|ui| {
            if ui
                .add_sized(half_width, egui::Button::new("Cancel"))
                .clicked()
            {
                self.confirm_quit = false;
            }

            if ui
                .add_sized(half_width, egui::Button::new("Quit"))
                .clicked()
            {
                result = Some(PauseAction::QuitToMenu);
            }
        });

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_pause_menu() {
        let menu = PauseMenu::new();
        assert!(!menu.confirm_quit);
    }

    #[test]
    fn test_reset_clears_confirm() {
        let mut menu = PauseMenu::new();
        menu.confirm_quit = true;
        menu.reset();
        assert!(!menu.confirm_quit);
    }

    #[test]
    fn test_pause_action_variants() {
        assert_eq!(PauseAction::Resume, PauseAction::Resume);
        assert_ne!(PauseAction::Resume, PauseAction::Settings);
        assert_ne!(PauseAction::QuitToMenu, PauseAction::QuitApp);
    }

    #[test]
    fn test_confirm_quit_flow() {
        let mut menu = PauseMenu::new();
        assert!(!menu.confirm_quit);

        // Simulate clicking "Quit to Menu"
        menu.confirm_quit = true;
        assert!(menu.confirm_quit);

        // Simulate clicking "Cancel"
        menu.confirm_quit = false;
        assert!(!menu.confirm_quit);
    }
}
