//! Per-cell fluid state with volume, pressure, temperature, and kind.

use serde::{Deserialize, Serialize};

use super::FluidKind;

/// Maximum volume a cell can hold (1.0 = fully filled).
pub const MAX_VOLUME: f32 = 1.0;

/// Minimum volume threshold below which cell is considered empty.
pub const MIN_VOLUME: f32 = 0.001;

/// Maximum pressure in atmospheres.
pub const MAX_PRESSURE: f32 = 100.0;

/// Minimum pressure (vacuum).
pub const MIN_PRESSURE: f32 = 0.0;

/// Maximum temperature in Celsius.
pub const MAX_TEMPERATURE: f32 = 2000.0;

/// Minimum temperature in Celsius (absolute zero approximation).
pub const MIN_TEMPERATURE: f32 = -273.0;

/// Fluid state for a single cell.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FluidCell {
    volume: f32,
    pressure: f32,
    temperature: f32,
    kind: FluidKind,
}

impl FluidCell {
    /// Empty cell with no fluid.
    pub const EMPTY: Self = Self {
        volume: 0.0,
        pressure: 1.0,
        temperature: 20.0,
        kind: FluidKind::Water,
    };

    /// Create a new fluid cell.
    #[must_use]
    pub fn new(kind: FluidKind, volume: f32) -> Self {
        Self {
            volume: volume.clamp(0.0, MAX_VOLUME),
            pressure: 1.0,
            temperature: kind.default_temperature(),
            kind,
        }
    }

    /// Create a fully filled cell.
    #[must_use]
    pub fn filled(kind: FluidKind) -> Self {
        Self::new(kind, MAX_VOLUME)
    }

    /// Create a cell with all properties specified.
    #[must_use]
    pub fn with_state(kind: FluidKind, volume: f32, pressure: f32, temperature: f32) -> Self {
        Self {
            volume: volume.clamp(0.0, MAX_VOLUME),
            pressure: pressure.clamp(MIN_PRESSURE, MAX_PRESSURE),
            temperature: temperature.clamp(MIN_TEMPERATURE, MAX_TEMPERATURE),
            kind,
        }
    }

    /// Get the fluid kind.
    #[must_use]
    pub const fn kind(&self) -> FluidKind {
        self.kind
    }

    /// Get the current volume (0.0 to 1.0).
    #[must_use]
    pub const fn volume(&self) -> f32 {
        self.volume
    }

    /// Get the current pressure in atmospheres.
    #[must_use]
    pub const fn pressure(&self) -> f32 {
        self.pressure
    }

    /// Get the current temperature in Celsius.
    #[must_use]
    pub const fn temperature(&self) -> f32 {
        self.temperature
    }

    /// Check if the cell is effectively empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.volume < MIN_VOLUME
    }

    /// Check if the cell is full.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.volume >= MAX_VOLUME - MIN_VOLUME
    }

    /// Set the volume (clamped to valid range).
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, MAX_VOLUME);
    }

    /// Add volume (clamped, returns overflow).
    pub fn add_volume(&mut self, delta: f32) -> f32 {
        let new_volume = self.volume + delta;
        if new_volume > MAX_VOLUME {
            self.volume = MAX_VOLUME;
            new_volume - MAX_VOLUME
        } else if new_volume < 0.0 {
            self.volume = 0.0;
            new_volume
        } else {
            self.volume = new_volume;
            0.0
        }
    }

    /// Remove volume (returns amount actually removed).
    pub fn remove_volume(&mut self, amount: f32) -> f32 {
        let removed = amount.min(self.volume);
        self.volume -= removed;
        removed
    }

    /// Set the pressure (clamped to valid range).
    pub fn set_pressure(&mut self, pressure: f32) {
        self.pressure = pressure.clamp(MIN_PRESSURE, MAX_PRESSURE);
    }

    /// Set the temperature (clamped to valid range).
    pub fn set_temperature(&mut self, temperature: f32) {
        self.temperature = temperature.clamp(MIN_TEMPERATURE, MAX_TEMPERATURE);
    }

    /// Set the fluid kind.
    pub fn set_kind(&mut self, kind: FluidKind) {
        self.kind = kind;
    }

    /// Available capacity (how much more can fit).
    #[must_use]
    pub fn available_capacity(&self) -> f32 {
        (MAX_VOLUME - self.volume).max(0.0)
    }

    /// Clamp all values to valid ranges.
    pub fn clamp(&mut self) {
        self.volume = self.volume.clamp(0.0, MAX_VOLUME);
        self.pressure = self.pressure.clamp(MIN_PRESSURE, MAX_PRESSURE);
        self.temperature = self.temperature.clamp(MIN_TEMPERATURE, MAX_TEMPERATURE);
    }
}

impl Default for FluidCell {
    fn default() -> Self {
        Self::EMPTY
    }
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    reason = "tests check exact constructor return values"
)]
mod tests {
    use super::*;

    #[test]
    fn empty_cell() {
        let cell = FluidCell::EMPTY;
        assert!(cell.is_empty());
        assert!(!cell.is_full());
        assert_eq!(cell.volume(), 0.0);
    }

    #[test]
    fn new_cell_defaults() {
        let cell = FluidCell::new(FluidKind::Water, 0.5);
        assert_eq!(cell.kind(), FluidKind::Water);
        assert_eq!(cell.volume(), 0.5);
        assert_eq!(cell.pressure(), 1.0);
        assert_eq!(cell.temperature(), 20.0);
    }

    #[test]
    fn filled_cell() {
        let cell = FluidCell::filled(FluidKind::Lava);
        assert!(cell.is_full());
        assert!(!cell.is_empty());
        assert_eq!(cell.volume(), 1.0);
        assert_eq!(cell.temperature(), 1200.0);
    }

    #[test]
    fn volume_clamping() {
        let mut cell = FluidCell::new(FluidKind::Water, 2.0);
        assert_eq!(cell.volume(), 1.0);

        cell.set_volume(-1.0);
        assert_eq!(cell.volume(), 0.0);
    }

    #[test]
    fn add_volume_overflow() {
        let mut cell = FluidCell::new(FluidKind::Water, 0.8);
        let overflow = cell.add_volume(0.5);
        assert_eq!(cell.volume(), 1.0);
        assert!((overflow - 0.3).abs() < 0.001);
    }

    #[test]
    fn add_volume_underflow() {
        let mut cell = FluidCell::new(FluidKind::Water, 0.2);
        let underflow = cell.add_volume(-0.5);
        assert_eq!(cell.volume(), 0.0);
        assert!((underflow - (-0.3)).abs() < 0.001);
    }

    #[test]
    fn remove_volume() {
        let mut cell = FluidCell::new(FluidKind::Water, 0.5);
        let removed = cell.remove_volume(0.3);
        assert!((removed - 0.3).abs() < 0.001);
        assert!((cell.volume() - 0.2).abs() < 0.001);
    }

    #[test]
    fn remove_volume_capped() {
        let mut cell = FluidCell::new(FluidKind::Water, 0.2);
        let removed = cell.remove_volume(0.5);
        assert!((removed - 0.2).abs() < 0.001);
        assert_eq!(cell.volume(), 0.0);
    }

    #[test]
    fn pressure_clamping() {
        let cell = FluidCell::with_state(FluidKind::Water, 0.5, 200.0, 20.0);
        assert_eq!(cell.pressure(), MAX_PRESSURE);

        let cell2 = FluidCell::with_state(FluidKind::Water, 0.5, -10.0, 20.0);
        assert_eq!(cell2.pressure(), MIN_PRESSURE);
    }

    #[test]
    fn temperature_clamping() {
        let cell = FluidCell::with_state(FluidKind::Water, 0.5, 1.0, 3000.0);
        assert_eq!(cell.temperature(), MAX_TEMPERATURE);

        let cell2 = FluidCell::with_state(FluidKind::Water, 0.5, 1.0, -500.0);
        assert_eq!(cell2.temperature(), MIN_TEMPERATURE);
    }

    #[test]
    fn available_capacity() {
        let cell = FluidCell::new(FluidKind::Water, 0.3);
        assert!((cell.available_capacity() - 0.7).abs() < 0.001);

        let full = FluidCell::filled(FluidKind::Water);
        assert_eq!(full.available_capacity(), 0.0);
    }

    #[test]
    fn serde_round_trip() {
        let cell = FluidCell::with_state(FluidKind::Lava, 0.75, 2.5, 1100.0);
        let json = serde_json::to_string(&cell).unwrap();
        let recovered: FluidCell = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, cell);
    }

    #[test]
    fn default_is_empty() {
        let cell = FluidCell::default();
        assert_eq!(cell, FluidCell::EMPTY);
    }
}
