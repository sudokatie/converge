//! Multi-state chunk storage for alternate dimensions, time-loop snapshots,
//! and phased realities.
//!
//! Provides storage and access for multiple coexisting chunk states at the
//! same spatial position.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::state_id::StateId;
use crate::chunk::Chunk;

/// Fallback behavior when accessing a missing state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum StateFallback {
    /// Return `None` when state is missing.
    #[default]
    None,

    /// Fall back to the primary state.
    Primary,

    /// Fall back to the active state.
    Active,

    /// Create an empty chunk for missing states.
    Empty,
}

/// Multi-state chunk storing multiple reality states at one position.
///
/// Each state is identified by a `StateId`. The primary state (`StateId::PRIMARY`)
/// is the canonical world state. Additional states represent alternate dimensions,
/// time snapshots, or phased realities.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MultiStateChunk {
    /// States stored for this chunk position, keyed by state ID.
    /// Uses `BTreeMap` for deterministic iteration order.
    states: BTreeMap<StateId, Chunk>,

    /// Currently active state for queries.
    active: StateId,

    /// Fallback behavior for missing states.
    fallback: StateFallback,
}

impl MultiStateChunk {
    /// Create a new multi-state chunk with a primary state.
    #[must_use]
    pub fn new(primary: Chunk) -> Self {
        let mut states = BTreeMap::new();
        states.insert(StateId::PRIMARY, primary);

        Self {
            states,
            active: StateId::PRIMARY,
            fallback: StateFallback::Primary,
        }
    }

    /// Create an empty multi-state chunk with no states.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            states: BTreeMap::new(),
            active: StateId::PRIMARY,
            fallback: StateFallback::None,
        }
    }

    /// Get the currently active state ID.
    #[must_use]
    pub fn active_state(&self) -> StateId {
        self.active
    }

    /// Set the active state ID.
    ///
    /// The active state is used for default queries and determines which
    /// state is returned by `get_active()`.
    pub fn set_active_state(&mut self, state: StateId) {
        self.active = state;
    }

    /// Get the fallback behavior.
    #[must_use]
    pub fn fallback(&self) -> StateFallback {
        self.fallback
    }

    /// Set the fallback behavior for missing states.
    pub fn set_fallback(&mut self, fallback: StateFallback) {
        self.fallback = fallback;
    }

    /// Check if a specific state exists.
    #[must_use]
    pub fn has_state(&self, state: StateId) -> bool {
        self.states.contains_key(&state)
    }

    /// Check if the primary state exists.
    #[must_use]
    pub fn has_primary(&self) -> bool {
        self.has_state(StateId::PRIMARY)
    }

    /// Get the number of stored states.
    #[must_use]
    pub fn state_count(&self) -> usize {
        self.states.len()
    }

    /// Check if no states are stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    /// Get a reference to a specific state.
    #[must_use]
    pub fn get(&self, state: StateId) -> Option<&Chunk> {
        self.states.get(&state)
    }

    /// Get a mutable reference to a specific state.
    pub fn get_mut(&mut self, state: StateId) -> Option<&mut Chunk> {
        self.states.get_mut(&state)
    }

    /// Get the active state with fallback behavior.
    #[must_use]
    pub fn get_active(&self) -> Option<&Chunk> {
        self.get_with_fallback(self.active)
    }

    /// Get a mutable reference to the active state.
    pub fn get_active_mut(&mut self) -> Option<&mut Chunk> {
        let active = self.active;
        self.states.get_mut(&active)
    }

    /// Get a state with fallback behavior applied.
    #[must_use]
    pub fn get_with_fallback(&self, state: StateId) -> Option<&Chunk> {
        if let Some(chunk) = self.states.get(&state) {
            return Some(chunk);
        }

        match self.fallback {
            StateFallback::Primary => self.states.get(&StateId::PRIMARY),
            StateFallback::Active => {
                if state == self.active {
                    None
                } else {
                    self.states.get(&self.active)
                }
            }
            StateFallback::None | StateFallback::Empty => None,
        }
    }

    /// Get the primary state.
    #[must_use]
    pub fn get_primary(&self) -> Option<&Chunk> {
        self.get(StateId::PRIMARY)
    }

    /// Get mutable reference to the primary state.
    pub fn get_primary_mut(&mut self) -> Option<&mut Chunk> {
        self.get_mut(StateId::PRIMARY)
    }

    /// Insert or replace a state.
    ///
    /// Returns the previous chunk if one existed for this state.
    pub fn insert(&mut self, state: StateId, chunk: Chunk) -> Option<Chunk> {
        self.states.insert(state, chunk)
    }

    /// Remove a state.
    ///
    /// Returns the removed chunk if it existed.
    ///
    /// Note: Removing the primary state is allowed but may cause issues
    /// with fallback behavior.
    pub fn remove(&mut self, state: StateId) -> Option<Chunk> {
        self.states.remove(&state)
    }

    /// Clone a state to create a new state (e.g., for time snapshots).
    ///
    /// Returns `false` if the source state doesn't exist.
    pub fn clone_state(&mut self, source: StateId, target: StateId) -> bool {
        if let Some(chunk) = self.states.get(&source) {
            self.states.insert(target, chunk.clone());
            true
        } else {
            false
        }
    }

    /// Iterate over all states.
    pub fn iter(&self) -> impl Iterator<Item = (StateId, &Chunk)> + '_ {
        self.states.iter().map(|(&id, chunk)| (id, chunk))
    }

    /// Iterate over all state IDs.
    pub fn state_ids(&self) -> impl Iterator<Item = StateId> + '_ {
        self.states.keys().copied()
    }

    /// Get the state with the lowest ID (usually primary).
    #[must_use]
    pub fn first_state(&self) -> Option<(StateId, &Chunk)> {
        self.states
            .first_key_value()
            .map(|(&id, chunk)| (id, chunk))
    }

    /// Get or create a state.
    ///
    /// If the state doesn't exist, creates an empty chunk for it.
    pub fn get_or_create(&mut self, state: StateId) -> &mut Chunk {
        self.states.entry(state).or_default()
    }

    /// Merge another multi-state chunk into this one.
    ///
    /// States from `other` overwrite existing states with the same ID.
    pub fn merge(&mut self, other: Self) {
        for (state, chunk) in other.states {
            self.states.insert(state, chunk);
        }
    }

    /// Retain only states matching a predicate.
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(StateId, &Chunk) -> bool,
    {
        self.states.retain(|&id, chunk| f(id, chunk));
    }

    /// Clear all states.
    pub fn clear(&mut self) {
        self.states.clear();
    }
}

impl Default for MultiStateChunk {
    fn default() -> Self {
        Self::new(Chunk::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{BlockId, STONE};
    use engine_core::coords::LocalPos;

    fn test_chunk(marker: BlockId) -> Chunk {
        let mut chunk = Chunk::new();
        chunk.set(LocalPos::new(0, 0, 0), marker);
        chunk
    }

    #[test]
    fn test_new_has_primary() {
        let chunk = test_chunk(STONE);
        let msc = MultiStateChunk::new(chunk);

        assert!(msc.has_primary());
        assert_eq!(msc.state_count(), 1);
        assert_eq!(msc.active_state(), StateId::PRIMARY);
    }

    #[test]
    fn test_empty_chunk() {
        let msc = MultiStateChunk::empty();

        assert!(!msc.has_primary());
        assert!(msc.is_empty());
        assert_eq!(msc.state_count(), 0);
    }

    #[test]
    fn test_insert_and_get() {
        let mut msc = MultiStateChunk::new(test_chunk(STONE));
        let alt_state = StateId::new(1);

        msc.insert(alt_state, test_chunk(BlockId(100)));

        assert!(msc.has_state(alt_state));
        assert_eq!(msc.state_count(), 2);
        assert_eq!(
            msc.get(alt_state).unwrap().get(LocalPos::new(0, 0, 0)),
            BlockId(100)
        );
    }

    #[test]
    fn test_active_state_selection() {
        let mut msc = MultiStateChunk::new(test_chunk(STONE));
        let alt_state = StateId::new(1);
        msc.insert(alt_state, test_chunk(BlockId(200)));

        assert_eq!(msc.get_active().unwrap().get(LocalPos::new(0, 0, 0)), STONE);

        msc.set_active_state(alt_state);
        assert_eq!(msc.active_state(), alt_state);
        assert_eq!(
            msc.get_active().unwrap().get(LocalPos::new(0, 0, 0)),
            BlockId(200)
        );
    }

    #[test]
    fn test_fallback_none() {
        let mut msc = MultiStateChunk::new(test_chunk(STONE));
        msc.set_fallback(StateFallback::None);

        assert!(msc.get_with_fallback(StateId::new(999)).is_none());
    }

    #[test]
    fn test_fallback_primary() {
        let mut msc = MultiStateChunk::new(test_chunk(STONE));
        msc.set_fallback(StateFallback::Primary);

        let result = msc.get_with_fallback(StateId::new(999));
        assert!(result.is_some());
        assert_eq!(result.unwrap().get(LocalPos::new(0, 0, 0)), STONE);
    }

    #[test]
    fn test_fallback_active() {
        let mut msc = MultiStateChunk::new(test_chunk(STONE));
        let alt_state = StateId::new(1);
        msc.insert(alt_state, test_chunk(BlockId(300)));
        msc.set_active_state(alt_state);
        msc.set_fallback(StateFallback::Active);

        let result = msc.get_with_fallback(StateId::new(999));
        assert!(result.is_some());
        assert_eq!(result.unwrap().get(LocalPos::new(0, 0, 0)), BlockId(300));
    }

    #[test]
    fn test_fallback_active_same_state() {
        let mut msc = MultiStateChunk::new(test_chunk(STONE));
        msc.remove(StateId::PRIMARY);
        let missing = StateId::new(5);
        msc.set_active_state(missing);
        msc.set_fallback(StateFallback::Active);

        assert!(msc.get_with_fallback(missing).is_none());
    }

    #[test]
    fn test_clone_state() {
        let mut msc = MultiStateChunk::new(test_chunk(STONE));
        let snapshot = StateId::new(1000);

        assert!(msc.clone_state(StateId::PRIMARY, snapshot));
        assert!(msc.has_state(snapshot));
        assert_eq!(msc.state_count(), 2);

        assert_eq!(
            msc.get(snapshot).unwrap().get(LocalPos::new(0, 0, 0)),
            STONE
        );
    }

    #[test]
    fn test_clone_state_missing_source() {
        let mut msc = MultiStateChunk::new(test_chunk(STONE));

        assert!(!msc.clone_state(StateId::new(999), StateId::new(1000)));
    }

    #[test]
    fn test_remove_state() {
        let mut msc = MultiStateChunk::new(test_chunk(STONE));
        let alt = StateId::new(1);
        msc.insert(alt, test_chunk(BlockId(400)));

        let removed = msc.remove(alt);
        assert!(removed.is_some());
        assert!(!msc.has_state(alt));
        assert_eq!(msc.state_count(), 1);
    }

    #[test]
    fn test_iterate_states() {
        let mut msc = MultiStateChunk::new(test_chunk(STONE));
        msc.insert(StateId::new(5), test_chunk(BlockId(50)));
        msc.insert(StateId::new(2), test_chunk(BlockId(20)));

        let ids: Vec<_> = msc.state_ids().collect();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&StateId::PRIMARY));
        assert!(ids.contains(&StateId::new(2)));
        assert!(ids.contains(&StateId::new(5)));
    }

    #[test]
    fn test_deterministic_iteration_order() {
        let mut msc = MultiStateChunk::new(test_chunk(STONE));
        msc.insert(StateId::new(10), test_chunk(BlockId(10)));
        msc.insert(StateId::new(5), test_chunk(BlockId(5)));
        msc.insert(StateId::new(20), test_chunk(BlockId(20)));

        let order: Vec<_> = msc.state_ids().map(StateId::id).collect();
        assert_eq!(order, vec![0, 5, 10, 20]);
    }

    #[test]
    fn test_get_or_create() {
        let mut msc = MultiStateChunk::new(test_chunk(STONE));
        let new_state = StateId::new(42);

        {
            let chunk = msc.get_or_create(new_state);
            chunk.set(LocalPos::new(1, 1, 1), BlockId(999));
        }

        assert!(msc.has_state(new_state));
        assert_eq!(
            msc.get(new_state).unwrap().get(LocalPos::new(1, 1, 1)),
            BlockId(999)
        );
    }

    #[test]
    fn test_merge() {
        let mut msc1 = MultiStateChunk::new(test_chunk(STONE));
        let mut msc2 = MultiStateChunk::empty();
        msc2.insert(StateId::new(1), test_chunk(BlockId(100)));
        msc2.insert(StateId::new(2), test_chunk(BlockId(200)));

        msc1.merge(msc2);

        assert_eq!(msc1.state_count(), 3);
        assert!(msc1.has_state(StateId::new(1)));
        assert!(msc1.has_state(StateId::new(2)));
    }

    #[test]
    fn test_retain() {
        let mut msc = MultiStateChunk::new(test_chunk(STONE));
        msc.insert(StateId::new(1), test_chunk(BlockId(10)));
        msc.insert(StateId::new(2), Chunk::new());
        msc.insert(StateId::new(3), test_chunk(BlockId(30)));

        msc.retain(|_, chunk| !chunk.is_empty());

        assert_eq!(msc.state_count(), 3);
        assert!(!msc.has_state(StateId::new(2)));
    }

    #[test]
    fn test_first_state() {
        let mut msc = MultiStateChunk::empty();
        assert!(msc.first_state().is_none());

        msc.insert(StateId::new(10), test_chunk(STONE));
        msc.insert(StateId::new(5), test_chunk(BlockId(5)));

        let (id, _) = msc.first_state().unwrap();
        assert_eq!(id, StateId::new(5));
    }

    #[test]
    fn test_serde_roundtrip() {
        let mut msc = MultiStateChunk::new(test_chunk(STONE));
        msc.insert(StateId::new(1), test_chunk(BlockId(100)));
        msc.set_active_state(StateId::new(1));
        msc.set_fallback(StateFallback::Primary);

        let serialized = bincode::serialize(&msc).unwrap();
        let deserialized: MultiStateChunk = bincode::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.state_count(), 2);
        assert_eq!(deserialized.active_state(), StateId::new(1));
        assert_eq!(deserialized.fallback(), StateFallback::Primary);
        assert!(deserialized.has_primary());
        assert!(deserialized.has_state(StateId::new(1)));
    }

    #[test]
    fn test_clear() {
        let mut msc = MultiStateChunk::new(test_chunk(STONE));
        msc.insert(StateId::new(1), test_chunk(BlockId(1)));
        msc.clear();

        assert!(msc.is_empty());
        assert_eq!(msc.state_count(), 0);
    }
}
