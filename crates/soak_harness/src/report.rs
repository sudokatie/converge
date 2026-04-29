//! Report generation for soak test results.

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::invariant::InvariantViolation;
use engine_world::StepChecksum;

/// Output format for reports.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable text.
    #[default]
    Text,
    /// JSON format for machine processing.
    Json,
}

impl OutputFormat {
    /// Parse from string (case-insensitive).
    #[must_use]
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "text" | "txt" => Some(Self::Text),
            "json" => Some(Self::Json),
            _ => None,
        }
    }
}

/// Summary for a single tick.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TickSummary {
    /// Tick number.
    pub tick: u64,
    /// Checksum after this tick.
    pub checksum: u32,
    /// Number of active hazards.
    pub active_hazards: u32,
    /// Number of loaded chunks.
    pub chunk_count: usize,
    /// Whether any changes occurred.
    pub had_changes: bool,
}

/// Checkpoint report generated at intervals during soak run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckpointReport {
    /// Tick when checkpoint was taken.
    pub tick: u64,
    /// Elapsed wall-clock time.
    pub elapsed_secs: f64,
    /// Ticks per second throughput.
    pub ticks_per_sec: f64,
    /// Current checksum.
    pub checksum: u32,
    /// Number of active hazards.
    pub active_hazards: u32,
    /// Number of loaded chunks.
    pub chunk_count: usize,
    /// Violations since last checkpoint.
    pub violations_in_interval: usize,
    /// Total violations so far.
    pub total_violations: usize,
}

impl CheckpointReport {
    /// Format as human-readable text.
    #[must_use]
    pub fn to_text(&self) -> String {
        let tick = self.tick;
        let elapsed = self.elapsed_secs;
        let tps = self.ticks_per_sec;
        let checksum = self.checksum;
        let hazards = self.active_hazards;
        let chunks = self.chunk_count;
        let violations = self.total_violations;
        format!(
            "CHECKPOINT tick={tick} elapsed={elapsed:.1}s tps={tps:.1} checksum={checksum:08x} hazards={hazards} chunks={chunks} violations={violations}"
        )
    }

    /// Format as JSON.
    ///
    /// # Errors
    /// Returns an error if serialization fails.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Final report after soak test completes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinalReport {
    /// Whether the test passed (no critical violations).
    pub passed: bool,
    /// Seed used for the run.
    pub seed: u64,
    /// Total ticks executed.
    pub ticks_completed: u64,
    /// Target tick count from config.
    pub ticks_requested: u64,
    /// Total wall-clock duration.
    pub duration_secs: f64,
    /// Average ticks per second.
    pub avg_ticks_per_sec: f64,
    /// Final checksum.
    pub final_checksum: u32,
    /// Peak active hazard count.
    pub peak_hazards: u32,
    /// Peak chunk count.
    pub peak_chunks: usize,
    /// Total invariant violations.
    pub total_violations: usize,
    /// Critical violations.
    pub critical_violations: usize,
    /// Sample of violations (first N).
    pub sample_violations: Vec<InvariantViolation>,
    /// Reason if terminated early.
    pub termination_reason: Option<String>,
    /// Checkpoint history (if recorded).
    pub checkpoints: Vec<u32>,
}

impl Default for FinalReport {
    fn default() -> Self {
        Self {
            passed: true,
            seed: 0,
            ticks_completed: 0,
            ticks_requested: 0,
            duration_secs: 0.0,
            avg_ticks_per_sec: 0.0,
            final_checksum: 0,
            peak_hazards: 0,
            peak_chunks: 0,
            total_violations: 0,
            critical_violations: 0,
            sample_violations: Vec::new(),
            termination_reason: None,
            checkpoints: Vec::new(),
        }
    }
}

impl FinalReport {
    /// Format as human-readable text.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut lines = Vec::new();

        let status = if self.passed { "PASSED" } else { "FAILED" };
        let seed = self.seed;
        let completed = self.ticks_completed;
        let requested = self.ticks_requested;
        lines.push(format!(
            "SOAK TEST {status} seed={seed} ticks={completed}/{requested}"
        ));

        let dur = self.duration_secs;
        let tps = self.avg_ticks_per_sec;
        lines.push(format!("  duration: {dur:.1}s ({tps:.1} ticks/sec)"));

        let checksum = self.final_checksum;
        lines.push(format!("  final checksum: {checksum:08x}"));

        let peak_h = self.peak_hazards;
        let peak_c = self.peak_chunks;
        lines.push(format!("  peak hazards: {peak_h}  peak chunks: {peak_c}"));

        let total_v = self.total_violations;
        let crit_v = self.critical_violations;
        lines.push(format!("  violations: {total_v} total, {crit_v} critical"));

        if let Some(reason) = &self.termination_reason {
            lines.push(format!("  terminated: {reason}"));
        }

        if !self.sample_violations.is_empty() {
            lines.push(String::new());
            lines.push("SAMPLE VIOLATIONS:".to_string());
            for v in &self.sample_violations {
                lines.push(format!("  {v}"));
            }
        }

        lines.join("\n")
    }

    /// Format as JSON.
    ///
    /// # Errors
    /// Returns an error if serialization fails.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Format according to specified output format.
    #[must_use]
    pub fn format(&self, fmt: OutputFormat) -> String {
        match fmt {
            OutputFormat::Text => self.to_text(),
            OutputFormat::Json => self
                .to_json()
                .unwrap_or_else(|e| format!("JSON error: {e}")),
        }
    }
}

/// Builder for constructing final reports.
pub struct ReportBuilder {
    seed: u64,
    ticks_requested: u64,
    ticks_completed: u64,
    start_time: std::time::Instant,
    final_checksum: StepChecksum,
    peak_hazards: u32,
    peak_chunks: usize,
    violations: Vec<InvariantViolation>,
    checkpoints: Vec<u32>,
    termination_reason: Option<String>,
    max_sample_violations: usize,
}

impl ReportBuilder {
    /// Create a new report builder.
    #[must_use]
    pub fn new(seed: u64, ticks_requested: u64) -> Self {
        Self {
            seed,
            ticks_requested,
            ticks_completed: 0,
            start_time: std::time::Instant::now(),
            final_checksum: StepChecksum::from_raw(0),
            peak_hazards: 0,
            peak_chunks: 0,
            violations: Vec::new(),
            checkpoints: Vec::new(),
            termination_reason: None,
            max_sample_violations: 10,
        }
    }

    /// Record tick completion.
    pub fn tick_completed(&mut self, tick: u64, checksum: StepChecksum) {
        self.ticks_completed = tick;
        self.final_checksum = checksum;
    }

    /// Update peak values.
    pub fn update_peaks(&mut self, hazards: u32, chunks: usize) {
        self.peak_hazards = self.peak_hazards.max(hazards);
        self.peak_chunks = self.peak_chunks.max(chunks);
    }

    /// Record a violation.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn add_violation(&mut self, violation: InvariantViolation) {
        self.violations.push(violation);
    }

    /// Record multiple violations.
    pub fn add_violations(&mut self, violations: impl IntoIterator<Item = InvariantViolation>) {
        self.violations.extend(violations);
    }

    /// Record a checkpoint.
    pub fn add_checkpoint(&mut self, checksum: u32) {
        self.checkpoints.push(checksum);
    }

    /// Set termination reason.
    pub fn set_termination_reason(&mut self, reason: impl Into<String>) {
        self.termination_reason = Some(reason.into());
    }

    /// Build the final report.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn build(self) -> FinalReport {
        let duration = self.start_time.elapsed();
        let duration_secs = duration.as_secs_f64();
        let avg_ticks_per_sec = if duration_secs > 0.0 {
            self.ticks_completed as f64 / duration_secs
        } else {
            0.0
        };

        let critical_violations = self.violations.iter().filter(|v| v.is_critical()).count();

        let sample_violations: Vec<_> = self
            .violations
            .iter()
            .take(self.max_sample_violations)
            .cloned()
            .collect();

        let passed = critical_violations == 0
            && self.ticks_completed == self.ticks_requested
            && self.termination_reason.is_none();

        FinalReport {
            passed,
            seed: self.seed,
            ticks_completed: self.ticks_completed,
            ticks_requested: self.ticks_requested,
            duration_secs,
            avg_ticks_per_sec,
            final_checksum: self.final_checksum.value(),
            peak_hazards: self.peak_hazards,
            peak_chunks: self.peak_chunks,
            total_violations: self.violations.len(),
            critical_violations,
            sample_violations,
            termination_reason: self.termination_reason,
            checkpoints: self.checkpoints,
        }
    }

    /// Get elapsed duration.
    #[must_use]
    #[allow(dead_code)]
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::invariant::InvariantKind;

    #[test]
    fn output_format_from_str() {
        assert_eq!(OutputFormat::from_str("text"), Some(OutputFormat::Text));
        assert_eq!(OutputFormat::from_str("TEXT"), Some(OutputFormat::Text));
        assert_eq!(OutputFormat::from_str("json"), Some(OutputFormat::Json));
        assert_eq!(OutputFormat::from_str("JSON"), Some(OutputFormat::Json));
        assert_eq!(OutputFormat::from_str("invalid"), None);
    }

    #[test]
    fn checkpoint_report_text() {
        let report = CheckpointReport {
            tick: 1000,
            elapsed_secs: 10.0,
            ticks_per_sec: 100.0,
            checksum: 0xDEAD_BEEF,
            active_hazards: 50,
            chunk_count: 10,
            violations_in_interval: 0,
            total_violations: 0,
        };
        let text = report.to_text();
        assert!(text.contains("tick=1000"));
        assert!(text.contains("tps=100.0"));
        assert!(text.contains("0deadbeef") || text.contains("deadbeef"));
    }

    #[test]
    fn final_report_passed() {
        let mut builder = ReportBuilder::new(42, 100);
        for i in 1..=100u64 {
            #[allow(clippy::cast_possible_truncation)]
            builder.tick_completed(i, StepChecksum::from_raw(i as u32));
            builder.update_peaks(10, 5);
        }
        let report = builder.build();
        assert!(report.passed);
        assert_eq!(report.ticks_completed, 100);
        assert_eq!(report.total_violations, 0);
    }

    #[test]
    fn final_report_failed_violations() {
        let mut builder = ReportBuilder::new(42, 100);
        builder.tick_completed(100, StepChecksum::from_raw(123));
        builder.add_violation(
            InvariantViolation::new(InvariantKind::HazardCountBounds, 50, "too many")
                .with_severity(2),
        );
        let report = builder.build();
        assert!(!report.passed);
        assert_eq!(report.critical_violations, 1);
    }

    #[test]
    fn final_report_text_format() {
        let report = FinalReport {
            passed: true,
            seed: 42,
            ticks_completed: 1000,
            ticks_requested: 1000,
            duration_secs: 10.0,
            avg_ticks_per_sec: 100.0,
            final_checksum: 0x1234_5678,
            peak_hazards: 100,
            peak_chunks: 20,
            total_violations: 0,
            critical_violations: 0,
            sample_violations: vec![],
            termination_reason: None,
            checkpoints: vec![],
        };
        let text = report.to_text();
        assert!(text.contains("PASSED"));
        assert!(text.contains("seed=42"));
        assert!(text.contains("1000/1000"));
    }

    #[test]
    fn final_report_json_format() {
        let report = FinalReport::default();
        let json = report.to_json().unwrap();
        assert!(json.contains("\"passed\""));
        assert!(json.contains("\"seed\""));
    }
}
