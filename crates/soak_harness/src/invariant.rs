//! Invariant checking for soak tests.

use engine_core::coords::ChunkPos;
use engine_world::{HazardKind, SandboxState, StepChecksum};
use serde::{Deserialize, Serialize};

/// Kind of invariant being checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvariantKind {
    /// Hazard intensity must be in valid range [0, 1].
    HazardIntensityBounds,
    /// Active hazard count should not explode beyond threshold.
    HazardCountBounds,
    /// Checksum must be non-zero for active simulation.
    NonZeroChecksum,
    /// Determinism: repeated runs with same seed produce same checksum.
    Determinism,
    /// Chunk count should not exceed configured maximum.
    ChunkCountBounds,
    /// Tick counter must advance monotonically.
    TickMonotonic,
    /// No NaN or infinity values in simulation state.
    FiniteValues,
    /// Custom invariant with user-defined check.
    Custom,
}

impl std::fmt::Display for InvariantKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HazardIntensityBounds => write!(f, "HazardIntensityBounds"),
            Self::HazardCountBounds => write!(f, "HazardCountBounds"),
            Self::NonZeroChecksum => write!(f, "NonZeroChecksum"),
            Self::Determinism => write!(f, "Determinism"),
            Self::ChunkCountBounds => write!(f, "ChunkCountBounds"),
            Self::TickMonotonic => write!(f, "TickMonotonic"),
            Self::FiniteValues => write!(f, "FiniteValues"),
            Self::Custom => write!(f, "Custom"),
        }
    }
}

/// An invariant violation detected during soak testing.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InvariantViolation {
    /// Kind of invariant violated.
    pub kind: InvariantKind,
    /// Tick when violation occurred.
    pub tick: u64,
    /// Human-readable description.
    pub message: String,
    /// Optional chunk position if violation is localized.
    pub chunk_pos: Option<ChunkPos>,
    /// Severity (0=warning, 1=error, 2=critical).
    pub severity: u8,
}

impl InvariantViolation {
    /// Create a new violation.
    #[must_use]
    pub fn new(kind: InvariantKind, tick: u64, message: impl Into<String>) -> Self {
        Self {
            kind,
            tick,
            message: message.into(),
            chunk_pos: None,
            severity: 1,
        }
    }

    /// Create a violation localized to a chunk.
    #[must_use]
    pub fn at_chunk(
        kind: InvariantKind,
        tick: u64,
        chunk_pos: ChunkPos,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            tick,
            message: message.into(),
            chunk_pos: Some(chunk_pos),
            severity: 1,
        }
    }

    /// Set severity level.
    #[must_use]
    pub fn with_severity(mut self, severity: u8) -> Self {
        self.severity = severity;
        self
    }

    /// Whether this is a critical violation.
    #[must_use]
    pub fn is_critical(&self) -> bool {
        self.severity >= 2
    }
}

impl std::fmt::Display for InvariantViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[tick {}] {}: {}", self.tick, self.kind, self.message)?;
        if let Some(pos) = self.chunk_pos {
            write!(f, " at chunk ({}, {}, {})", pos.x(), pos.y(), pos.z())?;
        }
        Ok(())
    }
}

/// Invariant checker for soak tests.
pub struct Invariant {
    last_tick: u64,
    max_hazard_count: u32,
    max_chunk_count: usize,
}

impl Default for Invariant {
    fn default() -> Self {
        Self::new()
    }
}

impl Invariant {
    /// Create a new invariant checker with default thresholds.
    #[must_use]
    pub fn new() -> Self {
        Self {
            last_tick: 0,
            max_hazard_count: 1_000_000,
            max_chunk_count: 10_000,
        }
    }

    /// Create an invariant checker with custom thresholds.
    #[must_use]
    pub fn with_thresholds(max_hazard_count: u32, max_chunk_count: usize) -> Self {
        Self {
            last_tick: 0,
            max_hazard_count,
            max_chunk_count,
        }
    }

    /// Check all invariants against current state.
    pub fn check(
        &mut self,
        state: &SandboxState,
        checksum: StepChecksum,
    ) -> Vec<InvariantViolation> {
        let mut violations = Vec::new();

        self.check_tick_monotonic(state.tick, &mut violations);
        self.check_hazard_count_bounds(state, &mut violations);
        self.check_chunk_count_bounds(state, &mut violations);
        self.check_checksum(state.tick, checksum, state, &mut violations);

        violations
    }

    fn check_tick_monotonic(&mut self, tick: u64, violations: &mut Vec<InvariantViolation>) {
        if tick > 0 && tick <= self.last_tick {
            violations.push(InvariantViolation::new(
                InvariantKind::TickMonotonic,
                tick,
                format!(
                    "tick did not advance: {} <= previous {}",
                    tick, self.last_tick
                ),
            ));
        }
        self.last_tick = tick;
    }

    fn check_hazard_count_bounds(
        &self,
        state: &SandboxState,
        violations: &mut Vec<InvariantViolation>,
    ) {
        if state.total_active_hazards > self.max_hazard_count {
            violations.push(
                InvariantViolation::new(
                    InvariantKind::HazardCountBounds,
                    state.tick,
                    format!(
                        "hazard count {} exceeds maximum {}",
                        state.total_active_hazards, self.max_hazard_count
                    ),
                )
                .with_severity(2),
            );
        }
    }

    fn check_chunk_count_bounds(
        &self,
        state: &SandboxState,
        violations: &mut Vec<InvariantViolation>,
    ) {
        if state.chunk_count > self.max_chunk_count {
            violations.push(
                InvariantViolation::new(
                    InvariantKind::ChunkCountBounds,
                    state.tick,
                    format!(
                        "chunk count {} exceeds maximum {}",
                        state.chunk_count, self.max_chunk_count
                    ),
                )
                .with_severity(2),
            );
        }
    }

    #[allow(clippy::unused_self)]
    fn check_checksum(
        &self,
        tick: u64,
        checksum: StepChecksum,
        state: &SandboxState,
        violations: &mut Vec<InvariantViolation>,
    ) {
        if tick > 0 && state.total_active_hazards > 0 && u32::from(checksum) == 0 {
            violations.push(InvariantViolation::new(
                InvariantKind::NonZeroChecksum,
                tick,
                "checksum is zero for active simulation",
            ));
        }
    }

    /// Check hazard intensity bounds for a specific value.
    pub fn check_hazard_intensity(
        &self,
        tick: u64,
        kind: HazardKind,
        intensity: f32,
        chunk_pos: ChunkPos,
    ) -> Option<InvariantViolation> {
        if !intensity.is_finite() {
            return Some(InvariantViolation::at_chunk(
                InvariantKind::FiniteValues,
                tick,
                chunk_pos,
                format!("hazard {kind:?} has non-finite intensity: {intensity}"),
            ));
        }
        if !(0.0..=1.0).contains(&intensity) {
            return Some(InvariantViolation::at_chunk(
                InvariantKind::HazardIntensityBounds,
                tick,
                chunk_pos,
                format!("hazard {kind:?} intensity {intensity} out of bounds [0, 1]"),
            ));
        }
        None
    }

    /// Check determinism by comparing checksums from two runs.
    pub fn check_determinism(
        tick: u64,
        checksum_a: StepChecksum,
        checksum_b: StepChecksum,
    ) -> Option<InvariantViolation> {
        if checksum_a == checksum_b {
            None
        } else {
            Some(InvariantViolation::new(
                InvariantKind::Determinism,
                tick,
                format!(
                    "checksum mismatch: {:08x} vs {:08x}",
                    u32::from(checksum_a),
                    u32::from(checksum_b)
                ),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invariant_violation_display() {
        let v = InvariantViolation::new(InvariantKind::HazardCountBounds, 100, "too many hazards");
        let s = v.to_string();
        assert!(s.contains("tick 100"));
        assert!(s.contains("HazardCountBounds"));
        assert!(s.contains("too many hazards"));
    }

    #[test]
    fn invariant_violation_at_chunk() {
        let v = InvariantViolation::at_chunk(
            InvariantKind::HazardIntensityBounds,
            50,
            ChunkPos::new(1, 2, 3),
            "intensity out of range",
        );
        let s = v.to_string();
        assert!(s.contains("chunk (1, 2, 3)"));
    }

    #[test]
    fn tick_monotonic_check() {
        let mut inv = Invariant::new();
        let state1 = SandboxState {
            tick: 1,
            ..Default::default()
        };
        let state2 = SandboxState {
            tick: 1,
            ..Default::default()
        };

        let v1 = inv.check(&state1, StepChecksum::from_raw(123));
        assert!(v1.is_empty());

        let v2 = inv.check(&state2, StepChecksum::from_raw(456));
        assert_eq!(v2.len(), 1);
        assert_eq!(v2[0].kind, InvariantKind::TickMonotonic);
    }

    #[test]
    fn hazard_count_bounds_check() {
        let mut inv = Invariant::with_thresholds(100, 1000);
        let state = SandboxState {
            tick: 1,
            total_active_hazards: 200,
            ..Default::default()
        };
        let violations = inv.check(&state, StepChecksum::from_raw(123));
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].kind, InvariantKind::HazardCountBounds);
        assert!(violations[0].is_critical());
    }

    #[test]
    fn chunk_count_bounds_check() {
        let mut inv = Invariant::with_thresholds(1_000_000, 10);
        let state = SandboxState {
            tick: 1,
            chunk_count: 20,
            ..Default::default()
        };
        let violations = inv.check(&state, StepChecksum::from_raw(123));
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].kind, InvariantKind::ChunkCountBounds);
    }

    #[test]
    fn hazard_intensity_check() {
        let inv = Invariant::new();

        assert!(
            inv.check_hazard_intensity(0, HazardKind::Fire, 0.5, ChunkPos::new(0, 0, 0))
                .is_none()
        );

        let v = inv.check_hazard_intensity(0, HazardKind::Fire, 1.5, ChunkPos::new(0, 0, 0));
        assert!(v.is_some());
        assert_eq!(v.unwrap().kind, InvariantKind::HazardIntensityBounds);

        let v = inv.check_hazard_intensity(0, HazardKind::Fire, f32::NAN, ChunkPos::new(0, 0, 0));
        assert!(v.is_some());
        assert_eq!(v.unwrap().kind, InvariantKind::FiniteValues);
    }

    #[test]
    fn determinism_check() {
        let a = StepChecksum::from_raw(12345);
        let b = StepChecksum::from_raw(12345);
        let c = StepChecksum::from_raw(99999);

        assert!(Invariant::check_determinism(0, a, b).is_none());

        let v = Invariant::check_determinism(0, a, c);
        assert!(v.is_some());
        assert_eq!(v.unwrap().kind, InvariantKind::Determinism);
    }
}
