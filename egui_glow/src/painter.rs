#![allow(unsafe_code)]

use std::collections::HashMap;

use bytemuck::cast_slice;
use egui::{
    emath::Rect,
    epaint::{Color32, Mesh, Vertex},
};
use glow::HasContext;
use memoffset::offset_of;

use crate::misc_util::{
    as_u8_slice, check_for_gl_error, compile_shader, glow_print, link_program, srgb_texture2d,
};
use crate::post_process::PostProcess;
use crate::shader_version::ShaderVersion;
use crate::vao_emulate;

pub use glow::Context;

const VERT_SRC: &str = include_str!("shader/vertex.glsl");
const FRAG_SRC: &str = include_str!("shader/fragment.glsl");

/// OpenGL painter
///
/// This struct must be destroyed with [`Painter::destroy`] before dropping, to ensure OpenGL
/// objects have been properly deleted and are not leaked.
pub struct Painter {
    program: glow::Program,
    u_screen_size: glow::UniformLocation,
    u_sampler: glow::UniformLocation,
    egui_texture: Option<glow::Texture>,
    egui_texture_version: Option<u64>,
    is_webgl_1: bool,
    is_embedded: bool,
    vertex_array: crate::misc_util::VAO,
    srgb_support: bool,
    /// The filter used for subsequent textures.
    texture_filter: TextureFilter,
    post_process: Option<PostProcess>,
    vertex_buffer: glow::Buffer,
    element_array_buffer: glow::Buffer,

    /// Index is the same as in [`egui::TextureId::User`].
    user_textures: HashMap<u64, glow::Texture>,

    #[cfg(feature = "epi")]
    next_native_tex_id: u64, // TODO: 128-bit texture space?

    /// Stores outdated OpenGL textures that are yet to be deleted
    textures_to_destroy: Vec<glow::Texture>,

    /// Used to make sure we are destroyed correctly.
    destroyed: bool,
}

#[derive(Copy, Clone)]
pub enum TextureFilter {
    Linear,
    Nearest,
}

impl Default for TextureFilter {
    fn default() -> Self {
        TextureFilter::Linear
    }
}

impl TextureFilter {
    pub(crate) fn glow_code(&self) -> u32 {
        match self {
            TextureFilter::Linear => glow::LINEAR,
            TextureFilter::Nearest => glow::NEAREST,
        }
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
        gl: &glow::Context,
        pp_fb_extent: Option<[i32; 2]>,
        shader_prefix: &str,
    ) -> Result<Painter, String> {
        check_for_gl_error(gl, "before Painter::new");

        let support_vao = crate::misc_util::supports_vao(gl);
        let shader_version = ShaderVersion::get(gl);
        let is_webgl_1 = shader_version == ShaderVersion::Es100;
        let header = shader_version.version();
        glow_print(format!("Shader header: {:?}.", header));
        let srgb_support = gl.supported_extensions().contains("EXT_sRGB");

        let (post_process, srgb_support_define) = match (shader_version, srgb_support) {
            // WebGL2 support sRGB default
            (ShaderVersion::Es300, _) | (ShaderVersion::Es100, true) => unsafe {
                // Add sRGB support marker for fragment shader
                if let Some([width, height]) = pp_fb_extent {
                    glow_print("WebGL with sRGB enabled. Turning on post processing for linear framebuffer blending.");
                    // install post process to correct sRGB color:
                    (
                        Some(PostProcess::new(
                            gl,
                            shader_prefix,
                            support_vao,
                            is_webgl_1,
                            width,
                            height,
                        )?),
                        "#define SRGB_SUPPORTED",
                    )
                } else {
                    glow_print("WebGL or OpenGL ES detected but PostProcess disabled because dimension is None");
                    (None, "")
                }
            },

            // WebGL1 without sRGB support disable postprocess and use fallback shader
            (ShaderVersion::Es100, false) => (None, ""),

            // OpenGL 2.1 or above always support sRGB so add sRGB support marker
            _ => (None, "#define SRGB_SUPPORTED"),
        };

        unsafe {
            let vert = compile_shader(
                gl,
                glow::VERTEX_SHADER,
                &format!(
                    "{}\n{}\n{}\n{}",
                    header,
                    shader_prefix,
                    shader_version.is_new_shader_interface(),
                    VERT_SRC
                ),
            )?;
            let frag = compile_shader(
                gl,
                glow::FRAGMENT_SHADER,
                &format!(
                    "{}\n{}\n{}\n{}\n{}",
                    header,
                    shader_prefix,
                    srgb_support_define,
                    shader_version.is_new_shader_interface(),
                    FRAG_SRC
                ),
            )?;
            let program = link_program(gl, [vert, frag].iter())?;
            gl.detach_shader(program, vert);
            gl.detach_shader(program, frag);
            gl.delete_shader(vert);
            gl.delete_shader(frag);
            let u_screen_size = gl.get_uniform_location(program, "u_screen_size").unwrap();
            let u_sampler = gl.get_uniform_location(program, "u_sampler").unwrap();
            let vertex_buffer = gl.create_buffer()?;
            let element_array_buffer = gl.create_buffer()?;
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));
            let a_pos_loc = gl.get_attrib_location(program, "a_pos").unwrap();
            let a_tc_loc = gl.get_attrib_location(program, "a_tc").unwrap();
            let a_srgba_loc = gl.get_attrib_location(program, "a_srgba").unwrap();
            let mut vertex_array = if support_vao {
                crate::misc_util::VAO::native(gl)
            } else {
                crate::misc_util::VAO::emulated()
            };
            vertex_array.bind_vertex_array(gl);
            vertex_array.bind_buffer(gl, &vertex_buffer);
            let stride = std::mem::size_of::<Vertex>() as i32;
            let position_buffer_info = vao_emulate::BufferInfo {
                location: a_pos_loc,
                vector_size: 2,
                data_type: glow::FLOAT,
                normalized: false,
                stride,
                offset: offset_of!(Vertex, pos) as i32,
            };
            let tex_coord_buffer_info = vao_emulate::BufferInfo {
                location: a_tc_loc,
                vector_size: 2,
                data_type: glow::FLOAT,
                normalized: false,
                stride,
                offset: offset_of!(Vertex, uv) as i32,
            };
            let color_buffer_info = vao_emulate::BufferInfo {
                location: a_srgba_loc,
                vector_size: 4,
                data_type: glow::UNSIGNED_BYTE,
                normalized: false,
                stride,
                offset: offset_of!(Vertex, color) as i32,
            };
            vertex_array.add_new_attribute(gl, position_buffer_info);
            vertex_array.add_new_attribute(gl, tex_coord_buffer_info);
            vertex_array.add_new_attribute(gl, color_buffer_info);
            check_for_gl_error(gl, "after Painter::new");

            Ok(Painter {
                program,
                u_screen_size,
                u_sampler,
                egui_texture: None,
                egui_texture_version: None,
                is_webgl_1,
                is_embedded: matches!(shader_version, ShaderVersion::Es100 | ShaderVersion::Es300),
                vertex_array,
                srgb_support,
                texture_filter: Default::default(),
                post_process,
                vertex_buffer,
                element_array_buffer,
                user_textures: Default::default(),
                #[cfg(feature = "epi")]
                next_native_tex_id: 1 << 32,
                textures_to_destroy: Vec::new(),
                destroyed: false,
            })
        }
    }

    pub fn upload_egui_texture(&mut self, gl: &glow::Context, font_image: &egui::FontImage) {
        self.assert_not_destroyed();

        if self.egui_texture_version == Some(font_image.version) {
            return; // No change
        }
        let gamma = if self.is_embedded && self.post_process.is_none() {
            1.0 / 2.2
        } else {
            1.0
        };
        let pixels: Vec<u8> = font_image
            .srgba_pixels(gamma)
            .flat_map(|a| Vec::from(a.to_array()))
            .collect();

        if let Some(old_tex) = std::mem::replace(
            &mut self.egui_texture,
            Some(srgb_texture2d(
                gl,
                self.is_webgl_1,
                self.srgb_support,
                self.texture_filter,
                &pixels,
                font_image.width,
                font_image.height,
            )),
        ) {
            unsafe {
                gl.delete_texture(old_tex);
            }
        }
        self.egui_texture_version = Some(font_image.version);
    }

    unsafe fn prepare_painting(
        &mut self,
        [width_in_pixels, height_in_pixels]: [u32; 2],
        gl: &glow::Context,
        pixels_per_point: f32,
    ) -> (u32, u32) {
        gl.enable(glow::SCISSOR_TEST);
        // egui outputs mesh in both winding orders
        gl.disable(glow::CULL_FACE);

        gl.enable(glow::BLEND);
        gl.blend_equation(glow::FUNC_ADD);
        gl.blend_func_separate(
            // egui outputs colors with premultiplied alpha:
            glow::ONE,
            glow::ONE_MINUS_SRC_ALPHA,
            // Less important, but this is technically the correct alpha blend function
            // when you want to make use of the framebuffer alpha (for screenshots, compositing, etc).
            glow::ONE_MINUS_DST_ALPHA,
            glow::ONE,
        );

        let width_in_points = width_in_pixels as f32 / pixels_per_point;
        let height_in_points = height_in_pixels as f32 / pixels_per_point;

        gl.viewport(0, 0, width_in_pixels as i32, height_in_pixels as i32);
        gl.use_program(Some(self.program));

        gl.uniform_2_f32(Some(&self.u_screen_size), width_in_points, height_in_points);
        gl.uniform_1_i32(Some(&self.u_sampler), 0);
        gl.active_texture(glow::TEXTURE0);
        self.vertex_array.bind_vertex_array(gl);

        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.element_array_buffer));

        (width_in_pixels, height_in_pixels)
    }

    /// Main entry-point for painting a frame.
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
    pub fn paint_meshes(
        &mut self,
        gl: &glow::Context,
        inner_size: [u32; 2],
        pixels_per_point: f32,
        clipped_meshes: Vec<egui::ClippedMesh>,
    ) {
        self.assert_not_destroyed();

        if let Some(ref mut post_process) = self.post_process {
            unsafe {
                post_process.begin(gl, inner_size[0] as i32, inner_size[1] as i32);
            }
        }
        let size_in_pixels = unsafe { self.prepare_painting(inner_size, gl, pixels_per_point) };
        for egui::ClippedMesh(clip_rect, mesh) in clipped_meshes {
            self.paint_mesh(gl, size_in_pixels, pixels_per_point, clip_rect, &mesh);
        }
        unsafe {
            self.vertex_array.unbind_vertex_array(gl);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);

            if let Some(ref post_process) = self.post_process {
                post_process.end(gl);
            }

            gl.disable(glow::SCISSOR_TEST);

            check_for_gl_error(gl, "painting");
        }
    }

    #[inline(never)] // Easier profiling
    fn paint_mesh(
        &mut self,
        gl: &glow::Context,
        size_in_pixels: (u32, u32),
        pixels_per_point: f32,
        clip_rect: Rect,
        mesh: &Mesh,
    ) {
        debug_assert!(mesh.is_valid());
        if let Some(texture) = self.get_texture(mesh.texture_id) {
            unsafe {
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vertex_buffer));
                gl.buffer_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    as_u8_slice(mesh.vertices.as_slice()),
                    glow::STREAM_DRAW,
                );

                gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.element_array_buffer));
                gl.buffer_data_u8_slice(
                    glow::ELEMENT_ARRAY_BUFFER,
                    as_u8_slice(mesh.indices.as_slice()),
                    glow::STREAM_DRAW,
                );

                gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            }
            // Transform clip rect to physical pixels:
            let clip_min_x = pixels_per_point * clip_rect.min.x;
            let clip_min_y = pixels_per_point * clip_rect.min.y;
            let clip_max_x = pixels_per_point * clip_rect.max.x;
            let clip_max_y = pixels_per_point * clip_rect.max.y;

            // Make sure clip rect can fit within a `u32`:
            let clip_min_x = clip_min_x.clamp(0.0, size_in_pixels.0 as f32);
            let clip_min_y = clip_min_y.clamp(0.0, size_in_pixels.1 as f32);
            let clip_max_x = clip_max_x.clamp(clip_min_x, size_in_pixels.0 as f32);
            let clip_max_y = clip_max_y.clamp(clip_min_y, size_in_pixels.1 as f32);

            let clip_min_x = clip_min_x.round() as i32;
            let clip_min_y = clip_min_y.round() as i32;
            let clip_max_x = clip_max_x.round() as i32;
            let clip_max_y = clip_max_y.round() as i32;

            unsafe {
                gl.scissor(
                    clip_min_x,
                    size_in_pixels.1 as i32 - clip_max_y,
                    clip_max_x - clip_min_x,
                    clip_max_y - clip_min_y,
                );
                gl.draw_elements(
                    glow::TRIANGLES,
                    mesh.indices.len() as i32,
                    glow::UNSIGNED_INT,
                    0,
                );
            }
        }
    }

    // Set the filter to be used for any subsequent textures loaded via
    // [`Self::set_texture`].
    pub fn set_texture_filter(&mut self, texture_filter: TextureFilter) {
        self.texture_filter = texture_filter;
    }

    // ------------------------------------------------------------------------

    #[cfg(feature = "epi")]
    pub fn set_texture(&mut self, gl: &glow::Context, tex_id: u64, image: &epi::Image) {
        self.assert_not_destroyed();

        assert_eq!(
            image.size[0] * image.size[1],
            image.pixels.len(),
            "Mismatch between texture size and texel count"
        );

        let data: &[u8] = cast_slice(image.pixels.as_ref());

        let gl_texture = srgb_texture2d(
            gl,
            self.is_webgl_1,
            self.srgb_support,
            self.texture_filter,
            data,
            image.size[0],
            image.size[1],
        );

        if let Some(old_tex) = self.user_textures.insert(tex_id, gl_texture) {
            self.textures_to_destroy.push(old_tex);
        }
    }

    pub fn free_texture(&mut self, tex_id: u64) {
        self.user_textures.remove(&tex_id);
    }

    fn get_texture(&self, texture_id: egui::TextureId) -> Option<glow::Texture> {
        self.assert_not_destroyed();

        match texture_id {
            egui::TextureId::Egui => self.egui_texture,
            egui::TextureId::User(id) => self.user_textures.get(&id).copied(),
        }
    }

    unsafe fn destroy_gl(&self, gl: &glow::Context) {
        gl.delete_program(self.program);
        if let Some(tex) = self.egui_texture {
            gl.delete_texture(tex);
        }
        for tex in self.user_textures.values() {
            gl.delete_texture(*tex);
        }
        gl.delete_buffer(self.vertex_buffer);
        gl.delete_buffer(self.element_array_buffer);
        for t in &self.textures_to_destroy {
            gl.delete_texture(*t);
        }
    }

    /// This function must be called before Painter is dropped, as Painter has some OpenGL objects
    /// that should be deleted.

    pub fn destroy(&mut self, gl: &glow::Context) {
        if !self.destroyed {
            unsafe {
                self.destroy_gl(gl);
                if let Some(ref post_process) = self.post_process {
                    post_process.destroy(gl);
                }
            }
            self.destroyed = true;
        }
    }

    fn assert_not_destroyed(&self) {
        assert!(!self.destroyed, "the egui glow has already been destroyed!");
    }
}

pub fn clear(gl: &glow::Context, dimension: [u32; 2], clear_color: egui::Rgba) {
    unsafe {
        gl.disable(glow::SCISSOR_TEST);

        gl.viewport(0, 0, dimension[0] as i32, dimension[1] as i32);

        let clear_color: Color32 = clear_color.into();
        gl.clear_color(
            clear_color[0] as f32 / 255.0,
            clear_color[1] as f32 / 255.0,
            clear_color[2] as f32 / 255.0,
            clear_color[3] as f32 / 255.0,
        );
        gl.clear(glow::COLOR_BUFFER_BIT);
    }
}

impl Drop for Painter {
    fn drop(&mut self) {
        if !self.destroyed {
            eprintln!(
                "You forgot to call destroy() on the egui glow painter. Resources will leak!"
            );
        }
    }
}

#[cfg(feature = "epi")]
impl epi::NativeTexture for Painter {
    type Texture = glow::Texture;

    fn register_native_texture(&mut self, native: Self::Texture) -> egui::TextureId {
        self.assert_not_destroyed();

        let id = self.next_native_tex_id;
        self.next_native_tex_id += 1;

        self.user_textures.insert(id, native);

        egui::TextureId::User(id as u64)
    }

    fn replace_native_texture(&mut self, id: egui::TextureId, replacing: Self::Texture) {
        if let egui::TextureId::User(id) = id {
            if let Some(old_tex) = self.user_textures.insert(id, replacing) {
                self.textures_to_destroy.push(old_tex);
            }
        }
    }
}
