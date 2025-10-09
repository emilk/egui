use std::{convert::Infallible, error::Error, fmt::Display, num::ParseIntError};

pub trait TextType: Sized {
    type Err: Error;

    fn read_from_string(s: &str) -> Option<Result<Self, Self::Err>>;
    fn string_representation(&self) -> String;

    fn is_mutable() -> bool {
        !Self::read_from_string("").is_none()
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

    fn read_from_string(_s: &str) -> Option<Result<Self, Self::Err>> {
        None
    }

    fn string_representation(&self) -> String {
        self.to_string()
    }
}

impl TextType for String {
    type Err = Infallible;

    fn read_from_string(s: &str) -> Option<Result<Self, Self::Err>> {
        Some(Ok(s.to_string()))
    }

    fn string_representation(&self) -> String {
        self.to_string()
    }
}

impl TextType for u8 {
    type Err = ParseIntError;

    fn read_from_string(s: &str) -> Option<Result<Self, Self::Err>> {
        Some(s.parse())
    }

    fn string_representation(&self) -> String {
        self.to_string()
    }
}

impl TextType for char {
    type Err = ConversionError;

    fn read_from_string(s: &str) -> Option<Result<Self, Self::Err>> {
        let mut chars = s.chars();
        let character = chars.next();
        let additional = chars.next().is_some();

        Some(match (character, additional) {
            (None, _) => Err(ConversionError("Zero characters present".to_owned())),
            (Some(_), true) => Err(ConversionError(
                "More than one character present".to_owned(),
            )),
            (Some(character), false) => Ok(character),
        })
    }

    fn string_representation(&self) -> String {
        self.to_string()
    }
}

#[cfg(test)]
mod tests {
    use crate::TextEdit;

    use super::*;

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
