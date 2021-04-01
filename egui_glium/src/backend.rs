use crate::{window_settings::WindowSettings, *};
use egui::Color32;
use std::time::Instant;

#[cfg(feature = "persistence")]
const EGUI_MEMORY_KEY: &str = "egui";
#[cfg(feature = "persistence")]
const WINDOW_KEY: &str = "window";

#[cfg(feature = "persistence")]
fn deserialize_window_settings(storage: &Option<Box<dyn epi::Storage>>) -> Option<WindowSettings> {
    epi::get_value(&**storage.as_ref()?, WINDOW_KEY)
}

#[cfg(not(feature = "persistence"))]
fn deserialize_window_settings(_: &Option<Box<dyn epi::Storage>>) -> Option<WindowSettings> {
    None
}

#[cfg(feature = "persistence")]
fn deserialize_memory(storage: &Option<Box<dyn epi::Storage>>) -> Option<egui::Memory> {
    epi::get_value(&**storage.as_ref()?, EGUI_MEMORY_KEY)
}

#[cfg(not(feature = "persistence"))]
fn deserialize_memory(_: &Option<Box<dyn epi::Storage>>) -> Option<egui::Memory> {
    None
}

impl epi::TextureAllocator for Painter {
    fn alloc_srgba_premultiplied(
        &mut self,
        size: (usize, usize),
        srgba_pixels: &[Color32],
    ) -> egui::TextureId {
        let id = self.alloc_user_texture();
        self.set_user_texture(id, size, srgba_pixels);
        id
    }

    fn free(&mut self, id: egui::TextureId) {
        self.free_user_texture(id)
    }
}

struct RequestRepaintEvent;

struct GliumRepaintSignal(
    std::sync::Mutex<glutin::event_loop::EventLoopProxy<RequestRepaintEvent>>,
);

impl epi::RepaintSignal for GliumRepaintSignal {
    fn request_repaint(&self) {
        self.0.lock().unwrap().send_event(RequestRepaintEvent).ok();
    }
}

fn create_display(
    app: &dyn epi::App,
    window_settings: Option<WindowSettings>,
    window_icon: Option<glutin::window::Icon>,
    event_loop: &glutin::event_loop::EventLoop<RequestRepaintEvent>,
) -> glium::Display {
    let mut window_builder = glutin::window::WindowBuilder::new()
        .with_decorations(app.decorated())
        .with_resizable(app.is_resizable())
        .with_title(app.name())
        .with_window_icon(window_icon)
        .with_transparent(app.transparent());

    let initial_size_points = app.initial_window_size();

    if let Some(window_settings) = &window_settings {
        window_builder = window_settings.initialize_size(window_builder);
    } else if let Some(initial_size_points) = initial_size_points {
        window_builder = window_builder.with_inner_size(glutin::dpi::LogicalSize {
            width: initial_size_points.x as f64,
            height: initial_size_points.y as f64,
        });
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

#[cfg(not(feature = "persistence"))]
fn create_storage(_app_name: &str) -> Option<Box<dyn epi::Storage>> {
    None
}

#[cfg(feature = "persistence")]
fn create_storage(app_name: &str) -> Option<Box<dyn epi::Storage>> {
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
            let storage = crate::persistence::FileStorage::from_path(config_dir);
            Some(Box::new(storage))
        }
    } else {
        eprintln!("Saving disabled: Failed to find path to data_dir.");
        None
    }
}

fn integration_info(
    display: &glium::Display,
    previous_frame_time: Option<f32>,
) -> epi::IntegrationInfo {
    epi::IntegrationInfo {
        web_info: None,
        cpu_usage: previous_frame_time,
        seconds_since_midnight: seconds_since_midnight(),
        native_pixels_per_point: Some(native_pixels_per_point(&display)),
    }
}

fn load_icon(icon_data: Option<epi::IconData>) -> Option<glutin::window::Icon> {
    let icon_data = icon_data?;
    glutin::window::Icon::from_rgba(icon_data.rgba, icon_data.width, icon_data.height).ok()
}

/// Run an egui app
pub fn run(mut app: Box<dyn epi::App>) -> ! {
    let mut storage = create_storage(app.name());

    if let Some(storage) = &mut storage {
        app.load(storage.as_ref());
    }

    let window_settings = deserialize_window_settings(&storage);
    let event_loop = glutin::event_loop::EventLoop::with_user_event();
    let icon = load_icon(app.icon_data());
    let display = create_display(&*app, window_settings, icon, &event_loop);

    let repaint_signal = std::sync::Arc::new(GliumRepaintSignal(std::sync::Mutex::new(
        event_loop.create_proxy(),
    )));

    let mut ctx = egui::CtxRef::default();
    *ctx.memory() = deserialize_memory(&storage).unwrap_or_default();

    app.setup(&ctx);

    let mut input_state = GliumInputState::from_pixels_per_point(native_pixels_per_point(&display));

    let start_time = Instant::now();
    let mut previous_frame_time = None;
    let mut painter = Painter::new(&display);
    let mut clipboard = init_clipboard();
    let mut current_cursor_icon = CursorIcon::Default;

    #[cfg(feature = "persistence")]
    let mut last_auto_save = Instant::now();

    #[cfg(feature = "http")]
    let http = std::sync::Arc::new(crate::http::GliumHttp {});

    let mut screen_reader = crate::screen_reader::ScreenReader::default();

    if app.warm_up_enabled() {
        // let warm_up_start = Instant::now();
        input_state.raw.time = Some(0.0);
        input_state.raw.screen_rect = Some(Rect::from_min_size(
            Default::default(),
            screen_size_in_pixels(&display) / input_state.raw.pixels_per_point.unwrap(),
        ));
        ctx.begin_frame(input_state.raw.take());
        let mut app_output = epi::backend::AppOutput::default();
        let mut frame = epi::backend::FrameBuilder {
            info: integration_info(&display, None),
            tex_allocator: &mut painter,
            #[cfg(feature = "http")]
            http: http.clone(),
            output: &mut app_output,
            repaint_signal: repaint_signal.clone(),
        }
        .build();

        let saved_memory = ctx.memory().clone();
        ctx.memory().set_everything_is_visible(true);
        app.update(&ctx, &mut frame);
        *ctx.memory() = saved_memory; // We don't want to remember that windows were huge.
        ctx.clear_animations();

        let (egui_output, _shapes) = ctx.end_frame();

        set_cursor_icon(&display, egui_output.cursor_icon);
        current_cursor_icon = egui_output.cursor_icon;
        handle_output(egui_output, clipboard.as_mut(), &display);

        // TODO: handle app_output
        // eprintln!("Warmed up in {} ms", warm_up_start.elapsed().as_millis())
    }

    event_loop.run(move |event, _, control_flow| {
        let mut redraw = || {
            let pixels_per_point = input_state
                .raw
                .pixels_per_point
                .unwrap_or_else(|| ctx.pixels_per_point());

            let frame_start = Instant::now();
            input_state.raw.time = Some(start_time.elapsed().as_nanos() as f64 * 1e-9);
            input_state.raw.screen_rect = Some(Rect::from_min_size(
                Default::default(),
                screen_size_in_pixels(&display) / pixels_per_point,
            ));

            ctx.begin_frame(input_state.raw.take());
            let mut app_output = epi::backend::AppOutput::default();
            let mut frame = epi::backend::FrameBuilder {
                info: integration_info(&display, previous_frame_time),
                tex_allocator: &mut painter,
                #[cfg(feature = "http")]
                http: http.clone(),
                output: &mut app_output,
                repaint_signal: repaint_signal.clone(),
            }
            .build();
            app.update(&ctx, &mut frame);
            let (egui_output, shapes) = ctx.end_frame();
            let clipped_meshes = ctx.tessellate(shapes);

            let frame_time = (Instant::now() - frame_start).as_secs_f64() as f32;
            previous_frame_time = Some(frame_time);
            painter.paint_meshes(
                &display,
                ctx.pixels_per_point(),
                app.clear_color(),
                clipped_meshes,
                &ctx.texture(),
            );

            {
                let epi::backend::AppOutput { quit, window_size } = app_output;

                if let Some(window_size) = window_size {
                    display.gl_window().window().set_inner_size(
                        glutin::dpi::PhysicalSize {
                            width: (ctx.pixels_per_point() * window_size.x).round(),
                            height: (ctx.pixels_per_point() * window_size.y).round(),
                        }
                        .to_logical::<f32>(native_pixels_per_point(&display) as f64),
                    );
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

            if ctx.memory().options.screen_reader {
                screen_reader.speak(&egui_output.events_description());
            }
            if current_cursor_icon != egui_output.cursor_icon {
                // call only when changed to prevent flickering near frame boundary
                // when Windows OS tries to control cursor icon for window resizing
                set_cursor_icon(&display, egui_output.cursor_icon);
                current_cursor_icon = egui_output.cursor_icon;
            }
            handle_output(egui_output, clipboard.as_mut(), &display);

            #[cfg(feature = "persistence")]
            if let Some(storage) = &mut storage {
                let now = Instant::now();
                if now - last_auto_save > app.auto_save_interval() {
                    epi::set_value(
                        storage.as_mut(),
                        WINDOW_KEY,
                        &WindowSettings::from_display(&display),
                    );
                    epi::set_value(storage.as_mut(), EGUI_MEMORY_KEY, &*ctx.memory());
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
                input_to_egui(
                    ctx.pixels_per_point(),
                    event,
                    clipboard.as_mut(),
                    &mut input_state,
                    control_flow,
                );
                display.gl_window().window().request_redraw(); // TODO: ask egui if the events warrants a repaint instead
            }
            glutin::event::Event::LoopDestroyed => {
                app.on_exit();
                #[cfg(feature = "persistence")]
                if let Some(storage) = &mut storage {
                    epi::set_value(
                        storage.as_mut(),
                        WINDOW_KEY,
                        &WindowSettings::from_display(&display),
                    );
                    epi::set_value(storage.as_mut(), EGUI_MEMORY_KEY, &*ctx.memory());
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
