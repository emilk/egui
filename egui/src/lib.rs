#![deny(warnings)]
#![warn(
    clippy::all,
    clippy::dbg_macro,
    clippy::doc_markdown,
    clippy::empty_enum,
    clippy::enum_glob_use,
    clippy::filter_map_next,
    clippy::fn_params_excessive_bools,
    clippy::imprecise_flops,
    clippy::lossy_float_literal,
    clippy::mem_forget,
    clippy::needless_borrow,
    clippy::needless_continue,
    clippy::pub_enum_variant_names,
    clippy::rest_pat_in_fully_bound_structs,
    // clippy::suboptimal_flops, // TODO
    clippy::todo,
    // clippy::use_self,
    future_incompatible,
    nonstandard_style,
    rust_2018_idioms,
)]

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
mod movement_tracker;
pub mod paint;
mod painter;
mod style;
mod types;
mod ui;
pub mod widgets;

pub use {
    containers::*,
    context::Context,
    demos::DemoApp,
    id::Id,
    input::*,
    layers::*,
    layout::*,
    math::*,
    memory::Memory,
    movement_tracker::MovementTracker,
    paint::{color, Color, PaintJobs, TextStyle, Texture},
    painter::Painter,
    style::Style,
    types::*,
    ui::Ui,
    widgets::*,
};
