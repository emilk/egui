//! Helper module for a Mutex that
//! detects double-locking on the same thread in debug mode.

// TODO: feature-flag for `parking_lot` vs `std::sync::Mutex`.
pub use parking_lot::MutexGuard;

#[derive(Default)]
pub struct Mutex<T>(parking_lot::Mutex<T>);

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

impl<T> Clone for Mutex<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self::new(self.lock().clone())
    }
}

// impl<T> PartialEq for Mutex<T>
// where
//     T: PartialEq,
// {
//     fn eq(&self, other: &Self) -> bool {
//         std::ptr::eq(self, other) || self.lock().eq(&other.lock())
//     }
// }
