//! How a number is written, read back, and applied while being edited.
//!
//! Used by [`crate::DragValue`], [`crate::Slider`], and [`crate::RangeSlider`].

use core::ops::RangeInclusive;
use std::sync::Arc;

use crate::{Atoms, IntoAtoms, MINUS_CHAR_STR, emath};

/// Formats a value for display, given the range of decimals it may show.
pub type NumFormatter<'a> = Arc<dyn 'a + Fn(f64, RangeInclusive<usize>) -> String>;

/// Parses what the user typed, returning `None` if it is not a number.
pub type NumParser<'a> = Arc<dyn 'a + Fn(&str) -> Option<f64>>;

// ----------------------------------------------------------------------------

/// How a number is written, read back, and applied while being edited.
///
/// Owned by [`crate::DragValue`], and by [`crate::Slider`] and [`crate::RangeSlider`]
/// for the numbers they show beside the rail.
#[derive(Clone)]
pub struct ValueFormat<'a> {
    /// Shown before the number, e.g. `"x: "`.
    pub prefix: Atoms<'a>,

    /// Shown after the number, e.g. a unit.
    pub suffix: Atoms<'a>,

    /// Show at least this many decimals.
    pub min_decimals: usize,

    /// Show at most this many decimals, and round values to it.
    pub max_decimals: Option<usize>,

    /// Turns a value into text. Default: [`crate::style::NumberFormatter`].
    pub custom_formatter: Option<NumFormatter<'a>>,

    /// Turns text back into a value. Default: accepts plain decimal numbers.
    pub custom_parser: Option<NumParser<'a>>,

    /// Apply each keystroke while typing, rather than on enter.
    pub update_while_editing: bool,
}

impl Default for ValueFormat<'_> {
    fn default() -> Self {
        Self {
            prefix: Atoms::default(),
            suffix: Atoms::default(),
            min_decimals: 0,
            max_decimals: None,
            custom_formatter: None,
            custom_parser: None,
            update_while_editing: true,
        }
    }
}

impl<'a> ValueFormat<'a> {
    /// Show a prefix before the number, e.g. "x: ".
    ///
    /// Goes in front of any prefix already set, so `.prefix("b").prefix("a")` shows `ab`.
    #[inline]
    pub fn prefix(mut self, prefix: impl IntoAtoms<'a>) -> Self {
        self.prefix.extend_left(prefix.into_atoms());
        self
    }

    /// Add a suffix to the number, this can be e.g. a unit ("°" or " m").
    ///
    /// Goes after any suffix already set, so `.suffix("a").suffix("b")` shows `ab`.
    #[inline]
    pub fn suffix(mut self, suffix: impl IntoAtoms<'a>) -> Self {
        self.suffix.extend_right(suffix.into_atoms());
        self
    }

    /// Set a minimum number of decimals to display.
    #[inline]
    pub fn min_decimals(mut self, min_decimals: usize) -> Self {
        self.min_decimals = min_decimals;
        self
    }

    /// Set a maximum number of decimals to display.
    /// Values will also be rounded to this number of decimals.
    #[inline]
    pub fn max_decimals(mut self, max_decimals: usize) -> Self {
        self.max_decimals = Some(max_decimals);
        self
    }

    #[inline]
    pub fn max_decimals_opt(mut self, max_decimals: Option<usize>) -> Self {
        self.max_decimals = max_decimals;
        self
    }

    /// Set an exact number of decimals to display.
    /// Values will also be rounded to this number of decimals.
    #[inline]
    pub fn fixed_decimals(mut self, num_decimals: usize) -> Self {
        self.min_decimals = num_decimals;
        self.max_decimals = Some(num_decimals);
        self
    }

    /// Set custom formatter defining how numbers are converted into text.
    ///
    /// A custom formatter takes a `f64` for the numeric value and a `RangeInclusive<usize>` representing
    /// the decimal range i.e. minimum and maximum number of decimal places shown.
    ///
    /// See also: [`Self::custom_parser`]
    pub fn custom_formatter(
        mut self,
        formatter: impl 'a + Fn(f64, RangeInclusive<usize>) -> String,
    ) -> Self {
        self.custom_formatter = Some(Arc::new(formatter));
        self
    }

    /// Set custom parser defining how the text input is parsed into a number.
    ///
    /// A custom parser takes an `&str` to parse into a number and returns a `f64` if it was successfully parsed
    /// or `None` otherwise.
    ///
    /// See also: [`Self::custom_formatter`]
    #[inline]
    pub fn custom_parser(mut self, parser: impl 'a + Fn(&str) -> Option<f64>) -> Self {
        self.custom_parser = Some(Arc::new(parser));
        self
    }

    /// Set `custom_formatter` and `custom_parser` to display and parse numbers as binary integers. Floating point
    /// numbers are *not* supported.
    ///
    /// `min_width` specifies the minimum number of displayed digits; if the number is shorter than this, it will be
    /// prefixed with additional 0s to match `min_width`.
    ///
    /// If `twos_complement` is true, negative values will be displayed as the 2's complement representation. Otherwise
    /// they will be prefixed with a '-' sign.
    ///
    /// # Panics
    ///
    /// Panics if `min_width` is 0.
    pub fn binary(self, min_width: usize, twos_complement: bool) -> Self {
        assert!(
            0 < min_width,
            "ValueFormat::binary: `min_width` must be greater than 0"
        );
        if twos_complement {
            self.custom_formatter(move |n, _| format!("{:0>min_width$b}", n as i64))
        } else {
            self.custom_formatter(move |n, _| {
                let sign = if n < 0.0 { MINUS_CHAR_STR } else { "" };
                format!("{sign}{:0>min_width$b}", n.abs() as i64)
            })
        }
        .custom_parser(|s| i64::from_str_radix(s, 2).map(|n| n as f64).ok())
    }

    /// Set `custom_formatter` and `custom_parser` to display and parse numbers as octal integers. Floating point
    /// numbers are *not* supported.
    ///
    /// `min_width` specifies the minimum number of displayed digits; if the number is shorter than this, it will be
    /// prefixed with additional 0s to match `min_width`.
    ///
    /// If `twos_complement` is true, negative values will be displayed as the 2's complement representation. Otherwise
    /// they will be prefixed with a '-' sign.
    ///
    /// # Panics
    ///
    /// Panics if `min_width` is 0.
    pub fn octal(self, min_width: usize, twos_complement: bool) -> Self {
        assert!(
            0 < min_width,
            "ValueFormat::octal: `min_width` must be greater than 0"
        );
        if twos_complement {
            self.custom_formatter(move |n, _| format!("{:0>min_width$o}", n as i64))
        } else {
            self.custom_formatter(move |n, _| {
                let sign = if n < 0.0 { MINUS_CHAR_STR } else { "" };
                format!("{sign}{:0>min_width$o}", n.abs() as i64)
            })
        }
        .custom_parser(|s| i64::from_str_radix(s, 8).map(|n| n as f64).ok())
    }

    /// Set `custom_formatter` and `custom_parser` to display and parse numbers as hexadecimal integers. Floating point
    /// numbers are *not* supported.
    ///
    /// `min_width` specifies the minimum number of displayed digits; if the number is shorter than this, it will be
    /// prefixed with additional 0s to match `min_width`.
    ///
    /// If `twos_complement` is true, negative values will be displayed as the 2's complement representation. Otherwise
    /// they will be prefixed with a '-' sign.
    ///
    /// # Panics
    ///
    /// Panics if `min_width` is 0.
    pub fn hexadecimal(self, min_width: usize, twos_complement: bool, upper: bool) -> Self {
        assert!(
            0 < min_width,
            "ValueFormat::hexadecimal: `min_width` must be greater than 0"
        );
        match (twos_complement, upper) {
            (true, true) => {
                self.custom_formatter(move |n, _| format!("{:0>min_width$X}", n as i64))
            }
            (true, false) => {
                self.custom_formatter(move |n, _| format!("{:0>min_width$x}", n as i64))
            }
            (false, true) => self.custom_formatter(move |n, _| {
                let sign = if n < 0.0 { MINUS_CHAR_STR } else { "" };
                format!("{sign}{:0>min_width$X}", n.abs() as i64)
            }),
            (false, false) => self.custom_formatter(move |n, _| {
                let sign = if n < 0.0 { MINUS_CHAR_STR } else { "" };
                format!("{sign}{:0>min_width$x}", n.abs() as i64)
            }),
        }
        .custom_parser(|s| i64::from_str_radix(s, 16).map(|n| n as f64).ok())
    }

    /// Update the value on each key press when text-editing the value.
    ///
    /// Default: `true`.
    /// If `false`, the value will only be updated when user presses enter or deselects the value.
    #[inline]
    pub fn update_while_editing(mut self, update: bool) -> Self {
        self.update_while_editing = update;
        self
    }

    /// Rounds `value` to `max_decimals`, if set.
    pub fn round(&self, value: f64) -> f64 {
        match self.max_decimals {
            Some(max_decimals) => emath::round_to_decimals(value, max_decimals),
            None => value,
        }
    }
}

// ----------------------------------------------------------------------------
