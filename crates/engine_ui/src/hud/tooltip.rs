//! Item tooltip rendering.
//!
//! Shows item details on hover: name, description, durability,
//! and enchantment/effect info.

use egui::{Color32, RichText};

/// Tooltip data for an item.
#[derive(Debug, Clone)]
pub struct ItemTooltip {
    /// Item display name.
    pub name: String,
    /// Item category label.
    pub category: String,
    /// Maximum durability (if tool/armor).
    pub max_durability: Option<u32>,
    /// Current durability.
    pub current_durability: Option<u32>,
    /// Mining speed (if tool).
    pub mining_speed: Option<f32>,
    /// Damage (if weapon).
    pub damage: Option<f32>,
    /// Food value (if food).
    pub food_value: Option<f32>,
    /// Stack size.
    pub stack_size: u32,
    /// Additional description lines.
    pub description_lines: Vec<String>,
}

impl ItemTooltip {
    /// Create a minimal tooltip with just a name.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            category: String::new(),
            max_durability: None,
            current_durability: None,
            mining_speed: None,
            damage: None,
            food_value: None,
            stack_size: 1,
            description_lines: Vec::new(),
        }
    }

    /// Set the category.
    #[must_use]
    pub fn with_category(mut self, category: &str) -> Self {
        self.category = category.to_string();
        self
    }

    /// Set durability info.
    #[must_use]
    pub fn with_durability(mut self, current: u32, max: u32) -> Self {
        self.current_durability = Some(current);
        self.max_durability = Some(max);
        self
    }

    /// Set mining speed.
    #[must_use]
    pub fn with_mining_speed(mut self, speed: f32) -> Self {
        self.mining_speed = Some(speed);
        self
    }

    /// Set damage.
    #[must_use]
    pub fn with_damage(mut self, damage: f32) -> Self {
        self.damage = Some(damage);
        self
    }

    /// Set food value.
    #[must_use]
    pub fn with_food_value(mut self, value: f32) -> Self {
        self.food_value = Some(value);
        self
    }

    /// Add a description line.
    #[must_use]
    pub fn with_description(mut self, line: &str) -> Self {
        self.description_lines.push(line.to_string());
        self
    }

    /// Set stack size.
    #[must_use]
    pub fn with_stack_size(mut self, size: u32) -> Self {
        self.stack_size = size;
        self
    }

    /// Get durability fraction (0.0 to 1.0).
    #[must_use]
    pub fn durability_fraction(&self) -> Option<f32> {
        match (self.current_durability, self.max_durability) {
            (Some(cur), Some(max)) if max > 0 => Some(cur as f32 / max as f32),
            _ => None,
        }
    }

    /// Get durability color based on remaining fraction.
    #[must_use]
    pub fn durability_color(&self) -> Option<Color32> {
        let frac = self.durability_fraction()?;

        if frac > 0.7 {
            Some(Color32::GREEN)
        } else if frac > 0.4 {
            Some(Color32::YELLOW)
        } else if frac > 0.15 {
            Some(Color32::from_rgb(255, 140, 0))
        } else {
            Some(Color32::RED)
        }
    }

    /// Check if the tooltip has any detail beyond the name.
    #[must_use]
    pub fn has_details(&self) -> bool {
        self.max_durability.is_some()
            || self.mining_speed.is_some()
            || self.damage.is_some()
            || self.food_value.is_some()
            || !self.description_lines.is_empty()
    }
}

/// Draw an item tooltip at the cursor.
pub fn draw_tooltip(ui: &mut egui::Ui, tooltip: &ItemTooltip) {
    let mut frame = egui::Frame::popup(ui.style());
    frame.fill = Color32::from_rgba_unmultiplied(20, 20, 30, 240);
    frame.stroke = egui::Stroke::new(1.0, Color32::from_rgb(80, 80, 120));

    frame.show(ui, |ui| {
        ui.set_min_width(160.0);

        // Item name
        ui.label(RichText::new(&tooltip.name).size(14.0).color(Color32::WHITE));

        // Category
        if !tooltip.category.is_empty() {
            ui.label(
                RichText::new(&tooltip.category)
                    .size(11.0)
                    .color(Color32::GRAY),
            );
        }

        ui.add_space(4.0);

        // Durability bar
        if let Some(frac) = tooltip.durability_fraction() {
            if let Some(color) = tooltip.durability_color() {
                let label = format!(
                    "Durability: {}/{}",
                    tooltip.current_durability.unwrap(),
                    tooltip.max_durability.unwrap()
                );
                ui.label(RichText::new(label).size(11.0).color(color));

                let bar_width = 140.0;
                let bar_height = 4.0;
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(bar_width, bar_height),
                    egui::Sense::hover(),
                );
                let bg_rect = rect;
                let fill_rect = egui::Rect::from_min_max(
                    rect.left_top(),
                    egui::pos2(rect.left() + bar_width * frac, rect.bottom()),
                );
                ui.painter().rect_filled(bg_rect, 0.0, Color32::from_rgb(40, 40, 40));
                ui.painter().rect_filled(fill_rect, 0.0, color);
            }
        }

        // Stats
        if let Some(speed) = tooltip.mining_speed {
            ui.label(
                RichText::new(format!("Mining Speed: {:.1}", speed))
                    .size(11.0)
                    .color(Color32::LIGHT_BLUE),
            );
        }

        if let Some(dmg) = tooltip.damage {
            ui.label(
                RichText::new(format!("Attack Damage: {:.1}", dmg))
                    .size(11.0)
                    .color(Color32::from_rgb(255, 100, 100)),
            );
        }

        if let Some(food) = tooltip.food_value {
            ui.label(
                RichText::new(format!("Food: {:.0}", food))
                    .size(11.0)
                    .color(Color32::from_rgb(200, 150, 50)),
            );
        }

        // Description
        if !tooltip.description_lines.is_empty() {
            ui.add_space(4.0);
            for line in &tooltip.description_lines {
                ui.label(
                    RichText::new(line)
                        .size(10.0)
                        .color(Color32::from_rgb(180, 180, 180)),
                );
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_tooltip() {
        let tip = ItemTooltip::new("Stone");
        assert_eq!(tip.name, "Stone");
        assert!(!tip.has_details());
    }

    #[test]
    fn test_tool_tooltip() {
        let tip = ItemTooltip::new("Iron Pickaxe")
            .with_category("Tool")
            .with_durability(200, 250)
            .with_mining_speed(4.0)
            .with_damage(2.5);

        assert!(tip.has_details());
        assert_eq!(tip.durability_fraction(), Some(0.8));
        assert_eq!(tip.durability_color(), Some(Color32::GREEN));
    }

    #[test]
    fn test_food_tooltip() {
        let tip = ItemTooltip::new("Apple")
            .with_category("Food")
            .with_food_value(4.0);

        assert!(tip.has_details());
        assert!(tip.food_value.is_some());
    }

    #[test]
    fn test_durability_colors() {
        // High durability
        let tip = ItemTooltip::new("Sword").with_durability(90, 100);
        assert_eq!(tip.durability_color(), Some(Color32::GREEN));

        // Medium
        let tip = ItemTooltip::new("Sword").with_durability(50, 100);
        assert_eq!(tip.durability_color(), Some(Color32::YELLOW));

        // Low
        let tip = ItemTooltip::new("Sword").with_durability(10, 100);
        assert_eq!(tip.durability_color(), Some(Color32::RED));

        // Critical
        let tip = ItemTooltip::new("Sword").with_durability(20, 100);
        assert_eq!(tip.durability_color(), Some(Color32::from_rgb(255, 140, 0)));
    }

    #[test]
    fn test_no_durability_color() {
        let tip = ItemTooltip::new("Dirt");
        assert!(tip.durability_color().is_none());
        assert!(tip.durability_fraction().is_none());
    }

    #[test]
    fn test_builder_pattern() {
        let tip = ItemTooltip::new("Diamond Sword")
            .with_category("Weapon")
            .with_durability(1561, 1561)
            .with_damage(7.0)
            .with_description("A powerful blade")
            .with_stack_size(1);

        assert_eq!(tip.name, "Diamond Sword");
        assert_eq!(tip.category, "Weapon");
        assert_eq!(tip.stack_size, 1);
        assert_eq!(tip.description_lines.len(), 1);
    }

    #[test]
    fn test_zero_max_durability() {
        let tip = ItemTooltip::new("Broken").with_durability(0, 0);
        assert_eq!(tip.durability_fraction(), None);
    }
}
