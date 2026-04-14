//! Debug overlay for performance metrics.
//!
//! Displays FPS, frame time, and other performance statistics.

use egui::{Align2, Color32, RichText, Vec2};

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
    pub fn with_fps(mut self, fps: f32, frame_time_ms: f32) -> Self {
        self.fps = fps;
        self.frame_time_ms = frame_time_ms;
        self
    }

    /// Set average frame time.
    pub fn with_avg_frame_time(mut self, avg_ms: f32) -> Self {
        self.avg_frame_time_ms = avg_ms;
        self
    }

    /// Set 1% low FPS.
    pub fn with_fps_1_low(mut self, fps_1_low: f32) -> Self {
        self.fps_1_low = fps_1_low;
        self
    }

    /// Set render stats.
    pub fn with_render_stats(mut self, draw_calls: u32, triangles: u32) -> Self {
        self.draw_calls = draw_calls;
        self.triangles = triangles;
        self
    }

    /// Set chunk stats.
    pub fn with_chunk_stats(mut self, rendered: u32, loaded: u32) -> Self {
        self.chunks_rendered = rendered;
        self.chunks_loaded = loaded;
        self
    }

    /// Set entity count.
    pub fn with_entity_count(mut self, count: u32) -> Self {
        self.entity_count = count;
        self
    }

    /// Set player position.
    pub fn with_player_pos(mut self, pos: [f32; 3]) -> Self {
        self.player_pos = Some(pos);
        self
    }

    /// Set chunk position.
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

        if let Some(pos) = stats.player_pos {
            ui.separator();
            ui.label(
                RichText::new(format!("Pos: {:.1}, {:.1}, {:.1}", pos[0], pos[1], pos[2]))
                    .color(Color32::LIGHT_BLUE)
                    .monospace(),
            );
        }

        if let Some(chunk) = stats.chunk_pos {
            ui.label(
                RichText::new(format!("Chunk: {}, {}, {}", chunk[0], chunk[1], chunk[2]))
                    .color(Color32::LIGHT_BLUE)
                    .monospace(),
            );
        }
    }
}

/// Format large numbers with K/M suffixes.
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
}
