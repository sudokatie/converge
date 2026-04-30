//! Identifier types for structure grammar.

use serde::{Deserialize, Serialize};

/// Unique identifier for a structure template.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct TemplateId(pub u64);

impl TemplateId {
    /// Create a new template ID.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw value.
    #[must_use]
    pub const fn value(&self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for TemplateId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "T{}", self.0)
    }
}

impl From<u64> for TemplateId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

/// Unique identifier for a grammar rule.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct RuleId(pub u64);

impl RuleId {
    /// Create a new rule ID.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw value.
    #[must_use]
    pub const fn value(&self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for RuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "R{}", self.0)
    }
}

impl From<u64> for RuleId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

/// Symbol identifier for grammar expansion.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SymbolId(pub String);

impl SymbolId {
    /// Create a new symbol ID.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Get the name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.0
    }

    /// Check if this is a terminal symbol (starts with lowercase).
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        self.0
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase())
    }

    /// Check if this is a non-terminal symbol (starts with uppercase).
    #[must_use]
    pub fn is_non_terminal(&self) -> bool {
        self.0
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_uppercase())
    }
}

impl std::fmt::Display for SymbolId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for SymbolId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for SymbolId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Unique identifier for a placed template instance.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct PlacementId(pub u64);

impl PlacementId {
    /// Create a new placement ID.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw value.
    #[must_use]
    pub const fn value(&self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for PlacementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "P{}", self.0)
    }
}

impl From<u64> for PlacementId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_id_basics() {
        let id = TemplateId::new(42);
        assert_eq!(id.value(), 42);
        assert_eq!(format!("{id}"), "T42");
    }

    #[test]
    fn rule_id_basics() {
        let id = RuleId::new(7);
        assert_eq!(id.value(), 7);
        assert_eq!(format!("{id}"), "R7");
    }

    #[test]
    fn symbol_id_terminal_check() {
        let terminal = SymbolId::new("wall");
        assert!(terminal.is_terminal());
        assert!(!terminal.is_non_terminal());

        let non_terminal = SymbolId::new("Room");
        assert!(!non_terminal.is_terminal());
        assert!(non_terminal.is_non_terminal());
    }

    #[test]
    fn placement_id_basics() {
        let id = PlacementId::new(100);
        assert_eq!(id.value(), 100);
        assert_eq!(format!("{id}"), "P100");
    }

    #[test]
    fn id_ordering() {
        let ids = vec![TemplateId::new(3), TemplateId::new(1), TemplateId::new(2)];
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(
            sorted,
            vec![TemplateId::new(1), TemplateId::new(2), TemplateId::new(3)]
        );
    }

    #[test]
    fn serde_roundtrip() {
        let template_id = TemplateId::new(42);
        let json = serde_json::to_string(&template_id).unwrap();
        let recovered: TemplateId = serde_json::from_str(&json).unwrap();
        assert_eq!(template_id, recovered);

        let symbol = SymbolId::new("Room");
        let json = serde_json::to_string(&symbol).unwrap();
        let recovered: SymbolId = serde_json::from_str(&json).unwrap();
        assert_eq!(symbol, recovered);
    }
}
