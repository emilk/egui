#![allow(unsafe_code)]
use glow::HasContext;
use std::option::Option::Some;

pub fn check_for_gl_error(gl: &glow::Context, context: &str) {
    let error_code = unsafe { gl.get_error() };
    if error_code != glow::NO_ERROR {
        tracing::error!(
            "GL error, at: '{}', code: {} (0x{:X})",
            context,
            error_code,
            error_code
        );
    }
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
    const WEBGL_PREFIX: &str = "WebGL ";
    const OPENGL_ES_PREFIX: &str = "OpenGL ES ";

    let version_string = unsafe { gl.get_parameter_string(glow::VERSION) };
    tracing::debug!("GL version: {:?}.", version_string);

    // Examples:
    // * "WebGL 2.0 (OpenGL ES 3.0 Chromium)"
    // * "WebGL 2.0"

    if let Some(pos) = version_string.rfind(WEBGL_PREFIX) {
        let version_str = &version_string[pos + WEBGL_PREFIX.len()..];
        if version_str.contains("1.0") {
            // need to test OES_vertex_array_object .
            gl.supported_extensions()
                .contains("OES_vertex_array_object")
        } else {
            true
        }
    } else if version_string.contains(OPENGL_ES_PREFIX) {
        // glow targets es2.0+ so we don't concern about OpenGL ES-CM,OpenGL ES-CL
        if version_string.contains("2.0") {
            // need to test OES_vertex_array_object .
            gl.supported_extensions()
                .contains("OES_vertex_array_object")
        } else {
            true
        }
    } else {
        // from OpenGL 3 vao into core
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
