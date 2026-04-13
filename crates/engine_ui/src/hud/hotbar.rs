//! Hotbar HUD element.

use egui::{Color32, Pos2, Rect, Rounding, Sense, Stroke, TextureId, Vec2};

/// Number of hotbar slots.
pub const HOTBAR_SLOTS: usize = 9;

/// Hotbar slot dimensions.
const SLOT_SIZE: f32 = 48.0;
const SLOT_PADDING: f32 = 4.0;
const SLOT_MARGIN: f32 = 2.0;

/// Background color for slots.
const SLOT_BG: Color32 = Color32::from_rgba_premultiplied(40, 40, 40, 200);

/// Selected slot highlight color.
const SLOT_SELECTED: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 100);

/// Border color.
const SLOT_BORDER: Color32 = Color32::from_rgba_premultiplied(80, 80, 80, 255);

/// Data for a single hotbar slot.
#[derive(Clone, Debug, Default)]
pub struct HotbarSlot {
    /// Item ID (None if empty).
    pub item_id: Option<u16>,
    /// Stack count.
    pub count: u32,
}

impl HotbarSlot {
    /// Create an empty slot.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Create a slot with an item.
    #[must_use]
    pub fn with_item(item_id: u16, count: u32) -> Self {
        Self {
            item_id: Some(item_id),
            count,
        }
    }

    /// Check if slot is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.item_id.is_none()
    }
}

/// Texture lookup for item icons.
pub trait ItemTextures {
    /// Get the texture ID for an item, or None if no texture.
    fn get_texture(&self, item_id: u16) -> Option<TextureId>;
}

/// Default implementation that returns no textures.
pub struct NoTextures;

impl ItemTextures for NoTextures {
    fn get_texture(&self, _item_id: u16) -> Option<TextureId> {
        None
    }
}

/// Draw the hotbar HUD.
///
/// # Arguments
/// * `ctx` - egui context
/// * `slots` - Array of 9 hotbar slots
/// * `selected` - Currently selected slot index (0-8)
/// * `textures` - Item texture lookup
pub fn draw_hotbar(
    ctx: &egui::Context,
    slots: &[HotbarSlot; HOTBAR_SLOTS],
    selected: usize,
    textures: &impl ItemTextures,
) {
    let screen_rect = ctx.screen_rect();

    // Calculate hotbar position (centered at bottom)
    let total_width =
        HOTBAR_SLOTS as f32 * (SLOT_SIZE + SLOT_MARGIN * 2.0) + SLOT_PADDING * 2.0 - SLOT_MARGIN * 2.0;
    let hotbar_x = (screen_rect.width() - total_width) / 2.0;
    let hotbar_y = screen_rect.height() - SLOT_SIZE - SLOT_PADDING * 2.0 - 20.0;

    egui::Area::new(egui::Id::new("hotbar"))
        .fixed_pos(Pos2::new(hotbar_x, hotbar_y))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::splat(SLOT_MARGIN);

                for (i, slot) in slots.iter().enumerate() {
                    draw_slot(ui, slot, i == selected, textures);
                }
            });
        });
}

/// Draw a single hotbar slot.
fn draw_slot(ui: &mut egui::Ui, slot: &HotbarSlot, is_selected: bool, textures: &impl ItemTextures) {
    let (rect, _response) = ui.allocate_exact_size(Vec2::splat(SLOT_SIZE), Sense::hover());

    let painter = ui.painter();

    // Background
    painter.rect_filled(rect, Rounding::same(4.0), SLOT_BG);

    // Selection highlight
    if is_selected {
        painter.rect_filled(rect, Rounding::same(4.0), SLOT_SELECTED);
    }

    // Border
    painter.rect_stroke(rect, Rounding::same(4.0), Stroke::new(1.0, SLOT_BORDER));

    // Item icon and count
    if let Some(item_id) = slot.item_id {
        // Draw item texture if available
        if let Some(texture_id) = textures.get_texture(item_id) {
            let icon_rect = rect.shrink(6.0);
            painter.image(
                texture_id,
                icon_rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            // Placeholder colored square based on item ID
            let color = item_id_to_color(item_id);
            let icon_rect = rect.shrink(8.0);
            painter.rect_filled(icon_rect, Rounding::same(2.0), color);
        }

        // Stack count (only show if > 1)
        if slot.count > 1 {
            let count_text = format!("{}", slot.count);
            let text_pos = rect.right_bottom() - Vec2::new(6.0, 4.0);

            // Shadow
            painter.text(
                text_pos + Vec2::new(1.0, 1.0),
                egui::Align2::RIGHT_BOTTOM,
                &count_text,
                egui::FontId::proportional(12.0),
                Color32::BLACK,
            );

            // Text
            painter.text(
                text_pos,
                egui::Align2::RIGHT_BOTTOM,
                &count_text,
                egui::FontId::proportional(12.0),
                Color32::WHITE,
            );
        }
    }
}

/// Generate a placeholder color from item ID.
fn item_id_to_color(item_id: u16) -> Color32 {
    // Simple hash to generate varied colors (use u32 to avoid overflow)
    let id = item_id as u32;
    let r = ((id.wrapping_mul(123)) % 200 + 55) as u8;
    let g = ((id.wrapping_mul(456)) % 200 + 55) as u8;
    let b = ((id.wrapping_mul(789)) % 200 + 55) as u8;
    Color32::from_rgb(r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hotbar_slot_empty() {
        let slot = HotbarSlot::empty();
        assert!(slot.is_empty());
        assert!(slot.item_id.is_none());
    }

    #[test]
    fn test_hotbar_slot_with_item() {
        let slot = HotbarSlot::with_item(5, 32);
        assert!(!slot.is_empty());
        assert_eq!(slot.item_id, Some(5));
        assert_eq!(slot.count, 32);
    }

    #[test]
    fn test_item_id_to_color() {
        let c1 = item_id_to_color(1);
        let c2 = item_id_to_color(2);
        let c3 = item_id_to_color(100);

        // Colors should be different
        assert_ne!(c1, c2);
        assert_ne!(c2, c3);

        // Colors should be valid (not too dark)
        assert!(c1.r() >= 55);
        assert!(c1.g() >= 55);
        assert!(c1.b() >= 55);
    }

    #[test]
    fn test_no_textures() {
        let textures = NoTextures;
        assert!(textures.get_texture(1).is_none());
        assert!(textures.get_texture(999).is_none());
    }
}
