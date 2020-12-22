use std::time::Instant;

use crate::{storage::WindowSettings, *};

pub use egui::{
    app::{self, App, Storage},
    Srgba,
};

const EGUI_MEMORY_KEY: &str = "egui";
const WINDOW_KEY: &str = "window";

impl egui::app::TextureAllocator for Painter {
    fn alloc(&mut self) -> egui::TextureId {
        self.alloc_user_texture()
    }

    fn set_srgba_premultiplied(
        &mut self,
        id: egui::TextureId,
        size: (usize, usize),
        srgba_pixels: &[Srgba],
    ) {
        self.set_user_texture(id, size, srgba_pixels);
    }

    fn free(&mut self, id: egui::TextureId) {
        self.free_user_texture(id)
    }
}

struct RequestRepaintEvent;

struct GliumRepaintSignal(glutin::event_loop::EventLoopProxy<RequestRepaintEvent>);

impl egui::app::RepaintSignal for GliumRepaintSignal {
    fn request_repaint(&self) {
        self.0.send_event(RequestRepaintEvent).ok();
    }
}

fn create_display(
    title: &str,
    window_settings: Option<WindowSettings>,
    is_resizable: bool,
    event_loop: &glutin::event_loop::EventLoop<RequestRepaintEvent>,
) -> glium::Display {
    let mut window_builder = glutin::window::WindowBuilder::new()
        .with_decorations(true)
        .with_resizable(is_resizable)
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

    let display = glium::Display::new(window_builder, context_builder, &event_loop).unwrap();

    if let Some(window_settings) = &window_settings {
        window_settings.restore_positions(&display);
    }

    display
}

fn create_storage(app_name: &str) -> Option<Box<dyn egui::app::Storage>> {
    if let Some(proj_dirs) = directories_next::ProjectDirs::from("", "", app_name) {
        let data_dir = proj_dirs.data_dir().to_path_buf();
        if let Err(err) = std::fs::create_dir_all(&data_dir) {
            eprintln!(
                "Saving disabled: Failed to create app path at {:?}: {}",
                data_dir, err
            );
            None
        } else {
            let mut config_dir = data_dir;
            config_dir.push("app.json");
            let storage = crate::storage::FileStorage::from_path(config_dir);
            Some(Box::new(storage))
        }
    } else {
        eprintln!("Saving disabled: Failed to find path to data_dir.");
        None
    }
}

/// Run an egui app
pub fn run(mut app: Box<dyn App>) -> ! {
    let mut storage = create_storage(app.name());

    if let Some(storage) = &mut storage {
        app.load(storage.as_ref());
    }

    let window_settings: Option<WindowSettings> = storage
        .as_mut()
        .and_then(|storage| egui::app::get_value(storage.as_ref(), WINDOW_KEY));
    let event_loop = glutin::event_loop::EventLoop::with_user_event();
    let display = create_display(app.name(), window_settings, app.is_resizable(), &event_loop);

    let repaint_signal = std::sync::Arc::new(GliumRepaintSignal(event_loop.create_proxy()));

    let mut ctx = egui::CtxRef::default();
    *ctx.memory() = storage
        .as_mut()
        .and_then(|storage| egui::app::get_value(storage.as_ref(), EGUI_MEMORY_KEY))
        .unwrap_or_default();
    app.setup(&ctx);

    let mut input_state = GliumInputState::from_pixels_per_point(native_pixels_per_point(&display));

    let start_time = Instant::now();
    let mut previous_frame_time = None;
    let mut painter = Painter::new(&display);
    let mut clipboard = init_clipboard();

    let mut last_auto_save = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        let mut redraw = || {
            let frame_start = Instant::now();
            input_state.raw.time = Some(start_time.elapsed().as_nanos() as f64 * 1e-9);
            input_state.raw.screen_rect = Some(Rect::from_min_size(
                Default::default(),
                screen_size_in_pixels(&display) / input_state.raw.pixels_per_point.unwrap(),
            ));

            ctx.begin_frame(input_state.raw.take());
            let mut integration_context = egui::app::IntegrationContext {
                info: egui::app::IntegrationInfo {
                    web_info: None,
                    cpu_usage: previous_frame_time,
                    seconds_since_midnight: Some(seconds_since_midnight()),
                    native_pixels_per_point: Some(native_pixels_per_point(&display)),
                },
                tex_allocator: Some(&mut painter),
                output: Default::default(),
                repaint_signal: repaint_signal.clone(),
            };
            app.ui(&ctx, &mut integration_context);
            let app_output = integration_context.output;
            let (egui_output, paint_commands) = ctx.end_frame();
            let paint_jobs = ctx.tesselate(paint_commands);

            let frame_time = (Instant::now() - frame_start).as_secs_f64() as f32;
            previous_frame_time = Some(frame_time);
            painter.paint_jobs(
                &display,
                ctx.pixels_per_point(),
                app.clear_color(),
                paint_jobs,
                &ctx.texture(),
            );

            {
                let egui::app::AppOutput {
                    quit,
                    window_size,
                    pixels_per_point,
                } = app_output;

                if let Some(pixels_per_point) = pixels_per_point {
                    // User changed GUI scale
                    input_state.raw.pixels_per_point = Some(pixels_per_point);
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

            if let Some(storage) = &mut storage {
                let now = Instant::now();
                if now - last_auto_save > app.auto_save_interval() {
                    egui::app::set_value(
                        storage.as_mut(),
                        WINDOW_KEY,
                        &WindowSettings::from_display(&display),
                    );
                    egui::app::set_value(storage.as_mut(), EGUI_MEMORY_KEY, &*ctx.memory());
                    app.save(storage.as_mut());
                    storage.flush();
                    last_auto_save = now;
                }
            }
        };

        match event {
            // Platform-dependent event handlers to workaround a winit bug
            // See: https://github.com/rust-windowing/winit/issues/987
            // See: https://github.com/rust-windowing/winit/issues/1619
            glutin::event::Event::RedrawEventsCleared if cfg!(windows) => redraw(),
            glutin::event::Event::RedrawRequested(_) if !cfg!(windows) => redraw(),

            glutin::event::Event::WindowEvent { event, .. } => {
                input_to_egui(event, clipboard.as_mut(), &mut input_state, control_flow);
                display.gl_window().window().request_redraw(); // TODO: ask Egui if the events warrants a repaint instead
            }
            glutin::event::Event::LoopDestroyed => {
                app.on_exit();
                if let Some(storage) = &mut storage {
                    egui::app::set_value(
                        storage.as_mut(),
                        WINDOW_KEY,
                        &WindowSettings::from_display(&display),
                    );
                    egui::app::set_value(storage.as_mut(), EGUI_MEMORY_KEY, &*ctx.memory());
                    app.save(storage.as_mut());
                    storage.flush();
                }
            }

            glutin::event::Event::UserEvent(RequestRepaintEvent) => {
                display.gl_window().window().request_redraw();
            }

            _ => (),
        }
    });
}
