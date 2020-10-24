use std::time::Instant;

use crate::{storage::WindowSettings, *};

pub use egui::{
    app::{self, App, Storage},
    Srgba,
};

const EGUI_MEMORY_KEY: &str = "egui";
const WINDOW_KEY: &str = "window";

impl egui::app::TextureAllocator for Painter {
    fn new_texture_srgba_premultiplied(
        &mut self,
        size: (usize, usize),
        pixels: &[Srgba],
    ) -> egui::TextureId {
        self.new_user_texture(size, pixels)
    }
}

fn create_display(
    title: &str,
    window_settings: Option<WindowSettings>,
    event_loop: &glutin::event_loop::EventLoop<()>,
) -> glium::Display {
    let mut window_builder = glutin::window::WindowBuilder::new()
        .with_decorations(true)
        .with_resizable(true)
        .with_title(title)
        .with_transparent(false);

    if let Some(window_settings) = &window_settings {
        window_builder = window_settings.initialize_size(window_builder);
    }

    let context_builder = glutin::ContextBuilder::new()
        .with_depth_buffer(0)
        .with_srgb(true)
        .with_stencil_buffer(0)
        .with_vsync(true);

    glium::Display::new(window_builder, context_builder, &event_loop).unwrap()
}

/// Run an egui app
pub fn run(
    title: &str,
    mut storage: Box<dyn egui::app::Storage>,
    mut app: impl App + 'static,
) -> ! {
    let window_settings: Option<WindowSettings> =
        egui::app::get_value(storage.as_ref(), WINDOW_KEY);
    let event_loop = glutin::event_loop::EventLoop::new();
    let display = create_display(title, window_settings, &event_loop);

    let mut ctx = egui::Context::new();
    *ctx.memory() = egui::app::get_value(storage.as_ref(), EGUI_MEMORY_KEY).unwrap_or_default();

    let mut raw_input = egui::RawInput {
        pixels_per_point: Some(native_pixels_per_point(&display)),
        ..Default::default()
    };

    let start_time = Instant::now();
    let mut previous_frame_time = None;
    let mut painter = Painter::new(&display);
    let mut clipboard = init_clipboard();

    event_loop.run(move |event, _, control_flow| {
        let mut redraw = || {
            let egui_start = Instant::now();
            raw_input.time = start_time.elapsed().as_nanos() as f64 * 1e-9;
            raw_input.screen_size =
                screen_size_in_pixels(&display) / raw_input.pixels_per_point.unwrap();

            ctx.begin_frame(raw_input.take());
            let mut integration_context = egui::app::IntegrationContext {
                info: egui::app::IntegrationInfo {
                    web_info: None,
                    cpu_usage: previous_frame_time,
                    seconds_since_midnight: Some(seconds_since_midnight()),
                    native_pixels_per_point: Some(native_pixels_per_point(&display)),
                },
                tex_allocator: Some(&mut painter),
                output: Default::default(),
            };
            app.ui(&ctx, &mut integration_context);
            let app_output = integration_context.output;
            let (egui_output, paint_jobs) = ctx.end_frame();

            let frame_time = (Instant::now() - egui_start).as_secs_f64() as f32;
            previous_frame_time = Some(frame_time);
            painter.paint_jobs(&display, ctx.pixels_per_point(), paint_jobs, &ctx.texture());

            {
                let egui::app::AppOutput {
                    quit,
                    window_size,
                    pixels_per_point,
                } = app_output;

                if let Some(pixels_per_point) = pixels_per_point {
                    // User changed GUI scale
                    raw_input.pixels_per_point = Some(pixels_per_point);
                }

                if let Some(window_size) = window_size {
                    display
                        .gl_window()
                        .window()
                        .set_inner_size(glutin::dpi::LogicalSize {
                            width: window_size.x,
                            height: window_size.y,
                        });
                }

                *control_flow = if quit {
                    glutin::event_loop::ControlFlow::Exit
                } else if egui_output.needs_repaint {
                    display.gl_window().window().request_redraw();
                    glutin::event_loop::ControlFlow::Poll
                } else {
                    glutin::event_loop::ControlFlow::Wait
                };
            }

            handle_output(egui_output, &display, clipboard.as_mut());
        };

        match event {
            // Platform-dependent event handlers to workaround a winit bug
            // See: https://github.com/rust-windowing/winit/issues/987
            // See: https://github.com/rust-windowing/winit/issues/1619
            glutin::event::Event::RedrawEventsCleared if cfg!(windows) => redraw(),
            glutin::event::Event::RedrawRequested(_) if !cfg!(windows) => redraw(),

            glutin::event::Event::WindowEvent { event, .. } => {
                input_to_egui(event, clipboard.as_mut(), &mut raw_input, control_flow);
                display.gl_window().window().request_redraw(); // TODO: ask Egui if the events warrants a repaint instead
            }
            glutin::event::Event::LoopDestroyed => {
                egui::app::set_value(
                    storage.as_mut(),
                    WINDOW_KEY,
                    &WindowSettings::from_display(&display),
                );
                egui::app::set_value(storage.as_mut(), EGUI_MEMORY_KEY, &*ctx.memory());
                app.on_exit(storage.as_mut());
                storage.flush();
            }
            _ => (),
        }
    });
}
