// #![deny(warnings)]

use {
    emigui::{
        label,
        math::vec2,
        widgets::{Button, Label},
        Emigui, Mesh,
    },
    glium::{glutin, implement_vertex, index::PrimitiveType, program, texture, uniform, Surface},
};

struct Painter {
    program: glium::Program,
    texture: texture::texture2d::Texture2d,
    current_texture_id: Option<u64>,
}

impl Painter {
    fn new(facade: &glium::backend::Facade) -> Painter {
        let program = program!(facade,
            140 => {
                    vertex: "
                        #version 140
                        uniform vec2 u_screen_size;
                        uniform vec2 u_tex_size;
                        in vec2 a_pos;
                        in vec4 a_color;
                        in vec2 a_tc;
                        out vec4 v_color;
                        out vec2 v_tc;
                        void main() {
                            gl_Position = vec4(
                                2.0 * a_pos.x / u_screen_size.x - 1.0,
                                1.0 - 2.0 * a_pos.y / u_screen_size.y,
                                0.0,
                                1.0);
                            v_color = a_color / 255.0;
                            v_tc = a_tc / u_tex_size;
                        }
                    ",

                    fragment: "
                        #version 140
                        uniform sampler2D u_sampler;
                        in vec4 v_color;
                        in vec2 v_tc;
                        out vec4 f_color;
                        void main() {
                            f_color = v_color;
                            f_color.a *= texture(u_sampler, v_tc).r;
                        }
                    "
            },

            110 => {
                    vertex: "
                        #version 110
                        uniform vec2 u_screen_size;
                        uniform vec2 u_tex_size;
                        attribute vec2 a_pos;
                        attribute vec4 a_color;
                        attribute vec2 a_tc;
                        varying vec4 v_color;
                        varying vec2 v_tc;
                        void main() {
                            gl_Position = vec4(
                                2.0 * a_pos.x / u_screen_size.x - 1.0,
                                1.0 - 2.0 * a_pos.y / u_screen_size.y,
                                0.0,
                                1.0);
                            v_color = a_color / 255.0;
                            v_tc = a_tc / u_tex_size;
                        }
                    ",

                    fragment: "
                        #version 110
                        uniform sampler2D u_sampler;
                        varying vec4 v_color;
                        varying vec2 v_tc;
                        void main() {
                            gl_FragColor = v_color;
                            gl_FragColor.a *= texture2D(u_sampler, v_tc).r;
                        }
                    ",
            },

            100 => {
                    vertex: "
                        #version 100
                        uniform mediump vec2 u_screen_size;
                        uniform mediump vec2 u_tex_size;
                        attribute mediump vec2 a_pos;
                        attribute mediump vec4 a_color;
                        attribute mediump vec2 a_tc;
                        varying mediump vec4 v_color;
                        varying mediump vec2 v_tc;
                        void main() {
                            gl_Position = vec4(
                                2.0 * a_pos.x / u_screen_size.x - 1.0,
                                1.0 - 2.0 * a_pos.y / u_screen_size.y,
                                0.0,
                                1.0);
                            v_color = a_color / 255.0;
                            v_tc = a_tc / u_tex_size;
                        }
                    ",

                    fragment: "
                        #version 100
                        uniform sampler2D u_sampler;
                        varying mediump vec4 v_color;
                        varying mediump vec2 v_tc;
                        void main() {
                            gl_FragColor = v_color;
                            gl_FragColor.a *= texture2D(u_sampler, v_tc).r;
                        }
                    ",
            },
        )
        .unwrap();

        let pixels = vec![vec![255u8, 0u8], vec![0u8, 255u8]];
        let format = texture::UncompressedFloatFormat::U8;
        let mipmaps = texture::MipmapsOption::NoMipmap;
        let texture =
            texture::texture2d::Texture2d::with_format(facade, pixels, format, mipmaps).unwrap();

        Painter {
            program,
            texture,
            current_texture_id: None,
        }
    }

    fn upload_texture(&mut self, facade: &glium::backend::Facade, texture: &emigui::Texture) {
        if self.current_texture_id == Some(texture.id) {
            return; // No change
        }

        let pixels: Vec<Vec<u8>> = texture
            .pixels
            .chunks(texture.width as usize)
            .map(|row| row.to_vec())
            .collect();

        let format = texture::UncompressedFloatFormat::U8;
        let mipmaps = texture::MipmapsOption::NoMipmap;
        self.texture =
            texture::texture2d::Texture2d::with_format(facade, pixels, format, mipmaps).unwrap();
        self.current_texture_id = Some(texture.id);
    }

    fn paint(&mut self, display: &glium::Display, mesh: Mesh, texture: &emigui::Texture) {
        self.upload_texture(display, texture);

        let vertex_buffer = {
            #[derive(Copy, Clone)]
            struct Vertex {
                a_pos: [f32; 2],
                a_color: [u8; 4],
                a_tc: [u16; 2],
            }
            implement_vertex!(Vertex, a_pos, a_color, a_tc);

            let vertices: Vec<Vertex> = mesh
                .vertices
                .iter()
                .map(|v| Vertex {
                    a_pos: [v.pos.x, v.pos.y],
                    a_color: [v.color.r, v.color.g, v.color.b, v.color.a],
                    a_tc: [v.uv.0, v.uv.1],
                })
                .collect();

            glium::VertexBuffer::new(display, &vertices).unwrap()
        };

        let indices: Vec<u16> = mesh.indices.iter().map(|idx| *idx as u16).collect();

        let index_buffer =
            glium::IndexBuffer::new(display, PrimitiveType::TrianglesList, &indices).unwrap();

        let (width, height) = display.get_framebuffer_dimensions();

        let uniforms = uniform! {
            u_screen_size: [width as f32, height as f32],
            u_tex_size: [texture.width as f32, texture.height as f32],
            u_sampler: &self.texture,
        };

        let params = glium::DrawParameters {
            blend: glium::Blend::alpha_blending(),
            ..Default::default()
        };

        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 0.0);
        target
            .draw(
                &vertex_buffer,
                &index_buffer,
                &self.program,
                &uniforms,
                &params,
            )
            .unwrap();
        target.finish().unwrap();
    }
}

fn main() {
    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new().with_title("Emigui example");
    let context = glutin::ContextBuilder::new();
    let display = glium::Display::new(window, context, &events_loop).unwrap();

    let pixels_per_point = display.gl_window().get_hidpi_factor() as f32;

    let mut emigui = Emigui::new(pixels_per_point);
    let mut painter = Painter::new(&display);

    let mut raw_input = emigui::RawInput {
        mouse_down: false,
        mouse_pos: None,
        screen_size: {
            let (width, height) = display.get_framebuffer_dimensions();
            vec2(width as f32, height as f32)
        },
        pixels_per_point,
    };

    let mut paint = |raw_input| {
        emigui.new_frame(raw_input);
        let mut region = emigui.whole_screen_region();
        let mut region = region.left_column(region.width().min(480.0));
        region.add(label!("Emigui!").text_style(emigui::TextStyle::Heading));
        let exit = region.add(Button::new("Quit")).clicked;
        emigui.example(&mut region);
        let mesh = emigui.paint();
        painter.paint(&display, mesh, emigui.texture());
        exit
    };

    paint(raw_input);

    events_loop.run_forever(|event| {
        match event {
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::CloseRequested => return glutin::ControlFlow::Break,

                glutin::WindowEvent::Resized(glutin::dpi::LogicalSize { width, height }) => {
                    raw_input.screen_size = vec2(width as f32, height as f32);
                    if paint(raw_input) {
                        return glutin::ControlFlow::Break;
                    }
                }
                glutin::WindowEvent::MouseInput { state, .. } => {
                    raw_input.mouse_down = state == glutin::ElementState::Pressed;
                    if paint(raw_input) {
                        return glutin::ControlFlow::Break;
                    }
                }
                glutin::WindowEvent::CursorMoved { position, .. } => {
                    raw_input.mouse_pos = Some(vec2(position.x as f32, position.y as f32));
                    if paint(raw_input) {
                        return glutin::ControlFlow::Break;
                    }
                }
                _ => (),
            },
            _ => (),
        }
        glutin::ControlFlow::Continue
    });
}
