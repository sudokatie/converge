//! Navigation mesh for efficient pathfinding in voxel worlds

use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Ordering;

/// 3D position with floating point precision
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn distance(&self, other: &Vec3) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    pub fn distance_squared(&self, other: &Vec3) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        dx * dx + dy * dy + dz * dz
    }

    pub fn lerp(&self, other: &Vec3, t: f32) -> Vec3 {
        Vec3 {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
            z: self.z + (other.z - self.z) * t,
        }
    }
}

/// Navigation node in the mesh
#[derive(Debug, Clone)]
pub struct NavNode {
    /// Unique node ID
    pub id: u32,
    /// Center position of this node
    pub position: Vec3,
    /// Radius of this node (for area representation)
    pub radius: f32,
    /// Connected edges (indices into NavMesh.edges)
    pub edges: Vec<u32>,
    /// Whether this node is enabled
    pub enabled: bool,
    /// Custom flags for gameplay
    pub flags: u32,
}

impl NavNode {
    pub fn new(id: u32, position: Vec3, radius: f32) -> Self {
        Self {
            id,
            position,
            radius,
            edges: Vec::new(),
            enabled: true,
            flags: 0,
        }
    }

    pub fn contains_point(&self, point: &Vec3) -> bool {
        self.position.distance_squared(point) <= self.radius * self.radius
    }
}

/// Edge connecting two navigation nodes
#[derive(Debug, Clone)]
pub struct NavEdge {
    /// Unique edge ID
    pub id: u32,
    /// Source node ID
    pub from: u32,
    /// Destination node ID
    pub to: u32,
    /// Cost to traverse this edge
    pub cost: f32,
    /// Whether this edge is bidirectional
    pub bidirectional: bool,
    /// Whether this edge is enabled
    pub enabled: bool,
    /// Custom flags (e.g., requires jump, swim, etc.)
    pub flags: u32,
}

impl NavEdge {
    pub fn new(id: u32, from: u32, to: u32, cost: f32) -> Self {
        Self {
            id,
            from,
            to,
            cost,
            bidirectional: true,
            enabled: true,
            flags: 0,
        }
    }

    pub fn one_way(id: u32, from: u32, to: u32, cost: f32) -> Self {
        let mut edge = Self::new(id, from, to, cost);
        edge.bidirectional = false;
        edge
    }
}

/// Configuration for NavMesh
#[derive(Debug, Clone)]
pub struct NavMeshConfig {
    /// Maximum path search iterations
    pub max_iterations: u32,
    /// Heuristic weight
    pub heuristic_weight: f32,
    /// Maximum distance to search for nearest node
    pub max_node_search_distance: f32,
}

impl Default for NavMeshConfig {
    fn default() -> Self {
        Self {
            max_iterations: 5000,
            heuristic_weight: 1.0,
            max_node_search_distance: 50.0,
        }
    }
}

/// Navigation mesh for pathfinding
#[derive(Debug)]
pub struct NavMesh {
    /// All nodes in the mesh
    nodes: HashMap<u32, NavNode>,
    /// All edges in the mesh
    edges: HashMap<u32, NavEdge>,
    /// Configuration
    config: NavMeshConfig,
    /// Next node ID
    next_node_id: u32,
    /// Next edge ID
    next_edge_id: u32,
}

/// Path node for A* search
#[derive(Debug)]
struct PathNode {
    node_id: u32,
    f_score: f32,
}

impl PartialEq for PathNode {
    fn eq(&self, other: &Self) -> bool {
        self.node_id == other.node_id
    }
}

impl Eq for PathNode {}

impl Ord for PathNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f_score.partial_cmp(&self.f_score).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for PathNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl NavMesh {
    /// Create a new empty navigation mesh
    pub fn new(config: NavMeshConfig) -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            config,
            next_node_id: 0,
            next_edge_id: 0,
        }
    }

    /// Add a node to the mesh
    pub fn add_node(&mut self, position: Vec3, radius: f32) -> u32 {
        let id = self.next_node_id;
        self.next_node_id += 1;
        self.nodes.insert(id, NavNode::new(id, position, radius));
        id
    }

    /// Add an edge between two nodes
    pub fn add_edge(&mut self, from: u32, to: u32, cost: Option<f32>) -> Option<u32> {
        // Validate nodes exist
        let from_node = self.nodes.get(&from)?;
        let to_node = self.nodes.get(&to)?;

        // Calculate cost if not provided
        let cost = cost.unwrap_or_else(|| from_node.position.distance(&to_node.position));

        let id = self.next_edge_id;
        self.next_edge_id += 1;

        let edge = NavEdge::new(id, from, to, cost);
        let bidirectional = edge.bidirectional;

        self.edges.insert(id, edge);

        // Update node connections
        if let Some(node) = self.nodes.get_mut(&from) {
            node.edges.push(id);
        }
        if bidirectional {
            if let Some(node) = self.nodes.get_mut(&to) {
                node.edges.push(id);
            }
        }

        Some(id)
    }

    /// Add a one-way edge
    pub fn add_one_way_edge(&mut self, from: u32, to: u32, cost: Option<f32>) -> Option<u32> {
        let from_node = self.nodes.get(&from)?;
        let to_node = self.nodes.get(&to)?;

        let cost = cost.unwrap_or_else(|| from_node.position.distance(&to_node.position));

        let id = self.next_edge_id;
        self.next_edge_id += 1;

        let edge = NavEdge::one_way(id, from, to, cost);
        self.edges.insert(id, edge);

        if let Some(node) = self.nodes.get_mut(&from) {
            node.edges.push(id);
        }

        Some(id)
    }

    /// Remove a node and all its edges
    pub fn remove_node(&mut self, node_id: u32) -> Option<NavNode> {
        let node = self.nodes.remove(&node_id)?;
        
        // Remove all edges connected to this node
        let edge_ids: Vec<u32> = self.edges
            .iter()
            .filter(|(_, e)| e.from == node_id || e.to == node_id)
            .map(|(id, _)| *id)
            .collect();

        for edge_id in edge_ids {
            self.edges.remove(&edge_id);
        }

        // Remove edge references from other nodes
        for other_node in self.nodes.values_mut() {
            other_node.edges.retain(|&e| {
                self.edges.get(&e).map(|edge| edge.from != node_id && edge.to != node_id).unwrap_or(false)
            });
        }

        Some(node)
    }

    /// Find the nearest node to a position
    pub fn find_nearest_node(&self, position: &Vec3) -> Option<u32> {
        let mut nearest: Option<(u32, f32)> = None;

        for (id, node) in &self.nodes {
            if !node.enabled {
                continue;
            }

            let dist = node.position.distance_squared(position);
            if dist > self.config.max_node_search_distance * self.config.max_node_search_distance {
                continue;
            }

            match nearest {
                None => nearest = Some((*id, dist)),
                Some((_, best_dist)) if dist < best_dist => nearest = Some((*id, dist)),
                _ => {}
            }
        }

        nearest.map(|(id, _)| id)
    }

    /// Find a path between two positions
    pub fn find_path(&self, start: Vec3, goal: Vec3) -> Option<Vec<Vec3>> {
        let start_node = self.find_nearest_node(&start)?;
        let goal_node = self.find_nearest_node(&goal)?;

        if start_node == goal_node {
            return Some(vec![start, goal]);
        }

        let node_path = self.find_node_path(start_node, goal_node)?;
        
        // Convert node path to position path
        let mut path = vec![start];
        for node_id in &node_path[1..node_path.len() - 1] {
            if let Some(node) = self.nodes.get(node_id) {
                path.push(node.position);
            }
        }
        path.push(goal);

        Some(path)
    }

    /// Find a path between two nodes using A*
    fn find_node_path(&self, start: u32, goal: u32) -> Option<Vec<u32>> {
        let goal_node = self.nodes.get(&goal)?;
        
        let mut open_set = BinaryHeap::new();
        let mut closed_set = HashSet::new();
        let mut came_from: HashMap<u32, u32> = HashMap::new();
        let mut g_score: HashMap<u32, f32> = HashMap::new();

        g_score.insert(start, 0.0);
        open_set.push(PathNode {
            node_id: start,
            f_score: self.heuristic(start, goal)?,
        });

        let mut iterations = 0;

        while let Some(current) = open_set.pop() {
            iterations += 1;
            if iterations > self.config.max_iterations {
                return None;
            }

            if current.node_id == goal {
                return Some(self.reconstruct_path(&came_from, goal));
            }

            if closed_set.contains(&current.node_id) {
                continue;
            }
            closed_set.insert(current.node_id);

            let current_node = self.nodes.get(&current.node_id)?;
            let current_g = g_score.get(&current.node_id).copied().unwrap_or(f32::MAX);

            for &edge_id in &current_node.edges {
                let edge = self.edges.get(&edge_id)?;
                if !edge.enabled {
                    continue;
                }

                let neighbor_id = if edge.from == current.node_id {
                    edge.to
                } else if edge.bidirectional {
                    edge.from
                } else {
                    continue;
                };

                if closed_set.contains(&neighbor_id) {
                    continue;
                }

                let neighbor_node = self.nodes.get(&neighbor_id)?;
                if !neighbor_node.enabled {
                    continue;
                }

                let tentative_g = current_g + edge.cost;
                let neighbor_g = g_score.get(&neighbor_id).copied().unwrap_or(f32::MAX);

                if tentative_g < neighbor_g {
                    came_from.insert(neighbor_id, current.node_id);
                    g_score.insert(neighbor_id, tentative_g);

                    let h = neighbor_node.position.distance(&goal_node.position) * self.config.heuristic_weight;
                    open_set.push(PathNode {
                        node_id: neighbor_id,
                        f_score: tentative_g + h,
                    });
                }
            }
        }

        None
    }

    /// Calculate heuristic between two nodes
    fn heuristic(&self, from: u32, to: u32) -> Option<f32> {
        let from_node = self.nodes.get(&from)?;
        let to_node = self.nodes.get(&to)?;
        Some(from_node.position.distance(&to_node.position) * self.config.heuristic_weight)
    }

    /// Reconstruct path from came_from map
    fn reconstruct_path(&self, came_from: &HashMap<u32, u32>, goal: u32) -> Vec<u32> {
        let mut path = vec![goal];
        let mut current = goal;

        while let Some(&prev) = came_from.get(&current) {
            path.push(prev);
            current = prev;
        }

        path.reverse();
        path
    }

    /// Get a node by ID
    pub fn get_node(&self, id: u32) -> Option<&NavNode> {
        self.nodes.get(&id)
    }

    /// Get a mutable node by ID
    pub fn get_node_mut(&mut self, id: u32) -> Option<&mut NavNode> {
        self.nodes.get_mut(&id)
    }

    /// Get all nodes
    pub fn nodes(&self) -> impl Iterator<Item = &NavNode> {
        self.nodes.values()
    }

    /// Get all edges
    pub fn edges(&self) -> impl Iterator<Item = &NavEdge> {
        self.edges.values()
    }

    /// Number of nodes
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of edges
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Enable/disable a node
    pub fn set_node_enabled(&mut self, id: u32, enabled: bool) {
        if let Some(node) = self.nodes.get_mut(&id) {
            node.enabled = enabled;
        }
    }

    /// Enable/disable an edge
    pub fn set_edge_enabled(&mut self, id: u32, enabled: bool) {
        if let Some(edge) = self.edges.get_mut(&id) {
            edge.enabled = enabled;
        }
    }
}

impl Default for NavMesh {
    fn default() -> Self {
        Self::new(NavMeshConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_path() {
        let mut mesh = NavMesh::default();
        
        let n0 = mesh.add_node(Vec3::new(0.0, 0.0, 0.0), 1.0);
        let n1 = mesh.add_node(Vec3::new(10.0, 0.0, 0.0), 1.0);
        let n2 = mesh.add_node(Vec3::new(20.0, 0.0, 0.0), 1.0);
        
        mesh.add_edge(n0, n1, None);
        mesh.add_edge(n1, n2, None);
        
        let path = mesh.find_path(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(20.0, 0.0, 0.0),
        );
        
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.len() >= 2);
    }

    #[test]
    fn test_no_path() {
        let mut mesh = NavMesh::default();
        
        let _n0 = mesh.add_node(Vec3::new(0.0, 0.0, 0.0), 1.0);
        let _n1 = mesh.add_node(Vec3::new(100.0, 0.0, 0.0), 1.0);
        // No edge connecting them
        
        let path = mesh.find_path(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(100.0, 0.0, 0.0),
        );
        
        assert!(path.is_none());
    }

    #[test]
    fn test_disabled_edge() {
        let mut mesh = NavMesh::default();
        
        let n0 = mesh.add_node(Vec3::new(0.0, 0.0, 0.0), 1.0);
        let n1 = mesh.add_node(Vec3::new(10.0, 0.0, 0.0), 1.0);
        
        let edge_id = mesh.add_edge(n0, n1, None).unwrap();
        mesh.set_edge_enabled(edge_id, false);
        
        let path = mesh.find_path(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(10.0, 0.0, 0.0),
        );
        
        assert!(path.is_none());
    }
}
