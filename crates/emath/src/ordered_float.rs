//! Total order on floating point types.
//! Can be used for sorting, min/max computation, and other collection algorithms.

use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

/// Wraps a floating-point value to add total order and hash.
/// Possible types for `T` are `f32` and `f64`.
///
/// All NaNs are considered equal to each other.
/// The size of zero is ignored.
///
/// See also [`Float`].
#[derive(Clone, Copy)]
pub struct OrderedFloat<T>(pub T);

impl<T: Float + Copy> OrderedFloat<T> {
    #[inline]
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for OrderedFloat<T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

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
        Some(self.cmp(other))
    }
}

impl<T: Float> Ord for OrderedFloat<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        match self.0.partial_cmp(&other.0) {
            Some(ord) => ord,
            None => self.0.is_nan().cmp(&other.0.is_nan()),
        }
    }
}

impl<T: Float> Hash for OrderedFloat<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T> From<T> for OrderedFloat<T> {
    #[inline]
    fn from(val: T) -> Self {
        Self(val)
    }
}

// ----------------------------------------------------------------------------

/// Extension trait to provide `ord()` method.
///
/// Example with `f64`:
/// ```
/// use emath::Float as _;
///
/// let array = [1.0, 2.5, 2.0];
/// let max = array.iter().max_by_key(|val| val.ord());
///
/// assert_eq!(max, Some(&2.5));
/// ```
pub trait Float: PartialOrd + PartialEq + private::FloatImpl {
    /// Type to provide total order, useful as key in sorted contexts.
    fn ord(self) -> OrderedFloat<Self>
    where
        Self: Sized;
}

impl Float for f32 {
    #[inline]
    fn ord(self) -> OrderedFloat<Self> {
        OrderedFloat(self)
    }
}

impl Float for f64 {
    #[inline]
    fn ord(self) -> OrderedFloat<Self> {
        OrderedFloat(self)
    }
}

// Keep this trait in private module, to avoid exposing its methods as extensions in user code
mod private {
    use super::{Hash as _, Hasher};

    pub trait FloatImpl {
        fn is_nan(&self) -> bool;

        fn hash<H: Hasher>(&self, state: &mut H);
    }

    impl FloatImpl for f32 {
        #[inline]
        fn is_nan(&self) -> bool {
            Self::is_nan(*self)
        }

        #[inline]
        fn hash<H: Hasher>(&self, state: &mut H) {
            if *self == 0.0 {
                state.write_u8(0);
            } else if self.is_nan() {
                state.write_u8(1);
            } else {
                self.to_bits().hash(state);
            }
        }
    }

    impl FloatImpl for f64 {
        #[inline]
        fn is_nan(&self) -> bool {
            Self::is_nan(*self)
        }

        #[inline]
        fn hash<H: Hasher>(&self, state: &mut H) {
            if *self == 0.0 {
                state.write_u8(0);
            } else if self.is_nan() {
                state.write_u8(1);
            } else {
                self.to_bits().hash(state);
            }
        }
    }
}
