//! Expedition contracts with owner/faction metadata.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::{ContractId, MissionDefinition, ObjectiveSpec};

/// Risk/difficulty level for a contract.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[repr(u8)]
pub enum RiskLevel {
    /// Minimal risk, suitable for beginners.
    #[default]
    Minimal = 0,
    /// Low risk, straightforward objectives.
    Low = 1,
    /// Moderate risk, some challenges expected.
    Moderate = 2,
    /// High risk, significant dangers.
    High = 3,
    /// Extreme risk, survival not guaranteed.
    Extreme = 4,
    /// Unknown risk, proceed with caution.
    Unknown = 5,
}

impl RiskLevel {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
            Self::Low => "low",
            Self::Moderate => "moderate",
            Self::High => "high",
            Self::Extreme => "extreme",
            Self::Unknown => "unknown",
        }
    }

    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Minimal => "Minimal Risk",
            Self::Low => "Low Risk",
            Self::Moderate => "Moderate Risk",
            Self::High => "High Risk",
            Self::Extreme => "Extreme Risk",
            Self::Unknown => "Unknown Risk",
        }
    }

    #[must_use]
    pub const fn reward_multiplier(&self) -> f32 {
        match self {
            Self::Minimal => 0.5,
            Self::Low => 1.0,
            Self::Moderate => 1.5,
            Self::High => 2.0,
            Self::Extreme => 3.0,
            Self::Unknown => 1.75,
        }
    }

    #[must_use]
    pub const fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Minimal),
            1 => Some(Self::Low),
            2 => Some(Self::Moderate),
            3 => Some(Self::High),
            4 => Some(Self::Extreme),
            5 => Some(Self::Unknown),
            _ => None,
        }
    }
}

/// Source of an expedition contract.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FactionSource {
    /// Faction identifier.
    pub faction_id: String,

    /// Display name of the faction.
    pub faction_name: String,

    /// Owner entity within the faction (e.g., specific NPC).
    pub owner: Option<String>,

    /// Reputation required to accept.
    pub required_reputation: i32,

    /// Reputation reward on completion.
    pub reputation_reward: i32,

    /// Reputation penalty on failure.
    pub reputation_penalty: i32,
}

impl FactionSource {
    /// Create a new faction source.
    #[must_use]
    pub fn new(faction_id: impl Into<String>, faction_name: impl Into<String>) -> Self {
        Self {
            faction_id: faction_id.into(),
            faction_name: faction_name.into(),
            owner: None,
            required_reputation: 0,
            reputation_reward: 10,
            reputation_penalty: 5,
        }
    }

    /// Set owner.
    #[must_use]
    pub fn with_owner(mut self, owner: impl Into<String>) -> Self {
        self.owner = Some(owner.into());
        self
    }

    /// Set required reputation.
    #[must_use]
    pub fn with_required_reputation(mut self, rep: i32) -> Self {
        self.required_reputation = rep;
        self
    }

    /// Set reputation rewards.
    #[must_use]
    pub fn with_reputation_rewards(mut self, reward: i32, penalty: i32) -> Self {
        self.reputation_reward = reward;
        self.reputation_penalty = penalty;
        self
    }
}

/// Deadline configuration for contracts.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeadlineConfig {
    /// Base deadline in ticks from acceptance.
    pub base_ticks: u64,

    /// Per-objective additional time allowance.
    pub per_objective_ticks: u64,

    /// Grace period after deadline before failure.
    pub grace_period: u64,

    /// Whether deadline can be extended.
    pub extendable: bool,

    /// Maximum extensions allowed.
    pub max_extensions: u32,

    /// Cost per extension (resource type and amount).
    pub extension_cost: Option<(String, u32)>,
}

impl DeadlineConfig {
    /// Create a fixed deadline.
    #[must_use]
    pub fn fixed(ticks: u64) -> Self {
        Self {
            base_ticks: ticks,
            per_objective_ticks: 0,
            grace_period: 0,
            extendable: false,
            max_extensions: 0,
            extension_cost: None,
        }
    }

    /// Create a deadline with per-objective time.
    #[must_use]
    pub fn per_objective(base: u64, per_objective: u64) -> Self {
        Self {
            base_ticks: base,
            per_objective_ticks: per_objective,
            grace_period: 0,
            extendable: false,
            max_extensions: 0,
            extension_cost: None,
        }
    }

    /// Set grace period.
    #[must_use]
    pub fn with_grace_period(mut self, ticks: u64) -> Self {
        self.grace_period = ticks;
        self
    }

    /// Make extendable.
    #[must_use]
    pub fn with_extensions(mut self, max: u32, cost: Option<(String, u32)>) -> Self {
        self.extendable = true;
        self.max_extensions = max;
        self.extension_cost = cost;
        self
    }

    /// Calculate total deadline for given objective count.
    #[must_use]
    pub fn total_ticks(&self, objective_count: u32) -> u64 {
        self.base_ticks + (self.per_objective_ticks * u64::from(objective_count))
    }
}

impl Default for DeadlineConfig {
    fn default() -> Self {
        Self::fixed(18000)
    }
}

/// Region/chunk scope configuration.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScopeConfig {
    /// Region identifiers where this contract is valid.
    pub regions: Vec<String>,

    /// Chunk position bounds (`min_x`, `min_z`, `max_x`, `max_z`).
    pub chunk_bounds: Option<(i32, i32, i32, i32)>,

    /// Whether contract is global (all regions).
    pub global: bool,

    /// Required biomes.
    pub required_biomes: Vec<String>,
}

impl ScopeConfig {
    /// Create a global scope.
    #[must_use]
    pub fn global() -> Self {
        Self {
            regions: Vec::new(),
            chunk_bounds: None,
            global: true,
            required_biomes: Vec::new(),
        }
    }

    /// Create a regional scope.
    #[must_use]
    pub fn regional(regions: Vec<String>) -> Self {
        Self {
            regions,
            chunk_bounds: None,
            global: false,
            required_biomes: Vec::new(),
        }
    }

    /// Set chunk bounds.
    #[must_use]
    pub fn with_chunk_bounds(mut self, min_x: i32, min_z: i32, max_x: i32, max_z: i32) -> Self {
        self.chunk_bounds = Some((min_x, min_z, max_x, max_z));
        self
    }

    /// Add required biome.
    #[must_use]
    pub fn with_biome(mut self, biome: impl Into<String>) -> Self {
        self.required_biomes.push(biome.into());
        self
    }

    /// Check if a region is in scope.
    #[must_use]
    pub fn contains_region(&self, region: &str) -> bool {
        self.global || self.regions.iter().any(|r| r == region)
    }

    /// Check if a chunk is in scope.
    #[must_use]
    pub fn contains_chunk(&self, x: i32, z: i32) -> bool {
        if let Some((min_x, min_z, max_x, max_z)) = self.chunk_bounds {
            x >= min_x && x <= max_x && z >= min_z && z <= max_z
        } else {
            true
        }
    }
}

impl Default for ScopeConfig {
    fn default() -> Self {
        Self::global()
    }
}

/// Repeat/chain behavior configuration.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RepeatConfig {
    /// Whether this contract can repeat.
    pub repeatable: bool,

    /// Cooldown between repeats in ticks.
    pub cooldown_ticks: u64,

    /// Maximum repeat count (None = infinite).
    pub max_repeats: Option<u32>,

    /// Contract ID to chain to on completion.
    pub chain_to: Option<String>,

    /// Whether chain is mandatory.
    pub chain_mandatory: bool,
}

impl RepeatConfig {
    /// Create a one-time contract.
    #[must_use]
    pub fn once() -> Self {
        Self {
            repeatable: false,
            cooldown_ticks: 0,
            max_repeats: Some(0),
            chain_to: None,
            chain_mandatory: false,
        }
    }

    /// Create a repeatable contract.
    #[must_use]
    pub fn repeating(cooldown: u64) -> Self {
        Self {
            repeatable: true,
            cooldown_ticks: cooldown,
            max_repeats: None,
            chain_to: None,
            chain_mandatory: false,
        }
    }

    /// Create a limited-repeat contract.
    #[must_use]
    pub fn limited(max: u32, cooldown: u64) -> Self {
        Self {
            repeatable: true,
            cooldown_ticks: cooldown,
            max_repeats: Some(max),
            chain_to: None,
            chain_mandatory: false,
        }
    }

    /// Set chain contract.
    #[must_use]
    pub fn with_chain(mut self, contract_id: impl Into<String>, mandatory: bool) -> Self {
        self.chain_to = Some(contract_id.into());
        self.chain_mandatory = mandatory;
        self
    }
}

impl Default for RepeatConfig {
    fn default() -> Self {
        Self::once()
    }
}

/// Reward definition for contract completion.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RewardDefinition {
    /// Base resource rewards (`resource_id` -> amount).
    pub resources: BTreeMap<String, u32>,

    /// Base currency reward.
    pub currency: u32,

    /// Experience points reward.
    pub experience: u32,

    /// Bonus multiplier for completing all optional objectives.
    pub optional_bonus_multiplier: f32,

    /// Bonus multiplier for early completion.
    pub early_completion_multiplier: f32,

    /// Time threshold (% remaining) for early completion bonus.
    pub early_threshold: f32,

    /// Special item rewards (`item_id` -> count).
    pub items: BTreeMap<String, u32>,

    /// Unlocks (flags, missions, etc.).
    pub unlocks: Vec<String>,
}

impl RewardDefinition {
    /// Create an empty reward definition.
    #[must_use]
    pub fn new() -> Self {
        Self {
            resources: BTreeMap::new(),
            currency: 0,
            experience: 0,
            optional_bonus_multiplier: 1.25,
            early_completion_multiplier: 1.5,
            early_threshold: 0.5,
            items: BTreeMap::new(),
            unlocks: Vec::new(),
        }
    }

    /// Add resource reward.
    #[must_use]
    pub fn with_resource(mut self, resource: impl Into<String>, amount: u32) -> Self {
        self.resources.insert(resource.into(), amount);
        self
    }

    /// Set currency reward.
    #[must_use]
    pub fn with_currency(mut self, amount: u32) -> Self {
        self.currency = amount;
        self
    }

    /// Set experience reward.
    #[must_use]
    pub fn with_experience(mut self, xp: u32) -> Self {
        self.experience = xp;
        self
    }

    /// Set optional completion bonus.
    #[must_use]
    pub fn with_optional_bonus(mut self, multiplier: f32) -> Self {
        self.optional_bonus_multiplier = multiplier;
        self
    }

    /// Set early completion bonus.
    #[must_use]
    pub fn with_early_bonus(mut self, multiplier: f32, threshold: f32) -> Self {
        self.early_completion_multiplier = multiplier;
        self.early_threshold = threshold;
        self
    }

    /// Add item reward.
    #[must_use]
    pub fn with_item(mut self, item: impl Into<String>, count: u32) -> Self {
        self.items.insert(item.into(), count);
        self
    }

    /// Add unlock.
    #[must_use]
    pub fn with_unlock(mut self, unlock: impl Into<String>) -> Self {
        self.unlocks.push(unlock.into());
        self
    }
}

impl Default for RewardDefinition {
    fn default() -> Self {
        Self::new()
    }
}

/// Penalty definition for contract failure.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PenaltyDefinition {
    /// Reputation penalty.
    pub reputation: i32,

    /// Currency penalty.
    pub currency: u32,

    /// Cooldown before retry in ticks.
    pub retry_cooldown: u64,

    /// Locks (flags that get cleared).
    pub locks: Vec<String>,
}

/// Multi-objective expedition contract with full metadata.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExpeditionContract {
    /// Unique contract identifier.
    pub id: ContractId,

    /// Contract type identifier (references `MissionDefinition`).
    pub definition_id: String,

    /// Human-readable title.
    pub title: String,

    /// Detailed briefing text.
    pub briefing: String,

    /// Source faction/owner.
    pub source: Option<FactionSource>,

    /// Required objectives (must all be completed).
    pub required_objectives: Vec<ObjectiveSpec>,

    /// Optional objectives (bonus rewards).
    pub optional_objectives: Vec<ObjectiveSpec>,

    /// Deadline configuration.
    pub deadline: DeadlineConfig,

    /// Scope configuration.
    pub scope: ScopeConfig,

    /// Repeat/chain configuration.
    pub repeat: RepeatConfig,

    /// Reward on completion.
    pub reward: RewardDefinition,

    /// Penalty on failure.
    pub penalty: PenaltyDefinition,

    /// Risk level.
    pub risk: RiskLevel,

    /// Tags for filtering.
    pub tags: Vec<String>,

    /// Whether contract is currently available.
    pub available: bool,

    /// Tick when contract was generated.
    pub generated_at: u64,

    /// Tick when contract expires (no longer available).
    pub expires_at: Option<u64>,

    /// Custom data.
    pub custom_data: BTreeMap<String, String>,
}

impl ExpeditionContract {
    /// Create a new expedition contract.
    #[must_use]
    pub fn new(id: ContractId, definition_id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id,
            definition_id: definition_id.into(),
            title: title.into(),
            briefing: String::new(),
            source: None,
            required_objectives: Vec::new(),
            optional_objectives: Vec::new(),
            deadline: DeadlineConfig::default(),
            scope: ScopeConfig::default(),
            repeat: RepeatConfig::default(),
            reward: RewardDefinition::default(),
            penalty: PenaltyDefinition::default(),
            risk: RiskLevel::default(),
            tags: Vec::new(),
            available: true,
            generated_at: 0,
            expires_at: None,
            custom_data: BTreeMap::new(),
        }
    }

    /// Create from a mission definition.
    #[must_use]
    pub fn from_definition(id: ContractId, def: &MissionDefinition, generated_at: u64) -> Self {
        let mut contract = Self::new(id, &def.id, &def.display_name)
            .with_briefing(&def.description)
            .at_tick(generated_at);

        for obj in &def.objectives {
            if obj.optional {
                contract.optional_objectives.push(obj.clone());
            } else {
                contract.required_objectives.push(obj.clone());
            }
        }

        if let Some(duration) = def.base_duration {
            contract.deadline = DeadlineConfig::fixed(duration);
        }

        for tag in &def.tags {
            contract.tags.push(tag.clone());
        }

        contract
    }

    /// Set briefing.
    #[must_use]
    pub fn with_briefing(mut self, briefing: impl Into<String>) -> Self {
        self.briefing = briefing.into();
        self
    }

    /// Set source faction.
    #[must_use]
    pub fn with_source(mut self, source: FactionSource) -> Self {
        self.source = Some(source);
        self
    }

    /// Add required objective.
    #[must_use]
    pub fn with_required_objective(mut self, objective: ObjectiveSpec) -> Self {
        self.required_objectives.push(objective);
        self
    }

    /// Add optional objective.
    #[must_use]
    pub fn with_optional_objective(mut self, objective: ObjectiveSpec) -> Self {
        self.optional_objectives.push(objective);
        self
    }

    /// Set deadline config.
    #[must_use]
    pub fn with_deadline(mut self, deadline: DeadlineConfig) -> Self {
        self.deadline = deadline;
        self
    }

    /// Set scope config.
    #[must_use]
    pub fn with_scope(mut self, scope: ScopeConfig) -> Self {
        self.scope = scope;
        self
    }

    /// Set repeat config.
    #[must_use]
    pub fn with_repeat(mut self, repeat: RepeatConfig) -> Self {
        self.repeat = repeat;
        self
    }

    /// Set reward.
    #[must_use]
    pub fn with_reward(mut self, reward: RewardDefinition) -> Self {
        self.reward = reward;
        self
    }

    /// Set penalty.
    #[must_use]
    pub fn with_penalty(mut self, penalty: PenaltyDefinition) -> Self {
        self.penalty = penalty;
        self
    }

    /// Set risk level.
    #[must_use]
    pub fn with_risk(mut self, risk: RiskLevel) -> Self {
        self.risk = risk;
        self
    }

    /// Add tag.
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set generation tick.
    #[must_use]
    pub fn at_tick(mut self, tick: u64) -> Self {
        self.generated_at = tick;
        self
    }

    /// Set expiration.
    #[must_use]
    pub fn expires_at_tick(mut self, tick: u64) -> Self {
        self.expires_at = Some(tick);
        self
    }

    /// Add custom data.
    #[must_use]
    pub fn with_custom(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom_data.insert(key.into(), value.into());
        self
    }

    /// Check if contract has a tag.
    #[must_use]
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Total objective count.
    #[must_use]
    pub fn total_objective_count(&self) -> usize {
        self.required_objectives.len() + self.optional_objectives.len()
    }

    /// Check if contract is expired.
    #[must_use]
    pub fn is_expired(&self, tick: u64) -> bool {
        self.expires_at.is_some_and(|exp| tick >= exp)
    }

    /// Check if contract is available.
    #[must_use]
    pub fn is_available(&self, tick: u64) -> bool {
        self.available && !self.is_expired(tick)
    }

    /// Calculate deadline tick from start.
    #[must_use]
    pub fn deadline_tick(&self, start_tick: u64) -> u64 {
        #[allow(clippy::cast_possible_truncation)]
        let obj_count = self.total_objective_count() as u32;
        start_tick + self.deadline.total_ticks(obj_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mission::ObjectiveKind;

    #[test]
    fn risk_level_properties() {
        assert_eq!(RiskLevel::Minimal.name(), "minimal");
        assert!((RiskLevel::High.reward_multiplier() - 2.0).abs() < f32::EPSILON);
        assert_eq!(RiskLevel::from_raw(3), Some(RiskLevel::High));
        assert_eq!(RiskLevel::from_raw(10), None);
    }

    #[test]
    fn faction_source_creation() {
        let source = FactionSource::new("traders_guild", "Traders Guild")
            .with_owner("merchant_king")
            .with_required_reputation(50)
            .with_reputation_rewards(25, 10);

        assert_eq!(source.faction_id, "traders_guild");
        assert_eq!(source.owner, Some("merchant_king".to_string()));
        assert_eq!(source.required_reputation, 50);
        assert_eq!(source.reputation_reward, 25);
    }

    #[test]
    fn deadline_config_calculation() {
        let deadline = DeadlineConfig::per_objective(1000, 500);
        assert_eq!(deadline.total_ticks(3), 2500);

        let with_grace = deadline.with_grace_period(100);
        assert_eq!(with_grace.grace_period, 100);
    }

    #[test]
    fn scope_config_regions() {
        let scope = ScopeConfig::regional(vec!["north".into(), "south".into()]);
        assert!(scope.contains_region("north"));
        assert!(!scope.contains_region("east"));

        let global = ScopeConfig::global();
        assert!(global.contains_region("anywhere"));
    }

    #[test]
    fn scope_config_chunks() {
        let scope = ScopeConfig::global().with_chunk_bounds(-10, -10, 10, 10);
        assert!(scope.contains_chunk(0, 0));
        assert!(scope.contains_chunk(-10, 10));
        assert!(!scope.contains_chunk(11, 0));
    }

    #[test]
    fn repeat_config_types() {
        let once = RepeatConfig::once();
        assert!(!once.repeatable);

        let repeating = RepeatConfig::repeating(1000);
        assert!(repeating.repeatable);
        assert!(repeating.max_repeats.is_none());

        let limited = RepeatConfig::limited(3, 500);
        assert_eq!(limited.max_repeats, Some(3));
    }

    #[test]
    fn repeat_config_chain() {
        let chained = RepeatConfig::once().with_chain("next_mission", true);
        assert_eq!(chained.chain_to, Some("next_mission".to_string()));
        assert!(chained.chain_mandatory);
    }

    #[test]
    fn reward_definition_builders() {
        let reward = RewardDefinition::new()
            .with_resource("gold", 100)
            .with_currency(500)
            .with_experience(1000)
            .with_item("rare_artifact", 1)
            .with_unlock("advanced_missions");

        assert_eq!(reward.resources.get("gold"), Some(&100));
        assert_eq!(reward.currency, 500);
        assert_eq!(reward.experience, 1000);
        assert_eq!(reward.items.get("rare_artifact"), Some(&1));
        assert_eq!(reward.unlocks.len(), 1);
    }

    #[test]
    fn expedition_contract_creation() {
        let contract = ExpeditionContract::new(ContractId::new(1), "supply_run", "Supply Run")
            .with_briefing("Deliver supplies to the outpost")
            .with_required_objective(
                ObjectiveSpec::new(ObjectiveKind::Gather)
                    .with_resource("supplies")
                    .with_target_count(10),
            )
            .with_optional_objective(ObjectiveSpec::new(ObjectiveKind::Explore).with_optional(true))
            .with_risk(RiskLevel::Moderate)
            .with_tag("logistics");

        assert_eq!(contract.id, ContractId::new(1));
        assert_eq!(contract.title, "Supply Run");
        assert_eq!(contract.required_objectives.len(), 1);
        assert_eq!(contract.optional_objectives.len(), 1);
        assert_eq!(contract.total_objective_count(), 2);
        assert_eq!(contract.risk, RiskLevel::Moderate);
        assert!(contract.has_tag("logistics"));
    }

    #[test]
    fn expedition_contract_expiration() {
        let contract = ExpeditionContract::new(ContractId::new(1), "test", "Test")
            .at_tick(100)
            .expires_at_tick(500);

        assert!(contract.is_available(100));
        assert!(contract.is_available(499));
        assert!(!contract.is_available(500));
        assert!(contract.is_expired(500));
    }

    #[test]
    fn expedition_contract_deadline() {
        let contract = ExpeditionContract::new(ContractId::new(1), "test", "Test")
            .with_deadline(DeadlineConfig::per_objective(1000, 200))
            .with_required_objective(ObjectiveSpec::new(ObjectiveKind::Gather))
            .with_required_objective(ObjectiveSpec::new(ObjectiveKind::Deliver));

        assert_eq!(contract.deadline_tick(100), 1500);
    }

    #[test]
    fn serde_risk_level() {
        let risk = RiskLevel::Extreme;
        let json = serde_json::to_string(&risk).unwrap();
        let recovered: RiskLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, risk);
    }

    #[test]
    fn serde_expedition_contract() {
        let contract = ExpeditionContract::new(ContractId::new(42), "test", "Test Contract")
            .with_required_objective(ObjectiveSpec::new(ObjectiveKind::Gather))
            .with_risk(RiskLevel::High)
            .with_source(FactionSource::new("guild", "The Guild"));

        let json = serde_json::to_string(&contract).unwrap();
        let recovered: ExpeditionContract = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, contract);
    }
}
