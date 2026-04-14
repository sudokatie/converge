//! Performance profiling utilities.
//!
//! Provides frame timing, FPS tracking, and puffin integration for detailed profiling.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

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
        self.stats.avg_frame_time_ms = sum / self.samples.len() as f32;

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
}
