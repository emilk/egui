#![deny(warnings)]
#![warn(clippy::all)]

use std::time::{Duration, Instant};

use {
    emigui::{examples::ExampleApp, paint::TextStyle, widgets::*, *},
    glium::glutin,
};

#[derive(Default, serde_derive::Deserialize, serde_derive::Serialize)]
struct Window {
    pos: Option<Pos2>,
    size: Option<Vec2>,
}

fn read_state(memory_json_path: impl AsRef<std::path::Path>) -> Option<Window> {
    match std::fs::File::open(memory_json_path) {
        Ok(file) => {
            let reader = std::io::BufReader::new(file);
            match serde_json::from_reader(reader) {
                Ok(value) => Some(value),
                Err(err) => {
                    eprintln!("ERROR: Failed to parse json: {}", err);
                    None
                }
            }
        }
        Err(_err) => {
            // File probably doesn't exist. That's fine.
            None
        }
    }
}

fn main() {
    // TODO: combine
    let memory_path = "emigui.json";
    let settings_json_path: &str = "window.json";

    let mut window_settings: Window = read_state(settings_json_path).unwrap_or_default();

    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new().with_title("Emigui example");
    let context = glutin::ContextBuilder::new();
    let display = glium::Display::new(window, context, &events_loop).unwrap();

    let size = window_settings.size.unwrap_or(vec2(1024.0, 800.0));

    display
        .gl_window()
        .set_inner_size(glutin::dpi::LogicalSize {
            width: size.x as f64,
            height: size.y as f64,
        });

    if let Some(pos) = window_settings.pos {
        display
            .gl_window()
            .set_position((pos.x as f64, pos.y as f64).into());
    }

    let pixels_per_point = display.gl_window().get_hidpi_factor() as f32;

    let mut ctx = profile("initializing emilib", || Context::new(pixels_per_point));
    let mut painter = profile("initializing painter", || {
        emigui_glium::Painter::new(&display)
    });

    let mut raw_input = emigui::RawInput {
        screen_size: {
            let (width, height) = display.get_framebuffer_dimensions();
            vec2(width as f32, height as f32) / pixels_per_point
        },
        pixels_per_point: Some(pixels_per_point),
        ..Default::default()
    };

    // used to keep track of time for animations
    let start_time = Instant::now();
    let mut running = true;
    let mut frame_start = Instant::now();
    let mut frame_times = emigui::MovementTracker::new(1000, 1.0);
    let mut example_app = ExampleApp::default();
    let mut clipboard = emigui_glium::init_clipboard();

    emigui_glium::read_memory(&ctx, memory_path);

    while running {
        {
            // Keep smooth frame rate because vsync doesn't work on mac
            let frame_duration = frame_start.elapsed();
            if frame_duration < Duration::from_millis(16) {
                std::thread::sleep(Duration::from_millis(16) - frame_duration);
            }
            frame_start = Instant::now();
        }

        {
            raw_input.time = start_time.elapsed().as_nanos() as f64 * 1e-9;
            raw_input.seconds_since_midnight = Some(emigui_glium::local_time_of_day());
            raw_input.scroll_delta = vec2(0.0, 0.0);
            raw_input.events.clear();
            events_loop.poll_events(|event| {
                emigui_glium::input_event(event, clipboard.as_mut(), &mut raw_input, &mut running)
            });
        }

        let emigui_start = Instant::now();
        ctx.begin_frame(raw_input.clone()); // TODO: avoid clone
        let mut ui = ctx.fullscreen_ui();
        example_app.ui(&mut ui, "");
        let mut ui = ui.centered_column(ui.available().width().min(480.0));
        ui.set_layout(Layout::vertical(Align::Min));
        ui.add(label!("Emigui running inside of Glium").text_style(TextStyle::Heading));
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

        let (output, paint_batches) = ctx.end_frame();

        frame_times.add(
            raw_input.time,
            (Instant::now() - emigui_start).as_secs_f64() as f32,
        );

        painter.paint_batches(&display, paint_batches, ctx.texture());
        emigui_glium::handle_output(output, &display, clipboard.as_mut());
    }

    // Save state to disk:
    window_settings.pos = display
        .gl_window()
        .get_position()
        .map(|p| pos2(p.x as f32, p.y as f32));
    window_settings.size = display
        .gl_window()
        .get_inner_size()
        .map(|size| vec2(size.width as f32, size.height as f32));

    if let Err(err) = emigui_glium::write_memory(&ctx, memory_path) {
        eprintln!("ERROR: Failed to save emigui state: {}", err);
    }

    serde_json::to_writer_pretty(
        std::fs::File::create(settings_json_path).unwrap(),
        &window_settings,
    )
    .unwrap();
}

fn profile<R>(name: &str, action: impl FnOnce() -> R) -> R {
    let start = Instant::now();
    let r = action();
    let elapsed = start.elapsed();
    eprintln!("{}: {} ms", name, elapsed.as_millis());
    r
}
