//! Everything related to text, fonts, text layout, cursors etc.

pub mod cursor;
mod font;
mod fonts;
mod galley;

/// Default size for a `\t` character.
pub const MAX_TAB_SIZE: usize = 4;

pub use {
    fonts::{FontDefinitions, FontFamily, Fonts, TextStyle},
    galley::{Galley, Row},
};

/// Suggested character to use to replace those in password text fields.
pub const PASSWORD_REPLACEMENT_CHAR: char = '•';
