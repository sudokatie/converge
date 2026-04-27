//! Per-cell structural state with load, stress, and support chain tracking.

use serde::{Deserialize, Serialize};

use super::SupportKind;

/// Maximum load a cell can carry (normalized).
pub const MAX_LOAD: f32 = 1.0;

/// Maximum stress before failure.
pub const MAX_STRESS: f32 = 1.0;

/// Stress threshold above which cell is considered overstressed.
pub const OVERSTRESS_THRESHOLD: f32 = 0.8;

/// Stress threshold at which structural failure occurs.
pub const FAILURE_THRESHOLD: f32 = 1.0;

/// Structural state for a single cell.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StructuralCell {
    support_kind: SupportKind,
    load: f32,
    stress: f32,
    support_distance: u8,
    integrity: f32,
    is_supported: bool,
}

impl StructuralCell {
    /// Empty/air cell with no structural properties.
    pub const EMPTY: Self = Self {
        support_kind: SupportKind::None,
        load: 0.0,
        stress: 0.0,
        support_distance: 255,
        integrity: 0.0,
        is_supported: false,
    };

    /// Create a new structural cell.
    #[must_use]
    pub fn new(support_kind: SupportKind) -> Self {
        Self {
            support_kind,
            load: 0.0,
            stress: 0.0,
            support_distance: if support_kind.is_foundation() { 0 } else { 255 },
            integrity: 1.0,
            is_supported: support_kind.is_foundation(),
        }
    }

    /// Create a foundation cell (always supported).
    #[must_use]
    pub fn foundation() -> Self {
        Self {
            support_kind: SupportKind::Foundation,
            load: 0.0,
            stress: 0.0,
            support_distance: 0,
            integrity: 1.0,
            is_supported: true,
        }
    }

    /// Create a cell with all properties specified.
    #[must_use]
    pub fn with_state(
        support_kind: SupportKind,
        load: f32,
        stress: f32,
        support_distance: u8,
        integrity: f32,
        is_supported: bool,
    ) -> Self {
        Self {
            support_kind,
            load: load.clamp(0.0, MAX_LOAD),
            stress: stress.clamp(0.0, MAX_STRESS),
            support_distance,
            integrity: integrity.clamp(0.0, 1.0),
            is_supported,
        }
    }

    /// Get the support kind.
    #[must_use]
    pub const fn support_kind(&self) -> SupportKind {
        self.support_kind
    }

    /// Get the current load (0.0 to 1.0).
    #[must_use]
    pub const fn load(&self) -> f32 {
        self.load
    }

    /// Get the current stress level (0.0 to 1.0).
    #[must_use]
    pub const fn stress(&self) -> f32 {
        self.stress
    }

    /// Get the distance to nearest foundation support.
    #[must_use]
    pub const fn support_distance(&self) -> u8 {
        self.support_distance
    }

    /// Get the structural integrity (0.0 = destroyed, 1.0 = perfect).
    #[must_use]
    pub const fn integrity(&self) -> f32 {
        self.integrity
    }

    /// Whether this cell is connected to a foundation.
    #[must_use]
    pub const fn is_supported(&self) -> bool {
        self.is_supported
    }

    /// Check if this cell provides structural support.
    #[must_use]
    pub fn provides_support(&self) -> bool {
        self.support_kind.provides_support() && self.integrity > 0.0 && self.is_supported
    }

    /// Check if this cell is overstressed (stress > threshold).
    #[must_use]
    pub fn is_overstressed(&self) -> bool {
        self.stress > OVERSTRESS_THRESHOLD
    }

    /// Check if this cell has failed structurally.
    #[must_use]
    pub fn has_failed(&self) -> bool {
        self.stress >= FAILURE_THRESHOLD || self.integrity <= 0.0
    }

    /// Check if this cell is unsupported and can collapse.
    #[must_use]
    pub fn can_collapse(&self) -> bool {
        !self.is_supported && self.support_kind.provides_support() && self.support_distance == 255
    }

    /// Set the support kind.
    pub fn set_support_kind(&mut self, kind: SupportKind) {
        self.support_kind = kind;
        if kind.is_foundation() {
            self.support_distance = 0;
            self.is_supported = true;
        }
    }

    /// Set the load (clamped to valid range).
    pub fn set_load(&mut self, load: f32) {
        self.load = load.clamp(0.0, MAX_LOAD);
    }

    /// Add load and update stress based on capacity.
    pub fn add_load(&mut self, delta: f32) {
        self.load = (self.load + delta).clamp(0.0, MAX_LOAD);
        self.update_stress();
    }

    /// Set the stress level (clamped to valid range).
    pub fn set_stress(&mut self, stress: f32) {
        self.stress = stress.clamp(0.0, MAX_STRESS);
    }

    /// Set the support distance.
    pub fn set_support_distance(&mut self, distance: u8) {
        self.support_distance = distance;
    }

    /// Set the structural integrity (clamped to valid range).
    pub fn set_integrity(&mut self, integrity: f32) {
        self.integrity = integrity.clamp(0.0, 1.0);
    }

    /// Mark as supported with given distance from foundation.
    pub fn mark_supported(&mut self, distance: u8) {
        self.is_supported = true;
        self.support_distance = distance;
    }

    /// Mark as unsupported.
    pub fn mark_unsupported(&mut self) {
        self.is_supported = false;
        self.support_distance = 255;
    }

    /// Apply damage to integrity.
    pub fn apply_damage(&mut self, damage: f32) {
        self.integrity = (self.integrity - damage).max(0.0);
    }

    /// Calculate effective support capacity accounting for load and integrity.
    #[must_use]
    pub fn effective_capacity(&self) -> f32 {
        let base = self.support_kind.max_load_factor();
        let remaining = (1.0 - self.load).max(0.0);
        base * remaining * self.integrity
    }

    /// Update stress based on current load vs capacity.
    fn update_stress(&mut self) {
        let capacity = self.support_kind.max_load_factor() * self.integrity;
        if capacity > 0.0 {
            self.stress = (self.load / capacity).clamp(0.0, MAX_STRESS);
        } else {
            self.stress = if self.load > 0.0 { MAX_STRESS } else { 0.0 };
        }
    }

    /// Clamp all values to valid ranges.
    pub fn clamp(&mut self) {
        self.load = self.load.clamp(0.0, MAX_LOAD);
        self.stress = self.stress.clamp(0.0, MAX_STRESS);
        self.integrity = self.integrity.clamp(0.0, 1.0);
    }
}

impl Default for StructuralCell {
    fn default() -> Self {
        Self::EMPTY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_cell() {
        let cell = StructuralCell::EMPTY;
        assert_eq!(cell.support_kind(), SupportKind::None);
        assert_eq!(cell.load(), 0.0);
        assert_eq!(cell.stress(), 0.0);
        assert_eq!(cell.support_distance(), 255);
        assert!(!cell.is_supported());
        assert!(!cell.provides_support());
    }

    #[test]
    fn new_cell_defaults() {
        let cell = StructuralCell::new(SupportKind::Column);
        assert_eq!(cell.support_kind(), SupportKind::Column);
        assert_eq!(cell.load(), 0.0);
        assert_eq!(cell.stress(), 0.0);
        assert_eq!(cell.integrity(), 1.0);
        assert!(!cell.is_supported());
    }

    #[test]
    fn foundation_cell() {
        let cell = StructuralCell::foundation();
        assert_eq!(cell.support_kind(), SupportKind::Foundation);
        assert_eq!(cell.support_distance(), 0);
        assert!(cell.is_supported());
        assert!(cell.provides_support());
    }

    #[test]
    fn with_state_clamping() {
        let cell = StructuralCell::with_state(
            SupportKind::Solid,
            2.0,  // over max
            -0.5, // under min
            10,
            1.5, // over max
            true,
        );
        assert_eq!(cell.load(), MAX_LOAD);
        assert_eq!(cell.stress(), 0.0);
        assert_eq!(cell.integrity(), 1.0);
    }

    #[test]
    fn add_load_updates_stress() {
        let mut cell = StructuralCell::new(SupportKind::Column);
        cell.mark_supported(1);
        cell.add_load(0.45);
        assert!((cell.load() - 0.45).abs() < 0.001);
        assert!(cell.stress() > 0.0);
    }

    #[test]
    fn overstress_detection() {
        let mut cell = StructuralCell::new(SupportKind::Weak);
        cell.mark_supported(1);
        cell.add_load(0.3);
        assert!(cell.is_overstressed());
    }

    #[test]
    fn failure_detection() {
        let mut cell = StructuralCell::new(SupportKind::Weak);
        cell.mark_supported(1);
        cell.add_load(0.5);
        assert!(cell.has_failed());
    }

    #[test]
    fn integrity_failure() {
        let mut cell = StructuralCell::new(SupportKind::Column);
        cell.set_integrity(0.0);
        assert!(cell.has_failed());
    }

    #[test]
    fn can_collapse() {
        let mut cell = StructuralCell::new(SupportKind::Solid);
        cell.mark_unsupported();
        assert!(cell.can_collapse());

        cell.mark_supported(5);
        assert!(!cell.can_collapse());
    }

    #[test]
    fn effective_capacity() {
        let mut cell = StructuralCell::new(SupportKind::Column);
        cell.mark_supported(1);
        let full_capacity = cell.effective_capacity();
        assert!((full_capacity - 0.9).abs() < 0.001);

        cell.add_load(0.5);
        let reduced = cell.effective_capacity();
        assert!(reduced < full_capacity);
    }

    #[test]
    fn apply_damage() {
        let mut cell = StructuralCell::new(SupportKind::Solid);
        cell.apply_damage(0.3);
        assert!((cell.integrity() - 0.7).abs() < 0.001);

        cell.apply_damage(1.0);
        assert_eq!(cell.integrity(), 0.0);
    }

    #[test]
    fn mark_supported() {
        let mut cell = StructuralCell::new(SupportKind::Column);
        assert!(!cell.is_supported());

        cell.mark_supported(3);
        assert!(cell.is_supported());
        assert_eq!(cell.support_distance(), 3);
    }

    #[test]
    fn mark_unsupported() {
        let mut cell = StructuralCell::foundation();
        cell.set_support_kind(SupportKind::Column);
        cell.mark_unsupported();
        assert!(!cell.is_supported());
        assert_eq!(cell.support_distance(), 255);
    }

    #[test]
    fn default_is_empty() {
        let cell = StructuralCell::default();
        assert_eq!(cell, StructuralCell::EMPTY);
    }

    #[test]
    fn serde_round_trip() {
        let cell = StructuralCell::with_state(SupportKind::Beam, 0.4, 0.3, 5, 0.9, true);
        let json = serde_json::to_string(&cell).unwrap();
        let recovered: StructuralCell = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, cell);
    }

    #[test]
    fn provides_support_requirements() {
        let mut cell = StructuralCell::new(SupportKind::Column);

        assert!(!cell.provides_support());

        cell.mark_supported(1);
        assert!(cell.provides_support());

        cell.set_integrity(0.0);
        assert!(!cell.provides_support());
    }
}
