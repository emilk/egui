//! Total order on floating point types, assuming absence of NaN.
//! Can be used for sorting, min/max computation, and other collection algorithms.

use std::cmp::Ordering;

/// Totally orderable floating-point value
/// For not `f32` is supported; could be made generic if necessary.
pub(crate) struct OrderedFloat(f32);

impl Eq for OrderedFloat {}

impl PartialEq<Self> for OrderedFloat {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // NaNs are considered equal (equivalent when it comes to ordering
        if self.0.is_nan() {
            other.0.is_nan()
        } else {
            self.0 == other.0
        }
    }
}

impl PartialOrd<Self> for OrderedFloat {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.0.partial_cmp(&other.0) {
            Some(ord) => Some(ord),
            None => Some(self.0.is_nan().cmp(&other.0.is_nan())),
        }
    }
}

impl Ord for OrderedFloat {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        match self.partial_cmp(other) {
            Some(ord) => ord,
            None => unreachable!(),
        }
    }
}

/// Extension trait to provide `ord` method
pub(crate) trait FloatOrd {
    /// Type to provide total order, useful as key in sorted contexts.
    fn ord(self) -> OrderedFloat;
}

impl FloatOrd for f32 {
    #[inline]
    fn ord(self) -> OrderedFloat {
        OrderedFloat(self)
    }
}
