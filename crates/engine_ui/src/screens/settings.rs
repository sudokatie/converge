//! Settings screen UI.
//!
//! Provides video, audio, and control settings.

use egui::{Color32, RichText, ScrollArea, Slider, Vec2};

/// Settings tab selection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SettingsTab {
    #[default]
    Video,
    Audio,
    Controls,
}

/// Actions returned by the settings screen.
#[derive(Clone, Debug, PartialEq)]
pub enum SettingsAction {
    /// Close settings without saving.
    Cancel,
    /// Apply and save settings.
    Apply,
    /// Change a video setting.
    VideoChanged(VideoSetting),
    /// Change an audio setting.
    AudioChanged(AudioSetting),
    /// Change a control setting.
    ControlChanged(ControlSetting),
    /// Start rebinding a key.
    StartRebind(String),
}

/// Video setting changes.
#[derive(Clone, Debug, PartialEq)]
pub enum VideoSetting {
    Fullscreen(bool),
    Vsync(bool),
    ViewDistance(i32),
    Fov(f32),
    Resolution(u32, u32),
}

/// Audio setting changes.
#[derive(Clone, Debug, PartialEq)]
pub enum AudioSetting {
    MasterVolume(f32),
    EffectsVolume(f32),
    MusicVolume(f32),
    AmbientVolume(f32),
    UiVolume(f32),
}

/// Control setting changes.
#[derive(Clone, Debug, PartialEq)]
pub enum ControlSetting {
    MouseSensitivity(f32),
    InvertY(bool),
    Rebind { action: String, key: String },
}

/// Video settings state.
#[derive(Clone, Debug)]
pub struct VideoSettings {
    /// Fullscreen mode.
    pub fullscreen: bool,
    /// VSync enabled.
    pub vsync: bool,
    /// View distance in chunks.
    pub view_distance: i32,
    /// Field of view in degrees.
    pub fov: f32,
    /// Resolution width.
    pub width: u32,
    /// Resolution height.
    pub height: u32,
}

impl Default for VideoSettings {
    fn default() -> Self {
        Self {
            fullscreen: false,
            vsync: true,
            view_distance: 12,
            fov: 70.0,
            width: 1920,
            height: 1080,
        }
    }
}

/// Audio settings state.
#[derive(Clone, Debug)]
pub struct AudioSettings {
    /// Master volume (0.0 - 1.0).
    pub master: f32,
    /// Effects volume.
    pub effects: f32,
    /// Music volume.
    pub music: f32,
    /// Ambient volume.
    pub ambient: f32,
    /// UI sounds volume.
    pub ui: f32,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            master: 1.0,
            effects: 1.0,
            music: 0.7,
            ambient: 0.8,
            ui: 1.0,
        }
    }
}

/// Control binding display.
#[derive(Clone, Debug)]
pub struct ControlBinding {
    /// Action name for display.
    pub name: String,
    /// Action ID for events.
    pub action_id: String,
    /// Current primary binding.
    pub primary: String,
    /// Current secondary binding (optional).
    pub secondary: Option<String>,
}

impl ControlBinding {
    /// Create a new control binding entry.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        action_id: impl Into<String>,
        primary: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            action_id: action_id.into(),
            primary: primary.into(),
            secondary: None,
        }
    }

    /// Add a secondary binding.
    pub fn with_secondary(mut self, key: impl Into<String>) -> Self {
        self.secondary = Some(key.into());
        self
    }
}

/// Control settings state.
#[derive(Clone, Debug)]
pub struct ControlSettings {
    /// Mouse sensitivity.
    pub mouse_sensitivity: f32,
    /// Invert Y axis.
    pub invert_y: bool,
    /// Key bindings.
    pub bindings: Vec<ControlBinding>,
}

impl Default for ControlSettings {
    fn default() -> Self {
        Self {
            mouse_sensitivity: 0.1,
            invert_y: false,
            bindings: vec![
                ControlBinding::new("Move Forward", "move_forward", "W"),
                ControlBinding::new("Move Back", "move_back", "S"),
                ControlBinding::new("Move Left", "move_left", "A"),
                ControlBinding::new("Move Right", "move_right", "D"),
                ControlBinding::new("Jump", "jump", "Space"),
                ControlBinding::new("Crouch", "crouch", "Left Ctrl"),
                ControlBinding::new("Sprint", "sprint", "Left Shift"),
                ControlBinding::new("Attack", "attack", "Mouse 1"),
                ControlBinding::new("Use Item", "use_item", "Mouse 2"),
                ControlBinding::new("Interact", "interact", "E"),
                ControlBinding::new("Inventory", "inventory", "Tab"),
                ControlBinding::new("Pause", "pause", "Escape"),
                ControlBinding::new("Chat", "chat", "T"),
            ],
        }
    }
}

/// Settings screen state.
#[derive(Clone, Debug, Default)]
pub struct SettingsScreen {
    /// Whether screen is open.
    is_open: bool,
    /// Current tab.
    tab: SettingsTab,
    /// Video settings.
    video: VideoSettings,
    /// Audio settings.
    audio: AudioSettings,
    /// Control settings.
    controls: ControlSettings,
    /// Currently rebinding action (if any).
    rebinding: Option<String>,
    /// Has unsaved changes.
    has_changes: bool,
}

impl SettingsScreen {
    /// Create a new settings screen.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if screen is open.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    /// Open the settings screen.
    pub fn open(&mut self) {
        self.is_open = true;
        self.has_changes = false;
    }

    /// Close the settings screen.
    pub fn close(&mut self) {
        self.is_open = false;
        self.rebinding = None;
    }

    /// Toggle the settings screen.
    pub fn toggle(&mut self) {
        if self.is_open {
            self.close();
        } else {
            self.open();
        }
    }

    /// Get current video settings.
    #[must_use]
    pub fn video(&self) -> &VideoSettings {
        &self.video
    }

    /// Get current audio settings.
    #[must_use]
    pub fn audio(&self) -> &AudioSettings {
        &self.audio
    }

    /// Get current control settings.
    #[must_use]
    pub fn controls(&self) -> &ControlSettings {
        &self.controls
    }

    /// Set video settings.
    pub fn set_video(&mut self, video: VideoSettings) {
        self.video = video;
    }

    /// Set audio settings.
    pub fn set_audio(&mut self, audio: AudioSettings) {
        self.audio = audio;
    }

    /// Set control settings.
    pub fn set_controls(&mut self, controls: ControlSettings) {
        self.controls = controls;
    }

    /// Check if waiting for key rebind.
    #[must_use]
    pub fn is_rebinding(&self) -> bool {
        self.rebinding.is_some()
    }

    /// Get action currently being rebound.
    #[must_use]
    pub fn rebinding_action(&self) -> Option<&str> {
        self.rebinding.as_deref()
    }

    /// Complete a rebind with the pressed key.
    pub fn complete_rebind(&mut self, key: impl Into<String>) -> Option<ControlSetting> {
        if let Some(action) = self.rebinding.take() {
            let key = key.into();
            // Update the binding in our list
            for binding in &mut self.controls.bindings {
                if binding.action_id == action {
                    binding.primary = key.clone();
                    break;
                }
            }
            self.has_changes = true;
            Some(ControlSetting::Rebind { action, key })
        } else {
            None
        }
    }

    /// Cancel rebinding.
    pub fn cancel_rebind(&mut self) {
        self.rebinding = None;
    }

    /// Draw the settings screen and return any action.
    #[must_use]
    pub fn draw(&mut self, ctx: &egui::Context) -> Option<SettingsAction> {
        if !self.is_open {
            return None;
        }

        let mut action = None;

        egui::Window::new("Settings")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .fixed_size(Vec2::new(600.0, 450.0))
            .show(ctx, |ui| {
                // Tab bar
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(self.tab == SettingsTab::Video, "Video")
                        .clicked()
                    {
                        self.tab = SettingsTab::Video;
                    }
                    if ui
                        .selectable_label(self.tab == SettingsTab::Audio, "Audio")
                        .clicked()
                    {
                        self.tab = SettingsTab::Audio;
                    }
                    if ui
                        .selectable_label(self.tab == SettingsTab::Controls, "Controls")
                        .clicked()
                    {
                        self.tab = SettingsTab::Controls;
                    }
                });

                ui.separator();

                // Tab content
                ScrollArea::vertical()
                    .max_height(340.0)
                    .show(ui, |ui| match self.tab {
                        SettingsTab::Video => {
                            action = self.draw_video_tab(ui);
                        }
                        SettingsTab::Audio => {
                            action = self.draw_audio_tab(ui);
                        }
                        SettingsTab::Controls => {
                            action = self.draw_controls_tab(ui);
                        }
                    });

                ui.separator();

                // Bottom buttons
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.close();
                        action = Some(SettingsAction::Cancel);
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Apply").clicked() {
                            self.has_changes = false;
                            action = Some(SettingsAction::Apply);
                        }
                    });
                });
            });

        action
    }

    fn draw_video_tab(&mut self, ui: &mut egui::Ui) -> Option<SettingsAction> {
        let mut action = None;

        ui.heading("Display");
        ui.add_space(8.0);

        // Fullscreen
        if ui.checkbox(&mut self.video.fullscreen, "Fullscreen").changed() {
            self.has_changes = true;
            action = Some(SettingsAction::VideoChanged(VideoSetting::Fullscreen(
                self.video.fullscreen,
            )));
        }

        // VSync
        if ui.checkbox(&mut self.video.vsync, "VSync").changed() {
            self.has_changes = true;
            action = Some(SettingsAction::VideoChanged(VideoSetting::Vsync(
                self.video.vsync,
            )));
        }

        ui.add_space(16.0);
        ui.heading("Graphics");
        ui.add_space(8.0);

        // View distance
        ui.horizontal(|ui| {
            ui.label("View Distance:");
            let slider = Slider::new(&mut self.video.view_distance, 4..=24)
                .suffix(" chunks")
                .clamp_to_range(true);
            if ui.add(slider).changed() {
                self.has_changes = true;
                action = Some(SettingsAction::VideoChanged(VideoSetting::ViewDistance(
                    self.video.view_distance,
                )));
            }
        });

        // FOV
        ui.horizontal(|ui| {
            ui.label("Field of View:");
            let slider = Slider::new(&mut self.video.fov, 60.0..=120.0)
                .suffix("deg")
                .clamp_to_range(true);
            if ui.add(slider).changed() {
                self.has_changes = true;
                action = Some(SettingsAction::VideoChanged(VideoSetting::Fov(
                    self.video.fov,
                )));
            }
        });

        action
    }

    fn draw_audio_tab(&mut self, ui: &mut egui::Ui) -> Option<SettingsAction> {
        let mut action = None;

        ui.heading("Volume");
        ui.add_space(8.0);

        // Master volume
        ui.horizontal(|ui| {
            ui.label("Master:");
            let slider = Slider::new(&mut self.audio.master, 0.0..=1.0)
                .show_value(false)
                .clamp_to_range(true);
            if ui.add(slider).changed() {
                self.has_changes = true;
                action = Some(SettingsAction::AudioChanged(AudioSetting::MasterVolume(
                    self.audio.master,
                )));
            }
            ui.label(format!("{}%", (self.audio.master * 100.0) as i32));
        });

        // Effects volume
        ui.horizontal(|ui| {
            ui.label("Effects:");
            let slider = Slider::new(&mut self.audio.effects, 0.0..=1.0)
                .show_value(false)
                .clamp_to_range(true);
            if ui.add(slider).changed() {
                self.has_changes = true;
                action = Some(SettingsAction::AudioChanged(AudioSetting::EffectsVolume(
                    self.audio.effects,
                )));
            }
            ui.label(format!("{}%", (self.audio.effects * 100.0) as i32));
        });

        // Music volume
        ui.horizontal(|ui| {
            ui.label("Music:");
            let slider = Slider::new(&mut self.audio.music, 0.0..=1.0)
                .show_value(false)
                .clamp_to_range(true);
            if ui.add(slider).changed() {
                self.has_changes = true;
                action = Some(SettingsAction::AudioChanged(AudioSetting::MusicVolume(
                    self.audio.music,
                )));
            }
            ui.label(format!("{}%", (self.audio.music * 100.0) as i32));
        });

        // Ambient volume
        ui.horizontal(|ui| {
            ui.label("Ambient:");
            let slider = Slider::new(&mut self.audio.ambient, 0.0..=1.0)
                .show_value(false)
                .clamp_to_range(true);
            if ui.add(slider).changed() {
                self.has_changes = true;
                action = Some(SettingsAction::AudioChanged(AudioSetting::AmbientVolume(
                    self.audio.ambient,
                )));
            }
            ui.label(format!("{}%", (self.audio.ambient * 100.0) as i32));
        });

        // UI volume
        ui.horizontal(|ui| {
            ui.label("Interface:");
            let slider = Slider::new(&mut self.audio.ui, 0.0..=1.0)
                .show_value(false)
                .clamp_to_range(true);
            if ui.add(slider).changed() {
                self.has_changes = true;
                action = Some(SettingsAction::AudioChanged(AudioSetting::UiVolume(
                    self.audio.ui,
                )));
            }
            ui.label(format!("{}%", (self.audio.ui * 100.0) as i32));
        });

        action
    }

    fn draw_controls_tab(&mut self, ui: &mut egui::Ui) -> Option<SettingsAction> {
        let mut action = None;

        ui.heading("Mouse");
        ui.add_space(8.0);

        // Mouse sensitivity
        ui.horizontal(|ui| {
            ui.label("Sensitivity:");
            let slider = Slider::new(&mut self.controls.mouse_sensitivity, 0.01..=1.0)
                .logarithmic(true)
                .clamp_to_range(true);
            if ui.add(slider).changed() {
                self.has_changes = true;
                action = Some(SettingsAction::ControlChanged(
                    ControlSetting::MouseSensitivity(self.controls.mouse_sensitivity),
                ));
            }
        });

        // Invert Y
        if ui
            .checkbox(&mut self.controls.invert_y, "Invert Y Axis")
            .changed()
        {
            self.has_changes = true;
            action = Some(SettingsAction::ControlChanged(ControlSetting::InvertY(
                self.controls.invert_y,
            )));
        }

        ui.add_space(16.0);
        ui.heading("Key Bindings");
        ui.add_space(8.0);

        // Rebinding overlay
        if let Some(ref action_id) = self.rebinding {
            ui.label(
                RichText::new(format!("Press a key for '{}'...", action_id))
                    .color(Color32::YELLOW),
            );
            ui.label(RichText::new("Press Escape to cancel").color(Color32::GRAY));
            ui.add_space(8.0);
        }

        // Key bindings grid
        egui::Grid::new("controls_grid")
            .num_columns(3)
            .spacing([20.0, 4.0])
            .show(ui, |ui| {
                ui.label(RichText::new("Action").strong());
                ui.label(RichText::new("Key").strong());
                ui.label("");
                ui.end_row();

                let bindings = self.controls.bindings.clone();
                for binding in &bindings {
                    ui.label(&binding.name);
                    ui.label(&binding.primary);

                    let is_rebinding = self
                        .rebinding
                        .as_ref()
                        .map_or(false, |a| a == &binding.action_id);

                    if is_rebinding {
                        if ui.button("...").clicked() {
                            self.rebinding = None;
                        }
                    } else if ui.button("Rebind").clicked() {
                        self.rebinding = Some(binding.action_id.clone());
                        action = Some(SettingsAction::StartRebind(binding.action_id.clone()));
                    }
                    ui.end_row();
                }
            });

        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_screen_new() {
        let screen = SettingsScreen::new();
        assert!(!screen.is_open());
        assert_eq!(screen.tab, SettingsTab::Video);
        assert!(!screen.is_rebinding());
    }

    #[test]
    fn test_settings_screen_open_close() {
        let mut screen = SettingsScreen::new();
        screen.open();
        assert!(screen.is_open());
        screen.close();
        assert!(!screen.is_open());
    }

    #[test]
    fn test_settings_screen_toggle() {
        let mut screen = SettingsScreen::new();
        assert!(!screen.is_open());
        screen.toggle();
        assert!(screen.is_open());
        screen.toggle();
        assert!(!screen.is_open());
    }

    #[test]
    fn test_video_settings_default() {
        let video = VideoSettings::default();
        assert!(!video.fullscreen);
        assert!(video.vsync);
        assert_eq!(video.view_distance, 12);
        assert!((video.fov - 70.0).abs() < 0.001);
    }

    #[test]
    fn test_audio_settings_default() {
        let audio = AudioSettings::default();
        assert!((audio.master - 1.0).abs() < 0.001);
        assert!((audio.effects - 1.0).abs() < 0.001);
        assert!((audio.music - 0.7).abs() < 0.001);
        assert!((audio.ambient - 0.8).abs() < 0.001);
        assert!((audio.ui - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_control_settings_default() {
        let controls = ControlSettings::default();
        assert!((controls.mouse_sensitivity - 0.1).abs() < 0.001);
        assert!(!controls.invert_y);
        assert!(!controls.bindings.is_empty());
    }

    #[test]
    fn test_control_binding_new() {
        let binding = ControlBinding::new("Jump", "jump", "Space");
        assert_eq!(binding.name, "Jump");
        assert_eq!(binding.action_id, "jump");
        assert_eq!(binding.primary, "Space");
        assert!(binding.secondary.is_none());
    }

    #[test]
    fn test_control_binding_with_secondary() {
        let binding = ControlBinding::new("Jump", "jump", "Space").with_secondary("W");
        assert_eq!(binding.secondary, Some("W".to_string()));
    }

    #[test]
    fn test_rebinding_flow() {
        let mut screen = SettingsScreen::new();
        assert!(!screen.is_rebinding());

        // Start rebind
        screen.rebinding = Some("jump".to_string());
        assert!(screen.is_rebinding());
        assert_eq!(screen.rebinding_action(), Some("jump"));

        // Complete rebind
        let result = screen.complete_rebind("X");
        assert!(result.is_some());
        assert!(!screen.is_rebinding());

        // Verify binding updated
        let binding = screen
            .controls
            .bindings
            .iter()
            .find(|b| b.action_id == "jump")
            .unwrap();
        assert_eq!(binding.primary, "X");
    }

    #[test]
    fn test_cancel_rebind() {
        let mut screen = SettingsScreen::new();
        screen.rebinding = Some("jump".to_string());
        assert!(screen.is_rebinding());
        screen.cancel_rebind();
        assert!(!screen.is_rebinding());
    }

    #[test]
    fn test_set_settings() {
        let mut screen = SettingsScreen::new();

        let video = VideoSettings {
            fullscreen: true,
            vsync: false,
            view_distance: 16,
            fov: 90.0,
            width: 2560,
            height: 1440,
        };
        screen.set_video(video.clone());
        assert!(screen.video().fullscreen);
        assert!(!screen.video().vsync);
        assert_eq!(screen.video().view_distance, 16);

        let audio = AudioSettings {
            master: 0.5,
            effects: 0.6,
            music: 0.4,
            ambient: 0.3,
            ui: 0.8,
        };
        screen.set_audio(audio.clone());
        assert!((screen.audio().master - 0.5).abs() < 0.001);

        let controls = ControlSettings {
            mouse_sensitivity: 0.2,
            invert_y: true,
            bindings: vec![],
        };
        screen.set_controls(controls.clone());
        assert!(screen.controls().invert_y);
    }
}
