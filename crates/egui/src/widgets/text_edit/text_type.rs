use std::{convert::Infallible, error::Error, fmt::Display, num::ParseIntError};

pub trait TextType: Sized {
    type Err: Error;

    /// The value of represented data type depending on the previous valid value and the string modified by the user.
    ///
    /// `None` is output if this type is immutable.
    /// `Some(result)` is the result of parsing.
    ///
    /// This **must** be parse output from [`TextType::string_representation`].
    fn read_from_string(previous: &Self, modified: &str) -> Option<Result<Self, Self::Err>>;
    /// Generate the string representation of this type.
    ///
    /// This **must** be parseable by [`TextType::read_from_strings`].
    fn string_representation(&self) -> String;

    /// Whether this data type can be modified.
    fn is_mutable() -> bool;
}

#[derive(Debug)]
pub struct ConversionError(String);

impl Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for ConversionError {}

impl TextType for &str {
    type Err = Infallible;

    fn read_from_string(_previous: &Self, _modified: &str) -> Option<Result<Self, Self::Err>> {
        None
    }

    fn string_representation(&self) -> String {
        self.to_string()
    }

    fn is_mutable() -> bool {
        false
    }
}

impl TextType for String {
    type Err = Infallible;

    fn read_from_string(_previous: &Self, modified: &str) -> Option<Result<Self, Self::Err>> {
        Some(Ok(modified.to_string()))
    }

    fn string_representation(&self) -> String {
        self.to_string()
    }

    fn is_mutable() -> bool {
        true
    }
}

impl TextType for u8 {
    type Err = ParseIntError;

    fn read_from_string(_previous: &Self, modified: &str) -> Option<Result<Self, Self::Err>> {
        Some(modified.parse())
    }

    fn string_representation(&self) -> String {
        self.to_string()
    }

    fn is_mutable() -> bool {
        true
    }
}

impl TextType for char {
    type Err = ConversionError;

    fn read_from_string(previous: &Self, modified: &str) -> Option<Result<Self, Self::Err>> {
        let modified: Vec<char> = modified.chars().collect();

        Some(match (modified.get(0), modified.get(1), modified.get(2)) {
            (Some(_), Some(_), Some(_)) => Err(ConversionError(
                "Three or more characters present".to_string(),
            )),
            (Some(first), Some(second), None) if first == previous => Ok(*second),
            (Some(first), Some(second), None) if first == second => Ok(*first),
            (Some(_), Some(_), None) => Err(ConversionError(
                "Two different characters present".to_string(),
            )),
            (None, _, _) => Err(ConversionError("Zero characters present".to_string())),
            (Some(only), _, _) => Ok(*only),
        })
    }

    fn string_representation(&self) -> String {
        self.to_string()
    }

    fn is_mutable() -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::TextEdit;

    #[allow(unused_must_use)]
    #[test]
    fn type_validation() {
        TextEdit::singleline(&mut String::new());
        TextEdit::singleline(&mut 10u8);
        let mut c = char::MIN;
        TextEdit::singleline(&mut c);
        TextEdit::singleline(&mut "abc");
    }
}
