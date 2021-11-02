#![allow(unsafe_code)]
use glow::HasContext;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

pub(crate) fn srgbtexture2d(
    gl: &glow::Context,
    is_webgl_1: bool,
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
        assert_eq!(gl.get_error(), glow::NO_ERROR, "OpenGL error occurred!");
        tex
    }
}

pub(crate) unsafe fn as_u8_slice<T>(s: &[T]) -> &[u8] {
    std::slice::from_raw_parts(s.as_ptr().cast::<u8>(), s.len() * std::mem::size_of::<T>())
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn glow_debug_print(s: impl Into<JsValue>) {
    web_sys::console::log_1(&s.into());
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn glow_debug_print(s: impl std::fmt::Display) {
    println!("{}", s);
}

pub(crate) fn compile_shader(
    gl: &glow::Context,
    shader_type: u32,
    source: &str,
) -> Result<glow::Shader, String> {
    let shader = unsafe { gl.create_shader(shader_type) }?;
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

pub(crate) fn link_program<'a, T: IntoIterator<Item = &'a glow::Shader>>(
    gl: &glow::Context,
    shaders: T,
) -> Result<glow::Program, String> {
    let program = unsafe { gl.create_program() }?;
    unsafe {
        for shader in shaders {
            gl.attach_shader(program, *shader);
        }
    }
    unsafe {
        gl.link_program(program);
    }

    if unsafe { gl.get_program_link_status(program) } {
        Ok(program)
    } else {
        Err(unsafe { gl.get_program_info_log(program) })
    }
}
///Wrapper around Emulated VAO and GL's VAO
pub(crate) enum VAO {
    Emulated(crate::vao_emulate::EmulatedVao),
    Native(crate::glow::VertexArray),
}

impl VAO {
    pub(crate) unsafe fn new(gl: &glow::Context, is_native_vao: bool) -> Self {
        if is_native_vao {
            Self::Native(gl.create_vertex_array().unwrap())
        } else {
            Self::Emulated(crate::vao_emulate::EmulatedVao::new())
        }
    }
    pub(crate) unsafe fn bind_vertex_array(&self, gl: &glow::Context) {
        match self {
            VAO::Emulated(vao) => vao.bind_vertex_array(gl),
            VAO::Native(vao) => gl.bind_vertex_array(Some(*vao)),
        }
    }
    pub(crate) fn bind_buffer(&mut self, gl: &glow::Context, buffer: glow::Buffer) {
        match self {
            VAO::Emulated(vao) => vao.bind_buffer(&buffer),
            VAO::Native(_) => unsafe {
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(buffer));
            },
        }
    }
    pub(crate) fn add_new_attribute(
        &mut self,
        gl: &glow::Context,
        buffer_info: crate::vao_emulate::BufferInfo,
    ) {
        match self {
            VAO::Emulated(vao) => vao.add_new_attribute(buffer_info),
            VAO::Native(_) => unsafe {
                gl.vertex_attrib_pointer_f32(
                    buffer_info.location,
                    buffer_info.vector_size,
                    buffer_info.data_type,
                    buffer_info.normalized,
                    buffer_info.stride,
                    buffer_info.offset,
                );
                gl.enable_vertex_attrib_array(buffer_info.location);
            },
        }
    }
    pub(crate) fn unbind_vertex_array(&self, gl: &glow::Context) {
        match self {
            VAO::Emulated(vao) => vao.unbind_vertex_array(gl),
            VAO::Native(_) => unsafe {
                gl.bind_vertex_array(None);
            },
        }
    }
}
