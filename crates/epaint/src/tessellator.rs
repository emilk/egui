//! Converts graphics primitives into textured triangles.
//!
//! This module converts lines, circles, text and more represented by [`Shape`]
//! into textured triangles represented by [`Mesh`].

#![allow(clippy::identity_op)]

use crate::texture_atlas::PreparedDisc;
use crate::*;
use emath::*;

// ----------------------------------------------------------------------------

#[allow(clippy::approx_constant)]
mod precomputed_vertices {
    /*
    fn main() {
        let n = 64;
        println!("pub const CIRCLE_{}: [Vec2; {}] = [", n, n+1);
        for i in 0..=n {
            let a = std::f64::consts::TAU * i as f64 / n as f64;
            println!("    vec2({:.06}, {:.06}),", a.cos(), a.sin());
        }
        println!("];")
    }
    */

    use emath::{vec2, Vec2};

    pub const CIRCLE_8: [Vec2; 9] = [
        vec2(1.000000, 0.000000),
        vec2(0.707107, 0.707107),
        vec2(0.000000, 1.000000),
        vec2(-0.707107, 0.707107),
        vec2(-1.000000, 0.000000),
        vec2(-0.707107, -0.707107),
        vec2(0.000000, -1.000000),
        vec2(0.707107, -0.707107),
        vec2(1.000000, 0.000000),
    ];

    pub const CIRCLE_16: [Vec2; 17] = [
        vec2(1.000000, 0.000000),
        vec2(0.923880, 0.382683),
        vec2(0.707107, 0.707107),
        vec2(0.382683, 0.923880),
        vec2(0.000000, 1.000000),
        vec2(-0.382684, 0.923880),
        vec2(-0.707107, 0.707107),
        vec2(-0.923880, 0.382683),
        vec2(-1.000000, 0.000000),
        vec2(-0.923880, -0.382683),
        vec2(-0.707107, -0.707107),
        vec2(-0.382684, -0.923880),
        vec2(0.000000, -1.000000),
        vec2(0.382684, -0.923879),
        vec2(0.707107, -0.707107),
        vec2(0.923880, -0.382683),
        vec2(1.000000, 0.000000),
    ];

    pub const CIRCLE_32: [Vec2; 33] = [
        vec2(1.000000, 0.000000),
        vec2(0.980785, 0.195090),
        vec2(0.923880, 0.382683),
        vec2(0.831470, 0.555570),
        vec2(0.707107, 0.707107),
        vec2(0.555570, 0.831470),
        vec2(0.382683, 0.923880),
        vec2(0.195090, 0.980785),
        vec2(0.000000, 1.000000),
        vec2(-0.195090, 0.980785),
        vec2(-0.382683, 0.923880),
        vec2(-0.555570, 0.831470),
        vec2(-0.707107, 0.707107),
        vec2(-0.831470, 0.555570),
        vec2(-0.923880, 0.382683),
        vec2(-0.980785, 0.195090),
        vec2(-1.000000, 0.000000),
        vec2(-0.980785, -0.195090),
        vec2(-0.923880, -0.382683),
        vec2(-0.831470, -0.555570),
        vec2(-0.707107, -0.707107),
        vec2(-0.555570, -0.831470),
        vec2(-0.382683, -0.923880),
        vec2(-0.195090, -0.980785),
        vec2(-0.000000, -1.000000),
        vec2(0.195090, -0.980785),
        vec2(0.382683, -0.923880),
        vec2(0.555570, -0.831470),
        vec2(0.707107, -0.707107),
        vec2(0.831470, -0.555570),
        vec2(0.923880, -0.382683),
        vec2(0.980785, -0.195090),
        vec2(1.000000, -0.000000),
    ];

    pub const CIRCLE_64: [Vec2; 65] = [
        vec2(1.000000, 0.000000),
        vec2(0.995185, 0.098017),
        vec2(0.980785, 0.195090),
        vec2(0.956940, 0.290285),
        vec2(0.923880, 0.382683),
        vec2(0.881921, 0.471397),
        vec2(0.831470, 0.555570),
        vec2(0.773010, 0.634393),
        vec2(0.707107, 0.707107),
        vec2(0.634393, 0.773010),
        vec2(0.555570, 0.831470),
        vec2(0.471397, 0.881921),
        vec2(0.382683, 0.923880),
        vec2(0.290285, 0.956940),
        vec2(0.195090, 0.980785),
        vec2(0.098017, 0.995185),
        vec2(0.000000, 1.000000),
        vec2(-0.098017, 0.995185),
        vec2(-0.195090, 0.980785),
        vec2(-0.290285, 0.956940),
        vec2(-0.382683, 0.923880),
        vec2(-0.471397, 0.881921),
        vec2(-0.555570, 0.831470),
        vec2(-0.634393, 0.773010),
        vec2(-0.707107, 0.707107),
        vec2(-0.773010, 0.634393),
        vec2(-0.831470, 0.555570),
        vec2(-0.881921, 0.471397),
        vec2(-0.923880, 0.382683),
        vec2(-0.956940, 0.290285),
        vec2(-0.980785, 0.195090),
        vec2(-0.995185, 0.098017),
        vec2(-1.000000, 0.000000),
        vec2(-0.995185, -0.098017),
        vec2(-0.980785, -0.195090),
        vec2(-0.956940, -0.290285),
        vec2(-0.923880, -0.382683),
        vec2(-0.881921, -0.471397),
        vec2(-0.831470, -0.555570),
        vec2(-0.773010, -0.634393),
        vec2(-0.707107, -0.707107),
        vec2(-0.634393, -0.773010),
        vec2(-0.555570, -0.831470),
        vec2(-0.471397, -0.881921),
        vec2(-0.382683, -0.923880),
        vec2(-0.290285, -0.956940),
        vec2(-0.195090, -0.980785),
        vec2(-0.098017, -0.995185),
        vec2(-0.000000, -1.000000),
        vec2(0.098017, -0.995185),
        vec2(0.195090, -0.980785),
        vec2(0.290285, -0.956940),
        vec2(0.382683, -0.923880),
        vec2(0.471397, -0.881921),
        vec2(0.555570, -0.831470),
        vec2(0.634393, -0.773010),
        vec2(0.707107, -0.707107),
        vec2(0.773010, -0.634393),
        vec2(0.831470, -0.555570),
        vec2(0.881921, -0.471397),
        vec2(0.923880, -0.382683),
        vec2(0.956940, -0.290285),
        vec2(0.980785, -0.195090),
        vec2(0.995185, -0.098017),
        vec2(1.000000, -0.000000),
    ];

    pub const CIRCLE_128: [Vec2; 129] = [
        vec2(1.000000, 0.000000),
        vec2(0.998795, 0.049068),
        vec2(0.995185, 0.098017),
        vec2(0.989177, 0.146730),
        vec2(0.980785, 0.195090),
        vec2(0.970031, 0.242980),
        vec2(0.956940, 0.290285),
        vec2(0.941544, 0.336890),
        vec2(0.923880, 0.382683),
        vec2(0.903989, 0.427555),
        vec2(0.881921, 0.471397),
        vec2(0.857729, 0.514103),
        vec2(0.831470, 0.555570),
        vec2(0.803208, 0.595699),
        vec2(0.773010, 0.634393),
        vec2(0.740951, 0.671559),
        vec2(0.707107, 0.707107),
        vec2(0.671559, 0.740951),
        vec2(0.634393, 0.773010),
        vec2(0.595699, 0.803208),
        vec2(0.555570, 0.831470),
        vec2(0.514103, 0.857729),
        vec2(0.471397, 0.881921),
        vec2(0.427555, 0.903989),
        vec2(0.382683, 0.923880),
        vec2(0.336890, 0.941544),
        vec2(0.290285, 0.956940),
        vec2(0.242980, 0.970031),
        vec2(0.195090, 0.980785),
        vec2(0.146730, 0.989177),
        vec2(0.098017, 0.995185),
        vec2(0.049068, 0.998795),
        vec2(0.000000, 1.000000),
        vec2(-0.049068, 0.998795),
        vec2(-0.098017, 0.995185),
        vec2(-0.146730, 0.989177),
        vec2(-0.195090, 0.980785),
        vec2(-0.242980, 0.970031),
        vec2(-0.290285, 0.956940),
        vec2(-0.336890, 0.941544),
        vec2(-0.382683, 0.923880),
        vec2(-0.427555, 0.903989),
        vec2(-0.471397, 0.881921),
        vec2(-0.514103, 0.857729),
        vec2(-0.555570, 0.831470),
        vec2(-0.595699, 0.803208),
        vec2(-0.634393, 0.773010),
        vec2(-0.671559, 0.740951),
        vec2(-0.707107, 0.707107),
        vec2(-0.740951, 0.671559),
        vec2(-0.773010, 0.634393),
        vec2(-0.803208, 0.595699),
        vec2(-0.831470, 0.555570),
        vec2(-0.857729, 0.514103),
        vec2(-0.881921, 0.471397),
        vec2(-0.903989, 0.427555),
        vec2(-0.923880, 0.382683),
        vec2(-0.941544, 0.336890),
        vec2(-0.956940, 0.290285),
        vec2(-0.970031, 0.242980),
        vec2(-0.980785, 0.195090),
        vec2(-0.989177, 0.146730),
        vec2(-0.995185, 0.098017),
        vec2(-0.998795, 0.049068),
        vec2(-1.000000, 0.000000),
        vec2(-0.998795, -0.049068),
        vec2(-0.995185, -0.098017),
        vec2(-0.989177, -0.146730),
        vec2(-0.980785, -0.195090),
        vec2(-0.970031, -0.242980),
        vec2(-0.956940, -0.290285),
        vec2(-0.941544, -0.336890),
        vec2(-0.923880, -0.382683),
        vec2(-0.903989, -0.427555),
        vec2(-0.881921, -0.471397),
        vec2(-0.857729, -0.514103),
        vec2(-0.831470, -0.555570),
        vec2(-0.803208, -0.595699),
        vec2(-0.773010, -0.634393),
        vec2(-0.740951, -0.671559),
        vec2(-0.707107, -0.707107),
        vec2(-0.671559, -0.740951),
        vec2(-0.634393, -0.773010),
        vec2(-0.595699, -0.803208),
        vec2(-0.555570, -0.831470),
        vec2(-0.514103, -0.857729),
        vec2(-0.471397, -0.881921),
        vec2(-0.427555, -0.903989),
        vec2(-0.382683, -0.923880),
        vec2(-0.336890, -0.941544),
        vec2(-0.290285, -0.956940),
        vec2(-0.242980, -0.970031),
        vec2(-0.195090, -0.980785),
        vec2(-0.146730, -0.989177),
        vec2(-0.098017, -0.995185),
        vec2(-0.049068, -0.998795),
        vec2(-0.000000, -1.000000),
        vec2(0.049068, -0.998795),
        vec2(0.098017, -0.995185),
        vec2(0.146730, -0.989177),
        vec2(0.195090, -0.980785),
        vec2(0.242980, -0.970031),
        vec2(0.290285, -0.956940),
        vec2(0.336890, -0.941544),
        vec2(0.382683, -0.923880),
        vec2(0.427555, -0.903989),
        vec2(0.471397, -0.881921),
        vec2(0.514103, -0.857729),
        vec2(0.555570, -0.831470),
        vec2(0.595699, -0.803208),
        vec2(0.634393, -0.773010),
        vec2(0.671559, -0.740951),
        vec2(0.707107, -0.707107),
        vec2(0.740951, -0.671559),
        vec2(0.773010, -0.634393),
        vec2(0.803208, -0.595699),
        vec2(0.831470, -0.555570),
        vec2(0.857729, -0.514103),
        vec2(0.881921, -0.471397),
        vec2(0.903989, -0.427555),
        vec2(0.923880, -0.382683),
        vec2(0.941544, -0.336890),
        vec2(0.956940, -0.290285),
        vec2(0.970031, -0.242980),
        vec2(0.980785, -0.195090),
        vec2(0.989177, -0.146730),
        vec2(0.995185, -0.098017),
        vec2(0.998795, -0.049068),
        vec2(1.000000, -0.000000),
    ];
}

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
        use precomputed_vertices::*;

        // These cutoffs are based on a high-dpi display. TODO(emilk): use pixels_per_point here?
        // same cutoffs as in add_circle_quadrant

        if radius <= 2.0 {
            self.0.extend(CIRCLE_8.iter().map(|&n| PathPoint {
                pos: center + radius * n,
                normal: n,
            }));
        } else if radius <= 5.0 {
            self.0.extend(CIRCLE_16.iter().map(|&n| PathPoint {
                pos: center + radius * n,
                normal: n,
            }));
        } else if radius < 18.0 {
            self.0.extend(CIRCLE_32.iter().map(|&n| PathPoint {
                pos: center + radius * n,
                normal: n,
            }));
        } else if radius < 50.0 {
            self.0.extend(CIRCLE_64.iter().map(|&n| PathPoint {
                pos: center + radius * n,
                normal: n,
            }));
        } else {
            self.0.extend(CIRCLE_128.iter().map(|&n| PathPoint {
                pos: center + radius * n,
                normal: n,
            }));
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
    pub fn stroke_open(&self, feathering: f32, stroke: Stroke, out: &mut Mesh) {
        stroke_path(feathering, &self.0, PathType::Open, stroke, out);
    }

    /// A closed path (returning to the first point).
    pub fn stroke_closed(&self, feathering: f32, stroke: Stroke, out: &mut Mesh) {
        stroke_path(feathering, &self.0, PathType::Closed, stroke, out);
    }

    pub fn stroke(&self, feathering: f32, path_type: PathType, stroke: Stroke, out: &mut Mesh) {
        stroke_path(feathering, &self.0, path_type, stroke, out);
    }

    /// The path is taken to be closed (i.e. returning to the start again).
    ///
    /// Calling this may reverse the vertices in the path if they are wrong winding order.
    ///
    /// The preferred winding order is clockwise.
    pub fn fill(&mut self, feathering: f32, color: Color32, out: &mut Mesh) {
        fill_closed_path(feathering, &mut self.0, color, out);
    }
}

pub mod path {
    //! Helpers for constructing paths
    use crate::shape::Rounding;
    use emath::*;

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
        use super::precomputed_vertices::*;

        // These cutoffs are based on a high-dpi display. TODO(emilk): use pixels_per_point here?
        // same cutoffs as in add_circle

        if radius <= 0.0 {
            path.push(center);
        } else if radius <= 2.0 {
            let offset = quadrant as usize * 2;
            let quadrant_vertices = &CIRCLE_8[offset..=offset + 2];
            path.extend(quadrant_vertices.iter().map(|&n| center + radius * n));
        } else if radius <= 5.0 {
            let offset = quadrant as usize * 4;
            let quadrant_vertices = &CIRCLE_16[offset..=offset + 4];
            path.extend(quadrant_vertices.iter().map(|&n| center + radius * n));
        } else if radius < 18.0 {
            let offset = quadrant as usize * 8;
            let quadrant_vertices = &CIRCLE_32[offset..=offset + 8];
            path.extend(quadrant_vertices.iter().map(|&n| center + radius * n));
        } else if radius < 50.0 {
            let offset = quadrant as usize * 16;
            let quadrant_vertices = &CIRCLE_64[offset..=offset + 16];
            path.extend(quadrant_vertices.iter().map(|&n| center + radius * n));
        } else {
            let offset = quadrant as usize * 32;
            let quadrant_vertices = &CIRCLE_128[offset..=offset + 32];
            path.extend(quadrant_vertices.iter().map(|&n| center + radius * n));
        }
    }

    // Ensures the radius of each corner is within a valid range
    fn clamp_radius(rounding: Rounding, rect: Rect) -> Rounding {
        let half_width = rect.width() * 0.5;
        let half_height = rect.height() * 0.5;
        let max_cr = half_width.min(half_height);
        rounding.at_most(max_cr).at_least(0.0)
    }
}

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PathType {
    Open,
    Closed,
}

/// Tessellation quality options
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct TessellationOptions {
    /// Use "feathering" to smooth out the edges of shapes as a form of anti-aliasing.
    ///
    /// Feathering works by making each edge into a thin gradient into transparency.
    /// The size of this edge is controlled by [`Self::feathering_size_in_pixels`].
    ///
    /// This makes shapes appear smoother, but requires more triangles and is therefore slower.
    ///
    /// This setting does not affect text.
    ///
    /// Default: `true`.
    pub feathering: bool,

    /// The size of the the feathering, in physical pixels.
    ///
    /// The default, and suggested, value for this is `1.0`.
    /// If you use a larger value, edges will appear blurry.
    pub feathering_size_in_pixels: f32,

    /// If `true` (default) cull certain primitives before tessellating them.
    /// This likely makes
    pub coarse_tessellation_culling: bool,

    /// If `true`, small filled circled will be optimized by using pre-rasterized circled
    /// from the font atlas.
    pub prerasterized_discs: bool,

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
            feathering: true,
            feathering_size_in_pixels: 1.0,
            coarse_tessellation_culling: true,
            prerasterized_discs: true,
            round_text_to_pixels: true,
            debug_paint_text_rects: false,
            debug_paint_clip_rects: false,
            debug_ignore_clip_rects: false,
            bezier_tolerance: 0.1,
            epsilon: 1.0e-5,
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
fn fill_closed_path(feathering: f32, path: &mut [PathPoint], color: Color32, out: &mut Mesh) {
    if color == Color32::TRANSPARENT {
        return;
    }

    let n = path.len() as u32;
    if feathering > 0.0 {
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
            let dm = 0.5 * feathering * p1.normal;
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
    feathering: f32,
    path: &[PathPoint],
    path_type: PathType,
    stroke: Stroke,
    out: &mut Mesh,
) {
    let n = path.len() as u32;

    if stroke.width <= 0.0 || stroke.color == Color32::TRANSPARENT || n < 2 {
        return;
    }

    let idx = out.vertices.len() as u32;

    if feathering > 0.0 {
        let color_inner = stroke.color;
        let color_outer = Color32::TRANSPARENT;

        let thin_line = stroke.width <= feathering;
        if thin_line {
            /*
            We paint the line using three edges: outer, inner, outer.

            .       o   i   o      outer, inner, outer
            .       |---|          feathering (pixel width)
            */

            // Fade out as it gets thinner:
            let color_inner = mul_color(color_inner, stroke.width / feathering);
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
                out.colored_vertex(p + n * feathering, color_outer);
                out.colored_vertex(p, color_inner);
                out.colored_vertex(p - n * feathering, color_outer);

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
            .       |---|                  feathering (pixel width)
            .         |--------------|     width
            .       |---------|            outer_rad
            .           |-----|            inner_rad
            */

            let inner_rad = 0.5 * (stroke.width - feathering);
            let outer_rad = 0.5 * (stroke.width + feathering);

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
                    //   | \    added    / |  feathering
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
                        let back_extrude = n.rot90() * feathering;
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
                        let back_extrude = -n.rot90() * feathering;
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

        let thin_line = stroke.width <= feathering;
        if thin_line {
            // Fade out thin lines rather than making them thinner
            let radius = feathering / 2.0;
            let color = mul_color(stroke.color, stroke.width / feathering);
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
    // The fast gamma-space multiply also happens to be perceptually better.
    // Win-win!
    color.gamma_multiply(factor)
}

// ----------------------------------------------------------------------------

/// Converts [`Shape`]s into triangles ([`Mesh`]).
///
/// For performance reasons it is smart to reuse the same [`Tessellator`].
///
/// Se also [`tessellate_shapes`], a convenient wrapper around [`Tessellator`].
pub struct Tessellator {
    pixels_per_point: f32,
    options: TessellationOptions,
    font_tex_size: [usize; 2],

    /// See [`TextureAtlas::prepared_discs`].
    prepared_discs: Vec<PreparedDisc>,

    /// size of feathering in points. normally the size of a physical pixel. 0.0 if disabled
    feathering: f32,

    /// Only used for culling
    clip_rect: Rect,

    scratchpad_points: Vec<Pos2>,
    scratchpad_path: Path,
}

impl Tessellator {
    /// Create a new [`Tessellator`].
    ///
    /// * `font_tex_size`: size of the font texture. Required to normalize glyph uv rectangles when tessellating text.
    /// * `prepared_discs`: What [`TextureAtlas::prepared_discs`] returns. Can safely be set to an empty vec.
    pub fn new(
        pixels_per_point: f32,
        options: TessellationOptions,
        font_tex_size: [usize; 2],
        prepared_discs: Vec<PreparedDisc>,
    ) -> Self {
        let feathering = if options.feathering {
            let pixel_size = 1.0 / pixels_per_point;
            options.feathering_size_in_pixels * pixel_size
        } else {
            0.0
        };
        Self {
            pixels_per_point,
            options,
            font_tex_size,
            prepared_discs,
            feathering,
            clip_rect: Rect::EVERYTHING,
            scratchpad_points: Default::default(),
            scratchpad_path: Default::default(),
        }
    }

    /// Set the [`Rect`] to use for culling.
    pub fn set_clip_rect(&mut self, clip_rect: Rect) {
        self.clip_rect = clip_rect;
    }

    #[inline(always)]
    pub fn round_to_pixel(&self, point: f32) -> f32 {
        if self.options.round_text_to_pixels {
            (point * self.pixels_per_point).round() / self.pixels_per_point
        } else {
            point
        }
    }

    /// Tessellate a clipped shape into a list of primitives.
    pub fn tessellate_clipped_shape(
        &mut self,
        clipped_shape: ClippedShape,
        out_primitives: &mut Vec<ClippedPrimitive>,
    ) {
        let ClippedShape(new_clip_rect, new_shape) = clipped_shape;

        if !new_clip_rect.is_positive() {
            return; // skip empty clip rectangles
        }

        if let Shape::Vec(shapes) = new_shape {
            for shape in shapes {
                self.tessellate_clipped_shape(ClippedShape(new_clip_rect, shape), out_primitives);
            }
            return;
        }

        if let Shape::Callback(callback) = new_shape {
            out_primitives.push(ClippedPrimitive {
                clip_rect: new_clip_rect,
                primitive: Primitive::Callback(callback),
            });
            return;
        }

        let start_new_mesh = match out_primitives.last() {
            None => true,
            Some(output_clipped_primitive) => {
                output_clipped_primitive.clip_rect != new_clip_rect
                    || match &output_clipped_primitive.primitive {
                        Primitive::Mesh(output_mesh) => {
                            output_mesh.texture_id != new_shape.texture_id()
                        }
                        Primitive::Callback(_) => true,
                    }
            }
        };

        if start_new_mesh {
            out_primitives.push(ClippedPrimitive {
                clip_rect: new_clip_rect,
                primitive: Primitive::Mesh(Mesh::default()),
            });
        }

        let out = out_primitives.last_mut().unwrap();

        if let Primitive::Mesh(out_mesh) = &mut out.primitive {
            self.clip_rect = new_clip_rect;
            self.tessellate_shape(new_shape, out_mesh);
        } else {
            unreachable!();
        }
    }

    /// Tessellate a single [`Shape`] into a [`Mesh`].
    ///
    /// This call can panic the given shape is of [`Shape::Vec`] or [`Shape::Callback`].
    /// For that, use [`Self::tessellate_clipped_shape`] instead.
    /// * `shape`: the shape to tessellate.
    /// * `out`: triangles are appended to this.
    pub fn tessellate_shape(&mut self, shape: Shape, out: &mut Mesh) {
        match shape {
            Shape::Noop => {}
            Shape::Vec(vec) => {
                for shape in vec {
                    self.tessellate_shape(shape, out);
                }
            }
            Shape::Circle(circle) => {
                self.tessellate_circle(circle, out);
            }
            Shape::Mesh(mesh) => {
                if !mesh.is_valid() {
                    crate::epaint_assert!(false, "Invalid Mesh in Shape::Mesh");
                    return;
                }

                if self.options.coarse_tessellation_culling
                    && !self.clip_rect.intersects(mesh.calc_bounds())
                {
                    return;
                }
                out.append(mesh);
            }
            Shape::LineSegment { points, stroke } => self.tessellate_line(points, stroke, out),
            Shape::Path(path_shape) => {
                self.tessellate_path(&path_shape, out);
            }
            Shape::Rect(rect_shape) => {
                self.tessellate_rect(&rect_shape, out);
            }
            Shape::Text(text_shape) => {
                if self.options.debug_paint_text_rects {
                    let rect = text_shape.galley.rect.translate(text_shape.pos.to_vec2());
                    self.tessellate_rect(
                        &RectShape::stroke(rect.expand(0.5), 2.0, (0.5, Color32::GREEN)),
                        out,
                    );
                }
                self.tessellate_text(&text_shape, out);
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

    /// Tessellate a single [`CircleShape`] into a [`Mesh`].
    ///
    /// * `shape`: the circle to tessellate.
    /// * `out`: triangles are appended to this.
    pub fn tessellate_circle(&mut self, shape: CircleShape, out: &mut Mesh) {
        let CircleShape {
            center,
            radius,
            mut fill,
            stroke,
        } = shape;

        if radius <= 0.0 {
            return;
        }

        if self.options.coarse_tessellation_culling
            && !self
                .clip_rect
                .expand(radius + stroke.width)
                .contains(center)
        {
            return;
        }

        if self.options.prerasterized_discs && fill != Color32::TRANSPARENT {
            let radius_px = radius * self.pixels_per_point;
            // strike the right balance between some circles becoming too blurry, and some too sharp.
            let cutoff_radius = radius_px * 2.0_f32.powf(0.25);

            // Find the right disc radius for a crisp edge:
            // TODO(emilk): perhaps we can do something faster than this linear search.
            for disc in &self.prepared_discs {
                if cutoff_radius <= disc.r {
                    let side = radius_px * disc.w / (self.pixels_per_point * disc.r);
                    let rect = Rect::from_center_size(center, Vec2::splat(side));
                    out.add_rect_with_uv(rect, disc.uv, fill);

                    if stroke.is_empty() {
                        return; // we are done
                    } else {
                        // we still need to do the stroke
                        fill = Color32::TRANSPARENT; // don't fill again below
                        break;
                    }
                }
            }
        }

        self.scratchpad_path.clear();
        self.scratchpad_path.add_circle(center, radius);
        self.scratchpad_path.fill(self.feathering, fill, out);
        self.scratchpad_path
            .stroke_closed(self.feathering, stroke, out);
    }

    /// Tessellate a single [`Mesh`] into a [`Mesh`].
    ///
    /// * `mesh`: the mesh to tessellate.
    /// * `out`: triangles are appended to this.
    pub fn tessellate_mesh(&mut self, mesh: &Mesh, out: &mut Mesh) {
        if !mesh.is_valid() {
            crate::epaint_assert!(false, "Invalid Mesh in Shape::Mesh");
            return;
        }

        if self.options.coarse_tessellation_culling
            && !self.clip_rect.intersects(mesh.calc_bounds())
        {
            return;
        }

        out.append_ref(mesh);
    }

    /// Tessellate a line segment between the two points with the given stoken into a [`Mesh`].
    ///
    /// * `shape`: the mesh to tessellate.
    /// * `out`: triangles are appended to this.
    pub fn tessellate_line(&mut self, points: [Pos2; 2], stroke: Stroke, out: &mut Mesh) {
        if stroke.is_empty() {
            return;
        }

        if self.options.coarse_tessellation_culling
            && !self
                .clip_rect
                .intersects(Rect::from_two_pos(points[0], points[1]).expand(stroke.width))
        {
            return;
        }

        self.scratchpad_path.clear();
        self.scratchpad_path.add_line_segment(points);
        self.scratchpad_path
            .stroke_open(self.feathering, stroke, out);
    }

    /// Tessellate a single [`PathShape`] into a [`Mesh`].
    ///
    /// * `path_shape`: the path to tessellate.
    /// * `out`: triangles are appended to this.
    pub fn tessellate_path(&mut self, path_shape: &PathShape, out: &mut Mesh) {
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
        if *closed {
            self.scratchpad_path.add_line_loop(points);
        } else {
            self.scratchpad_path.add_open_points(points);
        }

        if *fill != Color32::TRANSPARENT {
            crate::epaint_assert!(
                closed,
                "You asked to fill a path that is not closed. That makes no sense."
            );
            self.scratchpad_path.fill(self.feathering, *fill, out);
        }
        let typ = if *closed {
            PathType::Closed
        } else {
            PathType::Open
        };
        self.scratchpad_path
            .stroke(self.feathering, typ, *stroke, out);
    }

    /// Tessellate a single [`Rect`] into a [`Mesh`].
    ///
    /// * `rect`: the rectangle to tessellate.
    /// * `out`: triangles are appended to this.
    pub fn tessellate_rect(&mut self, rect: &RectShape, out: &mut Mesh) {
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

        if rect.width() < self.feathering {
            // Very thin - approximate by a vertial line-segment:
            let line = [rect.center_top(), rect.center_bottom()];
            if fill != Color32::TRANSPARENT {
                self.tessellate_line(line, Stroke::new(rect.width(), fill), out);
            }
            if !stroke.is_empty() {
                self.tessellate_line(line, stroke, out); // back…
                self.tessellate_line(line, stroke, out); // …and forth
            }
        } else if rect.height() < self.feathering {
            // Very thin - approximate by a horizontal line-segment:
            let line = [rect.left_center(), rect.right_center()];
            if fill != Color32::TRANSPARENT {
                self.tessellate_line(line, Stroke::new(rect.height(), fill), out);
            }
            if !stroke.is_empty() {
                self.tessellate_line(line, stroke, out); // back…
                self.tessellate_line(line, stroke, out); // …and forth
            }
        } else {
            let path = &mut self.scratchpad_path;
            path.clear();
            path::rounded_rectangle(&mut self.scratchpad_points, rect, rounding);
            path.add_line_loop(&self.scratchpad_points);
            path.fill(self.feathering, fill, out);
            path.stroke_closed(self.feathering, stroke, out);
        }
    }

    /// Tessellate a single [`TextShape`] into a [`Mesh`].
    /// * `text_shape`: the text to tessellate.
    /// * `out`: triangles are appended to this.
    pub fn tessellate_text(&mut self, text_shape: &TextShape, out: &mut Mesh) {
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

        if galley.pixels_per_point != self.pixels_per_point {
            eprintln!("epaint: WARNING: pixels_per_point (dpi scale) have changed between text layout and tessellation. \
                       You must recreate your text shapes if pixels_per_point changes.");
        }

        out.vertices.reserve(galley.num_vertices);
        out.indices.reserve(galley.num_indices);

        // The contents of the galley is already snapped to pixel coordinates,
        // but we need to make sure the galley ends up on the start of a physical pixel:
        let galley_pos = pos2(
            self.round_to_pixel(galley_pos.x),
            self.round_to_pixel(galley_pos.y),
        );

        let uv_normalizer = vec2(
            1.0 / self.font_tex_size[0] as f32,
            1.0 / self.font_tex_size[1] as f32,
        );

        let rotator = Rot2::from_angle(*angle);

        for row in &galley.rows {
            if row.visuals.mesh.is_empty() {
                continue;
            }

            let mut row_rect = row.visuals.mesh_bounds;
            if *angle != 0.0 {
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
                                color = *override_text_color;
                            }
                        }

                        let offset = if *angle == 0.0 {
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

            if *underline != Stroke::NONE {
                self.scratchpad_path.clear();
                self.scratchpad_path
                    .add_line_segment([row_rect.left_bottom(), row_rect.right_bottom()]);
                self.scratchpad_path
                    .stroke_open(self.feathering, *underline, out);
            }
        }
    }

    /// Tessellate a single [`QuadraticBezierShape`] into a [`Mesh`].
    ///
    /// * `quadratic_shape`: the shape to tessellate.
    /// * `out`: triangles are appended to this.
    pub fn tessellate_quadratic_bezier(
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

    /// Tessellate a single [`CubicBezierShape`] into a [`Mesh`].
    ///
    /// * `cubic_shape`: the shape to tessellate.
    /// * `out`: triangles are appended to this.
    pub fn tessellate_cubic_bezier(&mut self, cubic_shape: CubicBezierShape, out: &mut Mesh) {
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
        if points.len() < 2 {
            return;
        }

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
            self.scratchpad_path.fill(self.feathering, fill, out);
        }
        let typ = if closed {
            PathType::Closed
        } else {
            PathType::Open
        };
        self.scratchpad_path
            .stroke(self.feathering, typ, stroke, out);
    }
}

/// Turns [`Shape`]:s into sets of triangles.
///
/// The given shapes will tessellated in the same order as they are given.
/// They will be batched together by clip rectangle.
///
/// * `pixels_per_point`: number of physical pixels to each logical point
/// * `options`: tessellation quality
/// * `shapes`: what to tessellate
/// * `font_tex_size`: size of the font texture. Required to normalize glyph uv rectangles when tessellating text.
/// * `prepared_discs`: What [`TextureAtlas::prepared_discs`] returns. Can safely be set to an empty vec.
///
/// The implementation uses a [`Tessellator`].
///
/// ## Returns
/// A list of clip rectangles with matching [`Mesh`].
pub fn tessellate_shapes(
    pixels_per_point: f32,
    options: TessellationOptions,
    font_tex_size: [usize; 2],
    prepared_discs: Vec<PreparedDisc>,
    shapes: Vec<ClippedShape>,
) -> Vec<ClippedPrimitive> {
    let mut tessellator =
        Tessellator::new(pixels_per_point, options, font_tex_size, prepared_discs);

    let mut clipped_primitives: Vec<ClippedPrimitive> = Vec::default();

    for clipped_shape in shapes {
        tessellator.tessellate_clipped_shape(clipped_shape, &mut clipped_primitives);
    }

    if options.debug_paint_clip_rects {
        clipped_primitives = add_clip_rects(&mut tessellator, clipped_primitives);
    }

    if options.debug_ignore_clip_rects {
        for clipped_primitive in &mut clipped_primitives {
            clipped_primitive.clip_rect = Rect::EVERYTHING;
        }
    }

    clipped_primitives.retain(|p| {
        p.clip_rect.is_positive()
            && match &p.primitive {
                Primitive::Mesh(mesh) => !mesh.is_empty(),
                Primitive::Callback(_) => true,
            }
    });

    for clipped_primitive in &clipped_primitives {
        if let Primitive::Mesh(mesh) = &clipped_primitive.primitive {
            crate::epaint_assert!(mesh.is_valid(), "Tessellator generated invalid Mesh");
        }
    }

    clipped_primitives
}

fn add_clip_rects(
    tessellator: &mut Tessellator,
    clipped_primitives: Vec<ClippedPrimitive>,
) -> Vec<ClippedPrimitive> {
    tessellator.clip_rect = Rect::EVERYTHING;
    let stroke = Stroke::new(2.0, Color32::from_rgb(150, 255, 150));

    clipped_primitives
        .into_iter()
        .flat_map(|clipped_primitive| {
            let mut clip_rect_mesh = Mesh::default();
            tessellator.tessellate_shape(
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

#[test]
fn test_tessellator() {
    use crate::*;

    let mut shapes = Vec::with_capacity(2);

    let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));

    let mut mesh = Mesh::with_texture(TextureId::Managed(1));
    mesh.add_rect_with_uv(rect, uv, Color32::WHITE);
    shapes.push(Shape::mesh(mesh));

    let mut mesh = Mesh::with_texture(TextureId::Managed(2));
    mesh.add_rect_with_uv(rect, uv, Color32::WHITE);
    shapes.push(Shape::mesh(mesh));

    let shape = Shape::Vec(shapes);
    let clipped_shapes = vec![ClippedShape(rect, shape)];

    let font_tex_size = [1024, 1024]; // unused
    let prepared_discs = vec![]; // unused

    let primitives = tessellate_shapes(
        1.0,
        Default::default(),
        font_tex_size,
        prepared_discs,
        clipped_shapes,
    );
    assert_eq!(primitives.len(), 2);
}
