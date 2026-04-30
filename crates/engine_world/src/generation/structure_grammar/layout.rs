//! Generated layout and placement structures.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::id::{PlacementId, TemplateId};
use super::template::{Bounds, TemplateKind};

/// A placed template instance in the layout.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Placement {
    /// Unique placement identifier.
    pub id: PlacementId,
    /// Template used.
    pub template_id: TemplateId,
    /// Template kind (cached from template).
    pub kind: TemplateKind,
    /// Position offset from origin.
    pub position: [i32; 3],
    /// World-space bounds.
    pub bounds: Bounds,
    /// Depth in generation tree.
    pub depth: u32,
    /// Parent placement (if any).
    pub parent: Option<PlacementId>,
    /// Socket used for connection (if connected).
    pub connected_socket: Option<String>,
    /// Tags from template.
    pub tags: Vec<String>,
    /// Custom metadata.
    pub metadata: BTreeMap<String, String>,
}

impl Placement {
    /// Create a new placement.
    #[must_use]
    pub fn new(
        id: PlacementId,
        template_id: TemplateId,
        position: [i32; 3],
        bounds: Bounds,
    ) -> Self {
        Self {
            id,
            template_id,
            kind: TemplateKind::Room,
            position,
            bounds,
            depth: 0,
            parent: None,
            connected_socket: None,
            tags: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }

    /// Set kind.
    #[must_use]
    pub fn with_kind(mut self, kind: TemplateKind) -> Self {
        self.kind = kind;
        self
    }

    /// Set depth.
    #[must_use]
    pub fn with_depth(mut self, depth: u32) -> Self {
        self.depth = depth;
        self
    }

    /// Set parent.
    #[must_use]
    pub fn with_parent(mut self, parent: PlacementId) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Set connected socket.
    #[must_use]
    pub fn with_socket(mut self, socket: impl Into<String>) -> Self {
        self.connected_socket = Some(socket.into());
        self
    }

    /// Add tags.
    #[must_use]
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Add metadata.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Check if placement has a tag.
    #[must_use]
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Get metadata value.
    #[must_use]
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(String::as_str)
    }

    /// Check if a point is within this placement's bounds.
    #[must_use]
    pub fn contains(&self, point: [i32; 3]) -> bool {
        self.bounds.contains(point)
    }

    /// Check if this placement overlaps another.
    #[must_use]
    pub fn overlaps(&self, other: &Placement) -> bool {
        self.bounds.overlaps(&other.bounds)
    }
}

/// Summary of a connector in the layout.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConnectorSummary {
    /// Placement ID.
    pub placement_id: PlacementId,
    /// Socket name.
    pub socket_name: String,
    /// Socket type.
    pub socket_type: String,
    /// World position.
    pub world_position: [i32; 3],
    /// Whether connected.
    pub connected: bool,
    /// Connected to placement (if connected).
    pub connected_to: Option<PlacementId>,
}

/// A generated structure layout.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GeneratedLayout {
    /// Seed used for generation.
    pub seed: u64,
    /// All placements indexed by ID.
    placements: BTreeMap<PlacementId, Placement>,
    /// Placements by template.
    by_template: BTreeMap<TemplateId, Vec<PlacementId>>,
    /// Placements by kind.
    by_kind: BTreeMap<TemplateKind, Vec<PlacementId>>,
    /// Placements by tag.
    by_tag: BTreeMap<String, Vec<PlacementId>>,
    /// Occupied cells.
    occupied_cells: BTreeSet<[i32; 3]>,
    /// Connectors.
    connectors: Vec<ConnectorSummary>,
    /// Maximum depth reached.
    pub max_depth: u32,
    /// Total generation steps.
    pub total_steps: u32,
}

impl GeneratedLayout {
    /// Create a new empty layout.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            ..Default::default()
        }
    }

    /// Add a placement to the layout.
    pub fn add_placement(&mut self, placement: Placement) {
        let id = placement.id;
        let template_id = placement.template_id;
        let kind = placement.kind;

        self.by_template.entry(template_id).or_default().push(id);
        self.by_kind.entry(kind).or_default().push(id);

        for tag in &placement.tags {
            self.by_tag.entry(tag.clone()).or_default().push(id);
        }

        for cell in placement.bounds.cells() {
            self.occupied_cells.insert(cell);
        }

        if placement.depth > self.max_depth {
            self.max_depth = placement.depth;
        }

        self.placements.insert(id, placement);
    }

    /// Add a connector.
    pub fn add_connector(&mut self, connector: ConnectorSummary) {
        self.connectors.push(connector);
    }

    /// Get a placement by ID.
    #[must_use]
    pub fn placement(&self, id: PlacementId) -> Option<&Placement> {
        self.placements.get(&id)
    }

    /// Get all placements.
    pub fn placements(&self) -> impl Iterator<Item = &Placement> {
        self.placements.values()
    }

    /// Get placement count.
    #[must_use]
    pub fn placement_count(&self) -> usize {
        self.placements.len()
    }

    /// Check if layout is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.placements.is_empty()
    }

    /// Get placements by template.
    #[must_use]
    pub fn placements_by_template(&self, template_id: TemplateId) -> Vec<&Placement> {
        self.by_template
            .get(&template_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.placements.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get placements by kind.
    #[must_use]
    pub fn placements_by_kind(&self, kind: TemplateKind) -> Vec<&Placement> {
        self.by_kind
            .get(&kind)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.placements.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get placements by tag.
    #[must_use]
    pub fn placements_by_tag(&self, tag: &str) -> Vec<&Placement> {
        self.by_tag
            .get(tag)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.placements.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Check if a cell is occupied.
    #[must_use]
    pub fn is_occupied(&self, cell: [i32; 3]) -> bool {
        self.occupied_cells.contains(&cell)
    }

    /// Get all occupied cells.
    pub fn occupied_cells(&self) -> impl Iterator<Item = &[i32; 3]> {
        self.occupied_cells.iter()
    }

    /// Get occupied cell count.
    #[must_use]
    pub fn occupied_cell_count(&self) -> usize {
        self.occupied_cells.len()
    }

    /// Check if bounds would overlap any existing placement.
    #[must_use]
    pub fn would_overlap(&self, bounds: &Bounds) -> bool {
        for cell in bounds.cells() {
            if self.occupied_cells.contains(&cell) {
                return true;
            }
        }
        false
    }

    /// Get placement at a specific cell.
    #[must_use]
    pub fn placement_at(&self, cell: [i32; 3]) -> Option<&Placement> {
        self.placements.values().find(|p| p.contains(cell))
    }

    /// Get overall bounds of the layout.
    #[must_use]
    pub fn overall_bounds(&self) -> Option<Bounds> {
        if self.placements.is_empty() {
            return None;
        }

        let mut min = [i32::MAX, i32::MAX, i32::MAX];
        let mut max = [i32::MIN, i32::MIN, i32::MIN];

        for placement in self.placements.values() {
            for i in 0..3 {
                min[i] = min[i].min(placement.bounds.min[i]);
                max[i] = max[i].max(placement.bounds.max[i]);
            }
        }

        Some(Bounds::new(min, max))
    }

    /// Get all connectors.
    pub fn connectors(&self) -> impl Iterator<Item = &ConnectorSummary> {
        self.connectors.iter()
    }

    /// Get unconnected connectors.
    pub fn unconnected_connectors(&self) -> impl Iterator<Item = &ConnectorSummary> {
        self.connectors.iter().filter(|c| !c.connected)
    }

    /// Get connector count.
    #[must_use]
    pub fn connector_count(&self) -> usize {
        self.connectors.len()
    }

    /// Get children of a placement.
    #[must_use]
    pub fn children_of(&self, parent_id: PlacementId) -> Vec<&Placement> {
        self.placements
            .values()
            .filter(|p| p.parent == Some(parent_id))
            .collect()
    }

    /// Get root placements (no parent).
    #[must_use]
    pub fn roots(&self) -> Vec<&Placement> {
        self.placements
            .values()
            .filter(|p| p.parent.is_none())
            .collect()
    }
}

/// Summary statistics for a layout.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LayoutSummary {
    /// Total placements.
    pub placement_count: usize,
    /// Placements by kind.
    pub by_kind: BTreeMap<String, usize>,
    /// Occupied cell count.
    pub cell_count: usize,
    /// Overall bounds.
    pub bounds: Option<Bounds>,
    /// Max generation depth.
    pub max_depth: u32,
    /// Total connectors.
    pub connector_count: usize,
    /// Unconnected connector count.
    pub unconnected_count: usize,
}

impl GeneratedLayout {
    /// Get a summary of the layout.
    #[must_use]
    pub fn summary(&self) -> LayoutSummary {
        let mut by_kind = BTreeMap::new();
        for (kind, ids) in &self.by_kind {
            by_kind.insert(kind.name().to_string(), ids.len());
        }

        LayoutSummary {
            placement_count: self.placements.len(),
            by_kind,
            cell_count: self.occupied_cells.len(),
            bounds: self.overall_bounds(),
            max_depth: self.max_depth,
            connector_count: self.connectors.len(),
            unconnected_count: self.unconnected_connectors().count(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_placement(id: u64, pos: [i32; 3], size: i32) -> Placement {
        Placement::new(
            PlacementId::new(id),
            TemplateId::new(1),
            pos,
            Bounds::new(
                pos,
                [pos[0] + size - 1, pos[1] + size - 1, pos[2] + size - 1],
            ),
        )
    }

    #[test]
    fn placement_creation() {
        let placement = test_placement(1, [0, 0, 0], 5)
            .with_kind(TemplateKind::Corridor)
            .with_depth(2)
            .with_tags(vec!["interior".to_string()])
            .with_metadata("key", "value");

        assert_eq!(placement.kind, TemplateKind::Corridor);
        assert_eq!(placement.depth, 2);
        assert!(placement.has_tag("interior"));
        assert_eq!(placement.get_metadata("key"), Some("value"));
    }

    #[test]
    fn placement_contains() {
        let placement = test_placement(1, [0, 0, 0], 5);
        assert!(placement.contains([2, 2, 2]));
        assert!(!placement.contains([10, 0, 0]));
    }

    #[test]
    fn placement_overlap() {
        let p1 = test_placement(1, [0, 0, 0], 5);
        let p2 = test_placement(2, [3, 3, 3], 5);
        let p3 = test_placement(3, [10, 10, 10], 5);

        assert!(p1.overlaps(&p2));
        assert!(!p1.overlaps(&p3));
    }

    #[test]
    fn layout_basic_operations() {
        let mut layout = GeneratedLayout::new(42);
        assert!(layout.is_empty());

        layout.add_placement(
            test_placement(1, [0, 0, 0], 5)
                .with_kind(TemplateKind::Room)
                .with_tags(vec!["start".to_string()]),
        );

        assert_eq!(layout.placement_count(), 1);
        assert!(!layout.is_empty());
        assert!(layout.placement(PlacementId::new(1)).is_some());
    }

    #[test]
    fn layout_occupied_cells() {
        let mut layout = GeneratedLayout::new(42);
        layout.add_placement(test_placement(1, [0, 0, 0], 3));

        assert!(layout.is_occupied([0, 0, 0]));
        assert!(layout.is_occupied([2, 2, 2]));
        assert!(!layout.is_occupied([5, 5, 5]));
        assert_eq!(layout.occupied_cell_count(), 27);
    }

    #[test]
    fn layout_would_overlap() {
        let mut layout = GeneratedLayout::new(42);
        layout.add_placement(test_placement(1, [0, 0, 0], 5));

        let overlapping = Bounds::new([3, 3, 3], [8, 8, 8]);
        let non_overlapping = Bounds::new([10, 10, 10], [15, 15, 15]);

        assert!(layout.would_overlap(&overlapping));
        assert!(!layout.would_overlap(&non_overlapping));
    }

    #[test]
    fn layout_queries() {
        let mut layout = GeneratedLayout::new(42);
        layout.add_placement(
            test_placement(1, [0, 0, 0], 5)
                .with_kind(TemplateKind::Room)
                .with_tags(vec!["start".to_string()]),
        );
        layout.add_placement(test_placement(2, [10, 0, 0], 5).with_kind(TemplateKind::Corridor));
        layout.add_placement(
            test_placement(3, [20, 0, 0], 5)
                .with_kind(TemplateKind::Room)
                .with_tags(vec!["end".to_string()]),
        );

        assert_eq!(layout.placements_by_kind(TemplateKind::Room).len(), 2);
        assert_eq!(layout.placements_by_kind(TemplateKind::Corridor).len(), 1);
        assert_eq!(layout.placements_by_tag("start").len(), 1);
    }

    #[test]
    fn layout_overall_bounds() {
        let mut layout = GeneratedLayout::new(42);
        layout.add_placement(test_placement(1, [0, 0, 0], 5));
        layout.add_placement(test_placement(2, [10, 10, 10], 5));

        let bounds = layout.overall_bounds().unwrap();
        assert_eq!(bounds.min, [0, 0, 0]);
        assert_eq!(bounds.max, [14, 14, 14]);
    }

    #[test]
    fn layout_parent_child() {
        let mut layout = GeneratedLayout::new(42);
        layout.add_placement(test_placement(1, [0, 0, 0], 5));
        layout.add_placement(test_placement(2, [10, 0, 0], 5).with_parent(PlacementId::new(1)));
        layout.add_placement(test_placement(3, [20, 0, 0], 5).with_parent(PlacementId::new(1)));

        assert_eq!(layout.roots().len(), 1);
        assert_eq!(layout.children_of(PlacementId::new(1)).len(), 2);
    }

    #[test]
    fn layout_summary() {
        let mut layout = GeneratedLayout::new(42);
        layout.add_placement(test_placement(1, [0, 0, 0], 5).with_kind(TemplateKind::Room));
        layout.max_depth = 3;

        let summary = layout.summary();
        assert_eq!(summary.placement_count, 1);
        assert_eq!(summary.max_depth, 3);
        assert!(summary.bounds.is_some());
    }

    #[test]
    fn serde_roundtrip() {
        let mut layout = GeneratedLayout::new(42);
        layout.add_placement(test_placement(1, [0, 0, 0], 5).with_kind(TemplateKind::Room));

        let json = serde_json::to_string(&layout).unwrap();
        let recovered: GeneratedLayout = serde_json::from_str(&json).unwrap();

        assert_eq!(layout.seed, recovered.seed);
        assert_eq!(layout.placement_count(), recovered.placement_count());
    }
}
