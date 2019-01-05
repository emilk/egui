/// Outputs render info in a format suitable for e.g. OpenGL.
use crate::{
    font::Font,
    math::{remap, Vec2, TAU},
    types::{Color, PaintCmd},
};

#[derive(Clone, Copy, Debug, Default)]
pub struct Vertex {
    /// Pixel coordinated
    pub x: f32,
    /// Pixel coordinated
    pub y: f32,
    /// Texel indices into the texture
    pub u: u16,
    /// Texel indices into the texture
    pub v: u16,
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
            x: bottom_right.x,
            y: top_left.y,
            u: bottom_right.u,
            v: top_left.v,
            color: top_left.color,
        };
        let botom_left = Vertex {
            x: top_left.x,
            y: bottom_right.y,
            u: top_left.u,
            v: bottom_right.v,
            color: top_left.color,
        };
        self.vertices.push(top_left);
        self.vertices.push(top_right);
        self.vertices.push(botom_left);
        self.vertices.push(bottom_right);
    }

    pub fn fill_closed_path(&mut self, points: &[Vec2], normals: &[Vec2], color: Color) {
        self.vertices.extend(points.iter().map(|p| Vertex {
            x: p.x,
            y: p.y,
            u: 0,
            v: 0,
            color,
        }));
        // TODO: use normals for anti-aliasing
        assert_eq!(points.len(), normals.len());
        let n = points.len() as u32;
        let idx = self.vertices.len() as u32;
        for i in 2..n {
            self.indices.push(idx);
            self.indices.push(idx + i - 1);
            self.indices.push(idx + i);
        }
    }

    pub fn draw_closed_path(
        &mut self,
        points: &[Vec2],
        normals: &[Vec2],
        width: f32,
        color: Color,
    ) {
        // TODO: anti-aliasing
        assert_eq!(points.len(), normals.len());
        let n = points.len() as u32;
        let hw = width / 2.0;

        let idx = self.vertices.len() as u32;
        for i in 0..n {
            self.indices.push(idx + (2 * i + 0) % (2 * n));
            self.indices.push(idx + (2 * i + 1) % (2 * n));
            self.indices.push(idx + (2 * i + 2) % (2 * n));
            self.indices.push(idx + (2 * i + 2) % (2 * n));
            self.indices.push(idx + (2 * i + 1) % (2 * n));
            self.indices.push(idx + (2 * i + 3) % (2 * n));
        }

        for (p, n) in points.iter().zip(normals) {
            self.vertices.push(Vertex {
                x: p.x + hw * n.x,
                y: p.y + hw * n.x,
                u: 0,
                v: 0,
                color,
            });
            self.vertices.push(Vertex {
                x: p.x - hw * n.x,
                y: p.y - hw * n.x,
                u: 0,
                v: 0,
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
        // let mut path_points = Vec::new();
        // let mut path_normals = Vec::new();

        let mut frame = Frame::default();
        for cmd in commands {
            match cmd {
                PaintCmd::Circle {
                    center,
                    fill_color,
                    outline,
                    radius,
                } => {
                    let n = 64; // TODO: parameter
                    if let Some(color) = fill_color {
                        let idx = frame.vertices.len() as u32;
                        for i in 2..n {
                            frame.indices.push(idx);
                            frame.indices.push(idx + i - 1);
                            frame.indices.push(idx + i);
                        }

                        for i in 0..n {
                            let angle = remap(i as f32, 0.0, n as f32, 0.0, TAU);
                            frame.vertices.push(Vertex {
                                x: center.x + radius * angle.cos(),
                                y: center.y + radius * angle.sin(),
                                u: 0,
                                v: 0,
                                color: *color,
                            });
                        }
                    }
                    if let Some(_outline) = outline {
                        // TODO
                    }
                }
                PaintCmd::Clear { fill_color } => {
                    frame.clear_color = Some(*fill_color);
                }
                PaintCmd::Line { .. } => {} // TODO
                PaintCmd::Rect {
                    fill_color,
                    outline,
                    pos,
                    size,
                    ..
                } => {
                    // TODO: rounded corners
                    // TODO: anti-aliasing
                    // TODO: FilledRect and RectOutline as separate commands?
                    if let Some(color) = fill_color {
                        let vert = |pos: Vec2| Vertex {
                            x: pos.x,
                            y: pos.y,
                            u: 0,
                            v: 0,
                            color: *color,
                        };
                        frame.add_rect(vert(*pos), vert(*pos + *size));
                    }
                    if let Some(outline) = outline {
                        let vert = |x, y| Vertex {
                            x,
                            y,
                            u: 0,
                            v: 0,
                            color: outline.color,
                        };

                        // Draw this counter-clockwise from top-left corner,
                        // outer to inner on each step.
                        let hw = outline.width / 2.0;

                        let idx = frame.vertices.len() as u32;
                        for i in 0..4 {
                            frame.indices.push(idx + (2 * i + 0) % 8);
                            frame.indices.push(idx + (2 * i + 1) % 8);
                            frame.indices.push(idx + (2 * i + 2) % 8);
                            frame.indices.push(idx + (2 * i + 2) % 8);
                            frame.indices.push(idx + (2 * i + 1) % 8);
                            frame.indices.push(idx + (2 * i + 3) % 8);
                        }

                        let min = *pos;
                        let max = *pos + *size;
                        frame.vertices.push(vert(min.x - hw, min.y - hw));
                        frame.vertices.push(vert(min.x + hw, min.y + hw));
                        frame.vertices.push(vert(max.x + hw, min.y - hw));
                        frame.vertices.push(vert(max.x - hw, min.y + hw));
                        frame.vertices.push(vert(max.x + hw, max.y + hw));
                        frame.vertices.push(vert(max.x - hw, max.y - hw));
                        frame.vertices.push(vert(min.x - hw, max.y + hw));
                        frame.vertices.push(vert(min.x + hw, max.y - hw));
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
                                x: pos.x + x_offset + (glyph.offset_x as f32),
                                y: pos.y + (glyph.offset_y as f32),
                                u: glyph.min_x,
                                v: glyph.min_y,
                                color: *color,
                            };
                            let bottom_right = Vertex {
                                x: top_left.x + (1 + glyph.max_x - glyph.min_x) as f32,
                                y: top_left.y + (1 + glyph.max_y - glyph.min_y) as f32,
                                u: glyph.max_x + 1,
                                v: glyph.max_y + 1,
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
