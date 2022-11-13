use epaint::util::hash;

const FIXED_CACHE_SIZE: usize = 1024; // must be small for web/WASM build (for unknown reason)

/// Very stupid/simple key-value cache. TODO(emilk): improve
#[derive(Clone)]
pub(crate) struct FixedCache<K, V>([Option<(K, V)>; FIXED_CACHE_SIZE]);

impl<K, V> Default for FixedCache<K, V>
where
    K: Copy,
    V: Copy,
{
    fn default() -> Self {
        Self([None; FIXED_CACHE_SIZE])
    }
}

impl<K, V> std::fmt::Debug for FixedCache<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cache")
    }
}

impl<K, V> FixedCache<K, V>
where
    K: std::hash::Hash + PartialEq,
{
    pub fn get(&self, key: &K) -> Option<&V> {
        let bucket = (hash(key) % (FIXED_CACHE_SIZE as u64)) as usize;
        match &self.0[bucket] {
            Some((k, v)) if k == key => Some(v),
            _ => None,
        }
    }

    pub fn set(&mut self, key: K, value: V) {
        let bucket = (hash(&key) % (FIXED_CACHE_SIZE as u64)) as usize;
        self.0[bucket] = Some((key, value));
    }
}
