use super::*;

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

pub(crate) fn install_document_events(runner_ref: &WebRunner) -> Result<(), JsValue> {
    let document = web_sys::window().unwrap().document().unwrap();

    for event_name in ["blur", "focus"] {
        let closure = move |_event: web_sys::MouseEvent, runner: &mut AppRunner| {
            // log::debug!("{event_name:?}");
            let has_focus = event_name == "focus";

            if !has_focus {
                // We lost focus - good idea to save
                runner.save();
            }

            runner.input.on_web_page_focus_change(has_focus);
            runner.egui_ctx().request_repaint();
        };

        runner_ref.add_event_listener(&document, event_name, closure)?;
    }

    runner_ref.add_event_listener(
        &document,
        "keydown",
        |event: web_sys::KeyboardEvent, runner| {
            if event.is_composing() || event.key_code() == 229 {
                // https://web.archive.org/web/20200526195704/https://www.fxsitecompat.dev/en-CA/docs/2018/keydown-and-keyup-events-are-now-fired-during-ime-composition/
                return;
            }

            let modifiers = modifiers_from_kb_event(&event);
            runner.input.raw.modifiers = modifiers;

            let key = event.key();
            let egui_key = translate_key(&key);

            if let Some(key) = egui_key {
                runner.input.raw.events.push(egui::Event::Key {
                    key,
                    physical_key: None, // TODO(fornwall)
                    pressed: true,
                    repeat: false, // egui will fill this in for us!
                    modifiers,
                });
            }
            if !modifiers.ctrl
                && !modifiers.command
                && !should_ignore_key(&key)
                // When text agent is focused, it is responsible for handling input events
                && !runner.text_agent.has_focus()
            {
                runner.input.raw.events.push(egui::Event::Text(key));
            }
            runner.needs_repaint.repaint_asap();

            let egui_wants_keyboard = runner.egui_ctx().wants_keyboard_input();

            #[allow(clippy::if_same_then_else)]
            let prevent_default = if egui_key == Some(egui::Key::Tab) {
                // Always prevent moving cursor to url bar.
                // egui wants to use tab to move to the next text field.
                true
            } else if egui_key == Some(egui::Key::P) {
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

            // log::debug!(
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

    runner_ref.add_event_listener(
        &document,
        "keyup",
        |event: web_sys::KeyboardEvent, runner| {
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
            runner.needs_repaint.repaint_asap();
        },
    )?;

    #[cfg(web_sys_unstable_apis)]
    runner_ref.add_event_listener(
        &document,
        "paste",
        |event: web_sys::ClipboardEvent, runner| {
            if let Some(data) = event.clipboard_data() {
                if let Ok(text) = data.get_data("text") {
                    let text = text.replace("\r\n", "\n");
                    if !text.is_empty() {
                        runner.input.raw.events.push(egui::Event::Paste(text));
                        runner.needs_repaint.repaint_asap();
                    }
                    event.stop_propagation();
                    event.prevent_default();
                }
            }
        },
    )?;

    #[cfg(web_sys_unstable_apis)]
    runner_ref.add_event_listener(
        &document,
        "cut",
        |event: web_sys::ClipboardEvent, runner| {
            runner.input.raw.events.push(egui::Event::Cut);

            // In Safari we are only allowed to write to the clipboard during the
            // event callback, which is why we run the app logic here and now:
            runner.logic();

            // Make sure we paint the output of the above logic call asap:
            runner.needs_repaint.repaint_asap();

            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    #[cfg(web_sys_unstable_apis)]
    runner_ref.add_event_listener(
        &document,
        "copy",
        |event: web_sys::ClipboardEvent, runner| {
            runner.input.raw.events.push(egui::Event::Copy);

            // In Safari we are only allowed to write to the clipboard during the
            // event callback, which is why we run the app logic here and now:
            runner.logic();

            // Make sure we paint the output of the above logic call asap:
            runner.needs_repaint.repaint_asap();

            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    Ok(())
}

pub(crate) fn install_window_events(runner_ref: &WebRunner) -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();

    for event_name in ["blur", "focus"] {
        let closure = move |_event: web_sys::MouseEvent, runner: &mut AppRunner| {
            // log::debug!("{event_name:?}");
            let has_focus = event_name == "focus";

            if !has_focus {
                // We lost focus - good idea to save
                runner.save();
            }

            runner.input.on_web_page_focus_change(has_focus);
            runner.egui_ctx().request_repaint();
        };

        runner_ref.add_event_listener(&window, event_name, closure)?;
    }

    // Save-on-close
    runner_ref.add_event_listener(&window, "onbeforeunload", |_: web_sys::Event, runner| {
        runner.save();
    })?;

    // NOTE: resize is handled by `ResizeObserver` below
    for event_name in &["load", "pagehide", "pageshow"] {
        runner_ref.add_event_listener(&window, event_name, move |_: web_sys::Event, runner| {
            // log::debug!("{event_name:?}");
            runner.needs_repaint.repaint_asap();
        })?;
    }

    runner_ref.add_event_listener(&window, "hashchange", |_: web_sys::Event, runner| {
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

pub(crate) fn install_canvas_events(runner_ref: &WebRunner) -> Result<(), JsValue> {
    let canvas = runner_ref.try_lock().unwrap().canvas().clone();
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();

    {
        let prevent_default_events = [
            // By default, right-clicks open a context menu.
            // We don't want to do that (right clicks is handled by egui):
            "contextmenu",
            // Allow users to use ctrl-p for e.g. a command palette:
            "afterprint",
        ];

        for event_name in prevent_default_events {
            let closure = move |event: web_sys::MouseEvent, _runner: &mut AppRunner| {
                event.prevent_default();
                // event.stop_propagation();
                // log::debug!("Preventing event {event_name:?}");
            };

            runner_ref.add_event_listener(&canvas, event_name, closure)?;
        }
    }

    runner_ref.add_event_listener(
        &canvas,
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
    )?;

    // NOTE: we register "mousemove" on `document` instead of just the canvas
    // in order to track a dragged mouse outside the canvas.
    // See https://github.com/emilk/egui/issues/3157
    runner_ref.add_event_listener(
        &document,
        "mousemove",
        |event: web_sys::MouseEvent, runner| {
            let modifiers = modifiers_from_mouse_event(&event);
            runner.input.raw.modifiers = modifiers;
            let pos = pos_from_mouse_event(runner.canvas(), &event, runner.egui_ctx());
            runner.input.raw.events.push(egui::Event::PointerMoved(pos));
            runner.needs_repaint.repaint_asap();
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    // Use `document` here to notice if the user releases a drag outside of the canvas.
    // See https://github.com/emilk/egui/issues/3157
    runner_ref.add_event_listener(
        &document,
        "mouseup",
        |event: web_sys::MouseEvent, runner| {
            let modifiers = modifiers_from_mouse_event(&event);
            runner.input.raw.modifiers = modifiers;
            if let Some(button) = button_from_mouse_event(&event) {
                let pos = pos_from_mouse_event(runner.canvas(), &event, runner.egui_ctx());
                let modifiers = runner.input.raw.modifiers;
                runner.input.raw.events.push(egui::Event::PointerButton {
                    pos,
                    button,
                    pressed: false,
                    modifiers,
                });

                // In Safari we are only allowed to write to the clipboard during the
                // event callback, which is why we run the app logic here and now:
                runner.logic();

                runner
                    .text_agent
                    .set_focus(runner.mutable_text_under_cursor);

                // Make sure we paint the output of the above logic call asap:
                runner.needs_repaint.repaint_asap();
            }
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    runner_ref.add_event_listener(
        &canvas,
        "mouseleave",
        |event: web_sys::MouseEvent, runner| {
            runner.input.raw.events.push(egui::Event::PointerGone);
            runner.needs_repaint.repaint_asap();
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    runner_ref.add_event_listener(
        &canvas,
        "touchstart",
        |event: web_sys::TouchEvent, runner| {
            let mut latest_touch_pos_id = runner.input.latest_touch_pos_id;
            let pos = pos_from_touch_event(
                runner.canvas(),
                &event,
                &mut latest_touch_pos_id,
                runner.egui_ctx(),
            );
            runner.input.latest_touch_pos_id = latest_touch_pos_id;
            runner.input.latest_touch_pos = Some(pos);
            let modifiers = runner.input.raw.modifiers;
            runner.input.raw.events.push(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers,
            });

            push_touches(runner, egui::TouchPhase::Start, &event);
            runner.needs_repaint.repaint_asap();
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    // Use `document` here to notice if the user drag outside of the canvas.
    // See https://github.com/emilk/egui/issues/3157
    runner_ref.add_event_listener(
        &document,
        "touchmove",
        |event: web_sys::TouchEvent, runner| {
            let mut latest_touch_pos_id = runner.input.latest_touch_pos_id;
            let pos = pos_from_touch_event(
                runner.canvas(),
                &event,
                &mut latest_touch_pos_id,
                runner.egui_ctx(),
            );
            runner.input.latest_touch_pos_id = latest_touch_pos_id;
            runner.input.latest_touch_pos = Some(pos);
            runner.input.raw.events.push(egui::Event::PointerMoved(pos));

            push_touches(runner, egui::TouchPhase::Move, &event);
            runner.needs_repaint.repaint_asap();
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    // Use `document` here to notice if the user releases a drag outside of the canvas.
    // See https://github.com/emilk/egui/issues/3157
    runner_ref.add_event_listener(
        &document,
        "touchend",
        |event: web_sys::TouchEvent, runner| {
            if let Some(pos) = runner.input.latest_touch_pos {
                let modifiers = runner.input.raw.modifiers;
                // First release mouse to click:
                runner.input.raw.events.push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers,
                });
                // Then remove hover effect:
                runner.input.raw.events.push(egui::Event::PointerGone);

                push_touches(runner, egui::TouchPhase::End, &event);

                runner
                    .text_agent
                    .set_focus(runner.mutable_text_under_cursor);

                runner.needs_repaint.repaint_asap();
                event.stop_propagation();
                event.prevent_default();
            }
        },
    )?;

    runner_ref.add_event_listener(
        &canvas,
        "touchcancel",
        |event: web_sys::TouchEvent, runner| {
            push_touches(runner, egui::TouchPhase::Cancel, &event);
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    runner_ref.add_event_listener(&canvas, "wheel", |event: web_sys::WheelEvent, runner| {
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
    })?;

    runner_ref.add_event_listener(&canvas, "dragover", |event: web_sys::DragEvent, runner| {
        if let Some(data_transfer) = event.data_transfer() {
            runner.input.raw.hovered_files.clear();
            for i in 0..data_transfer.items().length() {
                if let Some(item) = data_transfer.items().get(i) {
                    runner.input.raw.hovered_files.push(egui::HoveredFile {
                        mime: item.type_(),
                        ..Default::default()
                    });
                }
            }
            runner.needs_repaint.repaint_asap();
            event.stop_propagation();
            event.prevent_default();
        }
    })?;

    runner_ref.add_event_listener(&canvas, "dragleave", |event: web_sys::DragEvent, runner| {
        runner.input.raw.hovered_files.clear();
        runner.needs_repaint.repaint_asap();
        event.stop_propagation();
        event.prevent_default();
    })?;

    runner_ref.add_event_listener(&canvas, "drop", {
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
