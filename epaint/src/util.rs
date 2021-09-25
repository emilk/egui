/// Hash the given value with a predictable hasher.
#[inline]
pub fn hash(value: impl std::hash::Hash) -> u64 {
    hash_with(value, std::collections::hash_map::DefaultHasher::default())
}

/// Hash the given value with the given hasher.
#[inline]
pub fn hash_with(value: impl std::hash::Hash, mut hasher: impl std::hash::Hasher) -> u64 {
    value.hash(&mut hasher);
    hasher.finish()
}
