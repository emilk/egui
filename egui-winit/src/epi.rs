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
) -> epi::backend::TexAllocationData {
    let epi::backend::AppOutput {
        quit: _,
        tex_allocation_data,
        window_size,
        window_title,
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

    if let Some(window_title) = window_title {
        window.set_title(&window_title);
    }

    if drag_window {
        let _ = window.drag_window();
    }

    tex_allocation_data
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
        _egui_ctx: &egui::CtxRef,
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
        egui_ctx: &egui::CtxRef,
        window: &winit::window::Window,
    ) {
        let now = std::time::Instant::now();
        if now - self.last_auto_save > app.auto_save_interval() {
            self.save(app, egui_ctx, window);
            self.last_auto_save = now;
        }
    }
}

// ----------------------------------------------------------------------------

/// Everything needed to make a winit-based integration for [`epi`].
pub struct EpiIntegration {
    frame: epi::Frame,
    persistence: crate::epi::Persistence,
    pub egui_ctx: egui::CtxRef,
    egui_winit: crate::State,
    pub app: Box<dyn epi::App>,
    /// When set, it is time to quit
    quit: bool,
}

impl EpiIntegration {
    pub fn new(
        integration_name: &'static str,
        window: &winit::window::Window,
        repaint_signal: std::sync::Arc<dyn epi::backend::RepaintSignal>,
        persistence: crate::epi::Persistence,
        app: Box<dyn epi::App>,
    ) -> Self {
        let egui_ctx = egui::CtxRef::default();

        *egui_ctx.memory() = persistence.load_memory().unwrap_or_default();

        let frame = epi::Frame::new(epi::backend::FrameData {
            info: epi::IntegrationInfo {
                name: integration_name,
                web_info: None,
                prefer_dark_mode: None, // TODO: figure out system default
                cpu_usage: None,
                native_pixels_per_point: Some(crate::native_pixels_per_point(window)),
            },
            output: Default::default(),
            repaint_signal,
        });

        let mut slf = Self {
            frame,
            persistence,
            egui_ctx,
            egui_winit: crate::State::new(window),
            app,
            quit: false,
        };

        slf.setup(window);
        if slf.app.warm_up_enabled() {
            slf.warm_up(window);
        }

        slf
    }

    fn setup(&mut self, window: &winit::window::Window) {
        self.app
            .setup(&self.egui_ctx, &self.frame, self.persistence.storage());
        let app_output = self.frame.take_app_output();
        self.quit |= app_output.quit;
        let tex_alloc_data =
            crate::epi::handle_app_output(window, self.egui_ctx.pixels_per_point(), app_output);
        self.frame.lock().output.tex_allocation_data = tex_alloc_data; // Do it later
    }

    fn warm_up(&mut self, window: &winit::window::Window) {
        let saved_memory = self.egui_ctx.memory().clone();
        self.egui_ctx.memory().set_everything_is_visible(true);
        let (_, tex_alloc_data, _) = self.update(window);
        self.frame.lock().output.tex_allocation_data = tex_alloc_data; // handle it next frame
        *self.egui_ctx.memory() = saved_memory; // We don't want to remember that windows were huge.
        self.egui_ctx.clear_animations();
    }

    /// If `true`, it is time to shut down.
    pub fn should_quit(&self) -> bool {
        self.quit
    }

    pub fn on_event(&mut self, event: &winit::event::WindowEvent<'_>) {
        use winit::event::WindowEvent;
        self.quit |= matches!(event, WindowEvent::CloseRequested | WindowEvent::Destroyed);
        self.egui_winit.on_event(&self.egui_ctx, event);
    }

    /// Returns `needs_repaint` and shapes to paint.
    pub fn update(
        &mut self,
        window: &winit::window::Window,
    ) -> (
        bool,
        epi::backend::TexAllocationData,
        Vec<egui::epaint::ClippedShape>,
    ) {
        let frame_start = std::time::Instant::now();

        let raw_input = self.egui_winit.take_egui_input(window);
        let (egui_output, shapes) = self.egui_ctx.run(raw_input, |egui_ctx| {
            self.app.update(egui_ctx, &self.frame);
        });

        let needs_repaint = egui_output.needs_repaint;
        self.egui_winit
            .handle_output(window, &self.egui_ctx, egui_output);

        let app_output = self.frame.take_app_output();
        self.quit |= app_output.quit;
        let tex_allocation_data =
            crate::epi::handle_app_output(window, self.egui_ctx.pixels_per_point(), app_output);

        let frame_time = (std::time::Instant::now() - frame_start).as_secs_f64() as f32;
        self.frame.lock().info.cpu_usage = Some(frame_time);

        (needs_repaint, tex_allocation_data, shapes)
    }

    pub fn maybe_autosave(&mut self, window: &winit::window::Window) {
        self.persistence
            .maybe_autosave(&mut *self.app, &self.egui_ctx, window);
    }

    pub fn on_exit(&mut self, window: &winit::window::Window) {
        self.app.on_exit();
        self.persistence
            .save(&mut *self.app, &self.egui_ctx, window);
    }
}
