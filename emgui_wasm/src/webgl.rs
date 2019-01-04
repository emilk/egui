use {
    js_sys::WebAssembly,
    wasm_bindgen::{prelude::*, JsCast},
    web_sys::{WebGlBuffer, WebGlProgram, WebGlRenderingContext, WebGlShader},
};

use emgui::Frame;

#[wasm_bindgen]
pub struct Painter {
    canvas: web_sys::HtmlCanvasElement,
    gl: WebGlRenderingContext,
    program: WebGlProgram,
    index_buffer: WebGlBuffer,
    pos_buffer: WebGlBuffer,
    color_buffer: WebGlBuffer,
}

impl Painter {
    pub fn new(canvas_id: &str) -> Result<Painter, JsValue> {
        let document = web_sys::window().unwrap().document().unwrap();
        let canvas = document.get_element_by_id(canvas_id).unwrap();
        let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>()?;

        let gl = canvas
            .get_context("webgl")?
            .unwrap()
            .dyn_into::<WebGlRenderingContext>()?;

        gl.enable(WebGlRenderingContext::BLEND);
        gl.blend_func(
            WebGlRenderingContext::SRC_ALPHA,
            WebGlRenderingContext::ONE_MINUS_SRC_ALPHA,
        );

        let vert_shader = compile_shader(
            &gl,
            WebGlRenderingContext::VERTEX_SHADER,
            r#"
            uniform vec2 u_screen_size;
            attribute vec2 a_pos;
            attribute vec4 a_color;
            varying vec4 v_color;
            void main() {
                gl_Position = vec4(
                    2.0 * a_pos.x / u_screen_size.x - 1.0,
                    1.0 - 2.0 * a_pos.y / u_screen_size.y,
                    0.0,
                    1.0);
                // v_color = vec4(1.0, 0.0, 0.0, 0.5);
                v_color = a_color;
            }
        "#,
        )?;
        let frag_shader = compile_shader(
            &gl,
            WebGlRenderingContext::FRAGMENT_SHADER,
            r#"
            precision highp float;
            varying vec4 v_color;
            void main() {
                gl_FragColor = v_color;
            }
        "#,
        )?;
        let program = link_program(&gl, [vert_shader, frag_shader].iter())?;
        let index_buffer = gl.create_buffer().ok_or("failed to create index_buffer")?;
        let pos_buffer = gl.create_buffer().ok_or("failed to create pos_buffer")?;
        let color_buffer = gl.create_buffer().ok_or("failed to create color_buffer")?;

        Ok(Painter {
            canvas,
            gl,
            program,
            index_buffer,
            pos_buffer,
            color_buffer,
        })
    }

    pub fn paint(&self, frame: &Frame) -> Result<(), JsValue> {
        let gl = &self.gl;

        // --------------------------------------------------------------------

        gl.use_program(Some(&self.program));

        // --------------------------------------------------------------------

        let indices: Vec<u16> = frame.indices.iter().map(|idx| *idx as u16).collect();

        let mut positions = Vec::with_capacity(2 * frame.vertices.len());
        for v in &frame.vertices {
            positions.push(v.x);
            positions.push(v.y);
        }

        let mut colors = Vec::with_capacity(4 * frame.vertices.len());
        for v in &frame.vertices {
            colors.push(v.color.r);
            colors.push(v.color.g);
            colors.push(v.color.b);
            colors.push(v.color.a);
        }

        // --------------------------------------------------------------------

        let indices_memory_buffer = wasm_bindgen::memory()
            .dyn_into::<WebAssembly::Memory>()?
            .buffer();
        let indices_ptr = indices.as_ptr() as u32 / 2;
        let indices_array = js_sys::Int16Array::new(&indices_memory_buffer)
            .subarray(indices_ptr, indices_ptr + indices.len() as u32);

        gl.bind_buffer(
            WebGlRenderingContext::ELEMENT_ARRAY_BUFFER,
            Some(&self.index_buffer),
        );
        gl.buffer_data_with_array_buffer_view(
            WebGlRenderingContext::ELEMENT_ARRAY_BUFFER,
            &indices_array,
            WebGlRenderingContext::STATIC_DRAW, // TODO: STREAM ?
        );

        // --------------------------------------------------------------------

        let pos_memory_buffer = wasm_bindgen::memory()
            .dyn_into::<WebAssembly::Memory>()?
            .buffer();
        let pos_ptr = positions.as_ptr() as u32 / 4;
        let pos_array = js_sys::Float32Array::new(&pos_memory_buffer)
            .subarray(pos_ptr, pos_ptr + positions.len() as u32);

        gl.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&self.pos_buffer));
        gl.buffer_data_with_array_buffer_view(
            WebGlRenderingContext::ARRAY_BUFFER,
            &pos_array,
            WebGlRenderingContext::STATIC_DRAW, // TODO: STREAM ?
        );

        let a_pos_loc = gl.get_attrib_location(&self.program, "a_pos");
        assert!(a_pos_loc >= 0);
        let a_pos_loc = a_pos_loc as u32;

        let normalize = false;
        let stride = 0;
        let offset = 0;
        gl.vertex_attrib_pointer_with_i32(
            a_pos_loc,
            2,
            WebGlRenderingContext::FLOAT,
            normalize,
            stride,
            offset,
        );
        gl.enable_vertex_attrib_array(a_pos_loc);

        // --------------------------------------------------------------------

        let colors_memory_buffer = wasm_bindgen::memory()
            .dyn_into::<WebAssembly::Memory>()?
            .buffer();
        let colors_ptr = colors.as_ptr() as u32;
        let colors_array = js_sys::Uint8Array::new(&colors_memory_buffer)
            .subarray(colors_ptr, colors_ptr + colors.len() as u32);

        gl.bind_buffer(
            WebGlRenderingContext::ARRAY_BUFFER,
            Some(&self.color_buffer),
        );
        gl.buffer_data_with_array_buffer_view(
            WebGlRenderingContext::ARRAY_BUFFER,
            &colors_array,
            WebGlRenderingContext::STATIC_DRAW, // TODO: STREAM ?
        );

        let a_color_loc = gl.get_attrib_location(&self.program, "a_color");
        assert!(a_color_loc >= 0);
        let a_color_loc = a_color_loc as u32;

        let normalize = true;
        let stride = 0;
        let offset = 0;
        gl.vertex_attrib_pointer_with_i32(
            a_color_loc,
            4,
            WebGlRenderingContext::UNSIGNED_BYTE,
            normalize,
            stride,
            offset,
        );
        gl.enable_vertex_attrib_array(a_color_loc);

        // --------------------------------------------------------------------

        let u_screen_size_loc = gl
            .get_uniform_location(&self.program, "u_screen_size")
            .unwrap();
        gl.uniform2f(
            Some(&u_screen_size_loc),
            self.canvas.width() as f32,
            self.canvas.height() as f32,
        );
        // gl.uniform2f(Some(&u_screen_size_loc), 4.0, 1.0);

        gl.clear_color(0.05, 0.05, 0.05, 1.0);
        gl.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);

        gl.draw_elements_with_i32(
            WebGlRenderingContext::TRIANGLE_STRIP,
            indices.len() as i32,
            WebGlRenderingContext::UNSIGNED_SHORT,
            0,
        );

        Ok(())
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
        .get_shader_parameter(&shader, WebGlRenderingContext::COMPILE_STATUS)
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
        .get_program_parameter(&program, WebGlRenderingContext::LINK_STATUS)
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
