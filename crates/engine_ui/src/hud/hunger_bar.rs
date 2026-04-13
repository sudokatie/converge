//! Hunger bar HUD element.
//!
//! Displays player hunger as 10 drumsticks with half-drumstick precision.
//! Shakes when hunger is critically low.

use egui::{Color32, Pos2, Rect, Rounding, Vec2};

/// Number of drumstick icons.
pub const DRUMSTICK_COUNT: usize = 10;

/// Drumstick icon dimensions.
const DRUMSTICK_SIZE: f32 = 18.0;
const DRUMSTICK_SPACING: f32 = 2.0;
const BAR_PADDING: f32 = 4.0;

/// Hunger threshold for shake effect.
const SHAKE_THRESHOLD: f32 = 3.0;

/// Drumstick colors.
const DRUMSTICK_FULL: Color32 = Color32::from_rgb(180, 130, 70);
const DRUMSTICK_EMPTY: Color32 = Color32::from_rgb(60, 50, 40);

/// Background color.
const BAR_BG: Color32 = Color32::from_rgba_premultiplied(20, 20, 20, 180);

/// Hunger bar state for animations.
#[derive(Clone, Debug, Default)]
pub struct HungerBarState {
    /// Shake animation timer.
    shake_timer: f32,
    /// Current shake offset.
    shake_offset: f32,
}

impl HungerBarState {
    /// Create a new hunger bar state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the hunger bar state.
    ///
    /// Returns the current shake offset for rendering.
    pub fn update(&mut self, current_hunger: f32, dt: f32) -> f32 {
        if current_hunger <= SHAKE_THRESHOLD && current_hunger > 0.0 {
            self.shake_timer += dt * 15.0; // Shake frequency
            self.shake_offset = (self.shake_timer.sin() * 2.0).round();
        } else {
            self.shake_timer = 0.0;
            self.shake_offset = 0.0;
        }

        self.shake_offset
    }

    /// Get the current shake offset.
    #[must_use]
    pub fn shake_offset(&self) -> f32 {
        self.shake_offset
    }

    /// Check if currently shaking.
    #[must_use]
    pub fn is_shaking(&self) -> bool {
        self.shake_offset.abs() > 0.1
    }
}

/// Draw the hunger bar HUD.
///
/// # Arguments
/// * `ctx` - egui context
/// * `current` - Current hunger (0.0 to max)
/// * `max` - Maximum hunger (typically 20.0)
/// * `state` - Animation state
pub fn draw_hunger_bar(
    ctx: &egui::Context,
    current: f32,
    max: f32,
    state: &HungerBarState,
) {
    let screen_rect = ctx.screen_rect();

    // Position at top-right (opposite of health bar)
    let total_width = DRUMSTICK_COUNT as f32 * (DRUMSTICK_SIZE + DRUMSTICK_SPACING) - DRUMSTICK_SPACING;
    let bar_x = screen_rect.width() - total_width - BAR_PADDING * 2.0 - 10.0;
    let bar_y = BAR_PADDING + 10.0 + state.shake_offset();

    egui::Area::new(egui::Id::new("hunger_bar"))
        .fixed_pos(Pos2::new(bar_x, bar_y))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let bar_rect = Rect::from_min_size(
                Pos2::ZERO,
                Vec2::new(total_width + BAR_PADDING * 2.0, DRUMSTICK_SIZE + BAR_PADDING * 2.0),
            );

            let painter = ui.painter();

            // Background
            painter.rect_filled(bar_rect, Rounding::same(4.0), BAR_BG);

            // Calculate half-drumsticks
            let half_drumsticks = ((current / max) * (DRUMSTICK_COUNT as f32 * 2.0)).round() as i32;

            // Draw drumsticks (right to left to match typical game UI)
            for i in 0..DRUMSTICK_COUNT {
                // Draw from right to left
                let reversed_i = DRUMSTICK_COUNT - 1 - i;
                let drumstick_x = BAR_PADDING + reversed_i as f32 * (DRUMSTICK_SIZE + DRUMSTICK_SPACING);
                let drumstick_rect = Rect::from_min_size(
                    Pos2::new(drumstick_x, BAR_PADDING),
                    Vec2::splat(DRUMSTICK_SIZE),
                );

                let drumstick_index = i as i32;
                let full_threshold = drumstick_index * 2 + 2;
                let half_threshold = drumstick_index * 2 + 1;

                let is_full = half_drumsticks >= full_threshold;
                let is_half = half_drumsticks >= half_threshold && !is_full;

                draw_drumstick(painter, drumstick_rect, is_full, is_half);
            }
        });
}

/// Draw a single drumstick icon.
fn draw_drumstick(painter: &egui::Painter, rect: Rect, is_full: bool, is_half: bool) {
    if is_full {
        painter.rect_filled(rect, Rounding::same(2.0), DRUMSTICK_FULL);
    } else if is_half {
        // Draw half drumstick (left half filled)
        let left_rect = Rect::from_min_max(
            rect.min,
            Pos2::new(rect.center().x, rect.max.y),
        );
        let right_rect = Rect::from_min_max(
            Pos2::new(rect.center().x, rect.min.y),
            rect.max,
        );

        painter.rect_filled(left_rect, Rounding::same(2.0), DRUMSTICK_FULL);
        painter.rect_filled(right_rect, Rounding::same(2.0), DRUMSTICK_EMPTY);
    } else {
        painter.rect_filled(rect, Rounding::same(2.0), DRUMSTICK_EMPTY);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hunger_bar_state_new() {
        let state = HungerBarState::new();
        assert!(!state.is_shaking());
        assert_eq!(state.shake_offset(), 0.0);
    }

    #[test]
    fn test_low_hunger_triggers_shake() {
        let mut state = HungerBarState::new();

        // Update with low hunger
        state.update(2.0, 0.1);
        state.update(2.0, 0.1);
        state.update(2.0, 0.1);

        // Should be shaking (may need multiple updates to get non-zero offset)
        assert!(state.shake_timer > 0.0);
    }

    #[test]
    fn test_normal_hunger_no_shake() {
        let mut state = HungerBarState::new();

        state.update(15.0, 0.1);

        assert!(!state.is_shaking());
        assert_eq!(state.shake_offset(), 0.0);
    }

    #[test]
    fn test_empty_hunger_no_shake() {
        let mut state = HungerBarState::new();

        // Empty hunger (0) shouldn't shake either
        state.update(0.0, 0.1);

        assert_eq!(state.shake_offset(), 0.0);
    }

    #[test]
    fn test_shake_stops_when_hunger_restored() {
        let mut state = HungerBarState::new();

        // Low hunger causes shake
        state.update(2.0, 0.5);

        // Restore hunger
        state.update(15.0, 0.1);

        assert!(!state.is_shaking());
        assert_eq!(state.shake_timer, 0.0);
    }
}
