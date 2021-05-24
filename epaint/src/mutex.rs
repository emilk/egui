//! Helper module that wraps some Mutex types with different implementations.
//!
//! When the `single_threaded` feature is on the mutexes will panic when locked from different threads.

// ----------------------------------------------------------------------------

/// The lock you get from [`Mutex`].
#[cfg(feature = "multi_threaded")]
#[cfg(not(debug_assertions))]
pub use parking_lot::MutexGuard;

/// The lock you get from [`Mutex`].
#[cfg(feature = "multi_threaded")]
#[cfg(debug_assertions)]
pub struct MutexGuard<'a, T>(parking_lot::MutexGuard<'a, T>, *const ());

/// Provides interior mutability. Only thread-safe if the `multi_threaded` feature is enabled.
#[cfg(feature = "multi_threaded")]
#[derive(Default)]
pub struct Mutex<T>(parking_lot::Mutex<T>);

#[cfg(debug_assertions)]
thread_local! {
    static HELD_LOCKS_TLS: std::cell::RefCell<std::collections::HashSet<*const ()>> = std::cell::RefCell::new(std::collections::HashSet::new());
}

#[cfg(feature = "multi_threaded")]
impl<T> Mutex<T> {
    #[inline(always)]
    pub fn new(val: T) -> Self {
        Self(parking_lot::Mutex::new(val))
    }

    #[cfg(debug_assertions)]
    pub fn lock(&self) -> MutexGuard<'_, T> {
        // Detect if we are recursively taking out a lock on this mutex.

        // use a pointer to the inner data as an id for this lock
        let ptr = (&self.0 as *const parking_lot::Mutex<_>).cast::<()>();

        // Store it in thread local storage while we have a lock guard taken out
        HELD_LOCKS_TLS.with(|locks| {
            if locks.borrow().contains(&ptr) {
                panic!("Recursively locking a Mutex in the same thread is not supported")
            } else {
                locks.borrow_mut().insert(ptr);
            }
        });

        MutexGuard(self.0.lock(), ptr)
    }

    #[inline(always)]
    #[cfg(not(debug_assertions))]
    pub fn lock(&self) -> MutexGuard<'_, T> {
        self.0.lock()
    }
}

#[cfg(debug_assertions)]
#[cfg(feature = "multi_threaded")]
impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        let ptr = self.1;
        HELD_LOCKS_TLS.with(|locks| {
            locks.borrow_mut().remove(&ptr);
        });
    }
}

#[cfg(debug_assertions)]
#[cfg(feature = "multi_threaded")]
impl<T> std::ops::Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(debug_assertions)]
#[cfg(feature = "multi_threaded")]
impl<T> std::ops::DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// ---------------------

/// The lock you get from [`RwLock::read`].
#[cfg(feature = "multi_threaded")]
pub use parking_lot::RwLockReadGuard;

/// The lock you get from [`RwLock::write`].
#[cfg(feature = "multi_threaded")]
pub use parking_lot::RwLockWriteGuard;

/// Provides interior mutability. Only thread-safe if the `multi_threaded` feature is enabled.
#[cfg(feature = "multi_threaded")]
#[derive(Default)]
pub struct RwLock<T>(parking_lot::RwLock<T>);

#[cfg(feature = "multi_threaded")]
impl<T> RwLock<T> {
    #[inline(always)]
    pub fn new(val: T) -> Self {
        Self(parking_lot::RwLock::new(val))
    }

    #[inline(always)]
    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        self.0.read()
    }

    #[inline(always)]
    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        self.0.write()
    }
}

// ----------------------------------------------------------------------------
// `atomic_refcell` will panic if multiple threads try to access the same value

/// The lock you get from [`Mutex`].
#[cfg(not(feature = "multi_threaded"))]
pub use atomic_refcell::AtomicRefMut as MutexGuard;

/// Provides interior mutability. Only thread-safe if the `multi_threaded` feature is enabled.
#[cfg(not(feature = "multi_threaded"))]
#[derive(Default)]
pub struct Mutex<T>(atomic_refcell::AtomicRefCell<T>);

#[cfg(not(feature = "multi_threaded"))]
impl<T> Mutex<T> {
    #[inline(always)]
    pub fn new(val: T) -> Self {
        Self(atomic_refcell::AtomicRefCell::new(val))
    }

    /// Panics if already locked.
    #[inline(always)]
    pub fn lock(&self) -> MutexGuard<'_, T> {
        self.0.borrow_mut()
    }
}

// ---------------------

/// The lock you get from [`RwLock::read`].
#[cfg(not(feature = "multi_threaded"))]
pub use atomic_refcell::AtomicRef as RwLockReadGuard;

/// The lock you get from [`RwLock::write`].
#[cfg(not(feature = "multi_threaded"))]
pub use atomic_refcell::AtomicRefMut as RwLockWriteGuard;

/// Provides interior mutability. Only thread-safe if the `multi_threaded` feature is enabled.
#[cfg(not(feature = "multi_threaded"))]
#[derive(Default)]
pub struct RwLock<T>(atomic_refcell::AtomicRefCell<T>);

#[cfg(not(feature = "multi_threaded"))]
impl<T> RwLock<T> {
    #[inline(always)]
    pub fn new(val: T) -> Self {
        Self(atomic_refcell::AtomicRefCell::new(val))
    }

    #[inline(always)]
    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        self.0.borrow()
    }

    /// Panics if already locked.
    #[inline(always)]
    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        self.0.borrow_mut()
    }
}

// ----------------------------------------------------------------------------

impl<T> Clone for Mutex<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self::new(self.lock().clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::mutex::Mutex;
    use std::time::Duration;

    #[test]
    fn lock_two_different_mutexes_single_thread() {
        let one = Mutex::new(());
        let two = Mutex::new(());
        let _a = one.lock();
        let _b = two.lock();
    }

    #[test]
    #[should_panic]
    fn lock_reentry_single_thread() {
        let one = Mutex::new(());
        let _a = one.lock();
        let _a2 = one.lock(); // panics
    }

    #[test]
    fn lock_multiple_threads() {
        use std::sync::Arc;
        let one = Arc::new(Mutex::new(()));
        let our_lock = one.lock();
        let other_thread = {
            let one = Arc::clone(&one);
            std::thread::spawn(move || {
                let _ = one.lock();
            })
        };
        std::thread::sleep(Duration::from_millis(200));
        drop(our_lock);
        other_thread.join().unwrap();
    }
}
