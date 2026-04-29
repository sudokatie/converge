//! Performance profiling utilities.
//!
//! Provides frame timing, FPS tracking, subsystem budget tracking, and puffin integration
//! for detailed profiling.

use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};

// ============================================================================
// Subsystem Budget Tracking
// ============================================================================

/// Unique identifier for a subsystem being profiled.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubsystemId(pub String);

impl SubsystemId {
    /// Create a new subsystem identifier.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Get the subsystem name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.0
    }
}

impl<S: Into<String>> From<S> for SubsystemId {
    fn from(s: S) -> Self {
        Self::new(s)
    }
}

/// Category for grouping subsystems.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SubsystemCategory {
    /// Simulation/physics updates.
    #[default]
    Simulation,
    /// Rendering operations.
    Rendering,
    /// Audio processing.
    Audio,
    /// Network operations.
    Network,
    /// Input handling.
    Input,
    /// UI processing.
    Ui,
    /// Asset loading/streaming.
    Assets,
    /// Custom/other category.
    Custom,
}

impl SubsystemCategory {
    /// Get the display name for this category.
    #[must_use]
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Simulation => "Simulation",
            Self::Rendering => "Rendering",
            Self::Audio => "Audio",
            Self::Network => "Network",
            Self::Input => "Input",
            Self::Ui => "UI",
            Self::Assets => "Assets",
            Self::Custom => "Custom",
        }
    }
}

/// Severity level for budget utilization.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum BudgetSeverity {
    /// Within budget (< 80%).
    #[default]
    Ok,
    /// Approaching budget (80-100%).
    Warning,
    /// Over budget (> 100%).
    Critical,
}

impl BudgetSeverity {
    /// Determine severity from utilization percentage.
    #[must_use]
    pub fn from_utilization(utilization_pct: f32) -> Self {
        if utilization_pct >= 100.0 {
            Self::Critical
        } else if utilization_pct >= 80.0 {
            Self::Warning
        } else {
            Self::Ok
        }
    }
}

/// Configuration for a subsystem's budget.
#[derive(Clone, Debug)]
pub struct SubsystemBudgetConfig {
    /// The subsystem identifier.
    pub id: SubsystemId,
    /// Category for grouping.
    pub category: SubsystemCategory,
    /// Target budget in milliseconds.
    pub budget_ms: f32,
    /// Maximum samples to retain for statistics.
    pub max_samples: usize,
}

impl SubsystemBudgetConfig {
    /// Create a new budget configuration.
    #[must_use]
    pub fn new(id: impl Into<SubsystemId>, budget_ms: f32) -> Self {
        Self {
            id: id.into(),
            category: SubsystemCategory::default(),
            budget_ms,
            max_samples: 120,
        }
    }

    /// Set the category.
    #[must_use]
    pub fn with_category(mut self, category: SubsystemCategory) -> Self {
        self.category = category;
        self
    }

    /// Set the maximum sample count.
    #[must_use]
    pub fn with_max_samples(mut self, max_samples: usize) -> Self {
        self.max_samples = max_samples;
        self
    }
}

/// Per-subsystem timing sample data.
#[derive(Clone, Debug, Default)]
pub struct SubsystemSample {
    /// Current frame duration in milliseconds.
    pub current_ms: f32,
    /// Average duration over sample window.
    pub avg_ms: f32,
    /// Minimum duration in sample window.
    pub min_ms: f32,
    /// Maximum duration in sample window.
    pub max_ms: f32,
    /// 95th percentile duration.
    pub p95_ms: f32,
    /// Budget utilization as percentage (current / budget * 100).
    pub utilization_pct: f32,
    /// Number of frames over budget in sample window.
    pub over_budget_count: u32,
    /// Current consecutive frames over budget.
    pub over_budget_streak: u32,
    /// Current severity level.
    pub severity: BudgetSeverity,
}

/// Snapshot of a single subsystem's state.
#[derive(Clone, Debug)]
pub struct SubsystemSnapshot {
    /// Subsystem identifier.
    pub id: SubsystemId,
    /// Category.
    pub category: SubsystemCategory,
    /// Configured budget in milliseconds.
    pub budget_ms: f32,
    /// Current sample data.
    pub sample: SubsystemSample,
}

/// Frame-level summary of all subsystem budgets.
#[derive(Clone, Debug, Default)]
pub struct BudgetFrameSummary {
    /// Total time across all subsystems in milliseconds.
    pub total_ms: f32,
    /// Number of subsystems over budget.
    pub subsystems_over_budget: u32,
    /// Highest utilization percentage among all subsystems.
    pub max_utilization_pct: f32,
    /// Subsystem with highest utilization.
    pub max_utilization_subsystem: Option<SubsystemId>,
    /// Overall severity (worst among all subsystems).
    pub overall_severity: BudgetSeverity,
}

/// Complete snapshot of all subsystem budgets.
#[derive(Clone, Debug, Default)]
pub struct BudgetSnapshot {
    /// Per-subsystem snapshots, ordered by category then name.
    pub subsystems: Vec<SubsystemSnapshot>,
    /// Frame-level summary.
    pub summary: BudgetFrameSummary,
}

/// Internal state for a single subsystem.
#[derive(Debug)]
struct SubsystemState {
    config: SubsystemBudgetConfig,
    samples: VecDeque<f32>,
    current_ms: f32,
    over_budget_streak: u32,
    over_budget_count_in_window: u32,
}

impl SubsystemState {
    fn new(config: SubsystemBudgetConfig) -> Self {
        let max_samples = config.max_samples;
        Self {
            config,
            samples: VecDeque::with_capacity(max_samples),
            current_ms: 0.0,
            over_budget_streak: 0,
            over_budget_count_in_window: 0,
        }
    }

    fn record(&mut self, duration_ms: f32) {
        self.current_ms = duration_ms;

        let was_over_budget = duration_ms > self.config.budget_ms;

        if self.samples.len() >= self.config.max_samples
            && let Some(old) = self.samples.pop_front()
            && old > self.config.budget_ms
        {
            self.over_budget_count_in_window = self.over_budget_count_in_window.saturating_sub(1);
        }

        self.samples.push_back(duration_ms);

        if was_over_budget {
            self.over_budget_streak += 1;
            self.over_budget_count_in_window += 1;
        } else {
            self.over_budget_streak = 0;
        }
    }

    fn compute_sample(&self) -> SubsystemSample {
        if self.samples.is_empty() {
            return SubsystemSample::default();
        }

        let sum: f32 = self.samples.iter().sum();
        #[expect(
            clippy::cast_precision_loss,
            reason = "sample count is small; precision loss is negligible"
        )]
        let count = self.samples.len() as f32;
        let avg_ms = sum / count;

        let min_ms = self.samples.iter().copied().fold(f32::MAX, f32::min);
        let max_ms = self.samples.iter().copied().fold(0.0, f32::max);

        let p95_ms = if self.samples.len() >= 2 {
            let mut sorted: Vec<f32> = self.samples.iter().copied().collect();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let idx = (sorted.len() * 95) / 100;
            sorted[idx.min(sorted.len() - 1)]
        } else {
            self.current_ms
        };

        let utilization_pct = if self.config.budget_ms > 0.0 {
            (self.current_ms / self.config.budget_ms) * 100.0
        } else {
            0.0
        };

        let severity = BudgetSeverity::from_utilization(utilization_pct);

        SubsystemSample {
            current_ms: self.current_ms,
            avg_ms,
            min_ms,
            max_ms,
            p95_ms,
            utilization_pct,
            over_budget_count: self.over_budget_count_in_window,
            over_budget_streak: self.over_budget_streak,
            severity,
        }
    }

    fn reset_frame(&mut self) {
        self.current_ms = 0.0;
    }
}

/// RAII guard for timing a subsystem section.
pub struct SubsystemTimingGuard<'a> {
    tracker: &'a mut SubsystemBudgetTracker,
    subsystem_id: SubsystemId,
    start: Instant,
}

impl<'a> SubsystemTimingGuard<'a> {
    fn new(tracker: &'a mut SubsystemBudgetTracker, subsystem_id: SubsystemId) -> Self {
        Self {
            tracker,
            subsystem_id,
            start: Instant::now(),
        }
    }
}

impl Drop for SubsystemTimingGuard<'_> {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        let duration_ms = elapsed.as_secs_f32() * 1000.0;
        if let Some(state) = self.tracker.subsystems.get_mut(&self.subsystem_id) {
            state.record(duration_ms);
        }
    }
}

/// Tracks CPU-side performance budgets for multiple subsystems.
#[derive(Debug, Default)]
pub struct SubsystemBudgetTracker {
    subsystems: BTreeMap<SubsystemId, SubsystemState>,
}

impl SubsystemBudgetTracker {
    /// Create a new empty tracker.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a subsystem with a budget configuration.
    pub fn register(&mut self, config: SubsystemBudgetConfig) {
        let id = config.id.clone();
        self.subsystems.insert(id, SubsystemState::new(config));
    }

    /// Begin timing a subsystem section. Returns a guard that records duration on drop.
    pub fn begin_section(
        &mut self,
        subsystem_id: impl Into<SubsystemId>,
    ) -> SubsystemTimingGuard<'_> {
        SubsystemTimingGuard::new(self, subsystem_id.into())
    }

    /// Record an explicit duration for a subsystem.
    pub fn record_duration(&mut self, subsystem_id: impl Into<SubsystemId>, duration: Duration) {
        let id = subsystem_id.into();
        let duration_ms = duration.as_secs_f32() * 1000.0;
        if let Some(state) = self.subsystems.get_mut(&id) {
            state.record(duration_ms);
        }
    }

    /// Record an explicit duration in milliseconds for a subsystem.
    pub fn record_duration_ms(&mut self, subsystem_id: impl Into<SubsystemId>, duration_ms: f32) {
        let id = subsystem_id.into();
        if let Some(state) = self.subsystems.get_mut(&id) {
            state.record(duration_ms);
        }
    }

    /// Reset per-frame current values. Call at frame start.
    pub fn reset_frame(&mut self) {
        for state in self.subsystems.values_mut() {
            state.reset_frame();
        }
    }

    /// Produce a snapshot of all subsystem budgets.
    #[must_use]
    pub fn snapshot(&self) -> BudgetSnapshot {
        let mut subsystems: Vec<SubsystemSnapshot> = self
            .subsystems
            .iter()
            .map(|(id, state)| SubsystemSnapshot {
                id: id.clone(),
                category: state.config.category,
                budget_ms: state.config.budget_ms,
                sample: state.compute_sample(),
            })
            .collect();

        subsystems.sort_by(|a, b| {
            a.category
                .cmp(&b.category)
                .then_with(|| a.id.0.cmp(&b.id.0))
        });

        let mut summary = BudgetFrameSummary::default();

        for snap in &subsystems {
            summary.total_ms += snap.sample.current_ms;

            if snap.sample.severity == BudgetSeverity::Critical {
                summary.subsystems_over_budget += 1;
            }

            if snap.sample.utilization_pct > summary.max_utilization_pct {
                summary.max_utilization_pct = snap.sample.utilization_pct;
                summary.max_utilization_subsystem = Some(snap.id.clone());
            }

            if snap.sample.severity > summary.overall_severity {
                summary.overall_severity = snap.sample.severity;
            }
        }

        BudgetSnapshot {
            subsystems,
            summary,
        }
    }

    /// Check if a subsystem is registered.
    #[must_use]
    pub fn is_registered(&self, subsystem_id: impl Into<SubsystemId>) -> bool {
        self.subsystems.contains_key(&subsystem_id.into())
    }

    /// Get the number of registered subsystems.
    #[must_use]
    pub fn subsystem_count(&self) -> usize {
        self.subsystems.len()
    }
}

/// Frame timing statistics.
#[derive(Clone, Debug)]
pub struct FrameStats {
    /// Current frame time in milliseconds.
    pub frame_time_ms: f32,
    /// Current frames per second.
    pub fps: f32,
    /// Average frame time over sample window.
    pub avg_frame_time_ms: f32,
    /// Minimum frame time in sample window.
    pub min_frame_time_ms: f32,
    /// Maximum frame time in sample window.
    pub max_frame_time_ms: f32,
    /// 1% low FPS (99th percentile frame time).
    pub fps_1_low: f32,
}

impl Default for FrameStats {
    fn default() -> Self {
        Self {
            frame_time_ms: 16.67,
            fps: 60.0,
            avg_frame_time_ms: 16.67,
            min_frame_time_ms: 16.67,
            max_frame_time_ms: 16.67,
            fps_1_low: 60.0,
        }
    }
}

/// Frame time tracker for performance monitoring.
#[derive(Debug)]
pub struct FrameTimer {
    /// Last frame start time.
    last_frame: Instant,
    /// Frame time samples for averaging.
    samples: VecDeque<f32>,
    /// Maximum samples to keep.
    max_samples: usize,
    /// Current stats.
    stats: FrameStats,
}

impl Default for FrameTimer {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameTimer {
    /// Create a new frame timer.
    #[must_use]
    pub fn new() -> Self {
        Self::with_sample_count(120)
    }

    /// Create a frame timer with custom sample count.
    #[must_use]
    pub fn with_sample_count(max_samples: usize) -> Self {
        Self {
            last_frame: Instant::now(),
            samples: VecDeque::with_capacity(max_samples),
            max_samples,
            stats: FrameStats::default(),
        }
    }

    /// Update the timer at the start of each frame.
    pub fn tick(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame);
        self.last_frame = now;

        let frame_time_ms = dt.as_secs_f32() * 1000.0;

        // Add sample
        self.samples.push_back(frame_time_ms);
        if self.samples.len() > self.max_samples {
            self.samples.pop_front();
        }

        // Update stats
        self.update_stats(frame_time_ms);
    }

    fn update_stats(&mut self, frame_time_ms: f32) {
        self.stats.frame_time_ms = frame_time_ms;
        self.stats.fps = if frame_time_ms > 0.0 {
            1000.0 / frame_time_ms
        } else {
            0.0
        };

        if self.samples.is_empty() {
            return;
        }

        // Calculate average
        let sum: f32 = self.samples.iter().sum();
        #[expect(
            clippy::cast_precision_loss,
            reason = "sample count is small (max 120); precision loss is negligible"
        )]
        let count = self.samples.len() as f32;
        self.stats.avg_frame_time_ms = sum / count;

        // Calculate min/max
        self.stats.min_frame_time_ms = self.samples.iter().copied().fold(f32::MAX, f32::min);
        self.stats.max_frame_time_ms = self.samples.iter().copied().fold(0.0, f32::max);

        // Calculate 1% low (99th percentile frame time)
        if self.samples.len() >= 10 {
            let mut sorted: Vec<f32> = self.samples.iter().copied().collect();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let idx = (sorted.len() * 99) / 100;
            let percentile_time = sorted[idx.min(sorted.len() - 1)];
            self.stats.fps_1_low = if percentile_time > 0.0 {
                1000.0 / percentile_time
            } else {
                0.0
            };
        }
    }

    /// Get current frame statistics.
    #[must_use]
    pub fn stats(&self) -> &FrameStats {
        &self.stats
    }

    /// Get the last frame time.
    #[must_use]
    pub fn frame_time(&self) -> Duration {
        Duration::from_secs_f32(self.stats.frame_time_ms / 1000.0)
    }

    /// Get current FPS.
    #[must_use]
    pub fn fps(&self) -> f32 {
        self.stats.fps
    }

    /// Get average frame time in milliseconds.
    #[must_use]
    pub fn avg_frame_time_ms(&self) -> f32 {
        self.stats.avg_frame_time_ms
    }
}

/// Profiling scope guard using puffin.
#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => {
        puffin::profile_scope!($name);
    };
}

/// Profile a function using puffin.
#[macro_export]
macro_rules! profile_function {
    () => {
        puffin::profile_function!();
    };
}

/// Initialize the profiler.
pub fn init_profiler() {
    puffin::set_scopes_on(true);
}

/// Check if profiler is enabled.
#[must_use]
pub fn is_profiler_enabled() -> bool {
    puffin::are_scopes_on()
}

/// Enable or disable profiler.
pub fn set_profiler_enabled(enabled: bool) {
    puffin::set_scopes_on(enabled);
}

/// Start a new profiler frame.
pub fn new_frame() {
    puffin::GlobalProfiler::lock().new_frame();
}

/// GPU timing tracker.
#[derive(Debug, Default)]
pub struct GpuTimings {
    /// Last recorded GPU frame time in milliseconds.
    pub frame_time_ms: f32,
    /// GPU memory usage in bytes (if available).
    pub memory_bytes: Option<u64>,
}

/// Performance metrics aggregator.
#[derive(Debug)]
pub struct PerformanceMetrics {
    /// Frame timing.
    pub frame_timer: FrameTimer,
    /// GPU timings.
    pub gpu: GpuTimings,
    /// Number of draw calls this frame.
    pub draw_calls: u32,
    /// Number of triangles rendered.
    pub triangles: u32,
    /// Number of chunks rendered.
    pub chunks_rendered: u32,
    /// Number of chunks loaded.
    pub chunks_loaded: u32,
    /// Number of entities.
    pub entity_count: u32,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceMetrics {
    /// Create new performance metrics tracker.
    #[must_use]
    pub fn new() -> Self {
        Self {
            frame_timer: FrameTimer::new(),
            gpu: GpuTimings::default(),
            draw_calls: 0,
            triangles: 0,
            chunks_rendered: 0,
            chunks_loaded: 0,
            entity_count: 0,
        }
    }

    /// Reset per-frame counters.
    pub fn reset_frame_counters(&mut self) {
        self.draw_calls = 0;
        self.triangles = 0;
        self.chunks_rendered = 0;
    }

    /// Record a draw call.
    pub fn record_draw_call(&mut self, triangle_count: u32) {
        self.draw_calls += 1;
        self.triangles += triangle_count;
    }

    /// Record chunk render.
    pub fn record_chunk_render(&mut self) {
        self.chunks_rendered += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_frame_timer_new() {
        let timer = FrameTimer::new();
        assert!(timer.samples.is_empty());
        assert_eq!(timer.max_samples, 120);
    }

    #[test]
    fn test_frame_timer_with_sample_count() {
        let timer = FrameTimer::with_sample_count(60);
        assert_eq!(timer.max_samples, 60);
    }

    #[test]
    fn test_frame_timer_tick() {
        let mut timer = FrameTimer::new();
        thread::sleep(Duration::from_millis(10));
        timer.tick();
        assert_eq!(timer.samples.len(), 1);
        assert!(timer.stats().frame_time_ms > 0.0);
    }

    #[test]
    fn test_frame_timer_multiple_ticks() {
        let mut timer = FrameTimer::with_sample_count(5);
        for _ in 0..10 {
            thread::sleep(Duration::from_millis(1));
            timer.tick();
        }
        // Should cap at max_samples
        assert_eq!(timer.samples.len(), 5);
    }

    #[test]
    fn test_frame_stats_default() {
        let stats = FrameStats::default();
        assert!((stats.fps - 60.0).abs() < 0.1);
        assert!((stats.frame_time_ms - 16.67).abs() < 0.1);
    }

    #[test]
    fn test_frame_timer_stats() {
        let mut timer = FrameTimer::new();
        for _ in 0..20 {
            thread::sleep(Duration::from_millis(5));
            timer.tick();
        }
        let stats = timer.stats();
        assert!(stats.avg_frame_time_ms > 0.0);
        assert!(stats.min_frame_time_ms > 0.0);
        assert!(stats.max_frame_time_ms >= stats.min_frame_time_ms);
    }

    #[test]
    fn test_performance_metrics_new() {
        let metrics = PerformanceMetrics::new();
        assert_eq!(metrics.draw_calls, 0);
        assert_eq!(metrics.triangles, 0);
        assert_eq!(metrics.chunks_rendered, 0);
    }

    #[test]
    fn test_performance_metrics_record_draw_call() {
        let mut metrics = PerformanceMetrics::new();
        metrics.record_draw_call(100);
        metrics.record_draw_call(200);
        assert_eq!(metrics.draw_calls, 2);
        assert_eq!(metrics.triangles, 300);
    }

    #[test]
    fn test_performance_metrics_reset() {
        let mut metrics = PerformanceMetrics::new();
        metrics.record_draw_call(100);
        metrics.record_chunk_render();
        metrics.reset_frame_counters();
        assert_eq!(metrics.draw_calls, 0);
        assert_eq!(metrics.triangles, 0);
        assert_eq!(metrics.chunks_rendered, 0);
    }

    #[test]
    fn test_profiler_toggle() {
        init_profiler();
        assert!(is_profiler_enabled());
        set_profiler_enabled(false);
        assert!(!is_profiler_enabled());
        set_profiler_enabled(true);
        assert!(is_profiler_enabled());
    }

    // ========================================================================
    // Subsystem Budget Tracking Tests
    // ========================================================================

    #[test]
    fn test_subsystem_id_creation() {
        let id = SubsystemId::new("physics");
        assert_eq!(id.name(), "physics");

        let id2: SubsystemId = "rendering".into();
        assert_eq!(id2.name(), "rendering");
    }

    #[test]
    fn test_subsystem_category_display() {
        assert_eq!(SubsystemCategory::Simulation.display_name(), "Simulation");
        assert_eq!(SubsystemCategory::Rendering.display_name(), "Rendering");
        assert_eq!(SubsystemCategory::Audio.display_name(), "Audio");
        assert_eq!(SubsystemCategory::Network.display_name(), "Network");
        assert_eq!(SubsystemCategory::Input.display_name(), "Input");
        assert_eq!(SubsystemCategory::Ui.display_name(), "UI");
        assert_eq!(SubsystemCategory::Assets.display_name(), "Assets");
        assert_eq!(SubsystemCategory::Custom.display_name(), "Custom");
    }

    #[test]
    fn test_budget_severity_from_utilization() {
        assert_eq!(BudgetSeverity::from_utilization(0.0), BudgetSeverity::Ok);
        assert_eq!(BudgetSeverity::from_utilization(50.0), BudgetSeverity::Ok);
        assert_eq!(BudgetSeverity::from_utilization(79.9), BudgetSeverity::Ok);
        assert_eq!(
            BudgetSeverity::from_utilization(80.0),
            BudgetSeverity::Warning
        );
        assert_eq!(
            BudgetSeverity::from_utilization(99.9),
            BudgetSeverity::Warning
        );
        assert_eq!(
            BudgetSeverity::from_utilization(100.0),
            BudgetSeverity::Critical
        );
        assert_eq!(
            BudgetSeverity::from_utilization(150.0),
            BudgetSeverity::Critical
        );
    }

    #[test]
    fn test_budget_config_builder() {
        let config = SubsystemBudgetConfig::new("physics", 8.0)
            .with_category(SubsystemCategory::Simulation)
            .with_max_samples(60);

        assert_eq!(config.id.name(), "physics");
        assert_eq!(config.category, SubsystemCategory::Simulation);
        assert!((config.budget_ms - 8.0).abs() < f32::EPSILON);
        assert_eq!(config.max_samples, 60);
    }

    #[test]
    fn test_tracker_register_and_count() {
        let mut tracker = SubsystemBudgetTracker::new();
        assert_eq!(tracker.subsystem_count(), 0);
        assert!(!tracker.is_registered("physics"));

        tracker.register(SubsystemBudgetConfig::new("physics", 8.0));
        assert_eq!(tracker.subsystem_count(), 1);
        assert!(tracker.is_registered("physics"));

        tracker.register(SubsystemBudgetConfig::new("rendering", 10.0));
        assert_eq!(tracker.subsystem_count(), 2);
    }

    #[test]
    fn test_record_duration_ms() {
        let mut tracker = SubsystemBudgetTracker::new();
        tracker.register(SubsystemBudgetConfig::new("physics", 10.0));

        tracker.record_duration_ms("physics", 5.0);
        let snapshot = tracker.snapshot();

        assert_eq!(snapshot.subsystems.len(), 1);
        let physics = &snapshot.subsystems[0];
        assert!((physics.sample.current_ms - 5.0).abs() < f32::EPSILON);
        assert!((physics.sample.utilization_pct - 50.0).abs() < f32::EPSILON);
        assert_eq!(physics.sample.severity, BudgetSeverity::Ok);
    }

    #[test]
    fn test_record_duration() {
        let mut tracker = SubsystemBudgetTracker::new();
        tracker.register(SubsystemBudgetConfig::new("audio", 5.0));

        tracker.record_duration("audio", Duration::from_millis(3));
        let snapshot = tracker.snapshot();

        let audio = &snapshot.subsystems[0];
        assert!((audio.sample.current_ms - 3.0).abs() < 0.1);
    }

    #[test]
    fn test_over_budget_detection() {
        let mut tracker = SubsystemBudgetTracker::new();
        tracker.register(SubsystemBudgetConfig::new("physics", 5.0));

        tracker.record_duration_ms("physics", 6.0);
        let snapshot = tracker.snapshot();

        let physics = &snapshot.subsystems[0];
        assert_eq!(physics.sample.severity, BudgetSeverity::Critical);
        assert_eq!(physics.sample.over_budget_count, 1);
        assert_eq!(physics.sample.over_budget_streak, 1);
    }

    #[test]
    fn test_over_budget_streak() {
        let mut tracker = SubsystemBudgetTracker::new();
        tracker.register(SubsystemBudgetConfig::new("physics", 5.0));

        // Record 3 over-budget frames
        tracker.record_duration_ms("physics", 6.0);
        tracker.record_duration_ms("physics", 7.0);
        tracker.record_duration_ms("physics", 8.0);

        let snapshot = tracker.snapshot();
        let physics = &snapshot.subsystems[0];
        assert_eq!(physics.sample.over_budget_streak, 3);
        assert_eq!(physics.sample.over_budget_count, 3);

        // Record one within-budget frame, streak should reset
        tracker.record_duration_ms("physics", 4.0);
        let snapshot = tracker.snapshot();
        let physics = &snapshot.subsystems[0];
        assert_eq!(physics.sample.over_budget_streak, 0);
        assert_eq!(physics.sample.over_budget_count, 3);
    }

    #[test]
    fn test_statistics_calculation() {
        let mut tracker = SubsystemBudgetTracker::new();
        tracker.register(SubsystemBudgetConfig::new("test", 10.0).with_max_samples(5));

        // Record values: 2, 4, 6, 8, 10
        for v in [2.0, 4.0, 6.0, 8.0, 10.0] {
            tracker.record_duration_ms("test", v);
        }

        let snapshot = tracker.snapshot();
        let test = &snapshot.subsystems[0];

        assert!((test.sample.current_ms - 10.0).abs() < f32::EPSILON);
        assert!((test.sample.avg_ms - 6.0).abs() < f32::EPSILON);
        assert!((test.sample.min_ms - 2.0).abs() < f32::EPSILON);
        assert!((test.sample.max_ms - 10.0).abs() < f32::EPSILON);
        // p95 with 5 samples: index (5 * 95) / 100 = 4, so value at index 4 = 10
        assert!((test.sample.p95_ms - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_sample_window_cap() {
        let mut tracker = SubsystemBudgetTracker::new();
        tracker.register(SubsystemBudgetConfig::new("test", 10.0).with_max_samples(3));

        // Record more samples than max
        for v in [1.0, 2.0, 3.0, 4.0, 5.0] {
            tracker.record_duration_ms("test", v);
        }

        let snapshot = tracker.snapshot();
        let test = &snapshot.subsystems[0];

        // Should only have last 3 values: 3, 4, 5
        assert!((test.sample.min_ms - 3.0).abs() < f32::EPSILON);
        assert!((test.sample.max_ms - 5.0).abs() < f32::EPSILON);
        assert!((test.sample.avg_ms - 4.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_over_budget_count_with_window_cap() {
        let mut tracker = SubsystemBudgetTracker::new();
        tracker.register(SubsystemBudgetConfig::new("test", 5.0).with_max_samples(3));

        // All over budget
        tracker.record_duration_ms("test", 6.0);
        tracker.record_duration_ms("test", 7.0);
        tracker.record_duration_ms("test", 8.0);

        let snapshot = tracker.snapshot();
        assert_eq!(snapshot.subsystems[0].sample.over_budget_count, 3);

        // Add one more, oldest should be evicted
        tracker.record_duration_ms("test", 9.0);
        let snapshot = tracker.snapshot();
        assert_eq!(snapshot.subsystems[0].sample.over_budget_count, 3);

        // Add within-budget, oldest over-budget evicted
        tracker.record_duration_ms("test", 4.0);
        let snapshot = tracker.snapshot();
        assert_eq!(snapshot.subsystems[0].sample.over_budget_count, 2);
    }

    #[test]
    fn test_reset_frame() {
        let mut tracker = SubsystemBudgetTracker::new();
        tracker.register(SubsystemBudgetConfig::new("physics", 10.0));

        tracker.record_duration_ms("physics", 5.0);
        let snapshot = tracker.snapshot();
        assert!((snapshot.subsystems[0].sample.current_ms - 5.0).abs() < f32::EPSILON);

        tracker.reset_frame();
        let snapshot = tracker.snapshot();
        assert!((snapshot.subsystems[0].sample.current_ms).abs() < f32::EPSILON);
        // Samples should still be retained
        assert!((snapshot.subsystems[0].sample.avg_ms - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_snapshot_ordering() {
        let mut tracker = SubsystemBudgetTracker::new();

        // Register in random order
        tracker.register(
            SubsystemBudgetConfig::new("z_system", 5.0).with_category(SubsystemCategory::Rendering),
        );
        tracker.register(
            SubsystemBudgetConfig::new("a_system", 5.0).with_category(SubsystemCategory::Rendering),
        );
        tracker.register(
            SubsystemBudgetConfig::new("physics", 5.0).with_category(SubsystemCategory::Simulation),
        );

        let snapshot = tracker.snapshot();

        // Should be ordered by category (Simulation < Rendering) then by name
        assert_eq!(snapshot.subsystems[0].id.name(), "physics");
        assert_eq!(snapshot.subsystems[1].id.name(), "a_system");
        assert_eq!(snapshot.subsystems[2].id.name(), "z_system");
    }

    #[test]
    fn test_frame_summary() {
        let mut tracker = SubsystemBudgetTracker::new();
        tracker.register(SubsystemBudgetConfig::new("physics", 5.0));
        tracker.register(SubsystemBudgetConfig::new("rendering", 10.0));

        tracker.record_duration_ms("physics", 6.0); // 120% utilization, critical
        tracker.record_duration_ms("rendering", 8.0); // 80% utilization, warning

        let snapshot = tracker.snapshot();
        let summary = &snapshot.summary;

        assert!((summary.total_ms - 14.0).abs() < 0.01);
        assert_eq!(summary.subsystems_over_budget, 1);
        assert!((summary.max_utilization_pct - 120.0).abs() < 0.01);
        assert_eq!(
            summary
                .max_utilization_subsystem
                .as_ref()
                .map(SubsystemId::name),
            Some("physics")
        );
        assert_eq!(summary.overall_severity, BudgetSeverity::Critical);
    }

    #[test]
    fn test_timing_guard() {
        let mut tracker = SubsystemBudgetTracker::new();
        tracker.register(SubsystemBudgetConfig::new("test_section", 100.0));

        {
            let _guard = tracker.begin_section("test_section");
            thread::sleep(Duration::from_millis(5));
        }

        let snapshot = tracker.snapshot();
        let section = &snapshot.subsystems[0];
        // Should have recorded some duration
        assert!(section.sample.current_ms >= 4.0);
    }

    #[test]
    fn test_unregistered_subsystem_ignored() {
        let mut tracker = SubsystemBudgetTracker::new();
        tracker.register(SubsystemBudgetConfig::new("registered", 10.0));

        // Recording to unregistered subsystem should not panic
        tracker.record_duration_ms("unregistered", 5.0);

        let snapshot = tracker.snapshot();
        assert_eq!(snapshot.subsystems.len(), 1);
        assert_eq!(snapshot.subsystems[0].id.name(), "registered");
    }

    #[test]
    fn test_empty_tracker_snapshot() {
        let tracker = SubsystemBudgetTracker::new();
        let snapshot = tracker.snapshot();

        assert!(snapshot.subsystems.is_empty());
        assert!((snapshot.summary.total_ms).abs() < f32::EPSILON);
        assert_eq!(snapshot.summary.subsystems_over_budget, 0);
        assert_eq!(snapshot.summary.overall_severity, BudgetSeverity::Ok);
    }

    #[test]
    fn test_zero_budget_handling() {
        let mut tracker = SubsystemBudgetTracker::new();
        tracker.register(SubsystemBudgetConfig::new("zero_budget", 0.0));

        tracker.record_duration_ms("zero_budget", 5.0);
        let snapshot = tracker.snapshot();

        // Utilization should be 0 (not divide by zero)
        assert!((snapshot.subsystems[0].sample.utilization_pct).abs() < f32::EPSILON);
    }
}
