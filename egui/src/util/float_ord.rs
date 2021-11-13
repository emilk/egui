//! Total order on floating point types, assuming absence of NaN.
//! Can be used for sorting, min/max computation, and other collection algorithms.

use std::cmp::Ordering;

/// Totally orderable floating-point value
/// For not `f32` is supported; could be made generic if necessary.
pub(crate) struct OrderedFloat(f32);

impl Eq for OrderedFloat {}

impl PartialEq<Self> for OrderedFloat {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl PartialOrd<Self> for OrderedFloat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for OrderedFloat {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).expect("NaN or Inf not permitted")
    }
}

/// Extension trait to provide `ord` method
pub(crate) trait FloatOrd {
    /// Type to provide total order, useful as key in sorted contexts.
    fn ord(self) -> OrderedFloat;
}

impl FloatOrd for f32 {
    fn ord(self) -> OrderedFloat {
        OrderedFloat(self)
    }
}
