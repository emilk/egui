//! Common tools used by [`super::glow_integration`] and [`super::wgpu_integration`].

use web_time::Instant;

use std::{path::PathBuf, sync::Arc};
use winit::event_loop::ActiveEventLoop;

use raw_window_handle::{HasDisplayHandle as _, HasWindowHandle as _};

use egui::{DeferredViewportUiCallback, ViewportBuilder, ViewportId};
use egui_winit::{EventResponse, WindowSettings};

use crate::epi;

#[cfg_attr(target_os = "ios", allow(dead_code, unused_variables, unused_mut))]
pub fn viewport_builder(
    egui_zoom_factor: f32,
    event_loop: &ActiveEventLoop,
    native_options: &mut epi::NativeOptions,
    window_settings: Option<WindowSettings>,
) -> ViewportBuilder {
    profiling::function_scope!();

    let mut viewport_builder = native_options.viewport.clone();

    // On some Linux systems, a window size larger than the monitor causes crashes,
    // and on Windows the window does not appear at all.
    let clamp_size_to_monitor_size = viewport_builder.clamp_size_to_monitor_size.unwrap_or(true);

    // Always use the default window size / position on iOS. Trying to restore the previous position
    // causes the window to be shown too small.
    #[cfg(not(target_os = "ios"))]
    let restored = window_settings.is_some();
    #[cfg(not(target_os = "ios"))]
    let inner_size_points = if let Some(mut window_settings) = window_settings {
        // Restore pos/size from previous session

        if clamp_size_to_monitor_size {
            window_settings.clamp_size_to_sane_values(largest_monitor_point_size(
                egui_zoom_factor,
                event_loop,
            ));
        }
        window_settings.clamp_position_to_monitors(egui_zoom_factor, event_loop);

        viewport_builder = window_settings.initialize_viewport_builder(
            egui_zoom_factor,
            event_loop,
            viewport_builder,
        );
        window_settings.inner_size_points()
    } else {
        if let Some(pos) = viewport_builder.position {
            viewport_builder = viewport_builder.with_position(pos);
        }

        if clamp_size_to_monitor_size && let Some(initial_window_size) = viewport_builder.inner_size
        {
            let initial_window_size = egui::NumExt::at_most(
                initial_window_size,
                largest_monitor_point_size(egui_zoom_factor, event_loop),
            );
            viewport_builder = viewport_builder.with_inner_size(initial_window_size);
        }

        viewport_builder.inner_size
    };

    // Don't center over a restored position (would fight multi-monitor restore).
    #[cfg(not(target_os = "ios"))]
    if native_options.centered && !restored {
        profiling::scope!("center");
        if let Some(monitor) = event_loop
            .primary_monitor()
            .or_else(|| event_loop.available_monitors().next())
        {
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
    profiling::function_scope!();
    if let Some(window_settings) = window_settings {
        window_settings.initialize_window(window);
    }
}

#[cfg(not(target_os = "ios"))]
fn largest_monitor_point_size(egui_zoom_factor: f32, event_loop: &ActiveEventLoop) -> egui::Vec2 {
    profiling::function_scope!();
    let mut max_size = egui::Vec2::ZERO;

    let available_monitors = {
        profiling::scope!("available_monitors");
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

#[allow(clippy::allow_attributes, clippy::unnecessary_wraps)]
pub fn create_storage_with_file(_file: impl Into<PathBuf>) -> Option<Box<dyn epi::Storage>> {
    #[cfg(feature = "persistence")]
    return Some(Box::new(
        super::file_storage::FileStorage::from_ron_filepath(_file),
    ));
    #[cfg(not(feature = "persistence"))]
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
    /// Deferred first show so we reveal after the first paint (avoids empty flash).
    pending_reveal: bool,
    /// Windows wgpu: uncloak after the redraw that follows a cloaked first show.
    #[cfg(all(feature = "wgpu_no_default_features", target_os = "windows"))]
    pub(crate) pending_finish_reveal: bool,
    /// False until first reveal — avoids persisting mid-DPI-restore geometry.
    #[cfg(feature = "persistence")]
    persist_window_geometry: bool,
    /// Windows: placement used for cloaked first show / maximize.
    #[cfg(all(feature = "persistence", target_os = "windows"))]
    reveal_window_settings: Option<WindowSettings>,
    pub egui_ctx: egui::Context,
    pending_full_output: egui::FullOutput,

    /// When set, it is time to close the native window.
    close: bool,

    can_drag_window: bool,
    #[cfg(feature = "persistence")]
    persist_window: bool,
    app_icon_setter: super::app_icon::AppTitleIconSetter,
}

impl EpiIntegration {
    #[allow(clippy::allow_attributes, clippy::too_many_arguments)]
    pub fn new(
        egui_ctx: egui::Context,
        window: &Arc<winit::window::Window>,
        app_name: &str,
        native_options: &crate::NativeOptions,
        storage: Option<Box<dyn epi::Storage>>,
        #[cfg(feature = "persistence")] reveal_window_settings: Option<WindowSettings>,
        #[cfg(feature = "glow")] gl: Option<std::sync::Arc<glow::Context>>,
        #[cfg(feature = "glow")] glow_register_native_texture: Option<
            Box<dyn FnMut(glow::Texture) -> egui::TextureId>,
        >,
        #[cfg(feature = "wgpu_no_default_features")] wgpu_render_state: Option<
            egui_wgpu::RenderState,
        >,
    ) -> Self {
        #[cfg(all(feature = "persistence", not(target_os = "windows")))]
        let _ = reveal_window_settings;

        let frame = epi::Frame {
            info: epi::IntegrationInfo { cpu_usage: None },
            storage,
            #[cfg(feature = "glow")]
            gl,
            #[cfg(feature = "glow")]
            glow_register_native_texture,
            #[cfg(feature = "wgpu_no_default_features")]
            wgpu_render_state,
            window: Some(Arc::clone(window)),
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
            pending_full_output: Default::default(),
            close: false,
            can_drag_window: false,
            #[cfg(feature = "persistence")]
            persist_window: native_options.persist_window,
            #[cfg(all(feature = "persistence", target_os = "windows"))]
            reveal_window_settings,
            #[cfg(feature = "persistence")]
            persist_window_geometry: false,
            pending_reveal: false,
            #[cfg(all(feature = "wgpu_no_default_features", target_os = "windows"))]
            pending_finish_reveal: false,
            app_icon_setter,
            beginning: Instant::now()
                .checked_sub(web_time::Duration::from_secs_f64(egui_ctx.time()))
                .unwrap_or_else(Instant::now),
            is_first_frame: true,
            egui_ctx,
        }
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
        profiling::function_scope!(egui_winit::short_window_event_description(event));

        use winit::event::{ElementState, MouseButton, WindowEvent};

        if let WindowEvent::MouseInput {
            button: MouseButton::Left,
            state: ElementState::Pressed,
            ..
        } = event
        {
            self.can_drag_window = true;
        }

        egui_winit.on_window_event(window, event)
    }

    pub fn pre_update(&mut self) {
        self.app_icon_setter.update();
    }

    /// Run user code - this can create immediate viewports, so hold no locks over this!
    ///
    /// If `viewport_ui_cb` is None, we are in the root viewport and will call [`crate::App::ui`].
    pub fn update(
        &mut self,
        app: &mut dyn epi::App,
        viewport_ui_cb: Option<&DeferredViewportUiCallback>,
        mut raw_input: egui::RawInput,
        is_visible: bool,
    ) -> egui::FullOutput {
        raw_input.time = Some(self.beginning.elapsed().as_secs_f64());

        let close_requested = raw_input.viewport().close_requested();

        app.raw_input_hook(&self.egui_ctx, &mut raw_input);

        let full_output = self.egui_ctx.run_ui(raw_input, |ui| {
            if let Some(viewport_ui_cb) = viewport_ui_cb {
                // Child viewport
                if is_visible {
                    profiling::scope!("viewport_callback");
                    viewport_ui_cb(ui);
                }
            } else {
                {
                    profiling::scope!("App::logic");
                    app.logic(ui.ctx(), &mut self.frame);
                }

                if is_visible {
                    {
                        profiling::scope!("App::ui");
                        app.ui(ui, &mut self.frame);
                    }
                }
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

    pub fn post_rendering(&mut self) {
        profiling::function_scope!();
        if std::mem::take(&mut self.is_first_frame) {
            // Reveal after present so the first visible frame already has pixels.
            // See https://github.com/emilk/egui/pull/2279 / #3631.
            self.pending_reveal = true;
        }
    }

    /// Show just before the first present. On Windows: DWM-cloak first.
    /// Call [`Self::finish_reveal_after_present`] after that present.
    pub fn reveal_after_present(&mut self, window: &winit::window::Window) -> bool {
        if !std::mem::take(&mut self.pending_reveal) {
            return false;
        }
        #[cfg(target_os = "windows")]
        {
            #[cfg(feature = "persistence")]
            self.reveal_window_settings
                .take()
                .unwrap_or_default()
                .reveal_window(window);
            #[cfg(not(feature = "persistence"))]
            WindowSettings::default().reveal_window(window);
        }
        #[cfg(not(target_os = "windows"))]
        window.set_visible(true);

        #[cfg(feature = "persistence")]
        {
            self.persist_window_geometry = true;
        }
        true
    }

    /// Uncloak after the first present (no-op when `revealed` is false / off Windows).
    pub fn finish_reveal_after_present(window: &winit::window::Window, revealed: bool) {
        if revealed {
            WindowSettings::finish_first_show(window);
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

    pub fn save(&mut self, app: &mut dyn epi::App, window: Option<&winit::window::Window>) {
        #[cfg(not(feature = "persistence"))]
        let _ = (self, app, window);

        #[cfg(feature = "persistence")]
        if let Some(storage) = self.frame.storage_mut() {
            profiling::function_scope!();

            if let Some(window) = window
                && self.persist_window
                && self.persist_window_geometry
            {
                profiling::scope!("native_window");
                epi::set_value(
                    storage,
                    STORAGE_WINDOW_KEY,
                    &WindowSettings::from_window(self.egui_ctx.zoom_factor(), window),
                );
            }
            if app.persist_egui_memory() {
                profiling::scope!("egui_memory");
                self.egui_ctx
                    .memory(|mem| epi::set_value(storage, STORAGE_EGUI_MEMORY_KEY, mem));
            }
            {
                profiling::scope!("App::save");
                app.save(storage);
            }

            profiling::scope!("Storage::flush");
            storage.flush();
        }
    }
}

fn load_default_egui_icon() -> egui::IconData {
    profiling::function_scope!();
    #[expect(clippy::unwrap_used)]
    crate::icon_data::from_png_bytes(&include_bytes!("../../data/icon.png")[..]).unwrap()
}

#[cfg(feature = "persistence")]
const STORAGE_EGUI_MEMORY_KEY: &str = "egui";

#[cfg(feature = "persistence")]
const STORAGE_WINDOW_KEY: &str = "window";

pub fn load_window_settings(_storage: Option<&dyn epi::Storage>) -> Option<WindowSettings> {
    profiling::function_scope!();
    #[cfg(feature = "persistence")]
    {
        epi::get_value(_storage?, STORAGE_WINDOW_KEY)
    }
    #[cfg(not(feature = "persistence"))]
    None
}

pub fn load_egui_memory(_storage: Option<&dyn epi::Storage>) -> Option<egui::Memory> {
    profiling::function_scope!();
    #[cfg(feature = "persistence")]
    {
        epi::get_value(_storage?, STORAGE_EGUI_MEMORY_KEY)
    }
    #[cfg(not(feature = "persistence"))]
    None
}
