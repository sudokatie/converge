//! Diagnostic channel and category filtering.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::channel::{DiagnosticCategory, DiagnosticChannel};

/// Filter mode for diagnostic channel selection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FilterMode {
    /// Include all channels (default).
    #[default]
    All,
    /// Include only specified channels/categories.
    Include,
    /// Exclude specified channels/categories.
    Exclude,
}

/// Filter for selecting which diagnostic channels to display.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DiagnosticFilter {
    mode: FilterMode,
    channels: HashSet<DiagnosticChannel>,
    categories: HashSet<DiagnosticCategory>,
    min_intensity: f32,
    max_samples: Option<usize>,
}

impl DiagnosticFilter {
    /// Create a filter that passes all channels.
    #[must_use]
    pub fn all() -> Self {
        Self {
            mode: FilterMode::All,
            channels: HashSet::new(),
            categories: HashSet::new(),
            min_intensity: 0.0,
            max_samples: None,
        }
    }

    /// Create a filter that includes only specified channels.
    #[must_use]
    pub fn include(channels: impl IntoIterator<Item = DiagnosticChannel>) -> Self {
        Self {
            mode: FilterMode::Include,
            channels: channels.into_iter().collect(),
            categories: HashSet::new(),
            min_intensity: 0.0,
            max_samples: None,
        }
    }

    /// Create a filter that excludes specified channels.
    #[must_use]
    pub fn exclude(channels: impl IntoIterator<Item = DiagnosticChannel>) -> Self {
        Self {
            mode: FilterMode::Exclude,
            channels: channels.into_iter().collect(),
            categories: HashSet::new(),
            min_intensity: 0.0,
            max_samples: None,
        }
    }

    /// Create a filter for a single category.
    #[must_use]
    pub fn category(category: DiagnosticCategory) -> Self {
        let mut categories = HashSet::new();
        categories.insert(category);
        Self {
            mode: FilterMode::Include,
            channels: HashSet::new(),
            categories,
            min_intensity: 0.0,
            max_samples: None,
        }
    }

    /// Add a channel to include/exclude.
    pub fn add_channel(&mut self, channel: DiagnosticChannel) {
        self.channels.insert(channel);
    }

    /// Remove a channel from the filter.
    pub fn remove_channel(&mut self, channel: &DiagnosticChannel) {
        self.channels.remove(channel);
    }

    /// Add a category to include/exclude.
    pub fn add_category(&mut self, category: DiagnosticCategory) {
        self.categories.insert(category);
    }

    /// Remove a category from the filter.
    pub fn remove_category(&mut self, category: &DiagnosticCategory) {
        self.categories.remove(category);
    }

    /// Set the filter mode.
    pub fn set_mode(&mut self, mode: FilterMode) {
        self.mode = mode;
    }

    /// Get the current filter mode.
    #[must_use]
    pub fn mode(&self) -> FilterMode {
        self.mode
    }

    /// Set minimum intensity threshold.
    pub fn set_min_intensity(&mut self, intensity: f32) {
        self.min_intensity = intensity.clamp(0.0, 1.0);
    }

    /// Get minimum intensity threshold.
    #[must_use]
    pub fn min_intensity(&self) -> f32 {
        self.min_intensity
    }

    /// Set maximum number of samples to return.
    pub fn set_max_samples(&mut self, max: Option<usize>) {
        self.max_samples = max;
    }

    /// Get maximum number of samples.
    #[must_use]
    pub fn max_samples(&self) -> Option<usize> {
        self.max_samples
    }

    /// Builder method for minimum intensity.
    #[must_use]
    pub fn with_min_intensity(mut self, intensity: f32) -> Self {
        self.set_min_intensity(intensity);
        self
    }

    /// Builder method for maximum samples.
    #[must_use]
    pub fn with_max_samples(mut self, max: usize) -> Self {
        self.max_samples = Some(max);
        self
    }

    /// Check if a channel passes the filter.
    #[must_use]
    pub fn accepts_channel(&self, channel: DiagnosticChannel) -> bool {
        let category = channel.category();

        match self.mode {
            FilterMode::All => true,
            FilterMode::Include => {
                self.channels.contains(&channel) || self.categories.contains(&category)
            }
            FilterMode::Exclude => {
                !self.channels.contains(&channel) && !self.categories.contains(&category)
            }
        }
    }

    /// Check if a category passes the filter.
    #[must_use]
    pub fn accepts_category(&self, category: DiagnosticCategory) -> bool {
        match self.mode {
            FilterMode::All => true,
            FilterMode::Include => {
                self.categories.contains(&category)
                    || self.channels.iter().any(|ch| ch.category() == category)
            }
            FilterMode::Exclude => {
                !self.categories.contains(&category)
                    && !self.channels.iter().any(|ch| ch.category() == category)
            }
        }
    }

    /// Check if an intensity passes the filter.
    #[must_use]
    pub fn accepts_intensity(&self, intensity: f32) -> bool {
        intensity >= self.min_intensity
    }

    /// Get the list of included channels (only meaningful in Include mode).
    pub fn channels(&self) -> impl Iterator<Item = &DiagnosticChannel> + '_ {
        self.channels.iter()
    }

    /// Get the list of included categories (only meaningful in Include mode).
    pub fn categories(&self) -> impl Iterator<Item = &DiagnosticCategory> + '_ {
        self.categories.iter()
    }

    /// Get the number of explicitly filtered channels.
    #[must_use]
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    /// Get the number of explicitly filtered categories.
    #[must_use]
    pub fn category_count(&self) -> usize {
        self.categories.len()
    }

    /// Check if filter is effectively a pass-all filter.
    #[must_use]
    pub fn is_pass_all(&self) -> bool {
        self.mode == FilterMode::All
            || (self.mode == FilterMode::Exclude
                && self.channels.is_empty()
                && self.categories.is_empty())
    }

    /// Clear all channel and category filters.
    pub fn clear(&mut self) {
        self.channels.clear();
        self.categories.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::FieldChannel;

    #[test]
    fn test_filter_all_accepts_everything() {
        let filter = DiagnosticFilter::all();
        assert!(filter.accepts_channel(DiagnosticChannel::Scalar(FieldChannel::Temperature)));
        assert!(filter.accepts_channel(DiagnosticChannel::Custom(42)));
        assert!(filter.accepts_category(DiagnosticCategory::ScalarField));
        assert!(filter.accepts_category(DiagnosticCategory::Custom));
    }

    #[test]
    fn test_filter_include_channels() {
        let filter = DiagnosticFilter::include([
            DiagnosticChannel::Scalar(FieldChannel::Temperature),
            DiagnosticChannel::Scalar(FieldChannel::Oxygen),
        ]);
        assert!(filter.accepts_channel(DiagnosticChannel::Scalar(FieldChannel::Temperature)));
        assert!(filter.accepts_channel(DiagnosticChannel::Scalar(FieldChannel::Oxygen)));
        assert!(!filter.accepts_channel(DiagnosticChannel::Scalar(FieldChannel::Radiation)));
        assert!(!filter.accepts_channel(DiagnosticChannel::Custom(1)));
    }

    #[test]
    fn test_filter_exclude_channels() {
        let filter =
            DiagnosticFilter::exclude([DiagnosticChannel::Scalar(FieldChannel::Temperature)]);
        assert!(!filter.accepts_channel(DiagnosticChannel::Scalar(FieldChannel::Temperature)));
        assert!(filter.accepts_channel(DiagnosticChannel::Scalar(FieldChannel::Oxygen)));
        assert!(filter.accepts_channel(DiagnosticChannel::Custom(1)));
    }

    #[test]
    fn test_filter_category() {
        let filter = DiagnosticFilter::category(DiagnosticCategory::ScalarField);
        assert!(filter.accepts_channel(DiagnosticChannel::Scalar(FieldChannel::Temperature)));
        assert!(filter.accepts_channel(DiagnosticChannel::Scalar(FieldChannel::Oxygen)));
        assert!(!filter.accepts_channel(DiagnosticChannel::Custom(1)));
        assert!(filter.accepts_category(DiagnosticCategory::ScalarField));
        assert!(!filter.accepts_category(DiagnosticCategory::Custom));
    }

    #[test]
    fn test_filter_min_intensity() {
        let filter = DiagnosticFilter::all().with_min_intensity(0.5);
        assert!(!filter.accepts_intensity(0.3));
        assert!(filter.accepts_intensity(0.5));
        assert!(filter.accepts_intensity(0.8));
    }

    #[test]
    fn test_filter_max_samples() {
        let filter = DiagnosticFilter::all().with_max_samples(100);
        assert_eq!(filter.max_samples(), Some(100));
    }

    #[test]
    fn test_filter_is_pass_all() {
        assert!(DiagnosticFilter::all().is_pass_all());
        assert!(DiagnosticFilter::exclude([]).is_pass_all());
        assert!(!DiagnosticFilter::include([DiagnosticChannel::Custom(1)]).is_pass_all());
        assert!(!DiagnosticFilter::exclude([DiagnosticChannel::Custom(1)]).is_pass_all());
    }

    #[test]
    fn test_filter_add_remove_channel() {
        let mut filter = DiagnosticFilter::include([]);
        assert_eq!(filter.channel_count(), 0);

        filter.add_channel(DiagnosticChannel::Scalar(FieldChannel::Temperature));
        assert_eq!(filter.channel_count(), 1);
        assert!(filter.accepts_channel(DiagnosticChannel::Scalar(FieldChannel::Temperature)));

        filter.remove_channel(&DiagnosticChannel::Scalar(FieldChannel::Temperature));
        assert_eq!(filter.channel_count(), 0);
    }

    #[test]
    fn test_filter_add_remove_category() {
        let mut filter = DiagnosticFilter::include([]);
        filter.add_category(DiagnosticCategory::Hazard);
        assert_eq!(filter.category_count(), 1);
        assert!(filter.accepts_category(DiagnosticCategory::Hazard));

        filter.remove_category(&DiagnosticCategory::Hazard);
        assert_eq!(filter.category_count(), 0);
    }

    #[test]
    fn test_filter_clear() {
        let mut filter =
            DiagnosticFilter::include([DiagnosticChannel::Scalar(FieldChannel::Temperature)]);
        filter.add_category(DiagnosticCategory::Hazard);
        filter.clear();
        assert_eq!(filter.channel_count(), 0);
        assert_eq!(filter.category_count(), 0);
    }

    #[test]
    fn test_filter_mode_setter() {
        let mut filter = DiagnosticFilter::all();
        assert_eq!(filter.mode(), FilterMode::All);
        filter.set_mode(FilterMode::Include);
        assert_eq!(filter.mode(), FilterMode::Include);
    }

    #[test]
    fn test_deterministic_ordering() {
        let channels = [
            DiagnosticChannel::Scalar(FieldChannel::Temperature),
            DiagnosticChannel::Scalar(FieldChannel::Oxygen),
        ];
        let filter1 = DiagnosticFilter::include(channels);
        let filter2 = DiagnosticFilter::include(channels.into_iter().rev());

        for ch in channels {
            assert_eq!(filter1.accepts_channel(ch), filter2.accepts_channel(ch));
        }
    }

    #[test]
    fn test_serde_round_trip() {
        let filter =
            DiagnosticFilter::include([DiagnosticChannel::Scalar(FieldChannel::Temperature)])
                .with_min_intensity(0.25)
                .with_max_samples(500);

        let json = serde_json::to_string(&filter).unwrap();
        let recovered: DiagnosticFilter = serde_json::from_str(&json).unwrap();

        assert_eq!(recovered.mode(), filter.mode());
        assert!(
            (recovered.min_intensity() - filter.min_intensity()).abs() < f32::EPSILON,
            "min_intensity mismatch"
        );
        assert_eq!(recovered.max_samples(), filter.max_samples());
        assert_eq!(recovered.channel_count(), filter.channel_count());
    }
}
