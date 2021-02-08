//! egui core library
//!
//! To quickly get started with egui, you can take a look at [`egui_template`](https://github.com/emilk/egui_template)
//! which uses [`eframe`](https://docs.rs/eframe).
//!
//! To create a GUI using egui you first need a [`CtxRef`] (by convention referred to by `ctx`).
//! Then you add a [`Window`] or a [`SidePanel`] to get a [`Ui`], which is what you'll be using to add all the buttons and labels that you need.
//!
//! ## Integrating with egui
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
//! ## Using egui
//!
//! To see what is possible to build we egui you can check out the online demo at <https://emilk.github.io/egui/#demo>.
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
//!
//!

#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![forbid(unsafe_code)]
#![warn(
    clippy::all,
    clippy::await_holding_lock,
    clippy::dbg_macro,
    clippy::doc_markdown,
    clippy::empty_enum,
    clippy::enum_glob_use,
    clippy::exit,
    clippy::filter_map_next,
    clippy::fn_params_excessive_bools,
    clippy::if_let_mutex,
    clippy::imprecise_flops,
    clippy::inefficient_to_string,
    clippy::linkedlist,
    clippy::lossy_float_literal,
    clippy::macro_use_imports,
    clippy::match_on_vec_items,
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
    clippy::rest_pat_in_fully_bound_structs,
    clippy::todo,
    clippy::unimplemented,
    clippy::unnested_or_patterns,
    clippy::verbose_file_reads,
    future_incompatible,
    missing_crate_level_docs,
    missing_doc_code_examples,
    // missing_docs,
    nonstandard_style,
    rust_2018_idioms,
    unused_doc_comments,
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

pub use emath as math;
pub use epaint as paint;
pub use epaint::emath;

pub use emath::{
    clamp, lerp, pos2, remap, remap_clamp, vec2, Align, Align2, NumExt, Pos2, Rect, Vec2,
};
pub use epaint::{
    color, mutex,
    text::{FontDefinitions, FontFamily, TextStyle},
    ClippedMesh, Color32, Rgba, Shape, Stroke, Texture, TextureId,
};

pub use {
    containers::*,
    context::{Context, CtxRef},
    data::{input::*, output::*},
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
