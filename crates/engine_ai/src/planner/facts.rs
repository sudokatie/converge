//! Fact values, requirements, modifications, and belief state for the planner.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FactValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
}

impl FactValue {
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(v) => Some(*v),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(v) => Some(*v),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(v) => Some(*v),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(v) => Some(v),
            _ => None,
        }
    }

    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Bool(_) => "bool",
            Self::Int(_) => "int",
            Self::Float(_) => "float",
            Self::Text(_) => "text",
        }
    }
}

impl From<bool> for FactValue {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}

impl From<i64> for FactValue {
    fn from(v: i64) -> Self {
        Self::Int(v)
    }
}

impl From<f64> for FactValue {
    fn from(v: f64) -> Self {
        Self::Float(v)
    }
}

impl From<String> for FactValue {
    fn from(v: String) -> Self {
        Self::Text(v)
    }
}

impl From<&str> for FactValue {
    fn from(v: &str) -> Self {
        Self::Text(v.to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FactRequirement {
    IsTrue(String),
    IsFalse(String),
    Equals(String, FactValue),
    NotEquals(String, FactValue),
    AtLeast(String, i64),
    AtMost(String, i64),
    InRange(String, i64, i64),
    TextEquals(String, String),
    Exists(String),
    NotExists(String),
}

impl FactRequirement {
    #[must_use]
    pub fn is_true(key: impl Into<String>) -> Self {
        Self::IsTrue(key.into())
    }

    #[must_use]
    pub fn is_false(key: impl Into<String>) -> Self {
        Self::IsFalse(key.into())
    }

    #[must_use]
    pub fn equals(key: impl Into<String>, value: impl Into<FactValue>) -> Self {
        Self::Equals(key.into(), value.into())
    }

    #[must_use]
    pub fn not_equals(key: impl Into<String>, value: impl Into<FactValue>) -> Self {
        Self::NotEquals(key.into(), value.into())
    }

    #[must_use]
    pub fn at_least(key: impl Into<String>, value: i64) -> Self {
        Self::AtLeast(key.into(), value)
    }

    #[must_use]
    pub fn at_most(key: impl Into<String>, value: i64) -> Self {
        Self::AtMost(key.into(), value)
    }

    #[must_use]
    pub fn in_range(key: impl Into<String>, min: i64, max: i64) -> Self {
        Self::InRange(key.into(), min, max)
    }

    #[must_use]
    pub fn text_equals(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self::TextEquals(key.into(), value.into())
    }

    #[must_use]
    pub fn exists(key: impl Into<String>) -> Self {
        Self::Exists(key.into())
    }

    #[must_use]
    pub fn not_exists(key: impl Into<String>) -> Self {
        Self::NotExists(key.into())
    }

    #[must_use]
    pub fn key(&self) -> &str {
        match self {
            Self::IsTrue(k)
            | Self::IsFalse(k)
            | Self::Equals(k, _)
            | Self::NotEquals(k, _)
            | Self::AtLeast(k, _)
            | Self::AtMost(k, _)
            | Self::InRange(k, _, _)
            | Self::TextEquals(k, _)
            | Self::Exists(k)
            | Self::NotExists(k) => k,
        }
    }

    #[must_use]
    pub fn check(&self, state: &BeliefState) -> bool {
        match self {
            Self::IsTrue(k) => state.get_bool(k).unwrap_or(false),
            Self::IsFalse(k) => !state.get_bool(k).unwrap_or(true),
            Self::Equals(k, v) => state.get(k) == Some(v),
            Self::NotEquals(k, v) => state.get(k) != Some(v),
            Self::AtLeast(k, v) => state.get_int(k).unwrap_or(0) >= *v,
            Self::AtMost(k, v) => state.get_int(k).unwrap_or(0) <= *v,
            Self::InRange(k, min, max) => {
                let val = state.get_int(k).unwrap_or(0);
                val >= *min && val <= *max
            }
            Self::TextEquals(k, v) => state.get_text(k).is_some_and(|sv| sv == v),
            Self::Exists(k) => state.contains(k),
            Self::NotExists(k) => !state.contains(k),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FactModification {
    SetBool(String, bool),
    SetInt(String, i64),
    SetFloat(String, f64),
    SetText(String, String),
    Increment(String, i64),
    Decrement(String, i64),
    Remove(String),
    SetTrue(String),
    SetFalse(String),
}

impl FactModification {
    #[must_use]
    pub fn set_bool(key: impl Into<String>, value: bool) -> Self {
        Self::SetBool(key.into(), value)
    }

    #[must_use]
    pub fn set_int(key: impl Into<String>, value: i64) -> Self {
        Self::SetInt(key.into(), value)
    }

    #[must_use]
    pub fn set_float(key: impl Into<String>, value: f64) -> Self {
        Self::SetFloat(key.into(), value)
    }

    #[must_use]
    pub fn set_text(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self::SetText(key.into(), value.into())
    }

    #[must_use]
    pub fn increment(key: impl Into<String>, amount: i64) -> Self {
        Self::Increment(key.into(), amount)
    }

    #[must_use]
    pub fn decrement(key: impl Into<String>, amount: i64) -> Self {
        Self::Decrement(key.into(), amount)
    }

    #[must_use]
    pub fn remove(key: impl Into<String>) -> Self {
        Self::Remove(key.into())
    }

    #[must_use]
    pub fn set_true(key: impl Into<String>) -> Self {
        Self::SetTrue(key.into())
    }

    #[must_use]
    pub fn set_false(key: impl Into<String>) -> Self {
        Self::SetFalse(key.into())
    }

    #[must_use]
    pub fn key(&self) -> &str {
        match self {
            Self::SetBool(k, _)
            | Self::SetInt(k, _)
            | Self::SetFloat(k, _)
            | Self::SetText(k, _)
            | Self::Increment(k, _)
            | Self::Decrement(k, _)
            | Self::Remove(k)
            | Self::SetTrue(k)
            | Self::SetFalse(k) => k,
        }
    }

    pub fn apply(&self, state: &mut BeliefState) {
        match self {
            Self::SetBool(k, v) => state.set_bool(k, *v),
            Self::SetInt(k, v) => state.set_int(k, *v),
            Self::SetFloat(k, v) => state.set_float(k, *v),
            Self::SetText(k, v) => state.set_text(k, v.clone()),
            Self::Increment(k, amount) => {
                let current = state.get_int(k).unwrap_or(0);
                state.set_int(k, current.saturating_add(*amount));
            }
            Self::Decrement(k, amount) => {
                let current = state.get_int(k).unwrap_or(0);
                state.set_int(k, current.saturating_sub(*amount));
            }
            Self::Remove(k) => {
                state.remove(k);
            }
            Self::SetTrue(k) => state.set_bool(k, true),
            Self::SetFalse(k) => state.set_bool(k, false),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BeliefState {
    facts: BTreeMap<String, FactValue>,
}

impl BeliefState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&FactValue> {
        self.facts.get(key)
    }

    #[must_use]
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.facts.get(key).and_then(FactValue::as_bool)
    }

    #[must_use]
    pub fn get_int(&self, key: &str) -> Option<i64> {
        self.facts.get(key).and_then(FactValue::as_int)
    }

    #[must_use]
    pub fn get_float(&self, key: &str) -> Option<f64> {
        self.facts.get(key).and_then(FactValue::as_float)
    }

    #[must_use]
    pub fn get_text(&self, key: &str) -> Option<&str> {
        self.facts.get(key).and_then(FactValue::as_text)
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<FactValue>) {
        self.facts.insert(key.into(), value.into());
    }

    pub fn set_bool(&mut self, key: impl Into<String>, value: bool) {
        self.facts.insert(key.into(), FactValue::Bool(value));
    }

    pub fn set_int(&mut self, key: impl Into<String>, value: i64) {
        self.facts.insert(key.into(), FactValue::Int(value));
    }

    pub fn set_float(&mut self, key: impl Into<String>, value: f64) {
        self.facts.insert(key.into(), FactValue::Float(value));
    }

    pub fn set_text(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.facts.insert(key.into(), FactValue::Text(value.into()));
    }

    pub fn remove(&mut self, key: &str) -> Option<FactValue> {
        self.facts.remove(key)
    }

    #[must_use]
    pub fn contains(&self, key: &str) -> bool {
        self.facts.contains_key(key)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.facts.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.facts.is_empty()
    }

    pub fn clear(&mut self) {
        self.facts.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &FactValue)> {
        self.facts.iter()
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.facts.keys()
    }

    #[must_use]
    pub fn satisfies(&self, requirements: &[FactRequirement]) -> bool {
        requirements.iter().all(|r| r.check(self))
    }

    pub fn apply(&mut self, modification: &FactModification) {
        modification.apply(self);
    }

    pub fn apply_all(&mut self, modifications: &[FactModification]) {
        for m in modifications {
            m.apply(self);
        }
    }

    #[must_use]
    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        for (key, value) in &self.facts {
            hasher.update(key.as_bytes());
            hasher.update(&[0]);
            match value {
                FactValue::Bool(v) => {
                    hasher.update(&[0]);
                    hasher.update(&[u8::from(*v)]);
                }
                FactValue::Int(v) => {
                    hasher.update(&[1]);
                    hasher.update(&v.to_le_bytes());
                }
                FactValue::Float(v) => {
                    hasher.update(&[2]);
                    hasher.update(&v.to_le_bytes());
                }
                FactValue::Text(v) => {
                    hasher.update(&[3]);
                    hasher.update(v.as_bytes());
                }
            }
            hasher.update(&[0]);
        }
        hasher.finalize()
    }

    #[must_use]
    pub fn fingerprint(&self) -> BeliefFingerprint {
        BeliefFingerprint {
            checksum: self.checksum(),
            fact_count: self.facts.len(),
        }
    }

    #[must_use]
    pub fn with_modification(&self, modification: &FactModification) -> Self {
        let mut new_state = self.clone();
        modification.apply(&mut new_state);
        new_state
    }

    #[must_use]
    pub fn with_modifications(&self, modifications: &[FactModification]) -> Self {
        let mut new_state = self.clone();
        new_state.apply_all(modifications);
        new_state
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BeliefFingerprint {
    pub checksum: u32,
    pub fact_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fact_value_types() {
        let b = FactValue::Bool(true);
        assert_eq!(b.as_bool(), Some(true));
        assert_eq!(b.as_int(), None);
        assert_eq!(b.type_name(), "bool");

        let i = FactValue::Int(42);
        assert_eq!(i.as_int(), Some(42));
        assert_eq!(i.as_bool(), None);
        assert_eq!(i.type_name(), "int");

        let f = FactValue::Float(2.5);
        assert_eq!(f.as_float(), Some(2.5));
        assert_eq!(f.type_name(), "float");

        let t = FactValue::Text("hello".to_string());
        assert_eq!(t.as_text(), Some("hello"));
        assert_eq!(t.type_name(), "text");
    }

    #[test]
    fn test_fact_value_from() {
        let v: FactValue = true.into();
        assert_eq!(v, FactValue::Bool(true));

        let v: FactValue = 42i64.into();
        assert_eq!(v, FactValue::Int(42));

        let v: FactValue = 2.5f64.into();
        assert_eq!(v, FactValue::Float(2.5));

        let v: FactValue = "hello".into();
        assert_eq!(v, FactValue::Text("hello".to_string()));
    }

    #[test]
    fn test_fact_requirement_check() {
        let mut state = BeliefState::new();
        state.set_bool("armed", true);
        state.set_int("health", 50);
        state.set_text("status", "active");

        assert!(FactRequirement::is_true("armed").check(&state));
        assert!(!FactRequirement::is_false("armed").check(&state));
        assert!(FactRequirement::at_least("health", 50).check(&state));
        assert!(FactRequirement::at_least("health", 30).check(&state));
        assert!(!FactRequirement::at_least("health", 60).check(&state));
        assert!(FactRequirement::at_most("health", 50).check(&state));
        assert!(FactRequirement::in_range("health", 40, 60).check(&state));
        assert!(FactRequirement::text_equals("status", "active").check(&state));
        assert!(FactRequirement::exists("health").check(&state));
        assert!(FactRequirement::not_exists("missing").check(&state));
    }

    #[test]
    fn test_fact_modification_apply() {
        let mut state = BeliefState::new();

        FactModification::set_true("flag").apply(&mut state);
        assert_eq!(state.get_bool("flag"), Some(true));

        FactModification::set_int("count", 10).apply(&mut state);
        assert_eq!(state.get_int("count"), Some(10));

        FactModification::increment("count", 5).apply(&mut state);
        assert_eq!(state.get_int("count"), Some(15));

        FactModification::decrement("count", 3).apply(&mut state);
        assert_eq!(state.get_int("count"), Some(12));

        FactModification::remove("flag").apply(&mut state);
        assert!(!state.contains("flag"));
    }

    #[test]
    fn test_belief_state_basic() {
        let mut state = BeliefState::new();
        assert!(state.is_empty());

        state.set_bool("a", true);
        state.set_int("b", 42);
        state.set_float("c", 2.5);
        state.set_text("d", "hello");

        assert_eq!(state.len(), 4);
        assert!(!state.is_empty());
        assert!(state.contains("a"));
        assert_eq!(state.get_bool("a"), Some(true));
        assert_eq!(state.get_int("b"), Some(42));
        assert!((state.get_float("c").unwrap() - 2.5).abs() < f64::EPSILON);
        assert_eq!(state.get_text("d"), Some("hello"));
    }

    #[test]
    fn test_belief_state_satisfies() {
        let mut state = BeliefState::new();
        state.set_bool("has_weapon", true);
        state.set_int("ammo", 20);

        let requirements = vec![
            FactRequirement::is_true("has_weapon"),
            FactRequirement::at_least("ammo", 10),
        ];

        assert!(state.satisfies(&requirements));

        state.set_int("ammo", 5);
        assert!(!state.satisfies(&requirements));
    }

    #[test]
    fn test_belief_state_checksum_deterministic() {
        let mut state1 = BeliefState::new();
        state1.set_bool("a", true);
        state1.set_int("b", 42);

        let mut state2 = BeliefState::new();
        state2.set_int("b", 42);
        state2.set_bool("a", true);

        assert_eq!(state1.checksum(), state2.checksum());
    }

    #[test]
    fn test_belief_state_checksum_different() {
        let mut state1 = BeliefState::new();
        state1.set_bool("a", true);

        let mut state2 = BeliefState::new();
        state2.set_bool("a", false);

        assert_ne!(state1.checksum(), state2.checksum());
    }

    #[test]
    fn test_belief_state_fingerprint() {
        let mut state = BeliefState::new();
        state.set_bool("a", true);
        state.set_int("b", 42);

        let fp = state.fingerprint();
        assert_eq!(fp.fact_count, 2);
        assert_eq!(fp.checksum, state.checksum());
    }

    #[test]
    fn test_belief_state_with_modification() {
        let mut state = BeliefState::new();
        state.set_int("count", 10);

        let new_state = state.with_modification(&FactModification::increment("count", 5));

        assert_eq!(state.get_int("count"), Some(10));
        assert_eq!(new_state.get_int("count"), Some(15));
    }

    #[test]
    fn test_belief_state_serde() {
        let mut state = BeliefState::new();
        state.set_bool("flag", true);
        state.set_int("count", 42);

        let json = serde_json::to_string(&state).unwrap();
        let restored: BeliefState = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.get_bool("flag"), Some(true));
        assert_eq!(restored.get_int("count"), Some(42));
        assert_eq!(restored.checksum(), state.checksum());
    }

    #[test]
    fn test_fact_requirement_serde() {
        let req = FactRequirement::at_least("health", 50);
        let json = serde_json::to_string(&req).unwrap();
        let restored: FactRequirement = serde_json::from_str(&json).unwrap();

        assert_eq!(restored, req);
    }

    #[test]
    fn test_fact_modification_serde() {
        let modification = FactModification::increment("count", 10);
        let json = serde_json::to_string(&modification).unwrap();
        let restored: FactModification = serde_json::from_str(&json).unwrap();

        assert_eq!(restored, modification);
    }

    #[test]
    fn test_belief_state_bincode() {
        let mut state = BeliefState::new();
        state.set_bool("flag", true);
        state.set_int("count", 42);
        state.set_float("rate", 0.5);
        state.set_text("name", "test");

        let bytes = bincode::serialize(&state).unwrap();
        let restored: BeliefState = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.get_bool("flag"), Some(true));
        assert_eq!(restored.get_int("count"), Some(42));
        assert_eq!(restored.checksum(), state.checksum());
    }

    #[test]
    fn test_fact_requirement_bincode() {
        let req = FactRequirement::in_range("health", 20, 100);
        let bytes = bincode::serialize(&req).unwrap();
        let restored: FactRequirement = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored, req);
    }

    #[test]
    fn test_fact_modification_bincode() {
        let modification = FactModification::decrement("ammo", 5);
        let bytes = bincode::serialize(&modification).unwrap();
        let restored: FactModification = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored, modification);
    }

    #[test]
    fn test_belief_fingerprint_bincode() {
        let mut state = BeliefState::new();
        state.set_bool("a", true);
        let fp = state.fingerprint();

        let bytes = bincode::serialize(&fp).unwrap();
        let restored: BeliefFingerprint = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.checksum, fp.checksum);
        assert_eq!(restored.fact_count, fp.fact_count);
    }
}
