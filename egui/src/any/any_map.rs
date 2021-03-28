use crate::any::element::{AnyMapElement, AnyMapTrait};
use std::any::TypeId;
use std::collections::HashMap;

/// Stores object with any type.
#[derive(Clone, Debug, Default)]
pub struct AnyMap(HashMap<TypeId, AnyMapElement>);

// ----------------------------------------------------------------------------

impl AnyMap {
    pub fn get<T: AnyMapTrait>(&self) -> Option<&T> {
        self.0.get(&TypeId::of::<T>())?.get()
    }

    pub fn get_mut<T: AnyMapTrait>(&mut self) -> Option<&mut T> {
        self.0.get_mut(&TypeId::of::<T>())?.get_mut()
    }
}

impl AnyMap {
    pub fn get_or_insert_with<T: AnyMapTrait>(&mut self, or_insert_with: impl FnOnce() -> T) -> &T {
        &*self.get_mut_or_insert_with(or_insert_with)
    }

    pub fn get_or_default<T: AnyMapTrait + Default>(&mut self) -> &T {
        self.get_or_insert_with(Default::default)
    }

    pub fn get_mut_or_insert_with<T: AnyMapTrait>(
        &mut self,
        or_insert_with: impl FnOnce() -> T,
    ) -> &mut T {
        use std::collections::hash_map::Entry;
        match self.0.entry(TypeId::of::<T>()) {
            Entry::Vacant(vacant) => vacant
                .insert(AnyMapElement::new(or_insert_with()))
                .get_mut()
                .unwrap(), // this unwrap will never panic, because we insert correct type right now
            Entry::Occupied(occupied) => occupied.into_mut().get_mut_or_set_with(or_insert_with),
        }
    }

    pub fn get_mut_or_default<T: AnyMapTrait + Default>(&mut self) -> &mut T {
        self.get_mut_or_insert_with(Default::default)
    }
}

impl AnyMap {
    pub fn insert<T: AnyMapTrait>(&mut self, element: T) {
        self.0
            .insert(TypeId::of::<T>(), AnyMapElement::new(element));
    }
}

impl AnyMap {
    pub fn reset<T: AnyMapTrait>(&mut self) {
        self.0.remove(&TypeId::of::<T>());
    }

    pub fn reset_all(&mut self) {
        self.0.clear();
    }
}
