use crate::{compile_shader, link_program};
use glow::HasContext;

/// Uses a framebuffer to render everything in linear color space and convert it back to sRGB
/// in a separate "post processing" step
pub(crate) struct PostProcess {
    pos_buffer: glow::Buffer,
    index_buffer: glow::Buffer,
    vertex_array: glow::VertexArray,
    is_webgl_1: bool,
    texture: glow::Texture,
    texture_size: (i32, i32),
    fbo: glow::Framebuffer,
    program: glow::Program,
}

impl PostProcess {
    pub(crate) fn new(
        gl: &glow::Context,
        is_webgl_1: bool,
        width: i32,
        height: i32,
    ) -> Result<PostProcess, String> {
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
        let (internal_format, format) = if is_webgl_1 {
            (glow::SRGB_ALPHA, glow::SRGB_ALPHA)
        } else {
            (glow::SRGB8_ALPHA8, glow::RGBA)
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
            let error_code = gl.get_error();
            assert_eq!(
                error_code,
                glow::NO_ERROR,
                "Error occurred in post process texture initialization. code : 0x{:x}",
                error_code
            );
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
            gl,
            glow::VERTEX_SHADER,
            include_str!("shader/post_vertex_100es.glsl"),
        )?;
        let frag_shader = compile_shader(
            gl,
            glow::FRAGMENT_SHADER,
            include_str!("shader/post_fragment_100es.glsl"),
        )?;
        let program = link_program(gl, [vert_shader, frag_shader].iter())?;
        let vertex_array = unsafe { gl.create_vertex_array() }?;
        unsafe { gl.bind_vertex_array(Some(vertex_array)) }

        let positions = vec![0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0];

        let indices = vec![0u8, 1, 2, 1, 2, 3];
        unsafe {
            let pos_buffer = gl.create_buffer()?;
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(pos_buffer));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                std::slice::from_raw_parts(
                    positions.as_ptr() as *const u8,
                    positions.len() * std::mem::size_of::<f32>(),
                ),
                glow::STATIC_DRAW,
            );

            let a_pos_loc = gl
                .get_attrib_location(program, "a_pos")
                .ok_or_else(|| "failed to get location of a_pos".to_string())?;

            gl.vertex_attrib_pointer_f32(a_pos_loc, 2, glow::FLOAT, false, 0, 0);
            gl.enable_vertex_attrib_array(a_pos_loc);

            gl.bind_buffer(glow::ARRAY_BUFFER, None);

            let index_buffer = gl.create_buffer()?;
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buffer));
            gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, &indices, glow::STATIC_DRAW);

            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);
            let error_code = gl.get_error();
            assert_eq!(
                error_code,
                glow::NO_ERROR,
                "Error occurred in post process initialization. code : 0x{:x}",
                error_code
            );

            Ok(PostProcess {
                pos_buffer,
                index_buffer,
                vertex_array,
                is_webgl_1,
                texture,
                texture_size: (width, height),
                fbo,
                program,
            })
        }
    }

    pub(crate) fn begin(&mut self, gl: &glow::Context, width: i32, height: i32) {
        if (width, height) != self.texture_size {
            unsafe {
                gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            }
            unsafe {
                gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            }
            let (internal_format, format) = if self.is_webgl_1 {
                (glow::SRGB_ALPHA, glow::SRGB_ALPHA)
            } else {
                (glow::SRGB8_ALPHA8, glow::RGBA)
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

    pub(crate) fn end(&self, gl: &glow::Context) {
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.disable(glow::SCISSOR_TEST);

            gl.use_program(Some(self.program));

            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            let u_sampler_loc = gl.get_uniform_location(self.program, "u_sampler").unwrap();
            gl.uniform_1_i32(Some(&u_sampler_loc), 0);

            gl.bind_vertex_array(Some(self.vertex_array));

            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.index_buffer));
            gl.draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_BYTE, 0);

            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.bind_vertex_array(None);
            gl.use_program(None);
        }
    }
    pub(crate) fn destroy(&self, gl: &glow::Context) {
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
