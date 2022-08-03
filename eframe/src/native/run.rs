//! Note that this file contains two similar paths - one for [`glow`], one for [`wgpu`].
//! When making changes to one you often also want to apply it to the other.

use std::sync::Arc;

use egui_winit::winit;
use winit::event_loop::{ControlFlow, EventLoop};

use super::epi_integration;
use crate::epi;

#[derive(Debug)]
struct RequestRepaintEvent;

#[cfg(feature = "glow")]
#[allow(unsafe_code)]
fn create_display(
    native_options: &NativeOptions,
    window_builder: winit::window::WindowBuilder,
    event_loop: &EventLoop<RequestRepaintEvent>,
) -> (
    glutin::WindowedContext<glutin::PossiblyCurrent>,
    glow::Context,
) {
    crate::profile_function!();

    use crate::HardwareAcceleration;

    let hardware_acceleration = match native_options.hardware_acceleration {
        HardwareAcceleration::Required => Some(true),
        HardwareAcceleration::Preferred => None,
        HardwareAcceleration::Off => Some(false),
    };

    let gl_window = unsafe {
        glutin::ContextBuilder::new()
            .with_hardware_acceleration(hardware_acceleration)
            .with_depth_buffer(native_options.depth_buffer)
            .with_multisampling(native_options.multisampling)
            .with_srgb(true)
            .with_stencil_buffer(native_options.stencil_buffer)
            .with_vsync(native_options.vsync)
            .build_windowed(window_builder, event_loop)
            .unwrap()
            .make_current()
            .unwrap()
    };

    let gl = unsafe { glow::Context::from_loader_function(|s| gl_window.get_proc_address(s)) };

    (gl_window, gl)
}

// ----------------------------------------------------------------------------

pub use epi::NativeOptions;

enum EventResult {
    Repaint,
    Exit,
    Continue,
}

/// Run an egui app
#[cfg(feature = "glow")]
mod glow_integration {
    use std::time::{Duration, Instant};

    use super::*;

    struct GlowEframe {
        gl_window: glutin::WindowedContext<glutin::PossiblyCurrent>,
        gl: Arc<glow::Context>,
        painter: egui_glow::Painter,
        integration: epi_integration::EpiIntegration,
        app: Box<dyn epi::App>,
        is_focused: bool,
    }

    impl GlowEframe {
        fn new(
            event_loop: &EventLoop<RequestRepaintEvent>,
            app_name: &str,
            native_options: &epi::NativeOptions,
            app_creator: epi::AppCreator,
        ) -> Self {
            let storage = epi_integration::create_storage(app_name);
            let window_settings = epi_integration::load_window_settings(storage.as_deref());

            let window_builder = epi_integration::window_builder(native_options, &window_settings)
                .with_title(app_name);
            let (gl_window, gl) = create_display(native_options, window_builder, event_loop);
            let gl = Arc::new(gl);

            let painter = egui_glow::Painter::new(gl.clone(), None, "")
                .unwrap_or_else(|error| panic!("some OpenGL error occurred {}\n", error));

            let system_theme = native_options.system_theme();
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
            let theme = system_theme.unwrap_or(native_options.default_theme);
            integration.egui_ctx.set_visuals(theme.egui_visuals());

            {
                let event_loop_proxy = egui::mutex::Mutex::new(event_loop.create_proxy());
                integration.egui_ctx.set_request_repaint_callback(move || {
                    event_loop_proxy.lock().send_event(RequestRepaintEvent).ok();
                });
            }

            let mut app = app_creator(&epi::CreationContext {
                egui_ctx: integration.egui_ctx.clone(),
                integration_info: integration.frame.info(),
                storage: integration.frame.storage(),
                gl: Some(gl.clone()),
                #[cfg(feature = "wgpu")]
                render_state: None,
            });

            if app.warm_up_enabled() {
                integration.warm_up(app.as_mut(), gl_window.window());
            }

            Self {
                gl_window,
                gl,
                painter,
                integration,
                app,
                is_focused: true,
            }
        }

        fn save_and_destroy(&mut self) {
            self.integration
                .save(&mut *self.app, self.gl_window.window());
            self.app.on_exit(Some(&self.gl));
            self.painter.destroy();
        }

        fn paint(&mut self) -> ControlFlow {
            #[cfg(feature = "puffin")]
            puffin::GlobalProfiler::lock().new_frame();
            crate::profile_scope!("frame");

            let Self {
                gl_window,
                gl,
                app,
                integration,
                painter,
                ..
            } = self;
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

            let control_flow = if integration.should_quit() {
                ControlFlow::Exit
            } else if repaint_after.is_zero() {
                window.request_redraw();
                ControlFlow::Poll
            } else if let Some(repaint_after_instant) =
                std::time::Instant::now().checked_add(repaint_after)
            {
                // if repaint_after is something huge and can't be added to Instant,
                // we will use `ControlFlow::Wait` instead.
                // technically, this might lead to some weird corner cases where the user *WANTS*
                // winit to use `WaitUntil(MAX_INSTANT)` explicitly. they can roll their own
                // egui backend impl i guess.
                ControlFlow::WaitUntil(repaint_after_instant)
            } else {
                ControlFlow::Wait
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
        }

        fn on_event(&mut self, event: winit::event::Event<'_, RequestRepaintEvent>) -> EventResult {
            match event {
                // Platform-dependent event handlers to workaround a winit bug
                // See: https://github.com/rust-windowing/winit/issues/987
                // See: https://github.com/rust-windowing/winit/issues/1619
                winit::event::Event::RedrawEventsCleared if cfg!(windows) => EventResult::Repaint,
                winit::event::Event::RedrawRequested(_) if !cfg!(windows) => EventResult::Repaint,

                winit::event::Event::WindowEvent { event, .. } => {
                    match &event {
                        winit::event::WindowEvent::Focused(new_focused) => {
                            self.is_focused = *new_focused;
                        }
                        winit::event::WindowEvent::Resized(physical_size) => {
                            // Resize with 0 width and height is used by winit to signal a minimize event on Windows.
                            // See: https://github.com/rust-windowing/winit/issues/208
                            // This solves an issue where the app would panic when minimizing on Windows.
                            if physical_size.width > 0 && physical_size.height > 0 {
                                self.gl_window.resize(*physical_size);
                            }
                        }
                        winit::event::WindowEvent::ScaleFactorChanged {
                            new_inner_size, ..
                        } => {
                            self.gl_window.resize(**new_inner_size);
                        }
                        winit::event::WindowEvent::CloseRequested
                            if self.integration.should_quit() =>
                        {
                            return EventResult::Exit
                        }
                        _ => {}
                    }

                    self.integration.on_event(self.app.as_mut(), &event);

                    if self.integration.should_quit() {
                        EventResult::Exit
                    } else {
                        self.gl_window.window().request_redraw(); // TODO(emilk): ask egui if the event warrants a repaint
                        EventResult::Continue
                    }
                }
                winit::event::Event::LoopDestroyed => {
                    unreachable!("Should be handled outside this function!")
                }
                winit::event::Event::UserEvent(RequestRepaintEvent)
                | winit::event::Event::NewEvents(winit::event::StartCause::ResumeTimeReached {
                    ..
                }) => {
                    self.gl_window.window().request_redraw();
                    EventResult::Continue
                }
                _ => EventResult::Continue,
            }
        }
    }

    pub fn run_glow(
        app_name: &str,
        native_options: &epi::NativeOptions,
        app_creator: epi::AppCreator,
    ) {
        let event_loop = EventLoop::with_user_event();
        let glow_eframe = GlowEframe::new(&event_loop, app_name, native_options, app_creator);

        if native_options.exit_on_window_close {
            run_then_exit(event_loop, glow_eframe);
        } else {
            run_and_continue(event_loop, glow_eframe);
        }
    }

    fn suggest_sleep_duration(glow_eframe: &GlowEframe) -> Duration {
        if glow_eframe.is_focused || glow_eframe.integration.files_are_hovering() {
            Duration::from_millis(10)
        } else {
            Duration::from_millis(50)
        }
    }

    fn run_and_continue(
        mut event_loop: EventLoop<RequestRepaintEvent>,
        mut glow_eframe: GlowEframe,
    ) {
        let mut running = true;
        let mut needs_repaint_by = Instant::now();

        while running {
            use winit::platform::run_return::EventLoopExtRunReturn as _;
            event_loop.run_return(|event, _, control_flow| {
                *control_flow = match event {
                    winit::event::Event::LoopDestroyed => ControlFlow::Exit,
                    winit::event::Event::MainEventsCleared => ControlFlow::Wait,
                    event => {
                        let event_result = glow_eframe.on_event(event);
                        match event_result {
                            EventResult::Continue => ControlFlow::Wait,
                            EventResult::Repaint => {
                                needs_repaint_by = Instant::now();
                                ControlFlow::Exit
                            }
                            EventResult::Exit => {
                                running = false;
                                ControlFlow::Exit
                            }
                        }
                    }
                };

                match needs_repaint_by.checked_duration_since(Instant::now()) {
                    None => {
                        *control_flow = ControlFlow::Exit; // Time to redraw
                    }
                    Some(duration_until_repaint) => {
                        if *control_flow == ControlFlow::Wait {
                            // On Mac, ControlFlow::WaitUntil doesn't sleep enough. It uses a lot of CPU.
                            // So we sleep manually. But, it still uses 1-3% CPU :(
                            let sleep_duration =
                                duration_until_repaint.min(suggest_sleep_duration(&glow_eframe));
                            std::thread::sleep(sleep_duration);

                            *control_flow = ControlFlow::WaitUntil(needs_repaint_by);
                        }
                    }
                }
            });

            if running && needs_repaint_by <= Instant::now() {
                let paint_result = glow_eframe.paint();
                match paint_result {
                    ControlFlow::Poll => {
                        needs_repaint_by = Instant::now();
                    }
                    ControlFlow::Wait => {
                        // wait a long time unless something happens
                        needs_repaint_by = Instant::now() + Duration::from_secs(3600);
                    }
                    ControlFlow::WaitUntil(repaint_time) => {
                        needs_repaint_by = repaint_time;
                    }
                    ControlFlow::Exit => {
                        running = false;
                    }
                }
            }
        }
        glow_eframe.save_and_destroy();
    }

    fn run_then_exit(event_loop: EventLoop<RequestRepaintEvent>, mut glow_eframe: GlowEframe) -> ! {
        event_loop.run(move |event, _, control_flow| {
            if let winit::event::Event::LoopDestroyed = event {
                glow_eframe.save_and_destroy();
            } else {
                let event_result = glow_eframe.on_event(event);
                match event_result {
                    EventResult::Continue => {}
                    EventResult::Repaint => {
                        *control_flow = glow_eframe.paint();
                    }
                    EventResult::Exit => {
                        *control_flow = ControlFlow::Exit;
                    }
                }
            }
        })
    }
}

#[cfg(feature = "glow")]
pub use glow_integration::run_glow;

// ----------------------------------------------------------------------------

#[cfg(feature = "wgpu")]
mod wgpu_integration {
    use std::time::{Duration, Instant};

    use super::*;

    struct WgpuEframe {
        window: winit::window::Window,
        painter: egui_wgpu::winit::Painter<'static>,
        integration: epi_integration::EpiIntegration,
        app: Box<dyn epi::App>,
        is_focused: bool,
    }

    impl WgpuEframe {
        fn new(
            event_loop: &EventLoop<RequestRepaintEvent>,
            app_name: &str,
            native_options: &epi::NativeOptions,
            app_creator: epi::AppCreator,
        ) -> Self {
            let storage = epi_integration::create_storage(app_name);
            let window_settings = epi_integration::load_window_settings(storage.as_deref());

            let window = epi_integration::window_builder(native_options, &window_settings)
                .with_title(app_name)
                .build(event_loop)
                .unwrap();

            // SAFETY: `window` must outlive `painter`.
            #[allow(unsafe_code)]
            let painter = unsafe {
                let mut painter = egui_wgpu::winit::Painter::new(
                    wgpu::Backends::PRIMARY | wgpu::Backends::GL,
                    wgpu::PowerPreference::HighPerformance,
                    wgpu::DeviceDescriptor {
                        label: None,
                        features: wgpu::Features::default(),
                        limits: wgpu::Limits::default(),
                    },
                    wgpu::PresentMode::Fifo,
                    native_options.multisampling.max(1) as _,
                );
                #[cfg(not(target_os = "android"))]
                painter.set_window(Some(&window));
                painter
            };

            let render_state = painter.get_render_state().expect("Uninitialized");

            let system_theme = native_options.system_theme();
            let mut integration = epi_integration::EpiIntegration::new(
                event_loop,
                painter.max_texture_side().unwrap_or(2048),
                &window,
                system_theme,
                storage,
                #[cfg(feature = "glow")]
                None,
                Some(render_state.clone()),
            );
            let theme = system_theme.unwrap_or(native_options.default_theme);
            integration.egui_ctx.set_visuals(theme.egui_visuals());

            {
                let event_loop_proxy = egui::mutex::Mutex::new(event_loop.create_proxy());
                integration.egui_ctx.set_request_repaint_callback(move || {
                    event_loop_proxy.lock().send_event(RequestRepaintEvent).ok();
                });
            }

            let mut app = app_creator(&epi::CreationContext {
                egui_ctx: integration.egui_ctx.clone(),
                integration_info: integration.frame.info(),
                storage: integration.frame.storage(),
                #[cfg(feature = "glow")]
                gl: None,
                render_state: Some(render_state),
            });

            if app.warm_up_enabled() {
                integration.warm_up(app.as_mut(), &window);
            }

            Self {
                window,
                painter,
                integration,
                app,
                is_focused: true,
            }
        }

        fn save_and_destroy(&mut self) {
            self.integration.save(&mut *self.app, &self.window);

            #[cfg(feature = "glow")]
            self.app.on_exit(None);

            #[cfg(not(feature = "glow"))]
            self.app.on_exit();

            self.painter.destroy();
        }

        fn paint(&mut self) -> ControlFlow {
            #[cfg(feature = "puffin")]
            puffin::GlobalProfiler::lock().new_frame();
            crate::profile_scope!("frame");

            let Self {
                window,
                app,
                integration,
                painter,
                ..
            } = self;

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

            let control_flow = if integration.should_quit() {
                ControlFlow::Exit
            } else if repaint_after.is_zero() {
                window.request_redraw();
                ControlFlow::Poll
            } else if let Some(repaint_after_instant) =
                std::time::Instant::now().checked_add(repaint_after)
            {
                // if repaint_after is something huge and can't be added to Instant,
                // we will use `ControlFlow::Wait` instead.
                // technically, this might lead to some weird corner cases where the user *WANTS*
                // winit to use `WaitUntil(MAX_INSTANT)` explicitly. they can roll their own
                // egui backend impl i guess.
                ControlFlow::WaitUntil(repaint_after_instant)
            } else {
                ControlFlow::Wait
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
        }

        fn on_event(&mut self, event: winit::event::Event<'_, RequestRepaintEvent>) -> EventResult {
            match event {
                // Platform-dependent event handlers to workaround a winit bug
                // See: https://github.com/rust-windowing/winit/issues/987
                // See: https://github.com/rust-windowing/winit/issues/1619
                winit::event::Event::RedrawEventsCleared if cfg!(windows) => EventResult::Repaint,
                winit::event::Event::RedrawRequested(_) if !cfg!(windows) => EventResult::Repaint,

                #[cfg(target_os = "android")]
                winit::event::Event::Resumed => unsafe {
                    painter.set_window(Some(&window));
                },
                #[cfg(target_os = "android")]
                winit::event::Event::Paused => unsafe {
                    painter.set_window(None);
                },

                winit::event::Event::WindowEvent { event, .. } => {
                    match &event {
                        winit::event::WindowEvent::Focused(new_focused) => {
                            self.is_focused = *new_focused;
                        }
                        winit::event::WindowEvent::Resized(physical_size) => {
                            // Resize with 0 width and height is used by winit to signal a minimize event on Windows.
                            // See: https://github.com/rust-windowing/winit/issues/208
                            // This solves an issue where the app would panic when minimizing on Windows.
                            if physical_size.width > 0 && physical_size.height > 0 {
                                self.painter
                                    .on_window_resized(physical_size.width, physical_size.height);
                            }
                        }
                        winit::event::WindowEvent::ScaleFactorChanged {
                            new_inner_size, ..
                        } => {
                            self.painter
                                .on_window_resized(new_inner_size.width, new_inner_size.height);
                        }
                        winit::event::WindowEvent::CloseRequested
                            if self.integration.should_quit() =>
                        {
                            return EventResult::Exit
                        }
                        _ => {}
                    };

                    self.integration.on_event(self.app.as_mut(), &event);
                    if self.integration.should_quit() {
                        EventResult::Exit
                    } else {
                        self.window.request_redraw(); // TODO(emilk): ask egui if the event warrants a repaint
                        EventResult::Continue
                    }
                }
                winit::event::Event::LoopDestroyed => {
                    unreachable!("Should be handled outside this function!")
                }
                winit::event::Event::UserEvent(RequestRepaintEvent)
                | winit::event::Event::NewEvents(winit::event::StartCause::ResumeTimeReached {
                    ..
                }) => {
                    self.window.request_redraw();
                    EventResult::Continue
                }
                _ => EventResult::Continue,
            }
        }
    }

    pub fn run_wgpu(
        app_name: &str,
        native_options: &epi::NativeOptions,
        app_creator: epi::AppCreator,
    ) {
        let event_loop = EventLoop::with_user_event();
        let wgpu_eframe = WgpuEframe::new(&event_loop, app_name, native_options, app_creator);

        if native_options.exit_on_window_close {
            run_then_exit(event_loop, wgpu_eframe);
        } else {
            run_and_continue(event_loop, wgpu_eframe);
        }
    }

    fn suggest_sleep_duration(wgpu_eframe: &WgpuEframe) -> Duration {
        if wgpu_eframe.is_focused || wgpu_eframe.integration.files_are_hovering() {
            Duration::from_millis(10)
        } else {
            Duration::from_millis(50)
        }
    }

    fn run_and_continue(
        mut event_loop: EventLoop<RequestRepaintEvent>,
        mut wgpu_eframe: WgpuEframe,
    ) {
        let mut running = true;
        let mut needs_repaint_by = Instant::now();

        while running {
            use winit::platform::run_return::EventLoopExtRunReturn as _;
            event_loop.run_return(|event, _, control_flow| {
                *control_flow = match event {
                    winit::event::Event::LoopDestroyed => ControlFlow::Exit,
                    winit::event::Event::MainEventsCleared => ControlFlow::Wait,
                    event => {
                        let event_result = wgpu_eframe.on_event(event);
                        match event_result {
                            EventResult::Continue => ControlFlow::Wait,
                            EventResult::Repaint => {
                                needs_repaint_by = Instant::now();
                                ControlFlow::Exit
                            }
                            EventResult::Exit => {
                                running = false;
                                ControlFlow::Exit
                            }
                        }
                    }
                };

                match needs_repaint_by.checked_duration_since(Instant::now()) {
                    None => {
                        *control_flow = ControlFlow::Exit; // Time to redraw
                    }
                    Some(duration_until_repaint) => {
                        if *control_flow == ControlFlow::Wait {
                            // On Mac, ControlFlow::WaitUntil doesn't sleep enough. It uses a lot of CPU.
                            // So we sleep manually. But, it still uses 1-3% CPU :(
                            let sleep_duration =
                                duration_until_repaint.min(suggest_sleep_duration(&wgpu_eframe));
                            std::thread::sleep(sleep_duration);

                            *control_flow = ControlFlow::WaitUntil(needs_repaint_by);
                        }
                    }
                }
            });

            if running && needs_repaint_by <= Instant::now() {
                let paint_result = wgpu_eframe.paint();
                match paint_result {
                    ControlFlow::Poll => {
                        needs_repaint_by = Instant::now();
                    }
                    ControlFlow::Wait => {
                        // wait a long time unless something happens
                        needs_repaint_by = Instant::now() + Duration::from_secs(3600);
                    }
                    ControlFlow::WaitUntil(repaint_time) => {
                        needs_repaint_by = repaint_time;
                    }
                    ControlFlow::Exit => {
                        running = false;
                    }
                }
            }
        }
        wgpu_eframe.save_and_destroy();
    }

    fn run_then_exit(event_loop: EventLoop<RequestRepaintEvent>, mut wgpu_eframe: WgpuEframe) -> ! {
        event_loop.run(move |event, _, control_flow| {
            if let winit::event::Event::LoopDestroyed = event {
                wgpu_eframe.save_and_destroy();
            } else {
                let event_result = wgpu_eframe.on_event(event);
                match event_result {
                    EventResult::Continue => {}
                    EventResult::Repaint => {
                        *control_flow = wgpu_eframe.paint();
                    }
                    EventResult::Exit => {
                        *control_flow = ControlFlow::Exit;
                    }
                }
            }
        })
    }
}

// ----------------------------------------------------------------------------

#[cfg(feature = "wgpu")]
pub use wgpu_integration::run_wgpu;
