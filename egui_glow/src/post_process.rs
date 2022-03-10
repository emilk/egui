#![allow(unsafe_code)]
use crate::misc_util::{check_for_gl_error, compile_shader, link_program};
use crate::vao_emulate::BufferInfo;
use glow::HasContext;

/// Uses a framebuffer to render everything in linear color space and convert it back to `sRGB`
/// in a separate "post processing" step
pub(crate) struct PostProcess {
    pos_buffer: glow::Buffer,
    index_buffer: glow::Buffer,
    vertex_array: crate::misc_util::VAO,
    is_webgl_1: bool,
    texture: glow::Texture,
    texture_size: (i32, i32),
    fbo: glow::Framebuffer,
    program: glow::Program,
}

impl PostProcess {
    pub(crate) unsafe fn new(
        gl: &glow::Context,
        shader_prefix: &str,
        need_to_emulate_vao: bool,
        is_webgl_1: bool,
        width: i32,
        height: i32,
    ) -> Result<PostProcess, String> {
        let fbo = gl.create_framebuffer()?;

        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));

        let texture = gl.create_texture().unwrap();

        gl.bind_texture(glow::TEXTURE_2D, Some(texture));

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

        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::NEAREST as i32,
        );

        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::NEAREST as i32,
        );

        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);

        let (internal_format, format) = if is_webgl_1 {
            (glow::SRGB_ALPHA, glow::SRGB_ALPHA)
        } else {
            (glow::SRGB8_ALPHA8, glow::RGBA)
        };

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
        check_for_gl_error(gl, "post process texture initialization");

        gl.framebuffer_texture_2d(
            glow::FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            glow::TEXTURE_2D,
            Some(texture),
            0,
        );
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);

        let vert_shader = compile_shader(
            gl,
            glow::VERTEX_SHADER,
            &format!(
                "{}\n{}",
                shader_prefix,
                include_str!("shader/post_vertex_100es.glsl")
            ),
        )?;
        let frag_shader = compile_shader(
            gl,
            glow::FRAGMENT_SHADER,
            &format!(
                "{}\n{}",
                shader_prefix,
                include_str!("shader/post_fragment_100es.glsl")
            ),
        )?;
        let program = link_program(gl, [vert_shader, frag_shader].iter())?;

        let positions = vec![0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0];

        let indices = vec![0u8, 1, 2, 1, 2, 3];

        let pos_buffer = gl.create_buffer()?;
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(pos_buffer));
        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck::cast_slice(&positions),
            glow::STATIC_DRAW,
        );

        let a_pos_loc = gl
            .get_attrib_location(program, "a_pos")
            .ok_or_else(|| "failed to get location of a_pos".to_string())?;
        let mut vertex_array = if need_to_emulate_vao {
            crate::misc_util::VAO::emulated()
        } else {
            crate::misc_util::VAO::native(gl)
        };
        vertex_array.bind_vertex_array(gl);
        vertex_array.bind_buffer(gl, &pos_buffer);
        let buffer_info_a_pos = BufferInfo {
            location: a_pos_loc,
            vector_size: 2,
            data_type: glow::FLOAT,
            normalized: false,
            stride: 0,
            offset: 0,
        };
        vertex_array.add_new_attribute(gl, buffer_info_a_pos);

        let index_buffer = gl.create_buffer()?;
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buffer));
        gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, &indices, glow::STATIC_DRAW);

        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);
        check_for_gl_error(gl, "post process initialization");

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

    pub(crate) unsafe fn begin(&mut self, gl: &glow::Context, width: i32, height: i32) {
        if (width, height) != self.texture_size {
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);

            let (internal_format, format) = if self.is_webgl_1 {
                (glow::SRGB_ALPHA, glow::SRGB_ALPHA)
            } else {
                (glow::SRGB8_ALPHA8, glow::RGBA)
            };
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

            gl.bind_texture(glow::TEXTURE_2D, None);
            self.texture_size = (width, height);
        }

        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
        gl.clear_color(0.0, 0.0, 0.0, 0.0);
        gl.clear(glow::COLOR_BUFFER_BIT);
    }

    pub(crate) unsafe fn bind(&self, gl: &glow::Context) {
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
    }

    pub(crate) unsafe fn end(&self, gl: &glow::Context) {
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        gl.disable(glow::SCISSOR_TEST);

        gl.use_program(Some(self.program));

        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
        let u_sampler_loc = gl.get_uniform_location(self.program, "u_sampler").unwrap();
        gl.uniform_1_i32(Some(&u_sampler_loc), 0);
        self.vertex_array.bind_vertex_array(gl);

        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.index_buffer));
        gl.draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_BYTE, 0);
        self.vertex_array.unbind_vertex_array(gl);
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.use_program(None);
    }

    pub(crate) unsafe fn destroy(&self, gl: &glow::Context) {
        gl.delete_buffer(self.pos_buffer);
        gl.delete_buffer(self.index_buffer);
        gl.delete_program(self.program);
        gl.delete_framebuffer(self.fbo);
        gl.delete_texture(self.texture);
    }
}
