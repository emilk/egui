#![allow(unsafe_code)]

use glow::HasContext as _;

use crate::check_for_gl_error;

// ----------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct BufferInfo {
    pub location: u32, //
    pub vector_size: i32,
    pub data_type: u32, //GL_FLOAT,GL_UNSIGNED_BYTE
    pub normalized: bool,
    pub stride: i32,
    pub offset: i32,
}

// ----------------------------------------------------------------------------

pub struct EmulatedVao {
    buffer: Option<glow::Buffer>,
    buffer_infos: Vec<BufferInfo>,
}

impl EmulatedVao {
    pub(crate) fn new() -> Self {
        Self {
            buffer: None,
            buffer_infos: vec![],
        }
    }

    pub(crate) fn bind_buffer(&mut self, buffer: &glow::Buffer) {
        let _old = self.buffer.replace(*buffer);
    }

    pub(crate) fn add_new_attribute(&mut self, buffer_info: BufferInfo) {
        self.buffer_infos.push(buffer_info);
    }

    pub(crate) fn bind_vertex_array(&self, gl: &glow::Context) {
        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, self.buffer);
            check_for_gl_error!(gl, "bind_buffer");
        }
        for attribute in &self.buffer_infos {
            dbg!(attribute);
            unsafe {
                gl.vertex_attrib_pointer_f32(
                    attribute.location,
                    attribute.vector_size,
                    attribute.data_type,
                    attribute.normalized,
                    attribute.stride,
                    attribute.offset,
                );
                check_for_gl_error!(gl, "vertex_attrib_pointer_f32");
                gl.enable_vertex_attrib_array(attribute.location);
                check_for_gl_error!(gl, "enable_vertex_attrib_array");
            }
        }
    }

    pub(crate) fn unbind_vertex_array(&self, gl: &glow::Context) {
        for attribute in &self.buffer_infos {
            unsafe {
                gl.disable_vertex_attrib_array(attribute.location);
            }
        }
        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
        }
    }
}

// ----------------------------------------------------------------------------

/// Wrapper around either Emulated VAO and GL's VAO
pub(crate) enum VAO {
    Emulated(crate::vao::EmulatedVao),
    Native(crate::glow::VertexArray),
}

impl VAO {
    pub(crate) unsafe fn native(gl: &glow::Context) -> Self {
        Self::Native(gl.create_vertex_array().unwrap())
    }

    pub(crate) unsafe fn emulated() -> Self {
        Self::Emulated(crate::vao::EmulatedVao::new())
    }

    pub(crate) unsafe fn bind_vertex_array(&self, gl: &glow::Context) {
        match self {
            VAO::Emulated(emulated_vao) => emulated_vao.bind_vertex_array(gl),
            VAO::Native(vao) => {
                gl.bind_vertex_array(Some(*vao));
                check_for_gl_error!(gl, "bind_vertex_array");
            }
        }
    }

    pub(crate) unsafe fn bind_buffer(&mut self, gl: &glow::Context, buffer: &glow::Buffer) {
        match self {
            VAO::Emulated(emulated_vao) => emulated_vao.bind_buffer(buffer),
            VAO::Native(_) => gl.bind_buffer(glow::ARRAY_BUFFER, Some(*buffer)),
        }
    }

    pub(crate) unsafe fn add_new_attribute(
        &mut self,
        gl: &glow::Context,
        buffer_info: crate::vao::BufferInfo,
    ) {
        match self {
            VAO::Emulated(emulated_vao) => emulated_vao.add_new_attribute(buffer_info),
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
            VAO::Emulated(emulated_vao) => emulated_vao.unbind_vertex_array(gl),
            VAO::Native(_) => {
                gl.bind_vertex_array(None);
            }
        }
    }
}

// ----------------------------------------------------------------------------

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
