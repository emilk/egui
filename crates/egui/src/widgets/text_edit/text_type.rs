//! Contains [`TextType`] alongide its implementations.
use std::{borrow::Cow, convert::Infallible, error::Error, fmt::Display};

/// Represents types can be displayed and validated by a [`TextEdit`].
///
/// All [`TextType`]s can represent their value as a string, which is displayed by a [`TextEdit`].
/// If a [`TextType`] can be parsed from a string the [`TextEdit`] will attempt to parse and update
/// the [`TextType`] value. If parsing fails, the value of the [`TextType`] willnot change.
/// This ensures that a [`TextType`] can never represent an invalid value.
///
/// [`TextType`] is implemented for many of the numeric and string types (including references) within
/// the standard library. If custom parsing behavior is needed, or an implementation does not exist,
/// the [`New Type`] pattern can be used.
///
/// ## Example Implementation
/// These are example for a string that cannot contain any capital letters.
/// This example converts any text the user enters to lowercase.
#[doc = "```
# use egui::TextType;
struct NoCaps(String);

impl TextType for NoCaps {
    type Err = std::convert::Infallible;

    fn read_from_string(_previous: &Self, modified: &str) -> Option<Result<Self, Self::Err>> {
        Some(Ok(NoCaps(modified.to_lowercase())))
    }

    fn string_representation(&self) -> String {
        self.0.clone()
    }

    fn is_parsable() -> bool {
        true
    }
}
```"]
/// An alternate implementation may choose to reject user input if it contains any capital letters.
#[doc = "```
# use egui::TextType;
struct NoCaps(String);

impl TextType for NoCaps {
    // Type implementation hidden for brevity
    type Err = IncorrectCaseError;

    fn read_from_string(_previous: &Self, modified: &str) -> Option<Result<Self, Self::Err>> {
        if modified.to_lowercase() == modified {
            Some(Ok(NoCaps(modified.to_owned())))
        } else {
            Some(Err(IncorrectCaseError(
                \"Contained uppercase letters\".to_owned(),
            )))
        }
    }

    fn string_representation(&self) -> String {
        self.0.clone()
    }

    fn is_parsable() -> bool {
        true
    }
}
# #[derive(Debug)]
# pub struct IncorrectCaseError(String);
# impl std::fmt::Display for IncorrectCaseError {
#     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
#         f.write_str(&self.0)
#     }
# }
# impl std::error::Error for IncorrectCaseError {}
```"]
///
/// [`TextEdit`]: super::TextEdit
/// [`New Type`]: https://doc.rust-lang.org/rust-by-example/generics/new_types.html
pub trait TextType: Sized {
    /// Error returned when [`read_from_string`] parsing fails.
    /// If this parsing cannot fail, then [`Infallible`] can be used.
    ///
    /// [`read_from_string`]: TextType::read_from_string()
    /// [`Infallible`]: std::convert::Infallible
    type Err: Error;

    /// Parses the string representation of the implementing data type.
    /// This **must** be able to parse output from [`TextType::string_representation`].
    ///
    /// Types that cannot be parsed will return `None`.
    /// ```
    /// # use egui::TextType;
    /// assert!(!<&f32 as TextType>::is_parsable());
    /// assert!(<&f32 as TextType>::read_from_string(&&0.1, "0.11").is_none());
    /// ```
    ///
    /// Types that are parsable will return the result of parsing.
    /// ```
    /// # use egui::TextType;
    /// assert!(f32::is_parsable());
    ///
    /// assert_eq!(<f32 as TextType>::read_from_string(&0.1, "0.11"), Some(Ok(0.11)));
    /// assert!(<f32 as TextType>::read_from_string(&0.1, "0.1a").unwrap().is_err());
    /// ```
    ///
    /// ## Example Implementations
    /// See [`TextType`] documentation and its module for examples.
    fn read_from_string(previous: &Self, modified: &str) -> Option<Result<Self, Self::Err>>;

    /// Generates the string representation of this type.
    /// This **must** be parseable by [`TextType::read_from_string`].
    ///
    /// ## Example
    /// ```
    /// # use egui::TextType;
    /// let age = -42i32;
    /// let text = age.string_representation();
    ///
    /// assert_eq!(text, "-42".to_owned());
    ///
    /// // Using unwrap to keep example short
    /// let parsed = TextType::read_from_string(&age, &text).unwrap().unwrap();
    /// assert_eq!(age, parsed);
    /// ```
    fn string_representation(&self) -> String;

    /// Whether this data type can be modified.
    ///
    /// If true for a data type cannot be modified (such as a referenced type), it will appear editable, but no modifications will persist.
    /// This will not cause unexpected behavior but will be confusing for users.
    ///
    /// ## Example
    /// ```
    /// # use egui::TextType;
    /// # use std::{borrow::Cow, num::NonZeroI32};
    /// // These types are mutable since they can modify their data.
    /// assert!(String::is_parsable());
    /// assert!(<Cow<'_, str> as TextType>::is_parsable());
    /// assert!(NonZeroI32::is_parsable());
    ///
    /// // These types are immutable since they cannot modify their data.
    /// assert!(!<&str as TextType>::is_parsable());
    /// assert!(!<&char as TextType>::is_parsable());
    /// assert!(!<&f32 as TextType>::is_parsable());
    /// ```
    #[doc(alias = "is_parseable")]
    fn is_parsable() -> bool;
}

/// A generic error that can occur when parsing a type as [`TextType`].
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
        (*self).to_owned()
    }

    fn is_parsable() -> bool {
        false
    }
}

impl TextType for String {
    type Err = Infallible;

    fn read_from_string(_previous: &Self, modified: &str) -> Option<Result<Self, Self::Err>> {
        Some(Ok(modified.to_owned()))
    }

    fn string_representation(&self) -> String {
        self.to_string()
    }

    fn is_parsable() -> bool {
        true
    }
}

impl TextType for char {
    type Err = ConversionError;

    fn read_from_string(previous: &Self, modified: &str) -> Option<Result<Self, Self::Err>> {
        let modified: Vec<Self> = modified.chars().collect();

        Some(match (modified.first(), modified.get(1), modified.get(2)) {
            (Some(_), Some(_), Some(_)) => Err(ConversionError(
                "Three or more characters present".to_owned(),
            )),
            (Some(first), Some(second), None) if first == previous => Ok(*second),
            (Some(first), Some(second), None) if first == second => Ok(*first),
            (Some(_), Some(_), None) => Err(ConversionError(
                "Two different characters present".to_owned(),
            )),
            (None, _, _) => Err(ConversionError("Zero characters present".to_owned())),
            (Some(only), _, _) => Ok(*only),
        })
    }

    fn string_representation(&self) -> String {
        self.to_string()
    }

    fn is_parsable() -> bool {
        true
    }
}

impl TextType for &char {
    type Err = Infallible;

    fn read_from_string(_previous: &Self, _modified: &str) -> Option<Result<Self, Self::Err>> {
        None
    }

    fn string_representation(&self) -> String {
        self.to_string()
    }

    fn is_parsable() -> bool {
        false
    }
}

impl TextType for Cow<'_, str> {
    type Err = Infallible;

    fn read_from_string(_previous: &Self, modified: &str) -> Option<Result<Self, Self::Err>> {
        Some(Ok(Cow::from(modified.to_owned())))
    }

    fn string_representation(&self) -> String {
        self.to_string()
    }

    fn is_parsable() -> bool {
        true
    }
}

/// Implementation for number types.
mod num_impls {
    /// Reduces repetition in implementation and tests for implementing on numeric types.
    macro_rules! num_impl {
        ($num:path, $err:path; $test_name:ident, $($init:expr),*) => {
            impl super::TextType for $num {
                type Err = $err;

                fn read_from_string(
                    _previous: &Self,
                    modified: &str,
                ) -> Option<Result<Self, Self::Err>> {
                    Some(modified.parse())
                }

                fn string_representation(&self) -> String {
                    self.to_string()
                }

                fn is_parsable() -> bool {
                    true
                }
            }

            impl super::TextType for &$num {
                type Err = std::convert::Infallible;

                fn read_from_string(
                    _previous: &Self,
                    _modified: &str,
                ) -> Option<Result<Self, Self::Err>> {
                    None
                }

                fn string_representation(&self) -> String {
                    self.to_string()
                }

                fn is_parsable() -> bool {
                    false
                }
            }
            // Requires separate parameter as an "identity" cannot be constructed in a declarative macro
            #[test]
            fn $test_name() {
                use super::TextType;
                // Test if values can be parsed from it's string representation
                $(
                    let string = $init.string_representation();
                    let parsed_string = TextType::read_from_string(&$init, &string).expect("Can Parse");
                    assert_eq!(Ok($init), parsed_string, stringify!(Failed parsing $num with value of $init));
                    assert!(TextType::read_from_string(&(&$init), &string).is_none(), stringify!(Parsing a reference (&$init) must return None));
                )*
                // Test mutability
                assert!(<$num as TextType>::is_parsable(), stringify!($num must be mutable));
                assert!(!<&$num as TextType>::is_parsable(), stringify!(&$num must not be mutable));
            }
        };
        ($num:path; $($tail:tt)*) => {
            num_impl!($num, std::num::ParseIntError; $($tail)*);
        };
    }

    num_impl!(u8; u8_test, 0, 1);
    num_impl!(u16; u16_test, 0, 1);
    num_impl!(u32; u32_test, 0, 1);
    num_impl!(u64; u64_test, 0, 1);
    num_impl!(u128; u128_test, 0, 1);
    num_impl!(usize; usize_test, 0, 1);
    num_impl!(i8; i8_test, -1, 0, 1);
    num_impl!(i16; i16_test, -1, 0, 1);
    num_impl!(i32; i32_test, -1, 0, 1);
    num_impl!(i64; i64_test, -1, 0, 1);
    num_impl!(i128; i128_test, -1, 0, 1);
    num_impl!(isize; isize_test, -1, 0, 1);

    // These imports also affect the macro.
    use std::num::{
        NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128, NonZeroIsize, NonZeroU8,
        NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128, NonZeroUsize,
    };
    num_impl!(NonZeroU8; non0u8_test, NonZeroU8::MIN, NonZeroU8::MAX);
    num_impl!(NonZeroU16; non0u16_test, NonZeroU16::MIN, NonZeroU16::MAX);
    num_impl!(NonZeroU32; non0u32_test, NonZeroU32::MIN, NonZeroU32::MAX);
    num_impl!(NonZeroU64; non0u64_test, NonZeroU64::MIN, NonZeroU64::MAX);
    num_impl!(NonZeroU128; non0u128_test, NonZeroU128::MIN, NonZeroU128::MAX);
    num_impl!(NonZeroUsize; non0usize_test, NonZeroUsize::MIN, NonZeroUsize::MAX);
    num_impl!(NonZeroI8; non0i8_test, NonZeroI8::MIN, NonZeroI8::MAX);
    num_impl!(NonZeroI16; non0i16_test, NonZeroI16::MIN, NonZeroI16::MAX);
    num_impl!(NonZeroI32; non0i32_test, NonZeroI32::MIN, NonZeroI32::MAX);
    num_impl!(NonZeroI64; non0i64_test, NonZeroI64::MIN, NonZeroI64::MAX);
    num_impl!(NonZeroI128; non0i128_test, NonZeroI128::MIN, NonZeroI128::MAX);
    num_impl!(NonZeroIsize; non0isize_test, NonZeroIsize::MIN, NonZeroIsize::MAX);

    // NAN can be parsed, it just errors since NAN != NAN
    num_impl!(f32, std::num::ParseFloatError; f32_test, -1.0, 0.0, 1.0, f32::INFINITY, f32::NEG_INFINITY);
    num_impl!(f64, std::num::ParseFloatError; f64_test, -1.0, 0.0, 1.0, f64::INFINITY, f64::NEG_INFINITY);
}
