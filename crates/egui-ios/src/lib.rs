//! iOS FFI bindings for egui via swift-bridge
//!
//! This crate provides Swift-compatible types for embedding egui in iOS apps:
//!
//! - [`InputEvent`] - Touch, keyboard, and lifecycle events from Swift to egui
//! - [`OutputState`] - Cursor, keyboard, and IME state from egui to Swift
//! - [`CursorIcon`] - Cursor icons mapped to iOS equivalents
//!
//! ## Usage
//!
//! Add this crate to your iOS Rust library and include the generated Swift bindings:
//!
//! ```rust,ignore
//! use egui_ios::{InputEvent, OutputState};
//!
//! // Convert input events to egui events
//! let egui_events: Vec<egui::Event> = input_events
//!     .into_iter()
//!     .filter_map(|e| e.into_egui_event())
//!     .collect();
//!
//! // After running egui, create output state
//! let output = OutputState::with_keyboard_state(
//!     cursor_icon.into(),
//!     ctx.wants_keyboard_input(),
//!     platform_output.ime.as_ref().map(|ime| ime.rect),
//! );
//! ```
//!
//! ## Swift Integration
//!
//! See the `SWIFTUI_EMBEDDING.md` guide for complete Swift integration examples.

mod ffi;
mod input;
mod output;

pub use input::{InputEvent, ScenePhase};
pub use output::{CursorIcon, OutputState};
