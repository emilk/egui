//! Total order on floating point types.
//! Can be used for sorting, min/max computation, and other collection algorithms.

use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

/// Wraps a floating-point value to add total order and hash.
/// Possible types for `T` are `f32` and `f64`.
///
/// See also [`FloatOrd`].
pub struct OrderedFloat<T>(T);

impl<T: Float> Eq for OrderedFloat<T> {}

impl<T: Float> PartialEq<Self> for OrderedFloat<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // NaNs are considered equal (equivalent) when it comes to ordering
        if self.0.is_nan() {
            other.0.is_nan()
        } else {
            self.0 == other.0
        }
    }
}

impl<T: Float> PartialOrd<Self> for OrderedFloat<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.0.partial_cmp(&other.0) {
            Some(ord) => Some(ord),
            None => Some(self.0.is_nan().cmp(&other.0.is_nan())),
        }
    }
}

impl<T: Float> Ord for OrderedFloat<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        match self.partial_cmp(other) {
            Some(ord) => ord,
            None => unreachable!(),
        }
    }
}

impl<T: Float> Hash for OrderedFloat<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

// ----------------------------------------------------------------------------

/// Extension trait to provide `ord()` method.
///
/// Example with `f64`:
/// ```
/// use epaint::util::FloatOrd;
///
/// let array = [1.0, 2.5, 2.0];
/// let max = array.iter().max_by_key(|val| val.ord());
///
/// assert_eq!(max, Some(&2.5));
/// ```
pub trait FloatOrd {
    /// Type to provide total order, useful as key in sorted contexts.
    fn ord(self) -> OrderedFloat<Self>
    where
        Self: Sized;
}

impl FloatOrd for f32 {
    #[inline]
    fn ord(self) -> OrderedFloat<f32> {
        OrderedFloat(self)
    }
}

impl FloatOrd for f64 {
    #[inline]
    fn ord(self) -> OrderedFloat<f64> {
        OrderedFloat(self)
    }
}

// ----------------------------------------------------------------------------

/// Internal abstraction over floating point types
#[doc(hidden)]
pub trait Float: PartialOrd + PartialEq + private::FloatImpl {}

impl Float for f32 {}

impl Float for f64 {}

// Keep this trait in private module, to avoid exposing its methods as extensions in user code
mod private {
    use super::*;

    pub trait FloatImpl {
        fn is_nan(&self) -> bool;

        fn hash<H: Hasher>(&self, state: &mut H);
    }

    impl FloatImpl for f32 {
        #[inline]
        fn is_nan(&self) -> bool {
            f32::is_nan(*self)
        }

        #[inline]
        fn hash<H: Hasher>(&self, state: &mut H) {
            crate::f32_hash(state, *self);
        }
    }

    impl FloatImpl for f64 {
        #[inline]
        fn is_nan(&self) -> bool {
            f64::is_nan(*self)
        }

        #[inline]
        fn hash<H: Hasher>(&self, state: &mut H) {
            crate::f64_hash(state, *self);
        }
    }
}
