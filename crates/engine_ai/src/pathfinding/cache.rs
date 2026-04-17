//! Path caching with invalidation on world changes.
//!
//! Implements spec 8.2.3: cache valid paths for 5 seconds,
//! invalidate on nearby world change.

use glam::IVec3;
use std::collections::HashMap;
use std::time::Instant;

/// Default cache duration in seconds.
pub const CACHE_DURATION_SECS: f64 = 5.0;

/// Maximum number of cached paths.
pub const MAX_CACHE_SIZE: usize = 256;

/// Invalidation radius around a block change (blocks).
pub const INVALIDATION_RADIUS: i32 = 8;

/// A cached path.
#[derive(Debug, Clone)]
pub struct CachedPath {
    /// Waypoints in the path.
    pub waypoints: Vec<IVec3>,
    /// When this path was cached.
    pub cached_at: Instant,
    /// Source position.
    pub from: IVec3,
    /// Destination position.
    pub to: IVec3,
}

impl CachedPath {
    /// Create a new cached path.
    #[must_use]
    pub fn new(from: IVec3, to: IVec3, waypoints: Vec<IVec3>) -> Self {
        Self {
            waypoints,
            cached_at: Instant::now(),
            from,
            to,
        }
    }

    /// Check if the cache has expired.
    #[must_use]
    pub fn is_expired(&self, now: Instant, cache_duration: f64) -> bool {
        now.duration_since(self.cached_at).as_secs_f64() >= cache_duration
    }

    /// Check if this path passes near a given position.
    #[must_use]
    pub fn passes_near(&self, pos: IVec3, radius: i32) -> bool {
        let radius_sq = (radius * radius) as f32;

        // Check if start/end are near
        if (self.from - pos).as_vec3().length_squared() <= radius_sq {
            return true;
        }
        if (self.to - pos).as_vec3().length_squared() <= radius_sq {
            return true;
        }

        // Check waypoints
        for waypoint in &self.waypoints {
            if (*waypoint - pos).as_vec3().length_squared() <= radius_sq {
                return true;
            }
        }

        false
    }

    /// Get the path length (number of waypoints).
    #[must_use]
    pub fn len(&self) -> usize {
        self.waypoints.len()
    }

    /// Check if the path is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.waypoints.is_empty()
    }
}

/// Path cache key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PathKey {
    /// Source position.
    pub from: IVec3,
    /// Destination position.
    pub to: IVec3,
}

/// Path cache with time-based and change-based invalidation.
#[derive(Debug)]
pub struct PathCache {
    /// Cached paths.
    paths: HashMap<PathKey, CachedPath>,
    /// Cache duration in seconds.
    cache_duration: f64,
    /// Number of cache hits.
    hits: u64,
    /// Number of cache misses.
    misses: u64,
    /// Number of invalidations.
    invalidations: u64,
}

impl Default for PathCache {
    fn default() -> Self {
        Self::new()
    }
}

impl PathCache {
    /// Create a new path cache.
    #[must_use]
    pub fn new() -> Self {
        Self {
            paths: HashMap::new(),
            cache_duration: CACHE_DURATION_SECS,
            hits: 0,
            misses: 0,
            invalidations: 0,
        }
    }

    /// Set the cache duration.
    pub fn set_cache_duration(&mut self, duration: f64) {
        self.cache_duration = duration.max(0.0);
    }

    /// Store a path in the cache.
    pub fn store(&mut self, from: IVec3, to: IVec3, waypoints: Vec<IVec3>) {
        // Evict oldest entries if at capacity
        if self.paths.len() >= MAX_CACHE_SIZE {
            self.evict_oldest();
        }

        let key = PathKey { from, to };
        self.paths.insert(key, CachedPath::new(from, to, waypoints));
    }

    /// Look up a path in the cache.
    ///
    /// Returns None if not cached or expired.
    pub fn get(&mut self, from: IVec3, to: IVec3) -> Option<CachedPath> {
        let key = PathKey { from, to };
        let now = Instant::now();

        let expired = if let Some(cached) = self.paths.get(&key) {
            cached.is_expired(now, self.cache_duration)
        } else {
            self.misses += 1;
            return None;
        };

        if expired {
            self.paths.remove(&key);
            self.misses += 1;
            return None;
        }

        self.hits += 1;
        self.paths.get(&key).cloned()
    }

    /// Invalidate paths near a block change position.
    pub fn invalidate_near(&mut self, pos: IVec3) -> usize {
        let keys_to_remove: Vec<PathKey> = self
            .paths
            .iter()
            .filter(|(_, path)| path.passes_near(pos, INVALIDATION_RADIUS))
            .map(|(key, _)| *key)
            .collect();

        let count = keys_to_remove.len();
        for key in keys_to_remove {
            self.paths.remove(&key);
        }
        self.invalidations += count as u64;
        count
    }

    /// Remove all expired entries.
    ///
    /// Returns the number of entries removed.
    pub fn evict_expired(&mut self) -> usize {
        let now = Instant::now();
        let keys_to_remove: Vec<PathKey> = self
            .paths
            .iter()
            .filter(|(_, path)| path.is_expired(now, self.cache_duration))
            .map(|(key, _)| *key)
            .collect();

        let count = keys_to_remove.len();
        for key in keys_to_remove {
            self.paths.remove(&key);
        }
        count
    }

    /// Evict the oldest entry.
    fn evict_oldest(&mut self) {
        if let Some(oldest_key) = self
            .paths
            .iter()
            .min_by_key(|(_, path)| path.cached_at)
            .map(|(key, _)| *key)
        {
            self.paths.remove(&oldest_key);
        }
    }

    /// Clear the entire cache.
    pub fn clear(&mut self) {
        self.paths.clear();
    }

    /// Number of cached paths.
    #[must_use]
    pub fn len(&self) -> usize {
        self.paths.len()
    }

    /// Check if cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    /// Get cache hit count.
    #[must_use]
    pub fn hits(&self) -> u64 {
        self.hits
    }

    /// Get cache miss count.
    #[must_use]
    pub fn misses(&self) -> u64 {
        self.misses
    }

    /// Get cache invalidation count.
    #[must_use]
    pub fn invalidations(&self) -> u64 {
        self.invalidations
    }

    /// Get hit rate as a fraction (0.0 to 1.0).
    #[must_use]
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            return 0.0;
        }
        self.hits as f64 / total as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_get() {
        let mut cache = PathCache::new();
        let from = IVec3::new(0, 0, 0);
        let to = IVec3::new(10, 0, 10);
        let waypoints = vec![IVec3::new(5, 0, 5), IVec3::new(10, 0, 10)];

        cache.store(from, to, waypoints.clone());
        let result = cache.get(from, to);
        assert!(result.is_some());
        assert_eq!(result.unwrap().waypoints, waypoints);
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = PathCache::new();
        assert!(cache.get(IVec3::ZERO, IVec3::ONE).is_none());
        assert_eq!(cache.misses(), 1);
    }

    #[test]
    fn test_cache_hit_count() {
        let mut cache = PathCache::new();
        cache.store(IVec3::ZERO, IVec3::ONE, vec![IVec3::ONE]);
        cache.get(IVec3::ZERO, IVec3::ONE);
        assert_eq!(cache.hits(), 1);
    }

    #[test]
    fn test_invalidate_near() {
        let mut cache = PathCache::new();
        let waypoints = vec![IVec3::new(5, 0, 5)];
        cache.store(IVec3::ZERO, IVec3::new(10, 0, 10), waypoints);

        // Invalidate near the path
        let count = cache.invalidate_near(IVec3::new(5, 0, 5));
        assert_eq!(count, 1);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_invalidate_far_no_effect() {
        let mut cache = PathCache::new();
        cache.store(IVec3::ZERO, IVec3::new(10, 0, 10), vec![IVec3::new(10, 0, 10)]);

        // Change far from the path
        let count = cache.invalidate_near(IVec3::new(100, 0, 100));
        assert_eq!(count, 0);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_evict_expired() {
        let mut cache = PathCache::new();
        cache.set_cache_duration(0.0); // Immediate expiry

        cache.store(IVec3::ZERO, IVec3::ONE, vec![IVec3::ONE]);

        // Small sleep to ensure time passes
        std::thread::sleep(std::time::Duration::from_millis(10));

        let evicted = cache.evict_expired();
        assert_eq!(evicted, 1);
    }

    #[test]
    fn test_max_cache_size() {
        let mut cache = PathCache::new();
        for i in 0..MAX_CACHE_SIZE + 10 {
            cache.store(
                IVec3::new(i as i32, 0, 0),
                IVec3::new(i as i32 + 1, 0, 0),
                vec![],
            );
        }
        assert!(cache.len() <= MAX_CACHE_SIZE);
    }

    #[test]
    fn test_hit_rate() {
        let mut cache = PathCache::new();
        cache.store(IVec3::ZERO, IVec3::ONE, vec![IVec3::ONE]);
        cache.get(IVec3::ZERO, IVec3::ONE); // Hit
        cache.get(IVec3::ZERO, IVec3::new(2, 0, 0)); // Miss

        assert!((cache.hit_rate() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_hit_rate_empty() {
        let cache = PathCache::new();
        assert_eq!(cache.hit_rate(), 0.0);
    }

    #[test]
    fn test_cached_path_passes_near() {
        let path = CachedPath::new(
            IVec3::ZERO,
            IVec3::new(10, 0, 10),
            vec![IVec3::new(5, 0, 5)],
        );

        assert!(path.passes_near(IVec3::new(5, 0, 5), 2));
        assert!(path.passes_near(IVec3::ZERO, 2));
        assert!(!path.passes_near(IVec3::new(100, 0, 100), 2));
    }

    #[test]
    fn test_clear() {
        let mut cache = PathCache::new();
        cache.store(IVec3::ZERO, IVec3::ONE, vec![]);
        cache.clear();
        assert!(cache.is_empty());
    }
}
