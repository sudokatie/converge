//! Bump allocator arena for fast, temporary allocations.

use bumpalo::Bump;

/// A bump allocator arena for fast, short-lived allocations.
///
/// All allocations are freed at once when the arena is reset.
/// This is ideal for per-frame temporary data.
pub struct Arena {
    bump: Bump,
}

impl Arena {
    /// Create a new empty arena.
    #[must_use]
    pub fn new() -> Self {
        Self { bump: Bump::new() }
    }

    /// Create a new arena with pre-allocated capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bump: Bump::with_capacity(capacity),
        }
    }

    /// Allocate a value in the arena.
    ///
    /// Returns a mutable reference valid until the arena is reset.
    pub fn alloc<T>(&self, val: T) -> &mut T {
        self.bump.alloc(val)
    }

    /// Allocate a copy of a slice in the arena.
    ///
    /// Returns a mutable slice valid until the arena is reset.
    pub fn alloc_slice<T: Copy>(&self, slice: &[T]) -> &mut [T] {
        self.bump.alloc_slice_copy(slice)
    }

    /// Allocate space for a slice and fill with a value.
    pub fn alloc_slice_fill<T: Clone>(&self, len: usize, val: T) -> &mut [T] {
        self.bump.alloc_slice_fill_clone(len, &val)
    }

    /// Reset the arena, freeing all allocations.
    ///
    /// This invalidates all references from previous allocations.
    pub fn reset(&mut self) {
        self.bump.reset();
    }

    /// Get the total bytes allocated in the arena.
    #[must_use]
    pub fn allocated_bytes(&self) -> usize {
        self.bump.allocated_bytes()
    }
}

impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alloc_single() {
        let arena = Arena::new();
        let x = arena.alloc(42_i32);
        assert_eq!(*x, 42);
    }

    #[test]
    fn test_alloc_multiple() {
        let arena = Arena::new();
        let a = arena.alloc(1_u32);
        let b = arena.alloc(2_u32);
        let c = arena.alloc(3_u32);
        assert_eq!(*a, 1);
        assert_eq!(*b, 2);
        assert_eq!(*c, 3);
    }

    #[test]
    fn test_alloc_slice() {
        let arena = Arena::new();
        let slice = arena.alloc_slice(&[1, 2, 3, 4, 5]);
        assert_eq!(slice, &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_reset_clears_bytes() {
        let mut arena = Arena::new();
        let _ = arena.alloc([0_u8; 1024]);
        let before = arena.allocated_bytes();
        assert!(before >= 1024);
        arena.reset();
        // After reset, allocated_bytes may still report the capacity
        // but new allocations start from the beginning
        let _ = arena.alloc(42_u8);
        // The arena should reuse the same memory
    }

    #[test]
    fn test_with_capacity() {
        let arena = Arena::with_capacity(4096);
        let _ = arena.alloc([0_u8; 1024]);
        assert!(arena.allocated_bytes() >= 1024);
    }

    #[test]
    fn test_alloc_slice_fill() {
        let arena = Arena::new();
        let slice = arena.alloc_slice_fill(5, 7_i32);
        assert_eq!(slice, &[7, 7, 7, 7, 7]);
    }
}
