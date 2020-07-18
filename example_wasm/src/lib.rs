#![deny(warnings)]
#![warn(clippy::all)]

use egui::{examples::ExampleApp, label, widgets::Separator, Align, RawInput, TextStyle, *};

use wasm_bindgen::prelude::*;

// ----------------------------------------------------------------------------

/// Data gathered between frames.
/// Is translated to `egui::RawInput` at the start of each frame.
#[derive(Default)]
pub struct WebInput {
    pub mouse_pos: Option<Pos2>,
    pub mouse_down: bool, // TODO: which button
    pub is_touch: bool,
    pub scroll_delta: Vec2,
    pub events: Vec<Event>,
}

impl WebInput {
    pub fn new_frame(&mut self) -> egui::RawInput {
        egui::RawInput {
            mouse_down: self.mouse_down,
            mouse_pos: self.mouse_pos,
            scroll_delta: std::mem::take(&mut self.scroll_delta),
            screen_size: egui_web::screen_size().unwrap(),
            pixels_per_point: Some(egui_web::pixels_per_point()),
            time: egui_web::now_sec(),
            seconds_since_midnight: Some(egui_web::seconds_since_midnight()),
            events: std::mem::take(&mut self.events),
        }
    }
}

// ----------------------------------------------------------------------------

pub struct State {
    egui_web: egui_web::EguiWeb,
    web_input: WebInput,
    example_app: ExampleApp,
}

impl State {
    fn new(canvas_id: &str) -> Result<State, JsValue> {
        Ok(State {
            egui_web: egui_web::EguiWeb::new(canvas_id)?,
            web_input: Default::default(),
            example_app: Default::default(),
        })
    }

    /// id of the canvas html element containing the rendering
    pub fn canvas_id(&self) -> &str {
        self.egui_web.canvas_id()
    }

    fn run(&mut self, raw_input: RawInput, web_location_hash: &str) -> Result<Output, JsValue> {
        let mut ui = self.egui_web.begin_frame(raw_input);
        self.ui(&mut ui, web_location_hash);
        self.egui_web.end_frame()
    }

    fn ui(&mut self, ui: &mut egui::Ui, web_location_hash: &str) {
        self.example_app.ui(ui, web_location_hash);
        let mut ui = ui.centered_column(ui.available().width().min(480.0));
        ui.set_layout(Layout::vertical(Align::Min));
        ui.add(label!("Egui!").text_style(TextStyle::Heading));
        ui.label("Egui is an immediate mode GUI written in Rust, compiled to WebAssembly, rendered with WebGL.");
        ui.label(
            "Everything you see is rendered as textured triangles. There is no DOM. There are no HTML elements."
        );
        ui.label("This is not JavaScript. This is Rust, running at 60 FPS. This is the web page, reinvented with game tech.");
        ui.label("This is also work in progress, and not ready for production... yet :)");
        ui.horizontal(|ui| {
            ui.label("Project home page:");
            ui.hyperlink("https://github.com/emilk/emigui/");
        });
        ui.add(Separator::new());

        ui.label("WebGl painter info:");
        ui.indent("webgl region id", |ui| {
            ui.label(self.egui_web.painter_debug_info());
        });

        ui.add(
            label!(
                "CPU usage: {:.2} ms (excludes painting)",
                1e3 * self.egui_web.cpu_usage()
            )
            .text_style(TextStyle::Monospace),
        );
        ui.add(label!("FPS: {:.1}", self.egui_web.fps()).text_style(TextStyle::Monospace));
    }
}

// ----------------------------------------------------------------------------
use parking_lot::Mutex;
use std::sync::Arc;

#[wasm_bindgen]
#[derive(Clone)]
pub struct StateRef(Arc<Mutex<State>>);

/// If true, paint at full framerate always.
/// If false, only paint on input.
/// TODO: if this is turned off we must turn off animations too (which hasn't been implemented yet).
const ANIMATION_FRAME: bool = true;

#[wasm_bindgen]
pub fn start(canvas_id: &str) -> Result<StateRef, JsValue> {
    let state = State::new(canvas_id)?;
    let state = StateRef(Arc::new(Mutex::new(state)));

    install_canvas_events(&state)?;
    install_document_events(&state)?;
    paint_and_schedule(state.clone())?;

    Ok(state)
}

fn paint_and_schedule(state: StateRef) -> Result<(), JsValue> {
    paint(&mut state.0.lock())?;
    if ANIMATION_FRAME {
        request_animation_frame(state)?;
    }
    Ok(())
}

fn request_animation_frame(state: StateRef) -> Result<(), JsValue> {
    use wasm_bindgen::JsCast;
    let window = web_sys::window().unwrap();
    let closure = Closure::once(move || paint_and_schedule(state));
    window.request_animation_frame(closure.as_ref().unchecked_ref())?;
    closure.forget(); // We must forget it, or else the callback is canceled on drop
    Ok(())
}

fn paint(state: &mut State) -> Result<(), JsValue> {
    egui_web::resize_to_screen_size(state.canvas_id());
    let raw_input = state.web_input.new_frame();
    let web_location_hash = egui_web::location_hash().unwrap_or_default();
    let output = state.run(raw_input, &web_location_hash)?;
    egui_web::handle_output(&output);
    Ok(())
}

fn invalidate(state: &mut State) -> Result<(), JsValue> {
    if ANIMATION_FRAME {
        Ok(()) // No need to invalidate - we repaint all the time
    } else {
        paint(state) // TODO: schedule repaint instead?
    }
}

fn install_document_events(state: &StateRef) -> Result<(), JsValue> {
    use wasm_bindgen::JsCast;
    let document = web_sys::window().unwrap().document().unwrap();

    {
        // keydown
        let state = state.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            let mut state = state.0.lock();
            let key = event.key();
            if let Some(key) = egui_web::translate_key(&key) {
                state
                    .web_input
                    .events
                    .push(Event::Key { key, pressed: true });
            } else {
                state.web_input.events.push(Event::Text(key));
            }
            invalidate(&mut state).unwrap();
        }) as Box<dyn FnMut(_)>);
        document.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        // keyup
        let state = state.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            let mut state = state.0.lock();
            let key = event.key();
            if let Some(key) = egui_web::translate_key(&key) {
                state.web_input.events.push(Event::Key {
                    key,
                    pressed: false,
                });
                invalidate(&mut state).unwrap();
            }
        }) as Box<dyn FnMut(_)>);
        document.add_event_listener_with_callback("keyup", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    for event_name in &["load", "pagehide", "pageshow", "resize"] {
        let state = state.clone();
        let closure = Closure::wrap(Box::new(move || {
            invalidate(&mut state.0.lock()).unwrap();
        }) as Box<dyn FnMut()>);
        document.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    Ok(())
}

fn install_canvas_events(state: &StateRef) -> Result<(), JsValue> {
    use egui_web::pos_from_mouse_event;
    use wasm_bindgen::JsCast;
    let canvas = egui_web::canvas_element(state.0.lock().canvas_id()).unwrap();

    {
        let event_name = "mousedown";
        let state = state.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            let mut state = state.0.lock();
            if !state.web_input.is_touch {
                state.web_input.mouse_pos = Some(pos_from_mouse_event(state.canvas_id(), &event));
                state.web_input.mouse_down = true;
                invalidate(&mut state).unwrap();
                event.stop_propagation();
                event.prevent_default();
            }
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        let event_name = "mousemove";
        let state = state.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            let mut state = state.0.lock();
            if !state.web_input.is_touch {
                state.web_input.mouse_pos = Some(pos_from_mouse_event(state.canvas_id(), &event));
                invalidate(&mut state).unwrap();
                event.stop_propagation();
                event.prevent_default();
            }
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        let event_name = "mouseup";
        let state = state.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            let mut state = state.0.lock();
            if !state.web_input.is_touch {
                state.web_input.mouse_pos = Some(pos_from_mouse_event(state.canvas_id(), &event));
                state.web_input.mouse_down = false;
                invalidate(&mut state).unwrap();
                event.stop_propagation();
                event.prevent_default();
            }
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        let event_name = "mouseleave";
        let state = state.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            let mut state = state.0.lock();
            if !state.web_input.is_touch {
                state.web_input.mouse_pos = None;
                invalidate(&mut state).unwrap();
                event.stop_propagation();
                event.prevent_default();
            }
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        let event_name = "touchstart";
        let state = state.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::TouchEvent| {
            let mut state = state.0.lock();
            state.web_input.is_touch = true;
            state.web_input.mouse_pos = Some(egui_web::pos_from_touch_event(&event));
            state.web_input.mouse_down = true;
            invalidate(&mut state).unwrap();
            event.stop_propagation();
            event.prevent_default();
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        let event_name = "touchmove";
        let state = state.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::TouchEvent| {
            let mut state = state.0.lock();
            state.web_input.is_touch = true;
            state.web_input.mouse_pos = Some(egui_web::pos_from_touch_event(&event));
            invalidate(&mut state).unwrap();
            event.stop_propagation();
            event.prevent_default();
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        let event_name = "touchend";
        let state = state.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::TouchEvent| {
            let mut state = state.0.lock();
            state.web_input.is_touch = true;
            state.web_input.mouse_down = false; // First release mouse to click...
            paint(&mut state).unwrap(); // ...do the clicking...
            state.web_input.mouse_pos = None; // ...remove hover effect
            invalidate(&mut state).unwrap();
            event.stop_propagation();
            event.prevent_default();
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        let event_name = "wheel";
        let state = state.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::WheelEvent| {
            let mut state = state.0.lock();
            state.web_input.scroll_delta.x -= event.delta_x() as f32;
            state.web_input.scroll_delta.y -= event.delta_y() as f32;
            invalidate(&mut state).unwrap();
            event.stop_propagation();
            event.prevent_default();
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    Ok(())
}
