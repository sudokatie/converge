//! A* pathfinding algorithm for voxel grids

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

/// 3D position in the voxel grid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl GridPos {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// Manhattan distance to another position
    pub fn manhattan_distance(&self, other: &GridPos) -> u32 {
        ((self.x - other.x).abs() + (self.y - other.y).abs() + (self.z - other.z).abs()) as u32
    }

    /// Euclidean distance squared (for comparison without sqrt)
    pub fn distance_squared(&self, other: &GridPos) -> f32 {
        let dx = (self.x - other.x) as f32;
        let dy = (self.y - other.y) as f32;
        let dz = (self.z - other.z) as f32;
        dx * dx + dy * dy + dz * dz
    }

    /// Get neighbors (6 cardinal directions + diagonals if configured)
    pub fn neighbors(&self, allow_diagonal: bool, allow_vertical: bool) -> Vec<GridPos> {
        let mut neighbors = vec![
            GridPos::new(self.x + 1, self.y, self.z),
            GridPos::new(self.x - 1, self.y, self.z),
            GridPos::new(self.x, self.y, self.z + 1),
            GridPos::new(self.x, self.y, self.z - 1),
        ];

        if allow_vertical {
            neighbors.push(GridPos::new(self.x, self.y + 1, self.z));
            neighbors.push(GridPos::new(self.x, self.y - 1, self.z));
        }

        if allow_diagonal {
            // Horizontal diagonals
            neighbors.push(GridPos::new(self.x + 1, self.y, self.z + 1));
            neighbors.push(GridPos::new(self.x + 1, self.y, self.z - 1));
            neighbors.push(GridPos::new(self.x - 1, self.y, self.z + 1));
            neighbors.push(GridPos::new(self.x - 1, self.y, self.z - 1));
        }

        neighbors
    }
}

/// Configuration for A* pathfinding
#[derive(Debug, Clone)]
pub struct AStarConfig {
    /// Maximum number of nodes to explore
    pub max_iterations: u32,
    /// Maximum path length
    pub max_path_length: u32,
    /// Allow diagonal movement
    pub allow_diagonal: bool,
    /// Allow vertical movement (climbing/falling)
    pub allow_vertical: bool,
    /// Maximum fall distance
    pub max_fall_distance: i32,
    /// Maximum jump height
    pub max_jump_height: i32,
    /// Heuristic weight (1.0 = standard A*, >1.0 = greedy)
    pub heuristic_weight: f32,
}

impl Default for AStarConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10000,
            max_path_length: 256,
            allow_diagonal: true,
            allow_vertical: true,
            max_fall_distance: 3,
            max_jump_height: 1,
            heuristic_weight: 1.0,
        }
    }
}

/// Result of a pathfinding operation
#[derive(Debug, Clone)]
pub enum PathResult {
    /// Path found successfully
    Found(Vec<GridPos>),
    /// No path exists
    NotFound,
    /// Search exceeded iteration limit
    IterationLimit,
    /// Path would be too long
    PathTooLong,
    /// Start or goal is invalid
    InvalidEndpoint,
}

impl PathResult {
    pub fn is_found(&self) -> bool {
        matches!(self, PathResult::Found(_))
    }

    pub fn path(&self) -> Option<&Vec<GridPos>> {
        match self {
            PathResult::Found(path) => Some(path),
            _ => None,
        }
    }

    pub fn into_path(self) -> Option<Vec<GridPos>> {
        match self {
            PathResult::Found(path) => Some(path),
            _ => None,
        }
    }
}

/// Node in the A* open set
#[derive(Debug)]
struct OpenNode {
    pos: GridPos,
    f_score: f32,
}

impl PartialEq for OpenNode {
    fn eq(&self, other: &Self) -> bool {
        self.pos == other.pos
    }
}

impl Eq for OpenNode {}

impl Ord for OpenNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap
        other.f_score.partial_cmp(&self.f_score).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for OpenNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Trait for checking if a position is walkable
pub trait Walkable {
    /// Check if position is walkable
    fn is_walkable(&self, pos: &GridPos) -> bool;
    
    /// Get movement cost (1.0 = normal, higher = slower)
    fn movement_cost(&self, _from: &GridPos, _to: &GridPos) -> f32 {
        1.0
    }
}

/// A* pathfinding algorithm
pub struct AStar {
    config: AStarConfig,
}

impl AStar {
    /// Create a new A* pathfinder
    pub fn new(config: AStarConfig) -> Self {
        Self { config }
    }

    /// Find a path from start to goal
    pub fn find_path<W: Walkable>(
        &self,
        start: GridPos,
        goal: GridPos,
        world: &W,
    ) -> PathResult {
        // Validate endpoints
        if !world.is_walkable(&start) {
            return PathResult::InvalidEndpoint;
        }
        if !world.is_walkable(&goal) {
            return PathResult::InvalidEndpoint;
        }

        // Already at goal
        if start == goal {
            return PathResult::Found(vec![start]);
        }

        let mut open_set = BinaryHeap::new();
        let mut closed_set = HashSet::new();
        let mut came_from: HashMap<GridPos, GridPos> = HashMap::new();
        let mut g_score: HashMap<GridPos, f32> = HashMap::new();

        g_score.insert(start, 0.0);
        open_set.push(OpenNode {
            pos: start,
            f_score: self.heuristic(&start, &goal),
        });

        let mut iterations = 0;

        while let Some(current) = open_set.pop() {
            iterations += 1;
            if iterations > self.config.max_iterations {
                return PathResult::IterationLimit;
            }

            if current.pos == goal {
                return PathResult::Found(self.reconstruct_path(&came_from, current.pos));
            }

            if closed_set.contains(&current.pos) {
                continue;
            }
            closed_set.insert(current.pos);

            let current_g = g_score.get(&current.pos).copied().unwrap_or(f32::MAX);

            for neighbor in current.pos.neighbors(self.config.allow_diagonal, self.config.allow_vertical) {
                if closed_set.contains(&neighbor) {
                    continue;
                }

                // Check if neighbor is walkable
                if !world.is_walkable(&neighbor) {
                    continue;
                }

                // Check vertical constraints
                let dy = neighbor.y - current.pos.y;
                if dy > self.config.max_jump_height {
                    continue;
                }
                if dy < -self.config.max_fall_distance {
                    continue;
                }

                let movement_cost = world.movement_cost(&current.pos, &neighbor);
                let tentative_g = current_g + movement_cost;

                // Check path length
                if tentative_g > self.config.max_path_length as f32 {
                    continue;
                }

                let neighbor_g = g_score.get(&neighbor).copied().unwrap_or(f32::MAX);
                if tentative_g < neighbor_g {
                    came_from.insert(neighbor, current.pos);
                    g_score.insert(neighbor, tentative_g);
                    
                    let f_score = tentative_g + self.heuristic(&neighbor, &goal);
                    open_set.push(OpenNode {
                        pos: neighbor,
                        f_score,
                    });
                }
            }
        }

        PathResult::NotFound
    }

    /// Calculate heuristic (estimated cost to goal)
    fn heuristic(&self, from: &GridPos, to: &GridPos) -> f32 {
        let manhattan = from.manhattan_distance(to) as f32;
        manhattan * self.config.heuristic_weight
    }

    /// Reconstruct path from came_from map
    fn reconstruct_path(&self, came_from: &HashMap<GridPos, GridPos>, mut current: GridPos) -> Vec<GridPos> {
        let mut path = vec![current];
        
        while let Some(&prev) = came_from.get(&current) {
            path.push(prev);
            current = prev;
        }
        
        path.reverse();
        path
    }
}

/// Simple world implementation for testing
pub struct SimpleWorld {
    blocked: HashSet<GridPos>,
}

impl SimpleWorld {
    pub fn new() -> Self {
        Self {
            blocked: HashSet::new(),
        }
    }

    pub fn block(&mut self, pos: GridPos) {
        self.blocked.insert(pos);
    }
}

impl Default for SimpleWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl Walkable for SimpleWorld {
    fn is_walkable(&self, pos: &GridPos) -> bool {
        !self.blocked.contains(pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_straight_line() {
        let astar = AStar::new(AStarConfig::default());
        let world = SimpleWorld::new();
        
        let result = astar.find_path(
            GridPos::new(0, 0, 0),
            GridPos::new(5, 0, 0),
            &world,
        );
        
        assert!(result.is_found());
        let path = result.path().unwrap();
        assert_eq!(path.first(), Some(&GridPos::new(0, 0, 0)));
        assert_eq!(path.last(), Some(&GridPos::new(5, 0, 0)));
    }

    #[test]
    fn test_path_around_obstacle() {
        let astar = AStar::new(AStarConfig::default());
        let mut world = SimpleWorld::new();
        
        // Create a wall
        for z in -2..=2 {
            world.block(GridPos::new(2, 0, z));
        }
        
        let result = astar.find_path(
            GridPos::new(0, 0, 0),
            GridPos::new(4, 0, 0),
            &world,
        );
        
        assert!(result.is_found());
    }

    #[test]
    fn test_no_path() {
        // Use config that disables vertical movement to make blocking easier
        let config = AStarConfig {
            allow_vertical: false,
            allow_diagonal: false,
            ..Default::default()
        };
        let astar = AStar::new(config);
        let mut world = SimpleWorld::new();
        
        // Completely surround the start position
        world.block(GridPos::new(1, 0, 0));
        world.block(GridPos::new(-1, 0, 0));
        world.block(GridPos::new(0, 0, 1));
        world.block(GridPos::new(0, 0, -1));
        
        let result = astar.find_path(
            GridPos::new(0, 0, 0),
            GridPos::new(5, 0, 0),
            &world,
        );
        
        assert!(matches!(result, PathResult::NotFound), "Expected NotFound, got {:?}", result);
    }

    #[test]
    fn test_already_at_goal() {
        let astar = AStar::new(AStarConfig::default());
        let world = SimpleWorld::new();
        
        let result = astar.find_path(
            GridPos::new(0, 0, 0),
            GridPos::new(0, 0, 0),
            &world,
        );
        
        assert!(result.is_found());
        let path = result.path().unwrap();
        assert_eq!(path.len(), 1);
    }
}
