//! Wrappers around `parking_lot` locks, with a simple deadlock detection mechanism.

// ----------------------------------------------------------------------------

const DEADLOCK_DURATION: std::time::Duration = std::time::Duration::from_secs(10);

/// Provides interior mutability.
///
/// It's tailored for internal use in egui should only be used for short locks (as a guideline,
/// locks should never be held longer than a single frame). In debug builds, when a lock can't
/// be acquired within 10 seconds, we assume a deadlock and will panic.
///
/// This is a thin wrapper around [`parking_lot::Mutex`].
#[derive(Default)]
pub struct Mutex<T>(parking_lot::Mutex<T>);

/// The lock you get from [`Mutex`].
pub use parking_lot::MutexGuard;

impl<T> Mutex<T> {
    #[inline(always)]
    pub fn new(val: T) -> Self {
        Self(parking_lot::Mutex::new(val))
    }

    /// Try to acquire the lock.
    ///
    /// ## Panics
    /// Will panic in debug builds if the lock can't be acquired within 10 seconds.
    #[inline(always)]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn lock(&self) -> MutexGuard<'_, T> {
        if cfg!(debug_assertions) {
            self.0.try_lock_for(DEADLOCK_DURATION).unwrap_or_else(|| {
                panic!(
                    "DEBUG PANIC: Failed to acquire Mutex after {}s. Deadlock?",
                    DEADLOCK_DURATION.as_secs()
                )
            })
        } else {
            self.0.lock()
        }
    }
}

// ----------------------------------------------------------------------------

/// The lock you get from [`RwLock::read`].
pub use parking_lot::MappedRwLockReadGuard as RwLockReadGuard;

/// The lock you get from [`RwLock::write`].
pub use parking_lot::MappedRwLockWriteGuard as RwLockWriteGuard;

/// Provides interior mutability.
///
/// It's tailored for internal use in egui should only be used for short locks (as a guideline,
/// locks should never be held longer than a single frame). In debug builds, when a lock can't
/// be acquired within 10 seconds, we assume a deadlock and will panic.
///
/// This is a thin wrapper around [`parking_lot::RwLock`].
#[derive(Default)]
pub struct RwLock<T: ?Sized>(parking_lot::RwLock<T>);

impl<T> RwLock<T> {
    #[inline(always)]
    pub fn new(val: T) -> Self {
        Self(parking_lot::RwLock::new(val))
    }
}

impl<T: ?Sized> RwLock<T> {
    /// Try to acquire read-access to the lock.
    ///
    /// ## Panics
    /// Will panic in debug builds if the lock can't be acquired within 10 seconds.
    #[inline(always)]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        let guard = if cfg!(debug_assertions) {
            self.0.try_read_for(DEADLOCK_DURATION).unwrap_or_else(|| {
                panic!(
                    "DEBUG PANIC: Failed to acquire RwLock read after {}s. Deadlock?",
                    DEADLOCK_DURATION.as_secs()
                )
            })
        } else {
            self.0.read()
        };
        parking_lot::RwLockReadGuard::map(guard, |v| v)
    }

    /// Try to acquire write-access to the lock.
    ///
    /// ## Panics
    /// Will panic in debug builds if the lock can't be acquired within 10 seconds.
    #[inline(always)]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        let guard = if cfg!(debug_assertions) {
            self.0.try_write_for(DEADLOCK_DURATION).unwrap_or_else(|| {
                panic!(
                    "DEBUG PANIC: Failed to acquire RwLock write after {}s. Deadlock?",
                    DEADLOCK_DURATION.as_secs()
                )
            })
        } else {
            self.0.write()
        };
        parking_lot::RwLockWriteGuard::map(guard, |v| v)
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

// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![expect(clippy::disallowed_methods)] // Ok for tests

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
    fn lock_multiple_threads() {
        use std::sync::Arc;
        let one = Arc::new(Mutex::new(()));
        let our_lock = one.lock();
        let other_thread = {
            let one = Arc::clone(&one);
            std::thread::spawn(move || {
                let _lock = one.lock();
            })
        };
        std::thread::sleep(Duration::from_millis(200));
        drop(our_lock);
        other_thread.join().unwrap();
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests_rwlock {
    #![expect(clippy::disallowed_methods)] // Ok for tests

    use crate::mutex::RwLock;
    use std::time::Duration;

    #[test]
    fn lock_two_different_rwlocks_single_thread() {
        let one = RwLock::new(());
        let two = RwLock::new(());
        let _a = one.write();
        let _b = two.write();
    }

    #[test]
    fn rwlock_multiple_threads() {
        use std::sync::Arc;
        let one = Arc::new(RwLock::new(()));
        let our_lock = one.write();
        let other_thread1 = {
            let one = Arc::clone(&one);
            std::thread::spawn(move || {
                let _ = one.write();
            })
        };
        let other_thread2 = {
            let one = Arc::clone(&one);
            std::thread::spawn(move || {
                let _ = one.read();
            })
        };
        std::thread::sleep(Duration::from_millis(200));
        drop(our_lock);
        other_thread1.join().unwrap();
        other_thread2.join().unwrap();
    }

    #[test]
    #[should_panic]
    fn rwlock_write_write_reentrancy() {
        let one = RwLock::new(());
        let _a1 = one.write();
        let _a2 = one.write(); // panics
    }

    #[test]
    #[should_panic]
    fn rwlock_write_read_reentrancy() {
        let one = RwLock::new(());
        let _a1 = one.write();
        let _a2 = one.read(); // panics
    }

    #[test]
    #[should_panic]
    fn rwlock_read_write_reentrancy() {
        let one = RwLock::new(());
        let _a1 = one.read();
        let _a2 = one.write(); // panics
    }

    #[test]
    fn rwlock_read_read_reentrancy() {
        let one = RwLock::new(());
        let _a1 = one.read();
        // This is legal: this test suite specifically targets native, which relies
        // on parking_lot's rw-locks, which are reentrant.
        let _a2 = one.read();
    }

    #[test]
    fn rwlock_short_read_foreign_read_write_reentrancy() {
        use std::sync::Arc;

        let lock = Arc::new(RwLock::new(()));

        // Thread #0 grabs a read lock
        let t0r0 = lock.read();

        // Thread #1 grabs the same read lock
        let other_thread = {
            let lock = Arc::clone(&lock);
            std::thread::spawn(move || {
                let _t1r0 = lock.read();
            })
        };
        other_thread.join().unwrap();

        // Thread #0 releases its read lock
        drop(t0r0);

        // Thread #0 now grabs a write lock, which is legal
        let _t0w0 = lock.write();
    }

    #[test]
    #[should_panic]
    fn rwlock_read_foreign_read_write_reentrancy() {
        use std::sync::Arc;

        let lock = Arc::new(RwLock::new(()));

        // Thread #0 grabs a read lock
        let _t0r0 = lock.read();

        // Thread #1 grabs the same read lock
        let other_thread = {
            let lock = Arc::clone(&lock);
            std::thread::spawn(move || {
                let _t1r0 = lock.read();
            })
        };
        other_thread.join().unwrap();

        // Thread #0 now grabs a write lock, which should panic (read-write)
        let _t0w0 = lock.write(); // panics
    }
}
