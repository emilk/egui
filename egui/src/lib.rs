//! Egui core library
//!
//! To quickly get started with Egui, you can take a look at [`egui_template`](https://github.com/emilk/egui_template)
//! which uses [`eframe`](https://docs.rs/eframe).
//!
//! To create a GUI using Egui you first need a [`CtxRef`] (by convention referred to by `ctx`).
//! Use one of [`SidePanel`], [`TopPanel`], [`CentralPanel`], [`Window`] or [`Area`] to
//! get access to an [`Ui`] where you can put widgets. For example:
//!
//! ```
//! # let mut ctx = egui::CtxRef::default();
//! # ctx.begin_frame(Default::default());
//! egui::CentralPanel::default().show(&ctx, |ui| {
//!     ui.label("Hello");
//! });
//! ```
//!
//!
//! To write your own integration for Egui you need to do this:
//!
//! ``` ignore
//! let mut egui_ctx = egui::CtxRef::default();
//!
//! // Game loop:
//! loop {
//!     let raw_input: egui::RawInput = my_integration.gather_input();
//!     egui_ctx.begin_frame(raw_input);
//!     my_app.ui(&egui_ctx); // add panels, windows and widgets to `egui_ctx` here
//!     let (output, paint_commands) = egui_ctx.end_frame();
//!     let paint_jobs = egui_ctx.tessellate(paint_commands); // create triangles to paint
//!     my_integration.paint(paint_jobs);
//!     my_integration.set_cursor_icon(output.cursor_icon);
//!     // Also see `egui::Output` for more
//! }
//! ```

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

pub mod align;
mod animation_manager;
pub mod containers;
mod context;
mod id;
mod input;
mod introspection;
mod layers;
mod layout;
pub mod math;
mod memory;
pub mod menu;
pub mod paint;
mod painter;
mod style;
mod types;
mod ui;
pub mod util;
pub mod widgets;

pub use {
    align::Align,
    containers::*,
    context::{Context, CtxRef},
    id::Id,
    input::*,
    layers::*,
    layout::*,
    math::*,
    memory::Memory,
    paint::{
        color, Color32, FontDefinitions, FontFamily, PaintCmd, PaintJobs, Rgba, Stroke, TextStyle,
        Texture, TextureId,
    },
    painter::Painter,
    style::Style,
    types::*,
    ui::Ui,
    util::mutex,
    widgets::*,
};

// ----------------------------------------------------------------------------

#[cfg(debug_assertions)]
pub(crate) const fn has_debug_assertions() -> bool {
    true
}

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
        .on_hover_text("Egui was compiled with debug assertions enabled.");
    }
}

// ----------------------------------------------------------------------------

/// Create a [`Hyperlink`](crate::Hyperlink) to this file (and line) on Github
///
/// Example: `ui.add(github_link_file_line!("https://github.com/YOUR/PROJECT/blob/master/", "(source code)"));`
#[macro_export]
macro_rules! github_link_file_line {
    ($github_url:expr, $label:expr) => {{
        let url = format!("{}{}#L{}", $github_url, file!(), line!());
        $crate::Hyperlink::new(url).text($label)
    }};
}

/// Create a [`Hyperlink`](crate::Hyperlink) to this file on github.
///
/// Example: `ui.add(github_link_file!("https://github.com/YOUR/PROJECT/blob/master/", "(source code)"));`
#[macro_export]
macro_rules! github_link_file {
    ($github_url:expr, $label:expr) => {{
        let url = format!("{}{}", $github_url, file!());
        $crate::Hyperlink::new(url).text($label)
    }};
}
