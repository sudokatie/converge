//! Deterministic hazard propagation step logic.

use engine_core::coords::LocalPos;

use super::{ChunkHazards, DecayConfig, HazardCell, HazardKind, PropagationConfig, Resistance};

/// Offset to a neighbor cell (dx, dy, dz, `weight_type`).
/// `weight_type`: 0 = face, 1 = edge, 2 = corner.
const NEIGHBOR_OFFSETS: [(i32, i32, i32, u8); 26] = [
    // 6 face neighbors
    (-1, 0, 0, 0),
    (1, 0, 0, 0),
    (0, -1, 0, 0),
    (0, 1, 0, 0),
    (0, 0, -1, 0),
    (0, 0, 1, 0),
    // 12 edge neighbors
    (-1, -1, 0, 1),
    (-1, 1, 0, 1),
    (1, -1, 0, 1),
    (1, 1, 0, 1),
    (-1, 0, -1, 1),
    (-1, 0, 1, 1),
    (1, 0, -1, 1),
    (1, 0, 1, 1),
    (0, -1, -1, 1),
    (0, -1, 1, 1),
    (0, 1, -1, 1),
    (0, 1, 1, 1),
    // 8 corner neighbors
    (-1, -1, -1, 2),
    (-1, -1, 1, 2),
    (-1, 1, -1, 2),
    (-1, 1, 1, 2),
    (1, -1, -1, 2),
    (1, -1, 1, 2),
    (1, 1, -1, 2),
    (1, 1, 1, 2),
];

/// A delta representing a change to a single cell.
#[derive(Clone, Debug, PartialEq)]
pub struct CellDelta {
    /// Position within chunk.
    pub pos: LocalPos,
    /// New intensity (None = deactivate).
    pub intensity: Option<f32>,
}

impl CellDelta {
    /// Create a delta to set intensity.
    #[must_use]
    pub fn set(pos: LocalPos, intensity: f32) -> Self {
        Self {
            pos,
            intensity: Some(intensity),
        }
    }

    /// Create a delta to deactivate.
    #[must_use]
    pub fn deactivate(pos: LocalPos) -> Self {
        Self {
            pos,
            intensity: None,
        }
    }
}

/// Result of a propagation step.
#[derive(Clone, Debug, Default)]
pub struct PropagationResult {
    /// Deltas to apply within this chunk.
    pub deltas: Vec<CellDelta>,

    /// Cells that want to spread to neighbor chunks (pos, direction, intensity).
    pub boundary_spreads: Vec<(LocalPos, (i32, i32, i32), f32)>,

    /// Number of cells that decayed.
    pub decayed_count: u32,

    /// Number of cells that spread.
    pub spread_count: u32,

    /// Number of cells extinguished.
    pub extinguished_count: u32,
}

impl PropagationResult {
    /// Create an empty result.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any changes occurred.
    #[must_use]
    pub fn has_changes(&self) -> bool {
        !self.deltas.is_empty() || !self.boundary_spreads.is_empty()
    }

    /// Merge another result into this one.
    pub fn merge(&mut self, other: Self) {
        self.deltas.extend(other.deltas);
        self.boundary_spreads.extend(other.boundary_spreads);
        self.decayed_count += other.decayed_count;
        self.spread_count += other.spread_count;
        self.extinguished_count += other.extinguished_count;
    }
}

/// Resistance lookup for spread calculations.
pub trait ResistanceMap {
    /// Get resistance at a local position for a hazard kind.
    fn resistance(&self, kind: HazardKind, pos: LocalPos) -> Resistance;
}

/// Default implementation: no resistance anywhere.
impl ResistanceMap for () {
    fn resistance(&self, _kind: HazardKind, _pos: LocalPos) -> Resistance {
        Resistance::NONE
    }
}

/// Function-based resistance map.
impl<F> ResistanceMap for F
where
    F: Fn(HazardKind, LocalPos) -> Resistance,
{
    fn resistance(&self, kind: HazardKind, pos: LocalPos) -> Resistance {
        self(kind, pos)
    }
}

/// Execute a deterministic propagation step.
pub fn propagation_step<R: ResistanceMap>(
    hazards: &ChunkHazards,
    kind: HazardKind,
    config: &PropagationConfig,
    dt: f32,
    resistance_map: &R,
) -> PropagationResult {
    let mut result = PropagationResult::new();

    let layer = match hazards.layer(kind) {
        Some(l) if l.active_count() > 0 => l,
        _ => return result,
    };

    let mut active_cells: Vec<(LocalPos, HazardCell)> = layer.iter_active().collect();
    active_cells.sort_by_key(|(pos, _)| pos.to_index());

    let spread_interval = config.spread.spread_interval();

    for (pos, cell) in active_cells {
        let mut new_cell = cell;

        if config.decay.is_active() {
            new_cell.tick_decay(dt);

            if new_cell.decay_timer() >= config.decay.grace_period {
                let decay_amount = config.decay.rate * dt;
                let new_intensity = (new_cell.intensity() - decay_amount).max(0.0);
                new_cell.set_intensity(new_intensity);
                result.decayed_count += 1;
            }
        }

        if new_cell.intensity() < config.decay.extinction_threshold && !config.persist_at_zero {
            result.deltas.push(CellDelta::deactivate(pos));
            result.extinguished_count += 1;
            continue;
        }

        if config.spread.is_active()
            && new_cell.intensity() >= config.spread.min_intensity
            && new_cell.tick_spread(dt, spread_interval)
        {
            spread_to_neighbors(
                pos,
                new_cell.intensity(),
                kind,
                config,
                layer,
                resistance_map,
                &mut result,
            );
        }

        #[expect(
            clippy::float_cmp,
            reason = "exact comparison intentional: detecting timer changes for state tracking"
        )]
        let timers_changed = new_cell.spread_timer() != cell.spread_timer()
            || new_cell.decay_timer() != cell.decay_timer();

        #[expect(
            clippy::float_cmp,
            reason = "exact comparison intentional: detecting intensity changes for state tracking"
        )]
        let intensity_changed = new_cell.intensity() != cell.intensity();

        if intensity_changed || timers_changed {
            result
                .deltas
                .push(CellDelta::set(pos, new_cell.intensity()));
        }
    }

    result
}

fn spread_to_neighbors<R: ResistanceMap>(
    pos: LocalPos,
    intensity: f32,
    kind: HazardKind,
    config: &PropagationConfig,
    layer: &super::HazardLayer,
    resistance_map: &R,
    result: &mut PropagationResult,
) {
    let spread = &config.spread;
    let base_transfer = intensity * spread.transfer_fraction;

    for &(dx, dy, dz, weight_type) in &NEIGHBOR_OFFSETS {
        let weight = match weight_type {
            0 => spread.face_weight,
            1 => spread.edge_weight,
            2 => spread.corner_weight,
            _ => 0.0,
        };

        if weight <= 0.0 {
            continue;
        }

        let mut transfer = base_transfer * weight;

        if kind.gravity_affected() && dy == -1 {
            transfer *= spread.gravity_multiplier;
        }

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
            result.boundary_spreads.push((pos, (dx, dy, dz), transfer));
            continue;
        }

        #[expect(
            clippy::cast_sign_loss,
            reason = "bounds check above guarantees nx, ny, nz are in 0..16"
        )]
        let neighbor_pos = LocalPos::new(nx as u32, ny as u32, nz as u32);
        let resistance = resistance_map.resistance(kind, neighbor_pos);

        if resistance.blocks() {
            continue;
        }

        let effective_transfer = resistance.apply(transfer);
        let neighbor_cell = layer.get(neighbor_pos);
        let current_intensity = neighbor_cell.intensity();

        if current_intensity < effective_transfer {
            let new_intensity = (current_intensity + effective_transfer).min(config.max_intensity);
            result
                .deltas
                .push(CellDelta::set(neighbor_pos, new_intensity));
            result.spread_count += 1;
        }
    }
}

/// Apply deltas to hazards storage.
pub fn apply_deltas(hazards: &mut ChunkHazards, kind: HazardKind, deltas: &[CellDelta]) {
    for delta in deltas {
        match delta.intensity {
            Some(intensity) => {
                let cell = hazards.layer_mut(kind).get_mut(delta.pos);
                cell.set_intensity(intensity);
            }
            None => {
                hazards.deactivate(kind, delta.pos);
            }
        }
    }
}

/// Execute decay-only step (no spread).
pub fn decay_step(
    hazards: &ChunkHazards,
    kind: HazardKind,
    config: &DecayConfig,
    dt: f32,
    extinction_threshold: f32,
) -> PropagationResult {
    let mut result = PropagationResult::new();

    let layer = match hazards.layer(kind) {
        Some(l) if l.active_count() > 0 => l,
        _ => return result,
    };

    if !config.is_active() {
        return result;
    }

    let mut active_cells: Vec<(LocalPos, HazardCell)> = layer.iter_active().collect();
    active_cells.sort_by_key(|(pos, _)| pos.to_index());

    for (pos, cell) in active_cells {
        let mut new_cell = cell;
        new_cell.tick_decay(dt);

        if new_cell.decay_timer() >= config.grace_period {
            let decay_amount = config.rate * dt;
            let new_intensity = (new_cell.intensity() - decay_amount).max(0.0);
            new_cell.set_intensity(new_intensity);
            result.decayed_count += 1;
        }

        if new_cell.intensity() < extinction_threshold {
            result.deltas.push(CellDelta::deactivate(pos));
            result.extinguished_count += 1;
        } else if (new_cell.intensity() - cell.intensity()).abs() > f32::EPSILON {
            result
                .deltas
                .push(CellDelta::set(pos, new_cell.intensity()));
        }
    }

    result
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    clippy::uninlined_format_args,
    reason = "tests check exact values; format args clearer with explicit args"
)]
mod tests {
    use super::*;

    fn make_hazards_with_fire(positions: &[(u32, u32, u32, f32)]) -> ChunkHazards {
        let mut hazards = ChunkHazards::new();
        for &(x, y, z, intensity) in positions {
            hazards.activate(HazardKind::Fire, LocalPos::new(x, y, z), intensity);
        }
        hazards
    }

    #[test]
    fn propagation_step_empty_layer() {
        let hazards = ChunkHazards::new();
        let config = PropagationConfig::new(HazardKind::Fire);
        let result = propagation_step(&hazards, HazardKind::Fire, &config, 0.1, &());
        assert!(!result.has_changes());
    }

    #[test]
    fn propagation_step_spread_to_face_neighbors() {
        let hazards = make_hazards_with_fire(&[(8, 8, 8, 1.0)]);
        let config = PropagationConfig::new(HazardKind::Fire);

        let result = propagation_step(&hazards, HazardKind::Fire, &config, 1.0, &());

        assert!(result.spread_count > 0);
        let spread_positions: Vec<_> = result
            .deltas
            .iter()
            .filter(|d| d.intensity.is_some())
            .map(|d| d.pos)
            .collect();

        assert!(spread_positions.contains(&LocalPos::new(7, 8, 8)));
        assert!(spread_positions.contains(&LocalPos::new(9, 8, 8)));
    }

    #[test]
    fn propagation_step_decay() {
        let hazards = make_hazards_with_fire(&[(5, 5, 5, 0.5)]);
        let mut config = PropagationConfig::new(HazardKind::Fire);
        config.spread = super::super::SpreadConfig::NONE;
        config.decay.grace_period = 0.0;

        let result = propagation_step(&hazards, HazardKind::Fire, &config, 1.0, &());

        assert!(result.decayed_count > 0);
        let decay_delta = result
            .deltas
            .iter()
            .find(|d| d.pos == LocalPos::new(5, 5, 5));
        assert!(decay_delta.is_some());
        let new_intensity = decay_delta.unwrap().intensity.unwrap();
        assert!(new_intensity < 0.5);
    }

    #[test]
    fn propagation_step_extinction() {
        let hazards = make_hazards_with_fire(&[(5, 5, 5, 0.02)]);
        let mut config = PropagationConfig::new(HazardKind::Fire);
        config.spread = super::super::SpreadConfig::NONE;
        config.decay.grace_period = 0.0;
        config.decay.rate = 0.5;
        config.decay.extinction_threshold = 0.05;

        let result = propagation_step(&hazards, HazardKind::Fire, &config, 1.0, &());

        assert!(result.extinguished_count > 0);
        assert!(result.deltas.iter().any(|d| d.intensity.is_none()));
    }

    #[test]
    fn propagation_step_resistance_blocks() {
        let hazards = make_hazards_with_fire(&[(5, 5, 5, 1.0)]);
        let config = PropagationConfig::new(HazardKind::Fire);

        let blocker = |_kind: HazardKind, pos: LocalPos| {
            if pos.x() == 6 && pos.y() == 5 && pos.z() == 5 {
                Resistance::FULL
            } else {
                Resistance::NONE
            }
        };

        let result = propagation_step(&hazards, HazardKind::Fire, &config, 1.0, &blocker);

        let blocked_pos = LocalPos::new(6, 5, 5);
        let spread_to_blocked = result.deltas.iter().any(|d| d.pos == blocked_pos);
        assert!(!spread_to_blocked);
    }

    #[test]
    fn propagation_step_resistance_reduces() {
        let hazards = make_hazards_with_fire(&[(5, 5, 5, 1.0)]);
        let config = PropagationConfig::new(HazardKind::Fire);

        let partial_resistance = |_kind: HazardKind, pos: LocalPos| {
            if pos.x() == 6 {
                Resistance::new(0.5)
            } else {
                Resistance::NONE
            }
        };

        let result = propagation_step(
            &hazards,
            HazardKind::Fire,
            &config,
            1.0,
            &partial_resistance,
        );

        let resisted_delta = result
            .deltas
            .iter()
            .find(|d| d.pos == LocalPos::new(6, 5, 5));
        let unresisted_delta = result
            .deltas
            .iter()
            .find(|d| d.pos == LocalPos::new(4, 5, 5));

        if let (Some(resisted), Some(unresisted)) = (resisted_delta, unresisted_delta) {
            let resisted_intensity = resisted.intensity.unwrap_or(0.0);
            let unresisted_intensity = unresisted.intensity.unwrap_or(0.0);
            assert!(resisted_intensity < unresisted_intensity);
        }
    }

    #[test]
    fn propagation_step_boundary_spreads() {
        let hazards = make_hazards_with_fire(&[(0, 8, 8, 1.0)]);
        let config = PropagationConfig::new(HazardKind::Fire);

        let result = propagation_step(&hazards, HazardKind::Fire, &config, 1.0, &());

        assert!(!result.boundary_spreads.is_empty());
        let has_negative_x = result
            .boundary_spreads
            .iter()
            .any(|(_, dir, _)| dir.0 == -1);
        assert!(has_negative_x);
    }

    #[test]
    fn propagation_step_deterministic_order() {
        let hazards = make_hazards_with_fire(&[(3, 3, 3, 0.8), (10, 10, 10, 0.9), (5, 5, 5, 0.7)]);
        let config = PropagationConfig::new(HazardKind::Fire);

        let result1 = propagation_step(&hazards, HazardKind::Fire, &config, 0.5, &());
        let result2 = propagation_step(&hazards, HazardKind::Fire, &config, 0.5, &());

        assert_eq!(result1.deltas.len(), result2.deltas.len());
        for (d1, d2) in result1.deltas.iter().zip(result2.deltas.iter()) {
            assert_eq!(d1.pos, d2.pos);
            assert_eq!(d1.intensity, d2.intensity);
        }
    }

    #[test]
    fn apply_deltas_updates_hazards() {
        let mut hazards = make_hazards_with_fire(&[(5, 5, 5, 1.0)]);
        let deltas = vec![
            CellDelta::set(LocalPos::new(5, 5, 5), 0.5),
            CellDelta::set(LocalPos::new(6, 5, 5), 0.3),
            CellDelta::deactivate(LocalPos::new(0, 0, 0)),
        ];

        apply_deltas(&mut hazards, HazardKind::Fire, &deltas);

        assert!(
            (hazards
                .get(HazardKind::Fire, LocalPos::new(5, 5, 5))
                .intensity()
                - 0.5)
                .abs()
                < 0.001
        );
        assert!(
            (hazards
                .get(HazardKind::Fire, LocalPos::new(6, 5, 5))
                .intensity()
                - 0.3)
                .abs()
                < 0.001
        );
    }

    #[test]
    fn decay_step_only() {
        let hazards = make_hazards_with_fire(&[(5, 5, 5, 0.5)]);
        let config = super::super::DecayConfig::FAST;

        let result = decay_step(&hazards, HazardKind::Fire, &config, 1.0, 0.05);

        assert!(result.decayed_count > 0);
    }

    #[test]
    fn gravity_affected_spreads_down_faster() {
        let hazards = make_hazards_with_fire(&[(8, 8, 8, 1.0)]);
        let config = PropagationConfig::new(HazardKind::Fire);

        let result = propagation_step(&hazards, HazardKind::Fire, &config, 1.0, &());

        let down_delta = result
            .deltas
            .iter()
            .find(|d| d.pos == LocalPos::new(8, 7, 8));
        let up_delta = result
            .deltas
            .iter()
            .find(|d| d.pos == LocalPos::new(8, 9, 8));

        if let (Some(down), Some(up)) = (down_delta, up_delta) {
            let down_intensity = down.intensity.unwrap_or(0.0);
            let up_intensity = up.intensity.unwrap_or(0.0);
            assert!(
                down_intensity >= up_intensity,
                "down={}, up={}",
                down_intensity,
                up_intensity
            );
        }
    }

    #[test]
    fn cell_delta_constructors() {
        let set = CellDelta::set(LocalPos::new(1, 2, 3), 0.5);
        assert_eq!(set.pos, LocalPos::new(1, 2, 3));
        assert_eq!(set.intensity, Some(0.5));

        let deactivate = CellDelta::deactivate(LocalPos::new(4, 5, 6));
        assert_eq!(deactivate.pos, LocalPos::new(4, 5, 6));
        assert_eq!(deactivate.intensity, None);
    }

    #[test]
    fn propagation_result_merge() {
        let mut a = PropagationResult {
            deltas: vec![CellDelta::set(LocalPos::new(0, 0, 0), 1.0)],
            boundary_spreads: vec![(LocalPos::new(0, 0, 0), (-1, 0, 0), 0.5)],
            decayed_count: 1,
            spread_count: 2,
            extinguished_count: 0,
        };

        let b = PropagationResult {
            deltas: vec![CellDelta::set(LocalPos::new(1, 1, 1), 0.5)],
            boundary_spreads: vec![],
            decayed_count: 1,
            spread_count: 1,
            extinguished_count: 1,
        };

        a.merge(b);

        assert_eq!(a.deltas.len(), 2);
        assert_eq!(a.boundary_spreads.len(), 1);
        assert_eq!(a.decayed_count, 2);
        assert_eq!(a.spread_count, 3);
        assert_eq!(a.extinguished_count, 1);
    }
}
