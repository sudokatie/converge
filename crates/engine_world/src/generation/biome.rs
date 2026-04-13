//! Biome system for terrain variety.
//!
//! Biomes are determined by temperature and humidity at each location.

use noise::{NoiseFn, Perlin};

/// Biome types available in the world.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Biome {
    /// Flat grasslands with occasional trees.
    Plains,
    /// Dense tree coverage.
    Forest,
    /// Hot, dry, sandy terrain.
    Desert,
    /// High elevation with stone and snow.
    Mountains,
    /// Water bodies.
    Ocean,
}

impl Biome {
    /// Get the base surface block for this biome.
    #[must_use]
    pub fn surface_block(&self) -> u16 {
        match self {
            Biome::Plains => 3,    // Grass
            Biome::Forest => 3,   // Grass
            Biome::Desert => 4,   // Sand
            Biome::Mountains => 1, // Stone
            Biome::Ocean => 4,    // Sand (underwater)
        }
    }

    /// Get the subsurface block for this biome.
    #[must_use]
    pub fn subsurface_block(&self) -> u16 {
        match self {
            Biome::Plains => 2,    // Dirt
            Biome::Forest => 2,   // Dirt
            Biome::Desert => 4,   // Sand
            Biome::Mountains => 1, // Stone
            Biome::Ocean => 2,    // Dirt
        }
    }

    /// Get tree density for this biome (0.0 - 1.0).
    #[must_use]
    pub fn tree_density(&self) -> f32 {
        match self {
            Biome::Plains => 0.02,
            Biome::Forest => 0.15,
            Biome::Desert => 0.0,
            Biome::Mountains => 0.01,
            Biome::Ocean => 0.0,
        }
    }

    /// Check if this biome can have water at sea level.
    #[must_use]
    pub fn has_water(&self) -> bool {
        matches!(self, Biome::Ocean)
    }

    /// Get height modifier for this biome.
    #[must_use]
    pub fn height_modifier(&self) -> f64 {
        match self {
            Biome::Plains => 0.0,
            Biome::Forest => 0.0,
            Biome::Desert => -5.0,
            Biome::Mountains => 30.0,
            Biome::Ocean => -20.0,
        }
    }

    /// Get height scale for this biome (affects terrain roughness).
    #[must_use]
    pub fn height_scale(&self) -> f64 {
        match self {
            Biome::Plains => 0.3,
            Biome::Forest => 0.5,
            Biome::Desert => 0.2,
            Biome::Mountains => 1.5,
            Biome::Ocean => 0.1,
        }
    }
}

/// Biome selector using temperature and humidity noise.
pub struct BiomeSelector {
    temperature_noise: Perlin,
    humidity_noise: Perlin,
    offset: f64,
}

impl BiomeSelector {
    /// Create a new biome selector with the given seed.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        // Use different seeds for each noise layer
        let temp_seed = seed.wrapping_mul(31337);
        let humid_seed = seed.wrapping_mul(65537);
        
        Self {
            temperature_noise: Perlin::new(temp_seed as u32),
            humidity_noise: Perlin::new(humid_seed as u32),
            offset: ((seed as f64) * 3571.0) % 100_000.0,
        }
    }

    /// Get the biome at a world position.
    #[must_use]
    pub fn biome_at(&self, x: f64, z: f64) -> Biome {
        let (temp, humid) = self.sample(x, z);
        Self::select_biome(temp, humid)
    }

    /// Sample temperature and humidity at a position.
    ///
    /// Returns (temperature, humidity) both in range [0, 1].
    #[must_use]
    pub fn sample(&self, x: f64, z: f64) -> (f64, f64) {
        // Use larger scale for biome noise (smoother transitions)
        let scale = 0.002;
        let ox = x * scale + self.offset;
        let oz = z * scale + self.offset;

        // Sample and normalize to 0-1
        let temp = (self.temperature_noise.get([ox, oz]) + 1.0) * 0.5;
        let humid = (self.humidity_noise.get([ox * 1.3, oz * 1.3 + 1000.0]) + 1.0) * 0.5;

        (temp.clamp(0.0, 1.0), humid.clamp(0.0, 1.0))
    }

    /// Select biome based on temperature and humidity.
    ///
    /// Temperature: 0 = cold, 1 = hot
    /// Humidity: 0 = dry, 1 = wet
    #[must_use]
    pub fn select_biome(temperature: f64, humidity: f64) -> Biome {
        // Hot + Dry = Desert
        if temperature > 0.6 && humidity < 0.3 {
            return Biome::Desert;
        }

        // Very wet = Ocean (low areas will be flooded)
        if humidity > 0.8 {
            return Biome::Ocean;
        }

        // Cold + varied humidity = Mountains
        if temperature < 0.3 {
            return Biome::Mountains;
        }

        // Moderate temp + high humidity = Forest
        if humidity > 0.5 {
            return Biome::Forest;
        }

        // Default = Plains
        Biome::Plains
    }

    /// Get temperature value at position (0 = cold, 1 = hot).
    #[must_use]
    pub fn temperature_at(&self, x: f64, z: f64) -> f64 {
        self.sample(x, z).0
    }

    /// Get humidity value at position (0 = dry, 1 = wet).
    #[must_use]
    pub fn humidity_at(&self, x: f64, z: f64) -> f64 {
        self.sample(x, z).1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biome_surface_blocks() {
        assert_eq!(Biome::Plains.surface_block(), 3);
        assert_eq!(Biome::Desert.surface_block(), 4);
        assert_eq!(Biome::Mountains.surface_block(), 1);
    }

    #[test]
    fn test_biome_tree_density() {
        assert!(Biome::Forest.tree_density() > Biome::Plains.tree_density());
        assert_eq!(Biome::Desert.tree_density(), 0.0);
        assert_eq!(Biome::Ocean.tree_density(), 0.0);
    }

    #[test]
    fn test_biome_selector_deterministic() {
        let selector1 = BiomeSelector::new(12345);
        let selector2 = BiomeSelector::new(12345);

        let biome1 = selector1.biome_at(100.0, 200.0);
        let biome2 = selector2.biome_at(100.0, 200.0);

        assert_eq!(biome1, biome2);
    }

    #[test]
    fn test_biome_selection_desert() {
        // Hot + Dry = Desert
        assert_eq!(BiomeSelector::select_biome(0.8, 0.1), Biome::Desert);
    }

    #[test]
    fn test_biome_selection_mountains() {
        // Cold = Mountains
        assert_eq!(BiomeSelector::select_biome(0.1, 0.4), Biome::Mountains);
    }

    #[test]
    fn test_biome_selection_forest() {
        // Moderate temp + humid = Forest
        assert_eq!(BiomeSelector::select_biome(0.5, 0.7), Biome::Forest);
    }

    #[test]
    fn test_biome_selection_ocean() {
        // Very wet = Ocean
        assert_eq!(BiomeSelector::select_biome(0.5, 0.9), Biome::Ocean);
    }

    #[test]
    fn test_biome_selection_plains() {
        // Default moderate conditions = Plains
        assert_eq!(BiomeSelector::select_biome(0.5, 0.4), Biome::Plains);
    }

    #[test]
    fn test_sample_in_range() {
        let selector = BiomeSelector::new(42);

        for i in 0..100 {
            let x = (i * 17) as f64;
            let z = (i * 23) as f64;
            let (temp, humid) = selector.sample(x, z);

            assert!(temp >= 0.0 && temp <= 1.0, "Temperature out of range: {}", temp);
            assert!(humid >= 0.0 && humid <= 1.0, "Humidity out of range: {}", humid);
        }
    }

    #[test]
    fn test_biome_height_modifiers() {
        assert!(Biome::Mountains.height_modifier() > Biome::Plains.height_modifier());
        assert!(Biome::Ocean.height_modifier() < Biome::Plains.height_modifier());
    }

    #[test]
    fn test_biome_has_water() {
        assert!(Biome::Ocean.has_water());
        assert!(!Biome::Plains.has_water());
        assert!(!Biome::Desert.has_water());
    }
}
