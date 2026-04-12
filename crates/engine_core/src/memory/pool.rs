//! Object pool for reusable allocations.

use std::marker::PhantomData;

/// Handle to a pooled object.
///
/// The object is returned to the pool when the handle is released.
#[derive(Debug)]
pub struct PoolHandle<T> {
    index: usize,
    _marker: PhantomData<T>,
}

impl<T> PoolHandle<T> {
    /// Get the pool index of this handle.
    #[must_use]
    pub fn index(&self) -> usize {
        self.index
    }
}

/// A fixed-capacity object pool.
///
/// Pre-allocates objects that can be acquired and released.
/// Useful for frequently created/destroyed objects like particles.
pub struct Pool<T> {
    items: Vec<Option<T>>,
    free_list: Vec<usize>,
}

impl<T: Default> Pool<T> {
    /// Create a new pool with the given capacity.
    ///
    /// All slots are initialized with `T::default()`.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let mut items = Vec::with_capacity(capacity);
        let mut free_list = Vec::with_capacity(capacity);

        for i in 0..capacity {
            items.push(Some(T::default()));
            free_list.push(capacity - 1 - i); // Push in reverse so pop gives 0, 1, 2...
        }

        Self { items, free_list }
    }
}

impl<T> Pool<T> {
    /// Create a pool from an iterator of items.
    pub fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let items: Vec<Option<T>> = iter.into_iter().map(Some).collect();
        let capacity = items.len();
        let mut free_list = Vec::with_capacity(capacity);
        for i in (0..capacity).rev() {
            free_list.push(i);
        }
        Self { items, free_list }
    }

    /// Acquire an object from the pool.
    ///
    /// Returns `None` if the pool is exhausted.
    pub fn acquire(&mut self) -> Option<PoolHandle<T>> {
        self.free_list.pop().map(|index| PoolHandle {
            index,
            _marker: PhantomData,
        })
    }

    /// Release an object back to the pool.
    pub fn release(&mut self, handle: PoolHandle<T>) {
        debug_assert!(handle.index < self.items.len());
        self.free_list.push(handle.index);
    }

    /// Get a reference to a pooled object.
    #[must_use]
    pub fn get(&self, handle: &PoolHandle<T>) -> Option<&T> {
        self.items.get(handle.index).and_then(|opt| opt.as_ref())
    }

    /// Get a mutable reference to a pooled object.
    pub fn get_mut(&mut self, handle: &PoolHandle<T>) -> Option<&mut T> {
        self.items.get_mut(handle.index).and_then(|opt| opt.as_mut())
    }

    /// Get the total capacity of the pool.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.items.len()
    }

    /// Get the number of available (free) slots.
    #[must_use]
    pub fn available(&self) -> usize {
        self.free_list.len()
    }

    /// Get the number of acquired (in-use) slots.
    #[must_use]
    pub fn in_use(&self) -> usize {
        self.capacity() - self.available()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acquire_release() {
        let mut pool: Pool<i32> = Pool::new(3);
        assert_eq!(pool.available(), 3);

        let h1 = pool.acquire().unwrap();
        assert_eq!(pool.available(), 2);

        let h2 = pool.acquire().unwrap();
        assert_eq!(pool.available(), 1);

        pool.release(h1);
        assert_eq!(pool.available(), 2);

        pool.release(h2);
        assert_eq!(pool.available(), 3);
    }

    #[test]
    fn test_exhaustion() {
        let mut pool: Pool<u8> = Pool::new(2);

        let _h1 = pool.acquire().unwrap();
        let _h2 = pool.acquire().unwrap();
        assert!(pool.acquire().is_none());
    }

    #[test]
    fn test_get_mut() {
        let mut pool: Pool<String> = Pool::from_iter(vec![
            String::from("hello"),
            String::from("world"),
        ]);

        let handle = pool.acquire().unwrap();
        if let Some(s) = pool.get_mut(&handle) {
            s.push_str(" modified");
        }

        assert!(pool.get(&handle).unwrap().contains("modified"));
    }

    #[test]
    fn test_in_use() {
        let mut pool: Pool<i32> = Pool::new(5);
        assert_eq!(pool.in_use(), 0);

        let h1 = pool.acquire().unwrap();
        let h2 = pool.acquire().unwrap();
        assert_eq!(pool.in_use(), 2);

        pool.release(h1);
        assert_eq!(pool.in_use(), 1);

        pool.release(h2);
        assert_eq!(pool.in_use(), 0);
    }
}
