//! Timing utilities for game loops.

use std::time::{Duration, Instant};

/// Maximum delta time to prevent spiral of death.
const MAX_DELTA: Duration = Duration::from_millis(250);

/// Maximum fixed timestep iterations per frame.
const MAX_FIXED_STEPS: u32 = 4;

/// Game clock for tracking frame timing.
#[derive(Debug)]
pub struct Clock {
    last_instant: Instant,
    delta: Duration,
    total: Duration,
}

impl Clock {
    /// Create a new clock.
    #[must_use]
    pub fn new() -> Self {
        Self {
            last_instant: Instant::now(),
            delta: Duration::ZERO,
            total: Duration::ZERO,
        }
    }

    /// Update the clock at the start of each frame.
    ///
    /// Returns the time since the last tick.
    pub fn tick(&mut self) -> Duration {
        let now = Instant::now();
        let mut delta = now - self.last_instant;
        self.last_instant = now;

        // Cap delta to prevent spiral of death
        if delta > MAX_DELTA {
            delta = MAX_DELTA;
        }

        self.delta = delta;
        self.total += delta;
        delta
    }

    /// Get the time since the last frame in seconds.
    #[must_use]
    pub fn delta_secs(&self) -> f32 {
        self.delta.as_secs_f32()
    }

    /// Get the delta as a Duration.
    #[must_use]
    pub fn delta(&self) -> Duration {
        self.delta
    }

    /// Get the total elapsed time in seconds.
    #[must_use]
    pub fn total_secs(&self) -> f64 {
        self.total.as_secs_f64()
    }

    /// Get the total elapsed time as a Duration.
    #[must_use]
    pub fn total(&self) -> Duration {
        self.total
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self::new()
    }
}

/// Fixed timestep accumulator for physics/gameplay updates.
///
/// Ensures consistent update rate regardless of frame rate.
#[derive(Debug)]
pub struct FixedTimestep {
    accumulator: Duration,
    step: Duration,
    step_count: u32,
}

impl FixedTimestep {
    /// Create a fixed timestep at the given frequency (Hz).
    #[must_use]
    pub fn new(hz: f32) -> Self {
        Self {
            accumulator: Duration::ZERO,
            step: Duration::from_secs_f32(1.0 / hz),
            step_count: 0,
        }
    }

    /// Create a fixed timestep from a Duration.
    #[must_use]
    pub fn from_duration(step: Duration) -> Self {
        Self {
            accumulator: Duration::ZERO,
            step,
            step_count: 0,
        }
    }

    /// Add frame time to the accumulator.
    ///
    /// Call this once per frame with the frame delta.
    pub fn accumulate(&mut self, dt: Duration) {
        self.accumulator += dt;
        self.step_count = 0;
    }

    /// Check if a fixed step should run.
    ///
    /// Returns true and subtracts from accumulator if enough time has passed.
    /// Limited to MAX_FIXED_STEPS per frame to prevent spiral of death.
    pub fn should_step(&mut self) -> bool {
        if self.accumulator >= self.step && self.step_count < MAX_FIXED_STEPS {
            self.accumulator -= self.step;
            self.step_count += 1;
            true
        } else {
            false
        }
    }

    /// Get the interpolation alpha for rendering.
    ///
    /// Returns a value in [0, 1] representing how far into the next step we are.
    /// Use this to interpolate between physics states for smooth rendering.
    #[must_use]
    pub fn alpha(&self) -> f32 {
        (self.accumulator.as_secs_f32() / self.step.as_secs_f32()).clamp(0.0, 1.0)
    }

    /// Get the fixed step duration.
    #[must_use]
    pub fn step(&self) -> Duration {
        self.step
    }

    /// Get the fixed step duration in seconds.
    #[must_use]
    pub fn step_secs(&self) -> f32 {
        self.step.as_secs_f32()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_clock_delta_positive() {
        let mut clock = Clock::new();
        thread::sleep(Duration::from_millis(10));
        let delta = clock.tick();
        assert!(delta > Duration::ZERO);
        assert!(clock.delta_secs() > 0.0);
    }

    #[test]
    fn test_clock_total_accumulates() {
        let mut clock = Clock::new();
        thread::sleep(Duration::from_millis(5));
        clock.tick();
        let t1 = clock.total_secs();

        thread::sleep(Duration::from_millis(5));
        clock.tick();
        let t2 = clock.total_secs();

        assert!(t2 > t1);
    }

    #[test]
    fn test_fixed_timestep_steps() {
        let mut ts = FixedTimestep::new(60.0);

        // Add about 3 frames worth at 60 FPS
        ts.accumulate(Duration::from_millis(50));

        let mut count = 0;
        while ts.should_step() {
            count += 1;
        }

        // 50ms / 16.67ms = ~3 steps
        assert!(count >= 2 && count <= 4, "Expected 2-4 steps, got {count}");
    }

    #[test]
    fn test_fixed_timestep_alpha() {
        let mut ts = FixedTimestep::new(60.0);

        // Accumulate half a step
        ts.accumulate(Duration::from_millis(8));

        // Should not step yet
        assert!(!ts.should_step());

        // Alpha should be around 0.5
        let alpha = ts.alpha();
        assert!(alpha > 0.3 && alpha < 0.7, "Alpha was {alpha}");
    }

    #[test]
    fn test_fixed_timestep_max_steps() {
        let mut ts = FixedTimestep::new(60.0);

        // Add a huge amount of time (simulating a lag spike)
        ts.accumulate(Duration::from_secs(1));

        let mut count = 0;
        while ts.should_step() {
            count += 1;
        }

        // Should be capped at MAX_FIXED_STEPS
        assert_eq!(count, MAX_FIXED_STEPS);
    }

    #[test]
    fn test_alpha_clamped() {
        let mut ts = FixedTimestep::new(60.0);
        ts.accumulate(Duration::from_millis(100)); // More than one step

        // Drain all steps
        while ts.should_step() {}

        // Alpha should be in valid range
        let alpha = ts.alpha();
        assert!((0.0..=1.0).contains(&alpha));
    }
}
