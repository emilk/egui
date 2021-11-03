#![allow(unsafe_code)]
use glow::HasContext;

pub(crate) struct BufferInfo {
    pub location: u32, //
    pub vector_size: i32,
    pub data_type: u32, //GL_FLOAT,GL_UNSIGNED_BYTE
    pub normalized: bool,
    pub stride: i32,
    pub offset: i32,
}
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
        }
        for attribute in self.buffer_infos.iter() {
            unsafe {
                gl.vertex_attrib_pointer_f32(
                    attribute.location,
                    attribute.vector_size,
                    attribute.data_type,
                    attribute.normalized,
                    attribute.stride,
                    attribute.offset,
                );
                gl.enable_vertex_attrib_array(attribute.location);
            }
        }
    }
    pub(crate) fn unbind_vertex_array(&self, gl: &glow::Context) {
        for attribute in self.buffer_infos.iter() {
            unsafe {
                gl.disable_vertex_attrib_array(attribute.location);
            }
        }
        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
        }
    }
}
