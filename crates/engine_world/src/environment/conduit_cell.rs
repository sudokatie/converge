//! Per-cell conduit state for infrastructure networks.

use serde::{Deserialize, Serialize};

use super::ConduitKind;

/// Maximum stored amount in a conduit cell.
pub const MAX_STORED: f32 = 100.0;
/// Minimum stored amount (below this is considered empty).
pub const MIN_STORED: f32 = 0.001;
/// Maximum temperature for heat conduits.
pub const MAX_TEMPERATURE: f32 = 2000.0;
/// Minimum temperature for heat conduits.
pub const MIN_TEMPERATURE: f32 = -273.15;
/// Maximum pressure for fluid conduits.
pub const MAX_PRESSURE: f32 = 100.0;
/// Minimum pressure for fluid conduits.
pub const MIN_PRESSURE: f32 = 0.0;

/// Per-cell state for a conduit segment.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConduitCell {
    kind: ConduitKind,
    stored: f32,
    capacity: f32,
    resistance: f32,
    temperature: f32,
    pressure: f32,
    active: bool,
}

impl ConduitCell {
    /// Empty/absent conduit cell.
    pub const EMPTY: Self = Self {
        kind: ConduitKind::Power,
        stored: 0.0,
        capacity: 0.0,
        resistance: 0.0,
        temperature: 20.0,
        pressure: 1.0,
        active: false,
    };

    /// Create a new conduit cell with default parameters.
    #[must_use]
    pub fn new(kind: ConduitKind) -> Self {
        Self {
            kind,
            stored: 0.0,
            capacity: kind.base_capacity(),
            resistance: kind.base_resistance(),
            temperature: 20.0,
            pressure: 1.0,
            active: true,
        }
    }

    /// Create a conduit cell with custom capacity and resistance.
    #[must_use]
    pub fn with_params(kind: ConduitKind, capacity: f32, resistance: f32) -> Self {
        Self {
            kind,
            stored: 0.0,
            capacity: capacity.max(0.0),
            resistance: resistance.clamp(0.0, 1.0),
            temperature: 20.0,
            pressure: 1.0,
            active: true,
        }
    }

    /// Create a conduit cell with full state.
    #[must_use]
    pub fn with_state(
        kind: ConduitKind,
        stored: f32,
        capacity: f32,
        resistance: f32,
        temperature: f32,
        pressure: f32,
    ) -> Self {
        Self {
            kind,
            stored: stored.clamp(0.0, MAX_STORED),
            capacity: capacity.max(0.0),
            resistance: resistance.clamp(0.0, 1.0),
            temperature: temperature.clamp(MIN_TEMPERATURE, MAX_TEMPERATURE),
            pressure: pressure.clamp(MIN_PRESSURE, MAX_PRESSURE),
            active: true,
        }
    }

    /// Get the conduit kind.
    #[must_use]
    pub const fn kind(&self) -> ConduitKind {
        self.kind
    }

    /// Get the currently stored amount.
    #[must_use]
    pub const fn stored(&self) -> f32 {
        self.stored
    }

    /// Get the maximum capacity.
    #[must_use]
    pub const fn capacity(&self) -> f32 {
        self.capacity
    }

    /// Get the resistance factor.
    #[must_use]
    pub const fn resistance(&self) -> f32 {
        self.resistance
    }

    /// Get the temperature (for heat conduits).
    #[must_use]
    pub const fn temperature(&self) -> f32 {
        self.temperature
    }

    /// Get the pressure (for fluid conduits).
    #[must_use]
    pub const fn pressure(&self) -> f32 {
        self.pressure
    }

    /// Check if the conduit is active.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.active
    }

    /// Check if this is an empty/absent conduit.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.active || self.capacity <= 0.0
    }

    /// Get available capacity for storage.
    #[must_use]
    pub fn available_capacity(&self) -> f32 {
        (self.capacity - self.stored).max(0.0)
    }

    /// Get fill ratio (0.0 to 1.0).
    #[must_use]
    pub fn fill_ratio(&self) -> f32 {
        if self.capacity > 0.0 {
            (self.stored / self.capacity).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// Set the stored amount, clamping to valid range.
    pub fn set_stored(&mut self, amount: f32) {
        self.stored = amount.clamp(0.0, self.capacity.min(MAX_STORED));
    }

    /// Add to stored amount, returning overflow.
    pub fn add_stored(&mut self, amount: f32) -> f32 {
        let available = self.available_capacity();
        let accepted = amount.min(available);
        self.stored += accepted;
        amount - accepted
    }

    /// Remove from stored amount, returning actual removed.
    pub fn remove_stored(&mut self, amount: f32) -> f32 {
        let removed = amount.min(self.stored);
        self.stored -= removed;
        removed
    }

    /// Set the temperature, clamping to valid range.
    pub fn set_temperature(&mut self, temp: f32) {
        self.temperature = temp.clamp(MIN_TEMPERATURE, MAX_TEMPERATURE);
    }

    /// Set the pressure, clamping to valid range.
    pub fn set_pressure(&mut self, pressure: f32) {
        self.pressure = pressure.clamp(MIN_PRESSURE, MAX_PRESSURE);
    }

    /// Set the active state.
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    /// Clamp all values to valid ranges.
    pub fn clamp(&mut self) {
        self.stored = self.stored.clamp(0.0, self.capacity.min(MAX_STORED));
        self.temperature = self.temperature.clamp(MIN_TEMPERATURE, MAX_TEMPERATURE);
        self.pressure = self.pressure.clamp(MIN_PRESSURE, MAX_PRESSURE);
        self.resistance = self.resistance.clamp(0.0, 1.0);
    }
}

impl Default for ConduitCell {
    fn default() -> Self {
        Self::EMPTY
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp, reason = "tests check exact constructor values")]
mod tests {
    use super::*;

    #[test]
    fn empty_cell() {
        let cell = ConduitCell::EMPTY;
        assert!(cell.is_empty());
        assert!(!cell.is_active());
        assert_eq!(cell.stored(), 0.0);
        assert_eq!(cell.capacity(), 0.0);
    }

    #[test]
    fn new_cell_defaults() {
        let cell = ConduitCell::new(ConduitKind::Power);
        assert!(!cell.is_empty());
        assert!(cell.is_active());
        assert_eq!(cell.kind(), ConduitKind::Power);
        assert_eq!(cell.stored(), 0.0);
        assert_eq!(cell.capacity(), ConduitKind::Power.base_capacity());
        assert_eq!(cell.resistance(), ConduitKind::Power.base_resistance());
    }

    #[test]
    fn with_params() {
        let cell = ConduitCell::with_params(ConduitKind::Fluid, 50.0, 0.1);
        assert_eq!(cell.capacity(), 50.0);
        assert_eq!(cell.resistance(), 0.1);
    }

    #[test]
    fn with_state_clamps() {
        let cell = ConduitCell::with_state(ConduitKind::Heat, 200.0, 10.0, 2.0, 5000.0, 200.0);
        assert_eq!(cell.stored(), MAX_STORED);
        assert_eq!(cell.resistance(), 1.0);
        assert_eq!(cell.temperature(), MAX_TEMPERATURE);
        assert_eq!(cell.pressure(), MAX_PRESSURE);
    }

    #[test]
    fn fill_ratio() {
        let mut cell = ConduitCell::new(ConduitKind::Power);
        assert_eq!(cell.fill_ratio(), 0.0);

        cell.set_stored(cell.capacity() / 2.0);
        assert!((cell.fill_ratio() - 0.5).abs() < 0.001);
    }

    #[test]
    fn add_stored_overflow() {
        let mut cell = ConduitCell::with_params(ConduitKind::Power, 10.0, 0.0);
        cell.set_stored(8.0);

        let overflow = cell.add_stored(5.0);
        assert!((overflow - 3.0).abs() < 0.001);
        assert!((cell.stored() - 10.0).abs() < 0.001);
    }

    #[test]
    fn remove_stored() {
        let mut cell = ConduitCell::new(ConduitKind::Power);
        cell.set_stored(10.0);

        let removed = cell.remove_stored(7.0);
        assert!((removed - 7.0).abs() < 0.001);
        assert!((cell.stored() - 3.0).abs() < 0.001);

        let removed = cell.remove_stored(10.0);
        assert!((removed - 3.0).abs() < 0.001);
        assert!((cell.stored() - 0.0).abs() < 0.001);
    }

    #[test]
    fn temperature_clamp() {
        let mut cell = ConduitCell::new(ConduitKind::Heat);
        cell.set_temperature(-500.0);
        assert_eq!(cell.temperature(), MIN_TEMPERATURE);

        cell.set_temperature(5000.0);
        assert_eq!(cell.temperature(), MAX_TEMPERATURE);
    }

    #[test]
    fn pressure_clamp() {
        let mut cell = ConduitCell::new(ConduitKind::Fluid);
        cell.set_pressure(-5.0);
        assert_eq!(cell.pressure(), MIN_PRESSURE);

        cell.set_pressure(200.0);
        assert_eq!(cell.pressure(), MAX_PRESSURE);
    }

    #[test]
    fn serde_round_trip() {
        let cell = ConduitCell::with_state(ConduitKind::Fluid, 5.0, 10.0, 0.05, 25.0, 2.0);
        let json = serde_json::to_string(&cell).unwrap();
        let recovered: ConduitCell = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, cell);
    }

    #[test]
    fn default_is_empty() {
        let cell = ConduitCell::default();
        assert!(cell.is_empty());
    }
}
