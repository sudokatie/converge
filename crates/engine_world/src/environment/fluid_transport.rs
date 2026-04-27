//! Deterministic fluid volume transport between adjacent cells within a chunk.

use engine_core::coords::LocalPos;
use serde::{Deserialize, Serialize};

use super::{ChunkFluids, FluidCell, FluidKind};

/// 6 face neighbor offsets (dx, dy, dz).
const FACE_NEIGHBORS: [(i32, i32, i32); 6] = [
    (-1, 0, 0),
    (1, 0, 0),
    (0, -1, 0),
    (0, 1, 0),
    (0, 0, -1),
    (0, 0, 1),
];

/// Configuration for fluid transport simulation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FluidTransportConfig {
    /// Base flow rate (volume per second at zero viscosity/resistance).
    pub base_flow_rate: f32,
    /// Viscosity multiplier (higher = slower flow).
    pub viscosity_multiplier: f32,
    /// Gravity bias factor (how much gravity affects downward flow).
    pub gravity_bias: f32,
    /// Pressure equalization rate (0.0 to 1.0).
    pub pressure_equalization: f32,
    /// Enable evaporation.
    pub evaporation_enabled: bool,
    /// Enable cooling.
    pub cooling_enabled: bool,
    /// Ambient temperature for cooling target.
    pub ambient_temperature: f32,
    /// Minimum volume to consider for transport.
    pub min_transport_volume: f32,
}

impl FluidTransportConfig {
    /// Default configuration for water-like fluids.
    pub const WATER: Self = Self {
        base_flow_rate: 0.5,
        viscosity_multiplier: 1.0,
        gravity_bias: 2.0,
        pressure_equalization: 0.3,
        evaporation_enabled: true,
        cooling_enabled: true,
        ambient_temperature: 20.0,
        min_transport_volume: 0.01,
    };

    /// Fast-flowing gas configuration.
    pub const GAS: Self = Self {
        base_flow_rate: 2.0,
        viscosity_multiplier: 0.1,
        gravity_bias: -1.5,
        pressure_equalization: 0.8,
        evaporation_enabled: false,
        cooling_enabled: true,
        ambient_temperature: 20.0,
        min_transport_volume: 0.001,
    };

    /// Slow slurry configuration.
    pub const SLURRY: Self = Self {
        base_flow_rate: 0.1,
        viscosity_multiplier: 5.0,
        gravity_bias: 3.0,
        pressure_equalization: 0.1,
        evaporation_enabled: true,
        cooling_enabled: true,
        ambient_temperature: 15.0,
        min_transport_volume: 0.05,
    };

    /// Very slow lava configuration.
    pub const LAVA: Self = Self {
        base_flow_rate: 0.05,
        viscosity_multiplier: 10.0,
        gravity_bias: 4.0,
        pressure_equalization: 0.05,
        evaporation_enabled: false,
        cooling_enabled: true,
        ambient_temperature: 20.0,
        min_transport_volume: 0.1,
    };

    /// Get default config for a fluid kind.
    #[must_use]
    pub fn for_kind(kind: FluidKind) -> Self {
        match kind {
            FluidKind::Water => Self::WATER,
            FluidKind::Gas => Self::GAS,
            FluidKind::Slurry => Self::SLURRY,
            FluidKind::Lava => Self::LAVA,
        }
    }

    /// Effective flow rate accounting for viscosity.
    #[must_use]
    pub fn effective_flow_rate(&self, kind: FluidKind) -> f32 {
        self.base_flow_rate / (self.viscosity_multiplier * kind.base_viscosity())
    }
}

impl Default for FluidTransportConfig {
    fn default() -> Self {
        Self::WATER
    }
}

/// Resistance map for blocking/reducing fluid flow.
pub trait FluidResistanceMap {
    /// Get resistance at a local position (0.0 = no resistance, 1.0 = blocked).
    fn resistance(&self, kind: FluidKind, pos: LocalPos) -> f32;
}

impl FluidResistanceMap for () {
    fn resistance(&self, _kind: FluidKind, _pos: LocalPos) -> f32 {
        0.0
    }
}

impl<F> FluidResistanceMap for F
where
    F: Fn(FluidKind, LocalPos) -> f32,
{
    fn resistance(&self, kind: FluidKind, pos: LocalPos) -> f32 {
        self(kind, pos).clamp(0.0, 1.0)
    }
}

/// A single transport delta for a cell.
#[derive(Clone, Debug, PartialEq)]
pub struct FluidDelta {
    /// Cell position.
    pub pos: LocalPos,
    /// Volume change (positive = gain, negative = loss).
    pub volume_delta: f32,
    /// New pressure (if pressure equalization occurred).
    pub new_pressure: Option<f32>,
    /// New temperature (if cooling/evaporation occurred).
    pub new_temperature: Option<f32>,
}

impl FluidDelta {
    /// Create a volume-only delta.
    #[must_use]
    pub fn volume(pos: LocalPos, delta: f32) -> Self {
        Self {
            pos,
            volume_delta: delta,
            new_pressure: None,
            new_temperature: None,
        }
    }

    /// Create a full state delta.
    #[must_use]
    pub fn full(pos: LocalPos, volume_delta: f32, pressure: f32, temperature: f32) -> Self {
        Self {
            pos,
            volume_delta,
            new_pressure: Some(pressure),
            new_temperature: Some(temperature),
        }
    }
}

/// Outflow to a neighboring chunk.
#[derive(Clone, Debug, PartialEq)]
pub struct BoundaryOutflow {
    /// Source cell position (on chunk boundary).
    pub source_pos: LocalPos,
    /// Direction of outflow (-1, 0, 1 for each axis).
    pub direction: (i32, i32, i32),
    /// Volume attempting to flow out.
    pub volume: f32,
    /// Pressure at source.
    pub pressure: f32,
    /// Temperature at source.
    pub temperature: f32,
}

/// Result of a fluid transport step.
#[derive(Clone, Debug, Default)]
pub struct FluidTransportResult {
    /// Deltas to apply within this chunk.
    pub deltas: Vec<FluidDelta>,
    /// Outflows at chunk boundaries for adjacent chunks.
    pub boundary_outflows: Vec<BoundaryOutflow>,
    /// Volume evaporated this step.
    pub evaporated_volume: f32,
    /// Number of cells that flowed.
    pub flow_count: u32,
    /// Number of cells that equalized pressure.
    pub pressure_equalized_count: u32,
}

impl FluidTransportResult {
    /// Create empty result.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any changes occurred.
    #[must_use]
    pub fn has_changes(&self) -> bool {
        !self.deltas.is_empty() || !self.boundary_outflows.is_empty()
    }

    /// Merge another result into this one.
    pub fn merge(&mut self, other: Self) {
        self.deltas.extend(other.deltas);
        self.boundary_outflows.extend(other.boundary_outflows);
        self.evaporated_volume += other.evaporated_volume;
        self.flow_count += other.flow_count;
        self.pressure_equalized_count += other.pressure_equalized_count;
    }
}

/// Execute a deterministic transport step for one fluid kind.
#[expect(
    clippy::too_many_lines,
    reason = "complex simulation logic kept together for clarity"
)]
pub fn transport_step<R: FluidResistanceMap>(
    fluids: &ChunkFluids,
    kind: FluidKind,
    config: &FluidTransportConfig,
    dt: f32,
    resistance_map: &R,
) -> FluidTransportResult {
    let mut result = FluidTransportResult::new();

    let layer = match fluids.layer(kind) {
        Some(l) if l.active_count() > 0 => l,
        _ => return result,
    };

    let mut active_cells: Vec<(LocalPos, FluidCell)> = layer.iter_active().collect();
    active_cells.sort_by_key(|(pos, _)| pos.to_index());

    let effective_flow = config.effective_flow_rate(kind);
    let rises = kind.rises();

    for (pos, cell) in &active_cells {
        if cell.volume() < config.min_transport_volume {
            continue;
        }

        let mut total_delta = 0.0f32;
        let mut new_pressure = cell.pressure();
        let mut new_temperature = cell.temperature();

        for &(dx, dy, dz) in &FACE_NEIGHBORS {
            #[expect(
                clippy::cast_possible_wrap,
                reason = "pos coordinates are 0..16, so cast to i32 is safe"
            )]
            let nx = pos.x() as i32 + dx;
            #[expect(
                clippy::cast_possible_wrap,
                reason = "pos coordinates are 0..16, so cast to i32 is safe"
            )]
            let ny = pos.y() as i32 + dy;
            #[expect(
                clippy::cast_possible_wrap,
                reason = "pos coordinates are 0..16, so cast to i32 is safe"
            )]
            let nz = pos.z() as i32 + dz;

            if !(0..16).contains(&nx) || !(0..16).contains(&ny) || !(0..16).contains(&nz) {
                let gravity_factor = compute_gravity_factor(dy, rises, config.gravity_bias);
                if gravity_factor > 0.0 {
                    let flow_amount = (cell.volume() * effective_flow * dt * gravity_factor)
                        .min(cell.volume() * 0.25);

                    if flow_amount >= config.min_transport_volume {
                        result.boundary_outflows.push(BoundaryOutflow {
                            source_pos: *pos,
                            direction: (dx, dy, dz),
                            volume: flow_amount,
                            pressure: cell.pressure(),
                            temperature: cell.temperature(),
                        });
                    }
                }
                continue;
            }

            #[expect(
                clippy::cast_sign_loss,
                reason = "bounds check above guarantees nx, ny, nz are in 0..16"
            )]
            let neighbor_pos = LocalPos::new(nx as u32, ny as u32, nz as u32);
            let resistance = resistance_map.resistance(kind, neighbor_pos);

            if resistance >= 1.0 {
                continue;
            }

            let neighbor_cell = layer.get(neighbor_pos);
            let volume_diff = cell.volume() - neighbor_cell.volume();

            if volume_diff <= 0.0 {
                continue;
            }

            let gravity_factor = compute_gravity_factor(dy, rises, config.gravity_bias);
            let flow_rate = effective_flow * (1.0 - resistance) * gravity_factor.max(0.1);
            let max_flow = (volume_diff * 0.5).min(neighbor_cell.available_capacity());
            let flow_amount = (volume_diff * flow_rate * dt).min(max_flow);

            if flow_amount >= config.min_transport_volume {
                total_delta -= flow_amount;

                result
                    .deltas
                    .push(FluidDelta::volume(neighbor_pos, flow_amount));
                result.flow_count += 1;
            }

            if config.pressure_equalization > 0.0 {
                let pressure_diff = cell.pressure() - neighbor_cell.pressure();
                if pressure_diff.abs() > 0.01 {
                    let equalization = pressure_diff * config.pressure_equalization * dt;
                    new_pressure -= equalization;
                    result.pressure_equalized_count += 1;
                }
            }
        }

        if config.evaporation_enabled && new_temperature > kind.evaporation_threshold() {
            let excess_temp = new_temperature - kind.evaporation_threshold();
            let evap_amount = excess_temp * kind.evaporation_rate() * dt;
            let remaining = (cell.volume() + total_delta).max(0.0);
            let actual_evap = evap_amount.min(remaining);
            total_delta -= actual_evap;
            result.evaporated_volume += actual_evap;
        }

        if config.cooling_enabled {
            let temp_diff = new_temperature - config.ambient_temperature;
            if temp_diff > 0.0 {
                let cooling = temp_diff * kind.cooling_rate() * dt;
                new_temperature = (new_temperature - cooling).max(config.ambient_temperature);
            }
        }

        let pressure_changed =
            (new_pressure - cell.pressure()).abs() > 0.001 && config.pressure_equalization > 0.0;
        let temp_changed =
            (new_temperature - cell.temperature()).abs() > 0.01 && config.cooling_enabled;

        if total_delta.abs() >= config.min_transport_volume || pressure_changed || temp_changed {
            result.deltas.push(FluidDelta::full(
                *pos,
                total_delta,
                new_pressure,
                new_temperature,
            ));
        }
    }

    result
}

/// Apply transport deltas to fluid storage.
pub fn apply_fluid_deltas(fluids: &mut ChunkFluids, kind: FluidKind, deltas: &[FluidDelta]) {
    for delta in deltas {
        let layer = fluids.layer_mut(kind);
        let cell = layer.get_mut(delta.pos);

        cell.add_volume(delta.volume_delta);

        if let Some(pressure) = delta.new_pressure {
            cell.set_pressure(pressure);
        }
        if let Some(temperature) = delta.new_temperature {
            cell.set_temperature(temperature);
        }

        cell.clamp();
    }

    fluids.layer_mut(kind).recount();
}

fn compute_gravity_factor(dy: i32, rises: bool, gravity_bias: f32) -> f32 {
    if rises {
        match dy {
            1 => 1.0 + gravity_bias.abs(),
            -1 => (1.0 - gravity_bias.abs() * 0.5).max(0.1),
            _ => 1.0,
        }
    } else {
        match dy {
            -1 => 1.0 + gravity_bias,
            1 => (1.0 - gravity_bias * 0.5).max(0.1),
            _ => 1.0,
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    clippy::uninlined_format_args,
    clippy::collapsible_if,
    clippy::cast_sign_loss,
    reason = "tests check exact values; format args clearer; test arithmetic is bounded"
)]
mod tests {
    use super::*;

    fn make_fluids_with_water(positions: &[(u32, u32, u32, f32)]) -> ChunkFluids {
        let mut fluids = ChunkFluids::new();
        for &(x, y, z, volume) in positions {
            fluids.set(
                FluidKind::Water,
                LocalPos::new(x, y, z),
                FluidCell::new(FluidKind::Water, volume),
            );
        }
        fluids
    }

    #[test]
    fn transport_step_empty_layer() {
        let fluids = ChunkFluids::new();
        let config = FluidTransportConfig::WATER;
        let result = transport_step(&fluids, FluidKind::Water, &config, 0.1, &());
        assert!(!result.has_changes());
    }

    #[test]
    fn transport_step_flows_to_empty_neighbor() {
        let fluids = make_fluids_with_water(&[(8, 8, 8, 0.8)]);
        let config = FluidTransportConfig::WATER;

        let result = transport_step(&fluids, FluidKind::Water, &config, 0.5, &());

        assert!(result.flow_count > 0);
        let has_neighbor_gain = result
            .deltas
            .iter()
            .any(|d| d.pos != LocalPos::new(8, 8, 8) && d.volume_delta > 0.0);
        assert!(has_neighbor_gain);
    }

    #[test]
    fn transport_step_gravity_bias_down() {
        let fluids = make_fluids_with_water(&[(8, 8, 8, 0.8)]);
        let config = FluidTransportConfig::WATER;

        let result = transport_step(&fluids, FluidKind::Water, &config, 0.5, &());

        let down_flow = result
            .deltas
            .iter()
            .find(|d| d.pos == LocalPos::new(8, 7, 8) && d.volume_delta > 0.0);
        let up_flow = result
            .deltas
            .iter()
            .find(|d| d.pos == LocalPos::new(8, 9, 8) && d.volume_delta > 0.0);

        if let (Some(down), Some(up)) = (down_flow, up_flow) {
            assert!(
                down.volume_delta > up.volume_delta,
                "down={}, up={}",
                down.volume_delta,
                up.volume_delta
            );
        }
    }

    #[test]
    fn transport_step_gas_rises() {
        let mut fluids = ChunkFluids::new();
        fluids.set(
            FluidKind::Gas,
            LocalPos::new(8, 8, 8),
            FluidCell::new(FluidKind::Gas, 0.8),
        );

        let config = FluidTransportConfig::GAS;
        let result = transport_step(&fluids, FluidKind::Gas, &config, 0.0001, &());

        let up_flow = result
            .deltas
            .iter()
            .find(|d| d.pos == LocalPos::new(8, 9, 8) && d.volume_delta > 0.0);
        let down_flow = result
            .deltas
            .iter()
            .find(|d| d.pos == LocalPos::new(8, 7, 8) && d.volume_delta > 0.0);

        assert!(up_flow.is_some(), "expected upward flow for rising gas");
        if let Some(down) = down_flow {
            let up = up_flow.unwrap();
            assert!(
                up.volume_delta > down.volume_delta,
                "up={}, down={}",
                up.volume_delta,
                down.volume_delta
            );
        }
    }

    #[test]
    fn transport_step_viscosity_slows_flow() {
        let water_fluids = make_fluids_with_water(&[(8, 8, 8, 0.8)]);
        let mut lava_fluids = ChunkFluids::new();
        lava_fluids.set(
            FluidKind::Lava,
            LocalPos::new(8, 8, 8),
            FluidCell::new(FluidKind::Lava, 0.8),
        );

        let water_result = transport_step(
            &water_fluids,
            FluidKind::Water,
            &FluidTransportConfig::WATER,
            0.5,
            &(),
        );
        let lava_result = transport_step(
            &lava_fluids,
            FluidKind::Lava,
            &FluidTransportConfig::LAVA,
            0.5,
            &(),
        );

        let water_total: f32 = water_result
            .deltas
            .iter()
            .filter(|d| d.volume_delta > 0.0)
            .map(|d| d.volume_delta)
            .sum();
        let lava_total: f32 = lava_result
            .deltas
            .iter()
            .filter(|d| d.volume_delta > 0.0)
            .map(|d| d.volume_delta)
            .sum();

        assert!(
            water_total > lava_total,
            "water={}, lava={}",
            water_total,
            lava_total
        );
    }

    #[test]
    fn transport_step_resistance_blocks() {
        let fluids = make_fluids_with_water(&[(8, 8, 8, 0.8)]);
        let config = FluidTransportConfig::WATER;

        let blocker = |_kind: FluidKind, pos: LocalPos| {
            if pos.x() == 9 && pos.y() == 8 && pos.z() == 8 {
                1.0
            } else {
                0.0
            }
        };

        let result = transport_step(&fluids, FluidKind::Water, &config, 0.5, &blocker);

        let blocked_pos = LocalPos::new(9, 8, 8);
        let flow_to_blocked = result
            .deltas
            .iter()
            .any(|d| d.pos == blocked_pos && d.volume_delta > 0.0);
        assert!(!flow_to_blocked);
    }

    #[test]
    fn transport_step_resistance_reduces() {
        let fluids = make_fluids_with_water(&[(8, 8, 8, 0.8)]);
        let config = FluidTransportConfig::WATER;

        let partial = |_kind: FluidKind, pos: LocalPos| {
            if pos.x() == 9 { 0.5 } else { 0.0 }
        };

        let result = transport_step(&fluids, FluidKind::Water, &config, 0.5, &partial);

        let resisted = result
            .deltas
            .iter()
            .find(|d| d.pos == LocalPos::new(9, 8, 8));
        let unresisted = result
            .deltas
            .iter()
            .find(|d| d.pos == LocalPos::new(7, 8, 8));

        if let (Some(r), Some(u)) = (resisted, unresisted) {
            assert!(
                r.volume_delta < u.volume_delta,
                "resisted={}, unresisted={}",
                r.volume_delta,
                u.volume_delta
            );
        }
    }

    #[test]
    fn transport_step_boundary_outflows() {
        let fluids = make_fluids_with_water(&[(0, 8, 8, 0.8)]);
        let config = FluidTransportConfig::WATER;

        let result = transport_step(&fluids, FluidKind::Water, &config, 0.5, &());

        assert!(!result.boundary_outflows.is_empty());
        let has_neg_x = result.boundary_outflows.iter().any(|o| o.direction.0 == -1);
        assert!(has_neg_x);
    }

    #[test]
    fn transport_step_pressure_equalization() {
        let mut fluids = ChunkFluids::new();
        fluids.set(
            FluidKind::Water,
            LocalPos::new(8, 8, 8),
            FluidCell::with_state(FluidKind::Water, 0.5, 5.0, 20.0),
        );
        fluids.set(
            FluidKind::Water,
            LocalPos::new(9, 8, 8),
            FluidCell::with_state(FluidKind::Water, 0.5, 1.0, 20.0),
        );

        let config = FluidTransportConfig::WATER;
        let result = transport_step(&fluids, FluidKind::Water, &config, 0.5, &());

        assert!(result.pressure_equalized_count > 0);
    }

    #[test]
    fn transport_step_cooling() {
        let mut fluids = ChunkFluids::new();
        fluids.set(
            FluidKind::Lava,
            LocalPos::new(8, 8, 8),
            FluidCell::with_state(FluidKind::Lava, 0.5, 1.0, 1200.0),
        );

        let config = FluidTransportConfig::LAVA;
        let result = transport_step(&fluids, FluidKind::Lava, &config, 1.0, &());

        let source_delta = result
            .deltas
            .iter()
            .find(|d| d.pos == LocalPos::new(8, 8, 8));
        if let Some(delta) = source_delta {
            if let Some(new_temp) = delta.new_temperature {
                assert!(new_temp < 1200.0, "temp={}", new_temp);
            }
        }
    }

    #[test]
    fn transport_step_evaporation() {
        let mut fluids = ChunkFluids::new();
        let hot_cell = FluidCell::with_state(FluidKind::Water, 0.5, 1.0, 150.0);
        fluids.set(FluidKind::Water, LocalPos::new(8, 8, 8), hot_cell);
        for &(dx, dy, dz) in &[
            (-1, 0, 0),
            (1, 0, 0),
            (0, -1, 0),
            (0, 1, 0),
            (0, 0, -1),
            (0, 0, 1),
        ] {
            let neighbor = LocalPos::new((8 + dx) as u32, (8 + dy) as u32, (8 + dz) as u32);
            fluids.set(
                FluidKind::Water,
                neighbor,
                FluidCell::with_state(FluidKind::Water, 0.5, 1.0, 150.0),
            );
        }

        let config = FluidTransportConfig::WATER;
        let result = transport_step(&fluids, FluidKind::Water, &config, 1.0, &());

        assert!(
            result.evaporated_volume > 0.0,
            "expected evaporation for hot water above 100C"
        );
    }

    #[test]
    fn transport_step_deterministic() {
        let fluids = make_fluids_with_water(&[(3, 3, 3, 0.8), (10, 10, 10, 0.7), (5, 5, 5, 0.6)]);
        let config = FluidTransportConfig::WATER;

        let result1 = transport_step(&fluids, FluidKind::Water, &config, 0.5, &());
        let result2 = transport_step(&fluids, FluidKind::Water, &config, 0.5, &());

        assert_eq!(result1.deltas.len(), result2.deltas.len());
        for (d1, d2) in result1.deltas.iter().zip(result2.deltas.iter()) {
            assert_eq!(d1.pos, d2.pos);
            assert!((d1.volume_delta - d2.volume_delta).abs() < 0.0001);
        }
    }

    #[test]
    fn apply_fluid_deltas_updates() {
        let mut fluids = make_fluids_with_water(&[(8, 8, 8, 0.8)]);

        let deltas = vec![
            FluidDelta::volume(LocalPos::new(8, 8, 8), -0.2),
            FluidDelta::volume(LocalPos::new(9, 8, 8), 0.2),
        ];

        apply_fluid_deltas(&mut fluids, FluidKind::Water, &deltas);

        let source = fluids.get(FluidKind::Water, LocalPos::new(8, 8, 8));
        let dest = fluids.get(FluidKind::Water, LocalPos::new(9, 8, 8));

        assert!((source.volume() - 0.6).abs() < 0.001);
        assert!((dest.volume() - 0.2).abs() < 0.001);
    }

    #[test]
    fn apply_fluid_deltas_pressure_temp() {
        let mut fluids = make_fluids_with_water(&[(8, 8, 8, 0.5)]);

        let deltas = vec![FluidDelta::full(LocalPos::new(8, 8, 8), 0.0, 3.0, 50.0)];

        apply_fluid_deltas(&mut fluids, FluidKind::Water, &deltas);

        let cell = fluids.get(FluidKind::Water, LocalPos::new(8, 8, 8));
        assert!((cell.pressure() - 3.0).abs() < 0.001);
        assert!((cell.temperature() - 50.0).abs() < 0.001);
    }

    #[test]
    fn config_effective_flow_rate() {
        let water_config = FluidTransportConfig::WATER;
        let lava_config = FluidTransportConfig::LAVA;

        let water_rate = water_config.effective_flow_rate(FluidKind::Water);
        let lava_rate = lava_config.effective_flow_rate(FluidKind::Lava);

        assert!(water_rate > lava_rate);
    }

    #[test]
    fn config_for_kind() {
        assert_eq!(
            FluidTransportConfig::for_kind(FluidKind::Water),
            FluidTransportConfig::WATER
        );
        assert_eq!(
            FluidTransportConfig::for_kind(FluidKind::Gas),
            FluidTransportConfig::GAS
        );
        assert_eq!(
            FluidTransportConfig::for_kind(FluidKind::Slurry),
            FluidTransportConfig::SLURRY
        );
        assert_eq!(
            FluidTransportConfig::for_kind(FluidKind::Lava),
            FluidTransportConfig::LAVA
        );
    }

    #[test]
    fn result_merge() {
        let mut a = FluidTransportResult {
            deltas: vec![FluidDelta::volume(LocalPos::new(0, 0, 0), 0.1)],
            boundary_outflows: vec![BoundaryOutflow {
                source_pos: LocalPos::new(0, 0, 0),
                direction: (-1, 0, 0),
                volume: 0.1,
                pressure: 1.0,
                temperature: 20.0,
            }],
            evaporated_volume: 0.01,
            flow_count: 1,
            pressure_equalized_count: 0,
        };

        let b = FluidTransportResult {
            deltas: vec![FluidDelta::volume(LocalPos::new(1, 1, 1), 0.2)],
            boundary_outflows: vec![],
            evaporated_volume: 0.02,
            flow_count: 2,
            pressure_equalized_count: 1,
        };

        a.merge(b);

        assert_eq!(a.deltas.len(), 2);
        assert_eq!(a.boundary_outflows.len(), 1);
        assert!((a.evaporated_volume - 0.03).abs() < 0.001);
        assert_eq!(a.flow_count, 3);
        assert_eq!(a.pressure_equalized_count, 1);
    }

    #[test]
    fn fluid_delta_constructors() {
        let vol = FluidDelta::volume(LocalPos::new(1, 2, 3), 0.5);
        assert_eq!(vol.pos, LocalPos::new(1, 2, 3));
        assert!((vol.volume_delta - 0.5).abs() < 0.001);
        assert!(vol.new_pressure.is_none());
        assert!(vol.new_temperature.is_none());

        let full = FluidDelta::full(LocalPos::new(4, 5, 6), -0.1, 2.0, 30.0);
        assert_eq!(full.pos, LocalPos::new(4, 5, 6));
        assert!((full.volume_delta - (-0.1)).abs() < 0.001);
        assert_eq!(full.new_pressure, Some(2.0));
        assert_eq!(full.new_temperature, Some(30.0));
    }

    #[test]
    fn boundary_outflow_fields() {
        let outflow = BoundaryOutflow {
            source_pos: LocalPos::new(0, 5, 5),
            direction: (-1, 0, 0),
            volume: 0.25,
            pressure: 1.5,
            temperature: 25.0,
        };

        assert_eq!(outflow.source_pos, LocalPos::new(0, 5, 5));
        assert_eq!(outflow.direction, (-1, 0, 0));
        assert!((outflow.volume - 0.25).abs() < 0.001);
        assert!((outflow.pressure - 1.5).abs() < 0.001);
        assert!((outflow.temperature - 25.0).abs() < 0.001);
    }

    #[test]
    fn config_serde_round_trip() {
        let config = FluidTransportConfig::SLURRY;
        let json = serde_json::to_string(&config).unwrap();
        let recovered: FluidTransportConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, config);
    }
}
