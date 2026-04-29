//! Summary statistics for diagnostic data.

use serde::{Deserialize, Serialize};

use super::channel::{DiagnosticCategory, DiagnosticChannel};

/// Per-category sample counts.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryCounts {
    pub counts: [usize; DiagnosticCategory::COUNT],
}

impl CategoryCounts {
    #[must_use]
    pub fn new() -> Self {
        Self {
            counts: [0; DiagnosticCategory::COUNT],
        }
    }

    pub fn increment(&mut self, category: DiagnosticCategory) {
        self.counts[category.as_index()] += 1;
    }

    #[must_use]
    pub fn get(&self, category: DiagnosticCategory) -> usize {
        self.counts[category.as_index()]
    }

    #[must_use]
    pub fn total(&self) -> usize {
        self.counts.iter().sum()
    }

    pub fn clear(&mut self) {
        self.counts = [0; DiagnosticCategory::COUNT];
    }
}

/// Statistics for a single diagnostic channel.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ChannelStats {
    pub channel: Option<DiagnosticChannel>,
    pub sample_count: usize,
    pub min_value: f32,
    pub max_value: f32,
    pub mean_value: f32,
}

impl ChannelStats {
    #[must_use]
    pub fn new(channel: DiagnosticChannel) -> Self {
        Self {
            channel: Some(channel),
            sample_count: 0,
            min_value: f32::MAX,
            max_value: f32::MIN,
            mean_value: 0.0,
        }
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn record(&mut self, value: f32) {
        self.min_value = self.min_value.min(value);
        self.max_value = self.max_value.max(value);
        let n = self.sample_count as f32;
        self.mean_value = (self.mean_value * n + value) / (n + 1.0);
        self.sample_count += 1;
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sample_count == 0
    }
}

/// Aggregate summary of diagnostic data.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DiagnosticSummary {
    pub category_counts: CategoryCounts,
    pub channel_stats: Vec<ChannelStats>,
    pub total_samples: usize,
}

impl DiagnosticSummary {
    #[must_use]
    pub fn new() -> Self {
        Self {
            category_counts: CategoryCounts::new(),
            channel_stats: Vec::new(),
            total_samples: 0,
        }
    }

    pub fn add_channel_stats(&mut self, stats: ChannelStats) {
        if let Some(channel) = stats.channel {
            self.category_counts.increment(channel.category());
        }
        self.total_samples += stats.sample_count;
        self.channel_stats.push(stats);
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.total_samples == 0
    }

    pub fn clear(&mut self) {
        self.category_counts.clear();
        self.channel_stats.clear();
        self.total_samples = 0;
    }
}
