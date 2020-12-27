//! Converts graphics primitives into textured triangles.
//!
//! This module converts lines, circles, text and more represented by [`PaintCmd`]
//! into textured triangles represented by [`Triangles`].

#![allow(clippy::identity_op)]

use {
    super::{
        color::{self, srgba, Rgba, Srgba, TRANSPARENT},
        fonts::Fonts,
        PaintCmd, Stroke,
    },
    crate::math::*,
    std::f32::consts::TAU,
};

/// What texture to use in a [`Triangles`] mesh.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextureId {
    /// The Egui font texture.
    /// If you don't want to use a texture, pick this and the [`WHITE_UV`] for uv-coord.
    Egui,

    /// Your own texture, defined in any which way you want.
    /// Egui won't care. The backend renderer will presumably use this to look up what texture to use.
    User(u64),
}

impl Default for TextureId {
    fn default() -> Self {
        Self::Egui
    }
}

/// The UV coordinate of a white region of the texture mesh.
/// The default Egui texture has the top-left corner pixel fully white.
/// You need need use a clamping texture sampler for this to work
/// (so it doesn't do bilinear blending with bottom right corner).
pub const WHITE_UV: Pos2 = pos2(0.0, 0.0);

/// The vertex type.
///
/// Should be friendly to send to GPU as is.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Vertex {
    /// Logical pixel coordinates (points).
    /// (0,0) is the top left corner of the screen.
    pub pos: Pos2, // 64 bit

    /// Normalized texture coordinates.
    /// (0, 0) is the top left corner of the texture.
    /// (1, 1) is the bottom right corner of the texture.
    pub uv: Pos2, // 64 bit

    /// sRGBA with premultiplied alpha
    pub color: Srgba, // 32 bit
}

/// Textured triangles.
#[derive(Clone, Debug, Default)]
pub struct Triangles {
    /// Draw as triangles (i.e. the length is always multiple of three).
    pub indices: Vec<u32>,

    /// The vertex data indexed by `indices`.
    pub vertices: Vec<Vertex>,

    /// The texture to use when drawing these triangles
    pub texture_id: TextureId,
}

/// A clip triangle and some textured triangles.
pub type PaintJob = (Rect, Triangles);

/// Grouped by clip rectangles, in pixel coordinates
pub type PaintJobs = Vec<PaintJob>;

// ----------------------------------------------------------------------------

/// ## Helpers for adding
impl Triangles {
    pub fn with_texture(texture_id: TextureId) -> Self {
        Self {
            texture_id,
            ..Default::default()
        }
    }

    pub fn bytes_used(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.vertices.len() * std::mem::size_of::<Vertex>()
            + self.indices.len() * std::mem::size_of::<u32>()
    }

    /// Are all indices within the bounds of the contained vertices?
    pub fn is_valid(&self) -> bool {
        let n = self.vertices.len() as u32;
        self.indices.iter().all(|&i| i < n)
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty() && self.vertices.is_empty()
    }

    /// Append all the indices and vertices of `other` to `self`.
    pub fn append(&mut self, other: Triangles) {
        debug_assert!(other.is_valid());

        if self.is_empty() {
            *self = other;
        } else {
            assert_eq!(
                self.texture_id, other.texture_id,
                "Can't merge Triangles using different textures"
            );

            let index_offset = self.vertices.len() as u32;
            for index in &other.indices {
                self.indices.push(index_offset + index);
            }
            self.vertices.extend(other.vertices.iter());
        }
    }

    pub fn colored_vertex(&mut self, pos: Pos2, color: Srgba) {
        debug_assert!(self.texture_id == TextureId::Egui);
        self.vertices.push(Vertex {
            pos,
            uv: WHITE_UV,
            color,
        });
    }

    /// Add a triangle.
    pub fn add_triangle(&mut self, a: u32, b: u32, c: u32) {
        self.indices.push(a);
        self.indices.push(b);
        self.indices.push(c);
    }

    /// Make room for this many additional triangles (will reserve 3x as many indices).
    /// See also `reserve_vertices`.
    pub fn reserve_triangles(&mut self, additional_triangles: usize) {
        self.indices.reserve(3 * additional_triangles);
    }

    /// Make room for this many additional vertices.
    /// See also `reserve_triangles`.
    pub fn reserve_vertices(&mut self, additional: usize) {
        self.vertices.reserve(additional);
    }

    /// Rectangle with a texture and color.
    pub fn add_rect_with_uv(&mut self, pos: Rect, uv: Rect, color: Srgba) {
        let idx = self.vertices.len() as u32;
        self.add_triangle(idx + 0, idx + 1, idx + 2);
        self.add_triangle(idx + 2, idx + 1, idx + 3);

        let right_top = Vertex {
            pos: pos.right_top(),
            uv: uv.right_top(),
            color,
        };
        let left_top = Vertex {
            pos: pos.left_top(),
            uv: uv.left_top(),
            color,
        };
        let left_bottom = Vertex {
            pos: pos.left_bottom(),
            uv: uv.left_bottom(),
            color,
        };
        let right_bottom = Vertex {
            pos: pos.right_bottom(),
            uv: uv.right_bottom(),
            color,
        };
        self.vertices.push(left_top);
        self.vertices.push(right_top);
        self.vertices.push(left_bottom);
        self.vertices.push(right_bottom);
    }

    /// Uniformly colored rectangle.
    pub fn add_colored_rect(&mut self, rect: Rect, color: Srgba) {
        debug_assert!(self.texture_id == TextureId::Egui);
        self.add_rect_with_uv(rect, [WHITE_UV, WHITE_UV].into(), color)
    }

    /// This is for platforms that only support 16-bit index buffers.
    ///
    /// Splits this mesh into many smaller meshes (if needed).
    /// All the returned meshes will have indices that fit into a `u16`.
    pub fn split_to_u16(self) -> Vec<Triangles> {
        const MAX_SIZE: u32 = 1 << 16;

        if self.vertices.len() < MAX_SIZE as usize {
            return vec![self]; // Common-case optimization
        }

        let mut output = vec![];
        let mut index_cursor = 0;

        while index_cursor < self.indices.len() {
            let span_start = index_cursor;
            let mut min_vindex = self.indices[index_cursor];
            let mut max_vindex = self.indices[index_cursor];

            while index_cursor < self.indices.len() {
                let (mut new_min, mut new_max) = (min_vindex, max_vindex);
                for i in 0..3 {
                    let idx = self.indices[index_cursor + i];
                    new_min = new_min.min(idx);
                    new_max = new_max.max(idx);
                }

                if new_max - new_min < MAX_SIZE {
                    // Triangle fits
                    min_vindex = new_min;
                    max_vindex = new_max;
                    index_cursor += 3;
                } else {
                    break;
                }
            }

            assert!(
                index_cursor > span_start,
                "One triangle spanned more than {} vertices",
                MAX_SIZE
            );

            output.push(Triangles {
                indices: self.indices[span_start..index_cursor]
                    .iter()
                    .map(|vi| vi - min_vindex)
                    .collect(),
                vertices: self.vertices[(min_vindex as usize)..=(max_vindex as usize)].to_vec(),
                texture_id: self.texture_id,
            });
        }
        output
    }

    /// Translate location by this much, in-place
    pub fn translate(&mut self, delta: Vec2) {
        for v in &mut self.vertices {
            v.pos += delta;
        }
    }
}

// ----------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct PathPoint {
    pos: Pos2,

    /// For filled paths the normal is used for anti-aliasing (both strokes and filled areas).
    ///
    /// For strokes the normal is also used for giving thickness to the path
    /// (i.e. in what direction to expand).
    ///
    /// The normal could be estimated by differences between successive points,
    /// but that would be less accurate (and in some cases slower).
    ///
    /// Normals are normally unit-length.
    normal: Vec2,
}

/// A connected line (without thickness or gaps) which can be tessellated
/// to either to a stroke (with thickness) or a filled convex area.
/// Used as a scratch-pad during tesselation.
#[derive(Clone, Debug, Default)]
struct Path(Vec<PathPoint>);

impl Path {
    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional)
    }

    #[inline(always)]
    pub fn add_point(&mut self, pos: Pos2, normal: Vec2) {
        self.0.push(PathPoint { pos, normal });
    }

    pub fn add_circle(&mut self, center: Pos2, radius: f32) {
        let n = (radius * 4.0).round() as i32; // TODO: tweak a bit more
        let n = clamp(n, 4..=64);
        self.reserve(n as usize);
        for i in 0..n {
            let angle = remap(i as f32, 0.0..=n as f32, 0.0..=TAU);
            let normal = vec2(angle.cos(), angle.sin());
            self.add_point(center + radius * normal, normal);
        }
    }

    pub fn add_line_segment(&mut self, points: [Pos2; 2]) {
        self.reserve(2);
        let normal = (points[1] - points[0]).normalized().rot90();
        self.add_point(points[0], normal);
        self.add_point(points[1], normal);
    }

    pub fn add_open_points(&mut self, points: &[Pos2]) {
        let n = points.len();
        assert!(n >= 2);

        if n == 2 {
            // Common case optimization:
            self.add_line_segment([points[0], points[1]]);
        } else {
            // TODO: optimize
            self.reserve(n);
            self.add_point(points[0], (points[1] - points[0]).normalized().rot90());
            for i in 1..n - 1 {
                let mut n0 = (points[i] - points[i - 1]).normalized().rot90();
                let mut n1 = (points[i + 1] - points[i]).normalized().rot90();

                // Handle duplicated points (but not triplicated...):
                if n0 == Vec2::zero() {
                    n0 = n1;
                } else if n1 == Vec2::zero() {
                    n1 = n0;
                }

                let v = (n0 + n1) / 2.0;
                let normal = v / v.length_sq(); // TODO: handle VERY sharp turns better
                self.add_point(points[i], normal);
            }
            self.add_point(
                points[n - 1],
                (points[n - 1] - points[n - 2]).normalized().rot90(),
            );
        }
    }

    pub fn add_line_loop(&mut self, points: &[Pos2]) {
        let n = points.len();
        assert!(n >= 2);
        self.reserve(n);

        // TODO: optimize
        for i in 0..n {
            let mut n0 = (points[i] - points[(i + n - 1) % n]).normalized().rot90();
            let mut n1 = (points[(i + 1) % n] - points[i]).normalized().rot90();

            // Handle duplicated points (but not triplicated...):
            if n0 == Vec2::zero() {
                n0 = n1;
            } else if n1 == Vec2::zero() {
                n1 = n0;
            }

            // if n1 == Vec2::zero() {
            //     continue
            // }
            let v = (n0 + n1) / 2.0;
            let normal = v / v.length_sq(); // TODO: handle VERY sharp turns better
            self.add_point(points[i], normal);
        }
    }
}

pub(crate) mod path {
    //! Helpers for constructing paths
    use super::*;

    /// overwrites existing points
    pub fn rounded_rectangle(path: &mut Vec<Pos2>, rect: Rect, corner_radius: f32) {
        path.clear();

        let min = rect.min;
        let max = rect.max;

        let cr = corner_radius
            .min(rect.width() * 0.5)
            .min(rect.height() * 0.5);

        if cr <= 0.0 {
            let min = rect.min;
            let max = rect.max;
            path.reserve(4);
            path.push(pos2(min.x, min.y));
            path.push(pos2(max.x, min.y));
            path.push(pos2(max.x, max.y));
            path.push(pos2(min.x, max.y));
        } else {
            add_circle_quadrant(path, pos2(max.x - cr, max.y - cr), cr, 0.0);
            add_circle_quadrant(path, pos2(min.x + cr, max.y - cr), cr, 1.0);
            add_circle_quadrant(path, pos2(min.x + cr, min.y + cr), cr, 2.0);
            add_circle_quadrant(path, pos2(max.x - cr, min.y + cr), cr, 3.0);
        }
    }

    /// Add one quadrant of a circle
    ///
    /// * quadrant 0: right bottom
    /// * quadrant 1: left bottom
    /// * quadrant 2: left top
    /// * quadrant 3: right top
    //
    // Derivation:
    //
    // * angle 0 * TAU / 4 = right
    //   - quadrant 0: right bottom
    // * angle 1 * TAU / 4 = bottom
    //   - quadrant 1: left bottom
    // * angle 2 * TAU / 4 = left
    //   - quadrant 2: left top
    // * angle 3 * TAU / 4 = top
    //   - quadrant 3: right top
    // * angle 4 * TAU / 4 = right
    pub fn add_circle_quadrant(path: &mut Vec<Pos2>, center: Pos2, radius: f32, quadrant: f32) {
        // TODO: optimize with precalculated vertices for some radii ranges

        let n = (radius * 0.75).round() as i32; // TODO: tweak a bit more
        let n = clamp(n, 2..=32);
        const RIGHT_ANGLE: f32 = TAU / 4.0;
        path.reserve(n as usize + 1);
        for i in 0..=n {
            let angle = remap(
                i as f32,
                0.0..=n as f32,
                quadrant * RIGHT_ANGLE..=(quadrant + 1.0) * RIGHT_ANGLE,
            );
            path.push(center + radius * Vec2::angled(angle));
        }
    }
}

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
pub enum PathType {
    Open,
    Closed,
}
use self::PathType::{Closed, Open};

/// Tesselation quality options
#[derive(Clone, Copy, Debug)]
pub struct TesselationOptions {
    /// Size of a pixel in points, e.g. 0.5
    pub aa_size: f32,
    /// Anti-aliasing makes shapes appear smoother, but requires more triangles and is therefore slower.
    /// By default this is enabled in release builds and disabled in debug builds.
    pub anti_alias: bool,
    /// If `true` (default) cull certain primitives before tessellating them
    pub coarse_tessellation_culling: bool,
    /// Output the clip rectangles to be painted?
    pub debug_paint_clip_rects: bool,
    /// If true, no clipping will be done
    pub debug_ignore_clip_rects: bool,
}

impl Default for TesselationOptions {
    fn default() -> Self {
        Self {
            aa_size: 1.0,
            anti_alias: true,
            coarse_tessellation_culling: true,
            debug_paint_clip_rects: false,
            debug_ignore_clip_rects: false,
        }
    }
}

/// Tesselate the given convex area into a polygon.
fn fill_closed_path(
    path: &[PathPoint],
    color: Srgba,
    options: TesselationOptions,
    out: &mut Triangles,
) {
    if color == color::TRANSPARENT {
        return;
    }

    let n = path.len() as u32;
    if options.anti_alias {
        out.reserve_triangles(3 * n as usize);
        out.reserve_vertices(2 * n as usize);
        let color_outer = color::TRANSPARENT;
        let idx_inner = out.vertices.len() as u32;
        let idx_outer = idx_inner + 1;
        for i in 2..n {
            out.add_triangle(idx_inner + 2 * (i - 1), idx_inner, idx_inner + 2 * i);
        }
        let mut i0 = n - 1;
        for i1 in 0..n {
            let p1 = &path[i1 as usize];
            let dm = p1.normal * options.aa_size * 0.5;
            out.colored_vertex(p1.pos - dm, color);
            out.colored_vertex(p1.pos + dm, color_outer);
            out.add_triangle(idx_inner + i1 * 2, idx_inner + i0 * 2, idx_outer + 2 * i0);
            out.add_triangle(idx_outer + i0 * 2, idx_outer + i1 * 2, idx_inner + 2 * i1);
            i0 = i1;
        }
    } else {
        out.reserve_triangles(n as usize);
        let idx = out.vertices.len() as u32;
        out.vertices.extend(path.iter().map(|p| Vertex {
            pos: p.pos,
            uv: WHITE_UV,
            color,
        }));
        for i in 2..n {
            out.add_triangle(idx, idx + i - 1, idx + i);
        }
    }
}

/// Tesselate the given path as a stroke with thickness.
fn stroke_path(
    path: &[PathPoint],
    path_type: PathType,
    stroke: Stroke,
    options: TesselationOptions,
    out: &mut Triangles,
) {
    if stroke.width <= 0.0 || stroke.color == color::TRANSPARENT {
        return;
    }

    let n = path.len() as u32;
    let idx = out.vertices.len() as u32;

    if options.anti_alias {
        let color_inner = stroke.color;
        let color_outer = color::TRANSPARENT;

        let thin_line = stroke.width <= options.aa_size;
        if thin_line {
            /*
            We paint the line using three edges: outer, inner, outer.

            .       o   i   o      outer, inner, outer
            .       |---|          aa_size (pixel width)
            */

            // Fade out as it gets thinner:
            let color_inner = mul_color(color_inner, stroke.width / options.aa_size);
            if color_inner == color::TRANSPARENT {
                return;
            }

            out.reserve_triangles(4 * n as usize);
            out.reserve_vertices(3 * n as usize);

            let mut i0 = n - 1;
            for i1 in 0..n {
                let connect_with_previous = path_type == PathType::Closed || i1 > 0;
                let p1 = &path[i1 as usize];
                let p = p1.pos;
                let n = p1.normal;
                out.colored_vertex(p + n * options.aa_size, color_outer);
                out.colored_vertex(p, color_inner);
                out.colored_vertex(p - n * options.aa_size, color_outer);

                if connect_with_previous {
                    out.add_triangle(idx + 3 * i0 + 0, idx + 3 * i0 + 1, idx + 3 * i1 + 0);
                    out.add_triangle(idx + 3 * i0 + 1, idx + 3 * i1 + 0, idx + 3 * i1 + 1);

                    out.add_triangle(idx + 3 * i0 + 1, idx + 3 * i0 + 2, idx + 3 * i1 + 1);
                    out.add_triangle(idx + 3 * i0 + 2, idx + 3 * i1 + 1, idx + 3 * i1 + 2);
                }
                i0 = i1;
            }
        } else {
            // thick line
            // TODO: line caps for really thick lines?

            /*
            We paint the line using four edges: outer, inner, inner, outer

            .       o   i     p    i   o   outer, inner, point, inner, outer
            .       |---|                  aa_size (pixel width)
            .         |--------------|     width
            .       |---------|            outer_rad
            .           |-----|            inner_rad
            */

            out.reserve_triangles(6 * n as usize);
            out.reserve_vertices(4 * n as usize);

            let mut i0 = n - 1;
            for i1 in 0..n {
                let connect_with_previous = path_type == PathType::Closed || i1 > 0;
                let inner_rad = 0.5 * (stroke.width - options.aa_size);
                let outer_rad = 0.5 * (stroke.width + options.aa_size);
                let p1 = &path[i1 as usize];
                let p = p1.pos;
                let n = p1.normal;
                out.colored_vertex(p + n * outer_rad, color_outer);
                out.colored_vertex(p + n * inner_rad, color_inner);
                out.colored_vertex(p - n * inner_rad, color_inner);
                out.colored_vertex(p - n * outer_rad, color_outer);

                if connect_with_previous {
                    out.add_triangle(idx + 4 * i0 + 0, idx + 4 * i0 + 1, idx + 4 * i1 + 0);
                    out.add_triangle(idx + 4 * i0 + 1, idx + 4 * i1 + 0, idx + 4 * i1 + 1);

                    out.add_triangle(idx + 4 * i0 + 1, idx + 4 * i0 + 2, idx + 4 * i1 + 1);
                    out.add_triangle(idx + 4 * i0 + 2, idx + 4 * i1 + 1, idx + 4 * i1 + 2);

                    out.add_triangle(idx + 4 * i0 + 2, idx + 4 * i0 + 3, idx + 4 * i1 + 2);
                    out.add_triangle(idx + 4 * i0 + 3, idx + 4 * i1 + 2, idx + 4 * i1 + 3);
                }
                i0 = i1;
            }
        }
    } else {
        out.reserve_triangles(2 * n as usize);
        out.reserve_vertices(2 * n as usize);

        let last_index = if path_type == Closed { n } else { n - 1 };
        for i in 0..last_index {
            out.add_triangle(
                idx + (2 * i + 0) % (2 * n),
                idx + (2 * i + 1) % (2 * n),
                idx + (2 * i + 2) % (2 * n),
            );
            out.add_triangle(
                idx + (2 * i + 2) % (2 * n),
                idx + (2 * i + 1) % (2 * n),
                idx + (2 * i + 3) % (2 * n),
            );
        }

        let thin_line = stroke.width <= options.aa_size;
        if thin_line {
            // Fade out thin lines rather than making them thinner
            let radius = options.aa_size / 2.0;
            let color = mul_color(stroke.color, stroke.width / options.aa_size);
            if color == color::TRANSPARENT {
                return;
            }
            for p in path {
                out.colored_vertex(p.pos + radius * p.normal, color);
                out.colored_vertex(p.pos - radius * p.normal, color);
            }
        } else {
            let radius = stroke.width / 2.0;
            for p in path {
                out.colored_vertex(p.pos + radius * p.normal, stroke.color);
                out.colored_vertex(p.pos - radius * p.normal, stroke.color);
            }
        }
    }
}

fn mul_color(color: Srgba, factor: f32) -> Srgba {
    debug_assert!(0.0 <= factor && factor <= 1.0);
    // sRGBA correct fading requires conversion to linear space and back again because of premultiplied alpha
    Rgba::from(color).multiply(factor).into()
}

// ----------------------------------------------------------------------------

/// Tesselate a single [`PaintCmd`] into a [`Triangles`].
///
/// * `command`: the command to tesselate
/// * `options`: tesselation quality
/// * `fonts`: font source when tessellating text
/// * `out`: where the triangles are put
/// * `scratchpad_path`: if you plan to run `tessellate_paint_command`
///    many times, pass it a reference to the same `Path` to avoid excessive allocations.
fn tessellate_paint_command(
    options: TesselationOptions,
    fonts: &Fonts,
    clip_rect: Rect,
    command: PaintCmd,
    out: &mut Triangles,
    scratchpad_points: &mut Vec<Pos2>,
    scratchpad_path: &mut Path,
) {
    let path = scratchpad_path;
    path.clear();

    match command {
        PaintCmd::Noop => {}
        PaintCmd::Circle {
            center,
            radius,
            fill,
            stroke,
        } => {
            if radius <= 0.0 {
                return;
            }

            if options.coarse_tessellation_culling
                && !clip_rect.expand(radius + stroke.width).contains(center)
            {
                return;
            }

            path.add_circle(center, radius);
            fill_closed_path(&path.0, fill, options, out);
            stroke_path(&path.0, Closed, stroke, options, out);
        }
        PaintCmd::Triangles(triangles) => {
            if triangles.is_valid() {
                out.append(triangles);
            } else {
                debug_assert!(false, "Invalid Triangles in PaintCmd::Triangles");
            }
        }
        PaintCmd::LineSegment { points, stroke } => {
            path.add_line_segment(points);
            stroke_path(&path.0, Open, stroke, options, out);
        }
        PaintCmd::Path {
            points,
            closed,
            fill,
            stroke,
        } => {
            if points.len() >= 2 {
                if closed {
                    path.add_line_loop(&points);
                } else {
                    path.add_open_points(&points);
                }

                if fill != TRANSPARENT {
                    debug_assert!(
                        closed,
                        "You asked to fill a path that is not closed. That makes no sense."
                    );
                    fill_closed_path(&path.0, fill, options, out);
                }
                let typ = if closed { Closed } else { Open };
                stroke_path(&path.0, typ, stroke, options, out);
            }
        }
        PaintCmd::Rect {
            mut rect,
            corner_radius,
            fill,
            stroke,
        } => {
            if rect.is_empty() {
                return;
            }

            if options.coarse_tessellation_culling
                && !rect.expand(stroke.width).intersects(clip_rect)
            {
                return;
            }

            // It is common to (sometimes accidentally) create an infinitely sized rectangle.
            // Make sure we can handle that:
            rect.min = rect.min.at_least(pos2(-1e7, -1e7));
            rect.max = rect.max.at_most(pos2(1e7, 1e7));

            path::rounded_rectangle(scratchpad_points, rect, corner_radius);
            path.add_line_loop(scratchpad_points);
            fill_closed_path(&path.0, fill, options, out);
            stroke_path(&path.0, Closed, stroke, options, out);
        }
        PaintCmd::Text {
            pos,
            galley,
            text_style,
            color,
        } => {
            tesselate_text(
                options, fonts, clip_rect, pos, &galley, text_style, color, out,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn tesselate_text(
    options: TesselationOptions,
    fonts: &Fonts,
    clip_rect: Rect,
    pos: Pos2,
    galley: &super::Galley,
    text_style: super::TextStyle,
    color: Srgba,
    out: &mut Triangles,
) {
    if color == TRANSPARENT {
        return;
    }
    galley.sanity_check();

    let num_chars = galley.text.chars().count();
    out.reserve_triangles(num_chars * 2);
    out.reserve_vertices(num_chars * 4);

    let tex_w = fonts.texture().width as f32;
    let tex_h = fonts.texture().height as f32;

    let text_offset = vec2(0.0, 1.0); // Eye-balled for buttons. TODO: why is this needed?

    let clip_rect = clip_rect.expand(2.0); // Some fudge to handle letters that are slightly larger than expected.

    let font = &fonts[text_style];
    let mut chars = galley.text.chars();
    for line in &galley.rows {
        let line_min_y = pos.y + line.y_min + text_offset.x;
        let line_max_y = line_min_y + font.row_height();
        let is_line_visible = line_max_y >= clip_rect.min.y && line_min_y <= clip_rect.max.y;

        for x_offset in line.x_offsets.iter().take(line.x_offsets.len() - 1) {
            let c = chars.next().unwrap();

            if options.coarse_tessellation_culling && !is_line_visible {
                // culling individual lines of text is important, since a single `PaintCmd::Text`
                // can span hundreds of lines.
                continue;
            }

            if let Some(glyph) = font.uv_rect(c) {
                let mut left_top = pos + glyph.offset + vec2(*x_offset, line.y_min) + text_offset;
                left_top.x = font.round_to_pixel(left_top.x); // Pixel-perfection.
                left_top.y = font.round_to_pixel(left_top.y); // Pixel-perfection.

                let pos = Rect::from_min_max(left_top, left_top + glyph.size);
                let uv = Rect::from_min_max(
                    pos2(glyph.min.0 as f32 / tex_w, glyph.min.1 as f32 / tex_h),
                    pos2(glyph.max.0 as f32 / tex_w, glyph.max.1 as f32 / tex_h),
                );
                out.add_rect_with_uv(pos, uv, color);
            }
        }
        if line.ends_with_newline {
            let newline = chars.next().unwrap();
            debug_assert_eq!(newline, '\n');
        }
    }
    assert_eq!(chars.next(), None);
}

/// Turns [`PaintCmd`]:s into sets of triangles.
///
/// The given commands will be painted back-to-front (painters algorithm).
/// They will be batched together by clip rectangle.
///
/// * `commands`: the command to tesselate
/// * `options`: tesselation quality
/// * `fonts`: font source when tessellating text
///
/// ## Returns
/// A list of clip rectangles with matching [`Triangles`].
pub fn tessellate_paint_commands(
    commands: Vec<(Rect, PaintCmd)>,
    options: TesselationOptions,
    fonts: &Fonts,
) -> Vec<(Rect, Triangles)> {
    let mut scratchpad_points = Vec::new();
    let mut scratchpad_path = Path::default();

    let mut jobs = PaintJobs::default();
    for (clip_rect, cmd) in commands {
        let start_new_job = match jobs.last() {
            None => true,
            Some(job) => job.0 != clip_rect || job.1.texture_id != cmd.texture_id(),
        };

        if start_new_job {
            jobs.push((clip_rect, Triangles::default()));
        }

        let out = &mut jobs.last_mut().unwrap().1;
        tessellate_paint_command(
            options,
            fonts,
            clip_rect,
            cmd,
            out,
            &mut scratchpad_points,
            &mut scratchpad_path,
        );
    }

    if options.debug_paint_clip_rects {
        for (clip_rect, triangles) in &mut jobs {
            tessellate_paint_command(
                options,
                fonts,
                Rect::everything(),
                PaintCmd::Rect {
                    rect: *clip_rect,
                    corner_radius: 0.0,
                    fill: Default::default(),
                    stroke: Stroke::new(2.0, srgba(150, 255, 150, 255)),
                },
                triangles,
                &mut scratchpad_points,
                &mut scratchpad_path,
            )
        }
    }

    if options.debug_ignore_clip_rects {
        for (clip_rect, _) in &mut jobs {
            *clip_rect = Rect::everything();
        }
    }

    for (_, triangles) in &jobs {
        debug_assert!(
            triangles.is_valid(),
            "Tesselator generated invalid Triangles"
        );
    }

    jobs
}
