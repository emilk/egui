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

pub mod color;
pub mod containers;
mod context;
mod emigui;
pub mod example_app;
mod font;
mod fonts;
mod id;
mod input;
mod layers;
mod layout;
pub mod math;
mod memory;
pub mod mesher;
mod movement_tracker;
mod region;
mod style;
mod texture_atlas;
mod types;
pub mod widgets;

pub use {
    crate::emigui::Emigui,
    color::Color,
    context::Context,
    fonts::{FontDefinitions, Fonts, TextStyle},
    id::Id,
    input::*,
    layers::*,
    layout::{Align, GuiResponse},
    math::*,
    memory::Memory,
    mesher::{Mesh, PaintBatches, Vertex},
    movement_tracker::MovementTracker,
    region::Region,
    style::Style,
    texture_atlas::Texture,
    types::*,
    widgets::Widget,
};
