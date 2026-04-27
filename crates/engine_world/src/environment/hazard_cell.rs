//! Per-cell hazard state.

use serde::{Deserialize, Serialize};

/// State of a hazard at a single cell.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HazardCell {
    /// Intensity of the hazard (0.0 = inactive, 1.0 = maximum).
    intensity: f32,

    /// Accumulated time since last spread attempt (seconds).
    spread_timer: f32,

    /// Accumulated time since intensity last increased (seconds).
    decay_timer: f32,
}

impl HazardCell {
    /// Inactive cell state.
    pub const INACTIVE: Self = Self {
        intensity: 0.0,
        spread_timer: 0.0,
        decay_timer: 0.0,
    };

    /// Create a new hazard cell with given intensity.
    #[must_use]
    pub fn new(intensity: f32) -> Self {
        Self {
            intensity: intensity.clamp(0.0, 1.0),
            spread_timer: 0.0,
            decay_timer: 0.0,
        }
    }

    /// Get the current intensity.
    #[must_use]
    pub const fn intensity(&self) -> f32 {
        self.intensity
    }

    /// Check if the cell is active (intensity > 0).
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.intensity > 0.0
    }

    /// Get the spread timer value.
    #[must_use]
    pub const fn spread_timer(&self) -> f32 {
        self.spread_timer
    }

    /// Get the decay timer value.
    #[must_use]
    pub const fn decay_timer(&self) -> f32 {
        self.decay_timer
    }

    /// Set intensity, clamping to valid range.
    pub fn set_intensity(&mut self, value: f32) {
        let old = self.intensity;
        self.intensity = value.clamp(0.0, 1.0);
        if self.intensity > old {
            self.decay_timer = 0.0;
        }
    }

    /// Add to intensity, clamping to valid range.
    pub fn add_intensity(&mut self, delta: f32) {
        self.set_intensity(self.intensity + delta);
    }

    /// Advance the spread timer, returning true if threshold reached.
    pub fn tick_spread(&mut self, dt: f32, threshold: f32) -> bool {
        self.spread_timer += dt;
        if self.spread_timer >= threshold {
            self.spread_timer = 0.0;
            true
        } else {
            false
        }
    }

    /// Advance the decay timer by dt.
    pub fn tick_decay(&mut self, dt: f32) {
        self.decay_timer += dt;
    }

    /// Apply decay based on rate and accumulated time, resetting timer.
    pub fn apply_decay(&mut self, rate: f32) {
        let decay_amount = rate * self.decay_timer;
        self.intensity = (self.intensity - decay_amount).max(0.0);
        self.decay_timer = 0.0;
    }

    /// Deactivate the cell completely.
    pub fn deactivate(&mut self) {
        *self = Self::INACTIVE;
    }

    /// Reset timers without changing intensity.
    pub fn reset_timers(&mut self) {
        self.spread_timer = 0.0;
        self.decay_timer = 0.0;
    }
}

impl Default for HazardCell {
    fn default() -> Self {
        Self::INACTIVE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inactive_default() {
        let cell = HazardCell::default();
        assert!(!cell.is_active());
        assert_eq!(cell.intensity(), 0.0);
    }

    #[test]
    fn new_clamps_intensity() {
        let low = HazardCell::new(-0.5);
        assert_eq!(low.intensity(), 0.0);

        let high = HazardCell::new(1.5);
        assert_eq!(high.intensity(), 1.0);

        let mid = HazardCell::new(0.5);
        assert!((mid.intensity() - 0.5).abs() < 0.001);
    }

    #[test]
    fn set_intensity_resets_decay_on_increase() {
        let mut cell = HazardCell::new(0.5);
        cell.tick_decay(1.0);
        assert!((cell.decay_timer() - 1.0).abs() < 0.001);

        cell.set_intensity(0.7);
        assert_eq!(cell.decay_timer(), 0.0);
    }

    #[test]
    fn set_intensity_preserves_decay_on_decrease() {
        let mut cell = HazardCell::new(0.5);
        cell.tick_decay(1.0);

        cell.set_intensity(0.3);
        assert!((cell.decay_timer() - 1.0).abs() < 0.001);
    }

    #[test]
    fn add_intensity_clamps() {
        let mut cell = HazardCell::new(0.8);
        cell.add_intensity(0.5);
        assert_eq!(cell.intensity(), 1.0);

        cell.add_intensity(-1.5);
        assert_eq!(cell.intensity(), 0.0);
    }

    #[test]
    fn tick_spread_threshold() {
        let mut cell = HazardCell::new(1.0);

        assert!(!cell.tick_spread(0.3, 1.0));
        assert!((cell.spread_timer() - 0.3).abs() < 0.001);

        assert!(!cell.tick_spread(0.5, 1.0));
        assert!((cell.spread_timer() - 0.8).abs() < 0.001);

        assert!(cell.tick_spread(0.3, 1.0));
        assert_eq!(cell.spread_timer(), 0.0);
    }

    #[test]
    fn apply_decay_reduces_intensity() {
        let mut cell = HazardCell::new(1.0);
        cell.tick_decay(2.0);
        cell.apply_decay(0.25);

        assert!((cell.intensity() - 0.5).abs() < 0.001);
        assert_eq!(cell.decay_timer(), 0.0);
    }

    #[test]
    fn apply_decay_floors_at_zero() {
        let mut cell = HazardCell::new(0.2);
        cell.tick_decay(10.0);
        cell.apply_decay(0.5);

        assert_eq!(cell.intensity(), 0.0);
    }

    #[test]
    fn deactivate_resets_all() {
        let mut cell = HazardCell::new(0.8);
        cell.tick_spread(0.5, 1.0);
        cell.tick_decay(0.5);

        cell.deactivate();

        assert_eq!(cell, HazardCell::INACTIVE);
    }

    #[test]
    fn serde_round_trip() {
        let cell = HazardCell::new(0.75);
        let json = serde_json::to_string(&cell).unwrap();
        let recovered: HazardCell = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, cell);
    }
}
