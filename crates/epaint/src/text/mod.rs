//! Everything related to text, fonts, text layout, cursors etc.

pub mod cursor;
mod fonts;
mod glyph_atlas;
#[doc(hidden)]
pub mod parley_layout;
pub mod style;
mod text_layout_types;

/// One `\t` character is this many spaces wide.
pub const TAB_SIZE: usize = 4;

pub use {
    fonts::{
        FontData, FontDefinitions, FontInsert, FontPriority, FontStore, FontTweak, Fonts,
        InsertFontFamily,
    },
    text_layout_types::*,
};

/// Suggested character to use to replace those in password text fields.
pub const PASSWORD_REPLACEMENT_CHAR: char = '•';
