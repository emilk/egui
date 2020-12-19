//! Egui core library
//!
//! To get started with Egui, you can use one of the available integrations
//! such as [`egui_web`](https://crates.io/crates/egui_web) or  [`egui_glium`](https://crates.io/crates/egui_glium).
//!
//! Whatever you use, you need an `egui::Context` (by convention referred to by `ctx`).
//! With it you can then get access to an `Ui` where you can put widgets.
//! Use one of `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`. For instace:
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
//!     let paint_jobs = self.ctx.tesselate(paint_commands); // create triangles to paint
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
    nonstandard_style,
    rust_2018_idioms
)]

pub mod align;
mod animation_manager;
pub mod app;
pub mod containers;
mod context;
pub mod demos;
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
    demos::DemoApp,
    id::Id,
    input::*,
    layers::*,
    layout::*,
    math::*,
    memory::Memory,
    paint::{
        color, FontDefinitions, FontFamily, PaintCmd, PaintJobs, Rgba, Srgba, Stroke, TextStyle,
        Texture, TextureId,
    },
    painter::Painter,
    style::Style,
    types::*,
    ui::Ui,
    util::mutex,
    widgets::*,
};

#[cfg(debug_assertions)]
pub(crate) fn has_debug_assertions() -> bool {
    true
}

#[cfg(not(debug_assertions))]
pub(crate) fn has_debug_assertions() -> bool {
    false
}

#[test]
fn test_egui_e2e() {
    let mut demo_windows = crate::demos::DemoWindows::default();
    let mut ctx = crate::CtxRef::default();
    let raw_input = crate::RawInput::default();

    const NUM_FRAMES: usize = 5;
    for _ in 0..NUM_FRAMES {
        ctx.begin_frame(raw_input.clone());
        demo_windows.ui(&ctx, &Default::default(), &mut None, |_ui| {});
        let (_output, paint_commands) = ctx.end_frame();
        let paint_jobs = ctx.tesselate(paint_commands);
        assert!(!paint_jobs.is_empty());
    }
}
