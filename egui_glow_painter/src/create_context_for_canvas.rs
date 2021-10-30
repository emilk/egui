#[cfg(target_arch = "wasm32")]
use web_sys::HtmlCanvasElement;

/// Create glow context from given canvas.
/// Automatically choose webgl or webgl2 context.
/// first try webgl2 falling back to webgl1
#[cfg(target_arch = "wasm32")]
pub fn init_glow_context_from_canvas(canvas: &HtmlCanvasElement) -> glow::Context {
    use crate::glow_debug_print;
    use std::process::exit;
    use wasm_bindgen::JsCast;
    let ctx = canvas.get_context("webgl2");
    if let Ok(ctx) = ctx {
        glow_debug_print("webgl found");
        if let Some(ctx) = ctx {
            glow_debug_print("webgl 2 selected");
            let gl_ctx = ctx.dyn_into::<web_sys::WebGl2RenderingContext>().unwrap();
            glow::Context::from_webgl2_context(gl_ctx)
        } else {
            let ctx = canvas.get_context("webgl");
            if let Ok(ctx) = ctx {
                glow_debug_print("falling back to webgl1");
                if let Some(ctx) = ctx {
                    glow_debug_print("webgl selected");
                    let gl_ctx = ctx.dyn_into::<web_sys::WebGlRenderingContext>().unwrap();
                    glow_debug_print("success");
                    glow::Context::from_webgl1_context(gl_ctx)
                } else {
                    panic!("tried webgl1 but can't get context");
                }
            } else {
                panic!("tried webgl1 but can't get context");
            }
        }
    } else {
        panic!("tried webgl2 but something went wrong");
    }
}
/// dummy for clippy
#[cfg(not(target_arch = "wasm32"))]
pub fn init_glow_context_from_canvas<T>(_: T) -> glow::Context {
    unimplemented!("this is only enabled wasm target")
}
