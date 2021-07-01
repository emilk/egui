//! 2D graphics/rendering. Fonts, textures, color, geometry, tessellation etc.

// Forbid warnings in release builds:
#![cfg_attr(not(debug_assertions), deny(warnings))]
// Disabled so we can support rust 1.51:
// #![deny(
//     rustdoc::broken_intra_doc_links,
//     rustdoc::invalid_codeblock_attributes,
//     rustdoc::missing_crate_level_docs,
//     rustdoc::private_intra_doc_links
// )]
#![forbid(unsafe_code)]
#![warn(
    clippy::all,
    clippy::await_holding_lock,
    clippy::char_lit_as_u8,
    clippy::checked_conversions,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::doc_markdown,
    clippy::empty_enum,
    clippy::enum_glob_use,
    clippy::exit,
    clippy::expl_impl_clone_on_copy,
    clippy::explicit_deref_methods,
    clippy::explicit_into_iter_loop,
    clippy::fallible_impl_from,
    clippy::filter_map_next,
    clippy::float_cmp_const,
    clippy::fn_params_excessive_bools,
    clippy::if_let_mutex,
    clippy::imprecise_flops,
    clippy::inefficient_to_string,
    clippy::invalid_upcast_comparisons,
    clippy::large_types_passed_by_value,
    clippy::let_unit_value,
    clippy::linkedlist,
    clippy::lossy_float_literal,
    clippy::macro_use_imports,
    clippy::manual_ok_or,
    clippy::map_err_ignore,
    clippy::map_flatten,
    clippy::match_on_vec_items,
    clippy::match_same_arms,
    clippy::match_wildcard_for_single_variants,
    clippy::mem_forget,
    clippy::mismatched_target_os,
    clippy::missing_errors_doc,
    clippy::missing_safety_doc,
    clippy::mut_mut,
    clippy::mutex_integer,
    clippy::needless_borrow,
    clippy::needless_continue,
    clippy::needless_pass_by_value,
    clippy::option_option,
    clippy::path_buf_push_overwrite,
    clippy::ptr_as_ptr,
    clippy::pub_enum_variant_names,
    clippy::ref_option_ref,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_functions_in_if_condition,
    clippy::string_add_assign,
    clippy::string_add,
    clippy::string_lit_as_bytes,
    clippy::string_to_string,
    clippy::todo,
    clippy::trait_duplication_in_bounds,
    clippy::unimplemented,
    clippy::unnested_or_patterns,
    clippy::unused_self,
    clippy::useless_transmute,
    clippy::verbose_file_reads,
    clippy::wrong_pub_self_convention,
    clippy::zero_sized_map_values,
    future_incompatible,
    nonstandard_style,
    rust_2018_idioms
)]
#![allow(clippy::float_cmp)]
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

// ----------------------------------------------------------------------------

/// An assert that is only active when `egui` is compiled with the `egui_assert` feature
/// or with the `debug_egui_assert` feature in debug builds.
#[macro_export]
macro_rules! epaint_assert {
    ($($arg:tt)*) => {
        if cfg!(any(
            feature = "extra_asserts",
            all(feature = "extra_debug_asserts", debug_assertions),
        )) {
            assert!($($arg)*);
        }
    }
}
