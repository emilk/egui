//! Note that this file contains two similar paths - one for [`glow`], one for [`wgpu`].
//! When making changes to one you often also want to apply it to the other.

use std::time::Duration;
use std::time::Instant;

#[cfg(feature = "accesskit")]
use egui_winit::accesskit_winit;
use egui_winit::winit;
use winit::event_loop::{
    ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget,
};

use super::epi_integration::{self, EpiIntegration};
use crate::epi;

#[derive(Debug)]
pub enum UserEvent {
    RequestRepaint,
    #[cfg(feature = "accesskit")]
    AccessKitActionRequest(accesskit_winit::ActionRequestEvent),
}

#[cfg(feature = "accesskit")]
impl From<accesskit_winit::ActionRequestEvent> for UserEvent {
    fn from(inner: accesskit_winit::ActionRequestEvent) -> Self {
        Self::AccessKitActionRequest(inner)
    }
}

// ----------------------------------------------------------------------------

pub use epi::NativeOptions;

#[derive(Debug)]
enum EventResult {
    Wait,
    /// Causes a synchronous repaint inside the event handler. This should only
    /// be used in special situations if the window must be repainted while
    /// handling a specific event. This occurs on Windows when handling resizes.
    ///
    /// `RepaintNow` creates a new frame synchronously, and should therefore
    /// only be used for extremely urgent repaints.
    RepaintNow,
    /// Queues a repaint for once the event loop handles its next redraw. Exists
    /// so that multiple input events can be handled in one frame. Does not
    /// cause any delay like `RepaintNow`.
    RepaintNext,
    RepaintAt(Instant),
    Exit,
}

trait WinitApp {
    fn is_focused(&self) -> bool;
    fn integration(&self) -> Option<&EpiIntegration>;
    fn window(&self) -> Option<&winit::window::Window>;
    fn save_and_destroy(&mut self);
    fn paint(&mut self) -> EventResult;
    fn on_event(
        &mut self,
        event_loop: &EventLoopWindowTarget<UserEvent>,
        event: &winit::event::Event<'_, UserEvent>,
    ) -> EventResult;
}

fn create_event_loop_builder(
    native_options: &mut epi::NativeOptions,
) -> EventLoopBuilder<UserEvent> {
    let mut event_loop_builder = winit::event_loop::EventLoopBuilder::with_user_event();

    if let Some(hook) = std::mem::take(&mut native_options.event_loop_builder) {
        hook(&mut event_loop_builder);
    }

    event_loop_builder
}

/// Access a thread-local event loop.
///
/// We reuse the event-loop so we can support closing and opening an eframe window
/// multiple times. This is just a limitation of winit.
fn with_event_loop(
    mut native_options: epi::NativeOptions,
    f: impl FnOnce(&mut EventLoop<UserEvent>, NativeOptions),
) {
    use std::cell::RefCell;
    thread_local!(static EVENT_LOOP: RefCell<Option<EventLoop<UserEvent>>> = RefCell::new(None));

    EVENT_LOOP.with(|event_loop| {
        // Since we want to reference NativeOptions when creating the EventLoop we can't
        // do that as part of the lazy thread local storage initialization and so we instead
        // create the event loop lazily here
        let mut event_loop = event_loop.borrow_mut();
        let event_loop = event_loop
            .get_or_insert_with(|| create_event_loop_builder(&mut native_options).build());
        f(event_loop, native_options);
    });
}

fn run_and_return(event_loop: &mut EventLoop<UserEvent>, mut winit_app: impl WinitApp) {
    use winit::platform::run_return::EventLoopExtRunReturn as _;

    tracing::debug!("event_loop.run_return");

    let mut next_repaint_time = Instant::now();

    event_loop.run_return(|event, event_loop, control_flow| {
        let event_result = match &event {
            winit::event::Event::LoopDestroyed => {
                tracing::debug!("winit::event::Event::LoopDestroyed");
                EventResult::Exit
            }

            // Platform-dependent event handlers to workaround a winit bug
            // See: https://github.com/rust-windowing/winit/issues/987
            // See: https://github.com/rust-windowing/winit/issues/1619
            winit::event::Event::RedrawEventsCleared if cfg!(windows) => {
                next_repaint_time = Instant::now() + Duration::from_secs(1_000_000_000);
                winit_app.paint()
            }
            winit::event::Event::RedrawRequested(_) if !cfg!(windows) => {
                next_repaint_time = Instant::now() + Duration::from_secs(1_000_000_000);
                winit_app.paint()
            }

            winit::event::Event::UserEvent(UserEvent::RequestRepaint)
            | winit::event::Event::NewEvents(winit::event::StartCause::ResumeTimeReached {
                ..
            }) => EventResult::RepaintNext,

            winit::event::Event::WindowEvent { window_id, .. }
                if winit_app.window().is_none()
                    || *window_id != winit_app.window().unwrap().id() =>
            {
                // This can happen if we close a window, and then reopen a new one,
                // or if we have multiple windows open.
                EventResult::Wait
            }

            event => winit_app.on_event(event_loop, event),
        };

        match event_result {
            EventResult::Wait => {}
            EventResult::RepaintNow => {
                tracing::trace!("Repaint caused by winit::Event: {:?}", event);
                next_repaint_time = Instant::now() + Duration::from_secs(1_000_000_000);
                winit_app.paint();
            }
            EventResult::RepaintNext => {
                tracing::trace!("Repaint caused by winit::Event: {:?}", event);
                next_repaint_time = Instant::now();
            }
            EventResult::RepaintAt(repaint_time) => {
                next_repaint_time = next_repaint_time.min(repaint_time);
            }
            EventResult::Exit => {
                // On Cmd-Q we get here and then `run_return` doesn't return,
                // so we need to save state now:
                tracing::debug!("Exiting event loop - saving app state…");
                winit_app.save_and_destroy();
                *control_flow = ControlFlow::Exit;
                return;
            }
        }

        *control_flow = match next_repaint_time.checked_duration_since(Instant::now()) {
            None => {
                if let Some(window) = winit_app.window() {
                    window.request_redraw();
                }
                next_repaint_time = Instant::now() + Duration::from_secs(1_000_000_000);
                ControlFlow::Poll
            }
            Some(time_until_next_repaint) => {
                ControlFlow::WaitUntil(Instant::now() + time_until_next_repaint)
            }
        }
    });

    tracing::debug!("eframe window closed");

    drop(winit_app);

    // On Windows this clears out events so that we can later create another window.
    // See https://github.com/emilk/egui/pull/1889 for details.
    event_loop.run_return(|_, _, control_flow| {
        control_flow.set_exit();
    });
}

fn run_and_exit(event_loop: EventLoop<UserEvent>, mut winit_app: impl WinitApp + 'static) -> ! {
    tracing::debug!("event_loop.run");

    let mut next_repaint_time = Instant::now();

    event_loop.run(move |event, event_loop, control_flow| {
        let event_result = match event {
            winit::event::Event::LoopDestroyed => EventResult::Exit,

            // Platform-dependent event handlers to workaround a winit bug
            // See: https://github.com/rust-windowing/winit/issues/987
            // See: https://github.com/rust-windowing/winit/issues/1619
            winit::event::Event::RedrawEventsCleared if cfg!(windows) => {
                next_repaint_time = Instant::now() + Duration::from_secs(1_000_000_000);
                winit_app.paint()
            }
            winit::event::Event::RedrawRequested(_) if !cfg!(windows) => {
                next_repaint_time = Instant::now() + Duration::from_secs(1_000_000_000);
                winit_app.paint()
            }

            winit::event::Event::UserEvent(UserEvent::RequestRepaint)
            | winit::event::Event::NewEvents(winit::event::StartCause::ResumeTimeReached {
                ..
            }) => EventResult::RepaintNext,

            event => winit_app.on_event(event_loop, &event),
        };

        match event_result {
            EventResult::Wait => {}
            EventResult::RepaintNow => {
                next_repaint_time = Instant::now() + Duration::from_secs(1_000_000_000);
                winit_app.paint();
            }
            EventResult::RepaintNext => {
                next_repaint_time = Instant::now();
            }
            EventResult::RepaintAt(repaint_time) => {
                next_repaint_time = next_repaint_time.min(repaint_time);
            }
            EventResult::Exit => {
                tracing::debug!("Quitting…");
                winit_app.save_and_destroy();
                #[allow(clippy::exit)]
                std::process::exit(0);
            }
        }

        *control_flow = match next_repaint_time.checked_duration_since(Instant::now()) {
            None => {
                if let Some(window) = winit_app.window() {
                    window.request_redraw();
                }
                ControlFlow::Poll
            }
            Some(time_until_next_repaint) => {
                ControlFlow::WaitUntil(Instant::now() + time_until_next_repaint)
            }
        }
    })
}

fn center_window_pos(
    monitor: Option<winit::monitor::MonitorHandle>,
    native_options: &mut epi::NativeOptions,
) {
    // Get the current_monitor.
    if let Some(monitor) = monitor {
        let monitor_size = monitor.size();
        let inner_size = native_options
            .initial_window_size
            .unwrap_or(egui::Vec2 { x: 800.0, y: 600.0 });
        if monitor_size.width > 0 && monitor_size.height > 0 {
            let x = (monitor_size.width - inner_size.x as u32) / 2;
            let y = (monitor_size.height - inner_size.y as u32) / 2;
            native_options.initial_window_pos = Some(egui::Pos2 {
                x: x as _,
                y: y as _,
            });
        }
    }
}

// ----------------------------------------------------------------------------
/// Run an egui app
#[cfg(feature = "glow")]
mod glow_integration {
    use std::sync::Arc;

    use super::*;

    // Note: that the current Glutin API design tightly couples the GL context with
    // the Window which means it's not practically possible to just destroy the
    // window and re-create a new window while continuing to use the same GL context.
    //
    // For now this means it's not possible to support Android as well as we can with
    // wgpu because we're basically forced to destroy and recreate _everything_ when
    // the application suspends and resumes.
    //
    // There is work in progress to improve the Glutin API so it has a separate Surface
    // API that would allow us to just destroy a Window/Surface when suspending, see:
    // https://github.com/rust-windowing/glutin/pull/1435
    //

    /// State that is initialized when the application is first starts running via
    /// a Resumed event. On Android this ensures that any graphics state is only
    /// initialized once the application has an associated `SurfaceView`.
    struct GlowWinitRunning {
        gl: Arc<glow::Context>,
        painter: egui_glow::Painter,
        integration: epi_integration::EpiIntegration,
        app: Box<dyn epi::App>,

        // Conceptually this will be split out eventually so that the rest of the state
        // can be persistent.
        gl_window: GlutinWindowContext,
    }
    struct GlutinWindowContext {
        window: winit::window::Window,
        gl_context: glutin::context::PossiblyCurrentContext,
        gl_display: glutin::display::Display,
        gl_surface: glutin::surface::Surface<glutin::surface::WindowSurface>,
    }

    impl GlutinWindowContext {
        // refactor this function to use `glutin-winit` crate eventually.
        // preferably add android support at the same time.
        #[allow(unsafe_code)]
        unsafe fn new(
            winit_window: winit::window::Window,
            native_options: &epi::NativeOptions,
        ) -> Self {
            use glutin::prelude::*;
            use raw_window_handle::*;
            let hardware_acceleration = match native_options.hardware_acceleration {
                crate::HardwareAcceleration::Required => Some(true),
                crate::HardwareAcceleration::Preferred => None,
                crate::HardwareAcceleration::Off => Some(false),
            };

            let raw_display_handle = winit_window.raw_display_handle();
            let raw_window_handle = winit_window.raw_window_handle();

            // EGL is crossplatform and the official khronos way
            // but sometimes platforms/drivers may not have it, so we use back up options where possible.
            // TODO: check whether we can expose these options as "features", so that users can select the relevant backend they want.

            // try egl and fallback to windows wgl. Windows is the only platform that *requires* window handle to create display.
            #[cfg(target_os = "windows")]
            let preference =
                glutin::display::DisplayApiPreference::EglThenWgl(Some(raw_window_handle));
            // try egl and fallback to x11 glx
            #[cfg(target_os = "linux")]
            let preference = glutin::display::DisplayApiPreference::EglThenGlx(Box::new(
                winit::platform::unix::register_xlib_error_hook,
            ));
            #[cfg(target_os = "macos")]
            let preference = glutin::display::DisplayApiPreference::Cgl;
            #[cfg(target_os = "android")]
            let preference = glutin::display::DisplayApiPreference::Egl;

            let gl_display = glutin::display::Display::new(raw_display_handle, preference)
                .expect("failed to create glutin display");
            let swap_interval = if native_options.vsync {
                glutin::surface::SwapInterval::Wait(std::num::NonZeroU32::new(1).unwrap())
            } else {
                glutin::surface::SwapInterval::DontWait
            };

            let config_template = glutin::config::ConfigTemplateBuilder::new()
                .prefer_hardware_accelerated(hardware_acceleration)
                .with_depth_size(native_options.depth_buffer);
            // we don't know if multi sampling option is set. so, check if its more than 0.
            let config_template = if native_options.multisampling > 0 {
                config_template.with_multisampling(
                    native_options
                        .multisampling
                        .try_into()
                        .expect("failed to fit multisamples into u8"),
                )
            } else {
                config_template
            };
            let config_template = config_template
                .with_stencil_size(native_options.stencil_buffer)
                .with_transparency(native_options.transparent)
                .compatible_with_native_window(raw_window_handle)
                .build();
            // finds all valid configurations supported by this display that match the config_template
            // this is where we will try to get a "fallback" config if we are okay with ignoring some native
            // options required by user like multi sampling, srgb, transparency etc..
            // TODO: need to figure out a good fallback config template
            let config = gl_display
                .find_configs(config_template)
                .expect("failed to find even a single matching configuration")
                .next()
                .expect("failed to find a matching configuration for creating opengl context");

            let context_attributes =
                glutin::context::ContextAttributesBuilder::new().build(Some(raw_window_handle));
            // for surface creation.
            let (width, height): (u32, u32) = winit_window.inner_size().into();
            let surface_attributes =
                glutin::surface::SurfaceAttributesBuilder::<glutin::surface::WindowSurface>::new()
                    .build(
                        raw_window_handle,
                        std::num::NonZeroU32::new(width).unwrap(),
                        std::num::NonZeroU32::new(height).unwrap(),
                    );
            // start creating the gl objects
            let gl_context = gl_display
                .create_context(&config, &context_attributes)
                .expect("failed to create opengl context");

            let gl_surface = gl_display
                .create_window_surface(&config, &surface_attributes)
                .expect("failed to create glutin window surface");
            let gl_context = gl_context
                .make_current(&gl_surface)
                .expect("failed to make gl context current");
            gl_surface
                .set_swap_interval(&gl_context, swap_interval)
                .expect("failed to set vsync swap interval");
            GlutinWindowContext {
                window: winit_window,
                gl_context,
                gl_display,
                gl_surface,
            }
        }
        fn window(&self) -> &winit::window::Window {
            &self.window
        }
        fn resize(&self, physical_size: winit::dpi::PhysicalSize<u32>) {
            use glutin::surface::GlSurface;
            self.gl_surface.resize(
                &self.gl_context,
                physical_size
                    .width
                    .try_into()
                    .expect("physical size must not be zero"),
                physical_size
                    .height
                    .try_into()
                    .expect("physical size must not be zero"),
            );
        }
        fn swap_buffers(&self) -> glutin::error::Result<()> {
            use glutin::surface::GlSurface;
            self.gl_surface.swap_buffers(&self.gl_context)
        }
        fn get_proc_address(&self, addr: &std::ffi::CStr) -> *const std::ffi::c_void {
            use glutin::display::GlDisplay;
            self.gl_display.get_proc_address(addr)
        }
    }

    struct GlowWinitApp {
        repaint_proxy: Arc<egui::mutex::Mutex<EventLoopProxy<UserEvent>>>,
        app_name: String,
        native_options: epi::NativeOptions,
        running: Option<GlowWinitRunning>,

        // Note that since this `AppCreator` is FnOnce we are currently unable to support
        // re-initializing the `GlowWinitRunning` state on Android if the application
        // suspends and resumes.
        app_creator: Option<epi::AppCreator>,
        is_focused: bool,

        frame_nr: u64,
    }

    impl GlowWinitApp {
        fn new(
            event_loop: &EventLoop<UserEvent>,
            app_name: &str,
            native_options: epi::NativeOptions,
            app_creator: epi::AppCreator,
        ) -> Self {
            Self {
                repaint_proxy: Arc::new(egui::mutex::Mutex::new(event_loop.create_proxy())),
                app_name: app_name.to_owned(),
                native_options,
                running: None,
                app_creator: Some(app_creator),
                is_focused: true,
                frame_nr: 0,
            }
        }

        #[allow(unsafe_code)]
        fn create_glutin_windowed_context(
            event_loop: &EventLoopWindowTarget<UserEvent>,
            storage: Option<&dyn epi::Storage>,
            title: &String,
            native_options: &NativeOptions,
        ) -> (GlutinWindowContext, glow::Context) {
            crate::profile_function!();

            let window_settings = epi_integration::load_window_settings(storage);

            let window_builder = epi_integration::window_builder(native_options, &window_settings)
                .with_title(title)
                .with_transparent(native_options.transparent)
                // Keep hidden until we've painted something. See https://github.com/emilk/egui/pull/2279
                // We must also keep the window hidden until AccessKit is initialized.
                .with_visible(false);
            let winit_window = window_builder
                .build(event_loop)
                .expect("failed to create winit window");
            // a lot of the code below has been lifted from glutin example in their repo.
            let glutin_window_context =
                unsafe { GlutinWindowContext::new(winit_window, native_options) };
            let gl = unsafe {
                glow::Context::from_loader_function(|s| {
                    let s = std::ffi::CString::new(s)
                        .expect("failed to construct C string from string for gl proc address");

                    glutin_window_context.get_proc_address(&s)
                })
            };

            (glutin_window_context, gl)
        }

        fn init_run_state(&mut self, event_loop: &EventLoopWindowTarget<UserEvent>) {
            let storage = epi_integration::create_storage(&self.app_name);

            let (gl_window, gl) = Self::create_glutin_windowed_context(
                event_loop,
                storage.as_deref(),
                &self.app_name,
                &self.native_options,
            );
            let gl = Arc::new(gl);

            let painter =
                egui_glow::Painter::new(gl.clone(), "", self.native_options.shader_version)
                    .unwrap_or_else(|error| panic!("some OpenGL error occurred {}\n", error));

            let system_theme = self.native_options.system_theme();
            let mut integration = epi_integration::EpiIntegration::new(
                event_loop,
                painter.max_texture_side(),
                gl_window.window(),
                system_theme,
                storage,
                Some(gl.clone()),
                #[cfg(feature = "wgpu")]
                None,
            );
            #[cfg(feature = "accesskit")]
            {
                integration.init_accesskit(gl_window.window(), self.repaint_proxy.lock().clone());
            }
            let theme = system_theme.unwrap_or(self.native_options.default_theme);
            integration.egui_ctx.set_visuals(theme.egui_visuals());

            gl_window.window().set_ime_allowed(true);
            if self.native_options.mouse_passthrough {
                gl_window.window().set_cursor_hittest(false).unwrap();
            }

            {
                let event_loop_proxy = self.repaint_proxy.clone();
                integration.egui_ctx.set_request_repaint_callback(move || {
                    event_loop_proxy
                        .lock()
                        .send_event(UserEvent::RequestRepaint)
                        .ok();
                });
            }

            let app_creator = std::mem::take(&mut self.app_creator)
                .expect("Single-use AppCreator has unexpectedly already been taken");
            let mut app = app_creator(&epi::CreationContext {
                egui_ctx: integration.egui_ctx.clone(),
                integration_info: integration.frame.info(),
                storage: integration.frame.storage(),
                gl: Some(gl.clone()),
                #[cfg(feature = "wgpu")]
                wgpu_render_state: None,
            });

            if app.warm_up_enabled() {
                integration.warm_up(app.as_mut(), gl_window.window());
            }

            self.running = Some(GlowWinitRunning {
                gl_window,
                gl,
                painter,
                integration,
                app,
            });
        }
    }

    impl WinitApp for GlowWinitApp {
        fn is_focused(&self) -> bool {
            self.is_focused
        }

        fn integration(&self) -> Option<&EpiIntegration> {
            self.running.as_ref().map(|r| &r.integration)
        }

        fn window(&self) -> Option<&winit::window::Window> {
            self.running.as_ref().map(|r| r.gl_window.window())
        }

        fn save_and_destroy(&mut self) {
            if let Some(mut running) = self.running.take() {
                running
                    .integration
                    .save(running.app.as_mut(), running.gl_window.window());
                running.app.on_exit(Some(&running.gl));
                running.painter.destroy();
            }
        }

        fn paint(&mut self) -> EventResult {
            if let Some(running) = &mut self.running {
                #[cfg(feature = "puffin")]
                puffin::GlobalProfiler::lock().new_frame();
                crate::profile_scope!("frame");

                let GlowWinitRunning {
                    gl_window,
                    gl,
                    app,
                    integration,
                    painter,
                } = running;

                let window = gl_window.window();

                let screen_size_in_pixels: [u32; 2] = window.inner_size().into();

                egui_glow::painter::clear(
                    gl,
                    screen_size_in_pixels,
                    app.clear_color(&integration.egui_ctx.style().visuals),
                );

                let egui::FullOutput {
                    platform_output,
                    repaint_after,
                    textures_delta,
                    shapes,
                } = integration.update(app.as_mut(), window);

                integration.handle_platform_output(window, platform_output);

                let clipped_primitives = {
                    crate::profile_scope!("tessellate");
                    integration.egui_ctx.tessellate(shapes)
                };

                painter.paint_and_update_textures(
                    screen_size_in_pixels,
                    integration.egui_ctx.pixels_per_point(),
                    &clipped_primitives,
                    &textures_delta,
                );

                integration.post_rendering(app.as_mut(), window);

                {
                    crate::profile_scope!("swap_buffers");
                    gl_window.swap_buffers().unwrap();
                }

                integration.post_present(window);

                #[cfg(feature = "__screenshot")]
                // give it time to settle:
                if self.frame_nr == 2 {
                    if let Ok(path) = std::env::var("EFRAME_SCREENSHOT_TO") {
                        assert!(
                            path.ends_with(".png"),
                            "Expected EFRAME_SCREENSHOT_TO to end with '.png', got {path:?}"
                        );
                        let [w, h] = screen_size_in_pixels;
                        let pixels = painter.read_screen_rgba(screen_size_in_pixels);
                        let image = image::RgbaImage::from_vec(w, h, pixels).unwrap();
                        let image = image::imageops::flip_vertical(&image);
                        image.save(&path).unwrap_or_else(|err| {
                            panic!("Failed to save screenshot to {path:?}: {err}");
                        });
                        eprintln!("Screenshot saved to {path:?}.");
                        std::process::exit(0);
                    }
                }

                let control_flow = if integration.should_close() {
                    EventResult::Exit
                } else if repaint_after.is_zero() {
                    EventResult::RepaintNext
                } else if let Some(repaint_after_instant) =
                    std::time::Instant::now().checked_add(repaint_after)
                {
                    // if repaint_after is something huge and can't be added to Instant,
                    // we will use `ControlFlow::Wait` instead.
                    // technically, this might lead to some weird corner cases where the user *WANTS*
                    // winit to use `WaitUntil(MAX_INSTANT)` explicitly. they can roll their own
                    // egui backend impl i guess.
                    EventResult::RepaintAt(repaint_after_instant)
                } else {
                    EventResult::Wait
                };

                integration.maybe_autosave(app.as_mut(), window);

                if !self.is_focused {
                    // On Mac, a minimized Window uses up all CPU: https://github.com/emilk/egui/issues/325
                    // We can't know if we are minimized: https://github.com/rust-windowing/winit/issues/208
                    // But we know if we are focused (in foreground). When minimized, we are not focused.
                    // However, a user may want an egui with an animation in the background,
                    // so we still need to repaint quite fast.
                    crate::profile_scope!("bg_sleep");
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }

                self.frame_nr += 1;

                control_flow
            } else {
                EventResult::Wait
            }
        }

        fn on_event(
            &mut self,
            event_loop: &EventLoopWindowTarget<UserEvent>,
            event: &winit::event::Event<'_, UserEvent>,
        ) -> EventResult {
            match event {
                winit::event::Event::Resumed => {
                    if self.running.is_none() {
                        self.init_run_state(event_loop);
                    }
                    EventResult::RepaintNow
                }
                winit::event::Event::Suspended => {
                    #[cfg(target_os = "android")]
                    {
                        tracing::error!("Suspended app can't destroy Window surface state with current Egui Glow backend (undefined behaviour)");
                        // Instead of destroying everything which we _know_ we can't re-create
                        // we instead currently just try our luck with not destroying anything.
                        //
                        // When the application resumes then it will get a new `SurfaceView` but
                        // we have no practical way currently of creating a new EGL surface
                        // via the Glutin API while keeping the GL context and the rest of
                        // our app state. This will likely result in a black screen or
                        // frozen screen.
                        //
                        //self.running = None;
                    }
                    EventResult::Wait
                }

                winit::event::Event::WindowEvent { event, .. } => {
                    if let Some(running) = &mut self.running {
                        // On Windows, if a window is resized by the user, it should repaint synchronously, inside the
                        // event handler.
                        //
                        // If this is not done, the compositor will assume that the window does not want to redraw,
                        // and continue ahead.
                        //
                        // In eframe's case, that causes the window to rapidly flicker, as it struggles to deliver
                        // new frames to the compositor in time.
                        //
                        // The flickering is technically glutin or glow's fault, but we should be responding properly
                        // to resizes anyway, as doing so avoids dropping frames.
                        //
                        // See: https://github.com/emilk/egui/issues/903
                        let mut repaint_asap = false;

                        match &event {
                            winit::event::WindowEvent::Focused(new_focused) => {
                                self.is_focused = *new_focused;
                            }
                            winit::event::WindowEvent::Resized(physical_size) => {
                                repaint_asap = true;

                                // Resize with 0 width and height is used by winit to signal a minimize event on Windows.
                                // See: https://github.com/rust-windowing/winit/issues/208
                                // This solves an issue where the app would panic when minimizing on Windows.
                                if physical_size.width > 0 && physical_size.height > 0 {
                                    running.gl_window.resize(*physical_size);
                                }
                            }
                            winit::event::WindowEvent::ScaleFactorChanged {
                                new_inner_size,
                                ..
                            } => {
                                repaint_asap = true;
                                running.gl_window.resize(**new_inner_size);
                            }
                            winit::event::WindowEvent::CloseRequested
                                if running.integration.should_close() =>
                            {
                                return EventResult::Exit
                            }
                            _ => {}
                        }

                        let event_response =
                            running.integration.on_event(running.app.as_mut(), event);

                        if running.integration.should_close() {
                            EventResult::Exit
                        } else if event_response.repaint {
                            if repaint_asap {
                                EventResult::RepaintNow
                            } else {
                                EventResult::RepaintNext
                            }
                        } else {
                            EventResult::Wait
                        }
                    } else {
                        EventResult::Wait
                    }
                }
                #[cfg(feature = "accesskit")]
                winit::event::Event::UserEvent(UserEvent::AccessKitActionRequest(
                    accesskit_winit::ActionRequestEvent { request, .. },
                )) => {
                    if let Some(running) = &mut self.running {
                        running
                            .integration
                            .on_accesskit_action_request(request.clone());
                        // As a form of user input, accessibility actions should
                        // lead to a repaint.
                        EventResult::RepaintNext
                    } else {
                        EventResult::Wait
                    }
                }
                _ => EventResult::Wait,
            }
        }
    }

    pub fn run_glow(
        app_name: &str,
        mut native_options: epi::NativeOptions,
        app_creator: epi::AppCreator,
    ) {
        if native_options.run_and_return {
            with_event_loop(native_options, |event_loop, mut native_options| {
                if native_options.centered {
                    center_window_pos(event_loop.available_monitors().next(), &mut native_options);
                }

                let glow_eframe =
                    GlowWinitApp::new(event_loop, app_name, native_options, app_creator);
                run_and_return(event_loop, glow_eframe);
            });
        } else {
            let event_loop = create_event_loop_builder(&mut native_options).build();

            if native_options.centered {
                center_window_pos(event_loop.available_monitors().next(), &mut native_options);
            }

            let glow_eframe = GlowWinitApp::new(&event_loop, app_name, native_options, app_creator);
            run_and_exit(event_loop, glow_eframe);
        }
    }
}

#[cfg(feature = "glow")]
pub use glow_integration::run_glow;
// ----------------------------------------------------------------------------

#[cfg(feature = "wgpu")]
mod wgpu_integration {
    use std::sync::Arc;

    use super::*;

    /// State that is initialized when the application is first starts running via
    /// a Resumed event. On Android this ensures that any graphics state is only
    /// initialized once the application has an associated `SurfaceView`.
    struct WgpuWinitRunning {
        painter: egui_wgpu::winit::Painter,
        integration: epi_integration::EpiIntegration,
        app: Box<dyn epi::App>,
    }

    struct WgpuWinitApp {
        repaint_proxy: Arc<std::sync::Mutex<EventLoopProxy<UserEvent>>>,
        app_name: String,
        native_options: epi::NativeOptions,
        app_creator: Option<epi::AppCreator>,
        running: Option<WgpuWinitRunning>,

        /// Window surface state that's initialized when the app starts running via a Resumed event
        /// and on Android will also be destroyed if the application is paused.
        window: Option<winit::window::Window>,
        is_focused: bool,
    }

    impl WgpuWinitApp {
        fn new(
            event_loop: &EventLoop<UserEvent>,
            app_name: &str,
            native_options: epi::NativeOptions,
            app_creator: epi::AppCreator,
        ) -> Self {
            #[cfg(feature = "__screenshot")]
            assert!(
                std::env::var("EFRAME_SCREENSHOT_TO").is_err(),
                "EFRAME_SCREENSHOT_TO not yet implemented for wgpu backend"
            );

            Self {
                repaint_proxy: Arc::new(std::sync::Mutex::new(event_loop.create_proxy())),
                app_name: app_name.to_owned(),
                native_options,
                running: None,
                window: None,
                app_creator: Some(app_creator),
                is_focused: true,
            }
        }

        fn create_window(
            event_loop: &EventLoopWindowTarget<UserEvent>,
            storage: Option<&dyn epi::Storage>,
            title: &String,
            native_options: &NativeOptions,
        ) -> winit::window::Window {
            let window_settings = epi_integration::load_window_settings(storage);
            epi_integration::window_builder(native_options, &window_settings)
                .with_title(title)
                // Keep hidden until we've painted something. See https://github.com/emilk/egui/pull/2279
                // We must also keep the window hidden until AccessKit is initialized.
                .with_visible(false)
                .build(event_loop)
                .unwrap()
        }

        #[allow(unsafe_code)]
        fn set_window(&mut self, window: winit::window::Window) {
            self.window = Some(window);
            if let Some(running) = &mut self.running {
                unsafe {
                    running.painter.set_window(self.window.as_ref());
                }
            }
        }

        #[allow(unsafe_code)]
        #[cfg(target_os = "android")]
        fn drop_window(&mut self) {
            self.window = None;
            if let Some(running) = &mut self.running {
                unsafe {
                    running.painter.set_window(None);
                }
            }
        }

        fn init_run_state(
            &mut self,
            event_loop: &EventLoopWindowTarget<UserEvent>,
            storage: Option<Box<dyn epi::Storage>>,
            window: winit::window::Window,
        ) {
            #[allow(unsafe_code, unused_mut, unused_unsafe)]
            let painter = unsafe {
                let mut painter = egui_wgpu::winit::Painter::new(
                    self.native_options.wgpu_options.clone(),
                    self.native_options.multisampling.max(1) as _,
                    self.native_options.depth_buffer,
                );
                painter.set_window(Some(&window));
                painter
            };

            let wgpu_render_state = painter.render_state();

            let system_theme = self.native_options.system_theme();
            let mut integration = epi_integration::EpiIntegration::new(
                event_loop,
                painter.max_texture_side().unwrap_or(2048),
                &window,
                system_theme,
                storage,
                #[cfg(feature = "glow")]
                None,
                wgpu_render_state.clone(),
            );
            #[cfg(feature = "accesskit")]
            {
                integration.init_accesskit(&window, self.repaint_proxy.lock().unwrap().clone());
            }
            let theme = system_theme.unwrap_or(self.native_options.default_theme);
            integration.egui_ctx.set_visuals(theme.egui_visuals());

            window.set_ime_allowed(true);

            {
                let event_loop_proxy = self.repaint_proxy.clone();
                integration.egui_ctx.set_request_repaint_callback(move || {
                    event_loop_proxy
                        .lock()
                        .unwrap()
                        .send_event(UserEvent::RequestRepaint)
                        .ok();
                });
            }

            let app_creator = std::mem::take(&mut self.app_creator)
                .expect("Single-use AppCreator has unexpectedly already been taken");
            let mut app = app_creator(&epi::CreationContext {
                egui_ctx: integration.egui_ctx.clone(),
                integration_info: integration.frame.info(),
                storage: integration.frame.storage(),
                #[cfg(feature = "glow")]
                gl: None,
                wgpu_render_state,
            });

            if app.warm_up_enabled() {
                integration.warm_up(app.as_mut(), &window);
            }

            self.running = Some(WgpuWinitRunning {
                painter,
                integration,
                app,
            });
            self.window = Some(window);
        }
    }

    impl WinitApp for WgpuWinitApp {
        fn is_focused(&self) -> bool {
            self.is_focused
        }

        fn integration(&self) -> Option<&EpiIntegration> {
            self.running.as_ref().map(|r| &r.integration)
        }

        fn window(&self) -> Option<&winit::window::Window> {
            self.window.as_ref()
        }

        fn save_and_destroy(&mut self) {
            if let Some(mut running) = self.running.take() {
                if let Some(window) = &self.window {
                    running.integration.save(running.app.as_mut(), window);
                }

                #[cfg(feature = "glow")]
                running.app.on_exit(None);

                #[cfg(not(feature = "glow"))]
                running.app.on_exit();

                running.painter.destroy();
            }
        }

        fn paint(&mut self) -> EventResult {
            if let (Some(running), Some(window)) = (&mut self.running, &self.window) {
                #[cfg(feature = "puffin")]
                puffin::GlobalProfiler::lock().new_frame();
                crate::profile_scope!("frame");

                let WgpuWinitRunning {
                    app,
                    integration,
                    painter,
                } = running;

                let egui::FullOutput {
                    platform_output,
                    repaint_after,
                    textures_delta,
                    shapes,
                } = integration.update(app.as_mut(), window);

                integration.handle_platform_output(window, platform_output);

                let clipped_primitives = {
                    crate::profile_scope!("tessellate");
                    integration.egui_ctx.tessellate(shapes)
                };

                painter.paint_and_update_textures(
                    integration.egui_ctx.pixels_per_point(),
                    app.clear_color(&integration.egui_ctx.style().visuals),
                    &clipped_primitives,
                    &textures_delta,
                );

                integration.post_rendering(app.as_mut(), window);
                integration.post_present(window);

                let control_flow = if integration.should_close() {
                    EventResult::Exit
                } else if repaint_after.is_zero() {
                    EventResult::RepaintNext
                } else if let Some(repaint_after_instant) =
                    std::time::Instant::now().checked_add(repaint_after)
                {
                    // if repaint_after is something huge and can't be added to Instant,
                    // we will use `ControlFlow::Wait` instead.
                    // technically, this might lead to some weird corner cases where the user *WANTS*
                    // winit to use `WaitUntil(MAX_INSTANT)` explicitly. they can roll their own
                    // egui backend impl i guess.
                    EventResult::RepaintAt(repaint_after_instant)
                } else {
                    EventResult::Wait
                };

                integration.maybe_autosave(app.as_mut(), window);

                if !self.is_focused {
                    // On Mac, a minimized Window uses up all CPU: https://github.com/emilk/egui/issues/325
                    // We can't know if we are minimized: https://github.com/rust-windowing/winit/issues/208
                    // But we know if we are focused (in foreground). When minimized, we are not focused.
                    // However, a user may want an egui with an animation in the background,
                    // so we still need to repaint quite fast.
                    crate::profile_scope!("bg_sleep");
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }

                control_flow
            } else {
                EventResult::Wait
            }
        }

        fn on_event(
            &mut self,
            event_loop: &EventLoopWindowTarget<UserEvent>,
            event: &winit::event::Event<'_, UserEvent>,
        ) -> EventResult {
            match event {
                winit::event::Event::Resumed => {
                    if let Some(running) = &self.running {
                        if self.window.is_none() {
                            let window = Self::create_window(
                                event_loop,
                                running.integration.frame.storage(),
                                &self.app_name,
                                &self.native_options,
                            );
                            self.set_window(window);
                        }
                    } else {
                        let storage = epi_integration::create_storage(&self.app_name);
                        let window = Self::create_window(
                            event_loop,
                            storage.as_deref(),
                            &self.app_name,
                            &self.native_options,
                        );
                        self.init_run_state(event_loop, storage, window);
                    }
                    EventResult::RepaintNow
                }
                winit::event::Event::Suspended => {
                    #[cfg(target_os = "android")]
                    self.drop_window();
                    EventResult::Wait
                }

                winit::event::Event::WindowEvent { event, .. } => {
                    if let Some(running) = &mut self.running {
                        // On Windows, if a window is resized by the user, it should repaint synchronously, inside the
                        // event handler.
                        //
                        // If this is not done, the compositor will assume that the window does not want to redraw,
                        // and continue ahead.
                        //
                        // In eframe's case, that causes the window to rapidly flicker, as it struggles to deliver
                        // new frames to the compositor in time.
                        //
                        // The flickering is technically glutin or glow's fault, but we should be responding properly
                        // to resizes anyway, as doing so avoids dropping frames.
                        //
                        // See: https://github.com/emilk/egui/issues/903
                        let mut repaint_asap = false;

                        match &event {
                            winit::event::WindowEvent::Focused(new_focused) => {
                                self.is_focused = *new_focused;
                            }
                            winit::event::WindowEvent::Resized(physical_size) => {
                                repaint_asap = true;

                                // Resize with 0 width and height is used by winit to signal a minimize event on Windows.
                                // See: https://github.com/rust-windowing/winit/issues/208
                                // This solves an issue where the app would panic when minimizing on Windows.
                                if physical_size.width > 0 && physical_size.height > 0 {
                                    running.painter.on_window_resized(
                                        physical_size.width,
                                        physical_size.height,
                                    );
                                }
                            }
                            winit::event::WindowEvent::ScaleFactorChanged {
                                new_inner_size,
                                ..
                            } => {
                                repaint_asap = true;
                                running
                                    .painter
                                    .on_window_resized(new_inner_size.width, new_inner_size.height);
                            }
                            winit::event::WindowEvent::CloseRequested
                                if running.integration.should_close() =>
                            {
                                return EventResult::Exit
                            }
                            _ => {}
                        };

                        let event_response =
                            running.integration.on_event(running.app.as_mut(), event);
                        if running.integration.should_close() {
                            EventResult::Exit
                        } else if event_response.repaint {
                            if repaint_asap {
                                EventResult::RepaintNow
                            } else {
                                EventResult::RepaintNext
                            }
                        } else {
                            EventResult::Wait
                        }
                    } else {
                        EventResult::Wait
                    }
                }
                #[cfg(feature = "accesskit")]
                winit::event::Event::UserEvent(UserEvent::AccessKitActionRequest(
                    accesskit_winit::ActionRequestEvent { request, .. },
                )) => {
                    if let Some(running) = &mut self.running {
                        running
                            .integration
                            .on_accesskit_action_request(request.clone());
                        // As a form of user input, accessibility actions should
                        // lead to a repaint.
                        EventResult::RepaintNext
                    } else {
                        EventResult::Wait
                    }
                }
                _ => EventResult::Wait,
            }
        }
    }

    pub fn run_wgpu(
        app_name: &str,
        mut native_options: epi::NativeOptions,
        app_creator: epi::AppCreator,
    ) {
        if native_options.run_and_return {
            with_event_loop(native_options, |event_loop, mut native_options| {
                if native_options.centered {
                    center_window_pos(event_loop.available_monitors().next(), &mut native_options);
                }

                let wgpu_eframe =
                    WgpuWinitApp::new(event_loop, app_name, native_options, app_creator);
                run_and_return(event_loop, wgpu_eframe);
            });
        } else {
            let event_loop = create_event_loop_builder(&mut native_options).build();

            if native_options.centered {
                center_window_pos(event_loop.available_monitors().next(), &mut native_options);
            }

            let wgpu_eframe = WgpuWinitApp::new(&event_loop, app_name, native_options, app_creator);
            run_and_exit(event_loop, wgpu_eframe);
        }
    }
}

// ----------------------------------------------------------------------------

#[cfg(feature = "wgpu")]
pub use wgpu_integration::run_wgpu;
