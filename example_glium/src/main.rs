#![deny(warnings)]
#![warn(clippy::all)]

use std::time::{Duration, Instant};

use {
    emigui::{example_app::ExampleApp, widgets::*, *},
    glium::glutin,
};

fn main() {
    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new().with_title("Emigui example");
    let context = glutin::ContextBuilder::new();
    let display = glium::Display::new(window, context, &events_loop).unwrap();

    // TODO: persist position/size
    display
        .gl_window()
        .set_inner_size(glutin::dpi::LogicalSize {
            width: 1024.0,
            height: 800.0,
        });
    display.gl_window().set_position((0, 24).into()); // Useful when ddeveloping and constantly restarting it

    let pixels_per_point = display.gl_window().get_hidpi_factor() as f32;

    let mut ctx = Context::new(pixels_per_point);
    let mut painter = emigui_glium::Painter::new(&display);

    let mut raw_input = emigui::RawInput {
        screen_size: {
            let (width, height) = display.get_framebuffer_dimensions();
            vec2(width as f32, height as f32) / pixels_per_point
        },
        pixels_per_point,
        ..Default::default()
    };

    // used to keep track of time for animations
    let start_time = Instant::now();
    let mut running = true;
    let mut frame_start = Instant::now();
    let mut frame_times = emigui::MovementTracker::new(1000, 1.0);
    let mut example_app = ExampleApp::default();
    let mut clipboard = emigui_glium::init_clipboard();

    let memory_path = "emigui.json";
    emigui_glium::read_memory(&ctx, memory_path);

    while running {
        {
            // Keep smooth frame rate. TODO: proper vsync
            let frame_duration = frame_start.elapsed();
            if frame_duration < Duration::from_millis(33) {
                std::thread::sleep(Duration::from_millis(33) - frame_duration);
            }
            frame_start = Instant::now();
        }

        {
            raw_input.time = start_time.elapsed().as_nanos() as f64 * 1e-9;
            raw_input.scroll_delta = vec2(0.0, 0.0);
            raw_input.dropped_files.clear();
            raw_input.hovered_files.clear();
            raw_input.events.clear();
            events_loop.poll_events(|event| {
                emigui_glium::input_event(event, clipboard.as_mut(), &mut raw_input, &mut running)
            });
        }

        let emigui_start = Instant::now();
        ctx.begin_frame(raw_input.clone()); // TODO: avoid clone
        let mut ui = ctx.fullscreen_ui();
        let mut ui = ui.centered_column(ui.available_width().min(480.0));
        ui.set_align(Align::Min);
        ui.add(label!("Emigui running inside of Glium").text_style(emigui::TextStyle::Heading));
        if ui.add(Button::new("Quit")).clicked {
            running = false;
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

        example_app.ui(&ctx);

        let (output, paint_batches) = ctx.end_frame();

        frame_times.add(
            raw_input.time,
            (Instant::now() - emigui_start).as_secs_f64() as f32,
        );

        painter.paint_batches(&display, paint_batches, ctx.texture());
        emigui_glium::handle_output(output, &display, clipboard.as_mut());
    }

    if let Err(err) = emigui_glium::write_memory(&ctx, memory_path) {
        eprintln!("ERROR: Failed to save emigui state: {}", err);
    }
}
