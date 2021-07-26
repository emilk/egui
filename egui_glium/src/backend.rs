use crate::{window_settings::WindowSettings, *};
use egui::Color32;
#[cfg(target_os = "windows")]
use glium::glutin::platform::windows::WindowBuilderExtWindows;
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

#[cfg(target_os = "windows")]
fn window_builder_drag_and_drop(
    window_builder: glutin::window::WindowBuilder,
    enable: bool,
) -> glutin::window::WindowBuilder {
    window_builder.with_drag_and_drop(enable)
}

#[cfg(not(target_os = "windows"))]
fn window_builder_drag_and_drop(
    window_builder: glutin::window::WindowBuilder,
    _enable: bool,
) -> glutin::window::WindowBuilder {
    // drag and drop can only be disabled on windows
    window_builder
}

fn create_display(
    app: &dyn epi::App,
    native_options: &epi::NativeOptions,
    window_settings: Option<WindowSettings>,
    window_icon: Option<glutin::window::Icon>,
    event_loop: &glutin::event_loop::EventLoop<RequestRepaintEvent>,
) -> glium::Display {
    let mut window_builder = glutin::window::WindowBuilder::new()
        .with_always_on_top(native_options.always_on_top)
        .with_decorations(native_options.decorated)
        .with_resizable(native_options.resizable)
        .with_title(app.name())
        .with_transparent(native_options.transparent)
        .with_window_icon(window_icon);

    window_builder =
        window_builder_drag_and_drop(window_builder, native_options.drag_and_drop_support);

    let initial_size_points = native_options.initial_window_size;

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

    let display = glium::Display::new(window_builder, context_builder, event_loop).unwrap();

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
            config_dir.push("app.ron");
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
        prefer_dark_mode: None, // TODO: figure out system default
        cpu_usage: previous_frame_time,
        seconds_since_midnight: seconds_since_midnight(),
        native_pixels_per_point: Some(native_pixels_per_point(display)),
    }
}

fn load_icon(icon_data: epi::IconData) -> Option<glutin::window::Icon> {
    glutin::window::Icon::from_rgba(icon_data.rgba, icon_data.width, icon_data.height).ok()
}

// ----------------------------------------------------------------------------

/// Run an egui app
pub fn run(mut app: Box<dyn epi::App>, nativve_options: epi::NativeOptions) -> ! {
    #[allow(unused_mut)]
    let mut storage = create_storage(app.name());

    #[cfg(feature = "http")]
    let http = std::sync::Arc::new(crate::http::GliumHttp {});

    let window_settings = deserialize_window_settings(&storage);
    let event_loop = glutin::event_loop::EventLoop::with_user_event();
    let icon = nativve_options.icon_data.clone().and_then(load_icon);
    let display = create_display(&*app, &nativve_options, window_settings, icon, &event_loop);

    let repaint_signal = std::sync::Arc::new(GliumRepaintSignal(std::sync::Mutex::new(
        event_loop.create_proxy(),
    )));

    let mut egui = EguiGlium::new(&display);
    *egui.ctx().memory() = deserialize_memory(&storage).unwrap_or_default();

    {
        let (ctx, painter) = egui.ctx_and_painter_mut();
        let mut app_output = epi::backend::AppOutput::default();
        let mut frame = epi::backend::FrameBuilder {
            info: integration_info(&display, None),
            tex_allocator: painter,
            #[cfg(feature = "http")]
            http: http.clone(),
            output: &mut app_output,
            repaint_signal: repaint_signal.clone(),
        }
        .build();
        app.setup(ctx, &mut frame, storage.as_deref());
    }

    let mut previous_frame_time = None;

    let mut is_focused = true;

    #[cfg(feature = "persistence")]
    let mut last_auto_save = Instant::now();

    if app.warm_up_enabled() {
        let saved_memory = egui.ctx().memory().clone();
        egui.ctx().memory().set_everything_is_visible(true);

        egui.begin_frame(&display);
        let (ctx, painter) = egui.ctx_and_painter_mut();
        let mut app_output = epi::backend::AppOutput::default();
        let mut frame = epi::backend::FrameBuilder {
            info: integration_info(&display, None),
            tex_allocator: painter,
            #[cfg(feature = "http")]
            http: http.clone(),
            output: &mut app_output,
            repaint_signal: repaint_signal.clone(),
        }
        .build();

        app.update(ctx, &mut frame);

        let _ = egui.end_frame(&display);

        *egui.ctx().memory() = saved_memory; // We don't want to remember that windows were huge.
        egui.ctx().clear_animations();

        // TODO: handle app_output
        // eprintln!("Warmed up in {} ms", warm_up_start.elapsed().as_millis())
    }

    event_loop.run(move |event, _, control_flow| {
        let mut redraw = || {
            if !is_focused {
                // On Mac, a minimized Window uses up all CPU: https://github.com/emilk/egui/issues/325
                // We can't know if we are minimized: https://github.com/rust-windowing/winit/issues/208
                // But we know if we are focused (in foreground). When minimized, we are not focused.
                // However, a user may want an egui with an animation in the background,
                // so we still need to repaint quite fast.
                std::thread::sleep(std::time::Duration::from_millis(10));
            }

            let frame_start = std::time::Instant::now();

            egui.begin_frame(&display);
            let (ctx, painter) = egui.ctx_and_painter_mut();
            let mut app_output = epi::backend::AppOutput::default();
            let mut frame = epi::backend::FrameBuilder {
                info: integration_info(&display, previous_frame_time),
                tex_allocator: painter,
                #[cfg(feature = "http")]
                http: http.clone(),
                output: &mut app_output,
                repaint_signal: repaint_signal.clone(),
            }
            .build();
            app.update(ctx, &mut frame);
            let (needs_repaint, shapes) = egui.end_frame(&display);

            let frame_time = (Instant::now() - frame_start).as_secs_f64() as f32;
            previous_frame_time = Some(frame_time);

            {
                use glium::Surface as _;
                let mut target = display.draw();
                let clear_color = app.clear_color();
                target.clear_color(
                    clear_color[0],
                    clear_color[1],
                    clear_color[2],
                    clear_color[3],
                );
                egui.paint(&display, &mut target, shapes);
                target.finish().unwrap();
            }

            {
                let epi::backend::AppOutput { quit, window_size } = app_output;

                if let Some(window_size) = window_size {
                    display.gl_window().window().set_inner_size(
                        glutin::dpi::PhysicalSize {
                            width: (egui.ctx().pixels_per_point() * window_size.x).round(),
                            height: (egui.ctx().pixels_per_point() * window_size.y).round(),
                        }
                        .to_logical::<f32>(native_pixels_per_point(&display) as f64),
                    );
                }

                *control_flow = if quit {
                    glutin::event_loop::ControlFlow::Exit
                } else if needs_repaint {
                    display.gl_window().window().request_redraw();
                    glutin::event_loop::ControlFlow::Poll
                } else {
                    glutin::event_loop::ControlFlow::Wait
                };
            }

            #[cfg(feature = "persistence")]
            if let Some(storage) = &mut storage {
                let now = Instant::now();
                if now - last_auto_save > app.auto_save_interval() {
                    epi::set_value(
                        storage.as_mut(),
                        WINDOW_KEY,
                        &WindowSettings::from_display(&display),
                    );
                    epi::set_value(storage.as_mut(), EGUI_MEMORY_KEY, &*egui.ctx().memory());
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
                if egui.is_quit_event(&event) {
                    *control_flow = glium::glutin::event_loop::ControlFlow::Exit;
                }

                if let glutin::event::WindowEvent::Focused(new_focused) = event {
                    is_focused = new_focused;
                }

                egui.on_event(&event);

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
                    epi::set_value(storage.as_mut(), EGUI_MEMORY_KEY, &*egui.ctx().memory());
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
