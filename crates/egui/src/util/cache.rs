//! Computing the same thing each frame can be expensive,
//! so often you want to save the result from the previous frame and reuse it.
//!
//! Enter [`FrameCache`]: it caches the results of a computation for one frame.
//! If it is still used next frame, it is not recomputed.
//! If it is not used next frame, it is evicted from the cache to save memory.

/// Something that does an expensive computation that we want to cache
/// to save us from recomputing it each frame.
pub trait ComputerMut<Key, Value>: 'static + Send + Sync {
    fn compute(&mut self, key: Key) -> Value;
}

/// Caches the results of a computation for one frame.
/// If it is still used next frame, it is not recomputed.
/// If it is not used next frame, it is evicted from the cache to save memory.
pub struct FrameCache<Value, Computer> {
    generation: u32,
    computer: Computer,
    cache: nohash_hasher::IntMap<u64, (u32, Value)>,
}

impl<Value, Computer> Default for FrameCache<Value, Computer>
where
    Computer: Default,
{
    fn default() -> Self {
        Self::new(Computer::default())
    }
}

impl<Value, Computer> FrameCache<Value, Computer> {
    pub fn new(computer: Computer) -> Self {
        Self {
            generation: 0,
            computer,
            cache: Default::default(),
        }
    }

    /// Must be called once per frame to clear the cache.
    pub fn evice_cache(&mut self) {
        let current_generation = self.generation;
        self.cache.retain(|_key, cached| {
            cached.0 == current_generation // only keep those that were used this frame
        });
        self.generation = self.generation.wrapping_add(1);
    }
}

impl<Value, Computer> FrameCache<Value, Computer> {
    /// Get from cache (if the same key was used last frame)
    /// or recompute and store in the cache.
    pub fn get<Key>(&mut self, key: Key) -> Value
    where
        Key: Copy + std::hash::Hash,
        Value: Clone,
        Computer: ComputerMut<Key, Value>,
    {
        let hash = crate::util::hash(key);

        match self.cache.entry(hash) {
            std::collections::hash_map::Entry::Occupied(entry) => {
                let cached = entry.into_mut();
                cached.0 = self.generation;
                cached.1.clone()
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                let value = self.computer.compute(key);
                entry.insert((self.generation, value.clone()));
                value
            }
        }
    }
}

#[allow(clippy::len_without_is_empty)]
pub trait CacheTrait: 'static + Send + Sync {
    /// Call once per frame to evict cache.
    fn update(&mut self);

    /// Number of values currently in the cache.
    fn len(&self) -> usize;

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

impl<Value: 'static + Send + Sync, Computer: 'static + Send + Sync> CacheTrait
    for FrameCache<Value, Computer>
{
    fn update(&mut self) {
        self.evice_cache();
    }

    fn len(&self) -> usize {
        self.cache.len()
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// ```
/// use egui::util::cache::{CacheStorage, ComputerMut, FrameCache};
///
/// #[derive(Default)]
/// struct CharCounter {}
/// impl ComputerMut<&str, usize> for CharCounter {
///     fn compute(&mut self, s: &str) -> usize {
///         s.chars().count()
///     }
/// }
/// type CharCountCache<'a> = FrameCache<usize, CharCounter>;
///
/// # let mut cache_storage = CacheStorage::default();
/// let mut cache = cache_storage.cache::<CharCountCache<'_>>();
/// assert_eq!(cache.get("hello"), 5);
/// ```
#[derive(Default)]
pub struct CacheStorage {
    caches: ahash::HashMap<std::any::TypeId, Box<dyn CacheTrait>>,
}

impl CacheStorage {
    pub fn cache<FrameCache: CacheTrait + Default>(&mut self) -> &mut FrameCache {
        self.caches
            .entry(std::any::TypeId::of::<FrameCache>())
            .or_insert_with(|| Box::new(FrameCache::default()))
            .as_any_mut()
            .downcast_mut::<FrameCache>()
            .unwrap()
    }

    /// Total number of cached values
    fn num_values(&self) -> usize {
        self.caches.values().map(|cache| cache.len()).sum()
    }

    /// Call once per frame to evict cache.
    pub fn update(&mut self) {
        for cache in self.caches.values_mut() {
            cache.update();
        }
    }
}

impl Clone for CacheStorage {
    fn clone(&self) -> Self {
        // We return an empty cache that can be filled in again.
        Self::default()
    }
}

impl std::fmt::Debug for CacheStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FrameCacheStorage[{} caches with {} elements]",
            self.caches.len(),
            self.num_values()
        )
    }
}
