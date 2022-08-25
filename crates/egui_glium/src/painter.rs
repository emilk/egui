#![allow(deprecated)] // legacy implement_vertex macro
#![allow(semicolon_in_expressions_from_macros)] // glium::program! macro

use egui::{epaint::Primitive, TextureFilter};

use {
    egui::{emath::Rect, epaint::Mesh},
    glium::{
        implement_vertex,
        index::PrimitiveType,
        program,
        texture::{self, srgb_texture2d::SrgbTexture2d},
        uniform,
        uniforms::{MagnifySamplerFilter, SamplerWrapFunction},
    },
    std::rc::Rc,
};

pub struct Painter {
    max_texture_side: usize,
    program: glium::Program,

    textures: ahash::HashMap<egui::TextureId, EguiTexture>,

    /// [`egui::TextureId::User`] index
    next_native_tex_id: u64,
}

impl Painter {
    pub fn new(facade: &dyn glium::backend::Facade) -> Painter {
        use glium::CapabilitiesSource as _;
        let max_texture_side = facade.get_capabilities().max_texture_size as _;

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
            max_texture_side,
            program,
            textures: Default::default(),
            next_native_tex_id: 0,
        }
    }

    pub fn max_texture_side(&self) -> usize {
        self.max_texture_side
    }

    pub fn paint_and_update_textures<T: glium::Surface>(
        &mut self,
        display: &glium::Display,
        target: &mut T,
        pixels_per_point: f32,
        clipped_primitives: &[egui::ClippedPrimitive],
        textures_delta: &egui::TexturesDelta,
    ) {
        for (id, image_delta) in &textures_delta.set {
            self.set_texture(display, *id, image_delta);
        }

        self.paint_primitives(display, target, pixels_per_point, clipped_primitives);

        for &id in &textures_delta.free {
            self.free_texture(id);
        }
    }

    /// Main entry-point for painting a frame.
    /// You should call `target.clear_color(..)` before
    /// and `target.finish()` after this.
    pub fn paint_primitives<T: glium::Surface>(
        &mut self,
        display: &glium::Display,
        target: &mut T,
        pixels_per_point: f32,
        clipped_primitives: &[egui::ClippedPrimitive],
    ) {
        for egui::ClippedPrimitive {
            clip_rect,
            primitive,
        } in clipped_primitives
        {
            match primitive {
                Primitive::Mesh(mesh) => {
                    self.paint_mesh(target, display, pixels_per_point, clip_rect, mesh);
                }
                Primitive::Callback(_) => {
                    panic!("Custom rendering callbacks are not implemented in egui_glium");
                }
            }
        }
    }

    #[inline(never)] // Easier profiling
    fn paint_mesh<T: glium::Surface>(
        &mut self,
        target: &mut T,
        display: &glium::Display,
        pixels_per_point: f32,
        clip_rect: &Rect,
        mesh: &Mesh,
    ) {
        debug_assert!(mesh.is_valid());

        let vertex_buffer = {
            #[repr(C)]
            #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
            struct Vertex {
                a_pos: [f32; 2],
                a_tc: [f32; 2],
                a_srgba: [u8; 4],
            }
            implement_vertex!(Vertex, a_pos, a_tc, a_srgba);

            let vertices: &[Vertex] = bytemuck::cast_slice(&mesh.vertices);

            // TODO(emilk): we should probably reuse the [`VertexBuffer`] instead of allocating a new one each frame.
            glium::VertexBuffer::new(display, vertices).unwrap()
        };

        // TODO(emilk): we should probably reuse the [`IndexBuffer`] instead of allocating a new one each frame.
        let index_buffer =
            glium::IndexBuffer::new(display, PrimitiveType::TrianglesList, &mesh.indices).unwrap();

        let (width_in_pixels, height_in_pixels) = display.get_framebuffer_dimensions();
        let width_in_points = width_in_pixels as f32 / pixels_per_point;
        let height_in_points = height_in_pixels as f32 / pixels_per_point;

        if let Some(texture) = self.texture(mesh.texture_id) {
            // The texture coordinates for text are so that both nearest and linear should work with the egui font texture.
            let filter = match texture.filter {
                TextureFilter::Nearest => MagnifySamplerFilter::Nearest,
                TextureFilter::Linear => MagnifySamplerFilter::Linear,
            };

            let uniforms = uniform! {
                u_screen_size: [width_in_points, height_in_points],
                u_sampler: texture.glium_texture.sampled().magnify_filter(filter).wrap_function(SamplerWrapFunction::Clamp),
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

    pub fn set_texture(
        &mut self,
        facade: &dyn glium::backend::Facade,
        tex_id: egui::TextureId,
        delta: &egui::epaint::ImageDelta,
    ) {
        let pixels: Vec<(u8, u8, u8, u8)> = match &delta.image {
            egui::ImageData::Color(image) => {
                assert_eq!(
                    image.width() * image.height(),
                    image.pixels.len(),
                    "Mismatch between texture size and texel count"
                );
                image.pixels.iter().map(|color| color.to_tuple()).collect()
            }
            egui::ImageData::Font(image) => {
                let gamma = 1.0;
                image
                    .srgba_pixels(gamma)
                    .map(|color| color.to_tuple())
                    .collect()
            }
        };
        let glium_image = glium::texture::RawImage2d {
            data: std::borrow::Cow::Owned(pixels),
            width: delta.image.width() as _,
            height: delta.image.height() as _,
            format: glium::texture::ClientFormat::U8U8U8U8,
        };
        let format = texture::SrgbFormat::U8U8U8U8;
        let mipmaps = texture::MipmapsOption::NoMipmap;

        if let Some(pos) = delta.pos {
            // update a sub-region
            if let Some(user_texture) = self.textures.get_mut(&tex_id) {
                let rect = glium::Rect {
                    left: pos[0] as _,
                    bottom: pos[1] as _,
                    width: glium_image.width,
                    height: glium_image.height,
                };
                user_texture
                    .glium_texture
                    .main_level()
                    .write(rect, glium_image);

                user_texture.filter = delta.filter;
            }
        } else {
            let gl_texture =
                SrgbTexture2d::with_format(facade, glium_image, format, mipmaps).unwrap();

            let user_texture = EguiTexture::new(gl_texture.into(), delta.filter);
            self.textures.insert(tex_id, user_texture);
        }
    }

    pub fn free_texture(&mut self, tex_id: egui::TextureId) {
        self.textures.remove(&tex_id);
    }

    fn texture(&self, texture_id: egui::TextureId) -> Option<&EguiTexture> {
        self.textures.get(&texture_id)
    }

    pub fn register_native_texture(
        &mut self,
        native: Rc<SrgbTexture2d>,
        filter: TextureFilter,
    ) -> egui::TextureId {
        let id = egui::TextureId::User(self.next_native_tex_id);
        self.next_native_tex_id += 1;

        let texture = EguiTexture::new(native, filter);
        self.textures.insert(id, texture);
        id
    }

    pub fn replace_native_texture(
        &mut self,
        id: egui::TextureId,
        replacing: Rc<SrgbTexture2d>,
        filter: TextureFilter,
    ) {
        let texture = EguiTexture::new(replacing, filter);
        self.textures.insert(id, texture);
    }
}

struct EguiTexture {
    glium_texture: Rc<SrgbTexture2d>,
    filter: TextureFilter,
}

impl EguiTexture {
    fn new(glium_texture: Rc<SrgbTexture2d>, filter: TextureFilter) -> Self {
        Self {
            glium_texture,
            filter,
        }
    }
}
