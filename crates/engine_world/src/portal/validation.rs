//! Validation errors for portal system.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::id::{PortalId, TraversalId, ZoneId};

/// Validation error types for portals.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PortalValidationError {
    /// Referenced zone does not exist.
    MissingZone {
        portal_id: PortalId,
        zone_id: ZoneId,
    },
    /// Referenced portal does not exist.
    MissingPortal {
        zone_id: ZoneId,
        portal_id: PortalId,
    },
    /// Portal endpoints are in the same zone.
    SameZoneEndpoints {
        portal_id: PortalId,
        zone_id: ZoneId,
    },
    /// Portal ID is duplicated.
    DuplicatePortalId(PortalId),
    /// Zone ID is duplicated.
    DuplicateZoneId(ZoneId),
    /// Portal transform is degenerate (not invertible).
    DegenerateTransform { portal_id: PortalId },
    /// Portal has zero or negative dimensions.
    InvalidDimensions {
        portal_id: PortalId,
        message: String,
    },
    /// Traversal path exceeds maximum depth.
    MaxDepthExceeded {
        traversal_id: TraversalId,
        depth: u32,
        max_depth: u32,
    },
    /// Traversal path forms a cycle.
    CycleDetected {
        traversal_id: TraversalId,
        zone_id: ZoneId,
    },
    /// Portal graph is disconnected.
    DisconnectedGraph { isolated_zones: Vec<ZoneId> },
    /// Zone has no portals.
    OrphanZone(ZoneId),
    /// Generic validation error.
    Other(String),
}

impl fmt::Display for PortalValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingZone { portal_id, zone_id } => {
                write!(f, "portal {portal_id} references missing zone {zone_id}")
            }
            Self::MissingPortal { zone_id, portal_id } => {
                write!(f, "zone {zone_id} references missing portal {portal_id}")
            }
            Self::SameZoneEndpoints { portal_id, zone_id } => {
                write!(f, "portal {portal_id} has both endpoints in zone {zone_id}")
            }
            Self::DuplicatePortalId(id) => {
                write!(f, "duplicate portal ID: {id}")
            }
            Self::DuplicateZoneId(id) => {
                write!(f, "duplicate zone ID: {id}")
            }
            Self::DegenerateTransform { portal_id } => {
                write!(
                    f,
                    "portal {portal_id} has degenerate (non-invertible) transform"
                )
            }
            Self::InvalidDimensions { portal_id, message } => {
                write!(f, "portal {portal_id} has invalid dimensions: {message}")
            }
            Self::MaxDepthExceeded {
                traversal_id,
                depth,
                max_depth,
            } => {
                write!(
                    f,
                    "traversal {traversal_id} exceeded max depth: {depth} > {max_depth}"
                )
            }
            Self::CycleDetected {
                traversal_id,
                zone_id,
            } => {
                write!(
                    f,
                    "traversal {traversal_id} detected cycle at zone {zone_id}"
                )
            }
            Self::DisconnectedGraph { isolated_zones } => {
                let ids: Vec<_> = isolated_zones.iter().map(ToString::to_string).collect();
                write!(f, "graph has disconnected zones: {}", ids.join(", "))
            }
            Self::OrphanZone(zone_id) => {
                write!(f, "zone {zone_id} has no portals")
            }
            Self::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for PortalValidationError {}

/// Collection of validation errors.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PortalValidationErrors {
    errors: Vec<PortalValidationError>,
}

impl PortalValidationErrors {
    /// Create empty error collection.
    #[must_use]
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    /// Add an error.
    pub fn add(&mut self, error: PortalValidationError) {
        self.errors.push(error);
    }

    /// Check if there are any errors.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get error count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.errors.len()
    }

    /// Get all errors.
    #[must_use]
    pub fn errors(&self) -> &[PortalValidationError] {
        &self.errors
    }

    /// Convert to result (Ok if empty, Err otherwise).
    ///
    /// # Errors
    ///
    /// Returns `self` if there are any validation errors.
    pub fn into_result(self) -> Result<(), Self> {
        if self.is_empty() { Ok(()) } else { Err(self) }
    }

    /// Merge another error collection.
    pub fn merge(&mut self, other: Self) {
        self.errors.extend(other.errors);
    }

    /// Filter errors by predicate.
    #[must_use]
    pub fn filter<F>(&self, predicate: F) -> Self
    where
        F: Fn(&PortalValidationError) -> bool,
    {
        Self {
            errors: self
                .errors
                .iter()
                .filter(|e| predicate(e))
                .cloned()
                .collect(),
        }
    }

    /// Check if any error matches the predicate.
    #[must_use]
    pub fn any<F>(&self, predicate: F) -> bool
    where
        F: Fn(&PortalValidationError) -> bool,
    {
        self.errors.iter().any(predicate)
    }

    /// Returns an iterator over the errors.
    pub fn iter(&self) -> std::slice::Iter<'_, PortalValidationError> {
        self.errors.iter()
    }
}

impl fmt::Display for PortalValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} validation error(s):", self.errors.len())?;
        for (i, err) in self.errors.iter().enumerate() {
            writeln!(f, "  {}: {err}", i + 1)?;
        }
        Ok(())
    }
}

impl std::error::Error for PortalValidationErrors {}

impl IntoIterator for PortalValidationErrors {
    type Item = PortalValidationError;
    type IntoIter = std::vec::IntoIter<PortalValidationError>;

    fn into_iter(self) -> Self::IntoIter {
        self.errors.into_iter()
    }
}

impl<'a> IntoIterator for &'a PortalValidationErrors {
    type Item = &'a PortalValidationError;
    type IntoIter = std::slice::Iter<'a, PortalValidationError>;

    fn into_iter(self) -> Self::IntoIter {
        self.errors.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = PortalValidationError::MissingZone {
            portal_id: PortalId::new(1, 2),
            zone_id: ZoneId::new(3, 4),
        };
        let display = format!("{err}");
        assert!(display.contains("portal"));
        assert!(display.contains("zone"));
    }

    #[test]
    fn errors_collection() {
        let mut errors = PortalValidationErrors::new();
        assert!(errors.is_empty());

        errors.add(PortalValidationError::OrphanZone(ZoneId::new(0, 0)));
        errors.add(PortalValidationError::DuplicatePortalId(PortalId::new(
            0, 1,
        )));

        assert_eq!(errors.len(), 2);
        assert!(!errors.is_empty());
    }

    #[test]
    fn errors_result() {
        let empty = PortalValidationErrors::new();
        assert!(empty.into_result().is_ok());

        let mut non_empty = PortalValidationErrors::new();
        non_empty.add(PortalValidationError::OrphanZone(ZoneId::new(0, 0)));
        assert!(non_empty.into_result().is_err());
    }

    #[test]
    fn errors_merge() {
        let mut a = PortalValidationErrors::new();
        a.add(PortalValidationError::OrphanZone(ZoneId::new(0, 0)));

        let mut b = PortalValidationErrors::new();
        b.add(PortalValidationError::DuplicatePortalId(PortalId::new(
            0, 1,
        )));

        a.merge(b);
        assert_eq!(a.len(), 2);
    }

    #[test]
    fn errors_filter() {
        let mut errors = PortalValidationErrors::new();
        errors.add(PortalValidationError::OrphanZone(ZoneId::new(0, 0)));
        errors.add(PortalValidationError::DuplicatePortalId(PortalId::new(
            0, 1,
        )));
        errors.add(PortalValidationError::OrphanZone(ZoneId::new(0, 2)));

        let orphans = errors.filter(|e| matches!(e, PortalValidationError::OrphanZone(_)));
        assert_eq!(orphans.len(), 2);
    }

    #[test]
    fn errors_any() {
        let mut errors = PortalValidationErrors::new();
        errors.add(PortalValidationError::OrphanZone(ZoneId::new(0, 0)));

        assert!(errors.any(|e| matches!(e, PortalValidationError::OrphanZone(_))));
        assert!(!errors.any(|e| matches!(e, PortalValidationError::CycleDetected { .. })));
    }

    #[test]
    fn errors_iterate() {
        let mut errors = PortalValidationErrors::new();
        errors.add(PortalValidationError::OrphanZone(ZoneId::new(0, 0)));
        errors.add(PortalValidationError::OrphanZone(ZoneId::new(0, 1)));

        let count = errors.into_iter().count();
        assert_eq!(count, 2);
    }

    #[test]
    fn degenerate_transform_display() {
        let err = PortalValidationError::DegenerateTransform {
            portal_id: PortalId::new(1, 1),
        };
        assert!(format!("{err}").contains("degenerate"));
    }

    #[test]
    fn max_depth_exceeded_display() {
        let err = PortalValidationError::MaxDepthExceeded {
            traversal_id: TraversalId::from_raw(42),
            depth: 100,
            max_depth: 50,
        };
        let display = format!("{err}");
        assert!(display.contains("100"));
        assert!(display.contains("50"));
    }

    #[test]
    fn disconnected_graph_display() {
        let err = PortalValidationError::DisconnectedGraph {
            isolated_zones: vec![ZoneId::new(0, 1), ZoneId::new(0, 2)],
        };
        assert!(format!("{err}").contains("disconnected"));
    }

    #[test]
    fn serde_roundtrip_error() {
        let err = PortalValidationError::SameZoneEndpoints {
            portal_id: PortalId::new(1, 2),
            zone_id: ZoneId::new(3, 4),
        };
        let json = serde_json::to_string(&err).unwrap();
        let recovered: PortalValidationError = serde_json::from_str(&json).unwrap();
        assert_eq!(err, recovered);
    }

    #[test]
    fn bincode_roundtrip_errors() {
        let mut errors = PortalValidationErrors::new();
        errors.add(PortalValidationError::OrphanZone(ZoneId::new(0, 0)));
        errors.add(PortalValidationError::DuplicatePortalId(PortalId::new(
            1, 1,
        )));
        errors.add(PortalValidationError::InvalidDimensions {
            portal_id: PortalId::new(2, 2),
            message: "negative width".to_string(),
        });

        let serialized = bincode::serialize(&errors).unwrap();
        let recovered: PortalValidationErrors = bincode::deserialize(&serialized).unwrap();
        assert_eq!(errors.len(), recovered.len());
    }
}
