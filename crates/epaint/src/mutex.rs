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
#[cfg(not(feature = "deadlock_detection"))]
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

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "deadlock_detection")]
mod rw_lock_impl {
    use std::{
        ops::{Deref, DerefMut},
        sync::Arc,
        thread::ThreadId,
    };

    use ahash::HashMap;
    use parking_lot::{MappedRwLockReadGuard, MappedRwLockWriteGuard};

    /// The lock you get from [`RwLock::read`].
    pub struct RwLockReadGuard<'a, T> {
        // The option is used only because we need to `take()` the guard out of self
        // when doing remappings (`map()`), i.e. it's used as a safe `ManuallyDrop`.
        guard: Option<MappedRwLockReadGuard<'a, T>>,
        holders: Arc<parking_lot::Mutex<HashMap<ThreadId, backtrace::Backtrace>>>,
    }

    impl<'a, T> RwLockReadGuard<'a, T> {
        #[inline]
        pub fn map<U, F>(mut s: Self, f: F) -> RwLockReadGuard<'a, U>
        where
            F: FnOnce(&T) -> &U,
        {
            RwLockReadGuard {
                guard: s
                    .guard
                    .take()
                    .map(|g| parking_lot::MappedRwLockReadGuard::map(g, f)),
                holders: Arc::clone(&s.holders),
            }
        }
    }

    impl<'a, T> Deref for RwLockReadGuard<'a, T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            self.guard.as_ref().unwrap()
        }
    }

    impl<'a, T> Drop for RwLockReadGuard<'a, T> {
        fn drop(&mut self) {
            let tid = std::thread::current().id();
            self.holders.lock().remove(&tid);
        }
    }

    /// The lock you get from [`RwLock::write`].
    pub struct RwLockWriteGuard<'a, T> {
        // The option is used only because we need to `take()` the guard out of self
        // when doing remappings (`map()`), i.e. it's used as a safe `ManuallyDrop`.
        guard: Option<MappedRwLockWriteGuard<'a, T>>,
        holders: Arc<parking_lot::Mutex<HashMap<ThreadId, backtrace::Backtrace>>>,
    }

    impl<'a, T> RwLockWriteGuard<'a, T> {
        #[inline]
        pub fn map<U, F>(mut s: Self, f: F) -> RwLockWriteGuard<'a, U>
        where
            F: FnOnce(&mut T) -> &mut U,
        {
            RwLockWriteGuard {
                guard: s
                    .guard
                    .take()
                    .map(|g| parking_lot::MappedRwLockWriteGuard::map(g, f)),
                holders: Arc::clone(&s.holders),
            }
        }
    }

    impl<'a, T> Deref for RwLockWriteGuard<'a, T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            self.guard.as_ref().unwrap()
        }
    }

    impl<'a, T> DerefMut for RwLockWriteGuard<'a, T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.guard.as_mut().unwrap()
        }
    }

    impl<'a, T> Drop for RwLockWriteGuard<'a, T> {
        fn drop(&mut self) {
            let tid = std::thread::current().id();
            self.holders.lock().remove(&tid);
        }
    }

    /// Provides interior mutability.
    ///
    /// Uses `parking_lot` crate on native targets, and `atomic_refcell` on `wasm32` targets.
    #[derive(Default)]
    pub struct RwLock<T> {
        lock: parking_lot::RwLock<T>,
        // Technically we'd need a list of backtraces per thread-id since parking_lot's
        // read-locks are reentrant.
        // In practice it's not that useful to have the whole list though, so we only
        // keep track of the first backtrace for now.
        holders: Arc<parking_lot::Mutex<HashMap<ThreadId, backtrace::Backtrace>>>,
    }

    impl<T> RwLock<T> {
        pub fn new(val: T) -> Self {
            Self {
                lock: parking_lot::RwLock::new(val),
                holders: Default::default(),
            }
        }

        pub fn read(&self) -> RwLockReadGuard<'_, T> {
            let tid = std::thread::current().id();

            // If it is write-locked, and we locked it (re-entrancy deadlock)
            let would_deadlock =
                self.lock.is_locked_exclusive() && self.holders.lock().contains_key(&tid);
            assert!(
                !would_deadlock,
                "{} DEAD-LOCK DETECTED ({:?})!\n\
                    Trying to grab read-lock at:\n{}\n\
                    which is already exclusively held by current thread at:\n{}\n\n",
                std::any::type_name::<Self>(),
                tid,
                format_backtrace(&mut make_backtrace()),
                format_backtrace(self.holders.lock().get_mut(&tid).unwrap())
            );

            self.holders
                .lock()
                .entry(tid)
                .or_insert_with(make_backtrace);

            RwLockReadGuard {
                guard: parking_lot::RwLockReadGuard::map(self.lock.read(), |v| v).into(),
                holders: Arc::clone(&self.holders),
            }
        }

        pub fn write(&self) -> RwLockWriteGuard<'_, T> {
            let tid = std::thread::current().id();

            // If it is locked in any way, and we locked it (re-entrancy deadlock)
            let would_deadlock = self.lock.is_locked() && self.holders.lock().contains_key(&tid);
            assert!(
                !would_deadlock,
                "{} DEAD-LOCK DETECTED ({:?})!\n\
                    Trying to grab write-lock at:\n{}\n\
                    which is already held by current thread at:\n{}\n\n",
                std::any::type_name::<Self>(),
                tid,
                format_backtrace(&mut make_backtrace()),
                format_backtrace(self.holders.lock().get_mut(&tid).unwrap())
            );

            self.holders
                .lock()
                .entry(tid)
                .or_insert_with(make_backtrace);

            RwLockWriteGuard {
                guard: parking_lot::RwLockWriteGuard::map(self.lock.write(), |v| v).into(),
                holders: Arc::clone(&self.holders),
            }
        }
    }

    fn make_backtrace() -> backtrace::Backtrace {
        backtrace::Backtrace::new_unresolved()
    }

    fn format_backtrace(backtrace: &mut backtrace::Backtrace) -> String {
        backtrace.resolve();

        let stacktrace = format!("{:?}", backtrace);

        // Remove irrelevant parts of the stacktrace:
        let end_offset = stacktrace
            .find("std::sys_common::backtrace::__rust_begin_short_backtrace")
            .unwrap_or(stacktrace.len());
        let stacktrace = &stacktrace[..end_offset];

        let first_interesting_function = "epaint::mutex::rw_lock_impl::make_backtrace\n";
        if let Some(start_offset) = stacktrace.find(first_interesting_function) {
            stacktrace[start_offset + first_interesting_function.len()..].to_owned()
        } else {
            stacktrace.to_owned()
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

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "deadlock_detection")]
#[cfg(test)]
mod tests_rwlock {
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
