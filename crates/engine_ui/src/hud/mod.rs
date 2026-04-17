//! In-game HUD elements.

mod crosshair;
mod debug_console;
mod debug_overlay;
mod health_bar;
mod hotbar;
mod hunger_bar;
mod status_effects;
mod tooltip;

pub use crosshair::{CrosshairConfig, CrosshairStyle, draw_crosshair};
pub use debug_console::{
    process_builtin_command, ConsoleAction, ConsoleLine, DebugConsole, LineKind,
};
pub use debug_overlay::{DebugLevel, DebugOverlay, DebugStats};
pub use health_bar::{draw_health_bar, HealthBarState};
pub use hotbar::{draw_hotbar, HotbarSlot, ItemTextures};
pub use hunger_bar::{draw_hunger_bar, HungerBarState};
pub use status_effects::{
    ActiveStatusEffect, StatusEffectKind, ICON_SIZE, draw_status_effects,
};
pub use tooltip::{ItemTooltip, draw_tooltip};
