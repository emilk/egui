mod type_map;
#[cfg(feature = "persistence")]
mod element;
mod id_map;
#[cfg(feature = "persistence")]
mod type_id;

#[cfg(not(feature = "persistence"))]
use super::element;

pub use self::{type_map::TypeMap, element::AnyMapTrait, id_map::AnyMapId};
