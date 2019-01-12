use std::sync::Arc;

const ANTI_ALIAS: bool = true;
const AA_SIZE: f32 = 1.0;

/// Outputs render info in a format suitable for e.g. OpenGL.
use crate::{
    fonts::Fonts,
    math::{remap, vec2, Vec2, TAU},
    types::{Color, PaintCmd},
};

#[derive(Clone, Copy, Debug, Default)]
pub struct Vertex {
    /// Pixel coordinates
    pub pos: Vec2,
    /// Texel indices into the texture
    pub uv: (u16, u16),
    /// sRGBA
    pub color: Color,
}

#[derive(Clone, Debug, Default)]
pub struct Frame {
    pub clear_color: Option<Color>,
    /// Draw as triangles (i.e. the length is a multiple of three)
    pub indices: Vec<u32>,
    pub vertices: Vec<Vertex>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum PathType {
    Open,
    Closed,
}
use self::PathType::*;

impl Frame {
    fn triangle(&mut self, a: u32, b: u32, c: u32) {
        self.indices.push(a);
        self.indices.push(b);
        self.indices.push(c);
    }

    /// Uniformly colored rectangle
    pub fn add_rect(&mut self, top_left: Vertex, bottom_right: Vertex) {
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

    pub fn fill_closed_path(&mut self, points: &[Vec2], normals: &[Vec2], color: Color) {
        assert_eq!(points.len(), normals.len());
        let n = points.len() as u32;
        let vert = |pos, color| Vertex {
            pos,
            uv: (0, 0),
            color,
        };
        if ANTI_ALIAS {
            let color_outer = color.transparent();
            let idx_inner = self.vertices.len() as u32;
            let idx_outer = idx_inner + 1;
            for i in 2..n {
                self.triangle(idx_inner + 2 * (i - 1), idx_inner, idx_inner + 2 * i);
            }
            let mut i0 = n - 1;
            for i1 in 0..n {
                let dm = normals[i1 as usize] * AA_SIZE * 0.5;
                self.vertices.push(vert(points[i1 as usize] - dm, color));
                self.vertices
                    .push(vert(points[i1 as usize] + dm, color_outer));
                self.triangle(idx_inner + i1 * 2, idx_inner + i0 * 2, idx_outer + 2 * i0);
                self.triangle(idx_outer + i0 * 2, idx_outer + i1 * 2, idx_inner + 2 * i1);
                i0 = i1;
            }
        } else {
            let idx = self.vertices.len() as u32;
            self.vertices
                .extend(points.iter().map(|&pos| vert(pos, color)));
            for i in 2..n {
                self.triangle(idx, idx + i - 1, idx + i);
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
        let idx = self.vertices.len() as u32;

        let vert = |pos, color| Vertex {
            pos,
            uv: (0, 0),
            color,
        };

        if ANTI_ALIAS {
            let color_outer = color.transparent();
            let thin_line = width <= 1.0;
            let mut color_inner = color;
            if thin_line {
                // Fade out as it gets thinner:
                color_inner.a = (color_inner.a as f32 * width).round() as u8;
            }
            // TODO: line caps ?
            let mut i0 = n - 1;
            for i1 in 0..n {
                let connect_with_previous = path_type == PathType::Closed || i1 > 0;
                if thin_line {
                    let p = points[i1 as usize];
                    let n = normals[i1 as usize];
                    self.vertices.push(vert(p + n * AA_SIZE, color_outer));
                    self.vertices.push(vert(p, color_inner));
                    self.vertices.push(vert(p - n * AA_SIZE, color_outer));

                    if connect_with_previous {
                        self.triangle(idx + 3 * i0 + 0, idx + 3 * i0 + 1, idx + 3 * i1 + 0);
                        self.triangle(idx + 3 * i0 + 1, idx + 3 * i1 + 0, idx + 3 * i1 + 1);

                        self.triangle(idx + 3 * i0 + 1, idx + 3 * i0 + 2, idx + 3 * i1 + 1);
                        self.triangle(idx + 3 * i0 + 2, idx + 3 * i1 + 1, idx + 3 * i1 + 2);
                    }
                } else {
                    let hw = (width - AA_SIZE) * 0.5;
                    let p = points[i1 as usize];
                    let n = normals[i1 as usize];
                    self.vertices
                        .push(vert(p + n * (hw + AA_SIZE), color_outer));
                    self.vertices.push(vert(p + n * (hw + 0.0), color_inner));
                    self.vertices.push(vert(p - n * (hw + 0.0), color_inner));
                    self.vertices
                        .push(vert(p - n * (hw + AA_SIZE), color_outer));

                    if connect_with_previous {
                        self.triangle(idx + 4 * i0 + 0, idx + 4 * i0 + 1, idx + 4 * i1 + 0);
                        self.triangle(idx + 4 * i0 + 1, idx + 4 * i1 + 0, idx + 4 * i1 + 1);

                        self.triangle(idx + 4 * i0 + 1, idx + 4 * i0 + 2, idx + 4 * i1 + 1);
                        self.triangle(idx + 4 * i0 + 2, idx + 4 * i1 + 1, idx + 4 * i1 + 2);

                        self.triangle(idx + 4 * i0 + 2, idx + 4 * i0 + 3, idx + 4 * i1 + 2);
                        self.triangle(idx + 4 * i0 + 3, idx + 4 * i1 + 2, idx + 4 * i1 + 3);
                    }
                }
                i0 = i1;
            }
        } else {
            let last_index = if path_type == Closed { n } else { n - 1 };
            for i in 0..last_index {
                self.triangle(
                    idx + (2 * i + 0) % (2 * n),
                    idx + (2 * i + 1) % (2 * n),
                    idx + (2 * i + 2) % (2 * n),
                );
                self.triangle(
                    idx + (2 * i + 2) % (2 * n),
                    idx + (2 * i + 1) % (2 * n),
                    idx + (2 * i + 3) % (2 * n),
                );
            }

            for (&p, &n) in points.iter().zip(normals) {
                self.vertices.push(vert(p + hw * n, color));
                self.vertices.push(vert(p - hw * n, color));
            }
        }
    }
}

#[derive(Clone)]
pub struct Painter {
    fonts: Arc<Fonts>,
}

impl Painter {
    pub fn new(fonts: Arc<Fonts>) -> Painter {
        Painter { fonts }
    }

    pub fn paint(&self, commands: &[PaintCmd]) -> Frame {
        let mut path_points = Vec::new();
        let mut path_normals = Vec::new();

        let mut frame = Frame::default();
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
                        frame.fill_closed_path(&path_points, &path_normals, *color);
                    }
                    if let Some(outline) = outline {
                        frame.paint_path(
                            Closed,
                            &path_points,
                            &path_normals,
                            outline.color,
                            outline.width,
                        );
                    }
                }
                PaintCmd::Clear { fill_color } => {
                    frame.clear_color = Some(*fill_color);
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

                        frame.paint_path(Open, &path_points, &path_normals, *color, *width);
                    }
                }
                PaintCmd::Rect {
                    corner_radius,
                    fill_color,
                    outline,
                    pos,
                    size,
                } => {
                    path_points.clear();
                    path_normals.clear();

                    let min = *pos;
                    let max = *pos + *size;

                    let cr = corner_radius.min(size.x * 0.5).min(size.y * 0.5);

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
                        frame.fill_closed_path(&path_points, &path_normals, *color);
                    }
                    if let Some(outline) = outline {
                        frame.paint_path(
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
                    let font = &self.fonts[*text_style];
                    for (c, x_offset) in text.chars().zip(x_offsets.iter()) {
                        if let Some(glyph) = font.uv_rect(c) {
                            let mut top_left = Vertex {
                                pos: *pos
                                    + vec2(
                                        x_offset + (glyph.offset.0 as f32),
                                        glyph.offset.1 as f32,
                                    ),
                                uv: (glyph.min.0, glyph.min.1),
                                color: *color,
                            };
                            top_left.pos.x = top_left.pos.x.round(); // Pixel-perfection.
                            top_left.pos.y = top_left.pos.y.round(); // Pixel-perfection.
                            let bottom_right = Vertex {
                                pos: top_left.pos
                                    + vec2(
                                        (1 + glyph.max.0 - glyph.min.0) as f32,
                                        (1 + glyph.max.1 - glyph.min.1) as f32,
                                    ),
                                uv: (glyph.max.0 + 1, glyph.max.1 + 1),
                                color: *color,
                            };
                            frame.add_rect(top_left, bottom_right);
                        }
                    }
                }
            }
        }
        frame
    }
}
