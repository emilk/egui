/// Outputs render info in a format suitable for e.g. OpenGL.
use crate::{
    font::Font,
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
    /// One big triangle strip
    pub indices: Vec<u32>,
    pub vertices: Vec<Vertex>,
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
    pub fn texture(&self) -> (usize, usize, &[u8]) {
        self.font.texture()
    }

    pub fn paint(&self, commands: &[PaintCmd]) -> Frame {
        let mut frame = Frame::default();
        for cmd in commands {
            match cmd {
                PaintCmd::Circle { .. } => {} // TODO
                PaintCmd::Clear { fill_color } => {
                    frame.clear_color = Some(*fill_color);
                }
                PaintCmd::Line { .. } => {} // TODO
                PaintCmd::Rect {
                    pos,
                    size,
                    fill_color,
                    ..
                } => {
                    // TODO: rounded corners, colors etc.
                    let idx = frame.vertices.len() as u32;
                    frame.indices.push(idx + 0);
                    frame.indices.push(idx + 0);
                    frame.indices.push(idx + 1);
                    frame.indices.push(idx + 2);
                    frame.indices.push(idx + 3);
                    frame.indices.push(idx + 3);

                    let vert = |x, y| Vertex {
                        x,
                        y,
                        u: 0,
                        v: 0,
                        color: fill_color.unwrap_or(Color::WHITE),
                    };

                    frame.vertices.push(vert(pos.x, pos.y));
                    frame.vertices.push(vert(pos.x + size.x, pos.y));
                    frame.vertices.push(vert(pos.x, pos.y + size.y));
                    frame.vertices.push(vert(pos.x + size.x, pos.y + size.y));
                }
                PaintCmd::Text { .. } => {} // TODO
            }
        }
        frame
    }
}
