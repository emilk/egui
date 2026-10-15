mod class_name;
mod classes;
mod has_classes;

pub use class_name::ClassName;
pub use classes::Classes;
pub use has_classes::HasClasses;

/// Present on every top-level [`crate::Ui`].
pub const ROOT: ClassName = ClassName::from_static("egui::root");
