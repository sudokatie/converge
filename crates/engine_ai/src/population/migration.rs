//! Migration waves and routes between regions.

use super::species::SpeciesCapId;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;

/// Identifier for a migration route.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MigrationRouteId(pub String);

impl MigrationRouteId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for MigrationRouteId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

/// Identifier for a migration wave.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MigrationWaveId(pub u64);

impl MigrationWaveId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Phase of a migration wave.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MigrationPhase {
    /// Migration is scheduled but not started.
    Pending,
    /// Migration is currently departing source.
    Departing,
    /// Migration is in transit.
    InTransit,
    /// Migration is arriving at destination.
    Arriving,
    /// Migration has completed.
    Completed,
    /// Migration was cancelled.
    Cancelled,
}

impl MigrationPhase {
    /// Check if migration is active.
    #[must_use]
    pub fn is_active(self) -> bool {
        matches!(self, Self::Departing | Self::InTransit | Self::Arriving)
    }

    /// Check if migration is finished.
    #[must_use]
    pub fn is_finished(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled)
    }
}

/// Status of a migration route.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MigrationStatus {
    /// Route is open for migrations.
    #[default]
    Open,
    /// Route is closed (blocked, dangerous, etc.).
    Closed,
    /// Route is congested (limited capacity).
    Congested,
    /// Route is seasonal (only available at certain times).
    Seasonal,
}

/// A migration route between regions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MigrationRoute {
    /// Route identifier.
    pub id: MigrationRouteId,
    /// Source region identifier.
    pub source_region: String,
    /// Destination region identifier.
    pub destination_region: String,
    /// Travel time in ticks.
    pub travel_ticks: u64,
    /// Maximum capacity per wave.
    pub max_capacity: u32,
    /// Current status.
    status: MigrationStatus,
    /// Species that can use this route.
    allowed_species: Vec<SpeciesCapId>,
    /// Whether route is bidirectional.
    pub bidirectional: bool,
    /// Danger level (0.0-1.0, affects attrition).
    pub danger_level: f32,
}

impl MigrationRoute {
    /// Create a new migration route.
    #[must_use]
    pub fn new(
        id: MigrationRouteId,
        source: impl Into<String>,
        destination: impl Into<String>,
    ) -> Self {
        Self {
            id,
            source_region: source.into(),
            destination_region: destination.into(),
            travel_ticks: 100,
            max_capacity: 50,
            status: MigrationStatus::Open,
            allowed_species: Vec::new(),
            bidirectional: true,
            danger_level: 0.0,
        }
    }

    #[must_use]
    pub fn with_travel_ticks(mut self, ticks: u64) -> Self {
        self.travel_ticks = ticks.max(1);
        self
    }

    #[must_use]
    pub fn with_capacity(mut self, capacity: u32) -> Self {
        self.max_capacity = capacity;
        self
    }

    #[must_use]
    pub fn with_status(mut self, status: MigrationStatus) -> Self {
        self.status = status;
        self
    }

    #[must_use]
    pub fn with_allowed_species(mut self, species: SpeciesCapId) -> Self {
        if !self.allowed_species.contains(&species) {
            self.allowed_species.push(species);
            self.allowed_species.sort();
        }
        self
    }

    #[must_use]
    pub fn with_bidirectional(mut self, bidirectional: bool) -> Self {
        self.bidirectional = bidirectional;
        self
    }

    #[must_use]
    pub fn with_danger_level(mut self, danger: f32) -> Self {
        self.danger_level = danger.clamp(0.0, 1.0);
        self
    }

    /// Get current status.
    #[must_use]
    pub fn status(&self) -> MigrationStatus {
        self.status
    }

    /// Set status.
    pub fn set_status(&mut self, status: MigrationStatus) {
        self.status = status;
    }

    /// Check if route is usable.
    #[must_use]
    pub fn is_usable(&self) -> bool {
        matches!(
            self.status,
            MigrationStatus::Open | MigrationStatus::Congested
        )
    }

    /// Check if species can use this route.
    #[must_use]
    pub fn allows_species(&self, species: &SpeciesCapId) -> bool {
        self.allowed_species.is_empty() || self.allowed_species.contains(species)
    }

    /// Get effective capacity (reduced if congested).
    #[must_use]
    pub fn effective_capacity(&self) -> u32 {
        match self.status {
            MigrationStatus::Congested => self.max_capacity / 2,
            MigrationStatus::Closed | MigrationStatus::Seasonal => 0,
            MigrationStatus::Open => self.max_capacity,
        }
    }

    /// Calculate expected attrition for a wave.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "attrition is bounded by count"
    )]
    pub fn expected_attrition(&self, count: u32) -> u32 {
        (count as f32 * self.danger_level * 0.2) as u32
    }

    /// Get allowed species list.
    pub fn allowed_species(&self) -> &[SpeciesCapId] {
        &self.allowed_species
    }
}

/// Configuration for migration system.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MigrationConfig {
    /// Minimum ticks between waves on same route.
    pub wave_cooldown: u64,
    /// Whether to enable automatic migrations.
    pub auto_migrate: bool,
    /// Population pressure threshold to trigger auto-migration.
    pub pressure_threshold: f32,
    /// Maximum concurrent waves per route.
    pub max_concurrent_waves: u32,
}

impl MigrationConfig {
    /// Create a new migration configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_wave_cooldown(mut self, ticks: u64) -> Self {
        self.wave_cooldown = ticks;
        self
    }

    #[must_use]
    pub fn with_auto_migrate(mut self, enabled: bool) -> Self {
        self.auto_migrate = enabled;
        self
    }

    #[must_use]
    pub fn with_pressure_threshold(mut self, threshold: f32) -> Self {
        self.pressure_threshold = threshold.clamp(0.0, 1.0);
        self
    }
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            wave_cooldown: 300,
            auto_migrate: true,
            pressure_threshold: 0.8,
            max_concurrent_waves: 3,
        }
    }
}

/// A migration wave in progress.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MigrationWave {
    /// Wave identifier.
    pub id: MigrationWaveId,
    /// Route being used.
    pub route_id: MigrationRouteId,
    /// Species migrating.
    pub species: SpeciesCapId,
    /// Number of entities migrating.
    pub count: u32,
    /// Current phase.
    phase: MigrationPhase,
    /// Tick when wave started.
    pub start_tick: u64,
    /// Tick when wave will arrive.
    pub arrival_tick: u64,
    /// Number lost to attrition.
    pub attrition: u32,
    /// Progress through transit (0.0-1.0).
    progress: f32,
}

impl MigrationWave {
    /// Create a new migration wave.
    #[must_use]
    pub fn new(
        id: MigrationWaveId,
        route: &MigrationRoute,
        species: SpeciesCapId,
        count: u32,
        current_tick: u64,
    ) -> Self {
        Self {
            id,
            route_id: route.id.clone(),
            species,
            count: count.min(route.effective_capacity()),
            phase: MigrationPhase::Pending,
            start_tick: current_tick,
            arrival_tick: current_tick + route.travel_ticks,
            attrition: 0,
            progress: 0.0,
        }
    }

    /// Get current phase.
    #[must_use]
    pub fn phase(&self) -> MigrationPhase {
        self.phase
    }

    /// Get progress (0.0-1.0).
    #[must_use]
    pub fn progress(&self) -> f32 {
        self.progress
    }

    /// Get number that will arrive (after attrition).
    #[must_use]
    pub fn arriving_count(&self) -> u32 {
        self.count.saturating_sub(self.attrition)
    }

    /// Start the migration.
    pub fn start(&mut self) {
        self.phase = MigrationPhase::Departing;
    }

    /// Cancel the migration.
    pub fn cancel(&mut self) {
        self.phase = MigrationPhase::Cancelled;
    }

    /// Update wave for a tick.
    #[expect(clippy::cast_precision_loss, reason = "tick values are bounded")]
    pub fn tick(&mut self, current_tick: u64, route_danger: f32) {
        if self.phase.is_finished() {
            return;
        }

        if self.phase == MigrationPhase::Pending {
            return;
        }

        let total_ticks = self.arrival_tick.saturating_sub(self.start_tick);
        let elapsed = current_tick.saturating_sub(self.start_tick);

        if total_ticks > 0 {
            self.progress = (elapsed as f32 / total_ticks as f32).clamp(0.0, 1.0);
        } else {
            self.progress = 1.0;
        }

        self.phase = if self.progress < 0.1 {
            MigrationPhase::Departing
        } else if self.progress < 0.9 {
            MigrationPhase::InTransit
        } else if current_tick < self.arrival_tick {
            MigrationPhase::Arriving
        } else {
            MigrationPhase::Completed
        };

        if self.phase == MigrationPhase::InTransit && route_danger > 0.0 {
            self.apply_attrition(route_danger);
        }
    }

    /// Apply attrition based on danger.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "attrition is bounded by remaining count"
    )]
    fn apply_attrition(&mut self, danger: f32) {
        let remaining = self.count.saturating_sub(self.attrition);
        let chance = danger * 0.02;
        let loss = (remaining as f32 * chance) as u32;
        self.attrition = self.attrition.saturating_add(loss);
    }
}

impl Eq for MigrationWave {}

impl PartialOrd for MigrationWave {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MigrationWave {
    fn cmp(&self, other: &Self) -> Ordering {
        self.arrival_tick
            .cmp(&other.arrival_tick)
            .then_with(|| self.id.cmp(&other.id))
    }
}

/// Manager for migration routes and waves.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MigrationManager {
    routes: BTreeMap<MigrationRouteId, MigrationRoute>,
    waves: Vec<MigrationWave>,
    config: MigrationConfig,
    route_cooldowns: BTreeMap<MigrationRouteId, u64>,
    next_wave_id: u64,
    current_tick: u64,
}

impl MigrationManager {
    /// Create a new migration manager.
    #[must_use]
    pub fn new(config: MigrationConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    /// Register a route.
    pub fn register_route(&mut self, route: MigrationRoute) {
        self.routes.insert(route.id.clone(), route);
    }

    /// Get a route.
    #[must_use]
    pub fn get_route(&self, id: &MigrationRouteId) -> Option<&MigrationRoute> {
        self.routes.get(id)
    }

    /// Get mutable route.
    pub fn get_route_mut(&mut self, id: &MigrationRouteId) -> Option<&mut MigrationRoute> {
        self.routes.get_mut(id)
    }

    /// Get all routes (deterministic order).
    pub fn routes(&self) -> impl Iterator<Item = &MigrationRoute> {
        self.routes.values()
    }

    /// Get routes from a region.
    pub fn routes_from(&self, region: &str) -> impl Iterator<Item = &MigrationRoute> {
        self.routes.values().filter(move |r| {
            r.source_region == region || (r.bidirectional && r.destination_region == region)
        })
    }

    /// Check if route is on cooldown.
    #[must_use]
    pub fn is_route_on_cooldown(&self, route_id: &MigrationRouteId) -> bool {
        self.route_cooldowns
            .get(route_id)
            .is_some_and(|&cd| self.current_tick < cd)
    }

    /// Get active waves.
    pub fn active_waves(&self) -> impl Iterator<Item = &MigrationWave> {
        self.waves.iter().filter(|w| w.phase().is_active())
    }

    /// Get waves on a route.
    pub fn waves_on_route(
        &self,
        route_id: &MigrationRouteId,
    ) -> impl Iterator<Item = &MigrationWave> {
        self.waves.iter().filter(move |w| &w.route_id == route_id)
    }

    /// Count active waves on a route.
    #[must_use]
    pub fn active_wave_count(&self, route_id: &MigrationRouteId) -> usize {
        self.waves
            .iter()
            .filter(|w| &w.route_id == route_id && w.phase().is_active())
            .count()
    }

    /// Check if a migration can be started.
    #[must_use]
    pub fn can_start_migration(&self, route_id: &MigrationRouteId, species: &SpeciesCapId) -> bool {
        let Some(route) = self.routes.get(route_id) else {
            return false;
        };

        if !route.is_usable() {
            return false;
        }

        if !route.allows_species(species) {
            return false;
        }

        if self.is_route_on_cooldown(route_id) {
            return false;
        }

        let active_count = self.active_wave_count(route_id);
        active_count < self.config.max_concurrent_waves as usize
    }

    /// Start a migration wave.
    pub fn start_migration(
        &mut self,
        route_id: &MigrationRouteId,
        species: SpeciesCapId,
        count: u32,
    ) -> Option<MigrationWaveId> {
        if !self.can_start_migration(route_id, &species) {
            return None;
        }

        let route = self.routes.get(route_id)?;
        let wave_id = MigrationWaveId::new(self.next_wave_id);
        self.next_wave_id += 1;

        let mut wave =
            MigrationWave::new(wave_id.clone(), route, species, count, self.current_tick);
        wave.start();

        let cooldown_end = self.current_tick + self.config.wave_cooldown;
        self.route_cooldowns.insert(route_id.clone(), cooldown_end);

        self.waves.push(wave);
        self.waves.sort();

        Some(wave_id)
    }

    /// Cancel a wave.
    pub fn cancel_wave(&mut self, wave_id: &MigrationWaveId) -> bool {
        if let Some(wave) = self.waves.iter_mut().find(|w| &w.id == wave_id) {
            wave.cancel();
            true
        } else {
            false
        }
    }

    /// Tick the migration system.
    pub fn tick(&mut self) -> Vec<MigrationWave> {
        self.current_tick += 1;

        let mut completed = Vec::new();

        for wave in &mut self.waves {
            let danger = self
                .routes
                .get(&wave.route_id)
                .map_or(0.0, |r| r.danger_level);
            wave.tick(self.current_tick, danger);
        }

        let current_tick = self.current_tick;
        self.waves.retain(|w| {
            if w.phase() == MigrationPhase::Completed {
                completed.push(w.clone());
                false
            } else {
                w.phase() != MigrationPhase::Cancelled
            }
        });

        self.route_cooldowns.retain(|_, &mut cd| cd > current_tick);

        completed
    }

    /// Get current tick.
    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Get configuration.
    #[must_use]
    pub fn config(&self) -> &MigrationConfig {
        &self.config
    }

    /// Get number of routes.
    #[must_use]
    pub fn route_count(&self) -> usize {
        self.routes.len()
    }

    /// Get number of active waves.
    #[must_use]
    pub fn wave_count(&self) -> usize {
        self.waves.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_route(id: &str) -> MigrationRoute {
        MigrationRoute::new(MigrationRouteId::new(id), "region_a", "region_b")
            .with_travel_ticks(100)
    }

    #[test]
    fn test_migration_route_id() {
        let id = MigrationRouteId::new("forest_path");
        assert_eq!(id.as_str(), "forest_path");

        let id2: MigrationRouteId = "mountain_pass".into();
        assert_eq!(id2.as_str(), "mountain_pass");
    }

    #[test]
    fn test_migration_phase_is_active() {
        assert!(!MigrationPhase::Pending.is_active());
        assert!(MigrationPhase::Departing.is_active());
        assert!(MigrationPhase::InTransit.is_active());
        assert!(MigrationPhase::Arriving.is_active());
        assert!(!MigrationPhase::Completed.is_active());
        assert!(!MigrationPhase::Cancelled.is_active());
    }

    #[test]
    fn test_migration_route_new() {
        let route = make_route("forest_path");

        assert_eq!(route.source_region, "region_a");
        assert_eq!(route.destination_region, "region_b");
        assert!(route.is_usable());
    }

    #[test]
    fn test_migration_route_status() {
        let mut route = make_route("test");

        route.set_status(MigrationStatus::Closed);
        assert!(!route.is_usable());

        route.set_status(MigrationStatus::Congested);
        assert!(route.is_usable());
        assert!(route.effective_capacity() < route.max_capacity);
    }

    #[test]
    fn test_migration_route_allowed_species() {
        let route = make_route("test").with_allowed_species(SpeciesCapId::new("deer"));

        assert!(route.allows_species(&SpeciesCapId::new("deer")));
        assert!(!route.allows_species(&SpeciesCapId::new("wolf")));

        let open_route = make_route("open");
        assert!(open_route.allows_species(&SpeciesCapId::new("anything")));
    }

    #[test]
    fn test_migration_route_attrition() {
        let route = make_route("dangerous").with_danger_level(0.5);

        let attrition = route.expected_attrition(100);
        assert!(attrition > 0);
        assert!(attrition < 100);
    }

    #[test]
    fn test_migration_config_defaults() {
        let config = MigrationConfig::new();

        assert!(config.auto_migrate);
        assert!(config.wave_cooldown > 0);
    }

    #[test]
    fn test_migration_wave_new() {
        let route = make_route("test");
        let wave = MigrationWave::new(
            MigrationWaveId::new(1),
            &route,
            SpeciesCapId::new("deer"),
            30,
            0,
        );

        assert_eq!(wave.count, 30);
        assert_eq!(wave.phase(), MigrationPhase::Pending);
        assert!((wave.progress()).abs() < f32::EPSILON);
    }

    #[test]
    fn test_migration_wave_lifecycle() {
        let route = make_route("test").with_travel_ticks(100);
        let mut wave = MigrationWave::new(
            MigrationWaveId::new(1),
            &route,
            SpeciesCapId::new("deer"),
            30,
            0,
        );

        wave.start();
        assert_eq!(wave.phase(), MigrationPhase::Departing);

        wave.tick(50, 0.0);
        assert_eq!(wave.phase(), MigrationPhase::InTransit);

        wave.tick(95, 0.0);
        assert_eq!(wave.phase(), MigrationPhase::Arriving);

        wave.tick(100, 0.0);
        assert_eq!(wave.phase(), MigrationPhase::Completed);
    }

    #[test]
    fn test_migration_wave_attrition() {
        let route = make_route("test")
            .with_travel_ticks(100)
            .with_danger_level(1.0);
        let mut wave = MigrationWave::new(
            MigrationWaveId::new(1),
            &route,
            SpeciesCapId::new("deer"),
            100,
            0,
        );

        wave.start();

        for tick in 1..100 {
            wave.tick(tick, 1.0);
        }

        assert!(wave.attrition > 0);
        assert!(wave.arriving_count() < 100);
    }

    #[test]
    fn test_migration_manager_new() {
        let manager = MigrationManager::new(MigrationConfig::new());

        assert_eq!(manager.route_count(), 0);
        assert_eq!(manager.wave_count(), 0);
    }

    #[test]
    fn test_migration_manager_register_route() {
        let mut manager = MigrationManager::new(MigrationConfig::new());

        manager.register_route(make_route("forest_path"));

        assert_eq!(manager.route_count(), 1);
        assert!(
            manager
                .get_route(&MigrationRouteId::new("forest_path"))
                .is_some()
        );
    }

    #[test]
    fn test_migration_manager_start_migration() {
        let mut manager = MigrationManager::new(MigrationConfig::new());
        manager.register_route(make_route("test"));

        let wave_id = manager.start_migration(
            &MigrationRouteId::new("test"),
            SpeciesCapId::new("deer"),
            30,
        );

        assert!(wave_id.is_some());
        assert_eq!(manager.wave_count(), 1);
    }

    #[test]
    fn test_migration_manager_cooldown() {
        let config = MigrationConfig::new().with_wave_cooldown(100);
        let mut manager = MigrationManager::new(config);
        manager.register_route(make_route("test"));

        manager.start_migration(
            &MigrationRouteId::new("test"),
            SpeciesCapId::new("deer"),
            10,
        );

        assert!(manager.is_route_on_cooldown(&MigrationRouteId::new("test")));

        let result = manager.start_migration(
            &MigrationRouteId::new("test"),
            SpeciesCapId::new("deer"),
            10,
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_migration_manager_tick_completes_waves() {
        let mut manager = MigrationManager::new(MigrationConfig::new());
        manager.register_route(make_route("test").with_travel_ticks(10));

        manager.start_migration(
            &MigrationRouteId::new("test"),
            SpeciesCapId::new("deer"),
            20,
        );

        for _ in 0..15 {
            let completed = manager.tick();
            if !completed.is_empty() {
                assert_eq!(completed[0].arriving_count(), 20);
                return;
            }
        }

        panic!("Wave should have completed");
    }

    #[test]
    fn test_migration_manager_cancel_wave() {
        let mut manager = MigrationManager::new(MigrationConfig::new());
        manager.register_route(make_route("test"));

        let wave_id = manager
            .start_migration(
                &MigrationRouteId::new("test"),
                SpeciesCapId::new("deer"),
                20,
            )
            .unwrap();

        assert!(manager.cancel_wave(&wave_id));

        manager.tick();
        assert_eq!(manager.wave_count(), 0);
    }

    #[test]
    fn test_migration_manager_max_concurrent() {
        let config = MigrationConfig::new().with_wave_cooldown(0);
        let mut config_with_max = config.clone();
        config_with_max.max_concurrent_waves = 2;

        let mut manager = MigrationManager::new(config_with_max);
        manager.register_route(make_route("test").with_travel_ticks(1000));

        manager.start_migration(
            &MigrationRouteId::new("test"),
            SpeciesCapId::new("deer"),
            10,
        );
        manager.start_migration(
            &MigrationRouteId::new("test"),
            SpeciesCapId::new("deer"),
            10,
        );

        let third = manager.start_migration(
            &MigrationRouteId::new("test"),
            SpeciesCapId::new("deer"),
            10,
        );
        assert!(third.is_none());
    }

    #[test]
    fn test_migration_manager_routes_from() {
        let mut manager = MigrationManager::new(MigrationConfig::new());

        manager.register_route(
            MigrationRoute::new(MigrationRouteId::new("a_to_b"), "region_a", "region_b")
                .with_bidirectional(true),
        );
        manager.register_route(
            MigrationRoute::new(MigrationRouteId::new("a_to_c"), "region_a", "region_c")
                .with_bidirectional(false),
        );

        let from_a: Vec<_> = manager.routes_from("region_a").collect();
        assert_eq!(from_a.len(), 2);

        let from_b: Vec<_> = manager.routes_from("region_b").collect();
        assert_eq!(from_b.len(), 1);
    }

    #[test]
    fn test_serde_migration_route() {
        let route = make_route("test")
            .with_allowed_species(SpeciesCapId::new("deer"))
            .with_danger_level(0.3);

        let json = serde_json::to_string(&route).unwrap();
        let restored: MigrationRoute = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id.as_str(), "test");
        assert!((restored.danger_level - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_serde_migration_wave() {
        let route = make_route("test");
        let wave = MigrationWave::new(
            MigrationWaveId::new(42),
            &route,
            SpeciesCapId::new("deer"),
            25,
            100,
        );

        let json = serde_json::to_string(&wave).unwrap();
        let restored: MigrationWave = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, MigrationWaveId::new(42));
        assert_eq!(restored.count, 25);
    }

    #[test]
    fn test_serde_migration_manager() {
        let mut manager = MigrationManager::new(MigrationConfig::new());
        manager.register_route(make_route("test"));

        let json = serde_json::to_string(&manager).unwrap();
        let restored: MigrationManager = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.route_count(), 1);
    }

    #[test]
    fn test_migration_deterministic_order() {
        let mut manager = MigrationManager::new(MigrationConfig::new().with_wave_cooldown(0));
        manager.register_route(make_route("route_z"));
        manager.register_route(make_route("route_a"));
        manager.register_route(make_route("route_m"));

        let ids: Vec<_> = manager.routes().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["route_a", "route_m", "route_z"]);
    }
}
