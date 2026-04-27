//! Structural event types for collapses, cave-ins, and decompression failures.

use engine_core::coords::LocalPos;
use serde::{Deserialize, Serialize};

use super::SupportKind;

/// Type of structural failure event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StructuralEventKind {
    /// Single cell collapsed due to lack of support.
    Collapse,
    /// Cell failed due to excess stress/load.
    StressFailure,
    /// Rapid decompression damaged structure.
    Decompression,
    /// Multiple connected cells collapsed together.
    CaveIn,
    /// Support chain was broken.
    SupportLost,
    /// Cell integrity dropped to zero.
    IntegrityLost,
}

impl StructuralEventKind {
    /// Get display name for the event type.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            StructuralEventKind::Collapse => "Collapse",
            StructuralEventKind::StressFailure => "Stress Failure",
            StructuralEventKind::Decompression => "Decompression",
            StructuralEventKind::CaveIn => "Cave-In",
            StructuralEventKind::SupportLost => "Support Lost",
            StructuralEventKind::IntegrityLost => "Integrity Lost",
        }
    }

    /// Whether this event can trigger cascading failures.
    #[must_use]
    pub const fn can_cascade(self) -> bool {
        matches!(
            self,
            StructuralEventKind::Collapse
                | StructuralEventKind::StressFailure
                | StructuralEventKind::CaveIn
        )
    }

    /// Severity level (1 = minor, 2 = moderate, 3 = severe).
    #[must_use]
    pub const fn severity(self) -> u8 {
        match self {
            StructuralEventKind::SupportLost => 1,
            StructuralEventKind::IntegrityLost => 1,
            StructuralEventKind::Collapse => 2,
            StructuralEventKind::StressFailure => 2,
            StructuralEventKind::Decompression => 2,
            StructuralEventKind::CaveIn => 3,
        }
    }
}

/// A structural failure event at a specific location.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StructuralEvent {
    /// Position within chunk where event occurred.
    pub pos: LocalPos,
    /// Type of structural event.
    pub kind: StructuralEventKind,
    /// Support type that failed (if applicable).
    pub support_kind: SupportKind,
    /// Stress level at failure (0.0 to 1.0).
    pub stress_at_failure: f32,
    /// Load at failure (0.0 to 1.0).
    pub load_at_failure: f32,
    /// Number of cells affected (for cave-ins).
    pub cells_affected: u32,
}

impl StructuralEvent {
    /// Create a simple collapse event.
    #[must_use]
    pub fn collapse(pos: LocalPos, support_kind: SupportKind) -> Self {
        Self {
            pos,
            kind: StructuralEventKind::Collapse,
            support_kind,
            stress_at_failure: 0.0,
            load_at_failure: 0.0,
            cells_affected: 1,
        }
    }

    /// Create a stress failure event.
    #[must_use]
    pub fn stress_failure(
        pos: LocalPos,
        support_kind: SupportKind,
        stress: f32,
        load: f32,
    ) -> Self {
        Self {
            pos,
            kind: StructuralEventKind::StressFailure,
            support_kind,
            stress_at_failure: stress,
            load_at_failure: load,
            cells_affected: 1,
        }
    }

    /// Create a decompression event.
    #[must_use]
    pub fn decompression(pos: LocalPos, support_kind: SupportKind, damage: f32) -> Self {
        Self {
            pos,
            kind: StructuralEventKind::Decompression,
            support_kind,
            stress_at_failure: damage,
            load_at_failure: 0.0,
            cells_affected: 1,
        }
    }

    /// Create a cave-in event.
    #[must_use]
    pub fn cavein(pos: LocalPos, cells_affected: u32) -> Self {
        Self {
            pos,
            kind: StructuralEventKind::CaveIn,
            support_kind: SupportKind::None,
            stress_at_failure: 0.0,
            load_at_failure: 0.0,
            cells_affected,
        }
    }

    /// Create a support lost event.
    #[must_use]
    pub fn support_lost(pos: LocalPos, support_kind: SupportKind) -> Self {
        Self {
            pos,
            kind: StructuralEventKind::SupportLost,
            support_kind,
            stress_at_failure: 0.0,
            load_at_failure: 0.0,
            cells_affected: 1,
        }
    }

    /// Create an integrity lost event.
    #[must_use]
    pub fn integrity_lost(pos: LocalPos, support_kind: SupportKind) -> Self {
        Self {
            pos,
            kind: StructuralEventKind::IntegrityLost,
            support_kind,
            stress_at_failure: 0.0,
            load_at_failure: 0.0,
            cells_affected: 1,
        }
    }
}

/// Boundary information for cross-chunk structural coordination.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StructuralBoundary {
    /// Position on chunk boundary.
    pub pos: LocalPos,
    /// Direction to neighboring chunk (-1, 0, 1 for each axis).
    pub direction: (i32, i32, i32),
    /// Support kind at this boundary position.
    pub support_kind: SupportKind,
    /// Whether this cell is supported.
    pub is_supported: bool,
    /// Support distance at boundary.
    pub support_distance: u8,
    /// Load being transferred across boundary.
    pub load_transfer: f32,
}

impl StructuralBoundary {
    /// Create a new boundary report.
    #[must_use]
    pub fn new(
        pos: LocalPos,
        direction: (i32, i32, i32),
        support_kind: SupportKind,
        is_supported: bool,
        support_distance: u8,
        load_transfer: f32,
    ) -> Self {
        Self {
            pos,
            direction,
            support_kind,
            is_supported,
            support_distance,
            load_transfer,
        }
    }

    /// Check if this boundary requires support from neighbor chunk.
    #[must_use]
    pub fn needs_external_support(&self) -> bool {
        !self.is_supported && self.support_kind.provides_support()
    }

    /// Check if this boundary provides support to neighbor chunk.
    #[must_use]
    pub fn provides_external_support(&self) -> bool {
        self.is_supported && self.support_kind.provides_support() && self.support_distance < 255
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_kind_names() {
        assert_eq!(StructuralEventKind::Collapse.name(), "Collapse");
        assert_eq!(StructuralEventKind::CaveIn.name(), "Cave-In");
    }

    #[test]
    fn event_kind_cascade() {
        assert!(StructuralEventKind::Collapse.can_cascade());
        assert!(StructuralEventKind::CaveIn.can_cascade());
        assert!(!StructuralEventKind::SupportLost.can_cascade());
    }

    #[test]
    fn event_kind_severity() {
        assert_eq!(StructuralEventKind::SupportLost.severity(), 1);
        assert_eq!(StructuralEventKind::Collapse.severity(), 2);
        assert_eq!(StructuralEventKind::CaveIn.severity(), 3);
    }

    #[test]
    fn collapse_event() {
        let event = StructuralEvent::collapse(LocalPos::new(5, 5, 5), SupportKind::Column);
        assert_eq!(event.kind, StructuralEventKind::Collapse);
        assert_eq!(event.support_kind, SupportKind::Column);
        assert_eq!(event.cells_affected, 1);
    }

    #[test]
    fn stress_failure_event() {
        let event =
            StructuralEvent::stress_failure(LocalPos::new(3, 3, 3), SupportKind::Beam, 0.95, 0.8);
        assert_eq!(event.kind, StructuralEventKind::StressFailure);
        assert!((event.stress_at_failure - 0.95).abs() < 0.001);
        assert!((event.load_at_failure - 0.8).abs() < 0.001);
    }

    #[test]
    fn cavein_event() {
        let event = StructuralEvent::cavein(LocalPos::new(0, 0, 0), 15);
        assert_eq!(event.kind, StructuralEventKind::CaveIn);
        assert_eq!(event.cells_affected, 15);
    }

    #[test]
    fn decompression_event() {
        let event = StructuralEvent::decompression(LocalPos::new(8, 8, 8), SupportKind::Solid, 0.6);
        assert_eq!(event.kind, StructuralEventKind::Decompression);
    }

    #[test]
    fn boundary_needs_support() {
        let needs = StructuralBoundary::new(
            LocalPos::new(0, 5, 5),
            (-1, 0, 0),
            SupportKind::Column,
            false,
            255,
            0.0,
        );
        assert!(needs.needs_external_support());

        let has = StructuralBoundary::new(
            LocalPos::new(0, 5, 5),
            (-1, 0, 0),
            SupportKind::Column,
            true,
            5,
            0.1,
        );
        assert!(!has.needs_external_support());
    }

    #[test]
    fn boundary_provides_support() {
        let provides = StructuralBoundary::new(
            LocalPos::new(15, 5, 5),
            (1, 0, 0),
            SupportKind::Foundation,
            true,
            0,
            0.0,
        );
        assert!(provides.provides_external_support());

        let unsupported = StructuralBoundary::new(
            LocalPos::new(15, 5, 5),
            (1, 0, 0),
            SupportKind::Column,
            false,
            255,
            0.0,
        );
        assert!(!unsupported.provides_external_support());
    }

    #[test]
    fn serde_event_kind() {
        let kind = StructuralEventKind::StressFailure;
        let json = serde_json::to_string(&kind).unwrap();
        let recovered: StructuralEventKind = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, kind);
    }

    #[test]
    fn serde_event() {
        let event =
            StructuralEvent::stress_failure(LocalPos::new(1, 2, 3), SupportKind::Brace, 0.9, 0.7);
        let json = serde_json::to_string(&event).unwrap();
        let recovered: StructuralEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, event);
    }

    #[test]
    fn serde_boundary() {
        let boundary = StructuralBoundary::new(
            LocalPos::new(0, 8, 8),
            (-1, 0, 0),
            SupportKind::Beam,
            true,
            3,
            0.15,
        );
        let json = serde_json::to_string(&boundary).unwrap();
        let recovered: StructuralBoundary = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, boundary);
    }
}
