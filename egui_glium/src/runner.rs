use std::time::Instant;

use crate::{
    persistence::{Persistence, WindowSettings},
    *,
};

const EGUI_MEMORY_KEY: &str = "egui";
const WINDOW_KEY: &str = "window";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunMode {
    /// Uses `request_animation_frame` to repaint the UI on each display Hz.
    /// This is good for games and stuff where you want to run logic at e.g. 60 FPS.
    Continuous,

    /// Only repaint when there are animations or input (mouse movement, keyboard input etc).
    Reactive,
}

pub trait App {
    /// Called onced per frame for you to draw the UI.
    fn ui(&mut self, ui: &mut egui::Ui, runner: &mut Runner);

    /// Called once on shutdown. Allows you to save state.
    fn on_exit(&mut self, persistence: &mut Persistence);
}

pub struct Runner {
    frame_times: egui::MovementTracker<f32>,
    quit: bool,
    run_mode: RunMode,
}

impl Runner {
    pub fn new(run_mode: RunMode) -> Self {
        Self {
            frame_times: egui::MovementTracker::new(1000, 1.0),
            quit: false,
            run_mode,
        }
    }

    pub fn run_mode(&self) -> RunMode {
        self.run_mode
    }

    pub fn set_run_mode(&mut self, run_mode: RunMode) {
        self.run_mode = run_mode;
    }

    pub fn quit(&mut self) {
        self.quit = true;
    }

    pub fn cpu_time(&self) -> f32 {
        self.frame_times.average().unwrap_or_default()
    }

    pub fn fps(&self) -> f32 {
        1.0 / self.frame_times.mean_time_interval().unwrap_or_default()
    }
}

/// Run an egui app
pub fn run(
    title: &str,
    run_mode: RunMode,
    mut persistence: Persistence,
    mut app: impl App + 'static,
) -> ! {
    let event_loop = glutin::event_loop::EventLoop::new();
    let mut window = glutin::window::WindowBuilder::new()
        .with_decorations(true)
        .with_resizable(true)
        .with_title(title)
        .with_transparent(false);

    let window_settings: Option<WindowSettings> = persistence.get_value(WINDOW_KEY);
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
    *ctx.memory() = persistence.get_value(EGUI_MEMORY_KEY).unwrap_or_default();

    let mut painter = Painter::new(&display);
    let mut raw_input = make_raw_input(&display);

    // used to keep track of time for animations
    let start_time = Instant::now();
    let mut runner = Runner::new(run_mode);
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
                persistence.set_value(WINDOW_KEY, &WindowSettings::from_display(&display));
                persistence.set_value(EGUI_MEMORY_KEY, &*ctx.memory());
                app.on_exit(&mut persistence);
                persistence.save();
            }
            _ => (),
        }
    });
}
