use winit::event_loop::EventLoopWindowTarget;

#[cfg(target_os = "macos")]
use winit::platform::macos::WindowBuilderExtMacOS as _;

#[cfg(feature = "accesskit")]
use egui::accesskit;
use egui::NumExt as _;
#[cfg(feature = "accesskit")]
use egui_winit::accesskit_winit;
use egui_winit::{native_pixels_per_point, EventResponse, WindowSettings};

use crate::{epi, Theme, WindowInfo};

#[derive(Default)]
pub struct WindowState {
    // We cannot simply call `winit::Window::is_minimized/is_maximized`
    // because that deadlocks on mac.
    pub minimized: bool,
    pub maximized: bool,
}

pub fn points_to_size(points: egui::Vec2) -> winit::dpi::LogicalSize<f64> {
    winit::dpi::LogicalSize {
        width: points.x as f64,
        height: points.y as f64,
    }
}

pub fn read_window_info(
    window: &winit::window::Window,
    pixels_per_point: f32,
    window_state: &WindowState,
) -> WindowInfo {
    let position = window
        .outer_position()
        .ok()
        .map(|pos| pos.to_logical::<f32>(pixels_per_point.into()))
        .map(|pos| egui::Pos2 { x: pos.x, y: pos.y });

    let monitor = window.current_monitor().is_some();
    let monitor_size = if monitor {
        let size = window
            .current_monitor()
            .unwrap()
            .size()
            .to_logical::<f32>(pixels_per_point.into());
        Some(egui::vec2(size.width, size.height))
    } else {
        None
    };

    let size = window
        .inner_size()
        .to_logical::<f32>(pixels_per_point.into());

    // NOTE: calling window.is_minimized() or window.is_maximized() deadlocks on Mac.

    WindowInfo {
        position,
        fullscreen: window.fullscreen().is_some(),
        minimized: window_state.minimized,
        maximized: window_state.maximized,
        size: egui::Vec2 {
            x: size.width,
            y: size.height,
        },
        monitor_size,
    }
}

pub fn window_builder<E>(
    event_loop: &EventLoopWindowTarget<E>,
    title: &str,
    native_options: &epi::NativeOptions,
    window_settings: Option<WindowSettings>,
) -> winit::window::WindowBuilder {
    let epi::NativeOptions {
        maximized,
        decorated,
        fullscreen,
        #[cfg(target_os = "macos")]
        fullsize_content,
        drag_and_drop_support,
        icon_data,
        initial_window_pos,
        initial_window_size,
        min_window_size,
        max_window_size,
        resizable,
        transparent,
        centered,
        ..
    } = native_options;

    let window_icon = icon_data.clone().and_then(load_icon);

    let mut window_builder = winit::window::WindowBuilder::new()
        .with_title(title)
        .with_decorations(*decorated)
        .with_fullscreen(fullscreen.then(|| winit::window::Fullscreen::Borderless(None)))
        .with_maximized(*maximized)
        .with_resizable(*resizable)
        .with_transparent(*transparent)
        .with_window_icon(window_icon)
        // Keep hidden until we've painted something. See https://github.com/emilk/egui/pull/2279
        // We must also keep the window hidden until AccessKit is initialized.
        .with_visible(false);

    #[cfg(target_os = "macos")]
    if *fullsize_content {
        window_builder = window_builder
            .with_title_hidden(true)
            .with_titlebar_transparent(true)
            .with_fullsize_content_view(true);
    }

    if let Some(min_size) = *min_window_size {
        window_builder = window_builder.with_min_inner_size(points_to_size(min_size));
    }
    if let Some(max_size) = *max_window_size {
        window_builder = window_builder.with_max_inner_size(points_to_size(max_size));
    }

    window_builder = window_builder_drag_and_drop(window_builder, *drag_and_drop_support);

    let inner_size_points = if let Some(mut window_settings) = window_settings {
        // Restore pos/size from previous session
        window_settings.clamp_to_sane_values(largest_monitor_point_size(event_loop));
        #[cfg(windows)]
        window_settings.clamp_window_to_sane_position(&event_loop);
        window_builder = window_settings.initialize_window(window_builder);
        window_settings.inner_size_points()
    } else {
        if let Some(pos) = *initial_window_pos {
            window_builder = window_builder.with_position(winit::dpi::LogicalPosition {
                x: pos.x as f64,
                y: pos.y as f64,
            });
        }

        if let Some(initial_window_size) = *initial_window_size {
            let initial_window_size =
                initial_window_size.at_most(largest_monitor_point_size(event_loop));
            window_builder = window_builder.with_inner_size(points_to_size(initial_window_size));
        }

        *initial_window_size
    };

    if *centered {
        if let Some(monitor) = event_loop.available_monitors().next() {
            let monitor_size = monitor.size().to_logical::<f64>(monitor.scale_factor());
            let inner_size = inner_size_points.unwrap_or(egui::Vec2 { x: 800.0, y: 600.0 });
            if monitor_size.width > 0.0 && monitor_size.height > 0.0 {
                let x = (monitor_size.width - inner_size.x as f64) / 2.0;
                let y = (monitor_size.height - inner_size.y as f64) / 2.0;
                window_builder = window_builder.with_position(winit::dpi::LogicalPosition { x, y });
            }
        }
    }
    window_builder
}

pub fn apply_native_options_to_window(
    window: &winit::window::Window,
    native_options: &crate::NativeOptions,
) {
    use winit::window::WindowLevel;
    window.set_window_level(if native_options.always_on_top {
        WindowLevel::AlwaysOnTop
    } else {
        WindowLevel::Normal
    });
}

fn largest_monitor_point_size<E>(event_loop: &EventLoopWindowTarget<E>) -> egui::Vec2 {
    let mut max_size = egui::Vec2::ZERO;

    for monitor in event_loop.available_monitors() {
        let size = monitor.size().to_logical::<f32>(monitor.scale_factor());
        let size = egui::vec2(size.width, size.height);
        max_size = max_size.max(size);
    }

    if max_size == egui::Vec2::ZERO {
        egui::Vec2::splat(16000.0)
    } else {
        max_size
    }
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
    window_state: &mut WindowState,
) {
    let epi::backend::AppOutput {
        close: _,
        window_size,
        window_title,
        decorated,
        fullscreen,
        drag_window,
        window_pos,
        visible: _, // handled in post_present
        always_on_top,
        minimized,
        maximized,
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
        window.set_fullscreen(fullscreen.then_some(winit::window::Fullscreen::Borderless(None)));
    }

    if let Some(window_title) = window_title {
        window.set_title(&window_title);
    }

    if let Some(window_pos) = window_pos {
        window.set_outer_position(winit::dpi::LogicalPosition {
            x: window_pos.x as f64,
            y: window_pos.y as f64,
        });
    }

    if drag_window {
        let _ = window.drag_window();
    }

    if let Some(always_on_top) = always_on_top {
        use winit::window::WindowLevel;
        window.set_window_level(if always_on_top {
            WindowLevel::AlwaysOnTop
        } else {
            WindowLevel::Normal
        });
    }

    if let Some(minimized) = minimized {
        window.set_minimized(minimized);
        window_state.minimized = minimized;
    }

    if let Some(maximized) = maximized {
        window.set_maximized(maximized);
        window_state.maximized = maximized;
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
    window_state: WindowState,
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

        let memory = load_egui_memory(storage.as_deref()).unwrap_or_default();
        egui_ctx.memory_mut(|mem| *mem = memory);

        let native_pixels_per_point = window.scale_factor() as f32;

        let window_state = WindowState {
            minimized: window.is_minimized().unwrap_or(false),
            maximized: window.is_maximized(),
        };

        let frame = epi::Frame {
            info: epi::IntegrationInfo {
                system_theme,
                cpu_usage: None,
                native_pixels_per_point: Some(native_pixels_per_point),
                window_info: read_window_info(window, egui_ctx.pixels_per_point(), &window_state),
            },
            output: epi::backend::AppOutput {
                visible: Some(true),
                ..Default::default()
            },
            storage,
            #[cfg(feature = "glow")]
            gl,
            #[cfg(feature = "wgpu")]
            wgpu_render_state,
        };

        let mut egui_winit = egui_winit::State::new(event_loop);
        egui_winit.set_max_texture_side(max_texture_side);
        egui_winit.set_pixels_per_point(native_pixels_per_point);

        Self {
            frame,
            last_auto_save: std::time::Instant::now(),
            egui_ctx,
            egui_winit,
            pending_full_output: Default::default(),
            close: false,
            can_drag_window: false,
            window_state,
        }
    }

    #[cfg(feature = "accesskit")]
    pub fn init_accesskit<E: From<accesskit_winit::ActionRequestEvent> + Send>(
        &mut self,
        window: &winit::window::Window,
        event_loop_proxy: winit::event_loop::EventLoopProxy<E>,
    ) {
        let egui_ctx = self.egui_ctx.clone();
        self.egui_winit
            .init_accesskit(window, event_loop_proxy, move || {
                // This function is called when an accessibility client
                // (e.g. screen reader) makes its first request. If we got here,
                // we know that an accessibility tree is actually wanted.
                egui_ctx.enable_accesskit();
                // Enqueue a repaint so we'll receive a full tree update soon.
                egui_ctx.request_repaint();
                egui_ctx.accesskit_placeholder_tree_update()
            });
    }

    pub fn warm_up(&mut self, app: &mut dyn epi::App, window: &winit::window::Window) {
        crate::profile_function!();
        let saved_memory: egui::Memory = self.egui_ctx.memory(|mem| mem.clone());
        self.egui_ctx
            .memory_mut(|mem| mem.set_everything_is_visible(true));
        let full_output = self.update(app, window);
        self.pending_full_output.append(full_output); // Handle it next frame
        self.egui_ctx.memory_mut(|mem| *mem = saved_memory); // We don't want to remember that windows were huge.
        self.egui_ctx.clear_animations();
    }

    /// If `true`, it is time to close the native window.
    pub fn should_close(&self) -> bool {
        self.close
    }

    pub fn on_event(
        &mut self,
        app: &mut dyn epi::App,
        event: &winit::event::WindowEvent<'_>,
    ) -> EventResponse {
        use winit::event::{ElementState, MouseButton, WindowEvent};

        match event {
            WindowEvent::CloseRequested => {
                tracing::debug!("Received WindowEvent::CloseRequested");
                self.close = app.on_close_event();
                tracing::debug!("App::on_close_event returned {}", self.close);
            }
            WindowEvent::Destroyed => {
                tracing::debug!("Received WindowEvent::Destroyed");
                self.close = true;
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Pressed,
                ..
            } => self.can_drag_window = true,
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.frame.info.native_pixels_per_point = Some(*scale_factor as _);
            }
            _ => {}
        }

        self.egui_winit.on_event(&self.egui_ctx, event)
    }

    #[cfg(feature = "accesskit")]
    pub fn on_accesskit_action_request(&mut self, request: accesskit::ActionRequest) {
        self.egui_winit.on_accesskit_action_request(request);
    }

    pub fn update(
        &mut self,
        app: &mut dyn epi::App,
        window: &winit::window::Window,
    ) -> egui::FullOutput {
        let frame_start = std::time::Instant::now();

        self.frame.info.window_info =
            read_window_info(window, self.egui_ctx.pixels_per_point(), &self.window_state);
        let raw_input = self.egui_winit.take_egui_input(window);

        // Run user code:
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
                tracing::debug!("App::on_close_event returned {}", self.close);
            }
            self.frame.output.visible = app_output.visible; // this is handled by post_present
            handle_app_output(
                window,
                self.egui_ctx.pixels_per_point(),
                app_output,
                &mut self.window_state,
            );
        }

        let frame_time = frame_start.elapsed().as_secs_f64() as f32;
        self.frame.info.cpu_usage = Some(frame_time);

        full_output
    }

    pub fn post_rendering(&mut self, app: &mut dyn epi::App, window: &winit::window::Window) {
        let inner_size = window.inner_size();
        let window_size_px = [inner_size.width, inner_size.height];

        app.post_rendering(window_size_px, &self.frame);
    }

    pub fn post_present(&mut self, window: &winit::window::Window) {
        if let Some(visible) = self.frame.output.visible.take() {
            window.set_visible(visible);
        }
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
                self.egui_ctx
                    .memory(|mem| epi::set_value(storage, STORAGE_EGUI_MEMORY_KEY, mem));
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
