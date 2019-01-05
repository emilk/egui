#![allow(unused_variables)]

/// Outputs render info in a format suitable for e.g. OpenGL.
use crate::{
    font::Font,
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
    /// Uniformly colored rectangle
    pub fn add_rect(&mut self, top_left: Vertex, bottom_right: Vertex) {
        let idx = self.vertices.len() as u32;
        self.indices.push(idx + 0);
        self.indices.push(idx + 1);
        self.indices.push(idx + 2);
        self.indices.push(idx + 2);
        self.indices.push(idx + 1);
        self.indices.push(idx + 3);

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
        // TODO: use normals for anti-aliasing
        assert_eq!(points.len(), normals.len());
        let n = points.len() as u32;
        let idx = self.vertices.len() as u32;
        self.vertices.extend(points.iter().map(|&pos| Vertex {
            pos,
            uv: (0, 0),
            color,
        }));
        for i in 2..n {
            self.indices.push(idx);
            self.indices.push(idx + i - 1);
            self.indices.push(idx + i);
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
        // TODO: anti-aliasing
        assert_eq!(points.len(), normals.len());
        let n = points.len() as u32;
        let hw = width / 2.0;

        let idx = self.vertices.len() as u32;
        let last_index = if path_type == Closed { n } else { n - 1 };
        for i in 0..last_index {
            self.indices.push(idx + (2 * i + 0) % (2 * n));
            self.indices.push(idx + (2 * i + 1) % (2 * n));
            self.indices.push(idx + (2 * i + 2) % (2 * n));
            self.indices.push(idx + (2 * i + 2) % (2 * n));
            self.indices.push(idx + (2 * i + 1) % (2 * n));
            self.indices.push(idx + (2 * i + 3) % (2 * n));
        }

        for (&p, &n) in points.iter().zip(normals) {
            self.vertices.push(Vertex {
                pos: p + hw * n,
                uv: (0, 0),
                color,
            });
            self.vertices.push(Vertex {
                pos: p - hw * n,
                uv: (0, 0),
                color,
            });
        }
    }
}

#[derive(Clone)]
pub struct Painter {
    font: Font,
}

impl Painter {
    pub fn new() -> Painter {
        Painter {
            font: Font::new(13),
        }
    }

    /// 8-bit row-major font atlas texture, (width, height, pixels).
    pub fn texture(&self) -> (u16, u16, &[u8]) {
        self.font.texture()
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
                    fill_color,
                    outline,
                    pos,
                    size,
                    ..
                } => {
                    path_points.clear();
                    path_normals.clear();

                    let min = *pos;
                    let max = *pos + *size;

                    // TODO: rounded corners
                    path_points.push(vec2(min.x, min.y));
                    path_normals.push(vec2(-1.0, -1.0));
                    path_points.push(vec2(max.x, min.y));
                    path_normals.push(vec2(1.0, -1.0));
                    path_points.push(vec2(max.x, max.y));
                    path_normals.push(vec2(1.0, 1.0));
                    path_points.push(vec2(min.x, max.y));
                    path_normals.push(vec2(-1.0, 1.0));

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
                    x_offsets,
                } => {
                    for (c, x_offset) in text.chars().zip(x_offsets.iter()) {
                        if let Some(glyph) = self.font.glyph_info(c) {
                            let top_left = Vertex {
                                pos: *pos
                                    + vec2(
                                        x_offset + (glyph.offset_x as f32),
                                        glyph.offset_y as f32,
                                    ),
                                uv: (glyph.min_x, glyph.min_y),
                                color: *color,
                            };
                            let bottom_right = Vertex {
                                pos: top_left.pos
                                    + vec2(
                                        (1 + glyph.max_x - glyph.min_x) as f32,
                                        (1 + glyph.max_y - glyph.min_y) as f32,
                                    ),
                                uv: (glyph.max_x + 1, glyph.max_y + 1),
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
