//! A library containing built-in fonts for `epaint`, embedded as bytes.
//!
//! This is intended to be consumed through the `epaint` crate.

pub const HACK_REGULAR: &[u8] = include_bytes!("../fonts/Hack-Regular.ttf");
pub const NOTO_EMOJI_REGULAR: &[u8] = include_bytes!("../fonts/NotoEmoji-Regular.ttf");
pub const UBUNTU_LIGHT: &[u8] = include_bytes!("../fonts/Ubuntu-Light.ttf");
pub const EMOJI_ICON: &[u8] = include_bytes!("../fonts/emoji-icon-font.ttf");
