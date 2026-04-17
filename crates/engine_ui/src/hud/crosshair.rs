//! Crosshair rendering for first-person view.
//!
//! Draws a centered crosshair with configurable style.

use egui::{Color32, Pos2, Rect, Rounding, Shape};

/// Crosshair style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrosshairStyle {
    /// Simple plus sign (+).
    Cross,
    /// Single center dot.
    Dot,
    /// Gap in the center with arms (standard FPS style).
    Gap,
}

impl Default for CrosshairStyle {
    fn default() -> Self {
        Self::Gap
    }
}

/// Crosshair configuration.
#[derive(Debug, Clone, Copy)]
pub struct CrosshairConfig {
    /// Crosshair style.
    pub style: CrosshairStyle,
    /// Color of the crosshair.
    pub color: Color32,
    /// Size of the crosshair arms in pixels.
    pub size: f32,
    /// Thickness of the crosshair lines in pixels.
    pub thickness: f32,
    /// Gap size (only used for Gap style).
    pub gap: f32,
    /// Dot radius (only used for Dot style).
    pub dot_radius: f32,
}

impl Default for CrosshairConfig {
    fn default() -> Self {
        Self {
            style: CrosshairStyle::Gap,
            color: Color32::WHITE,
            size: 6.0,
            thickness: 2.0,
            gap: 3.0,
            dot_radius: 2.0,
        }
    }
}

impl CrosshairConfig {
    /// Create a new crosshair configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the crosshair style.
    #[must_use]
    pub fn with_style(mut self, style: CrosshairStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the crosshair color.
    #[must_use]
    pub fn with_color(mut self, color: Color32) -> Self {
        self.color = color;
        self
    }
}

/// Draw the crosshair at the center of the screen.
pub fn draw_crosshair(ctx: &egui::Context, config: &CrosshairConfig) {
    let screen = ctx.screen_rect();
    let center = screen.center();

    let shapes = match config.style {
        CrosshairStyle::Cross => draw_cross(center, config),
        CrosshairStyle::Dot => draw_dot(center, config),
        CrosshairStyle::Gap => draw_gap(center, config),
    };

    // Draw on top of everything
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("crosshair"),
    ));

    for shape in shapes {
        painter.add(shape);
    }
}

/// Draw a plus-sign crosshair.
fn draw_cross(center: Pos2, config: &CrosshairConfig) -> Vec<Shape> {
    let s = config.size;
    let t = config.thickness * 0.5;

    // Horizontal bar
    let h_rect = Rect::from_min_max(
        Pos2::new(center.x - s, center.y - t),
        Pos2::new(center.x + s, center.y + t),
    );

    // Vertical bar
    let v_rect = Rect::from_min_max(
        Pos2::new(center.x - t, center.y - s),
        Pos2::new(center.x + t, center.y + s),
    );

    vec![
        Shape::rect_filled(h_rect, Rounding::ZERO, config.color),
        Shape::rect_filled(v_rect, Rounding::ZERO, config.color),
    ]
}

/// Draw a single dot crosshair.
fn draw_dot(center: Pos2, config: &CrosshairConfig) -> Vec<Shape> {
    let r = config.dot_radius;
    let rect = Rect::from_center_size(center, egui::vec2(r * 2.0, r * 2.0));

    vec![Shape::rect_filled(rect, Rounding::same(r), config.color)]
}

/// Draw a gap-style crosshair (standard FPS).
fn draw_gap(center: Pos2, config: &CrosshairConfig) -> Vec<Shape> {
    let s = config.size;
    let g = config.gap;
    let t = config.thickness * 0.5;

    // Top arm
    let top = Rect::from_min_max(
        Pos2::new(center.x - t, center.y - g - s),
        Pos2::new(center.x + t, center.y - g),
    );

    // Bottom arm
    let bottom = Rect::from_min_max(
        Pos2::new(center.x - t, center.y + g),
        Pos2::new(center.x + t, center.y + g + s),
    );

    // Left arm
    let left = Rect::from_min_max(
        Pos2::new(center.x - g - s, center.y - t),
        Pos2::new(center.x - g, center.y + t),
    );

    // Right arm
    let right = Rect::from_min_max(
        Pos2::new(center.x + g, center.y - t),
        Pos2::new(center.x + g + s, center.y + t),
    );

    vec![
        Shape::rect_filled(top, Rounding::ZERO, config.color),
        Shape::rect_filled(bottom, Rounding::ZERO, config.color),
        Shape::rect_filled(left, Rounding::ZERO, config.color),
        Shape::rect_filled(right, Rounding::ZERO, config.color),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_is_gap_style() {
        let config = CrosshairConfig::default();
        assert_eq!(config.style, CrosshairStyle::Gap);
        assert_eq!(config.color, Color32::WHITE);
    }

    #[test]
    fn test_builder_pattern() {
        let config = CrosshairConfig::new()
            .with_style(CrosshairStyle::Dot)
            .with_color(Color32::RED);

        assert_eq!(config.style, CrosshairStyle::Dot);
        assert_eq!(config.color, Color32::RED);
    }

    #[test]
    fn test_cross_shapes_valid() {
        let config = CrosshairConfig {
            style: CrosshairStyle::Cross,
            ..Default::default()
        };
        let shapes = draw_cross(Pos2::new(400.0, 300.0), &config);
        assert_eq!(shapes.len(), 2, "Cross should have 2 rectangles");
    }

    #[test]
    fn test_dot_shapes_valid() {
        let config = CrosshairConfig {
            style: CrosshairStyle::Dot,
            ..Default::default()
        };
        let shapes = draw_dot(Pos2::new(400.0, 300.0), &config);
        assert_eq!(shapes.len(), 1, "Dot should have 1 shape");
    }

    #[test]
    fn test_gap_shapes_valid() {
        let config = CrosshairConfig {
            style: CrosshairStyle::Gap,
            ..Default::default()
        };
        let shapes = draw_gap(Pos2::new(400.0, 300.0), &config);
        assert_eq!(shapes.len(), 4, "Gap should have 4 arm rectangles");
    }

    #[test]
    fn test_crosshair_style_equality() {
        assert_eq!(CrosshairStyle::Cross, CrosshairStyle::Cross);
        assert_ne!(CrosshairStyle::Cross, CrosshairStyle::Dot);
    }
}
