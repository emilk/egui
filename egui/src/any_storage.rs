pub use inner::*;

#[cfg(feature = "persistence")]
mod inner {
    use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};
    use std::any::{Any, TypeId};
    use std::fmt;
    use DataElement::*;

    pub enum DataElement {
        Deserialized {
            value: Box<dyn Any + 'static>,
            clone_fn: fn(&Box<dyn Any + 'static>) -> Box<dyn Any + 'static>,

            serialize_fn: fn(&Box<dyn Any + 'static>) -> String,
        },
        ToDeserialize(String),
    }

    impl Serialize for DataElement {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            match self {
                Deserialized {
                    value,
                    serialize_fn,
                    ..
                } => serializer.serialize_str(&serialize_fn(value)),
                ToDeserialize(s) => serializer.serialize_str(s),
            }
        }
    }

    impl<'de> Deserialize<'de> for DataElement {
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

            Ok(DataElement::ToDeserialize(
                deserializer.deserialize_str(StrVisitor)?.to_owned(),
            ))
        }
    }

    impl fmt::Debug for DataElement {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Deserialized { value, .. } => f
                    .debug_struct("DataElement_Deserialized")
                    .field("value_type_id", &value.type_id())
                    .finish(),
                ToDeserialize(s) => f
                    .debug_tuple("DataElement_ToDeserialize")
                    .field(&s)
                    .finish(),
            }
        }
    }

    impl Clone for DataElement {
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

    pub trait DataElementTrait:
        'static + Any + Clone + Serialize + for<'a> Deserialize<'a>
    {
    }
    impl<T: 'static + Any + Clone + Serialize + for<'a> Deserialize<'a>> DataElementTrait for T {}

    impl DataElement {
        pub fn new<T: DataElementTrait>(t: T) -> Self {
            Deserialized {
                value: Box::new(t),
                clone_fn: |x| Box::new(x.downcast_ref::<T>().unwrap().clone()),

                serialize_fn: |x| serde_json::to_string(x.downcast_ref::<T>().unwrap()).unwrap(),
            }
        }

        fn deserialize<T: DataElementTrait>(&mut self) {
            match self {
                Deserialized { .. } => {}
                ToDeserialize(s) => {
                    *self = Self::new(serde_json::from_str::<T>(s).unwrap());
                }
            }
        }

        pub fn get<T: DataElementTrait>(&mut self) -> Option<&T> {
            self.deserialize::<T>();
            match self {
                Deserialized { value, .. } => value.downcast_ref(),
                ToDeserialize(_) => unreachable!(),
            }
        }

        pub fn get_mut<T: DataElementTrait>(&mut self) -> Option<&mut T> {
            self.deserialize::<T>();
            match self {
                Deserialized { value, .. } => value.downcast_mut(),
                ToDeserialize(_) => unreachable!(),
            }
        }

        pub fn type_id(&self) -> Option<TypeId> {
            match self {
                Deserialized { value, .. } => Some(value.type_id()),
                ToDeserialize(_) => None,
            }
        }
    }
}

#[cfg(not(feature = "persistence"))]
mod inner {
    use std::any::{Any, TypeId};
    use std::fmt;

    pub struct DataElement {
        value: Box<dyn Any + 'static>,
        clone_fn: fn(&Box<dyn Any + 'static>) -> Box<dyn Any + 'static>,
    }

    impl fmt::Debug for DataElement {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("DataElement")
                .field("value_type_id", &self.value.type_id())
                .finish()
        }
    }

    impl Clone for DataElement {
        fn clone(&self) -> Self {
            DataElement {
                value: (self.clone_fn)(&self.value),
                clone_fn: self.clone_fn,
            }
        }
    }

    pub trait DataElementTrait: 'static + Any + Clone {}

    impl<T: 'static + Any + Clone> DataElementTrait for T {}

    impl DataElement {
        pub fn new<T: DataElementTrait>(t: T) -> Self {
            DataElement {
                value: Box::new(t),
                clone_fn: |x| Box::new(x.downcast_ref::<T>().unwrap().clone()),
            }
        }

        /// mut is needed to deserialization purposes under a feature
        pub fn get<T: DataElementTrait>(&mut self) -> Option<&T> {
            self.value.downcast_ref()
        }

        pub fn get_mut<T: DataElementTrait>(&mut self) -> Option<&mut T> {
            self.value.downcast_mut()
        }

        pub fn type_id(&self) -> Option<TypeId> {
            Some(self.value.type_id())
        }
    }
}
