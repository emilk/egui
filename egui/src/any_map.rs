use crate::Id;
use std::collections::HashMap;

/// This storage can store any object that implements `Any` and `'static`, but also can be cloned and serialized/deserialized. The temporary data for widgets is stored here. TODO
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
pub struct AnyMap(HashMap<Id, (AnyMapElement, SerializableTypeId)>);

/// We need this because `TypeId` can't be deserialized or serialized directly, but this can be done using hashing. However, there is small possibility that different types will have intersection by hashes of their type ids.
#[derive(Clone, Debug, Copy, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
struct SerializableTypeId(u64);

impl SerializableTypeId {
    fn new<T: AnyMapTrait>() -> Self {
        use std::any::TypeId;
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        TypeId::of::<T>().hash(&mut hasher);
        SerializableTypeId(hasher.finish())
    }
}

// ----------------------------------------------------------------------------

impl AnyMap {
    pub fn get<T: AnyMapTrait>(&mut self, id: Id) -> Option<&T> {
        self.get_mut(id).map(|x| &*x)
    }

    pub fn get_mut<T: AnyMapTrait>(&mut self, id: Id) -> Option<&mut T> {
        self.0.get_mut(&id)?.0.get_mut()
    }

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
                .insert((
                    AnyMapElement::new(or_insert_with()),
                    SerializableTypeId::new::<T>(),
                ))
                .0
                .get_mut()
                .unwrap(), // this unwrap will never panic, because we insert correct type right now
            Entry::Occupied(occupied) => occupied.into_mut().0.get_mut_or_set_with(or_insert_with),
        }
    }

    pub fn get_mut_or_default<T: AnyMapTrait + Default>(&mut self, id: Id) -> &mut T {
        self.get_mut_or_insert_with(id, Default::default)
    }

    pub fn insert<T: AnyMapTrait>(&mut self, id: Id, element: T) {
        self.0.insert(
            id,
            (AnyMapElement::new(element), SerializableTypeId::new::<T>()),
        );
    }

    pub fn count<T: AnyMapTrait>(&mut self) -> usize {
        let id = SerializableTypeId::new::<T>();
        self.0.iter().filter(|(_, v)| v.1 == id).count()
    }

    pub fn count_all(&mut self) -> usize {
        self.0.len()
    }

    pub fn reset<T: AnyMapTrait>(&mut self) {
        let id = SerializableTypeId::new::<T>();
        self.0.retain(|_, v| v.1 == id);
    }

    pub fn reset_all(&mut self) {
        self.0.clear();
    }
}

use element_impl::*;

// ----------------------------------------------------------------------------

#[cfg(feature = "persistence")]
mod element_impl {
    use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};
    use std::any::Any;
    use std::fmt;
    use AnyMapElement::{Deserialized, ToDeserialize};

    pub enum AnyMapElement {
        Deserialized {
            value: Box<dyn Any + 'static>,
            clone_fn: fn(&Box<dyn Any + 'static>) -> Box<dyn Any + 'static>,

            serialize_fn: fn(&Box<dyn Any + 'static>) -> Result<String, serde_json::Error>,
        },
        ToDeserialize(String),
    }

    impl Serialize for AnyMapElement {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            match self {
                Deserialized {
                    value,
                    serialize_fn,
                    ..
                } => {
                    let s = serialize_fn(value).map_err(serde::ser::Error::custom)?;
                    serializer.serialize_str(&s)
                }
                ToDeserialize(s) => serializer.serialize_str(s),
            }
        }
    }

    impl<'de> Deserialize<'de> for AnyMapElement {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct StrVisitor;

            impl<'de> Visitor<'de> for StrVisitor {
                type Value = String;

                fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    formatter.write_str("string that contains data json")
                }

                fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                    Ok(value.to_owned())
                }
            }

            Ok(AnyMapElement::ToDeserialize(
                deserializer.deserialize_str(StrVisitor)?,
            ))
        }
    }

    impl fmt::Debug for AnyMapElement {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Deserialized { value, .. } => f
                    .debug_struct("AnyMapElement_Deserialized")
                    .field("value_type_id", &value.type_id())
                    .finish(),
                ToDeserialize(s) => f
                    .debug_tuple("AnyMapElement_ToDeserialize")
                    .field(&s)
                    .finish(),
            }
        }
    }

    impl Clone for AnyMapElement {
        fn clone(&self) -> Self {
            match self {
                Deserialized {
                    value,
                    clone_fn,
                    serialize_fn,
                } => Deserialized {
                    value: clone_fn(value),
                    clone_fn: *clone_fn,
                    serialize_fn: *serialize_fn,
                },
                ToDeserialize(s) => ToDeserialize(s.clone()),
            }
        }
    }

    pub trait AnyMapTrait: 'static + Any + Clone + Serialize + for<'a> Deserialize<'a> {}
    impl<T: 'static + Any + Clone + Serialize + for<'a> Deserialize<'a>> AnyMapTrait for T {}

    impl AnyMapElement {
        pub fn new<T: AnyMapTrait>(t: T) -> Self {
            Deserialized {
                value: Box::new(t),
                clone_fn: |x| {
                    let x = x.downcast_ref::<T>().unwrap(); // This unwrap will never panic, because we always construct this type using this `new` function and because we return &mut reference only with this type `T`, so type cannot change.
                    Box::new(x.clone())
                },

                serialize_fn: |x| {
                    let x = x.downcast_ref::<T>().unwrap(); // This will never panic too, for same reason.
                    serde_json::to_string(x)
                },
            }
        }

        pub fn get_mut<T: AnyMapTrait>(&mut self) -> Option<&mut T> {
            match self {
                Deserialized { value, .. } => value.downcast_mut(),
                ToDeserialize(s) => {
                    *self = Self::new(serde_json::from_str::<T>(s).ok()?);
                    match self {
                        Deserialized { value, .. } => value.downcast_mut(),
                        ToDeserialize(_) => unreachable!(),
                    }
                }
            }
        }

        pub fn get_mut_or_set_with<T: AnyMapTrait>(
            &mut self,
            set_with: impl FnOnce() -> T,
        ) -> &mut T {
            match self {
                Deserialized { value, .. } => {
                    if !value.is::<T>() {
                        *self = Self::new(set_with());
                    }
                }
                ToDeserialize(s) => {
                    *self = Self::new(serde_json::from_str::<T>(s).unwrap_or_else(|_| set_with()));
                }
            }

            match self {
                Deserialized { value, .. } => value.downcast_mut().unwrap(), // This unwrap will never panic because we already converted object to required type
                ToDeserialize(_) => unreachable!(),
            }
        }
    }
}

// ----------------------------------------------------------------------------

#[cfg(not(feature = "persistence"))]
mod element_impl {
    use std::any::Any;
    use std::fmt;

    pub struct AnyMapElement {
        value: Box<dyn Any + 'static>,
        clone_fn: fn(&Box<dyn Any + 'static>) -> Box<dyn Any + 'static>,
    }

    impl fmt::Debug for AnyMapElement {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("AnyMapElement")
                .field("value_type_id", &self.value.type_id())
                .finish()
        }
    }

    impl Clone for AnyMapElement {
        fn clone(&self) -> Self {
            AnyMapElement {
                value: (self.clone_fn)(&self.value),
                clone_fn: self.clone_fn,
            }
        }
    }

    pub trait AnyMapTrait: 'static + Any + Clone {}

    impl<T: 'static + Any + Clone> AnyMapTrait for T {}

    impl AnyMapElement {
        pub fn new<T: AnyMapTrait>(t: T) -> Self {
            AnyMapElement {
                value: Box::new(t),
                clone_fn: |x| {
                    let x = x.downcast_ref::<T>().unwrap(); // This unwrap will never panic, because we always construct this type using this `new` function and because we return &mut reference only with type `T`, so type cannot change.
                    Box::new(x.clone())
                },
            }
        }

        // We have no `get -> Option<&T>` methods here, because it has no benefits compared to `get_mut`, because under `persistence` feature we will modify contents of element even with `get`.

        pub fn get_mut<T: AnyMapTrait>(&mut self) -> Option<&mut T> {
            self.value.downcast_mut()
        }

        pub fn get_mut_or_set_with<T: AnyMapTrait>(
            &mut self,
            set_with: impl FnOnce() -> T,
        ) -> &mut T {
            if !self.value.is::<T>() {
                *self = Self::new(set_with());
            }

            self.value.downcast_mut().unwrap() // This unwrap will never panic because we already converted object to required type
        }
    }
}
