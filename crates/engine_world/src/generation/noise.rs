//! Terrain noise generation using Perlin noise.

use noise::{NoiseFn, Perlin};

/// FBM (Fractal Brownian Motion) octaves.
const OCTAVES: u32 = 5;
/// Amplitude persistence per octave.
const PERSISTENCE: f64 = 0.5;
/// Frequency lacunarity per octave.
const LACUNARITY: f64 = 2.0;

/// Minimum terrain height.
const MIN_HEIGHT: f64 = 32.0;
/// Maximum terrain height.
const MAX_HEIGHT: f64 = 128.0;
/// Height scale factor.
const HEIGHT_SCALE: f64 = (MAX_HEIGHT - MIN_HEIGHT) / 2.0;
/// Base height (middle of range).
const BASE_HEIGHT: f64 = (MIN_HEIGHT + MAX_HEIGHT) / 2.0;

/// Terrain noise generator.
pub struct TerrainNoise {
    seed: u64,
    perlin: Perlin,
    /// Offset derived from seed for variation.
    offset: f64,
}

impl TerrainNoise {
    /// Create a new terrain noise generator with the given seed.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        let perlin = Perlin::new(seed as u32);
        // Use seed to create a unique offset with prime multiplier for better distribution
        let offset = ((seed as f64) * 7919.0) % 100_000.0;
        Self { seed, perlin, offset }
    }

    /// Get the terrain height at a world position.
    ///
    /// Returns height in range [MIN_HEIGHT, MAX_HEIGHT].
    #[must_use]
    pub fn height_at(&self, x: f64, z: f64) -> f64 {
        let noise_value = self.fbm_2d(x * 0.01, z * 0.01);
        BASE_HEIGHT + noise_value * HEIGHT_SCALE
    }

    /// Sample 3D noise at a position (for caves, etc.).
    ///
    /// Returns value in range [-1, 1].
    #[must_use]
    pub fn sample_3d(&self, x: f64, y: f64, z: f64) -> f64 {
        self.fbm_3d(x * 0.02, y * 0.02, z * 0.02)
    }

    /// Sample 2D noise at a position.
    ///
    /// Returns value in range [-1, 1].
    #[must_use]
    pub fn sample_2d(&self, x: f64, z: f64) -> f64 {
        self.fbm_2d(x * 0.01, z * 0.01)
    }

    /// Fractal Brownian Motion for 2D.
    fn fbm_2d(&self, x: f64, z: f64) -> f64 {
        let mut total = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = 1.0;
        let mut max_value = 0.0;

        // Add seed-based offset for unique terrain per seed
        let ox = x + self.offset;
        let oz = z + self.offset;

        for _ in 0..OCTAVES {
            total += self.perlin.get([ox * frequency, oz * frequency]) * amplitude;
            max_value += amplitude;
            amplitude *= PERSISTENCE;
            frequency *= LACUNARITY;
        }

        total / max_value
    }

    /// Fractal Brownian Motion for 3D.
    fn fbm_3d(&self, x: f64, y: f64, z: f64) -> f64 {
        let mut total = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = 1.0;
        let mut max_value = 0.0;

        // Add seed-based offset for unique caves per seed
        let ox = x + self.offset;
        let oy = y + self.offset;
        let oz = z + self.offset;

        for _ in 0..OCTAVES {
            total += self.perlin.get([ox * frequency, oy * frequency, oz * frequency]) * amplitude;
            max_value += amplitude;
            amplitude *= PERSISTENCE;
            frequency *= LACUNARITY;
        }

        total / max_value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_same_seed_same_values() {
        let noise1 = TerrainNoise::new(12345);
        let noise2 = TerrainNoise::new(12345);

        let h1 = noise1.height_at(100.0, 200.0);
        let h2 = noise2.height_at(100.0, 200.0);

        assert!((h1 - h2).abs() < 0.001);
    }

    #[test]
    fn test_different_seeds_different_offset() {
        let noise1 = TerrainNoise::new(12345);
        let noise2 = TerrainNoise::new(99999);

        // Different seeds should have different offsets
        assert_ne!(noise1.seed, noise2.seed);
        assert!((noise1.offset - noise2.offset).abs() > 1.0);
    }

    #[test]
    fn test_height_in_range() {
        let noise = TerrainNoise::new(42);

        for x in -100..100 {
            for z in -100..100 {
                let h = noise.height_at(x as f64, z as f64);
                assert!(
                    h >= MIN_HEIGHT && h <= MAX_HEIGHT,
                    "Height {h} out of range at ({x}, {z})"
                );
            }
        }
    }

    #[test]
    fn test_continuous() {
        let noise = TerrainNoise::new(42);

        let h1 = noise.height_at(100.0, 100.0);
        let h2 = noise.height_at(100.01, 100.0);

        // Small position change should give similar height
        assert!((h1 - h2).abs() < 1.0);
    }

    #[test]
    fn test_3d_sample_in_range() {
        let noise = TerrainNoise::new(42);

        for _ in 0..100 {
            let x = rand_coord();
            let y = rand_coord();
            let z = rand_coord();
            let v = noise.sample_3d(x, y, z);
            assert!(
                v >= -1.0 && v <= 1.0,
                "3D noise {v} out of range at ({x}, {y}, {z})"
            );
        }
    }

    fn rand_coord() -> f64 {
        // Simple pseudo-random for testing
        use std::time::SystemTime;
        let t = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as f64;
        (t % 1000.0) - 500.0
    }
}
