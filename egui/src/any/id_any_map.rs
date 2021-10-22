// TODO: it is possible we can simplify `Element` further by
// assuming everything is "faalible" serializable, and by supplying serialize/deserialize functions for them.
// For non-serializable types, these simply return `None`.
// This will also allow users to pick their own serialization format per type.

use std::any::Any;

// -----------------------------------------------------------------------------------------------

/// We need this because `TypeId` can't be deserialized or serialized directly, but this can be done using hashing. However, there is a small possibility that different types will have intersection by hashes of their type ids.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TypeId(u64);

impl TypeId {
    #[inline]
    pub fn of<T: Any + 'static>() -> Self {
        std::any::TypeId::of::<T>().into()
    }

    #[inline(always)]
    pub(crate) fn value(&self) -> u64 {
        self.0
    }
}

impl From<std::any::TypeId> for TypeId {
    #[inline]
    fn from(id: std::any::TypeId) -> Self {
        Self(epaint::util::hash(id))
    }
}

// -----------------------------------------------------------------------------------------------

#[cfg(feature = "serde")]
pub trait SerializableAny:
    'static + Any + Clone + serde::Serialize + for<'a> serde::Deserialize<'a> + Send + Sync
{
}

#[cfg(feature = "serde")]
impl<T> SerializableAny for T where
    T: 'static + Any + Clone + serde::Serialize + for<'a> serde::Deserialize<'a> + Send + Sync
{
}

#[cfg(not(feature = "serde"))]
pub trait SerializableAny: 'static + Any + Clone + for<'a> Send + Sync {}

#[cfg(not(feature = "serde"))]
impl<T> SerializableAny for T where T: 'static + Any + Clone + for<'a> Send + Sync {}

// -----------------------------------------------------------------------------------------------

#[cfg(feature = "persistence")]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
struct SerializedElement {
    type_id: TypeId,
    ron: String,
}

enum Element {
    /// Serializable data
    Value {
        value: Box<dyn Any + 'static + Send + Sync>,
        clone_fn: fn(&Box<dyn Any + 'static + Send + Sync>) -> Box<dyn Any + 'static + Send + Sync>,

        // None if non-serializable type.
        #[cfg(feature = "persistence")]
        serialize_fn:
            Option<fn(&Box<dyn Any + 'static + Send + Sync>) -> Result<String, ron::Error>>,
    },
    Serialized {
        type_id: TypeId,
        ron: String,
    },
}

impl Clone for Element {
    fn clone(&self) -> Self {
        match &self {
            Self::Value {
                value,
                clone_fn,
                #[cfg(feature = "persistence")]
                serialize_fn,
            } => Self::Value {
                value: clone_fn(value),
                clone_fn: *clone_fn,
                #[cfg(feature = "persistence")]
                serialize_fn: *serialize_fn,
            },

            Self::Serialized { type_id, ron } => Self::Serialized {
                type_id: *type_id,
                ron: ron.clone(),
            },
        }
    }
}

impl std::fmt::Debug for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::Value { value, .. } => f
                .debug_struct("MaybeSerializable::Temporary")
                .field("value_type_id", &value.type_id())
                .finish_non_exhaustive(),
            Self::Serialized { type_id, ron } => f
                .debug_struct("MaybeSerializable::Temporary")
                .field("type_id", &type_id)
                .field("ron", &ron)
                .finish(),
        }
    }
}

impl Element {
    #[inline]
    pub(crate) fn new_temp<T: 'static + Any + Clone + Send + Sync>(t: T) -> Self {
        Self::Value {
            value: Box::new(t),
            clone_fn: |x| {
                let x = x.downcast_ref::<T>().unwrap(); // This unwrap will never panic, because we always construct this type using this `new` function and because we return &mut reference only with this type `T`, so type cannot change.
                Box::new(x.clone())
            },
            #[cfg(feature = "persistence")]
            serialize_fn: None,
        }
    }

    #[inline]
    pub(crate) fn new_persisted<T: SerializableAny>(t: T) -> Self {
        Self::Value {
            value: Box::new(t),
            clone_fn: |x| {
                let x = x.downcast_ref::<T>().unwrap(); // This unwrap will never panic, because we always construct this type using this `new` function and because we return &mut reference only with this type `T`, so type cannot change.
                Box::new(x.clone())
            },
            #[cfg(feature = "persistence")]
            serialize_fn: Some(|x| {
                let x = x.downcast_ref::<T>().unwrap(); // This will never panic too, for same reason.
                ron::to_string(x)
            }),
        }
    }

    #[inline]
    pub(crate) fn type_id(&self) -> TypeId {
        match self {
            Self::Value { value, .. } => (**value).type_id().into(),
            Self::Serialized { type_id, .. } => *type_id,
        }
    }

    #[inline]
    pub(crate) fn get_mut_temp<T: 'static>(&mut self) -> Option<&mut T> {
        match self {
            Self::Value { value, .. } => value.downcast_mut(),
            Self::Serialized { .. } => None,
        }
    }

    #[inline]
    pub(crate) fn get_temp_mut_or_insert_with<T: 'static + Any + Clone + Send + Sync>(
        &mut self,
        insert_with: impl FnOnce() -> T,
    ) -> &mut T {
        match self {
            Self::Value { value, .. } => {
                if !value.is::<T>() {
                    *self = Self::new_temp(insert_with());
                }
            }
            Self::Serialized { .. } => {
                *self = Self::new_temp(insert_with());
            }
        }

        match self {
            Self::Value { value, .. } => value.downcast_mut().unwrap(), // This unwrap will never panic because we already converted object to required type
            Self::Serialized { .. } => unreachable!(),
        }
    }

    #[inline]
    pub(crate) fn get_persisted_mut_or_insert_with<T: SerializableAny>(
        &mut self,
        insert_with: impl FnOnce() -> T,
    ) -> &mut T {
        match self {
            Self::Value { value, .. } => {
                if !value.is::<T>() {
                    *self = Self::new_persisted(insert_with());
                }
            }

            #[cfg(feature = "persistence")]
            Self::Serialized { ron, .. } => {
                *self = Self::new_persisted(from_ron_str::<T>(ron).unwrap_or_else(insert_with));
            }

            #[cfg(not(feature = "persistence"))]
            Self::Serialized { .. } => {
                *self = Self::new_persisted(insert_with());
            }
        }

        match self {
            Self::Value { value, .. } => value.downcast_mut().unwrap(), // This unwrap will never panic because we already converted object to required type
            Self::Serialized { .. } => unreachable!(),
        }
    }

    pub(crate) fn get_mut_persisted<T: SerializableAny>(&mut self) -> Option<&mut T> {
        match self {
            Self::Value { value, .. } => value.downcast_mut(),

            #[cfg(feature = "persistence")]
            Self::Serialized { ron, .. } => {
                *self = Self::new_persisted(from_ron_str::<T>(ron)?);

                match self {
                    Self::Value { value, .. } => value.downcast_mut(),
                    Self::Serialized { .. } => unreachable!(),
                }
            }

            #[cfg(not(feature = "persistence"))]
            Self::Serialized { .. } => None,
        }
    }

    #[cfg(feature = "persistence")]
    fn to_serialize(&self) -> Option<SerializedElement> {
        match self {
            Self::Value {
                value,
                serialize_fn,
                ..
            } => {
                if let Some(serialize_fn) = serialize_fn {
                    let ron = serialize_fn(value).ok()?;
                    Some(SerializedElement {
                        type_id: (**value).type_id().into(),
                        ron,
                    })
                } else {
                    None
                }
            }
            Self::Serialized { type_id, ron } => Some(SerializedElement {
                type_id: *type_id,
                ron: ron.clone(),
            }),
        }
    }
}

#[cfg(feature = "persistence")]
fn from_ron_str<T: serde::de::DeserializeOwned>(ron: &str) -> Option<T> {
    match ron::from_str::<T>(ron) {
        Ok(value) => Some(value),
        Err(err) => {
            eprintln!(
                "egui: Failed to deserialize {} from memory: {}, ron: {:?}",
                std::any::type_name::<T>(),
                err,
                ron
            );
            None
        }
    }
}

// -----------------------------------------------------------------------------------------------

use crate::Id;

// TODO: make generic over the key, instead of using hard-coded `Id`.
/// Stores any value grouped by [`Id`] and [`TypeId`].
/// Values can optionally be serializable.
#[derive(Clone, Debug, Default)]
pub struct IdAnyMap(nohash_hasher::IntMap<u64, Element>);

impl IdAnyMap {
    #[inline]
    pub fn get_temp<T: 'static + Clone>(&mut self, id: Id) -> Option<T> {
        let hash = hash(TypeId::of::<T>(), id);
        self.0
            .get_mut(&hash)
            .and_then(|x| x.get_mut_temp())
            .cloned()
    }

    #[inline]
    pub fn get_persisted<T: SerializableAny>(&mut self, id: Id) -> Option<T> {
        let hash = hash(TypeId::of::<T>(), id);
        self.0
            .get_mut(&hash)
            .and_then(|x| x.get_mut_persisted())
            .cloned()
    }

    #[inline]
    pub fn get_temp_mut_or<T: 'static + Any + Clone + Send + Sync>(
        &mut self,
        id: Id,
        or_insert: T,
    ) -> &mut T {
        self.get_temp_mut_or_insert_with(id, || or_insert)
    }

    #[inline]
    pub fn get_persisted_mut_or<T: SerializableAny>(&mut self, id: Id, or_insert: T) -> &mut T {
        self.get_persisted_mut_or_insert_with(id, || or_insert)
    }

    #[inline]
    pub fn get_temp_mut_or_default<T: 'static + Any + Clone + Send + Sync + Default>(
        &mut self,
        id: Id,
    ) -> &mut T {
        self.get_temp_mut_or_insert_with(id, Default::default)
    }

    #[inline]
    pub fn get_persisted_mut_or_default<T: SerializableAny + Default>(&mut self, id: Id) -> &mut T {
        self.get_persisted_mut_or_insert_with(id, Default::default)
    }

    pub fn get_temp_mut_or_insert_with<T: 'static + Any + Clone + Send + Sync>(
        &mut self,
        id: Id,
        insert_with: impl FnOnce() -> T,
    ) -> &mut T {
        let hash = hash(TypeId::of::<T>(), id);
        use std::collections::hash_map::Entry;
        match self.0.entry(hash) {
            Entry::Vacant(vacant) => vacant
                .insert(Element::new_temp(insert_with()))
                .get_mut_temp()
                .unwrap(), // this unwrap will never panic, because we insert correct type right now
            Entry::Occupied(occupied) => {
                occupied.into_mut().get_temp_mut_or_insert_with(insert_with)
            }
        }
    }

    pub fn get_persisted_mut_or_insert_with<T: SerializableAny>(
        &mut self,
        id: Id,
        insert_with: impl FnOnce() -> T,
    ) -> &mut T {
        let hash = hash(TypeId::of::<T>(), id);
        use std::collections::hash_map::Entry;
        match self.0.entry(hash) {
            Entry::Vacant(vacant) => vacant
                .insert(Element::new_persisted(insert_with()))
                .get_mut_persisted()
                .unwrap(), // this unwrap will never panic, because we insert correct type right now
            Entry::Occupied(occupied) => occupied
                .into_mut()
                .get_persisted_mut_or_insert_with(insert_with),
        }
    }

    #[inline]
    pub fn insert_temp<T: 'static + Any + Clone + Send + Sync>(&mut self, id: Id, value: T) {
        let hash = hash(TypeId::of::<T>(), id);
        self.0.insert(hash, Element::new_temp(value));
    }

    #[inline]
    pub fn insert_persisted<T: SerializableAny>(&mut self, id: Id, value: T) {
        let hash = hash(TypeId::of::<T>(), id);
        self.0.insert(hash, Element::new_persisted(value));
    }

    #[inline]
    pub fn remove<T: 'static>(&mut self, id: Id) {
        let hash = hash(TypeId::of::<T>(), id);
        self.0.remove(&hash);
    }

    /// Note that this function could not remove all needed types between runs because if you upgraded the Rust version or for other reasons.
    pub fn remove_by_type<T: 'static>(&mut self) {
        let key = TypeId::of::<T>();
        self.0.retain(|_, e| {
            let e: &Element = e;
            e.type_id() != key
        });
    }

    #[inline]
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// You could use this function to find is there some leak or misusage. Note, that result of this function could break between runs, if you upgraded the Rust version or for other reasons.
    pub fn count<T: 'static>(&mut self) -> usize {
        let key = TypeId::of::<T>();
        self.0
            .iter()
            .filter(|(_, e)| {
                let e: &Element = e;
                e.type_id() == key
            })
            .count()
    }

    pub fn count_all(&mut self) -> usize {
        self.0.len()
    }
}

#[inline(always)]
fn hash(type_id: TypeId, id: Id) -> u64 {
    type_id.value() ^ id.value()
}

#[cfg(feature = "serde")]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct PersistedMap(Vec<(u64, SerializedElement)>);

#[cfg(feature = "serde")]
impl PersistedMap {
    fn from_map(map: &IdAnyMap) -> Self {
        Self(
            map.0
                .iter()
                .filter_map(|(&hash, element)| Some((hash, element.to_serialize()?)))
                .collect(),
        )
    }
    fn into_map(self) -> IdAnyMap {
        IdAnyMap(
            self.0
                .into_iter()
                .map(|(hash, SerializedElement { type_id, ron })| {
                    (hash, Element::Serialized { type_id, ron })
                })
                .collect(),
        )
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for IdAnyMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        PersistedMap::from_map(self).serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for IdAnyMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        <PersistedMap>::deserialize(deserializer).map(PersistedMap::into_map)
    }
}

// ----------------------------------------------------------------------------

#[test]
fn test_mix() {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[derive(Clone, Debug, PartialEq)]
    struct Foo(i32);

    #[derive(Clone, Debug, PartialEq)]
    struct Bar(f32);

    let id = Id::new("a");

    let mut map: IdAnyMap = Default::default();
    map.insert_persisted(id, Foo(555));
    map.insert_temp(id, Bar(1.0));

    assert_eq!(map.get_temp::<Foo>(id), Some(Foo(555)));
    assert_eq!(map.get_persisted::<Foo>(id), Some(Foo(555)));
    assert_eq!(map.get_temp::<Bar>(id), Some(Bar(1.0)));

    // ------------
    // Test removal:

    // We can remove:
    map.remove::<Bar>(id);
    assert_eq!(map.get_temp::<Bar>(id), None);

    // Other type is still there, even though it is the same if:
    assert_eq!(map.get_temp::<Foo>(id), Some(Foo(555)));
    assert_eq!(map.get_persisted::<Foo>(id), Some(Foo(555)));

    // But we can still remove the last:
    map.remove::<Foo>(id);
    assert_eq!(map.get_temp::<Foo>(id), None);
    assert_eq!(map.get_persisted::<Foo>(id), None);
}

#[cfg(feature = "persistence")]
#[test]
fn test_mix_serialize() {
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct Serializable(i32);

    #[derive(Clone, Debug, PartialEq)]
    struct NonSerializable(f32);

    let id = Id::new("a");

    let mut map: IdAnyMap = Default::default();
    map.insert_persisted(id, Serializable(555));
    map.insert_temp(id, NonSerializable(1.0));

    assert_eq!(map.get_temp::<Serializable>(id), Some(Serializable(555)));
    assert_eq!(
        map.get_persisted::<Serializable>(id),
        Some(Serializable(555))
    );
    assert_eq!(
        map.get_temp::<NonSerializable>(id),
        Some(NonSerializable(1.0))
    );

    // -----------

    let serialized = serde_json::to_string(&map).unwrap();

    // ------------
    // Test removal:

    // We can remove:
    map.remove::<NonSerializable>(id);
    assert_eq!(map.get_temp::<NonSerializable>(id), None);

    // Other type is still there, even though it is the same if:
    assert_eq!(map.get_temp::<Serializable>(id), Some(Serializable(555)));
    assert_eq!(
        map.get_persisted::<Serializable>(id),
        Some(Serializable(555))
    );

    // But we can still remove the last:
    map.remove::<Serializable>(id);
    assert_eq!(map.get_temp::<Serializable>(id), None);
    assert_eq!(map.get_persisted::<Serializable>(id), None);

    // --------------------
    // Test deserialization:

    let mut map: IdAnyMap = serde_json::from_str(&serialized).unwrap();
    assert_eq!(map.get_temp::<Serializable>(id), None);
    assert_eq!(
        map.get_persisted::<Serializable>(id),
        Some(Serializable(555))
    );
    assert_eq!(map.get_temp::<Serializable>(id), Some(Serializable(555)));
}
