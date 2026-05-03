//! Stockpile pressure tracking for director pacing.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Category of stockpile resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum StockpileCategory {
    /// Food and consumables.
    Food,
    /// Water and hydration.
    Water,
    /// Medical supplies.
    Medical,
    /// Building materials.
    Materials,
    /// Fuel and energy.
    Fuel,
    /// Ammunition and weapons.
    Ammunition,
    /// General supplies.
    General,
}

impl StockpileCategory {
    #[must_use]
    pub fn criticality_weight(self) -> f32 {
        match self {
            Self::Food => 1.5,
            Self::Water => 2.0,
            Self::Medical => 1.3,
            Self::Materials => 0.6,
            Self::Fuel => 1.0,
            Self::Ammunition => 0.8,
            Self::General => 0.5,
        }
    }

    #[must_use]
    pub fn is_critical(self) -> bool {
        matches!(self, Self::Food | Self::Water | Self::Medical)
    }
}

/// Status of a single stockpile.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StockpileStatus {
    pub category: StockpileCategory,
    /// Current quantity.
    pub current: u32,
    /// Maximum capacity.
    pub capacity: u32,
    /// Minimum safe level.
    pub safe_minimum: u32,
    /// Consumption rate per tick.
    pub consumption_rate: f32,
    /// Production rate per tick.
    pub production_rate: f32,
    /// Last update tick.
    pub last_update: u64,
}

impl StockpileStatus {
    #[must_use]
    pub fn new(category: StockpileCategory, current: u32, capacity: u32) -> Self {
        Self {
            category,
            current,
            capacity,
            safe_minimum: capacity / 5,
            consumption_rate: 0.0,
            production_rate: 0.0,
            last_update: 0,
        }
    }

    #[must_use]
    pub fn with_safe_minimum(mut self, minimum: u32) -> Self {
        self.safe_minimum = minimum;
        self
    }

    #[must_use]
    pub fn with_rates(mut self, consumption: f32, production: f32) -> Self {
        self.consumption_rate = consumption.max(0.0);
        self.production_rate = production.max(0.0);
        self
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "stockpile values bounded")]
    pub fn fill_ratio(&self) -> f32 {
        if self.capacity == 0 {
            return 0.0;
        }
        self.current as f32 / self.capacity as f32
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "stockpile values bounded")]
    pub fn safety_ratio(&self) -> f32 {
        if self.safe_minimum == 0 {
            return 1.0;
        }
        (self.current as f32 / self.safe_minimum as f32).min(1.0)
    }

    #[must_use]
    pub fn is_critical(&self) -> bool {
        self.current < self.safe_minimum
    }

    #[must_use]
    pub fn is_low(&self) -> bool {
        self.current < self.safe_minimum * 2
    }

    #[must_use]
    pub fn is_full(&self) -> bool {
        self.current >= self.capacity
    }

    #[must_use]
    pub fn net_rate(&self) -> f32 {
        self.production_rate - self.consumption_rate
    }

    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        reason = "bounded values"
    )]
    pub fn ticks_until_empty(&self) -> Option<u64> {
        let net = self.net_rate();
        if net >= 0.0 {
            return None;
        }
        Some((self.current as f32 / -net).ceil() as u64)
    }

    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        reason = "bounded values"
    )]
    pub fn ticks_until_full(&self) -> Option<u64> {
        let net = self.net_rate();
        if net <= 0.0 {
            return None;
        }
        let remaining = self.capacity.saturating_sub(self.current);
        Some((remaining as f32 / net).ceil() as u64)
    }

    #[must_use]
    pub fn pressure(&self) -> f32 {
        let fill_pressure = 1.0 - self.fill_ratio();
        let safety_pressure = 1.0 - self.safety_ratio();
        let rate_pressure = if self.net_rate() < 0.0 {
            (-self.net_rate() / self.consumption_rate.max(1.0)).min(1.0)
        } else {
            0.0
        };

        (fill_pressure * 0.3 + safety_pressure * 0.5 + rate_pressure * 0.2)
            * self.category.criticality_weight()
    }
}

/// Input for stockpile pressure to the director.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StockpilePressureInput {
    /// Status by category.
    pub stockpiles: BTreeMap<StockpileCategory, StockpileStatus>,
    /// Overall pressure (0.0 = abundant, 1.0 = critical shortage).
    pub overall_pressure: f32,
    /// Categories in critical state.
    pub critical_categories: Vec<StockpileCategory>,
    /// Tick when computed.
    pub tick: u64,
}

impl StockpilePressureInput {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_stockpile(&mut self, status: StockpileStatus) {
        self.stockpiles.insert(status.category, status);
    }

    pub fn update(&mut self, tick: u64) {
        self.tick = tick;
        self.critical_categories.clear();

        let mut weighted_pressure = 0.0;
        let mut total_weight = 0.0;

        for (category, status) in &self.stockpiles {
            let weight = category.criticality_weight();
            weighted_pressure += status.pressure() * weight;
            total_weight += weight;

            if status.is_critical() {
                self.critical_categories.push(*category);
            }
        }

        self.overall_pressure = if total_weight > 0.0 {
            (weighted_pressure / total_weight).clamp(0.0, 1.0)
        } else {
            0.0
        };
    }

    #[must_use]
    pub fn get_stockpile(&self, category: StockpileCategory) -> Option<&StockpileStatus> {
        self.stockpiles.get(&category)
    }

    #[must_use]
    pub fn has_critical(&self) -> bool {
        !self.critical_categories.is_empty()
    }

    #[must_use]
    pub fn category_pressure(&self, category: StockpileCategory) -> f32 {
        self.stockpiles
            .get(&category)
            .map_or(0.0, StockpileStatus::pressure)
    }

    #[must_use]
    pub fn worst_category(&self) -> Option<StockpileCategory> {
        self.stockpiles
            .iter()
            .max_by(|a, b| {
                a.1.pressure()
                    .partial_cmp(&b.1.pressure())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(cat, _)| *cat)
    }

    #[must_use]
    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.overall_pressure.to_le_bytes());
        hasher.update(&self.tick.to_le_bytes());
        #[expect(clippy::cast_possible_truncation, reason = "count bounded")]
        {
            hasher.update(&(self.stockpiles.len() as u32).to_le_bytes());
            hasher.update(&(self.critical_categories.len() as u32).to_le_bytes());
        }
        hasher.finalize()
    }
}

/// Summary of stockpile state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StockpileSummary {
    pub tick: u64,
    pub overall_pressure: f32,
    pub critical_count: usize,
    pub low_count: usize,
    pub worst_category: Option<StockpileCategory>,
    pub average_fill_ratio: f32,
}

impl StockpileSummary {
    #[must_use]
    pub fn from_input(input: &StockpilePressureInput) -> Self {
        let low_count = input
            .stockpiles
            .values()
            .filter(|s| s.is_low() && !s.is_critical())
            .count();

        let avg_fill = if input.stockpiles.is_empty() {
            0.0
        } else {
            #[expect(clippy::cast_precision_loss, reason = "count bounded")]
            {
                input
                    .stockpiles
                    .values()
                    .map(StockpileStatus::fill_ratio)
                    .sum::<f32>()
                    / input.stockpiles.len() as f32
            }
        };

        Self {
            tick: input.tick,
            overall_pressure: input.overall_pressure,
            critical_count: input.critical_categories.len(),
            low_count,
            worst_category: input.worst_category(),
            average_fill_ratio: avg_fill,
        }
    }

    #[must_use]
    pub fn is_healthy(&self) -> bool {
        self.critical_count == 0 && self.overall_pressure < 0.3
    }

    #[must_use]
    pub fn is_strained(&self) -> bool {
        self.critical_count > 0 || self.overall_pressure > 0.6
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stockpile_category_weight() {
        assert!(StockpileCategory::Water.criticality_weight() > 1.0);
        assert!(StockpileCategory::General.criticality_weight() < 1.0);
    }

    #[test]
    fn test_stockpile_status_new() {
        let status = StockpileStatus::new(StockpileCategory::Food, 50, 100);

        assert!((status.fill_ratio() - 0.5).abs() < f32::EPSILON);
        assert_eq!(status.safe_minimum, 20);
    }

    #[test]
    fn test_stockpile_status_ratios() {
        let status = StockpileStatus::new(StockpileCategory::Food, 10, 100).with_safe_minimum(20);

        assert!((status.fill_ratio() - 0.1).abs() < f32::EPSILON);
        assert!((status.safety_ratio() - 0.5).abs() < f32::EPSILON);
        assert!(status.is_critical());
    }

    #[test]
    fn test_stockpile_status_rates() {
        let status = StockpileStatus::new(StockpileCategory::Food, 100, 200).with_rates(2.0, 1.0);

        assert!((status.net_rate() - -1.0).abs() < f32::EPSILON);
        assert_eq!(status.ticks_until_empty(), Some(100));
        assert!(status.ticks_until_full().is_none());
    }

    #[test]
    fn test_stockpile_status_rates_positive() {
        let status = StockpileStatus::new(StockpileCategory::Food, 100, 200).with_rates(1.0, 3.0);

        assert!((status.net_rate() - 2.0).abs() < f32::EPSILON);
        assert!(status.ticks_until_empty().is_none());
        assert_eq!(status.ticks_until_full(), Some(50));
    }

    #[test]
    fn test_stockpile_pressure_input() {
        let mut input = StockpilePressureInput::new();

        input.add_stockpile(StockpileStatus::new(StockpileCategory::Food, 50, 100));
        input.add_stockpile(
            StockpileStatus::new(StockpileCategory::Water, 5, 100).with_safe_minimum(20),
        );

        input.update(100);

        assert!(input.has_critical());
        assert_eq!(input.critical_categories.len(), 1);
        assert!(input.overall_pressure > 0.0);
    }

    #[test]
    fn test_stockpile_pressure_worst_category() {
        let mut input = StockpilePressureInput::new();

        input.add_stockpile(StockpileStatus::new(StockpileCategory::Food, 80, 100));
        input.add_stockpile(StockpileStatus::new(StockpileCategory::Water, 10, 100));

        input.update(100);

        assert_eq!(input.worst_category(), Some(StockpileCategory::Water));
    }

    #[test]
    fn test_stockpile_summary() {
        let mut input = StockpilePressureInput::new();
        input.add_stockpile(StockpileStatus::new(StockpileCategory::Food, 80, 100));
        input.update(100);

        let summary = StockpileSummary::from_input(&input);

        assert!(summary.is_healthy());
        assert!(!summary.is_strained());
    }

    #[test]
    fn test_serde_stockpile_status() {
        let status = StockpileStatus::new(StockpileCategory::Medical, 30, 50)
            .with_safe_minimum(10)
            .with_rates(0.5, 0.3);

        let json = serde_json::to_string(&status).unwrap();
        let restored: StockpileStatus = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.current, 30);
        assert_eq!(restored.capacity, 50);
        assert!((restored.consumption_rate - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_bincode_stockpile_input() {
        let mut input = StockpilePressureInput::new();
        input.add_stockpile(StockpileStatus::new(StockpileCategory::Fuel, 60, 100));
        input.update(200);

        let bytes = bincode::serialize(&input).unwrap();
        let restored: StockpilePressureInput = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.checksum(), input.checksum());
        assert_eq!(restored.tick, 200);
    }

    #[test]
    fn test_bincode_stockpile_summary() {
        let summary = StockpileSummary {
            tick: 500,
            overall_pressure: 0.35,
            critical_count: 1,
            low_count: 2,
            worst_category: Some(StockpileCategory::Water),
            average_fill_ratio: 0.6,
        };

        let bytes = bincode::serialize(&summary).unwrap();
        let restored: StockpileSummary = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.tick, 500);
        assert!((restored.overall_pressure - 0.35).abs() < f32::EPSILON);
    }

    #[test]
    fn test_checksum_consistency() {
        let mut input1 = StockpilePressureInput::new();
        let mut input2 = StockpilePressureInput::new();

        input1.add_stockpile(StockpileStatus::new(StockpileCategory::Food, 50, 100));
        input2.add_stockpile(StockpileStatus::new(StockpileCategory::Food, 50, 100));

        input1.update(100);
        input2.update(100);

        assert_eq!(input1.checksum(), input2.checksum());
    }
}
