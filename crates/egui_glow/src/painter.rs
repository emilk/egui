#![allow(clippy::collapsible_else_if)]
#![allow(unsafe_code)]

use std::{collections::HashMap, sync::Arc};

use egui::{
    emath::Rect,
    epaint::{Mesh, PaintCallbackInfo, Primitive, Vertex},
};
use glow::HasContext as _;
use memoffset::offset_of;

use crate::check_for_gl_error;
use crate::misc_util::{compile_shader, link_program};
use crate::shader_version::ShaderVersion;
use crate::vao;

pub use glow::Context;

const VERT_SRC: &str = include_str!("shader/vertex.glsl");
const FRAG_SRC: &str = include_str!("shader/fragment.glsl");

trait TextureFilterExt {
    fn glow_code(&self) -> u32;
}

impl TextureFilterExt for egui::TextureFilter {
    fn glow_code(&self) -> u32 {
        match self {
            egui::TextureFilter::Linear => glow::LINEAR,
            egui::TextureFilter::Nearest => glow::NEAREST,
        }
    }
}

/// An OpenGL painter using [`glow`].
///
/// This is responsible for painting egui and managing egui textures.
/// You can access the underlying [`glow::Context`] with [`Self::gl`].
///
/// This struct must be destroyed with [`Painter::destroy`] before dropping, to ensure OpenGL
/// objects have been properly deleted and are not leaked.
pub struct Painter {
    gl: Arc<glow::Context>,

    max_texture_side: usize,

    program: glow::Program,
    u_screen_size: glow::UniformLocation,
    u_sampler: glow::UniformLocation,
    is_webgl_1: bool,
    vao: crate::vao::VertexArrayObject,
    srgb_textures: bool,
    vbo: glow::Buffer,
    element_array_buffer: glow::Buffer,

    textures: HashMap<egui::TextureId, glow::Texture>,

    next_native_tex_id: u64,

    /// Stores outdated OpenGL textures that are yet to be deleted
    textures_to_destroy: Vec<glow::Texture>,

    /// Used to make sure we are destroyed correctly.
    destroyed: bool,
}

/// A callback function that can be used to compose an [`egui::PaintCallback`] for custom rendering
/// with [`glow`].
///
/// The callback is passed, the [`egui::PaintCallbackInfo`] and the [`Painter`] which can be used to
/// access the OpenGL context.
///
/// # Example
///
/// See the [`custom3d_glow`](https://github.com/emilk/egui/blob/master/crates/egui_demo_app/src/apps/custom3d_wgpu.rs) demo source for a detailed usage example.
pub struct CallbackFn {
    f: Box<dyn Fn(PaintCallbackInfo, &Painter) + Sync + Send>,
}

impl CallbackFn {
    pub fn new<F: Fn(PaintCallbackInfo, &Painter) + Sync + Send + 'static>(callback: F) -> Self {
        let f = Box::new(callback);
        CallbackFn { f }
    }
}

impl Painter {
    /// Create painter.
    ///
    /// Set `pp_fb_extent` to the framebuffer size to enable `sRGB` support on OpenGL ES and WebGL.
    ///
    /// Set `shader_prefix` if you want to turn on shader workaround e.g. `"#define APPLY_BRIGHTENING_GAMMA\n"`
    /// (see <https://github.com/emilk/egui/issues/794>).
    ///
    /// # Errors
    /// will return `Err` below cases
    /// * failed to compile shader
    /// * failed to create postprocess on webgl with `sRGB` support
    /// * failed to create buffer
    pub fn new(
        gl: Arc<glow::Context>,
        shader_prefix: &str,
        shader_version: Option<ShaderVersion>,
    ) -> Result<Painter, String> {
        crate::profile_function!();
        crate::check_for_gl_error_even_in_release!(&gl, "before Painter::new");

        // some useful debug info. all three of them are present in gl 1.1.
        unsafe {
            let version = gl.get_parameter_string(glow::VERSION);
            let renderer = gl.get_parameter_string(glow::RENDERER);
            let vendor = gl.get_parameter_string(glow::VENDOR);
            tracing::debug!(
                "\nopengl version: {version}\nopengl renderer: {renderer}\nopengl vendor: {vendor}"
            );
        }

        #[cfg(not(target_arch = "wasm32"))]
        if gl.version().major < 2 {
            // this checks on desktop that we are not using opengl 1.1 microsoft sw rendering context.
            // ShaderVersion::get fn will segfault due to SHADING_LANGUAGE_VERSION (added in gl2.0)
            return Err("egui_glow requires opengl 2.0+. ".to_owned());
        }

        let max_texture_side = unsafe { gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE) } as usize;
        let shader_version = shader_version.unwrap_or_else(|| ShaderVersion::get(&gl));
        let is_webgl_1 = shader_version == ShaderVersion::Es100;
        let shader_version_declaration = shader_version.version_declaration();
        tracing::debug!("Shader header: {:?}.", shader_version_declaration);

        let supported_extensions = gl.supported_extensions();
        tracing::trace!("OpenGL extensions: {supported_extensions:?}");
        let srgb_textures = shader_version == ShaderVersion::Es300 // WebGL2 always support sRGB
            || supported_extensions.iter().any(|extension| {
                // EXT_sRGB, GL_ARB_framebuffer_sRGB, GL_EXT_sRGB, GL_EXT_texture_sRGB_decode, …
                extension.contains("sRGB")
            });
        tracing::debug!("SRGB texture Support: {:?}", srgb_textures);

        unsafe {
            let vert = compile_shader(
                &gl,
                glow::VERTEX_SHADER,
                &format!(
                    "{}\n#define NEW_SHADER_INTERFACE {}\n{}\n{}",
                    shader_version_declaration,
                    shader_version.is_new_shader_interface() as i32,
                    shader_prefix,
                    VERT_SRC
                ),
            )?;
            let frag = compile_shader(
                &gl,
                glow::FRAGMENT_SHADER,
                &format!(
                    "{}\n#define NEW_SHADER_INTERFACE {}\n#define SRGB_TEXTURES {}\n{}\n{}",
                    shader_version_declaration,
                    shader_version.is_new_shader_interface() as i32,
                    srgb_textures as i32,
                    shader_prefix,
                    FRAG_SRC
                ),
            )?;
            let program = link_program(&gl, [vert, frag].iter())?;
            gl.detach_shader(program, vert);
            gl.detach_shader(program, frag);
            gl.delete_shader(vert);
            gl.delete_shader(frag);
            let u_screen_size = gl.get_uniform_location(program, "u_screen_size").unwrap();
            let u_sampler = gl.get_uniform_location(program, "u_sampler").unwrap();

            let vbo = gl.create_buffer()?;

            let a_pos_loc = gl.get_attrib_location(program, "a_pos").unwrap();
            let a_tc_loc = gl.get_attrib_location(program, "a_tc").unwrap();
            let a_srgba_loc = gl.get_attrib_location(program, "a_srgba").unwrap();

            let stride = std::mem::size_of::<Vertex>() as i32;
            let buffer_infos = vec![
                vao::BufferInfo {
                    location: a_pos_loc,
                    vector_size: 2,
                    data_type: glow::FLOAT,
                    normalized: false,
                    stride,
                    offset: offset_of!(Vertex, pos) as i32,
                },
                vao::BufferInfo {
                    location: a_tc_loc,
                    vector_size: 2,
                    data_type: glow::FLOAT,
                    normalized: false,
                    stride,
                    offset: offset_of!(Vertex, uv) as i32,
                },
                vao::BufferInfo {
                    location: a_srgba_loc,
                    vector_size: 4,
                    data_type: glow::UNSIGNED_BYTE,
                    normalized: false,
                    stride,
                    offset: offset_of!(Vertex, color) as i32,
                },
            ];
            let vao = crate::vao::VertexArrayObject::new(&gl, vbo, buffer_infos);

            let element_array_buffer = gl.create_buffer()?;

            crate::check_for_gl_error_even_in_release!(&gl, "after Painter::new");

            Ok(Painter {
                gl,
                max_texture_side,
                program,
                u_screen_size,
                u_sampler,
                is_webgl_1,
                vao,
                srgb_textures,
                vbo,
                element_array_buffer,
                textures: Default::default(),
                next_native_tex_id: 1 << 32,
                textures_to_destroy: Vec::new(),
                destroyed: false,
            })
        }
    }

    /// Access the shared glow context.
    pub fn gl(&self) -> &Arc<glow::Context> {
        &self.gl
    }

    pub fn max_texture_side(&self) -> usize {
        self.max_texture_side
    }

    /// The framebuffer we use as an intermediate render target,
    /// or `None` if we are painting to the screen framebuffer directly.
    ///
    /// This is the framebuffer that is bound when [`egui::Shape::Callback`] is called,
    /// and is where any callbacks should ultimately render onto.
    ///
    /// So if in a [`egui::Shape::Callback`] you need to use an offscreen FBO, you should
    /// then restore to this afterwards with
    /// `gl.bind_framebuffer(glow::FRAMEBUFFER, painter.intermediate_fbo());`
    #[allow(clippy::unused_self)]
    pub fn intermediate_fbo(&self) -> Option<glow::Framebuffer> {
        // We don't currently ever render to an offscreen buffer,
        // but we may want to start to in order to do anti-aliasing on web, for instance.
        None
    }

    unsafe fn prepare_painting(
        &mut self,
        [width_in_pixels, height_in_pixels]: [u32; 2],
        pixels_per_point: f32,
    ) -> (u32, u32) {
        self.gl.enable(glow::SCISSOR_TEST);
        // egui outputs mesh in both winding orders
        self.gl.disable(glow::CULL_FACE);
        self.gl.disable(glow::DEPTH_TEST);

        self.gl.color_mask(true, true, true, true);

        self.gl.enable(glow::BLEND);
        self.gl
            .blend_equation_separate(glow::FUNC_ADD, glow::FUNC_ADD);
        self.gl.blend_func_separate(
            // egui outputs colors with premultiplied alpha:
            glow::ONE,
            glow::ONE_MINUS_SRC_ALPHA,
            // Less important, but this is technically the correct alpha blend function
            // when you want to make use of the framebuffer alpha (for screenshots, compositing, etc).
            glow::ONE_MINUS_DST_ALPHA,
            glow::ONE,
        );

        if !cfg!(target_arch = "wasm32") {
            self.gl.disable(glow::FRAMEBUFFER_SRGB);
            check_for_gl_error!(&self.gl, "FRAMEBUFFER_SRGB");
        }

        let width_in_points = width_in_pixels as f32 / pixels_per_point;
        let height_in_points = height_in_pixels as f32 / pixels_per_point;

        self.gl
            .viewport(0, 0, width_in_pixels as i32, height_in_pixels as i32);
        self.gl.use_program(Some(self.program));

        self.gl
            .uniform_2_f32(Some(&self.u_screen_size), width_in_points, height_in_points);
        self.gl.uniform_1_i32(Some(&self.u_sampler), 0);
        self.gl.active_texture(glow::TEXTURE0);

        self.vao.bind(&self.gl);
        self.gl
            .bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.element_array_buffer));

        check_for_gl_error!(&self.gl, "prepare_painting");

        (width_in_pixels, height_in_pixels)
    }

    /// You are expected to have cleared the color buffer before calling this.
    pub fn paint_and_update_textures(
        &mut self,
        screen_size_px: [u32; 2],
        pixels_per_point: f32,
        clipped_primitives: &[egui::ClippedPrimitive],
        textures_delta: &egui::TexturesDelta,
    ) {
        crate::profile_function!();
        for (id, image_delta) in &textures_delta.set {
            self.set_texture(*id, image_delta);
        }

        self.paint_primitives(screen_size_px, pixels_per_point, clipped_primitives);

        for &id in &textures_delta.free {
            self.free_texture(id);
        }
    }

    /// Main entry-point for painting a frame.
    ///
    /// You should call `target.clear_color(..)` before
    /// and `target.finish()` after this.
    ///
    /// The following OpenGL features will be set:
    /// - Scissor test will be enabled
    /// - Cull face will be disabled
    /// - Blend will be enabled
    ///
    /// The scissor area and blend parameters will be changed.
    ///
    /// As well as this, the following objects will be unset:
    /// - Vertex Buffer
    /// - Element Buffer
    /// - Texture (and active texture will be set to 0)
    /// - Program
    ///
    /// Please be mindful of these effects when integrating into your program, and also be mindful
    /// of the effects your program might have on this code. Look at the source if in doubt.
    pub fn paint_primitives(
        &mut self,
        screen_size_px: [u32; 2],
        pixels_per_point: f32,
        clipped_primitives: &[egui::ClippedPrimitive],
    ) {
        crate::profile_function!();
        self.assert_not_destroyed();

        let size_in_pixels = unsafe { self.prepare_painting(screen_size_px, pixels_per_point) };

        for egui::ClippedPrimitive {
            clip_rect,
            primitive,
        } in clipped_primitives
        {
            set_clip_rect(&self.gl, size_in_pixels, pixels_per_point, *clip_rect);

            match primitive {
                Primitive::Mesh(mesh) => {
                    self.paint_mesh(mesh);
                }
                Primitive::Callback(callback) => {
                    if callback.rect.is_positive() {
                        crate::profile_scope!("callback");
                        // Transform callback rect to physical pixels:
                        let rect_min_x = pixels_per_point * callback.rect.min.x;
                        let rect_min_y = pixels_per_point * callback.rect.min.y;
                        let rect_max_x = pixels_per_point * callback.rect.max.x;
                        let rect_max_y = pixels_per_point * callback.rect.max.y;

                        let rect_min_x = rect_min_x.round() as i32;
                        let rect_min_y = rect_min_y.round() as i32;
                        let rect_max_x = rect_max_x.round() as i32;
                        let rect_max_y = rect_max_y.round() as i32;

                        unsafe {
                            self.gl.viewport(
                                rect_min_x,
                                size_in_pixels.1 as i32 - rect_max_y,
                                rect_max_x - rect_min_x,
                                rect_max_y - rect_min_y,
                            );
                        }

                        let info = egui::PaintCallbackInfo {
                            viewport: callback.rect,
                            clip_rect: *clip_rect,
                            pixels_per_point,
                            screen_size_px,
                        };

                        if let Some(callback) = callback.callback.downcast_ref::<CallbackFn>() {
                            (callback.f)(info, self);
                        } else {
                            tracing::warn!("Warning: Unsupported render callback. Expected egui_glow::CallbackFn");
                        }

                        check_for_gl_error!(&self.gl, "callback");

                        // Restore state:
                        unsafe { self.prepare_painting(screen_size_px, pixels_per_point) };
                    }
                }
            }
        }

        unsafe {
            self.vao.unbind(&self.gl);
            self.gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);

            self.gl.disable(glow::SCISSOR_TEST);

            check_for_gl_error!(&self.gl, "painting");
        }
    }

    #[inline(never)] // Easier profiling
    fn paint_mesh(&mut self, mesh: &Mesh) {
        debug_assert!(mesh.is_valid());
        if let Some(texture) = self.texture(mesh.texture_id) {
            unsafe {
                self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
                self.gl.buffer_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    bytemuck::cast_slice(&mesh.vertices),
                    glow::STREAM_DRAW,
                );

                self.gl
                    .bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.element_array_buffer));
                self.gl.buffer_data_u8_slice(
                    glow::ELEMENT_ARRAY_BUFFER,
                    bytemuck::cast_slice(&mesh.indices),
                    glow::STREAM_DRAW,
                );

                self.gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            }

            unsafe {
                self.gl.draw_elements(
                    glow::TRIANGLES,
                    mesh.indices.len() as i32,
                    glow::UNSIGNED_INT,
                    0,
                );
            }

            check_for_gl_error!(&self.gl, "paint_mesh");
        } else {
            tracing::warn!("Failed to find texture {:?}", mesh.texture_id);
        }
    }

    // ------------------------------------------------------------------------

    pub fn set_texture(&mut self, tex_id: egui::TextureId, delta: &egui::epaint::ImageDelta) {
        crate::profile_function!();

        self.assert_not_destroyed();

        let glow_texture = *self
            .textures
            .entry(tex_id)
            .or_insert_with(|| unsafe { self.gl.create_texture().unwrap() });
        unsafe {
            self.gl.bind_texture(glow::TEXTURE_2D, Some(glow_texture));
        }

        match &delta.image {
            egui::ImageData::Color(image) => {
                assert_eq!(
                    image.width() * image.height(),
                    image.pixels.len(),
                    "Mismatch between texture size and texel count"
                );

                let data: &[u8] = bytemuck::cast_slice(image.pixels.as_ref());

                self.upload_texture_srgb(delta.pos, image.size, delta.options, data);
            }
            egui::ImageData::Font(image) => {
                assert_eq!(
                    image.width() * image.height(),
                    image.pixels.len(),
                    "Mismatch between texture size and texel count"
                );

                let data: Vec<u8> = image
                    .srgba_pixels(None)
                    .flat_map(|a| a.to_array())
                    .collect();

                self.upload_texture_srgb(delta.pos, image.size, delta.options, &data);
            }
        };
    }

    fn upload_texture_srgb(
        &mut self,
        pos: Option<[usize; 2]>,
        [w, h]: [usize; 2],
        options: egui::TextureOptions,
        data: &[u8],
    ) {
        assert_eq!(data.len(), w * h * 4);
        assert!(
            w <= self.max_texture_side && h <= self.max_texture_side,
            "Got a texture image of size {}x{}, but the maximum supported texture side is only {}",
            w,
            h,
            self.max_texture_side
        );

        unsafe {
            self.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                options.magnification.glow_code() as i32,
            );
            self.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                options.minification.glow_code() as i32,
            );

            self.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as i32,
            );
            self.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );
            check_for_gl_error!(&self.gl, "tex_parameter");

            let (internal_format, src_format) = if self.is_webgl_1 {
                let format = if self.srgb_textures {
                    glow::SRGB_ALPHA
                } else {
                    glow::RGBA
                };
                (format, format)
            } else if self.srgb_textures {
                (glow::SRGB8_ALPHA8, glow::RGBA)
            } else {
                (glow::RGBA8, glow::RGBA)
            };

            self.gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);

            let level = 0;
            if let Some([x, y]) = pos {
                self.gl.tex_sub_image_2d(
                    glow::TEXTURE_2D,
                    level,
                    x as _,
                    y as _,
                    w as _,
                    h as _,
                    src_format,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(data),
                );
                check_for_gl_error!(&self.gl, "tex_sub_image_2d");
            } else {
                let border = 0;
                self.gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    level,
                    internal_format as _,
                    w as _,
                    h as _,
                    border,
                    src_format,
                    glow::UNSIGNED_BYTE,
                    Some(data),
                );
                check_for_gl_error!(&self.gl, "tex_image_2d");
            }
        }
    }

    pub fn free_texture(&mut self, tex_id: egui::TextureId) {
        if let Some(old_tex) = self.textures.remove(&tex_id) {
            unsafe { self.gl.delete_texture(old_tex) };
        }
    }

    /// Get the [`glow::Texture`] bound to a [`egui::TextureId`].
    pub fn texture(&self, texture_id: egui::TextureId) -> Option<glow::Texture> {
        self.textures.get(&texture_id).copied()
    }

    #[deprecated = "renamed 'texture'"]
    pub fn get_texture(&self, texture_id: egui::TextureId) -> Option<glow::Texture> {
        self.texture(texture_id)
    }

    #[allow(clippy::needless_pass_by_value)] // False positive
    pub fn register_native_texture(&mut self, native: glow::Texture) -> egui::TextureId {
        self.assert_not_destroyed();
        let id = egui::TextureId::User(self.next_native_tex_id);
        self.next_native_tex_id += 1;
        self.textures.insert(id, native);
        id
    }

    #[allow(clippy::needless_pass_by_value)] // False positive
    pub fn replace_native_texture(&mut self, id: egui::TextureId, replacing: glow::Texture) {
        if let Some(old_tex) = self.textures.insert(id, replacing) {
            self.textures_to_destroy.push(old_tex);
        }
    }

    pub fn read_screen_rgba(&self, [w, h]: [u32; 2]) -> Vec<u8> {
        let mut pixels = vec![0_u8; (w * h * 4) as usize];
        unsafe {
            self.gl.read_pixels(
                0,
                0,
                w as _,
                h as _,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(&mut pixels),
            );
        }
        pixels
    }

    pub fn read_screen_rgb(&self, [w, h]: [u32; 2]) -> Vec<u8> {
        let mut pixels = vec![0_u8; (w * h * 3) as usize];
        unsafe {
            self.gl.read_pixels(
                0,
                0,
                w as _,
                h as _,
                glow::RGB,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(&mut pixels),
            );
        }
        pixels
    }

    unsafe fn destroy_gl(&self) {
        self.gl.delete_program(self.program);
        for tex in self.textures.values() {
            self.gl.delete_texture(*tex);
        }
        self.gl.delete_buffer(self.vbo);
        self.gl.delete_buffer(self.element_array_buffer);
        for t in &self.textures_to_destroy {
            self.gl.delete_texture(*t);
        }
    }

    /// This function must be called before [`Painter`] is dropped, as [`Painter`] has some OpenGL objects
    /// that should be deleted.
    pub fn destroy(&mut self) {
        if !self.destroyed {
            unsafe {
                self.destroy_gl();
            }
            self.destroyed = true;
        }
    }

    fn assert_not_destroyed(&self) {
        assert!(!self.destroyed, "the egui glow has already been destroyed!");
    }
}

pub fn clear(gl: &glow::Context, screen_size_in_pixels: [u32; 2], clear_color: [f32; 4]) {
    crate::profile_function!();
    unsafe {
        gl.disable(glow::SCISSOR_TEST);

        gl.viewport(
            0,
            0,
            screen_size_in_pixels[0] as i32,
            screen_size_in_pixels[1] as i32,
        );
        gl.clear_color(
            clear_color[0],
            clear_color[1],
            clear_color[2],
            clear_color[3],
        );
        gl.clear(glow::COLOR_BUFFER_BIT);
    }
}

impl Drop for Painter {
    fn drop(&mut self) {
        if !self.destroyed {
            tracing::warn!(
                "You forgot to call destroy() on the egui glow painter. Resources will leak!"
            );
        }
    }
}

fn set_clip_rect(
    gl: &glow::Context,
    size_in_pixels: (u32, u32),
    pixels_per_point: f32,
    clip_rect: Rect,
) {
    // Transform clip rect to physical pixels:
    let clip_min_x = pixels_per_point * clip_rect.min.x;
    let clip_min_y = pixels_per_point * clip_rect.min.y;
    let clip_max_x = pixels_per_point * clip_rect.max.x;
    let clip_max_y = pixels_per_point * clip_rect.max.y;

    // Round to integer:
    let clip_min_x = clip_min_x.round() as i32;
    let clip_min_y = clip_min_y.round() as i32;
    let clip_max_x = clip_max_x.round() as i32;
    let clip_max_y = clip_max_y.round() as i32;

    // Clamp:
    let clip_min_x = clip_min_x.clamp(0, size_in_pixels.0 as i32);
    let clip_min_y = clip_min_y.clamp(0, size_in_pixels.1 as i32);
    let clip_max_x = clip_max_x.clamp(clip_min_x, size_in_pixels.0 as i32);
    let clip_max_y = clip_max_y.clamp(clip_min_y, size_in_pixels.1 as i32);

    unsafe {
        gl.scissor(
            clip_min_x,
            size_in_pixels.1 as i32 - clip_max_y,
            clip_max_x - clip_min_x,
            clip_max_y - clip_min_y,
        );
    }
}
