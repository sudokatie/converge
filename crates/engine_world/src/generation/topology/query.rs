//! Query helpers for topology planners.

use serde::{Deserialize, Serialize};

use super::node::NodeId;

/// Result of a path query.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathQuery {
    /// Path from start to end (node IDs).
    pub path: Vec<NodeId>,
    /// Total cost of the path.
    pub total_cost: u32,
    /// Whether the path is complete.
    pub complete: bool,
}

impl PathQuery {
    /// Create an empty (incomplete) path result.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            path: Vec::new(),
            total_cost: 0,
            complete: false,
        }
    }

    /// Create a complete path result.
    #[must_use]
    pub fn complete(path: Vec<NodeId>, total_cost: u32) -> Self {
        Self {
            path,
            total_cost,
            complete: true,
        }
    }

    /// Get the length of the path.
    #[must_use]
    pub fn len(&self) -> usize {
        self.path.len()
    }

    /// Check if the path is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.path.is_empty()
    }

    /// Get the start node.
    #[must_use]
    pub fn start(&self) -> Option<NodeId> {
        self.path.first().copied()
    }

    /// Get the end node.
    #[must_use]
    pub fn end(&self) -> Option<NodeId> {
        self.path.last().copied()
    }

    /// Check if the path contains a node.
    #[must_use]
    pub fn contains(&self, node: NodeId) -> bool {
        self.path.contains(&node)
    }
}

/// Result of a general query.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QueryResult<T> {
    /// Query results.
    pub items: Vec<T>,
    /// Whether the query was truncated.
    pub truncated: bool,
    /// Total matching items (may be > `items.len()` if truncated).
    pub total_count: usize,
}

impl<T> QueryResult<T> {
    /// Create an empty result.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            items: Vec::new(),
            truncated: false,
            total_count: 0,
        }
    }

    /// Create a complete result.
    #[must_use]
    pub fn complete(items: Vec<T>) -> Self {
        let total_count = items.len();
        Self {
            items,
            truncated: false,
            total_count,
        }
    }

    /// Create a truncated result.
    #[must_use]
    pub fn truncated(items: Vec<T>, total_count: usize) -> Self {
        Self {
            items,
            truncated: true,
            total_count,
        }
    }

    /// Get the number of returned items.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get the first item.
    #[must_use]
    pub fn first(&self) -> Option<&T> {
        self.items.first()
    }

    /// Iterate over items.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }

    /// Check if the result contains an item.
    #[must_use]
    pub fn contains(&self, item: &T) -> bool
    where
        T: PartialEq,
    {
        self.items.contains(item)
    }

    /// Map items to a new type.
    #[must_use]
    pub fn map<U, F: FnMut(T) -> U>(self, f: F) -> QueryResult<U> {
        QueryResult {
            items: self.items.into_iter().map(f).collect(),
            truncated: self.truncated,
            total_count: self.total_count,
        }
    }
}

impl<T> Default for QueryResult<T> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<T> IntoIterator for QueryResult<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_query_empty() {
        let path = PathQuery::empty();
        assert!(!path.complete);
        assert!(path.is_empty());
        assert!(path.start().is_none());
        assert!(path.end().is_none());
    }

    #[test]
    fn path_query_complete() {
        let nodes = vec![NodeId::new(0), NodeId::new(1), NodeId::new(2)];
        let path = PathQuery::complete(nodes, 100);

        assert!(path.complete);
        assert_eq!(path.len(), 3);
        assert_eq!(path.start(), Some(NodeId::new(0)));
        assert_eq!(path.end(), Some(NodeId::new(2)));
        assert!(path.contains(NodeId::new(1)));
        assert!(!path.contains(NodeId::new(5)));
        assert_eq!(path.total_cost, 100);
    }

    #[test]
    fn query_result_empty() {
        let result: QueryResult<u32> = QueryResult::empty();
        assert!(result.is_empty());
        assert!(!result.truncated);
        assert_eq!(result.total_count, 0);
    }

    #[test]
    fn query_result_complete() {
        let result = QueryResult::complete(vec![1, 2, 3]);
        assert_eq!(result.len(), 3);
        assert!(!result.truncated);
        assert_eq!(result.total_count, 3);
        assert_eq!(result.first(), Some(&1));
    }

    #[test]
    fn query_result_truncated() {
        let result = QueryResult::truncated(vec![1, 2, 3], 10);
        assert_eq!(result.len(), 3);
        assert!(result.truncated);
        assert_eq!(result.total_count, 10);
    }

    #[test]
    fn query_result_map() {
        let result = QueryResult::complete(vec![1, 2, 3]);
        let mapped = result.map(|x| x * 2);
        assert_eq!(mapped.items, vec![2, 4, 6]);
    }

    #[test]
    fn query_result_iteration() {
        let result = QueryResult::complete(vec![1, 2, 3]);
        let sum: u32 = result.iter().sum();
        assert_eq!(sum, 6);

        let result = QueryResult::complete(vec![1, 2, 3]);
        let collected: Vec<_> = result.into_iter().collect();
        assert_eq!(collected, vec![1, 2, 3]);
    }

    #[test]
    fn serde_roundtrip() {
        let path = PathQuery::complete(vec![NodeId::new(0), NodeId::new(1)], 50);
        let json = serde_json::to_string(&path).unwrap();
        let recovered: PathQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(path, recovered);

        let result = QueryResult::truncated(vec![1u32, 2, 3], 10);
        let json = serde_json::to_string(&result).unwrap();
        let recovered: QueryResult<u32> = serde_json::from_str(&json).unwrap();
        assert_eq!(result, recovered);
    }
}
