//! Helper module that wraps some Mutex types with different implementations.

// ----------------------------------------------------------------------------

#[cfg(feature = "multi_threaded")]
pub use parking_lot::MutexGuard;

#[cfg(feature = "multi_threaded")]
#[derive(Default)]
pub struct Mutex<T>(parking_lot::Mutex<T>);

#[cfg(feature = "multi_threaded")]
impl<T> Mutex<T> {
    #[inline(always)]
    pub fn new(val: T) -> Self {
        Self(parking_lot::Mutex::new(val))
    }

    #[cfg(debug_assertions)]
    pub fn lock(&self) -> MutexGuard<'_, T> {
        // TODO: detect if we are trying to lock the same mutex from the same thread (bad)
        // vs locking it from another thread (fine).
        // At the moment we just panic on any double-locking of a mutex (so no multithreaded support in debug builds)
        self.0
            .try_lock()
            .expect("The Mutex is already locked. Probably a bug")
    }

    #[inline(always)]
    #[cfg(not(debug_assertions))]
    pub fn lock(&self) -> MutexGuard<'_, T> {
        self.0.lock()
    }
}

// ---------------------

#[cfg(feature = "multi_threaded")]
pub use parking_lot::{RwLockReadGuard, RwLockWriteGuard};

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

#[cfg(not(feature = "multi_threaded"))]
pub use atomic_refcell::AtomicRefMut as MutexGuard;

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

#[cfg(not(feature = "multi_threaded"))]
pub use {
    atomic_refcell::AtomicRef as RwLockReadGuard, atomic_refcell::AtomicRefMut as RwLockWriteGuard,
};

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
