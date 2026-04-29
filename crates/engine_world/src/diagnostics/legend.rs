//! Diagnostic legend types for UI rendering.

use serde::{Deserialize, Serialize};

use super::channel::{DiagnosticCategory, DiagnosticChannel};
use super::color::DiagnosticColor;

/// Single entry in a diagnostic legend.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LegendEntry {
    pub channel: DiagnosticChannel,
    pub name: String,
    pub color: DiagnosticColor,
    pub min_value: Option<f32>,
    pub max_value: Option<f32>,
    pub unit: Option<String>,
    pub active_count: usize,
    pub enabled: bool,
}

impl LegendEntry {
    /// Create a new legend entry for a channel.
    #[must_use]
    pub fn new(channel: DiagnosticChannel, color: DiagnosticColor) -> Self {
        Self {
            channel,
            name: channel.name().to_string(),
            color,
            min_value: None,
            max_value: None,
            unit: None,
            active_count: 0,
            enabled: true,
        }
    }

    /// Set the display name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set the value range.
    #[must_use]
    pub fn with_range(mut self, min: f32, max: f32) -> Self {
        self.min_value = Some(min);
        self.max_value = Some(max);
        self
    }

    /// Set the unit label.
    #[must_use]
    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    /// Set the active sample count.
    #[must_use]
    pub fn with_count(mut self, count: usize) -> Self {
        self.active_count = count;
        self
    }

    /// Set whether this entry is enabled.
    #[must_use]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Get the category of this entry.
    #[must_use]
    pub fn category(&self) -> DiagnosticCategory {
        self.channel.category()
    }

    /// Check if this entry has an active sample.
    #[must_use]
    pub fn has_samples(&self) -> bool {
        self.active_count > 0
    }

    /// Format the value range as a string.
    #[must_use]
    pub fn format_range(&self) -> Option<String> {
        match (self.min_value, self.max_value) {
            (Some(min), Some(max)) => {
                if let Some(ref unit) = self.unit {
                    Some(format!("{min:.1} - {max:.1} {unit}"))
                } else {
                    Some(format!("{min:.1} - {max:.1}"))
                }
            }
            _ => None,
        }
    }
}

/// Collection of legend entries organized by category.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DiagnosticLegend {
    entries: Vec<LegendEntry>,
    sorted: bool,
}

impl DiagnosticLegend {
    /// Create a new empty legend.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            sorted: true,
        }
    }

    /// Create a legend with preallocated capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            sorted: true,
        }
    }

    /// Add an entry to the legend.
    pub fn push(&mut self, entry: LegendEntry) {
        self.entries.push(entry);
        self.sorted = false;
    }

    /// Add multiple entries.
    pub fn extend(&mut self, entries: impl IntoIterator<Item = LegendEntry>) {
        self.entries.extend(entries);
        self.sorted = false;
    }

    /// Sort entries by category then channel for deterministic ordering.
    pub fn ensure_sorted(&mut self) {
        if !self.sorted {
            self.entries.sort_by_key(|e| (e.category(), e.channel));
            self.sorted = true;
        }
    }

    /// Get all entries.
    #[must_use]
    pub fn entries(&self) -> &[LegendEntry] {
        &self.entries
    }

    /// Get entries mutably.
    pub fn entries_mut(&mut self) -> &mut [LegendEntry] {
        &mut self.entries
    }

    /// Get the number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the legend is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get entries for a specific category.
    pub fn entries_for_category(
        &self,
        category: DiagnosticCategory,
    ) -> impl Iterator<Item = &LegendEntry> {
        self.entries
            .iter()
            .filter(move |e| e.category() == category)
    }

    /// Get only enabled entries.
    pub fn enabled_entries(&self) -> impl Iterator<Item = &LegendEntry> {
        self.entries.iter().filter(|e| e.enabled)
    }

    /// Get only entries with active samples.
    pub fn active_entries(&self) -> impl Iterator<Item = &LegendEntry> {
        self.entries.iter().filter(|e| e.has_samples())
    }

    /// Find an entry by channel.
    #[must_use]
    pub fn find_channel(&self, channel: DiagnosticChannel) -> Option<&LegendEntry> {
        self.entries.iter().find(|e| e.channel == channel)
    }

    /// Find an entry by channel mutably.
    #[must_use]
    pub fn find_channel_mut(&mut self, channel: DiagnosticChannel) -> Option<&mut LegendEntry> {
        self.entries.iter_mut().find(|e| e.channel == channel)
    }

    /// Update the sample count for a channel.
    pub fn update_count(&mut self, channel: DiagnosticChannel, count: usize) {
        if let Some(entry) = self.find_channel_mut(channel) {
            entry.active_count = count;
        }
    }

    /// Set enabled state for a channel.
    pub fn set_enabled(&mut self, channel: DiagnosticChannel, enabled: bool) {
        if let Some(entry) = self.find_channel_mut(channel) {
            entry.enabled = enabled;
        }
    }

    /// Enable all entries.
    pub fn enable_all(&mut self) {
        for entry in &mut self.entries {
            entry.enabled = true;
        }
    }

    /// Disable all entries.
    pub fn disable_all(&mut self) {
        for entry in &mut self.entries {
            entry.enabled = false;
        }
    }

    /// Toggle enabled state for a channel.
    pub fn toggle(&mut self, channel: DiagnosticChannel) {
        if let Some(entry) = self.find_channel_mut(channel) {
            entry.enabled = !entry.enabled;
        }
    }

    /// Get total sample count across all entries.
    #[must_use]
    pub fn total_samples(&self) -> usize {
        self.entries.iter().map(|e| e.active_count).sum()
    }

    /// Get the number of enabled entries.
    #[must_use]
    pub fn enabled_count(&self) -> usize {
        self.entries.iter().filter(|e| e.enabled).count()
    }

    /// Get unique categories present in the legend.
    #[must_use]
    pub fn categories(&self) -> Vec<DiagnosticCategory> {
        let mut cats: Vec<_> = self.entries.iter().map(LegendEntry::category).collect();
        cats.sort();
        cats.dedup();
        cats
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.sorted = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::FieldChannel;

    fn make_entry(channel: DiagnosticChannel) -> LegendEntry {
        LegendEntry::new(channel, DiagnosticColor::RED)
    }

    #[test]
    fn test_legend_entry_new() {
        let entry = make_entry(DiagnosticChannel::Scalar(FieldChannel::Temperature));
        assert_eq!(entry.name, "Temperature");
        assert!(entry.enabled);
        assert_eq!(entry.active_count, 0);
    }

    #[test]
    fn test_legend_entry_builders() {
        let entry = make_entry(DiagnosticChannel::Scalar(FieldChannel::Temperature))
            .with_name("Temp")
            .with_range(-40.0, 100.0)
            .with_unit("C")
            .with_count(42)
            .with_enabled(false);

        assert_eq!(entry.name, "Temp");
        assert_eq!(entry.min_value, Some(-40.0));
        assert_eq!(entry.max_value, Some(100.0));
        assert_eq!(entry.unit, Some("C".to_string()));
        assert_eq!(entry.active_count, 42);
        assert!(!entry.enabled);
    }

    #[test]
    fn test_legend_entry_format_range() {
        let entry = make_entry(DiagnosticChannel::Scalar(FieldChannel::Temperature))
            .with_range(0.0, 100.0)
            .with_unit("C");
        assert_eq!(entry.format_range(), Some("0.0 - 100.0 C".to_string()));

        let entry_no_unit =
            make_entry(DiagnosticChannel::Scalar(FieldChannel::Temperature)).with_range(0.0, 1.0);
        assert_eq!(entry_no_unit.format_range(), Some("0.0 - 1.0".to_string()));

        let entry_no_range = make_entry(DiagnosticChannel::Scalar(FieldChannel::Temperature));
        assert_eq!(entry_no_range.format_range(), None);
    }

    #[test]
    fn test_legend_push_and_len() {
        let mut legend = DiagnosticLegend::new();
        assert!(legend.is_empty());

        legend.push(make_entry(DiagnosticChannel::Scalar(
            FieldChannel::Temperature,
        )));
        assert_eq!(legend.len(), 1);

        legend.push(make_entry(DiagnosticChannel::Scalar(FieldChannel::Oxygen)));
        assert_eq!(legend.len(), 2);
    }

    #[test]
    fn test_legend_deterministic_ordering() {
        let entries = vec![
            make_entry(DiagnosticChannel::Custom(1)),
            make_entry(DiagnosticChannel::Scalar(FieldChannel::Temperature)),
            make_entry(DiagnosticChannel::Scalar(FieldChannel::Oxygen)),
        ];

        let mut legend1 = DiagnosticLegend::new();
        legend1.extend(entries.clone());
        legend1.ensure_sorted();

        let mut legend2 = DiagnosticLegend::new();
        legend2.extend(entries.into_iter().rev());
        legend2.ensure_sorted();

        let channels1: Vec<_> = legend1.entries().iter().map(|e| e.channel).collect();
        let channels2: Vec<_> = legend2.entries().iter().map(|e| e.channel).collect();
        assert_eq!(channels1, channels2);
    }

    #[test]
    fn test_legend_find_channel() {
        let mut legend = DiagnosticLegend::new();
        legend.push(make_entry(DiagnosticChannel::Scalar(
            FieldChannel::Temperature,
        )));
        legend.push(make_entry(DiagnosticChannel::Scalar(FieldChannel::Oxygen)));

        let found = legend.find_channel(DiagnosticChannel::Scalar(FieldChannel::Temperature));
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Temperature");

        let not_found = legend.find_channel(DiagnosticChannel::Custom(99));
        assert!(not_found.is_none());
    }

    #[test]
    fn test_legend_update_count() {
        let mut legend = DiagnosticLegend::new();
        legend.push(make_entry(DiagnosticChannel::Scalar(
            FieldChannel::Temperature,
        )));

        legend.update_count(DiagnosticChannel::Scalar(FieldChannel::Temperature), 100);
        let entry = legend
            .find_channel(DiagnosticChannel::Scalar(FieldChannel::Temperature))
            .unwrap();
        assert_eq!(entry.active_count, 100);
    }

    #[test]
    fn test_legend_enabled_state() {
        let mut legend = DiagnosticLegend::new();
        legend.push(make_entry(DiagnosticChannel::Scalar(
            FieldChannel::Temperature,
        )));
        legend.push(make_entry(DiagnosticChannel::Scalar(FieldChannel::Oxygen)));

        legend.set_enabled(DiagnosticChannel::Scalar(FieldChannel::Temperature), false);
        assert_eq!(legend.enabled_count(), 1);

        legend.enable_all();
        assert_eq!(legend.enabled_count(), 2);

        legend.disable_all();
        assert_eq!(legend.enabled_count(), 0);

        legend.toggle(DiagnosticChannel::Scalar(FieldChannel::Temperature));
        assert_eq!(legend.enabled_count(), 1);
    }

    #[test]
    fn test_legend_total_samples() {
        let mut legend = DiagnosticLegend::new();
        legend
            .push(make_entry(DiagnosticChannel::Scalar(FieldChannel::Temperature)).with_count(10));
        legend.push(make_entry(DiagnosticChannel::Scalar(FieldChannel::Oxygen)).with_count(20));
        assert_eq!(legend.total_samples(), 30);
    }

    #[test]
    fn test_legend_categories() {
        let mut legend = DiagnosticLegend::new();
        legend.push(make_entry(DiagnosticChannel::Scalar(
            FieldChannel::Temperature,
        )));
        legend.push(make_entry(DiagnosticChannel::Scalar(FieldChannel::Oxygen)));
        legend.push(make_entry(DiagnosticChannel::Custom(1)));

        let cats = legend.categories();
        assert_eq!(cats.len(), 2);
        assert!(cats.contains(&DiagnosticCategory::ScalarField));
        assert!(cats.contains(&DiagnosticCategory::Custom));
    }

    #[test]
    fn test_legend_entries_for_category() {
        let mut legend = DiagnosticLegend::new();
        legend.push(make_entry(DiagnosticChannel::Scalar(
            FieldChannel::Temperature,
        )));
        legend.push(make_entry(DiagnosticChannel::Scalar(FieldChannel::Oxygen)));
        legend.push(make_entry(DiagnosticChannel::Custom(1)));

        let scalar_entries: Vec<_> = legend
            .entries_for_category(DiagnosticCategory::ScalarField)
            .collect();
        assert_eq!(scalar_entries.len(), 2);
    }

    #[test]
    fn test_legend_active_entries() {
        let mut legend = DiagnosticLegend::new();
        legend
            .push(make_entry(DiagnosticChannel::Scalar(FieldChannel::Temperature)).with_count(10));
        legend.push(make_entry(DiagnosticChannel::Scalar(FieldChannel::Oxygen)).with_count(0));

        let active: Vec<_> = legend.active_entries().collect();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].name, "Temperature");
    }

    #[test]
    fn test_legend_clear() {
        let mut legend = DiagnosticLegend::new();
        legend.push(make_entry(DiagnosticChannel::Scalar(
            FieldChannel::Temperature,
        )));
        legend.clear();
        assert!(legend.is_empty());
    }

    #[test]
    fn test_serde_round_trip() {
        let mut legend = DiagnosticLegend::new();
        legend.push(
            make_entry(DiagnosticChannel::Scalar(FieldChannel::Temperature))
                .with_range(0.0, 100.0)
                .with_unit("C")
                .with_count(50),
        );

        let json = serde_json::to_string(&legend).unwrap();
        let recovered: DiagnosticLegend = serde_json::from_str(&json).unwrap();

        assert_eq!(recovered.len(), 1);
        let entry = &recovered.entries()[0];
        assert_eq!(entry.name, "Temperature");
        assert_eq!(entry.min_value, Some(0.0));
        assert_eq!(entry.active_count, 50);
    }
}
