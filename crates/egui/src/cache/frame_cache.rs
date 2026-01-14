use super::CacheTrait;

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
    pub fn evict_cache(&mut self) {
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
    pub fn get<Key>(&mut self, key: Key) -> &Value
    where
        Key: Copy + std::hash::Hash,
        Computer: ComputerMut<Key, Value>,
    {
        let hash = crate::util::hash(key);

        match self.cache.entry(hash) {
            std::collections::hash_map::Entry::Occupied(entry) => {
                let cached = entry.into_mut();
                cached.0 = self.generation;
                &cached.1
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                let value = self.computer.compute(key);
                let inserted = entry.insert((self.generation, value));
                &inserted.1
            }
        }
    }
}

impl<Value: 'static + Send + Sync, Computer: 'static + Send + Sync> CacheTrait
    for FrameCache<Value, Computer>
{
    fn update(&mut self) {
        self.evict_cache();
    }

    fn len(&self) -> usize {
        self.cache.len()
    }
}
