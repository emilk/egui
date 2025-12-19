//! Converts graphics primitives into textured triangles.
//!
//! This module converts lines, circles, text and more represented by [`Shape`]
//! into textured triangles represented by [`Mesh`].

#![expect(clippy::identity_op)]

use emath::{GuiRounding as _, NumExt as _, Pos2, Rect, Rot2, Vec2, pos2, remap, vec2};

use crate::{
    CircleShape, ClippedPrimitive, ClippedShape, Color32, CornerRadiusF32, CubicBezierShape,
    EllipseShape, Mesh, PathShape, Primitive, QuadraticBezierShape, RectShape, Shape, Stroke,
    StrokeKind, TextShape, TextureId, Vertex, WHITE_UV, color::ColorMode, emath,
    stroke::PathStroke, texture_atlas::PreparedDisc,
};

// ----------------------------------------------------------------------------

#[expect(clippy::approx_constant)]
mod precomputed_vertices {
    // fn main() {
    //     let n = 64;
    //     println!("pub const CIRCLE_{}: [Vec2; {}] = [", n, n+1);
    //     for i in 0..=n {
    //         let a = std::f64::consts::TAU * i as f64 / n as f64;
    //         println!("    vec2({:.06}, {:.06}),", a.cos(), a.sin());
    //     }
    //     println!("];")
    // }

    use emath::{Vec2, vec2};

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

#[derive(Clone, Copy, Debug, Default, PartialEq)]
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
        use precomputed_vertices::{CIRCLE_8, CIRCLE_16, CIRCLE_32, CIRCLE_64, CIRCLE_128};

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
        assert!(n >= 2, "A path needs at least two points, but got {n}");

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
        assert!(n >= 2, "A path needs at least two points, but got {n}");
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

    /// The path is taken to be closed (i.e. returning to the start again).
    ///
    /// Calling this may reverse the vertices in the path if they are wrong winding order.
    /// The preferred winding order is clockwise.
    pub fn fill_and_stroke(
        &mut self,
        feathering: f32,
        fill: Color32,
        stroke: &PathStroke,
        out: &mut Mesh,
    ) {
        stroke_and_fill_path(feathering, &mut self.0, PathType::Closed, stroke, fill, out);
    }

    /// Open-ended.
    pub fn stroke_open(&mut self, feathering: f32, stroke: &PathStroke, out: &mut Mesh) {
        stroke_path(feathering, &mut self.0, PathType::Open, stroke, out);
    }

    /// A closed path (returning to the first point).
    pub fn stroke_closed(&mut self, feathering: f32, stroke: &PathStroke, out: &mut Mesh) {
        stroke_path(feathering, &mut self.0, PathType::Closed, stroke, out);
    }

    pub fn stroke(
        &mut self,
        feathering: f32,
        path_type: PathType,
        stroke: &PathStroke,
        out: &mut Mesh,
    ) {
        stroke_path(feathering, &mut self.0, path_type, stroke, out);
    }

    /// The path is taken to be closed (i.e. returning to the start again).
    ///
    /// Calling this may reverse the vertices in the path if they are wrong winding order.
    /// The preferred winding order is clockwise.
    pub fn fill(&mut self, feathering: f32, color: Color32, out: &mut Mesh) {
        fill_closed_path(feathering, &mut self.0, color, out);
    }

    /// Like [`Self::fill`] but with texturing.
    ///
    /// The `uv_from_pos` is called for each vertex position.
    pub fn fill_with_uv(
        &mut self,
        feathering: f32,
        color: Color32,
        texture_id: TextureId,
        uv_from_pos: impl Fn(Pos2) -> Pos2,
        out: &mut Mesh,
    ) {
        fill_closed_path_with_uv(feathering, &mut self.0, color, texture_id, uv_from_pos, out);
    }
}

pub mod path {
    //! Helpers for constructing paths
    use crate::CornerRadiusF32;
    use emath::{Pos2, Rect, pos2};

    /// overwrites existing points
    pub fn rounded_rectangle(path: &mut Vec<Pos2>, rect: Rect, cr: CornerRadiusF32) {
        path.clear();

        let min = rect.min;
        let max = rect.max;

        let cr = clamp_corner_radius(cr, rect);

        if cr == CornerRadiusF32::ZERO {
            path.reserve(4);
            path.push(pos2(min.x, min.y)); // left top
            path.push(pos2(max.x, min.y)); // right top
            path.push(pos2(max.x, max.y)); // right bottom
            path.push(pos2(min.x, max.y)); // left bottom
        } else {
            // We need to avoid duplicated vertices, because that leads to visual artifacts later.
            // Duplicated vertices can happen when one side is all rounding, with no straight edge between.
            let eps = f32::EPSILON * rect.size().max_elem();

            add_circle_quadrant(path, pos2(max.x - cr.se, max.y - cr.se), cr.se, 0.0); // south east

            if rect.width() <= cr.se + cr.sw + eps {
                path.pop(); // avoid duplicated vertex
            }

            add_circle_quadrant(path, pos2(min.x + cr.sw, max.y - cr.sw), cr.sw, 1.0); // south west

            if rect.height() <= cr.sw + cr.nw + eps {
                path.pop(); // avoid duplicated vertex
            }

            add_circle_quadrant(path, pos2(min.x + cr.nw, min.y + cr.nw), cr.nw, 2.0); // north west

            if rect.width() <= cr.nw + cr.ne + eps {
                path.pop(); // avoid duplicated vertex
            }

            add_circle_quadrant(path, pos2(max.x - cr.ne, min.y + cr.ne), cr.ne, 3.0); // north east

            if rect.height() <= cr.ne + cr.se + eps {
                path.pop(); // avoid duplicated vertex
            }
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
        use super::precomputed_vertices::{CIRCLE_8, CIRCLE_16, CIRCLE_32, CIRCLE_64, CIRCLE_128};

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
    fn clamp_corner_radius(cr: CornerRadiusF32, rect: Rect) -> CornerRadiusF32 {
        let half_width = rect.width() * 0.5;
        let half_height = rect.height() * 0.5;
        let max_cr = half_width.min(half_height);
        cr.at_most(max_cr).at_least(0.0)
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

    /// The size of the feathering, in physical pixels.
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

    /// If `true` (default) align text to the physical pixel grid.
    /// This makes the text sharper on most platforms.
    pub round_text_to_pixels: bool,

    /// If `true` (default), align right-angled line segments to the physical pixel grid.
    ///
    /// This makes the line segments appear crisp on any display.
    pub round_line_segments_to_pixels: bool,

    /// If `true` (default), align rectangles to the physical pixel grid.
    ///
    /// This makes the rectangle strokes more crisp,
    /// and makes filled rectangles tile perfectly (without feathering).
    ///
    /// You can override this with [`crate::RectShape::round_to_pixels`].
    pub round_rects_to_pixels: bool,

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

    /// If `rayon` feature is activated, should we parallelize tessellation?
    pub parallel_tessellation: bool,

    /// If `true`, invalid meshes will be silently ignored.
    /// If `false`, invalid meshes will cause a panic.
    ///
    /// The default is `false` to save performance.
    pub validate_meshes: bool,
}

impl Default for TessellationOptions {
    fn default() -> Self {
        Self {
            feathering: true,
            feathering_size_in_pixels: 1.0,
            coarse_tessellation_culling: true,
            prerasterized_discs: true,
            round_text_to_pixels: true,
            round_line_segments_to_pixels: true,
            round_rects_to_pixels: true,
            debug_paint_text_rects: false,
            debug_paint_clip_rects: false,
            debug_ignore_clip_rects: false,
            bezier_tolerance: 0.1,
            epsilon: 1.0e-5,
            parallel_tessellation: true,
            validate_meshes: false,
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
fn fill_closed_path(feathering: f32, path: &mut [PathPoint], fill_color: Color32, out: &mut Mesh) {
    if fill_color == Color32::TRANSPARENT {
        return;
    }

    let n = path.len() as u32;
    if n < 3 {
        return;
    }

    if 0.0 < feathering {
        if cw_signed_area(path) < 0.0 {
            // Wrong winding order - fix:
            path.reverse();
            for point in &mut *path {
                point.normal = -point.normal;
            }
        }

        out.reserve_triangles(3 * n as usize);
        out.reserve_vertices(2 * n as usize);
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

            let pos_inner = p1.pos - dm;
            let pos_outer = p1.pos + dm;

            out.colored_vertex(pos_inner, fill_color);
            out.colored_vertex(pos_outer, Color32::TRANSPARENT);
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
            color: fill_color,
        }));
        for i in 2..n {
            out.add_triangle(idx, idx + i - 1, idx + i);
        }
    }
}

/// Like [`fill_closed_path`] but with texturing.
///
/// The `uv_from_pos` is called for each vertex position.
fn fill_closed_path_with_uv(
    feathering: f32,
    path: &mut [PathPoint],
    color: Color32,
    texture_id: TextureId,
    uv_from_pos: impl Fn(Pos2) -> Pos2,
    out: &mut Mesh,
) {
    if color == Color32::TRANSPARENT {
        return;
    }

    if out.is_empty() {
        out.texture_id = texture_id;
    } else {
        assert_eq!(
            out.texture_id, texture_id,
            "Mixing different `texture_id` in the same "
        );
    }

    let n = path.len() as u32;
    if 0.0 < feathering {
        if cw_signed_area(path) < 0.0 {
            // Wrong winding order - fix:
            path.reverse();
            for point in &mut *path {
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

            let pos = p1.pos - dm;
            out.vertices.push(Vertex {
                pos,
                uv: uv_from_pos(pos),
                color,
            });

            let pos = p1.pos + dm;
            out.vertices.push(Vertex {
                pos,
                uv: uv_from_pos(pos),
                color: color_outer,
            });

            out.add_triangle(idx_inner + i1 * 2, idx_inner + i0 * 2, idx_outer + 2 * i0);
            out.add_triangle(idx_outer + i0 * 2, idx_outer + i1 * 2, idx_inner + 2 * i1);
            i0 = i1;
        }
    } else {
        out.reserve_triangles(n as usize);
        let idx = out.vertices.len() as u32;
        out.vertices.extend(path.iter().map(|p| Vertex {
            pos: p.pos,
            uv: uv_from_pos(p.pos),
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
    path: &mut [PathPoint],
    path_type: PathType,
    stroke: &PathStroke,
    out: &mut Mesh,
) {
    let fill = Color32::TRANSPARENT;
    stroke_and_fill_path(feathering, path, path_type, stroke, fill, out);
}

/// Tessellate the given path as a stroke with thickness, with optional fill color.
///
/// Calling this may reverse the vertices in the path if they are wrong winding order.
///
/// The preferred winding order is clockwise.
fn stroke_and_fill_path(
    feathering: f32,
    path: &mut [PathPoint],
    path_type: PathType,
    stroke: &PathStroke,
    color_fill: Color32,
    out: &mut Mesh,
) {
    let n = path.len() as u32;

    if n < 2 {
        return;
    }

    if stroke.width == 0.0 {
        // Skip the stroke, just fill.
        return fill_closed_path(feathering, path, color_fill, out);
    }

    if color_fill != Color32::TRANSPARENT && cw_signed_area(path) < 0.0 {
        // Wrong winding order - fix:
        path.reverse();
        for point in &mut *path {
            point.normal = -point.normal;
        }
    }

    if stroke.color == ColorMode::TRANSPARENT {
        // Skip the stroke, just fill. But subtract the width from the path:
        match stroke.kind {
            StrokeKind::Inside => {
                for point in &mut *path {
                    point.pos -= stroke.width * point.normal;
                }
            }
            StrokeKind::Middle => {
                for point in &mut *path {
                    point.pos -= 0.5 * stroke.width * point.normal;
                }
            }
            StrokeKind::Outside => {}
        }

        // Skip the stroke, just fill.
        return fill_closed_path(feathering, path, color_fill, out);
    }

    let idx = out.vertices.len() as u32;

    // Move the points so that the stroke is on middle of the path.
    match stroke.kind {
        StrokeKind::Inside => {
            for point in &mut *path {
                point.pos -= 0.5 * stroke.width * point.normal;
            }
        }
        StrokeKind::Middle => {
            // correct
        }
        StrokeKind::Outside => {
            for point in &mut *path {
                point.pos += 0.5 * stroke.width * point.normal;
            }
        }
    }

    // Expand the bounding box to include the thickness of the path
    let uv_bbox = if matches!(stroke.color, ColorMode::UV(_)) {
        Rect::from_points(&path.iter().map(|p| p.pos).collect::<Vec<Pos2>>())
            .expand((stroke.width / 2.0) + feathering)
    } else {
        Rect::NAN
    };
    let get_color = |col: &ColorMode, pos: Pos2| match col {
        ColorMode::Solid(col) => *col,
        ColorMode::UV(fun) => fun(uv_bbox, pos),
    };

    if 0.0 < feathering {
        let color_outer = Color32::TRANSPARENT;
        let color_middle = &stroke.color;

        // We add a bit of an epsilon here, because when we round to pixels,
        // we can get rounding errors (unless pixels_per_point is an integer).
        // And it's better to err on the side of the nicer rendering with line caps
        // (the thin-line optimization has no line caps).
        let thin_line = stroke.width <= 0.9 * feathering;
        if thin_line {
            // If the stroke is painted smaller than the pixel width (=feathering width),
            // then we risk severe aliasing.
            // Instead, we paint the stroke as a triangular ridge, two feather-widths wide,
            // and lessen the opacity of the middle part instead of making it thinner.
            if color_fill != Color32::TRANSPARENT && stroke.width < feathering {
                // If this is filled shape, then we need to also compensate so that the
                // filled area remains the same as it would have been without the
                // artificially wide line.
                for point in &mut *path {
                    point.pos += 0.5 * (feathering - stroke.width) * point.normal;
                }
            }

            // TODO(emilk): add line caps (if this is an open line).

            let opacity = stroke.width / feathering;

            /*
            We paint the line using three edges: outer, middle, fill.

            .       o   m   i      outer, middle, fill
            .       |---|          feathering (pixel width)
            */

            out.reserve_triangles(4 * n as usize);
            out.reserve_vertices(3 * n as usize);

            let mut i0 = n - 1;
            for i1 in 0..n {
                let connect_with_previous = path_type == PathType::Closed || i1 > 0;
                let p1 = path[i1 as usize];
                let p = p1.pos;
                let n = p1.normal;
                out.colored_vertex(p + n * feathering, color_outer);
                out.colored_vertex(p, mul_color(get_color(color_middle, p), opacity));
                out.colored_vertex(p - n * feathering, color_fill);

                if connect_with_previous {
                    out.add_triangle(idx + 3 * i0 + 0, idx + 3 * i0 + 1, idx + 3 * i1 + 0);
                    out.add_triangle(idx + 3 * i0 + 1, idx + 3 * i1 + 0, idx + 3 * i1 + 1);

                    out.add_triangle(idx + 3 * i0 + 1, idx + 3 * i0 + 2, idx + 3 * i1 + 1);
                    out.add_triangle(idx + 3 * i0 + 2, idx + 3 * i1 + 1, idx + 3 * i1 + 2);
                }

                i0 = i1;
            }

            if color_fill != Color32::TRANSPARENT {
                out.reserve_triangles(n as usize - 2);
                let idx_fill = idx + 2;
                for i in 2..n {
                    out.add_triangle(idx_fill + 3 * (i - 1), idx_fill, idx_fill + 3 * i);
                }
            }
        } else {
            // thick anti-aliased line

            /*
            We paint the line using four edges: outer, middle, middle, fill

            .       o   m     p    m   f   outer, middle, point, middle, fill
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
                        let p1 = path[i1 as usize];
                        let p = p1.pos;
                        let n = p1.normal;
                        out.colored_vertex(p + n * outer_rad, color_outer);
                        out.colored_vertex(
                            p + n * inner_rad,
                            get_color(color_middle, p + n * inner_rad),
                        );
                        out.colored_vertex(
                            p - n * inner_rad,
                            get_color(color_middle, p - n * inner_rad),
                        );
                        out.colored_vertex(p - n * outer_rad, color_fill);

                        out.add_triangle(idx + 4 * i0 + 0, idx + 4 * i0 + 1, idx + 4 * i1 + 0);
                        out.add_triangle(idx + 4 * i0 + 1, idx + 4 * i1 + 0, idx + 4 * i1 + 1);

                        out.add_triangle(idx + 4 * i0 + 1, idx + 4 * i0 + 2, idx + 4 * i1 + 1);
                        out.add_triangle(idx + 4 * i0 + 2, idx + 4 * i1 + 1, idx + 4 * i1 + 2);

                        out.add_triangle(idx + 4 * i0 + 2, idx + 4 * i0 + 3, idx + 4 * i1 + 2);
                        out.add_triangle(idx + 4 * i0 + 3, idx + 4 * i1 + 2, idx + 4 * i1 + 3);

                        i0 = i1;
                    }

                    if color_fill != Color32::TRANSPARENT {
                        out.reserve_triangles(n as usize - 2);
                        let idx_fill = idx + 3;
                        for i in 2..n {
                            out.add_triangle(idx_fill + 4 * (i - 1), idx_fill, idx_fill + 4 * i);
                        }
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

                    // TODO(emilk): we should probably shrink before adding the line caps,
                    // so that we don't add to the area of the line.
                    // TODO(emilk): make line caps optional.

                    out.reserve_triangles(6 * n as usize + 4);
                    out.reserve_vertices(4 * n as usize);

                    {
                        let end = path[0];
                        let p = end.pos;
                        let n = end.normal;
                        let back_extrude = n.rot90() * feathering;
                        out.colored_vertex(p + n * outer_rad + back_extrude, color_outer);
                        out.colored_vertex(
                            p + n * inner_rad,
                            get_color(color_middle, p + n * inner_rad),
                        );
                        out.colored_vertex(
                            p - n * inner_rad,
                            get_color(color_middle, p - n * inner_rad),
                        );
                        out.colored_vertex(p - n * outer_rad + back_extrude, color_outer);

                        out.add_triangle(idx + 0, idx + 1, idx + 2);
                        out.add_triangle(idx + 0, idx + 2, idx + 3);
                    }

                    let mut i0 = 0;
                    for i1 in 1..n - 1 {
                        let point = path[i1 as usize];
                        let p = point.pos;
                        let n = point.normal;
                        out.colored_vertex(p + n * outer_rad, color_outer);
                        out.colored_vertex(
                            p + n * inner_rad,
                            get_color(color_middle, p + n * inner_rad),
                        );
                        out.colored_vertex(
                            p - n * inner_rad,
                            get_color(color_middle, p - n * inner_rad),
                        );
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
                        let end = path[i1 as usize];
                        let p = end.pos;
                        let n = end.normal;
                        let back_extrude = -n.rot90() * feathering;
                        out.colored_vertex(p + n * outer_rad + back_extrude, color_outer);
                        out.colored_vertex(
                            p + n * inner_rad,
                            get_color(color_middle, p + n * inner_rad),
                        );
                        out.colored_vertex(
                            p - n * inner_rad,
                            get_color(color_middle, p - n * inner_rad),
                        );
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
            let opacity = stroke.width / feathering;
            let radius = feathering / 2.0;
            for p in path.iter_mut() {
                out.colored_vertex(
                    p.pos + radius * p.normal,
                    mul_color(get_color(&stroke.color, p.pos + radius * p.normal), opacity),
                );
                out.colored_vertex(
                    p.pos - radius * p.normal,
                    mul_color(get_color(&stroke.color, p.pos - radius * p.normal), opacity),
                );
            }
        } else {
            let radius = stroke.width / 2.0;
            for p in path.iter_mut() {
                out.colored_vertex(
                    p.pos + radius * p.normal,
                    get_color(&stroke.color, p.pos + radius * p.normal),
                );
                out.colored_vertex(
                    p.pos - radius * p.normal,
                    get_color(&stroke.color, p.pos - radius * p.normal),
                );
            }
        }

        if color_fill != Color32::TRANSPARENT {
            // We Need to create new vertices, because the ones we used for the stroke
            // has the wrong color.

            // Shrink to ignore the stroke…
            for point in &mut *path {
                point.pos -= 0.5 * stroke.width * point.normal;
            }
            // …then fill:
            fill_closed_path(feathering, path, color_fill, out);
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
#[derive(Clone)]
pub struct Tessellator {
    pixels_per_point: f32,
    options: TessellationOptions,
    font_tex_size: [usize; 2],

    /// See [`crate::TextureAtlas::prepared_discs`].
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
    /// * `pixels_per_point`: number of physical pixels to each logical point
    /// * `options`: tessellation quality
    /// * `shapes`: what to tessellate
    /// * `font_tex_size`: size of the font texture. Required to normalize glyph uv rectangles when tessellating text.
    /// * `prepared_discs`: What [`crate::TextureAtlas::prepared_discs`] returns. Can safely be set to an empty vec.
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

    /// Tessellate a clipped shape into a list of primitives.
    pub fn tessellate_clipped_shape(
        &mut self,
        clipped_shape: ClippedShape,
        out_primitives: &mut Vec<ClippedPrimitive>,
    ) {
        let ClippedShape { clip_rect, shape } = clipped_shape;

        if !clip_rect.is_positive() {
            return; // skip empty clip rectangles
        }

        if let Shape::Vec(shapes) = shape {
            for shape in shapes {
                self.tessellate_clipped_shape(ClippedShape { clip_rect, shape }, out_primitives);
            }
            return;
        }

        if let Shape::Callback(callback) = shape {
            out_primitives.push(ClippedPrimitive {
                clip_rect,
                primitive: Primitive::Callback(callback),
            });
            return;
        }

        let start_new_mesh = match out_primitives.last() {
            None => true,
            Some(output_clipped_primitive) => {
                output_clipped_primitive.clip_rect != clip_rect
                    || match &output_clipped_primitive.primitive {
                        Primitive::Mesh(output_mesh) => {
                            output_mesh.texture_id != shape.texture_id()
                        }
                        Primitive::Callback(_) => true,
                    }
            }
        };

        if start_new_mesh {
            out_primitives.push(ClippedPrimitive {
                clip_rect,
                primitive: Primitive::Mesh(Mesh::default()),
            });
        }

        #[expect(clippy::unwrap_used)] // it's never empty
        let out = out_primitives.last_mut().unwrap();

        if let Primitive::Mesh(out_mesh) = &mut out.primitive {
            self.clip_rect = clip_rect;
            self.tessellate_shape(shape, out_mesh);
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
            Shape::Ellipse(ellipse) => {
                self.tessellate_ellipse(ellipse, out);
            }
            Shape::Mesh(mesh) => {
                profiling::scope!("mesh");

                if self.options.validate_meshes && !mesh.is_valid() {
                    debug_assert!(false, "Invalid Mesh in Shape::Mesh");
                    return;
                }
                // note: `append` still checks if the mesh is valid if extra asserts are enabled.

                if self.options.coarse_tessellation_culling
                    && !self.clip_rect.intersects(mesh.calc_bounds())
                {
                    return;
                }

                out.append_ref(&mesh);
            }
            Shape::LineSegment { points, stroke } => {
                self.tessellate_line_segment(points, stroke, out);
            }
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
                        &RectShape::stroke(rect, 2.0, (0.5, Color32::GREEN), StrokeKind::Outside),
                        out,
                    );
                }
                self.tessellate_text(&text_shape, out);
            }
            Shape::QuadraticBezier(quadratic_shape) => {
                self.tessellate_quadratic_bezier(&quadratic_shape, out);
            }
            Shape::CubicBezier(cubic_shape) => self.tessellate_cubic_bezier(&cubic_shape, out),
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

        let path_stroke = PathStroke::from(stroke).outside();
        self.scratchpad_path.clear();
        self.scratchpad_path.add_circle(center, radius);
        self.scratchpad_path
            .fill_and_stroke(self.feathering, fill, &path_stroke, out);
    }

    /// Tessellate a single [`EllipseShape`] into a [`Mesh`].
    ///
    /// * `shape`: the ellipse to tessellate.
    /// * `out`: triangles are appended to this.
    pub fn tessellate_ellipse(&mut self, shape: EllipseShape, out: &mut Mesh) {
        let EllipseShape {
            center,
            radius,
            fill,
            stroke,
        } = shape;

        if radius.x <= 0.0 || radius.y <= 0.0 {
            return;
        }

        if self.options.coarse_tessellation_culling
            && !self
                .clip_rect
                .expand2(radius + Vec2::splat(stroke.width))
                .contains(center)
        {
            return;
        }

        // Get the max pixel radius
        let max_radius = (radius.max_elem() * self.pixels_per_point) as u32;

        // Ensure there is at least 8 points in each quarter of the ellipse
        let num_points = u32::max(8, max_radius / 16);

        // Create an ease ratio based the ellipses a and b
        let ratio = ((radius.y / radius.x) / 2.0).clamp(0.0, 1.0);

        // Generate points between the 0 to pi/2
        let quarter: Vec<Vec2> = (1..num_points)
            .map(|i| {
                let percent = i as f32 / num_points as f32;

                // Ease the percent value, concentrating points around tight bends
                let eased = 2.0 * (percent - percent.powf(2.0)) * ratio + percent.powf(2.0);

                // Scale the ease to the quarter
                let t = eased * std::f32::consts::FRAC_PI_2;
                Vec2::new(radius.x * f32::cos(t), radius.y * f32::sin(t))
            })
            .collect();

        // Build the ellipse from the 4 known vertices filling arcs between
        // them by mirroring the points between 0 and pi/2
        let mut points = Vec::new();
        points.push(center + Vec2::new(radius.x, 0.0));
        points.extend(quarter.iter().map(|p| center + *p));
        points.push(center + Vec2::new(0.0, radius.y));
        points.extend(quarter.iter().rev().map(|p| center + Vec2::new(-p.x, p.y)));
        points.push(center + Vec2::new(-radius.x, 0.0));
        points.extend(quarter.iter().map(|p| center - *p));
        points.push(center + Vec2::new(0.0, -radius.y));
        points.extend(quarter.iter().rev().map(|p| center + Vec2::new(p.x, -p.y)));

        let path_stroke = PathStroke::from(stroke).outside();
        self.scratchpad_path.clear();
        self.scratchpad_path.add_line_loop(&points);
        self.scratchpad_path
            .fill_and_stroke(self.feathering, fill, &path_stroke, out);
    }

    /// Tessellate a single [`Mesh`] into a [`Mesh`].
    ///
    /// * `mesh`: the mesh to tessellate.
    /// * `out`: triangles are appended to this.
    pub fn tessellate_mesh(&self, mesh: &Mesh, out: &mut Mesh) {
        if !mesh.is_valid() {
            debug_assert!(false, "Invalid Mesh in Shape::Mesh");
            return;
        }

        if self.options.coarse_tessellation_culling
            && !self.clip_rect.intersects(mesh.calc_bounds())
        {
            return;
        }

        out.append_ref(mesh);
    }

    /// Tessellate a line segment between the two points with the given stroke into a [`Mesh`].
    ///
    /// * `shape`: the mesh to tessellate.
    /// * `out`: triangles are appended to this.
    pub fn tessellate_line_segment(
        &mut self,
        mut points: [Pos2; 2],
        stroke: impl Into<Stroke>,
        out: &mut Mesh,
    ) {
        let stroke = stroke.into();
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

        if self.options.round_line_segments_to_pixels {
            let feathering = self.feathering;
            let pixels_per_point = self.pixels_per_point;

            let quarter_pixel = 0.25 * feathering; // Used to avoid fence post problem.

            let [a, b] = &mut points;
            if a.x == b.x {
                // Vertical line
                let mut x = a.x;
                stroke.round_center_to_pixel(self.pixels_per_point, &mut x);
                a.x = x;
                b.x = x;

                // Often the ends of the line are exactly on a pixel boundary,
                // but we extend line segments with a cap that is a pixel wide…
                // Solution: first shrink the line segment (on each end),
                // then round to pixel center!
                // We shrink by half-a-pixel n total (a quarter on each end),
                // so that on average we avoid the fence-post-problem after rounding.
                if a.y < b.y {
                    a.y = (a.y + quarter_pixel).round_to_pixel_center(pixels_per_point);
                    b.y = (b.y - quarter_pixel).round_to_pixel_center(pixels_per_point);
                } else {
                    a.y = (a.y - quarter_pixel).round_to_pixel_center(pixels_per_point);
                    b.y = (b.y + quarter_pixel).round_to_pixel_center(pixels_per_point);
                }
            }
            if a.y == b.y {
                // Horizontal line
                let mut y = a.y;
                stroke.round_center_to_pixel(self.pixels_per_point, &mut y);
                a.y = y;
                b.y = y;

                // See earlier comment for vertical lines
                if a.x < b.x {
                    a.x = (a.x + quarter_pixel).round_to_pixel_center(pixels_per_point);
                    b.x = (b.x - quarter_pixel).round_to_pixel_center(pixels_per_point);
                } else {
                    a.x = (a.x - quarter_pixel).round_to_pixel_center(pixels_per_point);
                    b.x = (b.x + quarter_pixel).round_to_pixel_center(pixels_per_point);
                }
            }
        }

        self.scratchpad_path.clear();
        self.scratchpad_path.add_line_segment(points);
        self.scratchpad_path
            .stroke_open(self.feathering, &stroke.into(), out);
    }

    #[deprecated = "Use `tessellate_line_segment` instead"]
    pub fn tessellate_line(
        &mut self,
        points: [Pos2; 2],
        stroke: impl Into<Stroke>,
        out: &mut Mesh,
    ) {
        self.tessellate_line_segment(points, stroke, out);
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

        profiling::function_scope!();

        let PathShape {
            points,
            closed,
            fill,
            stroke,
        } = path_shape;

        self.scratchpad_path.clear();

        if *closed {
            self.scratchpad_path.add_line_loop(points);

            self.scratchpad_path
                .fill_and_stroke(self.feathering, *fill, stroke, out);
        } else {
            debug_assert_eq!(
                *fill,
                Color32::TRANSPARENT,
                "You asked to fill a path that is not closed. That makes no sense."
            );

            self.scratchpad_path.add_open_points(points);

            self.scratchpad_path
                .stroke(self.feathering, PathType::Open, stroke, out);
        }
    }

    /// Tessellate a single [`Rect`] into a [`Mesh`].
    ///
    /// * `rect`: the rectangle to tessellate.
    /// * `out`: triangles are appended to this.
    pub fn tessellate_rect(&mut self, rect_shape: &RectShape, out: &mut Mesh) {
        if self.options.coarse_tessellation_culling
            && !rect_shape.visual_bounding_rect().intersects(self.clip_rect)
        {
            return;
        }

        let brush = rect_shape.brush.as_ref();
        let RectShape {
            mut rect,
            corner_radius,
            mut fill,
            mut stroke,
            mut stroke_kind,
            round_to_pixels,
            mut blur_width,
            brush: _, // brush is extracted on its own, because it is not Copy
        } = *rect_shape;

        let mut corner_radius = CornerRadiusF32::from(corner_radius);
        let round_to_pixels = round_to_pixels.unwrap_or(self.options.round_rects_to_pixels);

        if stroke.width == 0.0 {
            stroke.color = Color32::TRANSPARENT;
        }

        // It is common to (sometimes accidentally) create an infinitely sized rectangle.
        // Make sure we can handle that:
        rect.min = rect.min.at_least(pos2(-1e7, -1e7));
        rect.max = rect.max.at_most(pos2(1e7, 1e7));

        if !stroke.is_empty() {
            // Check if the stroke covers the whole rectangle
            let rect_with_stroke = match stroke_kind {
                StrokeKind::Inside => rect,
                StrokeKind::Middle => rect.expand(stroke.width / 2.0),
                StrokeKind::Outside => rect.expand(stroke.width),
            };

            if rect_with_stroke.size().min_elem() <= 2.0 * stroke.width + 0.5 * self.feathering {
                // The stroke covers the fill.
                // Change this to be a fill-only shape, using the stroke color as the new fill color.
                rect = rect_with_stroke;

                // We blend so that if the stroke is semi-transparent,
                // the fill still shines through.
                fill = stroke.color;

                stroke = Stroke::NONE;
            }
        }

        if stroke.is_empty() && out.texture_id == TextureId::default() {
            // Approximate thin rectangles with line segments.
            // This is important so that thin rectangles look good.
            if rect.width() <= 2.0 * self.feathering {
                return self.tessellate_line_segment(
                    [rect.center_top(), rect.center_bottom()],
                    (rect.width(), fill),
                    out,
                );
            }
            if rect.height() <= 2.0 * self.feathering {
                return self.tessellate_line_segment(
                    [rect.left_center(), rect.right_center()],
                    (rect.height(), fill),
                    out,
                );
            }
        }

        // Important: round to pixels BEFORE modifying/applying stroke_kind
        if round_to_pixels {
            // The rounding is aware of the stroke kind.
            // It is designed to be clever in trying to divine the intentions of the user.
            match stroke_kind {
                StrokeKind::Inside => {
                    // The stroke is inside the rect, so the rect defines the _outside_ of the stroke.
                    // We round the outside of the stroke on a pixel boundary.
                    // This will make the outside of the stroke crisp.
                    //
                    // Will make each stroke asymmetric if not an even multiple of physical pixels,
                    // but the left stroke will always be the mirror image of the right stroke,
                    // and the top stroke will always be the mirror image of the bottom stroke.
                    //
                    // This is so that a user can tile rectangles with `StrokeKind::Inside`,
                    // and get no pixel overlap between them.
                    rect = rect.round_to_pixels(self.pixels_per_point);
                }
                StrokeKind::Middle => {
                    // On this path we optimize for crisp and symmetric strokes.
                    stroke.round_rect_to_pixel(self.pixels_per_point, &mut rect);
                }
                StrokeKind::Outside => {
                    // Put the inside of the stroke on a pixel boundary.
                    // Makes the inside of the stroke and the filled rect crisp,
                    // but the outside of the stroke may become feathered (blurry).
                    //
                    // Will make each stroke asymmetric if not an even multiple of physical pixels,
                    // but the left stroke will always be the mirror image of the right stroke,
                    // and the top stroke will always be the mirror image of the bottom stroke.
                    rect = rect.round_to_pixels(self.pixels_per_point);
                }
            }
        }

        let old_feathering = self.feathering;

        if self.feathering < blur_width {
            // We accomplish the blur by using a larger-than-normal feathering.
            // Feathering is usually used to make the edges of a shape softer for anti-aliasing.

            // The tessellator can't handle blurring/feathering larger than the smallest side of the rect.
            let eps = 0.1; // avoid numerical problems
            blur_width = blur_width
                .at_most(rect.size().min_elem() - eps - 2.0 * stroke.width)
                .at_least(0.0);

            corner_radius += 0.5 * blur_width;

            self.feathering = self.feathering.max(blur_width);
        }

        {
            // Modify `rect` so that it represents the OUTER border
            // We do this because `path::rounded_rectangle` uses the
            // corner radius to pick the fidelity/resolution of the corner.

            let original_cr = corner_radius;

            match stroke_kind {
                StrokeKind::Inside => {}
                StrokeKind::Middle => {
                    rect = rect.expand(stroke.width / 2.0);
                    corner_radius += stroke.width / 2.0;
                }
                StrokeKind::Outside => {
                    rect = rect.expand(stroke.width);
                    corner_radius += stroke.width;
                }
            }

            stroke_kind = StrokeKind::Inside;

            // A small corner_radius is incompatible with a wide stroke,
            // because the small bend will be extruded inwards and cross itself.
            // There are two ways to solve this (wile maintaining constant stroke width):
            // either we increase the corner_radius, or we set it to zero.
            // We choose the former: if the user asks for _any_ corner_radius, they should get it.

            let min_inside_cr = 0.1; // Large enough to avoid numerical issues
            let min_outside_cr = stroke.width + min_inside_cr;

            let extra_cr_tweak = 0.4; // Otherwise is doesn't _feels_  enough.

            if original_cr.nw == 0.0 {
                corner_radius.nw = 0.0;
            } else {
                corner_radius.nw += extra_cr_tweak;
                corner_radius.nw = corner_radius.nw.at_least(min_outside_cr);
            }
            if original_cr.ne == 0.0 {
                corner_radius.ne = 0.0;
            } else {
                corner_radius.ne += extra_cr_tweak;
                corner_radius.ne = corner_radius.ne.at_least(min_outside_cr);
            }
            if original_cr.sw == 0.0 {
                corner_radius.sw = 0.0;
            } else {
                corner_radius.sw += extra_cr_tweak;
                corner_radius.sw = corner_radius.sw.at_least(min_outside_cr);
            }
            if original_cr.se == 0.0 {
                corner_radius.se = 0.0;
            } else {
                corner_radius.se += extra_cr_tweak;
                corner_radius.se = corner_radius.se.at_least(min_outside_cr);
            }
        }

        let path = &mut self.scratchpad_path;
        path.clear();
        path::rounded_rectangle(&mut self.scratchpad_points, rect, corner_radius);
        path.add_line_loop(&self.scratchpad_points);

        let path_stroke = PathStroke::from(stroke).with_kind(stroke_kind);

        if let Some(brush) = brush {
            // Textured fill

            let fill_rect = match stroke_kind {
                StrokeKind::Inside => rect.shrink(stroke.width),
                StrokeKind::Middle => rect.shrink(stroke.width / 2.0),
                StrokeKind::Outside => rect,
            };

            if fill_rect.is_positive() {
                let crate::Brush {
                    fill_texture_id,
                    uv,
                } = **brush;
                let uv_from_pos = |p: Pos2| {
                    pos2(
                        remap(p.x, rect.x_range(), uv.x_range()),
                        remap(p.y, rect.y_range(), uv.y_range()),
                    )
                };
                path.fill_with_uv(self.feathering, fill, fill_texture_id, uv_from_pos, out);
            }

            if !stroke.is_empty() {
                path.stroke_closed(self.feathering, &path_stroke, out);
            }
        } else {
            // Stroke and maybe fill
            path.fill_and_stroke(self.feathering, fill, &path_stroke, out);
        }

        self.feathering = old_feathering; // restore
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
            fallback_color,
            opacity_factor,
            angle,
        } = text_shape;

        if galley.is_empty() {
            return;
        }

        if *opacity_factor <= 0.0 {
            return;
        }

        if galley.pixels_per_point != self.pixels_per_point {
            log::warn!(
                "epaint: WARNING: pixels_per_point (dpi scale) have changed between text layout and tessellation. \
                       You must recreate your text shapes if pixels_per_point changes."
            );
        }

        out.vertices.reserve(galley.num_vertices);
        out.indices.reserve(galley.num_indices);

        // The contents of the galley are already snapped to pixel coordinates,
        // but we need to make sure the galley ends up on the start of a physical pixel:
        let galley_pos = if self.options.round_text_to_pixels {
            galley_pos.round_to_pixels(self.pixels_per_point)
        } else {
            *galley_pos
        };

        let uv_normalizer = vec2(
            1.0 / self.font_tex_size[0] as f32,
            1.0 / self.font_tex_size[1] as f32,
        );

        let rotator = Rot2::from_angle(*angle);

        for row in &galley.rows {
            if row.visuals.mesh.is_empty() {
                continue;
            }

            let final_row_pos = galley_pos + rotator * row.pos.to_vec2();

            let mut row_rect = row.visuals.mesh_bounds;
            if *angle != 0.0 {
                row_rect = row_rect.rotate_bb(rotator);
            }
            row_rect = row_rect.translate(final_row_pos.to_vec2());

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
                            // Only override the glyph color (not background color, strike-through color, etc)
                            if row.visuals.glyph_vertex_range.contains(&i) {
                                color = *override_text_color;
                            }
                        } else if color == Color32::PLACEHOLDER {
                            color = *fallback_color;
                        }

                        if *opacity_factor < 1.0 {
                            color = color.gamma_multiply(*opacity_factor);
                        }

                        debug_assert!(color != Color32::PLACEHOLDER, "A placeholder color made it to the tessellator. You forgot to set a fallback color.");

                        let offset = if *angle == 0.0 {
                            pos.to_vec2()
                        } else {
                            rotator * pos.to_vec2()
                        };

                        Vertex {
                            pos: final_row_pos + offset,
                            uv: (uv.to_vec2() * uv_normalizer).to_pos2(),
                            color,
                        }
                    }),
            );

            if *underline != Stroke::NONE {
                self.tessellate_line_segment(
                    [row_rect.left_bottom(), row_rect.right_bottom()],
                    *underline,
                    out,
                );
            }
        }
    }

    /// Tessellate a single [`QuadraticBezierShape`] into a [`Mesh`].
    ///
    /// * `quadratic_shape`: the shape to tessellate.
    /// * `out`: triangles are appended to this.
    pub fn tessellate_quadratic_bezier(
        &mut self,
        quadratic_shape: &QuadraticBezierShape,
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
            &quadratic_shape.stroke,
            out,
        );
    }

    /// Tessellate a single [`CubicBezierShape`] into a [`Mesh`].
    ///
    /// * `cubic_shape`: the shape to tessellate.
    /// * `out`: triangles are appended to this.
    pub fn tessellate_cubic_bezier(&mut self, cubic_shape: &CubicBezierShape, out: &mut Mesh) {
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
                &cubic_shape.stroke,
                out,
            );
        }
    }

    fn tessellate_bezier_complete(
        &mut self,
        points: &[Pos2],
        fill: Color32,
        closed: bool,
        stroke: &PathStroke,
        out: &mut Mesh,
    ) {
        if points.len() < 2 {
            return;
        }

        self.scratchpad_path.clear();
        if closed {
            self.scratchpad_path.add_line_loop(points);

            self.scratchpad_path
                .fill_and_stroke(self.feathering, fill, stroke, out);
        } else {
            debug_assert_eq!(
                fill,
                Color32::TRANSPARENT,
                "You asked to fill a bezier path that is not closed. That makes no sense."
            );

            self.scratchpad_path.add_open_points(points);

            self.scratchpad_path
                .stroke(self.feathering, PathType::Open, stroke, out);
        }
    }
}

impl Tessellator {
    /// Turns [`Shape`]:s into sets of triangles.
    ///
    /// The given shapes will tessellated in the same order as they are given.
    /// They will be batched together by clip rectangle.
    ///
    /// * `pixels_per_point`: number of physical pixels to each logical point
    /// * `options`: tessellation quality
    /// * `shapes`: what to tessellate
    /// * `font_tex_size`: size of the font texture. Required to normalize glyph uv rectangles when tessellating text.
    /// * `prepared_discs`: What [`crate::TextureAtlas::prepared_discs`] returns. Can safely be set to an empty vec.
    ///
    /// The implementation uses a [`Tessellator`].
    ///
    /// ## Returns
    /// A list of clip rectangles with matching [`Mesh`].
    #[allow(clippy::allow_attributes, unused_mut)]
    pub fn tessellate_shapes(&mut self, mut shapes: Vec<ClippedShape>) -> Vec<ClippedPrimitive> {
        profiling::function_scope!();

        #[cfg(feature = "rayon")]
        if self.options.parallel_tessellation {
            self.parallel_tessellation_of_large_shapes(&mut shapes);
        }

        let mut clipped_primitives: Vec<ClippedPrimitive> = Vec::default();

        {
            profiling::scope!("tessellate");
            for clipped_shape in shapes {
                self.tessellate_clipped_shape(clipped_shape, &mut clipped_primitives);
            }
        }

        if self.options.debug_paint_clip_rects {
            clipped_primitives = self.add_clip_rects(clipped_primitives);
        }

        if self.options.debug_ignore_clip_rects {
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
                debug_assert!(mesh.is_valid(), "Tessellator generated invalid Mesh");
            }
        }

        clipped_primitives
    }

    /// Find large shapes and throw them on the rayon thread pool,
    /// then replace the original shape with their tessellated meshes.
    #[cfg(feature = "rayon")]
    fn parallel_tessellation_of_large_shapes(&self, shapes: &mut [ClippedShape]) {
        profiling::function_scope!();

        use rayon::prelude::*;

        // We only parallelize large/slow stuff, because each tessellation job
        // will allocate a new Mesh, and so it creates a lot of extra memory fragmentation
        // and allocations that is only worth it for large shapes.
        fn should_parallelize(shape: &Shape) -> bool {
            match shape {
                Shape::Vec(shapes) => 4 < shapes.len() || shapes.iter().any(should_parallelize),

                Shape::Path(path_shape) => 32 < path_shape.points.len(),

                Shape::QuadraticBezier(_) | Shape::CubicBezier(_) | Shape::Ellipse(_) => true,

                Shape::Noop
                | Shape::Text(_)
                | Shape::Circle(_)
                | Shape::Mesh(_)
                | Shape::LineSegment { .. }
                | Shape::Rect(_)
                | Shape::Callback(_) => false,
            }
        }

        let tessellated: Vec<(usize, Mesh)> = shapes
            .par_iter()
            .enumerate()
            .filter(|(_, clipped_shape)| should_parallelize(&clipped_shape.shape))
            .map(|(index, clipped_shape)| {
                profiling::scope!("tessellate_big_shape");
                // TODO(emilk): reuse tessellator in a thread local
                let mut tessellator = (*self).clone();
                let mut mesh = Mesh::default();
                tessellator.tessellate_shape(clipped_shape.shape.clone(), &mut mesh);
                (index, mesh)
            })
            .collect();

        profiling::scope!("distribute results", tessellated.len().to_string());
        for (index, mesh) in tessellated {
            shapes[index].shape = Shape::Mesh(mesh.into());
        }
    }

    fn add_clip_rects(
        &mut self,
        clipped_primitives: Vec<ClippedPrimitive>,
    ) -> Vec<ClippedPrimitive> {
        self.clip_rect = Rect::EVERYTHING;
        let stroke = Stroke::new(2.0, Color32::from_rgb(150, 255, 150));

        clipped_primitives
            .into_iter()
            .flat_map(|clipped_primitive| {
                let mut clip_rect_mesh = Mesh::default();
                self.tessellate_shape(
                    Shape::rect_stroke(
                        clipped_primitive.clip_rect,
                        0.0,
                        stroke,
                        StrokeKind::Outside,
                    ),
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
    let clipped_shapes = vec![ClippedShape {
        clip_rect: rect,
        shape,
    }];

    let font_tex_size = [1024, 1024]; // unused
    let prepared_discs = vec![]; // unused

    let primitives = Tessellator::new(1.0, Default::default(), font_tex_size, prepared_discs)
        .tessellate_shapes(clipped_shapes);

    assert_eq!(primitives.len(), 2);
}

#[test]
fn path_bounding_box() {
    use crate::*;

    for i in 1..=100 {
        let width = i as f32;

        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(10.0, 10.0));
        let expected_rect = rect.expand((width / 2.0) + 1.5);

        let mut mesh = Mesh::default();

        let mut path = Path::default();
        path.add_open_points(&[
            pos2(0.0, 0.0),
            pos2(2.0, 0.0),
            pos2(5.0, 5.0),
            pos2(0.0, 5.0),
            pos2(0.0, 7.0),
            pos2(10.0, 10.0),
        ]);

        path.stroke(
            1.5,
            PathType::Closed,
            &PathStroke::new_uv(width, move |r, p| {
                assert_eq!(r, expected_rect);
                // see https://github.com/emilk/egui/pull/4353#discussion_r1573879940 for why .contains() isn't used here.
                // TL;DR rounding errors.
                assert!(
                    r.distance_to_pos(p) <= 0.55,
                    "passed rect {r:?} didn't contain point {p:?} (distance: {})",
                    r.distance_to_pos(p)
                );
                assert!(
                    expected_rect.distance_to_pos(p) <= 0.55,
                    "expected rect {expected_rect:?} didn't contain point {p:?}"
                );
                Color32::WHITE
            }),
            &mut mesh,
        );
    }
}
