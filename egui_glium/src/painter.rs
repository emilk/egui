#![allow(deprecated)] // legacy implement_vertex macro

use {
    egui::{
        emath::Rect,
        epaint::{Color32, Mesh},
    },
    glium::{
        implement_vertex,
        index::PrimitiveType,
        program,
        texture::{self, srgb_texture2d::SrgbTexture2d},
        uniform,
        uniforms::{MagnifySamplerFilter, SamplerWrapFunction},
        Frame, Surface,
    },
};

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
        let program = program! {
            facade,
            120 => {
                vertex: include_str!("shader/vertex_120.glsl"),
                fragment: include_str!("shader/fragment_120.glsl"),
            },
            140 => {
                vertex: include_str!("shader/vertex_140.glsl"),
                fragment: include_str!("shader/fragment_140.glsl"),
            },
            100 es => {
                vertex: include_str!("shader/vertex_100es.glsl"),
                fragment: include_str!("shader/fragment_100es.glsl"),
            },
            300 es => {
                vertex: include_str!("shader/vertex_300es.glsl"),
                fragment: include_str!("shader/fragment_300es.glsl"),
            },
        }
        .expect("Failed to compile shader");

        Painter {
            program,
            egui_texture: None,
            egui_texture_version: None,
            user_textures: Default::default(),
        }
    }

    pub fn upload_egui_texture(
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
                    .map(|&a| Color32::from_white_alpha(a).to_tuple())
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
    pub fn paint_meshes(
        &mut self,
        display: &glium::Display,
        pixels_per_point: f32,
        clear_color: egui::Rgba,
        cipped_meshes: Vec<egui::ClippedMesh>,
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
        for egui::ClippedMesh(clip_rect, mesh) in cipped_meshes {
            self.paint_mesh(&mut target, display, pixels_per_point, clip_rect, &mesh)
        }
        target.finish().unwrap();
    }

    #[inline(never)] // Easier profiling
    pub fn paint_mesh(
        &mut self,
        target: &mut Frame,
        display: &glium::Display,
        pixels_per_point: f32,
        clip_rect: Rect,
        mesh: &Mesh,
    ) {
        debug_assert!(mesh.is_valid());

        let vertex_buffer = {
            #[derive(Copy, Clone)]
            struct Vertex {
                a_pos: [f32; 2],
                a_tc: [f32; 2],
                a_srgba: [u8; 4],
            }
            implement_vertex!(Vertex, a_pos, a_tc, a_srgba);

            let vertices: Vec<Vertex> = mesh
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

        // TODO: we should probably reuse the `IndexBuffer` instead of allocating a new one each frame.
        let index_buffer =
            glium::IndexBuffer::new(display, PrimitiveType::TrianglesList, &mesh.indices).unwrap();

        let (width_in_pixels, height_in_pixels) = display.get_framebuffer_dimensions();
        let width_in_points = width_in_pixels as f32 / pixels_per_point;
        let height_in_points = height_in_pixels as f32 / pixels_per_point;

        if let Some(texture) = self.get_texture(mesh.texture_id) {
            // The texture coordinates for text are so that both nearest and linear should work with the egui font texture.
            // For user textures linear sampling is more likely to be the right choice.
            let filter = MagnifySamplerFilter::Linear;

            let uniforms = uniform! {
                u_screen_size: [width_in_points, height_in_points],
                u_sampler: texture.sampled().magnify_filter(filter).wrap_function(SamplerWrapFunction::Clamp),
            };

            // egui outputs colors with premultiplied alpha:
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

            // egui outputs mesh in both winding orders:
            let backface_culling = glium::BackfaceCullingMode::CullingDisabled;

            // Transform clip rect to physical pixels:
            let clip_min_x = pixels_per_point * clip_rect.min.x;
            let clip_min_y = pixels_per_point * clip_rect.min.y;
            let clip_max_x = pixels_per_point * clip_rect.max.x;
            let clip_max_y = pixels_per_point * clip_rect.max.y;

            // Make sure clip rect can fit within a `u32`:
            let clip_min_x = clip_min_x.clamp(0.0, width_in_pixels as f32);
            let clip_min_y = clip_min_y.clamp(0.0, height_in_pixels as f32);
            let clip_max_x = clip_max_x.clamp(clip_min_x, width_in_pixels as f32);
            let clip_max_y = clip_max_y.clamp(clip_min_y, height_in_pixels as f32);

            let clip_min_x = clip_min_x.round() as u32;
            let clip_min_y = clip_min_y.round() as u32;
            let clip_max_x = clip_max_x.round() as u32;
            let clip_max_y = clip_max_y.round() as u32;

            let params = glium::DrawParameters {
                blend,
                backface_culling,
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
    // No need to implement this in your egui integration!

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
    /// register glium texture as egui texture
    /// Usable for render to image rectangle
    pub fn register_glium_texture(
        &mut self,
        texture: glium::texture::SrgbTexture2d,
    ) -> egui::TextureId {
        let id = self.alloc_user_texture();
        if let egui::TextureId::User(id) = id {
            if let Some(Some(user_texture)) = self.user_textures.get_mut(id as usize) {
                *user_texture = UserTexture {
                    pixels: vec![],
                    gl_texture: Some(texture),
                }
            }
        }
        id
    }
    pub fn set_user_texture(
        &mut self,
        id: egui::TextureId,
        size: (usize, usize),
        pixels: &[Color32],
    ) {
        assert_eq!(size.0 * size.1, pixels.len());

        if let egui::TextureId::User(id) = id {
            if let Some(Some(user_texture)) = self.user_textures.get_mut(id as usize) {
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

    pub fn free_user_texture(&mut self, id: egui::TextureId) {
        if let egui::TextureId::User(id) = id {
            let index = id as usize;
            if index < self.user_textures.len() {
                self.user_textures[index] = None;
            }
        }
    }

    pub fn get_texture(&self, texture_id: egui::TextureId) -> Option<&SrgbTexture2d> {
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

    pub fn upload_pending_user_textures(&mut self, facade: &dyn glium::backend::Facade) {
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
