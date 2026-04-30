//! Mission/expedition contract framework for structured survival goals.
//!
//! Provides deterministic, serde-covered foundation for missions and contracts:
//!
//! - [`MissionId`], [`ContractId`], [`ObjectiveId`] - Unique identifiers
//! - [`ObjectiveKind`] - Objective type classification
//! - [`ObjectiveSpec`] - Objective specification with targets
//! - [`MissionDefinition`] - Static mission template
//! - [`ExpeditionContract`] - Full contract with metadata
//! - [`MissionTracker`] - Central mission management
//!
//! # Determinism
//!
//! All operations are deterministic with stable ordering:
//! - Missions ordered by ID
//! - Objectives ordered by index
//! - Events ordered by tick, revision, and kind
//! - Fingerprints computed over ordered state
//!
//! # Objective Kinds
//!
//! - Gather: collect resources
//! - Deliver: transport items to location
//! - Build: construct structures
//! - Explore: scout/discover areas
//! - Survive: endure for duration
//! - Defend: protect against threats
//! - Repair: fix damaged structures
//! - Research: analyze phenomena
//! - Rescue: save/escort entities
//! - Custom: user-defined objectives

mod contract;
mod event;
mod fingerprint;
mod state;

pub use contract::{
    DeadlineConfig, ExpeditionContract, FactionSource, PenaltyDefinition, RepeatConfig,
    RewardDefinition, RiskLevel, ScopeConfig,
};
pub use event::{MissionEvent, MissionEventHistory, MissionEventKind, MissionEventPayload};
pub use fingerprint::{ChecksumBuilder, MissionChecksum, MissionFingerprint};
pub use state::{
    MissionProgress, MissionState, ObjectiveProgress, ObjectiveState, ProjectionSummary,
};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Unique mission instance identifier.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct MissionId(u64);

impl MissionId {
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Unique contract identifier.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct ContractId(u64);

impl ContractId {
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Unique objective identifier within a mission.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct ObjectiveId(u32);

impl ObjectiveId {
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn raw(&self) -> u32 {
        self.0
    }
}

/// Kind of objective.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[repr(u8)]
pub enum ObjectiveKind {
    /// Gather/collect resources.
    #[default]
    Gather = 0,
    /// Deliver items to a location.
    Deliver = 1,
    /// Build/construct structures.
    Build = 2,
    /// Explore/scout an area.
    Explore = 3,
    /// Survive for a duration.
    Survive = 4,
    /// Defend against threats.
    Defend = 5,
    /// Repair damaged structures.
    Repair = 6,
    /// Research/analyze phenomena.
    Research = 7,
    /// Rescue/escort entities.
    Rescue = 8,
    /// Custom objective type.
    Custom = 9,
}

impl ObjectiveKind {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Gather => "gather",
            Self::Deliver => "deliver",
            Self::Build => "build",
            Self::Explore => "explore",
            Self::Survive => "survive",
            Self::Defend => "defend",
            Self::Repair => "repair",
            Self::Research => "research",
            Self::Rescue => "rescue",
            Self::Custom => "custom",
        }
    }

    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Gather => "Gather",
            Self::Deliver => "Deliver",
            Self::Build => "Build",
            Self::Explore => "Explore",
            Self::Survive => "Survive",
            Self::Defend => "Defend",
            Self::Repair => "Repair",
            Self::Research => "Research",
            Self::Rescue => "Rescue",
            Self::Custom => "Custom",
        }
    }

    #[must_use]
    pub const fn is_count_based(&self) -> bool {
        matches!(
            self,
            Self::Gather | Self::Deliver | Self::Build | Self::Repair | Self::Rescue
        )
    }

    #[must_use]
    pub const fn is_duration_based(&self) -> bool {
        matches!(self, Self::Survive | Self::Defend)
    }

    #[must_use]
    pub const fn is_discovery_based(&self) -> bool {
        matches!(self, Self::Explore | Self::Research)
    }

    #[must_use]
    pub const fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Gather),
            1 => Some(Self::Deliver),
            2 => Some(Self::Build),
            3 => Some(Self::Explore),
            4 => Some(Self::Survive),
            5 => Some(Self::Defend),
            6 => Some(Self::Repair),
            7 => Some(Self::Research),
            8 => Some(Self::Rescue),
            9 => Some(Self::Custom),
            _ => None,
        }
    }
}

/// Objective specification within a mission definition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ObjectiveSpec {
    /// Objective kind.
    pub kind: ObjectiveKind,

    /// Human-readable description.
    pub description: String,

    /// Resource type (for Gather/Deliver).
    pub resource: Option<String>,

    /// Structure type (for Build/Repair).
    pub structure: Option<String>,

    /// Entity type (for Rescue).
    pub entity: Option<String>,

    /// Area/region target (for Explore).
    pub area: Option<String>,

    /// Target count for completion.
    pub target_count: u32,

    /// Target duration in ticks (for Survive/Defend).
    pub target_duration: u64,

    /// Whether this objective is optional.
    pub optional: bool,

    /// Whether this objective is initially hidden.
    pub hidden: bool,

    /// Index of prerequisite objective (-1 for none).
    pub prerequisite: i32,

    /// Custom objective type identifier.
    pub custom_type: Option<String>,

    /// Custom data.
    pub custom_data: BTreeMap<String, String>,
}

impl ObjectiveSpec {
    /// Create a new objective spec.
    #[must_use]
    pub fn new(kind: ObjectiveKind) -> Self {
        Self {
            kind,
            description: String::new(),
            resource: None,
            structure: None,
            entity: None,
            area: None,
            target_count: 1,
            target_duration: 0,
            optional: false,
            hidden: false,
            prerequisite: -1,
            custom_type: None,
            custom_data: BTreeMap::new(),
        }
    }

    /// Set description.
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set resource type.
    #[must_use]
    pub fn with_resource(mut self, resource: impl Into<String>) -> Self {
        self.resource = Some(resource.into());
        self
    }

    /// Set structure type.
    #[must_use]
    pub fn with_structure(mut self, structure: impl Into<String>) -> Self {
        self.structure = Some(structure.into());
        self
    }

    /// Set entity type.
    #[must_use]
    pub fn with_entity(mut self, entity: impl Into<String>) -> Self {
        self.entity = Some(entity.into());
        self
    }

    /// Set area target.
    #[must_use]
    pub fn with_area(mut self, area: impl Into<String>) -> Self {
        self.area = Some(area.into());
        self
    }

    /// Set target count.
    #[must_use]
    pub fn with_target_count(mut self, count: u32) -> Self {
        self.target_count = count;
        self
    }

    /// Set target duration.
    #[must_use]
    pub fn with_target_duration(mut self, ticks: u64) -> Self {
        self.target_duration = ticks;
        self
    }

    /// Mark as optional.
    #[must_use]
    pub fn with_optional(mut self, optional: bool) -> Self {
        self.optional = optional;
        self
    }

    /// Mark as hidden.
    #[must_use]
    pub fn with_hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    /// Set prerequisite index.
    #[must_use]
    pub fn with_prerequisite(mut self, index: i32) -> Self {
        self.prerequisite = index;
        self
    }

    /// Set custom type.
    #[must_use]
    pub fn with_custom_type(mut self, custom_type: impl Into<String>) -> Self {
        self.custom_type = Some(custom_type.into());
        self
    }

    /// Add custom data.
    #[must_use]
    pub fn with_custom(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom_data.insert(key.into(), value.into());
        self
    }

    /// Check if prerequisite is satisfied.
    #[must_use]
    pub fn prerequisite_satisfied(&self, completed_objectives: &[bool]) -> bool {
        if self.prerequisite < 0 {
            return true;
        }
        #[allow(clippy::cast_sign_loss)]
        let idx = self.prerequisite as usize;
        completed_objectives.get(idx).copied().unwrap_or(false)
    }
}

impl Default for ObjectiveSpec {
    fn default() -> Self {
        Self::new(ObjectiveKind::Gather)
    }
}

/// Mission definition template.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MissionDefinition {
    /// Unique definition identifier.
    pub id: String,

    /// Display name.
    pub display_name: String,

    /// Detailed description.
    pub description: String,

    /// Objective specifications.
    pub objectives: Vec<ObjectiveSpec>,

    /// Base duration in ticks (optional).
    pub base_duration: Option<u64>,

    /// Whether this mission is enabled.
    pub enabled: bool,

    /// Whether this mission is repeatable.
    pub repeatable: bool,

    /// Cooldown between repeats.
    pub repeat_cooldown: u64,

    /// Tags for filtering.
    pub tags: Vec<String>,

    /// Prerequisites (mission IDs that must be completed).
    pub prerequisites: Vec<String>,

    /// Custom data.
    pub custom_data: BTreeMap<String, String>,
}

impl MissionDefinition {
    /// Create a new mission definition.
    #[must_use]
    pub fn new(id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            description: String::new(),
            objectives: Vec::new(),
            base_duration: None,
            enabled: true,
            repeatable: false,
            repeat_cooldown: 0,
            tags: Vec::new(),
            prerequisites: Vec::new(),
            custom_data: BTreeMap::new(),
        }
    }

    /// Set description.
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Add an objective.
    #[must_use]
    pub fn with_objective(mut self, spec: ObjectiveSpec) -> Self {
        self.objectives.push(spec);
        self
    }

    /// Set base duration.
    #[must_use]
    pub fn with_duration(mut self, ticks: u64) -> Self {
        self.base_duration = Some(ticks);
        self
    }

    /// Set enabled state.
    #[must_use]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set repeatable.
    #[must_use]
    pub fn with_repeatable(mut self, repeatable: bool, cooldown: u64) -> Self {
        self.repeatable = repeatable;
        self.repeat_cooldown = cooldown;
        self
    }

    /// Add tag.
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add prerequisite.
    #[must_use]
    pub fn with_prerequisite(mut self, mission_id: impl Into<String>) -> Self {
        self.prerequisites.push(mission_id.into());
        self
    }

    /// Add custom data.
    #[must_use]
    pub fn with_custom(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom_data.insert(key.into(), value.into());
        self
    }

    /// Count required objectives.
    #[must_use]
    pub fn required_objective_count(&self) -> usize {
        self.objectives.iter().filter(|o| !o.optional).count()
    }

    /// Compute fingerprint.
    #[must_use]
    pub fn fingerprint(&self) -> MissionFingerprint {
        MissionFingerprint::from_definition(
            &self.id,
            self.objectives.len(),
            self.base_duration,
            self.enabled,
            self.repeatable,
        )
    }
}

impl Default for MissionDefinition {
    fn default() -> Self {
        Self::new("default", "Default")
    }
}

/// Error for registry operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegistryError {
    /// Definition not found.
    NotFound(String),
    /// Duplicate definition.
    Duplicate(String),
    /// Prerequisites not satisfied.
    PrerequisitesNotMet(Vec<String>),
    /// Mission not available.
    NotAvailable(String),
    /// Mission already active.
    AlreadyActive(MissionId),
    /// Mission not active.
    NotActive(MissionId),
    /// Invalid objective.
    InvalidObjective(ObjectiveId),
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "mission definition not found: {id}"),
            Self::Duplicate(id) => write!(f, "duplicate mission definition: {id}"),
            Self::PrerequisitesNotMet(ids) => {
                write!(f, "prerequisites not met: {}", ids.join(", "))
            }
            Self::NotAvailable(id) => write!(f, "mission not available: {id}"),
            Self::AlreadyActive(id) => write!(f, "mission already active: {id:?}"),
            Self::NotActive(id) => write!(f, "mission not active: {id:?}"),
            Self::InvalidObjective(id) => write!(f, "invalid objective: {id:?}"),
        }
    }
}

impl std::error::Error for RegistryError {}

/// Query filter for missions.
#[derive(Clone, Debug, Default)]
pub struct MissionQuery {
    /// Filter by definition ID.
    pub definition_id: Option<String>,
    /// Filter by state.
    pub state: Option<MissionState>,
    /// Filter by tag.
    pub tag: Option<String>,
    /// Filter by region.
    pub region: Option<String>,
    /// Include completed missions.
    pub include_completed: bool,
    /// Maximum results.
    pub limit: Option<usize>,
}

impl MissionQuery {
    /// Create a new query.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by definition.
    #[must_use]
    pub fn with_definition(mut self, id: impl Into<String>) -> Self {
        self.definition_id = Some(id.into());
        self
    }

    /// Filter by state.
    #[must_use]
    pub fn with_state(mut self, state: MissionState) -> Self {
        self.state = Some(state);
        self
    }

    /// Filter by tag.
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    /// Filter by region.
    #[must_use]
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Include completed missions.
    #[must_use]
    pub fn include_completed(mut self) -> Self {
        self.include_completed = true;
        self
    }

    /// Limit results.
    #[must_use]
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Summary of tracker state.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TrackerSummary {
    /// Number of registered definitions.
    pub definition_count: usize,
    /// Number of active missions.
    pub active_count: usize,
    /// Number of completed missions.
    pub completed_count: usize,
    /// Number of failed missions.
    pub failed_count: usize,
    /// Total events recorded.
    pub event_count: usize,
    /// Current tick.
    pub tick: u64,
}

/// Central mission tracker and registry.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MissionTracker {
    /// Registered mission definitions.
    definitions: BTreeMap<String, MissionDefinition>,
    /// Active mission progress.
    active: BTreeMap<MissionId, MissionProgress>,
    /// Completed missions (for history/repeat tracking).
    completed: BTreeMap<MissionId, MissionProgress>,
    /// Event history.
    history: MissionEventHistory,
    /// Next mission ID.
    next_mission_id: u64,
    /// Completed definition IDs (for prerequisite tracking).
    completed_definitions: Vec<String>,
    /// Repeat cooldowns (`definition_id` -> tick when available).
    repeat_cooldowns: BTreeMap<String, u64>,
    /// Current tick.
    current_tick: u64,
}

impl MissionTracker {
    /// Create a new tracker.
    #[must_use]
    pub fn new() -> Self {
        Self {
            definitions: BTreeMap::new(),
            active: BTreeMap::new(),
            completed: BTreeMap::new(),
            history: MissionEventHistory::new(),
            next_mission_id: 1,
            completed_definitions: Vec::new(),
            repeat_cooldowns: BTreeMap::new(),
            current_tick: 0,
        }
    }

    /// Register a mission definition.
    ///
    /// # Errors
    ///
    /// Returns `RegistryError::Duplicate` if definition already exists.
    pub fn register(&mut self, definition: MissionDefinition) -> Result<(), RegistryError> {
        if self.definitions.contains_key(&definition.id) {
            return Err(RegistryError::Duplicate(definition.id.clone()));
        }
        self.definitions.insert(definition.id.clone(), definition);
        Ok(())
    }

    /// Get a definition by ID.
    #[must_use]
    pub fn definition(&self, id: &str) -> Option<&MissionDefinition> {
        self.definitions.get(id)
    }

    /// Get all definitions.
    pub fn definitions(&self) -> impl Iterator<Item = &MissionDefinition> {
        self.definitions.values()
    }

    /// Check if a definition is available for acceptance.
    #[must_use]
    pub fn is_available(&self, definition_id: &str) -> bool {
        let Some(def) = self.definitions.get(definition_id) else {
            return false;
        };

        if !def.enabled {
            return false;
        }

        for prereq in &def.prerequisites {
            if !self.completed_definitions.contains(prereq) {
                return false;
            }
        }

        if self
            .repeat_cooldowns
            .get(definition_id)
            .is_some_and(|&cooldown_tick| self.current_tick < cooldown_tick)
        {
            return false;
        }

        if !def.repeatable && self.completed_definitions.contains(&def.id) {
            return false;
        }

        true
    }

    /// Get available mission definitions.
    pub fn available_definitions(&self) -> impl Iterator<Item = &MissionDefinition> {
        self.definitions
            .values()
            .filter(|d| self.is_available(&d.id))
    }

    /// Accept a mission.
    ///
    /// # Errors
    ///
    /// Returns `RegistryError::NotFound` if definition doesn't exist,
    /// or `RegistryError::NotAvailable` if prerequisites aren't met.
    ///
    /// # Panics
    ///
    /// Never panics - the expect is guarded by `is_available` check.
    pub fn accept(&mut self, definition_id: &str) -> Result<MissionId, RegistryError> {
        if !self.is_available(definition_id) {
            let def = self.definitions.get(definition_id);
            if def.is_none() {
                return Err(RegistryError::NotFound(definition_id.to_string()));
            }
            return Err(RegistryError::NotAvailable(definition_id.to_string()));
        }

        let def = self
            .definitions
            .get(definition_id)
            .expect("definition existence verified by is_available");
        let mission_id = MissionId::new(self.next_mission_id);
        self.next_mission_id += 1;

        let mut progress = MissionProgress::new(mission_id, definition_id, self.current_tick);

        if let Some(duration) = def.base_duration {
            progress.deadline = Some(self.current_tick + duration);
        }

        for (idx, obj_spec) in def.objectives.iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            let obj_id = ObjectiveId::new(idx as u32);
            let obj_progress = ObjectiveProgress::new(
                obj_id,
                obj_spec.target_count,
                obj_spec.target_duration,
                obj_spec.optional,
            );
            progress.add_objective(obj_progress);
        }

        self.active.insert(mission_id, progress);
        self.history
            .record(MissionEvent::accepted(mission_id, self.current_tick, 0));

        Ok(mission_id)
    }

    /// Start a mission.
    ///
    /// # Errors
    ///
    /// Returns `RegistryError::NotActive` if mission isn't active.
    pub fn start(&mut self, mission_id: MissionId) -> Result<(), RegistryError> {
        let progress = self
            .active
            .get_mut(&mission_id)
            .ok_or(RegistryError::NotActive(mission_id))?;

        let old_state = progress.state;
        progress.start(self.current_tick);

        self.history.record(MissionEvent::mission_state_changed(
            mission_id,
            old_state,
            MissionState::Active,
            self.current_tick,
            0,
        ));

        Ok(())
    }

    /// Record progress on an objective.
    ///
    /// # Errors
    ///
    /// Returns `RegistryError::NotActive` if mission isn't active,
    /// or `RegistryError::InvalidObjective` if objective doesn't exist.
    pub fn record_progress(
        &mut self,
        mission_id: MissionId,
        objective_id: ObjectiveId,
        amount: u32,
    ) -> Result<(), RegistryError> {
        let progress = self
            .active
            .get_mut(&mission_id)
            .ok_or(RegistryError::NotActive(mission_id))?;

        let obj = progress
            .objective_mut(objective_id)
            .ok_or(RegistryError::InvalidObjective(objective_id))?;

        let old_state = obj.state;
        obj.add_progress(amount, self.current_tick);
        let new_total = obj.current_count;

        self.history.record(MissionEvent::progress(
            mission_id,
            objective_id,
            amount,
            new_total,
            self.current_tick,
            0,
        ));

        if old_state != obj.state {
            self.history.record(MissionEvent::objective_state_changed(
                mission_id,
                objective_id,
                old_state,
                obj.state,
                self.current_tick,
                0,
            ));
        }

        Ok(())
    }

    /// Record elapsed time on a timed objective.
    ///
    /// # Errors
    ///
    /// Returns `RegistryError::NotActive` if mission isn't active,
    /// or `RegistryError::InvalidObjective` if objective doesn't exist.
    pub fn record_elapsed(
        &mut self,
        mission_id: MissionId,
        objective_id: ObjectiveId,
        ticks: u64,
    ) -> Result<(), RegistryError> {
        let progress = self
            .active
            .get_mut(&mission_id)
            .ok_or(RegistryError::NotActive(mission_id))?;

        let obj = progress
            .objective_mut(objective_id)
            .ok_or(RegistryError::InvalidObjective(objective_id))?;

        let old_state = obj.state;
        obj.add_elapsed(ticks, self.current_tick);

        if old_state != obj.state {
            self.history.record(MissionEvent::objective_state_changed(
                mission_id,
                objective_id,
                old_state,
                obj.state,
                self.current_tick,
                0,
            ));
        }

        Ok(())
    }

    /// Complete an objective manually.
    ///
    /// # Errors
    ///
    /// Returns `RegistryError::NotActive` if mission isn't active,
    /// or `RegistryError::InvalidObjective` if objective doesn't exist.
    pub fn complete_objective(
        &mut self,
        mission_id: MissionId,
        objective_id: ObjectiveId,
    ) -> Result<(), RegistryError> {
        let progress = self
            .active
            .get_mut(&mission_id)
            .ok_or(RegistryError::NotActive(mission_id))?;

        let obj = progress
            .objective_mut(objective_id)
            .ok_or(RegistryError::InvalidObjective(objective_id))?;

        let old_state = obj.state;
        obj.complete(self.current_tick);

        self.history.record(MissionEvent::objective_state_changed(
            mission_id,
            objective_id,
            old_state,
            ObjectiveState::Completed,
            self.current_tick,
            0,
        ));

        Ok(())
    }

    /// Abandon a mission.
    ///
    /// # Errors
    ///
    /// Returns `RegistryError::NotActive` if mission isn't active.
    ///
    /// # Panics
    ///
    /// Never panics - the expect is guarded by prior existence check.
    pub fn abandon(&mut self, mission_id: MissionId) -> Result<(), RegistryError> {
        let progress = self
            .active
            .get_mut(&mission_id)
            .ok_or(RegistryError::NotActive(mission_id))?;

        let old_state = progress.state;
        progress.abandon(self.current_tick);

        self.history.record(MissionEvent::mission_state_changed(
            mission_id,
            old_state,
            MissionState::Abandoned,
            self.current_tick,
            0,
        ));
        self.history
            .record(MissionEvent::abandoned(mission_id, self.current_tick, 0));

        let completed = self
            .active
            .remove(&mission_id)
            .expect("mission existence verified above");
        self.completed.insert(mission_id, completed);

        Ok(())
    }

    /// Tick deadlines and time windows.
    pub fn tick(&mut self, tick: u64) -> Vec<MissionId> {
        self.current_tick = tick;
        let mut expired = Vec::new();

        let mission_ids: Vec<_> = self.active.keys().copied().collect();
        for mission_id in mission_ids {
            if let Some(progress) = self.active.get(&mission_id)
                && progress.is_past_deadline(tick)
                && progress.state.is_active()
            {
                expired.push(mission_id);
            }
        }

        for mission_id in &expired {
            if let Some(progress) = self.active.get_mut(mission_id) {
                let old_state = progress.state;
                progress.expire(tick);

                self.history.record(MissionEvent::mission_state_changed(
                    *mission_id,
                    old_state,
                    MissionState::Expired,
                    tick,
                    0,
                ));
                self.history
                    .record(MissionEvent::expired(*mission_id, tick, 0));
            }
        }

        for mission_id in &expired {
            if let Some(completed) = self.active.remove(mission_id) {
                self.completed.insert(*mission_id, completed);
            }
        }

        expired
    }

    /// Evaluate mission completion.
    ///
    /// # Errors
    ///
    /// Returns `RegistryError::NotActive` if mission isn't active.
    ///
    /// # Panics
    ///
    /// Never panics - the expects are guarded by prior existence check.
    pub fn evaluate(&mut self, mission_id: MissionId) -> Result<bool, RegistryError> {
        let progress = self
            .active
            .get(&mission_id)
            .ok_or(RegistryError::NotActive(mission_id))?;

        if progress.any_required_failed() {
            let old_state = progress.state;
            let progress = self
                .active
                .get_mut(&mission_id)
                .expect("mission existence verified above");
            progress.fail(self.current_tick);

            self.history.record(MissionEvent::mission_state_changed(
                mission_id,
                old_state,
                MissionState::Failed,
                self.current_tick,
                0,
            ));
            self.history
                .record(MissionEvent::failed(mission_id, self.current_tick, 0));

            let completed = self
                .active
                .remove(&mission_id)
                .expect("mission existence verified above");
            self.completed.insert(mission_id, completed);
            return Ok(false);
        }

        if progress.all_required_complete() {
            let def_id = progress.definition_id.clone();
            let old_state = progress.state;
            let progress = self
                .active
                .get_mut(&mission_id)
                .expect("mission existence verified above");
            progress.complete(self.current_tick);

            self.history.record(MissionEvent::mission_state_changed(
                mission_id,
                old_state,
                MissionState::Completed,
                self.current_tick,
                0,
            ));
            self.history
                .record(MissionEvent::completed(mission_id, self.current_tick, 0));

            if !self.completed_definitions.contains(&def_id) {
                self.completed_definitions.push(def_id.clone());
                self.completed_definitions.sort();
            }

            if let Some(def) = self.definitions.get(&def_id)
                && def.repeatable
                && def.repeat_cooldown > 0
            {
                self.repeat_cooldowns
                    .insert(def_id, self.current_tick + def.repeat_cooldown);
            }

            let completed = self
                .active
                .remove(&mission_id)
                .expect("mission existence verified above");
            self.completed.insert(mission_id, completed);
            return Ok(true);
        }

        Ok(false)
    }

    /// Get active mission progress.
    #[must_use]
    pub fn active(&self, mission_id: MissionId) -> Option<&MissionProgress> {
        self.active.get(&mission_id)
    }

    /// Get mutable active mission progress.
    pub fn active_mut(&mut self, mission_id: MissionId) -> Option<&mut MissionProgress> {
        self.active.get_mut(&mission_id)
    }

    /// Get all active missions.
    pub fn active_missions(&self) -> impl Iterator<Item = (&MissionId, &MissionProgress)> {
        self.active.iter()
    }

    /// Get completed mission.
    #[must_use]
    pub fn completed(&self, mission_id: MissionId) -> Option<&MissionProgress> {
        self.completed.get(&mission_id)
    }

    /// Get all completed missions.
    pub fn completed_missions(&self) -> impl Iterator<Item = (&MissionId, &MissionProgress)> {
        self.completed.iter()
    }

    /// Query missions.
    pub fn query(&self, query: &MissionQuery) -> Vec<&MissionProgress> {
        let mut results: Vec<_> = self.active.values().collect();

        if query.include_completed {
            results.extend(self.completed.values());
        }

        if let Some(ref def_id) = query.definition_id {
            results.retain(|p| p.definition_id == *def_id);
        }

        if let Some(state) = query.state {
            results.retain(|p| p.state == state);
        }

        if let Some(ref region) = query.region {
            results.retain(|p| p.active_region.as_ref() == Some(region));
        }

        results.sort_by_key(|p| p.id);

        if let Some(limit) = query.limit {
            results.truncate(limit);
        }

        results
    }

    /// Get event history.
    #[must_use]
    pub fn history(&self) -> &MissionEventHistory {
        &self.history
    }

    /// Get tracker summary.
    #[must_use]
    pub fn summary(&self) -> TrackerSummary {
        TrackerSummary {
            definition_count: self.definitions.len(),
            active_count: self.active.len(),
            completed_count: self
                .completed
                .values()
                .filter(|p| p.state == MissionState::Completed)
                .count(),
            failed_count: self
                .completed
                .values()
                .filter(|p| p.state == MissionState::Failed)
                .count(),
            event_count: self.history.len(),
            tick: self.current_tick,
        }
    }

    /// Compute checksum of current state.
    #[must_use]
    pub fn checksum(&self) -> MissionChecksum {
        let mut builder = ChecksumBuilder::new();

        for (id, progress) in &self.active {
            builder.add_active_mission(
                *id,
                &progress.definition_id,
                progress.state,
                progress.started_at,
            );
            for (obj_id, obj) in &progress.objectives {
                builder.add_objective_progress(
                    *id,
                    *obj_id,
                    obj.state,
                    obj.current_count,
                    obj.elapsed_ticks,
                );
            }
        }

        for (id, progress) in &self.completed {
            if let Some(ended_at) = progress.ended_at {
                builder.add_completed_mission(
                    *id,
                    &progress.definition_id,
                    progress.state,
                    ended_at,
                );
            }
        }

        builder.build(self.current_tick, self.history.checksum_since(0))
    }

    /// Compute combined fingerprint of all definitions.
    #[must_use]
    pub fn definitions_fingerprint(&self) -> MissionFingerprint {
        let mut combined = MissionFingerprint::new(0);
        for def in self.definitions.values() {
            combined = combined.combine(&def.fingerprint());
        }
        combined
    }

    /// Get projection summaries for unloaded region.
    pub fn projections(&self, region: &str) -> Vec<ProjectionSummary> {
        self.active
            .values()
            .filter(|p| p.active_region.as_ref() == Some(&region.to_string()))
            .map(MissionProgress::projection_summary)
            .collect()
    }
}

impl Default for MissionTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Built-in mission preset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MissionPreset {
    /// Supply run: gather and deliver resources.
    SupplyRun,
    /// Breach repair: fix hull breaches.
    BreachRepair,
    /// Anomaly survey: investigate anomalies.
    AnomalySurvey,
    /// Survivor rescue: rescue stranded survivors.
    SurvivorRescue,
    /// Base defense: defend against threats.
    BaseDefense,
    /// Research expedition: analyze phenomena.
    ResearchExpedition,
}

impl MissionPreset {
    /// Get preset identifier.
    #[must_use]
    pub const fn id(&self) -> &'static str {
        match self {
            Self::SupplyRun => "supply_run",
            Self::BreachRepair => "breach_repair",
            Self::AnomalySurvey => "anomaly_survey",
            Self::SurvivorRescue => "survivor_rescue",
            Self::BaseDefense => "base_defense",
            Self::ResearchExpedition => "research_expedition",
        }
    }

    /// Get preset display name.
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::SupplyRun => "Supply Run",
            Self::BreachRepair => "Breach Repair",
            Self::AnomalySurvey => "Anomaly Survey",
            Self::SurvivorRescue => "Survivor Rescue",
            Self::BaseDefense => "Base Defense",
            Self::ResearchExpedition => "Research Expedition",
        }
    }

    /// Create mission definition from preset.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn definition(&self) -> MissionDefinition {
        match self {
            Self::SupplyRun => MissionDefinition::new(self.id(), self.display_name())
                .with_description("Gather supplies and deliver them to the destination")
                .with_objective(
                    ObjectiveSpec::new(ObjectiveKind::Gather)
                        .with_description("Collect supplies")
                        .with_resource("supplies")
                        .with_target_count(20),
                )
                .with_objective(
                    ObjectiveSpec::new(ObjectiveKind::Deliver)
                        .with_description("Deliver supplies to outpost")
                        .with_resource("supplies")
                        .with_target_count(20)
                        .with_prerequisite(0),
                )
                .with_duration(18000)
                .with_repeatable(true, 6000)
                .with_tag("logistics"),

            Self::BreachRepair => MissionDefinition::new(self.id(), self.display_name())
                .with_description("Repair hull breaches before atmosphere vents")
                .with_objective(
                    ObjectiveSpec::new(ObjectiveKind::Gather)
                        .with_description("Collect repair materials")
                        .with_resource("repair_kit")
                        .with_target_count(5),
                )
                .with_objective(
                    ObjectiveSpec::new(ObjectiveKind::Repair)
                        .with_description("Seal hull breaches")
                        .with_structure("hull_breach")
                        .with_target_count(3)
                        .with_prerequisite(0),
                )
                .with_duration(9000)
                .with_tag("emergency")
                .with_tag("structural"),

            Self::AnomalySurvey => MissionDefinition::new(self.id(), self.display_name())
                .with_description("Investigate and document anomalous phenomena")
                .with_objective(
                    ObjectiveSpec::new(ObjectiveKind::Explore)
                        .with_description("Locate anomaly source")
                        .with_area("anomaly_zone"),
                )
                .with_objective(
                    ObjectiveSpec::new(ObjectiveKind::Research)
                        .with_description("Analyze anomaly readings")
                        .with_target_count(5)
                        .with_prerequisite(0),
                )
                .with_objective(
                    ObjectiveSpec::new(ObjectiveKind::Gather)
                        .with_description("Collect anomaly samples")
                        .with_resource("anomaly_sample")
                        .with_target_count(3)
                        .with_optional(true),
                )
                .with_duration(27000)
                .with_tag("research")
                .with_tag("exploration"),

            Self::SurvivorRescue => MissionDefinition::new(self.id(), self.display_name())
                .with_description("Locate and rescue stranded survivors")
                .with_objective(
                    ObjectiveSpec::new(ObjectiveKind::Explore)
                        .with_description("Find survivor signal")
                        .with_area("distress_beacon"),
                )
                .with_objective(
                    ObjectiveSpec::new(ObjectiveKind::Rescue)
                        .with_description("Rescue survivors")
                        .with_entity("survivor")
                        .with_target_count(3)
                        .with_prerequisite(0),
                )
                .with_objective(
                    ObjectiveSpec::new(ObjectiveKind::Deliver)
                        .with_description("Return survivors to base")
                        .with_entity("survivor")
                        .with_target_count(3)
                        .with_prerequisite(1),
                )
                .with_duration(21600)
                .with_tag("rescue")
                .with_tag("humanitarian"),

            Self::BaseDefense => MissionDefinition::new(self.id(), self.display_name())
                .with_description("Defend the base against incoming threats")
                .with_objective(
                    ObjectiveSpec::new(ObjectiveKind::Build)
                        .with_description("Erect defensive barriers")
                        .with_structure("barricade")
                        .with_target_count(4)
                        .with_optional(true),
                )
                .with_objective(
                    ObjectiveSpec::new(ObjectiveKind::Defend)
                        .with_description("Hold the perimeter")
                        .with_target_duration(3600),
                )
                .with_objective(
                    ObjectiveSpec::new(ObjectiveKind::Survive)
                        .with_description("Survive the assault")
                        .with_target_duration(3600)
                        .with_prerequisite(1),
                )
                .with_duration(7200)
                .with_tag("combat")
                .with_tag("defensive"),

            Self::ResearchExpedition => MissionDefinition::new(self.id(), self.display_name())
                .with_description("Conduct field research at remote location")
                .with_objective(
                    ObjectiveSpec::new(ObjectiveKind::Explore)
                        .with_description("Reach research site")
                        .with_area("research_site"),
                )
                .with_objective(
                    ObjectiveSpec::new(ObjectiveKind::Build)
                        .with_description("Set up research equipment")
                        .with_structure("research_station")
                        .with_target_count(1)
                        .with_prerequisite(0),
                )
                .with_objective(
                    ObjectiveSpec::new(ObjectiveKind::Research)
                        .with_description("Conduct experiments")
                        .with_target_count(10)
                        .with_prerequisite(1),
                )
                .with_objective(
                    ObjectiveSpec::new(ObjectiveKind::Gather)
                        .with_description("Collect rare specimens")
                        .with_resource("specimen")
                        .with_target_count(5)
                        .with_optional(true),
                )
                .with_duration(36000)
                .with_tag("research")
                .with_tag("expedition"),
        }
    }

    /// Get all presets.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::SupplyRun,
            Self::BreachRepair,
            Self::AnomalySurvey,
            Self::SurvivorRescue,
            Self::BaseDefense,
            Self::ResearchExpedition,
        ]
    }
}

/// Register all preset missions.
pub fn register_presets(tracker: &mut MissionTracker) {
    for preset in MissionPreset::all() {
        let _ = tracker.register(preset.definition());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mission_id_ordering() {
        let id1 = MissionId::new(1);
        let id2 = MissionId::new(2);
        let id3 = MissionId::new(1);

        assert!(id1 < id2);
        assert_eq!(id1, id3);
    }

    #[test]
    fn objective_kind_properties() {
        assert!(ObjectiveKind::Gather.is_count_based());
        assert!(ObjectiveKind::Survive.is_duration_based());
        assert!(ObjectiveKind::Explore.is_discovery_based());
        assert_eq!(ObjectiveKind::from_raw(0), Some(ObjectiveKind::Gather));
        assert_eq!(ObjectiveKind::from_raw(99), None);
    }

    #[test]
    fn objective_spec_prerequisite() {
        let spec = ObjectiveSpec::new(ObjectiveKind::Deliver).with_prerequisite(0);
        assert!(!spec.prerequisite_satisfied(&[false, false]));
        assert!(spec.prerequisite_satisfied(&[true, false]));
    }

    #[test]
    fn mission_definition_creation() {
        let def = MissionDefinition::new("test", "Test Mission")
            .with_description("A test mission")
            .with_objective(ObjectiveSpec::new(ObjectiveKind::Gather).with_target_count(10))
            .with_objective(ObjectiveSpec::new(ObjectiveKind::Deliver).with_optional(true))
            .with_duration(5000)
            .with_tag("test");

        assert_eq!(def.id, "test");
        assert_eq!(def.objectives.len(), 2);
        assert_eq!(def.required_objective_count(), 1);
        assert_eq!(def.base_duration, Some(5000));
    }

    #[test]
    fn tracker_register_and_accept() {
        let mut tracker = MissionTracker::new();
        let def = MissionDefinition::new("test", "Test").with_duration(1000);
        tracker.register(def).unwrap();

        let mission_id = tracker.accept("test").unwrap();
        assert!(tracker.active(mission_id).is_some());
        assert_eq!(tracker.summary().active_count, 1);
    }

    #[test]
    fn tracker_progress_completion() {
        let mut tracker = MissionTracker::new();
        let def = MissionDefinition::new("test", "Test")
            .with_objective(ObjectiveSpec::new(ObjectiveKind::Gather).with_target_count(10));
        tracker.register(def).unwrap();

        let mission_id = tracker.accept("test").unwrap();
        tracker.start(mission_id).unwrap();

        tracker
            .record_progress(mission_id, ObjectiveId::new(0), 5)
            .unwrap();
        assert_eq!(
            tracker
                .active(mission_id)
                .unwrap()
                .objective(ObjectiveId::new(0))
                .unwrap()
                .current_count,
            5
        );

        tracker
            .record_progress(mission_id, ObjectiveId::new(0), 5)
            .unwrap();
        let completed = tracker.evaluate(mission_id).unwrap();
        assert!(completed);
        assert!(tracker.completed(mission_id).is_some());
    }

    #[test]
    fn tracker_optional_objectives() {
        let mut tracker = MissionTracker::new();
        let def = MissionDefinition::new("test", "Test")
            .with_objective(ObjectiveSpec::new(ObjectiveKind::Gather).with_target_count(5))
            .with_objective(
                ObjectiveSpec::new(ObjectiveKind::Gather)
                    .with_target_count(10)
                    .with_optional(true),
            );
        tracker.register(def).unwrap();

        let mission_id = tracker.accept("test").unwrap();
        tracker.start(mission_id).unwrap();
        tracker
            .record_progress(mission_id, ObjectiveId::new(0), 5)
            .unwrap();

        let completed = tracker.evaluate(mission_id).unwrap();
        assert!(completed);
    }

    #[test]
    fn tracker_prerequisite_blocking() {
        let mut tracker = MissionTracker::new();
        let def = MissionDefinition::new("second", "Second").with_prerequisite("first");
        tracker.register(def).unwrap();

        assert!(!tracker.is_available("second"));

        let first = MissionDefinition::new("first", "First")
            .with_objective(ObjectiveSpec::new(ObjectiveKind::Gather).with_target_count(1));
        tracker.register(first).unwrap();

        let mission_id = tracker.accept("first").unwrap();
        tracker.start(mission_id).unwrap();
        tracker
            .record_progress(mission_id, ObjectiveId::new(0), 1)
            .unwrap();
        tracker.evaluate(mission_id).unwrap();

        assert!(tracker.is_available("second"));
    }

    #[test]
    fn tracker_deadline_expiry() {
        let mut tracker = MissionTracker::new();
        let def = MissionDefinition::new("test", "Test").with_duration(100);
        tracker.register(def).unwrap();

        let mission_id = tracker.accept("test").unwrap();
        tracker.start(mission_id).unwrap();

        let expired = tracker.tick(200);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0], mission_id);
        assert!(tracker.completed(mission_id).is_some());
        assert_eq!(
            tracker.completed(mission_id).unwrap().state,
            MissionState::Expired
        );
    }

    #[test]
    fn tracker_repeat_chain() {
        let mut tracker = MissionTracker::new();
        let def = MissionDefinition::new("test", "Test")
            .with_objective(ObjectiveSpec::new(ObjectiveKind::Gather).with_target_count(1))
            .with_repeatable(true, 1000);
        tracker.register(def).unwrap();

        let mission_id = tracker.accept("test").unwrap();
        tracker.start(mission_id).unwrap();
        tracker
            .record_progress(mission_id, ObjectiveId::new(0), 1)
            .unwrap();
        tracker.evaluate(mission_id).unwrap();

        assert!(!tracker.is_available("test"));

        tracker.tick(1001);
        assert!(tracker.is_available("test"));
    }

    #[test]
    fn tracker_region_filtering() {
        let mut tracker = MissionTracker::new();
        let def = MissionDefinition::new("test", "Test")
            .with_objective(ObjectiveSpec::new(ObjectiveKind::Gather).with_target_count(5));
        tracker.register(def).unwrap();

        let m1 = tracker.accept("test").unwrap();
        let m2 = tracker.accept("test").unwrap();

        tracker.active_mut(m1).unwrap().active_region = Some("north".to_string());
        tracker.active_mut(m2).unwrap().active_region = Some("south".to_string());

        let query = MissionQuery::new().with_region("north");
        let results = tracker.query(&query);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, m1);
    }

    #[test]
    fn tracker_fingerprint_stability() {
        let def = MissionDefinition::new("test", "Test")
            .with_objective(ObjectiveSpec::new(ObjectiveKind::Gather).with_target_count(10));
        let fp1 = def.fingerprint();
        let fp2 = def.fingerprint();
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn tracker_checksum_stability() {
        let mut tracker1 = MissionTracker::new();
        let mut tracker2 = MissionTracker::new();

        let def = MissionDefinition::new("test", "Test")
            .with_objective(ObjectiveSpec::new(ObjectiveKind::Gather).with_target_count(10));

        tracker1.register(def.clone()).unwrap();
        tracker2.register(def).unwrap();

        let m1 = tracker1.accept("test").unwrap();
        let m2 = tracker2.accept("test").unwrap();

        tracker1
            .record_progress(m1, ObjectiveId::new(0), 5)
            .unwrap();
        tracker2
            .record_progress(m2, ObjectiveId::new(0), 5)
            .unwrap();

        assert!(tracker1.checksum().matches(&tracker2.checksum()));
    }

    #[test]
    fn tracker_serde_round_trip() {
        let mut tracker = MissionTracker::new();
        let def = MissionDefinition::new("test", "Test")
            .with_objective(ObjectiveSpec::new(ObjectiveKind::Gather).with_target_count(10));
        tracker.register(def).unwrap();

        let mission_id = tracker.accept("test").unwrap();
        tracker.start(mission_id).unwrap();
        tracker
            .record_progress(mission_id, ObjectiveId::new(0), 5)
            .unwrap();

        let json = serde_json::to_string(&tracker).unwrap();
        let recovered: MissionTracker = serde_json::from_str(&json).unwrap();

        assert_eq!(recovered.summary().active_count, 1);
        assert_eq!(
            recovered
                .active(mission_id)
                .unwrap()
                .objective(ObjectiveId::new(0))
                .unwrap()
                .current_count,
            5
        );
    }

    #[test]
    fn preset_supply_run() {
        let def = MissionPreset::SupplyRun.definition();
        assert_eq!(def.id, "supply_run");
        assert_eq!(def.objectives.len(), 2);
        assert!(def.repeatable);
    }

    #[test]
    fn preset_breach_repair() {
        let def = MissionPreset::BreachRepair.definition();
        assert_eq!(def.id, "breach_repair");
        assert_eq!(def.required_objective_count(), 2);
    }

    #[test]
    fn preset_anomaly_survey() {
        let def = MissionPreset::AnomalySurvey.definition();
        assert_eq!(def.id, "anomaly_survey");
        assert_eq!(def.objectives.iter().filter(|o| o.optional).count(), 1);
    }

    #[test]
    fn preset_survivor_rescue() {
        let def = MissionPreset::SurvivorRescue.definition();
        assert_eq!(def.id, "survivor_rescue");
        assert_eq!(def.objectives.len(), 3);
    }

    #[test]
    fn preset_base_defense() {
        let def = MissionPreset::BaseDefense.definition();
        assert_eq!(def.id, "base_defense");
        assert!(
            def.objectives
                .iter()
                .any(|o| o.kind == ObjectiveKind::Defend)
        );
    }

    #[test]
    fn preset_research_expedition() {
        let def = MissionPreset::ResearchExpedition.definition();
        assert_eq!(def.id, "research_expedition");
        assert!(
            def.objectives
                .iter()
                .any(|o| o.kind == ObjectiveKind::Research)
        );
    }

    #[test]
    fn register_all_presets() {
        let mut tracker = MissionTracker::new();
        register_presets(&mut tracker);
        assert_eq!(
            tracker.summary().definition_count,
            MissionPreset::all().len()
        );
    }

    #[test]
    fn deterministic_ordering() {
        let mut tracker = MissionTracker::new();
        let def = MissionDefinition::new("test", "Test")
            .with_objective(ObjectiveSpec::new(ObjectiveKind::Gather).with_target_count(5));
        tracker.register(def).unwrap();

        for _ in 0..10 {
            tracker.accept("test").unwrap();
        }

        let ids: Vec<_> = tracker.active_missions().map(|(id, _)| *id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted);
    }

    #[test]
    fn serde_objective_spec() {
        let spec = ObjectiveSpec::new(ObjectiveKind::Gather)
            .with_resource("iron")
            .with_target_count(50)
            .with_custom("bonus", "double");

        let json = serde_json::to_string(&spec).unwrap();
        let recovered: ObjectiveSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, spec);
    }

    #[test]
    fn serde_mission_definition() {
        let def = MissionDefinition::new("test", "Test")
            .with_objective(ObjectiveSpec::new(ObjectiveKind::Gather).with_target_count(10))
            .with_duration(5000)
            .with_tag("test");

        let json = serde_json::to_string(&def).unwrap();
        let recovered: MissionDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, def);
    }
}
