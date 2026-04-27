//! Collision detection primitives.

mod capsule_aabb;
pub mod ccd;
mod sphere_aabb;

pub use capsule_aabb::{Capsule, Contact, capsule_aabb_intersection};
pub use sphere_aabb::{SphereAabbContact, sphere_aabb_intersection, sphere_aabb_sweep};
