//! All the data sent between egui and the backend

pub mod input;
mod key;
pub mod output;
mod user_data;

pub use key::Key;
pub use user_data::UserData;
