use crate::{epi, Theme, WindowInfo};
use egui_winit::{native_pixels_per_point, WindowSettings};
use winit::event_loop::EventLoopWindowTarget;

pub fn points_to_size(points: egui::Vec2) -> winit::dpi::LogicalSize<f64> {
    winit::dpi::LogicalSize {
        width: points.x as f64,
        height: points.y as f64,
    }
}

pub fn read_window_info(window: &winit::window::Window, pixels_per_point: f32) -> WindowInfo {
    let position = window
        .outer_position()
        .ok()
        .map(|pos| pos.to_logical::<f32>(pixels_per_point.into()))
        .map(|pos| egui::Pos2 { x: pos.x, y: pos.y });

    let size = window
        .inner_size()
        .to_logical::<f32>(pixels_per_point.into());

    WindowInfo {
        position,
        fullscreen: window.fullscreen().is_some(),
        size: egui::Vec2 {
            x: size.width,
            y: size.height,
        },
    }
}

pub fn window_builder(
    native_options: &epi::NativeOptions,
    window_settings: &Option<WindowSettings>,
) -> winit::window::WindowBuilder {
    let epi::NativeOptions {
        always_on_top,
        maximized,
        decorated,
        fullscreen,
        drag_and_drop_support,
        icon_data,
        initial_window_pos,
        initial_window_size,
        min_window_size,
        max_window_size,
        resizable,
        transparent,
        ..
    } = native_options;

    let window_icon = icon_data.clone().and_then(load_icon);

    let mut window_builder = winit::window::WindowBuilder::new()
        .with_always_on_top(*always_on_top)
        .with_decorations(*decorated)
        .with_fullscreen(fullscreen.then(|| winit::window::Fullscreen::Borderless(None)))
        .with_maximized(*maximized)
        .with_resizable(*resizable)
        .with_transparent(*transparent)
        .with_window_icon(window_icon);

    if let Some(min_size) = *min_window_size {
        window_builder = window_builder.with_min_inner_size(points_to_size(min_size));
    }
    if let Some(max_size) = *max_window_size {
        window_builder = window_builder.with_max_inner_size(points_to_size(max_size));
    }

    window_builder = window_builder_drag_and_drop(window_builder, *drag_and_drop_support);

    if let Some(window_settings) = window_settings {
        window_builder = window_settings.initialize_window(window_builder);
    } else {
        if let Some(pos) = *initial_window_pos {
            window_builder = window_builder.with_position(winit::dpi::PhysicalPosition {
                x: pos.x as f64,
                y: pos.y as f64,
            });
        }
        if let Some(initial_window_size) = *initial_window_size {
            window_builder = window_builder.with_inner_size(points_to_size(initial_window_size));
        }
    }

    window_builder
}

fn load_icon(icon_data: epi::IconData) -> Option<winit::window::Icon> {
    winit::window::Icon::from_rgba(icon_data.rgba, icon_data.width, icon_data.height).ok()
}

#[cfg(target_os = "windows")]
fn window_builder_drag_and_drop(
    window_builder: winit::window::WindowBuilder,
    enable: bool,
) -> winit::window::WindowBuilder {
    use winit::platform::windows::WindowBuilderExtWindows as _;
    window_builder.with_drag_and_drop(enable)
}

#[cfg(not(target_os = "windows"))]
fn window_builder_drag_and_drop(
    window_builder: winit::window::WindowBuilder,
    _enable: bool,
) -> winit::window::WindowBuilder {
    // drag and drop can only be disabled on windows
    window_builder
}

pub fn handle_app_output(
    window: &winit::window::Window,
    current_pixels_per_point: f32,
    app_output: epi::backend::AppOutput,
) {
    let epi::backend::AppOutput {
        close: _,
        window_size,
        window_title,
        decorated,
        fullscreen,
        drag_window,
        window_pos,
        visible,
    } = app_output;

    if let Some(decorated) = decorated {
        window.set_decorations(decorated);
    }

    if let Some(window_size) = window_size {
        window.set_inner_size(
            winit::dpi::PhysicalSize {
                width: (current_pixels_per_point * window_size.x).round(),
                height: (current_pixels_per_point * window_size.y).round(),
            }
            .to_logical::<f32>(native_pixels_per_point(window) as f64),
        );
    }

    if let Some(fullscreen) = fullscreen {
        window.set_fullscreen(fullscreen.then(|| winit::window::Fullscreen::Borderless(None)));
    }

    if let Some(window_title) = window_title {
        window.set_title(&window_title);
    }

    if let Some(window_pos) = window_pos {
        window.set_outer_position(winit::dpi::PhysicalPosition {
            x: window_pos.x as f64,
            y: window_pos.y as f64,
        });
    }

    if drag_window {
        let _ = window.drag_window();
    }

    if let Some(visible) = visible {
        window.set_visible(visible);
    }
}

// ----------------------------------------------------------------------------

/// For loading/saving app state and/or egui memory to disk.
pub fn create_storage(_app_name: &str) -> Option<Box<dyn epi::Storage>> {
    #[cfg(feature = "persistence")]
    if let Some(storage) = super::file_storage::FileStorage::from_app_name(_app_name) {
        return Some(Box::new(storage));
    }
    None
}

// ----------------------------------------------------------------------------

/// Everything needed to make a winit-based integration for [`epi`].
pub struct EpiIntegration {
    pub frame: epi::Frame,
    last_auto_save: std::time::Instant,
    pub egui_ctx: egui::Context,
    pending_full_output: egui::FullOutput,
    egui_winit: egui_winit::State,
    /// When set, it is time to close the native window.
    close: bool,
    can_drag_window: bool,
}

impl EpiIntegration {
    pub fn new<E>(
        event_loop: &EventLoopWindowTarget<E>,
        max_texture_side: usize,
        window: &winit::window::Window,
        system_theme: Option<Theme>,
        storage: Option<Box<dyn epi::Storage>>,
        #[cfg(feature = "glow")] gl: Option<std::sync::Arc<glow::Context>>,
        #[cfg(feature = "wgpu")] wgpu_render_state: Option<egui_wgpu::RenderState>,
    ) -> Self {
        let egui_ctx = egui::Context::default();

        *egui_ctx.memory() = load_egui_memory(storage.as_deref()).unwrap_or_default();

        let frame = epi::Frame {
            info: epi::IntegrationInfo {
                system_theme,
                cpu_usage: None,
                native_pixels_per_point: Some(native_pixels_per_point(window)),
                window_info: read_window_info(window, egui_ctx.pixels_per_point()),
            },
            output: Default::default(),
            storage,
            #[cfg(feature = "glow")]
            gl,
            #[cfg(feature = "wgpu")]
            wgpu_render_state,
        };

        let mut egui_winit = egui_winit::State::new(event_loop);
        egui_winit.set_max_texture_side(max_texture_side);
        let pixels_per_point = window.scale_factor() as f32;
        egui_winit.set_pixels_per_point(pixels_per_point);

        Self {
            frame,
            last_auto_save: std::time::Instant::now(),
            egui_ctx,
            egui_winit,
            pending_full_output: Default::default(),
            close: false,
            can_drag_window: false,
        }
    }

    pub fn warm_up(&mut self, app: &mut dyn epi::App, window: &winit::window::Window) {
        crate::profile_function!();
        let saved_memory: egui::Memory = self.egui_ctx.memory().clone();
        self.egui_ctx.memory().set_everything_is_visible(true);
        let full_output = self.update(app, window);
        self.pending_full_output.append(full_output); // Handle it next frame
        *self.egui_ctx.memory() = saved_memory; // We don't want to remember that windows were huge.
        self.egui_ctx.clear_animations();
    }

    /// If `true`, it is time to close the native window.
    pub fn should_close(&self) -> bool {
        self.close
    }

    pub fn on_event(&mut self, app: &mut dyn epi::App, event: &winit::event::WindowEvent<'_>) {
        use winit::event::{ElementState, MouseButton, WindowEvent};

        match event {
            WindowEvent::CloseRequested => self.close = app.on_close_event(),
            WindowEvent::Destroyed => self.close = true,
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Pressed,
                ..
            } => self.can_drag_window = true,
            _ => {}
        }

        self.egui_winit.on_event(&self.egui_ctx, event);
    }

    pub fn update(
        &mut self,
        app: &mut dyn epi::App,
        window: &winit::window::Window,
    ) -> egui::FullOutput {
        let frame_start = std::time::Instant::now();

        self.frame.info.window_info = read_window_info(window, self.egui_ctx.pixels_per_point());
        let raw_input = self.egui_winit.take_egui_input(window);
        let full_output = self.egui_ctx.run(raw_input, |egui_ctx| {
            crate::profile_scope!("App::update");
            app.update(egui_ctx, &mut self.frame);
        });
        self.pending_full_output.append(full_output);
        let full_output = std::mem::take(&mut self.pending_full_output);

        {
            let mut app_output = self.frame.take_app_output();
            app_output.drag_window &= self.can_drag_window; // Necessary on Windows; see https://github.com/emilk/egui/pull/1108
            self.can_drag_window = false;
            if app_output.close {
                self.close = app.on_close_event();
            }
            handle_app_output(window, self.egui_ctx.pixels_per_point(), app_output);
        }

        let frame_time = (std::time::Instant::now() - frame_start).as_secs_f64() as f32;
        self.frame.info.cpu_usage = Some(frame_time);

        full_output
    }

    pub fn post_rendering(&mut self, app: &mut dyn epi::App, window: &winit::window::Window) {
        let inner_size = window.inner_size();
        let window_size_px = [inner_size.width, inner_size.height];

        app.post_rendering(window_size_px, &self.frame);
    }

    pub fn handle_platform_output(
        &mut self,
        window: &winit::window::Window,
        platform_output: egui::PlatformOutput,
    ) {
        self.egui_winit
            .handle_platform_output(window, &self.egui_ctx, platform_output);
    }

    // ------------------------------------------------------------------------
    // Persistance stuff:

    pub fn maybe_autosave(&mut self, app: &mut dyn epi::App, window: &winit::window::Window) {
        let now = std::time::Instant::now();
        if now - self.last_auto_save > app.auto_save_interval() {
            self.save(app, window);
            self.last_auto_save = now;
        }
    }

    pub fn save(&mut self, _app: &mut dyn epi::App, _window: &winit::window::Window) {
        #[cfg(feature = "persistence")]
        if let Some(storage) = self.frame.storage_mut() {
            crate::profile_function!();

            if _app.persist_native_window() {
                crate::profile_scope!("native_window");
                epi::set_value(
                    storage,
                    STORAGE_WINDOW_KEY,
                    &WindowSettings::from_display(_window),
                );
            }
            if _app.persist_egui_memory() {
                crate::profile_scope!("egui_memory");
                epi::set_value(storage, STORAGE_EGUI_MEMORY_KEY, &*self.egui_ctx.memory());
            }
            {
                crate::profile_scope!("App::save");
                _app.save(storage);
            }

            crate::profile_scope!("Storage::flush");
            storage.flush();
        }
    }
}

#[cfg(feature = "persistence")]
const STORAGE_EGUI_MEMORY_KEY: &str = "egui";

#[cfg(feature = "persistence")]
const STORAGE_WINDOW_KEY: &str = "window";

pub fn load_window_settings(_storage: Option<&dyn epi::Storage>) -> Option<WindowSettings> {
    #[cfg(feature = "persistence")]
    {
        epi::get_value(_storage?, STORAGE_WINDOW_KEY)
    }
    #[cfg(not(feature = "persistence"))]
    None
}

pub fn load_egui_memory(_storage: Option<&dyn epi::Storage>) -> Option<egui::Memory> {
    #[cfg(feature = "persistence")]
    {
        epi::get_value(_storage?, STORAGE_EGUI_MEMORY_KEY)
    }
    #[cfg(not(feature = "persistence"))]
    None
}
