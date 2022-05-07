#![allow(unsafe_code)]
use crate::check_for_gl_error;
use crate::misc_util::{compile_shader, link_program};
use crate::vao::BufferInfo;
use glow::HasContext as _;

/// Uses a framebuffer to render everything in linear color space and convert it back to `sRGB`
/// in a separate "post processing" step
pub(crate) struct PostProcess {
    gl: std::rc::Rc<glow::Context>,
    pos_buffer: glow::Buffer,
    index_buffer: glow::Buffer,
    vao: crate::vao::VertexArrayObject,
    is_webgl_1: bool,
    color_texture: glow::Texture,
    depth_renderbuffer: Option<glow::Renderbuffer>,
    texture_size: (i32, i32),
    fbo: glow::Framebuffer,
    program: glow::Program,
}

impl PostProcess {
    pub(crate) unsafe fn new(
        gl: std::rc::Rc<glow::Context>,
        shader_prefix: &str,
        is_webgl_1: bool,
        [width, height]: [i32; 2],
    ) -> Result<PostProcess, String> {
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);

        let fbo = gl.create_framebuffer()?;

        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));

        // ----------------------------------------------
        // Set up color tesxture:

        let color_texture = gl.create_texture()?;
        gl.bind_texture(glow::TEXTURE_2D, Some(color_texture));
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
        crate::check_for_gl_error_even_in_release!(&gl, "post process texture initialization");

        gl.framebuffer_texture_2d(
            glow::FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            glow::TEXTURE_2D,
            Some(color_texture),
            0,
        );
        gl.bind_texture(glow::TEXTURE_2D, None);

        // ---------------------------------------------------------
        // Depth buffer - we only need this when embedding 3D within egui using `egui::PaintCallback`.
        // TODO: add a setting to enable/disable the depth buffer.

        let with_depth_buffer = true;
        let depth_renderbuffer = if with_depth_buffer {
            let depth_renderbuffer = gl.create_renderbuffer()?;
            gl.bind_renderbuffer(glow::RENDERBUFFER, Some(depth_renderbuffer));
            gl.renderbuffer_storage(glow::RENDERBUFFER, glow::DEPTH_COMPONENT16, width, height);
            gl.bind_renderbuffer(glow::RENDERBUFFER, None);
            Some(depth_renderbuffer)
        } else {
            None
        };

        // ---------------------------------------------------------

        gl.bind_framebuffer(glow::FRAMEBUFFER, None);

        // ---------------------------------------------------------

        let vert_shader = compile_shader(
            &gl,
            glow::VERTEX_SHADER,
            &format!(
                "{}\n{}",
                shader_prefix,
                include_str!("shader/post_vertex_100es.glsl")
            ),
        )?;
        let frag_shader = compile_shader(
            &gl,
            glow::FRAGMENT_SHADER,
            &format!(
                "{}\n{}",
                shader_prefix,
                include_str!("shader/post_fragment_100es.glsl")
            ),
        )?;
        let program = link_program(&gl, [vert_shader, frag_shader].iter())?;

        let positions: Vec<f32> = vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0];

        let indices: Vec<u8> = vec![0, 1, 2, 1, 2, 3];

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
        let vao = crate::vao::VertexArrayObject::new(
            &gl,
            pos_buffer,
            vec![BufferInfo {
                location: a_pos_loc,
                vector_size: 2,
                data_type: glow::FLOAT,
                normalized: false,
                stride: 0,
                offset: 0,
            }],
        );

        let index_buffer = gl.create_buffer()?;
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buffer));
        gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, &indices, glow::STATIC_DRAW);

        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);
        crate::check_for_gl_error_even_in_release!(&gl, "post process initialization");

        Ok(PostProcess {
            gl,
            pos_buffer,
            index_buffer,
            vao,
            is_webgl_1,
            color_texture,
            depth_renderbuffer,
            texture_size: (width, height),
            fbo,
            program,
        })
    }

    pub(crate) unsafe fn begin(&mut self, width: i32, height: i32) {
        if (width, height) != self.texture_size {
            self.gl
                .bind_texture(glow::TEXTURE_2D, Some(self.color_texture));
            self.gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            let (internal_format, format) = if self.is_webgl_1 {
                (glow::SRGB_ALPHA, glow::SRGB_ALPHA)
            } else {
                (glow::SRGB8_ALPHA8, glow::RGBA)
            };
            self.gl.tex_image_2d(
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
            self.gl.bind_texture(glow::TEXTURE_2D, None);

            if let Some(depth_renderbuffer) = self.depth_renderbuffer {
                self.gl
                    .bind_renderbuffer(glow::RENDERBUFFER, Some(depth_renderbuffer));
                self.gl.renderbuffer_storage(
                    glow::RENDERBUFFER,
                    glow::DEPTH_COMPONENT16,
                    width,
                    height,
                );
                self.gl.bind_renderbuffer(glow::RENDERBUFFER, None);
            }

            self.texture_size = (width, height);
        }

        check_for_gl_error!(&self.gl, "PostProcess::begin");
    }

    pub(crate) unsafe fn bind(&self) {
        self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));

        self.gl.framebuffer_texture_2d(
            glow::FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            glow::TEXTURE_2D,
            Some(self.color_texture),
            0,
        );

        self.gl.framebuffer_renderbuffer(
            glow::FRAMEBUFFER,
            glow::DEPTH_ATTACHMENT,
            glow::RENDERBUFFER,
            self.depth_renderbuffer,
        );

        check_for_gl_error!(&self.gl, "PostProcess::bind");
    }

    pub(crate) unsafe fn end(&self) {
        self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        self.gl.disable(glow::SCISSOR_TEST);

        self.gl.use_program(Some(self.program));

        self.gl.active_texture(glow::TEXTURE0);
        self.gl
            .bind_texture(glow::TEXTURE_2D, Some(self.color_texture));
        let u_sampler_loc = self
            .gl
            .get_uniform_location(self.program, "u_sampler")
            .unwrap();
        self.gl.uniform_1_i32(Some(&u_sampler_loc), 0);
        self.vao.bind(&self.gl);

        self.gl
            .bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.index_buffer));
        self.gl
            .draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_BYTE, 0);
        self.vao.unbind(&self.gl);
        self.gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);
        self.gl.bind_texture(glow::TEXTURE_2D, None);
        self.gl.use_program(None);

        check_for_gl_error!(&self.gl, "PostProcess::end");
    }

    pub(crate) unsafe fn destroy(&self) {
        self.gl.delete_buffer(self.pos_buffer);
        self.gl.delete_buffer(self.index_buffer);
        self.gl.delete_program(self.program);
        self.gl.delete_framebuffer(self.fbo);
        self.gl.delete_texture(self.color_texture);
        if let Some(depth_renderbuffer) = self.depth_renderbuffer {
            self.gl.delete_renderbuffer(depth_renderbuffer);
        }
    }
}
