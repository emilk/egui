use std::hash::Hash;
use std::collections::HashMap;
use crate::any::serializable::usages::*;

/// Stores any object by `Key`, and can be de/serialized.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
pub struct AnyMap<Key: Hash + Eq>(HashMap<Key, (AnyMapElement, TypeId)>);

impl<Key: Hash + Eq> Default for AnyMap<Key> {
    fn default() -> Self {
        AnyMap(HashMap::new())
    }
}

// ----------------------------------------------------------------------------

impl<Key: Hash + Eq> AnyMap<Key> {
    pub fn get<T: AnyMapTrait>(&mut self, key: &Key) -> Option<&T> {
        self.get_mut(key).map(|x| &*x)
    }

    pub fn get_mut<T: AnyMapTrait>(&mut self, key: &Key) -> Option<&mut T> {
        self.0.get_mut(key)?.0.get_mut()
    }
}

impl<Key: Hash + Eq> AnyMap<Key> {
    pub fn get_or_insert_with<T: AnyMapTrait>(
        &mut self,
        key: Key,
        or_insert_with: impl FnOnce() -> T,
    ) -> &T {
        &*self.get_mut_or_insert_with(key, or_insert_with)
    }

    pub fn get_or_default<T: AnyMapTrait + Default>(&mut self, key: Key) -> &T {
        self.get_or_insert_with(key, Default::default)
    }

    pub fn get_mut_or_insert_with<T: AnyMapTrait>(
        &mut self,
        key: Key,
        or_insert_with: impl FnOnce() -> T,
    ) -> &mut T {
        use std::collections::hash_map::Entry;
        match self.0.entry(key) {
            Entry::Vacant(vacant) => vacant
                .insert((AnyMapElement::new(or_insert_with()), TypeId::of::<T>()))
                .0
                .get_mut()
                .unwrap(), // this unwrap will never panic, because we insert correct type right now
            Entry::Occupied(occupied) => occupied.into_mut().0.get_mut_or_set_with(or_insert_with),
        }
    }

    pub fn get_mut_or_default<T: AnyMapTrait + Default>(&mut self, key: Key) -> &mut T {
        self.get_mut_or_insert_with(key, Default::default)
    }
}

impl<Key: Hash + Eq> AnyMap<Key> {
    pub fn insert<T: AnyMapTrait>(&mut self, key: Key, element: T) {
        self.0
            .insert(key, (AnyMapElement::new(element), TypeId::of::<T>()));
    }
}

impl<Key: Hash + Eq> AnyMap<Key> {
    /// You could use this function to find is there some leak or misusage. Note, that result of this function could break between runs, if you upgraded the Rust version or for other reasons.
    pub fn count<T: AnyMapTrait>(&mut self) -> usize {
        let key = TypeId::of::<T>();
        self.0.iter().filter(|(_, v)| v.1 == key).count()
    }

    pub fn count_all(&mut self) -> usize {
        self.0.len()
    }

    /// Note that this function could not reset all needed types between runs because if you upgraded the Rust version or for other reasons.
    pub fn reset<T: AnyMapTrait>(&mut self) {
        let key = TypeId::of::<T>();
        self.0.retain(|_, v| v.1 != key);
    }

    pub fn reset_all(&mut self) {
        self.0.clear();
    }
}
