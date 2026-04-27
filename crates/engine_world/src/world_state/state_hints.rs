//! Hooks for mapping world state to environment and scheduler hints.
//!
//! This module provides decoupled hints that other systems can use without
//! depending on concrete implementations.

use serde::{Deserialize, Serialize};

use super::{ActiveEffects, Season, WorldEventKind};

/// Lighting modifier hints derived from world state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LightingHint {
    /// Light level multiplier (0.0 to 1.0).
    pub multiplier: f32,
    /// Whether shadow intensity should increase.
    pub deep_shadows: bool,
}

impl LightingHint {
    /// No lighting modification.
    pub const NORMAL: Self = Self {
        multiplier: 1.0,
        deep_shadows: false,
    };

    /// Derive lighting hints from active effects.
    #[must_use]
    pub fn from_effects(effects: &ActiveEffects) -> Self {
        let mut multiplier = 1.0f32;
        let mut deep_shadows = false;

        for effect in effects.lighting_effects() {
            if effect.kind() == WorldEventKind::Eclipse {
                let eclipse_factor = 1.0 - (effect.intensity() * 0.7);
                multiplier = multiplier.min(eclipse_factor);
                deep_shadows = true;
            }
        }

        Self {
            multiplier: multiplier.clamp(0.0, 1.0),
            deep_shadows,
        }
    }
}

/// Temperature modifier hints derived from world state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TemperatureHint {
    /// Additive temperature offset (degrees).
    pub offset: f32,
    /// Temperature change rate multiplier.
    pub rate_multiplier: f32,
}

impl TemperatureHint {
    /// No temperature modification.
    pub const NORMAL: Self = Self {
        offset: 0.0,
        rate_multiplier: 1.0,
    };

    /// Derive temperature hints from active effects and season.
    #[must_use]
    pub fn from_effects(effects: &ActiveEffects, season: Season) -> Self {
        let mut offset = season.temperature_modifier() * 10.0;
        let mut rate_multiplier = 1.0f32;

        for effect in effects.temperature_effects() {
            match effect.kind() {
                WorldEventKind::Eclipse => {
                    offset -= effect.intensity() * 5.0;
                    rate_multiplier *= 0.5;
                }
                WorldEventKind::SeasonShift => {
                    if let Some(target) = effect.target_season() {
                        let progress = effect.progress();
                        let current_mod = season.temperature_modifier() * 10.0;
                        let target_mod = target.temperature_modifier() * 10.0;
                        offset = current_mod + (target_mod - current_mod) * progress;
                    }
                }
                _ => {}
            }
        }

        Self {
            offset,
            rate_multiplier: rate_multiplier.clamp(0.1, 2.0),
        }
    }
}

/// Structural stability hints derived from world state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StructuralHint {
    /// Stability multiplier (lower = less stable).
    pub stability_multiplier: f32,
    /// Whether active collapse events are affecting the area.
    pub collapse_active: bool,
    /// Suggested priority boost for structural simulation.
    pub priority_boost: i32,
}

impl StructuralHint {
    /// No structural modification.
    pub const NORMAL: Self = Self {
        stability_multiplier: 1.0,
        collapse_active: false,
        priority_boost: 0,
    };

    /// Derive structural hints from active effects.
    #[must_use]
    pub fn from_effects(effects: &ActiveEffects) -> Self {
        let mut stability_multiplier = 1.0f32;
        let mut collapse_active = false;
        let mut priority_boost = 0i32;

        for effect in effects.structural_effects() {
            if effect.kind() == WorldEventKind::Collapse {
                stability_multiplier *= 1.0 - (effect.intensity() * 0.5);
                collapse_active = true;
                priority_boost = priority_boost.saturating_add(100);
            }
        }

        Self {
            stability_multiplier: stability_multiplier.clamp(0.1, 1.0),
            collapse_active,
            priority_boost,
        }
    }
}

/// Hazard spread hints derived from world state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HazardHint {
    /// Corruption spread rate multiplier.
    pub corruption_rate: f32,
    /// Whether biome corruption is active.
    pub corruption_active: bool,
    /// Suggested priority boost for hazard simulation.
    pub priority_boost: i32,
}

impl HazardHint {
    /// No hazard modification.
    pub const NORMAL: Self = Self {
        corruption_rate: 1.0,
        corruption_active: false,
        priority_boost: 0,
    };

    /// Derive hazard hints from active effects.
    #[must_use]
    pub fn from_effects(effects: &ActiveEffects) -> Self {
        let mut corruption_rate = 1.0f32;
        let mut corruption_active = false;
        let mut priority_boost = 0i32;

        for effect in effects.hazard_effects() {
            if effect.kind() == WorldEventKind::BiomeCorruption {
                corruption_rate += effect.intensity() * 2.0;
                corruption_active = true;
                priority_boost = priority_boost.saturating_add(50);
            }
        }

        Self {
            corruption_rate: corruption_rate.clamp(0.0, 5.0),
            corruption_active,
            priority_boost,
        }
    }
}

/// Entity behavior hints derived from world state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EntityHint {
    /// Whether nocturnal entities should be active.
    pub nocturnal_active: bool,
    /// Migration direction bias (normalized, 0 = no bias).
    pub migration_direction: (f32, f32),
    /// Spawn rate multiplier.
    pub spawn_rate: f32,
}

impl EntityHint {
    /// No entity modification.
    pub const NORMAL: Self = Self {
        nocturnal_active: false,
        migration_direction: (0.0, 0.0),
        spawn_rate: 1.0,
    };

    /// Derive entity hints from active effects.
    #[must_use]
    pub fn from_effects(effects: &ActiveEffects) -> Self {
        let mut nocturnal_active = false;
        let mut migration_direction = (0.0f32, 0.0f32);
        let mut spawn_rate = 1.0f32;

        for effect in effects.entity_effects() {
            match effect.kind() {
                WorldEventKind::Eclipse => {
                    if effect.intensity() > 0.5 {
                        nocturnal_active = true;
                    }
                }
                WorldEventKind::MigrationWave => {
                    let progress = effect.progress();
                    let wave_phase = progress * std::f32::consts::TAU;
                    migration_direction.0 += wave_phase.cos() * effect.intensity();
                    migration_direction.1 += wave_phase.sin() * effect.intensity();
                    spawn_rate *= 1.0 + effect.intensity() * 0.5;
                }
                _ => {}
            }
        }

        let mag = (migration_direction.0.powi(2) + migration_direction.1.powi(2)).sqrt();
        if mag > 0.001 {
            migration_direction.0 /= mag;
            migration_direction.1 /= mag;
        } else {
            migration_direction = (0.0, 0.0);
        }

        Self {
            nocturnal_active,
            migration_direction,
            spawn_rate: spawn_rate.clamp(0.1, 3.0),
        }
    }
}

/// Combined world state hints for all systems.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WorldStateHints {
    /// Lighting hints.
    pub lighting: LightingHint,
    /// Temperature hints.
    pub temperature: TemperatureHint,
    /// Structural stability hints.
    pub structural: StructuralHint,
    /// Hazard spread hints.
    pub hazard: HazardHint,
    /// Entity behavior hints.
    pub entity: EntityHint,
}

impl WorldStateHints {
    /// No modifications to any system.
    pub const NORMAL: Self = Self {
        lighting: LightingHint::NORMAL,
        temperature: TemperatureHint::NORMAL,
        structural: StructuralHint::NORMAL,
        hazard: HazardHint::NORMAL,
        entity: EntityHint::NORMAL,
    };

    /// Derive all hints from active effects and season.
    #[must_use]
    pub fn from_effects(effects: &ActiveEffects, season: Season) -> Self {
        Self {
            lighting: LightingHint::from_effects(effects),
            temperature: TemperatureHint::from_effects(effects, season),
            structural: StructuralHint::from_effects(effects),
            hazard: HazardHint::from_effects(effects),
            entity: EntityHint::from_effects(effects),
        }
    }

    /// Get the total priority boost from all hints.
    #[must_use]
    pub fn total_priority_boost(&self) -> i32 {
        self.structural
            .priority_boost
            .saturating_add(self.hazard.priority_boost)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_state::{ActiveEffect, WorldEvent};
    use engine_core::coords::ChunkPos;

    fn make_effects(events: &[WorldEvent], tick: u64) -> ActiveEffects {
        let pos = ChunkPos::new(0, 0, 0);
        let mut effects = ActiveEffects::new();
        for event in events {
            if let Some(effect) = ActiveEffect::from_event(event, pos, tick) {
                effects.push(effect);
            }
        }
        effects
    }

    #[test]
    fn lighting_hint_normal() {
        let effects = ActiveEffects::new();
        let hint = LightingHint::from_effects(&effects);
        assert!((hint.multiplier - 1.0).abs() < 0.001);
        assert!(!hint.deep_shadows);
    }

    #[test]
    fn lighting_hint_eclipse() {
        let event = WorldEvent::global(1, WorldEventKind::Eclipse, 0, 100);
        let effects = make_effects(&[event], 50);
        let hint = LightingHint::from_effects(&effects);

        assert!(hint.multiplier < 1.0);
        assert!(hint.deep_shadows);
    }

    #[test]
    fn temperature_hint_season() {
        let effects = ActiveEffects::new();

        let summer = TemperatureHint::from_effects(&effects, Season::Summer);
        let winter = TemperatureHint::from_effects(&effects, Season::Winter);

        assert!(summer.offset > winter.offset);
    }

    #[test]
    fn temperature_hint_eclipse() {
        let event = WorldEvent::global(1, WorldEventKind::Eclipse, 0, 100);
        let effects = make_effects(&[event], 50);
        let hint = TemperatureHint::from_effects(&effects, Season::Summer);

        assert!(hint.offset < Season::Summer.temperature_modifier() * 10.0);
        assert!(hint.rate_multiplier < 1.0);
    }

    #[test]
    fn structural_hint_collapse() {
        let event = WorldEvent::regional(
            1,
            WorldEventKind::Collapse,
            0,
            100,
            ChunkPos::new(0, 0, 0),
            10,
        );
        let effects = make_effects(&[event], 50);
        let hint = StructuralHint::from_effects(&effects);

        assert!(hint.stability_multiplier < 1.0);
        assert!(hint.collapse_active);
        assert!(hint.priority_boost > 0);
    }

    #[test]
    fn hazard_hint_corruption() {
        let event = WorldEvent::regional(
            1,
            WorldEventKind::BiomeCorruption,
            0,
            100,
            ChunkPos::new(0, 0, 0),
            10,
        );
        let effects = make_effects(&[event], 50);
        let hint = HazardHint::from_effects(&effects);

        assert!(hint.corruption_rate > 1.0);
        assert!(hint.corruption_active);
        assert!(hint.priority_boost > 0);
    }

    #[test]
    fn entity_hint_eclipse() {
        let mut event = WorldEvent::global(1, WorldEventKind::Eclipse, 0, 100);
        event.set_intensity(0.8);
        let effects = make_effects(&[event], 50);
        let hint = EntityHint::from_effects(&effects);

        assert!(hint.nocturnal_active);
    }

    #[test]
    fn entity_hint_migration() {
        let event = WorldEvent::global(1, WorldEventKind::MigrationWave, 0, 100);
        let effects = make_effects(&[event], 50);
        let hint = EntityHint::from_effects(&effects);

        assert!(hint.spawn_rate > 1.0);
        let mag = (hint.migration_direction.0.powi(2) + hint.migration_direction.1.powi(2)).sqrt();
        assert!(mag > 0.9 && mag < 1.1);
    }

    #[test]
    fn combined_hints() {
        let events = [
            WorldEvent::global(1, WorldEventKind::Eclipse, 0, 100),
            WorldEvent::regional(
                2,
                WorldEventKind::Collapse,
                0,
                100,
                ChunkPos::new(0, 0, 0),
                10,
            ),
        ];
        let effects = make_effects(&events, 50);
        let hints = WorldStateHints::from_effects(&effects, Season::Summer);

        assert!(hints.lighting.multiplier < 1.0);
        assert!(hints.structural.collapse_active);
        assert!(hints.total_priority_boost() > 0);
    }

    #[test]
    fn serde_round_trip() {
        let hints = WorldStateHints {
            lighting: LightingHint {
                multiplier: 0.5,
                deep_shadows: true,
            },
            temperature: TemperatureHint {
                offset: -5.0,
                rate_multiplier: 0.8,
            },
            structural: StructuralHint {
                stability_multiplier: 0.7,
                collapse_active: true,
                priority_boost: 100,
            },
            hazard: HazardHint {
                corruption_rate: 2.5,
                corruption_active: true,
                priority_boost: 50,
            },
            entity: EntityHint {
                nocturnal_active: true,
                migration_direction: (0.7, 0.7),
                spawn_rate: 1.5,
            },
        };

        let json = serde_json::to_string(&hints).unwrap();
        let recovered: WorldStateHints = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, hints);
    }
}
