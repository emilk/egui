use super::{Action, InputData};

use std::{borrow::Cow, cmp};

/// A type that implement user input validation.
pub trait ValidateInput {
    /// Determine how to treat the user `input` based on the current
    /// value of the `buffer`.
    fn validate_input(data: InputData<'_>) -> Action;
}

impl ValidateInput for String {
    fn validate_input(data: InputData<'_>) -> Action {
        Action::Insert(data.input)
    }
}

impl ValidateInput for str {
    fn validate_input(data: InputData<'_>) -> Action {
        Action::Insert(data.input)
    }
}

macro_rules! impl_validate_input_unsigned {
    ($type: ty, $max_str: expr) => {
        impl ValidateInput for $type {
            fn validate_input(data: InputData<'_>) -> Action {
                let InputData { buffer, input, .. } = data;
                let max_str = $max_str;
                let max_str_len = max_str.len();

                // TODO should we assume that the user did not add data
                // to the buffer from outside the UI?

                // Filter out all the non-numeric characters.
                let filter = input.as_str().replace(|c: char| !c.is_numeric(), "");

                let new_size = buffer.len() + filter.len();

                match new_size.cmp(&max_str_len) {
                    // Guaranteed to be below the limit so we insert.
                    cmp::Ordering::Less => Action::Insert(filter),
                    // Guaranteed to exceed the limit so we overwrite.
                    cmp::Ordering::Greater => Action::Overwrite(Cow::Borrowed(max_str)),
                    // Slow case where we need to actually compare with the limit.
                    cmp::Ordering::Equal => {
                        // Note that we don't parse the strings because its too slow.
                        let mut iter = buffer.chars();
                        let front = max_str[..buffer.len()].chars();
                        for (current, max) in (&mut iter).zip(front) {
                            if current > max {
                                return Action::Overwrite(Cow::Borrowed(max_str));
                            }
                        }

                        let back = max_str[buffer.len()..].chars();
                        for (current, max) in iter.zip(back) {
                            if current > max {
                                return Action::Overwrite(Cow::Borrowed(max_str));
                            }
                        }

                        Action::Insert(filter)
                    }
                }
            }
        }
    };
}

impl_validate_input_unsigned!(u8, "255");
impl_validate_input_unsigned!(u16, "65535");
impl_validate_input_unsigned!(u32, "4294967295");
impl_validate_input_unsigned!(u64, "18446744073709551615");

macro_rules! impl_validate_input_signed {
    ($type: ty, $min_str: expr, $max_str: expr) => {
        impl ValidateInput for $type {
            fn validate_input(data: InputData<'_>) -> Action {
                let InputData {
                    buffer,
                    input,
                    cursor_start,
                } = data;

                let max_str = $max_str;
                let min_str = $min_str;
                let max_str_len = max_str.len();
                let min_str_len = min_str.len();

                // TODO should we assume that the user did not add data
                // to the buffer from outside the UI?

                // First we filter out all non-numeric characters,
                // but we allow `-` at the start.
                let mut is_neg = if buffer.is_empty() {
                    false
                } else {
                    buffer.starts_with('-')
                };
                let mut filter = String::new();

                for (i, c) in input.chars().enumerate() {
                    // Allow `-` at the start if not already negative.
                    if (i == 0 && cursor_start == 0 && c == '-' && !is_neg) {
                        is_neg = true;
                        filter.push('-');
                    } else if c.is_numeric() {
                        filter.push(c);
                    }
                }

                let new_size = buffer.len() + filter.len();

                if is_neg {
                    match new_size.cmp(&min_str_len) {
                        // Guaranteed to be below the limit so we insert.
                        cmp::Ordering::Less => Action::Insert(filter),
                        // Guaranteed to exceed the limit so we overwrite.
                        cmp::Ordering::Greater => Action::Overwrite(Cow::Borrowed(min_str)),
                        // Slow case where we need to actually compare with the limit.
                        cmp::Ordering::Equal => {
                            // Note that we don't parse the strings because its too slow.
                            let mut iter = buffer.chars();
                            let front = min_str[..buffer.len()].chars();
                            for (current, min) in (&mut iter).zip(front) {
                                if current < min {
                                    return Action::Overwrite(Cow::Borrowed(min_str));
                                }
                            }

                            let back = min_str[buffer.len()..].chars();
                            for (current, min) in iter.zip(back) {
                                if current > min {
                                    return Action::Overwrite(Cow::Borrowed(min_str));
                                }
                            }

                            Action::Insert(filter)
                        }
                    }
                } else {
                    match new_size.cmp(&max_str_len) {
                        // Guaranteed to be below the limit so we insert.
                        cmp::Ordering::Less => Action::Insert(filter),
                        // Guaranteed to exceed the limit so we overwrite.
                        cmp::Ordering::Greater => Action::Overwrite(Cow::Borrowed(max_str)),
                        // Slow case where we need to actually compare with the limit.
                        cmp::Ordering::Equal => {
                            // Note that we don't parse the strings because its too slow.
                            let mut iter = buffer.chars();
                            let front = max_str[..buffer.len()].chars();
                            for (current, max) in (&mut iter).zip(front) {
                                if current > max {
                                    return Action::Overwrite(Cow::Borrowed(max_str));
                                }
                            }

                            let back = max_str[buffer.len()..].chars();
                            for (current, max) in iter.zip(back) {
                                if current > max {
                                    return Action::Overwrite(Cow::Borrowed(max_str));
                                }
                            }

                            Action::Insert(filter)
                        }
                    }
                }
            }
        }
    };
}

impl_validate_input_signed!(i8, "-128", "127");
impl_validate_input_signed!(i16, "-32768", "32767");
impl_validate_input_signed!(i32, "-2147483648", "2147483647");
impl_validate_input_signed!(i64, "-9223372036854775808", "9223372036854775807");
