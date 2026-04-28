//! Region recovery summary types.

use engine_core::coords::ChunkPos;
use serde::{Deserialize, Serialize};

use super::{EventCategory, EventKind, Severity};

/// Summary statistics for a category of events.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryStats {
    /// Total event count.
    pub count: u64,
    /// Count by severity level.
    pub by_severity: [u64; Severity::COUNT],
}

impl CategoryStats {
    /// Create empty stats.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            count: 0,
            by_severity: [0; Severity::COUNT],
        }
    }

    /// Add an event with the given severity.
    pub fn add(&mut self, severity: Severity) {
        self.count += 1;
        self.by_severity[severity.as_index()] += 1;
    }

    /// Get count at or above a severity threshold.
    #[must_use]
    pub fn count_at_least(&self, severity: Severity) -> u64 {
        self.by_severity[severity.as_index()..]
            .iter()
            .copied()
            .sum()
    }

    /// Check if there are any events at or above a severity threshold.
    #[must_use]
    pub fn has_at_least(&self, severity: Severity) -> bool {
        self.count_at_least(severity) > 0
    }
}

/// Summary of events for a single region/chunk.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RegionSummary {
    /// Chunk position.
    pub chunk_pos: ChunkPos,
    /// Tick range covered (start, end inclusive).
    pub tick_range: (u64, u64),
    /// Total event count.
    pub total_events: u64,
    /// Stats by category.
    pub by_category: [CategoryStats; EventCategory::COUNT],
    /// Most recent event kinds (up to 8).
    pub recent_kinds: Vec<EventKind>,
    /// Most frequent tags.
    pub frequent_tags: Vec<(String, u64)>,
    /// Highest severity seen.
    pub max_severity: Severity,
}

impl RegionSummary {
    /// Create an empty summary for a chunk.
    #[must_use]
    pub fn empty(chunk_pos: ChunkPos) -> Self {
        Self {
            chunk_pos,
            tick_range: (u64::MAX, 0),
            total_events: 0,
            by_category: std::array::from_fn(|_| CategoryStats::empty()),
            recent_kinds: Vec::new(),
            frequent_tags: Vec::new(),
            max_severity: Severity::Trace,
        }
    }

    /// Check if summary is empty (no events).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.total_events == 0
    }

    /// Get stats for a specific category.
    #[must_use]
    pub fn category_stats(&self, category: EventCategory) -> &CategoryStats {
        &self.by_category[category.as_index()]
    }

    /// Check if region has any errors or critical events.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.max_severity.is_at_least(Severity::Error)
    }

    /// Check if region has any warnings or higher.
    #[must_use]
    pub fn has_warnings(&self) -> bool {
        self.max_severity.is_at_least(Severity::Warning)
    }
}

/// Recovery summary for simulation state reconstruction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecoverySummary {
    /// Tick range for recovery.
    pub tick_range: (u64, u64),
    /// Total events in range.
    pub total_events: u64,
    /// Number of regions affected.
    pub regions_affected: u64,
    /// Regions with errors.
    pub error_regions: Vec<ChunkPos>,
    /// Last checkpoint tick (if any).
    pub last_checkpoint: Option<u64>,
    /// Rollback events in range.
    pub rollback_count: u64,
    /// Sync mismatches detected.
    pub mismatch_count: u64,
    /// Per-region summaries (sorted by chunk position).
    pub region_summaries: Vec<RegionSummary>,
}

impl RecoverySummary {
    /// Create an empty recovery summary.
    #[must_use]
    pub fn empty(tick_range: (u64, u64)) -> Self {
        Self {
            tick_range,
            total_events: 0,
            regions_affected: 0,
            error_regions: Vec::new(),
            last_checkpoint: None,
            rollback_count: 0,
            mismatch_count: 0,
            region_summaries: Vec::new(),
        }
    }

    /// Check if recovery needed (errors or mismatches).
    #[must_use]
    pub fn needs_recovery(&self) -> bool {
        !self.error_regions.is_empty() || self.mismatch_count > 0
    }

    /// Get summary for a specific region.
    #[must_use]
    pub fn region_summary(&self, chunk_pos: ChunkPos) -> Option<&RegionSummary> {
        self.region_summaries
            .iter()
            .find(|s| s.chunk_pos == chunk_pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_stats_add() {
        let mut stats = CategoryStats::empty();
        stats.add(Severity::Info);
        stats.add(Severity::Warning);
        stats.add(Severity::Warning);

        assert_eq!(stats.count, 3);
        assert_eq!(stats.by_severity[Severity::Info.as_index()], 1);
        assert_eq!(stats.by_severity[Severity::Warning.as_index()], 2);
    }

    #[test]
    fn category_stats_count_at_least() {
        let mut stats = CategoryStats::empty();
        stats.add(Severity::Trace);
        stats.add(Severity::Info);
        stats.add(Severity::Warning);
        stats.add(Severity::Error);

        assert_eq!(stats.count_at_least(Severity::Warning), 2);
        assert_eq!(stats.count_at_least(Severity::Error), 1);
        assert_eq!(stats.count_at_least(Severity::Trace), 4);
    }

    #[test]
    fn region_summary_empty() {
        let summary = RegionSummary::empty(ChunkPos::new(1, 2, 3));
        assert!(summary.is_empty());
        assert_eq!(summary.chunk_pos, ChunkPos::new(1, 2, 3));
        assert!(!summary.has_errors());
        assert!(!summary.has_warnings());
    }

    #[test]
    fn recovery_summary_empty() {
        let summary = RecoverySummary::empty((0, 100));
        assert_eq!(summary.tick_range, (0, 100));
        assert!(!summary.needs_recovery());
    }

    #[test]
    fn recovery_summary_needs_recovery() {
        let mut summary = RecoverySummary::empty((0, 100));
        summary.error_regions.push(ChunkPos::new(0, 0, 0));

        assert!(summary.needs_recovery());
    }

    #[test]
    fn serde_round_trip_category_stats() {
        let mut stats = CategoryStats::empty();
        stats.add(Severity::Warning);
        stats.add(Severity::Error);

        let json = serde_json::to_string(&stats).unwrap();
        let recovered: CategoryStats = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, stats);
    }

    #[test]
    fn serde_round_trip_region_summary() {
        let summary = RegionSummary::empty(ChunkPos::new(5, 10, 15));
        let json = serde_json::to_string(&summary).unwrap();
        let recovered: RegionSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, summary);
    }

    #[test]
    fn serde_round_trip_recovery_summary() {
        let mut summary = RecoverySummary::empty((0, 1000));
        summary.total_events = 42;
        summary.last_checkpoint = Some(500);

        let json = serde_json::to_string(&summary).unwrap();
        let recovered: RecoverySummary = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, summary);
    }
}
