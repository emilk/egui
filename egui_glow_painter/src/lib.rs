#![allow(unsafe_code)]

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue};

use egui::{
    emath::Rect,
    epaint::{Color32, Mesh, Vertex},
};
pub use glow::Context;
use memoffset::offset_of;

use std::convert::TryInto;

use glow::HasContext;

use std::process::exit;

#[cfg(target_arch = "wasm32")]
use web_sys::HtmlCanvasElement;

const VERT_SRC: &str = include_str!("shader/vertex.glsl");
const FRAG_SRC: &str = include_str!("shader/fragment.glsl");
#[cfg(target_arch = "wasm32")]
pub fn init_glow_context_from_canvas(canvas: HtmlCanvasElement) -> glow::Context {
    let ctx = canvas.get_context("webgl2");
    if let Ok(ctx) = ctx {
        glow_debug_print("webgl found");
        if let Some(ctx) = ctx {
            glow_debug_print("webgl 2 selected");
            let gl_ctx = ctx.dyn_into::<web_sys::WebGl2RenderingContext>().unwrap();
            glow::Context::from_webgl2_context(gl_ctx)
        } else {
            let ctx = canvas.get_context("webgl");
            if let Ok(ctx) = ctx {
                glow_debug_print("falling back to webgl1");
                if let Some(ctx) = ctx {
                    glow_debug_print("webgl selected");
                    let gl_ctx = ctx.dyn_into::<web_sys::WebGlRenderingContext>().unwrap();
                    glow_debug_print("success");
                    glow::Context::from_webgl1_context(gl_ctx)
                } else {
                    glow_debug_print("tried webgl1 but cant get context");
                    exit(1)
                }
            } else {
                glow_debug_print("tried webgl1 but cant get context");
                exit(1)
            }
        }
    } else {
        glow_debug_print("tried webgl2 but something went wrong");
        exit(1)
    }
}

fn srgbtexture2d(
    gl: &glow::Context,
    compatibility_mode: bool,
    srgb_support: bool,
    data: &[u8],
    w: usize,
    h: usize,
) -> glow::Texture {
    assert_eq!(data.len(), w * h * 4);
    assert!(w >= 1);
    assert!(h >= 1);
    unsafe {
        let tex = gl.create_texture().unwrap();
        gl.bind_texture(glow::TEXTURE_2D, Some(tex));

        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::LINEAR as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::LINEAR as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_S,
            glow::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_T,
            glow::CLAMP_TO_EDGE as i32,
        );
        // not supported on WebGL2 disabled firefox
        if compatibility_mode {
            glow_debug_print(format!("w : {} h : {}", w as i32, h as i32));
            let format = if srgb_support {
                glow::SRGB_ALPHA
            } else {
                glow::RGBA
            };
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                format as i32,
                w as i32,
                h as i32,
                0,
                format,
                glow::UNSIGNED_BYTE,
                Some(data),
            );
            if !srgb_support{
                gl.generate_mipmap(glow::TEXTURE_2D);
            }
            //gl.bind_texture(glow::TEXTURE_2D, None);
        } else {
            gl.tex_storage_2d(glow::TEXTURE_2D, 1, glow::SRGB8_ALPHA8, w as i32, h as i32);
            gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                0,
                0,
                w as i32,
                h as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(data),
            );
        }
        assert_eq!(gl.get_error(), glow::NO_ERROR, "OpenGL error occurred!");
        tex
    }
}

unsafe fn as_u8_slice<T>(s: &[T]) -> &[u8] {
    std::slice::from_raw_parts(s.as_ptr().cast::<u8>(), s.len() * std::mem::size_of::<T>())
}

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
    webgl_1_compatibility_mode: bool,
    vertex_array:glow::VertexArray,
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
enum ShaderVersion {
    Gl120,
    Gl140,
    Es100,
    Es300,
}

impl ShaderVersion {
    fn get(gl: &glow::Context) -> Self {
        let shading_lang = unsafe { gl.get_parameter_string(glow::SHADING_LANGUAGE_VERSION) };
        glow_debug_print(&shading_lang);
        Self::parse(&shading_lang)
    }

    #[inline]
    fn parse(glsl_ver: &str) -> Self {
        let start = glsl_ver.find(|c| char::is_ascii_digit(&c)).unwrap();
        let es = glsl_ver[..start].contains(" ES ");
        let ver = glsl_ver[start..].splitn(2, ' ').next().unwrap();
        let [maj, min]: [u8; 2] = ver
            .splitn(3, '.')
            .take(2)
            .map(|x| x.parse().unwrap_or_default())
            .collect::<Vec<u8>>()
            .try_into()
            .unwrap();
        if es {
            if maj >= 3 {
                Self::Es300
            } else {
                Self::Es100
            }
        } else if maj > 1 || (maj == 1 && min >= 40) {
            Self::Gl140
        } else {
            Self::Gl120
        }
    }

    fn version(&self) -> &'static str {
        match self {
            Self::Gl120 => "#version 120\n",
            Self::Gl140 => "#version 140\n",
            Self::Es100 => "#version 100\n",
            Self::Es300 => "#version 300 es\n",
        }
    }
}

#[test]
fn test_shader_version() {
    use ShaderVersion::{Es100, Es300, Gl120, Gl140};
    for (s, v) in [
        ("1.2 OpenGL foo bar", Gl120),
        ("3.0", Gl140),
        ("0.0", Gl120),
        ("OpenGL ES GLSL 3.00 (WebGL2)", Es300),
        ("OpenGL ES GLSL 1.00 (WebGL)", Es100),
        ("OpenGL ES GLSL ES 1.00 foo bar", Es100),
        ("WebGL GLSL ES 3.00 foo bar", Es300),
        ("WebGL GLSL ES 3.00", Es300),
        ("WebGL GLSL ES 1.0 foo bar", Es100),
    ] {
        assert_eq!(ShaderVersion::parse(s), v);
    }
}
impl Painter {
    pub fn new(gl: &glow::Context, canvas_dimension: Option<[i32; 2]>) -> Painter {
        let shader_version = ShaderVersion::get(gl);
        let webgl_1_compatibility_mode = if shader_version == ShaderVersion::Es100 {
            true
        } else {
            false
        };
        let header = shader_version.version();
        glow_debug_print(header);
        let mut v_src = header.to_owned();
        v_src.push_str(VERT_SRC);
        let mut f_src = header.to_owned();


        let srgb_support = gl.supported_extensions().contains("EXT_sRGB");
        let post_process = match (shader_version,srgb_support) {
            //WebGL2 support sRGB default
            (ShaderVersion::Es300,_)|(ShaderVersion::Es100,true)=>{
                glow_debug_print("WebGL with sRGB enabled so turn on post process");
                let canvas_dimension=canvas_dimension.unwrap();
                let webgl_1= shader_version==ShaderVersion::Es100;
                f_src.push_str("#define SRGB_SUPPORTED \n");
                PostProcess::new(gl,webgl_1,canvas_dimension[0],canvas_dimension[1]).ok()
            },
            (ShaderVersion::Es100,false)=>{
                None
            }
            _=>{
                f_src.push_str("#define SRGB_SUPPORTED \n");
                None
            }
        };

        f_src.push_str(FRAG_SRC);

        unsafe {
            let v = gl.create_shader(glow::VERTEX_SHADER).unwrap();
            glow_debug_print("gl::create_shader success");
            gl.shader_source(v, &v_src);
            gl.compile_shader(v);
            if !gl.get_shader_compile_status(v) {
                glow_debug_print(format!(
                    "Failed to compile vertex shader: {}",
                    gl.get_shader_info_log(v)
                ));
                exit(1);
            }

            let f = gl.create_shader(glow::FRAGMENT_SHADER).unwrap();
            glow_debug_print("gl::create_shader success");
            gl.shader_source(f, &f_src);
            gl.compile_shader(f);
            if !gl.get_shader_compile_status(f) {
                glow_debug_print(format!(
                    "Failed to compile fragment shader: {}",
                    gl.get_shader_info_log(f)
                ));
                exit(1);
            }

            let program = gl.create_program().unwrap();
            glow_debug_print("gl::create_program successs");
            gl.attach_shader(program, v);
            gl.attach_shader(program, f);
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                glow_debug_print(format!(
                    "Failed to link shader: {}",
                    gl.get_program_info_log(program)
                ));
                exit(1);
            }
            gl.detach_shader(program, v);
            gl.detach_shader(program, f);
            gl.delete_shader(v);
            gl.delete_shader(f);

            let u_screen_size = gl.get_uniform_location(program, "u_screen_size").unwrap();
            let u_sampler = gl.get_uniform_location(program, "u_sampler").unwrap();
            glow_debug_print("gl::get_uniform_location success");
            let vertex_array = gl.create_vertex_array().unwrap();
            let vertex_buffer = gl.create_buffer().unwrap();
            let element_array_buffer = gl.create_buffer().unwrap();

            gl.bind_vertex_array(Some(vertex_array));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));
            // webgl and webgl2 should work
            let a_pos_loc = gl.get_attrib_location(program, "a_pos").unwrap();
            let a_tc_loc = gl.get_attrib_location(program, "a_tc").unwrap();
            let a_srgba_loc = gl.get_attrib_location(program, "a_srgba").unwrap();
            glow_debug_print(format!("gl::get_attrib_location success a_pos {} a_tc {} a_srgba {}",a_pos_loc,a_tc_loc,a_srgba_loc));
            // webgl and webgl2 should work
            gl.vertex_attrib_pointer_f32(
                a_pos_loc,
                2,
                glow::FLOAT,
                false,
                std::mem::size_of::<Vertex>() as i32,
                offset_of!(Vertex, pos) as i32,
            );
            gl.enable_vertex_attrib_array(a_pos_loc);
            gl.vertex_attrib_pointer_f32(
                a_tc_loc,
                2,
                glow::FLOAT,
                false,
                std::mem::size_of::<Vertex>() as i32,
                offset_of!(Vertex, uv) as i32,
            );
            gl.enable_vertex_attrib_array(a_tc_loc);

            gl.vertex_attrib_pointer_f32(
                a_srgba_loc,
                4,
                glow::UNSIGNED_BYTE,
                false,
                std::mem::size_of::<Vertex>() as i32,
                offset_of!(Vertex, color) as i32,
            );
            gl.enable_vertex_attrib_array(a_srgba_loc);
            assert_eq!(gl.get_error(), glow::NO_ERROR, "OpenGL error occurred!");

            Painter {
                program,
                u_screen_size,
                u_sampler,
                egui_texture: None,
                egui_texture_version: None,
                webgl_1_compatibility_mode,
                vertex_array: vertex_array,
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
                self.webgl_1_compatibility_mode,
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
        gl.bind_vertex_array(Some(self.vertex_array));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vertex_buffer));
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
        self.assert_not_destroyed();

        self.upload_egui_texture(gl, egui_texture);
        self.upload_pending_user_textures(gl);
        if let Some(ref mut post_process) = self.post_process {
            post_process.begin(gl,inner_size[0] as i32, inner_size[1] as i32);
        }
        let size_in_pixels = unsafe { self.prepare_painting(inner_size, gl, pixels_per_point) };
        for egui::ClippedMesh(clip_rect, mesh) in clipped_meshes {
            self.paint_mesh(gl, size_in_pixels, pixels_per_point, clip_rect, &mesh)
        }
        if let Some(ref post_process) = self.post_process {
            post_process.end(gl);
        }

        if glow::NO_ERROR != unsafe { gl.get_error() } {
            glow_debug_print("GL error occurred!");
            exit(1);
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
        // debug_assert!(mesh.is_valid());
        #[cfg(debug_assertions)]
        if !mesh.is_valid() {
            glow_debug_print("invalid mesh ");
            exit(1);
        }
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
        if size.0 * size.1 != pixels.len() {
            glow_debug_print("Mismatch between size and texel count");
            exit(1);
        }

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
                    self.webgl_1_compatibility_mode,
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
        if self.destroyed {
            glow_debug_print("Only destroy egui once!");
            exit(1);
        }
        unsafe {
            self.destroy_gl(gl);
        }
        self.destroyed = true;
    }

    #[cfg(not(debug_assertions))]
    pub fn destroy(&self, gl: &glow::Context) {
        unsafe {
            self.destroy_gl(gl);
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
#[cfg(target_arch = "wasm32")]
pub fn canvas_to_dimension(canvas: HtmlCanvasElement) -> [u32; 2] {
    [canvas.width() as u32, canvas.height() as u32]
}
#[cfg(target_arch = "wasm32")]
pub fn clear(canvas: HtmlCanvasElement, gl: &glow::Context, clear_color: egui::Rgba) {
    unsafe {
        gl.disable(glow::SCISSOR_TEST);

        let width = canvas.width() as i32;
        let height = canvas.height() as i32;
        gl.viewport(0, 0, width, height);

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
#[cfg(target_arch = "wasm32")]
fn glow_debug_print(s: impl Into<JsValue>) {
    web_sys::console::log_1(&s.into());
}
#[cfg(not(target_arch = "wasm32"))]
fn glow_debug_print(s: impl std::fmt::Display) {
    println!("{}", s)
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

/// Uses a framebuffer to render everything in linear color space and convert it back to sRGB
/// in a separate "post processing" step
struct PostProcess {
    pos_buffer: glow::Buffer,
    index_buffer: glow::Buffer,
    vertex_array: glow::VertexArray,
    one_compatibility:bool,
    texture: glow::Texture,
    texture_size: (i32, i32),
    fbo: glow::Framebuffer,
    program: glow::Program,
}

impl PostProcess {
    fn new(gl: &glow::Context,is_webgl_1:bool, width: i32, height: i32) -> Result<PostProcess, String> {
        let fbo = unsafe { gl.create_framebuffer() }?;
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
        }

        let texture = unsafe { gl.create_texture() }.unwrap();
        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        }
        unsafe {
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as i32,
            );
        }
        unsafe {
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );
        }
        unsafe {
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as i32,
            );
        }
        unsafe {
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
            );
        }
        unsafe {
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        }
        let (internal_format,format)=if is_webgl_1{
            (glow::SRGB_ALPHA,glow::SRGB_ALPHA)
        }else{
            (glow::SRGB8_ALPHA8,glow::RGBA)
        };

        unsafe {
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                internal_format as i32,
                width,
                height,
                0,
                format,
                glow::UNSIGNED_BYTE,
                None,
            );
            let error_code=gl.get_error();
            assert_eq!(error_code ,glow::NO_ERROR,"Error occurred in post process texture initialization. code : 0x{:x}",error_code);
        }
        unsafe {
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(texture),
                0,
            );
        }

        unsafe { gl.bind_texture(glow::TEXTURE_2D, None) }
        unsafe { gl.bind_framebuffer(glow::FRAMEBUFFER, None) }

        let vert_shader = compile_shader(
            &gl,
            glow::VERTEX_SHADER,
            include_str!("shader/post_vertex_100es.glsl"),
        )?;
        let frag_shader = compile_shader(
            &gl,
            glow::FRAGMENT_SHADER,
            include_str!("shader/post_fragment_100es.glsl"),
        )?;
        let program = unsafe { link_program(&gl, [vert_shader, frag_shader].iter()) }?;
        let vertex_array = unsafe { gl.create_vertex_array() }?;
        unsafe { gl.bind_vertex_array(Some(vertex_array)) }

        let positions = vec![0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0];

        let indices = vec![0u8, 1, 2, 1, 2, 3];
        unsafe {
            let pos_buffer = gl.create_buffer()?;
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(pos_buffer));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, std::slice::from_raw_parts(positions.as_ptr() as *const u8,positions.len()*std::mem::size_of::<f32>()), glow::STATIC_DRAW);

            let a_pos_loc = gl
                .get_attrib_location(program, "a_pos")
                .ok_or_else(|| "failed to get location of a_pos".to_string())?;
            assert!(a_pos_loc >= 0);
            gl.vertex_attrib_pointer_f32(a_pos_loc, 2, glow::FLOAT, false,0, 0);
            gl.enable_vertex_attrib_array(a_pos_loc);

            gl.bind_buffer(glow::ARRAY_BUFFER, None);

            let index_buffer = gl.create_buffer()?;
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buffer));
            gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, &indices, glow::STATIC_DRAW);

            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);
            let error_code=gl.get_error();
            assert_eq!(error_code ,glow::NO_ERROR,"Error occurred in post process initialization. code : 0x{:x}",error_code);

            Ok(PostProcess {
                pos_buffer,
                index_buffer,
                vertex_array,
                one_compatibility:is_webgl_1,
                texture,
                texture_size: (width, height),
                fbo,
                program,
            })
        }
    }

    fn begin(&mut self, gl: &glow::Context, width: i32, height: i32)  {
        if (width, height) != self.texture_size {
            unsafe {
                gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            }
            unsafe {
                gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            }
            let (internal_format,format)=if self.one_compatibility{
                (glow::SRGB_ALPHA,glow::SRGB_ALPHA)
            }else{
                (glow::SRGB8_ALPHA8,glow::RGBA)
            };
            unsafe {
                gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    internal_format as i32,
                    width,
                    height,
                    0,
                    format,
                    glow::UNSIGNED_BYTE,
                    None,
                );
            }
            unsafe {
                gl.bind_texture(glow::TEXTURE_2D, None);
            }

            self.texture_size = (width, height);
        }
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
        }

    }

    fn end(&self, gl: &glow::Context) {
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.disable(glow::SCISSOR_TEST);

            gl.use_program(Some(self.program));

            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            let u_sampler_loc = gl.get_uniform_location(self.program, "u_sampler").unwrap();
            gl.uniform_1_i32(Some(&u_sampler_loc), 0);

            gl.bind_vertex_array(Some(self.vertex_array));

            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER,Some(self.index_buffer));
            gl.draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_BYTE, 0);

            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.bind_vertex_array(None);
            gl.use_program(None);
        }
    }
    fn destroy(&mut self, gl: &glow::Context) {
        unsafe {
            gl.delete_vertex_array(self.vertex_array);
            gl.delete_buffer(self.pos_buffer);
            gl.delete_buffer(self.index_buffer);
            gl.delete_program(self.program);
            gl.delete_framebuffer(self.fbo);
            gl.delete_texture(self.texture);
        }
    }
}

fn compile_shader(
    gl: &glow::Context,
    shader_type: u32,
    source: &str,
) -> Result<glow::Shader, String> {
    let shader = unsafe { gl.create_shader(shader_type) }
        .map_err(|_| String::from("Unable to create shader object"))?;
    unsafe {
        gl.shader_source(shader, source);
    }
    unsafe {
        gl.compile_shader(shader);
    }

    if unsafe { gl.get_shader_compile_status(shader) } {
        Ok(shader)
    } else {
        Err(unsafe { gl.get_shader_info_log(shader) })
    }
}

unsafe fn link_program<'a, T: IntoIterator<Item = &'a glow::Shader>>(
    gl: &glow::Context,
    shaders: T,
) -> Result<glow::Program, String> {
    let program = unsafe { gl.create_program() }
        .map_err(|_| String::from("Unable to create shader object"))?;
    unsafe {
        for shader in shaders {
            gl.attach_shader(program, *shader)
        }
    }
    unsafe {
        gl.link_program(program);
    }

    if unsafe { gl.get_program_link_status(program) } {
        Ok(program)
    } else {
        Err(gl.get_program_info_log(program))
    }
}
