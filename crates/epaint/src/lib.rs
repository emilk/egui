//! A simple 2D graphics library for turning simple 2D shapes and text into textured triangles.
//!
//! Made for [`egui`](https://github.com/emilk/egui/).
//!
//! Create some [`Shape`]:s and pass them to [`tessellate_shapes`] to generate [`Mesh`]:es
//! that you can then paint using some graphics API of your choice (e.g. OpenGL).
//!
//! ## Feature flags
#![cfg_attr(feature = "document-features", doc = document_features::document_features!())]
//!

#![allow(clippy::float_cmp)]
#![allow(clippy::manual_range_contains)]

mod bezier;
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
    image::{ColorImage, FontImage, ImageData, ImageDelta},
    mesh::{Mesh, Mesh16, Vertex},
    shadow::Shadow,
    shape::{
        CircleShape, PaintCallback, PaintCallbackInfo, PathShape, RectShape, Rounding, Shape,
        TextShape,
    },
    stats::PaintStats,
    stroke::Stroke,
    tessellator::{tessellate_shapes, TessellationOptions, Tessellator},
    text::{FontFamily, FontId, Fonts, Galley},
    texture_atlas::TextureAtlas,
    texture_handle::TextureHandle,
    textures::TextureManager,
};

pub use ecolor::{Color32, Hsva, HsvaGamma, Rgba};
pub use emath::{pos2, vec2, Pos2, Rect, Vec2};

pub use ahash;
pub use ecolor;
pub use emath;

#[cfg(feature = "color-hex")]
pub use ecolor::hex_color;

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
