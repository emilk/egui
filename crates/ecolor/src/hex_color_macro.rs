/// Construct a [`crate::Color32`] from a hex RGB or RGBA string literal.
///
/// Requires the "color-hex" feature.
///
/// The string is checked at compile time. If the format is invalid, compilation fails. The valid
/// format is the one described in <https://drafts.csswg.org/css-color-4/#hex-color>. Only 6 (RGB) or 8 (RGBA)
/// digits are supported, and the leading `#` character is optional.
///
/// Note that despite being checked at compile-time, this macro is not usable in `const` contexts
/// because creating the [`crate::Color32`] instance requires floating-point arithmetic.
///
/// See also [`crate::Color32::from_hex`] and [`crate::Color32::to_hex`].
///
/// # Examples
///
/// ```
/// # use ecolor::{hex_color, Color32};
/// assert_eq!(hex_color!("#202122"), Color32::from_hex("#202122").unwrap());
/// assert_eq!(hex_color!("#202122"), Color32::from_rgb(0x20, 0x21, 0x22));
/// assert_eq!(hex_color!("#202122"), hex_color!("202122"));
/// assert_eq!(hex_color!("#abcdef12"), Color32::from_rgba_unmultiplied(0xab, 0xcd, 0xef, 0x12));
/// ```
///
/// If the literal string has the wrong format, the code does not compile.
///
/// ```compile_fail
/// let _ = ecolor::hex_color!("#abc");
/// ```
///
/// ```compile_fail
/// let _ = ecolor::hex_color!("#20212x");
/// ```
///
/// The macro cannot be used in a `const` context.
///
/// ```compile_fail
/// const COLOR: ecolor::Color32 = ecolor::hex_color!("#202122");
/// ```
#[macro_export]
macro_rules! hex_color {
    ($s:literal) => {{
        let array = $crate::color_hex::color_from_hex!($s);
        match array.as_slice() {
            [r, g, b] => $crate::Color32::from_rgb(*r, *g, *b),
            [r, g, b, a] => $crate::Color32::from_rgba_unmultiplied(*r, *g, *b, *a),
            _ => panic!("Invalid hex color length: expected 3 (RGB) or 4 (RGBA) bytes"),
        }
    }};
}

#[test]
fn test_from_rgb_hex() {
    assert_eq!(
        crate::Color32::from_rgb(0x20, 0x21, 0x22),
        hex_color!("#202122")
    );
    assert_eq!(
        crate::Color32::from_rgb_additive(0x20, 0x21, 0x22),
        hex_color!("#202122").additive()
    );
}

#[test]
fn test_from_rgba_hex() {
    assert_eq!(
        crate::Color32::from_rgba_unmultiplied(0x20, 0x21, 0x22, 0x50),
        hex_color!("20212250")
    );
}
