mod type_map;
#[cfg(feature = "persistence")]
mod element;
mod any_map;
#[cfg(feature = "persistence")]
mod type_id;

#[cfg(not(feature = "persistence"))]
use super::element;

pub use self::{type_map::TypeMap, element::AnyMapTrait, any_map::AnyMap};

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