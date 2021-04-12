use crate::any::serializable::element::{AnyMapElement, AnyMapTrait};
use crate::any::serializable::type_id::TypeId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;

/// Stores any object by `Key`, and can be de/serialized.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AnyMap<Key: Hash + Eq>(HashMap<Key, AnyMapElement>);

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

    pub fn remove(&mut self, key: &Key) {
        self.0.remove(key);
    }

    /// Note that this function could not remove all needed types between runs because if you upgraded the Rust version or for other reasons.
    pub fn remove_by_type<T: AnyMapTrait>(&mut self) {
        let key = TypeId::of::<T>();
        self.0.retain(|_, v| v.type_id() != key);
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }
}

impl<Key: Hash + Eq> AnyMap<Key> {
    /// You could use this function to find is there some leak or misusage. Note, that result of this function could break between runs, if you upgraded the Rust version or for other reasons.
    pub fn count<T: AnyMapTrait>(&mut self) -> usize {
        let key = TypeId::of::<T>();
        self.0.iter().filter(|(_, v)| v.type_id() == key).count()
    }

    pub fn count_all(&mut self) -> usize {
        self.0.len()
    }
}

// ----------------------------------------------------------------------------

#[test]
fn discard_different_struct() {
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    struct State1 {
        a: i32,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    struct State2 {
        b: String,
    }

    let file_string = {
        let mut map: AnyMap<i32> = Default::default();
        map.insert(1, State1 { a: 42 });
        serde_json::to_string(&map).unwrap()
    };

    let mut map: AnyMap<i32> = serde_json::from_str(&file_string).unwrap();
    assert!(map.get::<State2>(&1).is_none());
    assert_eq!(map.get::<State1>(&1), Some(&State1 { a: 42 }));
}

#[test]
fn new_field_between_runs() {
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Serialize, Deserialize)]
    struct State {
        a: i32,
    }

    #[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
    struct StateNew {
        a: i32,

        #[serde(default)]
        b: String,
    }

    let file_string = {
        let mut map: AnyMap<i32> = Default::default();
        map.insert(1, State { a: 42 });
        serde_json::to_string(&map).unwrap()
    };

    let mut map: AnyMap<i32> = serde_json::from_str(&file_string).unwrap();
    assert_eq!(
        map.get::<StateNew>(&1),
        Some(&StateNew {
            a: 42,
            b: String::default()
        })
    );
}

// ----------------------------------------------------------------------------

#[test]
fn basic_usage() {
    #[derive(Debug, Clone, Eq, PartialEq, Default, Deserialize, Serialize)]
    struct State {
        a: i32,
    }

    let mut map: AnyMap<i32> = Default::default();

    assert!(map.get::<State>(&0).is_none());
    map.insert(0, State { a: 42 });

    assert_eq!(*map.get::<State>(&0).unwrap(), State { a: 42 });
    assert!(map.get::<State>(&1).is_none());
    map.get_mut::<State>(&0).unwrap().a = 43;
    assert_eq!(*map.get::<State>(&0).unwrap(), State { a: 43 });

    map.remove(&0);
    assert!(map.get::<State>(&0).is_none());

    assert_eq!(
        *map.get_or_insert_with(0, || State { a: 55 }),
        State { a: 55 }
    );
    map.remove(&0);
    assert_eq!(
        *map.get_mut_or_insert_with(0, || State { a: 56 }),
        State { a: 56 }
    );
    map.remove(&0);
    assert_eq!(*map.get_or_default::<State>(0), State { a: 0 });
    map.remove(&0);
    assert_eq!(*map.get_mut_or_default::<State>(0), State { a: 0 });
}

#[test]
fn different_type_same_id() {
    #[derive(Debug, Clone, Eq, PartialEq, Default, Deserialize, Serialize)]
    struct State {
        a: i32,
    }

    let mut map: AnyMap<i32> = Default::default();

    map.insert(0, State { a: 42 });

    assert_eq!(*map.get::<State>(&0).unwrap(), State { a: 42 });
    assert!(map.get::<i32>(&0).is_none());

    map.insert(0, 255i32);

    assert_eq!(*map.get::<i32>(&0).unwrap(), 255);
    assert!(map.get::<State>(&0).is_none());
}

#[test]
fn cloning() {
    #[derive(Debug, Clone, Eq, PartialEq, Default, Deserialize, Serialize)]
    struct State {
        a: i32,
    }

    let mut map: AnyMap<i32> = Default::default();

    map.insert(0, State::default());
    map.insert(10, 10i32);
    map.insert(11, 11i32);

    let mut cloned_map = map.clone();

    map.insert(12, 12i32);
    map.insert(1, State { a: 10 });

    assert_eq!(*cloned_map.get::<State>(&0).unwrap(), State { a: 0 });
    assert!(cloned_map.get::<State>(&1).is_none());
    assert_eq!(*cloned_map.get::<i32>(&10).unwrap(), 10i32);
    assert_eq!(*cloned_map.get::<i32>(&11).unwrap(), 11i32);
    assert!(cloned_map.get::<i32>(&12).is_none());
}

#[test]
fn counting() {
    #[derive(Debug, Clone, Eq, PartialEq, Default, Deserialize, Serialize)]
    struct State {
        a: i32,
    }

    let mut map: AnyMap<i32> = Default::default();

    map.insert(0, State::default());
    map.insert(1, State { a: 10 });
    map.insert(10, 10i32);
    map.insert(11, 11i32);
    map.insert(12, 12i32);

    assert_eq!(map.count::<State>(), 2);
    assert_eq!(map.count::<i32>(), 3);

    map.remove_by_type::<State>();

    assert_eq!(map.count::<State>(), 0);
    assert_eq!(map.count::<i32>(), 3);

    map.clear();

    assert_eq!(map.count::<State>(), 0);
    assert_eq!(map.count::<i32>(), 0);
}
