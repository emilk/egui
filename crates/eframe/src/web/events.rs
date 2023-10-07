use super::*;

// ------------------------------------------------------------------------

/// Calls `request_animation_frame` to schedule repaint.
///
/// It will only paint if needed, but will always call `request_animation_frame` immediately.
fn paint_and_schedule(runner_ref: &WebRunner) -> Result<(), JsValue> {
    // Only paint and schedule if there has been no panic
    if let Some(mut runner_lock) = runner_ref.try_lock() {
        paint_if_needed(&mut runner_lock)?;
        drop(runner_lock);
        request_animation_frame(runner_ref.clone())?;
    }

    Ok(())
}

fn paint_if_needed(runner: &mut AppRunner) -> Result<(), JsValue> {
    if runner.needs_repaint.when_to_repaint() <= now_sec() {
        runner.needs_repaint.clear();
        let (repaint_after, clipped_primitives) = runner.logic();
        runner.paint(&clipped_primitives)?;
        runner
            .needs_repaint
            .repaint_after(repaint_after.as_secs_f64());
        runner.auto_save_if_needed();
    }
    Ok(())
}

pub(crate) fn request_animation_frame(runner_ref: WebRunner) -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();
    let closure = Closure::once(move || paint_and_schedule(&runner_ref));
    window.request_animation_frame(closure.as_ref().unchecked_ref())?;
    closure.forget(); // We must forget it, or else the callback is canceled on drop
    Ok(())
}

// ------------------------------------------------------------------------

pub(crate) fn install_document_events(runner_ref: &WebRunner) -> Result<(), JsValue> {
    let document = web_sys::window().unwrap().document().unwrap();

    {
        // Avoid sticky modifier keys on alt-tab:
        for event_name in ["blur", "focus"] {
            let closure = move |_event: web_sys::MouseEvent, runner: &mut AppRunner| {
                let has_focus = event_name == "focus";

                if !has_focus {
                    // We lost focus - good idea to save
                    runner.save();
                }

                runner.input.on_web_page_focus_change(has_focus);
                runner.egui_ctx().request_repaint();
                // log::debug!("{event_name:?}");
            };

            runner_ref.add_event_listener(&document, event_name, closure)?;
        }
    }

    runner_ref.add_event_listener(
        &document,
        "keydown",
        |event: web_sys::KeyboardEvent, runner| {
            if event.is_composing() || event.key_code() == 229 {
                // https://web.archive.org/web/20200526195704/https://www.fxsitecompat.dev/en-CA/docs/2018/keydown-and-keyup-events-are-now-fired-during-ime-composition/
                return;
            }

            let modifiers = modifiers_from_event(&event);
            runner.input.raw.modifiers = modifiers;

            let key = event.key();
            let egui_key = translate_key(&key);

            if let Some(key) = egui_key {
                runner.input.raw.events.push(egui::Event::Key {
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
            let modifiers = modifiers_from_event(&event);
            runner.input.raw.modifiers = modifiers;
            if let Some(key) = translate_key(&event.key()) {
                runner.input.raw.events.push(egui::Event::Key {
                    key,
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
    runner_ref.add_event_listener(&document, "cut", |_: web_sys::ClipboardEvent, runner| {
        runner.input.raw.events.push(egui::Event::Cut);
        runner.needs_repaint.repaint_asap();
    })?;

    #[cfg(web_sys_unstable_apis)]
    runner_ref.add_event_listener(&document, "copy", |_: web_sys::ClipboardEvent, runner| {
        runner.input.raw.events.push(egui::Event::Copy);
        runner.needs_repaint.repaint_asap();
    })?;

    Ok(())
}

pub(crate) fn install_window_events(runner_ref: &WebRunner) -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();

    // Save-on-close
    runner_ref.add_event_listener(&window, "onbeforeunload", |_: web_sys::Event, runner| {
        runner.save();
    })?;

    for event_name in &["load", "pagehide", "pageshow", "resize"] {
        runner_ref.add_event_listener(&window, event_name, |_: web_sys::Event, runner| {
            runner.needs_repaint.repaint_asap();
        })?;
    }

    runner_ref.add_event_listener(&window, "hashchange", |_: web_sys::Event, runner| {
        // `epi::Frame::info(&self)` clones `epi::IntegrationInfo`, but we need to modify the original here
        runner.frame.info.web_info.location.hash = location_hash();
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
    let canvas = canvas_element(runner_ref.try_lock().unwrap().canvas_id()).unwrap();

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
            if let Some(button) = button_from_mouse_event(&event) {
                let pos = pos_from_mouse_event(runner.canvas_id(), &event);
                let modifiers = runner.input.raw.modifiers;
                runner.input.raw.events.push(egui::Event::PointerButton {
                    pos,
                    button,
                    pressed: true,
                    modifiers,
                });
                runner.needs_repaint.repaint_asap();
            }
            event.stop_propagation();
            // Note: prevent_default breaks VSCode tab focusing, hence why we don't call it here.
        },
    )?;

    runner_ref.add_event_listener(
        &canvas,
        "mousemove",
        |event: web_sys::MouseEvent, runner| {
            let pos = pos_from_mouse_event(runner.canvas_id(), &event);
            runner.input.raw.events.push(egui::Event::PointerMoved(pos));
            runner.needs_repaint.repaint_asap();
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    runner_ref.add_event_listener(&canvas, "mouseup", |event: web_sys::MouseEvent, runner| {
        if let Some(button) = button_from_mouse_event(&event) {
            let pos = pos_from_mouse_event(runner.canvas_id(), &event);
            let modifiers = runner.input.raw.modifiers;
            runner.input.raw.events.push(egui::Event::PointerButton {
                pos,
                button,
                pressed: false,
                modifiers,
            });
            runner.needs_repaint.repaint_asap();

            text_agent::update_text_agent(runner);
        }
        event.stop_propagation();
        event.prevent_default();
    })?;

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
            let pos = pos_from_touch_event(runner.canvas_id(), &event, &mut latest_touch_pos_id);
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

    runner_ref.add_event_listener(
        &canvas,
        "touchmove",
        |event: web_sys::TouchEvent, runner| {
            let mut latest_touch_pos_id = runner.input.latest_touch_pos_id;
            let pos = pos_from_touch_event(runner.canvas_id(), &event, &mut latest_touch_pos_id);
            runner.input.latest_touch_pos_id = latest_touch_pos_id;
            runner.input.latest_touch_pos = Some(pos);
            runner.input.raw.events.push(egui::Event::PointerMoved(pos));

            push_touches(runner, egui::TouchPhase::Move, &event);
            runner.needs_repaint.repaint_asap();
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    runner_ref.add_event_listener(&canvas, "touchend", |event: web_sys::TouchEvent, runner| {
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
            runner.needs_repaint.repaint_asap();
            event.stop_propagation();
            event.prevent_default();
        }

        // Finally, focus or blur text agent to toggle mobile keyboard:
        text_agent::update_text_agent(runner);
    })?;

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
        // delta sign is flipped to match native (winit) convention.
        let delta = -egui::vec2(event.delta_x() as f32, event.delta_y() as f32);
        let modifiers = runner.input.raw.modifiers;

        runner.input.raw.events.push(egui::Event::MouseWheel {
            unit,
            delta,
            modifiers,
        });

        let scroll_multiplier = match unit {
            egui::MouseWheelUnit::Page => canvas_size_in_points(runner.canvas_id()).y,
            egui::MouseWheelUnit::Line => {
                #[allow(clippy::let_and_return)]
                let points_per_scroll_line = 8.0; // Note that this is intentionally different from what we use in winit.
                points_per_scroll_line
            }
            egui::MouseWheelUnit::Point => 1.0,
        };

        let mut delta = scroll_multiplier * delta;

        // Report a zoom event in case CTRL (on Windows or Linux) or CMD (on Mac) is pressed.
        // This if-statement is equivalent to how `Modifiers.command` is determined in
        // `modifiers_from_event()`, but we cannot directly use that fn for a [`WheelEvent`].
        if event.ctrl_key() || event.meta_key() {
            let factor = (delta.y / 200.0).exp();
            runner.input.raw.events.push(egui::Event::Zoom(factor));
        } else {
            if event.shift_key() {
                // Treat as horizontal scrolling.
                // Note: one Mac we already get horizontal scroll events when shift is down.
                delta = egui::vec2(delta.x + delta.y, 0.0);
            }

            runner.input.raw.events.push(egui::Event::Scroll(delta));
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
