use web_sys::EventTarget;

use super::*;

// TODO(emilk): there are more calls to `prevent_default` and `stop_propagaton`
// than what is probably needed.

// ------------------------------------------------------------------------

/// Calls `request_animation_frame` to schedule repaint.
///
/// It will only paint if needed, but will always call `request_animation_frame` immediately.
pub(crate) fn paint_and_schedule(runner_ref: &WebRunner) -> Result<(), JsValue> {
    // Only paint and schedule if there has been no panic
    if let Some(mut runner_lock) = runner_ref.try_lock() {
        paint_if_needed(&mut runner_lock);
        drop(runner_lock);
        runner_ref.request_animation_frame()?;
    }
    Ok(())
}

fn paint_if_needed(runner: &mut AppRunner) {
    if runner.needs_repaint.needs_repaint() {
        if runner.has_outstanding_paint_data() {
            // We have already run the logic, e.g. in an on-click event,
            // so let's only present the results:
            runner.paint();

            // We schedule another repaint asap, so that we can run the actual logic
            // again, which may schedule a new repaint (if there's animations):
            runner.needs_repaint.repaint_asap();
        } else {
            // Clear the `needs_repaint` flags _before_
            // running the logic, as the logic could cause it to be set again.
            runner.needs_repaint.clear();

            let mut stopwatch = crate::stopwatch::Stopwatch::new();
            stopwatch.start();

            // Run user code…
            runner.logic();

            // …and paint the result.
            runner.paint();

            runner.report_frame_time(stopwatch.total_time_sec());
        }
    }
    runner.auto_save_if_needed();
}

// ------------------------------------------------------------------------

pub(crate) fn install_event_handlers(runner_ref: &WebRunner) -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = runner_ref.try_lock().unwrap().canvas().clone();

    install_blur_focus(runner_ref, &canvas)?;

    prevent_default_and_stop_propagation(
        runner_ref,
        &canvas,
        &[
            // Allow users to use ctrl-p for e.g. a command palette:
            "afterprint",
            // By default, right-clicks open a browser context menu.
            // We don't want to do that (right clicks are handled by egui):
            "contextmenu",
        ],
    )?;

    install_keydown(runner_ref, &canvas)?;
    install_keyup(runner_ref, &canvas)?;

    // It seems copy/cut/paste events only work on the document,
    // so we check if we have focus inside of the handler.
    install_copy_cut_paste(runner_ref, &document)?;

    install_mousedown(runner_ref, &canvas)?;
    // Use `document` here to notice if the user releases a drag outside of the canvas:
    // See https://github.com/emilk/egui/issues/3157
    install_mousemove(runner_ref, &document)?;
    install_mouseup(runner_ref, &document)?;
    install_mouseleave(runner_ref, &canvas)?;

    install_touchstart(runner_ref, &canvas)?;
    // Use `document` here to notice if the user drag outside of the canvas:
    // See https://github.com/emilk/egui/issues/3157
    install_touchmove(runner_ref, &document)?;
    install_touchend(runner_ref, &document)?;
    install_touchcancel(runner_ref, &canvas)?;

    install_wheel(runner_ref, &canvas)?;
    install_drag_and_drop(runner_ref, &canvas)?;
    install_window_events(runner_ref, &window)?;
    Ok(())
}

fn install_blur_focus(runner_ref: &WebRunner, target: &EventTarget) -> Result<(), JsValue> {
    // NOTE: because of the text agent we sometime miss 'blur' events,
    // so we also poll the focus state each frame in `AppRunner::logic`.
    for event_name in ["blur", "focus"] {
        let closure = move |_event: web_sys::MouseEvent, runner: &mut AppRunner| {
            log::trace!("{} {event_name:?}", runner.canvas().id());
            runner.update_focus();

            if event_name == "blur" {
                // This might be a good time to save the state
                runner.save();
            }
        };

        runner_ref.add_event_listener(target, event_name, closure)?;
    }
    Ok(())
}

fn install_keydown(runner_ref: &WebRunner, target: &EventTarget) -> Result<(), JsValue> {
    runner_ref.add_event_listener(
        target,
        "keydown",
        |event: web_sys::KeyboardEvent, runner| {
            if !runner.input.raw.focused {
                return;
            }

            let modifiers = modifiers_from_kb_event(&event);
            if !modifiers.ctrl
                && !modifiers.command
                // When text agent is focused, it is responsible for handling input events
                && !runner.text_agent.has_focus()
            {
                if let Some(text) = text_from_keyboard_event(&event) {
                    runner.input.raw.events.push(egui::Event::Text(text));
                    runner.needs_repaint.repaint_asap();

                    // If this is indeed text, then prevent any other action.
                    event.prevent_default();

                    // Assume egui uses all key events, and don't let them propagate to parent elements.
                    event.stop_propagation();
                }
            }

            on_keydown(event, runner);
        },
    )
}

#[allow(clippy::needless_pass_by_value)] // So that we can pass it directly to `add_event_listener`
pub(crate) fn on_keydown(event: web_sys::KeyboardEvent, runner: &mut AppRunner) {
    let has_focus = runner.input.raw.focused;
    if !has_focus {
        return;
    }

    if event.is_composing() || event.key_code() == 229 {
        // https://web.archive.org/web/20200526195704/https://www.fxsitecompat.dev/en-CA/docs/2018/keydown-and-keyup-events-are-now-fired-during-ime-composition/
        return;
    }

    let modifiers = modifiers_from_kb_event(&event);
    runner.input.raw.modifiers = modifiers;

    let key = event.key();
    let egui_key = translate_key(&key);

    if let Some(egui_key) = egui_key {
        runner.input.raw.events.push(egui::Event::Key {
            key: egui_key,
            physical_key: None, // TODO(fornwall)
            pressed: true,
            repeat: false, // egui will fill this in for us!
            modifiers,
        });
        runner.needs_repaint.repaint_asap();

        let prevent_default = should_prevent_default_for_key(runner, &modifiers, egui_key);

        // log::debug!(
        //     "On keydown {:?} {egui_key:?}, has_focus: {has_focus}, egui_wants_keyboard: {}, prevent_default: {prevent_default}",
        //     event.key().as_str(),
        //     runner.egui_ctx().wants_keyboard_input()
        // );

        if prevent_default {
            event.prevent_default();
        }

        // Assume egui uses all key events, and don't let them propagate to parent elements.
        event.stop_propagation();
    }
}

/// If the canvas (or text agent) has focus:
/// should we prevent the default browser event action when the user presses this key?
fn should_prevent_default_for_key(
    runner: &AppRunner,
    modifiers: &egui::Modifiers,
    egui_key: egui::Key,
) -> bool {
    // NOTE: We never want to prevent:
    // * F5 / cmd-R (refresh)
    // * cmd-shift-C (debug tools)
    // * cmd/ctrl-c/v/x (lest we prevent copy/paste/cut events)

    // Prevent ctrl-P from opening the print dialog. Users may want to use it for a command palette.
    if egui_key == egui::Key::P && (modifiers.ctrl || modifiers.command || modifiers.mac_cmd) {
        return true;
    }

    if egui_key == egui::Key::Space && !runner.text_agent.has_focus() {
        // Space scrolls the web page, but we don't want that while canvas has focus
        // However, don't prevent it if text agent has focus, or we can't type space!
        return true;
    }

    matches!(
        egui_key,
        // Prevent browser from focusing the next HTML element.
        // egui uses Tab to move focus within the egui app.
        egui::Key::Tab

        // So we don't go back to previous page while canvas has focus
        | egui::Key::Backspace

        // Don't scroll web page while canvas has focus.
        // Also, cmd-left is "back" on Mac (https://github.com/emilk/egui/issues/58)
        | egui::Key::ArrowDown | egui::Key::ArrowLeft | egui::Key::ArrowRight |  egui::Key::ArrowUp
    )
}

fn install_keyup(runner_ref: &WebRunner, target: &EventTarget) -> Result<(), JsValue> {
    runner_ref.add_event_listener(target, "keyup", on_keyup)
}

#[allow(clippy::needless_pass_by_value)] // So that we can pass it directly to `add_event_listener`
pub(crate) fn on_keyup(event: web_sys::KeyboardEvent, runner: &mut AppRunner) {
    let modifiers = modifiers_from_kb_event(&event);
    runner.input.raw.modifiers = modifiers;

    if let Some(key) = translate_key(&event.key()) {
        runner.input.raw.events.push(egui::Event::Key {
            key,
            physical_key: None, // TODO(fornwall)
            pressed: false,
            repeat: false,
            modifiers,
        });
    }

    if event.key() == "Meta" || event.key() == "Control" {
        // When pressing Cmd+A (select all) or Ctrl+C (copy),
        // chromium will not fire a `keyup` for the letter key.
        // This leads to stuck keys, unless we do this hack.
        // See https://github.com/emilk/egui/issues/4724

        let keys_down = runner.egui_ctx().input(|i| i.keys_down.clone());
        for key in keys_down {
            runner.input.raw.events.push(egui::Event::Key {
                key,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers,
            });
        }
    }

    runner.needs_repaint.repaint_asap();

    let has_focus = runner.input.raw.focused;
    if has_focus {
        // Assume egui uses all key events, and don't let them propagate to parent elements.
        event.stop_propagation();
    }
}

fn install_copy_cut_paste(runner_ref: &WebRunner, target: &EventTarget) -> Result<(), JsValue> {
    #[cfg(web_sys_unstable_apis)]
    runner_ref.add_event_listener(target, "paste", |event: web_sys::ClipboardEvent, runner| {
        if let Some(data) = event.clipboard_data() {
            if let Ok(text) = data.get_data("text") {
                let text = text.replace("\r\n", "\n");
                if !text.is_empty() && runner.input.raw.focused {
                    runner.input.raw.events.push(egui::Event::Paste(text));
                    runner.needs_repaint.repaint_asap();
                }
                event.stop_propagation();
                event.prevent_default();
            }
        }
    })?;

    #[cfg(web_sys_unstable_apis)]
    runner_ref.add_event_listener(target, "cut", |event: web_sys::ClipboardEvent, runner| {
        if runner.input.raw.focused {
            runner.input.raw.events.push(egui::Event::Cut);

            // In Safari we are only allowed to write to the clipboard during the
            // event callback, which is why we run the app logic here and now:
            runner.logic();

            // Make sure we paint the output of the above logic call asap:
            runner.needs_repaint.repaint_asap();
        }

        event.stop_propagation();
        event.prevent_default();
    })?;

    #[cfg(web_sys_unstable_apis)]
    runner_ref.add_event_listener(target, "copy", |event: web_sys::ClipboardEvent, runner| {
        if runner.input.raw.focused {
            runner.input.raw.events.push(egui::Event::Copy);

            // In Safari we are only allowed to write to the clipboard during the
            // event callback, which is why we run the app logic here and now:
            runner.logic();

            // Make sure we paint the output of the above logic call asap:
            runner.needs_repaint.repaint_asap();
        }

        event.stop_propagation();
        event.prevent_default();
    })?;

    Ok(())
}

fn install_window_events(runner_ref: &WebRunner, window: &EventTarget) -> Result<(), JsValue> {
    // Save-on-close
    runner_ref.add_event_listener(window, "onbeforeunload", |_: web_sys::Event, runner| {
        runner.save();
    })?;

    // NOTE: resize is handled by `ResizeObserver` below
    for event_name in &["load", "pagehide", "pageshow"] {
        runner_ref.add_event_listener(window, event_name, move |_: web_sys::Event, runner| {
            // log::debug!("{event_name:?}");
            runner.needs_repaint.repaint_asap();
        })?;
    }

    runner_ref.add_event_listener(window, "hashchange", |_: web_sys::Event, runner| {
        // `epi::Frame::info(&self)` clones `epi::IntegrationInfo`, but we need to modify the original here
        runner.frame.info.web_info.location.hash = location_hash();
        runner.needs_repaint.repaint_asap(); // tell the user about the new hash
    })?;

    Ok(())
}

pub(crate) fn install_color_scheme_change_event(runner_ref: &WebRunner) -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();

    if let Some(media_query_list) = prefers_color_scheme_dark(&window)? {
        runner_ref.add_event_listener::<web_sys::MediaQueryListEvent>(
            &media_query_list,
            "change",
            |event, runner| {
                let theme = theme_from_dark_mode(event.matches());
                runner.frame.info.system_theme = Some(theme);
                runner.egui_ctx().set_visuals(theme.egui_visuals());
                runner.needs_repaint.repaint_asap();
            },
        )?;
    }

    Ok(())
}

fn prevent_default_and_stop_propagation(
    runner_ref: &WebRunner,
    target: &EventTarget,
    event_names: &[&'static str],
) -> Result<(), JsValue> {
    for event_name in event_names {
        let closure = move |event: web_sys::MouseEvent, _runner: &mut AppRunner| {
            event.prevent_default();
            event.stop_propagation();
            // log::debug!("Preventing event {event_name:?}");
        };

        runner_ref.add_event_listener(target, event_name, closure)?;
    }

    Ok(())
}

fn install_mousedown(runner_ref: &WebRunner, target: &EventTarget) -> Result<(), JsValue> {
    runner_ref.add_event_listener(
        target,
        "mousedown",
        |event: web_sys::MouseEvent, runner: &mut AppRunner| {
            let modifiers = modifiers_from_mouse_event(&event);
            runner.input.raw.modifiers = modifiers;
            if let Some(button) = button_from_mouse_event(&event) {
                let pos = pos_from_mouse_event(runner.canvas(), &event, runner.egui_ctx());
                let modifiers = runner.input.raw.modifiers;
                runner.input.raw.events.push(egui::Event::PointerButton {
                    pos,
                    button,
                    pressed: true,
                    modifiers,
                });

                // In Safari we are only allowed to write to the clipboard during the
                // event callback, which is why we run the app logic here and now:
                runner.logic();

                // Make sure we paint the output of the above logic call asap:
                runner.needs_repaint.repaint_asap();
            }
            event.stop_propagation();
            // Note: prevent_default breaks VSCode tab focusing, hence why we don't call it here.
        },
    )
}

/// Returns true if the cursor is above the canvas, or if we're dragging something.
fn is_interested_in_pointer_event(egui_ctx: &egui::Context, pos: egui::Pos2) -> bool {
    egui_ctx.input(|i| i.screen_rect().contains(pos) || i.pointer.any_down() || i.any_touches())
}

fn install_mousemove(runner_ref: &WebRunner, target: &EventTarget) -> Result<(), JsValue> {
    runner_ref.add_event_listener(target, "mousemove", |event: web_sys::MouseEvent, runner| {
        let modifiers = modifiers_from_mouse_event(&event);
        runner.input.raw.modifiers = modifiers;

        let pos = pos_from_mouse_event(runner.canvas(), &event, runner.egui_ctx());

        if is_interested_in_pointer_event(runner.egui_ctx(), pos) {
            runner.input.raw.events.push(egui::Event::PointerMoved(pos));
            runner.needs_repaint.repaint_asap();
            event.stop_propagation();
            event.prevent_default();
        }
    })
}

fn install_mouseup(runner_ref: &WebRunner, target: &EventTarget) -> Result<(), JsValue> {
    runner_ref.add_event_listener(target, "mouseup", |event: web_sys::MouseEvent, runner| {
        let modifiers = modifiers_from_mouse_event(&event);
        runner.input.raw.modifiers = modifiers;

        let pos = pos_from_mouse_event(runner.canvas(), &event, runner.egui_ctx());

        if is_interested_in_pointer_event(runner.egui_ctx(), pos) {
            if let Some(button) = button_from_mouse_event(&event) {
                let modifiers = runner.input.raw.modifiers;
                runner.input.raw.events.push(egui::Event::PointerButton {
                    pos,
                    button,
                    pressed: false,
                    modifiers,
                });

                // In Safari we are only allowed to do certain things
                // (like playing audio, start a download, etc)
                // on user action, such as a click.
                // So we need to run the app logic here and now:
                runner.logic();

                // Make sure we paint the output of the above logic call asap:
                runner.needs_repaint.repaint_asap();

                event.prevent_default();
                event.stop_propagation();
            }
        }
    })
}

fn install_mouseleave(runner_ref: &WebRunner, target: &EventTarget) -> Result<(), JsValue> {
    runner_ref.add_event_listener(
        target,
        "mouseleave",
        |event: web_sys::MouseEvent, runner| {
            runner.input.raw.events.push(egui::Event::PointerGone);
            runner.needs_repaint.repaint_asap();
            event.stop_propagation();
            event.prevent_default();
        },
    )
}

fn install_touchstart(runner_ref: &WebRunner, target: &EventTarget) -> Result<(), JsValue> {
    runner_ref.add_event_listener(
        target,
        "touchstart",
        |event: web_sys::TouchEvent, runner| {
            if let Some(pos) = primary_touch_pos(runner, &event) {
                runner.input.raw.events.push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: runner.input.raw.modifiers,
                });
            }

            push_touches(runner, egui::TouchPhase::Start, &event);
            runner.needs_repaint.repaint_asap();
            event.stop_propagation();
            event.prevent_default();
        },
    )
}

fn install_touchmove(runner_ref: &WebRunner, target: &EventTarget) -> Result<(), JsValue> {
    runner_ref.add_event_listener(target, "touchmove", |event: web_sys::TouchEvent, runner| {
        if let Some(pos) = primary_touch_pos(runner, &event) {
            if is_interested_in_pointer_event(runner.egui_ctx(), pos) {
                runner.input.raw.events.push(egui::Event::PointerMoved(pos));

                push_touches(runner, egui::TouchPhase::Move, &event);
                runner.needs_repaint.repaint_asap();
                event.stop_propagation();
                event.prevent_default();
            }
        }
    })
}

fn install_touchend(runner_ref: &WebRunner, target: &EventTarget) -> Result<(), JsValue> {
    runner_ref.add_event_listener(target, "touchend", |event: web_sys::TouchEvent, runner| {
        if let Some(pos) = primary_touch_pos(runner, &event) {
            if is_interested_in_pointer_event(runner.egui_ctx(), pos) {
                // First release mouse to click:
                runner.input.raw.events.push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: runner.input.raw.modifiers,
                });
                // Then remove hover effect:
                runner.input.raw.events.push(egui::Event::PointerGone);

                push_touches(runner, egui::TouchPhase::End, &event);

                runner.needs_repaint.repaint_asap();
                event.stop_propagation();
                event.prevent_default();
            }
        }
    })
}

fn install_touchcancel(runner_ref: &WebRunner, target: &EventTarget) -> Result<(), JsValue> {
    runner_ref.add_event_listener(
        target,
        "touchcancel",
        |event: web_sys::TouchEvent, runner| {
            push_touches(runner, egui::TouchPhase::Cancel, &event);
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    Ok(())
}

fn install_wheel(runner_ref: &WebRunner, target: &EventTarget) -> Result<(), JsValue> {
    runner_ref.add_event_listener(target, "wheel", |event: web_sys::WheelEvent, runner| {
        let unit = match event.delta_mode() {
            web_sys::WheelEvent::DOM_DELTA_PIXEL => egui::MouseWheelUnit::Point,
            web_sys::WheelEvent::DOM_DELTA_LINE => egui::MouseWheelUnit::Line,
            web_sys::WheelEvent::DOM_DELTA_PAGE => egui::MouseWheelUnit::Page,
            _ => return,
        };

        let delta = -egui::vec2(event.delta_x() as f32, event.delta_y() as f32);

        let modifiers = modifiers_from_wheel_event(&event);

        if modifiers.ctrl && !runner.input.raw.modifiers.ctrl {
            // The browser is saying the ctrl key is down, but it isn't _really_.
            // This happens on pinch-to-zoom on a Mac trackpad.
            // egui will treat ctrl+scroll as zoom, so it all works.
            // However, we explicitly handle it here in order to better match the pinch-to-zoom
            // speed of a native app, without being sensitive to egui's `scroll_zoom_speed` setting.
            let pinch_to_zoom_sensitivity = 0.01; // Feels good on a Mac trackpad in 2024
            let zoom_factor = (pinch_to_zoom_sensitivity * delta.y).exp();
            runner.input.raw.events.push(egui::Event::Zoom(zoom_factor));
        } else {
            runner.input.raw.events.push(egui::Event::MouseWheel {
                unit,
                delta,
                modifiers,
            });
        }

        runner.needs_repaint.repaint_asap();
        event.stop_propagation();
        event.prevent_default();
    })
}

fn install_drag_and_drop(runner_ref: &WebRunner, target: &EventTarget) -> Result<(), JsValue> {
    runner_ref.add_event_listener(target, "dragover", |event: web_sys::DragEvent, runner| {
        if let Some(data_transfer) = event.data_transfer() {
            runner.input.raw.hovered_files.clear();

            // NOTE: data_transfer.files() is always empty in dragover

            let items = data_transfer.items();
            for i in 0..items.length() {
                if let Some(item) = items.get(i) {
                    runner.input.raw.hovered_files.push(egui::HoveredFile {
                        mime: item.type_(),
                        ..Default::default()
                    });
                }
            }

            if runner.input.raw.hovered_files.is_empty() {
                // Fallback: just preview anything. Needed on Desktop Safari.
                runner
                    .input
                    .raw
                    .hovered_files
                    .push(egui::HoveredFile::default());
            }

            runner.needs_repaint.repaint_asap();
            event.stop_propagation();
            event.prevent_default();
        }
    })?;

    runner_ref.add_event_listener(target, "dragleave", |event: web_sys::DragEvent, runner| {
        runner.input.raw.hovered_files.clear();
        runner.needs_repaint.repaint_asap();
        event.stop_propagation();
        event.prevent_default();
    })?;

    runner_ref.add_event_listener(target, "drop", {
        let runner_ref = runner_ref.clone();

        move |event: web_sys::DragEvent, runner| {
            if let Some(data_transfer) = event.data_transfer() {
                // TODO(https://github.com/emilk/egui/issues/3702): support dropping folders
                runner.input.raw.hovered_files.clear();
                runner.needs_repaint.repaint_asap();

                if let Some(files) = data_transfer.files() {
                    for i in 0..files.length() {
                        if let Some(file) = files.get(i) {
                            let name = file.name();
                            let mime = file.type_();
                            let last_modified = std::time::UNIX_EPOCH
                                + std::time::Duration::from_millis(file.last_modified() as u64);

                            log::debug!("Loading {:?} ({} bytes)…", name, file.size());

                            let future = wasm_bindgen_futures::JsFuture::from(file.array_buffer());

                            let runner_ref = runner_ref.clone();
                            let future = async move {
                                match future.await {
                                    Ok(array_buffer) => {
                                        let bytes = js_sys::Uint8Array::new(&array_buffer).to_vec();
                                        log::debug!("Loaded {:?} ({} bytes).", name, bytes.len());

                                        if let Some(mut runner_lock) = runner_ref.try_lock() {
                                            runner_lock.input.raw.dropped_files.push(
                                                egui::DroppedFile {
                                                    name,
                                                    mime,
                                                    last_modified: Some(last_modified),
                                                    bytes: Some(bytes.into()),
                                                    ..Default::default()
                                                },
                                            );
                                            runner_lock.needs_repaint.repaint_asap();
                                        }
                                    }
                                    Err(err) => {
                                        log::error!("Failed to read file: {:?}", err);
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

/// Install a `ResizeObserver` to observe changes to the size of the canvas.
///
/// This is the only way to ensure a canvas size change without an associated window `resize` event
/// actually results in a resize of the canvas.
///
/// The resize observer is called the by the browser at `observe` time, instead of just on the first actual resize.
/// We use that to trigger the first `request_animation_frame` _after_ updating the size of the canvas to the correct dimensions,
/// to avoid [#4622](https://github.com/emilk/egui/issues/4622).
pub(crate) fn install_resize_observer(runner_ref: &WebRunner) -> Result<(), JsValue> {
    let closure = Closure::wrap(Box::new({
        let runner_ref = runner_ref.clone();
        move |entries: js_sys::Array| {
            // Only call the wrapped closure if the egui code has not panicked
            if let Some(mut runner_lock) = runner_ref.try_lock() {
                let canvas = runner_lock.canvas();
                let (width, height) = match get_display_size(&entries) {
                    Ok(v) => v,
                    Err(err) => {
                        log::error!("{}", super::string_from_js_value(&err));
                        return;
                    }
                };
                canvas.set_width(width);
                canvas.set_height(height);

                // force an immediate repaint
                runner_lock.needs_repaint.repaint_asap();
                paint_if_needed(&mut runner_lock);
                drop(runner_lock);
                // we rely on the resize observer to trigger the first `request_animation_frame`:
                if let Err(err) = runner_ref.request_animation_frame() {
                    log::error!("{}", super::string_from_js_value(&err));
                };
            }
        }
    }) as Box<dyn FnMut(js_sys::Array)>);

    let observer = web_sys::ResizeObserver::new(closure.as_ref().unchecked_ref())?;
    let mut options = web_sys::ResizeObserverOptions::new();
    options.box_(web_sys::ResizeObserverBoxOptions::ContentBox);
    if let Some(runner_lock) = runner_ref.try_lock() {
        observer.observe_with_options(runner_lock.canvas(), &options);
        drop(runner_lock);
        runner_ref.set_resize_observer(observer, closure);
    }

    Ok(())
}

// Code ported to Rust from:
// https://webglfundamentals.org/webgl/lessons/webgl-resizing-the-canvas.html
fn get_display_size(resize_observer_entries: &js_sys::Array) -> Result<(u32, u32), JsValue> {
    let width;
    let height;
    let mut dpr = web_sys::window().unwrap().device_pixel_ratio();

    let entry: web_sys::ResizeObserverEntry = resize_observer_entries.at(0).dyn_into()?;
    if JsValue::from_str("devicePixelContentBoxSize").js_in(entry.as_ref()) {
        // NOTE: Only this path gives the correct answer for most browsers.
        // Unfortunately this doesn't work perfectly everywhere.
        let size: web_sys::ResizeObserverSize =
            entry.device_pixel_content_box_size().at(0).dyn_into()?;
        width = size.inline_size();
        height = size.block_size();
        dpr = 1.0; // no need to apply
    } else if JsValue::from_str("contentBoxSize").js_in(entry.as_ref()) {
        let content_box_size = entry.content_box_size();
        let idx0 = content_box_size.at(0);
        if !idx0.is_undefined() {
            let size: web_sys::ResizeObserverSize = idx0.dyn_into()?;
            width = size.inline_size();
            height = size.block_size();
        } else {
            // legacy
            let size = JsValue::clone(content_box_size.as_ref());
            let size: web_sys::ResizeObserverSize = size.dyn_into()?;
            width = size.inline_size();
            height = size.block_size();
        }
    } else {
        // legacy
        let content_rect = entry.content_rect();
        width = content_rect.width();
        height = content_rect.height();
    }

    Ok(((width.round() * dpr) as u32, (height.round() * dpr) as u32))
}
