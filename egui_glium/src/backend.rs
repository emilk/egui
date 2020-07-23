use std::time::Instant;

use crate::{
    storage::{FileStorage, WindowSettings},
    *,
};

pub use egui::app::{App, Backend, RunMode, Storage};

const EGUI_MEMORY_KEY: &str = "egui";
const WINDOW_KEY: &str = "window";

pub struct GliumBackend {
    frame_times: egui::MovementTracker<f32>,
    quit: bool,
    run_mode: RunMode,
}

impl GliumBackend {
    pub fn new(run_mode: RunMode) -> Self {
        Self {
            frame_times: egui::MovementTracker::new(1000, 1.0),
            quit: false,
            run_mode,
        }
    }
}

impl Backend for GliumBackend {
    fn run_mode(&self) -> RunMode {
        self.run_mode
    }

    fn set_run_mode(&mut self, run_mode: RunMode) {
        self.run_mode = run_mode;
    }

    fn cpu_time(&self) -> f32 {
        self.frame_times.average().unwrap_or_default()
    }

    fn fps(&self) -> f32 {
        1.0 / self.frame_times.mean_time_interval().unwrap_or_default()
    }

    fn quit(&mut self) {
        self.quit = true;
    }
}

/// Run an egui app
pub fn run(
    title: &str,
    run_mode: RunMode,
    mut storage: FileStorage,
    mut app: impl App + 'static,
) -> ! {
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

    let mut painter = Painter::new(&display);
    let mut raw_input = make_raw_input(&display);

    // used to keep track of time for animations
    let start_time = Instant::now();
    let mut runner = GliumBackend::new(run_mode);
    let mut clipboard = init_clipboard();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = glutin::event_loop::ControlFlow::Wait;

        match event {
            glutin::event::Event::RedrawRequested(_) => {
                let egui_start = Instant::now();
                raw_input.time = start_time.elapsed().as_nanos() as f64 * 1e-9;
                raw_input.seconds_since_midnight = Some(local_time_of_day());

                let mut ui = ctx.begin_frame(raw_input.take());
                app.ui(&mut ui, &mut runner);
                let (output, paint_jobs) = ctx.end_frame();

                let frame_time = (Instant::now() - egui_start).as_secs_f64() as f32;
                runner.frame_times.add(raw_input.time, frame_time);

                painter.paint_jobs(&display, paint_jobs, ctx.texture());

                if runner.quit {
                    *control_flow = glutin::event_loop::ControlFlow::Exit
                } else if runner.run_mode() == RunMode::Continuous || output.needs_repaint {
                    display.gl_window().window().request_redraw();
                }

                handle_output(output, &display, clipboard.as_mut());
            }
            glutin::event::Event::WindowEvent { event, .. } => {
                input_to_egui(event, clipboard.as_mut(), &mut raw_input, control_flow);
                display.gl_window().window().request_redraw(); // TODO: maybe only on some events?
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
