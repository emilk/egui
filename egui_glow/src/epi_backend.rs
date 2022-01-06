use crate::*;

#[cfg(target_arch = "wasm32")]
use crate::epi_web::{create_gl_context, NeedRepaint};
#[cfg(not(target_arch = "wasm32"))]
struct RequestRepaintEvent;
#[cfg(not(target_arch = "wasm32"))]
struct GlowRepaintSignal(
    std::sync::Mutex<egui_winit::winit::event_loop::EventLoopProxy<RequestRepaintEvent>>,
);
#[cfg(not(target_arch = "wasm32"))]
impl epi::backend::RepaintSignal for GlowRepaintSignal {
    fn request_repaint(&self) {
        self.0.lock().unwrap().send_event(RequestRepaintEvent).ok();
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(unsafe_code)]
fn create_display(
    window_builder: egui_winit::winit::window::WindowBuilder,
    event_loop: &egui_winit::winit::event_loop::EventLoop<RequestRepaintEvent>,
) -> (
    glutin::WindowedContext<glutin::PossiblyCurrent>,
    glow::Context,
) {
    let gl_window = unsafe {
        glutin::ContextBuilder::new()
            .with_depth_buffer(0)
            .with_srgb(true)
            .with_stencil_buffer(0)
            .with_vsync(true)
            .build_windowed(window_builder, event_loop)
            .unwrap()
            .make_current()
            .unwrap()
    };

    let gl = unsafe { glow::Context::from_loader_function(|s| gl_window.get_proc_address(s)) };

    unsafe {
        use glow::HasContext as _;
        gl.enable(glow::FRAMEBUFFER_SRGB);
    }

    (gl_window, gl)
}

// ----------------------------------------------------------------------------

pub use epi::NativeOptions;

/// Run an egui app
#[allow(unsafe_code)]
pub fn run(app: Box<dyn epi::App>, native_options: &epi::NativeOptions) -> ! {
    // when startup on web need to delete loading animation
    // we should done in JS but winit wont return control.
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window().map(|window: web_sys::Window| {
            window.document().map(|document: web_sys::Document| {
                document
                    .get_element_by_id("loading")
                    .map(|element: web_sys::Element| element.remove())
            });
        });
    }

    let persistence = egui_winit::epi::Persistence::from_app_name(app.name());
    let window_settings = persistence.load_window_settings();
    let window_builder =
        egui_winit::epi::window_builder(native_options, &window_settings).with_title(app.name());
    let event_loop = egui_winit::winit::event_loop::EventLoop::with_user_event();
    #[cfg(not(target_arch = "wasm32"))]
    let (gl_window, gl) = create_display(window_builder, &event_loop);
    #[cfg(not(target_arch = "wasm32"))]
    let install_webkit_gtk_fix = false;
    #[cfg(target_arch = "wasm32")]
    let (gl_window, (gl, install_webkit_gtk_fix)) =
        create_gl_context(window_builder, &event_loop).unwrap();

    let dimension = {
        if cfg!(target_arch = "wasm32") {
            let inner_size = gl_window.window().inner_size();
            Some([inner_size.width as i32, inner_size.height as i32])
        } else {
            None
        }
    };
    #[cfg(not(target_arch = "wasm32"))]
    let repaint_signal = std::sync::Arc::new(GlowRepaintSignal(std::sync::Mutex::new(
        event_loop.create_proxy(),
    )));
    #[cfg(target_arch = "wasm32")]
    let repaint_signal = std::sync::Arc::new(NeedRepaint::default());
    // for WebKitGTK insert shader_prefix.
    let shader_prefix = if install_webkit_gtk_fix {
        crate::misc_util::glow_debug_print("Enabling webkitGTK brightening workaround");
        "#define APPLY_BRIGHTENING_GAMMA"
    } else {
        ""
    };

    let mut painter = crate::Painter::new(&gl, dimension, shader_prefix)
        .map_err(|error| eprintln!("some OpenGL error occurred {}\n", error))
        .unwrap();
    let mut integration = egui_winit::epi::EpiIntegration::new(
        "egui_glow",
        gl_window.window(),
        repaint_signal,
        persistence,
        app,
    );

    let mut is_focused = true;

    event_loop.run(move |event, _, control_flow| {
        let mut redraw = || {
            if !is_focused {
                // On Mac, a minimized Window uses up all CPU: https://github.com/emilk/egui/issues/325
                // We can't know if we are minimized: https://github.com/rust-windowing/winit/issues/208
                // But we know if we are focused (in foreground). When minimized, we are not focused.
                // However, a user may want an egui with an animation in the background,
                // so we still need to repaint quite fast.
                #[cfg(not(target_arch = "wasm32"))]
                std::thread::sleep(std::time::Duration::from_millis(10));
            }

            let (needs_repaint, mut tex_allocation_data, shapes) =
                integration.update(gl_window.window());
            let clipped_meshes = integration.egui_ctx.tessellate(shapes);

            for (id, image) in tex_allocation_data.creations {
                painter.set_texture(&gl, id, &image);
            }

            // paint:
            {
                let color = integration.app.clear_color();
                unsafe {
                    use glow::HasContext as _;
                    gl.disable(glow::SCISSOR_TEST);
                    gl.clear_color(color[0], color[1], color[2], color[3]);
                    gl.clear(glow::COLOR_BUFFER_BIT);
                }
                painter.upload_egui_texture(&gl, &integration.egui_ctx.font_image());
                painter.paint_meshes(
                    &gl,
                    gl_window.window().inner_size().into(),
                    integration.egui_ctx.pixels_per_point(),
                    clipped_meshes,
                );
                #[cfg(not(target_arch = "wasm32"))]
                gl_window.swap_buffers().unwrap();
            }

            for id in tex_allocation_data.destructions.drain(..) {
                painter.free_texture(id);
            }

            {
                *control_flow = if integration.should_quit() {
                    egui_winit::winit::event_loop::ControlFlow::Exit
                } else if needs_repaint {
                    gl_window.window().request_redraw();
                    egui_winit::winit::event_loop::ControlFlow::Poll
                } else {
                    egui_winit::winit::event_loop::ControlFlow::Wait
                };
            }

            integration.maybe_autosave(gl_window.window());
        };

        match event {
            // Platform-dependent event handlers to workaround a winit bug
            // See: https://github.com/rust-windowing/winit/issues/987
            // See: https://github.com/rust-windowing/winit/issues/1619
            egui_winit::winit::event::Event::RedrawEventsCleared if cfg!(windows) => redraw(),
            egui_winit::winit::event::Event::RedrawRequested(_) if !cfg!(windows) => redraw(),

            egui_winit::winit::event::Event::WindowEvent { event, .. } => {
                if let egui_winit::winit::event::WindowEvent::Focused(new_focused) = event {
                    is_focused = new_focused;
                }

                //#[cfg(not(target_arch = "wasm32"))]
                if let egui_winit::winit::event::WindowEvent::Resized(physical_size) = event {
                    gl_window.resize(physical_size);
                }

                integration.on_event(&event);
                if integration.should_quit() {
                    *control_flow = egui_winit::winit::event_loop::ControlFlow::Exit;
                }

                gl_window.window().request_redraw(); // TODO: ask egui if the events warrants a repaint instead
            }
            egui_winit::winit::event::Event::LoopDestroyed => {
                integration.on_exit(gl_window.window());
                painter.destroy(&gl);
            }
            egui_winit::winit::event::Event::UserEvent(_) => {
                gl_window.window().request_redraw();
            }
            _ => (),
        }
    });
}
