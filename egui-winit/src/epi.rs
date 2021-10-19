#[cfg(target_os = "windows")]
use winit::platform::windows::WindowBuilderExtWindows;

pub fn window_builder(
    native_options: &epi::NativeOptions,
    window_settings: &Option<crate::WindowSettings>,
) -> winit::window::WindowBuilder {
    let window_icon = native_options.icon_data.clone().and_then(load_icon);

    let mut window_builder = winit::window::WindowBuilder::new()
        .with_always_on_top(native_options.always_on_top)
        .with_maximized(native_options.maximized)
        .with_decorations(native_options.decorated)
        .with_resizable(native_options.resizable)
        .with_transparent(native_options.transparent)
        .with_window_icon(window_icon);

    window_builder =
        window_builder_drag_and_drop(window_builder, native_options.drag_and_drop_support);

    let initial_size_points = native_options.initial_window_size;

    if let Some(window_settings) = window_settings {
        window_builder = window_settings.initialize_window(window_builder);
    } else if let Some(initial_size_points) = initial_size_points {
        window_builder = window_builder.with_inner_size(winit::dpi::LogicalSize {
            width: initial_size_points.x as f64,
            height: initial_size_points.y as f64,
        });
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
        quit: _,
        window_size,
        decorated,
        drag_window,
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
            .to_logical::<f32>(crate::native_pixels_per_point(window) as f64),
        );
    }

    if drag_window {
        let _ = window.drag_window();
    }
}

// ----------------------------------------------------------------------------

/// For loading/saving app state and/or egui memory to disk.
pub struct Persistence {
    storage: Option<Box<dyn epi::Storage>>,
    last_auto_save: std::time::Instant,
}

#[allow(clippy::unused_self)]
impl Persistence {
    #[cfg(feature = "persistence")]
    const EGUI_MEMORY_KEY: &'static str = "egui";
    #[cfg(feature = "persistence")]
    const WINDOW_KEY: &'static str = "window";

    pub fn from_app_name(app_name: &str) -> Self {
        fn create_storage(_app_name: &str) -> Option<Box<dyn epi::Storage>> {
            #[cfg(feature = "persistence")]
            if let Some(storage) = epi::file_storage::FileStorage::from_app_name(_app_name) {
                return Some(Box::new(storage));
            }
            None
        }

        Self {
            storage: create_storage(app_name),
            last_auto_save: std::time::Instant::now(),
        }
    }

    pub fn storage(&self) -> Option<&dyn epi::Storage> {
        self.storage.as_deref()
    }

    #[cfg(feature = "persistence")]
    pub fn load_window_settings(&self) -> Option<crate::WindowSettings> {
        epi::get_value(&**self.storage.as_ref()?, Self::WINDOW_KEY)
    }

    #[cfg(not(feature = "persistence"))]
    pub fn load_window_settings(&self) -> Option<crate::WindowSettings> {
        None
    }

    #[cfg(feature = "persistence")]
    pub fn load_memory(&self) -> Option<egui::Memory> {
        epi::get_value(&**self.storage.as_ref()?, Self::EGUI_MEMORY_KEY)
    }

    #[cfg(not(feature = "persistence"))]
    pub fn load_memory(&self) -> Option<egui::Memory> {
        None
    }

    pub fn save(
        &mut self,
        _app: &mut dyn epi::App,
        _egui_ctx: &egui::Context,
        _window: &winit::window::Window,
    ) {
        #[cfg(feature = "persistence")]
        if let Some(storage) = &mut self.storage {
            if _app.persist_native_window() {
                epi::set_value(
                    storage.as_mut(),
                    Self::WINDOW_KEY,
                    &crate::WindowSettings::from_display(_window),
                );
            }
            if _app.persist_egui_memory() {
                epi::set_value(
                    storage.as_mut(),
                    Self::EGUI_MEMORY_KEY,
                    &*_egui_ctx.memory(),
                );
            }
            _app.save(storage.as_mut());
            storage.flush();
        }
    }

    pub fn maybe_autosave(
        &mut self,
        app: &mut dyn epi::App,
        egui_ctx: &egui::Context,
        window: &winit::window::Window,
    ) {
        let now = std::time::Instant::now();
        if now - self.last_auto_save > app.auto_save_interval() {
            self.save(app, egui_ctx, window);
            self.last_auto_save = now;
        }
    }
}
