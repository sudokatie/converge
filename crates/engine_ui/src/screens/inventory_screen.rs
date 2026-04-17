//! Inventory screen for managing player items and equipment.
//!
//! Displays the full player inventory grid (36 slots), armor/equipment
//! slots, and cursor-held item for drag-and-drop interactions.

use egui::{Color32, RichText, Vec2};

/// Equipment slot types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipmentSlot {
    Head,
    Chest,
    Legs,
    Feet,
    MainHand,
    OffHand,
}

impl EquipmentSlot {
    /// Get the display label for this slot.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            EquipmentSlot::Head => "Helmet",
            EquipmentSlot::Chest => "Chestplate",
            EquipmentSlot::Legs => "Leggings",
            EquipmentSlot::Feet => "Boots",
            EquipmentSlot::MainHand => "Main Hand",
            EquipmentSlot::OffHand => "Off Hand",
        }
    }

    /// All equipment slots in display order.
    #[must_use]
    pub fn all() -> &'static [EquipmentSlot] {
        &[
            EquipmentSlot::Head,
            EquipmentSlot::Chest,
            EquipmentSlot::Legs,
            EquipmentSlot::Feet,
            EquipmentSlot::MainHand,
            EquipmentSlot::OffHand,
        ]
    }
}

/// Item slot data for rendering.
#[derive(Debug, Clone)]
pub struct InventorySlot {
    /// Slot index in the inventory.
    pub index: usize,
    /// Item name (empty string if no item).
    pub item_name: String,
    /// Stack count (0 if empty).
    pub count: u32,
    /// Whether this slot is selected/highlighted.
    pub selected: bool,
}

/// Action from the inventory screen.
#[derive(Debug, Clone, PartialEq)]
pub enum InventoryAction {
    /// Clicked a slot in the main inventory.
    ClickSlot(usize),
    /// Clicked an equipment slot.
    ClickEquipment(EquipmentSlot),
    /// Closed the inventory screen.
    Close,
}

/// Inventory screen state and renderer.
#[derive(Debug, Clone)]
pub struct InventoryScreen {
    /// Currently held item by the cursor (drag-and-drop).
    cursor_item: Option<CursorItem>,
    /// Whether the screen is open.
    open: bool,
}

/// Item held by the cursor during drag.
#[derive(Debug, Clone)]
struct CursorItem {
    /// Item name.
    name: String,
    /// Stack count.
    count: u32,
    /// Source slot the item was picked up from.
    source: CursorSource,
}

/// Where a cursor item came from.
#[derive(Debug, Clone)]
enum CursorSource {
    /// From inventory slot index.
    Slot(usize),
    /// From equipment slot.
    Equipment(EquipmentSlot),
}

/// Inventory screen configuration.
#[derive(Debug, Clone, Copy)]
pub struct InventoryScreenConfig {
    /// Size of each inventory slot in pixels.
    pub slot_size: f32,
    /// Padding between slots.
    pub slot_padding: f32,
    /// Number of columns in the main inventory grid.
    pub columns: usize,
}

impl Default for InventoryScreenConfig {
    fn default() -> Self {
        Self {
            slot_size: 40.0,
            slot_padding: 4.0,
            columns: 9,
        }
    }
}

impl InventoryScreen {
    /// Create a new inventory screen.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cursor_item: None,
            open: false,
        }
    }

    /// Check if the screen is open.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Toggle the screen open/closed.
    pub fn toggle(&mut self) {
        self.open = !self.open;
        if !self.open {
            self.cursor_item = None;
        }
    }

    /// Open the screen.
    pub fn open(&mut self) {
        self.open = true;
    }

    /// Close the screen.
    pub fn close(&mut self) {
        self.open = false;
        self.cursor_item = None;
    }

    /// Get the currently held cursor item name.
    #[must_use]
    pub fn cursor_item_name(&self) -> Option<&str> {
        self.cursor_item.as_ref().map(|i| i.name.as_str())
    }

    /// Draw the inventory screen.
    ///
    /// Returns any action taken by the user.
    pub fn draw(
        &mut self,
        ctx: &egui::Context,
        slots: &[InventorySlot],
        equipment: &[(EquipmentSlot, Option<String>)],
        config: &InventoryScreenConfig,
    ) -> Option<InventoryAction> {
        if !self.open {
            return None;
        }

        let mut action = None;

        // Dark overlay
        let panel = egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(Color32::from_black_alpha(180)));

        panel.show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(30.0);

                // Title
                ui.label(RichText::new("Inventory").size(24.0).color(Color32::WHITE));
                ui.add_space(16.0);

                ui.horizontal(|ui| {
                    // Equipment panel (left side)
                    if action.is_none() {
                        action = self.draw_equipment(ui, equipment, config);
                    }

                    ui.add_space(20.0);

                    // Main inventory grid (right side)
                    if action.is_none() {
                        action = self.draw_grid(ui, slots, config);
                    }
                });

                ui.add_space(16.0);

                // Close hint
                ui.label(
                    RichText::new("Press E to close")
                        .size(12.0)
                        .color(Color32::GRAY),
                );
            });
        });

        action
    }

    /// Draw the equipment slots panel.
    fn draw_equipment(
        &mut self,
        ui: &mut egui::Ui,
        equipment: &[(EquipmentSlot, Option<String>)],
        config: &InventoryScreenConfig,
    ) -> Option<InventoryAction> {
        let mut action = None;

        ui.vertical(|ui| {
            ui.label(
                RichText::new("Equipment")
                    .size(16.0)
                    .color(Color32::WHITE),
            );
            ui.add_space(8.0);

            for (slot_type, item_name) in equipment {
                let slot_size = Vec2::splat(config.slot_size);
                let response = ui.add_sized(
                    slot_size,
                    egui::Button::new(
                        RichText::new(item_name.as_deref().unwrap_or(slot_type.label()))
                            .size(11.0),
                    ),
                );

                if response.clicked() {
                    action = Some(InventoryAction::ClickEquipment(*slot_type));
                }

                ui.add_space(4.0);
            }
        });

        action
    }

    /// Draw the main inventory grid.
    fn draw_grid(
        &mut self,
        ui: &mut egui::Ui,
        slots: &[InventorySlot],
        config: &InventoryScreenConfig,
    ) -> Option<InventoryAction> {
        let mut action = None;

        ui.vertical(|ui| {
            // Hotbar row (slots 0-8)
            ui.label(
                RichText::new("Hotbar")
                    .size(14.0)
                    .color(Color32::LIGHT_GRAY),
            );
            ui.add_space(4.0);

            if action.is_none() {
                action = self.draw_slot_row(ui, slots, 0, 9, config);
            }

            ui.add_space(12.0);

            // Main inventory (slots 9-35)
            ui.label(
                RichText::new("Inventory")
                    .size(14.0)
                    .color(Color32::LIGHT_GRAY),
            );
            ui.add_space(4.0);

            for row_start in (9..36).step_by(config.columns) {
                let row_end = (row_start + config.columns).min(36);
                if action.is_none() {
                    action = self.draw_slot_row(ui, slots, row_start, row_end, config);
                }
            }
        });

        action
    }

    /// Draw a row of inventory slots.
    fn draw_slot_row(
        &mut self,
        ui: &mut egui::Ui,
        slots: &[InventorySlot],
        start: usize,
        end: usize,
        config: &InventoryScreenConfig,
    ) -> Option<InventoryAction> {
        let mut action = None;

        ui.horizontal(|ui| {
            for i in start..end {
                let slot_data = slots.get(i);
                let label = slot_data
                    .and_then(|s| if s.count > 0 { Some(s.item_name.as_str()) } else { None })
                    .unwrap_or("");

                let count = slot_data.map_or(0, |s| s.count);
                let is_selected = slot_data.map_or(false, |s| s.selected);

                let slot_size = Vec2::splat(config.slot_size);

                let tint = if is_selected {
                    Color32::YELLOW
                } else if count > 0 {
                    Color32::WHITE
                } else {
                    Color32::DARK_GRAY
                };

                let display_text = if count > 1 {
                    format!("{}\n{}", label.chars().take(4).collect::<String>(), count)
                } else if !label.is_empty() {
                    label.chars().take(4).collect()
                } else {
                    String::new()
                };

                let response = ui.add_sized(
                    slot_size,
                    egui::Button::new(RichText::new(display_text).size(10.0).color(tint)),
                );

                if response.clicked() {
                    action = Some(InventoryAction::ClickSlot(i));
                }
            }
        });

        action
    }
}

impl Default for InventoryScreen {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_screen_is_closed() {
        let screen = InventoryScreen::new();
        assert!(!screen.is_open());
    }

    #[test]
    fn test_toggle_opens_and_closes() {
        let mut screen = InventoryScreen::new();
        screen.toggle();
        assert!(screen.is_open());
        screen.toggle();
        assert!(!screen.is_open());
    }

    #[test]
    fn test_open_close() {
        let mut screen = InventoryScreen::new();
        screen.open();
        assert!(screen.is_open());
        screen.close();
        assert!(!screen.is_open());
    }

    #[test]
    fn test_close_clears_cursor() {
        let mut screen = InventoryScreen::new();
        screen.open();
        screen.cursor_item = Some(CursorItem {
            name: "Test".to_string(),
            count: 1,
            source: CursorSource::Slot(0),
        });
        screen.close();
        assert!(screen.cursor_item.is_none());
    }

    #[test]
    fn test_cursor_item_name() {
        let mut screen = InventoryScreen::new();
        assert!(screen.cursor_item_name().is_none());

        screen.cursor_item = Some(CursorItem {
            name: "Pickaxe".to_string(),
            count: 1,
            source: CursorSource::Slot(0),
        });
        assert_eq!(screen.cursor_item_name(), Some("Pickaxe"));
    }

    #[test]
    fn test_equipment_slot_labels() {
        assert_eq!(EquipmentSlot::Head.label(), "Helmet");
        assert_eq!(EquipmentSlot::Chest.label(), "Chestplate");
        assert_eq!(EquipmentSlot::Legs.label(), "Leggings");
        assert_eq!(EquipmentSlot::Feet.label(), "Boots");
        assert_eq!(EquipmentSlot::MainHand.label(), "Main Hand");
        assert_eq!(EquipmentSlot::OffHand.label(), "Off Hand");
    }

    #[test]
    fn test_equipment_all_slots() {
        let all = EquipmentSlot::all();
        assert_eq!(all.len(), 6);
    }

    #[test]
    fn test_inventory_slot_creation() {
        let slot = InventorySlot {
            index: 5,
            item_name: "Stone".to_string(),
            count: 64,
            selected: false,
        };
        assert_eq!(slot.index, 5);
        assert_eq!(slot.count, 64);
    }

    #[test]
    fn test_inventory_action_equality() {
        let a = InventoryAction::ClickSlot(5);
        let b = InventoryAction::ClickSlot(5);
        assert_eq!(a, b);
    }

    #[test]
    fn test_screen_config_defaults() {
        let config = InventoryScreenConfig::default();
        assert_eq!(config.columns, 9);
        assert_eq!(config.slot_size, 40.0);
    }
}
