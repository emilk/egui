//! Input mask for the editable [`super::DatePickerButton`].
//!
//! The mask is derived from the `strftime` format (e.g. `%Y-%m-%d`). Only numeric formats
//! made of `%Y`, `%y`, `%m`, `%d` and literal separators can be masked; for anything else
//! [`DateMask::from_format`] returns `None` and the text is left unmasked.

/// The part of a date a group of digits stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DateField {
    Year,
    Month,
    Day,
}

impl DateField {
    /// A span that moves a date by `delta` units of this field.
    pub fn span(self, delta: i64) -> jiff::Span {
        match self {
            Self::Year => jiff::Span::new().years(delta),
            Self::Month => jiff::Span::new().months(delta),
            Self::Day => jiff::Span::new().days(delta),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MaskPart {
    Digits { field: DateField, len: usize },
    Literal(char),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DateMask {
    parts: Vec<MaskPart>,
}

impl DateMask {
    /// Builds a mask from a `strftime` format, or `None` if the format can't be masked.
    pub fn from_format(format: &str) -> Option<Self> {
        let mut parts = Vec::new();
        let mut chars = format.chars();
        while let Some(c) = chars.next() {
            if c == '%' {
                let part = match chars.next()? {
                    'Y' => MaskPart::Digits {
                        field: DateField::Year,
                        len: 4,
                    },
                    'y' => MaskPart::Digits {
                        field: DateField::Year,
                        len: 2,
                    },
                    'm' => MaskPart::Digits {
                        field: DateField::Month,
                        len: 2,
                    },
                    'd' => MaskPart::Digits {
                        field: DateField::Day,
                        len: 2,
                    },
                    '%' => MaskPart::Literal('%'),
                    _ => return None,
                };
                parts.push(part);
            } else if c.is_alphanumeric() {
                return None;
            } else {
                parts.push(MaskPart::Literal(c));
            }
        }

        let has_digits = parts
            .iter()
            .any(|part| matches!(part, MaskPart::Digits { .. }));
        has_digits.then_some(Self { parts })
    }

    /// How many digits a complete date has.
    pub fn digit_count(&self) -> usize {
        self.parts
            .iter()
            .map(|part| match part {
                MaskPart::Digits { len, .. } => *len,
                MaskPart::Literal(_) => 0,
            })
            .sum()
    }

    /// Is this character allowed when typing?
    pub fn accepts(&self, c: char) -> bool {
        c.is_ascii_digit()
            || self
                .parts
                .iter()
                .any(|part| matches!(part, MaskPart::Literal(literal) if *literal == c))
    }

    /// Which field the (char index) `cursor` is in or right after, for a complete text.
    pub fn field_at(&self, cursor: usize) -> Option<DateField> {
        let mut start = 0;
        let mut found = None;
        for part in &self.parts {
            match part {
                MaskPart::Digits { field, len } => {
                    if cursor >= start && cursor <= start + len {
                        // Keep looking only if we are exactly between two fields: prefer the
                        // one on the left, as that's the one just edited.
                        return Some(*field);
                    }
                    if found.is_none() {
                        found = Some(*field);
                    }
                    start += len;
                }
                MaskPart::Literal(_) => start += 1,
            }
        }
        found
    }

    /// Reformats whatever the user typed (or pasted) into the mask.
    ///
    /// Everything but digits is dropped, separators are inserted where the mask needs them,
    /// and extra digits are cut. A separator is only kept at the very end if the user just
    /// typed it, so that backspace can still delete it.
    ///
    /// `cursor` is a char index into `raw`. Returns the new text and the new cursor index.
    pub fn sanitize(&self, raw: &str, cursor: usize) -> (String, usize) {
        let chars: Vec<char> = raw.chars().collect();
        let cursor = cursor.min(chars.len());
        let total = self.digit_count();

        let mut digits = Vec::new();
        let mut digits_before_cursor = 0;
        for (index, c) in chars.iter().enumerate() {
            if c.is_ascii_digit() && digits.len() < total {
                digits.push(*c);
                if index < cursor {
                    digits_before_cursor += 1;
                }
            }
        }

        let mut out = String::new();
        let mut out_len = 0;
        let mut digit_ends = Vec::new(); // char index in `out` just after each digit
        let mut placed = 0;
        let mut digits_in_previous_parts = 0;
        for part in &self.parts {
            match part {
                MaskPart::Digits { len, .. } => {
                    for _ in 0..*len {
                        if placed < digits.len() {
                            out.push(digits[placed]);
                            out_len += 1;
                            placed += 1;
                            digit_ends.push(out_len);
                        }
                    }
                    digits_in_previous_parts += len;
                }
                MaskPart::Literal(literal) => {
                    if placed < digits.len() {
                        out.push(*literal);
                        out_len += 1;
                    } else {
                        let at_group_end = placed > 0 && placed == digits_in_previous_parts;
                        if at_group_end && chars.last() == Some(literal) {
                            out.push(*literal);
                        }
                        break;
                    }
                }
            }
        }

        let mut new_cursor = match digits_before_cursor {
            0 => 0,
            n => digit_ends[n - 1],
        };
        // If the user typed the separator right before the cursor, step over it.
        if cursor > 0 && !chars[cursor - 1].is_ascii_digit() {
            let typed = chars[cursor - 1];
            if out.chars().nth(new_cursor) == Some(typed) {
                new_cursor += 1;
            }
        }

        (out, new_cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::{DateField, DateMask};

    fn iso() -> DateMask {
        DateMask::from_format("%Y-%m-%d").expect("iso format can be masked")
    }

    #[test]
    fn unsupported_formats_have_no_mask() {
        assert!(DateMask::from_format("%B %d, %Y").is_none());
        assert!(DateMask::from_format("%-d/%m/%Y").is_none());
        assert!(DateMask::from_format("Day %d").is_none());
        assert!(DateMask::from_format("---").is_none());
    }

    #[test]
    fn other_numeric_formats_work() {
        let mask = DateMask::from_format("%d/%m/%Y").unwrap();
        assert_eq!(mask.digit_count(), 8);
        assert_eq!(mask.sanitize("17112026", 8), ("17/11/2026".to_owned(), 10));
    }

    #[test]
    fn inserts_separators_when_more_digits_follow() {
        let mask = iso();
        assert_eq!(mask.sanitize("2026", 4), ("2026".to_owned(), 4));
        assert_eq!(mask.sanitize("20261", 5), ("2026-1".to_owned(), 6));
        assert_eq!(mask.sanitize("202611", 6), ("2026-11".to_owned(), 7));
        assert_eq!(mask.sanitize("2026-111", 8), ("2026-11-1".to_owned(), 9));
    }

    #[test]
    fn drops_letters_and_extra_digits() {
        let mask = iso();
        assert_eq!(mask.sanitize("20a26", 5), ("2026".to_owned(), 4));
        assert_eq!(
            mask.sanitize("2026-11-170", 11),
            ("2026-11-17".to_owned(), 10)
        );
    }

    #[test]
    fn pasting_unformatted_digits() {
        let mask = iso();
        assert_eq!(mask.sanitize("20261117", 8), ("2026-11-17".to_owned(), 10));
        assert_eq!(
            mask.sanitize("2026/11/17", 10),
            ("2026-11-17".to_owned(), 10)
        );
    }

    #[test]
    fn typed_separator_is_kept_only_at_a_group_end() {
        let mask = iso();
        assert_eq!(mask.sanitize("2026-", 5), ("2026-".to_owned(), 5));
        assert_eq!(mask.sanitize("20-", 3), ("20".to_owned(), 2));
        // Backspace over it removes it for good.
        assert_eq!(mask.sanitize("2026", 4), ("2026".to_owned(), 4));
    }

    #[test]
    fn cursor_follows_digits_when_editing_in_the_middle() {
        let mask = iso();
        // Typing a digit after "20" in "2026-11-17" shifts everything and drops the last digit.
        assert_eq!(
            mask.sanitize("20926-11-17", 3),
            ("2092-61-11".to_owned(), 3)
        );
        // Deleting a digit pulls the rest to the left.
        assert_eq!(mask.sanitize("202-11-17", 3), ("2021-11-7".to_owned(), 3));
    }

    #[test]
    fn empty_text() {
        assert_eq!(iso().sanitize("", 0), (String::new(), 0));
    }

    #[test]
    fn field_under_the_cursor() {
        let mask = iso();
        assert_eq!(mask.field_at(0), Some(DateField::Year));
        assert_eq!(mask.field_at(4), Some(DateField::Year));
        assert_eq!(mask.field_at(5), Some(DateField::Month));
        assert_eq!(mask.field_at(7), Some(DateField::Month));
        assert_eq!(mask.field_at(8), Some(DateField::Day));
        assert_eq!(mask.field_at(10), Some(DateField::Day));
    }

    #[test]
    fn accepts_digits_and_separators_only() {
        let mask = iso();
        assert!(mask.accepts('7'));
        assert!(mask.accepts('-'));
        assert!(!mask.accepts('/'));
        assert!(!mask.accepts('a'));
    }
}
