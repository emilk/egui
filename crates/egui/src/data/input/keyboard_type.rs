// use std::{fmt, str::FromStr};
use alloc::{fmt, str::FromStr};
use log::debug;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
// @See: https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Global_attributes/inputmode
pub enum KeyboardType {
    #[default]

    /// Standard input keyboard for the user's current locale
    Text,

    /// Numeric input keyboard, but only requires the digits 0-9
    Numeric,

    /// Fractional numeric input keyboard containing the digits and decimal separator
    Decimal,

    /// A telephone keypad input, including the digits 0-9, the asterisk (*), and the pound (#) key
    Telephone,

    /// A virtual keyboard optimized for entering email addresses
    Email,

    /// A keypad optimized for entering URLs
    Url,

    /// Remove all 'helpful' things, like autocapitalize and autocomplete
    None,
}

impl fmt::Display for KeyboardType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Numeric => "numeric".to_owned(),
                Self::Decimal => "decimal".to_owned(),
                Self::Telephone => "tel".to_owned(),
                Self::Email => "email".to_owned(),
                Self::Url => "url".to_owned(),
                Self::None => "none".to_owned(),
                Self::Text => "text".to_owned(),
            }
        )
    }
}

impl FromStr for KeyboardType {
    type Err = ();

    fn from_str(input_str: &str) -> Result<Self, ()> {
        Ok(match input_str {
            "numeric" => Self::Numeric,
            "decimal" => Self::Decimal,
            "tel" => Self::Telephone,
            "email" => Self::Email,
            "url" => Self::Url,
            "text" => Self::Text,
            unknown => {
                debug!("Unknown inputmode {unknown}. Defaulting to 'text'.");
                Self::Text
            }
        })
    }
}
