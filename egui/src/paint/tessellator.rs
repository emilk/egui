#![allow(clippy::identity_op)]

use {
    super::{
        color::{self, srgba, Color},
        fonts::Fonts,
        LineStyle, PaintCmd,
    },
    crate::math::*,
};

/// The UV coordinate of a white region of the texture mesh.
const WHITE_UV: (u16, u16) = (1, 1);

#[derive(Clone, Copy, Debug, Default)]
pub struct Vertex {
    /// Logical pixel coordinates (points)
    pub pos: Pos2,
    /// Texel coordinates in the texture
    pub uv: (u16, u16),
    /// sRGBA with premultiplied alpha
    pub color: Color,
}

/// Textured triangles
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

impl Triangles {
    pub fn append(&mut self, triangles: &Triangles) {
        let index_offset = self.vertices.len() as u32;
        for index in &triangles.indices {
            self.indices.push(index_offset + index);
        }
        self.vertices.extend(triangles.vertices.iter());
    }

    fn triangle(&mut self, a: u32, b: u32, c: u32) {
        self.indices.push(a);
        self.indices.push(b);
        self.indices.push(c);
    }

    pub fn reserve_triangles(&mut self, additional_triangles: usize) {
        self.indices.reserve(3 * additional_triangles);
    }

    pub fn reserve_vertices(&mut self, additional: usize) {
        self.vertices.reserve(additional);
    }

    /// Uniformly colored rectangle
    pub fn add_rect(&mut self, top_left: Vertex, bottom_right: Vertex) {
        debug_assert_eq!(top_left.color, bottom_right.color);

        let idx = self.vertices.len() as u32;
        self.triangle(idx + 0, idx + 1, idx + 2);
        self.triangle(idx + 2, idx + 1, idx + 3);

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
    /// Splits this mesh into many small if needed.
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

    /// For filled paths the normal is used for antialiasing.
    /// For outlines the normal is used for figuring out how to make the line wide
    /// (i.e. in what direction to expand).
    /// The normal could be estimated by differences between successive points,
    /// but that would be less accurate (and in some cases slower).
    normal: Vec2,
}

/// A 2D path that can be tesselated into triangles.
#[derive(Clone, Debug, Default)]
pub struct Path(Vec<PathPoint>);

impl Path {
    pub fn from_point_loop(points: &[Pos2]) -> Self {
        let mut path = Self::default();
        path.add_line_loop(points);
        path
    }

    pub fn from_open_points(points: &[Pos2]) -> Self {
        let mut path = Self::default();
        path.add_open_points(points);
        path
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
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
            self.reserve(n);
            self.add_point(points[0], (points[1] - points[0]).normalized().rot90());
            for i in 1..n - 1 {
                let n0 = (points[i] - points[i - 1]).normalized().rot90(); // TODO: don't calculate each normal twice!
                let n1 = (points[i + 1] - points[i]).normalized().rot90(); // TODO: don't calculate each normal twice!
                let v = (n0 + n1) / 2.0;
                let normal = v / v.length_sq();
                self.add_point(points[i], normal); // TODO: handle VERY sharp turns better
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
            let n0 = (points[i] - points[(i + n - 1) % n]).normalized().rot90();
            let n1 = (points[(i + 1) % n] - points[i]).normalized().rot90();
            let v = (n0 + n1) / 2.0;
            let normal = v / v.length_sq();
            self.add_point(points[i], normal); // TODO: handle VERY sharp turns better
        }
    }

    pub fn add_rectangle(&mut self, rect: Rect) {
        let min = rect.min;
        let max = rect.max;
        self.reserve(4);
        self.add_point(pos2(min.x, min.y), vec2(-1.0, -1.0));
        self.add_point(pos2(max.x, min.y), vec2(1.0, -1.0));
        self.add_point(pos2(max.x, max.y), vec2(1.0, 1.0));
        self.add_point(pos2(min.x, max.y), vec2(-1.0, 1.0));
    }

    pub fn add_rounded_rectangle(&mut self, rect: Rect, corner_radius: f32) {
        let min = rect.min;
        let max = rect.max;

        let cr = corner_radius
            .min(rect.width() * 0.5)
            .min(rect.height() * 0.5);

        if cr <= 0.0 {
            self.add_rectangle(rect);
        } else {
            self.add_circle_quadrant(pos2(max.x - cr, max.y - cr), cr, 0.0);
            self.add_circle_quadrant(pos2(min.x + cr, max.y - cr), cr, 1.0);
            self.add_circle_quadrant(pos2(min.x + cr, min.y + cr), cr, 2.0);
            self.add_circle_quadrant(pos2(max.x - cr, min.y + cr), cr, 3.0);
        }
    }

    /// with x right, and y down (GUI coords) we have:
    /// angle       = dir
    /// 0 * TAU / 4 = right
    ///    quadrant 0, right bottom
    /// 1 * TAU / 4 = bottom
    ///    quadrant 1, left bottom
    /// 2 * TAU / 4 = left
    ///    quadrant 2 left top
    /// 3 * TAU / 4 = top
    ///    quadrant 3 right top
    /// 4 * TAU / 4 = right
    pub fn add_circle_quadrant(&mut self, center: Pos2, radius: f32, quadrant: f32) {
        // TODO: optimize with precalculated vertices for some radii ranges

        let n = (radius * 0.75).round() as i32; // TODO: tweak a bit more
        let n = clamp(n, 2..=32);
        self.reserve(n as usize + 1);
        const RIGHT_ANGLE: f32 = TAU / 4.0;
        for i in 0..=n {
            let angle = remap(
                i as f32,
                0.0..=n as f32,
                quadrant * RIGHT_ANGLE..=(quadrant + 1.0) * RIGHT_ANGLE,
            );
            let normal = vec2(angle.cos(), angle.sin());
            self.add_point(center + radius * normal, normal);
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

pub fn fill_closed_path(
    triangles: &mut Triangles,
    options: PaintOptions,
    path: &[PathPoint],
    color: Color,
) {
    if color == color::TRANSPARENT {
        return;
    }

    let n = path.len() as u32;
    let vert = |pos, color| Vertex {
        pos,
        uv: WHITE_UV,
        color,
    };
    if options.anti_alias {
        triangles.reserve_triangles(3 * n as usize);
        triangles.reserve_vertices(2 * n as usize);
        let color_outer = color::TRANSPARENT;
        let idx_inner = triangles.vertices.len() as u32;
        let idx_outer = idx_inner + 1;
        for i in 2..n {
            triangles.triangle(idx_inner + 2 * (i - 1), idx_inner, idx_inner + 2 * i);
        }
        let mut i0 = n - 1;
        for i1 in 0..n {
            let p1 = &path[i1 as usize];
            let dm = p1.normal * options.aa_size * 0.5;
            triangles.vertices.push(vert(p1.pos - dm, color));
            triangles.vertices.push(vert(p1.pos + dm, color_outer));
            triangles.triangle(idx_inner + i1 * 2, idx_inner + i0 * 2, idx_outer + 2 * i0);
            triangles.triangle(idx_outer + i0 * 2, idx_outer + i1 * 2, idx_inner + 2 * i1);
            i0 = i1;
        }
    } else {
        triangles.reserve_triangles(n as usize);
        let idx = triangles.vertices.len() as u32;
        triangles
            .vertices
            .extend(path.iter().map(|p| vert(p.pos, color)));
        for i in 2..n {
            triangles.triangle(idx, idx + i - 1, idx + i);
        }
    }
}

pub fn paint_path_outline(
    triangles: &mut Triangles,
    options: PaintOptions,
    path_type: PathType,
    path: &[PathPoint],
    style: LineStyle,
) {
    if style.color == color::TRANSPARENT {
        return;
    }

    let n = path.len() as u32;
    let idx = triangles.vertices.len() as u32;

    let vert = |pos, color| Vertex {
        pos,
        uv: WHITE_UV,
        color,
    };

    if options.anti_alias {
        let color_inner = style.color;
        let color_outer = color::TRANSPARENT;

        let thin_line = style.width <= options.aa_size;
        if thin_line {
            /*
            We paint the line using three edges: outer, inner, outer.

            .       o   i   o      outer, inner, outer
            .       |---|          aa_size (pixel width)
            */

            // Fade out as it gets thinner:
            let color_inner = mul_color(color_inner, style.width / options.aa_size);
            if color_inner == color::TRANSPARENT {
                return;
            }

            triangles.reserve_triangles(4 * n as usize);
            triangles.reserve_vertices(3 * n as usize);

            let mut i0 = n - 1;
            for i1 in 0..n {
                let connect_with_previous = path_type == PathType::Closed || i1 > 0;
                let p1 = &path[i1 as usize];
                let p = p1.pos;
                let n = p1.normal;
                triangles
                    .vertices
                    .push(vert(p + n * options.aa_size, color_outer));
                triangles.vertices.push(vert(p, color_inner));
                triangles
                    .vertices
                    .push(vert(p - n * options.aa_size, color_outer));

                if connect_with_previous {
                    triangles.triangle(idx + 3 * i0 + 0, idx + 3 * i0 + 1, idx + 3 * i1 + 0);
                    triangles.triangle(idx + 3 * i0 + 1, idx + 3 * i1 + 0, idx + 3 * i1 + 1);

                    triangles.triangle(idx + 3 * i0 + 1, idx + 3 * i0 + 2, idx + 3 * i1 + 1);
                    triangles.triangle(idx + 3 * i0 + 2, idx + 3 * i1 + 1, idx + 3 * i1 + 2);
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

            triangles.reserve_triangles(6 * n as usize);
            triangles.reserve_vertices(4 * n as usize);

            let mut i0 = n - 1;
            for i1 in 0..n {
                let connect_with_previous = path_type == PathType::Closed || i1 > 0;
                let inner_rad = 0.5 * (style.width - options.aa_size);
                let outer_rad = 0.5 * (style.width + options.aa_size);
                let p1 = &path[i1 as usize];
                let p = p1.pos;
                let n = p1.normal;
                triangles
                    .vertices
                    .push(vert(p + n * outer_rad, color_outer));
                triangles
                    .vertices
                    .push(vert(p + n * inner_rad, color_inner));
                triangles
                    .vertices
                    .push(vert(p - n * inner_rad, color_inner));
                triangles
                    .vertices
                    .push(vert(p - n * outer_rad, color_outer));

                if connect_with_previous {
                    triangles.triangle(idx + 4 * i0 + 0, idx + 4 * i0 + 1, idx + 4 * i1 + 0);
                    triangles.triangle(idx + 4 * i0 + 1, idx + 4 * i1 + 0, idx + 4 * i1 + 1);

                    triangles.triangle(idx + 4 * i0 + 1, idx + 4 * i0 + 2, idx + 4 * i1 + 1);
                    triangles.triangle(idx + 4 * i0 + 2, idx + 4 * i1 + 1, idx + 4 * i1 + 2);

                    triangles.triangle(idx + 4 * i0 + 2, idx + 4 * i0 + 3, idx + 4 * i1 + 2);
                    triangles.triangle(idx + 4 * i0 + 3, idx + 4 * i1 + 2, idx + 4 * i1 + 3);
                }
                i0 = i1;
            }
        }
    } else {
        triangles.reserve_triangles(2 * n as usize);
        triangles.reserve_vertices(2 * n as usize);

        let last_index = if path_type == Closed { n } else { n - 1 };
        for i in 0..last_index {
            triangles.triangle(
                idx + (2 * i + 0) % (2 * n),
                idx + (2 * i + 1) % (2 * n),
                idx + (2 * i + 2) % (2 * n),
            );
            triangles.triangle(
                idx + (2 * i + 2) % (2 * n),
                idx + (2 * i + 1) % (2 * n),
                idx + (2 * i + 3) % (2 * n),
            );
        }

        let thin_line = style.width <= options.aa_size;
        if thin_line {
            // Fade out thin lines rather than making them thinner
            let radius = options.aa_size / 2.0;
            let color = mul_color(style.color, style.width / options.aa_size);
            if color == color::TRANSPARENT {
                return;
            }
            for p in path {
                triangles
                    .vertices
                    .push(vert(p.pos + radius * p.normal, color));
                triangles
                    .vertices
                    .push(vert(p.pos - radius * p.normal, color));
            }
        } else {
            let radius = style.width / 2.0;
            for p in path {
                triangles
                    .vertices
                    .push(vert(p.pos + radius * p.normal, style.color));
                triangles
                    .vertices
                    .push(vert(p.pos - radius * p.normal, style.color));
            }
        }
    }
}

fn mul_color(color: Color, factor: f32) -> Color {
    // TODO: sRGBA correct fading
    debug_assert!(0.0 <= factor && factor <= 1.0);
    Color {
        r: (f32::from(color.r) * factor).round() as u8,
        g: (f32::from(color.g) * factor).round() as u8,
        b: (f32::from(color.b) * factor).round() as u8,
        a: (f32::from(color.a) * factor).round() as u8,
    }
}

// ----------------------------------------------------------------------------

/// `reused_path`: only used to reuse memory
pub fn tessellate_paint_command(
    reused_path: &mut Path,
    options: PaintOptions,
    fonts: &Fonts,
    command: PaintCmd,
    out: &mut Triangles,
) {
    let path = reused_path;
    path.clear();

    match command {
        PaintCmd::Noop => {}
        PaintCmd::Circle {
            center,
            fill,
            outline,
            radius,
        } => {
            if radius > 0.0 {
                path.add_circle(center, radius);
                if let Some(fill) = fill {
                    fill_closed_path(out, options, &path.0, fill);
                }
                if let Some(outline) = outline {
                    paint_path_outline(out, options, Closed, &path.0, outline);
                }
            }
        }
        PaintCmd::Triangles(triangles) => {
            out.append(&triangles);
        }
        PaintCmd::LineSegment { points, style } => {
            path.add_line_segment(points);
            paint_path_outline(out, options, Open, &path.0, style);
        }
        PaintCmd::Path {
            path,
            closed,
            fill,
            outline,
        } => {
            if path.len() >= 2 {
                if let Some(fill) = fill {
                    debug_assert!(
                        closed,
                        "You asked to fill a path that is not closed. That makes no sense."
                    );
                    fill_closed_path(out, options, &path.0, fill);
                }
                if let Some(outline) = outline {
                    let typ = if closed { Closed } else { Open };
                    paint_path_outline(out, options, typ, &path.0, outline);
                }
            }
        }
        PaintCmd::Rect {
            corner_radius,
            fill,
            outline,
            mut rect,
        } => {
            if !rect.is_empty() {
                // It is common to (sometimes accidentally) create an infinitely sized ractangle.
                // Make sure we can handle that:
                rect.min = rect.min.max(pos2(-1e7, -1e7));
                rect.max = rect.max.min(pos2(1e7, 1e7));

                path.add_rounded_rectangle(rect, corner_radius);
                if let Some(fill) = fill {
                    fill_closed_path(out, options, &path.0, fill);
                }
                if let Some(outline) = outline {
                    paint_path_outline(out, options, Closed, &path.0, outline);
                }
            }
        }
        PaintCmd::Text {
            pos,
            galley,
            text_style,
            color,
        } => {
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

/// Turns `PaintCmd`:s into sets of triangles
pub fn tessellate_paint_commands(
    options: PaintOptions,
    fonts: &Fonts,
    commands: Vec<(Rect, PaintCmd)>,
) -> Vec<(Rect, Triangles)> {
    let mut reused_path = Path::default();

    let mut jobs = PaintJobs::default();
    for (clip_rect, cmd) in commands {
        // TODO: cull(clip_rect, cmd)

        if jobs.is_empty() || jobs.last().unwrap().0 != clip_rect {
            jobs.push((clip_rect, Triangles::default()));
        }

        let out = &mut jobs.last_mut().unwrap().1;
        tessellate_paint_command(&mut reused_path, options, fonts, cmd, out);
    }

    if options.debug_paint_clip_rects {
        for (clip_rect, triangles) in &mut jobs {
            tessellate_paint_command(
                &mut reused_path,
                options,
                fonts,
                PaintCmd::Rect {
                    rect: *clip_rect,
                    corner_radius: 0.0,
                    fill: None,
                    outline: Some(LineStyle::new(2.0, srgba(150, 255, 150, 255))),
                },
                triangles,
            )
        }
    }

    jobs
}
