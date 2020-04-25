#![allow(deprecated)] // legacy implement_vertex macro

use {
    emigui::{Mesh, PaintBatches, Rect},
    glium::{implement_vertex, index::PrimitiveType, program, texture, uniform, Frame, Surface},
};

pub struct Painter {
    program: glium::Program,
    texture: texture::texture2d::Texture2d,
    current_texture_id: Option<u64>,
}

impl Painter {
    pub fn new(facade: &dyn glium::backend::Facade) -> Painter {
        let program = program!(facade,
            140 => {
                    vertex: "
                        #version 140
                        uniform vec4 u_clip_rect; // min_x, min_y, max_x, max_y
                        uniform vec2 u_screen_size;
                        uniform vec2 u_tex_size;
                        in vec2 a_pos;
                        in vec4 a_color;
                        in vec2 a_tc;
                        out vec2 v_pos;
                        out vec4 v_color;
                        out vec2 v_tc;
                        out vec4 v_clip_rect;
                        void main() {
                            gl_Position = vec4(
                                2.0 * a_pos.x / u_screen_size.x - 1.0,
                                1.0 - 2.0 * a_pos.y / u_screen_size.y,
                                0.0,
                                1.0);
                            v_pos = a_pos;
                            v_color = a_color / 255.0;
                            v_tc = a_tc / u_tex_size;
                            v_clip_rect = u_clip_rect;
                        }
                    ",

                    fragment: "
                        #version 140
                        uniform sampler2D u_sampler;
                        in vec2 v_pos;
                        in vec4 v_color;
                        in vec2 v_tc;
                        in vec4 v_clip_rect;
                        out vec4 f_color;

                        // glium expects linear output.
                        vec3 linear_from_srgb(vec3 srgb) {
                            bvec3 cutoff = lessThan(srgb, vec3(0.04045));
                            vec3 higher = pow((srgb + vec3(0.055)) / vec3(1.055), vec3(2.4));
                            vec3 lower = srgb / vec3(12.92);
                            return mix(higher, lower, cutoff);
                        }

                        void main() {
                            if (v_pos.x < v_clip_rect.x) { discard; }
                            if (v_pos.y < v_clip_rect.y) { discard; }
                            if (v_pos.x > v_clip_rect.z) { discard; }
                            if (v_pos.y > v_clip_rect.w) { discard; }
                            f_color = v_color;
                            f_color.rgb = linear_from_srgb(f_color.rgb);
                            f_color.a *= texture(u_sampler, v_tc).r;
                        }
                    "
            },

            110 => {
                    vertex: "
                        #version 110
                        uniform vec4 u_clip_rect; // min_x, min_y, max_x, max_y
                        uniform vec2 u_screen_size;
                        uniform vec2 u_tex_size;
                        attribute vec2 a_pos;
                        attribute vec4 a_color;
                        attribute vec2 a_tc;
                        varying vec2 v_pos;
                        varying vec4 v_color;
                        varying vec2 v_tc;
                        varying vec4 v_clip_rect;
                        void main() {
                            gl_Position = vec4(
                                2.0 * a_pos.x / u_screen_size.x - 1.0,
                                1.0 - 2.0 * a_pos.y / u_screen_size.y,
                                0.0,
                                1.0);
                            v_pos = a_pos;
                            v_color = a_color / 255.0;
                            v_tc = a_tc / u_tex_size;
                            v_clip_rect = u_clip_rect;
                        }
                    ",

                    fragment: "
                        #version 110
                        uniform sampler2D u_sampler;
                        varying vec2 v_pos;
                        varying vec4 v_color;
                        varying vec2 v_tc;
                        varying vec4 v_clip_rect;

                        // glium expects linear output.
                        vec3 linear_from_srgb(vec3 srgb) {
                            bvec3 cutoff = lessThan(srgb, vec3(0.04045));
                            vec3 higher = pow((srgb + vec3(0.055)) / vec3(1.055), vec3(2.4));
                            vec3 lower = srgb / vec3(12.92);
                            return mix(higher, lower, cutoff);
                        }

                        void main() {
                            if (v_pos.x < v_clip_rect.x) { discard; }
                            if (v_pos.y < v_clip_rect.y) { discard; }
                            if (v_pos.x > v_clip_rect.z) { discard; }
                            if (v_pos.y > v_clip_rect.w) { discard; }
                            gl_FragColor = v_color;
                            gl_FragColor.rgb = linear_from_srgb(gl_FragColor.rgb);
                            gl_FragColor.a *= texture2D(u_sampler, v_tc).r;
                        }
                    ",
            },

            100 => {
                    vertex: "
                        #version 100
                        uniform mediump vec4 u_clip_rect; // min_x, min_y, max_x, max_y
                        uniform mediump vec2 u_screen_size;
                        uniform mediump vec2 u_tex_size;
                        attribute mediump vec2 a_pos;
                        attribute mediump vec4 a_color;
                        attribute mediump vec2 a_tc;
                        varying mediump vec2 v_pos;
                        varying mediump vec4 v_color;
                        varying mediump vec2 v_tc;
                        varying mediump vec4 v_clip_rect;
                        void main() {
                            gl_Position = vec4(
                                2.0 * a_pos.x / u_screen_size.x - 1.0,
                                1.0 - 2.0 * a_pos.y / u_screen_size.y,
                                0.0,
                                1.0);
                            v_pos = a_pos;
                            v_color = a_color / 255.0;
                            v_tc = a_tc / u_tex_size;
                            v_clip_rect = u_clip_rect;
                        }
                    ",

                    fragment: "
                        #version 100
                        uniform sampler2D u_sampler;
                        varying mediump vec2 v_pos;
                        varying mediump vec4 v_color;
                        varying mediump vec2 v_tc;
                        varying mediump vec4 v_clip_rect

                        // glium expects linear output.
                        vec3 linear_from_srgb(vec3 srgb) {
                            bvec3 cutoff = lessThan(srgb, vec3(0.04045));
                            vec3 higher = pow((srgb + vec3(0.055)) / vec3(1.055), vec3(2.4));
                            vec3 lower = srgb / vec3(12.92);
                            return mix(higher, lower, cutoff);
                        }

                        void main() {
                            if (v_pos.x < v_clip_rect.x) { discard; }
                            if (v_pos.y < v_clip_rect.y) { discard; }
                            if (v_pos.x > v_clip_rect.z) { discard; }
                            if (v_pos.y > v_clip_rect.w) { discard; }
                            gl_FragColor = v_color;
                            gl_FragColor.rgb = linear_from_srgb(gl_FragColor.rgb);
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

    fn upload_texture(&mut self, facade: &dyn glium::backend::Facade, texture: &emigui::Texture) {
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

    pub fn paint_batches(
        &mut self,
        display: &glium::Display,
        batches: PaintBatches,
        texture: &emigui::Texture,
    ) {
        self.upload_texture(display, texture);

        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 0.0);
        for (clip_rect, mesh) in batches {
            self.paint_batch(&mut target, display, &clip_rect, &mesh, texture)
        }
        target.finish().unwrap();
    }

    fn paint_batch(
        &mut self,
        target: &mut Frame,
        display: &glium::Display,
        clip_rect: &Rect,
        mesh: &Mesh,
        texture: &emigui::Texture,
    ) {
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

        let pixels_per_point = display.gl_window().get_hidpi_factor() as f32;
        let (width_pixels, height_pixels) = display.get_framebuffer_dimensions();
        let width_points = width_pixels as f32 / pixels_per_point;
        let height_points = height_pixels as f32 / pixels_per_point;

        let uniforms = uniform! {
            u_clip_rect: [clip_rect.min.x, clip_rect.min.y, clip_rect.max.x, clip_rect.max.y],
            u_screen_size: [width_points, height_points],
            u_tex_size: [texture.width as f32, texture.height as f32],
            u_sampler: &self.texture,
        };

        let params = glium::DrawParameters {
            blend: glium::Blend::alpha_blending(),
            ..Default::default()
        };

        target
            .draw(
                &vertex_buffer,
                &index_buffer,
                &self.program,
                &uniforms,
                &params,
            )
            .unwrap();
    }
}
