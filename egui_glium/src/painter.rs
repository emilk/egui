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
        texture::{self, srgb_texture2d::SrgbTexture2d},
        uniform,
        uniforms::SamplerWrapFunction,
        Frame, Surface,
    },
};

const VERTEX_SHADER_SOURCE: &str = r#"
    #version 140
    uniform vec2 u_screen_size;
    in vec2 a_pos;
    in vec4 a_srgba; // 0-255 sRGB
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
"#;

const FRAGMENT_SHADER_SOURCE: &str = r#"
    #version 140
    uniform sampler2D u_sampler;
    in vec4 v_rgba;
    in vec2 v_tc;
    out vec4 f_color;

    void main() {
        // The texture sampler is sRGB aware, and glium already expects linear rgba output
        // so no need for any sRGB conversions here:
        f_color = v_rgba * texture(u_sampler, v_tc);
    }
"#;

pub struct Painter {
    program: glium::Program,
    egui_texture: Option<SrgbTexture2d>,
    egui_texture_version: Option<u64>,

    /// `None` means unallocated (freed) slot.
    user_textures: Vec<Option<UserTexture>>,
}

#[derive(Default)]
struct UserTexture {
    /// Pending upload (will be emptied later).
    /// This is the format glium likes.
    pixels: Vec<Vec<(u8, u8, u8, u8)>>,

    /// Lazily uploaded
    gl_texture: Option<SrgbTexture2d>,
}

impl Painter {
    pub fn new(facade: &dyn glium::backend::Facade) -> Painter {
        let program =
            glium::Program::from_source(facade, VERTEX_SHADER_SOURCE, FRAGMENT_SHADER_SOURCE, None)
                .expect("Failed to compile shader");

        Painter {
            program,
            egui_texture: None,
            egui_texture_version: None,
            user_textures: Default::default(),
        }
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
        self.egui_texture =
            Some(SrgbTexture2d::with_format(facade, pixels, format, mipmaps).unwrap());
        self.egui_texture_version = Some(texture.version);
    }

    /// Main entry-point for painting a frame
    pub fn paint_jobs(
        &mut self,
        display: &glium::Display,
        pixels_per_point: f32,
        clear_color: egui::Rgba,
        jobs: PaintJobs,
        egui_texture: &egui::Texture,
    ) {
        self.upload_egui_texture(display, egui_texture);
        self.upload_pending_user_textures(display);

        let mut target = display.draw();
        // Verified to be gamma-correct.
        target.clear_color(
            clear_color[0],
            clear_color[1],
            clear_color[2],
            clear_color[3],
        );
        for (clip_rect, triangles) in jobs {
            self.paint_job(
                &mut target,
                display,
                pixels_per_point,
                clip_rect,
                &triangles,
            )
        }
        target.finish().unwrap();
    }

    #[inline(never)] // Easier profiling
    fn paint_job(
        &mut self,
        target: &mut Frame,
        display: &glium::Display,
        pixels_per_point: f32,
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

            // TODO: we should probably reuse the `VertexBuffer` instead of allocating a new one each frame.
            glium::VertexBuffer::new(display, &vertices).unwrap()
        };

        let indices: Vec<u32> = triangles.indices.iter().map(|idx| *idx as u32).collect();

        // TODO: we should probably reuse the `IndexBuffer` instead of allocating a new one each frame.
        let index_buffer =
            glium::IndexBuffer::new(display, PrimitiveType::TrianglesList, &indices).unwrap();

        let (width_in_pixels, height_in_pixels) = display.get_framebuffer_dimensions();
        let width_in_points = width_in_pixels as f32 / pixels_per_point;
        let height_in_points = height_in_pixels as f32 / pixels_per_point;

        if let Some(texture) = self.get_texture(triangles.texture_id) {
            let uniforms = uniform! {
                u_screen_size: [width_in_points, height_in_points],
                u_sampler: texture.sampled().wrap_function(SamplerWrapFunction::Clamp),
            };

            // Egui outputs colors with premultiplied alpha:
            let color_blend_func = glium::BlendingFunction::Addition {
                source: glium::LinearBlendingFactor::One,
                destination: glium::LinearBlendingFactor::OneMinusSourceAlpha,
            };

            // Less important, but this is technically the correct alpha blend function
            // when you want to make use of the framebuffer alpha (for screenshots, compositing, etc).
            let alpha_blend_func = glium::BlendingFunction::Addition {
                source: glium::LinearBlendingFactor::OneMinusDestinationAlpha,
                destination: glium::LinearBlendingFactor::One,
            };

            let blend = glium::Blend {
                color: color_blend_func,
                alpha: alpha_blend_func,
                ..Default::default()
            };

            // Transform clip rect to physical pixels:
            let clip_min_x = pixels_per_point * clip_rect.min.x;
            let clip_min_y = pixels_per_point * clip_rect.min.y;
            let clip_max_x = pixels_per_point * clip_rect.max.x;
            let clip_max_y = pixels_per_point * clip_rect.max.y;

            // Make sure clip rect can fit withing an `u32`:
            let clip_min_x = clamp(clip_min_x, 0.0..=width_in_pixels as f32);
            let clip_min_y = clamp(clip_min_y, 0.0..=height_in_pixels as f32);
            let clip_max_x = clamp(clip_max_x, clip_min_x..=width_in_pixels as f32);
            let clip_max_y = clamp(clip_max_y, clip_min_y..=height_in_pixels as f32);

            let clip_min_x = clip_min_x.round() as u32;
            let clip_min_y = clip_min_y.round() as u32;
            let clip_max_x = clip_max_x.round() as u32;
            let clip_max_y = clip_max_y.round() as u32;

            let params = glium::DrawParameters {
                blend,
                scissor: Some(glium::Rect {
                    left: clip_min_x,
                    bottom: height_in_pixels - clip_max_y,
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

    // ------------------------------------------------------------------------
    // user textures: this is an experimental feature.
    // No need to implement this in your Egui integration!

    pub fn alloc_user_texture(&mut self) -> egui::TextureId {
        for (i, tex) in self.user_textures.iter_mut().enumerate() {
            if tex.is_none() {
                *tex = Some(Default::default());
                return egui::TextureId::User(i as u64);
            }
        }
        let id = egui::TextureId::User(self.user_textures.len() as u64);
        self.user_textures.push(Some(Default::default()));
        id
    }

    pub fn set_user_texture(
        &mut self,
        id: egui::TextureId,
        size: (usize, usize),
        pixels: &[Srgba],
    ) {
        assert_eq!(size.0 * size.1, pixels.len());

        if let egui::TextureId::User(id) = id {
            if let Some(user_texture) = self.user_textures.get_mut(id as usize) {
                if let Some(user_texture) = user_texture {
                    let pixels: Vec<Vec<(u8, u8, u8, u8)>> = pixels
                        .chunks(size.0 as usize)
                        .map(|row| row.iter().map(|srgba| srgba.to_tuple()).collect())
                        .collect();

                    *user_texture = UserTexture {
                        pixels,
                        gl_texture: None,
                    };
                }
            }
        }
    }

    pub fn free_user_texture(&mut self, id: egui::TextureId) {
        if let egui::TextureId::User(id) = id {
            let index = id as usize;
            if index < self.user_textures.len() {
                self.user_textures[index] = None;
            }
        }
    }

    fn get_texture(&self, texture_id: egui::TextureId) -> Option<&SrgbTexture2d> {
        match texture_id {
            egui::TextureId::Egui => self.egui_texture.as_ref(),
            egui::TextureId::User(id) => self
                .user_textures
                .get(id as usize)?
                .as_ref()?
                .gl_texture
                .as_ref(),
        }
    }

    fn upload_pending_user_textures(&mut self, facade: &dyn glium::backend::Facade) {
        for user_texture in &mut self.user_textures {
            if let Some(user_texture) = user_texture {
                if user_texture.gl_texture.is_none() {
                    let pixels = std::mem::take(&mut user_texture.pixels);
                    let format = texture::SrgbFormat::U8U8U8U8;
                    let mipmaps = texture::MipmapsOption::NoMipmap;
                    user_texture.gl_texture =
                        Some(SrgbTexture2d::with_format(facade, pixels, format, mipmaps).unwrap());
                }
            }
        }
    }
}
