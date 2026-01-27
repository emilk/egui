use super::CacheTrait;

/// A typemap of many caches, all implemented with [`CacheTrait`].
///
/// You can access egui's caches via [`crate::Memory::caches`],
/// found with [`crate::Context::memory_mut`].
///
/// ```
/// use egui::cache::{CacheStorage, ComputerMut, FrameCache};
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
/// assert_eq!(*cache.get("hello"), 5);
/// ```
#[derive(Default)]
pub struct CacheStorage {
    caches: ahash::HashMap<std::any::TypeId, Box<dyn CacheTrait>>,
}

impl CacheStorage {
    pub fn cache<Cache: CacheTrait + Default>(&mut self) -> &mut Cache {
        let cache = self
            .caches
            .entry(std::any::TypeId::of::<Cache>())
            .or_insert_with(|| Box::<Cache>::default());

        #[expect(clippy::unwrap_used)]
        (cache.as_mut() as &mut dyn std::any::Any)
            .downcast_mut::<Cache>()
            .unwrap()
    }

    /// Total number of cached values
    fn num_values(&self) -> usize {
        self.caches.values().map(|cache| cache.len()).sum()
    }

    /// Call once per frame to evict cache.
    pub fn update(&mut self) {
        self.caches.retain(|_, cache| {
            cache.update();
            cache.len() > 0
        });
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
