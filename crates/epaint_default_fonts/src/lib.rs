//! A library containing built-in fonts for `epaint`, embedded as bytes.
//!
//! This is intended to be consumed through the `epaint` crate.

/// A typeface designed for source code.
///
/// Hack is designed to be a workhorse typeface for source code. It has deep
/// roots in the free, open source typeface community and expands upon the
/// contributions of the [Bitstream Vera](https://www.gnome.org/fonts/) and
/// [DejaVu](https://dejavu-fonts.github.io/) projects.  The large x-height +
/// wide aperture + low contrast design make it legible at commonly used source
/// code text sizes with a sweet spot that runs in the 8 - 14 range.
///
/// See [the Hack repository](https://github.com/source-foundry/Hack) for more
/// information.
pub const HACK_REGULAR: &[u8] = include_bytes!("../fonts/Hack-Regular.ttf");

/// A typeface containing emoji characters as designed for the Noto font family.
///
/// Noto is a collection of high-quality fonts with multiple weights and widths
/// in sans, serif, mono, and other styles, in more than 1,000 languages and
/// over 150 writing systems. Noto Emoji contains black-and-white emoji
/// characters that match Google's emoji designs.
///
/// See [Google Fonts](https://fonts.google.com/noto/specimen/Noto+Emoji) for
/// more information.
pub const NOTO_EMOJI_REGULAR: &[u8] = include_bytes!("../fonts/NotoEmoji-Regular.ttf");

/// A typeface designed for use by Ubuntu.
///
/// The Ubuntu typeface has been specially created to complement the Ubuntu tone
/// of voice. It has a contemporary style and contains characteristics unique to
/// the Ubuntu brand that convey a precise, reliable and free attitude.
///
/// See [Ubuntu design](https://design.ubuntu.com/font) for more information.
pub const UBUNTU_LIGHT: &[u8] = include_bytes!("../fonts/Ubuntu-Light.ttf");

/// An experimental typeface that uses standardized
/// [UNICODE planes](http://en.wikipedia.org/wiki/Plane_(Unicode))
/// for icon fonts.
///
/// The icons in this font are designed to be styled with minimal effort. Each
/// icon is solid, which is useful for changing icon colors.
///
/// See [the `emoji-icon-font` repository](https://github.com/jslegers/emoji-icon-font)
/// for more information.
pub const EMOJI_ICON: &[u8] = include_bytes!("../fonts/emoji-icon-font.ttf");
