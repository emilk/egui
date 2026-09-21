//! Helpers for the accessibility tree.

use crate::accesskit::Role;

/// Roles a user reaches by name: everything you can click, toggle, pick from, drag, or type into.
///
/// A widget with one of these roles and no accessible name cannot be found by a screen reader,
/// nor by a test that looks it up by label.
pub const INPUT_ROLES: &[Role] = &[
    Role::Button,
    Role::CheckBox,
    Role::ColorWell,
    Role::ComboBox,
    Role::DisclosureTriangle,
    Role::Link,
    Role::MultilineTextInput,
    Role::PasswordInput,
    Role::RadioButton,
    Role::Slider,
    Role::SpinButton,
    Role::TextInput,
];
