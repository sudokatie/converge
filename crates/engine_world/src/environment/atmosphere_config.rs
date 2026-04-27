//! Configuration and transition rules for atmosphere simulation.

use serde::{Deserialize, Serialize};

use super::{AtmosphereCell, AtmosphereLayer, MaterialProperties};

/// Configuration for atmosphere layer behavior and transitions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AtmosphereConfig {
    /// Regulated temperature for indoor spaces (Celsius-like units).
    pub indoor_temperature: f32,

    /// Temperature blend rate per tick for transitioning cells.
    pub temperature_blend_rate: f32,

    /// Seal degradation rate when adjacent to vacuum (per tick).
    pub vacuum_seal_degradation: f32,

    /// Contamination decay rate per tick.
    pub contamination_decay: f32,

    /// Contamination spread rate to adjacent cells.
    pub contamination_spread: f32,

    /// Minimum seal quality to maintain indoor classification.
    pub min_indoor_seal: f32,

    /// Ventilation threshold below which air quality degrades.
    pub min_ventilation: f32,
}

impl Default for AtmosphereConfig {
    fn default() -> Self {
        Self {
            indoor_temperature: 20.0,
            temperature_blend_rate: 0.1,
            vacuum_seal_degradation: 0.01,
            contamination_decay: 0.05,
            contamination_spread: 0.02,
            min_indoor_seal: 0.5,
            min_ventilation: 0.1,
        }
    }
}

impl AtmosphereConfig {
    /// Create a config optimized for space/vacuum environments.
    #[must_use]
    pub fn space() -> Self {
        Self {
            indoor_temperature: 20.0,
            temperature_blend_rate: 0.05,
            vacuum_seal_degradation: 0.02,
            contamination_decay: 0.1,
            contamination_spread: 0.01,
            min_indoor_seal: 0.9,
            min_ventilation: 0.05,
        }
    }

    /// Create a config for underground/cave environments.
    #[must_use]
    pub fn underground() -> Self {
        Self {
            indoor_temperature: 15.0,
            temperature_blend_rate: 0.02,
            vacuum_seal_degradation: 0.0,
            contamination_decay: 0.02,
            contamination_spread: 0.05,
            min_indoor_seal: 0.3,
            min_ventilation: 0.2,
        }
    }
}

/// Layer transition rules determining when cells change classification.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TransitionRules {
    /// Number of adjacent indoor cells required to become indoor.
    pub indoor_threshold: u8,

    /// Number of adjacent outdoor cells required to become outdoor.
    pub outdoor_threshold: u8,

    /// Number of adjacent vacuum cells required to become vacuum.
    pub vacuum_threshold: u8,

    /// Whether vacuum can propagate through sealed cells.
    pub vacuum_breaches_seals: bool,
}

impl Default for TransitionRules {
    fn default() -> Self {
        Self {
            indoor_threshold: 4,
            outdoor_threshold: 3,
            vacuum_threshold: 2,
            vacuum_breaches_seals: false,
        }
    }
}

impl TransitionRules {
    /// Create strict rules where transitions require more neighbors.
    #[must_use]
    pub fn strict() -> Self {
        Self {
            indoor_threshold: 5,
            outdoor_threshold: 4,
            vacuum_threshold: 3,
            vacuum_breaches_seals: false,
        }
    }

    /// Create permissive rules for rapid atmosphere changes.
    #[must_use]
    pub fn permissive() -> Self {
        Self {
            indoor_threshold: 3,
            outdoor_threshold: 2,
            vacuum_threshold: 1,
            vacuum_breaches_seals: true,
        }
    }

    /// Evaluate what layer a cell should transition to based on neighbor counts.
    ///
    /// Returns `Some(layer)` if transition should occur, `None` otherwise.
    #[must_use]
    pub fn evaluate_transition(
        &self,
        current: AtmosphereLayer,
        neighbors: &LayerNeighborCounts,
    ) -> Option<AtmosphereLayer> {
        if current == AtmosphereLayer::Vacuum {
            if neighbors.outdoor >= self.outdoor_threshold {
                return Some(AtmosphereLayer::Outdoor);
            }
            return None;
        }

        if neighbors.vacuum >= self.vacuum_threshold {
            return Some(AtmosphereLayer::Vacuum);
        }

        match current {
            AtmosphereLayer::Outdoor => {
                if neighbors.indoor >= self.indoor_threshold {
                    Some(AtmosphereLayer::Indoor)
                } else {
                    None
                }
            }
            AtmosphereLayer::Indoor => {
                if neighbors.outdoor >= self.outdoor_threshold {
                    Some(AtmosphereLayer::Exposed)
                } else {
                    None
                }
            }
            AtmosphereLayer::Exposed => {
                if neighbors.indoor >= self.indoor_threshold {
                    Some(AtmosphereLayer::Indoor)
                } else if neighbors.outdoor >= self.outdoor_threshold {
                    Some(AtmosphereLayer::Outdoor)
                } else {
                    None
                }
            }
            AtmosphereLayer::Vacuum => None,
        }
    }
}

/// Count of neighboring cells by layer type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LayerNeighborCounts {
    pub indoor: u8,
    pub outdoor: u8,
    pub exposed: u8,
    pub vacuum: u8,
}

impl LayerNeighborCounts {
    /// Create counts from an array indexed by layer.
    #[must_use]
    pub fn from_array(counts: [u8; AtmosphereLayer::COUNT]) -> Self {
        Self {
            indoor: counts[AtmosphereLayer::Indoor.as_index()],
            outdoor: counts[AtmosphereLayer::Outdoor.as_index()],
            exposed: counts[AtmosphereLayer::Exposed.as_index()],
            vacuum: counts[AtmosphereLayer::Vacuum.as_index()],
        }
    }

    /// Get total neighbor count.
    #[must_use]
    pub const fn total(&self) -> u8 {
        self.indoor + self.outdoor + self.exposed + self.vacuum
    }

    /// Get the dominant layer (most common among neighbors).
    #[must_use]
    pub fn dominant(&self) -> AtmosphereLayer {
        let mut max = self.indoor;
        let mut layer = AtmosphereLayer::Indoor;

        if self.outdoor > max {
            max = self.outdoor;
            layer = AtmosphereLayer::Outdoor;
        }
        if self.exposed > max {
            max = self.exposed;
            layer = AtmosphereLayer::Exposed;
        }
        if self.vacuum > max {
            layer = AtmosphereLayer::Vacuum;
        }

        layer
    }
}

/// Derive atmosphere layer from structural material properties.
pub fn layer_from_material(
    props: &MaterialProperties,
    above_ground: bool,
    has_ceiling: bool,
) -> AtmosphereLayer {
    if !above_ground {
        if props.is_airtight() {
            AtmosphereLayer::Indoor
        } else {
            AtmosphereLayer::Exposed
        }
    } else if has_ceiling {
        if props.is_airtight() {
            AtmosphereLayer::Indoor
        } else {
            AtmosphereLayer::Exposed
        }
    } else {
        AtmosphereLayer::Outdoor
    }
}

/// Create an atmosphere cell appropriate for a given material context.
pub fn cell_from_material(
    props: &MaterialProperties,
    above_ground: bool,
    has_ceiling: bool,
) -> AtmosphereCell {
    let layer = layer_from_material(props, above_ground, has_ceiling);
    match layer {
        AtmosphereLayer::Indoor => AtmosphereCell::indoor(props.airtightness()),
        AtmosphereLayer::Outdoor => AtmosphereCell::outdoor(),
        AtmosphereLayer::Exposed => AtmosphereCell::exposed(),
        AtmosphereLayer::Vacuum => AtmosphereCell::vacuum(),
    }
}

/// Environmental effects to apply based on atmosphere state.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AtmosphereEffects {
    /// Temperature modifier (additive, in temperature units).
    pub temperature_delta: f32,
    /// Oxygen multiplier (0.0-1.0).
    pub oxygen_factor: f32,
    /// Pressure multiplier (0.0-1.0).
    pub pressure_factor: f32,
    /// Radiation multiplier (0.0-1.0+).
    pub radiation_factor: f32,
    /// Movement speed multiplier.
    pub movement_factor: f32,
}

impl AtmosphereEffects {
    /// Standard outdoor effects (no modifications).
    #[must_use]
    pub const fn outdoor() -> Self {
        Self {
            temperature_delta: 0.0,
            oxygen_factor: 1.0,
            pressure_factor: 1.0,
            radiation_factor: 1.0,
            movement_factor: 1.0,
        }
    }

    /// Standard indoor effects.
    #[must_use]
    pub const fn indoor() -> Self {
        Self {
            temperature_delta: 0.0,
            oxygen_factor: 1.0,
            pressure_factor: 1.0,
            radiation_factor: 0.0,
            movement_factor: 1.0,
        }
    }

    /// Vacuum effects (no atmosphere, high radiation).
    #[must_use]
    pub const fn vacuum() -> Self {
        Self {
            temperature_delta: 0.0,
            oxygen_factor: 0.0,
            pressure_factor: 0.0,
            radiation_factor: 1.5,
            movement_factor: 0.5,
        }
    }

    /// Calculate effects for a specific atmosphere cell.
    #[must_use]
    pub fn from_cell(cell: &AtmosphereCell, config: &AtmosphereConfig) -> Self {
        let base = match cell.layer() {
            AtmosphereLayer::Indoor => Self::indoor(),
            AtmosphereLayer::Outdoor => Self::outdoor(),
            AtmosphereLayer::Exposed => Self {
                temperature_delta: 0.0,
                oxygen_factor: 1.0,
                pressure_factor: 1.0,
                radiation_factor: 0.3,
                movement_factor: 1.0,
            },
            AtmosphereLayer::Vacuum => Self::vacuum(),
        };

        let temp_blend = cell.temperature_blend();
        let temp_delta = if temp_blend > 0.0 {
            (config.indoor_temperature - 20.0) * temp_blend
        } else {
            0.0
        };

        let contamination_penalty = cell.contamination() * 0.2;

        Self {
            temperature_delta: base.temperature_delta + temp_delta,
            oxygen_factor: (base.oxygen_factor - contamination_penalty).max(0.0),
            pressure_factor: base.pressure_factor,
            radiation_factor: base.radiation_factor,
            movement_factor: base.movement_factor,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = AtmosphereConfig::default();
        assert_eq!(config.indoor_temperature, 20.0);
        assert!(config.min_indoor_seal > 0.0 && config.min_indoor_seal < 1.0);
    }

    #[test]
    fn space_config() {
        let config = AtmosphereConfig::space();
        assert!(config.min_indoor_seal > AtmosphereConfig::default().min_indoor_seal);
        assert!(config.vacuum_seal_degradation > 0.0);
    }

    #[test]
    fn underground_config() {
        let config = AtmosphereConfig::underground();
        assert!(config.indoor_temperature < 20.0);
        assert_eq!(config.vacuum_seal_degradation, 0.0);
    }

    #[test]
    fn default_transition_rules() {
        let rules = TransitionRules::default();
        assert!(rules.indoor_threshold > 0);
        assert!(rules.outdoor_threshold > 0);
        assert!(!rules.vacuum_breaches_seals);
    }

    #[test]
    fn transition_to_vacuum() {
        let rules = TransitionRules::default();
        let neighbors = LayerNeighborCounts {
            vacuum: 3,
            ..Default::default()
        };

        let result = rules.evaluate_transition(AtmosphereLayer::Outdoor, &neighbors);
        assert_eq!(result, Some(AtmosphereLayer::Vacuum));
    }

    #[test]
    fn transition_indoor_to_exposed() {
        let rules = TransitionRules::default();
        let neighbors = LayerNeighborCounts {
            outdoor: 4,
            ..Default::default()
        };

        let result = rules.evaluate_transition(AtmosphereLayer::Indoor, &neighbors);
        assert_eq!(result, Some(AtmosphereLayer::Exposed));
    }

    #[test]
    fn no_transition_when_stable() {
        let rules = TransitionRules::default();
        let neighbors = LayerNeighborCounts {
            indoor: 1,
            outdoor: 1,
            ..Default::default()
        };

        let result = rules.evaluate_transition(AtmosphereLayer::Outdoor, &neighbors);
        assert_eq!(result, None);
    }

    #[test]
    fn neighbor_counts_from_array() {
        let counts = LayerNeighborCounts::from_array([2, 3, 1, 0]);
        assert_eq!(counts.indoor, 2);
        assert_eq!(counts.outdoor, 3);
        assert_eq!(counts.exposed, 1);
        assert_eq!(counts.vacuum, 0);
    }

    #[test]
    fn neighbor_counts_total() {
        let counts = LayerNeighborCounts {
            indoor: 2,
            outdoor: 3,
            exposed: 1,
            vacuum: 0,
        };
        assert_eq!(counts.total(), 6);
    }

    #[test]
    fn neighbor_counts_dominant() {
        let counts = LayerNeighborCounts {
            indoor: 1,
            outdoor: 5,
            exposed: 0,
            vacuum: 0,
        };
        assert_eq!(counts.dominant(), AtmosphereLayer::Outdoor);
    }

    #[test]
    fn layer_from_material_outdoor() {
        let props = MaterialProperties::new("Air", 1.0, 0.1, 0.0, 0.5, 0.0);
        let layer = layer_from_material(&props, true, false);
        assert_eq!(layer, AtmosphereLayer::Outdoor);
    }

    #[test]
    fn layer_from_material_indoor() {
        let props = MaterialProperties::new("Metal", 1.0, 0.9, 1.0, 0.1, 0.2);
        let layer = layer_from_material(&props, true, true);
        assert_eq!(layer, AtmosphereLayer::Indoor);
    }

    #[test]
    fn layer_from_material_exposed() {
        let props = MaterialProperties::new("Mesh", 0.5, 0.5, 0.5, 0.5, 0.5);
        let layer = layer_from_material(&props, true, true);
        assert_eq!(layer, AtmosphereLayer::Exposed);
    }

    #[test]
    fn cell_from_material_matches_layer() {
        let props = MaterialProperties::new("Metal", 1.0, 0.9, 0.95, 0.1, 0.2);
        let cell = cell_from_material(&props, true, true);
        assert_eq!(cell.layer(), AtmosphereLayer::Indoor);
        assert!((cell.seal_quality() - 0.95).abs() < 0.01);
    }

    #[test]
    fn effects_outdoor() {
        let effects = AtmosphereEffects::outdoor();
        assert_eq!(effects.oxygen_factor, 1.0);
        assert_eq!(effects.pressure_factor, 1.0);
        assert_eq!(effects.radiation_factor, 1.0);
    }

    #[test]
    fn effects_indoor() {
        let effects = AtmosphereEffects::indoor();
        assert_eq!(effects.radiation_factor, 0.0);
    }

    #[test]
    fn effects_vacuum() {
        let effects = AtmosphereEffects::vacuum();
        assert_eq!(effects.oxygen_factor, 0.0);
        assert_eq!(effects.pressure_factor, 0.0);
        assert!(effects.radiation_factor > 1.0);
    }

    #[test]
    fn effects_from_cell_contamination() {
        let config = AtmosphereConfig::default();
        let mut cell = AtmosphereCell::outdoor();
        cell.set_contamination(1.0);

        let effects = AtmosphereEffects::from_cell(&cell, &config);
        assert!(effects.oxygen_factor < 1.0);
    }

    #[test]
    fn effects_from_cell_indoor_temp() {
        let mut config = AtmosphereConfig::default();
        config.indoor_temperature = 25.0;

        let cell = AtmosphereCell::indoor_sealed();
        let effects = AtmosphereEffects::from_cell(&cell, &config);

        assert!(effects.temperature_delta > 0.0);
    }

    #[test]
    fn serde_round_trip_config() {
        let config = AtmosphereConfig::space();
        let json = serde_json::to_string(&config).unwrap();
        let recovered: AtmosphereConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, config);
    }

    #[test]
    fn serde_round_trip_rules() {
        let rules = TransitionRules::permissive();
        let json = serde_json::to_string(&rules).unwrap();
        let recovered: TransitionRules = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, rules);
    }
}
