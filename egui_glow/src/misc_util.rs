#![allow(unsafe_code)]
use glow::HasContext;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

pub(crate) fn srgbtexture2d(
    gl: &glow::Context,
    compatibility_mode: bool,
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
        if compatibility_mode {
            glow_debug_print(format!("w : {} h : {}", w as i32, h as i32));
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
