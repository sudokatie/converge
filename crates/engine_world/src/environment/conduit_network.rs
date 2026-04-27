//! Deterministic conduit network solver for connected networks within a chunk.

use engine_core::coords::LocalPos;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

use super::{ChunkConduits, ConduitCell, ConduitKind, ConduitNetworkConfig, ConduitNode, NodeRole};

/// 6 face neighbor offsets.
const FACE_NEIGHBORS: [(i32, i32, i32); 6] = [
    (-1, 0, 0),
    (1, 0, 0),
    (0, -1, 0),
    (0, 1, 0),
    (0, 0, -1),
    (0, 0, 1),
];

/// A connected network of conduit cells.
#[derive(Clone, Debug, PartialEq)]
pub struct ConnectedNetwork {
    /// Network identifier (unique within this computation).
    pub id: u32,
    /// Conduit kind for this network.
    pub kind: ConduitKind,
    /// All cell positions in this network.
    pub cells: Vec<LocalPos>,
    /// Source nodes in this network.
    pub sources: Vec<usize>,
    /// Sink nodes in this network.
    pub sinks: Vec<usize>,
    /// Storage nodes in this network.
    pub storage: Vec<usize>,
    /// Total capacity of the network.
    pub total_capacity: f32,
    /// Total stored amount in the network.
    pub total_stored: f32,
}

impl ConnectedNetwork {
    /// Create a new empty network.
    #[must_use]
    pub fn new(id: u32, kind: ConduitKind) -> Self {
        Self {
            id,
            kind,
            cells: Vec::new(),
            sources: Vec::new(),
            sinks: Vec::new(),
            storage: Vec::new(),
            total_capacity: 0.0,
            total_stored: 0.0,
        }
    }

    /// Check if network has any sources.
    #[must_use]
    pub fn has_sources(&self) -> bool {
        !self.sources.is_empty()
    }

    /// Check if network has any sinks.
    #[must_use]
    pub fn has_sinks(&self) -> bool {
        !self.sinks.is_empty()
    }

    /// Check if network has any storage.
    #[must_use]
    pub fn has_storage(&self) -> bool {
        !self.storage.is_empty()
    }

    /// Get fill ratio of the network.
    #[must_use]
    pub fn fill_ratio(&self) -> f32 {
        if self.total_capacity > 0.0 {
            (self.total_stored / self.total_capacity).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

/// A delta to apply to conduit state.
#[derive(Clone, Debug, PartialEq)]
pub struct ConduitDelta {
    /// Cell position.
    pub pos: LocalPos,
    /// Stored amount change.
    pub stored_delta: f32,
    /// New temperature (if changed).
    pub new_temperature: Option<f32>,
    /// New pressure (if changed).
    pub new_pressure: Option<f32>,
}

impl ConduitDelta {
    /// Create a stored-only delta.
    #[must_use]
    pub fn stored(pos: LocalPos, delta: f32) -> Self {
        Self {
            pos,
            stored_delta: delta,
            new_temperature: None,
            new_pressure: None,
        }
    }

    /// Create a full state delta.
    #[must_use]
    pub fn full(pos: LocalPos, stored_delta: f32, temperature: f32, pressure: f32) -> Self {
        Self {
            pos,
            stored_delta,
            new_temperature: Some(temperature),
            new_pressure: Some(pressure),
        }
    }
}

/// Boundary handoff for cross-chunk network continuation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConduitBoundary {
    /// Position on chunk boundary.
    pub pos: LocalPos,
    /// Direction to neighboring chunk.
    pub direction: (i32, i32, i32),
    /// Conduit kind.
    pub kind: ConduitKind,
    /// Network ID within source chunk.
    pub network_id: u32,
    /// Available supply at boundary.
    pub supply: f32,
    /// Demand at boundary.
    pub demand: f32,
    /// Temperature at boundary.
    pub temperature: f32,
    /// Pressure at boundary.
    pub pressure: f32,
}

impl ConduitBoundary {
    /// Create a new boundary report.
    #[must_use]
    pub fn new(
        pos: LocalPos,
        direction: (i32, i32, i32),
        kind: ConduitKind,
        network_id: u32,
        cell: &ConduitCell,
    ) -> Self {
        Self {
            pos,
            direction,
            kind,
            network_id,
            supply: cell.stored(),
            demand: cell.available_capacity(),
            temperature: cell.temperature(),
            pressure: cell.pressure(),
        }
    }

    /// Check if boundary has excess supply.
    #[must_use]
    pub fn has_supply(&self) -> bool {
        self.supply > 0.0
    }

    /// Check if boundary has demand.
    #[must_use]
    pub fn has_demand(&self) -> bool {
        self.demand > 0.0
    }
}

/// Result of network simulation step.
#[derive(Clone, Debug, Default)]
pub struct ConduitNetworkResult {
    /// Deltas to apply to conduit cells.
    pub deltas: Vec<ConduitDelta>,
    /// Boundary handoffs for neighboring chunks.
    pub boundaries: Vec<ConduitBoundary>,
    /// Connected networks found.
    pub networks: Vec<ConnectedNetwork>,
    /// Total flow transferred.
    pub total_flow: f32,
    /// Total loss from resistance.
    pub total_loss: f32,
    /// Number of satisfied sinks.
    pub satisfied_sinks: u32,
    /// Number of active sources.
    pub active_sources: u32,
}

impl ConduitNetworkResult {
    /// Create empty result.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any changes occurred.
    #[must_use]
    pub fn has_changes(&self) -> bool {
        !self.deltas.is_empty() || !self.boundaries.is_empty()
    }

    /// Merge another result.
    pub fn merge(&mut self, other: Self) {
        self.deltas.extend(other.deltas);
        self.boundaries.extend(other.boundaries);
        self.networks.extend(other.networks);
        self.total_flow += other.total_flow;
        self.total_loss += other.total_loss;
        self.satisfied_sinks += other.satisfied_sinks;
        self.active_sources += other.active_sources;
    }
}

/// Resistance map for conduit flow.
pub trait ConduitResistanceMap {
    /// Get additional resistance at position (0.0 = none, 1.0 = blocked).
    fn resistance(&self, kind: ConduitKind, pos: LocalPos) -> f32;
}

impl ConduitResistanceMap for () {
    fn resistance(&self, _kind: ConduitKind, _pos: LocalPos) -> f32 {
        0.0
    }
}

impl<F> ConduitResistanceMap for F
where
    F: Fn(ConduitKind, LocalPos) -> f32,
{
    fn resistance(&self, kind: ConduitKind, pos: LocalPos) -> f32 {
        self(kind, pos).clamp(0.0, 1.0)
    }
}

/// Find all connected networks for a conduit kind.
pub fn find_networks(conduits: &ChunkConduits, kind: ConduitKind) -> Vec<ConnectedNetwork> {
    let layer = conduits.layer(kind);
    if !layer.has_active() {
        return Vec::new();
    }

    let mut visited = HashSet::new();
    let mut networks = Vec::new();
    let mut network_id = 0u32;

    let active: Vec<_> = layer.iter_active().collect();

    for (start_pos, _) in &active {
        if visited.contains(start_pos) {
            continue;
        }

        let mut network = ConnectedNetwork::new(network_id, kind);
        let mut queue = VecDeque::new();
        queue.push_back(*start_pos);
        visited.insert(*start_pos);

        while let Some(pos) = queue.pop_front() {
            let cell = layer.get(pos);
            network.cells.push(pos);
            network.total_capacity += cell.capacity();
            network.total_stored += cell.stored();

            for &(dx, dy, dz) in &FACE_NEIGHBORS {
                #[expect(clippy::cast_possible_wrap, reason = "pos coordinates are 0..16")]
                let nx = pos.x() as i32 + dx;
                #[expect(clippy::cast_possible_wrap, reason = "pos coordinates are 0..16")]
                let ny = pos.y() as i32 + dy;
                #[expect(clippy::cast_possible_wrap, reason = "pos coordinates are 0..16")]
                let nz = pos.z() as i32 + dz;

                if !(0..16).contains(&nx) || !(0..16).contains(&ny) || !(0..16).contains(&nz) {
                    continue;
                }

                #[expect(clippy::cast_sign_loss, reason = "bounds check above")]
                let neighbor_pos = LocalPos::new(nx as u32, ny as u32, nz as u32);

                if visited.contains(&neighbor_pos) {
                    continue;
                }

                let neighbor = layer.get(neighbor_pos);
                if !neighbor.is_empty() {
                    visited.insert(neighbor_pos);
                    queue.push_back(neighbor_pos);
                }
            }
        }

        networks.push(network);
        network_id += 1;
    }

    networks
}

/// Find boundary cells that connect to neighboring chunks.
pub fn find_boundary_cells(
    conduits: &ChunkConduits,
    kind: ConduitKind,
    networks: &[ConnectedNetwork],
) -> Vec<ConduitBoundary> {
    let mut boundaries = Vec::new();
    let layer = conduits.layer(kind);

    let network_map: HashMap<LocalPos, u32> = networks
        .iter()
        .flat_map(|n| n.cells.iter().map(|&pos| (pos, n.id)))
        .collect();

    for (pos, cell) in layer.iter_active() {
        let network_id = network_map.get(&pos).copied().unwrap_or(0);

        for &(dx, dy, dz) in &FACE_NEIGHBORS {
            #[expect(clippy::cast_possible_wrap, reason = "pos coordinates are 0..16")]
            let nx = pos.x() as i32 + dx;
            #[expect(clippy::cast_possible_wrap, reason = "pos coordinates are 0..16")]
            let ny = pos.y() as i32 + dy;
            #[expect(clippy::cast_possible_wrap, reason = "pos coordinates are 0..16")]
            let nz = pos.z() as i32 + dz;

            let is_boundary =
                !(0..16).contains(&nx) || !(0..16).contains(&ny) || !(0..16).contains(&nz);

            if is_boundary {
                boundaries.push(ConduitBoundary::new(
                    pos,
                    (dx, dy, dz),
                    kind,
                    network_id,
                    &cell,
                ));
            }
        }
    }

    boundaries
}

/// Distribute flow within a network based on sources and sinks.
#[expect(
    clippy::too_many_lines,
    reason = "network distribution logic kept together"
)]
pub fn distribute_network<R: ConduitResistanceMap>(
    conduits: &ChunkConduits,
    network: &ConnectedNetwork,
    nodes: &mut [ConduitNode],
    config: &ConduitNetworkConfig,
    dt: f32,
    resistance_map: &R,
) -> Vec<ConduitDelta> {
    let mut deltas = Vec::new();

    if !config.enabled {
        return deltas;
    }

    let mut source_indices: Vec<_> = network
        .sources
        .iter()
        .copied()
        .filter(|&idx| {
            nodes
                .get(idx)
                .is_some_and(|n| n.enabled && n.role.can_provide())
        })
        .collect();

    let mut sink_indices: Vec<_> = network
        .sinks
        .iter()
        .copied()
        .filter(|&idx| {
            nodes
                .get(idx)
                .is_some_and(|n| n.enabled && n.role.can_accept())
        })
        .collect();

    let storage_indices: Vec<_> = network
        .storage
        .iter()
        .copied()
        .filter(|&idx| nodes.get(idx).is_some_and(|n| n.enabled))
        .collect();

    if config.flow.use_priority {
        source_indices.sort_by(|&a, &b| {
            let pa = nodes.get(a).map_or(0, |n| n.priority);
            let pb = nodes.get(b).map_or(0, |n| n.priority);
            pb.cmp(&pa)
        });
        sink_indices.sort_by(|&a, &b| {
            let pa = nodes.get(a).map_or(0, |n| n.priority);
            let pb = nodes.get(b).map_or(0, |n| n.priority);
            pb.cmp(&pa)
        });
    }

    let total_supply: f32 = source_indices
        .iter()
        .filter_map(|&idx| nodes.get(idx))
        .map(|s| s.supply_available(dt))
        .sum();
    let total_demand: f32 = sink_indices
        .iter()
        .filter_map(|&idx| nodes.get(idx))
        .map(|s| s.demand(dt))
        .sum();

    if total_supply < config.flow.min_transfer && total_demand < config.flow.min_transfer {
        return deltas;
    }

    let loss_factor = 1.0 - config.flow.loss_per_segment;

    let effective_supply =
        (total_supply * loss_factor * config.flow.rate_multiplier).min(total_demand);

    let mut remaining = effective_supply;

    for &sink_idx in &sink_indices {
        if remaining < config.flow.min_transfer {
            break;
        }

        let Some(sink) = nodes.get_mut(sink_idx) else {
            continue;
        };

        let demand = sink.demand(dt);
        let transfer = remaining.min(demand);

        if transfer >= config.flow.min_transfer {
            let base_resistance = resistance_map.resistance(network.kind, sink.pos);
            let actual = transfer * (1.0 - base_resistance);
            let pos = sink.pos;

            sink.consume(actual);
            remaining -= transfer;

            deltas.push(ConduitDelta::stored(pos, actual));
        }
    }

    let actually_supplied = effective_supply - remaining;
    let supply_needed = actually_supplied / (loss_factor * config.flow.rate_multiplier).max(0.001);
    let mut supply_remaining = supply_needed;

    for &source_idx in &source_indices {
        if supply_remaining < config.flow.min_transfer {
            break;
        }

        let Some(source) = nodes.get_mut(source_idx) else {
            continue;
        };

        let available = source.supply_available(dt);
        let transfer = supply_remaining.min(available);

        if transfer >= config.flow.min_transfer {
            let pos = source.pos;
            source.produce(transfer);
            supply_remaining -= transfer;

            deltas.push(ConduitDelta::stored(pos, -transfer));
        }
    }

    if network.kind.uses_temperature() {
        apply_heat_equalization(conduits, network, config, dt, &mut deltas);
    }

    if network.kind.uses_pressure() {
        apply_pressure_equalization(conduits, network, config, dt, &mut deltas);
    }

    for &store_idx in &storage_indices {
        let Some(store) = nodes.get_mut(store_idx) else {
            continue;
        };

        let fill = store.stored / store.capacity.max(0.001);
        if fill > 0.5 && remaining > config.flow.min_transfer {
            let excess = (fill - 0.5) * store.capacity * 0.1 * dt;
            let transfer = excess.min(remaining);
            if transfer >= config.flow.min_transfer {
                store.produce(transfer);
                remaining -= transfer;
            }
        }
    }

    deltas
}

fn apply_heat_equalization(
    conduits: &ChunkConduits,
    network: &ConnectedNetwork,
    config: &ConduitNetworkConfig,
    dt: f32,
    deltas: &mut Vec<ConduitDelta>,
) {
    if network.cells.len() < 2 {
        return;
    }

    let layer = conduits.layer(network.kind);
    #[expect(
        clippy::cast_precision_loss,
        reason = "network cell count is bounded by chunk volume (4096)"
    )]
    let avg_temp: f32 = network
        .cells
        .iter()
        .map(|&pos| layer.get(pos).temperature())
        .sum::<f32>()
        / network.cells.len() as f32;

    for &pos in &network.cells {
        let cell = layer.get(pos);
        let temp_diff = cell.temperature() - avg_temp;

        if temp_diff.abs() > config.heat.min_delta {
            let new_temp = cell.temperature() - temp_diff * config.heat.conductivity * dt;
            let ambient_loss = (cell.temperature() - config.heat.ambient_temperature)
                * config.heat.ambient_loss_rate
                * dt;
            let final_temp = new_temp - ambient_loss;

            deltas.push(ConduitDelta::full(pos, 0.0, final_temp, cell.pressure()));
        }
    }
}

fn apply_pressure_equalization(
    conduits: &ChunkConduits,
    network: &ConnectedNetwork,
    config: &ConduitNetworkConfig,
    dt: f32,
    deltas: &mut Vec<ConduitDelta>,
) {
    if network.cells.len() < 2 {
        return;
    }

    let layer = conduits.layer(network.kind);
    #[expect(
        clippy::cast_precision_loss,
        reason = "network cell count is bounded by chunk volume (4096)"
    )]
    let avg_pressure: f32 = network
        .cells
        .iter()
        .map(|&pos| layer.get(pos).pressure())
        .sum::<f32>()
        / network.cells.len() as f32;

    for &pos in &network.cells {
        let cell = layer.get(pos);
        let pressure_diff = cell.pressure() - avg_pressure;

        if pressure_diff.abs() > 0.01 {
            let new_pressure =
                cell.pressure() - pressure_diff * config.pressure.equalization_rate * dt;

            deltas.push(ConduitDelta::full(
                pos,
                0.0,
                cell.temperature(),
                new_pressure,
            ));
        }
    }
}

/// Run a complete network simulation step for one conduit kind.
pub fn network_step<R: ConduitResistanceMap>(
    conduits: &ChunkConduits,
    kind: ConduitKind,
    nodes: &mut [ConduitNode],
    config: &ConduitNetworkConfig,
    dt: f32,
    resistance_map: &R,
) -> ConduitNetworkResult {
    let mut result = ConduitNetworkResult::new();

    if !config.enabled {
        return result;
    }

    let networks = find_networks(conduits, kind);
    let boundaries = find_boundary_cells(conduits, kind, &networks);

    result.boundaries = boundaries;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "node count is bounded by reasonable gameplay limits"
    )]
    {
        result.active_sources = nodes
            .iter()
            .filter(|n| n.kind == kind && n.role == NodeRole::Source && n.enabled)
            .count() as u32;
    }

    for network in &networks {
        let deltas = distribute_network(conduits, network, nodes, config, dt, resistance_map);

        for delta in &deltas {
            if delta.stored_delta > 0.0 {
                result.total_flow += delta.stored_delta;
            } else {
                result.total_loss += delta.stored_delta.abs() * config.flow.loss_per_segment;
            }
        }

        result.deltas.extend(deltas);
    }

    result.networks = networks;
    result
}

/// Apply deltas to conduit storage.
pub fn apply_conduit_deltas(
    conduits: &mut ChunkConduits,
    kind: ConduitKind,
    deltas: &[ConduitDelta],
) {
    for delta in deltas {
        let cell = conduits.get_mut(kind, delta.pos);

        if delta.stored_delta != 0.0 {
            if delta.stored_delta > 0.0 {
                cell.add_stored(delta.stored_delta);
            } else {
                cell.remove_stored(-delta.stored_delta);
            }
        }

        if let Some(temp) = delta.new_temperature {
            cell.set_temperature(temp);
        }
        if let Some(pressure) = delta.new_pressure {
            cell.set_pressure(pressure);
        }

        cell.clamp();
    }
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    clippy::cast_sign_loss,
    reason = "tests check exact values and bounded arithmetic"
)]
mod tests {
    use super::*;

    fn make_conduits_line(kind: ConduitKind, length: u32) -> ChunkConduits {
        let mut conduits = ChunkConduits::new();
        for x in 0..length {
            conduits.set(kind, LocalPos::new(x, 8, 8), ConduitCell::new(kind));
        }
        conduits
    }

    fn make_conduits_with_stored(
        positions: &[(u32, u32, u32, f32)],
        kind: ConduitKind,
    ) -> ChunkConduits {
        let mut conduits = ChunkConduits::new();
        for &(x, y, z, stored) in positions {
            let mut cell = ConduitCell::new(kind);
            cell.set_stored(stored);
            conduits.set(kind, LocalPos::new(x, y, z), cell);
        }
        conduits
    }

    #[test]
    fn find_networks_empty() {
        let conduits = ChunkConduits::new();
        let networks = find_networks(&conduits, ConduitKind::Power);
        assert!(networks.is_empty());
    }

    #[test]
    fn find_networks_single_cell() {
        let mut conduits = ChunkConduits::new();
        conduits.set(
            ConduitKind::Power,
            LocalPos::new(8, 8, 8),
            ConduitCell::new(ConduitKind::Power),
        );

        let networks = find_networks(&conduits, ConduitKind::Power);
        assert_eq!(networks.len(), 1);
        assert_eq!(networks[0].cells.len(), 1);
    }

    #[test]
    fn find_networks_connected_line() {
        let conduits = make_conduits_line(ConduitKind::Power, 5);

        let networks = find_networks(&conduits, ConduitKind::Power);
        assert_eq!(networks.len(), 1);
        assert_eq!(networks[0].cells.len(), 5);
    }

    #[test]
    fn find_networks_disconnected() {
        let mut conduits = ChunkConduits::new();
        conduits.set(
            ConduitKind::Power,
            LocalPos::new(0, 8, 8),
            ConduitCell::new(ConduitKind::Power),
        );
        conduits.set(
            ConduitKind::Power,
            LocalPos::new(15, 8, 8),
            ConduitCell::new(ConduitKind::Power),
        );

        let networks = find_networks(&conduits, ConduitKind::Power);
        assert_eq!(networks.len(), 2);
        assert_eq!(networks[0].cells.len(), 1);
        assert_eq!(networks[1].cells.len(), 1);
    }

    #[test]
    fn find_networks_different_kinds_separate() {
        let mut conduits = ChunkConduits::new();
        conduits.set(
            ConduitKind::Power,
            LocalPos::new(8, 8, 8),
            ConduitCell::new(ConduitKind::Power),
        );
        conduits.set(
            ConduitKind::Heat,
            LocalPos::new(9, 8, 8),
            ConduitCell::new(ConduitKind::Heat),
        );

        let power_networks = find_networks(&conduits, ConduitKind::Power);
        let heat_networks = find_networks(&conduits, ConduitKind::Heat);

        assert_eq!(power_networks.len(), 1);
        assert_eq!(heat_networks.len(), 1);
    }

    #[test]
    fn find_boundary_cells_interior() {
        let mut conduits = ChunkConduits::new();
        conduits.set(
            ConduitKind::Power,
            LocalPos::new(8, 8, 8),
            ConduitCell::new(ConduitKind::Power),
        );

        let networks = find_networks(&conduits, ConduitKind::Power);
        let boundaries = find_boundary_cells(&conduits, ConduitKind::Power, &networks);

        assert!(boundaries.is_empty());
    }

    #[test]
    fn find_boundary_cells_edge() {
        let mut conduits = ChunkConduits::new();
        conduits.set(
            ConduitKind::Power,
            LocalPos::new(0, 8, 8),
            ConduitCell::new(ConduitKind::Power),
        );

        let networks = find_networks(&conduits, ConduitKind::Power);
        let boundaries = find_boundary_cells(&conduits, ConduitKind::Power, &networks);

        assert!(!boundaries.is_empty());
        let has_neg_x = boundaries.iter().any(|b| b.direction.0 == -1);
        assert!(has_neg_x);
    }

    #[test]
    fn find_boundary_cells_corner() {
        let mut conduits = ChunkConduits::new();
        conduits.set(
            ConduitKind::Power,
            LocalPos::new(0, 0, 0),
            ConduitCell::new(ConduitKind::Power),
        );

        let networks = find_networks(&conduits, ConduitKind::Power);
        let boundaries = find_boundary_cells(&conduits, ConduitKind::Power, &networks);

        assert_eq!(boundaries.len(), 3);
    }

    #[test]
    fn network_step_empty() {
        let conduits = ChunkConduits::new();
        let mut nodes = Vec::new();
        let config = ConduitNetworkConfig::STANDARD;

        let result = network_step(&conduits, ConduitKind::Power, &mut nodes, &config, 0.1, &());

        assert!(!result.has_changes());
        assert!(result.networks.is_empty());
    }

    #[test]
    fn network_step_with_source_and_sink() {
        let conduits = make_conduits_line(ConduitKind::Power, 3);
        let mut nodes = vec![
            ConduitNode::source(LocalPos::new(0, 8, 8), ConduitKind::Power, 10.0),
            ConduitNode::sink(LocalPos::new(2, 8, 8), ConduitKind::Power, 10.0),
        ];

        let config = ConduitNetworkConfig::for_kind(ConduitKind::Power);
        let result = network_step(&conduits, ConduitKind::Power, &mut nodes, &config, 0.1, &());

        assert_eq!(result.networks.len(), 1);
        assert!(result.active_sources > 0);
    }

    #[test]
    fn apply_conduit_deltas_stored() {
        let mut conduits = make_conduits_with_stored(&[(8, 8, 8, 50.0)], ConduitKind::Power);

        let deltas = vec![ConduitDelta::stored(LocalPos::new(8, 8, 8), -10.0)];
        apply_conduit_deltas(&mut conduits, ConduitKind::Power, &deltas);

        let cell = conduits.get(ConduitKind::Power, LocalPos::new(8, 8, 8));
        assert!((cell.stored() - 40.0).abs() < 0.001);
    }

    #[test]
    fn apply_conduit_deltas_temperature() {
        let mut conduits = ChunkConduits::new();
        conduits.set(
            ConduitKind::Heat,
            LocalPos::new(5, 5, 5),
            ConduitCell::new(ConduitKind::Heat),
        );

        let deltas = vec![ConduitDelta::full(LocalPos::new(5, 5, 5), 0.0, 100.0, 1.0)];
        apply_conduit_deltas(&mut conduits, ConduitKind::Heat, &deltas);

        let cell = conduits.get(ConduitKind::Heat, LocalPos::new(5, 5, 5));
        assert!((cell.temperature() - 100.0).abs() < 0.001);
    }

    #[test]
    fn apply_conduit_deltas_pressure() {
        let mut conduits = ChunkConduits::new();
        conduits.set(
            ConduitKind::Fluid,
            LocalPos::new(3, 3, 3),
            ConduitCell::new(ConduitKind::Fluid),
        );

        let deltas = vec![ConduitDelta::full(LocalPos::new(3, 3, 3), 0.0, 20.0, 5.0)];
        apply_conduit_deltas(&mut conduits, ConduitKind::Fluid, &deltas);

        let cell = conduits.get(ConduitKind::Fluid, LocalPos::new(3, 3, 3));
        assert!((cell.pressure() - 5.0).abs() < 0.001);
    }

    #[test]
    fn network_fill_ratio() {
        let mut network = ConnectedNetwork::new(0, ConduitKind::Power);
        network.total_capacity = 100.0;
        network.total_stored = 25.0;

        assert!((network.fill_ratio() - 0.25).abs() < 0.001);
    }

    #[test]
    fn network_fill_ratio_empty_capacity() {
        let network = ConnectedNetwork::new(0, ConduitKind::Power);
        assert_eq!(network.fill_ratio(), 0.0);
    }

    #[test]
    fn conduit_delta_constructors() {
        let stored = ConduitDelta::stored(LocalPos::new(1, 2, 3), -5.0);
        assert_eq!(stored.pos, LocalPos::new(1, 2, 3));
        assert!((stored.stored_delta - (-5.0)).abs() < 0.001);
        assert!(stored.new_temperature.is_none());
        assert!(stored.new_pressure.is_none());

        let full = ConduitDelta::full(LocalPos::new(4, 5, 6), 2.0, 50.0, 3.0);
        assert_eq!(full.new_temperature, Some(50.0));
        assert_eq!(full.new_pressure, Some(3.0));
    }

    #[test]
    fn conduit_boundary_new() {
        let cell = ConduitCell::with_state(ConduitKind::Fluid, 5.0, 10.0, 0.1, 25.0, 2.0);
        let boundary = ConduitBoundary::new(
            LocalPos::new(0, 8, 8),
            (-1, 0, 0),
            ConduitKind::Fluid,
            0,
            &cell,
        );

        assert!(boundary.has_supply());
        assert!(boundary.has_demand());
        assert!((boundary.supply - 5.0).abs() < 0.001);
        assert!((boundary.temperature - 25.0).abs() < 0.001);
    }

    #[test]
    fn result_merge() {
        let mut a = ConduitNetworkResult {
            deltas: vec![ConduitDelta::stored(LocalPos::new(0, 0, 0), 1.0)],
            boundaries: vec![],
            networks: vec![ConnectedNetwork::new(0, ConduitKind::Power)],
            total_flow: 10.0,
            total_loss: 1.0,
            satisfied_sinks: 1,
            active_sources: 1,
        };

        let b = ConduitNetworkResult {
            deltas: vec![ConduitDelta::stored(LocalPos::new(1, 1, 1), 2.0)],
            boundaries: vec![],
            networks: vec![ConnectedNetwork::new(1, ConduitKind::Heat)],
            total_flow: 5.0,
            total_loss: 0.5,
            satisfied_sinks: 2,
            active_sources: 1,
        };

        a.merge(b);

        assert_eq!(a.deltas.len(), 2);
        assert_eq!(a.networks.len(), 2);
        assert!((a.total_flow - 15.0).abs() < 0.001);
        assert!((a.total_loss - 1.5).abs() < 0.001);
        assert_eq!(a.satisfied_sinks, 3);
        assert_eq!(a.active_sources, 2);
    }

    #[test]
    fn resistance_map_unit() {
        let map = ();
        assert_eq!(
            map.resistance(ConduitKind::Power, LocalPos::new(0, 0, 0)),
            0.0
        );
    }

    #[test]
    fn resistance_map_closure() {
        let map = |_kind: ConduitKind, pos: LocalPos| {
            if pos.x() == 5 { 0.5 } else { 0.0 }
        };

        assert_eq!(
            map.resistance(ConduitKind::Power, LocalPos::new(5, 0, 0)),
            0.5
        );
        assert_eq!(
            map.resistance(ConduitKind::Power, LocalPos::new(0, 0, 0)),
            0.0
        );
    }

    #[test]
    fn network_step_deterministic() {
        let conduits = make_conduits_line(ConduitKind::Power, 5);
        let nodes1 = vec![
            ConduitNode::source(LocalPos::new(0, 8, 8), ConduitKind::Power, 10.0),
            ConduitNode::sink(LocalPos::new(4, 8, 8), ConduitKind::Power, 10.0),
        ];
        let mut nodes2 = nodes1.clone();

        let config = ConduitNetworkConfig::for_kind(ConduitKind::Power);

        let result1 = network_step(
            &conduits,
            ConduitKind::Power,
            &mut nodes1.clone(),
            &config,
            0.1,
            &(),
        );
        let result2 = network_step(
            &conduits,
            ConduitKind::Power,
            &mut nodes2,
            &config,
            0.1,
            &(),
        );

        assert_eq!(result1.networks.len(), result2.networks.len());
        assert_eq!(result1.deltas.len(), result2.deltas.len());
    }

    #[test]
    fn serde_boundary_round_trip() {
        let cell = ConduitCell::new(ConduitKind::Fluid);
        let boundary = ConduitBoundary::new(
            LocalPos::new(15, 8, 8),
            (1, 0, 0),
            ConduitKind::Fluid,
            0,
            &cell,
        );

        let json = serde_json::to_string(&boundary).unwrap();
        let recovered: ConduitBoundary = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, boundary);
    }
}
