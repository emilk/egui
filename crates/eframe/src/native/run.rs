//! Note that this file contains two similar paths - one for [`glow`], one for [`wgpu`].
//! When making changes to one you often also want to apply it to the other.
//!
//! This is also very complex code, and not very pretty.
//! There is a bunch of improvements we could do,
//! like removing a bunch of `unwraps`.

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
    f: impl FnOnce(&mut EventLoop<UserEvent>, epi::NativeOptions) -> R,
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
    use winit::{event_loop::ControlFlow, platform::run_return::EventLoopExtRunReturn as _};

    log::debug!("Entering the winit event loop (run_return)…");

    // When to repaint what window
    let mut windows_next_repaint_times = HashMap::default();

    let mut returned_result = Ok(());

    event_loop.run_return(|event, event_loop, control_flow| {
        crate::profile_scope!("winit_event", short_event_description(&event));

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

            event => match winit_app.on_event(event_loop, event) {
                Ok(event_result) => event_result,
                Err(err) => {
                    log::error!("Exiting because of error: {err} during event {event:?}");
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
                log::trace!("Repaint caused by {}", short_event_description(&event));
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
                log::debug!("Asking to exit event loop…");
                winit_app.save_and_destroy();
                *control_flow = ControlFlow::Exit;
                return;
            }
        }

        let mut next_repaint_time = windows_next_repaint_times.values().min().copied();

        // This is for not duplicating redraw requests
        use winit::event::Event;
        if matches!(
            event,
            Event::RedrawEventsCleared | Event::RedrawRequested(_) | Event::Resumed
        ) {
            windows_next_repaint_times.retain(|window_id, repaint_time| {
                if Instant::now() < *repaint_time {
                    return true;
                };

                next_repaint_time = None;
                control_flow.set_poll();

                if let Some(window) = winit_app.window(*window_id) {
                    log::trace!("request_redraw for {window_id:?}");
                    window.request_redraw();
                    true
                } else {
                    false
                }
            });
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

    // When to repaint what window
    let mut windows_next_repaint_times = HashMap::default();

    event_loop.run(move |event, event_loop, control_flow| {
        crate::profile_scope!("winit_event", short_event_description(&event));

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

            event => match winit_app.on_event(event_loop, event) {
                Ok(event_result) => event_result,
                Err(err) => {
                    panic!("eframe encountered a fatal error: {err} during event {event:?}");
                }
            },
        };

        match event_result {
            EventResult::Wait => {
                control_flow.set_wait();
            }
            EventResult::RepaintNow(window_id) => {
                log::trace!("Repaint caused by {}", short_event_description(&event));
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
                log::trace!("Repaint caused by {}", short_event_description(&event));
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

        let mut next_repaint_time = windows_next_repaint_times.values().min().copied();

        // This is for not duplicating redraw requests
        use winit::event::Event;
        if matches!(
            event,
            Event::RedrawEventsCleared | Event::RedrawRequested(_) | Event::Resumed
        ) {
            windows_next_repaint_times.retain(|window_id, repaint_time| {
                if Instant::now() < *repaint_time {
                    return true;
                }

                next_repaint_time = None;
                control_flow.set_poll();

                if let Some(window) = winit_app.window(*window_id) {
                    log::trace!("request_redraw for {window_id:?}");
                    window.request_redraw();
                    true
                } else {
                    false
                }
            });
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
                        .map(|window| window.request_redraw())
                });

            control_flow.set_wait_until(next_repaint_time);
        };
    })
}

// ----------------------------------------------------------------------------

#[cfg(feature = "glow")]
pub fn run_glow(
    app_name: &str,
    mut native_options: epi::NativeOptions,
    app_creator: epi::AppCreator,
) -> Result<()> {
    use super::glow_integration::GlowWinitApp;

    #[cfg(not(target_os = "ios"))]
    if native_options.run_and_return {
        return with_event_loop(native_options, |event_loop, native_options| {
            let glow_eframe = GlowWinitApp::new(event_loop, app_name, native_options, app_creator);
            run_and_return(event_loop, glow_eframe)
        });
    }

    let event_loop = create_event_loop(&mut native_options);
    let glow_eframe = GlowWinitApp::new(&event_loop, app_name, native_options, app_creator);
    run_and_exit(event_loop, glow_eframe);
}

// ----------------------------------------------------------------------------

#[cfg(feature = "wgpu")]
pub fn run_wgpu(
    app_name: &str,
    mut native_options: epi::NativeOptions,
    app_creator: epi::AppCreator,
) -> Result<()> {
    use super::wgpu_integration::WgpuWinitApp;

    #[cfg(not(target_os = "ios"))]
    if native_options.run_and_return {
        return with_event_loop(native_options, |event_loop, native_options| {
            let wgpu_eframe = WgpuWinitApp::new(event_loop, app_name, native_options, app_creator);
            run_and_return(event_loop, wgpu_eframe)
        });
    }

    let event_loop = create_event_loop(&mut native_options);
    let wgpu_eframe = WgpuWinitApp::new(&event_loop, app_name, native_options, app_creator);
    run_and_exit(event_loop, wgpu_eframe);
}
