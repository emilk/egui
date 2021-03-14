//! Everything related to text, fonts, text layout, cursors etc.

pub mod cursor;
mod font;
mod fonts;
mod galley;

pub use {
    fonts::{FontDefinitions, FontFamily, Fonts, TextStyle},
    galley::{Galley, Row},
};
