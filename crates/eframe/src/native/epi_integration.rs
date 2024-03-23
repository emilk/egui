//! Common tools used by [`super::glow_integration`] and [`super::wgpu_integration`].

use web_time::Instant;
use winit::event_loop::EventLoopWindowTarget;

use raw_window_handle::{HasDisplayHandle as _, HasWindowHandle as _};

use egui::{DeferredViewportUiCallback, NumExt as _, ViewportBuilder, ViewportId};
use egui_winit::{EventResponse, WindowSettings};

use crate::{epi, Theme};

pub fn viewport_builder<E>(
    egui_zoom_factor: f32,
    event_loop: &EventLoopWindowTarget<E>,
    native_options: &mut epi::NativeOptions,
    window_settings: Option<WindowSettings>,
) -> ViewportBuilder {
    crate::profile_function!();

    let mut viewport_builder = native_options.viewport.clone();

    // Always use the default window size / position on iOS. Trying to restore the previous position
    // causes the window to be shown too small.
    #[cfg(not(target_os = "ios"))]
    let inner_size_points = if let Some(mut window_settings) = window_settings {
        // Restore pos/size from previous session

        window_settings
            .clamp_size_to_sane_values(largest_monitor_point_size(egui_zoom_factor, event_loop));
        window_settings.clamp_position_to_monitors(egui_zoom_factor, event_loop);

        viewport_builder = window_settings.initialize_viewport_builder(viewport_builder);
        window_settings.inner_size_points()
    } else {
        if let Some(pos) = viewport_builder.position {
            viewport_builder = viewport_builder.with_position(pos);
        }

        if let Some(initial_window_size) = viewport_builder.inner_size {
            let initial_window_size = initial_window_size
                .at_most(largest_monitor_point_size(egui_zoom_factor, event_loop));
            viewport_builder = viewport_builder.with_inner_size(initial_window_size);
        }

        viewport_builder.inner_size
    };

    #[cfg(not(target_os = "ios"))]
    if native_options.centered {
        crate::profile_scope!("center");
        if let Some(monitor) = event_loop.available_monitors().next() {
            let monitor_size = monitor
                .size()
                .to_logical::<f32>(egui_zoom_factor as f64 * monitor.scale_factor());
            let inner_size = inner_size_points.unwrap_or(egui::Vec2 { x: 800.0, y: 600.0 });
            if 0.0 < monitor_size.width && 0.0 < monitor_size.height {
                let x = (monitor_size.width - inner_size.x) / 2.0;
                let y = (monitor_size.height - inner_size.y) / 2.0;
                viewport_builder = viewport_builder.with_position([x, y]);
            }
        }
    }

    match std::mem::take(&mut native_options.window_builder) {
        Some(hook) => hook(viewport_builder),
        None => viewport_builder,
    }
}

pub fn apply_window_settings(
    window: &winit::window::Window,
    window_settings: Option<WindowSettings>,
) {
    crate::profile_function!();

    if let Some(window_settings) = window_settings {
        window_settings.initialize_window(window);
    }
}

fn largest_monitor_point_size<E>(
    egui_zoom_factor: f32,
    event_loop: &EventLoopWindowTarget<E>,
) -> egui::Vec2 {
    crate::profile_function!();

    let mut max_size = egui::Vec2::ZERO;

    let available_monitors = {
        crate::profile_scope!("available_monitors");
        event_loop.available_monitors()
    };

    for monitor in available_monitors {
        let size = monitor
            .size()
            .to_logical::<f32>(egui_zoom_factor as f64 * monitor.scale_factor());
        let size = egui::vec2(size.width, size.height);
        max_size = max_size.max(size);
    }

    if max_size == egui::Vec2::ZERO {
        egui::Vec2::splat(16000.0)
    } else {
        max_size
    }
}

// ----------------------------------------------------------------------------

/// For loading/saving app state and/or egui memory to disk.
pub fn create_storage(_app_name: &str) -> Option<Box<dyn epi::Storage>> {
    #[cfg(feature = "persistence")]
    if let Some(storage) = super::file_storage::FileStorage::from_app_id(_app_name) {
        return Some(Box::new(storage));
    }
    None
}

// ----------------------------------------------------------------------------

/// Everything needed to make a winit-based integration for [`epi`].
///
/// Only one instance per app (not one per viewport).
pub struct EpiIntegration {
    pub frame: epi::Frame,
    last_auto_save: Instant,
    pub beginning: Instant,
    is_first_frame: bool,
    pub frame_start: Instant,
    pub egui_ctx: egui::Context,
    pending_full_output: egui::FullOutput,

    /// When set, it is time to close the native window.
    close: bool,

    can_drag_window: bool,
    follow_system_theme: bool,
    #[cfg(feature = "persistence")]
    persist_window: bool,
    app_icon_setter: super::app_icon::AppTitleIconSetter,
}

impl EpiIntegration {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        egui_ctx: egui::Context,
        window: &winit::window::Window,
        system_theme: Option<Theme>,
        app_name: &str,
        native_options: &crate::NativeOptions,
        storage: Option<Box<dyn epi::Storage>>,
        #[cfg(feature = "glow")] gl: Option<std::sync::Arc<glow::Context>>,
        #[cfg(feature = "wgpu")] wgpu_render_state: Option<egui_wgpu::RenderState>,
    ) -> Self {
        let frame = epi::Frame {
            info: epi::IntegrationInfo {
                system_theme,
                cpu_usage: None,
            },
            storage,
            #[cfg(feature = "glow")]
            gl,
            #[cfg(feature = "wgpu")]
            wgpu_render_state,
            raw_display_handle: window.display_handle().map(|h| h.as_raw()),
            raw_window_handle: window.window_handle().map(|h| h.as_raw()),
        };

        let icon = native_options
            .viewport
            .icon
            .clone()
            .unwrap_or_else(|| std::sync::Arc::new(load_default_egui_icon()));

        let app_icon_setter = super::app_icon::AppTitleIconSetter::new(
            native_options
                .viewport
                .title
                .clone()
                .unwrap_or_else(|| app_name.to_owned()),
            Some(icon),
        );

        Self {
            frame,
            last_auto_save: Instant::now(),
            egui_ctx,
            pending_full_output: Default::default(),
            close: false,
            can_drag_window: false,
            follow_system_theme: native_options.follow_system_theme,
            #[cfg(feature = "persistence")]
            persist_window: native_options.persist_window,
            app_icon_setter,
            beginning: Instant::now(),
            is_first_frame: true,
            frame_start: Instant::now(),
        }
    }

    #[cfg(feature = "accesskit")]
    pub fn init_accesskit<E: From<egui_winit::accesskit_winit::ActionRequestEvent> + Send>(
        &self,
        egui_winit: &mut egui_winit::State,
        window: &winit::window::Window,
        event_loop_proxy: winit::event_loop::EventLoopProxy<E>,
    ) {
        crate::profile_function!();

        let egui_ctx = self.egui_ctx.clone();
        egui_winit.init_accesskit(window, event_loop_proxy, move || {
            // This function is called when an accessibility client
            // (e.g. screen reader) makes its first request. If we got here,
            // we know that an accessibility tree is actually wanted.
            egui_ctx.enable_accesskit();
            // Enqueue a repaint so we'll receive a full tree update soon.
            egui_ctx.request_repaint();
            egui_ctx.accesskit_placeholder_tree_update()
        });
    }

    /// If `true`, it is time to close the native window.
    pub fn should_close(&self) -> bool {
        self.close
    }

    pub fn on_window_event(
        &mut self,
        window: &winit::window::Window,
        egui_winit: &mut egui_winit::State,
        event: &winit::event::WindowEvent,
    ) -> EventResponse {
        crate::profile_function!(egui_winit::short_window_event_description(event));

        use winit::event::{ElementState, MouseButton, WindowEvent};

        match event {
            WindowEvent::Destroyed => {
                log::debug!("Received WindowEvent::Destroyed");
                self.close = true;
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Pressed,
                ..
            } => self.can_drag_window = true,
            WindowEvent::ThemeChanged(winit_theme) if self.follow_system_theme => {
                let theme = theme_from_winit_theme(*winit_theme);
                self.frame.info.system_theme = Some(theme);
                self.egui_ctx.set_visuals(theme.egui_visuals());
            }
            _ => {}
        }

        egui_winit.on_window_event(window, event)
    }

    pub fn pre_update(&mut self) {
        self.app_icon_setter.update();
    }

    /// Run user code - this can create immediate viewports, so hold no locks over this!
    ///
    /// If `viewport_ui_cb` is None, we are in the root viewport and will call [`crate::App::update`].
    pub fn update(
        &mut self,
        app: &mut dyn epi::App,
        viewport_ui_cb: Option<&DeferredViewportUiCallback>,
        mut raw_input: egui::RawInput,
    ) -> egui::FullOutput {
        raw_input.time = Some(self.beginning.elapsed().as_secs_f64());

        let close_requested = raw_input.viewport().close_requested();

        app.raw_input_hook(&self.egui_ctx, &mut raw_input);

        let full_output = self.egui_ctx.run(raw_input, |egui_ctx| {
            if let Some(viewport_ui_cb) = viewport_ui_cb {
                // Child viewport
                crate::profile_scope!("viewport_callback");
                viewport_ui_cb(egui_ctx);
            } else {
                crate::profile_scope!("App::update");
                app.update(egui_ctx, &mut self.frame);
            }
        });

        let is_root_viewport = viewport_ui_cb.is_none();
        if is_root_viewport && close_requested {
            let canceled = full_output.viewport_output[&ViewportId::ROOT]
                .commands
                .contains(&egui::ViewportCommand::CancelClose);
            if canceled {
                log::debug!("Closing of root viewport canceled with ViewportCommand::CancelClose");
            } else {
                log::debug!("Closing root viewport (ViewportCommand::CancelClose was not sent)");
                self.close = true;
            }
        }

        self.pending_full_output.append(full_output);
        std::mem::take(&mut self.pending_full_output)
    }

    pub fn report_frame_time(&mut self, seconds: f32) {
        self.frame.info.cpu_usage = Some(seconds);
    }

    pub fn post_rendering(&mut self, window: &winit::window::Window) {
        crate::profile_function!();
        if std::mem::take(&mut self.is_first_frame) {
            // We keep hidden until we've painted something. See https://github.com/emilk/egui/pull/2279
            window.set_visible(true);
        }
    }

    // ------------------------------------------------------------------------
    // Persistence stuff:

    pub fn maybe_autosave(
        &mut self,
        app: &mut dyn epi::App,
        window: Option<&winit::window::Window>,
    ) {
        let now = Instant::now();
        if now - self.last_auto_save > app.auto_save_interval() {
            self.save(app, window);
            self.last_auto_save = now;
        }
    }

    #[allow(clippy::unused_self)]
    pub fn save(&mut self, _app: &mut dyn epi::App, _window: Option<&winit::window::Window>) {
        #[cfg(feature = "persistence")]
        if let Some(storage) = self.frame.storage_mut() {
            crate::profile_function!();

            if let Some(window) = _window {
                if self.persist_window {
                    crate::profile_scope!("native_window");
                    epi::set_value(
                        storage,
                        STORAGE_WINDOW_KEY,
                        &WindowSettings::from_window(self.egui_ctx.zoom_factor(), window),
                    );
                }
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

fn load_default_egui_icon() -> egui::IconData {
    crate::profile_function!();
    crate::icon_data::from_png_bytes(&include_bytes!("../../data/icon.png")[..]).unwrap()
}

#[cfg(feature = "persistence")]
const STORAGE_EGUI_MEMORY_KEY: &str = "egui";

#[cfg(feature = "persistence")]
const STORAGE_WINDOW_KEY: &str = "window";

pub fn load_window_settings(_storage: Option<&dyn epi::Storage>) -> Option<WindowSettings> {
    crate::profile_function!();
    #[cfg(feature = "persistence")]
    {
        epi::get_value(_storage?, STORAGE_WINDOW_KEY)
    }
    #[cfg(not(feature = "persistence"))]
    None
}

pub fn load_egui_memory(_storage: Option<&dyn epi::Storage>) -> Option<egui::Memory> {
    crate::profile_function!();
    #[cfg(feature = "persistence")]
    {
        epi::get_value(_storage?, STORAGE_EGUI_MEMORY_KEY)
    }
    #[cfg(not(feature = "persistence"))]
    None
}

pub(crate) fn theme_from_winit_theme(theme: winit::window::Theme) -> Theme {
    match theme {
        winit::window::Theme::Dark => Theme::Dark,
        winit::window::Theme::Light => Theme::Light,
    }
}
