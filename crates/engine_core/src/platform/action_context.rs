//! Action contexts for context-sensitive input handling.
//!
//! Provides typed contexts (Walking, Building, Vehicle, Menu, Spectator) with
//! deterministic priority ordering and stack-based activation.

use std::cmp::Reverse;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::InputState;
use super::action_map::{Action, ActionMap, KeyBinding};

/// Input contexts that determine which action bindings are active.
///
/// Contexts have a deterministic priority order (highest to lowest):
/// Menu > Spectator > Vehicle > Building > Walking
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord,
)]
pub enum ActionContext {
    /// Base movement context (lowest priority, always available as fallback).
    #[default]
    Walking = 0,
    /// Building/construction mode.
    Building = 1,
    /// Vehicle control (car, boat, aircraft).
    Vehicle = 2,
    /// Free camera/spectator mode.
    Spectator = 3,
    /// Menu/UI interaction (highest priority, captures input).
    Menu = 4,
}

impl ActionContext {
    /// Returns the priority level (higher = takes precedence).
    #[must_use]
    pub const fn priority(self) -> u8 {
        self as u8
    }

    /// Returns all contexts in priority order (lowest to highest).
    #[must_use]
    pub const fn all() -> [ActionContext; 5] {
        [
            ActionContext::Walking,
            ActionContext::Building,
            ActionContext::Vehicle,
            ActionContext::Spectator,
            ActionContext::Menu,
        ]
    }
}

/// A binding conflict where the same key maps to different actions in different contexts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindingConflict {
    /// The conflicting key/button binding.
    pub binding: KeyBinding,
    /// First context and its action.
    pub first: (ActionContext, Action),
    /// Second context and its action.
    pub second: (ActionContext, Action),
}

/// Context-aware action mapping with stack-based context activation.
///
/// Wraps an `ActionMap` per context and maintains an activation stack.
/// The topmost (highest priority) active context handles input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextualActionMap {
    /// Per-context action maps.
    context_maps: HashMap<ActionContext, ActionMap>,
    /// Stack of active contexts (may contain duplicates if pushed multiple times).
    #[serde(skip, default = "default_context_stack")]
    context_stack: Vec<ActionContext>,
    /// Fallback context when stack is empty.
    fallback: ActionContext,
}

fn default_context_stack() -> Vec<ActionContext> {
    vec![ActionContext::Walking]
}

impl ContextualActionMap {
    /// Create a new contextual action map with empty bindings.
    #[must_use]
    pub fn new() -> Self {
        let mut context_maps = HashMap::new();
        for ctx in ActionContext::all() {
            context_maps.insert(ctx, ActionMap::new());
        }
        Self {
            context_maps,
            context_stack: vec![ActionContext::Walking],
            fallback: ActionContext::Walking,
        }
    }

    /// Create a contextual action map with sensible default bindings for all contexts.
    #[must_use]
    pub fn with_defaults() -> Self {
        use winit::event::MouseButton;
        use winit::keyboard::KeyCode;

        let mut cam = Self::new();

        // Walking: Full movement and combat controls (uses ActionMap defaults)
        cam.context_maps
            .insert(ActionContext::Walking, ActionMap::with_defaults());

        // Building: Movement + construction-friendly bindings
        let building = cam.get_context_map_mut(ActionContext::Building);
        building.bind(Action::MoveForward, KeyCode::KeyW);
        building.bind(Action::MoveBack, KeyCode::KeyS);
        building.bind(Action::MoveLeft, KeyCode::KeyA);
        building.bind(Action::MoveRight, KeyCode::KeyD);
        building.bind(Action::Jump, KeyCode::Space);
        building.bind(Action::Crouch, KeyCode::ControlLeft);
        building.bind(Action::Sprint, KeyCode::ShiftLeft);
        building.bind(Action::Attack, MouseButton::Left); // Place
        building.bind(Action::UseItem, MouseButton::Right); // Alternate place/rotate
        building.bind(Action::Interact, KeyCode::KeyE);
        building.bind(Action::Inventory, KeyCode::Tab);
        building.bind(Action::Pause, KeyCode::Escape);
        building.bind(Action::Chat, KeyCode::KeyT);
        building.bind(Action::Hotbar1, KeyCode::Digit1);
        building.bind(Action::Hotbar2, KeyCode::Digit2);
        building.bind(Action::Hotbar3, KeyCode::Digit3);
        building.bind(Action::Hotbar4, KeyCode::Digit4);
        building.bind(Action::Hotbar5, KeyCode::Digit5);
        building.bind(Action::Hotbar6, KeyCode::Digit6);
        building.bind(Action::Hotbar7, KeyCode::Digit7);
        building.bind(Action::Hotbar8, KeyCode::Digit8);
        building.bind(Action::Hotbar9, KeyCode::Digit9);

        // Vehicle: Throttle/steering with vehicle-appropriate controls
        let vehicle = cam.get_context_map_mut(ActionContext::Vehicle);
        vehicle.bind(Action::MoveForward, KeyCode::KeyW); // Throttle
        vehicle.bind(Action::MoveBack, KeyCode::KeyS); // Brake/reverse
        vehicle.bind(Action::MoveLeft, KeyCode::KeyA); // Steer left
        vehicle.bind(Action::MoveRight, KeyCode::KeyD); // Steer right
        vehicle.bind(Action::Sprint, KeyCode::ShiftLeft); // Boost
        vehicle.bind(Action::Crouch, KeyCode::ControlLeft); // Handbrake
        vehicle.bind(Action::Jump, KeyCode::Space); // Handbrake alt / vehicle jump
        vehicle.bind(Action::Interact, KeyCode::KeyE); // Exit vehicle
        vehicle.bind(Action::Inventory, KeyCode::Tab);
        vehicle.bind(Action::Pause, KeyCode::Escape);
        vehicle.bind(Action::Chat, KeyCode::KeyT);

        // Menu: UI navigation and escape actions, no world combat
        let menu = cam.get_context_map_mut(ActionContext::Menu);
        menu.bind(Action::MoveForward, KeyCode::KeyW);
        menu.bind(Action::MoveForward, KeyCode::ArrowUp);
        menu.bind(Action::MoveBack, KeyCode::KeyS);
        menu.bind(Action::MoveBack, KeyCode::ArrowDown);
        menu.bind(Action::MoveLeft, KeyCode::KeyA);
        menu.bind(Action::MoveLeft, KeyCode::ArrowLeft);
        menu.bind(Action::MoveRight, KeyCode::KeyD);
        menu.bind(Action::MoveRight, KeyCode::ArrowRight);
        menu.bind(Action::Interact, KeyCode::KeyE); // Select/confirm
        menu.bind(Action::Interact, KeyCode::Enter);
        menu.bind(Action::Pause, KeyCode::Escape); // Close/back
        menu.bind(Action::Inventory, KeyCode::Tab);

        // Spectator: Free camera movement for observation
        let spectator = cam.get_context_map_mut(ActionContext::Spectator);
        spectator.bind(Action::MoveForward, KeyCode::KeyW);
        spectator.bind(Action::MoveBack, KeyCode::KeyS);
        spectator.bind(Action::MoveLeft, KeyCode::KeyA);
        spectator.bind(Action::MoveRight, KeyCode::KeyD);
        spectator.bind(Action::Jump, KeyCode::Space); // Fly up
        spectator.bind(Action::Crouch, KeyCode::ControlLeft); // Fly down
        spectator.bind(Action::Sprint, KeyCode::ShiftLeft); // Fast movement
        spectator.bind(Action::Inventory, KeyCode::Tab);
        spectator.bind(Action::Pause, KeyCode::Escape);
        spectator.bind(Action::Chat, KeyCode::KeyT);

        cam
    }

    /// Get the action map for a specific context.
    ///
    /// # Panics
    ///
    /// Panics if `context` was not initialized. All contexts are initialized
    /// in `new()`, so this cannot panic for a properly constructed instance.
    #[must_use]
    pub fn get_context_map(&self, context: ActionContext) -> &ActionMap {
        self.context_maps
            .get(&context)
            .expect("all contexts initialized")
    }

    /// Get a mutable reference to the action map for a specific context.
    ///
    /// # Panics
    ///
    /// Panics if `context` was not initialized. All contexts are initialized
    /// in `new()`, so this cannot panic for a properly constructed instance.
    pub fn get_context_map_mut(&mut self, context: ActionContext) -> &mut ActionMap {
        self.context_maps
            .get_mut(&context)
            .expect("all contexts initialized")
    }

    /// Bind an input to an action in a specific context.
    pub fn bind(&mut self, context: ActionContext, action: Action, binding: impl Into<KeyBinding>) {
        self.get_context_map_mut(context).bind(action, binding);
    }

    /// Remove a binding from an action in a specific context.
    pub fn unbind(
        &mut self,
        context: ActionContext,
        action: Action,
        binding: impl Into<KeyBinding>,
    ) {
        self.get_context_map_mut(context).unbind(action, binding);
    }

    /// Push a context onto the activation stack.
    pub fn push_context(&mut self, context: ActionContext) {
        self.context_stack.push(context);
    }

    /// Pop the topmost context from the stack.
    /// Returns `None` if only the fallback context remains.
    pub fn pop_context(&mut self) -> Option<ActionContext> {
        if self.context_stack.len() > 1 {
            self.context_stack.pop()
        } else {
            None
        }
    }

    /// Remove all instances of a context from the stack.
    pub fn remove_context(&mut self, context: ActionContext) {
        self.context_stack.retain(|&c| c != context);
        if self.context_stack.is_empty() {
            self.context_stack.push(self.fallback);
        }
    }

    /// Clear the context stack, returning to only the fallback context.
    pub fn clear_contexts(&mut self) {
        self.context_stack.clear();
        self.context_stack.push(self.fallback);
    }

    /// Get the currently active context (highest priority on stack).
    #[must_use]
    pub fn active_context(&self) -> ActionContext {
        self.context_stack
            .iter()
            .copied()
            .max_by_key(|c| c.priority())
            .unwrap_or(self.fallback)
    }

    /// Check if a specific context is currently active (on the stack).
    #[must_use]
    pub fn is_context_active(&self, context: ActionContext) -> bool {
        self.context_stack.contains(&context)
    }

    /// Get all currently active contexts in priority order (highest first).
    #[must_use]
    pub fn active_contexts(&self) -> Vec<ActionContext> {
        let mut contexts = self.context_stack.clone();
        contexts.sort_by_key(|c| Reverse(c.priority()));
        contexts.dedup();
        contexts
    }

    /// Get the context stack (in push order, not priority order).
    #[must_use]
    pub fn context_stack(&self) -> &[ActionContext] {
        &self.context_stack
    }

    /// Set the fallback context used when the stack is empty.
    pub fn set_fallback(&mut self, context: ActionContext) {
        self.fallback = context;
    }

    /// Get the fallback context.
    #[must_use]
    pub fn fallback(&self) -> ActionContext {
        self.fallback
    }

    /// Check if an action was pressed this frame in the active context.
    #[must_use]
    pub fn is_action_pressed(&self, action: Action, input: &InputState) -> bool {
        self.get_context_map(self.active_context())
            .is_action_pressed(action, input)
    }

    /// Check if an action is held in the active context.
    #[must_use]
    pub fn is_action_held(&self, action: Action, input: &InputState) -> bool {
        self.get_context_map(self.active_context())
            .is_action_held(action, input)
    }

    /// Check if an action was released this frame in the active context.
    #[must_use]
    pub fn is_action_released(&self, action: Action, input: &InputState) -> bool {
        self.get_context_map(self.active_context())
            .is_action_released(action, input)
    }

    /// Check if an action was pressed in a specific context (ignoring active context).
    #[must_use]
    pub fn is_action_pressed_in(
        &self,
        context: ActionContext,
        action: Action,
        input: &InputState,
    ) -> bool {
        self.get_context_map(context)
            .is_action_pressed(action, input)
    }

    /// Check if an action is held in a specific context (ignoring active context).
    #[must_use]
    pub fn is_action_held_in(
        &self,
        context: ActionContext,
        action: Action,
        input: &InputState,
    ) -> bool {
        self.get_context_map(context).is_action_held(action, input)
    }

    /// Check if an action was released in a specific context (ignoring active context).
    #[must_use]
    pub fn is_action_released_in(
        &self,
        context: ActionContext,
        action: Action,
        input: &InputState,
    ) -> bool {
        self.get_context_map(context)
            .is_action_released(action, input)
    }

    /// Check if an action is pressed with fallback through the context stack.
    ///
    /// Checks contexts from highest to lowest priority, returning true if any
    /// active context has the action pressed.
    #[must_use]
    pub fn is_action_pressed_with_fallback(&self, action: Action, input: &InputState) -> bool {
        for context in self.active_contexts() {
            if self
                .get_context_map(context)
                .is_action_pressed(action, input)
            {
                return true;
            }
        }
        false
    }

    /// Check if an action is held with fallback through the context stack.
    #[must_use]
    pub fn is_action_held_with_fallback(&self, action: Action, input: &InputState) -> bool {
        for context in self.active_contexts() {
            if self.get_context_map(context).is_action_held(action, input) {
                return true;
            }
        }
        false
    }

    /// Check if an action was released with fallback through the context stack.
    #[must_use]
    pub fn is_action_released_with_fallback(&self, action: Action, input: &InputState) -> bool {
        for context in self.active_contexts() {
            if self
                .get_context_map(context)
                .is_action_released(action, input)
            {
                return true;
            }
        }
        false
    }

    /// Find all binding conflicts between contexts.
    ///
    /// A conflict occurs when the same key binding maps to different actions
    /// in different contexts.
    #[must_use]
    pub fn find_conflicts(&self) -> Vec<BindingConflict> {
        let mut conflicts = Vec::new();
        let contexts = ActionContext::all();

        // Build reverse map: binding -> (context, action)
        let mut binding_map: HashMap<KeyBinding, Vec<(ActionContext, Action)>> = HashMap::new();

        for &context in &contexts {
            let map = self.get_context_map(context);
            for &action in all_actions() {
                for &binding in map.get_bindings(action) {
                    binding_map
                        .entry(binding)
                        .or_default()
                        .push((context, action));
                }
            }
        }

        // Find conflicts
        for (binding, mappings) in binding_map {
            if mappings.len() < 2 {
                continue;
            }
            // Check for different actions
            for i in 0..mappings.len() {
                for j in (i + 1)..mappings.len() {
                    let (ctx1, action1) = mappings[i];
                    let (ctx2, action2) = mappings[j];
                    if action1 != action2 {
                        conflicts.push(BindingConflict {
                            binding,
                            first: (ctx1, action1),
                            second: (ctx2, action2),
                        });
                    }
                }
            }
        }

        conflicts
    }

    /// Find conflicts only between currently active contexts.
    #[must_use]
    pub fn find_active_conflicts(&self) -> Vec<BindingConflict> {
        let active = self.active_contexts();
        self.find_conflicts()
            .into_iter()
            .filter(|c| active.contains(&c.first.0) && active.contains(&c.second.0))
            .collect()
    }
}

impl Default for ContextualActionMap {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Returns all action variants for iteration.
fn all_actions() -> &'static [Action] {
    &[
        Action::MoveForward,
        Action::MoveBack,
        Action::MoveLeft,
        Action::MoveRight,
        Action::Jump,
        Action::Crouch,
        Action::Sprint,
        Action::Attack,
        Action::UseItem,
        Action::Interact,
        Action::Inventory,
        Action::Pause,
        Action::Chat,
        Action::Hotbar1,
        Action::Hotbar2,
        Action::Hotbar3,
        Action::Hotbar4,
        Action::Hotbar5,
        Action::Hotbar6,
        Action::Hotbar7,
        Action::Hotbar8,
        Action::Hotbar9,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::WindowEvent;
    use winit::event::MouseButton;
    use winit::keyboard::KeyCode;

    fn make_input_with_key(key: KeyCode) -> InputState {
        let mut input = InputState::new();
        input.update(&WindowEvent::KeyboardInput { key, pressed: true });
        input
    }

    fn make_input_with_key_released(key: KeyCode) -> InputState {
        let mut input = InputState::new();
        input.update(&WindowEvent::KeyboardInput { key, pressed: true });
        input.end_frame();
        input.update(&WindowEvent::KeyboardInput {
            key,
            pressed: false,
        });
        input
    }

    #[test]
    fn test_default_context_bindings() {
        let cam = ContextualActionMap::with_defaults();
        let input = make_input_with_key(KeyCode::KeyW);

        assert!(cam.is_action_pressed(Action::MoveForward, &input));
        assert!(cam.is_action_held(Action::MoveForward, &input));
        assert_eq!(cam.active_context(), ActionContext::Walking);
    }

    #[test]
    fn test_context_priority_order() {
        assert!(ActionContext::Menu.priority() > ActionContext::Spectator.priority());
        assert!(ActionContext::Spectator.priority() > ActionContext::Vehicle.priority());
        assert!(ActionContext::Vehicle.priority() > ActionContext::Building.priority());
        assert!(ActionContext::Building.priority() > ActionContext::Walking.priority());
    }

    #[test]
    fn test_context_stack_priority_resolution() {
        let mut cam = ContextualActionMap::with_defaults();

        // Initially Walking
        assert_eq!(cam.active_context(), ActionContext::Walking);

        // Push Building - now Building is active (higher priority)
        cam.push_context(ActionContext::Building);
        assert_eq!(cam.active_context(), ActionContext::Building);

        // Push Vehicle - now Vehicle is active (higher priority)
        cam.push_context(ActionContext::Vehicle);
        assert_eq!(cam.active_context(), ActionContext::Vehicle);

        // Push Walking again - Vehicle still active (higher priority)
        cam.push_context(ActionContext::Walking);
        assert_eq!(cam.active_context(), ActionContext::Vehicle);

        // Push Menu - Menu is active (highest priority)
        cam.push_context(ActionContext::Menu);
        assert_eq!(cam.active_context(), ActionContext::Menu);
    }

    #[test]
    fn test_context_pop() {
        let mut cam = ContextualActionMap::new();

        cam.push_context(ActionContext::Building);
        cam.push_context(ActionContext::Menu);

        assert_eq!(cam.pop_context(), Some(ActionContext::Menu));
        assert_eq!(cam.active_context(), ActionContext::Building);

        assert_eq!(cam.pop_context(), Some(ActionContext::Building));
        assert_eq!(cam.active_context(), ActionContext::Walking);

        // Can't pop below fallback
        assert_eq!(cam.pop_context(), None);
        assert_eq!(cam.active_context(), ActionContext::Walking);
    }

    #[test]
    fn test_context_remove() {
        let mut cam = ContextualActionMap::new();

        cam.push_context(ActionContext::Building);
        cam.push_context(ActionContext::Menu);
        cam.push_context(ActionContext::Building); // Push Building again

        cam.remove_context(ActionContext::Building);

        // All Building instances removed
        assert!(!cam.is_context_active(ActionContext::Building));
        assert!(cam.is_context_active(ActionContext::Menu));
    }

    #[test]
    fn test_context_specific_bindings() {
        let mut cam = ContextualActionMap::new();

        // W does MoveForward in Walking
        cam.bind(ActionContext::Walking, Action::MoveForward, KeyCode::KeyW);

        // W does Attack in Vehicle
        cam.bind(ActionContext::Vehicle, Action::Attack, KeyCode::KeyW);

        let input = make_input_with_key(KeyCode::KeyW);

        // In Walking context
        assert!(cam.is_action_held(Action::MoveForward, &input));
        assert!(!cam.is_action_held(Action::Attack, &input));

        // Switch to Vehicle context
        cam.push_context(ActionContext::Vehicle);
        assert!(!cam.is_action_held(Action::MoveForward, &input));
        assert!(cam.is_action_held(Action::Attack, &input));
    }

    #[test]
    fn test_context_override() {
        let mut cam = ContextualActionMap::with_defaults();

        // Escape is Pause in Walking (from defaults)
        let input = make_input_with_key(KeyCode::Escape);
        assert!(cam.is_action_held(Action::Pause, &input));

        // In Menu context, unbind Pause and rebind Escape to Inventory
        cam.unbind(ActionContext::Menu, Action::Pause, KeyCode::Escape);
        cam.bind(ActionContext::Menu, Action::Inventory, KeyCode::Escape);

        // Push Menu
        cam.push_context(ActionContext::Menu);

        // Now Escape is Inventory, not Pause
        assert!(cam.is_action_held(Action::Inventory, &input));
        assert!(!cam.is_action_held(Action::Pause, &input));
    }

    #[test]
    fn test_conflict_detection() {
        let mut cam = ContextualActionMap::new();

        // Same key, different actions in different contexts
        cam.bind(ActionContext::Walking, Action::Jump, KeyCode::Space);
        cam.bind(ActionContext::Vehicle, Action::UseItem, KeyCode::Space);

        let conflicts = cam.find_conflicts();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].binding, KeyBinding::Key(KeyCode::Space));
    }

    #[test]
    fn test_no_conflict_same_action() {
        let mut cam = ContextualActionMap::new();

        // Same key, same action in different contexts - no conflict
        cam.bind(ActionContext::Walking, Action::Pause, KeyCode::Escape);
        cam.bind(ActionContext::Menu, Action::Pause, KeyCode::Escape);

        let conflicts = cam.find_conflicts();
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_active_conflicts_only() {
        let mut cam = ContextualActionMap::new();

        cam.bind(ActionContext::Walking, Action::Jump, KeyCode::Space);
        cam.bind(ActionContext::Building, Action::Attack, KeyCode::Space);
        cam.bind(ActionContext::Vehicle, Action::UseItem, KeyCode::Space);

        // Only Walking is active
        let conflicts = cam.find_active_conflicts();
        assert!(conflicts.is_empty());

        // Activate Building - conflict with Walking
        cam.push_context(ActionContext::Building);
        let conflicts = cam.find_active_conflicts();
        assert_eq!(conflicts.len(), 1);
    }

    #[test]
    fn test_pressed_held_released_lookups() {
        let cam = ContextualActionMap::with_defaults();
        let input = make_input_with_key(KeyCode::KeyW);

        // Pressed and held on key down
        assert!(cam.is_action_pressed(Action::MoveForward, &input));
        assert!(cam.is_action_held(Action::MoveForward, &input));
        assert!(!cam.is_action_released(Action::MoveForward, &input));

        // Released lookup
        let input_released = make_input_with_key_released(KeyCode::KeyW);
        assert!(!cam.is_action_pressed(Action::MoveForward, &input_released));
        assert!(!cam.is_action_held(Action::MoveForward, &input_released));
        assert!(cam.is_action_released(Action::MoveForward, &input_released));
    }

    #[test]
    fn test_fallback_query() {
        let mut cam = ContextualActionMap::new();

        // Bind W only in Walking
        cam.bind(ActionContext::Walking, Action::MoveForward, KeyCode::KeyW);

        // Bind Space only in Building
        cam.bind(ActionContext::Building, Action::Jump, KeyCode::Space);

        // Activate both contexts
        cam.push_context(ActionContext::Building);

        let input_w = make_input_with_key(KeyCode::KeyW);
        let input_space = make_input_with_key(KeyCode::Space);

        // Active context is Building, but Walking's bindings available via fallback
        assert!(!cam.is_action_held(Action::MoveForward, &input_w)); // Not in Building
        assert!(cam.is_action_held_with_fallback(Action::MoveForward, &input_w)); // But in fallback

        assert!(cam.is_action_held(Action::Jump, &input_space)); // In Building
        assert!(cam.is_action_held_with_fallback(Action::Jump, &input_space)); // Also via fallback
    }

    #[test]
    fn test_serialization_round_trip() {
        let mut cam = ContextualActionMap::new();

        cam.bind(ActionContext::Walking, Action::Jump, KeyCode::Space);
        cam.bind(ActionContext::Vehicle, Action::Attack, MouseButton::Left);
        cam.bind(ActionContext::Menu, Action::Pause, KeyCode::Escape);
        cam.set_fallback(ActionContext::Walking);

        // Serialize
        let serialized = ron::to_string(&cam).expect("serialize");

        // Deserialize
        let loaded: ContextualActionMap = ron::from_str(&serialized).expect("deserialize");

        // Verify bindings preserved
        let input_space = make_input_with_key(KeyCode::Space);
        assert!(loaded.is_action_held_in(ActionContext::Walking, Action::Jump, &input_space));

        let input_esc = make_input_with_key(KeyCode::Escape);
        assert!(loaded.is_action_held_in(ActionContext::Menu, Action::Pause, &input_esc));

        // Verify fallback
        assert_eq!(loaded.fallback(), ActionContext::Walking);
    }

    #[test]
    fn test_clear_contexts() {
        let mut cam = ContextualActionMap::new();

        cam.push_context(ActionContext::Building);
        cam.push_context(ActionContext::Menu);
        cam.push_context(ActionContext::Vehicle);

        cam.clear_contexts();

        assert_eq!(cam.context_stack().len(), 1);
        assert_eq!(cam.active_context(), ActionContext::Walking);
    }

    #[test]
    fn test_is_context_active() {
        let mut cam = ContextualActionMap::new();

        assert!(cam.is_context_active(ActionContext::Walking));
        assert!(!cam.is_context_active(ActionContext::Menu));

        cam.push_context(ActionContext::Menu);
        assert!(cam.is_context_active(ActionContext::Walking));
        assert!(cam.is_context_active(ActionContext::Menu));
    }

    #[test]
    fn test_active_contexts_deduped() {
        let mut cam = ContextualActionMap::new();

        cam.push_context(ActionContext::Building);
        cam.push_context(ActionContext::Building);
        cam.push_context(ActionContext::Building);

        let active = cam.active_contexts();
        assert_eq!(active.len(), 2); // Walking + Building (deduped)
    }

    #[test]
    fn test_context_specific_query() {
        let mut cam = ContextualActionMap::new();

        cam.bind(ActionContext::Vehicle, Action::Attack, KeyCode::KeyF);

        let input = make_input_with_key(KeyCode::KeyF);

        // Query specific context regardless of active
        assert!(!cam.is_action_held(Action::Attack, &input)); // Walking is active
        assert!(cam.is_action_held_in(ActionContext::Vehicle, Action::Attack, &input));
    }

    #[test]
    fn test_all_contexts_have_defaults() {
        let cam = ContextualActionMap::with_defaults();

        for context in ActionContext::all() {
            let map = cam.get_context_map(context);
            assert!(
                !map.get_bindings(Action::Pause).is_empty(),
                "{context:?} should have Pause bound"
            );
        }
    }

    #[test]
    fn test_walking_defaults_compatible_with_action_map() {
        let cam = ContextualActionMap::with_defaults();
        let standalone = ActionMap::with_defaults();
        let walking_map = cam.get_context_map(ActionContext::Walking);

        let input_w = make_input_with_key(KeyCode::KeyW);
        assert_eq!(
            standalone.is_action_held(Action::MoveForward, &input_w),
            walking_map.is_action_held(Action::MoveForward, &input_w)
        );

        let input_space = make_input_with_key(KeyCode::Space);
        assert_eq!(
            standalone.is_action_held(Action::Jump, &input_space),
            walking_map.is_action_held(Action::Jump, &input_space)
        );
    }

    #[test]
    fn test_building_has_movement_and_construction_bindings() {
        let cam = ContextualActionMap::with_defaults();
        let building = cam.get_context_map(ActionContext::Building);

        let input_w = make_input_with_key(KeyCode::KeyW);
        assert!(building.is_action_held(Action::MoveForward, &input_w));

        let input_e = make_input_with_key(KeyCode::KeyE);
        assert!(building.is_action_held(Action::Interact, &input_e));

        assert!(!building.get_bindings(Action::Hotbar1).is_empty());
    }

    #[test]
    fn test_vehicle_has_throttle_and_steering() {
        let cam = ContextualActionMap::with_defaults();
        let vehicle = cam.get_context_map(ActionContext::Vehicle);

        let input_w = make_input_with_key(KeyCode::KeyW);
        assert!(vehicle.is_action_held(Action::MoveForward, &input_w));

        let input_shift = make_input_with_key(KeyCode::ShiftLeft);
        assert!(vehicle.is_action_held(Action::Sprint, &input_shift));

        let input_e = make_input_with_key(KeyCode::KeyE);
        assert!(vehicle.is_action_held(Action::Interact, &input_e));
    }

    #[test]
    fn test_vehicle_no_attack_or_hotbar_by_default() {
        let cam = ContextualActionMap::with_defaults();
        let vehicle = cam.get_context_map(ActionContext::Vehicle);

        assert!(vehicle.get_bindings(Action::Attack).is_empty());
        assert!(vehicle.get_bindings(Action::Hotbar1).is_empty());
    }

    #[test]
    fn test_menu_has_navigation_bindings() {
        let cam = ContextualActionMap::with_defaults();
        let menu = cam.get_context_map(ActionContext::Menu);

        let input_up = make_input_with_key(KeyCode::ArrowUp);
        assert!(menu.is_action_held(Action::MoveForward, &input_up));

        let input_enter = make_input_with_key(KeyCode::Enter);
        assert!(menu.is_action_held(Action::Interact, &input_enter));

        let input_esc = make_input_with_key(KeyCode::Escape);
        assert!(menu.is_action_held(Action::Pause, &input_esc));
    }

    #[test]
    fn test_menu_no_attack_or_use_item() {
        let cam = ContextualActionMap::with_defaults();
        let menu = cam.get_context_map(ActionContext::Menu);

        assert!(menu.get_bindings(Action::Attack).is_empty());
        assert!(menu.get_bindings(Action::UseItem).is_empty());
    }

    #[test]
    fn test_spectator_has_free_camera_controls() {
        let cam = ContextualActionMap::with_defaults();
        let spectator = cam.get_context_map(ActionContext::Spectator);

        let input_space = make_input_with_key(KeyCode::Space);
        assert!(spectator.is_action_held(Action::Jump, &input_space));

        let input_ctrl = make_input_with_key(KeyCode::ControlLeft);
        assert!(spectator.is_action_held(Action::Crouch, &input_ctrl));

        let input_shift = make_input_with_key(KeyCode::ShiftLeft);
        assert!(spectator.is_action_held(Action::Sprint, &input_shift));
    }

    #[test]
    fn test_spectator_no_attack() {
        let cam = ContextualActionMap::with_defaults();
        let spectator = cam.get_context_map(ActionContext::Spectator);

        assert!(spectator.get_bindings(Action::Attack).is_empty());
    }

    #[test]
    fn test_context_specific_override_with_defaults() {
        let mut cam = ContextualActionMap::with_defaults();

        cam.push_context(ActionContext::Menu);
        let input_enter = make_input_with_key(KeyCode::Enter);
        assert!(cam.is_action_held(Action::Interact, &input_enter));

        cam.remove_context(ActionContext::Menu);
        assert!(!cam.is_action_held(Action::Interact, &input_enter));
    }
}
