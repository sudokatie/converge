//! Regional climate simulation with prevailing winds, moisture transport, and seasonal shifts.
//!
//! This module provides deterministic regional-scale climate simulation supporting:
//! - Prevailing wind vectors per region
//! - Moisture transport and advection between neighboring regions
//! - Precipitation and evaporation balance
//! - Temperature, humidity, and pressure evolution over seasons
//! - Biome, altitude, and latitude modifiers
//! - Unloaded-region friendly snapshots
//! - Stable fingerprints and checksums for replay verification

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::replay::{ChecksumBuilder, StepChecksum};
use crate::world_state::Season;

/// Minimum valid temperature in Celsius.
pub const MIN_TEMPERATURE: f32 = -80.0;

/// Maximum valid temperature in Celsius.
pub const MAX_TEMPERATURE: f32 = 60.0;

/// Minimum valid humidity (0 to 1).
pub const MIN_HUMIDITY: f32 = 0.0;

/// Maximum valid humidity (0 to 1).
pub const MAX_HUMIDITY: f32 = 1.0;

/// Minimum valid pressure in atmospheres.
pub const MIN_PRESSURE: f32 = 0.5;

/// Maximum valid pressure in atmospheres.
pub const MAX_PRESSURE: f32 = 1.5;

/// Minimum valid moisture content (kg/m^2).
pub const MIN_MOISTURE: f32 = 0.0;

/// Maximum valid moisture content (kg/m^2).
pub const MAX_MOISTURE: f32 = 100.0;

/// Default base temperature in Celsius.
pub const DEFAULT_BASE_TEMPERATURE: f32 = 15.0;

/// Default base humidity.
pub const DEFAULT_BASE_HUMIDITY: f32 = 0.5;

/// Default base pressure in atmospheres.
pub const DEFAULT_BASE_PRESSURE: f32 = 1.0;

/// Default base moisture content (kg/m^2).
pub const DEFAULT_BASE_MOISTURE: f32 = 20.0;

/// Unique identifier for a climate region.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ClimateRegionId(pub u32);

impl ClimateRegionId {
    /// Create a new region ID.
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw ID value.
    #[must_use]
    pub const fn value(&self) -> u32 {
        self.0
    }
}

impl From<u32> for ClimateRegionId {
    fn from(id: u32) -> Self {
        Self::new(id)
    }
}

/// A 2D wind vector representing prevailing wind direction and strength.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindVector {
    /// East-west component (positive = eastward).
    pub x: f32,
    /// North-south component (positive = northward).
    pub y: f32,
}

impl WindVector {
    /// Create a new wind vector.
    #[must_use]
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Zero wind (calm).
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    /// Eastward wind.
    #[must_use]
    pub fn eastward(strength: f32) -> Self {
        Self::new(strength, 0.0)
    }

    /// Westward wind.
    #[must_use]
    pub fn westward(strength: f32) -> Self {
        Self::new(-strength, 0.0)
    }

    /// Northward wind.
    #[must_use]
    pub fn northward(strength: f32) -> Self {
        Self::new(0.0, strength)
    }

    /// Southward wind.
    #[must_use]
    pub fn southward(strength: f32) -> Self {
        Self::new(0.0, -strength)
    }

    /// Wind magnitude (speed).
    #[must_use]
    pub fn magnitude(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    /// Normalize to unit vector.
    #[must_use]
    pub fn normalize(&self) -> Self {
        let mag = self.magnitude();
        if mag < 0.001 {
            Self::ZERO
        } else {
            Self::new(self.x / mag, self.y / mag)
        }
    }

    /// Scale the wind vector.
    #[must_use]
    pub fn scale(&self, factor: f32) -> Self {
        Self::new(self.x * factor, self.y * factor)
    }

    /// Interpolate between two wind vectors.
    #[must_use]
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self::new(
            self.x + (other.x - self.x) * t,
            self.y + (other.y - self.y) * t,
        )
    }
}

impl Default for WindVector {
    fn default() -> Self {
        Self::ZERO
    }
}

/// Biome type affecting climate behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[repr(u8)]
pub enum BiomeType {
    /// Temperate forest.
    #[default]
    Temperate = 0,
    /// Hot, dry desert.
    Desert = 1,
    /// Tropical rainforest.
    Tropical = 2,
    /// Cold tundra.
    Tundra = 3,
    /// Grassland/prairie.
    Grassland = 4,
    /// Mountain/alpine.
    Mountain = 5,
    /// Coastal/oceanic.
    Coastal = 6,
    /// Arctic/polar.
    Polar = 7,
}

impl BiomeType {
    /// All biome types.
    pub const ALL: [BiomeType; 8] = [
        BiomeType::Temperate,
        BiomeType::Desert,
        BiomeType::Tropical,
        BiomeType::Tundra,
        BiomeType::Grassland,
        BiomeType::Mountain,
        BiomeType::Coastal,
        BiomeType::Polar,
    ];

    /// Temperature modifier for this biome.
    #[must_use]
    pub const fn temperature_modifier(&self) -> f32 {
        match self {
            BiomeType::Temperate | BiomeType::Coastal => 0.0,
            BiomeType::Desert => 10.0,
            BiomeType::Tropical => 8.0,
            BiomeType::Tundra => -15.0,
            BiomeType::Grassland => 2.0,
            BiomeType::Mountain => -8.0,
            BiomeType::Polar => -30.0,
        }
    }

    /// Humidity modifier for this biome.
    #[must_use]
    pub const fn humidity_modifier(&self) -> f32 {
        match self {
            BiomeType::Temperate => 0.0,
            BiomeType::Desert => -0.4,
            BiomeType::Tropical => 0.3,
            BiomeType::Tundra | BiomeType::Grassland => -0.1,
            BiomeType::Mountain => 0.1,
            BiomeType::Coastal => 0.2,
            BiomeType::Polar => -0.2,
        }
    }

    /// Evaporation rate modifier for this biome.
    #[must_use]
    pub const fn evaporation_modifier(&self) -> f32 {
        match self {
            BiomeType::Temperate => 1.0,
            BiomeType::Desert => 2.0,
            BiomeType::Tropical => 1.5,
            BiomeType::Tundra => 0.3,
            BiomeType::Grassland => 1.2,
            BiomeType::Mountain => 0.8,
            BiomeType::Coastal => 1.3,
            BiomeType::Polar => 0.1,
        }
    }

    /// As array index.
    #[must_use]
    pub const fn as_index(&self) -> usize {
        *self as usize
    }
}

/// A climate cell representing the climate state of a single region.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClimateCell {
    /// Base temperature in Celsius (before seasonal modifiers).
    base_temperature: f32,
    /// Current temperature in Celsius.
    temperature: f32,
    /// Humidity (0 to 1).
    humidity: f32,
    /// Atmospheric pressure in atmospheres.
    pressure: f32,
    /// Moisture content (kg/m^2).
    moisture: f32,
    /// Prevailing wind vector.
    wind: WindVector,
    /// Biome type.
    biome: BiomeType,
    /// Altitude in meters (affects temperature and pressure).
    altitude: f32,
    /// Latitude (-90 to 90, affects seasonal variation).
    latitude: f32,
}

impl ClimateCell {
    /// Create a new climate cell with validation.
    #[must_use]
    #[expect(clippy::too_many_arguments, reason = "constructor for domain struct")]
    pub fn new(
        base_temperature: f32,
        humidity: f32,
        pressure: f32,
        moisture: f32,
        wind: WindVector,
        biome: BiomeType,
        altitude: f32,
        latitude: f32,
    ) -> Self {
        Self {
            base_temperature: base_temperature.clamp(MIN_TEMPERATURE, MAX_TEMPERATURE),
            temperature: base_temperature.clamp(MIN_TEMPERATURE, MAX_TEMPERATURE),
            humidity: humidity.clamp(MIN_HUMIDITY, MAX_HUMIDITY),
            pressure: pressure.clamp(MIN_PRESSURE, MAX_PRESSURE),
            moisture: moisture.clamp(MIN_MOISTURE, MAX_MOISTURE),
            wind,
            biome,
            altitude: altitude.max(0.0),
            latitude: latitude.clamp(-90.0, 90.0),
        }
    }

    /// Create a default temperate climate cell.
    #[must_use]
    pub fn temperate() -> Self {
        Self::new(
            DEFAULT_BASE_TEMPERATURE,
            DEFAULT_BASE_HUMIDITY,
            DEFAULT_BASE_PRESSURE,
            DEFAULT_BASE_MOISTURE,
            WindVector::westward(5.0),
            BiomeType::Temperate,
            100.0,
            45.0,
        )
    }

    /// Create a desert climate cell.
    #[must_use]
    pub fn desert() -> Self {
        Self::new(
            25.0,
            0.15,
            DEFAULT_BASE_PRESSURE,
            5.0,
            WindVector::eastward(8.0),
            BiomeType::Desert,
            500.0,
            25.0,
        )
    }

    /// Create a tropical climate cell.
    #[must_use]
    pub fn tropical() -> Self {
        Self::new(
            28.0,
            0.85,
            DEFAULT_BASE_PRESSURE,
            60.0,
            WindVector::eastward(3.0),
            BiomeType::Tropical,
            50.0,
            5.0,
        )
    }

    /// Create a polar climate cell.
    #[must_use]
    pub fn polar() -> Self {
        Self::new(
            -25.0,
            0.3,
            1.05,
            10.0,
            WindVector::eastward(15.0),
            BiomeType::Polar,
            500.0,
            80.0,
        )
    }

    /// Get base temperature.
    #[must_use]
    pub const fn base_temperature(&self) -> f32 {
        self.base_temperature
    }

    /// Get current temperature.
    #[must_use]
    pub const fn temperature(&self) -> f32 {
        self.temperature
    }

    /// Get humidity.
    #[must_use]
    pub const fn humidity(&self) -> f32 {
        self.humidity
    }

    /// Get pressure.
    #[must_use]
    pub const fn pressure(&self) -> f32 {
        self.pressure
    }

    /// Get moisture content.
    #[must_use]
    pub const fn moisture(&self) -> f32 {
        self.moisture
    }

    /// Get wind vector.
    #[must_use]
    pub const fn wind(&self) -> WindVector {
        self.wind
    }

    /// Get biome type.
    #[must_use]
    pub const fn biome(&self) -> BiomeType {
        self.biome
    }

    /// Get altitude.
    #[must_use]
    pub const fn altitude(&self) -> f32 {
        self.altitude
    }

    /// Get latitude.
    #[must_use]
    pub const fn latitude(&self) -> f32 {
        self.latitude
    }

    /// Set temperature with clamping.
    pub fn set_temperature(&mut self, temp: f32) {
        self.temperature = temp.clamp(MIN_TEMPERATURE, MAX_TEMPERATURE);
    }

    /// Set humidity with clamping.
    pub fn set_humidity(&mut self, humidity: f32) {
        self.humidity = humidity.clamp(MIN_HUMIDITY, MAX_HUMIDITY);
    }

    /// Set pressure with clamping.
    pub fn set_pressure(&mut self, pressure: f32) {
        self.pressure = pressure.clamp(MIN_PRESSURE, MAX_PRESSURE);
    }

    /// Set moisture with clamping.
    pub fn set_moisture(&mut self, moisture: f32) {
        self.moisture = moisture.clamp(MIN_MOISTURE, MAX_MOISTURE);
    }

    /// Set wind vector.
    pub fn set_wind(&mut self, wind: WindVector) {
        self.wind = wind;
    }

    /// Apply moisture delta with clamping.
    pub fn apply_moisture_delta(&mut self, delta: f32) {
        self.set_moisture(self.moisture + delta);
    }

    /// Compute effective temperature including altitude effect.
    #[must_use]
    pub fn effective_temperature(&self) -> f32 {
        let altitude_effect = -0.0065 * self.altitude;
        (self.temperature + altitude_effect).clamp(MIN_TEMPERATURE, MAX_TEMPERATURE)
    }

    /// Compute saturation moisture based on temperature.
    #[must_use]
    pub fn saturation_moisture(&self) -> f32 {
        let t = self.effective_temperature();
        let base = 10.0 + (t + 20.0) * 0.5;
        base.clamp(5.0, MAX_MOISTURE)
    }

    /// Compute precipitation potential (excess moisture above saturation).
    #[must_use]
    pub fn precipitation_potential(&self) -> f32 {
        let saturation = self.saturation_moisture();
        (self.moisture - saturation * self.humidity).max(0.0)
    }

    /// Check if precipitation is likely.
    #[must_use]
    pub fn will_precipitate(&self) -> bool {
        self.precipitation_potential() > 1.0
    }

    /// Compute seasonal variation factor based on latitude.
    #[must_use]
    pub fn seasonal_variation(&self) -> f32 {
        (self.latitude.abs() / 90.0).clamp(0.0, 1.0)
    }
}

impl Default for ClimateCell {
    fn default() -> Self {
        Self::temperate()
    }
}

/// Seasonal cycle configuration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SeasonalCycle {
    /// Current season.
    pub season: Season,
    /// Progress through current season (0.0 to 1.0).
    pub progress: f32,
    /// Ticks per season.
    pub ticks_per_season: u64,
    /// Current tick within the year.
    pub year_tick: u64,
}

impl SeasonalCycle {
    /// Create a new seasonal cycle.
    #[must_use]
    pub fn new(ticks_per_season: u64) -> Self {
        Self {
            season: Season::Spring,
            progress: 0.0,
            ticks_per_season,
            year_tick: 0,
        }
    }

    /// Advance the cycle by one tick.
    #[expect(
        clippy::cast_precision_loss,
        reason = "progress ratio 0.0-1.0 is fine with f32 precision"
    )]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "season_index bounded by Season::COUNT (4), fits in usize"
    )]
    pub fn tick(&mut self) {
        self.year_tick = self.year_tick.saturating_add(1);
        let total_year_ticks = self.ticks_per_season * Season::COUNT as u64;
        self.year_tick %= total_year_ticks;

        let season_index = (self.year_tick / self.ticks_per_season) % (Season::COUNT as u64);
        self.season = Season::from_index(season_index as usize).unwrap_or(Season::Spring);
        self.progress =
            (self.year_tick % self.ticks_per_season) as f32 / self.ticks_per_season as f32;
    }

    /// Get temperature modifier for current season and progress.
    #[must_use]
    pub fn temperature_modifier(&self) -> f32 {
        let current = self.season.temperature_modifier();
        let next = self.season.next().temperature_modifier();
        current + (next - current) * self.progress
    }

    /// Set to a specific season.
    pub fn set_season(&mut self, season: Season) {
        self.season = season;
        self.progress = 0.0;
        self.year_tick = (season.as_index() as u64) * self.ticks_per_season;
    }
}

impl Default for SeasonalCycle {
    fn default() -> Self {
        Self::new(86400)
    }
}

/// A planned moisture transport between two regions.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MoistureTransport {
    /// Source region ID.
    pub source: ClimateRegionId,
    /// Destination region ID.
    pub dest: ClimateRegionId,
    /// Moisture amount to transfer (kg/m^2).
    pub amount: f32,
    /// Wind influence factor (0.0 to 1.0).
    pub wind_factor: f32,
}

impl MoistureTransport {
    /// Create a new moisture transport.
    #[must_use]
    pub fn new(
        source: ClimateRegionId,
        dest: ClimateRegionId,
        amount: f32,
        wind_factor: f32,
    ) -> Self {
        Self {
            source,
            dest,
            amount: amount.max(0.0),
            wind_factor: wind_factor.clamp(0.0, 1.0),
        }
    }

    /// Compute effective transfer amount.
    #[must_use]
    pub fn effective_amount(&self) -> f32 {
        self.amount * self.wind_factor
    }

    /// Sort key for deterministic ordering.
    #[must_use]
    pub fn sort_key(&self) -> (u32, u32) {
        (self.source.0, self.dest.0)
    }
}

/// A precipitation event in a region.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PrecipitationEvent {
    /// Region where precipitation occurs.
    pub region: ClimateRegionId,
    /// Amount of moisture precipitated (kg/m^2).
    pub amount: f32,
    /// Whether this is frozen precipitation (snow/sleet).
    pub frozen: bool,
}

impl PrecipitationEvent {
    /// Create a new precipitation event.
    #[must_use]
    pub fn new(region: ClimateRegionId, amount: f32, frozen: bool) -> Self {
        Self {
            region,
            amount: amount.max(0.0),
            frozen,
        }
    }

    /// Sort key for deterministic ordering.
    #[must_use]
    pub fn sort_key(&self) -> u32 {
        self.region.0
    }
}

/// Configuration for regional climate simulation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RegionalClimateConfig {
    /// Minimum moisture differential for transport.
    pub min_moisture_diff: f32,
    /// Base moisture transport rate.
    pub transport_rate: f32,
    /// Wind influence on transport direction.
    pub wind_influence: f32,
    /// Evaporation rate multiplier.
    pub evaporation_rate: f32,
    /// Precipitation threshold (moisture above saturation).
    pub precipitation_threshold: f32,
    /// Temperature equilibration rate with neighbors.
    pub temperature_equilibration: f32,
    /// Seasonal temperature amplitude.
    pub seasonal_amplitude: f32,
    /// Enable cross-region moisture transport.
    pub transport_enabled: bool,
}

impl RegionalClimateConfig {
    /// Default configuration.
    pub const DEFAULT: Self = Self {
        min_moisture_diff: 1.0,
        transport_rate: 0.1,
        wind_influence: 0.5,
        evaporation_rate: 0.02,
        precipitation_threshold: 5.0,
        temperature_equilibration: 0.01,
        seasonal_amplitude: 15.0,
        transport_enabled: true,
    };

    /// Configuration for arid/desert regions (low moisture transport).
    pub const ARID: Self = Self {
        min_moisture_diff: 2.0,
        transport_rate: 0.05,
        wind_influence: 0.7,
        evaporation_rate: 0.05,
        precipitation_threshold: 10.0,
        temperature_equilibration: 0.02,
        seasonal_amplitude: 20.0,
        transport_enabled: true,
    };

    /// Configuration for tropical regions (high moisture, low seasonal variation).
    pub const TROPICAL: Self = Self {
        min_moisture_diff: 0.5,
        transport_rate: 0.15,
        wind_influence: 0.3,
        evaporation_rate: 0.03,
        precipitation_threshold: 3.0,
        temperature_equilibration: 0.005,
        seasonal_amplitude: 5.0,
        transport_enabled: true,
    };

    /// Configuration for polar regions (low evaporation, high wind influence).
    pub const POLAR: Self = Self {
        min_moisture_diff: 0.5,
        transport_rate: 0.08,
        wind_influence: 0.8,
        evaporation_rate: 0.005,
        precipitation_threshold: 2.0,
        temperature_equilibration: 0.02,
        seasonal_amplitude: 25.0,
        transport_enabled: true,
    };

    /// Validate configuration values.
    #[must_use]
    pub fn validate(&self) -> RegionalClimateValidation {
        let mut issues = Vec::new();

        if self.min_moisture_diff < 0.0 {
            issues.push("min_moisture_diff must be non-negative".to_string());
        }
        if self.transport_rate < 0.0 || self.transport_rate > 1.0 {
            issues.push("transport_rate must be in [0, 1]".to_string());
        }
        if self.wind_influence < 0.0 || self.wind_influence > 1.0 {
            issues.push("wind_influence must be in [0, 1]".to_string());
        }
        if self.evaporation_rate < 0.0 || self.evaporation_rate > 1.0 {
            issues.push("evaporation_rate must be in [0, 1]".to_string());
        }
        if self.precipitation_threshold < 0.0 {
            issues.push("precipitation_threshold must be non-negative".to_string());
        }
        if self.temperature_equilibration < 0.0 || self.temperature_equilibration > 1.0 {
            issues.push("temperature_equilibration must be in [0, 1]".to_string());
        }
        if self.seasonal_amplitude < 0.0 {
            issues.push("seasonal_amplitude must be non-negative".to_string());
        }

        RegionalClimateValidation {
            is_valid: issues.is_empty(),
            issues,
        }
    }
}

impl Default for RegionalClimateConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Validation result for configuration or region state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RegionalClimateValidation {
    /// Whether validation passed.
    pub is_valid: bool,
    /// List of validation issues.
    pub issues: Vec<String>,
}

/// Neighbor information for a region.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RegionNeighbors {
    /// Neighbor to the north.
    pub north: Option<ClimateRegionId>,
    /// Neighbor to the south.
    pub south: Option<ClimateRegionId>,
    /// Neighbor to the east.
    pub east: Option<ClimateRegionId>,
    /// Neighbor to the west.
    pub west: Option<ClimateRegionId>,
}

impl RegionNeighbors {
    /// Create new neighbor info.
    #[must_use]
    pub fn new(
        north: Option<ClimateRegionId>,
        south: Option<ClimateRegionId>,
        east: Option<ClimateRegionId>,
        west: Option<ClimateRegionId>,
    ) -> Self {
        Self {
            north,
            south,
            east,
            west,
        }
    }

    /// Get all neighbors as an iterator.
    pub fn all(&self) -> impl Iterator<Item = ClimateRegionId> + '_ {
        [self.north, self.south, self.east, self.west]
            .into_iter()
            .flatten()
    }

    /// Count of valid neighbors.
    #[must_use]
    pub fn count(&self) -> usize {
        self.all().count()
    }

    /// Get neighbor in wind direction (returns primary destination for moisture).
    #[must_use]
    pub fn in_wind_direction(&self, wind: &WindVector) -> Option<ClimateRegionId> {
        if wind.x.abs() > wind.y.abs() {
            if wind.x > 0.0 { self.east } else { self.west }
        } else if wind.y > 0.0 {
            self.north
        } else {
            self.south
        }
    }
}

/// A climate region containing cells and neighbor relationships.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClimateRegion {
    /// Cells indexed by region ID.
    #[serde(
        serialize_with = "serialize_cells",
        deserialize_with = "deserialize_cells"
    )]
    cells: BTreeMap<ClimateRegionId, ClimateCell>,
    /// Neighbor relationships.
    #[serde(
        serialize_with = "serialize_neighbors",
        deserialize_with = "deserialize_neighbors"
    )]
    neighbors: BTreeMap<ClimateRegionId, RegionNeighbors>,
    /// Seasonal cycle state.
    seasonal_cycle: SeasonalCycle,
    /// Average temperature across all regions.
    avg_temperature: f32,
    /// Average humidity across all regions.
    avg_humidity: f32,
    /// Total moisture across all regions.
    total_moisture: f32,
}

fn serialize_cells<S>(
    cells: &BTreeMap<ClimateRegionId, ClimateCell>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeSeq;
    let mut seq = serializer.serialize_seq(Some(cells.len()))?;
    for (id, cell) in cells {
        seq.serialize_element(&(id, cell))?;
    }
    seq.end()
}

fn deserialize_cells<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<ClimateRegionId, ClimateCell>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let entries: Vec<(ClimateRegionId, ClimateCell)> = Deserialize::deserialize(deserializer)?;
    let mut map = BTreeMap::new();
    for (id, cell) in entries {
        map.insert(id, cell);
    }
    Ok(map)
}

fn serialize_neighbors<S>(
    neighbors: &BTreeMap<ClimateRegionId, RegionNeighbors>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeSeq;
    let mut seq = serializer.serialize_seq(Some(neighbors.len()))?;
    for (id, n) in neighbors {
        seq.serialize_element(&(id, n))?;
    }
    seq.end()
}

fn deserialize_neighbors<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<ClimateRegionId, RegionNeighbors>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let entries: Vec<(ClimateRegionId, RegionNeighbors)> = Deserialize::deserialize(deserializer)?;
    let mut map = BTreeMap::new();
    for (id, n) in entries {
        map.insert(id, n);
    }
    Ok(map)
}

impl Default for ClimateRegion {
    fn default() -> Self {
        Self::new()
    }
}

impl ClimateRegion {
    /// Create an empty climate region.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cells: BTreeMap::new(),
            neighbors: BTreeMap::new(),
            seasonal_cycle: SeasonalCycle::default(),
            avg_temperature: DEFAULT_BASE_TEMPERATURE,
            avg_humidity: DEFAULT_BASE_HUMIDITY,
            total_moisture: 0.0,
        }
    }

    /// Create with a specific seasonal cycle.
    #[must_use]
    pub fn with_seasonal_cycle(seasonal_cycle: SeasonalCycle) -> Self {
        Self {
            cells: BTreeMap::new(),
            neighbors: BTreeMap::new(),
            seasonal_cycle,
            avg_temperature: DEFAULT_BASE_TEMPERATURE,
            avg_humidity: DEFAULT_BASE_HUMIDITY,
            total_moisture: 0.0,
        }
    }

    /// Number of regions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Whether region is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Get current season.
    #[must_use]
    pub fn current_season(&self) -> Season {
        self.seasonal_cycle.season
    }

    /// Get seasonal cycle.
    #[must_use]
    pub fn seasonal_cycle(&self) -> &SeasonalCycle {
        &self.seasonal_cycle
    }

    /// Get mutable seasonal cycle.
    pub fn seasonal_cycle_mut(&mut self) -> &mut SeasonalCycle {
        &mut self.seasonal_cycle
    }

    /// Get average temperature.
    #[must_use]
    pub fn avg_temperature(&self) -> f32 {
        self.avg_temperature
    }

    /// Get average humidity.
    #[must_use]
    pub fn avg_humidity(&self) -> f32 {
        self.avg_humidity
    }

    /// Get total moisture.
    #[must_use]
    pub fn total_moisture(&self) -> f32 {
        self.total_moisture
    }

    /// Get a cell by region ID.
    #[must_use]
    pub fn get(&self, id: ClimateRegionId) -> Option<&ClimateCell> {
        self.cells.get(&id)
    }

    /// Get a mutable cell by region ID.
    pub fn get_mut(&mut self, id: ClimateRegionId) -> Option<&mut ClimateCell> {
        self.cells.get_mut(&id)
    }

    /// Get neighbors for a region.
    #[must_use]
    pub fn get_neighbors(&self, id: ClimateRegionId) -> Option<&RegionNeighbors> {
        self.neighbors.get(&id)
    }

    /// Insert a cell with neighbors.
    pub fn insert(&mut self, id: ClimateRegionId, cell: ClimateCell, neighbors: RegionNeighbors) {
        self.cells.insert(id, cell);
        self.neighbors.insert(id, neighbors);
        self.recompute_stats();
    }

    /// Insert a cell without neighbors (isolated region).
    pub fn insert_isolated(&mut self, id: ClimateRegionId, cell: ClimateCell) {
        self.cells.insert(id, cell);
        self.neighbors.insert(id, RegionNeighbors::default());
        self.recompute_stats();
    }

    /// Remove a cell.
    pub fn remove(&mut self, id: ClimateRegionId) -> Option<ClimateCell> {
        self.neighbors.remove(&id);
        let removed = self.cells.remove(&id);
        if removed.is_some() {
            self.recompute_stats();
        }
        removed
    }

    /// Iterate over cells in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = (&ClimateRegionId, &ClimateCell)> {
        self.cells.iter()
    }

    /// Iterate mutably.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&ClimateRegionId, &mut ClimateCell)> {
        self.cells.iter_mut()
    }

    /// Get all region IDs.
    pub fn region_ids(&self) -> impl Iterator<Item = &ClimateRegionId> {
        self.cells.keys()
    }

    /// Compute a summary of the region.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "region count fits in f32 for reasonable sizes"
    )]
    pub fn summary(&self) -> RegionalClimateSummary {
        if self.cells.is_empty() {
            return RegionalClimateSummary {
                region_count: 0,
                season: self.seasonal_cycle.season,
                season_progress: self.seasonal_cycle.progress,
                min_temperature: DEFAULT_BASE_TEMPERATURE,
                max_temperature: DEFAULT_BASE_TEMPERATURE,
                avg_temperature: DEFAULT_BASE_TEMPERATURE,
                min_humidity: DEFAULT_BASE_HUMIDITY,
                max_humidity: DEFAULT_BASE_HUMIDITY,
                avg_humidity: DEFAULT_BASE_HUMIDITY,
                total_moisture: 0.0,
                avg_wind_magnitude: 0.0,
                precipitating_regions: 0,
            };
        }

        let mut min_temp = f32::MAX;
        let mut max_temp = f32::MIN;
        let mut total_temp = 0.0f32;
        let mut min_hum = f32::MAX;
        let mut max_hum = f32::MIN;
        let mut total_hum = 0.0f32;
        let mut total_wind_mag = 0.0f32;
        let mut precip_count = 0usize;

        for cell in self.cells.values() {
            min_temp = min_temp.min(cell.temperature());
            max_temp = max_temp.max(cell.temperature());
            total_temp += cell.temperature();
            min_hum = min_hum.min(cell.humidity());
            max_hum = max_hum.max(cell.humidity());
            total_hum += cell.humidity();
            total_wind_mag += cell.wind().magnitude();
            if cell.will_precipitate() {
                precip_count += 1;
            }
        }

        let count = self.cells.len();

        RegionalClimateSummary {
            region_count: count,
            season: self.seasonal_cycle.season,
            season_progress: self.seasonal_cycle.progress,
            min_temperature: min_temp,
            max_temperature: max_temp,
            avg_temperature: total_temp / count as f32,
            min_humidity: min_hum,
            max_humidity: max_hum,
            avg_humidity: total_hum / count as f32,
            total_moisture: self.total_moisture,
            avg_wind_magnitude: total_wind_mag / count as f32,
            precipitating_regions: precip_count,
        }
    }

    /// Validate region state.
    #[must_use]
    pub fn validate(&self) -> RegionalClimateValidation {
        let mut issues = Vec::new();

        for (id, cell) in &self.cells {
            if cell.temperature() < MIN_TEMPERATURE || cell.temperature() > MAX_TEMPERATURE {
                issues.push(format!(
                    "Region {:?} has invalid temperature {}",
                    id,
                    cell.temperature()
                ));
            }
            if cell.humidity() < MIN_HUMIDITY || cell.humidity() > MAX_HUMIDITY {
                issues.push(format!(
                    "Region {:?} has invalid humidity {}",
                    id,
                    cell.humidity()
                ));
            }
            if cell.pressure() < MIN_PRESSURE || cell.pressure() > MAX_PRESSURE {
                issues.push(format!(
                    "Region {:?} has invalid pressure {}",
                    id,
                    cell.pressure()
                ));
            }
            if cell.moisture() < MIN_MOISTURE || cell.moisture() > MAX_MOISTURE {
                issues.push(format!(
                    "Region {:?} has invalid moisture {}",
                    id,
                    cell.moisture()
                ));
            }
        }

        for (id, neighbors) in &self.neighbors {
            for neighbor_id in neighbors.all() {
                if !self.cells.contains_key(&neighbor_id) {
                    issues.push(format!(
                        "Region {id:?} references non-existent neighbor {neighbor_id:?}"
                    ));
                }
            }
        }

        RegionalClimateValidation {
            is_valid: issues.is_empty(),
            issues,
        }
    }

    /// Compute checksum for determinism verification.
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "region count fits in u32")]
    pub fn checksum(&self) -> StepChecksum {
        let mut builder = ChecksumBuilder::new();
        builder.feed_u32(self.cells.len() as u32);
        builder.feed_u32(self.seasonal_cycle.season.as_index() as u32);
        builder.feed_f32(self.seasonal_cycle.progress);
        for (id, cell) in &self.cells {
            builder.feed_u32(id.0);
            builder.feed_f32(cell.temperature());
            builder.feed_f32(cell.humidity());
            builder.feed_f32(cell.pressure());
            builder.feed_f32(cell.moisture());
            builder.feed_f32(cell.wind().x);
            builder.feed_f32(cell.wind().y);
        }
        builder.build()
    }

    /// Compute compact fingerprint.
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "region count fits in u32")]
    pub fn fingerprint(&self) -> RegionalClimateFingerprint {
        let checksum = self.checksum();
        RegionalClimateFingerprint {
            region_count: self.cells.len() as u32,
            season: self.seasonal_cycle.season,
            avg_temp_bits: self.avg_temperature.to_bits(),
            total_moisture_bits: self.total_moisture.to_bits(),
            checksum: checksum.value(),
        }
    }

    /// Create a snapshot for unloaded-region persistence.
    #[must_use]
    pub fn snapshot(&self) -> RegionalClimateSnapshot {
        RegionalClimateSnapshot {
            fingerprint: self.fingerprint(),
            summary: self.summary(),
            seasonal_cycle: self.seasonal_cycle.clone(),
        }
    }

    /// Project future climate state.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "confidence calculation tolerates f32 precision"
    )]
    pub fn project(&self, ticks_ahead: u64, config: &RegionalClimateConfig) -> ClimateProjection {
        let mut projected_cycle = self.seasonal_cycle.clone();
        for _ in 0..ticks_ahead {
            projected_cycle.tick();
        }

        let season_temp_mod = projected_cycle.temperature_modifier() * config.seasonal_amplitude;
        let projected_avg_temp = self.avg_temperature + season_temp_mod;

        ClimateProjection {
            ticks_ahead,
            projected_season: projected_cycle.season,
            projected_avg_temperature: projected_avg_temp,
            projected_avg_humidity: self.avg_humidity,
            confidence: (1.0 - (ticks_ahead as f32 / 1_000_000.0).min(0.9)).max(0.1),
        }
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "region count fits in f32 for reasonable sizes"
    )]
    fn recompute_stats(&mut self) {
        if self.cells.is_empty() {
            self.avg_temperature = DEFAULT_BASE_TEMPERATURE;
            self.avg_humidity = DEFAULT_BASE_HUMIDITY;
            self.total_moisture = 0.0;
            return;
        }

        let mut total_temp = 0.0f32;
        let mut total_hum = 0.0f32;
        let mut total_moisture = 0.0f32;

        for cell in self.cells.values() {
            total_temp += cell.temperature();
            total_hum += cell.humidity();
            total_moisture += cell.moisture();
        }

        let count = self.cells.len() as f32;
        self.avg_temperature = total_temp / count;
        self.avg_humidity = total_hum / count;
        self.total_moisture = total_moisture;
    }
}

/// Summary statistics for a climate region.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RegionalClimateSummary {
    /// Number of climate regions.
    pub region_count: usize,
    /// Current season.
    pub season: Season,
    /// Progress through current season.
    pub season_progress: f32,
    /// Minimum temperature.
    pub min_temperature: f32,
    /// Maximum temperature.
    pub max_temperature: f32,
    /// Average temperature.
    pub avg_temperature: f32,
    /// Minimum humidity.
    pub min_humidity: f32,
    /// Maximum humidity.
    pub max_humidity: f32,
    /// Average humidity.
    pub avg_humidity: f32,
    /// Total moisture content.
    pub total_moisture: f32,
    /// Average wind magnitude.
    pub avg_wind_magnitude: f32,
    /// Count of regions with precipitation.
    pub precipitating_regions: usize,
}

/// Compact fingerprint for a climate region.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RegionalClimateFingerprint {
    /// Region count.
    pub region_count: u32,
    /// Current season.
    pub season: Season,
    /// Average temperature as bits.
    pub avg_temp_bits: u32,
    /// Total moisture as bits.
    pub total_moisture_bits: u32,
    /// Checksum value.
    pub checksum: u32,
}

/// Snapshot for unloaded-region persistence.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RegionalClimateSnapshot {
    /// Fingerprint for quick comparison.
    pub fingerprint: RegionalClimateFingerprint,
    /// Summary statistics.
    pub summary: RegionalClimateSummary,
    /// Seasonal cycle state.
    pub seasonal_cycle: SeasonalCycle,
}

impl RegionalClimateSnapshot {
    /// Validate that a snapshot is still applicable to a region.
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "region count fits in u32")]
    pub fn is_compatible(&self, region: &ClimateRegion) -> bool {
        self.fingerprint.region_count == region.len() as u32
            && self.fingerprint.season == region.current_season()
    }
}

/// Future climate state projection.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClimateProjection {
    /// Ticks into the future.
    pub ticks_ahead: u64,
    /// Projected season.
    pub projected_season: Season,
    /// Projected average temperature.
    pub projected_avg_temperature: f32,
    /// Projected average humidity.
    pub projected_avg_humidity: f32,
    /// Confidence in the projection (0.0 to 1.0).
    pub confidence: f32,
}

/// Result of a regional climate simulation step.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RegionalClimateResult {
    /// Moisture transports that occurred.
    pub moisture_transports: Vec<MoistureTransport>,
    /// Precipitation events.
    pub precipitation_events: Vec<PrecipitationEvent>,
    /// Total moisture transported.
    pub total_moisture_transported: f32,
    /// Total precipitation.
    pub total_precipitation: f32,
    /// Total evaporation.
    pub total_evaporation: f32,
    /// Whether season changed this tick.
    pub season_changed: bool,
    /// New season (if changed).
    pub new_season: Option<Season>,
    /// Regions updated.
    pub regions_updated: u32,
}

impl RegionalClimateResult {
    /// Create an empty result.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any changes occurred.
    #[must_use]
    pub fn has_changes(&self) -> bool {
        !self.moisture_transports.is_empty()
            || !self.precipitation_events.is_empty()
            || self.season_changed
            || self.regions_updated > 0
    }

    /// Compute checksum for determinism verification.
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "counts fit in u32")]
    pub fn checksum(&self) -> StepChecksum {
        let mut builder = ChecksumBuilder::new();
        builder.feed_u32(self.moisture_transports.len() as u32);
        for t in &self.moisture_transports {
            builder.feed_u32(t.source.0);
            builder.feed_u32(t.dest.0);
            builder.feed_f32(t.amount);
        }
        builder.feed_u32(self.precipitation_events.len() as u32);
        for p in &self.precipitation_events {
            builder.feed_u32(p.region.0);
            builder.feed_f32(p.amount);
        }
        builder.feed_f32(self.total_moisture_transported);
        builder.feed_f32(self.total_precipitation);
        builder.feed_f32(self.total_evaporation);
        builder.feed_u32(self.regions_updated);
        builder.build()
    }
}

/// Plan moisture transports between regions.
#[must_use]
pub fn plan_moisture_transports(
    region: &ClimateRegion,
    config: &RegionalClimateConfig,
) -> Vec<MoistureTransport> {
    if !config.transport_enabled {
        return Vec::new();
    }

    let mut transports = Vec::new();

    for (id, cell) in region.iter() {
        let Some(neighbors) = region.get_neighbors(*id) else {
            continue;
        };

        let wind = cell.wind();
        let wind_mag = wind.magnitude();

        for neighbor_id in neighbors.all() {
            let Some(neighbor_cell) = region.get(neighbor_id) else {
                continue;
            };

            let moisture_diff = cell.moisture() - neighbor_cell.moisture();
            if moisture_diff < config.min_moisture_diff {
                continue;
            }

            let base_transport = moisture_diff * config.transport_rate;

            let wind_factor = if let Some(primary_dest) = neighbors.in_wind_direction(&wind) {
                if primary_dest == neighbor_id {
                    1.0 + config.wind_influence * (wind_mag / 20.0).min(1.0)
                } else {
                    1.0 - config.wind_influence * 0.5
                }
            } else {
                1.0
            };

            let transport_amount = base_transport * wind_factor.max(0.1);

            if transport_amount > 0.01 {
                transports.push(MoistureTransport::new(
                    *id,
                    neighbor_id,
                    transport_amount,
                    wind_factor.clamp(0.0, 1.0),
                ));
            }
        }
    }

    transports.sort_by_key(MoistureTransport::sort_key);
    transports
}

/// Apply moisture transports to a region.
pub fn apply_moisture_transports(
    region: &mut ClimateRegion,
    transports: &[MoistureTransport],
) -> f32 {
    let mut total_transported = 0.0f32;

    for transport in transports {
        let effective = transport.effective_amount();
        if effective <= 0.0 {
            continue;
        }

        let source_moisture = region
            .get(transport.source)
            .map_or(0.0, ClimateCell::moisture);
        let actual_amount = effective.min(source_moisture);
        if actual_amount <= 0.01 {
            continue;
        }

        if let Some(source_cell) = region.get_mut(transport.source) {
            source_cell.apply_moisture_delta(-actual_amount);
        }

        if let Some(dest_cell) = region.get_mut(transport.dest) {
            dest_cell.apply_moisture_delta(actual_amount);
        }

        total_transported += actual_amount;
    }

    region.recompute_stats();
    total_transported
}

/// Compute precipitation events for a region.
#[must_use]
pub fn compute_precipitation(
    region: &ClimateRegion,
    config: &RegionalClimateConfig,
) -> Vec<PrecipitationEvent> {
    let mut events = Vec::new();

    for (id, cell) in region.iter() {
        let potential = cell.precipitation_potential();
        if potential < config.precipitation_threshold {
            continue;
        }

        let precip_amount = (potential - config.precipitation_threshold) * 0.5;
        let frozen = cell.effective_temperature() < 0.0;

        events.push(PrecipitationEvent::new(*id, precip_amount, frozen));
    }

    events.sort_by_key(PrecipitationEvent::sort_key);
    events
}

/// Apply precipitation to a region.
pub fn apply_precipitation(region: &mut ClimateRegion, events: &[PrecipitationEvent]) -> f32 {
    let mut total_precip = 0.0f32;

    for event in events {
        if let Some(cell) = region.get_mut(event.region) {
            cell.apply_moisture_delta(-event.amount);
            total_precip += event.amount;
        }
    }

    region.recompute_stats();
    total_precip
}

/// Apply evaporation to a region.
pub fn apply_evaporation(region: &mut ClimateRegion, config: &RegionalClimateConfig) -> f32 {
    let mut total_evap = 0.0f32;

    for (_, cell) in region.iter_mut() {
        let biome_modifier = cell.biome().evaporation_modifier();
        let temp_factor = ((cell.temperature() + 20.0) / 40.0).clamp(0.1, 2.0);
        let evap_rate = config.evaporation_rate * biome_modifier * temp_factor;

        let evap_amount = cell.moisture() * evap_rate;
        cell.apply_moisture_delta(-evap_amount);
        total_evap += evap_amount;
    }

    region.recompute_stats();
    total_evap
}

/// Apply seasonal temperature adjustments.
pub fn apply_seasonal_temperature(region: &mut ClimateRegion, config: &RegionalClimateConfig) {
    let temp_modifier = region.seasonal_cycle.temperature_modifier() * config.seasonal_amplitude;

    for (_, cell) in region.iter_mut() {
        let variation = cell.seasonal_variation();
        let seasonal_temp = cell.base_temperature() + temp_modifier * variation;
        let biome_mod = cell.biome().temperature_modifier();
        cell.set_temperature(seasonal_temp + biome_mod);
    }

    region.recompute_stats();
}

/// Execute a complete regional climate simulation step.
#[must_use]
pub fn regional_climate_step(
    region: &mut ClimateRegion,
    config: &RegionalClimateConfig,
) -> RegionalClimateResult {
    let old_season = region.seasonal_cycle.season;

    region.seasonal_cycle.tick();
    let season_changed = region.seasonal_cycle.season != old_season;

    apply_seasonal_temperature(region, config);

    let transports = plan_moisture_transports(region, config);
    let total_moisture_transported = apply_moisture_transports(region, &transports);

    let total_evaporation = apply_evaporation(region, config);

    let precipitation_events = compute_precipitation(region, config);
    let total_precipitation = apply_precipitation(region, &precipitation_events);

    #[expect(clippy::cast_possible_truncation, reason = "region count fits in u32")]
    RegionalClimateResult {
        moisture_transports: transports,
        precipitation_events,
        total_moisture_transported,
        total_precipitation,
        total_evaporation,
        season_changed,
        new_season: if season_changed {
            Some(region.seasonal_cycle.season)
        } else {
            None
        },
        regions_updated: region.len() as u32,
    }
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    clippy::similar_names,
    reason = "tests check exact values; similar names are intentional"
)]
mod tests {
    use super::*;

    fn make_id(id: u32) -> ClimateRegionId {
        ClimateRegionId::new(id)
    }

    #[test]
    fn wind_vector_creation() {
        let wind = WindVector::new(5.0, -3.0);
        assert!((wind.x - 5.0).abs() < 0.001);
        assert!((wind.y - (-3.0)).abs() < 0.001);
    }

    #[test]
    fn wind_vector_magnitude() {
        let wind = WindVector::new(3.0, 4.0);
        assert!((wind.magnitude() - 5.0).abs() < 0.001);
    }

    #[test]
    fn wind_vector_normalize() {
        let wind = WindVector::new(3.0, 4.0);
        let norm = wind.normalize();
        assert!((norm.magnitude() - 1.0).abs() < 0.001);
        assert!((norm.x - 0.6).abs() < 0.001);
        assert!((norm.y - 0.8).abs() < 0.001);
    }

    #[test]
    fn wind_vector_normalize_zero() {
        let wind = WindVector::ZERO;
        let norm = wind.normalize();
        assert!((norm.magnitude() - 0.0).abs() < 0.001);
    }

    #[test]
    fn wind_vector_lerp() {
        let a = WindVector::new(0.0, 0.0);
        let b = WindVector::new(10.0, 20.0);
        let mid = a.lerp(&b, 0.5);
        assert!((mid.x - 5.0).abs() < 0.001);
        assert!((mid.y - 10.0).abs() < 0.001);
    }

    #[test]
    fn climate_cell_creation() {
        let cell = ClimateCell::new(
            20.0,
            0.6,
            1.0,
            30.0,
            WindVector::westward(5.0),
            BiomeType::Temperate,
            500.0,
            45.0,
        );
        assert!((cell.temperature() - 20.0).abs() < 0.001);
        assert!((cell.humidity() - 0.6).abs() < 0.001);
        assert!((cell.moisture() - 30.0).abs() < 0.001);
    }

    #[test]
    fn climate_cell_clamping() {
        let cell = ClimateCell::new(
            200.0,
            2.0,
            5.0,
            500.0,
            WindVector::ZERO,
            BiomeType::Desert,
            -100.0,
            -200.0,
        );
        assert!((cell.temperature() - MAX_TEMPERATURE).abs() < 0.001);
        assert!((cell.humidity() - MAX_HUMIDITY).abs() < 0.001);
        assert!((cell.pressure() - MAX_PRESSURE).abs() < 0.001);
        assert!((cell.moisture() - MAX_MOISTURE).abs() < 0.001);
        assert!(cell.altitude() >= 0.0);
        assert!((cell.latitude() - (-90.0)).abs() < 0.001);
    }

    #[test]
    fn climate_cell_presets() {
        let temperate = ClimateCell::temperate();
        assert_eq!(temperate.biome(), BiomeType::Temperate);

        let desert = ClimateCell::desert();
        assert_eq!(desert.biome(), BiomeType::Desert);
        assert!(desert.humidity() < temperate.humidity());

        let tropical = ClimateCell::tropical();
        assert_eq!(tropical.biome(), BiomeType::Tropical);
        assert!(tropical.humidity() > temperate.humidity());

        let polar = ClimateCell::polar();
        assert_eq!(polar.biome(), BiomeType::Polar);
        assert!(polar.temperature() < temperate.temperature());
    }

    #[test]
    fn climate_cell_effective_temperature() {
        let low = ClimateCell::new(
            20.0,
            0.5,
            1.0,
            20.0,
            WindVector::ZERO,
            BiomeType::Temperate,
            0.0,
            45.0,
        );
        let high = ClimateCell::new(
            20.0,
            0.5,
            1.0,
            20.0,
            WindVector::ZERO,
            BiomeType::Mountain,
            3000.0,
            45.0,
        );
        assert!(high.effective_temperature() < low.effective_temperature());
    }

    #[test]
    fn climate_cell_saturation_moisture() {
        let warm = ClimateCell::new(
            30.0,
            0.5,
            1.0,
            20.0,
            WindVector::ZERO,
            BiomeType::Temperate,
            0.0,
            45.0,
        );
        let cold = ClimateCell::new(
            -10.0,
            0.5,
            1.0,
            20.0,
            WindVector::ZERO,
            BiomeType::Tundra,
            0.0,
            60.0,
        );
        assert!(warm.saturation_moisture() > cold.saturation_moisture());
    }

    #[test]
    fn climate_cell_precipitation_potential() {
        let wet = ClimateCell::new(
            20.0,
            1.0,
            1.0,
            80.0,
            WindVector::ZERO,
            BiomeType::Tropical,
            0.0,
            5.0,
        );
        let dry = ClimateCell::new(
            20.0,
            0.2,
            1.0,
            5.0,
            WindVector::ZERO,
            BiomeType::Desert,
            0.0,
            25.0,
        );
        assert!(wet.precipitation_potential() > dry.precipitation_potential());
    }

    #[test]
    fn seasonal_cycle_tick() {
        let mut cycle = SeasonalCycle::new(100);
        assert_eq!(cycle.season, Season::Spring);

        for _ in 0..100 {
            cycle.tick();
        }
        assert_eq!(cycle.season, Season::Summer);

        for _ in 0..100 {
            cycle.tick();
        }
        assert_eq!(cycle.season, Season::Autumn);
    }

    #[test]
    fn seasonal_cycle_full_year() {
        let mut cycle = SeasonalCycle::new(100);

        for _ in 0..400 {
            cycle.tick();
        }
        assert_eq!(cycle.season, Season::Spring);
    }

    #[test]
    fn seasonal_cycle_set_season() {
        let mut cycle = SeasonalCycle::new(100);
        cycle.set_season(Season::Winter);
        assert_eq!(cycle.season, Season::Winter);
        assert!((cycle.progress - 0.0).abs() < 0.001);
    }

    #[test]
    fn moisture_transport_creation() {
        let transport = MoistureTransport::new(make_id(1), make_id(2), 10.0, 0.8);
        assert!((transport.effective_amount() - 8.0).abs() < 0.001);
    }

    #[test]
    fn config_defaults_valid() {
        let config = RegionalClimateConfig::DEFAULT;
        let validation = config.validate();
        assert!(validation.is_valid);
    }

    #[test]
    fn config_presets_valid() {
        assert!(RegionalClimateConfig::ARID.validate().is_valid);
        assert!(RegionalClimateConfig::TROPICAL.validate().is_valid);
        assert!(RegionalClimateConfig::POLAR.validate().is_valid);
    }

    #[test]
    fn config_validation_invalid() {
        let mut config = RegionalClimateConfig::DEFAULT;
        config.transport_rate = -0.5;
        let validation = config.validate();
        assert!(!validation.is_valid);
        assert!(!validation.issues.is_empty());
    }

    #[test]
    fn region_neighbors_in_wind_direction() {
        let neighbors = RegionNeighbors::new(
            Some(make_id(1)),
            Some(make_id(2)),
            Some(make_id(3)),
            Some(make_id(4)),
        );

        let east_wind = WindVector::eastward(10.0);
        assert_eq!(neighbors.in_wind_direction(&east_wind), Some(make_id(3)));

        let north_wind = WindVector::northward(10.0);
        assert_eq!(neighbors.in_wind_direction(&north_wind), Some(make_id(1)));
    }

    #[test]
    fn region_insert_get() {
        let mut region = ClimateRegion::new();
        let id = make_id(1);
        let cell = ClimateCell::temperate();

        region.insert_isolated(id, cell);
        assert_eq!(region.len(), 1);
        assert!(!region.is_empty());

        let retrieved = region.get(id).unwrap();
        assert!((retrieved.temperature() - cell.temperature()).abs() < 0.001);
    }

    #[test]
    fn region_remove() {
        let mut region = ClimateRegion::new();
        let id = make_id(1);
        region.insert_isolated(id, ClimateCell::temperate());

        let removed = region.remove(id);
        assert!(removed.is_some());
        assert!(region.is_empty());
    }

    #[test]
    fn region_with_neighbors() {
        let mut region = ClimateRegion::new();

        let id1 = make_id(1);
        let id2 = make_id(2);

        region.insert(
            id1,
            ClimateCell::temperate(),
            RegionNeighbors::new(None, None, Some(id2), None),
        );
        region.insert(
            id2,
            ClimateCell::desert(),
            RegionNeighbors::new(None, None, None, Some(id1)),
        );

        let neighbors = region.get_neighbors(id1).unwrap();
        assert_eq!(neighbors.east, Some(id2));
    }

    #[test]
    fn region_stats() {
        let mut region = ClimateRegion::new();
        region.insert_isolated(
            make_id(1),
            ClimateCell::new(
                10.0,
                0.4,
                1.0,
                20.0,
                WindVector::ZERO,
                BiomeType::Temperate,
                0.0,
                45.0,
            ),
        );
        region.insert_isolated(
            make_id(2),
            ClimateCell::new(
                30.0,
                0.8,
                1.0,
                40.0,
                WindVector::ZERO,
                BiomeType::Tropical,
                0.0,
                5.0,
            ),
        );

        assert!((region.total_moisture() - 60.0).abs() < 0.001);
    }

    #[test]
    fn region_summary() {
        let mut region = ClimateRegion::new();
        region.insert_isolated(make_id(1), ClimateCell::temperate());
        region.insert_isolated(make_id(2), ClimateCell::desert());

        let summary = region.summary();
        assert_eq!(summary.region_count, 2);
        assert_eq!(summary.season, Season::Spring);
    }

    #[test]
    fn region_validation_valid() {
        let mut region = ClimateRegion::new();
        region.insert_isolated(make_id(1), ClimateCell::temperate());
        let validation = region.validate();
        assert!(validation.is_valid);
    }

    #[test]
    fn region_validation_invalid_neighbor() {
        let mut region = ClimateRegion::new();
        region.insert(
            make_id(1),
            ClimateCell::temperate(),
            RegionNeighbors::new(Some(make_id(999)), None, None, None),
        );
        let validation = region.validate();
        assert!(!validation.is_valid);
    }

    #[test]
    fn region_checksum_deterministic() {
        let make_region = || {
            let mut region = ClimateRegion::new();
            region.insert_isolated(make_id(1), ClimateCell::temperate());
            region.insert_isolated(make_id(2), ClimateCell::desert());
            region
        };

        let r1 = make_region();
        let r2 = make_region();

        assert_eq!(r1.checksum(), r2.checksum());
    }

    #[test]
    fn region_checksum_differs() {
        let mut r1 = ClimateRegion::new();
        r1.insert_isolated(make_id(1), ClimateCell::temperate());

        let mut r2 = ClimateRegion::new();
        r2.insert_isolated(make_id(1), ClimateCell::desert());

        assert_ne!(r1.checksum(), r2.checksum());
    }

    #[test]
    fn region_fingerprint() {
        let mut region = ClimateRegion::new();
        region.insert_isolated(make_id(1), ClimateCell::temperate());

        let fp = region.fingerprint();
        assert_eq!(fp.region_count, 1);
        assert_ne!(fp.checksum, 0);
    }

    #[test]
    fn region_snapshot() {
        let mut region = ClimateRegion::new();
        region.insert_isolated(make_id(1), ClimateCell::temperate());

        let snapshot = region.snapshot();
        assert!(snapshot.is_compatible(&region));
    }

    #[test]
    fn region_projection() {
        let mut region = ClimateRegion::new();
        region.insert_isolated(make_id(1), ClimateCell::temperate());

        let config = RegionalClimateConfig::DEFAULT;
        let projection = region.project(100, &config);

        assert_eq!(projection.ticks_ahead, 100);
        assert!(projection.confidence > 0.0);
    }

    #[test]
    fn plan_moisture_transports_basic() {
        let mut region = ClimateRegion::new();
        let id1 = make_id(1);
        let id2 = make_id(2);

        region.insert(
            id1,
            ClimateCell::new(
                20.0,
                0.5,
                1.0,
                50.0,
                WindVector::eastward(5.0),
                BiomeType::Temperate,
                0.0,
                45.0,
            ),
            RegionNeighbors::new(None, None, Some(id2), None),
        );
        region.insert(
            id2,
            ClimateCell::new(
                20.0,
                0.5,
                1.0,
                10.0,
                WindVector::eastward(5.0),
                BiomeType::Temperate,
                0.0,
                45.0,
            ),
            RegionNeighbors::new(None, None, None, Some(id1)),
        );

        let config = RegionalClimateConfig::DEFAULT;
        let transports = plan_moisture_transports(&region, &config);

        assert!(!transports.is_empty());
        assert_eq!(transports[0].source, id1);
        assert_eq!(transports[0].dest, id2);
    }

    #[test]
    fn plan_moisture_transports_deterministic() {
        let make_region = || {
            let mut region = ClimateRegion::new();
            let id1 = make_id(1);
            let id2 = make_id(2);
            region.insert(
                id1,
                ClimateCell::new(
                    20.0,
                    0.5,
                    1.0,
                    50.0,
                    WindVector::eastward(5.0),
                    BiomeType::Temperate,
                    0.0,
                    45.0,
                ),
                RegionNeighbors::new(None, None, Some(id2), None),
            );
            region.insert(
                id2,
                ClimateCell::new(
                    20.0,
                    0.5,
                    1.0,
                    10.0,
                    WindVector::eastward(5.0),
                    BiomeType::Temperate,
                    0.0,
                    45.0,
                ),
                RegionNeighbors::new(None, None, None, Some(id1)),
            );
            region
        };

        let config = RegionalClimateConfig::DEFAULT;
        let t1 = plan_moisture_transports(&make_region(), &config);
        let t2 = plan_moisture_transports(&make_region(), &config);

        assert_eq!(t1.len(), t2.len());
        for (a, b) in t1.iter().zip(t2.iter()) {
            assert_eq!(a.source, b.source);
            assert_eq!(a.dest, b.dest);
            assert!((a.amount - b.amount).abs() < 0.001);
        }
    }

    #[test]
    fn apply_moisture_transports_basic() {
        let mut region = ClimateRegion::new();
        let id1 = make_id(1);
        let id2 = make_id(2);

        region.insert(
            id1,
            ClimateCell::new(
                20.0,
                0.5,
                1.0,
                50.0,
                WindVector::ZERO,
                BiomeType::Temperate,
                0.0,
                45.0,
            ),
            RegionNeighbors::new(None, None, Some(id2), None),
        );
        region.insert(
            id2,
            ClimateCell::new(
                20.0,
                0.5,
                1.0,
                10.0,
                WindVector::ZERO,
                BiomeType::Temperate,
                0.0,
                45.0,
            ),
            RegionNeighbors::default(),
        );

        let transports = vec![MoistureTransport::new(id1, id2, 10.0, 1.0)];
        let transferred = apply_moisture_transports(&mut region, &transports);

        assert!(transferred > 0.0);
        assert!(region.get(id1).unwrap().moisture() < 50.0);
        assert!(region.get(id2).unwrap().moisture() > 10.0);
    }

    #[test]
    fn compute_precipitation_basic() {
        let mut region = ClimateRegion::new();
        region.insert_isolated(
            make_id(1),
            ClimateCell::new(
                20.0,
                1.0,
                1.0,
                80.0,
                WindVector::ZERO,
                BiomeType::Tropical,
                0.0,
                5.0,
            ),
        );

        let config = RegionalClimateConfig::DEFAULT;
        let events = compute_precipitation(&region, &config);

        assert!(!events.is_empty());
    }

    #[test]
    fn apply_evaporation_basic() {
        let mut region = ClimateRegion::new();
        region.insert_isolated(
            make_id(1),
            ClimateCell::new(
                30.0,
                0.5,
                1.0,
                50.0,
                WindVector::ZERO,
                BiomeType::Desert,
                0.0,
                25.0,
            ),
        );

        let initial_moisture = region.total_moisture();
        let config = RegionalClimateConfig::DEFAULT;
        let evap = apply_evaporation(&mut region, &config);

        assert!(evap > 0.0);
        assert!(region.total_moisture() < initial_moisture);
    }

    #[test]
    fn apply_seasonal_temperature() {
        let mut region = ClimateRegion::new();
        region.insert_isolated(
            make_id(1),
            ClimateCell::new(
                15.0,
                0.5,
                1.0,
                20.0,
                WindVector::ZERO,
                BiomeType::Temperate,
                0.0,
                60.0,
            ),
        );

        region.seasonal_cycle_mut().set_season(Season::Summer);
        let config = RegionalClimateConfig::DEFAULT;
        super::apply_seasonal_temperature(&mut region, &config);

        let cell = region.get(make_id(1)).unwrap();
        assert!(cell.temperature() > 15.0);
    }

    #[test]
    fn regional_climate_step_integration() {
        let mut region = ClimateRegion::new();
        let id1 = make_id(1);
        let id2 = make_id(2);

        region.insert(
            id1,
            ClimateCell::new(
                20.0,
                0.6,
                1.0,
                40.0,
                WindVector::eastward(5.0),
                BiomeType::Temperate,
                0.0,
                45.0,
            ),
            RegionNeighbors::new(None, None, Some(id2), None),
        );
        region.insert(
            id2,
            ClimateCell::new(
                25.0,
                0.4,
                1.0,
                20.0,
                WindVector::eastward(5.0),
                BiomeType::Grassland,
                0.0,
                40.0,
            ),
            RegionNeighbors::new(None, None, None, Some(id1)),
        );

        let config = RegionalClimateConfig::DEFAULT;
        let result = regional_climate_step(&mut region, &config);

        assert!(result.has_changes());
        assert_eq!(result.regions_updated, 2);
    }

    #[test]
    fn regional_climate_step_deterministic() {
        let make_region = || {
            let mut region = ClimateRegion::new();
            region.insert(
                make_id(1),
                ClimateCell::new(
                    20.0,
                    0.6,
                    1.0,
                    40.0,
                    WindVector::eastward(5.0),
                    BiomeType::Temperate,
                    0.0,
                    45.0,
                ),
                RegionNeighbors::new(None, None, Some(make_id(2)), None),
            );
            region.insert(
                make_id(2),
                ClimateCell::new(
                    25.0,
                    0.4,
                    1.0,
                    20.0,
                    WindVector::eastward(5.0),
                    BiomeType::Grassland,
                    0.0,
                    40.0,
                ),
                RegionNeighbors::new(None, None, None, Some(make_id(1))),
            );
            region
        };

        let config = RegionalClimateConfig::DEFAULT;

        let mut r1 = make_region();
        let res1 = regional_climate_step(&mut r1, &config);

        let mut r2 = make_region();
        let res2 = regional_climate_step(&mut r2, &config);

        assert_eq!(r1.checksum(), r2.checksum());
        assert_eq!(res1.checksum(), res2.checksum());
    }

    #[test]
    fn season_change_detection() {
        let mut region = ClimateRegion::with_seasonal_cycle(SeasonalCycle::new(10));
        region.insert_isolated(make_id(1), ClimateCell::temperate());

        let config = RegionalClimateConfig::DEFAULT;

        for _ in 0..9 {
            let result = regional_climate_step(&mut region, &config);
            assert!(!result.season_changed);
        }

        let result = regional_climate_step(&mut region, &config);
        assert!(result.season_changed);
        assert_eq!(result.new_season, Some(Season::Summer));
    }

    #[test]
    fn precipitation_evaporation_balance() {
        let mut region = ClimateRegion::new();
        region.insert_isolated(
            make_id(1),
            ClimateCell::new(
                25.0,
                0.7,
                1.0,
                50.0,
                WindVector::ZERO,
                BiomeType::Temperate,
                0.0,
                45.0,
            ),
        );

        let config = RegionalClimateConfig::DEFAULT;

        let initial_moisture = region.total_moisture();
        let result = regional_climate_step(&mut region, &config);

        let moisture_change = region.total_moisture() - initial_moisture;
        let expected_change = -result.total_evaporation - result.total_precipitation;

        assert!((moisture_change - expected_change).abs() < 0.1);
    }

    #[test]
    fn cross_region_transport_disabled() {
        let mut region = ClimateRegion::new();
        region.insert(
            make_id(1),
            ClimateCell::new(
                20.0,
                0.5,
                1.0,
                80.0,
                WindVector::eastward(10.0),
                BiomeType::Temperate,
                0.0,
                45.0,
            ),
            RegionNeighbors::new(None, None, Some(make_id(2)), None),
        );
        region.insert(
            make_id(2),
            ClimateCell::new(
                20.0,
                0.5,
                1.0,
                10.0,
                WindVector::ZERO,
                BiomeType::Temperate,
                0.0,
                45.0,
            ),
            RegionNeighbors::new(None, None, None, Some(make_id(1))),
        );

        let mut config = RegionalClimateConfig::DEFAULT;
        config.transport_enabled = false;

        let transports = plan_moisture_transports(&region, &config);
        assert!(transports.is_empty());
    }

    #[test]
    fn serde_cell_round_trip() {
        let cell = ClimateCell::tropical();
        let json = serde_json::to_string(&cell).unwrap();
        let recovered: ClimateCell = serde_json::from_str(&json).unwrap();
        assert!((recovered.temperature() - cell.temperature()).abs() < 0.001);
        assert!((recovered.humidity() - cell.humidity()).abs() < 0.001);
    }

    #[test]
    fn serde_region_round_trip() {
        let mut region = ClimateRegion::new();
        region.insert_isolated(make_id(1), ClimateCell::temperate());
        region.insert_isolated(make_id(2), ClimateCell::desert());

        let json = serde_json::to_string(&region).unwrap();
        let recovered: ClimateRegion = serde_json::from_str(&json).unwrap();

        assert_eq!(recovered.len(), region.len());
        assert_eq!(recovered.checksum(), region.checksum());
    }

    #[test]
    fn serde_config_round_trip() {
        let config = RegionalClimateConfig::TROPICAL;
        let json = serde_json::to_string(&config).unwrap();
        let recovered: RegionalClimateConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, config);
    }

    #[test]
    fn serde_result_round_trip() {
        let result = RegionalClimateResult {
            moisture_transports: vec![MoistureTransport::new(make_id(1), make_id(2), 5.0, 0.8)],
            precipitation_events: vec![PrecipitationEvent::new(make_id(1), 2.0, false)],
            total_moisture_transported: 4.0,
            total_precipitation: 2.0,
            total_evaporation: 1.0,
            season_changed: true,
            new_season: Some(Season::Summer),
            regions_updated: 2,
        };

        let json = serde_json::to_string(&result).unwrap();
        let recovered: RegionalClimateResult = serde_json::from_str(&json).unwrap();
        assert_eq!(
            recovered.moisture_transports.len(),
            result.moisture_transports.len()
        );
        assert_eq!(recovered.season_changed, result.season_changed);
    }

    #[test]
    fn bincode_region_round_trip() {
        let mut region = ClimateRegion::new();
        region.insert_isolated(make_id(1), ClimateCell::temperate());
        region.insert_isolated(make_id(2), ClimateCell::desert());

        let encoded = bincode::serialize(&region).unwrap();
        let decoded: ClimateRegion = bincode::deserialize(&encoded).unwrap();

        assert_eq!(decoded.len(), region.len());
        assert_eq!(decoded.checksum(), region.checksum());
    }

    #[test]
    fn bincode_snapshot_round_trip() {
        let mut region = ClimateRegion::new();
        region.insert_isolated(make_id(1), ClimateCell::temperate());

        let snapshot = region.snapshot();
        let encoded = bincode::serialize(&snapshot).unwrap();
        let decoded: RegionalClimateSnapshot = bincode::deserialize(&encoded).unwrap();

        assert_eq!(decoded.fingerprint, snapshot.fingerprint);
    }

    #[test]
    fn empty_region_operations() {
        let region = ClimateRegion::new();
        assert!(region.is_empty());
        assert_eq!(region.len(), 0);

        let summary = region.summary();
        assert_eq!(summary.region_count, 0);

        let validation = region.validate();
        assert!(validation.is_valid);
    }

    #[test]
    fn biome_modifiers_consistent() {
        for biome in BiomeType::ALL {
            let temp_mod = biome.temperature_modifier();
            let hum_mod = biome.humidity_modifier();
            let evap_mod = biome.evaporation_modifier();

            assert!((-50.0..=50.0).contains(&temp_mod));
            assert!((-1.0..=1.0).contains(&hum_mod));
            assert!((0.0..=5.0).contains(&evap_mod));
        }
    }

    #[test]
    fn result_checksum_deterministic() {
        let r1 = RegionalClimateResult {
            moisture_transports: vec![MoistureTransport::new(make_id(1), make_id(2), 5.0, 0.8)],
            precipitation_events: vec![PrecipitationEvent::new(make_id(1), 2.0, false)],
            total_moisture_transported: 4.0,
            total_precipitation: 2.0,
            total_evaporation: 1.0,
            season_changed: false,
            new_season: None,
            regions_updated: 2,
        };

        let r2 = RegionalClimateResult {
            moisture_transports: vec![MoistureTransport::new(make_id(1), make_id(2), 5.0, 0.8)],
            precipitation_events: vec![PrecipitationEvent::new(make_id(1), 2.0, false)],
            total_moisture_transported: 4.0,
            total_precipitation: 2.0,
            total_evaporation: 1.0,
            season_changed: false,
            new_season: None,
            regions_updated: 2,
        };

        assert_eq!(r1.checksum(), r2.checksum());
    }

    #[test]
    fn fingerprint_stability() {
        let mut region = ClimateRegion::new();
        region.insert_isolated(make_id(1), ClimateCell::temperate());

        let fp1 = region.fingerprint();
        let fp2 = region.fingerprint();

        assert_eq!(fp1, fp2);
    }

    #[test]
    fn snapshot_stability() {
        let mut region = ClimateRegion::new();
        region.insert_isolated(make_id(1), ClimateCell::temperate());

        let s1 = region.snapshot();
        let s2 = region.snapshot();

        assert_eq!(s1.fingerprint, s2.fingerprint);
    }
}
