#![deny(warnings)]
use std::time::{Duration, Instant};

use {
    emigui::{containers::*, example_app::ExampleWindow, widgets::*, *},
    glium::glutin,
};

fn main() {
    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new().with_title("Emigui example");
    let context = glutin::ContextBuilder::new();
    let display = glium::Display::new(window, context, &events_loop).unwrap();

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
    let mut example_app = ExampleWindow::default();
    let mut clipboard = emigui_glium::init_clipboard();

    let memory_path = "emigui.json";
    emigui_glium::read_memory(&ctx, memory_path);

    while running {
        {
            // Keep smooth frame rate. TODO: proper vsync
            let frame_duration = frame_start.elapsed();
            if frame_duration < Duration::from_millis(16) {
                std::thread::sleep(Duration::from_millis(16) - frame_duration);
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
        let mut region = ctx.background_region();
        let mut region = region.centered_column(region.available_width().min(480.0));
        region.set_align(Align::Min);
        region.add(label!("Emigui running inside of Glium").text_style(emigui::TextStyle::Heading));
        if region.add(Button::new("Quit")).clicked {
            running = false;
        }

        region.add(
            label!(
                "CPU usage: {:.2} ms (excludes painting)",
                1e3 * frame_times.average().unwrap_or_default()
            )
            .text_style(TextStyle::Monospace),
        );
        region.add(
            label!(
                "FPS: {:.1}",
                1.0 / frame_times.mean_time_interval().unwrap_or_default()
            )
            .text_style(TextStyle::Monospace),
        );

        // TODO: Make it even simpler to show a window

        Window::new("Examples")
            .default_pos(pos2(50.0, 100.0))
            .default_size(vec2(300.0, 600.0))
            // .mutate(|w| w.resize = w.resize.auto_expand_width(true))
            // .resize(|r| r.auto_expand_width(true))
            .show(region.ctx(), |region| {
                example_app.ui(region);
            });

        Window::new("Emigui settings")
            .default_pos(pos2(450.0, 100.0))
            .default_size(vec2(450.0, 500.0))
            .show(region.ctx(), |region| {
                ctx.ui(region);
            });

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
