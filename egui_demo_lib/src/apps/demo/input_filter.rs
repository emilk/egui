use std::str;

use super::*;
use egui::{
    widgets::{
        text_edit::{Action, FilterInput, InputData},
        TextEdit,
    },
    *,
};

/// Showcase text input filtering.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
#[derive(Default)]
pub struct InputFilterWindow {
    u8_buffer: String,
    u16_buffer: String,
    u32_buffer: String,
    u64_buffer: String,
    i8_buffer: String,
    i16_buffer: String,
    i32_buffer: String,
    i64_buffer: String,
    ascii_buffer: String,
}

// A optionally nul terminated ASCII string of at most 16 characters.
struct AsciiString([u8; 16]);

impl AsciiString {
    const CAPACITY: usize = 16;

    fn as_bytes(&self) -> &[u8] {
        match self.0.iter().position(|&b| b == 0) {
            Some(index) => &self.0[..index],
            None => &self.0,
        }
    }

    fn as_str(&self) -> &str {
        str::from_utf8(self.as_bytes()).unwrap()
    }
}

#[derive(Debug)]
enum BadString {
    TooLong,
    BadChar(char),
}

impl std::fmt::Display for BadString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLong => write!(f, "string exceeds maximum capacity"),
            Self::BadChar(c) => write!(f, "invalid character: {}", c),
        }
    }
}

impl std::error::Error for BadString {}

impl std::str::FromStr for AsciiString {
    type Err = BadString;
    fn from_str(s: &str) -> Result<Self, BadString> {
        let mut arr = [0u8; 16];
        for (i, c) in s.chars().enumerate() {
            if !c.is_ascii() {
                return Err(BadString::BadChar(c));
            }
            if i == AsciiString::CAPACITY {
                return Err(BadString::TooLong);
            }
            // Shouldn't panic since we checked that this is a ASCII character.
            arr[i] = u32::from(c).try_into().unwrap();
        }

        Ok(Self(arr))
    }
}

// Clippy being annoying since it does not consider whether a function
// is used as a function pointer.
// https://github.com/rust-lang/rust-clippy/issues/2434
#[allow(clippy::needless_pass_by_value)]
fn filter_ascii_string(data: InputData<'_>) -> Action {
    let filtered = data.input.as_str().replace(|c: char| !c.is_ascii(), "");

    if data.buffer.len() + filtered.len() > AsciiString::CAPACITY {
        Action::Insert(String::new())
    } else {
        Action::Insert(filtered)
    }
}

impl Demo for InputFilterWindow {
    fn name(&self) -> &'static str {
        "🔣 Input Filter"
    }

    fn show(&mut self, ctx: &Context, open: &mut bool) {
        Window::new(self.name())
            .open(open)
            .vscroll(true)
            .hscroll(true)
            .show(ctx, |ui| self.ui(ui));
    }
}

impl View for InputFilterWindow {
    fn ui(&mut self, ui: &mut Ui) {
        ui.set_min_width(250.0);

        ui.label("Use a `TextEdit` filter to do the input validation for you!");

        ui.horizontal(|ui| {
            ui.label("u16:");
            TextEdit::singleline(&mut self.u16_buffer)
                .filter(&mut u16::filter_input)
                .hint_text("a valid `u16`")
                .show(ui);
        });

        ui.horizontal(|ui| {
            ui.label("i64:");
            TextEdit::singleline(&mut self.i64_buffer)
                .filter(&mut i64::filter_input)
                .hint_text("a valid `i64`")
                .show(ui);
        });

        ui.horizontal(|ui| {
            ui.label("custom filter:");
            TextEdit::singleline(&mut self.ascii_buffer)
                .filter(&mut filter_ascii_string)
                .hint_text("can only be filled at most 16 ASCII characters")
                .show(ui);
        });

        if !self.ascii_buffer.is_empty() {
            let ascii: AsciiString = self.ascii_buffer.parse().unwrap();
            let _ = ascii.as_str();
        }
    }
}
