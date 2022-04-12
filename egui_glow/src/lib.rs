//! [`egui`] bindings for [`glow`](https://github.com/grovesNL/glow).
//!
//! The main type you want to use is [`EguiGlow`].
//!
//! This library is an [`epi`] backend.
//! If you are writing an app, you may want to look at [`eframe`](https://docs.rs/eframe) instead.

#![allow(clippy::float_cmp)]
#![allow(clippy::manual_range_contains)]

pub mod painter;
pub use glow;
pub use painter::Painter;
mod misc_util;
mod post_process;
mod shader_version;
mod vao;

#[cfg(all(not(target_arch = "wasm32"), feature = "winit"))]
pub mod winit;
#[cfg(all(not(target_arch = "wasm32"), feature = "winit"))]
pub use winit::*;

#[cfg(all(not(target_arch = "wasm32"), feature = "winit"))]
mod epi_backend;

#[cfg(all(not(target_arch = "wasm32"), feature = "winit"))]
pub use epi_backend::{run, NativeOptions};

/// Check for OpenGL error and report it using `tracing::error`.
///
/// ``` no_run
/// # let glow_context = todo!();
/// use egui_glow::check_for_gl_error;
/// check_for_gl_error!(glow_context);
/// check_for_gl_error!(glow_context, "during painting");
/// ```
#[macro_export]
macro_rules! check_for_gl_error {
    ($gl: expr) => {{
        $crate::check_for_gl_error_impl($gl, file!(), line!(), "")
    }};
    ($gl: expr, $context: literal) => {{
        $crate::check_for_gl_error_impl($gl, file!(), line!(), $context)
    }};
}

#[doc(hidden)]
pub fn check_for_gl_error_impl(gl: &glow::Context, file: &str, line: u32, context: &str) {
    use glow::HasContext as _;
    #[allow(unsafe_code)]
    let error_code = unsafe { gl.get_error() };
    if error_code != glow::NO_ERROR {
        let error_str = match error_code {
            glow::INVALID_ENUM => "GL_INVALID_ENUM",
            glow::INVALID_VALUE => "GL_INVALID_VALUE",
            glow::INVALID_OPERATION => "GL_INVALID_OPERATION",
            glow::STACK_OVERFLOW => "GL_STACK_OVERFLOW",
            glow::STACK_UNDERFLOW => "GL_STACK_UNDERFLOW",
            glow::OUT_OF_MEMORY => "GL_OUT_OF_MEMORY",
            glow::INVALID_FRAMEBUFFER_OPERATION => "GL_INVALID_FRAMEBUFFER_OPERATION",
            glow::CONTEXT_LOST => "GL_CONTEXT_LOST",
            0x8031 => "GL_TABLE_TOO_LARGE1",
            0x9242 => "CONTEXT_LOST_WEBGL",
            _ => "<unknown>",
        };

        if context.is_empty() {
            tracing::error!(
                "GL error, at {}:{}: {} (0x{:X}). Please file a bug at https://github.com/emilk/egui/issues",
                file,
                line,
                error_str,
                error_code,
            );
        } else {
            tracing::error!(
                "GL error, at {}:{} ({}): {} (0x{:X}). Please file a bug at https://github.com/emilk/egui/issues",
                file,
                line,
                context,
                error_str,
                error_code,
            );
        }
    }
}

// ---------------------------------------------------------------------------

/// Profiling macro for feature "puffin"
#[doc(hidden)]
#[macro_export]
macro_rules! profile_function {
    ($($arg: tt)*) => {
        #[cfg(feature = "puffin")]
        puffin::profile_function!($($arg)*);
    };
}

/// Profiling macro for feature "puffin"
#[doc(hidden)]
#[macro_export]
macro_rules! profile_scope {
    ($($arg: tt)*) => {
        #[cfg(feature = "puffin")]
        puffin::profile_scope!($($arg)*);
    };
}
