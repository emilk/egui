use std::sync::atomic::{AtomicBool, Ordering};

use egui::Key;

use super::*;

struct IsDestroyed(pub bool);

pub fn paint_and_schedule(
    runner_ref: &AppRunnerRef,
    panicked: Arc<AtomicBool>,
) -> Result<(), JsValue> {
    fn paint_if_needed(runner_ref: &AppRunnerRef) -> Result<IsDestroyed, JsValue> {
        let mut runner_lock = runner_ref.lock();
        let is_destroyed = runner_lock.is_destroyed.fetch();

        if !is_destroyed && runner_lock.needs_repaint.when_to_repaint() <= now_sec() {
            runner_lock.needs_repaint.clear();
            let (repaint_after, clipped_primitives) = runner_lock.logic()?;
            runner_lock.paint(&clipped_primitives)?;
            runner_lock
                .needs_repaint
                .repaint_after(repaint_after.as_secs_f64());
            runner_lock.auto_save();
        }

        Ok(IsDestroyed(is_destroyed))
    }

    fn request_animation_frame(
        runner_ref: AppRunnerRef,
        panicked: Arc<AtomicBool>,
    ) -> Result<(), JsValue> {
        let window = web_sys::window().unwrap();
        let closure = Closure::once(move || paint_and_schedule(&runner_ref, panicked));
        window.request_animation_frame(closure.as_ref().unchecked_ref())?;
        closure.forget(); // We must forget it, or else the callback is canceled on drop
        Ok(())
    }

    // Only paint and schedule if there has been no panic
    if !panicked.load(Ordering::SeqCst) {
        let is_destroyed = paint_if_needed(runner_ref)?;
        if !is_destroyed.0 {
            request_animation_frame(runner_ref.clone(), panicked)?;
        }
    }

    Ok(())
}

pub fn install_document_events(runner_container: &mut AppRunnerContainer) -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();

    runner_container.add_event_listener(
        &document,
        "keydown",
        |event: web_sys::KeyboardEvent, mut runner_lock| {
            if event.is_composing() || event.key_code() == 229 {
                // https://web.archive.org/web/20200526195704/https://www.fxsitecompat.dev/en-CA/docs/2018/keydown-and-keyup-events-are-now-fired-during-ime-composition/
                return;
            }

            let modifiers = modifiers_from_event(&event);
            runner_lock.input.raw.modifiers = modifiers;

            let key = event.key();
            let egui_key = translate_key(&key);

            if let Some(key) = egui_key {
                runner_lock.input.raw.events.push(egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false, // egui will fill this in for us!
                    modifiers,
                });
            }
            if !modifiers.ctrl
                && !modifiers.command
                && !should_ignore_key(&key)
                // When text agent is shown, it sends text event instead.
                && text_agent::text_agent().hidden()
            {
                runner_lock.input.raw.events.push(egui::Event::Text(key));
            }
            runner_lock.needs_repaint.repaint_asap();

            let egui_wants_keyboard = runner_lock.egui_ctx().wants_keyboard_input();

            #[allow(clippy::if_same_then_else)]
            let prevent_default = if egui_key == Some(Key::Tab) {
                // Always prevent moving cursor to url bar.
                // egui wants to use tab to move to the next text field.
                true
            } else if egui_key == Some(Key::P) {
                #[allow(clippy::needless_bool)]
                if modifiers.ctrl || modifiers.command || modifiers.mac_cmd {
                    true // Prevent ctrl-P opening the print dialog. Users may want to use it for a command palette.
                } else {
                    false // let normal P:s through
                }
            } else if egui_wants_keyboard {
                matches!(
                    event.key().as_str(),
                    "Backspace" // so we don't go back to previous page when deleting text
                    | "ArrowDown" | "ArrowLeft" | "ArrowRight" | "ArrowUp" // cmd-left is "back" on Mac (https://github.com/emilk/egui/issues/58)
                )
            } else {
                // We never want to prevent:
                // * F5 / cmd-R (refresh)
                // * cmd-shift-C (debug tools)
                // * cmd/ctrl-c/v/x (or we stop copy/past/cut events)
                false
            };

            // tracing::debug!(
            //     "On key-down {:?}, egui_wants_keyboard: {}, prevent_default: {}",
            //     event.key().as_str(),
            //     egui_wants_keyboard,
            //     prevent_default
            // );

            if prevent_default {
                event.prevent_default();
                // event.stop_propagation();
            }
        },
    )?;

    runner_container.add_event_listener(
        &document,
        "keyup",
        |event: web_sys::KeyboardEvent, mut runner_lock| {
            let modifiers = modifiers_from_event(&event);
            runner_lock.input.raw.modifiers = modifiers;
            if let Some(key) = translate_key(&event.key()) {
                runner_lock.input.raw.events.push(egui::Event::Key {
                    key,
                    pressed: false,
                    repeat: false,
                    modifiers,
                });
            }
            runner_lock.needs_repaint.repaint_asap();
        },
    )?;

    #[cfg(web_sys_unstable_apis)]
    runner_container.add_event_listener(
        &document,
        "paste",
        |event: web_sys::ClipboardEvent, mut runner_lock| {
            if let Some(data) = event.clipboard_data() {
                if let Ok(text) = data.get_data("text") {
                    let text = text.replace("\r\n", "\n");
                    if !text.is_empty() {
                        runner_lock.input.raw.events.push(egui::Event::Paste(text));
                        runner_lock.needs_repaint.repaint_asap();
                    }
                    event.stop_propagation();
                    event.prevent_default();
                }
            }
        },
    )?;

    #[cfg(web_sys_unstable_apis)]
    runner_container.add_event_listener(
        &document,
        "cut",
        |_: web_sys::ClipboardEvent, mut runner_lock| {
            runner_lock.input.raw.events.push(egui::Event::Cut);
            runner_lock.needs_repaint.repaint_asap();
        },
    )?;

    #[cfg(web_sys_unstable_apis)]
    runner_container.add_event_listener(
        &document,
        "copy",
        |_: web_sys::ClipboardEvent, mut runner_lock| {
            runner_lock.input.raw.events.push(egui::Event::Copy);
            runner_lock.needs_repaint.repaint_asap();
        },
    )?;

    for event_name in &["load", "pagehide", "pageshow", "resize"] {
        runner_container.add_event_listener(
            &window,
            event_name,
            |_: web_sys::Event, runner_lock| {
                runner_lock.needs_repaint.repaint_asap();
            },
        )?;
    }

    runner_container.add_event_listener(
        &window,
        "hashchange",
        |_: web_sys::Event, mut runner_lock| {
            // `epi::Frame::info(&self)` clones `epi::IntegrationInfo`, but we need to modify the original here
            runner_lock.frame.info.web_info.location.hash = location_hash();
        },
    )?;

    Ok(())
}

pub fn install_canvas_events(runner_container: &mut AppRunnerContainer) -> Result<(), JsValue> {
    let canvas = canvas_element(runner_container.runner.lock().canvas_id()).unwrap();

    let prevent_default_events = [
        // By default, right-clicks open a context menu.
        // We don't want to do that (right clicks is handled by egui):
        "contextmenu",
        // Allow users to use ctrl-p for e.g. a command palette
        "afterprint",
    ];

    for event_name in prevent_default_events {
        let closure =
            move |event: web_sys::MouseEvent,
                  mut _runner_lock: egui::mutex::MutexGuard<'_, AppRunner>| {
                event.prevent_default();
                // event.stop_propagation();
                // tracing::debug!("Preventing event {:?}", event_name);
            };

        runner_container.add_event_listener(&canvas, event_name, closure)?;
    }

    runner_container.add_event_listener(
        &canvas,
        "mousedown",
        |event: web_sys::MouseEvent, mut runner_lock: egui::mutex::MutexGuard<'_, AppRunner>| {
            if let Some(button) = button_from_mouse_event(&event) {
                let pos = pos_from_mouse_event(runner_lock.canvas_id(), &event);
                let modifiers = runner_lock.input.raw.modifiers;
                runner_lock
                    .input
                    .raw
                    .events
                    .push(egui::Event::PointerButton {
                        pos,
                        button,
                        pressed: true,
                        modifiers,
                    });
                runner_lock.needs_repaint.repaint_asap();
            }
            event.stop_propagation();
            // Note: prevent_default breaks VSCode tab focusing, hence why we don't call it here.
        },
    )?;

    runner_container.add_event_listener(
        &canvas,
        "mousemove",
        |event: web_sys::MouseEvent, mut runner_lock| {
            let pos = pos_from_mouse_event(runner_lock.canvas_id(), &event);
            runner_lock
                .input
                .raw
                .events
                .push(egui::Event::PointerMoved(pos));
            runner_lock.needs_repaint.repaint_asap();
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    runner_container.add_event_listener(
        &canvas,
        "mouseup",
        |event: web_sys::MouseEvent, mut runner_lock| {
            if let Some(button) = button_from_mouse_event(&event) {
                let pos = pos_from_mouse_event(runner_lock.canvas_id(), &event);
                let modifiers = runner_lock.input.raw.modifiers;
                runner_lock
                    .input
                    .raw
                    .events
                    .push(egui::Event::PointerButton {
                        pos,
                        button,
                        pressed: false,
                        modifiers,
                    });
                runner_lock.needs_repaint.repaint_asap();

                text_agent::update_text_agent(runner_lock);
            }
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    runner_container.add_event_listener(
        &canvas,
        "mouseleave",
        |event: web_sys::MouseEvent, mut runner_lock| {
            runner_lock.input.raw.events.push(egui::Event::PointerGone);
            runner_lock.needs_repaint.repaint_asap();
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    runner_container.add_event_listener(
        &canvas,
        "touchstart",
        |event: web_sys::TouchEvent, mut runner_lock| {
            let mut latest_touch_pos_id = runner_lock.input.latest_touch_pos_id;
            let pos =
                pos_from_touch_event(runner_lock.canvas_id(), &event, &mut latest_touch_pos_id);
            runner_lock.input.latest_touch_pos_id = latest_touch_pos_id;
            runner_lock.input.latest_touch_pos = Some(pos);
            let modifiers = runner_lock.input.raw.modifiers;
            runner_lock
                .input
                .raw
                .events
                .push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers,
                });

            push_touches(&mut runner_lock, egui::TouchPhase::Start, &event);
            runner_lock.needs_repaint.repaint_asap();
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    runner_container.add_event_listener(
        &canvas,
        "touchmove",
        |event: web_sys::TouchEvent, mut runner_lock| {
            let mut latest_touch_pos_id = runner_lock.input.latest_touch_pos_id;
            let pos =
                pos_from_touch_event(runner_lock.canvas_id(), &event, &mut latest_touch_pos_id);
            runner_lock.input.latest_touch_pos_id = latest_touch_pos_id;
            runner_lock.input.latest_touch_pos = Some(pos);
            runner_lock
                .input
                .raw
                .events
                .push(egui::Event::PointerMoved(pos));

            push_touches(&mut runner_lock, egui::TouchPhase::Move, &event);
            runner_lock.needs_repaint.repaint_asap();
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    runner_container.add_event_listener(
        &canvas,
        "touchend",
        |event: web_sys::TouchEvent, mut runner_lock| {
            if let Some(pos) = runner_lock.input.latest_touch_pos {
                let modifiers = runner_lock.input.raw.modifiers;
                // First release mouse to click:
                runner_lock
                    .input
                    .raw
                    .events
                    .push(egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers,
                    });
                // Then remove hover effect:
                runner_lock.input.raw.events.push(egui::Event::PointerGone);

                push_touches(&mut runner_lock, egui::TouchPhase::End, &event);
                runner_lock.needs_repaint.repaint_asap();
                event.stop_propagation();
                event.prevent_default();
            }

            // Finally, focus or blur text agent to toggle mobile keyboard:
            text_agent::update_text_agent(runner_lock);
        },
    )?;

    runner_container.add_event_listener(
        &canvas,
        "touchcancel",
        |event: web_sys::TouchEvent, mut runner_lock| {
            push_touches(&mut runner_lock, egui::TouchPhase::Cancel, &event);
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    runner_container.add_event_listener(
        &canvas,
        "wheel",
        |event: web_sys::WheelEvent, mut runner_lock| {
            let scroll_multiplier = match event.delta_mode() {
                web_sys::WheelEvent::DOM_DELTA_PAGE => {
                    canvas_size_in_points(runner_lock.canvas_id()).y
                }
                web_sys::WheelEvent::DOM_DELTA_LINE => {
                    #[allow(clippy::let_and_return)]
                    let points_per_scroll_line = 8.0; // Note that this is intentionally different from what we use in winit.
                    points_per_scroll_line
                }
                _ => 1.0, // DOM_DELTA_PIXEL
            };

            let mut delta =
                -scroll_multiplier * egui::vec2(event.delta_x() as f32, event.delta_y() as f32);

            // Report a zoom event in case CTRL (on Windows or Linux) or CMD (on Mac) is pressed.
            // This if-statement is equivalent to how `Modifiers.command` is determined in
            // `modifiers_from_event()`, but we cannot directly use that fn for a [`WheelEvent`].
            if event.ctrl_key() || event.meta_key() {
                let factor = (delta.y / 200.0).exp();
                runner_lock.input.raw.events.push(egui::Event::Zoom(factor));
            } else {
                if event.shift_key() {
                    // Treat as horizontal scrolling.
                    // Note: one Mac we already get horizontal scroll events when shift is down.
                    delta = egui::vec2(delta.x + delta.y, 0.0);
                }

                runner_lock
                    .input
                    .raw
                    .events
                    .push(egui::Event::Scroll(delta));
            }

            runner_lock.needs_repaint.repaint_asap();
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    runner_container.add_event_listener(
        &canvas,
        "dragover",
        |event: web_sys::DragEvent, mut runner_lock| {
            if let Some(data_transfer) = event.data_transfer() {
                runner_lock.input.raw.hovered_files.clear();
                for i in 0..data_transfer.items().length() {
                    if let Some(item) = data_transfer.items().get(i) {
                        runner_lock.input.raw.hovered_files.push(egui::HoveredFile {
                            mime: item.type_(),
                            ..Default::default()
                        });
                    }
                }
                runner_lock.needs_repaint.repaint_asap();
                event.stop_propagation();
                event.prevent_default();
            }
        },
    )?;

    runner_container.add_event_listener(
        &canvas,
        "dragleave",
        |event: web_sys::DragEvent, mut runner_lock| {
            runner_lock.input.raw.hovered_files.clear();
            runner_lock.needs_repaint.repaint_asap();
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    runner_container.add_event_listener(&canvas, "drop", {
        let runner_ref = runner_container.runner.clone();

        move |event: web_sys::DragEvent, mut runner_lock| {
            if let Some(data_transfer) = event.data_transfer() {
                runner_lock.input.raw.hovered_files.clear();
                runner_lock.needs_repaint.repaint_asap();
                // Unlock the runner so it can be locked after a future await point
                drop(runner_lock);

                if let Some(files) = data_transfer.files() {
                    for i in 0..files.length() {
                        if let Some(file) = files.get(i) {
                            let name = file.name();
                            let last_modified = std::time::UNIX_EPOCH
                                + std::time::Duration::from_millis(file.last_modified() as u64);

                            tracing::debug!("Loading {:?} ({} bytes)…", name, file.size());

                            let future = wasm_bindgen_futures::JsFuture::from(file.array_buffer());

                            let runner_ref = runner_ref.clone();
                            let future = async move {
                                match future.await {
                                    Ok(array_buffer) => {
                                        let bytes = js_sys::Uint8Array::new(&array_buffer).to_vec();
                                        tracing::debug!(
                                            "Loaded {:?} ({} bytes).",
                                            name,
                                            bytes.len()
                                        );

                                        // Re-lock the mutex on the other side of the await point
                                        let mut runner_lock = runner_ref.lock();
                                        runner_lock.input.raw.dropped_files.push(
                                            egui::DroppedFile {
                                                name,
                                                last_modified: Some(last_modified),
                                                bytes: Some(bytes.into()),
                                                ..Default::default()
                                            },
                                        );
                                        runner_lock.needs_repaint.repaint_asap();
                                    }
                                    Err(err) => {
                                        tracing::error!("Failed to read file: {:?}", err);
                                    }
                                }
                            };
                            wasm_bindgen_futures::spawn_local(future);
                        }
                    }
                }
                event.stop_propagation();
                event.prevent_default();
            }
        }
    })?;

    Ok(())
}
