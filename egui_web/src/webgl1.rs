use std::collections::HashMap;

use {
    js_sys::WebAssembly,
    wasm_bindgen::{prelude::*, JsCast},
    web_sys::{
        ExtSRgb, WebGlBuffer, WebGlFramebuffer, WebGlProgram, WebGlRenderingContext, WebGlShader,
        WebGlTexture,
    },
};

use egui::{
    emath::vec2,
    epaint::{Color32, FontImage},
};

type Gl = WebGlRenderingContext;

pub struct WebGlPainter {
    canvas_id: String,
    canvas: web_sys::HtmlCanvasElement,
    gl: WebGlRenderingContext,
    program: WebGlProgram,
    index_buffer: WebGlBuffer,
    pos_buffer: WebGlBuffer,
    tc_buffer: WebGlBuffer,
    color_buffer: WebGlBuffer,
    texture_format: u32,
    post_process: Option<PostProcess>,

    textures: HashMap<egui::TextureId, WebGlTexture>,
    next_native_tex_id: u64,
}

impl WebGlPainter {
    pub fn new(canvas_id: &str) -> Result<WebGlPainter, JsValue> {
        let canvas = crate::canvas_element_or_die(canvas_id);

        let gl = canvas
            .get_context("webgl")?
            .ok_or_else(|| JsValue::from("Failed to get WebGl context"))?
            .dyn_into::<WebGlRenderingContext>()?;

        // --------------------------------------------------------------------

        let egui_texture = gl.create_texture().unwrap();
        gl.bind_texture(Gl::TEXTURE_2D, Some(&egui_texture));
        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_S, Gl::CLAMP_TO_EDGE as i32);
        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_T, Gl::CLAMP_TO_EDGE as i32);
        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MIN_FILTER, Gl::LINEAR as i32);
        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MAG_FILTER, Gl::LINEAR as i32);

        let srgb_supported = matches!(gl.get_extension("EXT_sRGB"), Ok(Some(_)));

        let vert_shader = compile_shader(
            &gl,
            Gl::VERTEX_SHADER,
            include_str!("shader/main_vertex_100es.glsl"),
        )?;
        let (texture_format, program, post_process) = if srgb_supported {
            let frag_shader = compile_shader(
                &gl,
                Gl::FRAGMENT_SHADER,
                include_str!("shader/main_fragment_100es.glsl"),
            )?;
            let program = link_program(&gl, [vert_shader, frag_shader].iter())?;

            let post_process =
                PostProcess::new(gl.clone(), canvas.width() as i32, canvas.height() as i32)?;

            (ExtSRgb::SRGB_ALPHA_EXT, program, Some(post_process))
        } else {
            let frag_shader = compile_shader(
                &gl,
                Gl::FRAGMENT_SHADER,
                include_str!("shader/fragment_100es.glsl"),
            )?;
            let program = link_program(&gl, [vert_shader, frag_shader].iter())?;

            (Gl::RGBA, program, None)
        };

        let index_buffer = gl.create_buffer().ok_or("failed to create index_buffer")?;
        let pos_buffer = gl.create_buffer().ok_or("failed to create pos_buffer")?;
        let tc_buffer = gl.create_buffer().ok_or("failed to create tc_buffer")?;
        let color_buffer = gl.create_buffer().ok_or("failed to create color_buffer")?;

        Ok(WebGlPainter {
            canvas_id: canvas_id.to_owned(),
            canvas,
            gl,
            program,
            index_buffer,
            pos_buffer,
            tc_buffer,
            color_buffer,
            texture_format,
            post_process,
            egui_texture,
            egui_texture_version: None,
            textures: Default::default(),
            next_native_tex_id: 1 << 32,
        })
    }

    fn get_texture(&self, texture_id: egui::TextureId) -> Option<&WebGlTexture> {
        self.textures.get(&id)
    }

    fn paint_mesh(&self, mesh: &egui::epaint::Mesh16) -> Result<(), JsValue> {
        debug_assert!(mesh.is_valid());

        let mut positions: Vec<f32> = Vec::with_capacity(2 * mesh.vertices.len());
        let mut tex_coords: Vec<f32> = Vec::with_capacity(2 * mesh.vertices.len());
        let mut colors: Vec<u8> = Vec::with_capacity(4 * mesh.vertices.len());
        for v in &mesh.vertices {
            positions.push(v.pos.x);
            positions.push(v.pos.y);
            tex_coords.push(v.uv.x);
            tex_coords.push(v.uv.y);
            colors.push(v.color[0]);
            colors.push(v.color[1]);
            colors.push(v.color[2]);
            colors.push(v.color[3]);
        }

        // --------------------------------------------------------------------

        let gl = &self.gl;

        let indices_memory_buffer = wasm_bindgen::memory()
            .dyn_into::<WebAssembly::Memory>()?
            .buffer();
        let indices_ptr = mesh.indices.as_ptr() as u32 / 2;
        let indices_array = js_sys::Int16Array::new(&indices_memory_buffer)
            .subarray(indices_ptr, indices_ptr + mesh.indices.len() as u32);

        gl.bind_buffer(Gl::ELEMENT_ARRAY_BUFFER, Some(&self.index_buffer));
        gl.buffer_data_with_array_buffer_view(
            Gl::ELEMENT_ARRAY_BUFFER,
            &indices_array,
            Gl::STREAM_DRAW,
        );

        // --------------------------------------------------------------------

        let pos_memory_buffer = wasm_bindgen::memory()
            .dyn_into::<WebAssembly::Memory>()?
            .buffer();
        let pos_ptr = positions.as_ptr() as u32 / 4;
        let pos_array = js_sys::Float32Array::new(&pos_memory_buffer)
            .subarray(pos_ptr, pos_ptr + positions.len() as u32);

        gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&self.pos_buffer));
        gl.buffer_data_with_array_buffer_view(Gl::ARRAY_BUFFER, &pos_array, Gl::STREAM_DRAW);

        let a_pos_loc = gl.get_attrib_location(&self.program, "a_pos");
        assert!(a_pos_loc >= 0);
        let a_pos_loc = a_pos_loc as u32;

        let normalize = false;
        let stride = 0;
        let offset = 0;
        gl.vertex_attrib_pointer_with_i32(a_pos_loc, 2, Gl::FLOAT, normalize, stride, offset);
        gl.enable_vertex_attrib_array(a_pos_loc);

        // --------------------------------------------------------------------

        let tc_memory_buffer = wasm_bindgen::memory()
            .dyn_into::<WebAssembly::Memory>()?
            .buffer();
        let tc_ptr = tex_coords.as_ptr() as u32 / 4;
        let tc_array = js_sys::Float32Array::new(&tc_memory_buffer)
            .subarray(tc_ptr, tc_ptr + tex_coords.len() as u32);

        gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&self.tc_buffer));
        gl.buffer_data_with_array_buffer_view(Gl::ARRAY_BUFFER, &tc_array, Gl::STREAM_DRAW);

        let a_tc_loc = gl.get_attrib_location(&self.program, "a_tc");
        assert!(a_tc_loc >= 0);
        let a_tc_loc = a_tc_loc as u32;

        let normalize = false;
        let stride = 0;
        let offset = 0;
        gl.vertex_attrib_pointer_with_i32(a_tc_loc, 2, Gl::FLOAT, normalize, stride, offset);
        gl.enable_vertex_attrib_array(a_tc_loc);

        // --------------------------------------------------------------------

        let colors_memory_buffer = wasm_bindgen::memory()
            .dyn_into::<WebAssembly::Memory>()?
            .buffer();
        let colors_ptr = colors.as_ptr() as u32;
        let colors_array = js_sys::Uint8Array::new(&colors_memory_buffer)
            .subarray(colors_ptr, colors_ptr + colors.len() as u32);

        gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&self.color_buffer));
        gl.buffer_data_with_array_buffer_view(Gl::ARRAY_BUFFER, &colors_array, Gl::STREAM_DRAW);

        let a_srgba_loc = gl.get_attrib_location(&self.program, "a_srgba");
        assert!(a_srgba_loc >= 0);
        let a_srgba_loc = a_srgba_loc as u32;

        let normalize = false;
        let stride = 0;
        let offset = 0;
        gl.vertex_attrib_pointer_with_i32(
            a_srgba_loc,
            4,
            Gl::UNSIGNED_BYTE,
            normalize,
            stride,
            offset,
        );
        gl.enable_vertex_attrib_array(a_srgba_loc);

        // --------------------------------------------------------------------

        gl.draw_elements_with_i32(
            Gl::TRIANGLES,
            mesh.indices.len() as i32,
            Gl::UNSIGNED_SHORT,
            0,
        );

        Ok(())
    }
}

impl epi::NativeTexture for WebGlPainter {
    type Texture = WebGlTexture;

    fn register_native_texture(&mut self, native: Self::Texture) -> egui::TextureId {
        let id = egui::TextureId::User(self.next_native_tex_id);
        self.next_native_tex_id += 1;
        self.textures.insert(id, native);
        id
    }

    fn replace_native_texture(&mut self, id: egui::TextureId, replacing: Self::Texture) {
        self.textures.insert(id, native);
    }
}

impl crate::Painter for WebGlPainter {
    fn set_texture(&mut self, tex_id: egui::TextureId, image: egui::ImageData) {
        assert_eq!(
            image.size[0] * image.size[1],
            image.pixels.len(),
            "Mismatch between texture size and texel count"
        );

        // TODO: optimize
        let mut pixels: Vec<u8> = Vec::with_capacity(image.pixels.len() * 4);
        for srgba in image.pixels {
            pixels.push(srgba.r());
            pixels.push(srgba.g());
            pixels.push(srgba.b());
            pixels.push(srgba.a());
        }

        let gl = &self.gl;
        let gl_texture = gl.create_texture().unwrap();
        gl.bind_texture(Gl::TEXTURE_2D, Some(&gl_texture));
        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_S, Gl::CLAMP_TO_EDGE as _);
        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_T, Gl::CLAMP_TO_EDGE as _);
        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MIN_FILTER, Gl::LINEAR as _);
        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MAG_FILTER, Gl::LINEAR as _);

        gl.bind_texture(Gl::TEXTURE_2D, Some(&gl_texture));

        let level = 0;
        let internal_format = self.texture_format;
        let border = 0;
        let src_format = self.texture_format;
        let src_type = Gl::UNSIGNED_BYTE;
        gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            Gl::TEXTURE_2D,
            level,
            internal_format as _,
            image.size[0] as _,
            image.size[1] as _,
            border,
            src_format,
            src_type,
            Some(&pixels),
        )
        .unwrap();

        self.textures.insert(tex_id, gl_texture);
    }

    fn free_texture(&mut self, tex_id: egui::TextureId) {
        self.textures.remove(&tex_id);
    }

    fn debug_info(&self) -> String {
        format!(
            "Stored canvas size: {} x {}\n\
             gl context size: {} x {}",
            self.canvas.width(),
            self.canvas.height(),
            self.gl.drawing_buffer_width(),
            self.gl.drawing_buffer_height(),
        )
    }

    /// id of the canvas html element containing the rendering
    fn canvas_id(&self) -> &str {
        &self.canvas_id
    }

    fn upload_egui_texture(&mut self, font_image: &FontImage) {
        if self.egui_texture_version == Some(font_image.version) {
            return; // No change
        }

        let gamma = if self.post_process.is_none() {
            1.0 / 2.2 // HACK due to non-linear framebuffer blending.
        } else {
            1.0 // post process enables linear blending
        };
        let mut pixels: Vec<u8> = Vec::with_capacity(font_image.pixels.len() * 4);
        for srgba in font_image.srgba_pixels(gamma) {
            pixels.push(srgba.r());
            pixels.push(srgba.g());
            pixels.push(srgba.b());
            pixels.push(srgba.a());
        }

        let gl = &self.gl;
        gl.bind_texture(Gl::TEXTURE_2D, Some(&self.egui_texture));

        let level = 0;
        let internal_format = self.texture_format;
        let border = 0;
        let src_format = self.texture_format;
        let src_type = Gl::UNSIGNED_BYTE;
        gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            Gl::TEXTURE_2D,
            level,
            internal_format as i32,
            font_image.width as i32,
            font_image.height as i32,
            border,
            src_format,
            src_type,
            Some(&pixels),
        )
        .unwrap();

        self.egui_texture_version = Some(font_image.version);
    }

    fn clear(&mut self, clear_color: egui::Rgba) {
        let gl = &self.gl;

        gl.disable(Gl::SCISSOR_TEST);

        let width = self.canvas.width() as i32;
        let height = self.canvas.height() as i32;
        gl.viewport(0, 0, width, height);

        let clear_color: Color32 = clear_color.into();
        gl.clear_color(
            clear_color[0] as f32 / 255.0,
            clear_color[1] as f32 / 255.0,
            clear_color[2] as f32 / 255.0,
            clear_color[3] as f32 / 255.0,
        );
        gl.clear(Gl::COLOR_BUFFER_BIT);
    }

    fn paint_meshes(
        &mut self,
        clipped_meshes: Vec<egui::ClippedMesh>,
        pixels_per_point: f32,
    ) -> Result<(), JsValue> {
        let gl = &self.gl;

        if let Some(ref mut post_process) = self.post_process {
            post_process.begin(self.canvas.width() as i32, self.canvas.height() as i32)?;
        }

        gl.enable(Gl::SCISSOR_TEST);
        gl.disable(Gl::CULL_FACE); // egui is not strict about winding order.
        gl.enable(Gl::BLEND);
        gl.blend_func(Gl::ONE, Gl::ONE_MINUS_SRC_ALPHA); // premultiplied alpha
        gl.use_program(Some(&self.program));
        gl.active_texture(Gl::TEXTURE0);

        let u_screen_size_loc = gl
            .get_uniform_location(&self.program, "u_screen_size")
            .unwrap();
        let screen_size_pixels = vec2(self.canvas.width() as f32, self.canvas.height() as f32);
        let screen_size_points = screen_size_pixels / pixels_per_point;
        gl.uniform2f(
            Some(&u_screen_size_loc),
            screen_size_points.x,
            screen_size_points.y,
        );

        let u_sampler_loc = gl.get_uniform_location(&self.program, "u_sampler").unwrap();
        gl.uniform1i(Some(&u_sampler_loc), 0);

        for egui::ClippedMesh(clip_rect, mesh) in clipped_meshes {
            if let Some(gl_texture) = self.get_texture(mesh.texture_id) {
                gl.bind_texture(Gl::TEXTURE_2D, Some(gl_texture));

                let clip_min_x = pixels_per_point * clip_rect.min.x;
                let clip_min_y = pixels_per_point * clip_rect.min.y;
                let clip_max_x = pixels_per_point * clip_rect.max.x;
                let clip_max_y = pixels_per_point * clip_rect.max.y;
                let clip_min_x = clip_min_x.clamp(0.0, screen_size_pixels.x);
                let clip_min_y = clip_min_y.clamp(0.0, screen_size_pixels.y);
                let clip_max_x = clip_max_x.clamp(clip_min_x, screen_size_pixels.x);
                let clip_max_y = clip_max_y.clamp(clip_min_y, screen_size_pixels.y);
                let clip_min_x = clip_min_x.round() as i32;
                let clip_min_y = clip_min_y.round() as i32;
                let clip_max_x = clip_max_x.round() as i32;
                let clip_max_y = clip_max_y.round() as i32;

                // scissor Y coordinate is from the bottom
                gl.scissor(
                    clip_min_x,
                    self.canvas.height() as i32 - clip_max_y,
                    clip_max_x - clip_min_x,
                    clip_max_y - clip_min_y,
                );

                for mesh in mesh.split_to_u16() {
                    self.paint_mesh(&mesh)?;
                }
            } else {
                crate::console_warn(format!(
                    "WebGL: Failed to find texture {:?}",
                    mesh.texture_id
                ));
            }
        }

        if let Some(ref post_process) = self.post_process {
            post_process.end();
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "egui_web (WebGL1)"
    }
}

struct PostProcess {
    gl: Gl,
    pos_buffer: WebGlBuffer,
    a_pos_loc: u32,
    index_buffer: WebGlBuffer,
    texture: WebGlTexture,
    texture_size: (i32, i32),
    fbo: WebGlFramebuffer,
    program: WebGlProgram,
}

impl PostProcess {
    fn new(gl: Gl, width: i32, height: i32) -> Result<PostProcess, JsValue> {
        let fbo = gl
            .create_framebuffer()
            .ok_or("failed to create framebuffer")?;
        gl.bind_framebuffer(Gl::FRAMEBUFFER, Some(&fbo));

        let texture = gl.create_texture().unwrap();
        gl.bind_texture(Gl::TEXTURE_2D, Some(&texture));
        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_S, Gl::CLAMP_TO_EDGE as i32);
        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_T, Gl::CLAMP_TO_EDGE as i32);
        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MIN_FILTER, Gl::NEAREST as i32);
        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MAG_FILTER, Gl::NEAREST as i32);
        gl.pixel_storei(Gl::UNPACK_ALIGNMENT, 1);
        gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            Gl::TEXTURE_2D,
            0,
            ExtSRgb::SRGB_ALPHA_EXT as i32,
            width,
            height,
            0,
            ExtSRgb::SRGB_ALPHA_EXT,
            Gl::UNSIGNED_BYTE,
            None,
        )
        .unwrap();
        gl.framebuffer_texture_2d(
            Gl::FRAMEBUFFER,
            Gl::COLOR_ATTACHMENT0,
            Gl::TEXTURE_2D,
            Some(&texture),
            0,
        );

        gl.bind_texture(Gl::TEXTURE_2D, None);
        gl.bind_framebuffer(Gl::FRAMEBUFFER, None);

        let shader_prefix = if crate::webgl1_requires_brightening(&gl) {
            crate::console_log("Enabling webkitGTK brightening workaround");
            "#define APPLY_BRIGHTENING_GAMMA"
        } else {
            ""
        };

        let vert_shader = compile_shader(
            &gl,
            Gl::VERTEX_SHADER,
            include_str!("shader/post_vertex_100es.glsl"),
        )?;
        let frag_shader = compile_shader(
            &gl,
            Gl::FRAGMENT_SHADER,
            &format!(
                "{}{}",
                shader_prefix,
                include_str!("shader/post_fragment_100es.glsl")
            ),
        )?;
        let program = link_program(&gl, [vert_shader, frag_shader].iter())?;

        let positions = vec![0u8, 0, 1, 0, 0, 1, 1, 1];

        let indices = vec![0u8, 1, 2, 1, 2, 3];

        let pos_buffer = gl.create_buffer().ok_or("failed to create pos_buffer")?;
        gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&pos_buffer));
        gl.buffer_data_with_u8_array(Gl::ARRAY_BUFFER, &positions, Gl::STATIC_DRAW);
        gl.bind_buffer(Gl::ARRAY_BUFFER, None);

        let a_pos_loc = gl.get_attrib_location(&program, "a_pos");
        assert!(a_pos_loc >= 0);
        let a_pos_loc = a_pos_loc as u32;

        let index_buffer = gl.create_buffer().ok_or("failed to create index_buffer")?;
        gl.bind_buffer(Gl::ELEMENT_ARRAY_BUFFER, Some(&index_buffer));
        gl.buffer_data_with_u8_array(Gl::ELEMENT_ARRAY_BUFFER, &indices, Gl::STATIC_DRAW);
        gl.bind_buffer(Gl::ELEMENT_ARRAY_BUFFER, None);

        Ok(PostProcess {
            gl,
            pos_buffer,
            a_pos_loc,
            index_buffer,
            texture,
            texture_size: (width, height),
            fbo,
            program,
        })
    }

    fn begin(&mut self, width: i32, height: i32) -> Result<(), JsValue> {
        let gl = &self.gl;

        if (width, height) != self.texture_size {
            gl.bind_texture(Gl::TEXTURE_2D, Some(&self.texture));
            gl.pixel_storei(Gl::UNPACK_ALIGNMENT, 1);
            gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
                Gl::TEXTURE_2D,
                0,
                ExtSRgb::SRGB_ALPHA_EXT as i32,
                width,
                height,
                0,
                ExtSRgb::SRGB_ALPHA_EXT,
                Gl::UNSIGNED_BYTE,
                None,
            )?;
            gl.bind_texture(Gl::TEXTURE_2D, None);

            self.texture_size = (width, height);
        }

        gl.bind_framebuffer(Gl::FRAMEBUFFER, Some(&self.fbo));
        gl.clear_color(0.0, 0.0, 0.0, 0.0);
        gl.clear(Gl::COLOR_BUFFER_BIT);

        Ok(())
    }

    fn end(&self) {
        let gl = &self.gl;

        gl.bind_framebuffer(Gl::FRAMEBUFFER, None);
        gl.disable(Gl::SCISSOR_TEST);

        gl.use_program(Some(&self.program));

        gl.active_texture(Gl::TEXTURE0);
        gl.bind_texture(Gl::TEXTURE_2D, Some(&self.texture));
        let u_sampler_loc = gl.get_uniform_location(&self.program, "u_sampler").unwrap();
        gl.uniform1i(Some(&u_sampler_loc), 0);

        gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&self.pos_buffer));
        gl.vertex_attrib_pointer_with_i32(self.a_pos_loc, 2, Gl::UNSIGNED_BYTE, false, 0, 0);
        gl.enable_vertex_attrib_array(self.a_pos_loc);

        gl.bind_buffer(Gl::ELEMENT_ARRAY_BUFFER, Some(&self.index_buffer));

        gl.draw_elements_with_i32(Gl::TRIANGLES, 6, Gl::UNSIGNED_BYTE, 0);

        gl.bind_buffer(Gl::ELEMENT_ARRAY_BUFFER, None);
        gl.bind_buffer(Gl::ARRAY_BUFFER, None);
        gl.bind_texture(Gl::TEXTURE_2D, None);
        gl.use_program(None);
    }
}

impl Drop for PostProcess {
    fn drop(&mut self) {
        let gl = &self.gl;
        gl.delete_buffer(Some(&self.pos_buffer));
        gl.delete_buffer(Some(&self.index_buffer));
        gl.delete_program(Some(&self.program));
        gl.delete_framebuffer(Some(&self.fbo));
        gl.delete_texture(Some(&self.texture));
    }
}

fn compile_shader(
    gl: &WebGlRenderingContext,
    shader_type: u32,
    source: &str,
) -> Result<WebGlShader, String> {
    let shader = gl
        .create_shader(shader_type)
        .ok_or_else(|| String::from("Unable to create shader object"))?;
    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);

    if gl
        .get_shader_parameter(&shader, Gl::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(gl
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| "Unknown error creating shader".into()))
    }
}

fn link_program<'a, T: IntoIterator<Item = &'a WebGlShader>>(
    gl: &WebGlRenderingContext,
    shaders: T,
) -> Result<WebGlProgram, String> {
    let program = gl
        .create_program()
        .ok_or_else(|| String::from("Unable to create shader object"))?;
    for shader in shaders {
        gl.attach_shader(&program, shader)
    }
    gl.link_program(&program);

    if gl
        .get_program_parameter(&program, Gl::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(gl
            .get_program_info_log(&program)
            .unwrap_or_else(|| "Unknown error creating program object".into()))
    }
}
