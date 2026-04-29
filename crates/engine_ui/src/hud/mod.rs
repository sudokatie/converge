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
    ConsoleAction, ConsoleLine, DebugConsole, LineKind, process_builtin_command,
};
pub use debug_overlay::{
    BudgetDashboard, BudgetDashboardRow, BudgetDashboardSummary, DashboardSeverity, DebugLevel,
    DebugOverlay, DebugStats,
};
pub use health_bar::{HealthBarState, draw_health_bar};
pub use hotbar::{HotbarSlot, ItemTextures, draw_hotbar};
pub use hunger_bar::{HungerBarState, draw_hunger_bar};
pub use status_effects::{ActiveStatusEffect, ICON_SIZE, StatusEffectKind, draw_status_effects};
pub use tooltip::{ItemTooltip, draw_tooltip};
