use crate::Id;
use std::collections::HashMap;

#[cfg(feature = "persistence")]
use {
    crate::any::serializable::element::{AnyMapElement, AnyMapTrait},
    crate::any::serializable::type_id::TypeId,
};

#[cfg(not(feature = "persistence"))]
use {
    crate::any::element::{AnyMapElement, AnyMapTrait},
    std::any::TypeId,
};

/// Stores any object by [`Id`], and can be de/serialized.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
pub struct AnyMapId(HashMap<Id, (AnyMapElement, TypeId)>);

// ----------------------------------------------------------------------------

impl AnyMapId {
    pub fn get<T: AnyMapTrait>(&mut self, id: Id) -> Option<&T> {
        self.get_mut(id).map(|x| &*x)
    }

    pub fn get_mut<T: AnyMapTrait>(&mut self, id: Id) -> Option<&mut T> {
        self.0.get_mut(&id)?.0.get_mut()
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
                .insert((AnyMapElement::new(or_insert_with()), TypeId::of::<T>()))
                .0
                .get_mut()
                .unwrap(), // this unwrap will never panic, because we insert correct type right now
            Entry::Occupied(occupied) => occupied.into_mut().0.get_mut_or_set_with(or_insert_with),
        }
    }

    pub fn get_mut_or_default<T: AnyMapTrait + Default>(&mut self, id: Id) -> &mut T {
        self.get_mut_or_insert_with(id, Default::default)
    }
}

impl AnyMapId {
    pub fn insert<T: AnyMapTrait>(&mut self, id: Id, element: T) {
        self.0
            .insert(id, (AnyMapElement::new(element), TypeId::of::<T>()));
    }
}

impl AnyMapId {
    /// You could use this function to find is there some leak or misusage. Note, that result of this function could broke between runs, if you upgraded Rust version or for other reasons.
    pub fn count<T: AnyMapTrait>(&mut self) -> usize {
        let id = TypeId::of::<T>();
        self.0.iter().filter(|(_, v)| v.1 == id).count()
    }

    pub fn count_all(&mut self) -> usize {
        self.0.len()
    }

    /// Note that this function could not reset all needed types between runs because, if you upgraded Rust version or for other reasons.
    pub fn reset<T: AnyMapTrait>(&mut self) {
        let id = TypeId::of::<T>();
        self.0.retain(|_, v| v.1 == id);
    }

    pub fn reset_all(&mut self) {
        self.0.clear();
    }
}
