//! Converts graphics primitives into textured triangles.
//!
//! This module converts lines, circles, text and more represented by [`Shape`]
//! into textured triangles represented by [`Mesh`].

#![allow(clippy::identity_op)]

use crate::*;
use emath::*;
use std::f32::consts::TAU;

// ----------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
struct PathPoint {
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
/// Used as a scratch-pad during tessellation.
#[derive(Clone, Debug, Default)]
pub struct Path(Vec<PathPoint>);

impl Path {
    #[inline(always)]
    pub fn clear(&mut self) {
        self.0.clear();
    }

    #[inline(always)]
    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }

    #[inline(always)]
    pub fn add_point(&mut self, pos: Pos2, normal: Vec2) {
        self.0.push(PathPoint { pos, normal });
    }

    pub fn add_circle(&mut self, center: Pos2, radius: f32) {
        let n = (radius * 4.0).round() as i32; // TODO: tweak a bit more
        let n = n.clamp(4, 64);
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
            self.reserve(n);
            self.add_point(points[0], (points[1] - points[0]).normalized().rot90());
            let mut n0 = (points[1] - points[0]).normalized().rot90();
            for i in 1..n - 1 {
                let mut n1 = (points[i + 1] - points[i]).normalized().rot90();

                // Handle duplicated points (but not triplicated…):
                if n0 == Vec2::ZERO {
                    n0 = n1;
                } else if n1 == Vec2::ZERO {
                    n1 = n0;
                }

                let normal = (n0 + n1) / 2.0;
                let length_sq = normal.length_sq();
                let right_angle_length_sq = 0.5;
                let sharper_than_a_right_angle = length_sq < right_angle_length_sq;
                if sharper_than_a_right_angle {
                    // cut off the sharp corner
                    let center_normal = normal.normalized();
                    let n0c = (n0 + center_normal) / 2.0;
                    let n1c = (n1 + center_normal) / 2.0;
                    self.add_point(points[i], n0c / n0c.length_sq());
                    self.add_point(points[i], n1c / n1c.length_sq());
                } else {
                    // miter join
                    self.add_point(points[i], normal / length_sq);
                }

                n0 = n1;
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

        let mut n0 = (points[0] - points[n - 1]).normalized().rot90();

        for i in 0..n {
            let next_i = if i + 1 == n { 0 } else { i + 1 };
            let mut n1 = (points[next_i] - points[i]).normalized().rot90();

            // Handle duplicated points (but not triplicated…):
            if n0 == Vec2::ZERO {
                n0 = n1;
            } else if n1 == Vec2::ZERO {
                n1 = n0;
            }

            let normal = (n0 + n1) / 2.0;
            let length_sq = normal.length_sq();

            // We can't just cut off corners for filled shapes like this,
            // because the feather will both expand and contract the corner along the provided normals
            // to make sure it doesn't grow, and the shrinking will make the inner points cross each other.
            //
            // A better approach is to shrink the vertices in by half the feather-width here
            // and then only expand during feathering.
            //
            // See https://github.com/emilk/egui/issues/1226
            const CUT_OFF_SHARP_CORNERS: bool = false;

            let right_angle_length_sq = 0.5;
            let sharper_than_a_right_angle = length_sq < right_angle_length_sq;
            if CUT_OFF_SHARP_CORNERS && sharper_than_a_right_angle {
                // cut off the sharp corner
                let center_normal = normal.normalized();
                let n0c = (n0 + center_normal) / 2.0;
                let n1c = (n1 + center_normal) / 2.0;
                self.add_point(points[i], n0c / n0c.length_sq());
                self.add_point(points[i], n1c / n1c.length_sq());
            } else {
                // miter join
                self.add_point(points[i], normal / length_sq);
            }

            n0 = n1;
        }
    }

    /// Open-ended.
    pub fn stroke_open(&self, stroke: Stroke, options: &TessellationOptions, out: &mut Mesh) {
        stroke_path(&self.0, PathType::Open, stroke, options, out);
    }

    /// A closed path (returning to the first point).
    pub fn stroke_closed(&self, stroke: Stroke, options: &TessellationOptions, out: &mut Mesh) {
        stroke_path(&self.0, PathType::Closed, stroke, options, out);
    }

    pub fn stroke(
        &self,
        path_type: PathType,
        stroke: Stroke,
        options: &TessellationOptions,
        out: &mut Mesh,
    ) {
        stroke_path(&self.0, path_type, stroke, options, out);
    }

    /// The path is taken to be closed (i.e. returning to the start again).
    ///
    /// Calling this may reverse the vertices in the path if they are wrong winding order.
    ///
    /// The preferred winding order is clockwise.
    pub fn fill(&mut self, color: Color32, options: &TessellationOptions, out: &mut Mesh) {
        fill_closed_path(&mut self.0, color, options, out);
    }
}

pub mod path {
    //! Helpers for constructing paths
    use crate::shape::Rounding;

    use super::*;

    /// overwrites existing points
    pub fn rounded_rectangle(path: &mut Vec<Pos2>, rect: Rect, rounding: Rounding) {
        path.clear();

        let min = rect.min;
        let max = rect.max;

        let r = clamp_radius(rounding, rect);

        if r == Rounding::none() {
            let min = rect.min;
            let max = rect.max;
            path.reserve(4);
            path.push(pos2(min.x, min.y)); // left top
            path.push(pos2(max.x, min.y)); // right top
            path.push(pos2(max.x, max.y)); // right bottom
            path.push(pos2(min.x, max.y)); // left bottom
        } else {
            add_circle_quadrant(path, pos2(max.x - r.se, max.y - r.se), r.se, 0.0);
            add_circle_quadrant(path, pos2(min.x + r.sw, max.y - r.sw), r.sw, 1.0);
            add_circle_quadrant(path, pos2(min.x + r.nw, min.y + r.nw), r.nw, 2.0);
            add_circle_quadrant(path, pos2(max.x - r.ne, min.y + r.ne), r.ne, 3.0);
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
        let n = n.clamp(2, 32);
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

    // Ensures the radius of each corner is within a valid range
    fn clamp_radius(rounding: Rounding, rect: Rect) -> Rounding {
        let half_width = rect.width() * 0.5;
        let half_height = rect.height() * 0.5;
        let max_cr = half_width.min(half_height);

        Rounding {
            nw: rounding.nw.at_most(max_cr).at_least(0.0),
            ne: rounding.ne.at_most(max_cr).at_least(0.0),
            sw: rounding.sw.at_most(max_cr).at_least(0.0),
            se: rounding.se.at_most(max_cr).at_least(0.0),
        }
    }
}

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
pub enum PathType {
    Open,
    Closed,
}

/// Tessellation quality options
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct TessellationOptions {
    /// Size of a point in pixels (DPI scaling), e.g. 2.0. Used to snap text to pixel boundaries.
    pub pixels_per_point: f32,

    /// The size of a pixel (in points), used for anti-aliasing (smoothing of edges).
    /// This is normally the inverse of [`Self::pixels_per_point`],
    /// but you can make it larger if you want more blurry edges.
    pub aa_size: f32,

    /// Anti-aliasing makes shapes appear smoother, but requires more triangles and is therefore slower.
    /// This setting does not affect text.
    /// Default: `true`.
    pub anti_alias: bool,

    /// If `true` (default) cull certain primitives before tessellating them.
    /// This likely makes
    pub coarse_tessellation_culling: bool,

    /// If `true` (default) align text to mesh grid.
    /// This makes the text sharper on most platforms.
    pub round_text_to_pixels: bool,

    /// Output the clip rectangles to be painted.
    pub debug_paint_clip_rects: bool,

    /// Output the text-containing rectangles.
    pub debug_paint_text_rects: bool,

    /// If true, no clipping will be done.
    pub debug_ignore_clip_rects: bool,

    /// The maximum distance between the original curve and the flattened curve.
    pub bezier_tolerance: f32,

    /// The default value will be 1.0e-5, it will be used during float compare.
    pub epsilon: f32,
}

impl Default for TessellationOptions {
    fn default() -> Self {
        Self {
            pixels_per_point: 1.0,
            aa_size: 1.0,
            anti_alias: true,
            coarse_tessellation_culling: true,
            round_text_to_pixels: true,
            debug_paint_text_rects: false,
            debug_paint_clip_rects: false,
            debug_ignore_clip_rects: false,
            bezier_tolerance: 0.1,
            epsilon: 1.0e-5,
        }
    }
}

impl TessellationOptions {
    pub fn from_pixels_per_point(pixels_per_point: f32) -> Self {
        Self {
            pixels_per_point,
            aa_size: 1.0 / pixels_per_point,
            ..Default::default()
        }
    }
}

impl TessellationOptions {
    #[inline(always)]
    pub fn round_to_pixel(&self, point: f32) -> f32 {
        if self.round_text_to_pixels {
            (point * self.pixels_per_point).round() / self.pixels_per_point
        } else {
            point
        }
    }
}

fn cw_signed_area(path: &[PathPoint]) -> f64 {
    if let Some(last) = path.last() {
        let mut previous = last.pos;
        let mut area = 0.0;
        for p in path {
            area += (previous.x * p.pos.y - p.pos.x * previous.y) as f64;
            previous = p.pos;
        }
        area
    } else {
        0.0
    }
}

/// Tessellate the given convex area into a polygon.
///
/// Calling this may reverse the vertices in the path if they are wrong winding order.
///
/// The preferred winding order is clockwise.
fn fill_closed_path(
    path: &mut [PathPoint],
    color: Color32,
    options: &TessellationOptions,
    out: &mut Mesh,
) {
    if color == Color32::TRANSPARENT {
        return;
    }

    let n = path.len() as u32;
    if options.anti_alias {
        if cw_signed_area(path) < 0.0 {
            // Wrong winding order - fix:
            path.reverse();
            for point in path.iter_mut() {
                point.normal = -point.normal;
            }
        }

        out.reserve_triangles(3 * n as usize);
        out.reserve_vertices(2 * n as usize);
        let color_outer = Color32::TRANSPARENT;
        let idx_inner = out.vertices.len() as u32;
        let idx_outer = idx_inner + 1;

        // The fill:
        for i in 2..n {
            out.add_triangle(idx_inner + 2 * (i - 1), idx_inner, idx_inner + 2 * i);
        }

        // The feathering:
        let mut i0 = n - 1;
        for i1 in 0..n {
            let p1 = &path[i1 as usize];
            let dm = 0.5 * options.aa_size * p1.normal;
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

/// Tessellate the given path as a stroke with thickness.
fn stroke_path(
    path: &[PathPoint],
    path_type: PathType,
    stroke: Stroke,
    options: &TessellationOptions,
    out: &mut Mesh,
) {
    let n = path.len() as u32;

    if stroke.width <= 0.0 || stroke.color == Color32::TRANSPARENT || n < 2 {
        return;
    }

    let idx = out.vertices.len() as u32;

    if options.anti_alias {
        let color_inner = stroke.color;
        let color_outer = Color32::TRANSPARENT;

        let thin_line = stroke.width <= options.aa_size;
        if thin_line {
            /*
            We paint the line using three edges: outer, inner, outer.

            .       o   i   o      outer, inner, outer
            .       |---|          aa_size (pixel width)
            */

            // Fade out as it gets thinner:
            let color_inner = mul_color(color_inner, stroke.width / options.aa_size);
            if color_inner == Color32::TRANSPARENT {
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
            // thick anti-aliased line

            /*
            We paint the line using four edges: outer, inner, inner, outer

            .       o   i     p    i   o   outer, inner, point, inner, outer
            .       |---|                  aa_size (pixel width)
            .         |--------------|     width
            .       |---------|            outer_rad
            .           |-----|            inner_rad
            */

            let inner_rad = 0.5 * (stroke.width - options.aa_size);
            let outer_rad = 0.5 * (stroke.width + options.aa_size);

            match path_type {
                PathType::Closed => {
                    out.reserve_triangles(6 * n as usize);
                    out.reserve_vertices(4 * n as usize);

                    let mut i0 = n - 1;
                    for i1 in 0..n {
                        let p1 = &path[i1 as usize];
                        let p = p1.pos;
                        let n = p1.normal;
                        out.colored_vertex(p + n * outer_rad, color_outer);
                        out.colored_vertex(p + n * inner_rad, color_inner);
                        out.colored_vertex(p - n * inner_rad, color_inner);
                        out.colored_vertex(p - n * outer_rad, color_outer);

                        out.add_triangle(idx + 4 * i0 + 0, idx + 4 * i0 + 1, idx + 4 * i1 + 0);
                        out.add_triangle(idx + 4 * i0 + 1, idx + 4 * i1 + 0, idx + 4 * i1 + 1);

                        out.add_triangle(idx + 4 * i0 + 1, idx + 4 * i0 + 2, idx + 4 * i1 + 1);
                        out.add_triangle(idx + 4 * i0 + 2, idx + 4 * i1 + 1, idx + 4 * i1 + 2);

                        out.add_triangle(idx + 4 * i0 + 2, idx + 4 * i0 + 3, idx + 4 * i1 + 2);
                        out.add_triangle(idx + 4 * i0 + 3, idx + 4 * i1 + 2, idx + 4 * i1 + 3);

                        i0 = i1;
                    }
                }
                PathType::Open => {
                    // Anti-alias the ends by extruding the outer edge and adding
                    // two more triangles to each end:

                    //   | aa |       | aa |
                    //    _________________   ___
                    //   | \    added    / |  aa_size
                    //   |   \ ___p___ /   |  ___
                    //   |    |       |    |
                    //   |    |  opa  |    |
                    //   |    |  que  |    |
                    //   |    |       |    |

                    // (in the future it would be great with an option to add a circular end instead)

                    out.reserve_triangles(6 * n as usize + 4);
                    out.reserve_vertices(4 * n as usize);

                    {
                        let end = &path[0];
                        let p = end.pos;
                        let n = end.normal;
                        let back_extrude = n.rot90() * options.aa_size;
                        out.colored_vertex(p + n * outer_rad + back_extrude, color_outer);
                        out.colored_vertex(p + n * inner_rad, color_inner);
                        out.colored_vertex(p - n * inner_rad, color_inner);
                        out.colored_vertex(p - n * outer_rad + back_extrude, color_outer);

                        out.add_triangle(idx + 0, idx + 1, idx + 2);
                        out.add_triangle(idx + 0, idx + 2, idx + 3);
                    }

                    let mut i0 = 0;
                    for i1 in 1..n - 1 {
                        let point = &path[i1 as usize];
                        let p = point.pos;
                        let n = point.normal;
                        out.colored_vertex(p + n * outer_rad, color_outer);
                        out.colored_vertex(p + n * inner_rad, color_inner);
                        out.colored_vertex(p - n * inner_rad, color_inner);
                        out.colored_vertex(p - n * outer_rad, color_outer);

                        out.add_triangle(idx + 4 * i0 + 0, idx + 4 * i0 + 1, idx + 4 * i1 + 0);
                        out.add_triangle(idx + 4 * i0 + 1, idx + 4 * i1 + 0, idx + 4 * i1 + 1);

                        out.add_triangle(idx + 4 * i0 + 1, idx + 4 * i0 + 2, idx + 4 * i1 + 1);
                        out.add_triangle(idx + 4 * i0 + 2, idx + 4 * i1 + 1, idx + 4 * i1 + 2);

                        out.add_triangle(idx + 4 * i0 + 2, idx + 4 * i0 + 3, idx + 4 * i1 + 2);
                        out.add_triangle(idx + 4 * i0 + 3, idx + 4 * i1 + 2, idx + 4 * i1 + 3);

                        i0 = i1;
                    }

                    {
                        let i1 = n - 1;
                        let end = &path[i1 as usize];
                        let p = end.pos;
                        let n = end.normal;
                        let back_extrude = -n.rot90() * options.aa_size;
                        out.colored_vertex(p + n * outer_rad + back_extrude, color_outer);
                        out.colored_vertex(p + n * inner_rad, color_inner);
                        out.colored_vertex(p - n * inner_rad, color_inner);
                        out.colored_vertex(p - n * outer_rad + back_extrude, color_outer);

                        out.add_triangle(idx + 4 * i0 + 0, idx + 4 * i0 + 1, idx + 4 * i1 + 0);
                        out.add_triangle(idx + 4 * i0 + 1, idx + 4 * i1 + 0, idx + 4 * i1 + 1);

                        out.add_triangle(idx + 4 * i0 + 1, idx + 4 * i0 + 2, idx + 4 * i1 + 1);
                        out.add_triangle(idx + 4 * i0 + 2, idx + 4 * i1 + 1, idx + 4 * i1 + 2);

                        out.add_triangle(idx + 4 * i0 + 2, idx + 4 * i0 + 3, idx + 4 * i1 + 2);
                        out.add_triangle(idx + 4 * i0 + 3, idx + 4 * i1 + 2, idx + 4 * i1 + 3);

                        // The extension:
                        out.add_triangle(idx + 4 * i1 + 0, idx + 4 * i1 + 1, idx + 4 * i1 + 2);
                        out.add_triangle(idx + 4 * i1 + 0, idx + 4 * i1 + 2, idx + 4 * i1 + 3);
                    }
                }
            }
        }
    } else {
        // not anti-aliased:
        out.reserve_triangles(2 * n as usize);
        out.reserve_vertices(2 * n as usize);

        let last_index = if path_type == PathType::Closed {
            n
        } else {
            n - 1
        };
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
            if color == Color32::TRANSPARENT {
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

fn mul_color(color: Color32, factor: f32) -> Color32 {
    crate::epaint_assert!(0.0 <= factor && factor <= 1.0);
    // As an unfortunate side-effect of using premultiplied alpha
    // we need a somewhat expensive conversion to linear space and back.
    color.linear_multiply(factor)
}

// ----------------------------------------------------------------------------

/// Converts [`Shape`]s into triangles ([`Mesh`]).
///
/// For performance reasons it is smart to reuse the same `Tessellator`.
///
/// Se also [`tessellate_shapes`], a convenient wrapper around [`Tessellator`].
pub struct Tessellator {
    options: TessellationOptions,
    /// Only used for culling
    clip_rect: Rect,
    scratchpad_points: Vec<Pos2>,
    scratchpad_path: Path,
}

impl Tessellator {
    /// Create a new [`Tessellator`].
    pub fn from_options(options: TessellationOptions) -> Self {
        Self {
            options,
            clip_rect: Rect::EVERYTHING,
            scratchpad_points: Default::default(),
            scratchpad_path: Default::default(),
        }
    }

    /// Tessellate a single [`Shape`] into a [`Mesh`].
    ///
    /// * `tex_size`: size of the font texture (required to normalize glyph uv rectangles).
    /// * `shape`: the shape to tessellate.
    /// * `out`: triangles are appended to this.
    pub fn tessellate_shape(&mut self, tex_size: [usize; 2], shape: Shape, out: &mut Mesh) {
        let clip_rect = self.clip_rect;
        let options = &self.options;

        match shape {
            Shape::Noop => {}
            Shape::Vec(vec) => {
                for shape in vec {
                    self.tessellate_shape(tex_size, shape, out);
                }
            }
            Shape::Circle(CircleShape {
                center,
                radius,
                fill,
                stroke,
            }) => {
                if radius <= 0.0 {
                    return;
                }

                if options.coarse_tessellation_culling
                    && !clip_rect.expand(radius + stroke.width).contains(center)
                {
                    return;
                }

                self.scratchpad_path.clear();
                self.scratchpad_path.add_circle(center, radius);
                self.scratchpad_path.fill(fill, options, out);
                self.scratchpad_path.stroke_closed(stroke, options, out);
            }
            Shape::Mesh(mesh) => {
                if !mesh.is_valid() {
                    crate::epaint_assert!(false, "Invalid Mesh in Shape::Mesh");
                    return;
                }

                if options.coarse_tessellation_culling && !clip_rect.intersects(mesh.calc_bounds())
                {
                    return;
                }

                out.append(mesh);
            }
            Shape::LineSegment { points, stroke } => {
                if stroke.is_empty() {
                    return;
                }

                if options.coarse_tessellation_culling
                    && !clip_rect
                        .intersects(Rect::from_two_pos(points[0], points[1]).expand(stroke.width))
                {
                    return;
                }

                self.scratchpad_path.clear();
                self.scratchpad_path.add_line_segment(points);
                self.scratchpad_path.stroke_open(stroke, options, out);
            }
            Shape::Path(path_shape) => {
                self.tessellate_path(path_shape, out);
            }
            Shape::Rect(rect_shape) => {
                self.tessellate_rect(&rect_shape, out);
            }
            Shape::Text(text_shape) => {
                if options.debug_paint_text_rects {
                    let rect = text_shape.galley.rect.translate(text_shape.pos.to_vec2());
                    self.tessellate_rect(
                        &RectShape::stroke(rect.expand(0.5), 2.0, (0.5, Color32::GREEN)),
                        out,
                    );
                }
                self.tessellate_text(tex_size, text_shape, out);
            }
            Shape::QuadraticBezier(quadratic_shape) => {
                self.tessellate_quadratic_bezier(quadratic_shape, out);
            }
            Shape::CubicBezier(cubic_shape) => self.tessellate_cubic_bezier(cubic_shape, out),
            Shape::Callback(_) => {
                panic!("Shape::Callback passed to Tessellator");
            }
        }
    }

    pub(crate) fn tessellate_quadratic_bezier(
        &mut self,
        quadratic_shape: QuadraticBezierShape,
        out: &mut Mesh,
    ) {
        let options = &self.options;
        let clip_rect = self.clip_rect;

        if options.coarse_tessellation_culling
            && !quadratic_shape.visual_bounding_rect().intersects(clip_rect)
        {
            return;
        }

        let points = quadratic_shape.flatten(Some(options.bezier_tolerance));

        self.tessellate_bezier_complete(
            &points,
            quadratic_shape.fill,
            quadratic_shape.closed,
            quadratic_shape.stroke,
            out,
        );
    }

    pub(crate) fn tessellate_cubic_bezier(
        &mut self,
        cubic_shape: CubicBezierShape,
        out: &mut Mesh,
    ) {
        let options = &self.options;
        let clip_rect = self.clip_rect;
        if options.coarse_tessellation_culling
            && !cubic_shape.visual_bounding_rect().intersects(clip_rect)
        {
            return;
        }

        let points_vec =
            cubic_shape.flatten_closed(Some(options.bezier_tolerance), Some(options.epsilon));

        for points in points_vec {
            self.tessellate_bezier_complete(
                &points,
                cubic_shape.fill,
                cubic_shape.closed,
                cubic_shape.stroke,
                out,
            );
        }
    }

    fn tessellate_bezier_complete(
        &mut self,
        points: &[Pos2],
        fill: Color32,
        closed: bool,
        stroke: Stroke,
        out: &mut Mesh,
    ) {
        self.scratchpad_path.clear();
        if closed {
            self.scratchpad_path.add_line_loop(points);
        } else {
            self.scratchpad_path.add_open_points(points);
        }
        if fill != Color32::TRANSPARENT {
            crate::epaint_assert!(
                closed,
                "You asked to fill a path that is not closed. That makes no sense."
            );
            self.scratchpad_path.fill(fill, &self.options, out);
        }
        let typ = if closed {
            PathType::Closed
        } else {
            PathType::Open
        };
        self.scratchpad_path.stroke(typ, stroke, &self.options, out);
    }

    pub(crate) fn tessellate_path(&mut self, path_shape: PathShape, out: &mut Mesh) {
        if path_shape.points.len() < 2 {
            return;
        }

        if self.options.coarse_tessellation_culling
            && !path_shape.visual_bounding_rect().intersects(self.clip_rect)
        {
            return;
        }

        let PathShape {
            points,
            closed,
            fill,
            stroke,
        } = path_shape;

        self.scratchpad_path.clear();
        if closed {
            self.scratchpad_path.add_line_loop(&points);
        } else {
            self.scratchpad_path.add_open_points(&points);
        }

        if fill != Color32::TRANSPARENT {
            crate::epaint_assert!(
                closed,
                "You asked to fill a path that is not closed. That makes no sense."
            );
            self.scratchpad_path.fill(fill, &self.options, out);
        }
        let typ = if closed {
            PathType::Closed
        } else {
            PathType::Open
        };
        self.scratchpad_path.stroke(typ, stroke, &self.options, out);
    }

    pub(crate) fn tessellate_rect(&mut self, rect: &RectShape, out: &mut Mesh) {
        let RectShape {
            mut rect,
            rounding,
            fill,
            stroke,
        } = *rect;

        if self.options.coarse_tessellation_culling
            && !rect.expand(stroke.width).intersects(self.clip_rect)
        {
            return;
        }
        if rect.is_negative() {
            return;
        }

        // It is common to (sometimes accidentally) create an infinitely sized rectangle.
        // Make sure we can handle that:
        rect.min = rect.min.at_least(pos2(-1e7, -1e7));
        rect.max = rect.max.at_most(pos2(1e7, 1e7));

        let path = &mut self.scratchpad_path;
        path.clear();
        path::rounded_rectangle(&mut self.scratchpad_points, rect, rounding);
        path.add_line_loop(&self.scratchpad_points);
        path.fill(fill, &self.options, out);
        path.stroke_closed(stroke, &self.options, out);
    }

    pub fn tessellate_text(&mut self, tex_size: [usize; 2], text_shape: TextShape, out: &mut Mesh) {
        let TextShape {
            pos: galley_pos,
            galley,
            underline,
            override_text_color,
            angle,
        } = text_shape;

        if galley.is_empty() {
            return;
        }

        out.vertices.reserve(galley.num_vertices);
        out.indices.reserve(galley.num_indices);

        // The contents of the galley is already snapped to pixel coordinates,
        // but we need to make sure the galley ends up on the start of a physical pixel:
        let galley_pos = pos2(
            self.options.round_to_pixel(galley_pos.x),
            self.options.round_to_pixel(galley_pos.y),
        );

        let uv_normalizer = vec2(1.0 / tex_size[0] as f32, 1.0 / tex_size[1] as f32);

        let rotator = Rot2::from_angle(angle);

        for row in &galley.rows {
            if row.visuals.mesh.is_empty() {
                continue;
            }

            let mut row_rect = row.visuals.mesh_bounds;
            if angle != 0.0 {
                row_rect = row_rect.rotate_bb(rotator);
            }
            row_rect = row_rect.translate(galley_pos.to_vec2());

            if self.options.coarse_tessellation_culling && !self.clip_rect.intersects(row_rect) {
                // culling individual lines of text is important, since a single `Shape::Text`
                // can span hundreds of lines.
                continue;
            }

            let index_offset = out.vertices.len() as u32;

            out.indices.extend(
                row.visuals
                    .mesh
                    .indices
                    .iter()
                    .map(|index| index + index_offset),
            );

            out.vertices.extend(
                row.visuals
                    .mesh
                    .vertices
                    .iter()
                    .enumerate()
                    .map(|(i, vertex)| {
                        let Vertex { pos, uv, mut color } = *vertex;

                        if let Some(override_text_color) = override_text_color {
                            if row.visuals.glyph_vertex_range.contains(&i) {
                                color = override_text_color;
                            }
                        }

                        let offset = if angle == 0.0 {
                            pos.to_vec2()
                        } else {
                            rotator * pos.to_vec2()
                        };

                        Vertex {
                            pos: galley_pos + offset,
                            uv: (uv.to_vec2() * uv_normalizer).to_pos2(),
                            color,
                        }
                    }),
            );

            if underline != Stroke::none() {
                self.scratchpad_path.clear();
                self.scratchpad_path
                    .add_line_segment([row_rect.left_bottom(), row_rect.right_bottom()]);
                self.scratchpad_path
                    .stroke_open(underline, &self.options, out);
            }
        }
    }
}

/// Turns [`Shape`]:s into sets of triangles.
///
/// The given shapes will tessellated in the same order as they are given.
/// They will be batched together by clip rectangle.
///
/// * `shapes`: what to tessellate
/// * `options`: tessellation quality
/// * `tex_size`: size of the font texture (required to normalize glyph uv rectangles)
///
/// The implementation uses a [`Tessellator`].
///
/// ## Returns
/// A list of clip rectangles with matching [`Mesh`].
pub fn tessellate_shapes(
    shapes: Vec<ClippedShape>,
    options: TessellationOptions,
    tex_size: [usize; 2],
) -> Vec<ClippedPrimitive> {
    let mut tessellator = Tessellator::from_options(options);

    let mut clipped_primitives: Vec<ClippedPrimitive> = Vec::default();

    for ClippedShape(new_clip_rect, new_shape) in shapes {
        if !new_clip_rect.is_positive() {
            continue; // skip empty clip rectangles
        }

        if let Shape::Callback(callback) = new_shape {
            clipped_primitives.push(ClippedPrimitive {
                clip_rect: new_clip_rect,
                primitive: Primitive::Callback(callback),
            });
        } else {
            let start_new_mesh = match clipped_primitives.last() {
                None => true,
                Some(output_clipped_primitive) => {
                    output_clipped_primitive.clip_rect != new_clip_rect
                        || if let Primitive::Mesh(output_mesh) = &output_clipped_primitive.primitive
                        {
                            output_mesh.texture_id != new_shape.texture_id()
                        } else {
                            true
                        }
                }
            };

            if start_new_mesh {
                clipped_primitives.push(ClippedPrimitive {
                    clip_rect: new_clip_rect,
                    primitive: Primitive::Mesh(Mesh::default()),
                });
            }

            let out = clipped_primitives.last_mut().unwrap();

            if let Primitive::Mesh(out_mesh) = &mut out.primitive {
                tessellator.clip_rect = new_clip_rect;
                tessellator.tessellate_shape(tex_size, new_shape, out_mesh);
            } else {
                unreachable!();
            }
        }
    }

    if options.debug_paint_clip_rects {
        clipped_primitives = add_clip_rects(&mut tessellator, tex_size, clipped_primitives);
    }

    if options.debug_ignore_clip_rects {
        for clipped_primitive in &mut clipped_primitives {
            clipped_primitive.clip_rect = Rect::EVERYTHING;
        }
    }

    for clipped_primitive in &clipped_primitives {
        if let Primitive::Mesh(mesh) = &clipped_primitive.primitive {
            crate::epaint_assert!(mesh.is_valid(), "Tessellator generated invalid Mesh");
        }
    }

    clipped_primitives
}

fn add_clip_rects(
    tessellator: &mut Tessellator,
    tex_size: [usize; 2],
    clipped_primitives: Vec<ClippedPrimitive>,
) -> Vec<ClippedPrimitive> {
    tessellator.clip_rect = Rect::EVERYTHING;
    let stroke = Stroke::new(2.0, Color32::from_rgb(150, 255, 150));

    clipped_primitives
        .into_iter()
        .flat_map(|clipped_primitive| {
            let mut clip_rect_mesh = Mesh::default();
            tessellator.tessellate_shape(
                tex_size,
                Shape::rect_stroke(clipped_primitive.clip_rect, 0.0, stroke),
                &mut clip_rect_mesh,
            );

            [
                clipped_primitive,
                ClippedPrimitive {
                    clip_rect: Rect::EVERYTHING, // whatever
                    primitive: Primitive::Mesh(clip_rect_mesh),
                },
            ]
        })
        .collect()
}
