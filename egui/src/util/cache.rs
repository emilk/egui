use epaint::util::hash;

const SIZE: usize = 1024; // must be small for web/WASM build (for unknown reason)

/// Very stupid/simple key-value cache. TODO: improve
#[derive(Clone)]
pub struct Cache<K, V>([Option<(K, V)>; SIZE]);

impl<K, V> Default for Cache<K, V>
where
    K: Copy,
    V: Copy,
{
    fn default() -> Self {
        Self([None; SIZE])
    }
}

impl<K, V> std::fmt::Debug for Cache<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cache")
    }
}

impl<K, V> Cache<K, V>
where
    K: std::hash::Hash + PartialEq,
{
    pub fn get(&self, key: &K) -> Option<&V> {
        let bucket = (hash(key) % (SIZE as u64)) as usize;
        match &self.0[bucket] {
            Some((k, v)) if k == key => Some(v),
            _ => None,
        }
    }

    pub fn set(&mut self, key: K, value: V) {
        let bucket = (hash(&key) % (SIZE as u64)) as usize;
        self.0[bucket] = Some((key, value));
    }
}
