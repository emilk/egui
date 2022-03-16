pub fn points_to_size(points: egui::Vec2) -> winit::dpi::LogicalSize<f64> {
    winit::dpi::LogicalSize {
        width: points.x as f64,
        height: points.y as f64,
    }
}

pub fn window_builder(
    native_options: &epi::NativeOptions,
    window_settings: &Option<crate::WindowSettings>,
) -> winit::window::WindowBuilder {
    let epi::NativeOptions {
        always_on_top,
        maximized,
        decorated,
        drag_and_drop_support,
        icon_data,
        initial_window_pos,
        initial_window_size,
        min_window_size,
        max_window_size,
        resizable,
        transparent,
    } = native_options;

    let window_icon = icon_data.clone().and_then(load_icon);

    let mut window_builder = winit::window::WindowBuilder::new()
        .with_always_on_top(*always_on_top)
        .with_maximized(*maximized)
        .with_decorations(*decorated)
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
        quit: _,
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
}

// ----------------------------------------------------------------------------

/// For loading/saving app state and/or egui memory to disk.
pub struct Persistence {
    storage: Option<Box<dyn epi::Storage>>,
    last_auto_save: instant::Instant,
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
            last_auto_save: instant::Instant::now(),
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
        let now = instant::Instant::now();
        if now - self.last_auto_save > app.auto_save_interval() {
            self.save(app, egui_ctx, window);
            self.last_auto_save = now;
        }
    }
}

// ----------------------------------------------------------------------------

/// Everything needed to make a winit-based integration for [`epi`].
pub struct EpiIntegration {
    pub frame: epi::Frame,
    pub persistence: crate::epi::Persistence,
    pub egui_ctx: egui::Context,
    pending_full_output: egui::FullOutput,
    egui_winit: crate::State,
    /// When set, it is time to quit
    quit: bool,
    can_drag_window: bool,
}

impl EpiIntegration {
    pub fn new(
        integration_name: &'static str,
        max_texture_side: usize,
        window: &winit::window::Window,
        persistence: crate::epi::Persistence,
    ) -> Self {
        let egui_ctx = egui::Context::default();

        *egui_ctx.memory() = persistence.load_memory().unwrap_or_default();

        let prefer_dark_mode = prefer_dark_mode();

        let frame = epi::Frame::new(epi::backend::FrameData {
            info: epi::IntegrationInfo {
                name: integration_name,
                web_info: None,
                prefer_dark_mode,
                cpu_usage: None,
                native_pixels_per_point: Some(crate::native_pixels_per_point(window)),
            },
            output: Default::default(),
        });

        if prefer_dark_mode == Some(true) {
            egui_ctx.set_visuals(egui::Visuals::dark());
        } else {
            egui_ctx.set_visuals(egui::Visuals::light());
        }

        Self {
            frame,
            persistence,
            egui_ctx,
            egui_winit: crate::State::new(max_texture_side, window),
            pending_full_output: Default::default(),
            quit: false,
            can_drag_window: false,
        }
    }

    pub fn warm_up(&mut self, app: &mut dyn epi::App, window: &winit::window::Window) {
        let saved_memory: egui::Memory = self.egui_ctx.memory().clone();
        self.egui_ctx.memory().set_everything_is_visible(true);
        let full_output = self.update(app, window);
        self.pending_full_output.append(full_output); // Handle it next frame
        *self.egui_ctx.memory() = saved_memory; // We don't want to remember that windows were huge.
        self.egui_ctx.clear_animations();
    }

    /// If `true`, it is time to shut down.
    pub fn should_quit(&self) -> bool {
        self.quit
    }

    pub fn on_event(&mut self, app: &mut dyn epi::App, event: &winit::event::WindowEvent<'_>) {
        use winit::event::{ElementState, MouseButton, WindowEvent};

        match event {
            WindowEvent::CloseRequested => self.quit = app.on_exit_event(),
            WindowEvent::Destroyed => self.quit = true,
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
        let frame_start = instant::Instant::now();

        let raw_input = self.egui_winit.take_egui_input(window);
        let full_output = self.egui_ctx.run(raw_input, |egui_ctx| {
            app.update(egui_ctx, &self.frame);
        });
        self.pending_full_output.append(full_output);
        let full_output = std::mem::take(&mut self.pending_full_output);

        {
            let mut app_output = self.frame.take_app_output();
            app_output.drag_window &= self.can_drag_window; // Necessary on Windows; see https://github.com/emilk/egui/pull/1108
            self.can_drag_window = false;
            if app_output.quit {
                self.quit = app.on_exit_event();
            }
            crate::epi::handle_app_output(window, self.egui_ctx.pixels_per_point(), app_output);
        }

        let frame_time = (instant::Instant::now() - frame_start).as_secs_f64() as f32;
        self.frame.lock().info.cpu_usage = Some(frame_time);

        full_output
    }

    pub fn handle_platform_output(
        &mut self,
        window: &winit::window::Window,
        platform_output: egui::PlatformOutput,
    ) {
        self.egui_winit
            .handle_platform_output(window, &self.egui_ctx, platform_output);
    }

    pub fn maybe_autosave(&mut self, app: &mut dyn epi::App, window: &winit::window::Window) {
        self.persistence
            .maybe_autosave(&mut *app, &self.egui_ctx, window);
    }
}

#[cfg(feature = "dark-light")]
fn prefer_dark_mode() -> Option<bool> {
    match dark_light::detect() {
        dark_light::Mode::Dark => Some(true),
        dark_light::Mode::Light => Some(false),
    }
}

#[cfg(not(feature = "dark-light"))]
fn prefer_dark_mode() -> Option<bool> {
    None
}
