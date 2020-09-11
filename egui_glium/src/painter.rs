#![allow(deprecated)] // legacy implement_vertex macro

use {
    egui::{
        math::clamp,
        paint::{PaintJobs, Triangles},
        Rect, Srgba,
    },
    glium::{
        implement_vertex,
        index::PrimitiveType,
        program,
        texture::{self, srgb_texture2d::SrgbTexture2d},
        uniform,
        uniforms::SamplerWrapFunction,
        Frame, Surface,
    },
};

pub struct Painter {
    program: glium::Program,
    egui_texture: SrgbTexture2d,
    egui_texture_version: Option<u64>,

    user_textures: Vec<UserTexture>,
}

#[derive(Default)]
struct UserTexture {
    /// Pending upload (will be emptied later).
    /// This is the format glium likes.
    pixels: Vec<Vec<(u8, u8, u8, u8)>>,

    /// Lazily uploaded
    texture: Option<SrgbTexture2d>,
}

impl Painter {
    pub fn new(facade: &dyn glium::backend::Facade) -> Painter {
        let program = program!(facade,
            140 => {
                    vertex: "
                        #version 140
                        uniform vec2 u_screen_size;
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
                            v_tc = a_tc;
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
                            f_color = v_rgba * texture(u_sampler, v_tc);
                        }
                    "
            },

            110 => {
                    vertex: "
                        #version 110
                        uniform vec2 u_screen_size;
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
                            v_tc = a_tc;
                        }
                    ",

                    fragment: "
                        #version 110
                        uniform sampler2D u_sampler;
                        varying vec4 v_rgba;
                        varying vec2 v_tc;

                        void main() {
                            // glium expects linear rgba
                            gl_FragColor = v_rgba * texture2D(u_sampler, v_tc);
                        }
                    ",
            },

            100 => {
                    vertex: "
                        #version 100
                        uniform mediump vec2 u_screen_size;
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
                            v_tc = a_tc;
                        }
                    ",

                    fragment: "
                        #version 100
                        uniform sampler2D u_sampler;
                        varying mediump vec4 v_rgba;
                        varying mediump vec2 v_tc;

                        void main() {
                            // glium expects linear rgba
                            gl_FragColor = v_rgba * texture2D(u_sampler, v_tc);
                        }
                    ",
            },
        )
        .unwrap();

        let pixels = vec![vec![255u8, 0u8], vec![0u8, 255u8]];
        let format = texture::SrgbFormat::U8U8U8U8;
        let mipmaps = texture::MipmapsOption::NoMipmap;
        let egui_texture = SrgbTexture2d::with_format(facade, pixels, format, mipmaps).unwrap();

        Painter {
            program,
            egui_texture,
            egui_texture_version: None,
            user_textures: Default::default(),
        }
    }

    pub fn new_user_texture(&mut self, size: (usize, usize), pixels: &[Srgba]) -> egui::TextureId {
        assert_eq!(size.0 * size.1, pixels.len());

        let pixels: Vec<Vec<(u8, u8, u8, u8)>> = pixels
            .chunks(size.0 as usize)
            .map(|row| row.iter().map(|srgba| srgba.to_tuple()).collect())
            .collect();

        let id = egui::TextureId::User(self.user_textures.len() as u64);
        self.user_textures.push(UserTexture {
            pixels,
            texture: None,
        });
        id
    }

    fn upload_egui_texture(
        &mut self,
        facade: &dyn glium::backend::Facade,
        texture: &egui::Texture,
    ) {
        if self.egui_texture_version == Some(texture.version) {
            return; // No change
        }

        let pixels: Vec<Vec<(u8, u8, u8, u8)>> = texture
            .pixels
            .chunks(texture.width as usize)
            .map(|row| {
                row.iter()
                    .map(|&a| Srgba::white_alpha(a).to_tuple())
                    .collect()
            })
            .collect();

        let format = texture::SrgbFormat::U8U8U8U8;
        let mipmaps = texture::MipmapsOption::NoMipmap;
        self.egui_texture = SrgbTexture2d::with_format(facade, pixels, format, mipmaps).unwrap();
        self.egui_texture_version = Some(texture.version);
    }

    fn upload_user_textures(&mut self, facade: &dyn glium::backend::Facade) {
        for user_texture in &mut self.user_textures {
            if user_texture.texture.is_none() {
                let pixels = std::mem::take(&mut user_texture.pixels);
                let format = texture::SrgbFormat::U8U8U8U8;
                let mipmaps = texture::MipmapsOption::NoMipmap;
                user_texture.texture =
                    Some(SrgbTexture2d::with_format(facade, pixels, format, mipmaps).unwrap());
            }
        }
    }

    pub fn paint_jobs(
        &mut self,
        display: &glium::Display,
        jobs: PaintJobs,
        texture: &egui::Texture,
    ) {
        self.upload_egui_texture(display, texture);
        self.upload_user_textures(display);

        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 0.0);
        for (clip_rect, triangles) in jobs {
            self.paint_job(&mut target, display, clip_rect, &triangles)
        }
        target.finish().unwrap();
    }

    fn get_texture(&self, texture_id: egui::TextureId) -> &SrgbTexture2d {
        match texture_id {
            egui::TextureId::Egui => &self.egui_texture,
            egui::TextureId::User(id) => {
                let id = id as usize;
                assert!(id < self.user_textures.len());
                let texture = self.user_textures[id].texture.as_ref();
                texture.expect("Should have been uploaded")
            }
        }
    }

    #[inline(never)] // Easier profiling
    fn paint_job(
        &mut self,
        target: &mut Frame,
        display: &glium::Display,
        clip_rect: Rect,
        triangles: &Triangles,
    ) {
        debug_assert!(triangles.is_valid());

        let vertex_buffer = {
            #[derive(Copy, Clone)]
            struct Vertex {
                a_pos: [f32; 2],
                a_tc: [f32; 2],
                a_srgba: [u8; 4],
            }
            implement_vertex!(Vertex, a_pos, a_tc, a_srgba);

            let vertices: Vec<Vertex> = triangles
                .vertices
                .iter()
                .map(|v| Vertex {
                    a_pos: [v.pos.x, v.pos.y],
                    a_tc: [v.uv.x, v.uv.y],
                    a_srgba: v.color.to_array(),
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

        let texture = self.get_texture(triangles.texture_id);

        let uniforms = uniform! {
            u_screen_size: [width_points, height_points],
            u_sampler: texture.sampled().wrap_function(SamplerWrapFunction::Clamp),
        };

        // Egui outputs colors with premultiplied alpha:
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
