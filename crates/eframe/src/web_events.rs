//! Browser input events, normalized for transmission from a host page to a worker.
//!
//! Compiled on `wasm32` and on the host target under `cfg(test)`, so the
//! semantics below are unit-tested without a browser. The host page's message
//! objects are parsed into [`WebEvent`]s by `web/web_runner.rs`.
//!
//! The JavaScript host may only copy fields out of DOM events; the semantics
//! (button numbers, wheel units, modifier derivation, coordinate conversion)
//! live here or in `web/web_runner.rs`. See `web_demo/offscreen_host.js`.

#![cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]

use egui::{Event, Modifiers, MouseWheelUnit, Pos2, Rect, TouchPhase, Vec2};

/// A mouse button, as identified by `MouseEvent::button()`: <https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent/button>
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PointerButton {
    Primary,
    Secondary,
    Middle,
    Extra1,
    Extra2,
}

impl PointerButton {
    /// Map the DOM `MouseEvent.button` number. Returns `None` for unknown buttons.
    pub fn from_dom(button: i16) -> Option<Self> {
        match button {
            0 => Some(Self::Primary),
            1 => Some(Self::Middle),
            2 => Some(Self::Secondary),
            3 => Some(Self::Extra1),
            4 => Some(Self::Extra2),
            _ => None,
        }
    }

    fn to_egui(self) -> egui::PointerButton {
        match self {
            Self::Primary => egui::PointerButton::Primary,
            Self::Secondary => egui::PointerButton::Secondary,
            Self::Middle => egui::PointerButton::Middle,
            Self::Extra1 => egui::PointerButton::Extra1,
            Self::Extra2 => egui::PointerButton::Extra2,
        }
    }
}

/// A normalized event from the host page.
///
/// All positions are in CSS pixels relative to the viewport (`clientX`/`clientY`);
/// [`WebEventState`] converts them to egui points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum WebEvent {
    /// The canvas rectangle changed (also sent once at startup).
    CanvasRect(Rect),
    PointerMove {
        client: Vec2,
        modifiers: Modifiers,
    },
    PointerButton {
        client: Vec2,
        button: Option<PointerButton>,
        pressed: bool,
        modifiers: Modifiers,
    },
    PointerGone,
    Wheel {
        delta: Vec2,

        /// `WheelEvent.deltaMode`: 0 = pixel, 1 = line, 2 = page.
        dom_delta_mode: u8,
        modifiers: Modifiers,
    },
}

/// Mutable state needed to translate [`WebEvent`]s into [`egui::Event`]s.
#[derive(Clone, Debug)]
pub(crate) struct WebEventState {
    canvas_rect: Rect,
    zoom_factor: f32,
    pointer_pos: Option<Pos2>,
}

impl Default for WebEventState {
    fn default() -> Self {
        Self {
            canvas_rect: Rect::ZERO,
            zoom_factor: 1.0,
            pointer_pos: None,
        }
    }
}

impl WebEventState {
    /// The last known pointer position, in egui points.
    #[cfg(test)]
    pub fn pointer_pos(&self) -> Option<Pos2> {
        self.pointer_pos
    }

    /// Update the canvas rectangle (in CSS pixels, padding/border adjusted).
    pub fn set_canvas_rect(&mut self, rect: Rect) {
        self.canvas_rect = rect;
    }

    /// Update the egui zoom factor (see `egui::Context::zoom_factor`).
    pub fn set_zoom_factor(&mut self, zoom_factor: f32) {
        self.zoom_factor = zoom_factor;
    }

    /// Convert a client-space position to egui points.
    fn to_points(&self, client: Vec2) -> Pos2 {
        let zoom_factor = if self.zoom_factor == 0.0 {
            1.0
        } else {
            self.zoom_factor
        };
        ((client - self.canvas_rect.min.to_vec2()) / zoom_factor).to_pos2()
    }

    /// Translate one host event into zero or more egui events.
    pub fn apply(&mut self, event: WebEvent) -> Vec<Event> {
        match event {
            WebEvent::CanvasRect(rect) => {
                self.set_canvas_rect(rect);
                Vec::new()
            }
            WebEvent::PointerMove { client, .. } => {
                let pos = self.to_points(client);
                self.pointer_pos = Some(pos);
                vec![Event::PointerMoved(pos)]
            }
            WebEvent::PointerButton {
                client,
                button,
                pressed,
                modifiers,
            } => {
                let pos = self.to_points(client);
                self.pointer_pos = Some(pos);
                let Some(button) = button.map(PointerButton::to_egui) else {
                    return Vec::new();
                };
                vec![Event::PointerButton {
                    pos,
                    button,
                    pressed,
                    modifiers,
                }]
            }
            WebEvent::PointerGone => {
                self.pointer_pos = None;
                vec![Event::PointerGone]
            }
            WebEvent::Wheel {
                delta,
                dom_delta_mode,
                modifiers,
            } => {
                let unit = match dom_delta_mode {
                    0 => MouseWheelUnit::Point,
                    1 => MouseWheelUnit::Line,
                    _ => MouseWheelUnit::Page,
                };
                vec![Event::MouseWheel {
                    unit,
                    // Match the DOM path (`install_wheel` negates `WheelEvent` deltas).
                    delta: -delta,
                    phase: TouchPhase::Move,
                    modifiers,
                }]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Pos2, Rect, Vec2};

    fn state() -> WebEventState {
        let mut state = WebEventState::default();
        state.apply(WebEvent::CanvasRect(Rect::from_min_size(
            Pos2::new(10.0, 20.0),
            Vec2::new(200.0, 100.0),
        )));
        state
    }

    #[test]
    fn pointer_move_is_relative_to_canvas() {
        let mut state = state();
        let events = state.apply(WebEvent::PointerMove {
            client: Vec2::new(30.0, 50.0),
            modifiers: Modifiers::NONE,
        });
        assert_eq!(
            events,
            vec![egui::Event::PointerMoved(Pos2::new(20.0, 30.0))]
        );
    }

    #[test]
    fn pointer_position_respects_zoom_factor() {
        let mut state = state();
        state.set_zoom_factor(2.0);
        let events = state.apply(WebEvent::PointerMove {
            client: Vec2::new(30.0, 50.0),
            modifiers: Modifiers::NONE,
        });
        assert_eq!(
            events,
            vec![egui::Event::PointerMoved(Pos2::new(10.0, 15.0))]
        );
    }

    #[test]
    fn pointer_button_maps_dom_button_numbers() {
        let mut state = state();
        for (dom, expected) in [
            (0, egui::PointerButton::Primary),
            (1, egui::PointerButton::Middle),
            (2, egui::PointerButton::Secondary),
        ] {
            let events = state.apply(WebEvent::PointerButton {
                client: Vec2::new(10.0, 20.0),
                button: PointerButton::from_dom(dom),
                pressed: true,
                modifiers: Modifiers::NONE,
            });
            assert_eq!(
                events,
                vec![egui::Event::PointerButton {
                    pos: Pos2::ZERO,
                    button: expected,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                }]
            );
        }
    }

    #[test]
    fn wheel_point_mode_maps_to_mouse_wheel() {
        let mut state = state();
        let events = state.apply(WebEvent::Wheel {
            delta: Vec2::new(0.0, 120.0),
            dom_delta_mode: 0,
            modifiers: Modifiers::NONE,
        });
        assert_eq!(
            events,
            vec![egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: Vec2::new(0.0, -120.0),
                phase: egui::TouchPhase::Move,
                modifiers: Modifiers::NONE,
            }]
        );
    }

    #[test]
    fn wheel_line_mode_maps_to_lines() {
        let mut state = state();
        let events = state.apply(WebEvent::Wheel {
            delta: Vec2::new(0.0, 3.0),
            dom_delta_mode: 1,
            modifiers: Modifiers::NONE,
        });
        assert_eq!(
            events,
            vec![egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: Vec2::new(0.0, -3.0),
                phase: egui::TouchPhase::Move,
                modifiers: Modifiers::NONE,
            }]
        );
    }

    #[test]
    fn pointer_gone_clears_position() {
        let mut state = state();
        let _ = state.apply(WebEvent::PointerMove {
            client: Vec2::new(30.0, 50.0),
            modifiers: Modifiers::NONE,
        });
        let events = state.apply(WebEvent::PointerGone);
        assert_eq!(events, vec![egui::Event::PointerGone]);
        assert_eq!(state.pointer_pos(), None);
    }

    #[test]
    fn modifiers_are_reported_from_booleans() {
        let mut state = state();
        let events = state.apply(WebEvent::PointerButton {
            client: Vec2::ZERO,
            button: Some(PointerButton::Primary),
            pressed: false,
            modifiers: Modifiers {
                alt: true,
                ctrl: false,
                shift: true,
                mac_cmd: false,
                command: false,
            },
        });
        let egui::Event::PointerButton { modifiers, .. } = events[0] else {
            panic!("expected a pointer button event");
        };
        assert!(modifiers.alt && modifiers.shift && !modifiers.ctrl);
    }
}
