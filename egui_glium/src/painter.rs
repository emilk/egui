#![allow(deprecated)] // legacy implement_vertex macro

use {
    egui::{
        math::clamp,
        paint::{PaintJobs, Triangles},
        Rect,
    },
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
                        uniform vec2 u_screen_size;
                        uniform vec2 u_tex_size;
                        in vec2 a_pos;
                        in vec4 a_srgba;
                        in vec2 a_tc;
                        out vec4 v_rgba;
                        out vec2 v_tc;

                        // 0-1 linear  from  0-255 sRGB
                        vec3 linear_from_srgb(vec3 srgb) {
                            bvec3 cutoff = lessThan(srgb, vec3(10.31475));
                            vec3 lower = srgb / vec3(3294.6);
                            vec3 higher = pow((srgb + vec3(14.025)) / vec3(269.025), vec3(2.4));
                            return mix(higher, lower, cutoff);
                        }

                        vec4 linear_from_srgba(vec4 srgba) {
                            return vec4(linear_from_srgb(srgba.rgb), srgba.a / 255.0);
                        }

                        void main() {
                            gl_Position = vec4(
                                2.0 * a_pos.x / u_screen_size.x - 1.0,
                                1.0 - 2.0 * a_pos.y / u_screen_size.y,
                                0.0,
                                1.0);
                            v_rgba = linear_from_srgba(a_srgba);
                            v_tc = a_tc / u_tex_size;
                        }
                    ",

                    fragment: "
                        #version 140
                        uniform sampler2D u_sampler;
                        in vec4 v_rgba;
                        in vec2 v_tc;
                        out vec4 f_color;

                        void main() {
                            // glium expects linear rgba
                            f_color = v_rgba * texture(u_sampler, v_tc).r;
                        }
                    "
            },

            110 => {
                    vertex: "
                        #version 110
                        uniform vec2 u_screen_size;
                        uniform vec2 u_tex_size;
                        attribute vec2 a_pos;
                        attribute vec4 a_srgba;
                        attribute vec2 a_tc;
                        varying vec4 v_rgba;
                        varying vec2 v_tc;

                        // 0-1 linear  from  0-255 sRGB
                        vec3 linear_from_srgb(vec3 srgb) {
                            bvec3 cutoff = lessThan(srgb, vec3(10.31475));
                            vec3 lower = srgb / vec3(3294.6);
                            vec3 higher = pow((srgb + vec3(14.025)) / vec3(269.025), vec3(2.4));
                            return mix(higher, lower, cutoff);
                        }

                        vec4 linear_from_srgba(vec4 srgba) {
                            return vec4(linear_from_srgb(srgba.rgb), srgba.a / 255.0);
                        }

                        void main() {
                            gl_Position = vec4(
                                2.0 * a_pos.x / u_screen_size.x - 1.0,
                                1.0 - 2.0 * a_pos.y / u_screen_size.y,
                                0.0,
                                1.0);
                            v_rgba = linear_from_srgba(a_srgba);
                            v_tc = a_tc / u_tex_size;
                        }
                    ",

                    fragment: "
                        #version 110
                        uniform sampler2D u_sampler;
                        varying vec4 v_rgba;
                        varying vec2 v_tc;

                        void main() {
                            // glium expects linear rgba
                            gl_FragColor = v_rgba * texture2D`(u_sampler, v_tc).r;
                        }
                    ",
            },

            100 => {
                    vertex: "
                        #version 100
                        uniform mediump vec2 u_screen_size;
                        uniform mediump vec2 u_tex_size;
                        attribute mediump vec2 a_pos;
                        attribute mediump vec4 a_srgba;
                        attribute mediump vec2 a_tc;
                        varying mediump vec4 v_rgba;
                        varying mediump vec2 v_tc;

                        // 0-1 linear  from  0-255 sRGB
                        vec3 linear_from_srgb(vec3 srgb) {
                            bvec3 cutoff = lessThan(srgb, vec3(10.31475));
                            vec3 lower = srgb / vec3(3294.6);
                            vec3 higher = pow((srgb + vec3(14.025)) / vec3(269.025), vec3(2.4));
                            return mix(higher, lower, cutoff);
                        }

                        vec4 linear_from_srgba(vec4 srgba) {
                            return vec4(linear_from_srgb(srgba.rgb), srgba.a / 255.0);
                        }

                        void main() {
                            gl_Position = vec4(
                                2.0 * a_pos.x / u_screen_size.x - 1.0,
                                1.0 - 2.0 * a_pos.y / u_screen_size.y,
                                0.0,
                                1.0);
                            v_rgba = linear_from_srgba(a_srgba);
                            v_tc = a_tc / u_tex_size;
                        }
                    ",

                    fragment: "
                        #version 100
                        uniform sampler2D u_sampler;
                        varying mediump vec4 v_rgba;
                        varying mediump vec2 v_tc;

                        void main() {
                            // glium expects linear rgba
                            gl_FragColor = v_rgba * texture2D(u_sampler, v_tc).r;
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

    fn upload_texture(&mut self, facade: &dyn glium::backend::Facade, texture: &egui::Texture) {
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

    pub fn paint_jobs(
        &mut self,
        display: &glium::Display,
        jobs: PaintJobs,
        texture: &egui::Texture,
    ) {
        self.upload_texture(display, texture);

        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 0.0);
        for (clip_rect, triangles) in jobs {
            self.paint_job(&mut target, display, clip_rect, &triangles, texture)
        }
        target.finish().unwrap();
    }

    #[inline(never)] // Easier profiling
    fn paint_job(
        &mut self,
        target: &mut Frame,
        display: &glium::Display,
        clip_rect: Rect,
        triangles: &Triangles,
        texture: &egui::Texture,
    ) {
        debug_assert!(triangles.is_valid());

        let vertex_buffer = {
            #[derive(Copy, Clone)]
            struct Vertex {
                a_pos: [f32; 2],
                a_srgba: [u8; 4],
                a_tc: [u16; 2],
            }
            implement_vertex!(Vertex, a_pos, a_srgba, a_tc);

            let vertices: Vec<Vertex> = triangles
                .vertices
                .iter()
                .map(|v| Vertex {
                    a_pos: [v.pos.x, v.pos.y],
                    a_srgba: v.color.0,
                    a_tc: [v.uv.0, v.uv.1],
                })
                .collect();

            glium::VertexBuffer::new(display, &vertices).unwrap()
        };

        let indices: Vec<u32> = triangles.indices.iter().map(|idx| *idx as u32).collect();

        let index_buffer =
            glium::IndexBuffer::new(display, PrimitiveType::TrianglesList, &indices).unwrap();

        let pixels_per_point = display.gl_window().window().scale_factor() as f32;
        let (width_pixels, height_pixels) = display.get_framebuffer_dimensions();
        let width_points = width_pixels as f32 / pixels_per_point;
        let height_points = height_pixels as f32 / pixels_per_point;

        let uniforms = uniform! {
            u_screen_size: [width_points, height_points],
            u_tex_size: [texture.width as f32, texture.height as f32],
            u_sampler: &self.texture,
        };

        // Emilib outputs colors with premultiplied alpha:
        let blend_func = glium::BlendingFunction::Addition {
            source: glium::LinearBlendingFactor::One,
            destination: glium::LinearBlendingFactor::OneMinusSourceAlpha,
        };
        let blend = glium::Blend {
            color: blend_func,
            alpha: blend_func,
            ..Default::default()
        };

        let clip_min_x = pixels_per_point * clip_rect.min.x;
        let clip_min_y = pixels_per_point * clip_rect.min.y;
        let clip_max_x = pixels_per_point * clip_rect.max.x;
        let clip_max_y = pixels_per_point * clip_rect.max.y;
        let clip_min_x = clamp(clip_min_x, 0.0..=width_pixels as f32);
        let clip_min_y = clamp(clip_min_y, 0.0..=height_pixels as f32);
        let clip_max_x = clamp(clip_max_x, clip_min_x..=width_pixels as f32);
        let clip_max_y = clamp(clip_max_y, clip_min_y..=height_pixels as f32);
        let clip_min_x = clip_min_x.round() as u32;
        let clip_min_y = clip_min_y.round() as u32;
        let clip_max_x = clip_max_x.round() as u32;
        let clip_max_y = clip_max_y.round() as u32;

        let params = glium::DrawParameters {
            blend,
            scissor: Some(glium::Rect {
                left: clip_min_x,
                bottom: height_pixels - clip_max_y,
                width: clip_max_x - clip_min_x,
                height: clip_max_y - clip_min_y,
            }),
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
