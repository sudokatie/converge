//! Spatial sampling cells for topology queries.

use serde::{Deserialize, Serialize};

use super::node::NodeId;
use super::segment::SegmentId;

/// State of a cell in the topology.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum CellState {
    /// Outside the topology (solid).
    #[default]
    Solid = 0,
    /// Inside the topology (passable).
    Open = 1,
    /// On the boundary (wall).
    Wall = 2,
    /// Floor surface.
    Floor = 3,
    /// Ceiling surface.
    Ceiling = 4,
}

impl CellState {
    /// Check if the cell is passable.
    #[must_use]
    pub const fn is_passable(&self) -> bool {
        matches!(self, Self::Open | Self::Floor)
    }

    /// Check if the cell is solid.
    #[must_use]
    pub const fn is_solid(&self) -> bool {
        matches!(self, Self::Solid | Self::Wall | Self::Ceiling)
    }

    /// Check if the cell is a surface.
    #[must_use]
    pub const fn is_surface(&self) -> bool {
        matches!(self, Self::Wall | Self::Floor | Self::Ceiling)
    }

    /// Create from raw value.
    #[must_use]
    pub const fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Solid),
            1 => Some(Self::Open),
            2 => Some(Self::Wall),
            3 => Some(Self::Floor),
            4 => Some(Self::Ceiling),
            _ => None,
        }
    }

    /// Get raw value.
    #[must_use]
    pub const fn as_raw(self) -> u8 {
        self as u8
    }
}

/// A spatial cell in the topology.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TopologyCell {
    /// Position in grid coordinates.
    pub grid_pos: [i32; 3],
    /// State of the cell.
    pub state: CellState,
    /// Containing node (if any).
    pub node: Option<NodeId>,
    /// Containing segment (if any).
    pub segment: Option<SegmentId>,
    /// Distance to nearest surface.
    pub surface_distance: f32,
}

impl TopologyCell {
    /// Create a new solid cell.
    #[must_use]
    pub fn solid(grid_pos: [i32; 3]) -> Self {
        Self {
            grid_pos,
            state: CellState::Solid,
            node: None,
            segment: None,
            surface_distance: 0.0,
        }
    }

    /// Create a new open cell.
    #[must_use]
    pub fn open(grid_pos: [i32; 3]) -> Self {
        Self {
            grid_pos,
            state: CellState::Open,
            node: None,
            segment: None,
            surface_distance: 0.0,
        }
    }

    /// Set the state.
    #[must_use]
    pub fn with_state(mut self, state: CellState) -> Self {
        self.state = state;
        self
    }

    /// Set the containing node.
    #[must_use]
    pub fn with_node(mut self, node: NodeId) -> Self {
        self.node = Some(node);
        self
    }

    /// Set the containing segment.
    #[must_use]
    pub fn with_segment(mut self, segment: SegmentId) -> Self {
        self.segment = Some(segment);
        self
    }

    /// Set the surface distance.
    #[must_use]
    pub fn with_surface_distance(mut self, distance: f32) -> Self {
        self.surface_distance = distance;
        self
    }

    /// Get world position from grid position and cell size.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "grid coords always small")]
    pub fn world_position(&self, cell_size: f32) -> [f32; 3] {
        [
            self.grid_pos[0] as f32 * cell_size + cell_size * 0.5,
            self.grid_pos[1] as f32 * cell_size + cell_size * 0.5,
            self.grid_pos[2] as f32 * cell_size + cell_size * 0.5,
        ]
    }
}

/// Query helper for sampling cells.
#[derive(Clone, Debug)]
pub struct CellQuery {
    /// Cell size.
    pub cell_size: f32,
    /// Minimum bounds.
    pub min: [i32; 3],
    /// Maximum bounds.
    pub max: [i32; 3],
}

impl CellQuery {
    /// Create a new cell query.
    #[must_use]
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size,
            min: [0, 0, 0],
            max: [0, 0, 0],
        }
    }

    /// Set bounds from world coordinates.
    #[must_use]
    pub fn with_world_bounds(mut self, min: [f32; 3], max: [f32; 3]) -> Self {
        self.min = self.world_to_grid(min);
        self.max = self.world_to_grid(max);
        self
    }

    /// Set bounds from grid coordinates.
    #[must_use]
    pub fn with_grid_bounds(mut self, min: [i32; 3], max: [i32; 3]) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    /// Convert world position to grid position.
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "grid coords always small")]
    pub fn world_to_grid(&self, pos: [f32; 3]) -> [i32; 3] {
        [
            (pos[0] / self.cell_size).floor() as i32,
            (pos[1] / self.cell_size).floor() as i32,
            (pos[2] / self.cell_size).floor() as i32,
        ]
    }

    /// Convert grid position to world position (center of cell).
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "grid coords always small")]
    pub fn grid_to_world(&self, pos: [i32; 3]) -> [f32; 3] {
        [
            pos[0] as f32 * self.cell_size + self.cell_size * 0.5,
            pos[1] as f32 * self.cell_size + self.cell_size * 0.5,
            pos[2] as f32 * self.cell_size + self.cell_size * 0.5,
        ]
    }

    /// Get the number of cells in the query bounds.
    #[must_use]
    #[expect(clippy::cast_sign_loss, reason = "max(0) ensures non-negative")]
    pub fn cell_count(&self) -> usize {
        let dx = (self.max[0] - self.min[0] + 1).max(0) as usize;
        let dy = (self.max[1] - self.min[1] + 1).max(0) as usize;
        let dz = (self.max[2] - self.min[2] + 1).max(0) as usize;
        dx * dy * dz
    }

    /// Iterate over all grid positions in bounds.
    pub fn iter_positions(&self) -> impl Iterator<Item = [i32; 3]> + '_ {
        let min = self.min;
        let max = self.max;
        (min[0]..=max[0]).flat_map(move |x| {
            (min[1]..=max[1]).flat_map(move |y| (min[2]..=max[2]).map(move |z| [x, y, z]))
        })
    }

    /// Check if a grid position is within bounds.
    #[must_use]
    pub fn contains(&self, pos: [i32; 3]) -> bool {
        pos[0] >= self.min[0]
            && pos[0] <= self.max[0]
            && pos[1] >= self.min[1]
            && pos[1] <= self.max[1]
            && pos[2] >= self.min[2]
            && pos[2] <= self.max[2]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_state_properties() {
        assert!(CellState::Open.is_passable());
        assert!(CellState::Floor.is_passable());
        assert!(!CellState::Wall.is_passable());

        assert!(CellState::Solid.is_solid());
        assert!(!CellState::Open.is_solid());

        assert!(CellState::Wall.is_surface());
        assert!(!CellState::Open.is_surface());
    }

    #[test]
    fn cell_creation() {
        let cell = TopologyCell::solid([1, 2, 3]);
        assert_eq!(cell.state, CellState::Solid);
        assert_eq!(cell.grid_pos, [1, 2, 3]);

        let cell = TopologyCell::open([0, 0, 0])
            .with_node(NodeId::new(5))
            .with_surface_distance(2.5);
        assert_eq!(cell.state, CellState::Open);
        assert_eq!(cell.node, Some(NodeId::new(5)));
        assert!((cell.surface_distance - 2.5).abs() < f32::EPSILON);
    }

    #[test]
    fn cell_world_position() {
        let cell = TopologyCell::solid([1, 2, 3]);
        let world = cell.world_position(4.0);
        assert!((world[0] - 6.0).abs() < f32::EPSILON);
        assert!((world[1] - 10.0).abs() < f32::EPSILON);
        assert!((world[2] - 14.0).abs() < f32::EPSILON);
    }

    #[test]
    fn cell_query_conversion() {
        let query = CellQuery::new(4.0);

        let grid = query.world_to_grid([6.0, 10.0, 14.0]);
        assert_eq!(grid, [1, 2, 3]);

        let world = query.grid_to_world([1, 2, 3]);
        assert!((world[0] - 6.0).abs() < f32::EPSILON);
        assert!((world[1] - 10.0).abs() < f32::EPSILON);
        assert!((world[2] - 14.0).abs() < f32::EPSILON);
    }

    #[test]
    fn cell_query_iteration() {
        let query = CellQuery::new(1.0).with_grid_bounds([0, 0, 0], [1, 1, 1]);

        let positions: Vec<_> = query.iter_positions().collect();
        assert_eq!(positions.len(), 8);
        assert!(positions.contains(&[0, 0, 0]));
        assert!(positions.contains(&[1, 1, 1]));
    }

    #[test]
    fn cell_query_contains() {
        let query = CellQuery::new(1.0).with_grid_bounds([0, 0, 0], [10, 10, 10]);

        assert!(query.contains([5, 5, 5]));
        assert!(query.contains([0, 0, 0]));
        assert!(query.contains([10, 10, 10]));
        assert!(!query.contains([11, 5, 5]));
        assert!(!query.contains([-1, 5, 5]));
    }

    #[test]
    fn cell_state_from_raw() {
        for i in 0..5 {
            let state = CellState::from_raw(i);
            assert!(state.is_some());
            assert_eq!(state.unwrap().as_raw(), i);
        }
        assert!(CellState::from_raw(99).is_none());
    }

    #[test]
    fn serde_roundtrip() {
        let cell = TopologyCell::open([1, 2, 3])
            .with_node(NodeId::new(5))
            .with_state(CellState::Floor);

        let json = serde_json::to_string(&cell).unwrap();
        let recovered: TopologyCell = serde_json::from_str(&json).unwrap();
        assert_eq!(cell, recovered);
    }
}
