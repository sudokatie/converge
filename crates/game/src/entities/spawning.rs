//! Creature spawning system.
//!
//! Implements spec 8.3.3: periodic spawning in loaded chunks with
//! distance rules, biome restrictions, and population caps.

use glam::Vec3;
use hecs::World;

use super::creature::{CreatureKind, spawn_creature};

/// Default spawn check interval in seconds.
pub const SPAWN_CHECK_INTERVAL: f32 = 10.0;

/// Minimum distance from player for spawning (blocks).
pub const MIN_SPAWN_DISTANCE: f32 = 24.0;

/// Maximum distance from player for spawning (blocks).
pub const MAX_SPAWN_DISTANCE: f32 = 128.0;

/// Population cap per creature type per player.
pub const POPULATION_CAP: u32 = 8;

/// Maximum spawn attempts per check cycle.
pub const MAX_SPAWN_ATTEMPTS: u32 = 10;

/// Biome type for spawn restrictions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BiomeType {
    Plains,
    Forest,
    Desert,
    Mountains,
    Ocean,
    Swamp,
    Taiga,
    Jungle,
    Nether,
    End,
}

impl BiomeType {
    /// Get which creature types can spawn in this biome.
    #[must_use]
    pub fn allowed_spawns(&self) -> &[CreatureKind] {
        match self {
            BiomeType::Plains => &[
                CreatureKind::Pig,
                CreatureKind::Cow,
                CreatureKind::Sheep,
                CreatureKind::Chicken,
                CreatureKind::Zombie,
                CreatureKind::Skeleton,
                CreatureKind::Spider,
                CreatureKind::Creeper,
            ],
            BiomeType::Forest => &[
                CreatureKind::Pig,
                CreatureKind::Cow,
                CreatureKind::Sheep,
                CreatureKind::Chicken,
                CreatureKind::Zombie,
                CreatureKind::Skeleton,
                CreatureKind::Spider,
                CreatureKind::Creeper,
            ],
            BiomeType::Desert => &[
                CreatureKind::Zombie,
                CreatureKind::Skeleton,
                CreatureKind::Spider,
                CreatureKind::Creeper,
            ],
            BiomeType::Mountains => &[
                CreatureKind::Sheep,
                CreatureKind::Chicken,
                CreatureKind::Zombie,
                CreatureKind::Skeleton,
                CreatureKind::Spider,
            ],
            BiomeType::Ocean => &[],
            BiomeType::Swamp => &[
                CreatureKind::Chicken,
                CreatureKind::Zombie,
                CreatureKind::Skeleton,
                CreatureKind::Spider,
                CreatureKind::Creeper,
            ],
            BiomeType::Taiga => &[
                CreatureKind::Sheep,
                CreatureKind::Chicken,
                CreatureKind::Zombie,
                CreatureKind::Skeleton,
                CreatureKind::Spider,
            ],
            BiomeType::Jungle => &[
                CreatureKind::Pig,
                CreatureKind::Chicken,
                CreatureKind::Zombie,
                CreatureKind::Skeleton,
                CreatureKind::Spider,
                CreatureKind::Creeper,
            ],
            BiomeType::Nether => &[],
            BiomeType::End => &[],
        }
    }
}

/// Result of a spawn attempt.
#[derive(Debug, Clone)]
pub struct SpawnResult {
    /// The creature kind that was spawned.
    pub kind: CreatureKind,
    /// The position where it was spawned.
    pub position: Vec3,
}

/// Creature spawning system.
#[derive(Debug, Clone)]
pub struct SpawnSystem {
    /// Time since last spawn check.
    time_since_check: f32,
    /// Spawn check interval.
    check_interval: f32,
    /// Population counts per creature type.
    population: std::collections::HashMap<CreatureKind, u32>,
    /// Whether spawning is enabled.
    enabled: bool,
}

impl Default for SpawnSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl SpawnSystem {
    /// Create a new spawn system.
    #[must_use]
    pub fn new() -> Self {
        Self {
            time_since_check: 0.0,
            check_interval: SPAWN_CHECK_INTERVAL,
            enabled: true,
            population: std::collections::HashMap::new(),
        }
    }

    /// Check if spawning is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable or disable spawning.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Set the spawn check interval.
    pub fn set_check_interval(&mut self, interval: f32) {
        self.check_interval = interval.max(1.0);
    }

    /// Get the population count for a creature type.
    #[must_use]
    pub fn population(&self, kind: CreatureKind) -> u32 {
        *self.population.get(&kind).unwrap_or(&0)
    }

    /// Check if a creature type has reached its population cap.
    #[must_use]
    pub fn is_at_cap(&self, kind: CreatureKind) -> bool {
        self.population(kind) >= POPULATION_CAP
    }

    /// Register a spawned creature.
    pub fn register_spawn(&mut self, kind: CreatureKind) {
        *self.population.entry(kind).or_insert(0) += 1;
    }

    /// Register a creature death/despawn.
    pub fn register_death(&mut self, kind: CreatureKind) {
        if let Some(count) = self.population.get_mut(&kind) {
            *count = count.saturating_sub(1);
        }
    }

    /// Check if a spawn position is valid.
    ///
    /// Validates distance from player.
    #[must_use]
    pub fn is_valid_spawn_position(&self, spawn_pos: Vec3, player_pos: Vec3) -> bool {
        let distance = spawn_pos.distance(player_pos);
        distance >= MIN_SPAWN_DISTANCE && distance <= MAX_SPAWN_DISTANCE
    }

    /// Check if a creature can spawn at a position in a given biome.
    #[must_use]
    pub fn can_spawn_in_biome(&self, kind: CreatureKind, biome: BiomeType) -> bool {
        biome.allowed_spawns().contains(&kind)
    }

    /// Pick a random creature to spawn based on biome and population.
    #[must_use]
    pub fn pick_creature_to_spawn(&self, biome: BiomeType) -> Option<CreatureKind> {
        let allowed = biome.allowed_spawns();
        if allowed.is_empty() {
            return None;
        }

        // Filter out types at population cap
        let available: Vec<_> = allowed
            .iter()
            .filter(|kind| !self.is_at_cap(**kind))
            .copied()
            .collect();

        if available.is_empty() {
            return None;
        }

        // Simple random selection (in production, use weighted random)
        let index = (biome as usize) % available.len();
        Some(available[index])
    }

    /// Tick the spawn system.
    ///
    /// Returns a list of spawn requests if a check cycle triggers.
    pub fn tick(
        &mut self,
        dt: f32,
        player_pos: Vec3,
        biome_at_player: BiomeType,
        candidate_positions: &[Vec3],
    ) -> Vec<SpawnResult> {
        if !self.enabled {
            return Vec::new();
        }

        self.time_since_check += dt;

        if self.time_since_check < self.check_interval {
            return Vec::new();
        }

        self.time_since_check = 0.0;

        let mut spawns = Vec::new();

        for _ in 0..MAX_SPAWN_ATTEMPTS {
            // Pick a creature type
            let Some(kind) = self.pick_creature_to_spawn(biome_at_player) else {
                break;
            };

            // Pick a valid position
            let valid_positions: Vec<_> = candidate_positions
                .iter()
                .filter(|pos| self.is_valid_spawn_position(**pos, player_pos))
                .copied()
                .collect();

            if valid_positions.is_empty() {
                continue;
            }

            // Pick a position (round-robin for simplicity)
            let idx = spawns.len() % valid_positions.len();
            let position = valid_positions[idx];

            self.register_spawn(kind);
            spawns.push(SpawnResult { kind, position });
        }

        spawns
    }

    /// Apply spawn results to the ECS world.
    pub fn apply_spawns(&mut self, world: &mut World, spawns: Vec<SpawnResult>) {
        for spawn in spawns {
            spawn_creature(world, spawn.kind, spawn.position);
        }
    }

    /// Rebuild population counts from the world.
    pub fn rebuild_population(&mut self, world: &World) {
        self.population.clear();
        for (_id, creature) in world.query::<&CreatureKind>().iter() {
            *self.population.entry(*creature).or_insert(0) += 1;
        }
    }

    /// Total number of creatures across all types.
    #[must_use]
    pub fn total_population(&self) -> u32 {
        self.population.values().sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_system_is_enabled() {
        let system = SpawnSystem::new();
        assert!(system.is_enabled());
        assert_eq!(system.time_since_check, 0.0);
    }

    #[test]
    fn test_disable_spawning() {
        let mut system = SpawnSystem::new();
        system.set_enabled(false);
        assert!(!system.is_enabled());
    }

    #[test]
    fn test_valid_spawn_distance() {
        let system = SpawnSystem::new();
        let player = Vec3::new(0.0, 0.0, 0.0);

        // Too close
        assert!(!system.is_valid_spawn_position(Vec3::new(10.0, 0.0, 0.0), player));

        // Valid
        assert!(system.is_valid_spawn_position(Vec3::new(50.0, 0.0, 0.0), player));

        // Too far
        assert!(!system.is_valid_spawn_position(Vec3::new(200.0, 0.0, 0.0), player));
    }

    #[test]
    fn test_population_cap() {
        let mut system = SpawnSystem::new();
        for _ in 0..POPULATION_CAP {
            system.register_spawn(CreatureKind::Pig);
        }
        assert!(system.is_at_cap(CreatureKind::Pig));
    }

    #[test]
    fn test_population_below_cap() {
        let system = SpawnSystem::new();
        assert!(!system.is_at_cap(CreatureKind::Pig));
    }

    #[test]
    fn test_register_death() {
        let mut system = SpawnSystem::new();
        system.register_spawn(CreatureKind::Zombie);
        system.register_spawn(CreatureKind::Zombie);
        assert_eq!(system.population(CreatureKind::Zombie), 2);

        system.register_death(CreatureKind::Zombie);
        assert_eq!(system.population(CreatureKind::Zombie), 1);
    }

    #[test]
    fn test_death_below_zero_safe() {
        let mut system = SpawnSystem::new();
        system.register_death(CreatureKind::Pig);
        assert_eq!(system.population(CreatureKind::Pig), 0);
    }

    #[test]
    fn test_biome_spawn_restrictions() {
        // Ocean has no spawns
        assert!(BiomeType::Ocean.allowed_spawns().is_empty());

        // Plains has passive and hostile
        assert!(BiomeType::Plains.allowed_spawns().contains(&CreatureKind::Pig));
        assert!(BiomeType::Plains.allowed_spawns().contains(&CreatureKind::Zombie));

        // Desert has no passive animals
        assert!(!BiomeType::Desert.allowed_spawns().contains(&CreatureKind::Pig));
        assert!(BiomeType::Desert.allowed_spawns().contains(&CreatureKind::Skeleton));
    }

    #[test]
    fn test_can_spawn_in_biome() {
        let system = SpawnSystem::new();
        assert!(system.can_spawn_in_biome(CreatureKind::Pig, BiomeType::Plains));
        assert!(!system.can_spawn_in_biome(CreatureKind::Pig, BiomeType::Desert));
    }

    #[test]
    fn test_tick_before_interval() {
        let mut system = SpawnSystem::new();
        let spawns = system.tick(5.0, Vec3::ZERO, BiomeType::Plains, &[]);
        assert!(spawns.is_empty());
    }

    #[test]
    fn test_tick_at_interval() {
        let mut system = SpawnSystem::new();
        system.set_check_interval(10.0);
        let spawns = system.tick(10.0, Vec3::ZERO, BiomeType::Plains, &[Vec3::new(50.0, 64.0, 50.0)]);
        // Should attempt spawns (results depend on population)
        assert!(system.time_since_check < 1.0); // Reset after check
    }

    #[test]
    fn test_tick_disabled() {
        let mut system = SpawnSystem::new();
        system.set_enabled(false);
        let spawns = system.tick(15.0, Vec3::ZERO, BiomeType::Plains, &[]);
        assert!(spawns.is_empty());
    }

    #[test]
    fn test_total_population() {
        let mut system = SpawnSystem::new();
        system.register_spawn(CreatureKind::Pig);
        system.register_spawn(CreatureKind::Pig);
        system.register_spawn(CreatureKind::Zombie);
        assert_eq!(system.total_population(), 3);
    }

    #[test]
    fn test_custom_check_interval() {
        let mut system = SpawnSystem::new();
        system.set_check_interval(0.5); // Too low, should clamp
        assert!(system.check_interval >= 1.0);
    }
}
