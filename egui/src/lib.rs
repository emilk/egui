//! `egui`:  an easy-to-use GUI in pure Rust!
//!
//! Try the live web demo: <https://emilk.github.io/egui/index.html>. Read more about egui at <https://github.com/emilk/egui>.
//!
//! `egui` is in heavy development, with each new version having breaking changes.
//! You need to have the latest stable version of `rustc` to use `egui`.
//!
//! To quickly get started with egui, you can take a look at [`egui_template`](https://github.com/emilk/egui_template)
//! which uses [`eframe`](https://docs.rs/eframe).
//!
//! To create a GUI using egui you first need a [`CtxRef`] (by convention referred to by `ctx`).
//! Then you add a [`Window`] or a [`SidePanel`] to get a [`Ui`], which is what you'll be using to add all the buttons and labels that you need.
//!
//!
//! # Using egui
//!
//! To see what is possible to build with egui you can check out the online demo at <https://emilk.github.io/egui/#demo>.
//!
//! If you like the "learning by doing" approach, clone <https://github.com/emilk/egui_template> and get started using egui right away.
//!
//! ### A simple example
//!
//! Here is a simple counter that can be incremented and decremented using two buttons:
//! ```
//! fn ui_counter(ui: &mut egui::Ui, counter: &mut i32) {
//!     // Put the buttons and label on the same row:
//!     ui.horizontal(|ui| {
//!         if ui.button("-").clicked() {
//!             *counter -= 1;
//!         }
//!         ui.label(counter.to_string());
//!         if ui.button("+").clicked() {
//!             *counter += 1;
//!         }
//!     });
//! }
//! ```
//!
//! In some GUI frameworks this would require defining multiple types and functions with callbacks or message handlers,
//! but thanks to `egui` being immediate mode everything is one self-contained function!
//!
//! ### Getting a [`Ui`]
//!
//! Use one of [`SidePanel`], [`TopPanel`], [`CentralPanel`], [`Window`] or [`Area`] to
//! get access to an [`Ui`] where you can put widgets. For example:
//!
//! ```
//! # let mut ctx = egui::CtxRef::default();
//! # ctx.begin_frame(Default::default());
//! egui::CentralPanel::default().show(&ctx, |ui| {
//!     ui.add(egui::Label::new("Hello World!"));
//!     ui.label("A shorter and more convenient way to add a label.");
//!     if ui.button("Click me").clicked() {
//!         /* take some action here */
//!     }
//! });
//! ```
//!
//! ### Quick start
//!
//! ``` rust
//! # let ui = &mut egui::Ui::__test();
//! # let mut my_string = String::new();
//! # let mut my_boolean = true;
//! # let mut my_f32 = 42.0;
//! ui.label("This is a label");
//! ui.hyperlink("https://github.com/emilk/egui");
//! ui.text_edit_singleline(&mut my_string);
//! if ui.button("Click me").clicked() { }
//! ui.add(egui::Slider::f32(&mut my_f32, 0.0..=100.0));
//! ui.add(egui::DragValue::f32(&mut my_f32));
//!
//! ui.checkbox(&mut my_boolean, "Checkbox");
//!
//! #[derive(PartialEq)]
//! enum Enum { First, Second, Third }
//! let mut my_enum = Enum::First;
//! ui.horizontal(|ui| {
//!     ui.radio_value(&mut my_enum, Enum::First, "First");
//!     ui.radio_value(&mut my_enum, Enum::Second, "Second");
//!     ui.radio_value(&mut my_enum, Enum::Third, "Third");
//! });
//!
//! ui.separator();
//!
//! # let my_image = egui::TextureId::default();
//! ui.image(my_image, [640.0, 480.0]);
//!
//! ui.collapsing("Click to see what is hidden!", |ui| {
//!     ui.label("Not much, as it turns out");
//! });
//! ```
//!
//! ## Conventions
//!
//! Conventions unless otherwise specified:
//!
//! * angles are in radians
//! * `Vec2::X` is right and `Vec2::Y` is down.
//! * `Pos2::ZERO` is left top.
//! * Positions and sizes are measured in _points_. Each point may consist of many physical pixels.
//!
//! # Integrating with egui
//!
//! Most likely you are using an existing `egui` backend/integration such as [`eframe`](https://docs.rs/eframe) or [`bevy_egui`](https://docs.rs/bevy_egui),
//! but if you want to integrate `egui` into a new game engine, this is the section for you.
//!
//! To write your own integration for egui you need to do this:
//!
//! ``` no_run
//! # fn handle_output(_: egui::Output) {}
//! # fn paint(_: Vec<egui::ClippedMesh>) {}
//! # fn gather_input() -> egui::RawInput { egui::RawInput::default() }
//! let mut ctx = egui::CtxRef::default();
//!
//! // Game loop:
//! loop {
//!     let raw_input: egui::RawInput = gather_input();
//!     ctx.begin_frame(raw_input);
//!
//!     egui::CentralPanel::default().show(&ctx, |ui| {
//!         ui.label("Hello world!");
//!         if ui.button("Click me").clicked() {
//!             /* take some action here */
//!         }
//!     });
//!
//!     let (output, shapes) = ctx.end_frame();
//!     let clipped_meshes = ctx.tessellate(shapes); // create triangles to paint
//!     handle_output(output);
//!     paint(clipped_meshes);
//! }
//! ```
//!
//!
//! # Understanding immediate mode
//!
//! `egui` is an immediate mode GUI library. It is useful to fully grok what "immediate mode" implies.
//!
//! Here is an example to illustrate it:
//!
//! ```
//! # let ui = &mut egui::Ui::__test();
//! if ui.button("click me").clicked() {
//!     take_action()
//! }
//! # fn take_action() {}
//! ```
//!
//! This code is being executed each frame at maybe 60 frames per second.
//! Each frame egui does these things:
//!
//! * lays out the letters `click me` in order to figure out the size of the button
//! * decides where on screen to place the button
//! * check if the mouse is hovering or clicking that location
//! * chose button colors based on if it is being hovered or clicked
//! * add a [`Shape::Rect`] and [`Shape::Text`] to the list of shapes to be painted later this frame
//! * return a [`Response`] with the `clicked` member so the user can check for interactions
//!
//! There is no button being created and stored somewhere.
//! The only output of this call is some colored shapes, and a [`Response`].
//!
//! Read more about the pros and cons of immediate mode at <https://github.com/emilk/egui#why-immediate-mode>.
//!
//! ## How widgets works
//!
//! ```
//! # let ui = &mut egui::Ui::__test();
//! if ui.button("click me").clicked() { take_action() }
//! # fn take_action() {}
//! ```
//!
//! is short for
//!
//! ```
//! # let ui = &mut egui::Ui::__test();
//! let button = egui::Button::new("click me");
//! if ui.add(button).clicked() { take_action() }
//! # fn take_action() {}
//! ```
//!
//! which is short for
//!
//! ```
//! # use egui::Widget;
//! # let ui = &mut egui::Ui::__test();
//! let button = egui::Button::new("click me");
//! let response = button.ui(ui);
//! if response.clicked() { take_action() }
//! # fn take_action() {}
//! ```
//!
//! [`Button`] uses the builder pattern to create the data required to show it. The [`Button`] is then discarded.
//!
//! [`Button`] implements `trait` [`Widget`], which looks like this:
//! ```
//! # use egui::*;
//! pub trait Widget {
//!     /// Allocate space, interact, paint, and return a [`Response`].
//!     fn ui(self, ui: &mut Ui) -> Response;
//! }
//! ```
//!
//! ## Code snippets
//!
//! ```
//! # let ui = &mut egui::Ui::__test();
//! # let mut some_bool = true;
//! // Miscellaneous tips and tricks
//!
//! ui.horizontal_wrapped(|ui|{
//!     ui.spacing_mut().item_spacing.x = 0.0; // remove spacing between widgets
//!     // `radio_value` also works for enums, integers, and more.
//!     ui.radio_value(&mut some_bool, false, "Off");
//!     ui.radio_value(&mut some_bool, true, "On");
//! });
//!
//! ui.group(|ui|{
//!     ui.label("Within a frame");
//!     ui.set_min_height(200.0);
//! });
//!
//! // Change test color on subsequent widgets:
//! ui.visuals_mut().override_text_color = Some(egui::Color32::RED);
//!
//! // Turn off text wrapping on subsequent widgets:
//! ui.style_mut().wrap = Some(false);
//! ```

#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![forbid(unsafe_code)]
#![warn(
    clippy::all,
    clippy::await_holding_lock,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::doc_markdown,
    clippy::empty_enum,
    clippy::enum_glob_use,
    clippy::exit,
    clippy::explicit_into_iter_loop,
    clippy::filter_map_next,
    clippy::fn_params_excessive_bools,
    clippy::if_let_mutex,
    clippy::imprecise_flops,
    clippy::inefficient_to_string,
    clippy::large_types_passed_by_value,
    clippy::let_unit_value,
    clippy::linkedlist,
    clippy::lossy_float_literal,
    clippy::macro_use_imports,
    clippy::map_err_ignore,
    clippy::map_flatten,
    clippy::match_on_vec_items,
    clippy::match_same_arms,
    clippy::match_wildcard_for_single_variants,
    clippy::mem_forget,
    clippy::mismatched_target_os,
    clippy::missing_errors_doc,
    clippy::missing_safety_doc,
    clippy::needless_borrow,
    clippy::needless_continue,
    clippy::needless_pass_by_value,
    clippy::option_option,
    clippy::pub_enum_variant_names,
    clippy::ref_option_ref,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::string_add_assign,
    clippy::string_add,
    clippy::string_to_string,
    clippy::todo,
    clippy::unimplemented,
    clippy::unnested_or_patterns,
    clippy::unused_self,
    clippy::verbose_file_reads,
    future_incompatible,
    missing_crate_level_docs,
    nonstandard_style,
    rust_2018_idioms
)]
#![allow(clippy::manual_range_contains)]

mod animation_manager;
pub mod containers;
mod context;
mod data;
pub mod experimental;
mod frame_state;
pub(crate) mod grid;
mod id;
mod input_state;
mod introspection;
pub mod layers;
mod layout;
mod memory;
pub mod menu;
mod painter;
pub(crate) mod placer;
mod response;
mod sense;
pub mod style;
mod ui;
pub mod util;
pub mod widgets;

pub use epaint;
pub use epaint::emath;

// Can't add deprecation notice due to https://github.com/rust-lang/rust/issues/30827
pub use epaint as paint; // historical reasons

// Can't add deprecation notice due to https://github.com/rust-lang/rust/issues/30827
pub use emath as math; // historical reasons

pub use emath::{lerp, pos2, remap, remap_clamp, vec2, Align, Align2, NumExt, Pos2, Rect, Vec2};
pub use epaint::{
    color, mutex,
    text::{FontDefinitions, FontFamily, TextStyle},
    ClippedMesh, Color32, Rgba, Shape, Stroke, Texture, TextureId,
};

pub use {
    containers::*,
    context::{Context, CtxRef},
    data::{
        input::*,
        output::{self, CursorIcon, Output, WidgetInfo, WidgetType},
    },
    grid::Grid,
    id::Id,
    input_state::{InputState, PointerState},
    layers::{LayerId, Order},
    layout::*,
    memory::Memory,
    painter::Painter,
    response::{InnerResponse, Response},
    sense::Sense,
    style::{Style, Visuals},
    ui::Ui,
    widgets::*,
};

// ----------------------------------------------------------------------------

/// `true` if egui was compiled with debug assertions enabled.
#[cfg(debug_assertions)]
pub(crate) const fn has_debug_assertions() -> bool {
    true
}

/// `true` if egui was compiled with debug assertions enabled.
#[cfg(not(debug_assertions))]
pub(crate) const fn has_debug_assertions() -> bool {
    false
}

/// Helper function that adds a label when compiling with debug assertions enabled.
pub fn warn_if_debug_build(ui: &mut crate::Ui) {
    if crate::has_debug_assertions() {
        ui.label(
            crate::Label::new("‼ Debug build ‼")
                .small()
                .text_color(crate::Color32::RED),
        )
        .on_hover_text("egui was compiled with debug assertions enabled.");
    }
}

// ----------------------------------------------------------------------------

/// Create a [`Hyperlink`](crate::Hyperlink) to the current [`file!()`] (and line) on Github
///
/// Example: `ui.add(github_link_file_line!("https://github.com/YOUR/PROJECT/blob/master/", "(source code)"));`
#[macro_export]
macro_rules! github_link_file_line {
    ($github_url:expr, $label:expr) => {{
        let url = format!("{}{}#L{}", $github_url, file!(), line!());
        $crate::Hyperlink::new(url).text($label)
    }};
}

/// Create a [`Hyperlink`](crate::Hyperlink) to the current [`file!()`] on github.
///
/// Example: `ui.add(github_link_file!("https://github.com/YOUR/PROJECT/blob/master/", "(source code)"));`
#[macro_export]
macro_rules! github_link_file {
    ($github_url:expr, $label:expr) => {{
        let url = format!("{}{}", $github_url, file!());
        $crate::Hyperlink::new(url).text($label)
    }};
}

// ----------------------------------------------------------------------------

/// egui supports around 1216 emojis in total.
/// Here are some of the most useful:
/// ∞⊗⎗⎘⎙⏏⏴⏵⏶⏷
/// ⏩⏪⏭⏮⏸⏹⏺■▶📾🔀🔁🔃
/// ☀☁★☆☐☑☜☝☞☟⛃⛶✔
/// ↺↻⟲⟳⬅➡⬆⬇⬈⬉⬊⬋⬌⬍⮨⮩⮪⮫
/// ♡
/// 📅📆
/// 📈📉📊
/// 📋📌📎📤📥🔆
/// 🔈🔉🔊🔍🔎🔗🔘
/// 🕓🖧🖩🖮🖱🖴🖵🖼🗀🗁🗋🗐🗑🗙🚫❓
///
/// NOTE: In egui all emojis are monochrome!
///
/// You can explore them all in the Font Book in [the online demo](https://emilk.github.io/egui/).
///
/// In addition, egui supports a few special emojis that are not part of the unicode standard.
/// This module contains some of them:
pub mod special_emojis {
    /// Tux, the Linux penguin.
    pub const OS_LINUX: char = '🐧';
    /// The Windows logo.
    pub const OS_WINDOWS: char = '';
    /// The Android logo.
    pub const OS_ANDROID: char = '';
    /// The Apple logo.
    pub const OS_APPLE: char = '';

    /// The Github logo.
    pub const GITHUB: char = '';

    /// The word `git`.
    pub const GIT: char = '';

    // I really would like to have ferris here.
}
