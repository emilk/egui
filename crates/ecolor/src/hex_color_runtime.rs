//! Convert colors to and from the hex-color string format at runtime
//!
//! Supports the 3, 4, 6, and 8-digit formats, according to the specification in
//! <https://drafts.csswg.org/css-color-4/#hex-color>

use std::{fmt::Display, str::FromStr};

use crate::Color32;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
/// A wrapper around Color32 that converts to and from a hex-color string
///
/// Implements [`Display`] and [`FromStr`] to convert to and from the hex string.
pub enum HexColor {
    /// 3 hexadecimal digits, one for each of the r, g, b channels
    Hex3(Color32),

    /// 4 hexadecimal digits, one for each of the r, g, b, a channels
    Hex4(Color32),

    /// 6 hexadecimal digits, two for each of the r, g, b channels
    Hex6(Color32),

    /// 8 hexadecimal digits, one for each of the r, g, b, a channels
    Hex8(Color32),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseHexColorError {
    MissingHash,
    InvalidLength,
    InvalidInt(std::num::ParseIntError),
}

impl FromStr for HexColor {
    type Err = ParseHexColorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.strip_prefix('#')
            .ok_or(ParseHexColorError::MissingHash)
            .and_then(Self::from_str_without_hash)
    }
}

impl Display for HexColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HexColor::Hex3(color) => {
                let [r, g, b, _] = color.to_array().map(|u| u >> 4);
                f.write_fmt(format_args!("#{r:x}{g:x}{b:x}"))
            }
            HexColor::Hex4(color) => {
                let [r, g, b, a] = color.to_array().map(|u| u >> 4);
                f.write_fmt(format_args!("#{r:x}{g:x}{b:x}{a:x}"))
            }
            HexColor::Hex6(color) => {
                let [r, g, b, _] = color.to_array();
                let u = u32::from_be_bytes([0, r, g, b]);
                f.write_fmt(format_args!("#{u:06x}"))
            }
            HexColor::Hex8(color) => {
                let [r, g, b, a] = color.to_array();
                let u = u32::from_be_bytes([r, g, b, a]);
                f.write_fmt(format_args!("#{u:08x}"))
            }
        }
    }
}

impl HexColor {
    /// Retrieves the inner [`Color32`]
    #[inline]
    pub fn color(&self) -> Color32 {
        match self {
            HexColor::Hex3(color)
            | HexColor::Hex4(color)
            | HexColor::Hex6(color)
            | HexColor::Hex8(color) => *color,
        }
    }

    /// Parses a string as a hex color without the leading `#` character
    ///
    /// # Errors
    /// Returns an error if the length of the string does not correspond to one of the standard
    /// formats (3, 4, 6, or 8), or if it contains non-hex characters.
    #[inline]
    pub fn from_str_without_hash(s: &str) -> Result<Self, ParseHexColorError> {
        match s.len() {
            3 => {
                let [r, gb] = u16::from_str_radix(s, 16)
                    .map_err(ParseHexColorError::InvalidInt)?
                    .to_be_bytes();
                let [r, g, b] = [r, gb >> 4, gb & 0x0f].map(|u| u << 4 | u);
                Ok(HexColor::Hex3(Color32::from_rgb(r, g, b)))
            }
            4 => {
                let [rg, ba] = u16::from_str_radix(s, 16)
                    .map_err(ParseHexColorError::InvalidInt)?
                    .to_be_bytes();
                let [r, g, b, a] = [rg >> 4, rg & 0x0f, ba >> 4, ba & 0x0f].map(|u| u << 4 | u);
                Ok(HexColor::Hex4(Color32::from_rgba_premultiplied(r, g, b, a)))
            }
            6 => {
                let [_, r, g, b] = u32::from_str_radix(s, 16)
                    .map_err(ParseHexColorError::InvalidInt)?
                    .to_be_bytes();
                Ok(HexColor::Hex6(Color32::from_rgb(r, g, b)))
            }
            8 => {
                let [r, g, b, a] = u32::from_str_radix(s, 16)
                    .map_err(ParseHexColorError::InvalidInt)?
                    .to_be_bytes();
                Ok(HexColor::Hex8(Color32::from_rgba_premultiplied(r, g, b, a)))
            }
            _ => Err(ParseHexColorError::InvalidLength)?,
        }
    }
}

impl Color32 {
    /// Parses a color from a hex string
    ///
    /// Supports the 3, 4, 6, and 8-digit formats.
    ///
    /// # Example
    /// ```rust
    /// use ecolor::Color32;
    /// assert_eq!(Ok(Color32::RED), Color32::from_hex_string("#ff0000"));
    /// assert_eq!(Ok(Color32::GREEN), Color32::from_hex_string("#00ff00ff"));
    /// assert_eq!(Ok(Color32::BLUE), Color32::from_hex_string("#00f"));
    /// assert_eq!(Ok(Color32::TRANSPARENT), Color32::from_hex_string("#0000"));
    /// ```
    ///
    /// # Errors
    /// Returns an error if the string doesn't start with the hash `#` character, if the remaining
    /// length does not correspond to one of the standard formats (3, 4, 6, or 8), if it contains
    /// non-hex characters.
    pub fn from_hex_string(hex: &str) -> Result<Self, ParseHexColorError> {
        HexColor::from_str(hex).map(|h| h.color())
    }

    /// Formats the color as a hex string
    ///
    /// # Example
    /// ```rust
    /// use ecolor::Color32;
    /// assert_eq!(Color32::RED.to_hex_string(), "#ff0000ff");
    /// assert_eq!(Color32::GREEN.to_hex_string(), "#00ff00ff");
    /// assert_eq!(Color32::BLUE.to_hex_string(), "#0000ffff");
    /// assert_eq!(Color32::TRANSPARENT.to_hex_string(), "#00000000");
    /// ```
    ///
    /// Uses the 8-digit format, as that is the only format that is lossless.
    /// For other formats, see [`HexColor`].
    #[inline]
    pub fn to_hex_string(&self) -> String {
        HexColor::Hex8(*self).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_string_formats() {
        use Color32 as C;
        use HexColor as H;
        let cases = [
            (H::Hex3(C::RED), "#f00"),
            (H::Hex4(C::RED), "#f00f"),
            (H::Hex6(C::RED), "#ff0000"),
            (H::Hex8(C::RED), "#ff0000ff"),
            (H::Hex3(C::GREEN), "#0f0"),
            (H::Hex4(C::GREEN), "#0f0f"),
            (H::Hex6(C::GREEN), "#00ff00"),
            (H::Hex8(C::GREEN), "#00ff00ff"),
            (H::Hex3(C::BLUE), "#00f"),
            (H::Hex4(C::BLUE), "#00ff"),
            (H::Hex6(C::BLUE), "#0000ff"),
            (H::Hex8(C::BLUE), "#0000ffff"),
            (H::Hex3(C::WHITE), "#fff"),
            (H::Hex4(C::WHITE), "#ffff"),
            (H::Hex6(C::WHITE), "#ffffff"),
            (H::Hex8(C::WHITE), "#ffffffff"),
            (H::Hex3(C::BLACK), "#000"),
            (H::Hex4(C::BLACK), "#000f"),
            (H::Hex6(C::BLACK), "#000000"),
            (H::Hex8(C::BLACK), "#000000ff"),
            (H::Hex4(C::TRANSPARENT), "#0000"),
            (H::Hex8(C::TRANSPARENT), "#00000000"),
        ];
        for (color, string) in cases {
            assert_eq!(color.to_string(), string, "{color:?} <=> {string}");
            assert_eq!(
                H::from_str(string).unwrap(),
                color,
                "{color:?} <=> {string}"
            );
        }
    }

    #[test]
    fn hex_string_round_trip() {
        use Color32 as C;
        let cases = [
            C::from_rgba_premultiplied(10, 20, 30, 0),
            C::from_rgba_premultiplied(10, 20, 30, 40),
            C::from_rgba_premultiplied(10, 20, 30, 255),
            C::from_rgba_premultiplied(0, 20, 30, 0),
            C::from_rgba_premultiplied(10, 0, 30, 40),
            C::from_rgba_premultiplied(10, 20, 0, 255),
        ];
        for color in cases {
            assert_eq!(
                C::from_hex_string(color.to_hex_string().as_str()),
                Ok(color)
            );
        }
    }
}
