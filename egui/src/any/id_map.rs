use crate::any::element::{AnyMapElement, AnyMapTrait};
use crate::Id;
use std::any::TypeId;
use std::collections::HashMap;

/// Stores any object by [`Id`].
#[derive(Clone, Debug, Default)]
pub struct AnyMapId(HashMap<Id, AnyMapElement>);

// ----------------------------------------------------------------------------

impl AnyMapId {
    pub fn get<T: AnyMapTrait>(&self, id: Id) -> Option<&T> {
        self.0.get(&id)?.get()
    }

    pub fn get_mut<T: AnyMapTrait>(&mut self, id: Id) -> Option<&mut T> {
        self.0.get_mut(&id)?.get_mut()
    }
}

impl AnyMapId {
    pub fn get_or_insert_with<T: AnyMapTrait>(
        &mut self,
        id: Id,
        or_insert_with: impl FnOnce() -> T,
    ) -> &T {
        &*self.get_mut_or_insert_with(id, or_insert_with)
    }

    pub fn get_or_default<T: AnyMapTrait + Default>(&mut self, id: Id) -> &T {
        self.get_or_insert_with(id, Default::default)
    }

    pub fn get_mut_or_insert_with<T: AnyMapTrait>(
        &mut self,
        id: Id,
        or_insert_with: impl FnOnce() -> T,
    ) -> &mut T {
        use std::collections::hash_map::Entry;
        match self.0.entry(id) {
            Entry::Vacant(vacant) => vacant
                .insert(AnyMapElement::new(or_insert_with()))
                .get_mut()
                .unwrap(), // this unwrap will never panic, because we insert correct type right now
            Entry::Occupied(occupied) => occupied.into_mut().get_mut_or_set_with(or_insert_with),
        }
    }

    pub fn get_mut_or_default<T: AnyMapTrait + Default>(&mut self, id: Id) -> &mut T {
        self.get_mut_or_insert_with(id, Default::default)
    }
}

impl AnyMapId {
    pub fn insert<T: AnyMapTrait>(&mut self, id: Id, element: T) {
        self.0.insert(id, AnyMapElement::new(element));
    }
}

impl AnyMapId {
    /// You could use this function to find is there some leak or misusage.
    pub fn count<T: AnyMapTrait>(&mut self) -> usize {
        let id = TypeId::of::<T>();
        self.0.iter().filter(|(_, v)| v.type_id() == id).count()
    }

    pub fn count_all(&mut self) -> usize {
        self.0.len()
    }

    pub fn reset<T: AnyMapTrait>(&mut self) {
        let id = TypeId::of::<T>();
        self.0.retain(|_, v| v.type_id() == id);
    }

    pub fn reset_all(&mut self) {
        self.0.clear();
    }
}
