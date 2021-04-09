use crate::any::element::{AnyMapElement, AnyMapTrait};
use std::hash::Hash;
use std::any::TypeId;
use std::collections::HashMap;

/// Stores any object by `Key`.
#[derive(Clone, Debug)]
pub struct AnyMap<Key: Hash + Eq>(HashMap<Key, AnyMapElement>);

impl<Key: Hash + Eq> Default for AnyMap<Key> {
    fn default() -> Self {
        AnyMap(HashMap::new())
    }
}

// ----------------------------------------------------------------------------

impl<Key: Hash + Eq> AnyMap<Key> {
    pub fn get<T: AnyMapTrait>(&self, key: &Key) -> Option<&T> {
        self.0.get(key)?.get()
    }

    pub fn get_mut<T: AnyMapTrait>(&mut self, key: &Key) -> Option<&mut T> {
        self.0.get_mut(key)?.get_mut()
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
                .insert(AnyMapElement::new(or_insert_with()))
                .get_mut()
                .unwrap(), // this unwrap will never panic, because we insert correct type right now
            Entry::Occupied(occupied) => occupied.into_mut().get_mut_or_set_with(or_insert_with),
        }
    }

    pub fn get_mut_or_default<T: AnyMapTrait + Default>(&mut self, key: Key) -> &mut T {
        self.get_mut_or_insert_with(key, Default::default)
    }
}

impl<Key: Hash + Eq> AnyMap<Key> {
    pub fn insert<T: AnyMapTrait>(&mut self, key: Key, element: T) {
        self.0.insert(key, AnyMapElement::new(element));
    }
}

impl<Key: Hash + Eq> AnyMap<Key> {
    /// You could use this function to find is there some leak or misusage.
    pub fn count<T: AnyMapTrait>(&mut self) -> usize {
        let key = TypeId::of::<T>();
        self.0.iter().filter(|(_, v)| v.type_id() == key).count()
    }

    pub fn count_all(&mut self) -> usize {
        self.0.len()
    }

    pub fn reset<T: AnyMapTrait>(&mut self) {
        let key = TypeId::of::<T>();
        self.0.retain(|_, v| v.type_id() != key);
    }

    pub fn reset_all(&mut self) {
        self.0.clear();
    }
}
