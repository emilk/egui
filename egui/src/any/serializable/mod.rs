mod type_map;
#[cfg(feature = "persistence")]
mod element;
mod any_map;
#[cfg(feature = "persistence")]
mod type_id;

#[cfg(not(feature = "persistence"))]
use super::element;

pub use self::{type_map::TypeMap, element::AnyMapTrait, any_map::AnyMap};
