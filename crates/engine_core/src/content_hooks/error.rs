//! Error types for content hook operations.

use thiserror::Error;

use super::id::{ActionId, ConditionId, ContentHookId, EventId};

/// Result type for content hook operations.
pub type ContentHookResult<T> = Result<T, ContentHookError>;

/// Errors that can occur during content hook operations.
#[derive(Debug, Error)]
pub enum ContentHookError {
    #[error("duplicate hook ID: {0}")]
    DuplicateHookId(ContentHookId),

    #[error("duplicate hook name: {0}")]
    DuplicateHookName(String),

    #[error("duplicate event ID: {0}")]
    DuplicateEventId(EventId),

    #[error("duplicate event name: {0}")]
    DuplicateEventName(String),

    #[error("duplicate condition ID: {0}")]
    DuplicateConditionId(ConditionId),

    #[error("duplicate condition name: {0}")]
    DuplicateConditionName(String),

    #[error("duplicate action ID: {0}")]
    DuplicateActionId(ActionId),

    #[error("duplicate action name: {0}")]
    DuplicateActionName(String),

    #[error("hook '{hook}' references undefined event '{event}'")]
    UndefinedEventRef { hook: String, event: String },

    #[error("hook '{hook}' references undefined condition '{condition}'")]
    UndefinedConditionRef { hook: String, condition: String },

    #[error("hook '{hook}' references undefined action '{action}'")]
    UndefinedActionRef { hook: String, action: String },

    #[error("condition '{condition}' references undefined sub-condition '{sub_condition}'")]
    UndefinedSubConditionRef {
        condition: String,
        sub_condition: String,
    },

    #[error("action '{action}' references undefined sub-action '{sub_action}'")]
    UndefinedSubActionRef { action: String, sub_action: String },

    #[error("action '{action}' references undefined condition '{condition}'")]
    UndefinedActionConditionRef { action: String, condition: String },

    #[error("circular reference detected: {path}")]
    CircularReference { path: String },

    #[error("hook '{hook}' has no actions defined")]
    EmptyHookActions { hook: String },

    #[error("capability conflict: {0}")]
    CapabilityConflict(String),

    #[error("serialization error: {0}")]
    Serialization(String),
}

impl From<bincode::Error> for ContentHookError {
    fn from(err: bincode::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}
