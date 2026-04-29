//! Modular equipment framework for environmental suit systems.
//!
//! Provides tanks, heaters, filters, pressure gear, grapples, and other
//! specialized equipment for hazardous environment survival.

use serde::{Deserialize, Serialize};

/// Unique identifier for an equipment module.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ModuleId(pub u32);

impl ModuleId {
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn raw(&self) -> u32 {
        self.0
    }
}

/// Category of equipment module.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum ModuleCategory {
    /// Life support tank (oxygen, fuel, coolant).
    Tank = 0,
    /// Temperature regulation system.
    Heater = 1,
    /// Atmosphere filtration system.
    Filter = 2,
    /// Pressure protection gear.
    PressureGear = 3,
    /// Mobility grapple system.
    Grapple = 4,
    /// Integrated suit system.
    Suit = 5,
    /// Power supply module.
    Power = 6,
    /// Sensor/detection module.
    Sensor = 7,
}

impl ModuleCategory {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Tank => "tank",
            Self::Heater => "heater",
            Self::Filter => "filter",
            Self::PressureGear => "pressure_gear",
            Self::Grapple => "grapple",
            Self::Suit => "suit",
            Self::Power => "power",
            Self::Sensor => "sensor",
        }
    }

    #[must_use]
    pub const fn all() -> &'static [ModuleCategory] {
        &[
            Self::Tank,
            Self::Heater,
            Self::Filter,
            Self::PressureGear,
            Self::Grapple,
            Self::Suit,
            Self::Power,
            Self::Sensor,
        ]
    }

    #[must_use]
    pub const fn requires_power(&self) -> bool {
        matches!(
            self,
            Self::Heater | Self::Filter | Self::Grapple | Self::Sensor
        )
    }
}

/// Tank content type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum TankContent {
    Oxygen = 0,
    Fuel = 1,
    Coolant = 2,
    Water = 3,
    Nitrogen = 4,
}

impl TankContent {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Oxygen => "oxygen",
            Self::Fuel => "fuel",
            Self::Coolant => "coolant",
            Self::Water => "water",
            Self::Nitrogen => "nitrogen",
        }
    }
}

/// Filter type for atmosphere filtration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum FilterType {
    Particulate = 0,
    Toxic = 1,
    Radiation = 2,
    Biological = 3,
}

impl FilterType {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Particulate => "particulate",
            Self::Toxic => "toxic",
            Self::Radiation => "radiation",
            Self::Biological => "biological",
        }
    }
}

/// Grapple type for mobility systems.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum GrappleType {
    Magnetic = 0,
    Hook = 1,
    Pneumatic = 2,
    Tether = 3,
}

impl GrappleType {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Magnetic => "magnetic",
            Self::Hook => "hook",
            Self::Pneumatic => "pneumatic",
            Self::Tether => "tether",
        }
    }

    #[must_use]
    pub const fn max_range(&self) -> f32 {
        match self {
            Self::Magnetic => 5.0,
            Self::Hook => 15.0,
            Self::Pneumatic => 8.0,
            Self::Tether => 25.0,
        }
    }
}

/// Module quality tier affecting stats.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum ModuleTier {
    Basic = 0,
    Standard = 1,
    Advanced = 2,
    Elite = 3,
}

impl ModuleTier {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Standard => "standard",
            Self::Advanced => "advanced",
            Self::Elite => "elite",
        }
    }

    #[must_use]
    pub const fn efficiency_multiplier(&self) -> f32 {
        match self {
            Self::Basic => 1.0,
            Self::Standard => 1.25,
            Self::Advanced => 1.5,
            Self::Elite => 2.0,
        }
    }

    #[must_use]
    pub const fn durability_multiplier(&self) -> f32 {
        match self {
            Self::Basic => 1.0,
            Self::Standard => 1.5,
            Self::Advanced => 2.0,
            Self::Elite => 3.0,
        }
    }
}

/// Module-specific configuration data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ModuleConfig {
    Tank {
        content: TankContent,
        capacity: f32,
    },
    Heater {
        min_temp: f32,
        max_temp: f32,
        power_draw: f32,
    },
    Filter {
        filter_type: FilterType,
        efficiency: f32,
        power_draw: f32,
    },
    PressureGear {
        min_pressure: f32,
        max_pressure: f32,
    },
    Grapple {
        grapple_type: GrappleType,
        strength: f32,
        power_draw: f32,
    },
    Suit {
        armor: f32,
        thermal_insulation: f32,
        radiation_shielding: f32,
    },
    Power {
        capacity: f32,
        recharge_rate: f32,
    },
    Sensor {
        range: f32,
        power_draw: f32,
    },
}

impl ModuleConfig {
    #[must_use]
    pub fn category(&self) -> ModuleCategory {
        match self {
            Self::Tank { .. } => ModuleCategory::Tank,
            Self::Heater { .. } => ModuleCategory::Heater,
            Self::Filter { .. } => ModuleCategory::Filter,
            Self::PressureGear { .. } => ModuleCategory::PressureGear,
            Self::Grapple { .. } => ModuleCategory::Grapple,
            Self::Suit { .. } => ModuleCategory::Suit,
            Self::Power { .. } => ModuleCategory::Power,
            Self::Sensor { .. } => ModuleCategory::Sensor,
        }
    }

    #[must_use]
    pub fn power_draw(&self) -> f32 {
        match self {
            Self::Heater { power_draw, .. }
            | Self::Filter { power_draw, .. }
            | Self::Grapple { power_draw, .. }
            | Self::Sensor { power_draw, .. } => *power_draw,
            _ => 0.0,
        }
    }

    #[must_use]
    pub fn oxygen_tank(capacity: f32) -> Self {
        Self::Tank {
            content: TankContent::Oxygen,
            capacity,
        }
    }

    #[must_use]
    pub fn fuel_tank(capacity: f32) -> Self {
        Self::Tank {
            content: TankContent::Fuel,
            capacity,
        }
    }

    #[must_use]
    pub fn coolant_tank(capacity: f32) -> Self {
        Self::Tank {
            content: TankContent::Coolant,
            capacity,
        }
    }

    #[must_use]
    pub fn basic_heater() -> Self {
        Self::Heater {
            min_temp: -50.0,
            max_temp: 50.0,
            power_draw: 5.0,
        }
    }

    #[must_use]
    pub fn toxic_filter(efficiency: f32) -> Self {
        Self::Filter {
            filter_type: FilterType::Toxic,
            efficiency,
            power_draw: 3.0,
        }
    }

    #[must_use]
    pub fn magnetic_grapple() -> Self {
        Self::Grapple {
            grapple_type: GrappleType::Magnetic,
            strength: 100.0,
            power_draw: 10.0,
        }
    }

    #[must_use]
    pub fn hook_grapple() -> Self {
        Self::Grapple {
            grapple_type: GrappleType::Hook,
            strength: 150.0,
            power_draw: 0.0,
        }
    }
}

impl Default for ModuleConfig {
    fn default() -> Self {
        Self::Tank {
            content: TankContent::Oxygen,
            capacity: 100.0,
        }
    }
}

/// Runtime state for a module's consumable resource.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResourceState {
    pub current: f32,
    pub max: f32,
}

impl ResourceState {
    #[must_use]
    pub const fn new(max: f32) -> Self {
        Self { current: max, max }
    }

    #[must_use]
    pub const fn empty(max: f32) -> Self {
        Self { current: 0.0, max }
    }

    #[must_use]
    pub fn ratio(&self) -> f32 {
        if self.max <= 0.0 {
            0.0
        } else {
            (self.current / self.max).clamp(0.0, 1.0)
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.current <= 0.0
    }

    #[must_use]
    pub fn is_full(&self) -> bool {
        self.current >= self.max
    }

    pub fn consume(&mut self, amount: f32) -> f32 {
        let taken = amount.min(self.current);
        self.current -= taken;
        taken
    }

    pub fn refill(&mut self, amount: f32) -> f32 {
        let space = self.max - self.current;
        let added = amount.min(space);
        self.current += added;
        added
    }
}

impl Default for ResourceState {
    fn default() -> Self {
        Self::new(100.0)
    }
}

/// Module operational status.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[repr(u8)]
pub enum ModuleStatus {
    #[default]
    Active = 0,
    Standby = 1,
    Depleted = 2,
    Damaged = 3,
    Disabled = 4,
}

impl ModuleStatus {
    #[must_use]
    pub const fn is_functional(&self) -> bool {
        matches!(self, Self::Active | Self::Standby)
    }

    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }
}

/// An equipment module instance with state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EquipmentModule {
    pub id: ModuleId,
    pub tier: ModuleTier,
    pub config: ModuleConfig,
    pub status: ModuleStatus,
    pub durability: ResourceState,
    pub resource: Option<ResourceState>,
    pub last_tick: u64,
}

impl EquipmentModule {
    #[must_use]
    pub fn new(id: ModuleId, tier: ModuleTier, config: ModuleConfig) -> Self {
        let base_durability = 100.0 * tier.durability_multiplier();
        let resource = match &config {
            ModuleConfig::Tank { capacity, .. } | ModuleConfig::Power { capacity, .. } => {
                Some(ResourceState::new(*capacity))
            }
            _ => None,
        };

        Self {
            id,
            tier,
            config,
            status: ModuleStatus::Active,
            durability: ResourceState::new(base_durability),
            resource,
            last_tick: 0,
        }
    }

    #[must_use]
    pub fn category(&self) -> ModuleCategory {
        self.config.category()
    }

    #[must_use]
    pub fn effective_power_draw(&self) -> f32 {
        if self.status.is_active() {
            self.config.power_draw() / self.tier.efficiency_multiplier()
        } else {
            0.0
        }
    }

    #[must_use]
    pub fn is_functional(&self) -> bool {
        self.status.is_functional() && !self.durability.is_empty()
    }

    pub fn activate(&mut self) {
        if self.durability.is_empty() {
            self.status = ModuleStatus::Damaged;
        } else if self.resource.as_ref().is_some_and(ResourceState::is_empty) {
            self.status = ModuleStatus::Depleted;
        } else {
            self.status = ModuleStatus::Active;
        }
    }

    pub fn deactivate(&mut self) {
        if self.status == ModuleStatus::Active {
            self.status = ModuleStatus::Standby;
        }
    }

    pub fn damage(&mut self, amount: f32) {
        self.durability.consume(amount);
        if self.durability.is_empty() {
            self.status = ModuleStatus::Damaged;
        }
    }

    pub fn repair(&mut self, amount: f32) {
        self.durability.refill(amount);
        if self.status == ModuleStatus::Damaged && !self.durability.is_empty() {
            self.status = ModuleStatus::Standby;
        }
    }

    pub fn tick(&mut self, tick: u64, power_available: f32) -> ModuleTickResult {
        if tick <= self.last_tick {
            return ModuleTickResult::default();
        }
        self.last_tick = tick;

        if !self.is_functional() {
            return ModuleTickResult::default();
        }

        let power_needed = self.effective_power_draw();
        if power_needed > power_available {
            self.status = ModuleStatus::Standby;
            return ModuleTickResult {
                power_consumed: 0.0,
                resource_consumed: 0.0,
                effect: ModuleEffect::None,
            };
        }

        let (effect, resource_consumed) = self.compute_effect();
        ModuleTickResult {
            power_consumed: power_needed,
            resource_consumed,
            effect,
        }
    }

    fn compute_effect(&mut self) -> (ModuleEffect, f32) {
        let eff = self.tier.efficiency_multiplier();
        match &self.config {
            ModuleConfig::Tank { content, .. } => self.tick_tank(*content, eff),
            ModuleConfig::Heater {
                min_temp, max_temp, ..
            } => (
                ModuleEffect::TemperatureRegulation {
                    min: *min_temp,
                    max: *max_temp,
                    strength: eff,
                },
                0.0,
            ),
            ModuleConfig::Filter {
                filter_type,
                efficiency,
                ..
            } => (
                ModuleEffect::Filtration {
                    filter_type: *filter_type,
                    efficiency: *efficiency * eff,
                },
                0.0,
            ),
            ModuleConfig::PressureGear {
                min_pressure,
                max_pressure,
            } => (
                ModuleEffect::PressureProtection {
                    min: *min_pressure,
                    max: *max_pressure,
                },
                0.0,
            ),
            ModuleConfig::Grapple {
                grapple_type,
                strength,
                ..
            } => (
                ModuleEffect::GrappleReady {
                    grapple_type: *grapple_type,
                    strength: *strength * eff,
                    range: grapple_type.max_range(),
                },
                0.0,
            ),
            ModuleConfig::Suit {
                armor,
                thermal_insulation,
                radiation_shielding,
            } => (
                ModuleEffect::Protection {
                    armor: *armor * eff,
                    thermal: *thermal_insulation * eff,
                    radiation: *radiation_shielding * eff,
                },
                0.0,
            ),
            ModuleConfig::Power {
                recharge_rate,
                capacity,
            } => self.tick_power(*recharge_rate, *capacity),
            ModuleConfig::Sensor { range, .. } => (
                ModuleEffect::Detection {
                    range: *range * eff,
                },
                0.0,
            ),
        }
    }

    fn tick_tank(&mut self, content: TankContent, eff: f32) -> (ModuleEffect, f32) {
        if let Some(ref mut res) = self.resource {
            let consumed = res.consume(0.1 * eff);
            let effect = if consumed > 0.0 {
                ModuleEffect::Supply {
                    content,
                    amount: consumed,
                }
            } else {
                ModuleEffect::None
            };
            if res.is_empty() {
                self.status = ModuleStatus::Depleted;
            }
            (effect, consumed)
        } else {
            (ModuleEffect::None, 0.0)
        }
    }

    fn tick_power(&mut self, recharge_rate: f32, capacity: f32) -> (ModuleEffect, f32) {
        if let Some(ref mut res) = self.resource {
            let recharged = res.refill(recharge_rate);
            (
                ModuleEffect::PowerSupply {
                    available: res.current,
                    capacity,
                    recharged,
                },
                0.0,
            )
        } else {
            (ModuleEffect::None, 0.0)
        }
    }

    fn sort_key(&self) -> (u8, u32) {
        (self.config.category() as u8, self.id.0)
    }
}

impl PartialOrd for EquipmentModule {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EquipmentModule {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

impl Eq for EquipmentModule {}

/// Result of a module tick.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ModuleTickResult {
    pub power_consumed: f32,
    pub resource_consumed: f32,
    pub effect: ModuleEffect,
}

/// Effect produced by an active module.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum ModuleEffect {
    #[default]
    None,
    Supply {
        content: TankContent,
        amount: f32,
    },
    TemperatureRegulation {
        min: f32,
        max: f32,
        strength: f32,
    },
    Filtration {
        filter_type: FilterType,
        efficiency: f32,
    },
    PressureProtection {
        min: f32,
        max: f32,
    },
    GrappleReady {
        grapple_type: GrappleType,
        strength: f32,
        range: f32,
    },
    Protection {
        armor: f32,
        thermal: f32,
        radiation: f32,
    },
    PowerSupply {
        available: f32,
        capacity: f32,
        recharged: f32,
    },
    Detection {
        range: f32,
    },
}

/// Maximum number of equipment modules in a loadout.
pub const MAX_MODULES: usize = 8;

/// A complete equipment loadout containing multiple modules.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EquipmentLoadout {
    modules: Vec<EquipmentModule>,
}

impl EquipmentLoadout {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn module_count(&self) -> usize {
        self.modules.len()
    }

    #[must_use]
    pub fn is_full(&self) -> bool {
        self.modules.len() >= MAX_MODULES
    }

    #[must_use]
    pub fn get(&self, id: ModuleId) -> Option<&EquipmentModule> {
        self.modules.iter().find(|m| m.id == id)
    }

    pub fn get_mut(&mut self, id: ModuleId) -> Option<&mut EquipmentModule> {
        self.modules.iter_mut().find(|m| m.id == id)
    }

    #[must_use]
    pub fn by_category(&self, category: ModuleCategory) -> Vec<&EquipmentModule> {
        self.modules
            .iter()
            .filter(|m| m.category() == category)
            .collect()
    }

    /// # Errors
    /// Returns `LoadoutError::Full` if the loadout has reached `MAX_MODULES`.
    /// Returns `LoadoutError::DuplicateId` if a module with the same ID exists.
    pub fn install(&mut self, module: EquipmentModule) -> Result<(), LoadoutError> {
        if self.is_full() {
            return Err(LoadoutError::Full);
        }
        if self.modules.iter().any(|m| m.id == module.id) {
            return Err(LoadoutError::DuplicateId);
        }
        self.modules.push(module);
        self.modules.sort();
        Ok(())
    }

    pub fn uninstall(&mut self, id: ModuleId) -> Option<EquipmentModule> {
        if let Some(idx) = self.modules.iter().position(|m| m.id == id) {
            Some(self.modules.remove(idx))
        } else {
            None
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &EquipmentModule> {
        self.modules.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut EquipmentModule> {
        self.modules.iter_mut()
    }

    #[must_use]
    pub fn total_power_draw(&self) -> f32 {
        self.modules
            .iter()
            .map(EquipmentModule::effective_power_draw)
            .sum()
    }

    #[must_use]
    pub fn power_capacity(&self) -> f32 {
        self.modules
            .iter()
            .filter(|m| matches!(m.config, ModuleConfig::Power { .. }))
            .filter_map(|m| m.resource.as_ref())
            .map(|r| r.max)
            .sum()
    }

    #[must_use]
    pub fn available_power(&self) -> f32 {
        self.modules
            .iter()
            .filter(|m| matches!(m.config, ModuleConfig::Power { .. }))
            .filter_map(|m| m.resource.as_ref())
            .map(|r| r.current)
            .sum()
    }

    pub fn tick(&mut self, tick: u64) -> LoadoutTickResult {
        let mut power_pool = self.available_power();
        let mut results = Vec::with_capacity(self.modules.len());
        let mut total_power = 0.0;

        for module in &mut self.modules {
            if matches!(module.config, ModuleConfig::Power { .. }) {
                continue;
            }

            let result = module.tick(tick, power_pool);
            power_pool -= result.power_consumed;
            total_power += result.power_consumed;
            results.push((module.id, result));
        }

        for module in &mut self.modules {
            if matches!(module.config, ModuleConfig::Power { .. }) {
                let _ = module.tick(tick, f32::MAX);
            }
        }

        for module in &mut self.modules {
            if let ModuleConfig::Power { .. } = &module.config
                && let Some(ref mut res) = module.resource
            {
                res.consume(total_power);
            }
        }

        LoadoutTickResult {
            power_consumed: total_power,
            module_results: results,
        }
    }

    #[must_use]
    pub fn fingerprint(&self) -> EquipmentFingerprint {
        let mut hasher = crc32fast::Hasher::new();

        let len = u32::try_from(self.modules.len()).unwrap_or(u32::MAX);
        hasher.update(&len.to_le_bytes());

        for module in &self.modules {
            hasher.update(&module.id.0.to_le_bytes());
            hasher.update(&(module.tier as u8).to_le_bytes());
            hasher.update(&(module.status as u8).to_le_bytes());
            hasher.update(&module.durability.current.to_le_bytes());
            if let Some(ref res) = module.resource {
                hasher.update(&[1u8]);
                hasher.update(&res.current.to_le_bytes());
            } else {
                hasher.update(&[0u8]);
            }
        }

        EquipmentFingerprint(hasher.finalize())
    }
}

impl PartialEq for EquipmentLoadout {
    fn eq(&self, other: &Self) -> bool {
        self.modules == other.modules
    }
}

impl Eq for EquipmentLoadout {}

/// Result of ticking an entire loadout.
#[derive(Clone, Debug, Default)]
pub struct LoadoutTickResult {
    pub power_consumed: f32,
    pub module_results: Vec<(ModuleId, ModuleTickResult)>,
}

impl LoadoutTickResult {
    pub fn effects(&self) -> impl Iterator<Item = &ModuleEffect> {
        self.module_results
            .iter()
            .map(|(_, r)| &r.effect)
            .filter(|e| !matches!(e, ModuleEffect::None))
    }
}

/// Error when modifying a loadout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadoutError {
    Full,
    DuplicateId,
    NotFound,
}

impl std::fmt::Display for LoadoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Full => write!(f, "loadout is full"),
            Self::DuplicateId => write!(f, "module with this ID already installed"),
            Self::NotFound => write!(f, "module not found"),
        }
    }
}

impl std::error::Error for LoadoutError {}

/// Fingerprint for equipment state comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EquipmentFingerprint(pub u32);

impl EquipmentFingerprint {
    #[must_use]
    pub const fn value(&self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_oxygen_tank(id: u32) -> EquipmentModule {
        EquipmentModule::new(
            ModuleId::new(id),
            ModuleTier::Standard,
            ModuleConfig::oxygen_tank(100.0),
        )
    }

    fn make_power_module(id: u32) -> EquipmentModule {
        EquipmentModule::new(
            ModuleId::new(id),
            ModuleTier::Standard,
            ModuleConfig::Power {
                capacity: 500.0,
                recharge_rate: 1.0,
            },
        )
    }

    #[test]
    fn module_creation() {
        let module = make_oxygen_tank(1);
        assert_eq!(module.category(), ModuleCategory::Tank);
        assert!(module.is_functional());
        assert_eq!(module.tier, ModuleTier::Standard);
    }

    #[test]
    fn module_damage_and_repair() {
        let mut module = make_oxygen_tank(1);
        let initial = module.durability.current;

        module.damage(50.0);
        assert!(module.durability.current < initial);
        assert!(module.is_functional());

        module.damage(initial);
        assert_eq!(module.status, ModuleStatus::Damaged);
        assert!(!module.is_functional());

        module.repair(50.0);
        assert_eq!(module.status, ModuleStatus::Standby);
    }

    #[test]
    fn module_activate_deactivate() {
        let mut module = make_oxygen_tank(1);

        module.deactivate();
        assert_eq!(module.status, ModuleStatus::Standby);

        module.activate();
        assert_eq!(module.status, ModuleStatus::Active);
    }

    #[test]
    fn resource_state() {
        let mut res = ResourceState::new(100.0);
        assert!(res.is_full());
        assert!(!res.is_empty());
        assert!((res.ratio() - 1.0).abs() < 0.001);

        let taken = res.consume(30.0);
        assert!((taken - 30.0).abs() < 0.001);
        assert!((res.ratio() - 0.7).abs() < 0.001);

        let added = res.refill(50.0);
        assert!((added - 30.0).abs() < 0.001);
        assert!(res.is_full());
    }

    #[test]
    fn loadout_install_uninstall() {
        let mut loadout = EquipmentLoadout::new();

        loadout.install(make_oxygen_tank(1)).unwrap();
        assert_eq!(loadout.module_count(), 1);

        let result = loadout.install(make_oxygen_tank(1));
        assert_eq!(result, Err(LoadoutError::DuplicateId));

        loadout.install(make_oxygen_tank(2)).unwrap();
        assert_eq!(loadout.module_count(), 2);

        let removed = loadout.uninstall(ModuleId::new(1));
        assert!(removed.is_some());
        assert_eq!(loadout.module_count(), 1);
    }

    #[test]
    fn loadout_full() {
        let mut loadout = EquipmentLoadout::new();

        for i in 0..MAX_MODULES {
            loadout
                .install(make_oxygen_tank(u32::try_from(i).unwrap()))
                .unwrap();
        }

        assert!(loadout.is_full());
        let result = loadout.install(make_oxygen_tank(100));
        assert_eq!(result, Err(LoadoutError::Full));
    }

    #[test]
    fn loadout_by_category() {
        let mut loadout = EquipmentLoadout::new();
        loadout.install(make_oxygen_tank(1)).unwrap();
        loadout.install(make_oxygen_tank(2)).unwrap();
        loadout.install(make_power_module(3)).unwrap();

        let tanks = loadout.by_category(ModuleCategory::Tank);
        assert_eq!(tanks.len(), 2);

        let power = loadout.by_category(ModuleCategory::Power);
        assert_eq!(power.len(), 1);
    }

    #[test]
    fn loadout_power_tracking() {
        let mut loadout = EquipmentLoadout::new();
        loadout.install(make_power_module(1)).unwrap();

        assert!((loadout.power_capacity() - 500.0).abs() < 0.001);
        assert!((loadout.available_power() - 500.0).abs() < 0.001);
    }

    #[test]
    fn module_ordering() {
        let tank = make_oxygen_tank(1);
        let power = make_power_module(2);

        assert!(tank < power);
    }

    #[test]
    fn module_tick_consumes_resource() {
        let mut module = make_oxygen_tank(1);
        let initial = module.resource.as_ref().unwrap().current;

        let result = module.tick(1, f32::MAX);
        let after = module.resource.as_ref().unwrap().current;

        assert!(result.resource_consumed > 0.0);
        assert!(after < initial);
    }

    #[test]
    fn heater_effect() {
        let mut module = EquipmentModule::new(
            ModuleId::new(1),
            ModuleTier::Advanced,
            ModuleConfig::basic_heater(),
        );

        let result = module.tick(1, 100.0);

        match result.effect {
            ModuleEffect::TemperatureRegulation { min, max, strength } => {
                assert!((min - -50.0).abs() < 0.001);
                assert!((max - 50.0).abs() < 0.001);
                assert!((strength - 1.5).abs() < 0.001);
            }
            _ => panic!("unexpected effect"),
        }
    }

    #[test]
    fn grapple_types() {
        assert!((GrappleType::Magnetic.max_range() - 5.0).abs() < 0.001);
        assert!((GrappleType::Tether.max_range() - 25.0).abs() < 0.001);
    }

    #[test]
    fn tier_multipliers() {
        assert!((ModuleTier::Basic.efficiency_multiplier() - 1.0).abs() < 0.001);
        assert!((ModuleTier::Elite.efficiency_multiplier() - 2.0).abs() < 0.001);
        assert!((ModuleTier::Elite.durability_multiplier() - 3.0).abs() < 0.001);
    }

    #[test]
    fn fingerprint_deterministic() {
        let mut loadout1 = EquipmentLoadout::new();
        loadout1.install(make_oxygen_tank(1)).unwrap();
        loadout1.install(make_power_module(2)).unwrap();

        let mut loadout2 = EquipmentLoadout::new();
        loadout2.install(make_oxygen_tank(1)).unwrap();
        loadout2.install(make_power_module(2)).unwrap();

        assert_eq!(loadout1.fingerprint(), loadout2.fingerprint());
    }

    #[test]
    fn fingerprint_changes_with_state() {
        let mut loadout = EquipmentLoadout::new();
        loadout.install(make_oxygen_tank(1)).unwrap();

        let fp1 = loadout.fingerprint();

        loadout.get_mut(ModuleId::new(1)).unwrap().damage(10.0);
        let fp2 = loadout.fingerprint();

        assert_ne!(fp1, fp2);
    }

    #[test]
    fn serde_roundtrip_module() {
        let module = make_oxygen_tank(1);
        let json = serde_json::to_string(&module).unwrap();
        let recovered: EquipmentModule = serde_json::from_str(&json).unwrap();
        assert_eq!(module, recovered);
    }

    #[test]
    fn serde_roundtrip_loadout() {
        let mut loadout = EquipmentLoadout::new();
        loadout.install(make_oxygen_tank(1)).unwrap();
        loadout.install(make_power_module(2)).unwrap();

        let json = serde_json::to_string(&loadout).unwrap();
        let recovered: EquipmentLoadout = serde_json::from_str(&json).unwrap();
        assert_eq!(loadout, recovered);
    }

    #[test]
    fn module_no_power_standby() {
        let mut module = EquipmentModule::new(
            ModuleId::new(1),
            ModuleTier::Basic,
            ModuleConfig::basic_heater(),
        );

        let result = module.tick(1, 0.0);
        assert_eq!(module.status, ModuleStatus::Standby);
        assert!((result.power_consumed - 0.0).abs() < 0.001);
    }

    #[test]
    fn filter_effect() {
        let mut module = EquipmentModule::new(
            ModuleId::new(1),
            ModuleTier::Standard,
            ModuleConfig::toxic_filter(0.8),
        );

        let result = module.tick(1, 100.0);

        match result.effect {
            ModuleEffect::Filtration {
                filter_type,
                efficiency,
            } => {
                assert_eq!(filter_type, FilterType::Toxic);
                assert!((efficiency - 1.0).abs() < 0.001);
            }
            _ => panic!("unexpected effect"),
        }
    }

    #[test]
    fn loadout_tick_distributes_power() {
        let mut loadout = EquipmentLoadout::new();
        loadout.install(make_power_module(1)).unwrap();
        loadout
            .install(EquipmentModule::new(
                ModuleId::new(2),
                ModuleTier::Basic,
                ModuleConfig::basic_heater(),
            ))
            .unwrap();

        let result = loadout.tick(1);
        assert!(result.power_consumed > 0.0);
        assert!(!result.module_results.is_empty());
    }
}
