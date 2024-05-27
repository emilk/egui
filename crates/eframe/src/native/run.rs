use std::{cell::RefCell, time::Instant};

use winit::event_loop::{EventLoop, EventLoopBuilder};

use egui::epaint::ahash::HashMap;

use crate::{
    epi,
    native::winit_integration::{short_event_description, EventResult},
    Result,
};

use super::winit_integration::{UserEvent, WinitApp};

// ----------------------------------------------------------------------------

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

fn create_event_loop(native_options: &mut epi::NativeOptions) -> Result<EventLoop<UserEvent>> {
    crate::profile_function!();
    let mut builder = create_event_loop_builder(native_options);

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

#[cfg(not(target_os = "ios"))]
fn run_and_return(
    event_loop: &mut EventLoop<UserEvent>,
    mut winit_app: impl WinitApp,
) -> Result<()> {
    use winit::{event_loop::ControlFlow, platform::run_on_demand::EventLoopExtRunOnDemand};

    log::trace!("Entering the winit event loop (run_on_demand)…");

    // When to repaint what window
    let mut windows_next_repaint_times = HashMap::default();

    let mut returned_result = Ok(());

    event_loop.run_on_demand(|event, event_loop_window_target| {
        crate::profile_scope!("winit_event", short_event_description(&event));

        log::trace!("winit event: {event:?}");

        if matches!(event, winit::event::Event::AboutToWait) {
            return; // early-out: don't trigger another wait
        }

        let event_result = match &event {
            winit::event::Event::LoopExiting => {
                // First, `WindowEvent::CloseRequested` occurs which leads to `Result::Exit`
                // and when `event_loop_window_target.exit()` is executed, `Event::LoopExiting` event is given.

                // On Mac, Cmd-Q we get here and then `run_on_demand` doesn't return (despite its name),
                // so we need to save state now:
                log::debug!("Received Event::LoopExiting - saving app state…");
                winit_app.save_and_destroy();

                return;
            }

            winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::RedrawRequested,
                window_id,
            } => {
                // "glow": glow_integration, "wgpu": wgpu_integration
                winit_app.run_ui_and_paint(event_loop_window_target, *window_id)
            }

            winit::event::Event::UserEvent(UserEvent::RequestRepaint {
                when,
                frame_nr,
                viewport_id,
            }) => {
                log::trace!("UserEvent::RequestRepaint scheduling repaint at {when:?}");

                let current_frame_nr = winit_app.frame_nr(*viewport_id);
                if current_frame_nr < *frame_nr {
                    log::trace!("Got outdated UserEvent::RequestRepaint");
                }

                if let Some(window_id) = winit_app.window_id_from_viewport_id(*viewport_id) {
                    EventResult::RepaintAt(window_id, *when)
                } else {
                    EventResult::Wait
                }
            }

            winit::event::Event::NewEvents(winit::event::StartCause::ResumeTimeReached {
                start: _,
                requested_resume,
            }) => {
                log::trace!("Woke up to check next_repaint_time");
                if let Some(window_id) = winit_app.window_id_from_viewport_id(egui::ViewportId::ROOT) {
                    EventResult::RepaintAt(window_id, *requested_resume)
                } else {
                    EventResult::Wait
                }
            }

            event => match winit_app.on_event(event_loop_window_target, event) {
                Ok(event_result) => {
                    log::trace!("event_result: {event_result:?}");
                    event_result
                }
                Err(err) => {
                    log::error!("Exiting because of error: {err} during event {event:?}");
                    returned_result = Err(err);
                    event_loop_window_target.exit();
                    return;
                }
            },
        };

        let now = Instant::now();

        match event_result {
            EventResult::Wait => {
                event_loop_window_target.set_control_flow(ControlFlow::Wait);
            }
            EventResult::RepaintNow(window_id) => {
                log::trace!(
                    "RepaintNow of {window_id:?} caused by {}",
                    short_event_description(&event)
                );

                windows_next_repaint_times.insert(window_id, now);
            }
            EventResult::RepaintNext(window_id) => {
                log::trace!(
                    "RepaintNext of {window_id:?} caused by {}",
                    short_event_description(&event)
                );

                winit_app.run_ui_and_paint(event_loop_window_target, window_id);
                windows_next_repaint_times.insert(window_id, now);
                windows_next_repaint_times
                    .insert(window_id, now + std::time::Duration::from_millis(1));
            }
            EventResult::RepaintAt(window_id, repaint_time) => {
                windows_next_repaint_times.insert(window_id, repaint_time);
            }
            EventResult::ViewportExit(window_id) => {
                if let Some(window) = winit_app.window(window_id) {
                    window.set_minimized(true);
                    window.request_redraw();
                }
            }
            EventResult::Exit(window_id) => {
                event_loop_window_target.exit();

                if let Some(window) = winit_app.window(window_id) {
                    window.set_minimized(true);
                    window.request_redraw();
                }
            }
        }

        windows_next_repaint_times.retain(|window_id, repaint_time| {
            if now < *repaint_time {
                return true; // not yet ready
            };

            event_loop_window_target.set_control_flow(ControlFlow::Poll);

            if let Some(window) = winit_app.window(*window_id) {
                log::trace!("request_redraw for {window_id:?}");
                let is_minimized = window.is_minimized().unwrap_or(false);
                if is_minimized {
                    // Don't draw : Issues #3321 && This also affects CPU usage in a minimized state.
                    // See: https://github.com/rust-windowing/winit/issues/208
                    // See: https://github.com/emilk/egui/issues/3321
                } else {
                    window.request_redraw();
                }
            } else {
                log::trace!("No window found for {window_id:?}");
            }

            false
        });

        let next_repaint_time = windows_next_repaint_times.values().min().copied();

        if let Some(next_repaint_time) = next_repaint_time {
            event_loop_window_target.set_control_flow(ControlFlow::WaitUntil(next_repaint_time));
        }
    })?;

    log::debug!("eframe window closed");

    drop(winit_app);

    returned_result
}

fn run_and_exit(
    event_loop: EventLoop<UserEvent>,
    mut winit_app: impl WinitApp + 'static,
) -> Result<()> {
    use winit::event_loop::ControlFlow;
    log::trace!("Entering the winit event loop (run)…");

    // When to repaint what window
    let mut windows_next_repaint_times = HashMap::default();

    event_loop.run(move |event, event_loop_window_target| {
        crate::profile_scope!("winit_event", short_event_description(&event));

        log::trace!("winit event: {event:?}");

        if matches!(event, winit::event::Event::AboutToWait) {
            return; // early-out: don't trigger another wait
        }

        let event_result = match &event {
            winit::event::Event::LoopExiting => {
                // First, `WindowEvent::CloseRequested` occurs which leads to `Result::Exit`
                // and when `event_loop_window_target.exit()` is executed, `Event::LoopExiting` event is given.

                log::debug!("Received Event::LoopExiting");
                winit_app.save_and_destroy();

                log::debug!("Exiting with return code 0");
                #[allow(clippy::exit)]
                std::process::exit(0);
            }

            winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::RedrawRequested,
                window_id,
            } => {
                // "glow": glow_integration, "wgpu": wgpu_integration
                winit_app.run_ui_and_paint(event_loop_window_target, *window_id)
            }

            winit::event::Event::UserEvent(UserEvent::RequestRepaint {
                when,
                frame_nr,
                viewport_id,
            }) => {
                let current_frame_nr = winit_app.frame_nr(*viewport_id);
                if current_frame_nr < *frame_nr {
                    log::trace!("Got outdated UserEvent::RequestRepaint");
                }

                if let Some(window_id) = winit_app.window_id_from_viewport_id(*viewport_id) {
                    EventResult::RepaintAt(window_id, *when)
                } else {
                    EventResult::Wait
                }
            }

            winit::event::Event::NewEvents(winit::event::StartCause::ResumeTimeReached {
                ..
            }) => {
                log::trace!("Woke up to check next_repaint_time");
                EventResult::Wait
            }

            event => match winit_app.on_event(event_loop_window_target, event) {
                Ok(event_result) => {
                    log::trace!("event_result: {event_result:?}");
                    event_result
                }
                Err(err) => {
                    panic!("eframe encountered a fatal error: {err} during event {event:?}");
                }
            },
        };

        let now = Instant::now();

        match event_result {
            EventResult::Wait => {
                event_loop_window_target.set_control_flow(ControlFlow::Wait);
            }
            EventResult::RepaintNow(window_id) => {
                log::trace!("RepaintNow caused by {}", short_event_description(&event));

                windows_next_repaint_times.insert(window_id, now);
            }
            EventResult::RepaintNext(window_id) => {
                log::trace!("RepaintNext caused by {}", short_event_description(&event));

                winit_app.run_ui_and_paint(event_loop_window_target, window_id);
                windows_next_repaint_times.insert(window_id, now);
                windows_next_repaint_times
                    .insert(window_id, now + std::time::Duration::from_millis(1));
            }
            EventResult::RepaintAt(window_id, repaint_time) => {
                windows_next_repaint_times.insert(window_id, repaint_time);
            }
            EventResult::ViewportExit(window_id) => {
                if let Some(window) = winit_app.window(window_id) {
                    window.set_minimized(true);
                    window.request_redraw();
                }
            }
            EventResult::Exit(window_id) => {
                event_loop_window_target.exit();

                if let Some(window) = winit_app.window(window_id) {
                    window.set_minimized(true);
                    window.request_redraw();
                }
            }
        }

        windows_next_repaint_times.retain(|window_id, repaint_time| {
            if now < *repaint_time {
                return true; // not yet ready
            }

            event_loop_window_target.set_control_flow(ControlFlow::Poll);

            if let Some(window) = winit_app.window(*window_id) {
                log::trace!("request_redraw for {window_id:?}");
                let is_minimized = window.is_minimized().unwrap_or(false);
                if is_minimized {
                    // Don't draw : Issues #3321 && This also affects CPU usage in a minimized state.
                    // See: https://github.com/rust-windowing/winit/issues/208
                    // See: https://github.com/emilk/egui/issues/3321
                } else {
                    window.request_redraw();
                }
            } else {
                log::trace!("No window found for {window_id:?}");
            }

            false
        });

        let next_repaint_time = windows_next_repaint_times.values().min().copied();

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

            event_loop_window_target.set_control_flow(ControlFlow::WaitUntil(next_repaint_time));
        };
    })?;

    log::debug!("winit event loop unexpectedly returned");

    Ok(())
}

// ----------------------------------------------------------------------------

#[cfg(feature = "glow")]
pub fn run_glow(
    app_name: &str,
    mut native_options: epi::NativeOptions,
    app_creator: epi::AppCreator,
) -> Result<()> {
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
) -> Result<()> {
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
