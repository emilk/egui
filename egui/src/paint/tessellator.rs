//! Converts graphics primitives into textured triangles.
//!
//! This module converts lines, circles, text and more represented by `PaintCmd`
//! into textured triangles represented by `Triangles`.

#![allow(clippy::identity_op)]

use {
    super::{
        color::{self, srgba, Rgba, Srgba, TRANSPARENT},
        fonts::Fonts,
        PaintCmd, Stroke,
    },
    crate::math::*,
};

/// The UV coordinate of a white region of the texture mesh.
pub const WHITE_UV: (u16, u16) = (1, 1);

/// The vertex type.
///
/// Should be friendly to send to GPU as is.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Vertex {
    /// Logical pixel coordinates (points).
    /// (0,0) is the top left corner of the screen.
    pub pos: Pos2, // 64 bit
    /// Texel coordinates in the texture
    pub uv: (u16, u16), // 32 bit
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
}

/// A clip triangle and some textured triangles.
pub type PaintJob = (Rect, Triangles);

/// Grouped by clip rectangles, in pixel coordinates
pub type PaintJobs = Vec<PaintJob>;

// ----------------------------------------------------------------------------

/// ## Helpers for adding
impl Triangles {
    /// Are all indices within the bounds of the contained vertices?
    pub fn is_valid(&self) -> bool {
        let n = self.vertices.len() as u32;
        self.indices.iter().all(|&i| i < n)
    }

    /// Append all the indices and vertices of `other` to `self`.
    pub fn append(&mut self, other: &Triangles) {
        let index_offset = self.vertices.len() as u32;
        for index in &other.indices {
            self.indices.push(index_offset + index);
        }
        self.vertices.extend(other.vertices.iter());
    }

    pub fn colored_vertex(&mut self, pos: Pos2, color: Srgba) {
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

    /// Uniformly colored rectangle.
    pub fn add_rect(&mut self, top_left: Vertex, bottom_right: Vertex) {
        debug_assert_eq!(top_left.color, bottom_right.color);

        let idx = self.vertices.len() as u32;
        self.add_triangle(idx + 0, idx + 1, idx + 2);
        self.add_triangle(idx + 2, idx + 1, idx + 3);

        let top_right = Vertex {
            pos: pos2(bottom_right.pos.x, top_left.pos.y),
            uv: (bottom_right.uv.0, top_left.uv.1),
            color: top_left.color,
        };
        let botom_left = Vertex {
            pos: pos2(top_left.pos.x, bottom_right.pos.y),
            uv: (top_left.uv.0, bottom_right.uv.1),
            color: top_left.color,
        };
        self.vertices.push(top_left);
        self.vertices.push(top_right);
        self.vertices.push(botom_left);
        self.vertices.push(bottom_right);
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
            });
        }
        output
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
#[derive(Clone, Copy)]
pub struct PaintOptions {
    /// Anti-aliasing makes shapes appear smoother, but requires more triangles and is therefore slower.
    pub anti_alias: bool,
    /// Size of a pixel in points, e.g. 0.5
    pub aa_size: f32,
    /// Output the clip rectangles to be painted?
    pub debug_paint_clip_rects: bool,
}

impl Default for PaintOptions {
    fn default() -> Self {
        Self {
            anti_alias: true,
            aa_size: 1.0,
            debug_paint_clip_rects: false,
        }
    }
}

/// Tesselate the given convex area into a polygon.
fn fill_closed_path(path: &[PathPoint], color: Srgba, options: PaintOptions, out: &mut Triangles) {
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
    options: PaintOptions,
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

/// Tesselate a single `PaintCmd` into a `Triangles`.
///
/// * `command`: the command to tesselate
/// * `options`: tesselation quality
/// * `fonts`: font source when tessellating text
/// * `out`: where the triangles are put
/// * `scratchpad_path`: if you plan to run `tessellate_paint_command`
///    many times, pass it a reference to the same `Path` to avoid excessive allocations.
fn tessellate_paint_command(
    command: PaintCmd,
    options: PaintOptions,
    fonts: &Fonts,
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
            if radius > 0.0 {
                path.add_circle(center, radius);
                fill_closed_path(&path.0, fill, options, out);
                stroke_path(&path.0, Closed, stroke, options, out);
            }
        }
        PaintCmd::Triangles(triangles) => {
            out.append(&triangles);
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
            if !rect.is_empty() {
                // It is common to (sometimes accidentally) create an infinitely sized rectangle.
                // Make sure we can handle that:
                rect.min = rect.min.max(pos2(-1e7, -1e7));
                rect.max = rect.max.min(pos2(1e7, 1e7));

                path::rounded_rectangle(scratchpad_points, rect, corner_radius);
                path.add_line_loop(scratchpad_points);
                fill_closed_path(&path.0, fill, options, out);
                stroke_path(&path.0, Closed, stroke, options, out);
            }
        }
        PaintCmd::Text {
            pos,
            galley,
            text_style,
            color,
        } => {
            if color == TRANSPARENT {
                return;
            }
            galley.sanity_check();

            let num_chars = galley.text.chars().count();
            out.reserve_triangles(num_chars * 2);
            out.reserve_vertices(num_chars * 4);

            let text_offset = vec2(0.0, 1.0); // Eye-balled for buttons. TODO: why is this needed?

            let font = &fonts[text_style];
            let mut chars = galley.text.chars();
            for line in &galley.lines {
                for x_offset in line.x_offsets.iter().take(line.x_offsets.len() - 1) {
                    let c = chars.next().unwrap();
                    if let Some(glyph) = font.uv_rect(c) {
                        let mut top_left = Vertex {
                            pos: pos + glyph.offset + vec2(*x_offset, line.y_min) + text_offset,
                            uv: glyph.min,
                            color,
                        };
                        top_left.pos.x = font.round_to_pixel(top_left.pos.x); // Pixel-perfection.
                        top_left.pos.y = font.round_to_pixel(top_left.pos.y); // Pixel-perfection.
                        let bottom_right = Vertex {
                            pos: top_left.pos + glyph.size,
                            uv: glyph.max,
                            color,
                        };
                        out.add_rect(top_left, bottom_right);
                    }
                }
            }
            assert_eq!(chars.next(), None);
        }
    }
}

/// Turns `PaintCmd`:s into sets of triangles.
///
/// The given commands will be painted back-to-front (painters algorithm).
/// They will be batched together by clip rectangle.
///
/// * `commands`: the command to tesselate
/// * `options`: tesselation quality
/// * `fonts`: font source when tessellating text
///
/// ## Returns
/// A list of clip rectangles with matching `Triangles`.
pub fn tessellate_paint_commands(
    commands: Vec<(Rect, PaintCmd)>,
    options: PaintOptions,
    fonts: &Fonts,
) -> Vec<(Rect, Triangles)> {
    let mut scratchpad_points = Vec::new();
    let mut scratchpad_path = Path::default();

    let mut jobs = PaintJobs::default();
    for (clip_rect, cmd) in commands {
        // TODO: cull(clip_rect, cmd)

        if jobs.is_empty() || jobs.last().unwrap().0 != clip_rect {
            jobs.push((clip_rect, Triangles::default()));
        }

        let out = &mut jobs.last_mut().unwrap().1;
        tessellate_paint_command(
            cmd,
            options,
            fonts,
            out,
            &mut scratchpad_points,
            &mut scratchpad_path,
        );
    }

    if options.debug_paint_clip_rects {
        for (clip_rect, triangles) in &mut jobs {
            tessellate_paint_command(
                PaintCmd::Rect {
                    rect: *clip_rect,
                    corner_radius: 0.0,
                    fill: Default::default(),
                    stroke: Stroke::new(2.0, srgba(150, 255, 150, 255)),
                },
                options,
                fonts,
                triangles,
                &mut scratchpad_points,
                &mut scratchpad_path,
            )
        }
    }

    jobs
}
