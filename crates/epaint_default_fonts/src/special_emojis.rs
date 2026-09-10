//! A few special emojis that are not part of the unicode standard.
//!
//! Except for [`OS_LINUX`] they live in the private use area, so no platform
//! font has them, which is why they are bundled in [`crate::EGUI_ICONS`].
//!
//! Besides these, egui renders whatever emoji the platform fonts have.
//! With the `monochrome_emoji_fonts` feature you also get the bundled
//! [`crate::NOTO_EMOJI_REGULAR`] and [`crate::EMOJI_ICON`], which add
//! monochrome emoji and icons like:
//!
//! ```text
//! ∞⊗⎗⎘⎙⏏⏴⏵⏶⏷
//! ⏩⏪⏭⏮⏸⏹⏺■▶📾🔀🔁🔃
//! ☀☁★☆☐☑☜☝☞☟⛃⛶✔
//! ↺↻⟲⟳⬅➡⬆⬇⬈⬉⬊⬋⬌⬍⮨⮩⮪⮫
//! ♡
//! 📅📆
//! 📈📉📊
//! 📋📌📎📤📥🔆
//! 🔈🔉🔊🔍🔎🔗🔘
//! 🕓🖧🖩🖮🖱🖴🖵🖼🗀🗁🗋🗐🗑🗙🚫❓
//! ```
//!
//! You can explore all the emoji of the current fonts in the Font Book in
//! [the online demo](https://www.egui.rs/#demo).

/// Tux, the Linux penguin.
///
/// A normal emoji, covered by most emoji fonts.
pub const OS_LINUX: char = '🐧';

/// The Windows logo.
pub const OS_WINDOWS: char = '\u{E61F}';

/// The Android logo.
pub const OS_ANDROID: char = '\u{E618}';

/// The Apple logo.
pub const OS_APPLE: char = '\u{F8FF}';

/// The Github logo.
pub const GITHUB: char = '\u{E624}';

/// The word `git`.
pub const GIT: char = '\u{E625}';

// I really would like to have ferris here.
