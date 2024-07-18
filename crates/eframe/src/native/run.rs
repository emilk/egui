use std::{cell::RefCell, time::Instant};

use winit::{
    application::ApplicationHandler,
    event::Event,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::WindowId,
};

use ahash::HashMap;

use crate::{
    epi,
    native::winit_integration::{short_event_description, EventResult},
    Result,
};

use super::winit_integration::{UserEvent, WinitApp};

// ----------------------------------------------------------------------------
fn create_event_loop(native_options: &mut epi::NativeOptions) -> Result<EventLoop<UserEvent>> {
    crate::profile_function!();
    let mut builder = winit::event_loop::EventLoop::with_user_event();

    if let Some(hook) = std::mem::take(&mut native_options.event_loop_builder) {
        hook(&mut builder);
    }

    crate::profile_scope!("EventLoopBuilder::build");
    Ok(builder.build()?)
}

/// Access a thread-local event loop.
///
/// We reuse the event-loop so we can support closing and opening an eframe window
/// multiple times. This is just a limitation of winit.
fn with_event_loop<R>(
    mut native_options: epi::NativeOptions,
    f: impl FnOnce(&mut EventLoop<UserEvent>, epi::NativeOptions) -> R,
) -> Result<R> {
    thread_local!(static EVENT_LOOP: RefCell<Option<EventLoop<UserEvent>>> = RefCell::new(None));

    EVENT_LOOP.with(|event_loop| {
        // Since we want to reference NativeOptions when creating the EventLoop we can't
        // do that as part of the lazy thread local storage initialization and so we instead
        // create the event loop lazily here
        let mut event_loop_lock = event_loop.borrow_mut();
        let event_loop = if let Some(event_loop) = &mut *event_loop_lock {
            event_loop
        } else {
            event_loop_lock.insert(create_event_loop(&mut native_options)?)
        };
        Ok(f(event_loop, native_options))
    })
}

struct App<T: WinitApp> {
    windows_next_repaint_times: HashMap<WindowId, Instant>,
    winit_app: T,
    return_result: Result<(), crate::Error>,
}

impl<T: WinitApp> App<T> {
    fn new(winit_app: T) -> Self {
        Self {
            windows_next_repaint_times: HashMap::default(),
            winit_app,
            return_result: Ok(()),
        }
    }

    fn handle_event(&mut self, event_loop: &ActiveEventLoop, event: &Event<UserEvent>) {
        let event_result = match self.winit_app.on_event(event_loop, event) {
            Ok(event_result) => {
                log::trace!("event_result: {event_result:?}");
                event_result
            }
            Err(err) => {
                log::error!("Exiting because of error: {err} during event {event:?}");
                self.return_result = Err(err);
                EventResult::Exit
            }
        };
        self.handle_event_result(event_loop, event_result, event);
    }

    fn handle_event_result(
        &mut self,
        event_loop: &ActiveEventLoop,
        event_result: EventResult,
        event: &Event<UserEvent>,
    ) {
        match event_result {
            EventResult::Wait => {
                event_loop.set_control_flow(ControlFlow::Wait);
            }
            EventResult::RepaintNow(window_id) => {
                log::trace!(
                    "RepaintNow of {window_id:?} caused by {}",
                    short_event_description(event)
                );
                if cfg!(target_os = "windows") {
                    // Fix flickering on Windows, see https://github.com/emilk/egui/pull/2280
                    self.windows_next_repaint_times.remove(&window_id);
                    self.winit_app.run_ui_and_paint(event_loop, window_id);
                } else {
                    // Fix for https://github.com/emilk/egui/issues/2425
                    self.windows_next_repaint_times
                        .insert(window_id, Instant::now());
                }
            }
            EventResult::RepaintNext(window_id) => {
                log::trace!(
                    "RepaintNext of {window_id:?} caused by {}",
                    short_event_description(event)
                );
                self.windows_next_repaint_times
                    .insert(window_id, Instant::now());
            }
            EventResult::Exit => {
                log::debug!("Asking to exit event loop…");
                self.winit_app.save_and_destroy();
                event_loop.exit();
            }
        }
    }
}

impl<T: WinitApp> ApplicationHandler<UserEvent> for App<T> {
    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        self.handle_event(event_loop, &Event::Suspended);
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.handle_event(event_loop, &Event::Resumed);
    }

    fn exiting(&mut self, _: &ActiveEventLoop) {
        // On Mac, Cmd-Q we get here and then `run_on_demand` doesn't return (despite its name),
        // so we need to save state now:
        log::debug!("Received Event::LoopExiting - saving app state…");
        self.winit_app.save_and_destroy();
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        #[allow(irrefutable_let_patterns)]
        // when accesskit is not enabled, there are no other events to handle here.
        if let UserEvent::RequestRepaint {
            when,
            frame_nr,
            viewport_id,
        } = event
        {
            let current_frame_nr = self.winit_app.frame_nr(viewport_id);
            if current_frame_nr == frame_nr || current_frame_nr == frame_nr + 1 {
                log::trace!("UserEvent::RequestRepaint scheduling repaint at {when:?}");
                if let Some(window_id) = self.winit_app.window_id_from_viewport_id(viewport_id) {
                    self.windows_next_repaint_times.insert(
                        window_id,
                        self.windows_next_repaint_times
                            .get(&window_id)
                            .map_or(when, |last| (*last).min(when)),
                    );
                } else {
                    event_loop.set_control_flow(ControlFlow::Wait);
                }
            } else {
                log::trace!("Got outdated UserEvent::RequestRepaint");
                event_loop.set_control_flow(ControlFlow::Wait);
            }
        } else {
            self.handle_event(event_loop, &Event::UserEvent(event));
        }
    }

    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: winit::event::StartCause) {
        if matches!(cause, winit::event::StartCause::ResumeTimeReached { .. }) {
            log::trace!("Woke up to check next_repaint_time");
        }
        // TODO: Is this needed?
        event_loop.set_control_flow(ControlFlow::Wait);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: winit::event::WindowEvent,
    ) {
        if matches!(event, winit::event::WindowEvent::RedrawRequested) {
            self.windows_next_repaint_times.remove(&window_id);
            let event_result = self.winit_app.run_ui_and_paint(event_loop, window_id);

            self.handle_event_result(
                event_loop,
                event_result,
                &Event::WindowEvent { window_id, event },
            );
        } else {
            self.handle_event(
                event_loop,
                &Event::WindowEvent {
                    window_id,
                    event: event.clone(),
                },
            );
        }

        let mut next_repaint_time = self.windows_next_repaint_times.values().min().copied();

        self.windows_next_repaint_times
            .retain(|window_id, repaint_time| {
                if Instant::now() < *repaint_time {
                    return true; // not yet ready
                };

                next_repaint_time = None;
                event_loop.set_control_flow(ControlFlow::Poll);

                if let Some(window) = self.winit_app.window(*window_id) {
                    log::trace!("request_redraw for {window_id:?}");
                    let is_minimized = window.is_minimized().unwrap_or(false);
                    if is_minimized {
                        false
                    } else {
                        window.request_redraw();
                        true
                    }
                } else {
                    log::trace!("No window found for {window_id:?}");
                    false
                }
            });

        if let Some(next_repaint_time) = next_repaint_time {
            // WaitUntil seems to not work on iOS
            #[cfg(target_os = "ios")]
            winit_app
                .get_window_winit_id(ViewportId::ROOT)
                .map(|window_id| {
                    winit_app
                        .window(window_id)
                        .map(|window| window.request_redraw())
                });

            event_loop.set_control_flow(ControlFlow::WaitUntil(next_repaint_time));
        };
    }
}

#[cfg(not(target_os = "ios"))]
fn run_and_return(event_loop: &mut EventLoop<UserEvent>, winit_app: impl WinitApp) -> Result {
    use winit::platform::run_on_demand::EventLoopExtRunOnDemand;

    log::trace!("Entering the winit event loop (run_on_demand)…");

    let mut app = App::new(winit_app);
    event_loop.run_app_on_demand(&mut app)?;
    log::debug!("eframe window closed");
    app.return_result
}

fn run_and_exit(event_loop: EventLoop<UserEvent>, winit_app: impl WinitApp + 'static) -> Result {
    log::trace!("Entering the winit event loop (run)…");

    // When to repaint what window
    let mut app = App::new(winit_app);
    event_loop.run_app(&mut app)?;

    log::debug!("winit event loop unexpectedly returned");
    Ok(())
}

// ----------------------------------------------------------------------------

#[cfg(feature = "glow")]
pub fn run_glow(
    app_name: &str,
    mut native_options: epi::NativeOptions,
    app_creator: epi::AppCreator,
) -> Result {
    #![allow(clippy::needless_return_with_question_mark)] // False positive

    use super::glow_integration::GlowWinitApp;

    #[cfg(not(target_os = "ios"))]
    if native_options.run_and_return {
        return with_event_loop(native_options, |event_loop, native_options| {
            let glow_eframe = GlowWinitApp::new(event_loop, app_name, native_options, app_creator);
            run_and_return(event_loop, glow_eframe)
        })?;
    }

    let event_loop = create_event_loop(&mut native_options)?;
    let glow_eframe = GlowWinitApp::new(&event_loop, app_name, native_options, app_creator);
    run_and_exit(event_loop, glow_eframe)
}

// ----------------------------------------------------------------------------

#[cfg(feature = "wgpu")]
pub fn run_wgpu(
    app_name: &str,
    mut native_options: epi::NativeOptions,
    app_creator: epi::AppCreator,
) -> Result {
    #![allow(clippy::needless_return_with_question_mark)] // False positive

    use super::wgpu_integration::WgpuWinitApp;

    #[cfg(not(target_os = "ios"))]
    if native_options.run_and_return {
        return with_event_loop(native_options, |event_loop, native_options| {
            let wgpu_eframe = WgpuWinitApp::new(event_loop, app_name, native_options, app_creator);
            run_and_return(event_loop, wgpu_eframe)
        })?;
    }

    let event_loop = create_event_loop(&mut native_options)?;
    let wgpu_eframe = WgpuWinitApp::new(&event_loop, app_name, native_options, app_creator);
    run_and_exit(event_loop, wgpu_eframe)
}
