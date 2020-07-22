#![deny(warnings)]
#![warn(clippy::all)]

use std::time::Instant;

use {
    egui_glium::{
        make_raw_input,
        persistence::{Persistence, WindowSettings},
    },
    glium::glutin,
};

#[derive(Default, serde::Deserialize, serde::Serialize)]
struct App {
    egui_example_app: egui::ExampleApp,
}

impl App {
    pub fn ui(&mut self, ui: &mut egui::Ui, runner: &mut Runner) {
        self.egui_example_app.ui(ui, "");

        use egui::*;
        let mut ui = ui.centered_column(ui.available().width().min(480.0));
        ui.set_layout(Layout::vertical(Align::Min));
        ui.add(label!("Egui quit inside of Glium").text_style(TextStyle::Heading));
        if ui.add(Button::new("Quit")).clicked {
            runner.quit();
            return;
        }

        ui.add(
            label!(
                "CPU usage: {:.2} ms (excludes painting)",
                1e3 * runner.cpu_usage()
            )
            .text_style(TextStyle::Monospace),
        );
        ui.add(label!("FPS: {:.1}", runner.fps()).text_style(TextStyle::Monospace));
    }
}

struct Runner {
    frame_times: egui::MovementTracker<f32>,
    quit: bool,
}

impl Runner {
    pub fn new() -> Self {
        Self {
            frame_times: egui::MovementTracker::new(1000, 1.0),
            quit: false,
        }
    }

    pub fn quit(&mut self) {
        self.quit = true;
    }

    pub fn cpu_usage(&self) -> f32 {
        self.frame_times.average().unwrap_or_default()
    }

    pub fn fps(&self) -> f32 {
        1.0 / self.frame_times.mean_time_interval().unwrap_or_default()
    }
}

fn main() {
    const EGUI_MEMORY_KEY: &str = "egui";
    const WINDOW_KEY: &str = "window";
    const APP_KEY: &str = "app";

    let mut persistence = Persistence::from_path("egui_example_glium.json".into());

    let mut app: App = persistence.get_value("app").unwrap_or_default();

    let event_loop = glutin::event_loop::EventLoop::new();
    let mut window = glutin::window::WindowBuilder::new()
        .with_decorations(true)
        .with_resizable(true)
        .with_title("Egui glium example")
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

    let mut painter = egui_glium::Painter::new(&display);
    let mut raw_input = make_raw_input(&display);

    // used to keep track of time for animations
    let start_time = Instant::now();
    let mut runner = Runner::new();
    let mut clipboard = egui_glium::init_clipboard();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = glutin::event_loop::ControlFlow::Wait;

        match event {
            glutin::event::Event::RedrawRequested(_) => {
                let egui_start = Instant::now();
                raw_input.time = start_time.elapsed().as_nanos() as f64 * 1e-9;
                raw_input.seconds_since_midnight = Some(egui_glium::local_time_of_day());

                let mut ui = ctx.begin_frame(raw_input.take());
                app.ui(&mut ui, &mut runner);
                let (output, paint_jobs) = ctx.end_frame();

                runner.frame_times.add(
                    raw_input.time,
                    (Instant::now() - egui_start).as_secs_f64() as f32,
                );

                painter.paint_jobs(&display, paint_jobs, ctx.texture());
                egui_glium::handle_output(output, &display, clipboard.as_mut());

                if runner.quit {
                    *control_flow = glutin::event_loop::ControlFlow::Exit
                } else {
                    display.gl_window().window().request_redraw(); // TODO: only if needed (new events etc)
                }
            }
            glutin::event::Event::WindowEvent { event, .. } => {
                egui_glium::input_to_egui(event, clipboard.as_mut(), &mut raw_input, control_flow);
            }
            glutin::event::Event::LoopDestroyed => {
                persistence.set_value(APP_KEY, &app);
                persistence.set_value(WINDOW_KEY, &WindowSettings::from_display(&display));
                persistence.set_value(EGUI_MEMORY_KEY, &*ctx.memory());
                persistence.save();
            }
            _ => (),
        }
    });
}
