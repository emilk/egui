use std::hash::Hash;

use super::CacheTrait;

/// Stores a key:value pair for the duration of this frame and the next.
pub struct FramePublisher<Key: Eq + Hash, Value> {
    generation: u32,
    cache: ahash::HashMap<Key, (u32, Value)>,
}

impl<Key: Eq + Hash, Value> Default for FramePublisher<Key, Value> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Key: Eq + Hash, Value> FramePublisher<Key, Value> {
    pub fn new() -> Self {
        Self {
            generation: 0,
            cache: Default::default(),
        }
    }

    /// Publish the value. It will be available for the duration of this and the next frame.
    pub fn set(&mut self, key: Key, value: Value) {
        self.cache.insert(key, (self.generation, value));
    }

    /// Retrieve a value if it was published this or the previous frame.
    pub fn get(&self, key: &Key) -> Option<&Value> {
        self.cache.get(key).map(|(_, value)| value)
    }

    /// Must be called once per frame to clear the cache.
    pub fn evict_cache(&mut self) {
        let current_generation = self.generation;
        self.cache.retain(|_key, cached| {
            cached.0 == current_generation // only keep those that were published this frame
        });
        self.generation = self.generation.wrapping_add(1);
    }
}

impl<Key, Value> CacheTrait for FramePublisher<Key, Value>
where
    Key: 'static + Eq + Hash + Send + Sync,
    Value: 'static + Send + Sync,
{
    fn update(&mut self) {
        self.evict_cache();
    }

    fn len(&self) -> usize {
        self.cache.len()
    }
}
