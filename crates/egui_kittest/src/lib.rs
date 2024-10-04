mod event;
#[cfg(feature = "snapshot")]
pub mod snapshot;
#[cfg(feature = "wgpu")]
mod texture_to_bytes;
#[cfg(feature = "wgpu")]
pub mod wgpu;

pub use kittest;

use crate::event::{kittest_key_to_egui, pointer_button_to_egui};
pub use accesskit_consumer;
use egui::accesskit::NodeId;
use egui::{Event, Modifiers, Pos2, Rect, TexturesDelta, Vec2};
use kittest::{ElementState, Node, Queryable, SimulatedEvent, State};

pub struct Harness<'a> {
    pub ctx: egui::Context,
    input: egui::RawInput,
    tree: Option<State>,
    output: Option<egui::FullOutput>,
    texture_deltas: Vec<TexturesDelta>,
    update_fn: Box<dyn FnMut(&egui::Context) + 'a>,

    last_mouse_pos: Pos2,
    modifiers: Modifiers,
}

impl<'a> Harness<'a> {
    pub fn new(app: impl FnMut(&egui::Context) + 'a) -> Self {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();

        Self {
            update_fn: Box::new(app),
            ctx,
            input: egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
                ..Default::default()
            },
            tree: None,
            output: None,
            texture_deltas: Vec::new(),

            last_mouse_pos: Pos2::ZERO,
            modifiers: Modifiers::NONE,
        }
    }

    #[inline]
    pub fn with_size(mut self, size: Vec2) -> Self {
        self.input.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, size));
        self
    }

    #[inline]
    pub fn with_dpi(mut self, dpi: f32) -> Self {
        self.input
            .viewports
            .get_mut(&self.input.viewport_id)
            .unwrap()
            .native_pixels_per_point = Some(dpi);
        self
    }

    pub fn run(&mut self) {
        if let Some(tree) = &mut self.tree {
            for event in tree.take_events() {
                match event {
                    kittest::Event::ActionRequest(e) => {
                        self.input.events.push(Event::AccessKitActionRequest(e));
                    }
                    kittest::Event::Simulated(e) => match e {
                        SimulatedEvent::CursorMoved { position } => {
                            self.input.events.push(Event::PointerMoved(Pos2::new(
                                position.x as f32,
                                position.y as f32,
                            )));
                        }
                        SimulatedEvent::MouseInput { state, button } => {
                            let button = pointer_button_to_egui(button);
                            if let Some(button) = button {
                                self.input.events.push(Event::PointerButton {
                                    button,
                                    modifiers: self.modifiers,
                                    pos: self.last_mouse_pos,
                                    pressed: matches!(state, ElementState::Pressed),
                                });
                            }
                        }
                        SimulatedEvent::Ime(text) => {
                            self.input.events.push(Event::Text(text));
                        }
                        SimulatedEvent::KeyInput { state, key } => {
                            match key {
                                kittest::Key::Alt => {
                                    self.modifiers.alt = matches!(state, ElementState::Pressed);
                                }
                                kittest::Key::Command => {
                                    self.modifiers.command = matches!(state, ElementState::Pressed);
                                }
                                kittest::Key::Control => {
                                    self.modifiers.ctrl = matches!(state, ElementState::Pressed);
                                }
                                kittest::Key::Shift => {
                                    self.modifiers.shift = matches!(state, ElementState::Pressed);
                                }
                                _ => {}
                            }
                            let key = kittest_key_to_egui(key);
                            if let Some(key) = key {
                                self.input.events.push(Event::Key {
                                    key,
                                    modifiers: self.modifiers,
                                    pressed: matches!(state, ElementState::Pressed),
                                    repeat: false,
                                    physical_key: None,
                                });
                            }
                        }
                    },
                }
            }
        }
        let mut output = self.ctx.run(self.input.take(), self.update_fn.as_mut());
        if let Some(tree) = &mut self.tree {
            tree.update(
                output
                    .platform_output
                    .accesskit_update
                    .take()
                    .expect("AccessKit was disabled"),
            );
        } else {
            self.tree = Some(State::new(
                output
                    .platform_output
                    .accesskit_update
                    .take()
                    .expect("AccessKit was disabled"),
            ));
        }
        self.output = Some(output);
        self.texture_deltas
            .push(self.output().textures_delta.clone());
    }

    pub fn click(&mut self, id: NodeId) {
        let action = egui::accesskit::ActionRequest {
            target: id,
            action: egui::accesskit::Action::Default,
            data: None,
        };
        self.input
            .events
            .push(egui::Event::AccessKitActionRequest(action));
    }

    pub fn focus(&mut self, id: NodeId) {
        let action = egui::accesskit::ActionRequest {
            target: id,
            action: egui::accesskit::Action::Focus,
            data: None,
        };
        self.input
            .events
            .push(Event::AccessKitActionRequest(action));
    }

    // TODO(lucasmerlin): SetValue is currently not supported by egui
    // pub fn set_text(&mut self, id: NodeId, text: &str) {
    //     let action = egui::accesskit::ActionRequest {
    //         target: id,
    //         action: egui::accesskit::Action::SetValue,
    //         data: Some(ActionData::Value(Box::from(text))),
    //     };
    //     self.input
    //         .events
    //         .push(egui::Event::AccessKitActionRequest(action));
    // }

    pub fn type_text(&mut self, id: NodeId, text: &str) {
        self.focus(id);
        self.input.events.push(egui::Event::Text(text.to_owned()));
    }

    pub fn input(&self) -> &egui::RawInput {
        &self.input
    }

    pub fn input_mut(&mut self) -> &mut egui::RawInput {
        &mut self.input
    }

    pub fn output(&self) -> &egui::FullOutput {
        self.output.as_ref().expect("Not initialized")
    }

    pub fn kittest_state(&self) -> &State {
        self.tree.as_ref().expect("Not initialized")
    }
}

impl<'t, 'n, 'h> Queryable<'t, 'n> for Harness<'h>
where
    'n: 't,
{
    fn node(&'n self) -> Node<'t> {
        self.kittest_state().node()
    }
}
