//! Health bar HUD element.
//!
//! Displays player health as 10 hearts with half-heart precision.

use egui::{Color32, Pos2, Rect, Rounding, Vec2};

/// Number of heart icons.
pub const HEART_COUNT: usize = 10;

/// Heart icon dimensions.
const HEART_SIZE: f32 = 18.0;
const HEART_SPACING: f32 = 2.0;
const BAR_PADDING: f32 = 4.0;

/// Heart colors.
const HEART_FULL: Color32 = Color32::from_rgb(200, 40, 40);
const HEART_HALF: Color32 = Color32::from_rgb(200, 40, 40);
const HEART_EMPTY: Color32 = Color32::from_rgb(60, 30, 30);
const HEART_FLASH: Color32 = Color32::from_rgb(255, 100, 100);
const HEART_POISON: Color32 = Color32::from_rgb(80, 160, 60);

/// Background color.
const BAR_BG: Color32 = Color32::from_rgba_premultiplied(20, 20, 20, 180);

/// Health bar state for animations.
#[derive(Clone, Debug, Default)]
pub struct HealthBarState {
    /// Previous health for detecting damage.
    last_health: f32,
    /// Flash timer (counts down).
    flash_timer: f32,
    /// Whether currently flashing.
    is_flashing: bool,
}

impl HealthBarState {
    /// Create a new health bar state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the health bar state.
    ///
    /// Returns true if the bar should flash (took damage).
    pub fn update(&mut self, current_health: f32, dt: f32) -> bool {
        // Detect damage
        if current_health < self.last_health {
            self.flash_timer = 0.5;
            self.is_flashing = true;
        }

        self.last_health = current_health;

        // Update flash timer
        if self.flash_timer > 0.0 {
            self.flash_timer -= dt;
            if self.flash_timer <= 0.0 {
                self.is_flashing = false;
            }
        }

        self.is_flashing
    }

    /// Check if currently flashing.
    #[must_use]
    pub fn is_flashing(&self) -> bool {
        self.is_flashing
    }
}

/// Draw the health bar HUD.
///
/// # Arguments
/// * `ctx` - egui context
/// * `current` - Current health (0.0 to max)
/// * `max` - Maximum health (typically 20.0)
/// * `state` - Animation state
/// * `poisoned` - Whether player is poisoned (green hearts)
pub fn draw_health_bar(
    ctx: &egui::Context,
    current: f32,
    max: f32,
    state: &HealthBarState,
    poisoned: bool,
) {
    let screen_rect = ctx.screen_rect();

    // Position at top-left
    let bar_x = BAR_PADDING + 10.0;
    let bar_y = BAR_PADDING + 10.0;

    egui::Area::new(egui::Id::new("health_bar"))
        .fixed_pos(Pos2::new(bar_x, bar_y))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let total_width = HEART_COUNT as f32 * (HEART_SIZE + HEART_SPACING) - HEART_SPACING;
            let bar_rect = Rect::from_min_size(
                Pos2::ZERO,
                Vec2::new(total_width + BAR_PADDING * 2.0, HEART_SIZE + BAR_PADDING * 2.0),
            );

            let painter = ui.painter();

            // Background
            painter.rect_filled(bar_rect, Rounding::same(4.0), BAR_BG);

            // Calculate half-hearts
            let half_hearts = ((current / max) * (HEART_COUNT as f32 * 2.0)).round() as i32;

            // Draw hearts
            for i in 0..HEART_COUNT {
                let heart_x = BAR_PADDING + i as f32 * (HEART_SIZE + HEART_SPACING);
                let heart_rect = Rect::from_min_size(
                    Pos2::new(heart_x, BAR_PADDING),
                    Vec2::splat(HEART_SIZE),
                );

                let heart_index = i as i32;
                let full_threshold = heart_index * 2 + 2;
                let half_threshold = heart_index * 2 + 1;

                let base_color = if poisoned { HEART_POISON } else { HEART_FULL };
                let flash_color = HEART_FLASH;

                let color = if half_hearts >= full_threshold {
                    // Full heart
                    if state.is_flashing() {
                        flash_color
                    } else {
                        base_color
                    }
                } else if half_hearts >= half_threshold {
                    // Half heart - draw as partial
                    base_color
                } else {
                    HEART_EMPTY
                };

                // Draw heart shape (simplified as rounded rect for now)
                draw_heart(painter, heart_rect, color, half_hearts >= half_threshold && half_hearts < full_threshold);
            }
        });
}

/// Draw a single heart icon.
fn draw_heart(painter: &egui::Painter, rect: Rect, color: Color32, is_half: bool) {
    if is_half {
        // Draw half heart (left half filled, right half empty)
        let left_rect = Rect::from_min_max(
            rect.min,
            Pos2::new(rect.center().x, rect.max.y),
        );
        let right_rect = Rect::from_min_max(
            Pos2::new(rect.center().x, rect.min.y),
            rect.max,
        );

        painter.rect_filled(left_rect, Rounding::same(2.0), color);
        painter.rect_filled(right_rect, Rounding::same(2.0), HEART_EMPTY);
    } else {
        painter.rect_filled(rect, Rounding::same(2.0), color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_bar_state_new() {
        let state = HealthBarState::new();
        assert!(!state.is_flashing());
        assert_eq!(state.flash_timer, 0.0);
    }

    #[test]
    fn test_damage_triggers_flash() {
        let mut state = HealthBarState::new();
        state.last_health = 20.0;

        let should_flash = state.update(15.0, 0.0);

        assert!(should_flash);
        assert!(state.is_flashing());
        assert!(state.flash_timer > 0.0);
    }

    #[test]
    fn test_flash_expires() {
        let mut state = HealthBarState::new();
        state.last_health = 20.0;
        state.update(15.0, 0.0); // Trigger flash

        // Tick past flash duration
        state.update(15.0, 0.6);

        assert!(!state.is_flashing());
    }

    #[test]
    fn test_heal_no_flash() {
        let mut state = HealthBarState::new();
        state.last_health = 10.0;

        let should_flash = state.update(15.0, 0.0); // Healing

        assert!(!should_flash);
        assert!(!state.is_flashing());
    }

    #[test]
    fn test_no_change_no_flash() {
        let mut state = HealthBarState::new();
        state.last_health = 20.0;

        let should_flash = state.update(20.0, 0.1);

        assert!(!should_flash);
    }
}
