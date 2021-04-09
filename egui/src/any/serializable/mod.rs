mod any_map;
#[cfg(feature = "persistence")]
mod element;
#[cfg(feature = "persistence")]
mod type_id;
mod type_map;

#[cfg(not(feature = "persistence"))]
use super::element;

pub use self::{any_map::AnyMap, element::AnyMapTrait, type_map::TypeMap};

mod usages {
    #[cfg(feature = "persistence")]
    pub(crate) use {
        crate::any::serializable::element::{AnyMapElement, AnyMapTrait},
        crate::any::serializable::type_id::TypeId,
    };

    #[cfg(not(feature = "persistence"))]
    pub(crate) use {
        crate::any::element::{AnyMapElement, AnyMapTrait},
        std::any::TypeId,
    };
}
