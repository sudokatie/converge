//! Survival mechanics: health, hunger, and status effects.

mod health;
mod hunger;

pub use health::{DamageSource, Health};
pub use hunger::Hunger;
