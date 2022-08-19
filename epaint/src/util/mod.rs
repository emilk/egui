mod ordered_float;

pub use ordered_float::*;

/// Hash the given value with a predictable hasher.
#[inline]
pub fn hash(value: impl std::hash::Hash) -> u64 {
    use std::hash::Hasher as _;
    let mut hasher = ahash::AHasher::default();
    value.hash(&mut hasher);
    hasher.finish()
}

/// Hash the given value with the given hasher.
#[inline]
pub fn hash_with(value: impl std::hash::Hash, mut hasher: impl std::hash::Hasher) -> u64 {
    value.hash(&mut hasher);
    hasher.finish()
}
