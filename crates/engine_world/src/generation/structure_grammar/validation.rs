//! Validation errors for structure grammar.

use std::fmt;

use super::id::{RuleId, SymbolId, TemplateId};

/// Validation error types.
#[derive(Clone, Debug, PartialEq)]
pub enum ValidationError {
    /// Template ID is duplicated.
    DuplicateTemplateId(TemplateId),
    /// Rule ID is duplicated.
    DuplicateRuleId(RuleId),
    /// Referenced template not found.
    MissingTemplate(TemplateId),
    /// Referenced symbol has no matching rules.
    MissingSymbol(SymbolId),
    /// Rule has invalid weights.
    InvalidWeight { rule_id: RuleId, message: String },
    /// Socket referenced does not exist on template.
    InvalidSocket {
        template_id: TemplateId,
        socket_name: String,
    },
    /// Bounds are invalid (min > max).
    InvalidBounds {
        template_id: TemplateId,
        message: String,
    },
    /// Potential infinite recursion detected.
    PotentialRecursion { symbols: Vec<SymbolId> },
    /// Maximum depth exceeded during generation.
    MaxDepthExceeded { depth: u32, max_depth: u32 },
    /// Maximum steps exceeded during generation.
    MaxStepsExceeded { steps: u32, max_steps: u32 },
    /// Sockets are incompatible.
    IncompatibleSockets {
        from_socket: String,
        to_socket: String,
    },
    /// Placement would overlap existing.
    PlacementOverlap { existing: u64, new: u64 },
    /// No root symbol defined.
    NoRootSymbol,
    /// Empty grammar (no rules).
    EmptyGrammar,
    /// Generic validation error.
    Other(String),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateTemplateId(id) => write!(f, "duplicate template ID: {id}"),
            Self::DuplicateRuleId(id) => write!(f, "duplicate rule ID: {id}"),
            Self::MissingTemplate(id) => write!(f, "missing template: {id}"),
            Self::MissingSymbol(id) => write!(f, "missing symbol: {id}"),
            Self::InvalidWeight { rule_id, message } => {
                write!(f, "invalid weight in rule {rule_id}: {message}")
            }
            Self::InvalidSocket {
                template_id,
                socket_name,
            } => write!(
                f,
                "invalid socket '{socket_name}' on template {template_id}"
            ),
            Self::InvalidBounds {
                template_id,
                message,
            } => write!(f, "invalid bounds on template {template_id}: {message}"),
            Self::PotentialRecursion { symbols } => {
                let names: Vec<_> = symbols.iter().map(SymbolId::name).collect();
                write!(f, "potential recursion in symbols: {}", names.join(" -> "))
            }
            Self::MaxDepthExceeded { depth, max_depth } => {
                write!(f, "max depth exceeded: {depth} > {max_depth}")
            }
            Self::MaxStepsExceeded { steps, max_steps } => {
                write!(f, "max steps exceeded: {steps} > {max_steps}")
            }
            Self::IncompatibleSockets {
                from_socket,
                to_socket,
            } => write!(f, "incompatible sockets: '{from_socket}' and '{to_socket}'"),
            Self::PlacementOverlap { existing, new } => {
                write!(f, "placement {new} overlaps existing placement {existing}")
            }
            Self::NoRootSymbol => write!(f, "no root symbol defined"),
            Self::EmptyGrammar => write!(f, "grammar has no rules"),
            Self::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for ValidationError {}

/// Collection of validation errors.
#[derive(Clone, Debug, Default)]
pub struct ValidationErrors {
    errors: Vec<ValidationError>,
}

impl ValidationErrors {
    /// Create empty error collection.
    #[must_use]
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    /// Add an error.
    pub fn add(&mut self, error: ValidationError) {
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
    pub fn errors(&self) -> &[ValidationError] {
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
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} validation error(s):", self.errors.len())?;
        for (i, err) in self.errors.iter().enumerate() {
            writeln!(f, "  {}: {err}", i + 1)?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationErrors {}

impl ValidationErrors {
    /// Returns an iterator over the errors.
    pub fn iter(&self) -> std::slice::Iter<'_, ValidationError> {
        self.errors.iter()
    }
}

impl IntoIterator for ValidationErrors {
    type Item = ValidationError;
    type IntoIter = std::vec::IntoIter<ValidationError>;

    fn into_iter(self) -> Self::IntoIter {
        self.errors.into_iter()
    }
}

impl<'a> IntoIterator for &'a ValidationErrors {
    type Item = &'a ValidationError;
    type IntoIter = std::slice::Iter<'a, ValidationError>;

    fn into_iter(self) -> Self::IntoIter {
        self.errors.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = ValidationError::MissingTemplate(TemplateId::new(42));
        assert!(format!("{err}").contains("42"));

        let err = ValidationError::InvalidWeight {
            rule_id: RuleId::new(1),
            message: "negative weight".to_string(),
        };
        assert!(format!("{err}").contains("negative weight"));
    }

    #[test]
    fn validation_errors_collection() {
        let mut errors = ValidationErrors::new();
        assert!(errors.is_empty());

        errors.add(ValidationError::EmptyGrammar);
        errors.add(ValidationError::NoRootSymbol);

        assert_eq!(errors.len(), 2);
        assert!(!errors.is_empty());
    }

    #[test]
    fn validation_errors_result() {
        let empty = ValidationErrors::new();
        assert!(empty.into_result().is_ok());

        let mut non_empty = ValidationErrors::new();
        non_empty.add(ValidationError::EmptyGrammar);
        assert!(non_empty.into_result().is_err());
    }

    #[test]
    fn validation_errors_merge() {
        let mut a = ValidationErrors::new();
        a.add(ValidationError::EmptyGrammar);

        let mut b = ValidationErrors::new();
        b.add(ValidationError::NoRootSymbol);

        a.merge(b);
        assert_eq!(a.len(), 2);
    }
}
