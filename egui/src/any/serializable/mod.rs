mod any_map;
#[cfg(feature = "persistence")]
mod element;
mod id_map;
#[cfg(feature = "persistence")]
mod type_id;

#[cfg(not(feature = "persistence"))]
use super::element;

pub use self::{any_map::AnyMap, element::AnyMapTrait, id_map::AnyMapId};
