//! Debug overlay for performance metrics.
//!
//! Displays FPS, frame time, subsystem budgets, and other performance statistics.

use egui::{Align2, Color32, RichText, Vec2};

// ============================================================================
// Subsystem Budget Dashboard
// ============================================================================

/// Severity level for budget display (mirrors `engine_core` but avoids dependency coupling).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DashboardSeverity {
    /// Within budget.
    #[default]
    Ok,
    /// Approaching budget.
    Warning,
    /// Over budget.
    Critical,
}

impl DashboardSeverity {
    /// Get the display color for this severity.
    #[must_use]
    pub fn color(self) -> Color32 {
        match self {
            Self::Ok => Color32::from_rgb(100, 200, 100),
            Self::Warning => Color32::from_rgb(230, 200, 50),
            Self::Critical => Color32::from_rgb(230, 80, 80),
        }
    }

    /// Get the text label for this severity.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Warning => "WARN",
            Self::Critical => "OVER",
        }
    }
}

/// A single row in the budget dashboard.
#[derive(Clone, Debug, Default)]
pub struct BudgetDashboardRow {
    /// Subsystem name.
    pub name: String,
    /// Category name.
    pub category: String,
    /// Current frame time in milliseconds.
    pub current_ms: f32,
    /// Budget in milliseconds.
    pub budget_ms: f32,
    /// Utilization percentage.
    pub utilization_pct: f32,
    /// Average time over sample window.
    pub avg_ms: f32,
    /// 95th percentile time.
    pub p95_ms: f32,
    /// Over-budget streak count.
    pub over_budget_streak: u32,
    /// Severity level.
    pub severity: DashboardSeverity,
}

impl BudgetDashboardRow {
    /// Create a new dashboard row.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Set the category.
    #[must_use]
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = category.into();
        self
    }

    /// Set timing values.
    #[must_use]
    pub fn with_timing(mut self, current_ms: f32, budget_ms: f32) -> Self {
        self.current_ms = current_ms;
        self.budget_ms = budget_ms;
        self.utilization_pct = if budget_ms > 0.0 {
            (current_ms / budget_ms) * 100.0
        } else {
            0.0
        };
        self
    }

    /// Set statistics.
    #[must_use]
    pub fn with_stats(mut self, avg_ms: f32, p95_ms: f32) -> Self {
        self.avg_ms = avg_ms;
        self.p95_ms = p95_ms;
        self
    }

    /// Set over-budget streak.
    #[must_use]
    pub fn with_streak(mut self, streak: u32) -> Self {
        self.over_budget_streak = streak;
        self
    }

    /// Set severity.
    #[must_use]
    pub fn with_severity(mut self, severity: DashboardSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// Format the current timing as a string.
    #[must_use]
    pub fn format_current(&self) -> String {
        format!("{:.2} ms", self.current_ms)
    }

    /// Format the budget as a string.
    #[must_use]
    pub fn format_budget(&self) -> String {
        format!("{:.1} ms", self.budget_ms)
    }

    /// Format utilization as a string.
    #[must_use]
    pub fn format_utilization(&self) -> String {
        format!("{:.0}%", self.utilization_pct)
    }

    /// Format average/p95 as a string.
    #[must_use]
    pub fn format_stats(&self) -> String {
        format!("{:.2}/{:.2}", self.avg_ms, self.p95_ms)
    }

    /// Get the row color based on severity.
    #[must_use]
    pub fn row_color(&self) -> Color32 {
        self.severity.color()
    }

    /// Get a dimmed version of the row color for alternating rows.
    #[must_use]
    pub fn row_color_dim(&self) -> Color32 {
        let c = self.severity.color();
        Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 180)
    }
}

/// Summary row for the budget dashboard.
#[derive(Clone, Debug, Default)]
pub struct BudgetDashboardSummary {
    /// Total time across all subsystems.
    pub total_ms: f32,
    /// Number of subsystems over budget.
    pub over_budget_count: u32,
    /// Highest utilization percentage.
    pub max_utilization_pct: f32,
    /// Name of subsystem with highest utilization.
    pub max_utilization_name: Option<String>,
    /// Overall severity.
    pub severity: DashboardSeverity,
}

impl BudgetDashboardSummary {
    /// Format the total time.
    #[must_use]
    pub fn format_total(&self) -> String {
        format!("{:.2} ms", self.total_ms)
    }

    /// Format the over-budget count.
    #[must_use]
    pub fn format_over_budget(&self) -> String {
        if self.over_budget_count == 0 {
            "None".to_string()
        } else {
            format!("{} subsystem(s)", self.over_budget_count)
        }
    }
}

/// Complete budget dashboard data.
#[derive(Clone, Debug, Default)]
pub struct BudgetDashboard {
    /// Individual subsystem rows.
    pub rows: Vec<BudgetDashboardRow>,
    /// Summary information.
    pub summary: BudgetDashboardSummary,
    /// Whether the dashboard is visible.
    pub visible: bool,
}

impl BudgetDashboard {
    /// Create a new empty dashboard.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set visibility.
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// Toggle visibility.
    pub fn toggle_visible(&mut self) {
        self.visible = !self.visible;
    }

    /// Check if visible.
    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Clear all rows.
    pub fn clear(&mut self) {
        self.rows.clear();
        self.summary = BudgetDashboardSummary::default();
    }

    /// Add a row.
    pub fn add_row(&mut self, row: BudgetDashboardRow) {
        self.rows.push(row);
    }

    /// Set the summary.
    pub fn set_summary(&mut self, summary: BudgetDashboardSummary) {
        self.summary = summary;
    }

    /// Draw the budget dashboard.
    pub fn draw(&self, ctx: &egui::Context) {
        if !self.visible || self.rows.is_empty() {
            return;
        }

        egui::Area::new(egui::Id::new("budget_dashboard"))
            .anchor(Align2::RIGHT_TOP, Vec2::new(-10.0, 10.0))
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(Color32::from_rgba_unmultiplied(0, 0, 0, 200))
                    .inner_margin(8.0)
                    .rounding(4.0)
                    .show(ui, |ui| {
                        self.draw_content(ui);
                    });
            });
    }

    fn draw_content(&self, ui: &mut egui::Ui) {
        // Title
        ui.label(
            RichText::new("Subsystem Budgets")
                .color(Color32::WHITE)
                .strong()
                .monospace(),
        );

        ui.separator();

        // Header row
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{:<12}", "Name"))
                    .color(Color32::GRAY)
                    .monospace(),
            );
            ui.label(
                RichText::new(format!("{:>8}", "Current"))
                    .color(Color32::GRAY)
                    .monospace(),
            );
            ui.label(
                RichText::new(format!("{:>8}", "Budget"))
                    .color(Color32::GRAY)
                    .monospace(),
            );
            ui.label(
                RichText::new(format!("{:>6}", "Util"))
                    .color(Color32::GRAY)
                    .monospace(),
            );
            ui.label(
                RichText::new(format!("{:>12}", "Avg/P95"))
                    .color(Color32::GRAY)
                    .monospace(),
            );
        });

        // Data rows
        let mut current_category = String::new();
        for row in &self.rows {
            // Category header if changed
            if row.category != current_category && !row.category.is_empty() {
                current_category.clone_from(&row.category);
                ui.label(
                    RichText::new(format!("-- {} --", row.category))
                        .color(Color32::LIGHT_BLUE)
                        .small()
                        .monospace(),
                );
            }

            let color = row.row_color();
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("{:<12}", truncate_str(&row.name, 12)))
                        .color(color)
                        .monospace(),
                );
                ui.label(
                    RichText::new(format!("{:>8}", row.format_current()))
                        .color(color)
                        .monospace(),
                );
                ui.label(
                    RichText::new(format!("{:>8}", row.format_budget()))
                        .color(Color32::GRAY)
                        .monospace(),
                );
                ui.label(
                    RichText::new(format!("{:>6}", row.format_utilization()))
                        .color(color)
                        .monospace(),
                );
                ui.label(
                    RichText::new(format!("{:>12}", row.format_stats()))
                        .color(Color32::GRAY)
                        .monospace(),
                );
            });
        }

        // Summary
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("Total: {}", self.summary.format_total()))
                    .color(self.summary.severity.color())
                    .monospace(),
            );
            if self.summary.over_budget_count > 0 {
                ui.label(
                    RichText::new(format!("Over: {}", self.summary.over_budget_count))
                        .color(DashboardSeverity::Critical.color())
                        .monospace(),
                );
            }
        });
    }
}

/// Truncate a string to max length with ellipsis.
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len > 2 {
        format!("{}..", &s[..max_len - 2])
    } else {
        s[..max_len].to_string()
    }
}

/// Performance data for the debug overlay.
#[derive(Clone, Debug, Default)]
pub struct DebugStats {
    /// Current FPS.
    pub fps: f32,
    /// Frame time in milliseconds.
    pub frame_time_ms: f32,
    /// Average frame time.
    pub avg_frame_time_ms: f32,
    /// 1% low FPS.
    pub fps_1_low: f32,
    /// Number of draw calls.
    pub draw_calls: u32,
    /// Number of triangles.
    pub triangles: u32,
    /// Number of chunks rendered.
    pub chunks_rendered: u32,
    /// Number of chunks loaded.
    pub chunks_loaded: u32,
    /// Number of entities.
    pub entity_count: u32,
    /// Player position.
    pub player_pos: Option<[f32; 3]>,
    /// Current chunk position.
    pub chunk_pos: Option<[i32; 3]>,
}

impl DebugStats {
    /// Create new debug stats.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set FPS stats.
    #[must_use]
    pub fn with_fps(mut self, fps: f32, frame_time_ms: f32) -> Self {
        self.fps = fps;
        self.frame_time_ms = frame_time_ms;
        self
    }

    /// Set average frame time.
    #[must_use]
    pub fn with_avg_frame_time(mut self, avg_ms: f32) -> Self {
        self.avg_frame_time_ms = avg_ms;
        self
    }

    /// Set 1% low FPS.
    #[must_use]
    pub fn with_fps_1_low(mut self, fps_1_low: f32) -> Self {
        self.fps_1_low = fps_1_low;
        self
    }

    /// Set render stats.
    #[must_use]
    pub fn with_render_stats(mut self, draw_calls: u32, triangles: u32) -> Self {
        self.draw_calls = draw_calls;
        self.triangles = triangles;
        self
    }

    /// Set chunk stats.
    #[must_use]
    pub fn with_chunk_stats(mut self, rendered: u32, loaded: u32) -> Self {
        self.chunks_rendered = rendered;
        self.chunks_loaded = loaded;
        self
    }

    /// Set entity count.
    #[must_use]
    pub fn with_entity_count(mut self, count: u32) -> Self {
        self.entity_count = count;
        self
    }

    /// Set player position.
    #[must_use]
    pub fn with_player_pos(mut self, pos: [f32; 3]) -> Self {
        self.player_pos = Some(pos);
        self
    }

    /// Set chunk position.
    #[must_use]
    pub fn with_chunk_pos(mut self, pos: [i32; 3]) -> Self {
        self.chunk_pos = Some(pos);
        self
    }
}

/// Debug overlay display level.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DebugLevel {
    /// Hidden.
    #[default]
    Off,
    /// Basic FPS only.
    Minimal,
    /// FPS and frame times.
    Basic,
    /// Full stats including render and world info.
    Full,
}

impl DebugLevel {
    /// Cycle to next level.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::Minimal,
            Self::Minimal => Self::Basic,
            Self::Basic => Self::Full,
            Self::Full => Self::Off,
        }
    }
}

/// Debug overlay state.
#[derive(Clone, Debug, Default)]
pub struct DebugOverlay {
    /// Current display level.
    level: DebugLevel,
    /// Whether profiler view is open.
    profiler_open: bool,
}

impl DebugOverlay {
    /// Create a new debug overlay.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get current debug level.
    #[must_use]
    pub fn level(&self) -> DebugLevel {
        self.level
    }

    /// Set debug level.
    pub fn set_level(&mut self, level: DebugLevel) {
        self.level = level;
    }

    /// Cycle to next debug level.
    pub fn cycle_level(&mut self) {
        self.level = self.level.next();
    }

    /// Check if overlay is visible.
    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.level != DebugLevel::Off
    }

    /// Toggle profiler view.
    pub fn toggle_profiler(&mut self) {
        self.profiler_open = !self.profiler_open;
    }

    /// Check if profiler is open.
    #[must_use]
    pub fn is_profiler_open(&self) -> bool {
        self.profiler_open
    }

    /// Draw the debug overlay.
    pub fn draw(&self, ctx: &egui::Context, stats: &DebugStats) {
        if self.level == DebugLevel::Off {
            return;
        }

        egui::Area::new(egui::Id::new("debug_overlay"))
            .anchor(Align2::LEFT_TOP, Vec2::new(10.0, 10.0))
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(Color32::from_rgba_unmultiplied(0, 0, 0, 180))
                    .inner_margin(8.0)
                    .rounding(4.0)
                    .show(ui, |ui| {
                        self.draw_content(ui, stats);
                    });
            });
    }

    fn draw_content(&self, ui: &mut egui::Ui, stats: &DebugStats) {
        // FPS color based on performance
        let fps_color = if stats.fps >= 55.0 {
            Color32::GREEN
        } else if stats.fps >= 30.0 {
            Color32::YELLOW
        } else {
            Color32::RED
        };

        // Always show FPS
        ui.label(
            RichText::new(format!("FPS: {:.0}", stats.fps))
                .color(fps_color)
                .monospace(),
        );

        if self.level == DebugLevel::Minimal {
            return;
        }

        // Basic: add frame times
        ui.label(
            RichText::new(format!("Frame: {:.2} ms", stats.frame_time_ms))
                .color(Color32::WHITE)
                .monospace(),
        );

        if stats.avg_frame_time_ms > 0.0 {
            ui.label(
                RichText::new(format!("Avg: {:.2} ms", stats.avg_frame_time_ms))
                    .color(Color32::GRAY)
                    .monospace(),
            );
        }

        if stats.fps_1_low > 0.0 {
            ui.label(
                RichText::new(format!("1% Low: {:.0} FPS", stats.fps_1_low))
                    .color(Color32::GRAY)
                    .monospace(),
            );
        }

        if self.level == DebugLevel::Basic {
            return;
        }

        // Full: add render and world stats
        ui.separator();

        ui.label(
            RichText::new(format!("Draw calls: {}", stats.draw_calls))
                .color(Color32::WHITE)
                .monospace(),
        );

        ui.label(
            RichText::new(format!("Triangles: {}", format_number(stats.triangles)))
                .color(Color32::WHITE)
                .monospace(),
        );

        ui.separator();

        ui.label(
            RichText::new(format!(
                "Chunks: {}/{}",
                stats.chunks_rendered, stats.chunks_loaded
            ))
            .color(Color32::WHITE)
            .monospace(),
        );

        ui.label(
            RichText::new(format!("Entities: {}", stats.entity_count))
                .color(Color32::WHITE)
                .monospace(),
        );

        if let Some([pos_0, pos_1, pos_2]) = stats.player_pos {
            ui.separator();
            ui.label(
                RichText::new(format!("Pos: {pos_0:.1}, {pos_1:.1}, {pos_2:.1}"))
                    .color(Color32::LIGHT_BLUE)
                    .monospace(),
            );
        }

        if let Some([chunk_0, chunk_1, chunk_2]) = stats.chunk_pos {
            ui.label(
                RichText::new(format!("Chunk: {chunk_0}, {chunk_1}, {chunk_2}"))
                    .color(Color32::LIGHT_BLUE)
                    .monospace(),
            );
        }
    }
}

/// Format large numbers with K/M suffixes.
#[expect(
    clippy::cast_precision_loss,
    reason = "display formatting tolerates precision loss"
)]
fn format_number(n: u32) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f32 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f32 / 1_000.0)
    } else {
        n.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_overlay_new() {
        let overlay = DebugOverlay::new();
        assert_eq!(overlay.level(), DebugLevel::Off);
        assert!(!overlay.is_visible());
    }

    #[test]
    fn test_debug_level_cycle() {
        assert_eq!(DebugLevel::Off.next(), DebugLevel::Minimal);
        assert_eq!(DebugLevel::Minimal.next(), DebugLevel::Basic);
        assert_eq!(DebugLevel::Basic.next(), DebugLevel::Full);
        assert_eq!(DebugLevel::Full.next(), DebugLevel::Off);
    }

    #[test]
    fn test_overlay_cycle_level() {
        let mut overlay = DebugOverlay::new();
        assert_eq!(overlay.level(), DebugLevel::Off);
        overlay.cycle_level();
        assert_eq!(overlay.level(), DebugLevel::Minimal);
        assert!(overlay.is_visible());
    }

    #[test]
    fn test_overlay_set_level() {
        let mut overlay = DebugOverlay::new();
        overlay.set_level(DebugLevel::Full);
        assert_eq!(overlay.level(), DebugLevel::Full);
    }

    #[test]
    fn test_toggle_profiler() {
        let mut overlay = DebugOverlay::new();
        assert!(!overlay.is_profiler_open());
        overlay.toggle_profiler();
        assert!(overlay.is_profiler_open());
        overlay.toggle_profiler();
        assert!(!overlay.is_profiler_open());
    }

    #[test]
    fn test_debug_stats_builder() {
        let stats = DebugStats::new()
            .with_fps(60.0, 16.67)
            .with_avg_frame_time(16.5)
            .with_fps_1_low(55.0)
            .with_render_stats(100, 50000)
            .with_chunk_stats(64, 128)
            .with_entity_count(25)
            .with_player_pos([10.0, 64.0, 20.0])
            .with_chunk_pos([0, 4, 1]);

        assert!((stats.fps - 60.0).abs() < 0.01);
        assert!((stats.frame_time_ms - 16.67).abs() < 0.01);
        assert_eq!(stats.draw_calls, 100);
        assert_eq!(stats.triangles, 50000);
        assert_eq!(stats.chunks_rendered, 64);
        assert_eq!(stats.entity_count, 25);
        assert!(stats.player_pos.is_some());
        assert!(stats.chunk_pos.is_some());
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(500), "500");
        assert_eq!(format_number(1500), "1.5K");
        assert_eq!(format_number(1_500_000), "1.5M");
    }

    // ========================================================================
    // Budget Dashboard Tests
    // ========================================================================

    #[test]
    fn test_dashboard_severity_color() {
        let ok_color = DashboardSeverity::Ok.color();
        let warn_color = DashboardSeverity::Warning.color();
        let crit_color = DashboardSeverity::Critical.color();

        // Green should have high G
        assert!(ok_color.g() > ok_color.r());
        // Yellow should have high R and G
        assert!(warn_color.r() > 200 && warn_color.g() > 150);
        // Red should have high R
        assert!(crit_color.r() > crit_color.g());
    }

    #[test]
    fn test_dashboard_severity_label() {
        assert_eq!(DashboardSeverity::Ok.label(), "OK");
        assert_eq!(DashboardSeverity::Warning.label(), "WARN");
        assert_eq!(DashboardSeverity::Critical.label(), "OVER");
    }

    #[test]
    fn test_budget_row_builder() {
        let row = BudgetDashboardRow::new("physics")
            .with_category("Simulation")
            .with_timing(5.0, 8.0)
            .with_stats(4.5, 6.2)
            .with_streak(0)
            .with_severity(DashboardSeverity::Ok);

        assert_eq!(row.name, "physics");
        assert_eq!(row.category, "Simulation");
        assert!((row.current_ms - 5.0).abs() < f32::EPSILON);
        assert!((row.budget_ms - 8.0).abs() < f32::EPSILON);
        assert!((row.utilization_pct - 62.5).abs() < 0.1);
        assert!((row.avg_ms - 4.5).abs() < f32::EPSILON);
        assert!((row.p95_ms - 6.2).abs() < f32::EPSILON);
        assert_eq!(row.over_budget_streak, 0);
        assert_eq!(row.severity, DashboardSeverity::Ok);
    }

    #[test]
    fn test_budget_row_utilization_calculation() {
        let row = BudgetDashboardRow::new("test").with_timing(10.0, 8.0);
        assert!((row.utilization_pct - 125.0).abs() < 0.1);

        let row_zero = BudgetDashboardRow::new("test").with_timing(5.0, 0.0);
        assert!((row_zero.utilization_pct).abs() < f32::EPSILON);
    }

    #[test]
    fn test_budget_row_format_current() {
        let row = BudgetDashboardRow::new("test").with_timing(5.123, 10.0);
        assert_eq!(row.format_current(), "5.12 ms");
    }

    #[test]
    fn test_budget_row_format_budget() {
        let row = BudgetDashboardRow::new("test").with_timing(5.0, 8.333);
        assert_eq!(row.format_budget(), "8.3 ms");
    }

    #[test]
    fn test_budget_row_format_utilization() {
        let row = BudgetDashboardRow::new("test").with_timing(6.0, 8.0);
        assert_eq!(row.format_utilization(), "75%");
    }

    #[test]
    fn test_budget_row_format_stats() {
        let row = BudgetDashboardRow::new("test").with_stats(4.567, 5.891);
        assert_eq!(row.format_stats(), "4.57/5.89");
    }

    #[test]
    fn test_budget_row_color() {
        let ok_row = BudgetDashboardRow::new("test").with_severity(DashboardSeverity::Ok);
        let warn_row = BudgetDashboardRow::new("test").with_severity(DashboardSeverity::Warning);
        let crit_row = BudgetDashboardRow::new("test").with_severity(DashboardSeverity::Critical);

        assert_eq!(ok_row.row_color(), DashboardSeverity::Ok.color());
        assert_eq!(warn_row.row_color(), DashboardSeverity::Warning.color());
        assert_eq!(crit_row.row_color(), DashboardSeverity::Critical.color());
    }

    #[test]
    fn test_budget_row_color_dim() {
        let row = BudgetDashboardRow::new("test").with_severity(DashboardSeverity::Ok);
        let full = row.row_color();
        let dim = row.row_color_dim();

        // Dimmed version should have reduced alpha
        assert!(dim.a() < full.a());
        assert_eq!(dim.a(), 180);
    }

    #[test]
    fn test_budget_summary_format_total() {
        let summary = BudgetDashboardSummary {
            total_ms: 12.345,
            ..Default::default()
        };
        assert_eq!(summary.format_total(), "12.35 ms");
    }

    #[test]
    fn test_budget_summary_format_over_budget() {
        let summary_none = BudgetDashboardSummary {
            over_budget_count: 0,
            ..Default::default()
        };
        assert_eq!(summary_none.format_over_budget(), "None");

        let summary_some = BudgetDashboardSummary {
            over_budget_count: 2,
            ..Default::default()
        };
        assert_eq!(summary_some.format_over_budget(), "2 subsystem(s)");
    }

    #[test]
    fn test_budget_dashboard_new() {
        let dashboard = BudgetDashboard::new();
        assert!(!dashboard.is_visible());
        assert!(dashboard.rows.is_empty());
    }

    #[test]
    fn test_budget_dashboard_visibility() {
        let mut dashboard = BudgetDashboard::new();

        dashboard.set_visible(true);
        assert!(dashboard.is_visible());

        dashboard.toggle_visible();
        assert!(!dashboard.is_visible());

        dashboard.toggle_visible();
        assert!(dashboard.is_visible());
    }

    #[test]
    fn test_budget_dashboard_add_row() {
        let mut dashboard = BudgetDashboard::new();

        dashboard.add_row(BudgetDashboardRow::new("physics"));
        dashboard.add_row(BudgetDashboardRow::new("rendering"));

        assert_eq!(dashboard.rows.len(), 2);
        assert_eq!(dashboard.rows[0].name, "physics");
        assert_eq!(dashboard.rows[1].name, "rendering");
    }

    #[test]
    fn test_budget_dashboard_clear() {
        let mut dashboard = BudgetDashboard::new();
        dashboard.add_row(BudgetDashboardRow::new("physics"));
        dashboard.summary.total_ms = 10.0;

        dashboard.clear();

        assert!(dashboard.rows.is_empty());
        assert!((dashboard.summary.total_ms).abs() < f32::EPSILON);
    }

    #[test]
    fn test_budget_dashboard_set_summary() {
        let mut dashboard = BudgetDashboard::new();
        let summary = BudgetDashboardSummary {
            total_ms: 15.0,
            over_budget_count: 1,
            max_utilization_pct: 120.0,
            max_utilization_name: Some("physics".to_string()),
            severity: DashboardSeverity::Critical,
        };

        dashboard.set_summary(summary);

        assert!((dashboard.summary.total_ms - 15.0).abs() < f32::EPSILON);
        assert_eq!(dashboard.summary.over_budget_count, 1);
        assert_eq!(
            dashboard.summary.max_utilization_name.as_deref(),
            Some("physics")
        );
    }

    #[test]
    fn test_truncate_str() {
        assert_eq!(truncate_str("short", 10), "short");
        assert_eq!(truncate_str("exactly10c", 10), "exactly10c");
        assert_eq!(truncate_str("longerstring", 10), "longerst..");
        assert_eq!(truncate_str("toolongname", 8), "toolon..");
        assert_eq!(truncate_str("ab", 2), "ab");
        assert_eq!(truncate_str("abc", 2), "ab");
    }
}
