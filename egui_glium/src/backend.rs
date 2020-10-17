use std::time::Instant;

use crate::{
    storage::{FileStorage, WindowSettings},
    *,
};

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

/// Run an egui app
pub fn run(title: &str, mut storage: FileStorage, mut app: impl App + 'static) -> ! {
    let event_loop = glutin::event_loop::EventLoop::new();
    let mut window = glutin::window::WindowBuilder::new()
        .with_decorations(true)
        .with_resizable(true)
        .with_title(title)
        .with_transparent(false);

    let window_settings: Option<WindowSettings> = egui::app::get_value(&storage, WINDOW_KEY);
    if let Some(window_settings) = &window_settings {
        window = window_settings.initialize_size(window);
    }

    let context = glutin::ContextBuilder::new()
        .with_depth_buffer(0)
        .with_srgb(true)
        .with_stencil_buffer(0)
        .with_vsync(true);
    let display = glium::Display::new(window, context, &event_loop).unwrap();

    if let Some(window_settings) = &window_settings {
        window_settings.restore_positions(&display);
    }

    let mut ctx = egui::Context::new();
    *ctx.memory() = egui::app::get_value(&storage, EGUI_MEMORY_KEY).unwrap_or_default();

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

            let backend_info = egui::app::BackendInfo {
                web_info: None,
                cpu_usage: previous_frame_time,
                seconds_since_midnight: Some(seconds_since_midnight()),
                native_pixels_per_point: Some(native_pixels_per_point(&display)),
            };

            let mut ui = ctx.begin_frame(raw_input.take());
            let app_output = app.ui(&mut ui, &backend_info, Some(&mut painter));
            let (egui_output, paint_jobs) = ctx.end_frame();

            let frame_time = (Instant::now() - egui_start).as_secs_f64() as f32;
            previous_frame_time = Some(frame_time);
            painter.paint_jobs(&display, ctx.pixels_per_point(), paint_jobs, &ctx.texture());

            {
                let egui::app::AppOutput {
                    quit,
                    pixels_per_point,
                } = app_output;
                if let Some(pixels_per_point) = pixels_per_point {
                    // User changed GUI scale
                    raw_input.pixels_per_point = Some(pixels_per_point);
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
                    &mut storage,
                    WINDOW_KEY,
                    &WindowSettings::from_display(&display),
                );
                egui::app::set_value(&mut storage, EGUI_MEMORY_KEY, &*ctx.memory());
                app.on_exit(&mut storage);
                storage.save();
            }
            _ => (),
        }
    });
}
