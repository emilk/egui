//! Helper module that wraps some Mutex types with different implementations.

// ----------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
#[cfg(not(debug_assertions))]
mod mutex_impl {
    /// Provides interior mutability.
    ///
    /// Uses `parking_lot` crate on native targets, and `atomic_refcell` on `wasm32` targets.
    #[derive(Default)]
    pub struct Mutex<T>(parking_lot::Mutex<T>);

    /// The lock you get from [`Mutex`].
    pub use parking_lot::MutexGuard;

    impl<T> Mutex<T> {
        #[inline(always)]
        pub fn new(val: T) -> Self {
            Self(parking_lot::Mutex::new(val))
        }

        #[inline(always)]
        pub fn lock(&self) -> MutexGuard<'_, T> {
            self.0.lock()
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(debug_assertions)]
mod mutex_impl {
    /// Provides interior mutability.
    ///
    /// Uses `parking_lot` crate on native targets, and `atomic_refcell` on `wasm32` targets.
    #[derive(Default)]
    pub struct Mutex<T>(parking_lot::Mutex<T>);

    /// The lock you get from [`Mutex`].
    pub struct MutexGuard<'a, T>(parking_lot::MutexGuard<'a, T>, *const ());

    #[derive(Default)]
    struct HeldLocks(Vec<*const ()>);

    impl HeldLocks {
        #[inline(always)]
        fn insert(&mut self, lock: *const ()) {
            // Very few locks will ever be held at the same time, so a linear search is fast
            assert!(
                !self.0.contains(&lock),
                "Recursively locking a Mutex in the same thread is not supported"
            );
            self.0.push(lock);
        }

        #[inline(always)]
        fn remove(&mut self, lock: *const ()) {
            self.0.retain(|&ptr| ptr != lock);
        }
    }

    thread_local! {
        static HELD_LOCKS_TLS: std::cell::RefCell<HeldLocks> = Default::default();
    }

    impl<T> Mutex<T> {
        #[inline(always)]
        pub fn new(val: T) -> Self {
            Self(parking_lot::Mutex::new(val))
        }

        pub fn lock(&self) -> MutexGuard<'_, T> {
            // Detect if we are recursively taking out a lock on this mutex.

            // use a pointer to the inner data as an id for this lock
            let ptr = (&self.0 as *const parking_lot::Mutex<_>).cast::<()>();

            // Store it in thread local storage while we have a lock guard taken out
            HELD_LOCKS_TLS.with(|held_locks| {
                held_locks.borrow_mut().insert(ptr);
            });

            MutexGuard(self.0.lock(), ptr)
        }
    }

    impl<T> Drop for MutexGuard<'_, T> {
        fn drop(&mut self) {
            let ptr = self.1;
            HELD_LOCKS_TLS.with(|held_locks| {
                held_locks.borrow_mut().remove(ptr);
            });
        }
    }

    impl<T> std::ops::Deref for MutexGuard<'_, T> {
        type Target = T;

        #[inline(always)]
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<T> std::ops::DerefMut for MutexGuard<'_, T> {
        #[inline(always)]
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod rw_lock_impl {
    /// The lock you get from [`RwLock::read`].
    pub use parking_lot::MappedRwLockReadGuard as RwLockReadGuard;

    /// The lock you get from [`RwLock::write`].
    pub use parking_lot::MappedRwLockWriteGuard as RwLockWriteGuard;

    /// Provides interior mutability.
    ///
    /// Uses `parking_lot` crate on native targets, and `atomic_refcell` on `wasm32` targets.
    #[derive(Default)]
    pub struct RwLock<T>(parking_lot::RwLock<T>);

    impl<T> RwLock<T> {
        #[inline(always)]
        pub fn new(val: T) -> Self {
            Self(parking_lot::RwLock::new(val))
        }

        #[inline(always)]
        pub fn read(&self) -> RwLockReadGuard<'_, T> {
            parking_lot::RwLockReadGuard::map(self.0.read(), |v| v)
        }

        #[inline(always)]
        pub fn write(&self) -> RwLockWriteGuard<'_, T> {
            parking_lot::RwLockWriteGuard::map(self.0.write(), |v| v)
        }
    }
}

// ----------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
mod mutex_impl {
    // `atomic_refcell` will panic if multiple threads try to access the same value

    /// Provides interior mutability.
    ///
    /// Uses `parking_lot` crate on native targets, and `atomic_refcell` on `wasm32` targets.
    #[derive(Default)]
    pub struct Mutex<T>(atomic_refcell::AtomicRefCell<T>);

    /// The lock you get from [`Mutex`].
    pub use atomic_refcell::AtomicRefMut as MutexGuard;

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
}

#[cfg(target_arch = "wasm32")]
mod rw_lock_impl {
    // `atomic_refcell` will panic if multiple threads try to access the same value

    /// The lock you get from [`RwLock::read`].
    pub use atomic_refcell::AtomicRef as RwLockReadGuard;

    /// The lock you get from [`RwLock::write`].
    pub use atomic_refcell::AtomicRefMut as RwLockWriteGuard;

    /// Provides interior mutability.
    ///
    /// Uses `parking_lot` crate on native targets, and `atomic_refcell` on `wasm32` targets.
    #[derive(Default)]
    pub struct RwLock<T>(atomic_refcell::AtomicRefCell<T>);

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
}

// ----------------------------------------------------------------------------

pub use mutex_impl::{Mutex, MutexGuard};
pub use rw_lock_impl::{RwLock, RwLockReadGuard, RwLockWriteGuard};

impl<T> Clone for Mutex<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self::new(self.lock().clone())
    }
}

// ----------------------------------------------------------------------------

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
