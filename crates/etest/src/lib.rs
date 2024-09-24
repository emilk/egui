#[cfg(feature = "snapshot")]
pub mod snapshot;
#[cfg(feature = "wgpu")]
mod texture_to_bytes;
#[cfg(feature = "wgpu")]
pub mod wgpu;

pub use accesskit_consumer;
use accesskit_consumer::{Node, Tree};
use egui::accesskit::NodeId;
use egui::{Pos2, Rect, TexturesDelta, Vec2};

pub struct Harness {
    pub ctx: egui::Context,
    input: egui::RawInput,
    tree: Option<Tree>,
    output: Option<egui::FullOutput>,
    texture_deltas: Vec<TexturesDelta>,
}

impl Default for Harness {
    fn default() -> Self {
        Self::new()
    }
}

impl Harness {
    pub fn new() -> Self {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();

        Self {
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

    pub fn run(&mut self, app: impl FnMut(&egui::Context)) {
        let mut output = self.ctx.run(self.input.take(), app);
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
                true,
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

    pub fn root(&self) -> Node<'_> {
        self.tree().state().root()
    }
}
