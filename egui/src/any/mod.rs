//! Any-type storages for [`Memory`].
//!
//! This module contains structs to store arbitrary types using [`Any`] trait. Also, they can be cloned, and structs in [`serializable`] can be de/serialized.
//!
//! All this is just `HashMap<TypeId, Box<dyn Any + static>>` and `HashMap<Key, Box<dyn Any + static>>`, but with helper functions and hacks for cloning and de/serialization.
//!
//! # Trait requirements
//!
//! If you want to store your type here, it must implement `Clone` and `Any` and be `'static`, which means it must not contain references. If you want to store your data in serializable storage, it must implement `serde::Serialize` and `serde::Deserialize` under the `persistent` feature.
//!
//! # [`TypeMap`]
//!
//! It stores everything by just type. You should use this map for your widget when all instances of your widgets can have only one state. E.g. for popup windows, for color picker.
//!
//! To not have intersections, you should create newtype for anything you try to store here, like:
//! ```rust
//! struct MyEditBool(pub bool);
//! ```
//!
//! # [`AnyMap<Key>`]
//!
//! In [`Memory`] `Key` = [`Id`].
//!
//! [`TypeMap`] and [`AnyMap<Key>`] has a quite similar interface, except for [`AnyMap`] you should pass `Key` to get and insert things.
//!
//! It stores everything by `Key`, this should be used when your widget can have different data for different instances of the widget.
//!
//! # `serializable`
//!
//! [`TypeMap`] and [`serializable::TypeMap`] has exactly the same interface, but [`serializable::TypeMap`] only requires serde traits for stored object under `persistent` feature. Same thing for [`AnyMap`] and [`serializable::AnyMap`].
//!
//! # What could break
//!
//! Things here could break only when you trying to load this from file.
//!
//! First, serialized `TypeId` in [`serializable::TypeMap`] could broke if you updated the version of the Rust compiler between runs.
//!
//! Second, count and reset all instances of a type in [`serializable::AnyMap`] could return an incorrect value for the same reason.
//!
//! Deserialization errors of loaded elements of these storages can be determined only when you call `get_...` functions, they not logged and not provided to a user, on this errors value is just replaced with `or_insert()`/default value.
//!
//! # When not to use this
//!
//! This is not for important widget data. Some errors are just ignored and the correct value of type is inserted when you call. This is done to more simple interface.
//!
//! You shouldn't use any map here when you need very reliable state storage with rich error-handling. For this purpose you should create your own `Memory` struct and pass it everywhere you need it. Then, you should de/serialize it by yourself, handling all serialization or other errors as you wish.
//!
//! [`Id`]: crate::Id
//! [`Memory`]: crate::Memory
//! [`Any`]: std::any::Any
//! [`AnyMap<Key>`]: crate::any::AnyMap

mod any_map;
mod element;
mod type_map;

/// Same structs and traits, but also can be de/serialized under `persistence` feature.
#[cfg(feature = "persistence")]
pub mod serializable;

pub use self::{any_map::AnyMap, element::AnyMapTrait, type_map::TypeMap};
