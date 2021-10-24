// TODO: it is possible we can simplify `Element` further by
// assuming everything is possibly serializable, and by supplying serialize/deserialize functions for them.
// For non-serializable types, these simply return `None`.
// This will also allow users to pick their own serialization format per type.

use std::any::Any;

// -----------------------------------------------------------------------------------------------

/// Like [`std::any::TypeId`], but can be serialized and deserialized.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
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

#[cfg(feature = "persistence")]
pub trait SerializableAny:
    'static + Any + Clone + serde::Serialize + for<'a> serde::Deserialize<'a> + Send + Sync
{
}

#[cfg(feature = "persistence")]
impl<T> SerializableAny for T where
    T: 'static + Any + Clone + serde::Serialize + for<'a> serde::Deserialize<'a> + Send + Sync
{
}

#[cfg(not(feature = "persistence"))]
pub trait SerializableAny: 'static + Any + Clone + for<'a> Send + Sync {}

#[cfg(not(feature = "persistence"))]
impl<T> SerializableAny for T where T: 'static + Any + Clone + for<'a> Send + Sync {}

// -----------------------------------------------------------------------------------------------

#[cfg(feature = "persistence")]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
struct SerializedElement {
    type_id: TypeId,
    ron: String,
}

#[cfg(feature = "persistence")]
type Serializer = fn(&Box<dyn Any + 'static + Send + Sync>) -> Option<String>;

enum Element {
    /// A value, maybe serializable.
    Value {
        /// The actual value.
        value: Box<dyn Any + 'static + Send + Sync>,

        /// How to clone the value.
        clone_fn: fn(&Box<dyn Any + 'static + Send + Sync>) -> Box<dyn Any + 'static + Send + Sync>,

        /// How to serialize the value.
        /// None if non-serializable type.
        #[cfg(feature = "persistence")]
        serialize_fn: Option<Serializer>,
    },
    /// A serialized value
    Serialized {
        /// The type of value we are storing.
        type_id: TypeId,
        /// The ron data we can deserialize.
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
                .debug_struct("MaybeSerializable::Value")
                .field("type_id", &value.type_id())
                .finish_non_exhaustive(),
            Self::Serialized { type_id, ron } => f
                .debug_struct("MaybeSerializable::Serialized")
                .field("type_id", &type_id)
                .field("ron", &ron)
                .finish(),
        }
    }
}

impl Element {
    /// Create a value that won't be persisted.
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

    /// Create a value that will be persisted.
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
                ron::to_string(x).ok()
            }),
        }
    }

    /// The type of the stored value.
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
                    let ron = serialize_fn(value)?;
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
                "egui: Failed to deserialize {} from memory: {}, ron error: {:?}",
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
/// Stores any value identified by their type and a given [`Id`].
///
/// Values can either be "persisted" (serializable) or "temporary" (cleared when egui is shut down).
///
/// You can store state using the key [`Id::null`]. The state will then only be identified by its type.
#[derive(Clone, Debug, Default)]
// We store use `id XOR typeid` as a key, so we don't need to hash again!
pub struct IdAnyMap(nohash_hasher::IntMap<u64, Element>);

impl IdAnyMap {
    /// Insert a value that will not be persisted.
    #[inline]
    pub fn insert_temp<T: 'static + Any + Clone + Send + Sync>(&mut self, id: Id, value: T) {
        let hash = hash(TypeId::of::<T>(), id);
        self.0.insert(hash, Element::new_temp(value));
    }

    /// Insert a value that will be persisted next time you start the app.
    #[inline]
    pub fn insert_persisted<T: SerializableAny>(&mut self, id: Id, value: T) {
        let hash = hash(TypeId::of::<T>(), id);
        self.0.insert(hash, Element::new_persisted(value));
    }

    /// Read a value without trying to deserialize a persisted value.
    #[inline]
    pub fn get_temp<T: 'static + Clone>(&mut self, id: Id) -> Option<T> {
        let hash = hash(TypeId::of::<T>(), id);
        self.0
            .get_mut(&hash)
            .and_then(|x| x.get_mut_temp())
            .cloned()
    }

    /// Read a value, optionally deserializing it if available.
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

    /// Remove the state of this type an id.
    #[inline]
    pub fn remove<T: 'static>(&mut self, id: Id) {
        let hash = hash(TypeId::of::<T>(), id);
        self.0.remove(&hash);
    }

    /// Note all state of the given type.
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

    #[inline]
    pub fn is_empty(&mut self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    pub fn len(&mut self) -> usize {
        self.0.len()
    }

    /// Count how many values are stored but not yet deserialized.
    #[inline]
    pub fn count_serialized(&mut self) -> usize {
        self.0
            .values()
            .filter(|e| matches!(e, Element::Serialized { .. }))
            .count()
    }

    /// Count the number of values are stored with the given type.
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
}

#[inline(always)]
fn hash(type_id: TypeId, id: Id) -> u64 {
    type_id.value() ^ id.value()
}

// ----------------------------------------------------------------------------

/// How [`IdAnyMap`] is persisted.
#[cfg(feature = "persistence")]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
struct PersistedMap(Vec<(u64, SerializedElement)>);

#[cfg(feature = "persistence")]
impl PersistedMap {
    fn from_map(map: &IdAnyMap) -> Self {
        // filter out the elements which cannot be serialized:
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

#[cfg(feature = "persistence")]
impl serde::Serialize for IdAnyMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        PersistedMap::from_map(self).serialize(serializer)
    }
}

#[cfg(feature = "persistence")]
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
    #[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
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

    let serialized = ron::to_string(&map).unwrap();

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

    let mut map: IdAnyMap = ron::from_str(&serialized).unwrap();
    assert_eq!(map.get_temp::<Serializable>(id), None);
    assert_eq!(
        map.get_persisted::<Serializable>(id),
        Some(Serializable(555))
    );
    assert_eq!(map.get_temp::<Serializable>(id), Some(Serializable(555)));
}
