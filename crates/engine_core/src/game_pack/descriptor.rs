//! Descriptor types for game pack content.
//!
//! Descriptors are declarative definitions that don't contain callbacks.
//! They describe what content should exist and how it should behave.

use serde::{Deserialize, Serialize};

use super::id::{BlockId, HazardId, RuleProfileId, ShaderId, SystemId};

/// Execution phase for system hooks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SystemPhase {
    PreInit,
    Init,
    #[default]
    Update,
    LateUpdate,
    PreRender,
    Render,
    PostRender,
    Shutdown,
}

impl SystemPhase {
    #[must_use]
    pub const fn order(self) -> i32 {
        match self {
            Self::PreInit => 0,
            Self::Init => 100,
            Self::Update => 200,
            Self::LateUpdate => 300,
            Self::PreRender => 400,
            Self::Render => 500,
            Self::PostRender => 600,
            Self::Shutdown => 700,
        }
    }
}

/// Descriptor for a custom block type.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockDescriptor {
    pub id: BlockId,
    pub name: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub solid: bool,
    #[serde(default)]
    pub transparent: bool,
    #[serde(default)]
    pub light_emission: u8,
    #[serde(default)]
    pub hardness: f32,
    #[serde(default)]
    pub texture_id: Option<String>,
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl BlockDescriptor {
    #[must_use]
    pub fn new(id: BlockId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            display_name: None,
            solid: true,
            transparent: false,
            light_emission: 0,
            hardness: 1.0,
            texture_id: None,
            model_id: None,
            tags: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self
    }

    #[must_use]
    pub fn transparent(mut self) -> Self {
        self.transparent = true;
        self
    }

    #[must_use]
    pub fn non_solid(mut self) -> Self {
        self.solid = false;
        self
    }

    #[must_use]
    pub fn with_light(mut self, emission: u8) -> Self {
        self.light_emission = emission;
        self
    }

    #[must_use]
    pub fn with_hardness(mut self, hardness: f32) -> Self {
        self.hardness = hardness;
        self
    }

    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

/// Descriptor for a system hook.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemDescriptor {
    pub id: SystemId,
    pub name: String,
    #[serde(default)]
    pub phase: SystemPhase,
    #[serde(default)]
    pub order: i32,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub run_after: Vec<String>,
    #[serde(default)]
    pub run_before: Vec<String>,
    #[serde(default)]
    pub label: Option<String>,
}

impl SystemDescriptor {
    #[must_use]
    pub fn new(id: SystemId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            phase: SystemPhase::default(),
            order: 0,
            enabled: true,
            run_after: Vec::new(),
            run_before: Vec::new(),
            label: None,
        }
    }

    #[must_use]
    pub fn with_phase(mut self, phase: SystemPhase) -> Self {
        self.phase = phase;
        self
    }

    #[must_use]
    pub fn with_order(mut self, order: i32) -> Self {
        self.order = order;
        self
    }

    #[must_use]
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    #[must_use]
    pub fn after(mut self, system: impl Into<String>) -> Self {
        self.run_after.push(system.into());
        self
    }

    #[must_use]
    pub fn before(mut self, system: impl Into<String>) -> Self {
        self.run_before.push(system.into());
        self
    }

    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    #[must_use]
    pub fn effective_order(&self) -> i32 {
        self.phase.order() + self.order
    }
}

/// Severity level for hazards.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HazardSeverity {
    #[default]
    Minor,
    Moderate,
    Severe,
    Lethal,
}

/// Descriptor for a hazard type.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HazardDescriptor {
    pub id: HazardId,
    pub name: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub severity: HazardSeverity,
    #[serde(default)]
    pub damage_per_second: f32,
    #[serde(default)]
    pub effect_radius: f32,
    #[serde(default)]
    pub visual_effect: Option<String>,
    #[serde(default)]
    pub sound_effect: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl HazardDescriptor {
    #[must_use]
    pub fn new(id: HazardId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            display_name: None,
            severity: HazardSeverity::default(),
            damage_per_second: 0.0,
            effect_radius: 1.0,
            visual_effect: None,
            sound_effect: None,
            tags: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_severity(mut self, severity: HazardSeverity) -> Self {
        self.severity = severity;
        self
    }

    #[must_use]
    pub fn with_damage(mut self, dps: f32) -> Self {
        self.damage_per_second = dps;
        self
    }

    #[must_use]
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.effect_radius = radius;
        self
    }

    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

/// Shader stage type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShaderStage {
    #[default]
    Fragment,
    Vertex,
    Compute,
}

/// Descriptor for a shader reference.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShaderDescriptor {
    pub id: ShaderId,
    pub name: String,
    #[serde(default)]
    pub stage: ShaderStage,
    #[serde(default)]
    pub source_path: Option<String>,
    #[serde(default)]
    pub entry_point: String,
    #[serde(default)]
    pub defines: Vec<(String, String)>,
}

impl ShaderDescriptor {
    #[must_use]
    pub fn new(id: ShaderId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            stage: ShaderStage::default(),
            source_path: None,
            entry_point: "main".to_string(),
            defines: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_stage(mut self, stage: ShaderStage) -> Self {
        self.stage = stage;
        self
    }

    #[must_use]
    pub fn with_source(mut self, path: impl Into<String>) -> Self {
        self.source_path = Some(path.into());
        self
    }

    #[must_use]
    pub fn with_entry_point(mut self, entry: impl Into<String>) -> Self {
        self.entry_point = entry.into();
        self
    }

    #[must_use]
    pub fn with_define(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.defines.push((key.into(), value.into()));
        self
    }
}

/// Descriptor for a world rule profile.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuleProfileDescriptor {
    pub id: RuleProfileId,
    pub name: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub rules: Vec<(String, RuleValue)>,
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default)]
    pub exclusive: bool,
}

/// Value types for rule settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RuleValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

impl RuleProfileDescriptor {
    #[must_use]
    pub fn new(id: RuleProfileId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            display_name: None,
            rules: Vec::new(),
            parent: None,
            exclusive: false,
        }
    }

    #[must_use]
    pub fn with_rule_bool(mut self, key: impl Into<String>, value: bool) -> Self {
        self.rules.push((key.into(), RuleValue::Bool(value)));
        self
    }

    #[must_use]
    pub fn with_rule_int(mut self, key: impl Into<String>, value: i64) -> Self {
        self.rules.push((key.into(), RuleValue::Int(value)));
        self
    }

    #[must_use]
    pub fn with_rule_float(mut self, key: impl Into<String>, value: f64) -> Self {
        self.rules.push((key.into(), RuleValue::Float(value)));
        self
    }

    #[must_use]
    pub fn with_rule_string(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.rules
            .push((key.into(), RuleValue::String(value.into())));
        self
    }

    #[must_use]
    pub fn with_parent(mut self, parent: impl Into<String>) -> Self {
        self.parent = Some(parent.into());
        self
    }

    #[must_use]
    pub fn exclusive(mut self) -> Self {
        self.exclusive = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_phase_ordering() {
        assert!(SystemPhase::PreInit.order() < SystemPhase::Init.order());
        assert!(SystemPhase::Init.order() < SystemPhase::Update.order());
        assert!(SystemPhase::Update.order() < SystemPhase::Render.order());
        assert!(SystemPhase::Render.order() < SystemPhase::Shutdown.order());
    }

    #[test]
    fn block_descriptor_builder() {
        let block = BlockDescriptor::new(BlockId::new(1, 1), "test_block")
            .with_display_name("Test Block")
            .transparent()
            .with_light(15)
            .with_tag("luminous");

        assert_eq!(block.name, "test_block");
        assert!(block.transparent);
        assert_eq!(block.light_emission, 15);
        assert!(block.tags.contains(&"luminous".to_string()));
    }

    #[test]
    fn system_descriptor_effective_order() {
        let system = SystemDescriptor::new(SystemId::new(1, 1), "test_system")
            .with_phase(SystemPhase::Update)
            .with_order(10);

        assert_eq!(system.effective_order(), 210);
    }

    #[test]
    fn hazard_descriptor_builder() {
        let hazard = HazardDescriptor::new(HazardId::new(1, 1), "fire")
            .with_severity(HazardSeverity::Severe)
            .with_damage(25.0)
            .with_radius(3.0);

        assert_eq!(hazard.severity, HazardSeverity::Severe);
        assert!((hazard.damage_per_second - 25.0).abs() < f32::EPSILON);
    }

    #[test]
    fn rule_profile_builder() {
        let profile = RuleProfileDescriptor::new(RuleProfileId::new(1, 1), "hardcore")
            .with_rule_bool("pvp_enabled", true)
            .with_rule_float("damage_multiplier", 2.0)
            .exclusive();

        assert!(profile.exclusive);
        assert_eq!(profile.rules.len(), 2);
    }

    #[test]
    fn descriptor_serde_roundtrip() {
        let block = BlockDescriptor::new(BlockId::new(1, 2), "stone")
            .with_hardness(5.0)
            .with_tag("natural");

        let json = serde_json::to_string(&block).unwrap();
        let restored: BlockDescriptor = serde_json::from_str(&json).unwrap();

        assert_eq!(block.id, restored.id);
        assert_eq!(block.name, restored.name);
        assert_eq!(block.tags, restored.tags);
    }

    #[test]
    fn descriptor_bincode_roundtrip() {
        let system = SystemDescriptor::new(SystemId::new(1, 3), "physics")
            .with_phase(SystemPhase::LateUpdate)
            .after("movement");

        let bytes = bincode::serialize(&system).unwrap();
        let restored: SystemDescriptor = bincode::deserialize(&bytes).unwrap();

        assert_eq!(system.id, restored.id);
        assert_eq!(system.phase, restored.phase);
        assert_eq!(system.run_after, restored.run_after);
    }
}
