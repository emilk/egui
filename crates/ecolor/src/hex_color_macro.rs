/// Construct a [`crate::Color32`] from a hex RGB or RGBA string.
///
/// ```
/// # use ecolor::{hex_color, Color32};
/// assert_eq!(hex_color!("#202122"), Color32::from_rgb(0x20, 0x21, 0x22));
/// assert_eq!(hex_color!("#abcdef12"), Color32::from_rgba_unmultiplied(0xab, 0xcd, 0xef, 0x12));
/// ```
#[macro_export]
macro_rules! hex_color {
    ($s:literal) => {{
        let array = color_hex::color_from_hex!($s);
        if array.len() == 3 {
            $crate::Color32::from_rgb(array[0], array[1], array[2])
        } else {
            #[allow(unconditional_panic)]
            $crate::Color32::from_rgba_unmultiplied(array[0], array[1], array[2], array[3])
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
