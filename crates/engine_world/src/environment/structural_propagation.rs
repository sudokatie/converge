//! Deterministic structural integrity simulation step logic.

use engine_core::coords::LocalPos;

use super::{
    ChunkStructural, StructuralBoundary, StructuralCell, StructuralConfig, StructuralEvent,
};

/// 6 face neighbor offsets (dx, dy, dz).
const FACE_NEIGHBORS: [(i32, i32, i32); 6] = [
    (-1, 0, 0),
    (1, 0, 0),
    (0, -1, 0),
    (0, 1, 0),
    (0, 0, -1),
    (0, 0, 1),
];

/// 12 edge neighbor offsets.
const EDGE_NEIGHBORS: [(i32, i32, i32); 12] = [
    (-1, -1, 0),
    (-1, 1, 0),
    (1, -1, 0),
    (1, 1, 0),
    (-1, 0, -1),
    (-1, 0, 1),
    (1, 0, -1),
    (1, 0, 1),
    (0, -1, -1),
    (0, -1, 1),
    (0, 1, -1),
    (0, 1, 1),
];

/// 8 corner neighbor offsets.
const CORNER_NEIGHBORS: [(i32, i32, i32); 8] = [
    (-1, -1, -1),
    (-1, -1, 1),
    (-1, 1, -1),
    (-1, 1, 1),
    (1, -1, -1),
    (1, -1, 1),
    (1, 1, -1),
    (1, 1, 1),
];

/// A delta representing a change to a structural cell.
#[derive(Clone, Debug, PartialEq)]
pub struct StructuralDelta {
    /// Position within chunk.
    pub pos: LocalPos,
    /// New support distance (None = mark unsupported).
    pub support_distance: Option<u8>,
    /// Load change to apply.
    pub load_delta: f32,
    /// Integrity change to apply.
    pub integrity_delta: f32,
    /// Whether cell should collapse.
    pub collapse: bool,
}

impl StructuralDelta {
    /// Create a support update delta.
    #[must_use]
    pub fn support(pos: LocalPos, distance: u8) -> Self {
        Self {
            pos,
            support_distance: Some(distance),
            load_delta: 0.0,
            integrity_delta: 0.0,
            collapse: false,
        }
    }

    /// Create an unsupported delta.
    #[must_use]
    pub fn unsupported(pos: LocalPos) -> Self {
        Self {
            pos,
            support_distance: None,
            load_delta: 0.0,
            integrity_delta: 0.0,
            collapse: false,
        }
    }

    /// Create a load change delta.
    #[must_use]
    pub fn load(pos: LocalPos, delta: f32) -> Self {
        Self {
            pos,
            support_distance: None,
            load_delta: delta,
            integrity_delta: 0.0,
            collapse: false,
        }
    }

    /// Create a collapse delta.
    #[must_use]
    pub fn collapse(pos: LocalPos) -> Self {
        Self {
            pos,
            support_distance: None,
            load_delta: 0.0,
            integrity_delta: 0.0,
            collapse: true,
        }
    }

    /// Create a damage delta.
    #[must_use]
    pub fn damage(pos: LocalPos, damage: f32) -> Self {
        Self {
            pos,
            support_distance: None,
            load_delta: 0.0,
            integrity_delta: -damage.abs(),
            collapse: false,
        }
    }
}

/// Pressure lookup for decompression calculations.
pub trait PressureMap {
    /// Get pressure at a local position (in atmospheres).
    fn pressure(&self, pos: LocalPos) -> f32;
}

impl PressureMap for () {
    fn pressure(&self, _pos: LocalPos) -> f32 {
        1.0
    }
}

impl<F> PressureMap for F
where
    F: Fn(LocalPos) -> f32,
{
    fn pressure(&self, pos: LocalPos) -> f32 {
        self(pos)
    }
}

/// Material strength lookup for per-cell configuration.
pub trait StrengthMap {
    /// Get strength multiplier at position (1.0 = normal).
    fn strength(&self, pos: LocalPos) -> f32;
}

impl StrengthMap for () {
    fn strength(&self, _pos: LocalPos) -> f32 {
        1.0
    }
}

impl<F> StrengthMap for F
where
    F: Fn(LocalPos) -> f32,
{
    fn strength(&self, pos: LocalPos) -> f32 {
        self(pos).max(0.0)
    }
}

/// Result of a structural simulation step.
#[derive(Clone, Debug, Default)]
pub struct StructuralResult {
    /// Deltas to apply within this chunk.
    pub deltas: Vec<StructuralDelta>,
    /// Structural events that occurred.
    pub events: Vec<StructuralEvent>,
    /// Boundary cells needing cross-chunk coordination.
    pub boundaries: Vec<StructuralBoundary>,
    /// Number of cells that gained support.
    pub supported_count: u32,
    /// Number of cells that lost support.
    pub unsupported_count: u32,
    /// Number of cells that collapsed.
    pub collapsed_count: u32,
    /// Number of cells in cave-in cascades.
    pub cavein_count: u32,
}

impl StructuralResult {
    /// Create empty result.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any changes occurred.
    #[must_use]
    pub fn has_changes(&self) -> bool {
        !self.deltas.is_empty() || !self.events.is_empty()
    }

    /// Merge another result into this one.
    pub fn merge(&mut self, other: Self) {
        self.deltas.extend(other.deltas);
        self.events.extend(other.events);
        self.boundaries.extend(other.boundaries);
        self.supported_count += other.supported_count;
        self.unsupported_count += other.unsupported_count;
        self.collapsed_count += other.collapsed_count;
        self.cavein_count += other.cavein_count;
    }
}

/// Execute a deterministic support propagation step.
pub fn propagate_support(
    structural: &ChunkStructural,
    config: &StructuralConfig,
) -> StructuralResult {
    let mut result = StructuralResult::new();

    let mut cells: Vec<(LocalPos, StructuralCell)> = structural.iter_structural().collect();
    cells.sort_by_key(|(pos, _)| pos.to_index());

    let mut support_map: Vec<(LocalPos, u8)> = Vec::new();
    let mut visited = [false; 16 * 16 * 16];

    for (pos, cell) in &cells {
        if cell.support_kind().is_foundation() {
            support_map.push((*pos, 0));
            visited[pos.to_index()] = true;
        }
    }

    let max_dist = config.propagation.max_propagation_distance;
    let mut frontier = support_map.clone();

    while !frontier.is_empty() {
        frontier.sort_by_key(|(pos, dist)| (*dist, pos.to_index()));

        let mut next_frontier = Vec::new();

        for (pos, dist) in frontier {
            if dist >= max_dist {
                continue;
            }

            let neighbors = get_neighbor_offsets(config);

            for &(dx, dy, dz) in &neighbors {
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
                    report_boundary(&mut result, pos, (dx, dy, dz), structural, dist);
                    continue;
                }

                #[expect(
                    clippy::cast_sign_loss,
                    reason = "bounds check above guarantees nx, ny, nz are in 0..16"
                )]
                let neighbor_pos = LocalPos::new(nx as u32, ny as u32, nz as u32);
                let neighbor_idx = neighbor_pos.to_index();

                if visited[neighbor_idx] {
                    continue;
                }

                let neighbor_cell = structural.get(neighbor_pos);
                if !neighbor_cell.support_kind().provides_support() {
                    continue;
                }

                let new_dist = dist.saturating_add(1);
                if new_dist <= neighbor_cell.support_kind().support_range() {
                    visited[neighbor_idx] = true;
                    support_map.push((neighbor_pos, new_dist));
                    next_frontier.push((neighbor_pos, new_dist));
                    result.supported_count += 1;
                }
            }
        }

        frontier = next_frontier;
    }

    for (pos, cell) in &cells {
        let idx = pos.to_index();
        if !visited[idx] && cell.support_kind().provides_support() && cell.is_supported() {
            result.deltas.push(StructuralDelta::unsupported(*pos));
            result.unsupported_count += 1;
            result
                .events
                .push(StructuralEvent::support_lost(*pos, cell.support_kind()));
        }
    }

    for (pos, dist) in support_map {
        let cell = structural.get(pos);
        if !cell.is_supported() || cell.support_distance() != dist {
            result.deltas.push(StructuralDelta::support(pos, dist));
        }
    }

    result
}

/// Execute a deterministic load distribution step.
pub fn distribute_load<S: StrengthMap>(
    structural: &ChunkStructural,
    config: &StructuralConfig,
    dt: f32,
    strength_map: &S,
) -> StructuralResult {
    let mut result = StructuralResult::new();

    let mut cells: Vec<(LocalPos, StructuralCell)> = structural.iter_supported().collect();
    cells.sort_by_key(|(pos, _)| pos.to_index());

    if cells.is_empty() {
        return result;
    }

    for (pos, cell) in &cells {
        let strength = strength_map.strength(*pos);
        let base_load = config.load.base_cell_load * (1.0 / strength.max(0.1));

        let above_pos = LocalPos::new(pos.x(), pos.y().saturating_add(1).min(15), pos.z());
        let above_cell = structural.get(above_pos);
        let inherited_load = if above_cell.support_kind().provides_support() {
            above_cell.load() * config.load.gravity_factor * config.load.distribution_rate * dt
        } else {
            0.0
        };

        let total_load_delta = (base_load + inherited_load) * config.load.accumulation_rate * dt;

        if total_load_delta.abs() > 0.001 {
            result
                .deltas
                .push(StructuralDelta::load(*pos, total_load_delta));
        }

        let capacity = cell.support_kind().max_load_factor() * strength * cell.integrity();
        let new_load = cell.load() + total_load_delta;
        let stress = if capacity > 0.0 {
            new_load / capacity
        } else {
            1.0
        };

        if stress >= config.stability.failure_threshold {
            result.deltas.push(StructuralDelta::collapse(*pos));
            result.collapsed_count += 1;
            result.events.push(StructuralEvent::stress_failure(
                *pos,
                cell.support_kind(),
                stress,
                new_load,
            ));
        } else if stress > config.stability.warning_threshold {
            let damage = config.stability.overstress_damage_rate * dt;
            result.deltas.push(StructuralDelta::damage(*pos, damage));
        }
    }

    result
}

/// Check for decompression damage based on pressure differentials.
pub fn check_decompression<P: PressureMap>(
    structural: &ChunkStructural,
    config: &StructuralConfig,
    pressure_map: &P,
    _dt: f32,
) -> StructuralResult {
    let mut result = StructuralResult::new();

    if !config.decompression_enabled {
        return result;
    }

    let mut cells: Vec<(LocalPos, StructuralCell)> = structural.iter_supported().collect();
    cells.sort_by_key(|(pos, _)| pos.to_index());

    for (pos, cell) in &cells {
        let pressure = pressure_map.pressure(*pos);

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
                continue;
            }

            #[expect(
                clippy::cast_sign_loss,
                reason = "bounds check above guarantees nx, ny, nz are in 0..16"
            )]
            let neighbor_pos = LocalPos::new(nx as u32, ny as u32, nz as u32);
            let neighbor_pressure = pressure_map.pressure(neighbor_pos);
            let pressure_diff = (pressure - neighbor_pressure).abs();

            if pressure_diff >= config.decompression_threshold {
                let damage = pressure_diff * 0.1;
                result.deltas.push(StructuralDelta::damage(*pos, damage));
                result.events.push(StructuralEvent::decompression(
                    *pos,
                    cell.support_kind(),
                    damage,
                ));
                break;
            }
        }
    }

    result
}

/// Detect and report cave-in cascades.
pub fn detect_cavein(structural: &ChunkStructural, config: &StructuralConfig) -> StructuralResult {
    let mut result = StructuralResult::new();

    if !config.cavein_enabled {
        return result;
    }

    let unsupported: Vec<(LocalPos, StructuralCell)> = structural.iter_unsupported().collect();

    if unsupported.is_empty() {
        return result;
    }

    let mut visited = [false; 16 * 16 * 16];
    let mut cavein_groups: Vec<Vec<LocalPos>> = Vec::new();

    for (start_pos, _) in &unsupported {
        let idx = start_pos.to_index();
        if visited[idx] {
            continue;
        }

        let mut group = Vec::new();
        let mut stack = vec![*start_pos];

        while let Some(pos) = stack.pop() {
            let pos_idx = pos.to_index();
            if visited[pos_idx] {
                continue;
            }
            visited[pos_idx] = true;

            let cell = structural.get(pos);
            if !cell.support_kind().provides_support() || cell.is_supported() {
                continue;
            }

            group.push(pos);

            #[expect(
                clippy::cast_possible_truncation,
                reason = "group size capped by max_cascade_size (u32), cannot exceed u32::MAX"
            )]
            if group.len() as u32 >= config.max_cascade_size {
                break;
            }

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

                if (0..16).contains(&nx) && (0..16).contains(&ny) && (0..16).contains(&nz) {
                    #[expect(
                        clippy::cast_sign_loss,
                        reason = "bounds check above guarantees nx, ny, nz are in 0..16"
                    )]
                    let neighbor_pos = LocalPos::new(nx as u32, ny as u32, nz as u32);
                    if !visited[neighbor_pos.to_index()] {
                        stack.push(neighbor_pos);
                    }
                }
            }
        }

        if group.len() >= 2 {
            cavein_groups.push(group);
        }
    }

    for group in cavein_groups {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "group size capped by max_cascade_size (u32), cannot exceed u32::MAX"
        )]
        let count = group.len() as u32;
        let origin = group[0];

        for pos in &group {
            result.deltas.push(StructuralDelta::collapse(*pos));
        }

        result.cavein_count += count;
        result.events.push(StructuralEvent::cavein(origin, count));
    }

    result
}

/// Execute a full structural integrity step.
pub fn structural_step<P: PressureMap, S: StrengthMap>(
    structural: &ChunkStructural,
    config: &StructuralConfig,
    dt: f32,
    pressure_map: &P,
    strength_map: &S,
) -> StructuralResult {
    let mut result = propagate_support(structural, config);

    let load_result = distribute_load(structural, config, dt, strength_map);
    result.merge(load_result);

    let decomp_result = check_decompression(structural, config, pressure_map, dt);
    result.merge(decomp_result);

    let cavein_result = detect_cavein(structural, config);
    result.merge(cavein_result);

    result
}

/// Apply deltas to structural storage.
pub fn apply_structural_deltas(structural: &mut ChunkStructural, deltas: &[StructuralDelta]) {
    for delta in deltas {
        let cell = structural.get_mut(delta.pos);

        if delta.collapse {
            cell.set_integrity(0.0);
            cell.mark_unsupported();
            continue;
        }

        if let Some(dist) = delta.support_distance {
            cell.mark_supported(dist);
        } else if delta.load_delta == 0.0 && delta.integrity_delta == 0.0 {
            cell.mark_unsupported();
        }

        if delta.load_delta.abs() > 0.0001 {
            cell.add_load(delta.load_delta);
        }

        if delta.integrity_delta.abs() > 0.0001 {
            cell.apply_damage(-delta.integrity_delta);
        }
    }

    structural.recount();
}

fn get_neighbor_offsets(config: &StructuralConfig) -> Vec<(i32, i32, i32)> {
    let mut offsets = FACE_NEIGHBORS.to_vec();
    if config.propagation.include_edge_neighbors {
        offsets.extend_from_slice(&EDGE_NEIGHBORS);
    }
    if config.propagation.include_corner_neighbors {
        offsets.extend_from_slice(&CORNER_NEIGHBORS);
    }
    offsets
}

fn report_boundary(
    result: &mut StructuralResult,
    pos: LocalPos,
    direction: (i32, i32, i32),
    structural: &ChunkStructural,
    support_distance: u8,
) {
    let cell = structural.get(pos);
    if cell.support_kind().provides_support() {
        result.boundaries.push(StructuralBoundary::new(
            pos,
            direction,
            cell.support_kind(),
            cell.is_supported(),
            support_distance,
            cell.load(),
        ));
    }
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    reason = "tests check exact constructor return values"
)]
mod tests {
    use super::super::SupportKind;
    use super::*;

    fn make_structural_with_foundation() -> ChunkStructural {
        let mut structural = ChunkStructural::new();
        structural.set_support(LocalPos::new(8, 0, 8), SupportKind::Foundation);
        structural
    }

    #[test]
    fn propagate_support_empty() {
        let structural = ChunkStructural::new();
        let config = StructuralConfig::DEFAULT;
        let result = propagate_support(&structural, &config);
        assert!(!result.has_changes());
    }

    #[test]
    fn propagate_support_foundation_only() {
        let structural = make_structural_with_foundation();
        let config = StructuralConfig::DEFAULT;
        let result = propagate_support(&structural, &config);
        assert_eq!(result.unsupported_count, 0);
    }

    #[test]
    fn propagate_support_to_column() {
        let mut structural = make_structural_with_foundation();
        structural.set_support(LocalPos::new(8, 1, 8), SupportKind::Column);

        let config = StructuralConfig::DEFAULT;
        let result = propagate_support(&structural, &config);

        assert!(result.supported_count >= 1);
        let support_delta = result
            .deltas
            .iter()
            .find(|d| d.pos == LocalPos::new(8, 1, 8) && d.support_distance.is_some());
        assert!(support_delta.is_some());
    }

    #[test]
    fn propagate_support_max_distance() {
        let mut structural = ChunkStructural::new();
        structural.set_support(LocalPos::new(0, 0, 0), SupportKind::Foundation);

        for i in 1..15 {
            structural.set_support(LocalPos::new(i, 0, 0), SupportKind::Solid);
        }

        let mut config = StructuralConfig::DEFAULT;
        config.propagation.max_propagation_distance = 5;

        let result = propagate_support(&structural, &config);

        let supported_positions: Vec<_> = result
            .deltas
            .iter()
            .filter(|d| d.support_distance.is_some())
            .map(|d| d.pos.x())
            .collect();

        assert!(supported_positions.iter().all(|&x| x <= 5));
        assert!(result.supported_count <= 5);
    }

    #[test]
    fn distribute_load_accumulates() {
        let mut structural = make_structural_with_foundation();
        structural.set_support(LocalPos::new(8, 1, 8), SupportKind::Column);
        structural.get_mut(LocalPos::new(8, 1, 8)).mark_supported(1);
        structural.recount();

        let config = StructuralConfig::DEFAULT;
        let result = distribute_load(&structural, &config, 1.0, &());

        let load_delta = result.deltas.iter().find(|d| d.load_delta > 0.0);
        assert!(load_delta.is_some());
    }

    #[test]
    fn distribute_load_causes_collapse() {
        let mut structural = make_structural_with_foundation();
        let pos = LocalPos::new(8, 1, 8);
        structural.set_support(pos, SupportKind::Weak);
        structural.get_mut(pos).mark_supported(1);
        structural.get_mut(pos).add_load(0.9);
        structural.recount();

        let config = StructuralConfig::DEFAULT;
        let result = distribute_load(&structural, &config, 5.0, &());

        assert!(result.collapsed_count > 0 || result.deltas.iter().any(|d| d.collapse));
    }

    #[test]
    fn check_decompression_damage() {
        let mut structural = make_structural_with_foundation();
        structural.set_support(LocalPos::new(8, 1, 8), SupportKind::Solid);
        structural.get_mut(LocalPos::new(8, 1, 8)).mark_supported(1);
        structural.recount();

        let config = StructuralConfig::DEFAULT;

        let pressure_map = |pos: LocalPos| {
            if pos.y() == 0 { 1.0 } else { 0.0 }
        };

        let result = check_decompression(&structural, &config, &pressure_map, 0.1);

        let has_decompression = result
            .events
            .iter()
            .any(|e| matches!(e.kind, super::super::StructuralEventKind::Decompression));
        assert!(has_decompression);
    }

    #[test]
    fn decompression_disabled() {
        let mut structural = make_structural_with_foundation();
        structural.set_support(LocalPos::new(8, 1, 8), SupportKind::Solid);
        structural.get_mut(LocalPos::new(8, 1, 8)).mark_supported(1);
        structural.recount();

        let mut config = StructuralConfig::DEFAULT;
        config.decompression_enabled = false;

        let pressure_map = |pos: LocalPos| {
            if pos.y() == 0 { 1.0 } else { 0.0 }
        };

        let result = check_decompression(&structural, &config, &pressure_map, 0.1);
        assert!(!result.has_changes());
    }

    #[test]
    fn detect_cavein_groups() {
        let mut structural = ChunkStructural::new();
        for x in 5..10 {
            for z in 5..10 {
                structural.set_support(LocalPos::new(x, 5, z), SupportKind::Solid);
            }
        }
        structural.recount();

        let config = StructuralConfig::DEFAULT;
        let result = detect_cavein(&structural, &config);

        assert!(result.cavein_count > 0);
        let has_cavein_event = result
            .events
            .iter()
            .any(|e| matches!(e.kind, super::super::StructuralEventKind::CaveIn));
        assert!(has_cavein_event);
    }

    #[test]
    fn detect_cavein_max_size() {
        let mut structural = ChunkStructural::new();
        for x in 0..16 {
            for y in 0..16 {
                for z in 0..16 {
                    structural.set_support(LocalPos::new(x, y, z), SupportKind::Solid);
                }
            }
        }
        structural.recount();

        let mut config = StructuralConfig::DEFAULT;
        config.max_cascade_size = 10;

        let result = detect_cavein(&structural, &config);

        for event in &result.events {
            if matches!(event.kind, super::super::StructuralEventKind::CaveIn) {
                assert!(
                    event.cells_affected <= config.max_cascade_size,
                    "single cave-in event exceeded max cascade size"
                );
            }
        }
    }

    #[test]
    fn detect_cavein_disabled() {
        let mut structural = ChunkStructural::new();
        for x in 5..10 {
            structural.set_support(LocalPos::new(x, 5, 5), SupportKind::Solid);
        }
        structural.recount();

        let mut config = StructuralConfig::DEFAULT;
        config.cavein_enabled = false;

        let result = detect_cavein(&structural, &config);
        assert_eq!(result.cavein_count, 0);
    }

    #[test]
    fn structural_step_full() {
        let mut structural = make_structural_with_foundation();
        structural.set_support(LocalPos::new(8, 1, 8), SupportKind::Column);
        structural.set_support(LocalPos::new(8, 2, 8), SupportKind::Column);
        structural.recount();

        let config = StructuralConfig::DEFAULT;
        let result = structural_step(&structural, &config, 0.1, &(), &());

        assert!(result.has_changes());
    }

    #[test]
    fn apply_deltas_support() {
        let mut structural = ChunkStructural::new();
        structural.set_support(LocalPos::new(5, 5, 5), SupportKind::Column);

        let deltas = vec![StructuralDelta::support(LocalPos::new(5, 5, 5), 3)];

        apply_structural_deltas(&mut structural, &deltas);

        let cell = structural.get(LocalPos::new(5, 5, 5));
        assert!(cell.is_supported());
        assert_eq!(cell.support_distance(), 3);
    }

    #[test]
    fn apply_deltas_collapse() {
        let mut structural = ChunkStructural::new();
        structural.set_support(LocalPos::new(5, 5, 5), SupportKind::Solid);
        structural.get_mut(LocalPos::new(5, 5, 5)).mark_supported(1);
        structural.recount();

        let deltas = vec![StructuralDelta::collapse(LocalPos::new(5, 5, 5))];

        apply_structural_deltas(&mut structural, &deltas);

        let cell = structural.get(LocalPos::new(5, 5, 5));
        assert_eq!(cell.integrity(), 0.0);
        assert!(!cell.is_supported());
    }

    #[test]
    fn apply_deltas_load() {
        let mut structural = ChunkStructural::new();
        structural.set_support(LocalPos::new(5, 5, 5), SupportKind::Column);
        structural.get_mut(LocalPos::new(5, 5, 5)).mark_supported(1);

        let deltas = vec![StructuralDelta::load(LocalPos::new(5, 5, 5), 0.25)];

        apply_structural_deltas(&mut structural, &deltas);

        let cell = structural.get(LocalPos::new(5, 5, 5));
        assert!((cell.load() - 0.25).abs() < 0.01);
    }

    #[test]
    fn apply_deltas_damage() {
        let mut structural = ChunkStructural::new();
        structural.set_support(LocalPos::new(5, 5, 5), SupportKind::Beam);

        let deltas = vec![StructuralDelta::damage(LocalPos::new(5, 5, 5), 0.3)];

        apply_structural_deltas(&mut structural, &deltas);

        let cell = structural.get(LocalPos::new(5, 5, 5));
        assert!((cell.integrity() - 0.7).abs() < 0.01);
    }

    #[test]
    fn result_merge() {
        let mut a = StructuralResult {
            deltas: vec![StructuralDelta::support(LocalPos::new(0, 0, 0), 1)],
            events: vec![StructuralEvent::collapse(
                LocalPos::new(0, 0, 0),
                SupportKind::Solid,
            )],
            boundaries: vec![],
            supported_count: 1,
            unsupported_count: 0,
            collapsed_count: 1,
            cavein_count: 0,
        };

        let b = StructuralResult {
            deltas: vec![StructuralDelta::load(LocalPos::new(1, 1, 1), 0.5)],
            events: vec![],
            boundaries: vec![],
            supported_count: 2,
            unsupported_count: 1,
            collapsed_count: 0,
            cavein_count: 5,
        };

        a.merge(b);

        assert_eq!(a.deltas.len(), 2);
        assert_eq!(a.events.len(), 1);
        assert_eq!(a.supported_count, 3);
        assert_eq!(a.unsupported_count, 1);
        assert_eq!(a.collapsed_count, 1);
        assert_eq!(a.cavein_count, 5);
    }

    #[test]
    fn boundary_reporting() {
        let mut structural = ChunkStructural::new();
        structural.set_support(LocalPos::new(0, 0, 0), SupportKind::Foundation);
        structural.set_support(LocalPos::new(0, 0, 1), SupportKind::Column);

        let config = StructuralConfig::DEFAULT;
        let result = propagate_support(&structural, &config);

        assert!(!result.boundaries.is_empty());
    }

    #[test]
    fn deterministic_order() {
        let mut structural = ChunkStructural::new();
        structural.set_support(LocalPos::new(0, 0, 0), SupportKind::Foundation);
        structural.set_support(LocalPos::new(5, 5, 5), SupportKind::Column);
        structural.set_support(LocalPos::new(2, 2, 2), SupportKind::Beam);

        let config = StructuralConfig::DEFAULT;

        let result1 = propagate_support(&structural, &config);
        let result2 = propagate_support(&structural, &config);

        assert_eq!(result1.deltas.len(), result2.deltas.len());
        for (d1, d2) in result1.deltas.iter().zip(result2.deltas.iter()) {
            assert_eq!(d1.pos, d2.pos);
            assert_eq!(d1.support_distance, d2.support_distance);
        }
    }

    #[test]
    fn strength_map_affects_capacity() {
        let mut structural = make_structural_with_foundation();
        let pos = LocalPos::new(8, 1, 8);
        structural.set_support(pos, SupportKind::Column);
        structural.get_mut(pos).mark_supported(1);
        structural.get_mut(pos).add_load(0.5);
        structural.recount();

        let config = StructuralConfig::DEFAULT;

        let weak_strength = |_: LocalPos| 0.5f32;
        let result = distribute_load(&structural, &config, 1.0, &weak_strength);

        assert!(result.has_changes());
    }

    #[test]
    fn delta_constructors() {
        let support = StructuralDelta::support(LocalPos::new(1, 2, 3), 5);
        assert_eq!(support.support_distance, Some(5));
        assert!(!support.collapse);

        let unsupported = StructuralDelta::unsupported(LocalPos::new(4, 5, 6));
        assert_eq!(unsupported.support_distance, None);

        let load = StructuralDelta::load(LocalPos::new(7, 8, 9), 0.3);
        assert!((load.load_delta - 0.3).abs() < 0.001);

        let collapse = StructuralDelta::collapse(LocalPos::new(10, 11, 12));
        assert!(collapse.collapse);

        let damage = StructuralDelta::damage(LocalPos::new(13, 14, 15), 0.2);
        assert!((damage.integrity_delta - (-0.2)).abs() < 0.001);
    }
}
