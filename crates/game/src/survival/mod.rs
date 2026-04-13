//! Survival mechanics: health, hunger, and status effects.

mod combat;
mod health;
mod hunger;

pub use combat::{
    attempt_attack, calculate_knockback, can_attack, AttackCooldown, AttackResult, CombatStats,
};
pub use health::{DamageSource, Health};
pub use hunger::Hunger;
