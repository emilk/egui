//! A simple 2D graphics library for turning simple 2D shapes and text into textured triangles.
//!
//! Made for [`egui`](https://github.com/emilk/egui/).
//!
//! Create some [`Shape`]:s and pass them to [`Tessellator::tessellate_shapes`] to generate [`Mesh`]:es
//! that you can then paint using some graphics API of your choice (e.g. OpenGL).
//!
//! ## Coordinate system
//! The left-top corner of the screen is `(0.0, 0.0)`,
//! with X increasing to the right and Y increasing downwards.
//!
//! `epaint` uses logical _points_ as its coordinate system.
//! Those related to physical _pixels_ by the `pixels_per_point` scale factor.
//! For example, a high-dpi screen can have `pixels_per_point = 2.0`,
//! meaning there are two physical screen pixels for each logical point.
//!
//! Angles are in radians, and are measured clockwise from the X-axis, which has angle=0.
//!
//! ## Feature flags
#![cfg_attr(feature = "document-features", doc = document_features::document_features!())]
//!

#![expect(clippy::float_cmp)]
#![expect(clippy::manual_range_contains)]

mod brush;
pub mod color;
mod corner_radius;
mod corner_radius_f32;
pub mod image;
mod margin;
mod margin_f32;
mod mesh;
pub mod mutex;
mod shadow;
pub mod shape_transform;
mod shapes;
pub mod stats;
mod stroke;
pub mod tessellator;
pub mod text;
mod texture_atlas;
mod texture_handle;
pub mod textures;
pub mod util;
mod viewport;

pub use self::{
    brush::Brush,
    color::ColorMode,
    corner_radius::CornerRadius,
    corner_radius_f32::CornerRadiusF32,
    image::{AlphaFromCoverage, ColorImage, ImageData, ImageDelta},
    margin::Margin,
    margin_f32::*,
    mesh::{Mesh, Mesh16, Vertex},
    shadow::Shadow,
    shapes::{
        CircleShape, CubicBezierShape, EllipseShape, PaintCallback, PaintCallbackInfo, PathShape,
        QuadraticBezierShape, RectShape, Shape, TextShape,
    },
    stats::PaintStats,
    stroke::{PathStroke, Stroke, StrokeKind},
    tessellator::{TessellationOptions, Tessellator},
    text::{FontFamily, FontId, Fonts, FontsView, Galley, TextOptions},
    texture_atlas::TextureAtlas,
    texture_handle::TextureHandle,
    textures::TextureManager,
    viewport::ViewportInPixels,
};

#[deprecated = "Renamed to CornerRadius"]
pub type Rounding = CornerRadius;

pub use ecolor::{Color32, Hsva, HsvaGamma, Rgba};
pub use emath::{Pos2, Rect, Vec2, pos2, vec2};

#[deprecated = "Use the ahash crate directly."]
pub use ahash;

pub use ecolor;
pub use emath;

#[cfg(feature = "color-hex")]
pub use ecolor::hex_color;

/// The UV coordinate of a white region of the texture mesh.
///
/// The default egui texture has the top-left corner pixel fully white.
/// You need need use a clamping texture sampler for this to work
/// (so it doesn't do bilinear blending with bottom right corner).
pub const WHITE_UV: emath::Pos2 = emath::pos2(0.0, 0.0);

/// What texture to use in a [`Mesh`] mesh.
///
/// If you don't want to use a texture, use `TextureId::Managed(0)` and the [`WHITE_UV`] for uv-coord.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum TextureId {
    /// Textures allocated using [`TextureManager`].
    ///
    /// The first texture (`TextureId::Managed(0)`) is used for the font data.
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
pub struct ClippedShape {
    /// Clip / scissor rectangle.
    /// Only show the part of the [`Shape`] that falls within this.
    pub clip_rect: emath::Rect,

    /// The shape
    pub shape: Shape,
}

impl ClippedShape {
    /// Transform (move/scale) the shape in-place.
    ///
    /// If using a [`PaintCallback`], note that only the rect is scaled as opposed
    /// to other shapes where the stroke is also scaled.
    pub fn transform(&mut self, transform: emath::TSTransform) {
        let Self { clip_rect, shape } = self;
        *clip_rect = transform * *clip_rect;
        shape.transform(transform);
    }
}

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

// ---------------------------------------------------------------------------

/// Was epaint compiled with the `rayon` feature?
pub const HAS_RAYON: bool = cfg!(feature = "rayon");
