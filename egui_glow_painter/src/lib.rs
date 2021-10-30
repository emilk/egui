#![allow(unsafe_code)]

pub mod create_context_for_canvas;
mod misc_util;
mod post_process;
mod shader_version;
mod vao_emulate;

use egui::{
    emath::Rect,
    epaint::{Color32, Mesh, Vertex},
};
pub use glow::Context;

use memoffset::offset_of;

use glow::HasContext;

use crate::misc_util::{
    as_u8_slice, compile_shader, glow_debug_print, link_program, srgbtexture2d,
};
use crate::post_process::PostProcess;
use crate::shader_version::ShaderVersion;

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
    vertex_array: vao_emulate::EmulatedVao,
    srgb_support: bool,
    /// `None` means unallocated (freed) slot.
    user_textures: Vec<Option<UserTexture>>,
    post_process: Option<PostProcess>,
    vertex_buffer: glow::Buffer,
    element_array_buffer: glow::Buffer,

    // Stores outdated OpenGL textures that are yet to be deleted
    old_textures: Vec<glow::Texture>,
    // Only in debug builds, to make sure we are destroyed correctly.
    #[cfg(debug_assertions)]
    destroyed: bool,
}

#[derive(Default)]
struct UserTexture {
    /// Pending upload (will be emptied later).
    /// This is the format glow likes.
    data: Vec<u8>,
    size: (usize, usize),

    /// Lazily uploaded
    gl_texture: Option<glow::Texture>,
}

impl Painter {
    pub fn new(gl: &glow::Context, canvas_dimension: Option<[i32; 2]>) -> Painter {
        let shader_version = ShaderVersion::get(gl);
        let is_webgl_1 = shader_version == ShaderVersion::Es100;
        let header = shader_version.version();
        glow_debug_print(header);
        let mut v_src = header.to_owned();
        v_src.push_str(VERT_SRC);
        let mut f_src = header.to_owned();

        let srgb_support = gl.supported_extensions().contains("EXT_sRGB");
        let post_process = match (shader_version, srgb_support) {
            //WebGL2 support sRGB default
            (ShaderVersion::Es300, _) | (ShaderVersion::Es100, true) => {
                glow_debug_print("WebGL with sRGB enabled so turn on post process");
                let canvas_dimension = canvas_dimension.unwrap();
                //Add sRGB support marker for fragment shader
                f_src.push_str("#define SRGB_SUPPORTED \n");
                //install post process to correct sRGB color
                PostProcess::new(gl, is_webgl_1, canvas_dimension[0], canvas_dimension[1]).ok()
            }
            //WebGL1 without sRGB support disable postprocess and use fallback shader
            (ShaderVersion::Es100, false) => None,
            //OpenGL 2.1 or above always support sRGB so add sRGB support marker
            _ => {
                f_src.push_str("#define SRGB_SUPPORTED \n");
                None
            }
        };

        f_src.push_str(FRAG_SRC);

        unsafe {
            let v = compile_shader(gl, glow::VERTEX_SHADER, &v_src)
                .map_err(|problems| {
                    glow_debug_print(format!(
                        "failed to compile vertex shader due to errors \n {}",
                        problems
                    ))
                })
                .unwrap();
            let f = compile_shader(gl, glow::FRAGMENT_SHADER, &f_src)
                .map_err(|problems| {
                    glow_debug_print(format!(
                        "failed to compile fragment shader due to errors \n {}",
                        problems
                    ))
                })
                .unwrap();
            let program = link_program(gl, [v, f].iter())
                .map_err(|problems| {
                    glow_debug_print(format!(
                        "failed to link shaders due to errors \n {}",
                        problems
                    ))
                })
                .unwrap();
            gl.detach_shader(program, v);
            gl.detach_shader(program, f);
            gl.delete_shader(v);
            gl.delete_shader(f);
            let u_screen_size = gl.get_uniform_location(program, "u_screen_size").unwrap();
            let u_sampler = gl.get_uniform_location(program, "u_sampler").unwrap();
            let vertex_buffer = gl.create_buffer().unwrap();
            let element_array_buffer = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));
            let a_pos_loc = gl.get_attrib_location(program, "a_pos").unwrap();
            let a_tc_loc = gl.get_attrib_location(program, "a_tc").unwrap();
            let a_srgba_loc = gl.get_attrib_location(program, "a_srgba").unwrap();
            let mut vertex_array = vao_emulate::EmulatedVao::new(vertex_buffer);
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
            vertex_array.add_new_attribute(position_buffer_info);
            vertex_array.add_new_attribute(tex_coord_buffer_info);
            vertex_array.add_new_attribute(color_buffer_info);
            assert_eq!(gl.get_error(), glow::NO_ERROR, "OpenGL error occurred!");

            Painter {
                program,
                u_screen_size,
                u_sampler,
                egui_texture: None,
                egui_texture_version: None,
                is_webgl_1,
                vertex_array,
                srgb_support,
                user_textures: Default::default(),
                post_process,
                vertex_buffer,
                element_array_buffer,
                old_textures: Vec::new(),
                #[cfg(debug_assertions)]
                destroyed: false,
            }
        }
    }

    pub fn upload_egui_texture(&mut self, gl: &glow::Context, texture: &egui::Texture) {
        self.assert_not_destroyed();

        if self.egui_texture_version == Some(texture.version) {
            return; // No change
        }
        let gamma = if self.post_process.is_none() {
            1.0 / 2.0
        } else {
            1.0
        };
        let pixels: Vec<u8> = texture
            .srgba_pixels(gamma)
            .flat_map(|a| Vec::from(a.to_array()))
            .collect();

        if let Some(old_tex) = std::mem::replace(
            &mut self.egui_texture,
            Some(srgbtexture2d(
                gl,
                self.is_webgl_1,
                self.srgb_support,
                &pixels,
                texture.width,
                texture.height,
            )),
        ) {
            unsafe {
                gl.delete_texture(old_tex);
            }
        }
        self.egui_texture_version = Some(texture.version);
    }

    unsafe fn prepare_painting(
        &mut self,
        inner_size: [u32; 2],
        gl: &glow::Context,
        pixels_per_point: f32,
    ) -> (u32, u32) {
        gl.enable(glow::SCISSOR_TEST);
        // egui outputs mesh in both winding orders:
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

        let [width_in_pixels, height_in_pixels] = inner_size;
        let width_in_points = width_in_pixels as f32 / pixels_per_point;
        let height_in_points = height_in_pixels as f32 / pixels_per_point;

        gl.viewport(0, 0, width_in_pixels as i32, height_in_pixels as i32);

        gl.use_program(Some(self.program));

        // The texture coordinates for text are so that both nearest and linear should work with the egui font texture.
        // For user textures linear sampling is more likely to be the right choice.
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
    /// As well as this, the following objects will be rebound:
    /// - Vertex Array
    /// - Vertex Buffer
    /// - Element Buffer
    /// - Texture (and active texture will be set to 0)
    /// - Program
    ///
    /// Please be mindful of these effects when integrating into your program, and also be mindful
    /// of the effects your program might have on this code. Look at the source if in doubt.
    pub fn paint_meshes(
        &mut self,
        inner_size: [u32; 2],
        gl: &glow::Context,
        pixels_per_point: f32,
        clipped_meshes: Vec<egui::ClippedMesh>,
        egui_texture: &egui::Texture,
    ) {
        //chimera of egui_glow and egui_web
        self.assert_not_destroyed();

        self.upload_egui_texture(gl, egui_texture);
        self.upload_pending_user_textures(gl);
        if let Some(ref mut post_process) = self.post_process {
            post_process.begin(gl, inner_size[0] as i32, inner_size[1] as i32);
        }
        let size_in_pixels = unsafe { self.prepare_painting(inner_size, gl, pixels_per_point) };
        for egui::ClippedMesh(clip_rect, mesh) in clipped_meshes {
            self.paint_mesh(gl, size_in_pixels, pixels_per_point, clip_rect, &mesh)
        }
        if let Some(ref post_process) = self.post_process {
            post_process.end(gl);
        }
        unsafe { assert_eq!(glow::NO_ERROR, gl.get_error(), "GL error occurred!") }
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

    // ------------------------------------------------------------------------
    // user textures: this is an experimental feature.
    // No need to implement this in your egui integration!

    pub fn alloc_user_texture(&mut self) -> egui::TextureId {
        self.assert_not_destroyed();

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

    /// register glow texture as egui texture
    /// Usable for render to image rectangle
    pub fn register_glow_texture(&mut self, texture: glow::Texture) -> egui::TextureId {
        self.assert_not_destroyed();

        let id = self.alloc_user_texture();
        if let egui::TextureId::User(id) = id {
            if let Some(Some(user_texture)) = self.user_textures.get_mut(id as usize) {
                if let UserTexture {
                    gl_texture: Some(old_tex),
                    ..
                } = std::mem::replace(
                    user_texture,
                    UserTexture {
                        data: vec![],
                        size: (0, 0),
                        gl_texture: Some(texture),
                    },
                ) {
                    self.old_textures.push(old_tex);
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
        self.assert_not_destroyed();
        assert_eq!(
            size.0 * size.1,
            pixels.len(),
            "Mismatch between size and texel count"
        );

        if let egui::TextureId::User(id) = id {
            if let Some(Some(user_texture)) = self.user_textures.get_mut(id as usize) {
                let data: Vec<u8> = pixels
                    .iter()
                    .flat_map(|srgba| Vec::from(srgba.to_array()))
                    .collect();

                if let UserTexture {
                    gl_texture: Some(old_tex),
                    ..
                } = std::mem::replace(
                    user_texture,
                    UserTexture {
                        data,
                        size,
                        gl_texture: None,
                    },
                ) {
                    self.old_textures.push(old_tex);
                }
            }
        }
    }

    pub fn free_user_texture(&mut self, id: egui::TextureId) {
        self.assert_not_destroyed();

        if let egui::TextureId::User(id) = id {
            let index = id as usize;
            if index < self.user_textures.len() {
                self.user_textures[index] = None;
            }
        }
    }

    pub fn get_texture(&self, texture_id: egui::TextureId) -> Option<glow::Texture> {
        self.assert_not_destroyed();

        match texture_id {
            egui::TextureId::Egui => self.egui_texture,
            egui::TextureId::User(id) => self.user_textures.get(id as usize)?.as_ref()?.gl_texture,
        }
    }

    pub fn upload_pending_user_textures(&mut self, gl: &glow::Context) {
        self.assert_not_destroyed();

        for user_texture in self.user_textures.iter_mut().flatten() {
            if user_texture.gl_texture.is_none() {
                let data = std::mem::take(&mut user_texture.data);
                user_texture.gl_texture = Some(srgbtexture2d(
                    gl,
                    self.is_webgl_1,
                    self.srgb_support,
                    &data,
                    user_texture.size.0,
                    user_texture.size.1,
                ));
                user_texture.size = (0, 0);
            }
        }
        for t in self.old_textures.drain(..) {
            unsafe {
                gl.delete_texture(t);
            }
        }
    }

    unsafe fn destroy_gl(&self, gl: &glow::Context) {
        gl.delete_program(self.program);
        if let Some(tex) = self.egui_texture {
            gl.delete_texture(tex);
        }
        for tex in self.user_textures.iter().flatten() {
            if let Some(t) = tex.gl_texture {
                gl.delete_texture(t);
            }
        }
        gl.delete_buffer(self.vertex_buffer);
        gl.delete_buffer(self.element_array_buffer);
        for t in &self.old_textures {
            gl.delete_texture(*t);
        }
    }

    /// This function must be called before Painter is dropped, as Painter has some OpenGL objects
    /// that should be deleted.
    #[cfg(debug_assertions)]
    pub fn destroy(&mut self, gl: &glow::Context) {
        assert!(!self.destroyed, "Only destroy once!");
        unsafe {
            self.destroy_gl(gl);
            if let Some(ref post_process) = self.post_process {
                post_process.destroy(gl);
            }
        }
        self.destroyed = true;
    }

    #[cfg(not(debug_assertions))]
    pub fn destroy(&self, gl: &glow::Context) {
        unsafe {
            self.destroy_gl(gl);
            if let Some(ref post_process) = self.post_process {
                post_process.destroy(gl);
            }
        }
    }

    #[cfg(debug_assertions)]
    fn assert_not_destroyed(&self) {
        assert!(!self.destroyed, "egui has already been destroyed!");
    }

    #[inline(always)]
    #[cfg(not(debug_assertions))]
    #[allow(clippy::unused_self)]
    fn assert_not_destroyed(&self) {}
}
// ported from egui_web
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
        #[cfg(debug_assertions)]
        assert!(
            self.destroyed,
            "Make sure to destroy() rather than dropping, to avoid leaking OpenGL objects!"
        );
    }
}

impl epi::TextureAllocator for crate::Painter {
    fn alloc_srgba_premultiplied(
        &mut self,
        size: (usize, usize),
        srgba_pixels: &[Color32],
    ) -> egui::TextureId {
        let id = self.alloc_user_texture();
        self.set_user_texture(id, size, srgba_pixels);
        id
    }

    fn free(&mut self, id: egui::TextureId) {
        self.free_user_texture(id)
    }
}
