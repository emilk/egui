#[cfg(feature = "snapshot")]
pub mod snapshot;
#[cfg(feature = "wgpu")]
mod texture_to_bytes;
mod utils;
#[cfg(feature = "wgpu")]
pub mod wgpu;

use crate::utils::egui_vec2;
pub use accesskit_consumer;
use egui::accesskit::NodeId;
use egui::{Pos2, Rect, TexturesDelta, Vec2};
use kittest::{Node, Queryable, SimulatedEvent, Tree};
use std::iter;
use std::time::Duration;

pub struct Harness<'a> {
    pub ctx: egui::Context,
    input: egui::RawInput,
    tree: Option<Tree>,
    output: Option<egui::FullOutput>,
    texture_deltas: Vec<TexturesDelta>,
    update_fn: Box<dyn FnMut(&egui::Context) + 'a>,
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
        }
    }

    pub fn with_size(mut self, size: Vec2) -> Self {
        self.input.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, size));
        self
    }

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
                        self.input
                            .events
                            .push(egui::Event::AccessKitActionRequest(e));
                    }
                    kittest::Event::Simulated(e) => match e {
                        SimulatedEvent::Click { position } => {
                            let position = egui_vec2(position).to_pos2();
                            self.input.events.push(egui::Event::PointerButton {
                                pos: position,
                                button: egui::PointerButton::Primary,
                                pressed: true,
                                modifiers: Default::default(),
                            });
                            self.input.events.push(egui::Event::PointerButton {
                                pos: position,
                                button: egui::PointerButton::Primary,
                                pressed: false,
                                modifiers: Default::default(),
                            });
                        }
                        SimulatedEvent::Type { text } => {
                            self.input.events.push(egui::Event::Text(text));
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
            self.tree = Some(Tree::new(
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
            .push(egui::Event::AccessKitActionRequest(action));
    }

    // TODO: SetValue is currently not supported by egui
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

    pub fn tree(&self) -> &Tree {
        self.tree.as_ref().expect("Not initialized")
    }
}

impl<'t, 'n, 'h> Queryable<'t, 'n> for Harness<'h>
where
    'n: 't,
{
    fn node(&'n self) -> Node<'t> {
        self.tree().node()
    }
}
