/// A cache, storing some value for some length of time.
#[expect(clippy::len_without_is_empty)]
pub trait CacheTrait: 'static + Send + Sync + std::any::Any {
    /// Call once per frame to evict cache.
    fn update(&mut self);

    /// Number of values currently in the cache.
    fn len(&self) -> usize;
}
