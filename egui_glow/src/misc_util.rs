#![allow(unsafe_code)]
use glow::HasContext;
use std::option::Option::Some;

use crate::painter::TextureFilter;

pub(crate) fn srgb_texture2d(
    gl: &glow::Context,
    is_webgl_1: bool,
    srgb_support: bool,
    texture_filter: TextureFilter,
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
            texture_filter.glow_code() as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            texture_filter.glow_code() as i32,
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
        if is_webgl_1 {
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
        check_for_gl_error(gl, "srgb_texture2d");
        tex
    }
}

pub fn check_for_gl_error(gl: &glow::Context, context: &str) {
    let error_code = unsafe { gl.get_error() };
    if error_code != glow::NO_ERROR {
        glow_print_error(format!(
            "GL error, at: '{}', code: {} (0x{:X})",
            context, error_code, error_code
        ));
    }
}

pub(crate) unsafe fn as_u8_slice<T>(s: &[T]) -> &[u8] {
    std::slice::from_raw_parts(s.as_ptr().cast::<u8>(), s.len() * std::mem::size_of::<T>())
}

pub(crate) fn glow_print(s: impl std::fmt::Display) {
    #[cfg(target_arch = "wasm32")]
    web_sys::console::log_1(&format!("egui_glow: {}", s).into());

    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("egui_glow: {}", s);
}

pub(crate) fn glow_print_error(s: impl std::fmt::Display) {
    #[cfg(target_arch = "wasm32")]
    web_sys::console::error_1(&format!("egui_glow: {}", s).into());

    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("egui_glow ERROR: {}", s);
}

pub(crate) unsafe fn compile_shader(
    gl: &glow::Context,
    shader_type: u32,
    source: &str,
) -> Result<glow::Shader, String> {
    let shader = gl.create_shader(shader_type)?;

    gl.shader_source(shader, source);

    gl.compile_shader(shader);

    if gl.get_shader_compile_status(shader) {
        Ok(shader)
    } else {
        Err(gl.get_shader_info_log(shader))
    }
}

pub(crate) unsafe fn link_program<'a, T: IntoIterator<Item = &'a glow::Shader>>(
    gl: &glow::Context,
    shaders: T,
) -> Result<glow::Program, String> {
    let program = gl.create_program()?;

    for shader in shaders {
        gl.attach_shader(program, *shader);
    }

    gl.link_program(program);

    if gl.get_program_link_status(program) {
        Ok(program)
    } else {
        Err(gl.get_program_info_log(program))
    }
}
///Wrapper around Emulated VAO and GL's VAO
pub(crate) enum VAO {
    Emulated(crate::vao_emulate::EmulatedVao),
    Native(crate::glow::VertexArray),
}

impl VAO {
    pub(crate) unsafe fn native(gl: &glow::Context) -> Self {
        Self::Native(gl.create_vertex_array().unwrap())
    }

    pub(crate) unsafe fn emulated() -> Self {
        Self::Emulated(crate::vao_emulate::EmulatedVao::new())
    }

    pub(crate) unsafe fn bind_vertex_array(&self, gl: &glow::Context) {
        match self {
            VAO::Emulated(vao) => vao.bind_vertex_array(gl),
            VAO::Native(vao) => gl.bind_vertex_array(Some(*vao)),
        }
    }

    pub(crate) unsafe fn bind_buffer(&mut self, gl: &glow::Context, buffer: &glow::Buffer) {
        match self {
            VAO::Emulated(vao) => vao.bind_buffer(buffer),
            VAO::Native(_) => gl.bind_buffer(glow::ARRAY_BUFFER, Some(*buffer)),
        }
    }

    pub(crate) unsafe fn add_new_attribute(
        &mut self,
        gl: &glow::Context,
        buffer_info: crate::vao_emulate::BufferInfo,
    ) {
        match self {
            VAO::Emulated(vao) => vao.add_new_attribute(buffer_info),
            VAO::Native(_) => {
                gl.vertex_attrib_pointer_f32(
                    buffer_info.location,
                    buffer_info.vector_size,
                    buffer_info.data_type,
                    buffer_info.normalized,
                    buffer_info.stride,
                    buffer_info.offset,
                );
                gl.enable_vertex_attrib_array(buffer_info.location);
            }
        }
    }

    pub(crate) unsafe fn unbind_vertex_array(&self, gl: &glow::Context) {
        match self {
            VAO::Emulated(vao) => vao.unbind_vertex_array(gl),
            VAO::Native(_) => {
                gl.bind_vertex_array(None);
            }
        }
    }
}

/// If returned true no need to emulate vao
pub(crate) fn supports_vao(gl: &glow::Context) -> bool {
    let web_sig = "WebGL ";
    let es_sig = "OpenGL ES ";
    let version_string = unsafe { gl.get_parameter_string(glow::VERSION) };
    if let Some(pos) = version_string.rfind(web_sig) {
        let version_str = &version_string[pos + web_sig.len()..];
        glow_print(format!(
            "detected WebGL prefix at {}:{}",
            pos + web_sig.len(),
            version_str
        ));
        if version_str.contains("1.0") {
            //need to test OES_vertex_array_object .
            gl.supported_extensions()
                .contains("OES_vertex_array_object")
        } else {
            true
        }
    } else if let Some(pos) = version_string.rfind(es_sig) {
        //glow targets es2.0+ so we don't concern about OpenGL ES-CM,OpenGL ES-CL
        glow_print(format!(
            "detected OpenGL ES prefix at {}:{}",
            pos + es_sig.len(),
            &version_string[pos + es_sig.len()..]
        ));
        if version_string.contains("2.0") {
            //need to test OES_vertex_array_object .
            gl.supported_extensions()
                .contains("OES_vertex_array_object")
        } else {
            true
        }
    } else {
        glow_print(format!("detected OpenGL: {:?}", version_string));
        //from OpenGL 3 vao into core
        if version_string.starts_with('2') {
            // I found APPLE_vertex_array_object , GL_ATI_vertex_array_object ,ARB_vertex_array_object
            // but APPLE's and ATI's very old extension.
            gl.supported_extensions()
                .contains("ARB_vertex_array_object")
        } else {
            true
        }
    }
}
