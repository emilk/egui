//! 2D graphics/rendering. Fonts, textures, color, geometry, tessellation etc.

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

pub mod color;
mod mesh;
pub mod mutex;
mod shadow;
mod shape;
pub mod shape_transform;
pub mod stats;
mod stroke;
pub mod tessellator;
pub mod text;
mod texture_atlas;

pub use {
    color::{Color32, Rgba},
    mesh::{Mesh, Mesh16, Vertex},
    shadow::Shadow,
    shape::Shape,
    stats::PaintStats,
    stroke::Stroke,
    tessellator::{TessellationOptions, Tessellator},
    text::{Galley, TextStyle},
    texture_atlas::{Texture, TextureAtlas},
};

pub use ahash;
pub use emath;

/// The UV coordinate of a white region of the texture mesh.
/// The default egui texture has the top-left corner pixel fully white.
/// You need need use a clamping texture sampler for this to work
/// (so it doesn't do bilinear blending with bottom right corner).
pub const WHITE_UV: emath::Pos2 = emath::pos2(0.0, 0.0);

/// What texture to use in a [`Mesh`] mesh.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextureId {
    /// The egui font texture.
    /// If you don't want to use a texture, pick this and the [`WHITE_UV`] for uv-coord.
    Egui,

    /// Your own texture, defined in any which way you want.
    /// egui won't care. The backend renderer will presumably use this to look up what texture to use.
    User(u64),
}

impl Default for TextureId {
    fn default() -> Self {
        Self::Egui
    }
}

pub(crate) struct PaintRect {
    pub rect: emath::Rect,
    /// How rounded the corners are. Use `0.0` for no rounding.
    pub corner_radius: f32,
    pub fill: Color32,
    pub stroke: Stroke,
}

/// A [`Shape`] within a clip rectangle.
///
/// Everything is using logical points.
#[derive(Clone, Debug)]
pub struct ClippedShape(
    /// Clip / scissor rectangle.
    /// Only show the part of the [`Shape`] that falls within this.
    pub emath::Rect,
    /// The shape
    pub Shape,
);

/// A [`Mesh`] within a clip rectangle.
///
/// Everything is using logical points.
#[derive(Clone, Debug)]
pub struct ClippedMesh(
    /// Clip / scissor rectangle.
    /// Only show the part of the [`Mesh`] that falls within this.
    pub emath::Rect,
    /// The shape
    pub Mesh,
);
