//! Character controller abstraction for diverse movement modes.
//!
//! This module provides a unified character controller that supports multiple
//! movement modes: walking, swimming, climbing, zero-G, and tethered motion.
//! Each mode has configurable physics parameters and produces deterministic
//! per-tick output.
//!
//! # Architecture
//!
//! - [`MovementMode`]: Enum defining the five movement modes
//! - [`MovementConfig`]: Configurable physics parameters for each mode
//! - [`CharacterInput`]: Player input state for a single tick
//! - [`ContactState`]: Environment and surface contact information
//! - [`CharacterController`]: Main controller with physics simulation
//! - [`MovementOutput`]: Deterministic output including position, velocity, events
//! - [`TetherState`]: Optional tether attachment for constrained movement
//!
//! # Usage
//!
//! ```ignore
//! use engine_physics::character::{
//!     CharacterController, CharacterInput, ContactState, MovementConfig,
//! };
//! use glam::Vec3;
//!
//! let mut controller = CharacterController::new();
//! let input = CharacterInput::horizontal(0.0, 1.0).with_sprint(true);
//! let contact = ContactState::grounded();
//! let gravity = Vec3::new(0.0, -9.81, 0.0);
//!
//! let output = controller.update(position, &input, &contact, gravity, dt);
//! position = output.position;
//! ```
//!
//! # Movement Modes
//!
//! ## Walking
//! Standard ground-based movement with gravity, jumping, and friction.
//! Supports sprint and air control.
//!
//! ## Swimming
//! Fluid movement with buoyancy, drag, and 3D directional control.
//! Automatically activated when submerged beyond threshold depth.
//!
//! ## Climbing
//! Surface-attached movement for walls and ladders. Movement is relative
//! to the climbing surface normal.
//!
//! ## Zero-G
//! Microgravity movement with inertial damping. Push off surfaces to
//! move, with automatic velocity clamping.
//!
//! ## Tethered
//! Constrained pendulum motion on a tether. Supports reeling in/out
//! and swing physics with gravity.

mod config;
mod contact;
mod controller;
mod input;
mod mode;
mod output;
mod tether;

pub use config::{
    ClimbingConfig, MovementConfig, SwimmingConfig, TetheredConfig, WalkingConfig, ZeroGConfig,
};
pub use contact::{ContactState, EnvironmentType};
pub use controller::CharacterController;
pub use input::CharacterInput;
pub use mode::MovementMode;
pub use output::{MovementEvent, MovementOutput};
pub use tether::TetherState;
