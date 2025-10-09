use std::{
    collections::HashSet, convert::Infallible, error::Error, fmt::Display, num::ParseIntError,
};

pub trait TextType: Sized {
    type Err: Error;

    /// The value of represented data type depending on the previous string text and modified
    /// (by the user) string.
    ///
    /// `None` is output if this type is immutable.
    /// `Some(result)` is the result of parsing.
    ///
    /// This **must** be parse output from [`TextType::string_representation`].
    fn read_from_strings(previous: &str, modified: &str) -> Option<Result<Self, Self::Err>>;
    /// Generate the string representation of this type.
    ///
    /// This **must** be parseable by [`TextType::read_from_strings`].
    fn string_representation(&self) -> String;

    /// Can the user change the value of this display type?
    fn is_mutable() -> bool {
        !Self::read_from_strings("", "").is_none()
    }
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

    fn read_from_strings(_previous: &str, _modified: &str) -> Option<Result<Self, Self::Err>> {
        None
    }

    fn string_representation(&self) -> String {
        self.to_string()
    }
}

impl TextType for String {
    type Err = Infallible;

    fn read_from_strings(_previous: &str, modified: &str) -> Option<Result<Self, Self::Err>> {
        Some(Ok(modified.to_string()))
    }

    fn string_representation(&self) -> String {
        self.to_string()
    }
}

impl TextType for u8 {
    type Err = ParseIntError;

    fn read_from_strings(_previous: &str, modified: &str) -> Option<Result<Self, Self::Err>> {
        Some(modified.parse())
    }

    fn string_representation(&self) -> String {
        self.to_string()
    }
}

impl TextType for char {
    type Err = ConversionError;

    fn read_from_strings(previous: &str, modified: &str) -> Option<Result<Self, Self::Err>> {
        let previous: HashSet<char> = previous.chars().collect();
        let current_chars: HashSet<char> = modified.chars().collect();

        let mut chars = current_chars.difference(&previous);
        let diff_character = chars.next();
        let additional = chars.next().is_some();

        Some(match (diff_character, additional) {
            (None, _) => match modified.chars().next() {
                Some(c) => Ok(c),
                None => Err(ConversionError("Zero characters present".to_owned())),
            },
            (Some(_), true) => Err(ConversionError(
                "More than one character present".to_owned(),
            )),
            (Some(character), false) => Ok(*character),
        })
    }

    fn string_representation(&self) -> String {
        self.to_string()
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
