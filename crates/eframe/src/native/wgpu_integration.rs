//! Note that this file contains code very similar to [`glow_integration`].
//! When making changes to one you often also want to apply it to the other.
//!
//! This is also very complex code, and not very pretty.
//! There is a bunch of improvements we could do,
//! like removing a bunch of `unwraps`.

use std::{cell::RefCell, num::NonZeroU32, rc::Rc, sync::Arc, time::Instant};

use parking_lot::Mutex;
use raw_window_handle::{HasDisplayHandle as _, HasWindowHandle as _};
use winit::{
    event_loop::{EventLoop, EventLoopProxy, EventLoopWindowTarget},
    window::{Window, WindowId},
};

use egui::{
    ahash::HashMap, DeferredViewportUiCallback, FullOutput, ImmediateViewport, ViewportBuilder,
    ViewportClass, ViewportId, ViewportIdMap, ViewportIdPair, ViewportIdSet, ViewportInfo,
    ViewportOutput,
};
#[cfg(feature = "accesskit")]
use egui_winit::accesskit_winit;

use crate::{
    native::{epi_integration::EpiIntegration, winit_integration::EventResult},
    App, AppCreator, CreationContext, NativeOptions, Result, Storage, UserEvent,
};

use super::{winit_integration::WinitApp, *};

// ----------------------------------------------------------------------------
// Types:

pub struct WgpuWinitApp {
    repaint_proxy: Arc<Mutex<EventLoopProxy<UserEvent>>>,
    app_name: String,
    native_options: NativeOptions,

    /// Set at initialization, then taken and set to `None` in `init_run_state`.
    app_creator: Option<AppCreator>,

    /// Set when we are actually up and running.
    running: Option<WgpuWinitRunning>,
}

/// State that is initialized when the application is first starts running via
/// a Resumed event. On Android this ensures that any graphics state is only
/// initialized once the application has an associated `SurfaceView`.
struct WgpuWinitRunning {
    integration: EpiIntegration,

    /// The users application.
    app: Box<dyn App>,

    /// Wrapped in an `Rc<RefCell<…>>` so it can be re-entrantly shared via a weak-pointer.
    shared: Rc<RefCell<SharedState>>,
}

/// Everything needed by the immediate viewport renderer.\
///
/// This is shared by all viewports.
///
/// Wrapped in an `Rc<RefCell<…>>` so it can be re-entrantly shared via a weak-pointer.
pub struct SharedState {
    egui_ctx: egui::Context,
    viewports: Viewports,
    painter: egui_wgpu::winit::Painter,
    viewport_from_window: HashMap<WindowId, ViewportId>,
    focused_viewport: Option<ViewportId>,
}

pub type Viewports = ViewportIdMap<Viewport>;

pub struct Viewport {
    ids: ViewportIdPair,
    class: ViewportClass,
    builder: ViewportBuilder,
    info: ViewportInfo,
    screenshot_requested: bool,

    /// `None` for sync viewports.
    viewport_ui_cb: Option<Arc<DeferredViewportUiCallback>>,

    /// Window surface state that's initialized when the app starts running via a Resumed event
    /// and on Android will also be destroyed if the application is paused.
    window: Option<Arc<Window>>,

    /// `window` and `egui_winit` are initialized together.
    egui_winit: Option<egui_winit::State>,
}

// ----------------------------------------------------------------------------

impl WgpuWinitApp {
    pub fn new(
        event_loop: &EventLoop<UserEvent>,
        app_name: &str,
        native_options: NativeOptions,
        app_creator: AppCreator,
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
        }
    }

    /// Create a window for all viewports lacking one.
    fn initialized_all_windows(&mut self, event_loop: &EventLoopWindowTarget<UserEvent>) {
        let Some(running) = &mut self.running else {
            return;
        };
        let mut shared = running.shared.borrow_mut();
        let SharedState {
            viewports,
            painter,
            viewport_from_window,
            ..
        } = &mut *shared;

        for viewport in viewports.values_mut() {
            viewport.initialize_window(
                event_loop,
                &running.integration.egui_ctx,
                viewport_from_window,
                painter,
            );
        }
    }

    #[cfg(target_os = "android")]
    fn recreate_window(
        &self,
        event_loop: &EventLoopWindowTarget<UserEvent>,
        running: &WgpuWinitRunning,
    ) {
        let SharedState {
            egui_ctx,
            viewports,
            viewport_from_window,
            painter,
            ..
        } = &mut *running.shared.borrow_mut();

        initialize_or_update_viewport(
            egui_ctx,
            viewports,
            ViewportIdPair::ROOT,
            ViewportClass::Root,
            self.native_options.viewport.clone(),
            None,
            None,
        )
        .initialize_window(event_loop, egui_ctx, viewport_from_window, painter);
    }

    #[cfg(target_os = "android")]
    fn drop_window(&mut self) -> Result<(), egui_wgpu::WgpuError> {
        if let Some(running) = &mut self.running {
            let mut shared = running.shared.borrow_mut();
            shared.viewports.remove(&ViewportId::ROOT);
            pollster::block_on(shared.painter.set_window(ViewportId::ROOT, None))?;
        }
        Ok(())
    }

    fn init_run_state(
        &mut self,
        egui_ctx: egui::Context,
        event_loop: &EventLoopWindowTarget<UserEvent>,
        storage: Option<Box<dyn Storage>>,
        window: Window,
        builder: ViewportBuilder,
    ) -> Result<&mut WgpuWinitRunning, egui_wgpu::WgpuError> {
        crate::profile_function!();

        #[allow(unsafe_code, unused_mut, unused_unsafe)]
        let mut painter = egui_wgpu::winit::Painter::new(
            self.native_options.wgpu_options.clone(),
            self.native_options.multisampling.max(1) as _,
            egui_wgpu::depth_format_from_bits(
                self.native_options.depth_buffer,
                self.native_options.stencil_buffer,
            ),
            self.native_options.viewport.transparent.unwrap_or(false),
        );

        let window = Arc::new(window);

        {
            crate::profile_scope!("set_window");
            pollster::block_on(painter.set_window(ViewportId::ROOT, Some(window.clone())))?;
        }

        let wgpu_render_state = painter.render_state();

        let system_theme = winit_integration::system_theme(&window, &self.native_options);
        let integration = EpiIntegration::new(
            egui_ctx.clone(),
            &window,
            system_theme,
            &self.app_name,
            &self.native_options,
            storage,
            #[cfg(feature = "glow")]
            None,
            wgpu_render_state.clone(),
        );

        {
            let event_loop_proxy = self.repaint_proxy.clone();

            egui_ctx.set_request_repaint_callback(move |info| {
                log::trace!("request_repaint_callback: {info:?}");
                let when = Instant::now() + info.delay;
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

        #[allow(unused_mut)] // used for accesskit
        let mut egui_winit = egui_winit::State::new(
            egui_ctx.clone(),
            ViewportId::ROOT,
            event_loop,
            Some(window.scale_factor() as f32),
            painter.max_texture_side(),
        );

        #[cfg(feature = "accesskit")]
        {
            let event_loop_proxy = self.repaint_proxy.lock().clone();
            integration.init_accesskit(&mut egui_winit, &window, event_loop_proxy);
        }
        let theme = system_theme.unwrap_or(self.native_options.default_theme);
        egui_ctx.set_visuals(theme.egui_visuals());

        let app_creator = std::mem::take(&mut self.app_creator)
            .expect("Single-use AppCreator has unexpectedly already been taken");
        let cc = CreationContext {
            egui_ctx: egui_ctx.clone(),
            integration_info: integration.frame.info().clone(),
            storage: integration.frame.storage(),
            #[cfg(feature = "glow")]
            gl: None,
            #[cfg(feature = "glow")]
            get_proc_address: None,
            wgpu_render_state,
            raw_display_handle: window.display_handle().map(|h| h.as_raw()),
            raw_window_handle: window.window_handle().map(|h| h.as_raw()),
        };
        let app = {
            crate::profile_scope!("user_app_creator");
            app_creator(&cc)
        };

        let mut viewport_from_window = HashMap::default();
        viewport_from_window.insert(window.id(), ViewportId::ROOT);

        let mut viewports = Viewports::default();
        viewports.insert(
            ViewportId::ROOT,
            Viewport {
                ids: ViewportIdPair::ROOT,
                class: ViewportClass::Root,
                builder,
                info: ViewportInfo {
                    minimized: window.is_minimized(),
                    maximized: Some(window.is_maximized()),
                    ..Default::default()
                },
                screenshot_requested: false,
                viewport_ui_cb: None,
                window: Some(window),
                egui_winit: Some(egui_winit),
            },
        );

        let shared = Rc::new(RefCell::new(SharedState {
            egui_ctx,
            viewport_from_window,
            viewports,
            painter,
            focused_viewport: Some(ViewportId::ROOT),
        }));

        {
            // Create a weak pointer so that we don't keep state alive for too long.
            let shared = Rc::downgrade(&shared);
            let beginning = integration.beginning;

            let event_loop: *const EventLoopWindowTarget<UserEvent> = event_loop;

            egui::Context::set_immediate_viewport_renderer(move |_egui_ctx, immediate_viewport| {
                if let Some(shared) = shared.upgrade() {
                    // SAFETY: the event loop lives longer than
                    // the Rc:s we just upgraded above.
                    #[allow(unsafe_code)]
                    let event_loop = unsafe { event_loop.as_ref().unwrap() };

                    render_immediate_viewport(event_loop, beginning, &shared, immediate_viewport);
                } else {
                    log::warn!("render_sync_callback called after window closed");
                }
            });
        }

        Ok(self.running.insert(WgpuWinitRunning {
            integration,
            app,
            shared,
        }))
    }
}

impl WinitApp for WgpuWinitApp {
    fn frame_nr(&self, viewport_id: ViewportId) -> u64 {
        self.running
            .as_ref()
            .map_or(0, |r| r.integration.egui_ctx.frame_nr_for(viewport_id))
    }

    fn is_focused(&self, window_id: WindowId) -> bool {
        if let Some(running) = &self.running {
            let shared = running.shared.borrow();
            let viewport_id = shared.viewport_from_window.get(&window_id).copied();
            shared.focused_viewport.is_some() && shared.focused_viewport == viewport_id
        } else {
            false
        }
    }

    fn integration(&self) -> Option<&EpiIntegration> {
        self.running.as_ref().map(|r| &r.integration)
    }

    fn window(&self, window_id: WindowId) -> Option<Arc<Window>> {
        self.running
            .as_ref()
            .and_then(|r| {
                let shared = r.shared.borrow();
                shared
                    .viewport_from_window
                    .get(&window_id)
                    .and_then(|id| shared.viewports.get(id).map(|v| v.window.clone()))
            })
            .flatten()
    }

    fn window_id_from_viewport_id(&self, id: ViewportId) -> Option<WindowId> {
        Some(
            self.running
                .as_ref()?
                .shared
                .borrow()
                .viewports
                .get(&id)?
                .window
                .as_ref()?
                .id(),
        )
    }

    fn save_and_destroy(&mut self) {
        if let Some(mut running) = self.running.take() {
            running.save_and_destroy();
        }
    }

    fn run_ui_and_paint(
        &mut self,
        event_loop: &EventLoopWindowTarget<UserEvent>,
        window_id: WindowId,
    ) -> EventResult {
        self.initialized_all_windows(event_loop);

        if let Some(running) = &mut self.running {
            running.run_ui_and_paint(window_id)
        } else {
            EventResult::Wait
        }
    }

    fn on_event(
        &mut self,
        event_loop: &EventLoopWindowTarget<UserEvent>,
        event: &winit::event::Event<UserEvent>,
    ) -> Result<EventResult> {
        crate::profile_function!(winit_integration::short_event_description(event));

        self.initialized_all_windows(event_loop);

        Ok(match event {
            winit::event::Event::Resumed => {
                log::debug!("Event::Resumed");

                let running = if let Some(running) = &self.running {
                    #[cfg(target_os = "android")]
                    self.recreate_window(event_loop, running);
                    running
                } else {
                    let storage = epi_integration::create_storage(
                        self.native_options
                            .viewport
                            .app_id
                            .as_ref()
                            .unwrap_or(&self.app_name),
                    );
                    let egui_ctx = winit_integration::create_egui_context(storage.as_deref());
                    let (window, builder) = create_window(
                        &egui_ctx,
                        event_loop,
                        storage.as_deref(),
                        &mut self.native_options,
                    )?;
                    self.init_run_state(egui_ctx, event_loop, storage, window, builder)?
                };

                let viewport = &running.shared.borrow().viewports[&ViewportId::ROOT];
                if let Some(window) = &viewport.window {
                    EventResult::RepaintNow(window.id())
                } else {
                    EventResult::Wait
                }
            }

            winit::event::Event::Suspended => {
                #[cfg(target_os = "android")]
                self.drop_window()?;
                EventResult::Wait
            }

            winit::event::Event::WindowEvent { event, window_id } => {
                if let Some(running) = &mut self.running {
                    running.on_window_event(*window_id, event)
                } else {
                    EventResult::Wait
                }
            }

            winit::event::Event::DeviceEvent {
                device_id: _,
                event: winit::event::DeviceEvent::MouseMotion { delta },
            } => {
                if let Some(running) = &mut self.running {
                    let mut shared = running.shared.borrow_mut();
                    if let Some(viewport) = shared
                        .focused_viewport
                        .and_then(|viewport| shared.viewports.get_mut(&viewport))
                    {
                        if let Some(egui_winit) = viewport.egui_winit.as_mut() {
                            egui_winit.on_mouse_motion(*delta);
                        }

                        if let Some(window) = viewport.window.as_ref() {
                            EventResult::RepaintNext(window.id())
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
                    let mut shared_lock = running.shared.borrow_mut();
                    let SharedState {
                        viewport_from_window,
                        viewports,
                        ..
                    } = &mut *shared_lock;
                    if let Some(viewport) = viewport_from_window
                        .get(window_id)
                        .and_then(|id| viewports.get_mut(id))
                    {
                        if let Some(egui_winit) = &mut viewport.egui_winit {
                            egui_winit.on_accesskit_action_request(request.clone());
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

impl WgpuWinitRunning {
    fn save_and_destroy(&mut self) {
        crate::profile_function!();

        let mut shared = self.shared.borrow_mut();
        if let Some(Viewport { window, .. }) = shared.viewports.get(&ViewportId::ROOT) {
            self.integration.save(self.app.as_mut(), window.as_deref());
        }

        #[cfg(feature = "glow")]
        self.app.on_exit(None);

        #[cfg(not(feature = "glow"))]
        self.app.on_exit();

        shared.painter.destroy();
    }

    /// This is called both for the root viewport, and all deferred viewports
    fn run_ui_and_paint(&mut self, window_id: WindowId) -> EventResult {
        crate::profile_function!();

        let Some(viewport_id) = self
            .shared
            .borrow()
            .viewport_from_window
            .get(&window_id)
            .copied()
        else {
            return EventResult::Wait;
        };

        #[cfg(feature = "puffin")]
        puffin::GlobalProfiler::lock().new_frame();

        let Self {
            app,
            integration,
            shared,
        } = self;

        let mut frame_timer = crate::stopwatch::Stopwatch::new();
        frame_timer.start();

        let (viewport_ui_cb, raw_input) = {
            crate::profile_scope!("Prepare");
            let mut shared_lock = shared.borrow_mut();

            let SharedState {
                viewports, painter, ..
            } = &mut *shared_lock;

            if viewport_id != ViewportId::ROOT {
                let Some(viewport) = viewports.get(&viewport_id) else {
                    return EventResult::Wait;
                };

                if viewport.viewport_ui_cb.is_none() {
                    // This will only happen if this is an immediate viewport.
                    // That means that the viewport cannot be rendered by itself and needs his parent to be rendered.
                    if let Some(viewport) = viewports.get(&viewport.ids.parent) {
                        if let Some(window) = viewport.window.as_ref() {
                            return EventResult::RepaintNext(window.id());
                        }
                    }
                    return EventResult::Wait;
                }
            }

            let Some(viewport) = viewports.get_mut(&viewport_id) else {
                return EventResult::Wait;
            };

            let Viewport {
                viewport_ui_cb,
                window,
                egui_winit,
                info,
                ..
            } = viewport;

            let viewport_ui_cb = viewport_ui_cb.clone();

            let Some(window) = window else {
                return EventResult::Wait;
            };
            egui_winit::update_viewport_info(info, &integration.egui_ctx, window);

            {
                crate::profile_scope!("set_window");
                if let Err(err) =
                    pollster::block_on(painter.set_window(viewport_id, Some(window.clone())))
                {
                    log::warn!("Failed to set window: {err}");
                }
            }

            let Some(egui_winit) = egui_winit.as_mut() else {
                return EventResult::Wait;
            };
            let mut raw_input = egui_winit.take_egui_input(window);

            integration.pre_update();

            raw_input.time = Some(integration.beginning.elapsed().as_secs_f64());
            raw_input.viewports = viewports
                .iter()
                .map(|(id, viewport)| (*id, viewport.info.clone()))
                .collect();

            (viewport_ui_cb, raw_input)
        };

        // ------------------------------------------------------------

        // Runs the update, which could call immediate viewports,
        // so make sure we hold no locks here!
        let full_output = integration.update(app.as_mut(), viewport_ui_cb.as_deref(), raw_input);

        // ------------------------------------------------------------

        let mut shared = shared.borrow_mut();

        let SharedState {
            egui_ctx,
            viewports,
            painter,
            viewport_from_window,
            focused_viewport,
        } = &mut *shared;

        let Some(viewport) = viewports.get_mut(&viewport_id) else {
            return EventResult::Wait;
        };

        viewport.info.events.clear(); // they should have been processed

        let Viewport {
            window: Some(window),
            egui_winit: Some(egui_winit),
            ..
        } = viewport
        else {
            return EventResult::Wait;
        };

        let FullOutput {
            platform_output,
            textures_delta,
            shapes,
            pixels_per_point,
            viewport_output,
        } = full_output;

        egui_winit.handle_platform_output(window, platform_output);

        let clipped_primitives = egui_ctx.tessellate(shapes, pixels_per_point);

        let screenshot_requested = std::mem::take(&mut viewport.screenshot_requested);
        let (vsync_secs, screenshot) = painter.paint_and_update_textures(
            viewport_id,
            pixels_per_point,
            app.clear_color(&egui_ctx.style().visuals),
            &clipped_primitives,
            &textures_delta,
            screenshot_requested,
        );
        if let Some(screenshot) = screenshot {
            egui_winit
                .egui_input_mut()
                .events
                .push(egui::Event::Screenshot {
                    viewport_id,
                    image: screenshot.into(),
                });
        }

        integration.post_rendering(window);

        let active_viewports_ids: ViewportIdSet = viewport_output.keys().copied().collect();

        handle_viewport_output(
            &integration.egui_ctx,
            viewport_output,
            viewports,
            *focused_viewport,
        );

        // Prune dead viewports:
        viewports.retain(|id, _| active_viewports_ids.contains(id));
        viewport_from_window.retain(|_, id| active_viewports_ids.contains(id));
        painter.gc_viewports(&active_viewports_ids);

        let window = viewport_from_window
            .get(&window_id)
            .and_then(|id| viewports.get(id))
            .and_then(|vp| vp.window.as_ref());

        integration.report_frame_time(frame_timer.total_time_sec() - vsync_secs); // don't count auto-save time as part of regular frame time

        integration.maybe_autosave(app.as_mut(), window.map(|w| w.as_ref()));

        if let Some(window) = window {
            if window.is_minimized() == Some(true) {
                // On Mac, a minimized Window uses up all CPU:
                // https://github.com/emilk/egui/issues/325
                crate::profile_scope!("minimized_sleep");
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }

        if integration.should_close() {
            EventResult::Exit
        } else {
            EventResult::Wait
        }
    }

    fn on_window_event(
        &mut self,
        window_id: WindowId,
        event: &winit::event::WindowEvent,
    ) -> EventResult {
        crate::profile_function!(egui_winit::short_window_event_description(event));

        let Self {
            integration,
            shared,
            ..
        } = self;
        let mut shared = shared.borrow_mut();

        let viewport_id = shared.viewport_from_window.get(&window_id).copied();

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

        match event {
            winit::event::WindowEvent::Focused(new_focused) => {
                shared.focused_viewport = new_focused.then(|| viewport_id).flatten();
            }

            winit::event::WindowEvent::Resized(physical_size) => {
                // Resize with 0 width and height is used by winit to signal a minimize event on Windows.
                // See: https://github.com/rust-windowing/winit/issues/208
                // This solves an issue where the app would panic when minimizing on Windows.
                if let Some(viewport_id) = viewport_id {
                    if let (Some(width), Some(height)) = (
                        NonZeroU32::new(physical_size.width),
                        NonZeroU32::new(physical_size.height),
                    ) {
                        repaint_asap = true;
                        shared.painter.on_window_resized(viewport_id, width, height);
                    }
                }
            }

            winit::event::WindowEvent::CloseRequested => {
                if viewport_id == Some(ViewportId::ROOT) && integration.should_close() {
                    log::debug!(
                        "Received WindowEvent::CloseRequested for main viewport - shutting down."
                    );
                    return EventResult::Exit;
                }

                log::debug!("Received WindowEvent::CloseRequested for viewport {viewport_id:?}");

                if let Some(viewport_id) = viewport_id {
                    if let Some(viewport) = shared.viewports.get_mut(&viewport_id) {
                        // Tell viewport it should close:
                        viewport.info.events.push(egui::ViewportEvent::Close);

                        // We may need to repaint both us and our parent to close the window,
                        // and perhaps twice (once to notice the close-event, once again to enforce it).
                        // `request_repaint_of` does a double-repaint though:
                        integration.egui_ctx.request_repaint_of(viewport_id);
                        integration.egui_ctx.request_repaint_of(viewport.ids.parent);
                    }
                }
            }

            _ => {}
        };

        let event_response = viewport_id
            .and_then(|viewport_id| {
                shared.viewports.get_mut(&viewport_id).and_then(|viewport| {
                    Some(integration.on_window_event(
                        viewport.window.as_deref()?,
                        viewport.egui_winit.as_mut()?,
                        event,
                    ))
                })
            })
            .unwrap_or_default();

        if integration.should_close() {
            EventResult::Exit
        } else if event_response.repaint {
            if repaint_asap {
                EventResult::RepaintNow(window_id)
            } else {
                EventResult::RepaintNext(window_id)
            }
        } else {
            EventResult::Wait
        }
    }
}

impl Viewport {
    /// Create winit window, if needed.
    fn initialize_window(
        &mut self,
        event_loop: &EventLoopWindowTarget<UserEvent>,
        egui_ctx: &egui::Context,
        windows_id: &mut HashMap<WindowId, ViewportId>,
        painter: &mut egui_wgpu::winit::Painter,
    ) {
        if self.window.is_some() {
            return; // we already have one
        }

        crate::profile_function!();

        let viewport_id = self.ids.this;

        match egui_winit::create_window(egui_ctx, event_loop, &self.builder) {
            Ok(window) => {
                windows_id.insert(window.id(), viewport_id);

                let window = Arc::new(window);

                if let Err(err) =
                    pollster::block_on(painter.set_window(viewport_id, Some(window.clone())))
                {
                    log::error!("on set_window: viewport_id {viewport_id:?} {err}");
                }

                self.egui_winit = Some(egui_winit::State::new(
                    egui_ctx.clone(),
                    viewport_id,
                    event_loop,
                    Some(window.scale_factor() as f32),
                    painter.max_texture_side(),
                ));

                self.info.minimized = window.is_minimized();
                self.info.maximized = Some(window.is_maximized());

                self.window = Some(window);
            }
            Err(err) => {
                log::error!("Failed to create window: {err}");
            }
        }
    }
}

fn create_window(
    egui_ctx: &egui::Context,
    event_loop: &EventLoopWindowTarget<UserEvent>,
    storage: Option<&dyn Storage>,
    native_options: &mut NativeOptions,
) -> Result<(Window, ViewportBuilder), winit::error::OsError> {
    crate::profile_function!();

    let window_settings = epi_integration::load_window_settings(storage);
    let viewport_builder = epi_integration::viewport_builder(
        egui_ctx.zoom_factor(),
        event_loop,
        native_options,
        window_settings,
    )
    .with_visible(false); // Start hidden until we render the first frame to fix white flash on startup (https://github.com/emilk/egui/pull/3631)

    let window = egui_winit::create_window(egui_ctx, event_loop, &viewport_builder)?;
    epi_integration::apply_window_settings(&window, window_settings);
    Ok((window, viewport_builder))
}

fn render_immediate_viewport(
    event_loop: &EventLoopWindowTarget<UserEvent>,
    beginning: Instant,
    shared: &RefCell<SharedState>,
    immediate_viewport: ImmediateViewport<'_>,
) {
    crate::profile_function!();

    let ImmediateViewport {
        ids,
        builder,
        viewport_ui_cb,
    } = immediate_viewport;

    let input = {
        let SharedState {
            egui_ctx,
            viewports,
            painter,
            viewport_from_window,
            ..
        } = &mut *shared.borrow_mut();

        let viewport = initialize_or_update_viewport(
            egui_ctx,
            viewports,
            ids,
            ViewportClass::Immediate,
            builder,
            None,
            None,
        );
        if viewport.window.is_none() {
            viewport.initialize_window(event_loop, egui_ctx, viewport_from_window, painter);
        }

        let (Some(window), Some(egui_winit)) = (&viewport.window, &mut viewport.egui_winit) else {
            return;
        };
        egui_winit::update_viewport_info(&mut viewport.info, egui_ctx, window);

        let mut input = egui_winit.take_egui_input(window);
        input.viewports = viewports
            .iter()
            .map(|(id, viewport)| (*id, viewport.info.clone()))
            .collect();
        input.time = Some(beginning.elapsed().as_secs_f64());
        input
    };

    let egui_ctx = shared.borrow().egui_ctx.clone();

    // ------------------------------------------

    // Run the user code, which could re-entrantly call this function again (!).
    // Make sure no locks are held during this call.
    let egui::FullOutput {
        platform_output,
        textures_delta,
        shapes,
        pixels_per_point,
        viewport_output,
    } = egui_ctx.run(input, |ctx| {
        viewport_ui_cb(ctx);
    });

    // ------------------------------------------

    let mut shared = shared.borrow_mut();
    let SharedState {
        viewports,
        painter,
        focused_viewport,
        ..
    } = &mut *shared;

    let Some(viewport) = viewports.get_mut(&ids.this) else {
        return;
    };
    viewport.info.events.clear(); // they should have been processed
    let (Some(egui_winit), Some(window)) = (&mut viewport.egui_winit, &viewport.window) else {
        return;
    };

    {
        crate::profile_scope!("set_window");
        if let Err(err) = pollster::block_on(painter.set_window(ids.this, Some(window.clone()))) {
            log::error!(
                "when rendering viewport_id={:?}, set_window Error {err}",
                ids.this
            );
        }
    }

    let clipped_primitives = egui_ctx.tessellate(shapes, pixels_per_point);
    painter.paint_and_update_textures(
        ids.this,
        pixels_per_point,
        [0.0, 0.0, 0.0, 0.0],
        &clipped_primitives,
        &textures_delta,
        false,
    );

    egui_winit.handle_platform_output(window, platform_output);

    handle_viewport_output(&egui_ctx, viewport_output, viewports, *focused_viewport);
}

/// Add new viewports, and update existing ones:
fn handle_viewport_output(
    egui_ctx: &egui::Context,
    viewport_output: ViewportIdMap<ViewportOutput>,
    viewports: &mut ViewportIdMap<Viewport>,
    focused_viewport: Option<ViewportId>,
) {
    for (
        viewport_id,
        ViewportOutput {
            parent,
            class,
            builder,
            viewport_ui_cb,
            commands,
            repaint_delay: _, // ignored - we listened to the repaint callback instead
        },
    ) in viewport_output
    {
        let ids = ViewportIdPair::from_self_and_parent(viewport_id, parent);

        let viewport = initialize_or_update_viewport(
            egui_ctx,
            viewports,
            ids,
            class,
            builder,
            viewport_ui_cb,
            focused_viewport,
        );

        if let Some(window) = viewport.window.as_ref() {
            let is_viewport_focused = focused_viewport == Some(viewport_id);
            egui_winit::process_viewport_commands(
                egui_ctx,
                &mut viewport.info,
                commands,
                window,
                is_viewport_focused,
                &mut viewport.screenshot_requested,
            );
        }
    }
}

fn initialize_or_update_viewport<'vp>(
    egui_ctx: &egui::Context,
    viewports: &'vp mut Viewports,
    ids: ViewportIdPair,
    class: ViewportClass,
    mut builder: ViewportBuilder,
    viewport_ui_cb: Option<Arc<dyn Fn(&egui::Context) + Send + Sync>>,
    focused_viewport: Option<ViewportId>,
) -> &'vp mut Viewport {
    crate::profile_function!();

    if builder.icon.is_none() {
        // Inherit icon from parent
        builder.icon = viewports
            .get_mut(&ids.parent)
            .and_then(|vp| vp.builder.icon.clone());
    }

    match viewports.entry(ids.this) {
        std::collections::hash_map::Entry::Vacant(entry) => {
            // New viewport:
            log::debug!("Creating new viewport {:?} ({:?})", ids.this, builder.title);
            entry.insert(Viewport {
                ids,
                class,
                builder,
                info: Default::default(),
                screenshot_requested: false,
                viewport_ui_cb,
                window: None,
                egui_winit: None,
            })
        }

        std::collections::hash_map::Entry::Occupied(mut entry) => {
            // Patch an existing viewport:
            let viewport = entry.get_mut();

            viewport.class = class;
            viewport.ids.parent = ids.parent;
            viewport.viewport_ui_cb = viewport_ui_cb;

            let (delta_commands, recreate) = viewport.builder.patch(builder);

            if recreate {
                log::debug!(
                    "Recreating window for viewport {:?} ({:?})",
                    ids.this,
                    viewport.builder.title
                );
                viewport.window = None;
                viewport.egui_winit = None;
            } else if let Some(window) = &viewport.window {
                let is_viewport_focused = focused_viewport == Some(ids.this);
                egui_winit::process_viewport_commands(
                    egui_ctx,
                    &mut viewport.info,
                    delta_commands,
                    window,
                    is_viewport_focused,
                    &mut viewport.screenshot_requested,
                );
            }

            entry.into_mut()
        }
    }
}
