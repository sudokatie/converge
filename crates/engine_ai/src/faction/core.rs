//! Core faction types and registry.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Unique identifier for a faction.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FactionId(pub String);

impl FactionId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for FactionId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

/// Tag for categorizing or filtering factions.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FactionTag(pub String);

impl FactionTag {
    pub const PLAYER: &'static str = "player";
    pub const NPC: &'static str = "npc";
    pub const HOSTILE: &'static str = "hostile";
    pub const NEUTRAL: &'static str = "neutral";
    pub const WILDLIFE: &'static str = "wildlife";

    #[must_use]
    pub fn new(tag: impl Into<String>) -> Self {
        Self(tag.into())
    }

    #[must_use]
    pub fn player() -> Self {
        Self::new(Self::PLAYER)
    }

    #[must_use]
    pub fn npc() -> Self {
        Self::new(Self::NPC)
    }

    #[must_use]
    pub fn hostile() -> Self {
        Self::new(Self::HOSTILE)
    }

    #[must_use]
    pub fn neutral() -> Self {
        Self::new(Self::NEUTRAL)
    }

    #[must_use]
    pub fn wildlife() -> Self {
        Self::new(Self::WILDLIFE)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for FactionTag {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

/// A faction representing a group of actors with shared identity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Faction {
    /// Unique identifier.
    pub id: FactionId,
    /// Display name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// Tags for categorization.
    tags: BTreeSet<FactionTag>,
    /// Optional parent faction for hierarchies.
    pub parent: Option<FactionId>,
    /// Allied factions (formal alliance).
    allies: BTreeSet<FactionId>,
    /// Custom metadata.
    metadata: BTreeMap<String, String>,
    /// Tick when created.
    pub created_tick: u64,
}

impl Faction {
    /// Create a new faction.
    #[must_use]
    pub fn new(id: FactionId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            description: None,
            tags: BTreeSet::new(),
            parent: None,
            allies: BTreeSet::new(),
            metadata: BTreeMap::new(),
            created_tick: 0,
        }
    }

    /// Create with a specific creation tick.
    #[must_use]
    pub fn with_tick(mut self, tick: u64) -> Self {
        self.created_tick = tick;
        self
    }

    /// Set description.
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set parent faction.
    #[must_use]
    pub fn with_parent(mut self, parent: FactionId) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Add a tag.
    pub fn add_tag(&mut self, tag: FactionTag) {
        self.tags.insert(tag);
    }

    /// Remove a tag.
    pub fn remove_tag(&mut self, tag: &FactionTag) -> bool {
        self.tags.remove(tag)
    }

    /// Check if has tag.
    #[must_use]
    pub fn has_tag(&self, tag: &FactionTag) -> bool {
        self.tags.contains(tag)
    }

    /// Get all tags.
    pub fn tags(&self) -> impl Iterator<Item = &FactionTag> {
        self.tags.iter()
    }

    /// Add an ally.
    pub fn add_ally(&mut self, ally: FactionId) {
        self.allies.insert(ally);
    }

    /// Remove an ally.
    pub fn remove_ally(&mut self, ally: &FactionId) -> bool {
        self.allies.remove(ally)
    }

    /// Check if allied.
    #[must_use]
    pub fn is_allied(&self, other: &FactionId) -> bool {
        self.allies.contains(other)
    }

    /// Get all allies.
    pub fn allies(&self) -> impl Iterator<Item = &FactionId> {
        self.allies.iter()
    }

    /// Set metadata value.
    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Get metadata value.
    #[must_use]
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(String::as_str)
    }

    /// Remove metadata.
    pub fn remove_metadata(&mut self, key: &str) -> Option<String> {
        self.metadata.remove(key)
    }

    /// Iterate over metadata.
    pub fn metadata(&self) -> impl Iterator<Item = (&str, &str)> {
        self.metadata.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }
}

/// Registry of all factions with lookup and hierarchy support.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FactionRegistry {
    factions: BTreeMap<FactionId, Faction>,
}

impl FactionRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a faction.
    pub fn register(&mut self, faction: Faction) {
        self.factions.insert(faction.id.clone(), faction);
    }

    /// Unregister a faction.
    pub fn unregister(&mut self, id: &FactionId) -> Option<Faction> {
        self.factions.remove(id)
    }

    /// Get a faction by ID.
    #[must_use]
    pub fn get(&self, id: &FactionId) -> Option<&Faction> {
        self.factions.get(id)
    }

    /// Get a mutable faction.
    pub fn get_mut(&mut self, id: &FactionId) -> Option<&mut Faction> {
        self.factions.get_mut(id)
    }

    /// Check if faction exists.
    #[must_use]
    pub fn contains(&self, id: &FactionId) -> bool {
        self.factions.contains_key(id)
    }

    /// Get number of factions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.factions.len()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.factions.is_empty()
    }

    /// Iterate over all factions (deterministic order by ID).
    pub fn iter(&self) -> impl Iterator<Item = &Faction> {
        self.factions.values()
    }

    /// Get factions with a specific tag.
    pub fn by_tag(&self, tag: &FactionTag) -> impl Iterator<Item = &Faction> {
        self.factions.values().filter(move |f| f.has_tag(tag))
    }

    /// Get child factions of a parent.
    pub fn children_of(&self, parent: &FactionId) -> impl Iterator<Item = &Faction> {
        self.factions
            .values()
            .filter(move |f| f.parent.as_ref() == Some(parent))
    }

    /// Get the ancestry chain from a faction up to root.
    #[must_use]
    pub fn ancestry(&self, id: &FactionId) -> Vec<FactionId> {
        let mut result = Vec::new();
        let mut current = id.clone();

        while let Some(faction) = self.factions.get(&current) {
            if let Some(ref parent) = faction.parent {
                if result.contains(parent) {
                    break;
                }
                result.push(parent.clone());
                current = parent.clone();
            } else {
                break;
            }
        }

        result
    }

    /// Check if two factions share an ancestor (same hierarchy).
    #[must_use]
    pub fn share_ancestor(&self, a: &FactionId, b: &FactionId) -> bool {
        if a == b {
            return true;
        }

        let ancestry_a = self.ancestry(a);
        let ancestry_b = self.ancestry(b);

        if ancestry_a.contains(b) || ancestry_b.contains(a) {
            return true;
        }

        ancestry_a.iter().any(|id| ancestry_b.contains(id))
    }

    /// Get all faction IDs (deterministic order).
    pub fn ids(&self) -> impl Iterator<Item = &FactionId> {
        self.factions.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_faction_id() {
        let id = FactionId::new("test");
        assert_eq!(id.as_str(), "test");

        let id2: FactionId = "other".into();
        assert_eq!(id2.as_str(), "other");
    }

    #[test]
    fn test_faction_tag_constants() {
        assert_eq!(FactionTag::player().as_str(), "player");
        assert_eq!(FactionTag::npc().as_str(), "npc");
        assert_eq!(FactionTag::hostile().as_str(), "hostile");
    }

    #[test]
    fn test_faction_new() {
        let f = Faction::new(FactionId::new("miners"), "Miner's Guild");
        assert_eq!(f.id.as_str(), "miners");
        assert_eq!(f.name, "Miner's Guild");
        assert!(f.description.is_none());
        assert!(f.parent.is_none());
    }

    #[test]
    fn test_faction_tags() {
        let mut f = Faction::new(FactionId::new("test"), "Test");

        f.add_tag(FactionTag::npc());
        f.add_tag(FactionTag::hostile());

        assert!(f.has_tag(&FactionTag::npc()));
        assert!(f.has_tag(&FactionTag::hostile()));
        assert!(!f.has_tag(&FactionTag::player()));

        assert!(f.remove_tag(&FactionTag::hostile()));
        assert!(!f.has_tag(&FactionTag::hostile()));
    }

    #[test]
    fn test_faction_allies() {
        let mut f = Faction::new(FactionId::new("a"), "Faction A");

        f.add_ally(FactionId::new("b"));
        f.add_ally(FactionId::new("c"));

        assert!(f.is_allied(&FactionId::new("b")));
        assert!(f.is_allied(&FactionId::new("c")));
        assert!(!f.is_allied(&FactionId::new("d")));

        assert!(f.remove_ally(&FactionId::new("b")));
        assert!(!f.is_allied(&FactionId::new("b")));
    }

    #[test]
    fn test_faction_metadata() {
        let mut f = Faction::new(FactionId::new("test"), "Test");

        f.set_metadata("color", "blue");
        f.set_metadata("tier", "1");

        assert_eq!(f.get_metadata("color"), Some("blue"));
        assert_eq!(f.get_metadata("tier"), Some("1"));
        assert_eq!(f.get_metadata("missing"), None);

        f.remove_metadata("color");
        assert_eq!(f.get_metadata("color"), None);
    }

    #[test]
    fn test_faction_with_parent() {
        let f = Faction::new(FactionId::new("child"), "Child")
            .with_parent(FactionId::new("parent"))
            .with_description("A child faction");

        assert_eq!(f.parent, Some(FactionId::new("parent")));
        assert_eq!(f.description.as_deref(), Some("A child faction"));
    }

    #[test]
    fn test_registry_basic() {
        let mut reg = FactionRegistry::new();

        reg.register(Faction::new(FactionId::new("a"), "Faction A"));
        reg.register(Faction::new(FactionId::new("b"), "Faction B"));

        assert_eq!(reg.len(), 2);
        assert!(reg.contains(&FactionId::new("a")));
        assert!(reg.contains(&FactionId::new("b")));
        assert!(!reg.contains(&FactionId::new("c")));
    }

    #[test]
    fn test_registry_get() {
        let mut reg = FactionRegistry::new();
        reg.register(Faction::new(FactionId::new("test"), "Test Faction"));

        let f = reg.get(&FactionId::new("test")).unwrap();
        assert_eq!(f.name, "Test Faction");
    }

    #[test]
    fn test_registry_unregister() {
        let mut reg = FactionRegistry::new();
        reg.register(Faction::new(FactionId::new("test"), "Test"));

        let removed = reg.unregister(&FactionId::new("test"));
        assert!(removed.is_some());
        assert!(!reg.contains(&FactionId::new("test")));
    }

    #[test]
    fn test_registry_by_tag() {
        let mut reg = FactionRegistry::new();

        let mut f1 = Faction::new(FactionId::new("a"), "A");
        f1.add_tag(FactionTag::hostile());
        reg.register(f1);

        let mut f2 = Faction::new(FactionId::new("b"), "B");
        f2.add_tag(FactionTag::hostile());
        reg.register(f2);

        let mut f3 = Faction::new(FactionId::new("c"), "C");
        f3.add_tag(FactionTag::neutral());
        reg.register(f3);

        let hostile: Vec<_> = reg.by_tag(&FactionTag::hostile()).collect();
        assert_eq!(hostile.len(), 2);
    }

    #[test]
    fn test_registry_hierarchy() {
        let mut reg = FactionRegistry::new();

        reg.register(Faction::new(FactionId::new("root"), "Root"));
        reg.register(
            Faction::new(FactionId::new("child1"), "Child 1").with_parent(FactionId::new("root")),
        );
        reg.register(
            Faction::new(FactionId::new("child2"), "Child 2").with_parent(FactionId::new("root")),
        );
        reg.register(
            Faction::new(FactionId::new("grandchild"), "Grandchild")
                .with_parent(FactionId::new("child1")),
        );

        let children: Vec<_> = reg.children_of(&FactionId::new("root")).collect();
        assert_eq!(children.len(), 2);

        let ancestry = reg.ancestry(&FactionId::new("grandchild"));
        assert_eq!(ancestry.len(), 2);
        assert_eq!(ancestry[0], FactionId::new("child1"));
        assert_eq!(ancestry[1], FactionId::new("root"));
    }

    #[test]
    fn test_registry_share_ancestor() {
        let mut reg = FactionRegistry::new();

        reg.register(Faction::new(FactionId::new("root"), "Root"));
        reg.register(Faction::new(FactionId::new("a"), "A").with_parent(FactionId::new("root")));
        reg.register(Faction::new(FactionId::new("b"), "B").with_parent(FactionId::new("root")));
        reg.register(Faction::new(FactionId::new("c"), "C"));

        assert!(reg.share_ancestor(&FactionId::new("a"), &FactionId::new("b")));
        assert!(reg.share_ancestor(&FactionId::new("a"), &FactionId::new("a")));
        assert!(!reg.share_ancestor(&FactionId::new("a"), &FactionId::new("c")));
    }

    #[test]
    fn test_registry_deterministic_iteration() {
        let mut reg = FactionRegistry::new();
        reg.register(Faction::new(FactionId::new("z"), "Z"));
        reg.register(Faction::new(FactionId::new("a"), "A"));
        reg.register(Faction::new(FactionId::new("m"), "M"));

        let ids: Vec<_> = reg.ids().map(FactionId::as_str).collect();
        assert_eq!(ids, vec!["a", "m", "z"]);
    }

    #[test]
    fn test_faction_serde() {
        let mut f = Faction::new(FactionId::new("test"), "Test Faction")
            .with_description("A test")
            .with_parent(FactionId::new("parent"))
            .with_tick(100);

        f.add_tag(FactionTag::npc());
        f.add_ally(FactionId::new("ally"));
        f.set_metadata("key", "value");

        let json = serde_json::to_string(&f).unwrap();
        let restored: Faction = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, f.id);
        assert_eq!(restored.name, f.name);
        assert_eq!(restored.description, f.description);
        assert_eq!(restored.parent, f.parent);
        assert!(restored.has_tag(&FactionTag::npc()));
        assert!(restored.is_allied(&FactionId::new("ally")));
        assert_eq!(restored.get_metadata("key"), Some("value"));
    }

    #[test]
    fn test_registry_serde() {
        let mut reg = FactionRegistry::new();
        reg.register(Faction::new(FactionId::new("a"), "A"));
        reg.register(Faction::new(FactionId::new("b"), "B"));

        let json = serde_json::to_string(&reg).unwrap();
        let restored: FactionRegistry = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.len(), 2);
        assert!(restored.contains(&FactionId::new("a")));
        assert!(restored.contains(&FactionId::new("b")));
    }
}
