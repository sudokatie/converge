//! Event severity levels for journal entries.

use serde::{Deserialize, Serialize};

/// Severity level for journal events.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[repr(u8)]
pub enum Severity {
    /// Diagnostic/trace level events for detailed debugging.
    Trace = 0,

    /// Informational events for normal operation logging.
    #[default]
    Info = 1,

    /// Warning events that may indicate problems.
    Warning = 2,

    /// Error events that indicate failures.
    Error = 3,

    /// Critical events requiring immediate attention.
    Critical = 4,
}

impl Severity {
    /// Total number of severity levels.
    pub const COUNT: usize = 5;

    /// All severity levels in order (lowest to highest).
    pub const ALL: [Severity; Self::COUNT] = [
        Severity::Trace,
        Severity::Info,
        Severity::Warning,
        Severity::Error,
        Severity::Critical,
    ];

    /// Convert to array index.
    #[must_use]
    pub const fn as_index(self) -> usize {
        self as usize
    }

    /// Create from array index.
    #[must_use]
    pub const fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Severity::Trace),
            1 => Some(Severity::Info),
            2 => Some(Severity::Warning),
            3 => Some(Severity::Error),
            4 => Some(Severity::Critical),
            _ => None,
        }
    }

    /// Get the display name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Severity::Trace => "Trace",
            Severity::Info => "Info",
            Severity::Warning => "Warning",
            Severity::Error => "Error",
            Severity::Critical => "Critical",
        }
    }

    /// Check if this severity is at least as severe as another.
    #[must_use]
    pub const fn is_at_least(self, other: Severity) -> bool {
        self as u8 >= other as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_matches_all() {
        assert_eq!(Severity::ALL.len(), Severity::COUNT);
    }

    #[test]
    fn index_round_trip() {
        for sev in Severity::ALL {
            let index = sev.as_index();
            let recovered = Severity::from_index(index);
            assert_eq!(recovered, Some(sev));
        }
    }

    #[test]
    fn from_index_out_of_range() {
        assert_eq!(Severity::from_index(5), None);
        assert_eq!(Severity::from_index(255), None);
    }

    #[test]
    fn ordering() {
        assert!(Severity::Trace < Severity::Info);
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
        assert!(Severity::Error < Severity::Critical);
    }

    #[test]
    fn is_at_least() {
        assert!(Severity::Critical.is_at_least(Severity::Trace));
        assert!(Severity::Warning.is_at_least(Severity::Warning));
        assert!(!Severity::Info.is_at_least(Severity::Error));
    }

    #[test]
    fn names_not_empty() {
        for sev in Severity::ALL {
            assert!(!sev.name().is_empty());
        }
    }

    #[test]
    fn serde_round_trip() {
        for sev in Severity::ALL {
            let json = serde_json::to_string(&sev).unwrap();
            let recovered: Severity = serde_json::from_str(&json).unwrap();
            assert_eq!(recovered, sev);
        }
    }
}
