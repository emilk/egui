#![allow(clippy::identity_op)]

/// Outputs render info in a format suitable for e.g. OpenGL.
use crate::{
    color::{self, srgba, Color},
    fonts::Fonts,
    math::*,
    types::PaintCmd,
    Outline,
};

const WHITE_UV: (u16, u16) = (1, 1);

#[derive(Clone, Copy, Debug, Default, serde_derive::Serialize)]
pub struct Vertex {
    /// Pixel coordinates
    pub pos: Pos2,
    /// Texel indices into the texture
    pub uv: (u16, u16),
    /// sRGBA, premultiplied alpha
    pub color: Color,
}

#[derive(Clone, Debug, Default, serde_derive::Serialize)]
pub struct Mesh {
    /// Draw as triangles (i.e. the length is a multiple of three)
    pub indices: Vec<u32>,
    pub vertices: Vec<Vertex>,
}

/// Grouped by clip rectangles, in pixel coordinates
pub type PaintBatches = Vec<(Rect, Mesh)>;

// ----------------------------------------------------------------------------

impl Mesh {
    pub fn append(&mut self, mesh: &Mesh) {
        let index_offset = self.vertices.len() as u32;
        for index in &mesh.indices {
            self.indices.push(index_offset + index);
        }
        self.vertices.extend(mesh.vertices.iter());
    }

    fn triangle(&mut self, a: u32, b: u32, c: u32) {
        self.indices.push(a);
        self.indices.push(b);
        self.indices.push(c);
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

    /// Split a large mesh into many small.
    /// All the returned meshes will have indices that fit into u16.
    pub fn split_to_u16(self) -> Vec<Mesh> {
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

            output.push(Mesh {
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

#[derive(Clone, Debug, Default)]
pub struct Path(Vec<PathPoint>);

impl Path {
    pub fn from_point_loop(points: &[Pos2]) -> Self {
        let mut path = Self::default();
        path.add_line_loop(points);
        path
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    #[inline(always)]
    pub fn add_point(&mut self, pos: Pos2, normal: Vec2) {
        self.0.push(PathPoint { pos, normal });
    }

    pub fn add_circle(&mut self, center: Pos2, radius: f32) {
        let n = (radius * 2.0).round() as i32; // TODO: tweak a bit more
        let n = clamp(n, 4..=64);
        for i in 0..n {
            let angle = remap(i as f32, 0.0..=n as f32, 0.0..=TAU);
            let normal = vec2(angle.cos(), angle.sin());
            self.add_point(center + radius * normal, normal);
        }
    }

    pub fn add_line_segment(&mut self, points: [Pos2; 2]) {
        let normal = (points[1] - points[0]).normalized().rot90();
        self.add_point(points[0], normal);
        self.add_point(points[1], normal);
    }

    // TODO: make it clear it is an open (non-closed) thing.
    pub fn add_line(&mut self, points: &[Pos2]) {
        let n = points.len();
        assert!(n >= 2);

        if n == 2 {
            // Common case optimization:
            self.add_line_segment([points[0], points[1]]);
        } else {
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
    ///    quadrant 0, right down
    /// 1 * TAU / 4 = down
    ///    quadrant 1, down left
    /// 2 * TAU / 4 = left
    ///    quadrant 2 left up
    /// 3 * TAU / 4 = up
    ///    quadrant 3 up rigth
    /// 4 * TAU / 4 = right
    pub fn add_circle_quadrant(&mut self, center: Pos2, radius: f32, quadrant: f32) {
        let n = (radius * 0.5).round() as i32; // TODO: tweak a bit more
        let n = clamp(n, 2..=32);
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
pub struct MesherOptions {
    pub anti_alias: bool,
    /// Size of a pixel in points, e.g. 0.5
    pub aa_size: f32,
    pub debug_paint_clip_rects: bool,
}

impl Default for MesherOptions {
    fn default() -> Self {
        Self {
            anti_alias: true,
            aa_size: 1.0,
            debug_paint_clip_rects: false,
        }
    }
}

pub fn fill_closed_path(mesh: &mut Mesh, options: MesherOptions, path: &[PathPoint], color: Color) {
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
        let color_outer = color::TRANSPARENT;
        let idx_inner = mesh.vertices.len() as u32;
        let idx_outer = idx_inner + 1;
        for i in 2..n {
            mesh.triangle(idx_inner + 2 * (i - 1), idx_inner, idx_inner + 2 * i);
        }
        let mut i0 = n - 1;
        for i1 in 0..n {
            let p1 = &path[i1 as usize];
            let dm = p1.normal * options.aa_size * 0.5;
            mesh.vertices.push(vert(p1.pos - dm, color));
            mesh.vertices.push(vert(p1.pos + dm, color_outer));
            mesh.triangle(idx_inner + i1 * 2, idx_inner + i0 * 2, idx_outer + 2 * i0);
            mesh.triangle(idx_outer + i0 * 2, idx_outer + i1 * 2, idx_inner + 2 * i1);
            i0 = i1;
        }
    } else {
        let idx = mesh.vertices.len() as u32;
        mesh.vertices
            .extend(path.iter().map(|p| vert(p.pos, color)));
        for i in 2..n {
            mesh.triangle(idx, idx + i - 1, idx + i);
        }
    }
}

pub fn paint_path(
    mesh: &mut Mesh,
    options: MesherOptions,
    path_type: PathType,
    path: &[PathPoint],
    color: Color,
    width: f32,
) {
    if color == color::TRANSPARENT {
        return;
    }

    let n = path.len() as u32;
    let idx = mesh.vertices.len() as u32;

    let vert = |pos, color| Vertex {
        pos,
        uv: WHITE_UV,
        color,
    };

    if options.anti_alias {
        let color_inner = color;
        let color_outer = color::TRANSPARENT;

        let thin_line = width <= options.aa_size;
        if thin_line {
            /*
            We paint the line using three edges: outer, inner, outer.

            .       o   i   o      outer, inner, outer
            .       |---|          aa_size (pixel width)
            */

            // Fade out as it gets thinner:
            let color_inner = mul_color(color_inner, width / options.aa_size);
            if color_inner == color::TRANSPARENT {
                return;
            }

            let mut i0 = n - 1;
            for i1 in 0..n {
                let connect_with_previous = path_type == PathType::Closed || i1 > 0;
                let p1 = &path[i1 as usize];
                let p = p1.pos;
                let n = p1.normal;
                mesh.vertices
                    .push(vert(p + n * options.aa_size, color_outer));
                mesh.vertices.push(vert(p, color_inner));
                mesh.vertices
                    .push(vert(p - n * options.aa_size, color_outer));

                if connect_with_previous {
                    mesh.triangle(idx + 3 * i0 + 0, idx + 3 * i0 + 1, idx + 3 * i1 + 0);
                    mesh.triangle(idx + 3 * i0 + 1, idx + 3 * i1 + 0, idx + 3 * i1 + 1);

                    mesh.triangle(idx + 3 * i0 + 1, idx + 3 * i0 + 2, idx + 3 * i1 + 1);
                    mesh.triangle(idx + 3 * i0 + 2, idx + 3 * i1 + 1, idx + 3 * i1 + 2);
                }
                i0 = i1;
            }
        } else {
            // TODO: line caps for really thick lines?

            /*
            We paint the line using four edges: outer, inner, inner, outer

            .       o   i     p    i   o   outer, inner, point, inner, outer
            .       |---|                  aa_size (pixel width)
            .         |--------------|     width
            .       |---------|            outer_rad
            .           |-----|            inner_rad
            */

            let mut i0 = n - 1;
            for i1 in 0..n {
                let connect_with_previous = path_type == PathType::Closed || i1 > 0;
                let inner_rad = 0.5 * (width - options.aa_size);
                let outer_rad = 0.5 * (width + options.aa_size);
                let p1 = &path[i1 as usize];
                let p = p1.pos;
                let n = p1.normal;
                mesh.vertices.push(vert(p + n * outer_rad, color_outer));
                mesh.vertices.push(vert(p + n * inner_rad, color_inner));
                mesh.vertices.push(vert(p - n * inner_rad, color_inner));
                mesh.vertices.push(vert(p - n * outer_rad, color_outer));

                if connect_with_previous {
                    mesh.triangle(idx + 4 * i0 + 0, idx + 4 * i0 + 1, idx + 4 * i1 + 0);
                    mesh.triangle(idx + 4 * i0 + 1, idx + 4 * i1 + 0, idx + 4 * i1 + 1);

                    mesh.triangle(idx + 4 * i0 + 1, idx + 4 * i0 + 2, idx + 4 * i1 + 1);
                    mesh.triangle(idx + 4 * i0 + 2, idx + 4 * i1 + 1, idx + 4 * i1 + 2);

                    mesh.triangle(idx + 4 * i0 + 2, idx + 4 * i0 + 3, idx + 4 * i1 + 2);
                    mesh.triangle(idx + 4 * i0 + 3, idx + 4 * i1 + 2, idx + 4 * i1 + 3);
                }
                i0 = i1;
            }
        }
    } else {
        let last_index = if path_type == Closed { n } else { n - 1 };
        for i in 0..last_index {
            mesh.triangle(
                idx + (2 * i + 0) % (2 * n),
                idx + (2 * i + 1) % (2 * n),
                idx + (2 * i + 2) % (2 * n),
            );
            mesh.triangle(
                idx + (2 * i + 2) % (2 * n),
                idx + (2 * i + 1) % (2 * n),
                idx + (2 * i + 3) % (2 * n),
            );
        }

        let thin_line = width <= options.aa_size;
        if thin_line {
            // Fade out thin lines rather than making them thinner
            let radius = options.aa_size / 2.0;
            let color = mul_color(color, width / options.aa_size);
            if color == color::TRANSPARENT {
                return;
            }
            for p in path {
                mesh.vertices.push(vert(p.pos + radius * p.normal, color));
                mesh.vertices.push(vert(p.pos - radius * p.normal, color));
            }
        } else {
            let radius = width / 2.0;
            for p in path {
                mesh.vertices.push(vert(p.pos + radius * p.normal, color));
                mesh.vertices.push(vert(p.pos - radius * p.normal, color));
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

/// path: only used to reuse memory
pub fn mesh_command(
    path: &mut Path,
    options: MesherOptions,
    fonts: &Fonts,
    command: PaintCmd,
    out_mesh: &mut Mesh,
) {
    path.clear();

    match command {
        PaintCmd::Circle {
            center,
            fill_color,
            outline,
            radius,
        } => {
            path.add_circle(center, radius);
            if let Some(color) = fill_color {
                fill_closed_path(out_mesh, options, &path.0, color);
            }
            if let Some(outline) = outline {
                paint_path(
                    out_mesh,
                    options,
                    Closed,
                    &path.0,
                    outline.color,
                    outline.width,
                );
            }
        }
        PaintCmd::Mesh(mesh) => {
            out_mesh.append(&mesh);
        }
        PaintCmd::LineSegment {
            points,
            color,
            width,
        } => {
            path.add_line_segment(points);
            paint_path(out_mesh, options, Open, &path.0, color, width);
        }
        PaintCmd::LinePath {
            points,
            color,
            width,
        } => {
            let n = points.len();
            if n >= 2 {
                path.add_line(&points);
                paint_path(out_mesh, options, Open, &path.0, color, width);
            }
        }
        PaintCmd::Path {
            path,
            closed,
            fill_color,
            outline,
        } => {
            if let Some(fill_color) = fill_color {
                debug_assert!(
                    closed,
                    "You asked to fill a path that is not closed. That makes no sense."
                );
                fill_closed_path(out_mesh, options, &path.0, fill_color);
            }
            if let Some(outline) = outline {
                let typ = if closed { Closed } else { Open };
                paint_path(
                    out_mesh,
                    options,
                    typ,
                    &path.0,
                    outline.color,
                    outline.width,
                );
            }
        }
        PaintCmd::Rect {
            corner_radius,
            fill_color,
            outline,
            mut rect,
        } => {
            // Common bug is to accidentally create an infinitely sized ractangle.
            // Make sure we can visualize that:
            rect.min = rect.min.max(pos2(-1e7, -1e7));
            rect.max = rect.max.min(pos2(1e7, 1e7));

            path.add_rounded_rectangle(rect, corner_radius);
            if let Some(fill_color) = fill_color {
                fill_closed_path(out_mesh, options, &path.0, fill_color);
            }
            if let Some(outline) = outline {
                paint_path(
                    out_mesh,
                    options,
                    Closed,
                    &path.0,
                    outline.color,
                    outline.width,
                );
            }
        }
        PaintCmd::Text {
            pos,
            galley,
            text_style,
            color,
        } => {
            galley.sanity_check();

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
                        out_mesh.add_rect(top_left, bottom_right);
                    }
                }
            }
            assert_eq!(chars.next(), None);
        }
    }
}

pub fn mesh_paint_commands(
    options: MesherOptions,
    fonts: &Fonts,
    commands: Vec<(Rect, PaintCmd)>,
) -> Vec<(Rect, Mesh)> {
    let mut reused_path = Path::default();

    let mut batches = PaintBatches::default();
    for (clip_rect, cmd) in commands {
        // TODO: cull(clip_rect, cmd)

        if batches.is_empty() || batches.last().unwrap().0 != clip_rect {
            batches.push((clip_rect, Mesh::default()));
        }

        let out_mesh = &mut batches.last_mut().unwrap().1;
        mesh_command(&mut reused_path, options, fonts, cmd, out_mesh);
    }

    if options.debug_paint_clip_rects {
        for (clip_rect, mesh) in &mut batches {
            mesh_command(
                &mut reused_path,
                options,
                fonts,
                PaintCmd::Rect {
                    rect: *clip_rect,
                    corner_radius: 0.0,
                    fill_color: None,
                    outline: Some(Outline::new(2.0, srgba(150, 255, 150, 255))),
                },
                mesh,
            )
        }
    }

    batches
}
