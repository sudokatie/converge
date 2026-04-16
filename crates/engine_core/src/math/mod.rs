//! Math primitives for the Lattice engine.
//!
//! Provides geometric primitives, intersection tests, and frustum culling.

mod aabb;
mod frustum;
mod intersect;
mod obb;
mod plane;
mod ray;
mod sphere;

pub use aabb::Aabb;
pub use frustum::{Containment, Frustum};
pub use intersect::Intersection;
pub use obb::Obb;
pub use plane::Plane;
pub use ray::Ray;
pub use sphere::Sphere;
