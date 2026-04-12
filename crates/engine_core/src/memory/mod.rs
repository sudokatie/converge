//! Memory management utilities.
//!
//! Provides arena allocators and object pools for efficient memory reuse.

mod arena;
mod pool;
mod tracking;

pub use arena::Arena;
pub use pool::{Pool, PoolHandle};
pub use tracking::MemoryTracker;
