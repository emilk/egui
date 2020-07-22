#![deny(warnings)]
#![warn(clippy::all)]

use std::time::Instant;

use {
    egui::examples::ExampleApp,
    egui_glium::{make_raw_input, read_json, WindowSettings},
    glium::glutin,
};

fn main() {
    // TODO: combine into one json file?
    let memory_path = "egui.json";
    let settings_json_path: &str = "window.json";
    let app_json_path: &str = "egui_example_app.json";

    let mut egui_example_app: ExampleApp = read_json(app_json_path).unwrap_or_default();

    let event_loop = glutin::event_loop::EventLoop::new();
    let mut window = glutin::window::WindowBuilder::new()
        .with_decorations(true)
        .with_resizable(true)
        .with_title("Egui glium example")
        .with_transparent(false);

    let window_settings = WindowSettings::from_json_file(settings_json_path);
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
    let mut painter = egui_glium::Painter::new(&display);
    let mut raw_input = make_raw_input(&display);

    // used to keep track of time for animations
    let start_time = Instant::now();
    let mut frame_times = egui::MovementTracker::new(1000, 1.0);
    let mut clipboard = egui_glium::init_clipboard();

    egui_glium::read_memory(&ctx, memory_path);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = glutin::event_loop::ControlFlow::Wait;

        match event {
            glutin::event::Event::RedrawRequested(_) => {
                let egui_start = Instant::now();
                raw_input.time = start_time.elapsed().as_nanos() as f64 * 1e-9;
                raw_input.seconds_since_midnight = Some(egui_glium::local_time_of_day());

                let mut ui = ctx.begin_frame(raw_input.take());
                egui_example_app.ui(&mut ui, "");
                {
                    use egui::*;
                    let mut ui = ui.centered_column(ui.available().width().min(480.0));
                    ui.set_layout(Layout::vertical(Align::Min));
                    ui.add(label!("Egui running inside of Glium").text_style(TextStyle::Heading));
                    if ui.add(Button::new("Quit")).clicked {
                        *control_flow = glutin::event_loop::ControlFlow::Exit;
                    }

                    ui.add(
                        label!(
                            "CPU usage: {:.2} ms (excludes painting)",
                            1e3 * frame_times.average().unwrap_or_default()
                        )
                        .text_style(TextStyle::Monospace),
                    );
                    ui.add(
                        label!(
                            "FPS: {:.1}",
                            1.0 / frame_times.mean_time_interval().unwrap_or_default()
                        )
                        .text_style(TextStyle::Monospace),
                    );
                }

                let (output, paint_jobs) = ctx.end_frame();

                frame_times.add(
                    raw_input.time,
                    (Instant::now() - egui_start).as_secs_f64() as f32,
                );

                painter.paint_jobs(&display, paint_jobs, ctx.texture());
                egui_glium::handle_output(output, &display, clipboard.as_mut());

                display.gl_window().window().request_redraw(); // TODO: only if needed (new events etc)
            }
            glutin::event::Event::WindowEvent { event, .. } => {
                egui_glium::input_to_egui(event, clipboard.as_mut(), &mut raw_input, control_flow);
            }
            glutin::event::Event::LoopDestroyed => {
                // Save state to disk:
                if let Err(err) = egui_glium::write_memory(&ctx, memory_path) {
                    eprintln!("ERROR: Failed to save egui state: {}", err);
                }

                serde_json::to_writer_pretty(
                    std::fs::File::create(app_json_path).unwrap(),
                    &egui_example_app,
                )
                .unwrap();

                serde_json::to_writer_pretty(
                    std::fs::File::create(settings_json_path).unwrap(),
                    &WindowSettings::from_display(&display),
                )
                .unwrap();
            }
            _ => (),
        }
    });
}
