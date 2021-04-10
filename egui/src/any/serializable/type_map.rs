use crate::any::serializable::element::{AnyMapElement, AnyMapTrait};
use crate::any::serializable::type_id::TypeId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Maps types to a single instance of that type.
///
/// Used to store state per widget type. In effect a sort of singleton storage.
/// Similar to [the `typemap` crate](https://docs.rs/typemap/0.3.3/typemap/) but allows serialization.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct TypeMap(HashMap<TypeId, AnyMapElement>);

// ----------------------------------------------------------------------------

impl TypeMap {
    pub fn get<T: AnyMapTrait>(&mut self) -> Option<&T> {
        self.get_mut().map(|x| &*x)
    }

    pub fn get_mut<T: AnyMapTrait>(&mut self) -> Option<&mut T> {
        self.0.get_mut(&TypeId::of::<T>())?.get_mut()
    }
}

impl TypeMap {
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

impl TypeMap {
    pub fn insert<T: AnyMapTrait>(&mut self, element: T) {
        self.0
            .insert(TypeId::of::<T>(), AnyMapElement::new(element));
    }

    pub fn remove<T: AnyMapTrait>(&mut self) {
        self.0.remove(&TypeId::of::<T>());
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }
}

// ----------------------------------------------------------------------------

#[test]
fn discard_different_struct() {
    #[derive(Clone, Debug, Serialize, Deserialize)]
    struct State1 {
        a: i32,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    struct State2 {
        a: String,
    }

    let file_string = {
        let mut map: TypeMap = Default::default();
        map.insert(State1 { a: 42 });
        serde_json::to_string(&map).unwrap()
    };

    let mut map: TypeMap = serde_json::from_str(&file_string).unwrap();
    assert!(map.get::<State2>().is_none());
}

#[test]
fn new_field_between_runs() {
    #[derive(Clone, Debug, Serialize, Deserialize)]
    struct State {
        a: i32,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    struct StateNew {
        a: i32,

        #[serde(default)]
        b: i32,
    }

    let file_string = {
        let mut map: TypeMap = Default::default();
        map.insert(State { a: 42 });
        serde_json::to_string(&map).unwrap()
    };

    let mut map: TypeMap = serde_json::from_str(&file_string).unwrap();
    assert!(map.get::<StateNew>().is_none());
}

// ----------------------------------------------------------------------------

#[cfg(test)]
#[test]
fn basic_usage() {
    #[derive(Debug, Clone, Eq, PartialEq, Default, Serialize, Deserialize)]
    struct State {
        a: i32,
    }

    let mut map = TypeMap::default();

    assert!(map.get::<State>().is_none());
    map.insert(State { a: 42 });
    map.insert(5i32);
    map.insert((6.0f32, -1i16));

    assert_eq!(*map.get::<State>().unwrap(), State { a: 42 });
    map.get_mut::<State>().unwrap().a = 43;
    assert_eq!(*map.get::<State>().unwrap(), State { a: 43 });

    map.remove::<State>();
    assert!(map.get::<State>().is_none());

    assert_eq!(*map.get_or_insert_with(|| State { a: 55 }), State { a: 55 });
    map.remove::<State>();
    assert_eq!(
        *map.get_mut_or_insert_with(|| State { a: 56 }),
        State { a: 56 }
    );
    map.remove::<State>();
    assert_eq!(*map.get_or_default::<State>(), State { a: 0 });
    map.remove::<State>();
    assert_eq!(*map.get_mut_or_default::<State>(), State { a: 0 });
}

#[cfg(test)]
#[test]
fn cloning() {
    #[derive(Debug, Clone, Eq, PartialEq, Default, Serialize, Deserialize)]
    struct State {
        a: i32,
    }

    let mut map: TypeMap = Default::default();

    map.insert(State::default());
    map.insert(10i32);

    let mut cloned_map = map.clone();

    map.insert(11.5f32);
    map.insert("aoeu".to_string());

    assert_eq!(*cloned_map.get::<State>().unwrap(), State { a: 0 });
    assert_eq!(*cloned_map.get::<i32>().unwrap(), 10i32);
    assert!(cloned_map.get::<f32>().is_none());
    assert!(cloned_map.get::<String>().is_none());
}

#[cfg(test)]
#[test]
fn removing() {
    #[derive(Debug, Clone, Eq, PartialEq, Default, Serialize, Deserialize)]
    struct State {
        a: i32,
    }

    let mut map: TypeMap = Default::default();

    map.insert(State::default());
    map.insert(10i32);
    map.insert(11.5f32);
    map.insert("aoeu".to_string());

    map.remove::<State>();
    assert!(map.get::<State>().is_none());
    assert!(map.get::<i32>().is_some());
    assert!(map.get::<f32>().is_some());
    assert!(map.get::<String>().is_some());

    map.clear();
    assert!(map.get::<State>().is_none());
    assert!(map.get::<i32>().is_none());
    assert!(map.get::<f32>().is_none());
    assert!(map.get::<String>().is_none());
}
