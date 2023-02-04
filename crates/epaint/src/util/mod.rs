mod ordered_float;

pub use ordered_float::*;

/// Hash the given value with a predictable hasher.
#[inline]
pub fn hash(value: impl std::hash::Hash) -> u64 {
    ahash::RandomState::with_seeds(1, 2, 3, 4).hash_one(value)
}

/// Hash the given value with the given hasher.
#[inline]
pub fn hash_with(value: impl std::hash::Hash, mut hasher: impl std::hash::Hasher) -> u64 {
    value.hash(&mut hasher);
    hasher.finish()
}
