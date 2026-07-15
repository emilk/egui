//! Strongly-typed offsets into text.
//!
//! UTF-8 text can be indexed either by _byte_ offset or by _character_
//! (Unicode scalar) offset. Mixing the two is a common source of bugs,
//! so we use distinct types to keep them apart.

use std::ops::Range;

/// A byte offset into a UTF-8 string.
///
/// This is what you use to slice a [`str`] (e.g. `&text[range.start.0..range.end.0]`).
/// Not to be confused with [`CharIndex`], which counts characters instead of bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(transparent)
)]
pub struct ByteIndex(pub usize);

/// A character (Unicode scalar) offset into a string.
///
/// Counts characters, not bytes, so it is independent of the UTF-8 encoding.
/// Not to be confused with [`ByteIndex`]. See also [`super::cursor::CCursor`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(transparent)
)]
pub struct CharIndex(pub usize);

macro_rules! impl_text_index {
    ($Type:ident) => {
        impl $Type {
            /// The zero offset, i.e. the very start of the text.
            pub const ZERO: Self = Self(0);

            /// Saturating integer addition.
            #[inline]
            pub fn saturating_add(self, rhs: usize) -> Self {
                Self(self.0.saturating_add(rhs))
            }

            /// Saturating integer subtraction.
            #[inline]
            pub fn saturating_sub(self, rhs: usize) -> Self {
                Self(self.0.saturating_sub(rhs))
            }
        }

        impl From<usize> for $Type {
            #[inline]
            fn from(index: usize) -> Self {
                Self(index)
            }
        }

        impl From<$Type> for usize {
            #[inline]
            fn from(index: $Type) -> Self {
                index.0
            }
        }

        impl std::ops::Add<usize> for $Type {
            type Output = Self;

            #[inline]
            fn add(self, rhs: usize) -> Self {
                Self(self.0 + rhs)
            }
        }

        /// Compose offsets, e.g. a base position plus a relative one.
        impl std::ops::Add<$Type> for $Type {
            type Output = Self;

            #[inline]
            fn add(self, rhs: Self) -> Self {
                Self(self.0 + rhs.0)
            }
        }

        impl std::ops::Sub<usize> for $Type {
            type Output = Self;

            #[inline]
            fn sub(self, rhs: usize) -> Self {
                Self(self.0 - rhs)
            }
        }

        impl std::ops::Sub<$Type> for $Type {
            type Output = Self;

            #[inline]
            fn sub(self, rhs: Self) -> Self {
                Self(self.0 - rhs.0)
            }
        }

        impl std::ops::AddAssign<usize> for $Type {
            #[inline]
            fn add_assign(&mut self, rhs: usize) {
                self.0 += rhs;
            }
        }

        impl std::ops::AddAssign<$Type> for $Type {
            #[inline]
            fn add_assign(&mut self, rhs: Self) {
                self.0 += rhs.0;
            }
        }

        impl std::ops::SubAssign<usize> for $Type {
            #[inline]
            fn sub_assign(&mut self, rhs: usize) {
                self.0 -= rhs;
            }
        }

        impl std::fmt::Display for $Type {
            #[inline]
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

impl_text_index!(ByteIndex);
impl_text_index!(CharIndex);

/// A range of [`ByteIndex`], i.e. a byte range into a [`str`].
pub type ByteRange = Range<ByteIndex>;

/// A range of [`CharIndex`], i.e. a character range into a [`str`].
pub type CharRange = Range<CharIndex>;

/// Extension methods for a [`ByteRange`].
pub trait ByteRangeExt {
    /// The full byte range covering `text`, i.e. `0..text.len()`.
    fn full(text: &str) -> Self;

    /// The `start..end` byte range as plain `usize`, for slicing a [`str`].
    fn as_usize(&self) -> Range<usize>;

    /// Slice the given string by this byte range.
    fn slice<'s>(&self, text: &'s str) -> &'s str;
}

impl ByteRangeExt for ByteRange {
    #[inline]
    fn full(text: &str) -> Self {
        ByteIndex::ZERO..ByteIndex(text.len())
    }

    #[inline]
    fn as_usize(&self) -> Range<usize> {
        self.start.0..self.end.0
    }

    #[inline]
    fn slice<'s>(&self, text: &'s str) -> &'s str {
        &text[self.as_usize()]
    }
}

/// Extension methods for a [`CharRange`].
pub trait CharRangeExt {
    /// The full character range covering `text`, i.e. `0..text.chars().count()`.
    fn full(text: &str) -> Self;
}

impl CharRangeExt for CharRange {
    #[inline]
    fn full(text: &str) -> Self {
        CharIndex::ZERO..CharIndex(text.chars().count())
    }
}

#[cfg(test)]
mod tests {
    use super::CharIndex;

    #[test]
    fn arithmetic() {
        // Add a relative offset to a base position:
        assert_eq!(CharIndex(2) + CharIndex(3), CharIndex(5));
        assert_eq!(CharIndex(2) + 3, CharIndex(5));

        let mut idx = CharIndex(2);
        idx += CharIndex(3);
        assert_eq!(idx, CharIndex(5));

        // Subtract a relative offset from a position:
        assert_eq!(CharIndex(5) - CharIndex(2), CharIndex(3));
        assert_eq!(CharIndex(5) - 2, CharIndex(3));
    }
}
