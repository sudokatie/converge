//! Crystal seam and mineral deposit simulation.

use serde::{Deserialize, Serialize};

use super::config::CrystalGrowthConfig;
use super::identity::FeatureId;

/// Type of crystal formation.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[repr(u8)]
pub enum CrystalType {
    /// Quartz crystal formation.
    #[default]
    Quartz = 0,
    /// Amethyst crystal.
    Amethyst = 1,
    /// Emerald crystal.
    Emerald = 2,
    /// Ruby crystal.
    Ruby = 3,
    /// Sapphire crystal.
    Sapphire = 4,
    /// Diamond formation.
    Diamond = 5,
    /// Exotic/alien crystal.
    Exotic = 6,
}

impl CrystalType {
    pub const ALL: [CrystalType; 7] = [
        Self::Quartz,
        Self::Amethyst,
        Self::Emerald,
        Self::Ruby,
        Self::Sapphire,
        Self::Diamond,
        Self::Exotic,
    ];

    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Quartz => "quartz",
            Self::Amethyst => "amethyst",
            Self::Emerald => "emerald",
            Self::Ruby => "ruby",
            Self::Sapphire => "sapphire",
            Self::Diamond => "diamond",
            Self::Exotic => "exotic",
        }
    }

    #[must_use]
    pub const fn rarity(&self) -> f32 {
        match self {
            Self::Quartz => 1.0,
            Self::Amethyst => 0.5,
            Self::Emerald => 0.2,
            Self::Ruby | Self::Sapphire => 0.15,
            Self::Diamond => 0.05,
            Self::Exotic => 0.01,
        }
    }

    #[must_use]
    pub const fn base_value(&self) -> f32 {
        match self {
            Self::Quartz => 1.0,
            Self::Amethyst => 5.0,
            Self::Emerald => 20.0,
            Self::Ruby | Self::Sapphire => 25.0,
            Self::Diamond => 100.0,
            Self::Exotic => 500.0,
        }
    }

    #[must_use]
    pub const fn optimal_temperature(&self) -> f32 {
        match self {
            Self::Quartz => 200.0,
            Self::Amethyst => 250.0,
            Self::Emerald => 400.0,
            Self::Ruby | Self::Sapphire => 500.0,
            Self::Diamond => 1200.0,
            Self::Exotic => 800.0,
        }
    }

    #[must_use]
    pub const fn optimal_pressure(&self) -> f32 {
        match self {
            Self::Quartz => 10.0,
            Self::Amethyst => 20.0,
            Self::Emerald => 50.0,
            Self::Ruby | Self::Sapphire => 60.0,
            Self::Diamond => 150.0,
            Self::Exotic => 100.0,
        }
    }

    #[must_use]
    pub const fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Quartz),
            1 => Some(Self::Amethyst),
            2 => Some(Self::Emerald),
            3 => Some(Self::Ruby),
            4 => Some(Self::Sapphire),
            5 => Some(Self::Diamond),
            6 => Some(Self::Exotic),
            _ => None,
        }
    }
}

/// A seam of crystal formations.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CrystalSeam {
    /// Unique identifier.
    pub id: FeatureId,
    /// Crystal type.
    pub crystal_type: CrystalType,
    /// Position (x, y, z).
    pub position: (f32, f32, f32),
    /// Seam extent/radius.
    pub extent: f32,
    /// Quality factor (0-1, affects yield).
    pub quality: f32,
    /// Current crystal mass/quantity.
    quantity: f32,
    /// Maximum capacity.
    capacity: f32,
    /// Growth rate multiplier.
    growth_multiplier: f32,
    /// Whether seam is active (growing).
    active: bool,
    /// Ticks since conditions were favorable.
    favorable_ticks: u32,
    /// Last tick processed.
    last_tick: u64,
}

impl CrystalSeam {
    #[must_use]
    pub fn new(id: FeatureId, crystal_type: CrystalType, position: (f32, f32, f32)) -> Self {
        Self {
            id,
            crystal_type,
            position,
            extent: 5.0,
            quality: 0.5,
            quantity: 10.0,
            capacity: 100.0,
            growth_multiplier: 1.0,
            active: true,
            favorable_ticks: 0,
            last_tick: 0,
        }
    }

    #[must_use]
    pub fn with_extent(mut self, extent: f32) -> Self {
        self.extent = extent.clamp(1.0, 50.0);
        self
    }

    #[must_use]
    pub fn with_quality(mut self, quality: f32) -> Self {
        self.quality = quality.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_quantity(mut self, quantity: f32) -> Self {
        self.quantity = quantity.clamp(0.0, self.capacity);
        self
    }

    #[must_use]
    pub fn with_capacity(mut self, capacity: f32) -> Self {
        self.capacity = capacity.max(1.0);
        self.quantity = self.quantity.min(self.capacity);
        self
    }

    #[must_use]
    pub fn with_growth_multiplier(mut self, multiplier: f32) -> Self {
        self.growth_multiplier = multiplier.clamp(0.1, 5.0);
        self
    }

    #[must_use]
    pub fn quantity(&self) -> f32 {
        self.quantity
    }

    #[must_use]
    pub fn capacity(&self) -> f32 {
        self.capacity
    }

    #[must_use]
    pub fn fill_ratio(&self) -> f32 {
        if self.capacity > 0.0 {
            self.quantity / self.capacity
        } else {
            0.0
        }
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        self.active
    }

    #[must_use]
    pub fn is_depleted(&self) -> bool {
        self.quantity < 0.1
    }

    #[must_use]
    pub fn is_full(&self) -> bool {
        self.fill_ratio() >= 0.99
    }

    #[must_use]
    pub fn depth(&self) -> f32 {
        self.position.2
    }

    #[must_use]
    pub fn value(&self) -> f32 {
        self.quantity * self.quality * self.crystal_type.base_value()
    }

    pub fn harvest(&mut self, amount: f32) -> f32 {
        let harvested = amount.min(self.quantity);
        self.quantity -= harvested;
        harvested * self.quality
    }

    pub fn tick(
        &mut self,
        config: &CrystalGrowthConfig,
        temperature: f32,
        pressure: f32,
        current_tick: u64,
    ) {
        if current_tick <= self.last_tick {
            return;
        }
        self.last_tick = current_tick;

        if !self.active {
            return;
        }

        let temp_factor = self.temperature_factor(temperature);
        let pressure_factor = self.pressure_factor(pressure);
        let growth_factor = temp_factor * pressure_factor * self.growth_multiplier;

        if growth_factor > 0.5 {
            self.favorable_ticks += 1;
            #[allow(clippy::cast_precision_loss)]
            let ticks_bonus = self.favorable_ticks as f32 * 0.001;
            let growth = config.base_growth_rate * growth_factor * (1.0 + ticks_bonus);
            self.quantity = (self.quantity + growth).min(self.capacity);

            if growth_factor > 0.8 {
                self.quality = (self.quality + 0.0001).min(1.0);
            }
        } else if growth_factor < 0.2 {
            self.favorable_ticks = 0;
            self.quantity = (self.quantity - config.degradation_rate).max(0.0);

            if growth_factor < 0.1 && self.quantity > 0.0 {
                self.quality = (self.quality - 0.0001).max(0.1);
            }
        }

        if self.is_depleted() {
            self.active = false;
        }
    }

    fn temperature_factor(&self, temperature: f32) -> f32 {
        let optimal = self.crystal_type.optimal_temperature();
        let diff = (temperature - optimal).abs();
        let tolerance = optimal * 0.3;
        (1.0 - diff / tolerance).max(0.0)
    }

    fn pressure_factor(&self, pressure: f32) -> f32 {
        let optimal = self.crystal_type.optimal_pressure();
        let diff = (pressure - optimal).abs();
        let tolerance = optimal * 0.5;
        (1.0 - diff / tolerance).max(0.0)
    }

    #[must_use]
    pub fn fingerprint(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.id.raw().to_le_bytes());
        hasher.update(&self.quantity.to_le_bytes());
        hasher.update(&self.quality.to_le_bytes());
        hasher.update(&[u8::from(self.active)]);
        hasher.finalize()
    }
}

/// Type of mineral deposit.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[repr(u8)]
pub enum MineralType {
    /// Iron ore deposit.
    #[default]
    Iron = 0,
    /// Copper ore deposit.
    Copper = 1,
    /// Gold ore deposit.
    Gold = 2,
    /// Silver ore deposit.
    Silver = 3,
    /// Titanium ore deposit.
    Titanium = 4,
    /// Uranium ore deposit.
    Uranium = 5,
    /// Rare earth elements.
    RareEarth = 6,
}

impl MineralType {
    pub const ALL: [MineralType; 7] = [
        Self::Iron,
        Self::Copper,
        Self::Gold,
        Self::Silver,
        Self::Titanium,
        Self::Uranium,
        Self::RareEarth,
    ];

    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Iron => "iron",
            Self::Copper => "copper",
            Self::Gold => "gold",
            Self::Silver => "silver",
            Self::Titanium => "titanium",
            Self::Uranium => "uranium",
            Self::RareEarth => "rare_earth",
        }
    }

    #[must_use]
    pub const fn density(&self) -> f32 {
        match self {
            Self::Iron => 7.87,
            Self::Copper => 8.96,
            Self::Gold => 19.3,
            Self::Silver => 10.5,
            Self::Titanium => 4.5,
            Self::Uranium => 19.1,
            Self::RareEarth => 6.0,
        }
    }

    #[must_use]
    pub const fn base_value(&self) -> f32 {
        match self {
            Self::Iron => 1.0,
            Self::Copper => 3.0,
            Self::Gold => 50.0,
            Self::Silver => 20.0,
            Self::Titanium => 15.0,
            Self::Uranium => 100.0,
            Self::RareEarth => 80.0,
        }
    }

    #[must_use]
    pub const fn is_radioactive(&self) -> bool {
        matches!(self, Self::Uranium)
    }

    #[must_use]
    pub const fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Iron),
            1 => Some(Self::Copper),
            2 => Some(Self::Gold),
            3 => Some(Self::Silver),
            4 => Some(Self::Titanium),
            5 => Some(Self::Uranium),
            6 => Some(Self::RareEarth),
            _ => None,
        }
    }
}

/// A mineral deposit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MineralDeposit {
    /// Unique identifier.
    pub id: FeatureId,
    /// Mineral type.
    pub mineral_type: MineralType,
    /// Position (x, y, z).
    pub position: (f32, f32, f32),
    /// Deposit extent/radius.
    pub extent: f32,
    /// Ore concentration (0-1).
    pub concentration: f32,
    /// Remaining ore quantity.
    quantity: f32,
    /// Initial ore quantity.
    initial_quantity: f32,
    /// Extraction difficulty (higher = harder).
    pub difficulty: f32,
    /// Whether deposit has been discovered.
    discovered: bool,
    /// Total ore extracted.
    total_extracted: f32,
}

impl MineralDeposit {
    #[must_use]
    pub fn new(id: FeatureId, mineral_type: MineralType, position: (f32, f32, f32)) -> Self {
        Self {
            id,
            mineral_type,
            position,
            extent: 10.0,
            concentration: 0.3,
            quantity: 1000.0,
            initial_quantity: 1000.0,
            difficulty: 1.0,
            discovered: false,
            total_extracted: 0.0,
        }
    }

    #[must_use]
    pub fn with_extent(mut self, extent: f32) -> Self {
        self.extent = extent.clamp(1.0, 100.0);
        self
    }

    #[must_use]
    pub fn with_concentration(mut self, concentration: f32) -> Self {
        self.concentration = concentration.clamp(0.01, 1.0);
        self
    }

    #[must_use]
    pub fn with_quantity(mut self, quantity: f32) -> Self {
        self.quantity = quantity.max(0.0);
        self.initial_quantity = self.quantity;
        self
    }

    #[must_use]
    pub fn with_difficulty(mut self, difficulty: f32) -> Self {
        self.difficulty = difficulty.clamp(0.5, 5.0);
        self
    }

    #[must_use]
    pub fn quantity(&self) -> f32 {
        self.quantity
    }

    #[must_use]
    pub fn remaining_ratio(&self) -> f32 {
        if self.initial_quantity > 0.0 {
            self.quantity / self.initial_quantity
        } else {
            0.0
        }
    }

    #[must_use]
    pub fn is_discovered(&self) -> bool {
        self.discovered
    }

    #[must_use]
    pub fn is_depleted(&self) -> bool {
        self.quantity < 1.0
    }

    #[must_use]
    pub fn depth(&self) -> f32 {
        self.position.2
    }

    #[must_use]
    pub fn value(&self) -> f32 {
        self.quantity * self.concentration * self.mineral_type.base_value()
    }

    #[must_use]
    pub fn total_extracted(&self) -> f32 {
        self.total_extracted
    }

    pub fn discover(&mut self) {
        self.discovered = true;
    }

    pub fn extract(&mut self, amount: f32) -> f32 {
        let effective_amount = amount / self.difficulty;
        let extracted = effective_amount.min(self.quantity);
        self.quantity -= extracted;
        self.total_extracted += extracted;
        extracted * self.concentration
    }

    #[must_use]
    pub fn distance_to(&self, point: (f32, f32, f32)) -> f32 {
        let dx = point.0 - self.position.0;
        let dy = point.1 - self.position.1;
        let dz = point.2 - self.position.2;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    #[must_use]
    pub fn is_point_inside(&self, point: (f32, f32, f32)) -> bool {
        self.distance_to(point) <= self.extent
    }

    #[must_use]
    pub fn fingerprint(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.id.raw().to_le_bytes());
        hasher.update(&self.quantity.to_le_bytes());
        hasher.update(&[u8::from(self.discovered)]);
        hasher.finalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_seam() -> CrystalSeam {
        CrystalSeam::new(FeatureId::new(1), CrystalType::Quartz, (0.0, 0.0, 50.0))
    }

    fn test_deposit() -> MineralDeposit {
        MineralDeposit::new(FeatureId::new(2), MineralType::Iron, (0.0, 0.0, 100.0))
    }

    #[test]
    fn crystal_type_properties() {
        assert_eq!(CrystalType::Quartz.name(), "quartz");
        assert!(CrystalType::Diamond.rarity() < CrystalType::Quartz.rarity());
        assert!(CrystalType::Diamond.base_value() > CrystalType::Quartz.base_value());
        assert!(CrystalType::Diamond.optimal_pressure() > CrystalType::Quartz.optimal_pressure());
    }

    #[test]
    fn crystal_type_from_raw() {
        for (i, ct) in CrystalType::ALL.iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            let idx = i as u8;
            assert_eq!(CrystalType::from_raw(idx), Some(*ct));
        }
        assert_eq!(CrystalType::from_raw(20), None);
    }

    #[test]
    fn crystal_seam_creation() {
        let seam = test_seam().with_quality(0.8).with_quantity(50.0);

        assert!((seam.quality - 0.8).abs() < f32::EPSILON);
        assert!((seam.quantity() - 50.0).abs() < f32::EPSILON);
        assert!(seam.is_active());
    }

    #[test]
    fn crystal_seam_harvest() {
        let mut seam = test_seam().with_quality(0.5).with_quantity(100.0);

        let yield_amount = seam.harvest(30.0);
        assert!((yield_amount - 15.0).abs() < f32::EPSILON);
        assert!((seam.quantity() - 70.0).abs() < f32::EPSILON);
    }

    #[test]
    fn crystal_seam_depletion() {
        let mut seam = test_seam().with_quantity(0.05);
        assert!(seam.is_depleted());

        let config = CrystalGrowthConfig::new();
        seam.tick(&config, 100.0, 10.0, 1);
        assert!(!seam.is_active());
    }

    #[test]
    fn crystal_seam_value() {
        let seam = test_seam().with_quality(1.0).with_quantity(100.0);

        let value = seam.value();
        assert!((value - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn mineral_type_properties() {
        assert_eq!(MineralType::Iron.name(), "iron");
        assert!(MineralType::Gold.density() > MineralType::Iron.density());
        assert!(MineralType::Uranium.is_radioactive());
        assert!(!MineralType::Gold.is_radioactive());
    }

    #[test]
    fn mineral_type_from_raw() {
        for (i, mt) in MineralType::ALL.iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            let idx = i as u8;
            assert_eq!(MineralType::from_raw(idx), Some(*mt));
        }
        assert_eq!(MineralType::from_raw(20), None);
    }

    #[test]
    fn mineral_deposit_creation() {
        let deposit = test_deposit().with_concentration(0.5).with_quantity(500.0);

        assert!((deposit.concentration - 0.5).abs() < f32::EPSILON);
        assert!((deposit.quantity() - 500.0).abs() < f32::EPSILON);
        assert!(!deposit.is_discovered());
    }

    #[test]
    fn mineral_deposit_discovery() {
        let mut deposit = test_deposit();
        assert!(!deposit.is_discovered());

        deposit.discover();
        assert!(deposit.is_discovered());
    }

    #[test]
    fn mineral_deposit_extraction() {
        let mut deposit = test_deposit()
            .with_concentration(0.5)
            .with_quantity(100.0)
            .with_difficulty(1.0);

        let yield_amount = deposit.extract(20.0);
        assert!((yield_amount - 10.0).abs() < f32::EPSILON);
        assert!((deposit.quantity() - 80.0).abs() < f32::EPSILON);
        assert!((deposit.total_extracted() - 20.0).abs() < f32::EPSILON);
    }

    #[test]
    fn mineral_deposit_difficulty() {
        let mut easy = test_deposit().with_difficulty(0.5).with_quantity(100.0);
        let mut hard = test_deposit().with_difficulty(2.0).with_quantity(100.0);

        let easy_yield = easy.extract(10.0);
        let hard_yield = hard.extract(10.0);

        assert!(easy_yield > hard_yield);
    }

    #[test]
    fn mineral_deposit_geometry() {
        let deposit = test_deposit().with_extent(20.0);

        assert!(deposit.is_point_inside((10.0, 0.0, 100.0)));
        assert!(!deposit.is_point_inside((30.0, 0.0, 100.0)));
    }

    #[test]
    fn fingerprint_determinism() {
        let seam1 = test_seam().with_quantity(50.0);
        let seam2 = test_seam().with_quantity(50.0);
        assert_eq!(seam1.fingerprint(), seam2.fingerprint());

        let deposit1 = test_deposit().with_quantity(500.0);
        let deposit2 = test_deposit().with_quantity(500.0);
        assert_eq!(deposit1.fingerprint(), deposit2.fingerprint());
    }

    #[test]
    fn serde_crystal_type() {
        let ct = CrystalType::Diamond;
        let json = serde_json::to_string(&ct).unwrap();
        let recovered: CrystalType = serde_json::from_str(&json).unwrap();
        assert_eq!(ct, recovered);
    }

    #[test]
    fn serde_crystal_seam() {
        let seam = test_seam().with_quality(0.9).with_quantity(75.0);
        let json = serde_json::to_string(&seam).unwrap();
        let recovered: CrystalSeam = serde_json::from_str(&json).unwrap();
        assert_eq!(seam.id, recovered.id);
        assert!((seam.quality - recovered.quality).abs() < f32::EPSILON);
    }

    #[test]
    fn serde_mineral_type() {
        let mt = MineralType::Uranium;
        let json = serde_json::to_string(&mt).unwrap();
        let recovered: MineralType = serde_json::from_str(&json).unwrap();
        assert_eq!(mt, recovered);
    }

    #[test]
    fn serde_mineral_deposit() {
        let mut deposit = test_deposit().with_concentration(0.4).with_quantity(800.0);
        deposit.discover();

        let json = serde_json::to_string(&deposit).unwrap();
        let recovered: MineralDeposit = serde_json::from_str(&json).unwrap();
        assert_eq!(deposit.id, recovered.id);
        assert!(recovered.is_discovered());
    }
}
