//! Settler capabilities and skills.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::ids::CapabilityId;

/// Skill level for a capability.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum SkillLevel {
    #[default]
    Novice,
    Apprentice,
    Journeyman,
    Expert,
    Master,
}

impl SkillLevel {
    #[must_use]
    pub fn speed_multiplier(self) -> f32 {
        match self {
            Self::Novice => 0.5,
            Self::Apprentice => 0.75,
            Self::Journeyman => 1.0,
            Self::Expert => 1.25,
            Self::Master => 1.5,
        }
    }

    #[must_use]
    pub fn quality_bonus(self) -> f32 {
        match self {
            Self::Novice => 0.0,
            Self::Apprentice => 0.1,
            Self::Journeyman => 0.2,
            Self::Expert => 0.35,
            Self::Master => 0.5,
        }
    }

    #[must_use]
    pub fn next(self) -> Option<Self> {
        match self {
            Self::Novice => Some(Self::Apprentice),
            Self::Apprentice => Some(Self::Journeyman),
            Self::Journeyman => Some(Self::Expert),
            Self::Expert => Some(Self::Master),
            Self::Master => None,
        }
    }
}

/// A capability definition with metadata.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CapabilityDef {
    pub id: CapabilityId,
    pub name: String,
    pub description: Option<String>,
    pub category: CapabilityCategory,
    pub base_experience_rate: f32,
}

impl CapabilityDef {
    #[must_use]
    pub fn new(id: CapabilityId, name: impl Into<String>, category: CapabilityCategory) -> Self {
        Self {
            id,
            name: name.into(),
            description: None,
            category,
            base_experience_rate: 1.0,
        }
    }

    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    #[must_use]
    pub fn with_experience_rate(mut self, rate: f32) -> Self {
        self.base_experience_rate = rate;
        self
    }
}

/// Category for grouping capabilities.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CapabilityCategory {
    Labor,
    Crafting,
    Combat,
    Social,
    Research,
    Medical,
    Farming,
}

/// A settler's skill in a particular capability.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Skill {
    pub capability: CapabilityId,
    pub level: SkillLevel,
    pub experience: f32,
    pub enabled: bool,
}

impl Skill {
    #[must_use]
    pub fn new(capability: CapabilityId) -> Self {
        Self {
            capability,
            level: SkillLevel::Novice,
            experience: 0.0,
            enabled: true,
        }
    }

    #[must_use]
    pub fn with_level(mut self, level: SkillLevel) -> Self {
        self.level = level;
        self
    }

    #[must_use]
    pub fn effective_level(&self) -> Option<SkillLevel> {
        if self.enabled { Some(self.level) } else { None }
    }

    pub fn add_experience(&mut self, amount: f32) -> bool {
        self.experience += amount;
        let threshold = self.level_up_threshold();
        if self.experience >= threshold
            && let Some(next) = self.level.next()
        {
            self.level = next;
            self.experience -= threshold;
            return true;
        }
        false
    }

    #[must_use]
    pub fn level_up_threshold(&self) -> f32 {
        match self.level {
            SkillLevel::Novice => 100.0,
            SkillLevel::Apprentice => 250.0,
            SkillLevel::Journeyman => 500.0,
            SkillLevel::Expert => 1000.0,
            SkillLevel::Master => f32::INFINITY,
        }
    }
}

/// A set of skills for a settler.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SkillSet {
    skills: BTreeMap<CapabilityId, Skill>,
}

impl SkillSet {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_skill(&mut self, skill: Skill) {
        self.skills.insert(skill.capability.clone(), skill);
    }

    pub fn remove_skill(&mut self, capability: &CapabilityId) -> Option<Skill> {
        self.skills.remove(capability)
    }

    #[must_use]
    pub fn get_skill(&self, capability: &CapabilityId) -> Option<&Skill> {
        self.skills.get(capability)
    }

    pub fn get_skill_mut(&mut self, capability: &CapabilityId) -> Option<&mut Skill> {
        self.skills.get_mut(capability)
    }

    #[must_use]
    pub fn has_capability(&self, capability: &CapabilityId) -> bool {
        self.skills.get(capability).is_some_and(|s| s.enabled)
    }

    #[must_use]
    pub fn effective_level(&self, capability: &CapabilityId) -> Option<SkillLevel> {
        self.skills.get(capability).and_then(Skill::effective_level)
    }

    #[must_use]
    pub fn can_perform(&self, required: &[CapabilityId]) -> bool {
        required.iter().all(|cap| self.has_capability(cap))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&CapabilityId, &Skill)> {
        self.skills.iter()
    }

    pub fn enabled_skills(&self) -> impl Iterator<Item = &Skill> {
        self.skills.values().filter(|s| s.enabled)
    }

    #[must_use]
    pub fn skill_count(&self) -> usize {
        self.skills.len()
    }

    #[must_use]
    pub fn enabled_count(&self) -> usize {
        self.skills.values().filter(|s| s.enabled).count()
    }
}

/// Well-known capability IDs.
pub mod presets {
    use super::{CapabilityCategory, CapabilityDef, CapabilityId};

    pub fn mining() -> CapabilityId {
        CapabilityId::new("mining")
    }

    pub fn hauling() -> CapabilityId {
        CapabilityId::new("hauling")
    }

    pub fn construction() -> CapabilityId {
        CapabilityId::new("construction")
    }

    pub fn farming() -> CapabilityId {
        CapabilityId::new("farming")
    }

    pub fn crafting() -> CapabilityId {
        CapabilityId::new("crafting")
    }

    pub fn cooking() -> CapabilityId {
        CapabilityId::new("cooking")
    }

    pub fn medical() -> CapabilityId {
        CapabilityId::new("medical")
    }

    pub fn research() -> CapabilityId {
        CapabilityId::new("research")
    }

    pub fn combat() -> CapabilityId {
        CapabilityId::new("combat")
    }

    pub fn cleaning() -> CapabilityId {
        CapabilityId::new("cleaning")
    }

    pub fn standard_capability_defs() -> Vec<CapabilityDef> {
        vec![
            CapabilityDef::new(mining(), "Mining", CapabilityCategory::Labor),
            CapabilityDef::new(hauling(), "Hauling", CapabilityCategory::Labor),
            CapabilityDef::new(construction(), "Construction", CapabilityCategory::Crafting),
            CapabilityDef::new(farming(), "Farming", CapabilityCategory::Farming),
            CapabilityDef::new(crafting(), "Crafting", CapabilityCategory::Crafting),
            CapabilityDef::new(cooking(), "Cooking", CapabilityCategory::Crafting),
            CapabilityDef::new(medical(), "Medical", CapabilityCategory::Medical),
            CapabilityDef::new(research(), "Research", CapabilityCategory::Research),
            CapabilityDef::new(combat(), "Combat", CapabilityCategory::Combat),
            CapabilityDef::new(cleaning(), "Cleaning", CapabilityCategory::Labor),
        ]
    }
}

#[cfg(test)]
#[allow(clippy::wildcard_imports)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_level_progression() {
        assert_eq!(SkillLevel::Novice.next(), Some(SkillLevel::Apprentice));
        assert_eq!(SkillLevel::Master.next(), None);
    }

    #[test]
    fn test_skill_level_multipliers() {
        assert!(SkillLevel::Novice.speed_multiplier() < SkillLevel::Master.speed_multiplier());
        assert!(SkillLevel::Novice.quality_bonus() < SkillLevel::Master.quality_bonus());
    }

    #[test]
    fn test_skill_experience() {
        let mut skill = Skill::new(presets::mining());
        assert_eq!(skill.level, SkillLevel::Novice);

        let leveled = skill.add_experience(100.0);
        assert!(leveled);
        assert_eq!(skill.level, SkillLevel::Apprentice);
    }

    #[test]
    fn test_skill_disabled() {
        let mut skill = Skill::new(presets::hauling());
        skill.enabled = false;
        assert!(skill.effective_level().is_none());
    }

    #[test]
    fn test_skill_set_capabilities() {
        let mut skills = SkillSet::new();
        skills.add_skill(Skill::new(presets::mining()));
        skills.add_skill(Skill::new(presets::hauling()));

        assert!(skills.has_capability(&presets::mining()));
        assert!(skills.has_capability(&presets::hauling()));
        assert!(!skills.has_capability(&presets::cooking()));
    }

    #[test]
    fn test_skill_set_can_perform() {
        let mut skills = SkillSet::new();
        skills.add_skill(Skill::new(presets::mining()));
        skills.add_skill(Skill::new(presets::hauling()));

        assert!(skills.can_perform(&[presets::mining()]));
        assert!(skills.can_perform(&[presets::mining(), presets::hauling()]));
        assert!(!skills.can_perform(&[presets::mining(), presets::cooking()]));
    }

    #[test]
    fn test_skill_set_serde() {
        let mut skills = SkillSet::new();
        skills.add_skill(Skill::new(presets::mining()).with_level(SkillLevel::Expert));

        let json = serde_json::to_string(&skills).unwrap();
        let restored: SkillSet = serde_json::from_str(&json).unwrap();

        assert_eq!(
            restored.effective_level(&presets::mining()),
            Some(SkillLevel::Expert)
        );
    }

    #[test]
    fn test_capability_def() {
        let def = CapabilityDef::new(presets::mining(), "Mining", CapabilityCategory::Labor)
            .with_description("Extract resources from rock")
            .with_experience_rate(1.5);

        assert_eq!(def.name, "Mining");
        assert!(def.description.is_some());
        assert!((def.base_experience_rate - 1.5).abs() < f32::EPSILON);
    }
}
