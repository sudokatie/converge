//! Sound pool for playing randomized variations.
//!
//! Avoids audio repetition by selecting from a pool of sound variants
//! with random pitch and volume offsets per spec 9.2.

use crate::sound::SoundId;
use rand::Rng;

/// Maximum number of variants per sound pool.
pub const MAX_POOL_SIZE: usize = 8;

/// Pitch variation range (semi-tone offset).
const PITCH_VARIATION: f32 = 0.1;

/// Volume variation range.
const VOLUME_VARIATION: f32 = 0.05;

/// A pool of sound variants for randomized playback.
///
/// Each pool maps to a logical sound (e.g., "footstep_grass") and
/// contains multiple sound IDs that are randomly selected from.
#[derive(Debug, Clone)]
pub struct SoundPool {
    /// Logical name for this pool.
    pub name: String,
    /// Sound IDs in this pool.
    variants: Vec<SoundId>,
    /// Index of the last played variant (for anti-repeat).
    last_index: Option<usize>,
    /// Whether to apply random pitch offset.
    pub pitch_variation: bool,
    /// Whether to apply random volume offset.
    pub volume_variation: bool,
}

impl SoundPool {
    /// Create a new sound pool.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            variants: Vec::new(),
            last_index: None,
            pitch_variation: true,
            volume_variation: true,
        }
    }

    /// Add a sound variant to the pool.
    pub fn add_variant(&mut self, id: SoundId) {
        if self.variants.len() < MAX_POOL_SIZE {
            self.variants.push(id);
        }
    }

    /// Number of variants in the pool.
    #[must_use]
    pub fn len(&self) -> usize {
        self.variants.len()
    }

    /// Check if the pool is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.variants.is_empty()
    }

    /// Pick a random variant from the pool.
    ///
    /// Avoids repeating the same variant twice in a row.
    /// Returns the SoundId and optional pitch/volume offsets.
    #[must_use]
    pub fn pick(&mut self) -> Option<PoolPick> {
        if self.variants.is_empty() {
            return None;
        }

        if self.variants.len() == 1 {
            self.last_index = Some(0);
            return Some(PoolPick {
                sound_id: self.variants[0],
                pitch_offset: self.random_pitch(),
                volume_offset: self.random_volume(),
            });
        }

        // Pick random index, avoiding the last one played
        let mut rng = rand::thread_rng();
        let mut idx = rng.gen_range(0..self.variants.len());
        if let Some(last) = self.last_index {
            if idx == last {
                idx = (idx + 1) % self.variants.len();
            }
        }

        self.last_index = Some(idx);

        Some(PoolPick {
            sound_id: self.variants[idx],
            pitch_offset: self.random_pitch(),
            volume_offset: self.random_volume(),
        })
    }

    /// Generate a random pitch offset.
    fn random_pitch(&self) -> f32 {
        if !self.pitch_variation {
            return 0.0;
        }
        let mut rng = rand::thread_rng();
        rng.gen_range(-PITCH_VARIATION..PITCH_VARIATION)
    }

    /// Generate a random volume offset.
    fn random_volume(&self) -> f32 {
        if !self.volume_variation {
            return 0.0;
        }
        let mut rng = rand::thread_rng();
        rng.gen_range(-VOLUME_VARIATION..VOLUME_VARIATION)
    }

    /// Get the variant at a specific index.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<SoundId> {
        self.variants.get(index).copied()
    }
}

/// Result of picking from a sound pool.
#[derive(Debug, Clone, Copy)]
pub struct PoolPick {
    /// The selected sound ID.
    pub sound_id: SoundId,
    /// Pitch offset to apply (-0.1 to 0.1).
    pub pitch_offset: f32,
    /// Volume offset to apply (-0.05 to 0.05).
    pub volume_offset: f32,
}

/// Registry of sound pools.
#[derive(Debug, Clone)]
pub struct SoundPoolRegistry {
    pools: Vec<SoundPool>,
}

impl Default for SoundPoolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SoundPoolRegistry {
    /// Create a new pool registry.
    #[must_use]
    pub fn new() -> Self {
        Self { pools: Vec::new() }
    }

    /// Add a sound pool.
    pub fn add(&mut self, pool: SoundPool) {
        self.pools.push(pool);
    }

    /// Get a pool by name.
    #[must_use]
    pub fn get_by_name(&self, name: &str) -> Option<&SoundPool> {
        self.pools.iter().find(|p| p.name == name)
    }

    /// Get a mutable pool by name.
    pub fn get_by_name_mut(&mut self, name: &str) -> Option<&mut SoundPool> {
        self.pools.iter_mut().find(|p| p.name == name)
    }

    /// Pick from a named pool.
    pub fn pick(&mut self, name: &str) -> Option<PoolPick> {
        self.get_by_name_mut(name).and_then(|pool| pool.pick())
    }

    /// Number of registered pools.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pools.len()
    }

    /// Check if registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pools.is_empty()
    }

    /// Create default sound pools for common game sounds.
    #[must_use]
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();

        // Footstep pools
        let mut grass_steps = SoundPool::new("footstep_grass");
        grass_steps.add_variant(SoundId(10));
        grass_steps.add_variant(SoundId(100));
        grass_steps.add_variant(SoundId(101));
        grass_steps.add_variant(SoundId(102));
        registry.add(grass_steps);

        let mut stone_steps = SoundPool::new("footstep_stone");
        stone_steps.add_variant(SoundId(11));
        stone_steps.add_variant(SoundId(110));
        stone_steps.add_variant(SoundId(111));
        registry.add(stone_steps);

        // Block interaction pools
        let mut block_break = SoundPool::new("block_break");
        block_break.add_variant(SoundId(2));
        block_break.add_variant(SoundId(200));
        block_break.add_variant(SoundId(201));
        registry.add(block_break);

        registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_pool_is_empty() {
        let pool = SoundPool::new("test");
        assert!(pool.is_empty());
        assert_eq!(pool.name, "test");
    }

    #[test]
    fn test_add_variants() {
        let mut pool = SoundPool::new("test");
        pool.add_variant(SoundId(1));
        pool.add_variant(SoundId(2));
        pool.add_variant(SoundId(3));
        assert_eq!(pool.len(), 3);
    }

    #[test]
    fn test_max_pool_size() {
        let mut pool = SoundPool::new("test");
        for i in 0..MAX_POOL_SIZE + 5 {
            pool.add_variant(SoundId(i as u16));
        }
        assert_eq!(pool.len(), MAX_POOL_SIZE);
    }

    #[test]
    fn test_pick_from_empty() {
        let mut pool = SoundPool::new("test");
        assert!(pool.pick().is_none());
    }

    #[test]
    fn test_pick_from_single() {
        let mut pool = SoundPool::new("test");
        pool.add_variant(SoundId(42));
        let pick = pool.pick().unwrap();
        assert_eq!(pick.sound_id, SoundId(42));
    }

    #[test]
    fn test_pick_avoids_repeat() {
        let mut pool = SoundPool::new("test");
        pool.add_variant(SoundId(1));
        pool.add_variant(SoundId(2));

        let pick1 = pool.pick().unwrap();
        let pick2 = pool.pick().unwrap();

        // With only 2 variants, they should alternate
        assert_ne!(pick1.sound_id, pick2.sound_id);
    }

    #[test]
    fn test_pick_has_variation_offsets() {
        let mut pool = SoundPool::new("test");
        pool.add_variant(SoundId(1));
        pool.add_variant(SoundId(2));
        pool.add_variant(SoundId(3));

        // With variation enabled, offsets should be in range
        let pick = pool.pick().unwrap();
        assert!(pick.pitch_offset >= -PITCH_VARIATION);
        assert!(pick.pitch_offset <= PITCH_VARIATION);
        assert!(pick.volume_offset >= -VOLUME_VARIATION);
        assert!(pick.volume_offset <= VOLUME_VARIATION);
    }

    #[test]
    fn test_no_variation() {
        let mut pool = SoundPool::new("test");
        pool.pitch_variation = false;
        pool.volume_variation = false;
        pool.add_variant(SoundId(1));

        let pick = pool.pick().unwrap();
        assert_eq!(pick.pitch_offset, 0.0);
        assert_eq!(pick.volume_offset, 0.0);
    }

    #[test]
    fn test_registry_add_and_get() {
        let mut registry = SoundPoolRegistry::new();
        registry.add(SoundPool::new("footstep_grass"));

        assert!(registry.get_by_name("footstep_grass").is_some());
        assert!(registry.get_by_name("nonexistent").is_none());
    }

    #[test]
    fn test_registry_pick() {
        let mut registry = SoundPoolRegistry::new();
        let mut pool = SoundPool::new("test");
        pool.add_variant(SoundId(1));
        pool.add_variant(SoundId(2));
        registry.add(pool);

        let pick = registry.pick("test").unwrap();
        assert!(pick.sound_id == SoundId(1) || pick.sound_id == SoundId(2));
    }

    #[test]
    fn test_registry_pick_nonexistent() {
        let mut registry = SoundPoolRegistry::new();
        assert!(registry.pick("nope").is_none());
    }

    #[test]
    fn test_defaults() {
        let registry = SoundPoolRegistry::with_defaults();
        assert!(!registry.is_empty());
        assert!(registry.get_by_name("footstep_grass").is_some());
        assert!(registry.get_by_name("footstep_stone").is_some());
        assert!(registry.get_by_name("block_break").is_some());
    }
}
