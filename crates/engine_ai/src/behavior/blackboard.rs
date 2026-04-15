//! Blackboard for sharing state between behavior tree nodes

use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Blackboard for sharing state between behavior tree nodes
#[derive(Debug, Default)]
pub struct Blackboard {
    data: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl Blackboard {
    /// Create a new empty blackboard
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a value in the blackboard
    pub fn set<T: Any + Send + Sync>(&mut self, key: &str, value: T) {
        self.data.insert(key.to_string(), Box::new(value));
    }

    /// Get a value from the blackboard
    pub fn get<T: Any + Send + Sync>(&self, key: &str) -> Option<&T> {
        self.data.get(key).and_then(|v| v.downcast_ref::<T>())
    }

    /// Get a mutable reference to a value
    pub fn get_mut<T: Any + Send + Sync>(&mut self, key: &str) -> Option<&mut T> {
        self.data.get_mut(key).and_then(|v| v.downcast_mut::<T>())
    }

    /// Remove a value from the blackboard
    pub fn remove(&mut self, key: &str) -> bool {
        self.data.remove(key).is_some()
    }

    /// Check if a key exists
    pub fn contains(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    /// Check if a key exists and has the expected type
    pub fn contains_type<T: Any + Send + Sync>(&self, key: &str) -> bool {
        self.data
            .get(key)
            .map(|v| v.as_ref().type_id() == TypeId::of::<T>())
            .unwrap_or(false)
    }

    /// Clear all data
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get all keys
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.data.keys()
    }

    /// Get or insert a value
    pub fn get_or_insert<T: Any + Send + Sync + Clone>(
        &mut self,
        key: &str,
        default: T,
    ) -> &T {
        if !self.contains_type::<T>(key) {
            self.set(key, default);
        }
        self.get(key).unwrap()
    }

    /// Get or insert with a function
    pub fn get_or_insert_with<T: Any + Send + Sync, F: FnOnce() -> T>(
        &mut self,
        key: &str,
        f: F,
    ) -> &T {
        if !self.contains_type::<T>(key) {
            self.set(key, f());
        }
        self.get(key).unwrap()
    }

    /// Modify a value in place
    pub fn modify<T: Any + Send + Sync, F: FnOnce(&mut T)>(
        &mut self,
        key: &str,
        f: F,
    ) -> bool {
        if let Some(value) = self.get_mut::<T>(key) {
            f(value);
            true
        } else {
            false
        }
    }
}

/// Helper macros and common types for blackboard

/// Position in 3D space
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn distance(&self, other: &Vec3) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    pub fn distance_squared(&self, other: &Vec3) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        dx * dx + dy * dy + dz * dz
    }
}

/// Entity reference for blackboard
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityRef(pub u64);

/// Common blackboard keys
pub mod keys {
    pub const SELF_POSITION: &str = "self_position";
    pub const SELF_ENTITY: &str = "self_entity";
    pub const TARGET_ENTITY: &str = "target_entity";
    pub const TARGET_POSITION: &str = "target_position";
    pub const HEALTH: &str = "health";
    pub const MAX_HEALTH: &str = "max_health";
    pub const IS_HOSTILE: &str = "is_hostile";
    pub const IS_FLEEING: &str = "is_fleeing";
    pub const ATTACK_RANGE: &str = "attack_range";
    pub const DETECTION_RANGE: &str = "detection_range";
    pub const HOME_POSITION: &str = "home_position";
    pub const WANDER_RADIUS: &str = "wander_radius";
    pub const LAST_DAMAGE_TIME: &str = "last_damage_time";
    pub const PATH: &str = "current_path";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get() {
        let mut bb = Blackboard::new();
        bb.set("health", 100i32);
        
        assert_eq!(bb.get::<i32>("health"), Some(&100));
        assert_eq!(bb.get::<f32>("health"), None); // Wrong type
    }

    #[test]
    fn test_contains() {
        let mut bb = Blackboard::new();
        bb.set("key", "value".to_string());
        
        assert!(bb.contains("key"));
        assert!(!bb.contains("other"));
    }

    #[test]
    fn test_modify() {
        let mut bb = Blackboard::new();
        bb.set("counter", 0i32);
        
        bb.modify::<i32, _>("counter", |v| *v += 1);
        assert_eq!(bb.get::<i32>("counter"), Some(&1));
    }

    #[test]
    fn test_get_or_insert() {
        let mut bb = Blackboard::new();
        
        let val = bb.get_or_insert("key", 42i32);
        assert_eq!(*val, 42);
        
        bb.set("key", 100i32);
        let val = bb.get_or_insert("key", 42i32);
        assert_eq!(*val, 100);
    }

    #[test]
    fn test_vec3() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(3.0, 4.0, 0.0);
        assert!((a.distance(&b) - 5.0).abs() < 0.001);
    }
}
