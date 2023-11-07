//! Note that this file contains two similar paths - one for [`glow`], one for [`wgpu`].
//! When making changes to one you often also want to apply it to the other.

use std::{cell::RefCell, rc::Rc, sync::Arc, time::Instant};

use raw_window_handle::{HasRawDisplayHandle as _, HasRawWindowHandle as _};
use winit::event_loop::{
    ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget,
};

use egui::{epaint::ahash::HashMap, ViewportBuilder, ViewportId};

#[cfg(feature = "accesskit")]
use egui_winit::accesskit_winit;
use egui_winit::winit;

use crate::{epi, Result};

use super::epi_integration::{self, EpiIntegration};

// ----------------------------------------------------------------------------

pub const IS_DESKTOP: bool = cfg!(any(
    target_os = "freebsd",
    target_os = "linux",
    target_os = "macos",
    target_os = "openbsd",
    target_os = "windows",
));

// ----------------------------------------------------------------------------

thread_local! {
    /// This makes `Context::create_viewport_sync` to have a native window in the same frame!
    pub static WINIT_EVENT_LOOP: RefCell<*const EventLoopWindowTarget<UserEvent>> = RefCell::new(std::ptr::null());
}

// ----------------------------------------------------------------------------

/// The custom even `eframe` uses with the [`winit`] event loop.
#[derive(Debug)]
pub enum UserEvent {
    /// A repaint is requested.
    RequestRepaint {
        /// What to repaint.
        viewport_id: ViewportId,

        /// When to repaint.
        when: Instant,

        /// What the frame number was when the repaint was _requested_.
        frame_nr: u64,
    },

    /// A request related to [`accesskit`](https://accesskit.dev/).
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
    RepaintNow(winit::window::WindowId),

    /// Queues a repaint for once the event loop handles its next redraw. Exists
    /// so that multiple input events can be handled in one frame. Does not
    /// cause any delay like `RepaintNow`.
    RepaintNext(winit::window::WindowId),

    RepaintAt(winit::window::WindowId, Instant),

    Exit,
}

trait WinitApp {
    /// The current frame number, as reported by egui.
    fn frame_nr(&self, viewport_id: ViewportId) -> u64;

    fn is_focused(&self, window_id: winit::window::WindowId) -> bool;

    fn integration(&self) -> Option<Rc<RefCell<EpiIntegration>>>;

    fn window(
        &self,
        window_id: winit::window::WindowId,
    ) -> Option<Rc<RefCell<winit::window::Window>>>;

    fn window_id_from_viewport_id(&self, id: ViewportId) -> Option<winit::window::WindowId>;

    fn viewport_id_from_window_id(&self, id: &winit::window::WindowId) -> Option<ViewportId>;

    fn save_and_destroy(&mut self);

    fn run_ui_and_paint(&mut self, window_id: winit::window::WindowId) -> EventResult;

    fn on_event(
        &mut self,
        event_loop: &EventLoopWindowTarget<UserEvent>,
        event: &winit::event::Event<'_, UserEvent>,
    ) -> Result<EventResult>;
}

fn create_event_loop_builder(
    native_options: &mut epi::NativeOptions,
) -> EventLoopBuilder<UserEvent> {
    crate::profile_function!();
    let mut event_loop_builder = winit::event_loop::EventLoopBuilder::with_user_event();

    if let Some(hook) = std::mem::take(&mut native_options.event_loop_builder) {
        hook(&mut event_loop_builder);
    }

    event_loop_builder
}

fn create_event_loop(native_options: &mut epi::NativeOptions) -> EventLoop<UserEvent> {
    crate::profile_function!();
    let mut builder = create_event_loop_builder(native_options);

    crate::profile_scope!("EventLoopBuilder::build");
    builder.build()
}

/// Access a thread-local event loop.
///
/// We reuse the event-loop so we can support closing and opening an eframe window
/// multiple times. This is just a limitation of winit.
fn with_event_loop<R>(
    mut native_options: epi::NativeOptions,
    f: impl FnOnce(&mut EventLoop<UserEvent>, NativeOptions) -> R,
) -> R {
    thread_local!(static EVENT_LOOP: RefCell<Option<EventLoop<UserEvent>>> = RefCell::new(None));

    EVENT_LOOP.with(|event_loop| {
        // Since we want to reference NativeOptions when creating the EventLoop we can't
        // do that as part of the lazy thread local storage initialization and so we instead
        // create the event loop lazily here
        let mut event_loop = event_loop.borrow_mut();
        let event_loop = event_loop.get_or_insert_with(|| create_event_loop(&mut native_options));
        f(event_loop, native_options)
    })
}

#[cfg(not(target_os = "ios"))]
fn run_and_return(
    event_loop: &mut EventLoop<UserEvent>,
    mut winit_app: impl WinitApp,
) -> Result<()> {
    use winit::platform::run_return::EventLoopExtRunReturn as _;

    log::debug!("Entering the winit event loop (run_return)…");

    let mut windows_next_repaint_times = HashMap::default();
    let mut next_repaint_time = Option::<Instant>::None;

    let mut returned_result = Ok(());

    event_loop.run_return(|event, event_loop, control_flow| {
        crate::profile_scope!("winit_event", short_event_description(&event));

        WINIT_EVENT_LOOP.with(|row_event_loop| row_event_loop.replace(event_loop));

        let event_result = match &event {
            winit::event::Event::LoopDestroyed => {
                // On Mac, Cmd-Q we get here and then `run_return` doesn't return (despite its name),
                // so we need to save state now:
                log::debug!("Received Event::LoopDestroyed - saving app state…");
                winit_app.save_and_destroy();
                *control_flow = ControlFlow::Exit;
                return;
            }

            winit::event::Event::RedrawRequested(window_id) => {
                windows_next_repaint_times.remove(window_id);
                winit_app.run_ui_and_paint(*window_id)
            }

            winit::event::Event::UserEvent(UserEvent::RequestRepaint {
                when,
                frame_nr,
                viewport_id,
            }) => {
                let current_frame_nr = winit_app.frame_nr(*viewport_id);
                if current_frame_nr == *frame_nr || current_frame_nr == *frame_nr + 1 {
                    log::trace!("UserEvent::RequestRepaint scheduling repaint at {when:?}");
                    if let Some(window_id) = winit_app.window_id_from_viewport_id(*viewport_id) {
                        EventResult::RepaintAt(window_id, *when)
                    } else {
                        EventResult::Wait
                    }
                } else {
                    log::trace!("Got outdated UserEvent::RequestRepaint");
                    EventResult::Wait // old request - we've already repainted
                }
            }

            winit::event::Event::NewEvents(winit::event::StartCause::ResumeTimeReached {
                ..
            }) => {
                log::trace!("Woke up to check next_repaint_time");
                EventResult::Wait
            }

            winit::event::Event::WindowEvent { window_id, .. }
                if winit_app.window(*window_id).is_none() =>
            {
                // This can happen if we close a window, and then reopen a new one,
                // or if we have multiple windows open.
                EventResult::RepaintNext(*window_id)
            }

            event => match winit_app.on_event(event_loop, event) {
                Ok(event_result) => event_result,
                Err(err) => {
                    log::error!("Exiting because of error: {err:?} on event {event:?}");
                    returned_result = Err(err);
                    EventResult::Exit
                }
            },
        };

        match event_result {
            EventResult::Wait => {
                control_flow.set_wait();
            }
            EventResult::RepaintNow(window_id) => {
                log::trace!("Repaint caused by winit::Event: {:?}", event_result);
                if cfg!(target_os = "windows") {
                    // Fix flickering on Windows, see https://github.com/emilk/egui/pull/2280
                    windows_next_repaint_times.remove(&window_id);

                    winit_app.run_ui_and_paint(window_id);
                } else {
                    // Fix for https://github.com/emilk/egui/issues/2425
                    windows_next_repaint_times.insert(window_id, Instant::now());
                }
            }
            EventResult::RepaintNext(window_id) => {
                log::trace!("Repaint caused by winit::Event: {:?}", event_result);
                windows_next_repaint_times.insert(window_id, Instant::now());
            }
            EventResult::RepaintAt(window_id, repaint_time) => {
                windows_next_repaint_times.insert(
                    window_id,
                    windows_next_repaint_times
                        .get(&window_id)
                        .map_or(repaint_time, |last| (*last).min(repaint_time)),
                );
            }
            EventResult::Exit => {
                log::debug!("Asking to exit event loop…");
                winit_app.save_and_destroy();
                *control_flow = ControlFlow::Exit;
                return;
            }
        }

        // This is for not duplicating redraw requests

        use winit::event::Event;
        if matches!(
            event,
            Event::RedrawEventsCleared | Event::RedrawRequested(_) | Event::Resumed
        ) {
            for (window_id, repaint_time) in &windows_next_repaint_times.clone() {
                if *repaint_time <= Instant::now() {
                    if let Some(window) = winit_app.window(*window_id) {
                        log::trace!("request_redraw");
                        window.borrow().request_redraw();
                    } else {
                        windows_next_repaint_times.remove(window_id);
                    }
                    control_flow.set_poll();
                } else {
                    next_repaint_time = Some(
                        next_repaint_time.map_or(*repaint_time, |last| last.min(*repaint_time)),
                    );
                }
            }
        }

        if let Some(next_repaint_time) = next_repaint_time {
            let time_until_next = next_repaint_time.saturating_duration_since(Instant::now());
            if time_until_next < std::time::Duration::from_secs(10_000) {
                log::trace!("WaitUntil {time_until_next:?}");
            }
            control_flow.set_wait_until(next_repaint_time);
        };
    });

    log::debug!("eframe window closed");

    drop(winit_app);

    // On Windows this clears out events so that we can later create another window.
    // See https://github.com/emilk/egui/pull/1889 for details.
    //
    // Note that this approach may cause issues on macOS (emilk/egui#2768); therefore,
    // we only apply this approach on Windows to minimize the affect.
    #[cfg(target_os = "windows")]
    {
        event_loop.run_return(|_, _, control_flow| {
            control_flow.set_exit();
        });
    }

    returned_result
}

fn run_and_exit(event_loop: EventLoop<UserEvent>, mut winit_app: impl WinitApp + 'static) -> ! {
    log::debug!("Entering the winit event loop (run)…");

    let mut windows_next_repaint_times = HashMap::default();
    let mut next_repaint_time = Option::<Instant>::None;

    event_loop.run(move |event, event_loop, control_flow| {
        crate::profile_scope!("winit_event", short_event_description(&event));

        WINIT_EVENT_LOOP.with(|row_event_loop| row_event_loop.replace(event_loop));

        let event_result = match &event {
            winit::event::Event::LoopDestroyed => {
                log::debug!("Received Event::LoopDestroyed");
                EventResult::Exit
            }

            winit::event::Event::RedrawRequested(window_id) => {
                windows_next_repaint_times.remove(window_id);
                winit_app.run_ui_and_paint(*window_id)
            }

            winit::event::Event::UserEvent(UserEvent::RequestRepaint {
                when,
                frame_nr,
                viewport_id,
            }) => {
                let current_frame_nr = winit_app.frame_nr(*viewport_id);
                if current_frame_nr == *frame_nr || current_frame_nr == *frame_nr + 1 {
                    if let Some(window_id) = winit_app.window_id_from_viewport_id(*viewport_id) {
                        EventResult::RepaintAt(window_id, *when)
                    } else {
                        EventResult::Wait
                    }
                } else {
                    EventResult::Wait // old request - we've already repainted
                }
            }

            winit::event::Event::NewEvents(winit::event::StartCause::ResumeTimeReached {
                ..
            }) => EventResult::Wait, // We just woke up to check next_repaint_time

            event => match winit_app.on_event(event_loop, event) {
                Ok(event_result) => event_result,
                Err(err) => {
                    panic!("eframe encountered a fatal error: {err}");
                }
            },
        };

        match event_result {
            EventResult::Wait => {}
            EventResult::RepaintNow(window_id) => {
                if cfg!(target_os = "windows") {
                    // Fix flickering on Windows, see https://github.com/emilk/egui/pull/2280
                    windows_next_repaint_times.remove(&window_id);

                    winit_app.run_ui_and_paint(window_id);
                } else {
                    // Fix for https://github.com/emilk/egui/issues/2425
                    windows_next_repaint_times.insert(window_id, Instant::now());
                }
            }
            EventResult::RepaintNext(window_id) => {
                windows_next_repaint_times.insert(window_id, Instant::now());
            }
            EventResult::RepaintAt(window_id, repaint_time) => {
                windows_next_repaint_times.insert(
                    window_id,
                    windows_next_repaint_times
                        .get(&window_id)
                        .map_or(repaint_time, |last| (*last).min(repaint_time)),
                );
            }
            EventResult::Exit => {
                log::debug!("Quitting - saving app state…");
                winit_app.save_and_destroy();
                #[allow(clippy::exit)]
                std::process::exit(0);
            }
        }

        // This is for not duplicating redraw requests

        use winit::event::Event;
        if matches!(
            event,
            Event::RedrawEventsCleared | Event::RedrawRequested(_) | Event::Resumed
        ) {
            for (window_id, repaint_time) in &windows_next_repaint_times.clone() {
                if *repaint_time <= Instant::now() {
                    if let Some(window) = winit_app.window(*window_id) {
                        log::trace!("request_redraw");
                        window.borrow().request_redraw();
                    } else {
                        windows_next_repaint_times.remove(window_id);
                    }
                    control_flow.set_poll();
                } else {
                    next_repaint_time = Some(
                        next_repaint_time.map_or(*repaint_time, |last| last.min(*repaint_time)),
                    );
                }
            }
        }

        if let Some(next_repaint_time) = next_repaint_time {
            let time_until_next = next_repaint_time.saturating_duration_since(Instant::now());
            if time_until_next < std::time::Duration::from_secs(10_000) {
                log::trace!("WaitUntil {time_until_next:?}");
            }

            // WaitUntil seems to not work on iOS
            #[cfg(target_os = "ios")]
            winit_app
                .get_window_winit_id(ViewportId::ROOT)
                .map(|window_id| {
                    winit_app
                        .window(window_id)
                        .map(|window| window.borrow().request_redraw())
                });

            control_flow.set_wait_until(next_repaint_time);
        };
    })
}

// ----------------------------------------------------------------------------
/// Run an egui app
#[cfg(feature = "glow")]
mod glow_integration {

    use egui::{
        epaint::ahash::HashMap, NumExt as _, ViewportIdMap, ViewportIdPair, ViewportIdSet,
        ViewportOutput, ViewportUiCallback,
    };
    use egui_winit::{create_winit_window_builder, process_viewport_commands, EventResponse};
    use glutin::{
        display::GetGlDisplay,
        prelude::{GlDisplay, NotCurrentGlContextSurfaceAccessor, PossiblyCurrentGlContext},
        surface::GlSurface,
    };

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
        painter: Rc<RefCell<egui_glow::Painter>>,
        integration: Rc<RefCell<epi_integration::EpiIntegration>>,
        app: Rc<RefCell<Box<dyn epi::App>>>,
        // Conceptually this will be split out eventually so that the rest of the state
        // can be persistent.
        glutin_ctx: Rc<RefCell<GlutinWindowContext>>,
    }

    struct Viewport {
        gl_surface: Option<glutin::surface::Surface<glutin::surface::WindowSurface>>,
        window: Option<Rc<RefCell<winit::window::Window>>>,
        id_pair: ViewportIdPair,

        /// The user-callback that shows the ui.
        viewport_ui_cb: Option<Arc<Box<ViewportUiCallback>>>,

        egui_winit: Option<egui_winit::State>,
    }

    /// This struct will contain both persistent and temporary glutin state.
    ///
    /// Platform Quirks:
    /// * Microsoft Windows: requires that we create a window before opengl context.
    /// * Android: window and surface should be destroyed when we receive a suspend event. recreate on resume event.
    ///
    /// winit guarantees that we will get a Resumed event on startup on all platforms.
    /// * Before Resumed event: `gl_config`, `gl_context` can be created at any time. on windows, a window must be created to get `gl_context`.
    /// * Resumed: `gl_surface` will be created here. `window` will be re-created here for android.
    /// * Suspended: on android, we drop window + surface.  on other platforms, we don't get Suspended event.
    ///
    /// The setup is divided between the `new` fn and `on_resume` fn. we can just assume that `on_resume` is a continuation of
    /// `new` fn on all platforms. only on android, do we get multiple resumed events because app can be suspended.
    struct GlutinWindowContext {
        swap_interval: glutin::surface::SwapInterval,
        gl_config: glutin::config::Config,

        max_texture_side: usize,

        current_gl_context: Option<glutin::context::PossiblyCurrentContext>,
        not_current_gl_context: Option<glutin::context::NotCurrentContext>,

        viewports: ViewportIdMap<Rc<RefCell<Viewport>>>,
        viewport_maps: HashMap<winit::window::WindowId, ViewportId>,
        window_maps: ViewportIdMap<winit::window::WindowId>,

        builders: ViewportIdMap<ViewportBuilder>,
    }

    impl GlutinWindowContext {
        /// There is a lot of complexity with opengl creation, so prefer extensive logging to get all the help we can to debug issues.
        ///
        #[allow(unsafe_code)]
        unsafe fn new(
            window_builder: ViewportBuilder,
            native_options: &epi::NativeOptions,
            event_loop: &EventLoopWindowTarget<UserEvent>,
        ) -> Result<Self> {
            crate::profile_function!();

            use glutin::prelude::*;
            // convert native options to glutin options
            let hardware_acceleration = match native_options.hardware_acceleration {
                crate::HardwareAcceleration::Required => Some(true),
                crate::HardwareAcceleration::Preferred => None,
                crate::HardwareAcceleration::Off => Some(false),
            };
            let swap_interval = if native_options.vsync {
                glutin::surface::SwapInterval::Wait(std::num::NonZeroU32::new(1).unwrap())
            } else {
                glutin::surface::SwapInterval::DontWait
            };
            /*  opengl setup flow goes like this:
                1. we create a configuration for opengl "Display" / "Config" creation
                2. choose between special extensions like glx or egl or wgl and use them to create config/display
                3. opengl context configuration
                4. opengl context creation
            */
            // start building config for gl display
            let config_template_builder = glutin::config::ConfigTemplateBuilder::new()
                .prefer_hardware_accelerated(hardware_acceleration)
                .with_depth_size(native_options.depth_buffer)
                .with_stencil_size(native_options.stencil_buffer)
                .with_transparency(native_options.transparent);
            // we don't know if multi sampling option is set. so, check if its more than 0.
            let config_template_builder = if native_options.multisampling > 0 {
                config_template_builder.with_multisampling(
                    native_options
                        .multisampling
                        .try_into()
                        .expect("failed to fit multisamples option of native_options into u8"),
                )
            } else {
                config_template_builder
            };

            log::debug!(
                "trying to create glutin Display with config: {:?}",
                &config_template_builder
            );

            // Create GL display. This may probably create a window too on most platforms. Definitely on `MS windows`. Never on Android.
            let display_builder = glutin_winit::DisplayBuilder::new()
                // we might want to expose this option to users in the future. maybe using an env var or using native_options.
                .with_preference(glutin_winit::ApiPrefence::FallbackEgl) // https://github.com/emilk/egui/issues/2520#issuecomment-1367841150
                .with_window_builder(Some(create_winit_window_builder(&window_builder)));

            let (window, gl_config) = {
                crate::profile_scope!("DisplayBuilder::build");

                display_builder
                    .build(
                        event_loop,
                        config_template_builder.clone(),
                        |mut config_iterator| {
                            let config = config_iterator.next().expect(
                            "failed to find a matching configuration for creating glutin config",
                        );
                            log::debug!(
                                "using the first config from config picker closure. config: {:?}",
                                &config
                            );
                            config
                        },
                    )
                    .map_err(|e| {
                        crate::Error::NoGlutinConfigs(config_template_builder.build(), e)
                    })?
            };

            let gl_display = gl_config.display();
            log::debug!(
                "successfully created GL Display with version: {} and supported features: {:?}",
                gl_display.version_string(),
                gl_display.supported_features()
            );
            let raw_window_handle = window.as_ref().map(|w| w.raw_window_handle());
            log::debug!(
                "creating gl context using raw window handle: {:?}",
                raw_window_handle
            );

            // create gl context. if core context cannot be created, try gl es context as fallback.
            let context_attributes =
                glutin::context::ContextAttributesBuilder::new().build(raw_window_handle);
            let fallback_context_attributes = glutin::context::ContextAttributesBuilder::new()
                .with_context_api(glutin::context::ContextApi::Gles(None))
                .build(raw_window_handle);

            let gl_context_result = {
                crate::profile_scope!("create_context");
                gl_config
                    .display()
                    .create_context(&gl_config, &context_attributes)
            };

            let gl_context = match gl_context_result {
                Ok(it) => it,
                Err(err) => {
                    log::warn!("failed to create context using default context attributes {context_attributes:?} due to error: {err}");
                    log::debug!("retrying with fallback context attributes: {fallback_context_attributes:?}");
                    gl_config
                        .display()
                        .create_context(&gl_config, &fallback_context_attributes)?
                }
            };
            let not_current_gl_context = Some(gl_context);

            let mut viewport_maps = HashMap::default();
            let mut window_maps = ViewportIdMap::default();
            if let Some(window) = &window {
                viewport_maps.insert(window.id(), ViewportId::ROOT);
                window_maps.insert(ViewportId::ROOT, window.id());
            }

            let mut windows = ViewportIdMap::default();
            windows.insert(
                ViewportId::ROOT,
                Rc::new(RefCell::new(Viewport {
                    gl_surface: None,
                    window: window.map(|w| Rc::new(RefCell::new(w))),
                    egui_winit: None,
                    viewport_ui_cb: None,
                    id_pair: ViewportIdPair::ROOT,
                })),
            );

            let mut builders = ViewportIdMap::default();
            builders.insert(ViewportId::ROOT, window_builder);

            // the fun part with opengl gl is that we never know whether there is an error. the context creation might have failed, but
            // it could keep working until we try to make surface current or swap buffers or something else. future glutin improvements might
            // help us start from scratch again if we fail context creation and go back to preferEgl or try with different config etc..
            // https://github.com/emilk/egui/pull/2541#issuecomment-1370767582
            Ok(GlutinWindowContext {
                swap_interval,
                gl_config,
                current_gl_context: None,
                not_current_gl_context,
                viewports: windows,
                builders,
                viewport_maps,
                // This is initialize in init_run_state
                max_texture_side: 0,
                window_maps,
            })
        }

        /// This will be run after `new`. on android, it might be called multiple times over the course of the app's lifetime.
        /// roughly,
        /// 1. check if window already exists. otherwise, create one now.
        /// 2. create attributes for surface creation.
        /// 3. create surface.
        /// 4. make surface and context current.
        ///
        /// we presently assume that we will
        fn on_resume(&mut self, event_loop: &EventLoopWindowTarget<UserEvent>) -> Result<()> {
            crate::profile_function!();

            let windows = self
                .viewports
                .values()
                .cloned()
                .collect::<Vec<Rc<RefCell<Viewport>>>>();
            for window in windows {
                if window.borrow().gl_surface.is_some() {
                    continue;
                }
                self.init_window(&window, event_loop)?;
            }
            Ok(())
        }

        #[allow(unsafe_code)]
        pub(crate) fn init_window(
            &mut self,
            win: &Rc<RefCell<Viewport>>,
            event_loop: &EventLoopWindowTarget<UserEvent>,
        ) -> Result<()> {
            let builder = &self.builders[&win.borrow().id_pair.this];
            let mut win = win.borrow_mut();
            // make sure we have a window or create one.
            let window = win.window.take().unwrap_or_else(|| {
                log::debug!("window doesn't exist yet. creating one now with finalize_window");
                Rc::new(RefCell::new(
                    glutin_winit::finalize_window(
                        event_loop,
                        create_winit_window_builder(builder),
                        &self.gl_config,
                    )
                    .expect("failed to finalize glutin window"),
                ))
            });
            {
                let window = window.borrow();
                // surface attributes
                let (width, height): (u32, u32) = window.inner_size().into();
                let width = std::num::NonZeroU32::new(width.at_least(1)).unwrap();
                let height = std::num::NonZeroU32::new(height.at_least(1)).unwrap();
                let surface_attributes = glutin::surface::SurfaceAttributesBuilder::<
                    glutin::surface::WindowSurface,
                >::new()
                .build(window.raw_window_handle(), width, height);
                log::debug!(
                    "creating surface with attributes: {:?}",
                    &surface_attributes
                );
                // create surface
                let gl_surface = unsafe {
                    self.gl_config
                        .display()
                        .create_window_surface(&self.gl_config, &surface_attributes)?
                };
                log::debug!("surface created successfully: {gl_surface:?}.making context current");
                // make surface and context current.
                let not_current_gl_context =
                    if let Some(not_current_context) = self.not_current_gl_context.take() {
                        not_current_context
                    } else {
                        self.current_gl_context
                            .take()
                            .unwrap()
                            .make_not_current()
                            .unwrap()
                    };
                let current_gl_context = not_current_gl_context.make_current(&gl_surface)?;
                // try setting swap interval. but its not absolutely necessary, so don't panic on failure.
                log::debug!("made context current. setting swap interval for surface");
                if let Err(e) =
                    gl_surface.set_swap_interval(&current_gl_context, self.swap_interval)
                {
                    log::error!("failed to set swap interval due to error: {e:?}");
                }
                // we will reach this point only once in most platforms except android.
                // create window/surface/make context current once and just use them forever.

                let native_pixels_per_point = window.scale_factor() as f32;

                if win.egui_winit.is_none() {
                    let mut egui_winit = egui_winit::State::new(event_loop);
                    egui_winit.set_max_texture_side(self.max_texture_side);
                    egui_winit.set_pixels_per_point(native_pixels_per_point);
                    win.egui_winit = Some(egui_winit);
                }

                win.gl_surface = Some(gl_surface);
                self.current_gl_context = Some(current_gl_context);
                self.viewport_maps.insert(window.id(), win.id_pair.this);
                self.window_maps.insert(win.id_pair.this, window.id());
            }
            win.window = Some(window);
            Ok(())
        }

        /// only applies for android. but we basically drop surface + window and make context not current
        fn on_suspend(&mut self) -> Result<()> {
            log::debug!("received suspend event. dropping window and surface");
            for window in self.viewports.values() {
                let mut window = window.borrow_mut();
                window.gl_surface.take();
                window.window.take();
            }
            if let Some(current) = self.current_gl_context.take() {
                log::debug!("context is current, so making it non-current");
                self.not_current_gl_context = Some(current.make_not_current()?);
            } else {
                log::debug!("context is already not current??? could be duplicate suspend event");
            }
            Ok(())
        }

        fn window(&self, viewport_id: ViewportId) -> Rc<RefCell<Viewport>> {
            self.viewports
                .get(&viewport_id)
                .cloned()
                .expect("winit window doesn't exist")
        }

        fn resize(
            &mut self,
            viewport_id: ViewportId,
            physical_size: winit::dpi::PhysicalSize<u32>,
        ) {
            let width = std::num::NonZeroU32::new(physical_size.width.at_least(1)).unwrap();
            let height = std::num::NonZeroU32::new(physical_size.height.at_least(1)).unwrap();

            if let Some(viewport) = self.viewports.get(&viewport_id) {
                let viewport = viewport.borrow();
                if let Some(gl_surface) = &viewport.gl_surface {
                    self.current_gl_context = Some(
                        self.current_gl_context
                            .take()
                            .unwrap()
                            .make_not_current()
                            .unwrap()
                            .make_current(gl_surface)
                            .unwrap(),
                    );
                    gl_surface.resize(
                        self.current_gl_context
                            .as_ref()
                            .expect("failed to get current context to resize surface"),
                        width,
                        height,
                    );
                }
            }
        }

        fn get_proc_address(&self, addr: &std::ffi::CStr) -> *const std::ffi::c_void {
            self.gl_config.display().get_proc_address(addr)
        }
    }

    struct GlowWinitApp {
        repaint_proxy: Arc<egui::mutex::Mutex<EventLoopProxy<UserEvent>>>,
        app_name: String,
        native_options: epi::NativeOptions,
        running: Rc<RefCell<Option<GlowWinitRunning>>>,

        // Note that since this `AppCreator` is FnOnce we are currently unable to support
        // re-initializing the `GlowWinitRunning` state on Android if the application
        // suspends and resumes.
        app_creator: Option<epi::AppCreator>,
        is_focused: Rc<RefCell<Option<ViewportId>>>,
    }

    impl GlowWinitApp {
        fn new(
            event_loop: &EventLoop<UserEvent>,
            app_name: &str,
            native_options: epi::NativeOptions,
            app_creator: epi::AppCreator,
        ) -> Self {
            crate::profile_function!();
            Self {
                repaint_proxy: Arc::new(egui::mutex::Mutex::new(event_loop.create_proxy())),
                app_name: app_name.to_owned(),
                native_options,
                running: Rc::new(RefCell::new(None)),
                app_creator: Some(app_creator),
                is_focused: Rc::new(RefCell::new(Some(ViewportId::ROOT))),
            }
        }

        #[allow(unsafe_code)]
        fn create_glutin_windowed_context(
            event_loop: &EventLoopWindowTarget<UserEvent>,
            storage: Option<&dyn epi::Storage>,
            title: &str,
            native_options: &mut NativeOptions,
        ) -> Result<(GlutinWindowContext, glow::Context)> {
            crate::profile_function!();

            let window_settings = epi_integration::load_window_settings(storage);

            let winit_window_builder =
                epi_integration::window_builder(event_loop, title, native_options, window_settings);
            let mut glutin_window_context = unsafe {
                GlutinWindowContext::new(winit_window_builder, native_options, event_loop)?
            };
            glutin_window_context.on_resume(event_loop)?;

            if let Some(window) = &glutin_window_context.viewports.get(&ViewportId::ROOT) {
                let window = window.borrow();
                if let Some(window) = &window.window {
                    epi_integration::apply_native_options_to_window(
                        &window.borrow(),
                        native_options,
                        window_settings,
                    );
                }
            }

            let gl = unsafe {
                crate::profile_scope!("glow::Context::from_loader_function");
                glow::Context::from_loader_function(|s| {
                    let s = std::ffi::CString::new(s)
                        .expect("failed to construct C string from string for gl proc address");

                    glutin_window_context.get_proc_address(&s)
                })
            };

            Ok((glutin_window_context, gl))
        }

        fn init_run_state(&mut self, event_loop: &EventLoopWindowTarget<UserEvent>) -> Result<()> {
            crate::profile_function!();
            let storage = epi_integration::create_storage(
                self.native_options
                    .app_id
                    .as_ref()
                    .unwrap_or(&self.app_name),
            );

            let (mut glutin_ctx, gl) = Self::create_glutin_windowed_context(
                event_loop,
                storage.as_deref(),
                &self.app_name,
                &mut self.native_options,
            )?;
            let gl = Arc::new(gl);

            let painter =
                egui_glow::Painter::new(gl.clone(), "", self.native_options.shader_version)
                    .unwrap_or_else(|err| panic!("An OpenGL error occurred: {err}\n"));

            glutin_ctx.max_texture_side = painter.max_texture_side();
            glutin_ctx.viewports[&ViewportId::ROOT]
                .borrow_mut()
                .egui_winit
                .as_mut()
                .unwrap()
                .set_max_texture_side(glutin_ctx.max_texture_side);

            let system_theme = system_theme(
                &glutin_ctx
                    .window(ViewportId::ROOT)
                    .borrow()
                    .window
                    .as_ref()
                    .unwrap()
                    .borrow(),
                &self.native_options,
            );
            let mut integration = epi_integration::EpiIntegration::new(
                &glutin_ctx
                    .window(ViewportId::ROOT)
                    .borrow()
                    .window
                    .as_ref()
                    .unwrap()
                    .borrow(),
                system_theme,
                &self.app_name,
                &self.native_options,
                storage,
                IS_DESKTOP,
                Some(gl.clone()),
                #[cfg(feature = "wgpu")]
                None,
            );

            {
                let event_loop_proxy = self.repaint_proxy.clone();
                integration
                    .egui_ctx
                    .set_request_repaint_callback(move |info| {
                        log::trace!("request_repaint_callback: {info:?}");
                        let when = Instant::now() + info.after;
                        let frame_nr = info.current_frame_nr;
                        event_loop_proxy
                            .lock()
                            .send_event(UserEvent::RequestRepaint {
                                viewport_id: info.viewport_id,
                                when,
                                frame_nr,
                            })
                            .ok();
                    });
            }

            #[cfg(feature = "accesskit")]
            {
                let event_loop_proxy = self.repaint_proxy.lock().clone();
                let window = &glutin_ctx.viewports[&ViewportId::ROOT];
                let window = &mut *window.borrow_mut();
                integration.init_accesskit(
                    window.egui_winit.as_mut().unwrap(),
                    &window.window.as_ref().unwrap().borrow(),
                    event_loop_proxy,
                );
            }
            let theme = system_theme.unwrap_or(self.native_options.default_theme);
            integration.egui_ctx.set_visuals(theme.egui_visuals());

            if self.native_options.mouse_passthrough {
                glutin_ctx
                    .window(ViewportId::ROOT)
                    .borrow()
                    .window
                    .as_ref()
                    .unwrap()
                    .borrow()
                    .set_cursor_hittest(false)
                    .unwrap();
            }

            let app_creator = std::mem::take(&mut self.app_creator)
                .expect("Single-use AppCreator has unexpectedly already been taken");
            let mut app;
            {
                let window = glutin_ctx.window(ViewportId::ROOT);
                let window = &mut *window.borrow_mut();
                app = app_creator(&epi::CreationContext {
                    egui_ctx: integration.egui_ctx.clone(),
                    integration_info: integration.frame.info().clone(),
                    storage: integration.frame.storage(),
                    gl: Some(gl.clone()),
                    #[cfg(feature = "wgpu")]
                    wgpu_render_state: None,
                    raw_display_handle: window
                        .window
                        .as_ref()
                        .unwrap()
                        .borrow()
                        .raw_display_handle(),
                    raw_window_handle: window.window.as_ref().unwrap().borrow().raw_window_handle(),
                });

                if app.warm_up_enabled() {
                    integration.warm_up(
                        app.as_mut(),
                        &window.window.as_ref().unwrap().borrow(),
                        window.egui_winit.as_mut().unwrap(),
                    );
                }
            }

            let glutin_ctx = Rc::new(RefCell::new(glutin_ctx));
            let painter = Rc::new(RefCell::new(painter));

            {
                // Create weak pointers so that we don't keep
                // state alive for too long.
                let glutin = Rc::downgrade(&glutin_ctx);
                let gl = Arc::downgrade(&gl);
                let painter = Rc::downgrade(&painter);
                let beginning = integration.beginning;

                integration.egui_ctx.set_render_sync_callback(
                    move |egui_ctx, viewport_builder, id_pair, viewport_ui_cb| {
                        if let (Some(glutin), Some(gl), Some(painter)) =
                            (glutin.upgrade(), gl.upgrade(), painter.upgrade())
                        {
                            Self::render_sync_viewport(
                                egui_ctx,
                                viewport_builder,
                                id_pair,
                                viewport_ui_cb,
                                &glutin,
                                &gl,
                                &painter,
                                beginning,
                            );
                        } else {
                            log::warn!("render_sync_callback called after window closed");
                        }
                    },
                );
            }

            *self.running.borrow_mut() = Some(GlowWinitRunning {
                glutin_ctx,
                gl,
                painter,
                integration: Rc::new(RefCell::new(integration)),
                app: Rc::new(RefCell::new(app)),
            });

            Ok(())
        }

        #[inline(always)]
        #[allow(clippy::too_many_arguments)]
        fn render_sync_viewport(
            egui_ctx: &egui::Context,
            mut viewport_builder: ViewportBuilder,
            id_pair: ViewportIdPair,
            viewport_ui_cb: Box<dyn FnOnce(&egui::Context) + '_>,
            glutin: &RefCell<GlutinWindowContext>,
            gl: &glow::Context,
            painter: &RefCell<egui_glow::Painter>,
            beginning: Instant,
        ) {
            crate::profile_function!();

            let has_window = glutin.borrow().viewports.get(&id_pair.this).is_some();

            // This will create a new native window if is needed
            if !has_window {
                let mut glutin = glutin.borrow_mut();

                // Inherit parent icon if none
                {
                    if viewport_builder.icon.is_none()
                        && glutin.builders.get(&id_pair.this).is_none()
                    {
                        viewport_builder.icon = glutin
                            .builders
                            .get(&id_pair.parent)
                            .and_then(|b| b.icon.clone());
                    }
                }

                {
                    glutin
                        .viewports
                        .entry(id_pair.this)
                        .or_insert(Rc::new(RefCell::new(Viewport {
                            gl_surface: None,
                            window: None,
                            id_pair,
                            viewport_ui_cb: None,
                            egui_winit: None,
                        })));
                    glutin
                        .builders
                        .entry(id_pair.this)
                        .or_insert(viewport_builder);
                }

                let win = glutin.viewports[&id_pair.this].clone();

                #[allow(unsafe_code)]
                let event_loop = unsafe {
                    WINIT_EVENT_LOOP.with(|event_loop| {
                        event_loop.borrow().as_ref().expect("No winit event loop")
                    })
                };
                glutin
                    .init_window(&win, event_loop)
                    .expect("Cannot init window on egui::Context::create_viewport_sync");
            }

            // Rendering the sync viewport

            let window = glutin.borrow().viewports.get(&id_pair.this).cloned();
            let Some(window) = window else { return };

            let window = &mut *window.borrow_mut();
            let Some(winit_state) = &mut window.egui_winit else {
                return;
            };
            let Some(win) = window.window.clone() else {
                return;
            };
            let win = win.borrow();
            let mut input = winit_state.take_egui_input(&win);
            input.time = Some(beginning.elapsed().as_secs_f64());
            let output = egui_ctx.run(input, id_pair, |ctx| {
                viewport_ui_cb(ctx);
            });

            let screen_size_in_pixels: [u32; 2] = win.inner_size().into();

            let clipped_primitives = egui_ctx.tessellate(output.shapes, id_pair.this);

            let mut glutin = glutin.borrow_mut();
            let mut painter = painter.borrow_mut();

            glutin.current_gl_context = Some(
                glutin
                    .current_gl_context
                    .take()
                    .unwrap()
                    .make_not_current()
                    .unwrap()
                    .make_current(window.gl_surface.as_ref().unwrap())
                    .unwrap(),
            );

            if !window
                .gl_surface
                .as_ref()
                .unwrap()
                .is_current(glutin.current_gl_context.as_ref().unwrap())
            {
                let builder = &&glutin.builders[&window.id_pair.this];
                log::error!("egui::create_viewport_sync with title: `{:?}` is not created in main thread, try to use wgpu!", builder.title.clone().unwrap_or_default());
            }

            egui_glow::painter::clear(gl, screen_size_in_pixels, [0.0, 0.0, 0.0, 0.0]);

            let pixels_per_point = egui_ctx.input_for(id_pair.this, |i| i.pixels_per_point());
            painter.paint_and_update_textures(
                screen_size_in_pixels,
                pixels_per_point,
                &clipped_primitives,
                &output.textures_delta,
            );
            crate::profile_scope!("swap_buffers");
            if let Err(err) = window
                .gl_surface
                .as_ref()
                .expect("failed to get surface to swap buffers")
                .swap_buffers(
                    glutin
                        .current_gl_context
                        .as_ref()
                        .expect("failed to get current context to swap buffers"),
                )
            {
                log::error!("swap_buffers failed: {err}");
            }
            winit_state.handle_platform_output(
                &win,
                id_pair.this,
                egui_ctx,
                output.platform_output,
            );
        }

        fn process_viewport_builders(
            glutin_ctx: &Rc<RefCell<GlutinWindowContext>>,
            mut viewports: Vec<ViewportOutput>,
        ) {
            let mut active_viewports_ids = ViewportIdSet::default();
            active_viewports_ids.insert(ViewportId::ROOT);

            viewports.retain_mut(
                |ViewportOutput {
                     builder: new_builder,
                     id_pair: ViewportIdPair { this: id, parent },
                     viewport_ui_cb,
                 }| {
                    let mut glutin = glutin_ctx.borrow_mut();
                    let builder = glutin.builders.entry(*id).or_insert(new_builder.clone());
                    let (commands, recreate) = builder.patch(new_builder);
                    drop(glutin);
                    if let Some(viewport) = glutin_ctx.borrow().viewports.get(id) {
                        let mut viewport = viewport.borrow_mut();

                        viewport.id_pair.parent = *parent;
                        viewport.viewport_ui_cb = viewport_ui_cb.clone(); // always update the latest callback

                        if recreate {
                            viewport.window = None;
                            viewport.gl_surface = None;
                        } else if let Some(w) = viewport.window.clone() {
                            process_viewport_commands(commands, *id, None, &w.borrow());
                        }
                        active_viewports_ids.insert(*id);
                        false
                    } else {
                        true
                    }
                },
            );

            for ViewportOutput {
                mut builder,
                id_pair,
                viewport_ui_cb,
            } in viewports
            {
                if builder.icon.is_none() {
                    // Inherit from parent as fallback.
                    builder.icon = glutin_ctx
                        .borrow()
                        .builders
                        .get(&id_pair.parent)
                        .and_then(|b| b.icon.clone());
                }
                {
                    let mut glutin = glutin_ctx.borrow_mut();
                    glutin.viewports.insert(
                        id_pair.this,
                        Rc::new(RefCell::new(Viewport {
                            gl_surface: None,
                            window: None,
                            egui_winit: None,
                            viewport_ui_cb,
                            id_pair,
                        })),
                    );
                    glutin.builders.insert(id_pair.this, builder);
                }
                active_viewports_ids.insert(id_pair.this);
            }

            let mut glutin = glutin_ctx.borrow_mut();
            glutin
                .viewports
                .retain(|id, _| active_viewports_ids.contains(id));
            glutin
                .builders
                .retain(|id, _| active_viewports_ids.contains(id));
            glutin
                .viewport_maps
                .retain(|_, id| active_viewports_ids.contains(id));
            glutin
                .window_maps
                .retain(|id, _| active_viewports_ids.contains(id));
        }
    }

    impl WinitApp for GlowWinitApp {
        fn frame_nr(&self, viewport_id: ViewportId) -> u64 {
            self.running.borrow().as_ref().map_or(0, |r| {
                r.integration.borrow().egui_ctx.frame_nr_for(viewport_id)
            })
        }

        fn is_focused(&self, window_id: winit::window::WindowId) -> bool {
            if let Some(is_focused) = self.is_focused.borrow().as_ref() {
                if let Some(running) = self.running.borrow().as_ref() {
                    if let Some(window_id) =
                        running.glutin_ctx.borrow().viewport_maps.get(&window_id)
                    {
                        return *is_focused == *window_id;
                    }
                }
            }
            false
        }

        fn integration(&self) -> Option<Rc<RefCell<EpiIntegration>>> {
            self.running
                .borrow()
                .as_ref()
                .map(|r| r.integration.clone())
        }

        fn window(
            &self,
            window_id: winit::window::WindowId,
        ) -> Option<Rc<RefCell<winit::window::Window>>> {
            self.running.borrow().as_ref().and_then(|r| {
                let glutin_ctx = r.glutin_ctx.borrow();
                if let Some(viewport_id) = glutin_ctx.viewport_maps.get(&window_id) {
                    if let Some(viewport) = glutin_ctx.viewports.get(viewport_id) {
                        if let Some(window) = viewport.borrow().window.as_ref() {
                            return Some(window.clone());
                        }
                    }
                }
                None
            })
        }

        fn window_id_from_viewport_id(&self, id: ViewportId) -> Option<winit::window::WindowId> {
            self.running
                .borrow()
                .as_ref()
                .and_then(|r| r.glutin_ctx.borrow().window_maps.get(&id).copied())
        }

        fn viewport_id_from_window_id(&self, id: &winit::window::WindowId) -> Option<ViewportId> {
            self.running
                .borrow()
                .as_ref()
                .and_then(|r| r.glutin_ctx.borrow().viewport_maps.get(id).copied())
        }

        fn save_and_destroy(&mut self) {
            crate::profile_function!();
            if let Some(running) = self.running.borrow_mut().take() {
                crate::profile_function!();

                running.integration.borrow_mut().save(
                    running.app.borrow_mut().as_mut(),
                    running
                        .glutin_ctx
                        .borrow()
                        .window(ViewportId::ROOT)
                        .borrow()
                        .window
                        .as_ref()
                        .map(|w| w.borrow())
                        .as_deref(),
                );
                running.app.borrow_mut().on_exit(Some(&running.gl));
                running.painter.borrow_mut().destroy();
            }
        }

        fn run_ui_and_paint(&mut self, window_id: winit::window::WindowId) -> EventResult {
            if self.running.borrow().is_none() {
                return EventResult::Wait;
            }

            if let Some(viewport_id) = self.viewport_id_from_window_id(&window_id) {
                #[cfg(feature = "puffin")]
                puffin::GlobalProfiler::lock().new_frame();
                crate::profile_scope!("frame");

                let (integration, app, glutin, painter) = {
                    let running = self.running.borrow();
                    let running = running.as_ref().unwrap();
                    (
                        running.integration.clone(),
                        running.app.clone(),
                        running.glutin_ctx.clone(),
                        running.painter.clone(),
                    )
                };

                // This will only happen if the viewport is sync
                // That means that the viewport cannot be rendered by itself and needs his parent to be rendered
                {
                    let glutin = glutin.borrow();
                    let viewport = &glutin.viewports[&viewport_id].clone();
                    if viewport.borrow().viewport_ui_cb.is_none() && viewport_id != ViewportId::ROOT
                    {
                        if let Some(parent_viewport) =
                            glutin.viewports.get(&viewport.borrow().id_pair.parent)
                        {
                            if let Some(window) = parent_viewport.borrow().window.as_ref() {
                                return EventResult::RepaintNext(window.borrow().id());
                            }
                        }
                        return EventResult::Wait;
                    }
                }

                let egui::FullOutput {
                    platform_output,
                    textures_delta,
                    shapes,
                    viewports,
                    viewport_commands,
                };

                let control_flow;
                {
                    let viewport = glutin.borrow().viewports.get(&viewport_id).cloned();
                    let viewport = viewport.unwrap();

                    let screen_size_in_pixels: [u32; 2] = viewport
                        .borrow()
                        .window
                        .as_ref()
                        .unwrap()
                        .borrow()
                        .inner_size()
                        .into();

                    {
                        let viewport = &mut *viewport.borrow_mut();
                        let window = viewport.window.as_ref().unwrap().borrow();
                        let egui_winit = viewport.egui_winit.as_mut().unwrap();

                        egui::FullOutput {
                            platform_output,
                            textures_delta,
                            shapes,
                            viewports,
                            viewport_commands,
                        } = integration.borrow_mut().update(
                            app.borrow_mut().as_mut(),
                            &window,
                            egui_winit,
                            &viewport.viewport_ui_cb.clone(),
                            viewport.id_pair,
                        );

                        integration.borrow_mut().handle_platform_output(
                            &window,
                            viewport_id,
                            platform_output,
                            egui_winit,
                        );
                    }

                    let clipped_primitives = {
                        crate::profile_scope!("tessellate");
                        integration
                            .borrow()
                            .egui_ctx
                            .tessellate(shapes, viewport_id)
                    };
                    {
                        let mut glutin = glutin.borrow_mut();
                        glutin.current_gl_context = Some(
                            glutin
                                .current_gl_context
                                .take()
                                .unwrap()
                                .make_not_current()
                                .unwrap()
                                .make_current(viewport.borrow().gl_surface.as_ref().unwrap())
                                .unwrap(),
                        );
                    };

                    let gl = self.running.borrow().as_ref().unwrap().gl.clone();

                    egui_glow::painter::clear(
                        &gl,
                        screen_size_in_pixels,
                        app.borrow()
                            .clear_color(&integration.borrow().egui_ctx.style().visuals),
                    );

                    let pixels_per_point = integration
                        .borrow()
                        .egui_ctx
                        .input_for(viewport_id, |i| i.pixels_per_point());

                    painter.borrow_mut().paint_and_update_textures(
                        screen_size_in_pixels,
                        pixels_per_point,
                        &clipped_primitives,
                        &textures_delta,
                    );

                    let mut integration = integration.borrow_mut();
                    {
                        let screenshot_requested =
                            &mut integration.frame.output.screenshot_requested;

                        if *screenshot_requested {
                            *screenshot_requested = false;
                            let screenshot =
                                painter.borrow().read_screen_rgba(screen_size_in_pixels);
                            integration.frame.screenshot.set(Some(screenshot));
                        }

                        integration.post_rendering(
                            app.borrow_mut().as_mut(),
                            &viewport.borrow().window.as_ref().unwrap().borrow(),
                        );
                    }

                    {
                        crate::profile_scope!("swap_buffers");
                        if let Err(err) = viewport
                            .borrow()
                            .gl_surface
                            .as_ref()
                            .expect("failed to get surface to swap buffers")
                            .swap_buffers(
                                glutin
                                    .borrow()
                                    .current_gl_context
                                    .as_ref()
                                    .expect("failed to get current context to swap buffers"),
                            )
                        {
                            log::error!("swap_buffers failed: {err}");
                        }
                    }

                    integration.post_present(&viewport.borrow().window.as_ref().unwrap().borrow());

                    #[cfg(feature = "__screenshot")]
                    // give it time to settle:
                    if integration.egui_ctx.frame_nr() == 2 {
                        if let Ok(path) = std::env::var("EFRAME_SCREENSHOT_TO") {
                            assert!(
                                path.ends_with(".png"),
                                "Expected EFRAME_SCREENSHOT_TO to end with '.png', got {path:?}"
                            );
                            let screenshot =
                                painter.borrow().read_screen_rgba(screen_size_in_pixels);
                            image::save_buffer(
                                &path,
                                screenshot.as_raw(),
                                screenshot.width() as u32,
                                screenshot.height() as u32,
                                image::ColorType::Rgba8,
                            )
                            .unwrap_or_else(|err| {
                                panic!("Failed to save screenshot to {path:?}: {err}");
                            });
                            eprintln!("Screenshot saved to {path:?}.");
                            std::process::exit(0);
                        }
                    }

                    control_flow = if integration.should_close() {
                        EventResult::Exit
                    } else {
                        EventResult::Wait
                    };

                    integration.maybe_autosave(
                        app.borrow_mut().as_mut(),
                        viewport
                            .borrow()
                            .window
                            .as_ref()
                            .map(|w| w.borrow())
                            .as_deref(),
                    );

                    if viewport
                        .borrow()
                        .window
                        .as_ref()
                        .unwrap()
                        .borrow()
                        .is_minimized()
                        == Some(true)
                    {
                        // On Mac, a minimized Window uses up all CPU:
                        // https://github.com/emilk/egui/issues/325
                        crate::profile_scope!("minimized_sleep");
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                }

                Self::process_viewport_builders(&glutin, viewports);

                for (viewport_id, command) in viewport_commands {
                    if let Some(window) = glutin
                        .borrow()
                        .viewports
                        .get(&viewport_id)
                        .and_then(|viewport| viewport.borrow().window.clone())
                    {
                        egui_winit::process_viewport_commands(
                            vec![command],
                            viewport_id,
                            *self.is_focused.borrow(),
                            &window.borrow(),
                        );
                    }
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
        ) -> Result<EventResult> {
            crate::profile_function!();

            Ok(match event {
                winit::event::Event::Resumed => {
                    // first resume event.
                    // we can actually move this outside of event loop.
                    // and just run the on_resume fn of gl_window
                    if self.running.borrow().is_none() {
                        self.init_run_state(event_loop)?;
                    } else {
                        // not the first resume event. create whatever you need.
                        self.running
                            .borrow_mut()
                            .as_mut()
                            .unwrap()
                            .glutin_ctx
                            .borrow_mut()
                            .on_resume(event_loop)?;
                    }
                    EventResult::RepaintNow(
                        self.running
                            .borrow()
                            .as_ref()
                            .unwrap()
                            .glutin_ctx
                            .borrow()
                            .window_maps
                            .get(&ViewportId::ROOT)
                            .copied()
                            .unwrap(),
                    )
                }
                winit::event::Event::Suspended => {
                    self.running
                        .borrow_mut()
                        .as_mut()
                        .unwrap()
                        .glutin_ctx
                        .borrow_mut()
                        .on_suspend()?;

                    EventResult::Wait
                }

                winit::event::Event::MainEventsCleared => {
                    if let Some(running) = self.running.borrow().as_ref() {
                        if let Err(err) = running.glutin_ctx.borrow_mut().on_resume(event_loop) {
                            log::warn!("on_resume failed {err}");
                        }
                    }
                    EventResult::Wait
                }

                winit::event::Event::WindowEvent { event, window_id } => {
                    if let Some(running) = self.running.borrow_mut().as_mut() {
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
                                *self.is_focused.borrow_mut() = new_focused
                                    .then(|| {
                                        running
                                            .glutin_ctx
                                            .borrow_mut()
                                            .viewport_maps
                                            .get(window_id)
                                            .copied()
                                    })
                                    .flatten();
                            }
                            winit::event::WindowEvent::Resized(physical_size) => {
                                repaint_asap = true;

                                // Resize with 0 width and height is used by winit to signal a minimize event on Windows.
                                // See: https://github.com/rust-windowing/winit/issues/208
                                // This solves an issue where the app would panic when minimizing on Windows.
                                let glutin = &mut *running.glutin_ctx.borrow_mut();
                                if 0 < physical_size.width && 0 < physical_size.height {
                                    if let Some(id) = glutin.viewport_maps.get(window_id) {
                                        glutin.resize(*id, *physical_size);
                                    }
                                }
                            }
                            winit::event::WindowEvent::ScaleFactorChanged {
                                new_inner_size,
                                ..
                            } => {
                                let glutin = &mut *running.glutin_ctx.borrow_mut();
                                repaint_asap = true;
                                if let Some(id) = glutin.viewport_maps.get(window_id) {
                                    glutin.resize(*id, **new_inner_size);
                                }
                            }
                            winit::event::WindowEvent::CloseRequested
                                if running
                                    .glutin_ctx
                                    .borrow()
                                    .viewport_maps
                                    .get(window_id)
                                    .map_or(false, |id| *id == ViewportId::ROOT)
                                    && running.integration.borrow().should_close() =>
                            {
                                log::debug!("Received WindowEvent::CloseRequested");
                                return Ok(EventResult::Exit);
                            }
                            _ => {}
                        }

                        let event_response = 'res: {
                            let glutin = running.glutin_ctx.borrow();
                            if let Some(viewport_id) = glutin.viewport_maps.get(window_id).copied()
                            {
                                if let Some(viewport) = glutin.viewports.get(&viewport_id).cloned()
                                {
                                    let viewport = &mut *viewport.borrow_mut();

                                    break 'res running.integration.borrow_mut().on_event(
                                        running.app.borrow_mut().as_mut(),
                                        event,
                                        viewport.egui_winit.as_mut().unwrap(),
                                        viewport.id_pair.this,
                                    );
                                }
                            }

                            EventResponse {
                                consumed: false,
                                repaint: false,
                            }
                        };

                        if running.integration.borrow().should_close() {
                            EventResult::Exit
                        } else if event_response.repaint {
                            if repaint_asap {
                                EventResult::RepaintNow(*window_id)
                            } else {
                                EventResult::RepaintNext(*window_id)
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
                    accesskit_winit::ActionRequestEvent { request, window_id },
                )) => {
                    if let Some(running) = self.running.borrow().as_ref() {
                        crate::profile_scope!("on_accesskit_action_request");

                        let glutin = running.glutin_ctx.borrow();
                        if let Some(viewport_id) = glutin.viewport_maps.get(window_id).copied() {
                            if let Some(viewport) = glutin.viewports.get(&viewport_id).cloned() {
                                let mut viewport = viewport.borrow_mut();
                                viewport
                                    .egui_winit
                                    .as_mut()
                                    .unwrap()
                                    .on_accesskit_action_request(request.clone());
                            }
                        }
                        // As a form of user input, accessibility actions should
                        // lead to a repaint.
                        EventResult::RepaintNext(*window_id)
                    } else {
                        EventResult::Wait
                    }
                }
                _ => EventResult::Wait,
            })
        }
    }

    pub fn run_glow(
        app_name: &str,
        mut native_options: epi::NativeOptions,
        app_creator: epi::AppCreator,
    ) -> Result<()> {
        #[cfg(not(target_os = "ios"))]
        if native_options.run_and_return {
            return with_event_loop(native_options, |event_loop, native_options| {
                let glow_eframe =
                    GlowWinitApp::new(event_loop, app_name, native_options, app_creator);
                run_and_return(event_loop, glow_eframe)
            });
        }

        let event_loop = create_event_loop(&mut native_options);
        let glow_eframe = GlowWinitApp::new(&event_loop, app_name, native_options, app_creator);
        run_and_exit(event_loop, glow_eframe);
    }
}

#[cfg(feature = "glow")]
pub use glow_integration::run_glow;
// ----------------------------------------------------------------------------

#[cfg(feature = "wgpu")]
mod wgpu_integration {
    use parking_lot::Mutex;

    use egui::{ViewportIdMap, ViewportIdPair, ViewportIdSet, ViewportOutput, ViewportUiCallback};
    use egui_winit::{create_winit_window_builder, process_viewport_commands};

    use super::*;

    #[derive(Clone)]
    pub struct Viewport {
        window: Option<Rc<RefCell<winit::window::Window>>>,

        state: Rc<RefCell<Option<egui_winit::State>>>,

        /// `None` for sync viewports.
        viewport_ui_cb: Option<Arc<Box<ViewportUiCallback>>>,

        parent_id: ViewportId,
    }

    #[derive(Clone, Default)]
    pub struct Viewports(ViewportIdMap<Viewport>);

    impl std::ops::Deref for Viewports {
        type Target = ViewportIdMap<Viewport>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl std::ops::DerefMut for Viewports {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    /// State that is initialized when the application is first starts running via
    /// a Resumed event. On Android this ensures that any graphics state is only
    /// initialized once the application has an associated `SurfaceView`.
    struct WgpuWinitRunning {
        painter: Rc<RefCell<egui_wgpu::winit::Painter>>,
        integration: Rc<RefCell<epi_integration::EpiIntegration>>,
        app: Box<dyn epi::App>,
        viewports: Rc<RefCell<Viewports>>,
        builders: Rc<RefCell<ViewportIdMap<ViewportBuilder>>>,
        viewport_maps: Rc<RefCell<HashMap<winit::window::WindowId, ViewportId>>>,
    }

    struct WgpuWinitApp {
        repaint_proxy: Arc<Mutex<EventLoopProxy<UserEvent>>>,
        app_name: String,
        native_options: epi::NativeOptions,
        app_creator: Option<epi::AppCreator>,
        running: Option<WgpuWinitRunning>,

        /// Window surface state that's initialized when the app starts running via a Resumed event
        /// and on Android will also be destroyed if the application is paused.
        is_focused: Rc<RefCell<Option<ViewportId>>>,
    }

    impl WgpuWinitApp {
        fn new(
            event_loop: &EventLoop<UserEvent>,
            app_name: &str,
            native_options: epi::NativeOptions,
            app_creator: epi::AppCreator,
        ) -> Self {
            crate::profile_function!();
            #[cfg(feature = "__screenshot")]
            assert!(
                std::env::var("EFRAME_SCREENSHOT_TO").is_err(),
                "EFRAME_SCREENSHOT_TO not yet implemented for wgpu backend"
            );

            Self {
                repaint_proxy: Arc::new(Mutex::new(event_loop.create_proxy())),
                app_name: app_name.to_owned(),
                native_options,
                running: None,
                app_creator: Some(app_creator),
                is_focused: Rc::new(RefCell::new(Some(ViewportId::ROOT))),
            }
        }

        fn create_window(
            event_loop: &EventLoopWindowTarget<UserEvent>,
            storage: Option<&dyn epi::Storage>,
            title: &str,
            native_options: &mut NativeOptions,
        ) -> std::result::Result<(winit::window::Window, ViewportBuilder), winit::error::OsError>
        {
            crate::profile_function!();

            let window_settings = epi_integration::load_window_settings(storage);
            let window_builder =
                epi_integration::window_builder(event_loop, title, native_options, window_settings);
            let window = {
                crate::profile_scope!("WindowBuilder::build");
                create_winit_window_builder(&window_builder).build(event_loop)?
            };
            epi_integration::apply_native_options_to_window(
                &window,
                native_options,
                window_settings,
            );
            Ok((window, window_builder))
        }

        fn build_windows(&mut self, event_loop: &EventLoopWindowTarget<UserEvent>) {
            let Some(running) = &mut self.running else {
                return;
            };
            let viewport_builders = running.builders.borrow();

            for (id, Viewport { window, state, .. }) in running.viewports.borrow_mut().iter_mut() {
                let builder = viewport_builders.get(id).unwrap();
                if window.is_some() {
                    continue;
                }

                Self::init_window(
                    *id,
                    builder,
                    &mut running.viewport_maps.borrow_mut(),
                    &mut running.painter.borrow_mut(),
                    window,
                    &mut state.borrow_mut(),
                    event_loop,
                );
            }
        }

        fn init_window(
            id: ViewportId,
            builder: &ViewportBuilder,
            windows_id: &mut HashMap<winit::window::WindowId, ViewportId>,
            painter: &mut egui_wgpu::winit::Painter,
            window: &mut Option<Rc<RefCell<winit::window::Window>>>,
            state: &mut Option<egui_winit::State>,
            event_loop: &EventLoopWindowTarget<UserEvent>,
        ) {
            if let Ok(new_window) = create_winit_window_builder(builder).build(event_loop) {
                windows_id.insert(new_window.id(), id);

                if let Err(err) = pollster::block_on(painter.set_window(id, Some(&new_window))) {
                    log::error!("on set_window: viewport_id {id:?} {err}");
                }
                *window = Some(Rc::new(RefCell::new(new_window)));
                *state = Some(egui_winit::State::new(event_loop));
            }
        }

        fn set_window(&mut self, id: ViewportId) -> std::result::Result<(), egui_wgpu::WgpuError> {
            if let Some(running) = &mut self.running {
                crate::profile_function!();
                if let Some(Viewport { window, .. }) = running.viewports.borrow().get(&id) {
                    let window = window.clone();
                    if let Some(window) = &window {
                        return pollster::block_on(
                            running
                                .painter
                                .borrow_mut()
                                .set_window(id, Some(&*window.borrow())),
                        );
                    } else {
                        return pollster::block_on(
                            running.painter.borrow_mut().set_window(id, None),
                        );
                    };
                }
            }
            Ok(())
        }

        #[allow(unsafe_code)]
        #[cfg(target_os = "android")]
        fn drop_window(&mut self) -> std::result::Result<(), egui_wgpu::WgpuError> {
            if let Some(running) = &mut self.running {
                running.viewports.borrow_mut().remove(&ViewportId::ROOT);
                pollster::block_on(
                    running
                        .painter
                        .borrow_mut()
                        .set_window(ViewportId::ROOT, None),
                )?;
            }
            Ok(())
        }

        fn init_run_state(
            &mut self,
            event_loop: &EventLoopWindowTarget<UserEvent>,
            storage: Option<Box<dyn epi::Storage>>,
            window: winit::window::Window,
            builder: ViewportBuilder,
        ) -> std::result::Result<(), egui_wgpu::WgpuError> {
            crate::profile_function!();

            #[allow(unsafe_code, unused_mut, unused_unsafe)]
            let mut painter = egui_wgpu::winit::Painter::new(
                self.native_options.wgpu_options.clone(),
                self.native_options.multisampling.max(1) as _,
                egui_wgpu::depth_format_from_bits(
                    self.native_options.depth_buffer,
                    self.native_options.stencil_buffer,
                ),
                self.native_options.transparent,
            );
            pollster::block_on(painter.set_window(ViewportId::ROOT, Some(&window)))?;

            let wgpu_render_state = painter.render_state();

            let system_theme = system_theme(&window, &self.native_options);
            let mut integration = epi_integration::EpiIntegration::new(
                &window,
                system_theme,
                &self.app_name,
                &self.native_options,
                storage,
                IS_DESKTOP,
                #[cfg(feature = "glow")]
                None,
                wgpu_render_state.clone(),
            );

            {
                let event_loop_proxy = self.repaint_proxy.clone();

                integration
                    .egui_ctx
                    .set_request_repaint_callback(move |info| {
                        log::trace!("request_repaint_callback: {info:?}");
                        let when = Instant::now() + info.after;
                        let frame_nr = info.current_frame_nr;

                        event_loop_proxy
                            .lock()
                            .send_event(UserEvent::RequestRepaint {
                                when,
                                frame_nr,
                                viewport_id: info.viewport_id,
                            })
                            .ok();
                    });
            }

            let mut state = egui_winit::State::new(event_loop);
            #[cfg(feature = "accesskit")]
            {
                let event_loop_proxy = self.repaint_proxy.lock().clone();
                integration.init_accesskit(&mut state, &window, event_loop_proxy);
            }
            let theme = system_theme.unwrap_or(self.native_options.default_theme);
            integration.egui_ctx.set_visuals(theme.egui_visuals());

            let app_creator = std::mem::take(&mut self.app_creator)
                .expect("Single-use AppCreator has unexpectedly already been taken");
            let cc = epi::CreationContext {
                egui_ctx: integration.egui_ctx.clone(),
                integration_info: integration.frame.info().clone(),
                storage: integration.frame.storage(),
                #[cfg(feature = "glow")]
                gl: None,
                wgpu_render_state,
                raw_display_handle: window.raw_display_handle(),
                raw_window_handle: window.raw_window_handle(),
            };
            let mut app = {
                crate::profile_scope!("user_app_creator");
                app_creator(&cc)
            };

            if app.warm_up_enabled() {
                integration.warm_up(app.as_mut(), &window, &mut state);
            }

            let mut viewport_maps = HashMap::default();
            viewport_maps.insert(window.id(), ViewportId::ROOT);
            let viewport_maps = Rc::new(RefCell::new(viewport_maps));

            let viewports = Rc::new(RefCell::new(Viewports::default()));
            viewports.borrow_mut().insert(
                ViewportId::ROOT,
                Viewport {
                    window: Some(Rc::new(RefCell::new(window))),
                    state: Rc::new(RefCell::new(Some(state))),
                    viewport_ui_cb: None,
                    parent_id: ViewportId::ROOT,
                },
            );

            let builders = Rc::new(RefCell::new(ViewportIdMap::default()));
            builders.borrow_mut().insert(ViewportId::ROOT, builder);

            let painter = Rc::new(RefCell::new(painter));

            {
                // Create weak pointers so that we don't keep
                // state alive for too long.
                let viewports = Rc::downgrade(&viewports);
                let builders = Rc::downgrade(&builders);
                let painter = Rc::downgrade(&painter);
                let viewport_maps = Rc::downgrade(&viewport_maps);
                let beginning = integration.beginning;

                integration.egui_ctx.set_render_sync_callback(
                    move |egui_ctx, viewport_builder, id_pair, viewport_ui_cb| {
                        if let (
                            Some(viewports),
                            Some(builders),
                            Some(painter),
                            Some(viewport_maps),
                        ) = (
                            viewports.upgrade(),
                            builders.upgrade(),
                            painter.upgrade(),
                            viewport_maps.upgrade(),
                        ) {
                            Self::render_sync_viewport(
                                egui_ctx,
                                viewport_builder,
                                id_pair,
                                viewport_ui_cb,
                                &viewports,
                                &builders,
                                beginning,
                                &painter,
                                &viewport_maps,
                            );
                        } else {
                            log::warn!("render_sync_callback called after window closed");
                        }
                    },
                );
            }

            self.running = Some(WgpuWinitRunning {
                painter,
                integration: Rc::new(RefCell::new(integration)),
                app,
                viewports,
                viewport_maps,
                builders,
            });

            Ok(())
        }

        #[inline(always)]
        #[allow(clippy::too_many_arguments)]
        fn render_sync_viewport(
            egui_ctx: &egui::Context,
            mut viewport_builder: ViewportBuilder,
            id_pair: ViewportIdPair,
            viewport_ui_cb: Box<dyn FnOnce(&egui::Context) + '_>,
            viewports: &RefCell<Viewports>,
            builders: &RefCell<ViewportIdMap<ViewportBuilder>>,
            beginning: Instant,
            painter: &RefCell<egui_wgpu::winit::Painter>,
            viewport_maps: &RefCell<HashMap<winit::window::WindowId, ViewportId>>,
        ) {
            crate::profile_function!();

            // Creating a new native window if is needed
            if viewports.borrow().get(&id_pair.this).is_none() {
                let mut builders = builders.borrow_mut();

                {
                    if viewport_builder.icon.is_none() && builders.get(&id_pair.this).is_none() {
                        viewport_builder.icon =
                            builders.get(&id_pair.parent).and_then(|b| b.icon.clone());
                    }
                }

                let mut viewports = viewports.borrow_mut();

                let Viewport { window, state, .. } =
                    viewports.entry(id_pair.this).or_insert(Viewport {
                        window: None,
                        state: Rc::new(RefCell::new(None)),
                        viewport_ui_cb: None,
                        parent_id: id_pair.parent,
                    });
                builders
                    .entry(id_pair.this)
                    .or_insert(viewport_builder.clone());

                #[allow(unsafe_code)]
                let event_loop = unsafe {
                    WINIT_EVENT_LOOP.with(|event_loop| {
                        event_loop.borrow().as_ref().expect("No winit event loop")
                    })
                };

                Self::init_window(
                    id_pair.this,
                    &viewport_builder,
                    &mut viewport_maps.borrow_mut(),
                    &mut painter.borrow_mut(),
                    window,
                    &mut state.borrow_mut(),
                    event_loop,
                );
            }

            // Render sync viewport:
            let viewport = viewports.borrow().get(&id_pair.this).cloned();
            let Some(viewport) = viewport else { return };
            let Some(winit_state) = &mut *viewport.state.borrow_mut() else {
                return;
            };
            let Some(window) = viewport.window else {
                return;
            };
            let win = window.borrow();
            let mut input = winit_state.take_egui_input(&win);
            input.time = Some(beginning.elapsed().as_secs_f64());
            let output = egui_ctx.run(input, id_pair, |ctx| {
                viewport_ui_cb(ctx);
            });

            let mut painter = painter.borrow_mut();

            if let Err(err) = pollster::block_on(painter.set_window(id_pair.this, Some(&win))) {
                log::error!(
                    "when rendering viewport_id={:?}, set_window Error {err}",
                    id_pair.this
                );
            }

            let pixels_per_point = egui_ctx.input_for(id_pair.this, |i| i.pixels_per_point());
            let clipped_primitives = egui_ctx.tessellate(output.shapes, id_pair.this);
            painter.paint_and_update_textures(
                id_pair.this,
                pixels_per_point,
                [0.0, 0.0, 0.0, 0.0],
                &clipped_primitives,
                &output.textures_delta,
                false,
            );

            winit_state.handle_platform_output(
                &win,
                id_pair.this,
                egui_ctx,
                output.platform_output,
            );
        }
    }

    impl WinitApp for WgpuWinitApp {
        fn frame_nr(&self, viewport_id: ViewportId) -> u64 {
            self.running.as_ref().map_or(0, |r| {
                r.integration.borrow().egui_ctx.frame_nr_for(viewport_id)
            })
        }

        fn is_focused(&self, window_id: winit::window::WindowId) -> bool {
            if let Some(focus) = *self.is_focused.borrow() {
                self.viewport_id_from_window_id(&window_id)
                    .map_or(false, |i| i == focus)
            } else {
                false
            }
        }

        fn integration(&self) -> Option<Rc<RefCell<EpiIntegration>>> {
            self.running.as_ref().map(|r| r.integration.clone())
        }

        fn window(
            &self,
            window_id: winit::window::WindowId,
        ) -> Option<Rc<RefCell<winit::window::Window>>> {
            self.running
                .as_ref()
                .and_then(|r| {
                    r.viewport_maps
                        .borrow()
                        .get(&window_id)
                        .and_then(|id| r.viewports.borrow().get(id).map(|w| w.window.clone()))
                })
                .flatten()
        }

        fn window_id_from_viewport_id(&self, id: ViewportId) -> Option<winit::window::WindowId> {
            self.running.as_ref().and_then(|r| {
                r.viewports
                    .borrow()
                    .get(&id)
                    .and_then(|w| w.window.as_ref().map(|w| w.borrow().id()))
            })
        }

        fn save_and_destroy(&mut self) {
            if let Some(mut running) = self.running.take() {
                crate::profile_function!();
                if let Some(Viewport { window, .. }) =
                    running.viewports.borrow().get(&ViewportId::ROOT)
                {
                    running.integration.borrow_mut().save(
                        running.app.as_mut(),
                        window.as_ref().map(|w| w.borrow()).as_deref(),
                    );
                }

                #[cfg(feature = "glow")]
                running.app.on_exit(None);

                #[cfg(not(feature = "glow"))]
                running.app.on_exit();

                running.painter.borrow_mut().destroy();
            }
        }

        fn run_ui_and_paint(&mut self, window_id: winit::window::WindowId) -> EventResult {
            if let Some(running) = &mut self.running {
                #[cfg(feature = "puffin")]
                puffin::GlobalProfiler::lock().new_frame();
                crate::profile_scope!("frame");

                let WgpuWinitRunning {
                    app,
                    integration,
                    painter,
                    viewports,
                    viewport_maps,
                    builders,
                } = running;

                let egui::FullOutput {
                    platform_output,
                    textures_delta,
                    shapes,
                    viewports: mut out_viewports,
                    viewport_commands,
                };

                {
                    let Some((
                        viewport_id,
                        Viewport {
                            window: Some(window),
                            state,
                            viewport_ui_cb,
                            parent_id,
                        },
                    )) = viewport_maps
                        .borrow()
                        .get(&window_id)
                        .and_then(|id| (viewports.borrow().get(id).map(|w| (*id, w.clone()))))
                    else {
                        return EventResult::Wait;
                    };
                    // This is used to not render a viewport if is sync
                    if viewport_id != ViewportId::ROOT && viewport_ui_cb.is_none() {
                        if let Some(viewport) = running.viewports.borrow().get(&parent_id) {
                            if let Some(window) = viewport.window.as_ref() {
                                return EventResult::RepaintNext(window.borrow().id());
                            }
                        }
                        return EventResult::Wait;
                    }

                    let _ = pollster::block_on(
                        painter
                            .borrow_mut()
                            .set_window(viewport_id, Some(&window.borrow())),
                    );

                    egui::FullOutput {
                        platform_output,
                        textures_delta,
                        shapes,
                        viewports: out_viewports,
                        viewport_commands,
                    } = integration.borrow_mut().update(
                        app.as_mut(),
                        &window.borrow(),
                        state.borrow_mut().as_mut().unwrap(),
                        &viewport_ui_cb.clone(),
                        ViewportIdPair {
                            this: viewport_id,
                            parent: parent_id,
                        },
                    );

                    integration.borrow_mut().handle_platform_output(
                        &window.borrow(),
                        viewport_id,
                        platform_output,
                        state.borrow_mut().as_mut().unwrap(),
                    );

                    let clipped_primitives = {
                        crate::profile_scope!("tessellate");
                        integration
                            .borrow()
                            .egui_ctx
                            .tessellate(shapes, viewport_id)
                    };

                    let integration = &mut *integration.borrow_mut();
                    let screenshot_requested = &mut integration.frame.output.screenshot_requested;

                    let pixels_per_point = integration
                        .egui_ctx
                        .input_for(viewport_id, |i| i.pixels_per_point());

                    let screenshot = painter.borrow_mut().paint_and_update_textures(
                        viewport_id,
                        pixels_per_point,
                        app.clear_color(&integration.egui_ctx.style().visuals),
                        &clipped_primitives,
                        &textures_delta,
                        *screenshot_requested,
                    );
                    *screenshot_requested = false;
                    integration.frame.screenshot.set(screenshot);

                    integration.post_rendering(app.as_mut(), &window.borrow());
                    integration.post_present(&window.borrow());
                }

                let mut active_viewports_ids = ViewportIdSet::default();
                active_viewports_ids.insert(ViewportId::ROOT);

                out_viewports.retain_mut(
                    |ViewportOutput {
                         id_pair: ViewportIdPair { this, parent },
                         viewport_ui_cb,
                         ..
                     }| {
                        if let Some(viewport) = viewports.borrow_mut().get_mut(this) {
                            viewport.viewport_ui_cb = viewport_ui_cb.clone();
                            viewport.parent_id = *parent;
                            active_viewports_ids.insert(*this);
                            false
                        } else {
                            true
                        }
                    },
                );

                for ViewportOutput {
                    builder: mut new_builder,
                    id_pair,
                    viewport_ui_cb,
                } in out_viewports
                {
                    let mut builders = builders.borrow_mut();
                    let mut viewports = viewports.borrow_mut();

                    if new_builder.icon.is_none() {
                        new_builder.icon = builders
                            .get_mut(&id_pair.parent)
                            .and_then(|w| w.icon.clone());
                    }

                    if let Some(builder) = builders.get_mut(&id_pair.this) {
                        let (commands, recreate) = builder.patch(&new_builder);

                        if recreate {
                            if let Some(viewport) = viewports.get_mut(&id_pair.this) {
                                viewport.window = None;
                                viewport.state.replace(None);
                            }
                        } else if let Some(Viewport {
                            window: Some(window),
                            ..
                        }) = viewports.get(&id_pair.this)
                        {
                            process_viewport_commands(
                                commands,
                                id_pair.this,
                                None,
                                &window.borrow(),
                            );
                        }
                    } else {
                        viewports.insert(
                            id_pair.this,
                            Viewport {
                                window: None,
                                state: Rc::new(RefCell::new(None)),
                                viewport_ui_cb,
                                parent_id: id_pair.parent,
                            },
                        );
                        builders.insert(id_pair.this, new_builder);
                    }

                    active_viewports_ids.insert(id_pair.this);
                }

                for (viewport_id, command) in viewport_commands {
                    if let Some(window) = viewports
                        .borrow()
                        .get(&viewport_id)
                        .and_then(|w| w.window.clone())
                    {
                        egui_winit::process_viewport_commands(
                            vec![command],
                            viewport_id,
                            *self.is_focused.borrow(),
                            &window.borrow(),
                        );
                    }
                }

                viewports
                    .borrow_mut()
                    .retain(|id, _| active_viewports_ids.contains(id));
                builders
                    .borrow_mut()
                    .retain(|id, _| active_viewports_ids.contains(id));
                viewport_maps
                    .borrow_mut()
                    .retain(|_, id| active_viewports_ids.contains(id));
                painter.borrow_mut().clean_surfaces(&active_viewports_ids);

                let Some(
                    Viewport {
                        window: Some(window),
                        ..
                    },
                ) = viewport_maps
                    .borrow()
                    .get(&window_id)
                    .and_then(|id| viewports.borrow().get(id).cloned())
                else {
                    return EventResult::Wait;
                };
                integration
                    .borrow_mut()
                    .maybe_autosave(app.as_mut(), Some(&*window.borrow()));

                if window.borrow().is_minimized() == Some(true) {
                    // On Mac, a minimized Window uses up all CPU:
                    // https://github.com/emilk/egui/issues/325
                    crate::profile_scope!("minimized_sleep");
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }

                if integration.borrow().should_close() {
                    EventResult::Exit
                } else {
                    EventResult::Wait
                }
            } else {
                EventResult::Wait
            }
        }

        fn on_event(
            &mut self,
            event_loop: &EventLoopWindowTarget<UserEvent>,
            event: &winit::event::Event<'_, UserEvent>,
        ) -> Result<EventResult> {
            crate::profile_function!();
            self.build_windows(event_loop);

            Ok(match event {
                winit::event::Event::Resumed => {
                    if let Some(running) = &self.running {
                        if running.viewports.borrow().get(&ViewportId::ROOT).is_none() {
                            let _ = Self::create_window(
                                event_loop,
                                running.integration.borrow().frame.storage(),
                                &self.app_name,
                                &mut self.native_options,
                            )?;
                            self.set_window(ViewportId::ROOT)?;
                        }
                    } else {
                        let storage = epi_integration::create_storage(
                            self.native_options
                                .app_id
                                .as_ref()
                                .unwrap_or(&self.app_name),
                        );
                        let (window, builder) = Self::create_window(
                            event_loop,
                            storage.as_deref(),
                            &self.app_name,
                            &mut self.native_options,
                        )?;
                        self.init_run_state(event_loop, storage, window, builder)?;
                    }
                    EventResult::RepaintNow(
                        self.running
                            .as_ref()
                            .unwrap()
                            .viewports
                            .borrow()
                            .get(&ViewportId::ROOT)
                            .unwrap()
                            .window
                            .as_ref()
                            .unwrap()
                            .borrow()
                            .id(),
                    )
                }
                winit::event::Event::Suspended => {
                    #[cfg(target_os = "android")]
                    self.drop_window()?;
                    EventResult::Wait
                }

                winit::event::Event::WindowEvent { event, window_id } => {
                    let viewport_id = self.viewport_id_from_window_id(window_id);
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
                                *self.is_focused.borrow_mut() =
                                    new_focused.then(|| viewport_id).flatten();
                            }
                            winit::event::WindowEvent::Resized(physical_size) => {
                                repaint_asap = true;

                                // Resize with 0 width and height is used by winit to signal a minimize event on Windows.
                                // See: https://github.com/rust-windowing/winit/issues/208
                                // This solves an issue where the app would panic when minimizing on Windows.
                                if let Some(viewport_id) = viewport_id {
                                    use std::num::NonZeroU32;
                                    if let (Some(width), Some(height)) = (
                                        NonZeroU32::new(physical_size.width),
                                        NonZeroU32::new(physical_size.height),
                                    ) {
                                        running.painter.borrow_mut().on_window_resized(
                                            viewport_id,
                                            width,
                                            height,
                                        );
                                    }
                                }
                            }
                            winit::event::WindowEvent::ScaleFactorChanged {
                                new_inner_size,
                                ..
                            } => {
                                use std::num::NonZeroU32;
                                if let (Some(width), Some(height), Some(viewport_id)) = (
                                    NonZeroU32::new(new_inner_size.width),
                                    NonZeroU32::new(new_inner_size.height),
                                    running.viewport_maps.borrow().get(window_id).copied(),
                                ) {
                                    repaint_asap = true;
                                    running.painter.borrow_mut().on_window_resized(
                                        viewport_id,
                                        width,
                                        height,
                                    );
                                }
                            }
                            winit::event::WindowEvent::CloseRequested
                                if running.integration.borrow().should_close() =>
                            {
                                log::debug!("Received WindowEvent::CloseRequested");
                                return Ok(EventResult::Exit);
                            }
                            _ => {}
                        };

                        let event_response = if let Some((id, Viewport { state, .. })) = viewport_id
                            .and_then(|id| {
                                running.viewports.borrow().get(&id).map(|w| (id, w.clone()))
                            }) {
                            if let Some(state) = &mut *state.borrow_mut() {
                                Some(running.integration.borrow_mut().on_event(
                                    running.app.as_mut(),
                                    event,
                                    state,
                                    id,
                                ))
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        if running.integration.borrow().should_close() {
                            EventResult::Exit
                        } else if let Some(event_response) = event_response {
                            if event_response.repaint {
                                if repaint_asap {
                                    EventResult::RepaintNow(*window_id)
                                } else {
                                    EventResult::RepaintNext(*window_id)
                                }
                            } else {
                                EventResult::Wait
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
                    accesskit_winit::ActionRequestEvent { request, window_id },
                )) => {
                    if let Some(running) = &mut self.running {
                        if let Some(Viewport { state, .. }) = running
                            .viewport_maps
                            .borrow()
                            .get(window_id)
                            .and_then(|id| running.viewports.borrow().get(id).cloned())
                        {
                            if let Some(state) = &mut *state.borrow_mut() {
                                state.on_accesskit_action_request(request.clone());
                            }
                        }
                        // As a form of user input, accessibility actions should
                        // lead to a repaint.
                        EventResult::RepaintNext(*window_id)
                    } else {
                        EventResult::Wait
                    }
                }
                _ => EventResult::Wait,
            })
        }

        fn viewport_id_from_window_id(&self, id: &winit::window::WindowId) -> Option<ViewportId> {
            self.running
                .as_ref()
                .and_then(|r| r.viewport_maps.borrow().get(id).copied())
        }
    }

    pub fn run_wgpu(
        app_name: &str,
        mut native_options: epi::NativeOptions,
        app_creator: epi::AppCreator,
    ) -> Result<()> {
        #[cfg(not(target_os = "ios"))]
        if native_options.run_and_return {
            return with_event_loop(native_options, |event_loop, native_options| {
                let wgpu_eframe =
                    WgpuWinitApp::new(event_loop, app_name, native_options, app_creator);
                run_and_return(event_loop, wgpu_eframe)
            });
        }

        let event_loop = create_event_loop(&mut native_options);
        let wgpu_eframe = WgpuWinitApp::new(&event_loop, app_name, native_options, app_creator);
        run_and_exit(event_loop, wgpu_eframe);
    }
}

#[cfg(feature = "wgpu")]
pub use wgpu_integration::run_wgpu;

// ----------------------------------------------------------------------------

fn system_theme(window: &winit::window::Window, options: &NativeOptions) -> Option<crate::Theme> {
    if options.follow_system_theme {
        window
            .theme()
            .map(super::epi_integration::theme_from_winit_theme)
    } else {
        None
    }
}

// For the puffin profiler!
#[allow(dead_code)] // Only used for profiling
fn short_event_description(event: &winit::event::Event<'_, UserEvent>) -> &'static str {
    use winit::event::{DeviceEvent, Event, StartCause, WindowEvent};

    match event {
        Event::Suspended => "Event::Suspended",
        Event::Resumed => "Event::Resumed",
        Event::MainEventsCleared => "Event::MainEventsCleared",
        Event::RedrawRequested(_) => "Event::RedrawRequested",
        Event::RedrawEventsCleared => "Event::RedrawEventsCleared",
        Event::LoopDestroyed => "Event::LoopDestroyed",
        Event::UserEvent(user_event) => match user_event {
            UserEvent::RequestRepaint { .. } => "UserEvent::RequestRepaint",
            #[cfg(feature = "accesskit")]
            UserEvent::AccessKitActionRequest(_) => "UserEvent::AccessKitActionRequest",
        },
        Event::DeviceEvent { event, .. } => match event {
            DeviceEvent::Added { .. } => "DeviceEvent::Added",
            DeviceEvent::Removed { .. } => "DeviceEvent::Removed",
            DeviceEvent::MouseMotion { .. } => "DeviceEvent::MouseMotion",
            DeviceEvent::MouseWheel { .. } => "DeviceEvent::MouseWheel",
            DeviceEvent::Motion { .. } => "DeviceEvent::Motion",
            DeviceEvent::Button { .. } => "DeviceEvent::Button",
            DeviceEvent::Key { .. } => "DeviceEvent::Key",
            DeviceEvent::Text { .. } => "DeviceEvent::Text",
        },
        Event::NewEvents(start_cause) => match start_cause {
            StartCause::ResumeTimeReached { .. } => "NewEvents::ResumeTimeReached",
            StartCause::WaitCancelled { .. } => "NewEvents::WaitCancelled",
            StartCause::Poll => "NewEvents::Poll",
            StartCause::Init => "NewEvents::Init",
        },
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::Resized { .. } => "WindowEvent::Resized",
            WindowEvent::Moved { .. } => "WindowEvent::Moved",
            WindowEvent::CloseRequested { .. } => "WindowEvent::CloseRequested",
            WindowEvent::Destroyed { .. } => "WindowEvent::Destroyed",
            WindowEvent::DroppedFile { .. } => "WindowEvent::DroppedFile",
            WindowEvent::HoveredFile { .. } => "WindowEvent::HoveredFile",
            WindowEvent::HoveredFileCancelled { .. } => "WindowEvent::HoveredFileCancelled",
            WindowEvent::ReceivedCharacter { .. } => "WindowEvent::ReceivedCharacter",
            WindowEvent::Focused { .. } => "WindowEvent::Focused",
            WindowEvent::KeyboardInput { .. } => "WindowEvent::KeyboardInput",
            WindowEvent::ModifiersChanged { .. } => "WindowEvent::ModifiersChanged",
            WindowEvent::Ime { .. } => "WindowEvent::Ime",
            WindowEvent::CursorMoved { .. } => "WindowEvent::CursorMoved",
            WindowEvent::CursorEntered { .. } => "WindowEvent::CursorEntered",
            WindowEvent::CursorLeft { .. } => "WindowEvent::CursorLeft",
            WindowEvent::MouseWheel { .. } => "WindowEvent::MouseWheel",
            WindowEvent::MouseInput { .. } => "WindowEvent::MouseInput",
            WindowEvent::TouchpadMagnify { .. } => "WindowEvent::TouchpadMagnify",
            WindowEvent::SmartMagnify { .. } => "WindowEvent::SmartMagnify",
            WindowEvent::TouchpadRotate { .. } => "WindowEvent::TouchpadRotate",
            WindowEvent::TouchpadPressure { .. } => "WindowEvent::TouchpadPressure",
            WindowEvent::AxisMotion { .. } => "WindowEvent::AxisMotion",
            WindowEvent::Touch { .. } => "WindowEvent::Touch",
            WindowEvent::ScaleFactorChanged { .. } => "WindowEvent::ScaleFactorChanged",
            WindowEvent::ThemeChanged { .. } => "WindowEvent::ThemeChanged",
            WindowEvent::Occluded { .. } => "WindowEvent::Occluded",
        },
    }
}
