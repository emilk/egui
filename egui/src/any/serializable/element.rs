use crate::any::serializable::type_id::TypeId;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::any::Any;
use std::fmt;
use AnyMapElementInner::{Deserialized, Serialized};

pub(crate) struct AnyMapElement(AnyMapElementInner);

enum AnyMapElementInner {
    Deserialized {
        value: Box<dyn Any + 'static + Send + Sync>,
        clone_fn: fn(&Box<dyn Any + 'static + Send + Sync>) -> Box<dyn Any + 'static + Send + Sync>,

        serialize_fn: fn(&Box<dyn Any + 'static + Send + Sync>) -> Result<String, ron::Error>,
    },
    Serialized(String, TypeId),
}

#[derive(Deserialize, Serialize)]
struct AnyMapElementInnerSer(String, TypeId);

impl Serialize for AnyMapElement {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let inner = match &self.0 {
            Deserialized {
                value,
                serialize_fn,
                ..
            } => {
                let s = serialize_fn(value).map_err(serde::ser::Error::custom)?;
                AnyMapElementInnerSer(s, self.type_id())
            }
            Serialized(s, id) => AnyMapElementInnerSer(s.clone(), *id),
        };

        inner.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AnyMapElement {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let AnyMapElementInnerSer(s, id) = AnyMapElementInnerSer::deserialize(deserializer)?;

        Ok(AnyMapElement(Serialized(s, id)))
    }
}

impl fmt::Debug for AnyMapElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Deserialized { value, .. } => f
                .debug_struct("AnyMapElement_Deserialized")
                .field("value_type_id", &value.type_id())
                .finish(),
            Serialized(s, id) => f
                .debug_tuple("AnyMapElement_Serialized")
                .field(&s)
                .field(&id)
                .finish(),
        }
    }
}

impl Clone for AnyMapElement {
    fn clone(&self) -> Self {
        match &self.0 {
            Deserialized {
                value,
                clone_fn,
                serialize_fn,
            } => AnyMapElement(Deserialized {
                value: clone_fn(value),
                clone_fn: *clone_fn,
                serialize_fn: *serialize_fn,
            }),
            Serialized(s, id) => AnyMapElement(Serialized(s.clone(), *id)),
        }
    }
}

pub trait AnyMapTrait:
    'static + Any + Clone + Serialize + for<'a> Deserialize<'a> + Send + Sync
{
}
impl<T: 'static + Any + Clone + Serialize + for<'a> Deserialize<'a> + Send + Sync> AnyMapTrait
    for T
{
}

impl AnyMapElement {
    pub(crate) fn new<T: AnyMapTrait>(t: T) -> Self {
        AnyMapElement(Deserialized {
            value: Box::new(t),
            clone_fn: |x| {
                let x = x.downcast_ref::<T>().unwrap(); // This unwrap will never panic, because we always construct this type using this `new` function and because we return &mut reference only with this type `T`, so type cannot change.
                Box::new(x.clone())
            },

            serialize_fn: |x| {
                let x = x.downcast_ref::<T>().unwrap(); // This will never panic too, for same reason.
                ron::to_string(x)
            },
        })
    }

    pub(crate) fn type_id(&self) -> TypeId {
        match self {
            AnyMapElement(Deserialized { value, .. }) => (**value).type_id().into(),
            AnyMapElement(Serialized(_, id)) => *id,
        }
    }

    pub(crate) fn get_mut<T: AnyMapTrait>(&mut self) -> Option<&mut T> {
        match self {
            AnyMapElement(Deserialized { value, .. }) => value.downcast_mut(),
            AnyMapElement(Serialized(s, _)) => {
                *self = Self::new(ron::from_str::<T>(s).ok()?);

                match self {
                    AnyMapElement(Deserialized { value, .. }) => value.downcast_mut(),
                    AnyMapElement(Serialized(_, _)) => unreachable!(),
                }
            }
        }
    }

    pub(crate) fn get_mut_or_set_with<T: AnyMapTrait>(
        &mut self,
        set_with: impl FnOnce() -> T,
    ) -> &mut T {
        match &mut self.0 {
            Deserialized { value, .. } => {
                if !value.is::<T>() {
                    *self = Self::new(set_with());
                    // TODO: log this error, because it can occurs when user used same Id or same type for different widgets
                }
            }
            Serialized(s, _) => {
                *self = Self::new(ron::from_str::<T>(s).unwrap_or_else(|_| set_with()));
                // TODO: log deserialization error
            }
        }

        match &mut self.0 {
            Deserialized { value, .. } => value.downcast_mut().unwrap(), // This unwrap will never panic because we already converted object to required type
            Serialized(_, _) => unreachable!(),
        }
    }
}
