//! Query APIs for generated layouts.

use std::collections::BTreeSet;

use super::id::{PlacementId, TemplateId};
use super::layout::{ConnectorSummary, GeneratedLayout, Placement};
use super::template::TemplateKind;

/// Result of a layout query.
#[derive(Clone, Debug)]
pub struct LayoutQueryResult<T> {
    /// Query results.
    pub items: Vec<T>,
    /// Whether the query completed successfully.
    pub complete: bool,
}

impl<T> LayoutQueryResult<T> {
    /// Create a complete result.
    #[must_use]
    pub fn complete(items: Vec<T>) -> Self {
        Self {
            items,
            complete: true,
        }
    }

    /// Create an empty result.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            items: Vec::new(),
            complete: true,
        }
    }

    /// Check if result is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get item count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Get first item.
    #[must_use]
    pub fn first(&self) -> Option<&T> {
        self.items.first()
    }

    /// Iterate over items.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }
}

impl<T: Clone> LayoutQueryResult<T> {
    /// Check if result contains an item.
    #[must_use]
    pub fn contains(&self, item: &T) -> bool
    where
        T: PartialEq,
    {
        self.items.contains(item)
    }
}

impl<T> IntoIterator for LayoutQueryResult<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

/// Query builder for layout queries.
pub struct LayoutQuery<'a> {
    layout: &'a GeneratedLayout,
}

impl<'a> LayoutQuery<'a> {
    /// Create a new query builder.
    #[must_use]
    pub fn new(layout: &'a GeneratedLayout) -> Self {
        Self { layout }
    }

    /// Get all placements.
    #[must_use]
    pub fn all_placements(&self) -> LayoutQueryResult<&'a Placement> {
        LayoutQueryResult::complete(self.layout.placements().collect())
    }

    /// Get placements by template ID.
    #[must_use]
    pub fn by_template(&self, template_id: TemplateId) -> LayoutQueryResult<&'a Placement> {
        LayoutQueryResult::complete(self.layout.placements_by_template(template_id))
    }

    /// Get placements by kind.
    #[must_use]
    pub fn by_kind(&self, kind: TemplateKind) -> LayoutQueryResult<&'a Placement> {
        LayoutQueryResult::complete(self.layout.placements_by_kind(kind))
    }

    /// Get placements by tag.
    #[must_use]
    pub fn by_tag(&self, tag: &str) -> LayoutQueryResult<&'a Placement> {
        LayoutQueryResult::complete(self.layout.placements_by_tag(tag))
    }

    /// Get placements by multiple tags (AND).
    #[must_use]
    pub fn by_all_tags(&self, tags: &[&str]) -> LayoutQueryResult<&'a Placement> {
        let placements: Vec<_> = self
            .layout
            .placements()
            .filter(|p| tags.iter().all(|t| p.has_tag(t)))
            .collect();
        LayoutQueryResult::complete(placements)
    }

    /// Get placements by any tag (OR).
    #[must_use]
    pub fn by_any_tag(&self, tags: &[&str]) -> LayoutQueryResult<&'a Placement> {
        let placements: Vec<_> = self
            .layout
            .placements()
            .filter(|p| tags.iter().any(|t| p.has_tag(t)))
            .collect();
        LayoutQueryResult::complete(placements)
    }

    /// Get placements at a specific depth.
    #[must_use]
    pub fn at_depth(&self, depth: u32) -> LayoutQueryResult<&'a Placement> {
        let placements: Vec<_> = self
            .layout
            .placements()
            .filter(|p| p.depth == depth)
            .collect();
        LayoutQueryResult::complete(placements)
    }

    /// Get placements within depth range.
    #[must_use]
    pub fn in_depth_range(
        &self,
        min_depth: u32,
        max_depth: u32,
    ) -> LayoutQueryResult<&'a Placement> {
        let placements: Vec<_> = self
            .layout
            .placements()
            .filter(|p| p.depth >= min_depth && p.depth <= max_depth)
            .collect();
        LayoutQueryResult::complete(placements)
    }

    /// Get placement at a cell position.
    #[must_use]
    pub fn at_cell(&self, cell: [i32; 3]) -> Option<&'a Placement> {
        self.layout.placement_at(cell)
    }

    /// Get placements containing a point.
    #[must_use]
    pub fn containing_point(&self, point: [i32; 3]) -> LayoutQueryResult<&'a Placement> {
        let placements: Vec<_> = self
            .layout
            .placements()
            .filter(|p| p.contains(point))
            .collect();
        LayoutQueryResult::complete(placements)
    }

    /// Get placements with metadata key.
    #[must_use]
    pub fn with_metadata(&self, key: &str) -> LayoutQueryResult<&'a Placement> {
        let placements: Vec<_> = self
            .layout
            .placements()
            .filter(|p| p.metadata.contains_key(key))
            .collect();
        LayoutQueryResult::complete(placements)
    }

    /// Get placements with specific metadata value.
    #[must_use]
    pub fn with_metadata_value(&self, key: &str, value: &str) -> LayoutQueryResult<&'a Placement> {
        let placements: Vec<_> = self
            .layout
            .placements()
            .filter(|p| p.get_metadata(key) == Some(value))
            .collect();
        LayoutQueryResult::complete(placements)
    }

    /// Get root placements (no parent).
    #[must_use]
    pub fn roots(&self) -> LayoutQueryResult<&'a Placement> {
        LayoutQueryResult::complete(self.layout.roots())
    }

    /// Get children of a placement.
    #[must_use]
    pub fn children_of(&self, parent_id: PlacementId) -> LayoutQueryResult<&'a Placement> {
        LayoutQueryResult::complete(self.layout.children_of(parent_id))
    }

    /// Get descendants of a placement (recursive).
    #[must_use]
    pub fn descendants_of(&self, parent_id: PlacementId) -> LayoutQueryResult<&'a Placement> {
        let mut result = Vec::new();
        let mut queue = vec![parent_id];
        let mut visited = BTreeSet::new();

        while let Some(id) = queue.pop() {
            if !visited.insert(id) {
                continue;
            }
            for child in self.layout.children_of(id) {
                result.push(child);
                queue.push(child.id);
            }
        }

        LayoutQueryResult::complete(result)
    }

    /// Get all connectors.
    #[must_use]
    pub fn all_connectors(&self) -> LayoutQueryResult<&'a ConnectorSummary> {
        LayoutQueryResult::complete(self.layout.connectors().collect())
    }

    /// Get unconnected connectors.
    #[must_use]
    pub fn unconnected_connectors(&self) -> LayoutQueryResult<&'a ConnectorSummary> {
        LayoutQueryResult::complete(self.layout.unconnected_connectors().collect())
    }

    /// Get connectors by type.
    #[must_use]
    pub fn connectors_by_type(&self, socket_type: &str) -> LayoutQueryResult<&'a ConnectorSummary> {
        let connectors: Vec<_> = self
            .layout
            .connectors()
            .filter(|c| c.socket_type == socket_type)
            .collect();
        LayoutQueryResult::complete(connectors)
    }

    /// Get connectors for a placement.
    #[must_use]
    pub fn connectors_for(
        &self,
        placement_id: PlacementId,
    ) -> LayoutQueryResult<&'a ConnectorSummary> {
        let connectors: Vec<_> = self
            .layout
            .connectors()
            .filter(|c| c.placement_id == placement_id)
            .collect();
        LayoutQueryResult::complete(connectors)
    }

    /// Get occupied cells in a region.
    #[must_use]
    pub fn occupied_cells_in(&self, min: [i32; 3], max: [i32; 3]) -> LayoutQueryResult<[i32; 3]> {
        let cells: Vec<_> = self
            .layout
            .occupied_cells()
            .filter(|c| {
                c[0] >= min[0]
                    && c[0] <= max[0]
                    && c[1] >= min[1]
                    && c[1] <= max[1]
                    && c[2] >= min[2]
                    && c[2] <= max[2]
            })
            .copied()
            .collect();
        LayoutQueryResult::complete(cells)
    }
}

impl GeneratedLayout {
    /// Create a query builder for this layout.
    #[must_use]
    pub fn query(&self) -> LayoutQuery<'_> {
        LayoutQuery::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generation::structure_grammar::template::Bounds;

    fn test_placement(id: u64, pos: [i32; 3], kind: TemplateKind, tags: Vec<String>) -> Placement {
        Placement::new(
            PlacementId::new(id),
            TemplateId::new(1),
            pos,
            Bounds::from_size(5, 5, 5).translate(pos),
        )
        .with_kind(kind)
        .with_tags(tags)
    }

    fn test_layout() -> GeneratedLayout {
        let mut layout = GeneratedLayout::new(42);
        layout.add_placement(test_placement(
            1,
            [0, 0, 0],
            TemplateKind::Room,
            vec!["start".to_string()],
        ));
        layout.add_placement(
            test_placement(2, [10, 0, 0], TemplateKind::Corridor, vec![])
                .with_parent(PlacementId::new(1))
                .with_depth(1),
        );
        layout.add_placement(
            test_placement(3, [20, 0, 0], TemplateKind::Room, vec!["end".to_string()])
                .with_parent(PlacementId::new(2))
                .with_depth(2),
        );
        layout.add_placement(
            test_placement(
                4,
                [0, 10, 0],
                TemplateKind::Junction,
                vec!["start".to_string(), "important".to_string()],
            )
            .with_parent(PlacementId::new(1))
            .with_depth(1),
        );
        layout
    }

    #[test]
    fn query_all_placements() {
        let layout = test_layout();
        let result = layout.query().all_placements();
        assert_eq!(result.len(), 4);
        assert!(result.complete);
    }

    #[test]
    fn query_by_kind() {
        let layout = test_layout();
        let rooms = layout.query().by_kind(TemplateKind::Room);
        assert_eq!(rooms.len(), 2);

        let corridors = layout.query().by_kind(TemplateKind::Corridor);
        assert_eq!(corridors.len(), 1);
    }

    #[test]
    fn query_by_tag() {
        let layout = test_layout();
        let starts = layout.query().by_tag("start");
        assert_eq!(starts.len(), 2);

        let ends = layout.query().by_tag("end");
        assert_eq!(ends.len(), 1);
    }

    #[test]
    fn query_by_all_tags() {
        let layout = test_layout();
        let result = layout.query().by_all_tags(&["start", "important"]);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn query_by_any_tag() {
        let layout = test_layout();
        let result = layout.query().by_any_tag(&["start", "end"]);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn query_at_depth() {
        let layout = test_layout();
        let depth0 = layout.query().at_depth(0);
        assert_eq!(depth0.len(), 1);

        let depth1 = layout.query().at_depth(1);
        assert_eq!(depth1.len(), 2);
    }

    #[test]
    fn query_in_depth_range() {
        let layout = test_layout();
        let result = layout.query().in_depth_range(0, 1);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn query_at_cell() {
        let layout = test_layout();
        let placement = layout.query().at_cell([2, 2, 2]);
        assert!(placement.is_some());
        assert_eq!(placement.unwrap().id, PlacementId::new(1));

        let empty = layout.query().at_cell([100, 100, 100]);
        assert!(empty.is_none());
    }

    #[test]
    fn query_roots_and_children() {
        let layout = test_layout();
        let roots = layout.query().roots();
        assert_eq!(roots.len(), 1);

        let children = layout.query().children_of(PlacementId::new(1));
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn query_descendants() {
        let layout = test_layout();
        let descendants = layout.query().descendants_of(PlacementId::new(1));
        assert_eq!(descendants.len(), 3);
    }

    #[test]
    fn query_occupied_cells_in() {
        let layout = test_layout();
        let cells = layout.query().occupied_cells_in([0, 0, 0], [10, 10, 10]);
        assert!(!cells.is_empty());
    }

    #[test]
    fn query_result_methods() {
        let result = LayoutQueryResult::complete(vec![1, 2, 3]);
        assert!(!result.is_empty());
        assert_eq!(result.len(), 3);
        assert_eq!(result.first(), Some(&1));
        assert!(result.contains(&2));

        let empty: LayoutQueryResult<i32> = LayoutQueryResult::empty();
        assert!(empty.is_empty());
    }

    #[test]
    fn query_result_iterate() {
        let result = LayoutQueryResult::complete(vec![1, 2, 3]);
        let sum: i32 = result.iter().sum();
        assert_eq!(sum, 6);

        let result = LayoutQueryResult::complete(vec![1, 2, 3]);
        let vec: Vec<_> = result.into_iter().collect();
        assert_eq!(vec, vec![1, 2, 3]);
    }
}
