//! [`egui`] bindings for [`glow`](https://github.com/grovesNL/glow).
//!
//! The main type you want to use is [`EguiGlow`].
//!
//! This library is an [`epi`] backend.
//! If you are writing an app, you may want to look at [`eframe`](https://docs.rs/eframe) instead.

// Forbid warnings in release builds:
#![cfg_attr(not(debug_assertions), deny(warnings))]
#![deny(unsafe_code)]
#![warn(
    clippy::all,
    clippy::await_holding_lock,
    clippy::char_lit_as_u8,
    clippy::checked_conversions,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::disallowed_method,
    clippy::doc_markdown,
    clippy::empty_enum,
    clippy::enum_glob_use,
    clippy::exit,
    clippy::expl_impl_clone_on_copy,
    clippy::explicit_deref_methods,
    clippy::explicit_into_iter_loop,
    clippy::fallible_impl_from,
    clippy::filter_map_next,
    clippy::flat_map_option,
    clippy::float_cmp_const,
    clippy::fn_params_excessive_bools,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::implicit_clone,
    clippy::imprecise_flops,
    clippy::inefficient_to_string,
    clippy::invalid_upcast_comparisons,
    clippy::large_digit_groups,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::let_unit_value,
    clippy::linkedlist,
    clippy::lossy_float_literal,
    clippy::macro_use_imports,
    clippy::manual_ok_or,
    clippy::map_err_ignore,
    clippy::map_flatten,
    clippy::map_unwrap_or,
    clippy::match_on_vec_items,
    clippy::match_same_arms,
    clippy::match_wild_err_arm,
    clippy::match_wildcard_for_single_variants,
    clippy::mem_forget,
    clippy::mismatched_target_os,
    clippy::missing_errors_doc,
    clippy::missing_safety_doc,
    clippy::mut_mut,
    clippy::mutex_integer,
    clippy::needless_borrow,
    clippy::needless_continue,
    clippy::needless_for_each,
    clippy::needless_pass_by_value,
    clippy::option_option,
    clippy::path_buf_push_overwrite,
    clippy::ptr_as_ptr,
    clippy::ref_option_ref,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_functions_in_if_condition,
    clippy::semicolon_if_nothing_returned,
    clippy::single_match_else,
    clippy::string_add_assign,
    clippy::string_add,
    clippy::string_lit_as_bytes,
    clippy::string_to_string,
    clippy::todo,
    clippy::trait_duplication_in_bounds,
    clippy::unimplemented,
    clippy::unnested_or_patterns,
    clippy::unused_self,
    clippy::useless_transmute,
    clippy::verbose_file_reads,
    clippy::zero_sized_map_values,
    future_incompatible,
    missing_crate_level_docs,
    nonstandard_style,
    rust_2018_idioms
)]
#![allow(clippy::float_cmp)]
#![allow(clippy::manual_range_contains)]

pub mod painter;
pub use glow;
pub use painter::Painter;
#[cfg(feature = "winit")]
mod epi_backend;
mod misc_util;
mod post_process;
mod shader_version;
mod vao_emulate;

#[cfg(not(target_arch = "wasm32"))]
pub use egui_winit;
#[cfg(all(feature = "epi", feature = "winit"))]
pub use epi_backend::{run, NativeOptions};

// ----------------------------------------------------------------------------

/// Use [`egui`] from a [`glow`] app.
#[cfg(feature = "winit")]
pub struct EguiGlow {
    pub egui_ctx: egui::CtxRef,
    pub egui_winit: egui_winit::State,
    pub painter: crate::Painter,
}

#[cfg(feature = "winit")]
impl EguiGlow {
    pub fn new(
        gl_window: &glutin::WindowedContext<glutin::PossiblyCurrent>,
        gl: &glow::Context,
    ) -> Self {
        Self {
            egui_ctx: Default::default(),
            egui_winit: egui_winit::State::new(gl_window.window()),
            painter: crate::Painter::new(gl, None)
                .map_err(|error| {
                    eprintln!("some error occurred in initializing painter\n{}", error);
                })
                .unwrap(),
        }
    }

    /// Returns `true` if egui wants exclusive use of this event
    /// (e.g. a mouse click on an egui window, or entering text into a text field).
    /// For instance, if you use egui for a game, you want to first call this
    /// and only when this returns `false` pass on the events to your game.
    ///
    /// Note that egui uses `tab` to move focus between elements, so this will always return `true` for tabs.
    pub fn on_event(&mut self, event: &glutin::event::WindowEvent<'_>) -> bool {
        self.egui_winit.on_event(&self.egui_ctx, event)
    }

    /// Is this a close event or a Cmd-Q/Alt-F4 keyboard command?
    pub fn is_quit_event(&self, event: &glutin::event::WindowEvent<'_>) -> bool {
        self.egui_winit.is_quit_event(event)
    }

    /// Returns `needs_repaint` and shapes to draw.
    pub fn run(
        &mut self,
        window: &glutin::window::Window,
        mut run_ui: impl FnMut(&egui::CtxRef),
    ) -> (bool, Vec<egui::epaint::ClippedShape>) {
        let raw_input = self.egui_winit.take_egui_input(window);
        self.egui_ctx.begin_frame(raw_input);

        run_ui(&self.egui_ctx);

        let (egui_output, shapes) = self.egui_ctx.end_frame();
        let needs_repaint = egui_output.needs_repaint;
        self.egui_winit
            .handle_output(window, &self.egui_ctx, egui_output);
        (needs_repaint, shapes)
    }

    pub fn paint(
        &mut self,
        gl_window: &glutin::WindowedContext<glutin::PossiblyCurrent>,
        gl: &glow::Context,
        shapes: Vec<egui::epaint::ClippedShape>,
    ) {
        let clipped_meshes = self.egui_ctx.tessellate(shapes);
        let dimensions: [u32; 2] = gl_window.window().inner_size().into();
        self.painter
            .upload_egui_texture(gl, &self.egui_ctx.texture());
        self.painter.paint_meshes(
            dimensions,
            gl,
            self.egui_ctx.pixels_per_point(),
            clipped_meshes,
        );
    }

    /// Call to release the allocated graphics resources.
    pub fn destroy(&mut self, gl: &glow::Context) {
        self.painter.destroy(gl);
    }
}
