//! A simple 2D graphics library for turning simple 2D shapes and text into textured triangles.
//!
//! Made for [`egui`](https://github.com/emilk/egui/).
//!
//! Create some [`Shape`]:s and pass them to [`tessellate_shapes`] to generate [`Mesh`]:es
//! that you can then paint using some graphics API of your choice (e.g. OpenGL).

// Forbid warnings in release builds:
#![cfg_attr(not(debug_assertions), deny(warnings))]
#![forbid(unsafe_code)]
#![warn(
    clippy::all,
    clippy::await_holding_lock,
    clippy::char_lit_as_u8,
    clippy::checked_conversions,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::disallowed_method,
    clippy::doc_markdown,
    clippy::empty_enum,
    clippy::enum_glob_use,
    clippy::exit,
    clippy::expl_impl_clone_on_copy,
    clippy::explicit_deref_methods,
    clippy::explicit_into_iter_loop,
    clippy::fallible_impl_from,
    clippy::filter_map_next,
    clippy::flat_map_option,
    clippy::float_cmp_const,
    clippy::fn_params_excessive_bools,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::implicit_clone,
    clippy::imprecise_flops,
    clippy::inefficient_to_string,
    clippy::invalid_upcast_comparisons,
    clippy::large_digit_groups,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::let_unit_value,
    clippy::linkedlist,
    clippy::lossy_float_literal,
    clippy::macro_use_imports,
    clippy::manual_ok_or,
    clippy::map_err_ignore,
    clippy::map_flatten,
    clippy::map_unwrap_or,
    clippy::match_on_vec_items,
    clippy::match_same_arms,
    clippy::match_wild_err_arm,
    clippy::match_wildcard_for_single_variants,
    clippy::mem_forget,
    clippy::mismatched_target_os,
    clippy::missing_errors_doc,
    clippy::missing_safety_doc,
    clippy::mut_mut,
    clippy::mutex_integer,
    clippy::needless_borrow,
    clippy::needless_continue,
    clippy::needless_for_each,
    clippy::needless_pass_by_value,
    clippy::option_option,
    clippy::path_buf_push_overwrite,
    clippy::ptr_as_ptr,
    clippy::ref_option_ref,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_functions_in_if_condition,
    clippy::semicolon_if_nothing_returned,
    clippy::single_match_else,
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
    clippy::zero_sized_map_values,
    future_incompatible,
    nonstandard_style,
    rust_2018_idioms,
    rustdoc::missing_crate_level_docs
)]
#![allow(clippy::float_cmp)]
#![allow(clippy::manual_range_contains)]

mod bezier;
pub mod color;
pub mod image;
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
mod texture_handle;
pub mod textures;
pub mod util;

pub use {
    bezier::{CubicBezierShape, QuadraticBezierShape},
    color::{Color32, Rgba},
    image::{AlphaImage, ColorImage, ImageData, ImageDelta},
    mesh::{Mesh, Mesh16, Vertex},
    shadow::Shadow,
    shape::{CircleShape, PaintCallback, PathShape, RectShape, Rounding, Shape, TextShape},
    stats::PaintStats,
    stroke::Stroke,
    tessellator::{tessellate_shapes, TessellationOptions, Tessellator},
    text::{FontFamily, FontId, Fonts, Galley},
    texture_atlas::TextureAtlas,
    texture_handle::TextureHandle,
    textures::TextureManager,
};

pub use emath::{pos2, vec2, Pos2, Rect, Vec2};

pub use ahash;
pub use emath;

/// The UV coordinate of a white region of the texture mesh.
/// The default egui texture has the top-left corner pixel fully white.
/// You need need use a clamping texture sampler for this to work
/// (so it doesn't do bilinear blending with bottom right corner).
pub const WHITE_UV: emath::Pos2 = emath::pos2(0.0, 0.0);

/// What texture to use in a [`Mesh`] mesh.
///
/// If you don't want to use a texture, use `TextureId::Epaint(0)` and the [`WHITE_UV`] for uv-coord.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum TextureId {
    /// Textures allocated using [`TextureManager`].
    ///
    /// The first texture (`TextureId::Epaint(0)`) is used for the font data.
    Managed(u64),

    /// Your own texture, defined in any which way you want.
    /// The backend renderer will presumably use this to look up what texture to use.
    User(u64),
}

impl Default for TextureId {
    /// The epaint font texture.
    fn default() -> Self {
        Self::Managed(0)
    }
}

/// A [`Shape`] within a clip rectangle.
///
/// Everything is using logical points.
#[derive(Clone, Debug, PartialEq)]
pub struct ClippedShape(
    /// Clip / scissor rectangle.
    /// Only show the part of the [`Shape`] that falls within this.
    pub emath::Rect,
    /// The shape
    pub Shape,
);

/// A [`Mesh`] or [`PaintCallback`] within a clip rectangle.
///
/// Everything is using logical points.
#[derive(Clone, Debug)]
pub struct ClippedPrimitive {
    /// Clip / scissor rectangle.
    /// Only show the part of the [`Mesh`] that falls within this.
    pub clip_rect: emath::Rect,
    /// What to paint - either a [`Mesh`] or a [`PaintCallback`].
    pub primitive: Primitive,
}

/// A rendering primitive - either a [`Mesh`] or a [`PaintCallback`].
#[derive(Clone, Debug)]
pub enum Primitive {
    Mesh(Mesh),
    Callback(PaintCallback),
}

// ----------------------------------------------------------------------------

/// An assert that is only active when `epaint` is compiled with the `extra_asserts` feature
/// or with the `extra_debug_asserts` feature in debug builds.
#[macro_export]
macro_rules! epaint_assert {
    ($($arg: tt)*) => {
        if cfg!(any(
            feature = "extra_asserts",
            all(feature = "extra_debug_asserts", debug_assertions),
        )) {
            assert!($($arg)*);
        }
    }
}

// ----------------------------------------------------------------------------

#[inline(always)]
pub(crate) fn f32_hash<H: std::hash::Hasher>(state: &mut H, f: f32) {
    if f == 0.0 {
        state.write_u8(0);
    } else if f.is_nan() {
        state.write_u8(1);
    } else {
        use std::hash::Hash;
        f.to_bits().hash(state);
    }
}

#[inline(always)]
pub(crate) fn f64_hash<H: std::hash::Hasher>(state: &mut H, f: f64) {
    if f == 0.0 {
        state.write_u8(0);
    } else if f.is_nan() {
        state.write_u8(1);
    } else {
        use std::hash::Hash;
        f.to_bits().hash(state);
    }
}
