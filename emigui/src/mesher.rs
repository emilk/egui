#![allow(clippy::identity_op)]

/// Outputs render info in a format suitable for e.g. OpenGL.
use crate::{
    fonts::Fonts,
    math::{remap, vec2, Vec2, TAU},
    types::{Color, PaintCmd},
};

#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct Vertex {
    /// Pixel coordinates
    pub pos: Vec2,
    /// Texel indices into the texture
    pub uv: (u16, u16),
    /// sRGBA
    pub color: Color,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Mesh {
    /// Draw as triangles (i.e. the length is a multiple of three)
    pub indices: Vec<u32>,
    pub vertices: Vec<Vertex>,
}

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
            pos: vec2(bottom_right.pos.x, top_left.pos.y),
            uv: (bottom_right.uv.0, top_left.uv.1),
            color: top_left.color,
        };
        let botom_left = Vertex {
            pos: vec2(top_left.pos.x, bottom_right.pos.y),
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
        return output;
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum PathType {
    Open,
    Closed,
}
use self::PathType::*;

pub struct Mesher {
    pub anti_alias: bool,
    pub aa_size: f32,

    /// Where the output goes
    pub mesh: Mesh,
}

impl Mesher {
    pub fn new(pixels_per_point: f32) -> Mesher {
        Mesher {
            anti_alias: true,
            aa_size: 1.0 / pixels_per_point,
            mesh: Default::default(),
        }
    }

    pub fn fill_closed_path(&mut self, points: &[Vec2], normals: &[Vec2], color: Color) {
        assert_eq!(points.len(), normals.len());
        let n = points.len() as u32;
        let vert = |pos, color| Vertex {
            pos,
            uv: (0, 0),
            color,
        };
        let mesh = &mut self.mesh;
        if self.anti_alias {
            let color_outer = color.transparent();
            let idx_inner = mesh.vertices.len() as u32;
            let idx_outer = idx_inner + 1;
            for i in 2..n {
                mesh.triangle(idx_inner + 2 * (i - 1), idx_inner, idx_inner + 2 * i);
            }
            let mut i0 = n - 1;
            for i1 in 0..n {
                let dm = normals[i1 as usize] * self.aa_size * 0.5;
                mesh.vertices.push(vert(points[i1 as usize] - dm, color));
                mesh.vertices
                    .push(vert(points[i1 as usize] + dm, color_outer));
                mesh.triangle(idx_inner + i1 * 2, idx_inner + i0 * 2, idx_outer + 2 * i0);
                mesh.triangle(idx_outer + i0 * 2, idx_outer + i1 * 2, idx_inner + 2 * i1);
                i0 = i1;
            }
        } else {
            let idx = mesh.vertices.len() as u32;
            mesh.vertices
                .extend(points.iter().map(|&pos| vert(pos, color)));
            for i in 2..n {
                mesh.triangle(idx, idx + i - 1, idx + i);
            }
        }
    }

    pub fn paint_path(
        &mut self,
        path_type: PathType,
        points: &[Vec2],
        normals: &[Vec2],
        color: Color,
        width: f32,
    ) {
        assert_eq!(points.len(), normals.len());
        let n = points.len() as u32;
        let hw = width / 2.0;
        let idx = self.mesh.vertices.len() as u32;

        let vert = |pos, color| Vertex {
            pos,
            uv: (0, 0),
            color,
        };
        let mesh = &mut self.mesh;

        if self.anti_alias {
            let color_outer = color.transparent();
            let thin_line = width <= 1.0;
            let mut color_inner = color;
            if thin_line {
                // Fade out as it gets thinner:
                color_inner.a = (f32::from(color_inner.a) * width).round() as u8;
            }
            // TODO: line caps ?
            let mut i0 = n - 1;
            for i1 in 0..n {
                let connect_with_previous = path_type == PathType::Closed || i1 > 0;
                if thin_line {
                    let p = points[i1 as usize];
                    let n = normals[i1 as usize];
                    mesh.vertices.push(vert(p + n * self.aa_size, color_outer));
                    mesh.vertices.push(vert(p, color_inner));
                    mesh.vertices.push(vert(p - n * self.aa_size, color_outer));

                    if connect_with_previous {
                        mesh.triangle(idx + 3 * i0 + 0, idx + 3 * i0 + 1, idx + 3 * i1 + 0);
                        mesh.triangle(idx + 3 * i0 + 1, idx + 3 * i1 + 0, idx + 3 * i1 + 1);

                        mesh.triangle(idx + 3 * i0 + 1, idx + 3 * i0 + 2, idx + 3 * i1 + 1);
                        mesh.triangle(idx + 3 * i0 + 2, idx + 3 * i1 + 1, idx + 3 * i1 + 2);
                    }
                } else {
                    let hw = (width - self.aa_size) * 0.5;
                    let p = points[i1 as usize];
                    let n = normals[i1 as usize];
                    mesh.vertices
                        .push(vert(p + n * (hw + self.aa_size), color_outer));
                    mesh.vertices.push(vert(p + n * (hw + 0.0), color_inner));
                    mesh.vertices.push(vert(p - n * (hw + 0.0), color_inner));
                    mesh.vertices
                        .push(vert(p - n * (hw + self.aa_size), color_outer));

                    if connect_with_previous {
                        mesh.triangle(idx + 4 * i0 + 0, idx + 4 * i0 + 1, idx + 4 * i1 + 0);
                        mesh.triangle(idx + 4 * i0 + 1, idx + 4 * i1 + 0, idx + 4 * i1 + 1);

                        mesh.triangle(idx + 4 * i0 + 1, idx + 4 * i0 + 2, idx + 4 * i1 + 1);
                        mesh.triangle(idx + 4 * i0 + 2, idx + 4 * i1 + 1, idx + 4 * i1 + 2);

                        mesh.triangle(idx + 4 * i0 + 2, idx + 4 * i0 + 3, idx + 4 * i1 + 2);
                        mesh.triangle(idx + 4 * i0 + 3, idx + 4 * i1 + 2, idx + 4 * i1 + 3);
                    }
                }
                i0 = i1;
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

            for (&p, &n) in points.iter().zip(normals) {
                mesh.vertices.push(vert(p + hw * n, color));
                mesh.vertices.push(vert(p - hw * n, color));
            }
        }
    }

    pub fn paint(&mut self, fonts: &Fonts, commands: &[PaintCmd]) {
        let mut path_points = Vec::new();
        let mut path_normals = Vec::new();

        for cmd in commands {
            match cmd {
                PaintCmd::Circle {
                    center,
                    fill_color,
                    outline,
                    radius,
                } => {
                    path_points.clear();
                    path_normals.clear();

                    let n = 32; // TODO: parameter
                    for i in 0..n {
                        let angle = remap(i as f32, 0.0, n as f32, 0.0, TAU);
                        let normal = vec2(angle.cos(), angle.sin());
                        path_normals.push(normal);
                        path_points.push(*center + *radius * normal);
                    }

                    if let Some(color) = fill_color {
                        self.fill_closed_path(&path_points, &path_normals, *color);
                    }
                    if let Some(outline) = outline {
                        self.paint_path(
                            Closed,
                            &path_points,
                            &path_normals,
                            outline.color,
                            outline.width,
                        );
                    }
                }
                PaintCmd::Mesh(cmd_frame) => {
                    self.mesh.append(cmd_frame);
                }
                PaintCmd::Line {
                    points,
                    color,
                    width,
                } => {
                    let n = points.len();
                    if n >= 2 {
                        path_points = points.clone();
                        path_normals.clear();

                        path_normals.push((path_points[1] - path_points[0]).normalized().rot90());
                        for i in 1..n - 1 {
                            let n0 = (path_points[i] - path_points[i - 1]).normalized().rot90();
                            let n1 = (path_points[i + 1] - path_points[i]).normalized().rot90();
                            let v = (n0 + n1) / 2.0;
                            let normal = v / v.length_sq();
                            path_normals.push(normal); // TODO: handle VERY sharp turns better
                        }
                        path_normals.push(
                            (path_points[n - 1] - path_points[n - 2])
                                .normalized()
                                .rot90(),
                        );

                        self.paint_path(Open, &path_points, &path_normals, *color, *width);
                    }
                }
                PaintCmd::Rect {
                    corner_radius,
                    fill_color,
                    outline,
                    rect,
                } => {
                    path_points.clear();
                    path_normals.clear();

                    let min = rect.min();
                    let max = rect.max();

                    let cr = corner_radius.min(rect.size.x * 0.5).min(rect.size.y * 0.5);

                    if cr <= 0.0 {
                        path_points.push(vec2(min.x, min.y));
                        path_normals.push(vec2(-1.0, -1.0));
                        path_points.push(vec2(max.x, min.y));
                        path_normals.push(vec2(1.0, -1.0));
                        path_points.push(vec2(max.x, max.y));
                        path_normals.push(vec2(1.0, 1.0));
                        path_points.push(vec2(min.x, max.y));
                        path_normals.push(vec2(-1.0, 1.0));
                    } else {
                        let n = 8;

                        let mut add_arc = |c, quadrant| {
                            let quadrant = quadrant as f32;

                            const RIGHT_ANGLE: f32 = TAU / 4.0;
                            for i in 0..=n {
                                let angle = remap(
                                    i as f32,
                                    0.0,
                                    n as f32,
                                    quadrant * RIGHT_ANGLE,
                                    (quadrant + 1.0) * RIGHT_ANGLE,
                                );
                                let normal = vec2(angle.cos(), angle.sin());
                                path_points.push(c + cr * normal);
                                path_normals.push(normal);
                            }
                        };

                        add_arc(vec2(max.x - cr, max.y - cr), 0);
                        add_arc(vec2(min.x + cr, max.y - cr), 1);
                        add_arc(vec2(min.x + cr, min.y + cr), 2);
                        add_arc(vec2(max.x - cr, min.y + cr), 3);
                    }

                    if let Some(color) = fill_color {
                        self.fill_closed_path(&path_points, &path_normals, *color);
                    }
                    if let Some(outline) = outline {
                        self.paint_path(
                            Closed,
                            &path_points,
                            &path_normals,
                            outline.color,
                            outline.width,
                        );
                    }
                }
                PaintCmd::Text {
                    color,
                    pos,
                    text,
                    text_style,
                    x_offsets,
                } => {
                    let font = &fonts[*text_style];
                    for (c, x_offset) in text.chars().zip(x_offsets.iter()) {
                        if let Some(glyph) = font.uv_rect(c) {
                            let mut top_left = Vertex {
                                pos: *pos + glyph.offset + vec2(*x_offset, 0.0),
                                uv: glyph.min,
                                color: *color,
                            };
                            top_left.pos.x = font.round_to_pixel(top_left.pos.x); // Pixel-perfection.
                            top_left.pos.y = font.round_to_pixel(top_left.pos.y); // Pixel-perfection.
                            let bottom_right = Vertex {
                                pos: top_left.pos + glyph.size,
                                uv: glyph.max,
                                color: *color,
                            };
                            self.mesh.add_rect(top_left, bottom_right);
                        }
                    }
                }
            }
        }
    }
}
