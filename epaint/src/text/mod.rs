//! Everything related to text, fonts, text layout, cursors etc.

pub mod cursor;
mod font;
mod fonts;
mod galley;
mod text_color_map;

pub use {
    fonts::{FontDefinitions, FontFamily, Fonts, TextStyle},
    galley::{Galley, Row},
    text_color_map::TextColorMap,
};

/// Suggested character to use to replace those in password text fields.
pub const PASSWORD_REPLACEMENT_CHAR: char = '•';
