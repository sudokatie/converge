//! Event definitions and registry.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

use super::{CooldownConfig, NarrativeEventKind, NarrativeTrigger, OutputPriority};

/// Error type for registry operations.
#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("event definition '{0}' not found")]
    NotFound(String),

    #[error("event definition '{0}' already exists")]
    AlreadyExists(String),

    #[error("invalid event definition: {0}")]
    Invalid(String),
}

/// A static template for a narrative event type.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventDefinition {
    /// Unique identifier for this event type.
    pub id: String,

    /// Human-readable display name.
    pub display_name: String,

    /// Event category.
    pub kind: NarrativeEventKind,

    /// Trigger conditions for this event.
    pub triggers: Vec<NarrativeTrigger>,

    /// Cooldown and repeat configuration.
    pub cooldown: CooldownConfig,

    /// Default duration in ticks.
    pub duration: u64,

    /// Output priority for queuing.
    pub priority: OutputPriority,

    /// Text template for narrative output (supports placeholders).
    pub text_template: Option<String>,

    /// Audio cue identifier.
    pub audio_cue: Option<String>,

    /// Tags for filtering and grouping.
    pub tags: Vec<String>,

    /// Whether this event is enabled.
    pub enabled: bool,

    /// Prerequisites (other event IDs that must have fired).
    pub prerequisites: Vec<String>,

    /// Events that this event blocks while active.
    pub blocks: Vec<String>,

    /// Custom data for game-specific extensions.
    pub custom_data: HashMap<String, String>,
}

impl EventDefinition {
    /// Create a new event definition.
    #[must_use]
    pub fn new(id: impl Into<String>, kind: NarrativeEventKind) -> Self {
        let id = id.into();
        Self {
            display_name: id.clone(),
            id,
            kind,
            triggers: Vec::new(),
            cooldown: CooldownConfig::once(),
            duration: kind.default_duration(),
            priority: OutputPriority::from_level(kind.default_priority()),
            text_template: None,
            audio_cue: None,
            tags: Vec::new(),
            enabled: true,
            prerequisites: Vec::new(),
            blocks: Vec::new(),
            custom_data: HashMap::new(),
        }
    }

    /// Set display name.
    #[must_use]
    pub fn with_display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = name.into();
        self
    }

    /// Add a trigger.
    #[must_use]
    pub fn with_trigger(mut self, trigger: NarrativeTrigger) -> Self {
        self.triggers.push(trigger);
        self
    }

    /// Set triggers.
    #[must_use]
    pub fn with_triggers(mut self, triggers: Vec<NarrativeTrigger>) -> Self {
        self.triggers = triggers;
        self
    }

    /// Set cooldown configuration.
    #[must_use]
    pub fn with_cooldown(mut self, cooldown: CooldownConfig) -> Self {
        self.cooldown = cooldown;
        self
    }

    /// Set duration.
    #[must_use]
    pub fn with_duration(mut self, duration: u64) -> Self {
        self.duration = duration;
        self
    }

    /// Set priority.
    #[must_use]
    pub fn with_priority(mut self, priority: OutputPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set text template.
    #[must_use]
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text_template = Some(text.into());
        self
    }

    /// Set audio cue.
    #[must_use]
    pub fn with_audio(mut self, cue: impl Into<String>) -> Self {
        self.audio_cue = Some(cue.into());
        self
    }

    /// Add a tag.
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set tags.
    #[must_use]
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Set enabled state.
    #[must_use]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Add a prerequisite.
    #[must_use]
    pub fn with_prerequisite(mut self, event_id: impl Into<String>) -> Self {
        self.prerequisites.push(event_id.into());
        self
    }

    /// Add a blocked event.
    #[must_use]
    pub fn with_blocks(mut self, event_id: impl Into<String>) -> Self {
        self.blocks.push(event_id.into());
        self
    }

    /// Add custom data.
    #[must_use]
    pub fn with_custom(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom_data.insert(key.into(), value.into());
        self
    }

    /// Check if a tag is present.
    #[must_use]
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Validate the definition.
    ///
    /// # Errors
    ///
    /// Returns `RegistryError::Invalid` if the id is empty or no triggers are defined.
    pub fn validate(&self) -> Result<(), RegistryError> {
        if self.id.is_empty() {
            return Err(RegistryError::Invalid("id cannot be empty".into()));
        }
        if self.triggers.is_empty() {
            return Err(RegistryError::Invalid(format!(
                "event '{}' has no triggers",
                self.id
            )));
        }
        Ok(())
    }
}

/// Central registry of all event definitions.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EventRegistry {
    /// Definitions indexed by ID.
    definitions: HashMap<String, EventDefinition>,

    /// Index by kind for efficient filtering.
    by_kind: HashMap<NarrativeEventKind, Vec<String>>,

    /// Index by tag.
    by_tag: HashMap<String, Vec<String>>,
}

impl EventRegistry {
    /// Create a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an event definition.
    ///
    /// # Errors
    ///
    /// Returns `RegistryError::Invalid` if the definition fails validation, or
    /// `RegistryError::AlreadyExists` if an event with the same id is already registered.
    pub fn register(&mut self, def: EventDefinition) -> Result<(), RegistryError> {
        def.validate()?;
        if self.definitions.contains_key(&def.id) {
            return Err(RegistryError::AlreadyExists(def.id.clone()));
        }

        let id = def.id.clone();
        let kind = def.kind;
        let tags = def.tags.clone();

        self.definitions.insert(id.clone(), def);
        self.by_kind.entry(kind).or_default().push(id.clone());
        for tag in tags {
            self.by_tag.entry(tag).or_default().push(id.clone());
        }

        Ok(())
    }

    /// Register or replace an event definition.
    ///
    /// # Errors
    ///
    /// Returns `RegistryError::Invalid` if the definition fails validation.
    pub fn register_or_replace(&mut self, def: EventDefinition) -> Result<(), RegistryError> {
        def.validate()?;
        self.unregister(&def.id);
        self.register(def)
    }

    /// Unregister an event definition.
    pub fn unregister(&mut self, id: &str) -> Option<EventDefinition> {
        if let Some(def) = self.definitions.remove(id) {
            if let Some(ids) = self.by_kind.get_mut(&def.kind) {
                ids.retain(|i| i != id);
            }
            for tag in &def.tags {
                if let Some(ids) = self.by_tag.get_mut(tag) {
                    ids.retain(|i| i != id);
                }
            }
            Some(def)
        } else {
            None
        }
    }

    /// Get an event definition by ID.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&EventDefinition> {
        self.definitions.get(id)
    }

    /// Get a mutable event definition by ID.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut EventDefinition> {
        self.definitions.get_mut(id)
    }

    /// Check if an event is registered.
    #[must_use]
    pub fn contains(&self, id: &str) -> bool {
        self.definitions.contains_key(id)
    }

    /// Get all definitions.
    pub fn all(&self) -> impl Iterator<Item = &EventDefinition> {
        self.definitions.values()
    }

    /// Get all enabled definitions.
    pub fn enabled(&self) -> impl Iterator<Item = &EventDefinition> {
        self.definitions.values().filter(|d| d.enabled)
    }

    /// Get definitions by kind.
    pub fn by_kind(&self, kind: NarrativeEventKind) -> impl Iterator<Item = &EventDefinition> {
        self.by_kind
            .get(&kind)
            .into_iter()
            .flatten()
            .filter_map(move |id| self.definitions.get(id))
    }

    /// Get definitions by tag.
    pub fn by_tag(&self, tag: &str) -> impl Iterator<Item = &EventDefinition> {
        self.by_tag
            .get(tag)
            .into_iter()
            .flatten()
            .filter_map(move |id| self.definitions.get(id))
    }

    /// Get all definition IDs.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.definitions.keys().map(String::as_str)
    }

    /// Number of registered definitions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    /// Check if registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    /// Clear all definitions.
    pub fn clear(&mut self) {
        self.definitions.clear();
        self.by_kind.clear();
        self.by_tag.clear();
    }

    /// Compute a fingerprint of the registry state.
    #[must_use]
    pub fn fingerprint(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        let mut ids: Vec<_> = self.definitions.keys().collect();
        ids.sort();
        for id in ids {
            hasher.update(id.as_bytes());
            if let Some(def) = self.definitions.get(id) {
                hasher.update(&[def.kind as u8]);
                hasher.update(&[u8::from(def.enabled)]);
                hasher.update(&def.duration.to_le_bytes());
            }
        }
        hasher.finalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_def(id: &str) -> EventDefinition {
        EventDefinition::new(id, NarrativeEventKind::Radio).with_trigger(NarrativeTrigger::always())
    }

    #[test]
    fn definition_new() {
        let def = EventDefinition::new("test", NarrativeEventKind::Disaster);
        assert_eq!(def.id, "test");
        assert_eq!(def.display_name, "test");
        assert_eq!(def.kind, NarrativeEventKind::Disaster);
        assert!(def.enabled);
    }

    #[test]
    fn definition_builders() {
        let def = EventDefinition::new("test", NarrativeEventKind::Radio)
            .with_display_name("Test Event")
            .with_text("Something happened!")
            .with_audio("radio_static")
            .with_tag("important")
            .with_duration(1000);

        assert_eq!(def.display_name, "Test Event");
        assert_eq!(def.text_template, Some("Something happened!".into()));
        assert_eq!(def.audio_cue, Some("radio_static".into()));
        assert!(def.has_tag("important"));
        assert_eq!(def.duration, 1000);
    }

    #[test]
    fn definition_validate_empty_id() {
        let def = EventDefinition {
            id: String::new(),
            ..make_test_def("x")
        };
        assert!(def.validate().is_err());
    }

    #[test]
    fn definition_validate_no_triggers() {
        let def = EventDefinition::new("test", NarrativeEventKind::Radio);
        assert!(def.validate().is_err());
    }

    #[test]
    fn registry_register() {
        let mut reg = EventRegistry::new();
        let def = make_test_def("test");

        assert!(reg.register(def).is_ok());
        assert!(reg.contains("test"));
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn registry_duplicate() {
        let mut reg = EventRegistry::new();
        reg.register(make_test_def("test")).unwrap();

        let result = reg.register(make_test_def("test"));
        assert!(matches!(result, Err(RegistryError::AlreadyExists(_))));
    }

    #[test]
    fn registry_unregister() {
        let mut reg = EventRegistry::new();
        reg.register(make_test_def("test")).unwrap();

        let removed = reg.unregister("test");
        assert!(removed.is_some());
        assert!(!reg.contains("test"));
    }

    #[test]
    fn registry_by_kind() {
        let mut reg = EventRegistry::new();
        reg.register(
            EventDefinition::new("d1", NarrativeEventKind::Disaster)
                .with_trigger(NarrativeTrigger::always()),
        )
        .unwrap();
        reg.register(
            EventDefinition::new("r1", NarrativeEventKind::Radio)
                .with_trigger(NarrativeTrigger::always()),
        )
        .unwrap();
        reg.register(
            EventDefinition::new("d2", NarrativeEventKind::Disaster)
                .with_trigger(NarrativeTrigger::always()),
        )
        .unwrap();

        let disasters: Vec<_> = reg.by_kind(NarrativeEventKind::Disaster).collect();
        assert_eq!(disasters.len(), 2);
    }

    #[test]
    fn registry_by_tag() {
        let mut reg = EventRegistry::new();
        reg.register(make_test_def("a").with_tag("urgent")).unwrap();
        reg.register(make_test_def("b").with_tag("urgent")).unwrap();
        reg.register(make_test_def("c").with_tag("minor")).unwrap();

        let urgent: Vec<_> = reg.by_tag("urgent").collect();
        assert_eq!(urgent.len(), 2);
    }

    #[test]
    fn registry_enabled_filter() {
        let mut reg = EventRegistry::new();
        reg.register(make_test_def("a")).unwrap();
        reg.register(make_test_def("b").with_enabled(false))
            .unwrap();

        let enabled: Vec<_> = reg.enabled().collect();
        assert_eq!(enabled.len(), 1);
    }

    #[test]
    fn registry_fingerprint_deterministic() {
        let mut reg1 = EventRegistry::new();
        let mut reg2 = EventRegistry::new();

        reg1.register(make_test_def("a")).unwrap();
        reg1.register(make_test_def("b")).unwrap();

        reg2.register(make_test_def("a")).unwrap();
        reg2.register(make_test_def("b")).unwrap();

        assert_eq!(reg1.fingerprint(), reg2.fingerprint());
    }

    #[test]
    fn serde_round_trip() {
        let def = EventDefinition::new("test", NarrativeEventKind::Anomaly)
            .with_trigger(NarrativeTrigger::at_tick(100))
            .with_text("Strange readings detected")
            .with_tag("mysterious");

        let json = serde_json::to_string(&def).unwrap();
        let recovered: EventDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, def);

        let mut reg = EventRegistry::new();
        reg.register(make_test_def("a")).unwrap();
        reg.register(make_test_def("b")).unwrap();

        let json = serde_json::to_string(&reg).unwrap();
        let recovered: EventRegistry = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.len(), 2);
    }
}
